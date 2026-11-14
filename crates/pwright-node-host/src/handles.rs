//! Opt-in native Handle routes, owned publication, value reads and disposal.
//! The Node wrapper consumes these owners without exporting native IDs.

mod arguments;
mod batch_publication;
mod creation;
#[cfg(feature = "retained-file-input")]
pub(crate) mod file_input;
#[cfg(feature = "retained-serialized-payload-mutation")]
pub use file_input::serialized_payload_mutation_language_error;
#[cfg(feature = "retained-target-marking")]
pub use file_input::{
    RetainedMarkedFileInput, RetainedTargetMarkOutcome, TargetMarkOutcome,
    markers::MarkerCacheError, target_mark_language_error,
};
#[cfg(feature = "retained-target-prevalidation")]
pub use file_input::{
    RetainedPrevalidatedFileInput, RetainedTargetPrevalidationFlags,
    target_prevalidation_language_error,
};
#[cfg(feature = "retained-serialized-payload-import")]
pub use file_input::{
    RetainedSerializedPayload, SerializedPayloadAdmission, SerializedPayloadMetrics,
    serialized_payload_limits,
};
mod operations;
mod owner_frame;
mod pending;
mod properties;
mod publication;
mod release;
pub use arguments::{HandleArgumentSlot, OwnedHandleArgument};
pub(crate) use creation::Installation;
#[cfg(feature = "retained-file-input")]
pub use file_input::{
    RetainedFileInputRegistration, ScopedTimeoutSettings, TimeoutPolicyBudget,
    TimeoutPolicySession, TimeoutSettings,
};
#[cfg(feature = "retained-file-input")]
pub use pending::HandleSourceIdentity;
#[cfg(feature = "element-handle-click")]
pub(crate) use pending::ReadAdmission;
pub use pending::{HandleRelease, PendingHandle};
pub use properties::NamedPendingHandle;
pub use pwright_cap_handle_evaluation::{EvaluateHandleParams, HandleEvaluateParams};
pub use pwright_composition_chromium_handle_evaluation::CdpHandleEvaluationProgram as HandleEvaluationProgram;

/// Ordinary cancellation capacity, one recovery/result ticket per Handle and
/// eight abandoned-creation continuations plus eight unpublished batch
/// compensation jobs in the same executor. The opt-in retained click adds
/// eight parent-owner group continuations. Sibling tickets are independent.
pub(crate) const CLEANUP_SLOTS: usize = 32
    + MAX_HANDLES
    + 2 * MAX_CALLS
    + if cfg!(feature = "element-handle-click") {
        MAX_CALLS
    } else {
        0
    };

use crate::{HostError, HostSession, evaluation};
use pwright_backend_cdp_evaluation::{CdpEvaluationLimits, CdpEvaluationProductionProfile};
use pwright_codec_handle_evaluation::HandleEvaluationCodecLimits;
use pwright_codec_handle_lifecycle::HandleLifecycleCodecLimits;
use pwright_composition_chromium::{
    CapabilityWireProfile, ChromiumAttachProductionProfile, ChromiumHandleRouteProductionProfile,
    HandleEvaluationInstallationProfile, HandleLifecycleInstallationProfile,
};
pub use pwright_composition_chromium_handle_lifecycle::ChromiumHandleRouteMetrics as HandleMetrics;
use pwright_core::{ProtocolVersion, ResourceScope};
use pwright_transport::FrameLimits;
use std::num::NonZeroUsize;
#[cfg(feature = "console-argument-handles")]
use std::sync::Arc;

#[cfg(feature = "console-argument-handles")]
pub(crate) struct PendingEventHandles {
    pub(super) received: tokio::sync::oneshot::Receiver<batch_publication::Outcome>,
}

#[cfg(feature = "console-argument-handles")]
pub(crate) struct ReadyEventHandles {
    handles: Box<[Arc<PendingHandle>]>,
    _storage: Box<[Arc<pending::Allowance>]>,
}

#[cfg(feature = "console-argument-handles")]
impl PendingEventHandles {
    pub(crate) fn try_finish(&mut self) -> Result<Option<ReadyEventHandles>, HostError> {
        let outcome = match self.received.try_recv() {
            Ok(outcome) => outcome,
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => return Ok(None),
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                return Err(HostError::backend(
                    "event Handle publication ended without an outcome",
                ));
            }
        };
        let values = outcome.map_err(|error| {
            let mut details = None;
            let error = error.map_details(|value| details = Some(value));
            details.unwrap_or_else(|| HostError::operation(error, 0))
        })?;
        let mut handles = Vec::with_capacity(values.len());
        let mut storage = Vec::with_capacity(values.len());
        for value in values {
            storage.push(value._storage);
            handles.push(Arc::new(value.handle));
        }
        Ok(Some(ReadyEventHandles {
            handles: handles.into_boxed_slice(),
            _storage: storage.into_boxed_slice(),
        }))
    }
}

#[cfg(feature = "console-argument-handles")]
impl ReadyEventHandles {
    pub(crate) fn handles(&self) -> &[Arc<PendingHandle>] {
        &self.handles
    }
}

/// Explicit aggregate capacity, not one allocation for every Frame. Together
/// with Browser/16 Contexts/16 Pages/64 Frames this fits 256 runtime objects.
pub const MAX_HANDLES: usize = 64;
#[cfg(feature = "console-argument-handles")]
pub(crate) const MAX_EVENT_ARGUMENT_HANDLES: usize = 16;
pub const MAX_NATIVE_ID_BYTES: usize = 1024;
pub const MAX_PREVIEW_BYTES: usize = 1024;
pub const MAX_CALLS: usize = 8;
// Inline state for the largest staged producer, including publication. This
// remains inside each existing 8 MiB call allowance; no new lease is added.
pub(crate) const CREATION_FUTURE_BYTES: usize = 8 * 1024;

pub(crate) fn select(
    profile: ChromiumAttachProductionProfile,
) -> Result<ChromiumAttachProductionProfile, HostError> {
    let runtime_objects = NonZeroUsize::new(profile.runtime().live_objects).ok_or_else(|| {
        HostError::backend("Handle release requires a nonzero runtime object limit")
    })?;
    let wire = |version| {
        CapabilityWireProfile::new(
            version,
            FrameLimits::new(2 * 1024 * 1024, 0, 0, 2 * 1024 * 1024),
            FrameLimits::new(2 * 1024 * 1024, 0, 0, 2 * 1024 * 1024),
        )
    };
    Ok(profile
        .with_extension_requirements(pending::requirements())
        .with_extension_requirements(release::requirements(runtime_objects)?)
        .with_extension_requirements(publication::requirements())
        .with_extension_requirements(batch_publication::requirements())
        .with_handle_lifecycle(HandleLifecycleInstallationProfile::new(
            ChromiumHandleRouteProductionProfile::new(
                ResourceScope::extension(152, 1),
                MAX_HANDLES,
                MAX_NATIVE_ID_BYTES,
                MAX_PREVIEW_BYTES,
            ),
            HandleLifecycleCodecLimits::new(evaluation::ERROR_BYTES),
            wire(ProtocolVersion::new(1, 0)),
        ))
        .with_handle_evaluation(HandleEvaluationInstallationProfile::new(
            CdpEvaluationProductionProfile::new(
                ResourceScope::extension(152, 2),
                CdpEvaluationLimits::new(
                    evaluation::SOURCE_BYTES,
                    evaluation::GENERATED_SCRIPT_BYTES,
                    evaluation::ERROR_BYTES,
                    evaluation::input_value_limits(),
                ),
                MAX_CALLS,
            )
            .with_user_gesture(true),
            MAX_NATIVE_ID_BYTES,
            MAX_PREVIEW_BYTES,
            HandleEvaluationCodecLimits::new(
                evaluation::SOURCE_BYTES,
                evaluation::ERROR_BYTES,
                evaluation::input_value_limits(),
            )
            .map_err(HostError::backend)?,
            wire(ProtocolVersion::new(1, 2)),
        )))
}

impl HostSession {
    /// Native route accounting, not browser RSS or JS wrapper heap size.
    /// Closed sessions report an empty installation.
    pub fn handle_metrics(&self) -> Result<HandleMetrics, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        let Some(live) = state.as_ref() else {
            return Ok(HandleMetrics::default());
        };
        live.runtime
            .service()
            .handle_leaf_routes()
            .ok_or_else(|| HostError::backend("selected Handle route owner is missing"))?
            .metrics()
            .map_err(HostError::backend)
    }
}

#[cfg(test)]
mod tests;
