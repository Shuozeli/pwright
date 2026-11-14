//! Optional scroll-into-view action against one retained native ElementHandle.
//!
//! Neither the stable check nor the native CDP commit resolves a selector.

use std::{sync::Arc, time::Duration};

use pwright_cdp::connection::CdpError;
use pwright_composition_chromium::ChromiumFrameWorldSnapshot;
use pwright_composition_chromium_handle_evaluation::ChromiumRetainedElementSource;

use crate::{
    HostError, HostSession,
    evaluation::EvaluationValue,
    handles::{Installation, OwnedHandleArgument, PendingHandle},
};

// Only stable state is required before the native Chromium scroll command.
// Do not gate on getClientRects: display:contents has no own box but Chrome
// can scroll its contents via DOM.scrollIntoViewIfNeeded(objectId).
const CHECK_SOURCE: &str = r#"async (element) => {
  if (!element || !element.isConnected) return 'detached';
  if (element.nodeType !== 1) return 'invalid-target';
  const before = element.getBoundingClientRect();
  await new Promise(resolve => setTimeout(resolve, document.visibilityState === 'visible' ? 32 : 0));
  if (!element.isConnected) return 'detached';
  const after = element.getBoundingClientRect();
  const stable = Math.abs(before.x - after.x) <= 0.25 &&
    Math.abs(before.y - after.y) <= 0.25 &&
    Math.abs(before.width - after.width) <= 0.25 &&
    Math.abs(before.height - after.height) <= 0.25;
  return stable ? 'stable' : 'unstable';
}"#;

const RETRY_DELAY: Duration = Duration::from_millis(50);
const NOT_VISIBLE_PROGRESS: &str = "retrying scroll into view action\n  element is not visible";
const NOT_STABLE_PROGRESS: &str = "retrying scroll into view action\n  element is not stable";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StableAttempt {
    Stable,
    Unstable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum NativeScrollAttempt {
    Done,
    NoLayout,
}

fn parse_stable_attempt(value: EvaluationValue) -> Result<StableAttempt, HostError> {
    match value {
        EvaluationValue::String(status) => match status.as_str() {
            "stable" => Ok(StableAttempt::Stable),
            "unstable" => Ok(StableAttempt::Unstable),
            "detached" => Err(HostError::Backend(
                "Element is not attached to the DOM".into(),
            )),
            "invalid-target" => Err(HostError::InvalidArgument("Node is not an Element".into())),
            _ => Err(HostError::Backend(
                "retained ElementHandle stability check returned an unknown status".into(),
            )),
        },
        _ => Err(HostError::Backend(
            "retained ElementHandle stability check returned a non-string status".into(),
        )),
    }
}

fn native_scroll_attempt(
    result: Result<serde_json::Value, CdpError>,
) -> Result<NativeScrollAttempt, HostError> {
    match result {
        Ok(_) => Ok(NativeScrollAttempt::Done),
        Err(CdpError::Protocol { message, .. })
            if message.contains("Node does not have a layout object") =>
        {
            Ok(NativeScrollAttempt::NoLayout)
        }
        Err(CdpError::Protocol { message, .. })
            if message.contains("Node is detached from document") =>
        {
            Err(HostError::Backend(
                "Element is not attached to the DOM".into(),
            ))
        }
        Err(CdpError::Closed | CdpError::SessionDetached | CdpError::PageClosed) => {
            Err(HostError::TargetClosed)
        }
        Err(error) => Err(HostError::backend(error)),
    }
}

fn timeout_error(timeout_ms: u32, progress: &'static str) -> HostError {
    HostError::ActionTimeout {
        api: "ElementHandle.scrollIntoViewIfNeeded",
        timeout_ms,
        locator: "element".into(),
        progress,
    }
}

fn remaining(
    deadline: Option<tokio::time::Instant>,
    timeout_ms: u32,
    progress: &'static str,
) -> Result<Option<Duration>, HostError> {
    deadline
        .map(|deadline| {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                Err(timeout_error(timeout_ms, progress))
            } else {
                Ok(remaining)
            }
        })
        .transpose()
}

fn matches_creation_world(
    source_id: i64,
    source_unique: &str,
    main_id: i64,
    main_unique: &str,
    utility_id: i64,
    utility_unique: &str,
) -> bool {
    (source_id == main_id && source_unique == main_unique)
        || (source_id == utility_id && source_unique == utility_unique)
}

fn check_current(
    handles: &Installation,
    target: &PendingHandle,
    source: &ChromiumRetainedElementSource,
    world: &ChromiumFrameWorldSnapshot,
) -> Result<(), HostError> {
    handles.owners.check_arguments([target])?;
    handles
        .validate_retained_element_source(source.target(), source)
        .map_err(|error| HostError::operation(error, 0))?;
    // Creation normally follows the frame's main-world Handle route, but an
    // exact retained ElementHandle may originate in the utility world too.
    // Require one complete numeric/unique pair from this same frame snapshot.
    let source_world_matches = matches_creation_world(
        source.creation_execution_context_id(),
        source.creation_unique_context_id(),
        world.main_execution_context_id(),
        world.main_unique_id(),
        world.utility_execution_context_id(),
        world.utility_unique_id(),
    );
    if source.target() != target.initializer().object_id
        || target.source_world() != Some(&source.world_identity())
        || world.frame() != source.owner_frame()
        || world.page() != source.owner_page()
        || !source_world_matches
        || !Arc::ptr_eq(&world.client(), &source.client())
        || !world.is_current().map_err(HostError::backend)?
    {
        return Err(HostError::TargetClosed);
    }
    Ok(())
}

impl HostSession {
    /// Scroll one original ElementHandle into view, waiting for display/layout
    /// to become usable but never switching to a replacement DOM node.
    pub async fn element_handle_scroll_into_view_if_needed(
        &self,
        target: &PendingHandle,
        timeout_ms: impl Into<Option<u32>>,
    ) -> Result<(), HostError> {
        if !target.is_element() {
            return Err(HostError::InvalidArgument(
                "scroll_into_view_if_needed requires an ElementHandle".into(),
            ));
        }
        if target.source_world().is_none() {
            return Err(HostError::backend(
                "Handle creation-world identity is missing",
            ));
        }
        let (_, _, _operation) =
            self.admit_operation(&target.initializer().object_id.get().to_string())?;
        let handles = {
            let state = self.state.lock().map_err(HostError::backend)?;
            state
                .as_ref()
                .ok_or(HostError::TargetClosed)?
                .handles
                .clone()
        };
        // Four whole-action slots leave four of Handle's eight call slots
        // available to their nested exact-handle stability evaluations.
        let _action = handles
            .element_action_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| {
                HostError::ResourceExhausted("ElementHandle scroll capacity exceeded".into())
            })?;
        let timeout_ms = match timeout_ms.into() {
            Some(timeout_ms) => timeout_ms,
            #[cfg(feature = "timeout-settings")]
            None => self.inherited_scroll_timeout(target, &handles).await?,
            #[cfg(not(feature = "timeout-settings"))]
            None => {
                return Err(HostError::UnsupportedOperation(
                    "ElementHandle scroll default-timeout inheritance requires timeout-settings"
                        .into(),
                ));
            }
        };
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));
        let mut progress = NOT_STABLE_PROGRESS;
        loop {
            let budget = remaining(deadline, timeout_ms, progress)?;
            let evaluation = self.handle_evaluate_owned(
                target,
                CHECK_SOURCE.into(),
                OwnedHandleArgument::from_slots(EvaluationValue::Undefined, Vec::new())?,
            );
            let observation = match budget {
                Some(budget) => tokio::time::timeout(budget, evaluation)
                    .await
                    .map_err(|_| timeout_error(timeout_ms, progress))??,
                None => evaluation.await?,
            };
            match parse_stable_attempt(observation)? {
                StableAttempt::Stable => {
                    let budget = remaining(deadline, timeout_ms, progress)?;
                    let commit = self.scroll_retained_native(target, &handles);
                    let committed = match budget {
                        Some(budget) => tokio::time::timeout(budget, commit)
                            .await
                            .map_err(|_| timeout_error(timeout_ms, progress))??,
                        None => commit.await?,
                    };
                    match committed {
                        NativeScrollAttempt::Done => return Ok(()),
                        NativeScrollAttempt::NoLayout => progress = NOT_VISIBLE_PROGRESS,
                    }
                }
                StableAttempt::Unstable => {
                    progress = NOT_STABLE_PROGRESS;
                }
            }
            let delay = remaining(deadline, timeout_ms, progress)?
                .map_or(RETRY_DELAY, |remaining| RETRY_DELAY.min(remaining));
            tokio::time::sleep(delay).await;
        }
    }

    #[cfg(feature = "timeout-settings")]
    async fn inherited_scroll_timeout(
        &self,
        target: &PendingHandle,
        handles: &Arc<Installation>,
    ) -> Result<u32, HostError> {
        // Resolve the current live Rust owner route, not the JS wrapper's
        // creation Page or a selector query. The retained source and world
        // checks are the same exact-Handle proof used at native commit.
        let _admission = handles.owners.admit_tracked_read(target)?;
        let source = handles
            .retained_element_source(target.initializer().object_id)
            .map_err(|error| HostError::operation(error, 0))?;
        let world = handles
            .retained_element_world_snapshot(source.owner_frame())
            .await?;
        check_current(handles, target, &source, &world)?;
        self.page_default_timeout(&source.owner_page().get().to_string(), false)
    }

    async fn scroll_retained_native(
        &self,
        target: &PendingHandle,
        handles: &Arc<Installation>,
    ) -> Result<NativeScrollAttempt, HostError> {
        // This admission tracks one exact native use through its response and
        // keeps source/world descriptors inside the existing Handle budgets.
        let _admission = handles.owners.admit_tracked_read(target)?;
        let source = handles
            .retained_element_source(target.initializer().object_id)
            .map_err(|error| HostError::operation(error, 0))?;
        let world = handles
            .retained_element_world_snapshot(source.owner_frame())
            .await?;
        check_current(handles, target, &source, &world)?;
        let native = source.client().retained_native_session().ok_or_else(|| {
            HostError::UnsupportedOperation(
                "retained ElementHandle scroll requires a native CDP session".into(),
            )
        })?;
        let params = pwright_cdp::generated::dom::ScrollIntoViewIfNeededParams {
            object_id: Some(source.object_id().to_owned()),
            ..Default::default()
        };
        let params = serde_json::to_value(params).map_err(HostError::backend)?;
        // This send is the commit point. Dropping the waiter after dispatch
        // cannot retarget or replay the exact objectId on another node.
        let result = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            result = native.send("DOM.scrollIntoViewIfNeeded", params) => result,
        };
        let attempt = native_scroll_attempt(result)?;
        check_current(handles, target, &source, &world)?;
        Ok(attempt)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CHECK_SOURCE, NOT_VISIBLE_PROGRESS, NativeScrollAttempt, StableAttempt,
        matches_creation_world, native_scroll_attempt, parse_stable_attempt, remaining,
        timeout_error,
    };
    use crate::{HostError, evaluation::EvaluationValue};
    use pwright_cdp::connection::CdpError;
    use std::time::Duration;

    #[test]
    fn detached_target_is_terminal() {
        // Arrange
        let detached = EvaluationValue::String("detached".into());

        // Act
        let detached = parse_stable_attempt(detached);

        // Assert
        assert!(
            matches!(detached, Err(HostError::Backend(message)) if message == "Element is not attached to the DOM")
        );
    }

    #[test]
    fn unknown_status_fails_closed() {
        // Arrange
        let unknown = EvaluationValue::String("replacement".into());

        // Act
        let unknown = parse_stable_attempt(unknown);

        // Assert
        assert!(
            matches!(unknown, Err(HostError::Backend(message)) if message.contains("unknown status"))
        );
    }

    #[test]
    fn no_layout_is_a_retryable_result() {
        // Arrange
        let no_layout = Err(CdpError::Protocol {
            code: -32000,
            message: "Node does not have a layout object".into(),
        });

        // Act
        let no_layout = native_scroll_attempt(no_layout);

        // Assert
        assert_eq!(no_layout.unwrap(), NativeScrollAttempt::NoLayout);
    }

    #[test]
    fn native_detachment_is_terminal() {
        // Arrange
        let detached = Err(CdpError::Protocol {
            code: -32000,
            message: "Node is detached from document".into(),
        });

        // Act
        let detached = native_scroll_attempt(detached);

        // Assert
        assert!(
            matches!(detached, Err(HostError::Backend(message)) if message == "Element is not attached to the DOM")
        );
    }

    #[test]
    fn creation_world_accepts_only_a_complete_main_or_utility_identity() {
        // Arrange
        let main = (101, "main-a");
        let utility = (202, "utility-b");

        // Act
        let main_matches =
            matches_creation_world(main.0, main.1, main.0, main.1, utility.0, utility.1);
        let utility_matches =
            matches_creation_world(utility.0, utility.1, main.0, main.1, utility.0, utility.1);
        let mixed_world_is_rejected =
            matches_creation_world(main.0, utility.1, main.0, main.1, utility.0, utility.1);
        let reused_numeric_id_is_rejected =
            matches_creation_world(main.0, "old-main", main.0, main.1, utility.0, utility.1);

        // Assert
        assert!(main_matches);
        assert!(utility_matches);
        assert!(!mixed_world_is_rejected);
        assert!(!reused_numeric_id_is_rejected);
    }

    #[test]
    fn display_contents_hidden_and_zero_size_reach_native_layout_decision() {
        // Arrange: only Chromium's DOM command decides whether the original
        // node has a scrollable layout, including display:contents descendants.
        let check = CHECK_SOURCE;

        // Act
        let no_layout_filter = check.contains("getClientRects")
            || check.contains("style.visibility")
            || check.contains("rect.width <= 0")
            || check.contains("rect.height <= 0");

        // Assert
        assert!(!no_layout_filter);
    }

    #[test]
    fn stable_check_uses_exact_element_without_selector_lookup() {
        // Arrange
        let check = CHECK_SOURCE;

        // Act
        let read_only_check = !check.contains("scrollIntoView");

        // Assert
        assert!(check.starts_with("async (element) =>"));
        assert!(read_only_check);
        assert!(!check.contains("querySelector"));
        assert!(!check.contains("Node."));
        assert!(check.contains("if (!element || !element.isConnected)"));
    }

    #[test]
    fn moving_element_retries_and_browser_side_check_has_no_unbounded_observer() {
        // Arrange
        let check = CHECK_SOURCE;

        // Act
        let stable = parse_stable_attempt(EvaluationValue::String("unstable".into()));

        // Assert
        assert_eq!(stable.unwrap(), StableAttempt::Unstable);
        assert!(check.contains("setTimeout(resolve"));
        assert!(!check.contains("IntersectionObserver"));
        assert!(!check.contains("requestAnimationFrame"));
    }

    #[test]
    fn native_commit_uses_the_original_remote_object_id() {
        // Arrange
        let production = include_str!("element_handle_scroll.rs")
            .split_once("#[cfg(test)]")
            .expect("the module has a test section")
            .0;

        // Act
        let exact_source = production.contains("object_id: Some(source.object_id().to_owned())");

        // Assert
        assert!(exact_source);
        assert!(production.contains("native.send(\"DOM.scrollIntoViewIfNeeded\", params)"));
        assert!(!production.contains("selector_query"));
    }

    #[test]
    fn timeout_reports_the_official_scroll_progress_and_timeout_code() {
        // Arrange
        let progress = NOT_VISIBLE_PROGRESS;

        // Act
        let error = timeout_error(3000, progress);

        // Assert
        assert_eq!(error.code(), "PWRIGHT_TIMEOUT");
        assert!(
            error
                .to_string()
                .contains("retrying scroll into view action")
        );
        assert!(error.to_string().contains("element is not visible"));
    }

    #[test]
    fn expired_deadline_prevents_a_new_browser_evaluation() {
        // Arrange
        let deadline = Some(tokio::time::Instant::now() - Duration::from_millis(1));

        // Act
        let budget = remaining(deadline, 3000, NOT_VISIBLE_PROGRESS);

        // Assert
        assert!(matches!(
            budget,
            Err(HostError::ActionTimeout {
                timeout_ms: 3000,
                ..
            })
        ));
    }
}
