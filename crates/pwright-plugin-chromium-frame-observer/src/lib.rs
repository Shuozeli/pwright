//! Optional synchronous observation and metadata-projection contracts for a
//! Chromium topology driver's ordered cursor.
//!
//! This contract owns no connection, queue, subscriber, task, executor,
//! browser process, native domain, or topology driver. Implementations admit
//! their own storage. A selected driver invokes callbacks while holding its
//! topology lock; an unselected assembly installs no observer or work.

use std::error::Error;

use pwright_backend_cdp_topology::CdpTopology;
use pwright_backend_cdp_topology::{CdpContextOwnership, CdpTopologyEffect};
use pwright_cdp::{CdpEvent, generated::page};
use pwright_core::ObjectId;

pub type FrameObservationResult = Result<(), Box<dyn Error + Send + Sync>>;

/// Synchronous observation port for one production driver's ordered cursor.
///
/// Callbacks run under the topology lock and must not reenter topology, their
/// own observer, or JavaScript.
pub trait ChromiumFrameEventObserver: Send + Sync {
    fn observes_page(&self, page: ObjectId) -> bool;

    /// Keep dedicated-worker child sessions attached so an opt-in observer can
    /// publish and drive them. The topology driver still owns the single CDP
    /// cursor and performs the bounded native initialization before delivering
    /// the attachment event. The default preserves the historical behavior of
    /// detaching every non-Page/non-OOPIF child target.
    fn retains_worker_targets(&self) -> bool {
        false
    }

    /// Whether a retained Worker session is itself the parent of a nested
    /// dedicated Worker attachment. Page/OOPIF parents are resolved by the
    /// topology driver and do not use this callback.
    fn retains_worker_session(&self, _session: &str) -> bool {
        false
    }

    /// Enable Chromium's Network domain in every retained dedicated-Worker
    /// session before releasing its debugger pause. This remains opt-in so a
    /// Worker-only installation does not pay for network observation.
    fn enables_worker_network(&self) -> bool {
        false
    }

    /// Admit selected Page storage before enabling native domains. Newly
    /// discovered Page IDs may not have been published in topology yet.
    fn prepare_page(&self, _topology: &CdpTopology, _page: ObjectId) -> FrameObservationResult {
        Ok(())
    }

    /// Release an abandoned pre-publication reservation. A replacement attempt
    /// can reuse a live Page ID: implementations must preserve published owners.
    fn discard_prepared_page(
        &self,
        _topology: &CdpTopology,
        _page: ObjectId,
    ) -> FrameObservationResult {
        Ok(())
    }

    fn initialize_frame(
        &self,
        topology: &CdpTopology,
        session: &str,
        frame: &page::Frame,
    ) -> FrameObservationResult;
    fn reconcile(&self, topology: &CdpTopology) -> FrameObservationResult;
    fn event(&self, topology: &CdpTopology, event: &CdpEvent) -> FrameObservationResult;

    /// Terminal and non-blocking: retained handles must stop exposing readiness.
    fn seal(&self);
}

/// Optional synchronous projection of committed Frame URLs. Implementations
/// own their retention budgets and must not reenter topology from callbacks.
/// No observer, URL cache, task, or extra CDP subscription is installed by
/// default.
pub trait ChromiumFrameUrlObserver: Send + Sync {
    fn update(&self, frame: ObjectId, url: &str, fragment: Option<&str>) -> FrameObservationResult;

    /// Full navigation metadata. URL-only projections need not retain names.
    /// Same-document events continue to call `update`, preserving the name.
    fn update_navigation(
        &self,
        frame: ObjectId,
        url: &str,
        fragment: Option<&str>,
        _name: &str,
    ) -> FrameObservationResult {
        self.update(frame, url, fragment)
    }

    fn retain_live(&self, is_live: &dyn Fn(ObjectId) -> bool) -> FrameObservationResult;

    /// Optional ownership projection after URL observations for this committed
    /// update. The borrowed view needs no execution world or new allocation.
    fn reconcile_ownership(&self, _view: ChromiumFrameOwnershipView<'_>) -> FrameObservationResult {
        Ok(())
    }

    /// Observe the exact effects of one committed topology transaction together
    /// with its coherent ownership view. Default observers keep their previous
    /// snapshot behavior. This is native metadata projection under the topology
    /// lock, never a foreign callback/delivery boundary: do not reenter topology.
    /// Retention requires caller-owned admission before native mutation.
    /// Only driver transactions reach this hook. Direct lifecycle-service commits
    /// and terminal teardown are not an exhaustive event source here; snapshot
    /// refresh still uses `reconcile_ownership` without replaying effects.
    /// Errors propagate after native commit; they do not roll back that commit.
    fn reconcile_committed(
        &self,
        publication: ChromiumCommittedFrameOwnership<'_>,
    ) -> FrameObservationResult {
        self.reconcile_ownership(publication.ownership())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChromiumFrameOwnership {
    pub frame: ObjectId,
    pub page: ObjectId,
    pub parent: Option<ObjectId>,
}

#[derive(Clone, Copy)]
pub struct ChromiumFrameOwnershipView<'a>(&'a CdpTopology);

/// Borrowed committed cause data, not a reconstructed collection difference.
/// No queue, observer, allocation, or event subscription is installed by this
/// value. A later snapshot-only refresh carries no replay of these effects.
/// This view is delivered before a driver combines several transactions into
/// its outer outcome, and is not a Playwright callback group or delivery fence.
#[derive(Clone, Copy)]
pub struct ChromiumCommittedFrameOwnership<'a> {
    ownership: ChromiumFrameOwnershipView<'a>,
    effects: &'a [CdpTopologyEffect],
}

impl<'a> ChromiumCommittedFrameOwnership<'a> {
    /// Driver-integration constructor over one already committed transaction.
    pub const fn new(
        ownership: ChromiumFrameOwnershipView<'a>,
        effects: &'a [CdpTopologyEffect],
    ) -> Self {
        Self { ownership, effects }
    }

    pub const fn ownership(self) -> ChromiumFrameOwnershipView<'a> {
        self.ownership
    }

    /// Exact transaction order, including ready/linkage/disposal effects.
    /// Popup linkage can be deferred to a different readiness transaction;
    /// consumers must not assume every link follows a ready effect for itself.
    pub const fn effects(self) -> &'a [CdpTopologyEffect] {
        self.effects
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChromiumContextMembership {
    pub id: ObjectId,
    pub owned: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChromiumPageMembership {
    pub id: ObjectId,
    pub context: ObjectId,
}

impl<'a> ChromiumFrameOwnershipView<'a> {
    /// Driver-integration constructor over the coherent borrowed topology.
    pub const fn new(topology: &'a CdpTopology) -> Self {
        Self(topology)
    }

    /// Borrowed portable membership; no additional browser domain or allocation.
    pub fn collection_contexts(&self) -> impl Iterator<Item = ChromiumContextMembership> + '_ {
        self.0
            .browser_context_routes()
            .map(|context| ChromiumContextMembership {
                id: context.object_id(),
                owned: context.ownership() == CdpContextOwnership::Owned,
            })
    }

    pub fn collection_pages(&self) -> impl Iterator<Item = ChromiumPageMembership> + '_ {
        self.0
            .materialized_page_memberships()
            .map(|(id, context)| ChromiumPageMembership { id, context })
    }

    pub fn frames(&self) -> impl Iterator<Item = ChromiumFrameOwnership> + '_ {
        self.0
            .frame_page_routes()
            .map(|(frame, page)| ChromiumFrameOwnership {
                frame,
                page,
                parent: self.0.parent_frame_object_id(frame),
            })
    }

    /// Page liveness is independent of renderer/session/world availability.
    pub fn contains_page(&self, page: ObjectId) -> bool {
        self.0.main_frame_native_id(page).is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use pwright_backend_cdp_topology::{CdpTopologyLimits, ConnectionEpoch};

    use super::*;

    struct MinimalObserver;

    #[derive(Default)]
    struct RecordingUrlObserver(Mutex<Vec<(ObjectId, String, Option<String>)>>);

    impl ChromiumFrameUrlObserver for RecordingUrlObserver {
        fn update(
            &self,
            frame: ObjectId,
            url: &str,
            fragment: Option<&str>,
        ) -> FrameObservationResult {
            self.0.lock().expect("URL observations").push((
                frame,
                url.to_owned(),
                fragment.map(str::to_owned),
            ));
            Ok(())
        }

        fn retain_live(&self, _is_live: &dyn Fn(ObjectId) -> bool) -> FrameObservationResult {
            Ok(())
        }
    }

    impl ChromiumFrameEventObserver for MinimalObserver {
        fn observes_page(&self, _page: ObjectId) -> bool {
            false
        }

        fn initialize_frame(
            &self,
            _topology: &CdpTopology,
            _session: &str,
            _frame: &page::Frame,
        ) -> FrameObservationResult {
            Ok(())
        }

        fn reconcile(&self, _topology: &CdpTopology) -> FrameObservationResult {
            Ok(())
        }

        fn event(&self, _topology: &CdpTopology, _event: &CdpEvent) -> FrameObservationResult {
            Ok(())
        }

        fn seal(&self) {}
    }

    fn topology() -> CdpTopology {
        CdpTopology::new(
            ObjectId::new(1),
            ConnectionEpoch::new(1),
            CdpTopologyLimits::new(8, 1, 1, 1, 1, 32),
        )
        .expect("minimal topology")
    }

    #[test]
    fn page_preparation_is_optional_by_default() {
        // Arrange
        let observer = MinimalObserver;
        let topology = topology();

        // Act
        let result = observer.prepare_page(&topology, ObjectId::new(2));

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn abandoned_page_cleanup_is_optional_by_default() {
        // Arrange
        let observer = MinimalObserver;
        let topology = topology();

        // Act
        let result = observer.discard_prepared_page(&topology, ObjectId::new(2));

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn collection_membership_survives_transient_reconnect_routing_loss() {
        // Arrange
        let mut topology = topology();
        topology
            .register_context(
                ConnectionEpoch::new(1),
                "context-a",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should register");
        topology
            .attach_page(
                ConnectionEpoch::new(1),
                pwright_backend_cdp_topology::CdpPageTarget::new(
                    "target-a",
                    "session-a",
                    "context-a",
                ),
                pwright_backend_cdp_topology::CdpPageObjectIds::new(
                    ObjectId::new(3),
                    ObjectId::new(4),
                ),
            )
            .expect("page should attach");
        topology
            .begin_reconnect(ConnectionEpoch::new(2))
            .expect("new epoch should begin");

        // Act
        let pages = ChromiumFrameOwnershipView::new(&topology)
            .collection_pages()
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(
            pages,
            vec![ChromiumPageMembership {
                id: ObjectId::new(3),
                context: ObjectId::new(2),
            }]
        );
        assert_eq!(topology.page_session_routes().count(), 0);
    }

    #[test]
    fn navigation_metadata_defaults_to_the_url_projection() {
        // Arrange
        let observer = RecordingUrlObserver::default();

        // Act
        observer
            .update_navigation(
                ObjectId::new(7),
                "https://example.test/path",
                Some("fragment"),
                "ignored-by-url-only-observer",
            )
            .expect("default navigation projection");

        // Assert
        assert_eq!(
            *observer.0.lock().expect("URL observations"),
            vec![(
                ObjectId::new(7),
                "https://example.test/path".to_owned(),
                Some("fragment".to_owned()),
            )]
        );
    }
}
