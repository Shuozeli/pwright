//! Opt-in exact retained ElementHandle click for top-level and one-link Chromium Frames.

#[path = "element_handle_click_parent_hit.rs"]
mod parent_hit;

use crate::{HostError, HostSession, handles::PendingHandle};
use pwright_backend_cdp_locator::{
    CdpOwnedMousePress, RetainedClickChildTarget, RetainedClickDiagnostics,
};
use pwright_browser_model::ELEMENT_HANDLE_OBJECT_KIND_ID;
use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_cap_locator::{ClickOptionsV2, LocatorErrorDetails};
use pwright_composition_chromium::{
    CdpPageServices, ChromiumFrameWorldSnapshot, ChromiumLocatorKeyboardRoute,
};
use pwright_composition_chromium_handle_evaluation::ChromiumRetainedElementSource;
use pwright_core::{
    CallId, CallMetadata, CancellationReason, Command, CommandCancellation, CommandSpec,
    FailureKind, MethodTag, ObjectId, ObjectKindId, OperationContext, OperationError,
};
use pwright_runtime::TargetedService;
use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

struct RetainedElementClick;

const RETAINED_CLICK_TIMEOUT_MARKER: &str =
    "retained ElementHandle click actionability deadline exceeded";

impl CommandSpec for RetainedElementClick {
    const METHOD: MethodTag = MethodTag::new(65009);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[ELEMENT_HANDLE_OBJECT_KIND_ID];
    type Params = Params;
    type Result = ();
    type ErrorDetails = LocatorErrorDetails;
}

struct Params {
    handle: Arc<PendingHandle>,
    admission: crate::handles::ReadAdmission,
    frame_route: ClickFrameRoute,
    options: ClickOptionsV2,
}

enum ClickFrameRoute {
    TopLevel {
        frame: ObjectId,
    },
    OneLinkChild {
        frame: ObjectId,
        parent: ObjectId,
        frame_id: String,
        parent_frame_id: String,
    },
}

impl ClickFrameRoute {
    fn frame(&self) -> ObjectId {
        match self {
            Self::TopLevel { frame } | Self::OneLinkChild { frame, .. } => *frame,
        }
    }
}

struct Invocation {
    call_id: CallId,
    handles: Arc<crate::handles::Installation>,
    services: CdpPageServices,
    route: OnceLock<ChromiumLocatorKeyboardRoute>,
    owner: OnceLock<Arc<ClickOwner>>,
    press: OnceLock<CdpOwnedMousePress>,
    diagnostics: RetainedClickDiagnostics,
}

// The read admission alone does not pin a browser document. Keep the exact
// source and one bounded observed-world snapshot through command cancellation.
struct ClickOwner {
    call_id: CallId,
    next_parent_group: AtomicU64,
    handles: Arc<crate::handles::Installation>,
    snapshot: ChromiumFrameWorldSnapshot,
    parent_snapshot: Option<ChromiumFrameWorldSnapshot>,
    frame_route: ClickFrameRoute,
    source: ChromiumRetainedElementSource,
    handle: Arc<PendingHandle>,
    _admission: crate::handles::ReadAdmission,
}

impl ClickOwner {
    async fn check_parent_hit(
        &self,
        x: f64,
        y: f64,
    ) -> Result<bool, OperationError<LocatorErrorDetails>> {
        self.check_current()?;
        let ClickFrameRoute::OneLinkChild { frame_id, .. } = &self.frame_route else {
            return Err(closed_route());
        };
        let parent = self.parent_snapshot.as_ref().ok_or_else(closed_route)?;
        let sequence = self
            .next_parent_group
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                value.checked_add(1)
            })
            .map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "retained click parent-owner group IDs exhausted",
                    None,
                )
            })?;
        let group = format!(
            "pwright.retained-click.parent.{}.{}",
            self.call_id.get(),
            sequence
        );
        let hit = parent_hit::verify(
            parent.client(),
            parent_hit::ParentHitRoute {
                child_frame_id: frame_id,
                execution_context_id: parent.utility_execution_context_id(),
                unique_context_id: parent.utility_unique_id(),
            },
            x,
            y,
            group,
            &self.handles.cleanup,
            self.handles.parent_hit_slots.clone(),
        )
        .await;
        if hit
            .as_ref()
            .err()
            .is_some_and(|error| error.kind() == FailureKind::Cleanup)
        {
            return hit;
        }
        self.check_current()?;
        hit
    }

    fn check_current(&self) -> Result<(), OperationError<LocatorErrorDetails>> {
        self.handles
            .owners
            .check_arguments([self.handle.as_ref()])
            .map_err(owner_error)?;
        self.handles
            .validate_retained_element_source(self.source.target(), &self.source)
            .map_err(source_error)?;
        if self.snapshot.page() != self.source.owner_page()
            || self.snapshot.frame() != self.source.owner_frame()
            || self.frame_route.frame() != self.source.owner_frame()
            || self.snapshot.main_execution_context_id()
                != self.source.creation_execution_context_id()
            || self.snapshot.main_unique_id() != self.source.creation_unique_context_id()
            || !Arc::ptr_eq(&self.snapshot.client(), &self.source.client())
            || !self.snapshot.is_current().map_err(route_error)?
        {
            return Err(OperationError::new(
                FailureKind::TargetClosed,
                "retained ElementHandle click creation world was replaced",
                None,
            ));
        }
        if let (ClickFrameRoute::OneLinkChild { parent, .. }, Some(parent_snapshot)) =
            (&self.frame_route, &self.parent_snapshot)
        {
            if parent_snapshot.frame() != *parent
                || parent_snapshot.page() != self.source.owner_page()
                || !Arc::ptr_eq(&parent_snapshot.client(), &self.source.client())
                || !parent_snapshot.is_current().map_err(route_error)?
            {
                return Err(OperationError::new(
                    FailureKind::TargetClosed,
                    "retained ElementHandle click parent Frame world was replaced",
                    None,
                ));
            }
        } else if matches!(&self.frame_route, ClickFrameRoute::OneLinkChild { .. }) {
            return Err(closed_route());
        }
        Ok(())
    }
}

type Outcome = Result<(), OperationError<LocatorErrorDetails>>;
type ResultFuture<'a> = Pin<Box<dyn Future<Output = Outcome> + Send + 'a>>;

fn route_error(error: impl std::fmt::Display) -> OperationError<LocatorErrorDetails> {
    OperationError::new(FailureKind::Backend, error.to_string(), None)
}

fn source_error(
    error: OperationError<HandleEvaluationErrorDetails>,
) -> OperationError<LocatorErrorDetails> {
    OperationError::new(error.kind(), error.to_string(), None)
}

fn owner_error(error: HostError) -> OperationError<LocatorErrorDetails> {
    let kind = match error {
        HostError::TargetClosed | HostError::FrameDetached => FailureKind::TargetClosed,
        _ => FailureKind::Backend,
    };
    OperationError::new(kind, error.to_string(), None)
}

fn closed_route() -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::TargetClosed,
        "retained ElementHandle click route is unavailable",
        None,
    )
}

fn map_retained_click_error(
    error: OperationError<LocatorErrorDetails>,
    timeout_ms: u32,
) -> HostError {
    if error.kind() == FailureKind::Timeout
        && error.message() == RETAINED_CLICK_TIMEOUT_MARKER
        && let Some(LocatorErrorDetails::Backend(progress)) = error.details()
    {
        return HostError::RetainedClickTimeout {
            timeout_ms,
            progress: progress.clone(),
        };
    }
    if error.kind() == FailureKind::Backend
        && error.message() == "Element is not attached to the DOM"
        && matches!(
            error.details(),
            Some(LocatorErrorDetails::InvalidTarget(status)) if status == "detached"
        )
    {
        return HostError::Backend(error.message().to_owned());
    }
    HostError::operation(error, timeout_ms)
}

impl TargetedService<Params> for Invocation {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a> = ResultFuture<'a>;

    fn call_for(&self, target: ObjectId, params: Params) -> Self::Future<'_> {
        Box::pin(async move {
            let Params {
                handle,
                admission,
                frame_route,
                options,
            } = params;
            self.handles
                .owners
                .check_arguments([handle.as_ref()])
                .map_err(owner_error)?;
            let source = self
                .handles
                .retained_element_source(target)
                .map_err(source_error)?;
            let snapshot = self
                .handles
                .retained_element_world_snapshot(source.owner_frame())
                .await
                .map_err(owner_error)?;
            let parent_snapshot = if let ClickFrameRoute::OneLinkChild { parent, .. } = &frame_route
            {
                Some(
                    self.handles
                        .retained_element_world_snapshot(*parent)
                        .await
                        .map_err(owner_error)?,
                )
            } else {
                None
            };
            let owner = Arc::new(ClickOwner {
                call_id: self.call_id,
                next_parent_group: AtomicU64::new(1),
                handles: self.handles.clone(),
                snapshot,
                parent_snapshot,
                frame_route,
                source,
                handle,
                _admission: admission,
            });
            owner.check_current()?;
            let route = self
                .services
                .locator_keyboard_route(owner.source.owner_frame())
                .await
                .map_err(route_error)?
                .ok_or_else(closed_route)?;
            // The locator service owns the Page input coordinator. A child
            // Frame or OOPIF must never inherit top-level CSS coordinates.
            if route.page_id() != owner.source.owner_page()
                || !Arc::ptr_eq(route.keyboard_client(), &owner.source.client())
            {
                return Err(OperationError::new(
                    FailureKind::TargetClosed,
                    "retained ElementHandle click requires its original top-level Page route",
                    None,
                ));
            }
            owner.check_current()?;
            let press = route.service().new_retained_click_press()?;
            self.route.set(route).map_err(|_| {
                route_error("retained ElementHandle click route was selected twice")
            })?;
            self.owner.set(owner.clone()).map_err(|_| {
                route_error("retained ElementHandle click owner was selected twice")
            })?;
            self.press.set(press.clone()).map_err(|_| {
                route_error("retained ElementHandle click press was selected twice")
            })?;
            let validation_owner = owner.clone();
            let service = self
                .route
                .get()
                .expect("published exact click route")
                .service();
            match &owner.frame_route {
                ClickFrameRoute::TopLevel { .. } => {
                    service
                        .click_retained_top_level_element_with_diagnostics(
                            owner.source.object_id(),
                            owner.source.creation_unique_context_id(),
                            options,
                            &press,
                            &self.diagnostics,
                            move || validation_owner.check_current(),
                        )
                        .await
                }
                ClickFrameRoute::OneLinkChild {
                    frame_id,
                    parent_frame_id,
                    ..
                } => {
                    let hit_owner = owner.clone();
                    service
                        .click_retained_one_link_child_element_with_diagnostics(
                            RetainedClickChildTarget::new(
                                owner.source.object_id(),
                                owner.source.creation_unique_context_id(),
                                frame_id,
                                parent_frame_id,
                            ),
                            options,
                            &press,
                            &self.diagnostics,
                            move |x, y| {
                                let hit_owner = hit_owner.clone();
                                async move { hit_owner.check_parent_hit(x, y).await }
                            },
                            move || validation_owner.check_current(),
                        )
                        .await
                }
            }
        })
    }
}

impl CommandCancellation<RetainedElementClick> for Invocation {
    type Future<'a> = ResultFuture<'a>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async move {
            let _owner = self.owner.get().cloned();
            match (self.route.get(), self.press.get()) {
                (Some(route), Some(press)) => route
                    .service()
                    .cancel_retained_top_level_click(press)
                    .await
                    .map_err(|error| error.reclassify(FailureKind::Cleanup)),
                (None, None) => Ok(()),
                _ => Err(route_error(
                    "retained ElementHandle click cancel owner is incomplete",
                )),
            }
        })
    }

    fn terminal_error_after_cancel(
        &self,
        _context: OperationContext,
        reason: CancellationReason,
        _timeout: Option<Duration>,
    ) -> Option<OperationError<LocatorErrorDetails>> {
        if reason != CancellationReason::DeadlineExceeded {
            return None;
        }
        self.diagnostics.timeout_log().map(|progress| {
            OperationError::new(
                FailureKind::Timeout,
                RETAINED_CLICK_TIMEOUT_MARKER,
                Some(LocatorErrorDetails::Backend(progress)),
            )
        })
    }
}

impl HostSession {
    /// The caller retains the exact portable Handle; native routing is
    /// reacquired only by that identity, never by a selector.
    pub async fn element_handle_click(
        &self,
        handle: Arc<PendingHandle>,
        options: ClickOptionsV2,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        if !handle.is_element() {
            return Err(HostError::InvalidArgument(
                "click requires an ElementHandle".into(),
            ));
        }
        #[cfg(feature = "page-bindings")]
        let frame = handle.initializer().parent_id.get().to_string();
        let (target, call_id, _operation) =
            self.admit_operation(&handle.initializer().object_id.get().to_string())?;
        let (runtime, handles, frame_route) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let topology = topology.topology();
            let frame = ObjectId::new(handle.initializer().parent_id.get());
            let page = topology
                .page_object_id_for_frame(frame)
                .ok_or(HostError::FrameDetached)?;
            let frame_id = topology
                .frame_native_id(frame)
                .ok_or(HostError::FrameDetached)?;
            let frame_route = if let Some(parent) = topology.parent_frame_object_id(frame) {
                if topology.parent_frame_object_id(parent).is_some()
                    || topology.page_object_id_for_frame(parent) != Some(page)
                    || topology.frame_session_id(frame).is_none()
                    || topology.frame_session_id(frame) != topology.frame_session_id(parent)
                {
                    return Err(HostError::UnsupportedOperation(
                        "retained ElementHandle click supports only one same-session child Frame"
                            .into(),
                    ));
                }
                let parent_frame_id = topology
                    .frame_native_id(parent)
                    .ok_or(HostError::FrameDetached)?;
                ClickFrameRoute::OneLinkChild {
                    frame,
                    parent,
                    frame_id: frame_id.to_owned(),
                    parent_frame_id: parent_frame_id.to_owned(),
                }
            } else {
                if topology.main_frame_native_id(page) != Some(frame_id) {
                    return Err(HostError::FrameDetached);
                }
                ClickFrameRoute::TopLevel { frame }
            };
            (live.runtime.clone(), live.handles.clone(), frame_route)
        };
        let admission = handles.owners.admit_tracked_read(&handle)?;
        let invocation = Arc::new(Invocation {
            call_id,
            handles,
            services: runtime.service().clone(),
            route: OnceLock::new(),
            owner: OnceLock::new(),
            press: OnceLock::new(),
            diagnostics: RetainedClickDiagnostics::default(),
        });
        let timeout = (timeout_ms != 0).then(|| Duration::from_millis(u64::from(timeout_ms)));
        let command = Command::<RetainedElementClick>::new(
            call_id,
            target,
            CallMetadata::new(timeout, None),
            Params {
                handle,
                admission,
                frame_route,
                options,
            },
        );
        let dispatch = runtime.dispatch_with(command, invocation.as_ref(), invocation.clone());
        let result = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            result = dispatch => result.unwrap_or_else(|never| match never {}).into_outcome(),
        };
        let result = result.map_err(|error| map_retained_click_error(error, timeout_ms));
        let result = crate::completion::publish_command_result(
            result,
            self.is_connected(),
            self.flush_topology(),
        )
        .await;
        #[cfg(feature = "page-bindings")]
        if result.is_ok() {
            self.flush_page_binding_publications(&frame).await?;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retained_click_command_accepts_only_element_handles() {
        // Arrange
        let expected = ELEMENT_HANDLE_OBJECT_KIND_ID;

        // Act
        let kinds = RetainedElementClick::TARGET_OBJECT_KINDS;

        // Assert
        assert_eq!(kinds, &[expected]);
    }

    #[test]
    fn retained_click_source_failure_preserves_target_closed_kind() {
        // Arrange
        let error: OperationError<HandleEvaluationErrorDetails> =
            OperationError::new(FailureKind::TargetClosed, "creation world closed", None);

        // Act
        let mapped = source_error(error);

        // Assert
        assert_eq!(mapped.kind(), FailureKind::TargetClosed);
    }

    #[test]
    fn detached_element_action_preserves_the_exact_public_message() {
        // Arrange
        let failure = OperationError::new(
            FailureKind::Backend,
            "Element is not attached to the DOM",
            Some(LocatorErrorDetails::InvalidTarget("detached".into())),
        );

        // Act
        let mapped = map_retained_click_error(failure, 5000);

        // Assert
        assert_eq!(mapped.code(), "PWRIGHT_BACKEND");
        assert_eq!(mapped.to_string(), "Element is not attached to the DOM");
    }

    #[test]
    fn retired_frame_action_does_not_become_dom_detachment() {
        // Arrange
        let failure = OperationError::new(
            FailureKind::TargetClosed,
            "retained Element moved to another Frame",
            Some(LocatorErrorDetails::InvalidTarget("frame-mismatch".into())),
        );

        // Act
        let mapped = map_retained_click_error(failure, 5000);

        // Assert
        assert_eq!(mapped.code(), "PWRIGHT_TARGET_CLOSED");
    }

    #[test]
    fn observed_retained_deadline_preserves_python_timeout_class_and_bounded_log() {
        // Arrange
        let progress =
            "  - <div id=\"blocker\"></div> intercepts pointer events\n  - retrying click action";
        let failure = OperationError::new(
            FailureKind::Timeout,
            RETAINED_CLICK_TIMEOUT_MARKER,
            Some(LocatorErrorDetails::Backend(progress.into())),
        );

        // Act
        let mapped = map_retained_click_error(failure, 5000);

        // Assert
        assert_eq!(mapped.code(), "PWRIGHT_TIMEOUT");
        assert!(mapped.to_string().contains("Timeout 5000ms exceeded."));
        assert!(
            mapped
                .to_string()
                .contains("waiting for element to be visible, enabled and stable")
        );
        assert!(mapped.to_string().contains(progress));
    }

    #[test]
    fn unobserved_or_cleanup_failure_keeps_existing_classification() {
        // Arrange
        let unobserved: OperationError<LocatorErrorDetails> =
            OperationError::new(FailureKind::Timeout, "deadline exceeded", None);
        let cleanup: OperationError<LocatorErrorDetails> = OperationError::new(
            FailureKind::Cleanup,
            "owned press release failed",
            Some(LocatorErrorDetails::Backend("cleanup error".into())),
        );

        // Act
        let timeout_mapped = map_retained_click_error(unobserved, 3000);
        let cleanup_mapped = map_retained_click_error(cleanup, 3000);

        // Assert
        assert_eq!(timeout_mapped.code(), "PWRIGHT_TIMEOUT");
        assert_eq!(timeout_mapped.to_string(), "Timeout 3000ms exceeded");
        assert_eq!(cleanup_mapped.code(), "PWRIGHT_BACKEND");
        assert!(!cleanup_mapped.to_string().contains("retrying click action"));
    }
}
