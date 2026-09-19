//! Playwright operations translated into typed v2 commands, never JS/CDP RPCs.

use std::convert::Infallible;
use std::sync::atomic::Ordering;
use std::time::Duration;

use pwright_cap_dom_content::{DomContentParams, FrameContent, FrameTitle, TitleParams};
use pwright_cap_navigation::{Goto, GotoParams, WaitUntil};
use pwright_core::{
    CallId, CallMetadata, Command, CommandResult, CommandSpec, FailureKind, ObjectId,
    OperationError, Service,
};

use crate::{CommandRuntime, HostError, HostSession};

#[derive(Clone, Debug)]
pub struct NavigationResult {
    pub url: String,
    pub response: Option<ResponseMetadata>,
}

#[derive(Clone, Debug)]
pub struct ResponseMetadata {
    pub url: String,
    pub status: i64,
    pub status_text: String,
    pub ok: bool,
    pub headers: Vec<(String, String)>,
}

impl HostError {
    pub(crate) fn operation<D>(error: OperationError<D>, timeout: u32) -> Self {
        match error.kind() {
            FailureKind::Timeout => Self::Timeout(timeout),
            FailureKind::TargetClosed | FailureKind::TransportClosed => Self::TargetClosed,
            FailureKind::ResourceExhausted => Self::ResourceExhausted(error.message().to_owned()),
            FailureKind::UnsupportedCapability => {
                Self::UnsupportedOperation(error.message().to_owned())
            }
            FailureKind::Cancelled
            | FailureKind::BrowserCrashed
            | FailureKind::ProtocolViolation
            | FailureKind::Backend
            | FailureKind::Cleanup => Self::backend(error),
        }
    }
}

impl HostSession {
    pub(crate) fn admit_operation(
        &self,
        id: &str,
    ) -> Result<
        (
            ObjectId,
            CallId,
            tokio_util::task::task_tracker::TaskTrackerToken,
        ),
        HostError,
    > {
        let object_id = id
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable object ID".into()))?;
        let state = self.state.lock().map_err(HostError::backend)?;
        if !self.is_connected() || state.is_none() {
            return Err(HostError::TargetClosed);
        }
        let call_id = self
            .next_call
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |id| id.checked_add(1))
            .map(CallId::new)
            .map_err(|_| HostError::ResourceExhausted("call ID space exhausted".into()))?;
        Ok((object_id, call_id, self.operations.token()))
    }

    pub(crate) async fn invoke<C>(
        &self,
        id: &str,
        params: C::Params,
        timeout_ms: u32,
    ) -> Result<C::Result, HostError>
    where
        C: CommandSpec,
        CommandRuntime: Service<Command<C>, Response = CommandResult<C>, Error = Infallible>,
    {
        self.invoke_with_error::<C>(id, params, timeout_ms, HostError::operation)
            .await
    }

    pub(crate) async fn invoke_with_error<C>(
        &self,
        id: &str,
        params: C::Params,
        timeout_ms: u32,
        map_error: fn(OperationError<C::ErrorDetails>, u32) -> HostError,
    ) -> Result<C::Result, HostError>
    where
        C: CommandSpec,
        CommandRuntime: Service<Command<C>, Response = CommandResult<C>, Error = Infallible>,
    {
        let (object_id, call_id, _operation) = self.admit_operation(id)?;
        let runtime = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            live.runtime.clone()
        };
        let timeout = (timeout_ms != 0).then(|| Duration::from_millis(u64::from(timeout_ms)));
        let command =
            Command::<C>::new(call_id, object_id, CallMetadata::new(timeout, None), params);
        // Environment shutdown must abandon the controlled command while CDP
        // is still alive. Its drop guard schedules backend cancellation, and
        // the host drains that cleanup before closing the native transport.
        let outcome = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            outcome = runtime.call(command) => outcome,
        };
        let result = match outcome {
            Ok(result) => result,
            Err(never) => match never {},
        };
        result
            .into_outcome()
            .map_err(|error| map_error(error, timeout_ms))
    }

    pub async fn frame_title(&self, frame_id: &str) -> Result<String, HostError> {
        let result = self.invoke::<FrameTitle>(frame_id, TitleParams, 0).await?;
        Ok(result.as_str().to_owned())
    }

    pub async fn frame_content(&self, frame_id: &str) -> Result<String, HostError> {
        let result = self
            .invoke::<FrameContent>(frame_id, DomContentParams, 0)
            .await?;
        Ok(result.as_str().to_owned())
    }

    pub async fn page_goto(
        &self,
        page_id: &str,
        url: String,
        wait_until: &str,
        referrer: Option<String>,
        timeout_ms: u32,
    ) -> Result<NavigationResult, HostError> {
        let wait_until = parse_wait_until(wait_until)?;
        #[cfg(feature = "extra-headers")]
        let mut header_referrer = self.navigation_referrer(page_id, referrer)?;
        #[cfg(feature = "extra-headers")]
        let referrer = header_referrer.value.take();
        self.page_goto_prepared(page_id, url, wait_until, referrer, timeout_ms)
            .await
    }

    /// Capture Page-only referer policy in invocation order, before a foreign
    /// executor can schedule this navigation behind a later header setter.
    #[cfg(feature = "extra-headers")]
    pub fn start_page_goto(
        self: &std::sync::Arc<Self>,
        page_id: String,
        url: String,
        wait_until: String,
        referrer: Option<String>,
        timeout_ms: u32,
    ) -> impl Future<Output = Result<NavigationResult, HostError>> + Send + 'static + use<> {
        let prepared = parse_wait_until(&wait_until).and_then(|wait| {
            self.navigation_referrer(&page_id, referrer)
                .map(|headers| (wait, headers))
        });
        let host = self.clone();
        async move {
            let (wait, mut headers) = prepared?;
            host.page_goto_prepared(&page_id, url, wait, headers.value.take(), timeout_ms)
                .await
        }
    }

    async fn page_goto_prepared(
        &self,
        page_id: &str,
        url: String,
        wait_until: WaitUntil,
        referrer: Option<String>,
        timeout_ms: u32,
    ) -> Result<NavigationResult, HostError> {
        let result = self
            .invoke::<Goto>(
                page_id,
                GotoParams {
                    url,
                    referrer,
                    wait_until,
                },
                timeout_ms,
            )
            .await;
        // The navigation driver and topology share one native stream but
        // distinct bounded cursors. Publish its already-queued URL events
        // before resolving goto; never infer state from the requested URL.
        let result = crate::completion::publish_command_result(
            result,
            self.is_connected(),
            self.flush_topology(),
        )
        .await?;
        Ok(NavigationResult {
            url: result.url,
            response: result.response.map(|response| ResponseMetadata {
                url: response.url,
                status: response.status,
                status_text: response.status_text,
                ok: (200..=299).contains(&response.status),
                headers: response
                    .headers
                    .into_iter()
                    .map(|header| (header.name.to_ascii_lowercase(), header.value))
                    .collect(),
            }),
        })
    }
}

fn parse_wait_until(value: &str) -> Result<WaitUntil, HostError> {
    match value {
        "commit" => Ok(WaitUntil::Commit),
        "domcontentloaded" => Ok(WaitUntil::DomContentLoaded),
        "load" => Ok(WaitUntil::Load),
        "networkidle" => Ok(WaitUntil::NetworkIdle),
        _ => Err(HostError::InvalidArgument(
            "waitUntil must be commit, domcontentloaded, load, or networkidle".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_timeout_preserves_the_public_timeout_budget() {
        // Arrange
        let failure = OperationError::<()>::new(FailureKind::Timeout, "native timeout", None);
        // Act
        let error = HostError::operation(failure, 123);
        // Assert
        assert_eq!(error.code(), "PWRIGHT_TIMEOUT");
        assert_eq!(error.to_string(), "Timeout 123ms exceeded");
    }

    #[test]
    fn transport_and_target_close_share_the_public_target_closed_error() {
        // Arrange
        let failures = [FailureKind::TransportClosed, FailureKind::TargetClosed];
        // Act
        let errors = failures.map(|kind| {
            HostError::operation(OperationError::<()>::new(kind, "closed", None), 5000)
        });
        // Assert
        assert!(
            errors
                .iter()
                .all(|error| error.code() == "PWRIGHT_TARGET_CLOSED")
        );
    }
}
