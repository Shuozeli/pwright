//! Optional caller-owned initialization hook for newly discovered Chromium
//! sessions.
//!
//! This contract owns no connection, command implementation, queue, task,
//! executor, browser process, or topology driver. A driver invokes a selected
//! plugin before debugger resume and Page readiness; an unselected assembly
//! installs no settings or work.

use std::error::Error;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_core::ObjectId;

/// One caller-owned initialization operation.
pub type ChromiumSessionInitializationFuture<'a> =
    Pin<Box<dyn Future<Output = Result<(), Box<dyn Error + Send + Sync>>> + Send + 'a>>;

/// Target classes currently admitted by the Chromium topology driver.
/// Worker attachment is not represented as an out-of-process Frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumSessionTargetKind {
    Page,
    OutOfProcessFrame,
}

/// An opt-in, per-attachment command enqueue fence. Initializers activate it
/// only when a native command must reach the wire queue before debugger resume.
/// The driver keeps polling that same initializer while the fence is pending;
/// no task or retained session state is created here.
#[derive(Debug, Default)]
pub struct ChromiumSessionResumeFence(AtomicUsize);

impl PartialEq for ChromiumSessionResumeFence {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for ChromiumSessionResumeFence {}

impl ChromiumSessionResumeFence {
    /// Require the initializer's selected command to be queued before resume.
    pub fn require_enqueue(&self) {
        self.0.fetch_add(1, Ordering::AcqRel);
    }

    /// The selected command has been admitted to the ordered CDP write queue.
    pub fn command_enqueued(&self) {
        let outstanding = self.0.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(outstanding > 0, "resume fence enqueue was not required");
    }

    /// Any pending requirement must keep the target paused.
    pub fn is_waiting(&self) -> bool {
        self.0.load(Ordering::Acquire) != 0
    }
}

/// Borrowed native routing and portable ownership selected before debugger
/// resume. New Pages carry their reserved ID before topology publication; IDs
/// are not proof of public readiness. Retention requires caller-owned
/// admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumSessionInitializationTarget<'a> {
    pub session_id: &'a str,
    pub target_id: &'a str,
    pub page_target_id: &'a str,
    pub page: ObjectId,
    pub context: ObjectId,
    pub ownership: CdpContextOwnership,
    pub kind: ChromiumSessionTargetKind,
    /// Borrowed only during discovered-session initialization. Existing
    /// initializers leave it untouched and retain their nonblocking resume.
    pub resume_fence: Option<&'a ChromiumSessionResumeFence>,
}

/// Plugin invoked for a discovered Page or out-of-process Frame session.
pub trait ChromiumSessionInitializer: Send + Sync {
    fn initialize<'a>(
        &'a self,
        session_id: &'a str,
        target_id: &'a str,
        ownership: CdpContextOwnership,
        is_page: bool,
    ) -> ChromiumSessionInitializationFuture<'a>;

    /// Identity-aware extension. Existing initializers retain their original
    /// behavior without allocating a wrapper or another initialization task.
    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        self.initialize(
            target.session_id,
            target.target_id,
            target.ownership,
            target.kind == ChromiumSessionTargetKind::Page,
        )
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    #[derive(Debug, Eq, PartialEq)]
    struct RecordedInitialization {
        session_id: String,
        target_id: String,
        ownership: CdpContextOwnership,
        is_page: bool,
    }

    #[derive(Default)]
    struct RecordingInitializer(Mutex<Option<RecordedInitialization>>);

    impl ChromiumSessionInitializer for RecordingInitializer {
        fn initialize<'a>(
            &'a self,
            session_id: &'a str,
            target_id: &'a str,
            ownership: CdpContextOwnership,
            is_page: bool,
        ) -> ChromiumSessionInitializationFuture<'a> {
            *self.0.lock().expect("recording initializer lock") = Some(RecordedInitialization {
                session_id: session_id.to_owned(),
                target_id: target_id.to_owned(),
                ownership,
                is_page,
            });
            Box::pin(async { Ok(()) })
        }
    }

    #[test]
    fn scoped_initialization_preserves_legacy_page_arguments() {
        // Arrange
        let initializer = RecordingInitializer::default();
        let target = ChromiumSessionInitializationTarget {
            session_id: "session-7",
            target_id: "target-7",
            page_target_id: "target-7",
            page: ObjectId::new(7),
            context: ObjectId::new(3),
            ownership: CdpContextOwnership::Owned,
            kind: ChromiumSessionTargetKind::Page,
            resume_fence: None,
        };

        // Act
        drop(initializer.initialize_scoped(target));

        // Assert
        assert_eq!(
            *initializer.0.lock().expect("recording initializer lock"),
            Some(RecordedInitialization {
                session_id: "session-7".to_owned(),
                target_id: "target-7".to_owned(),
                ownership: CdpContextOwnership::Owned,
                is_page: true,
            })
        );
    }

    #[test]
    fn resume_waits_for_every_selected_command() {
        // Arrange
        let fence = ChromiumSessionResumeFence::default();

        // Act
        fence.require_enqueue();
        fence.require_enqueue();
        fence.command_enqueued();

        // Assert
        assert!(fence.is_waiting());
        fence.command_enqueued();
        assert!(!fence.is_waiting());
    }
}
