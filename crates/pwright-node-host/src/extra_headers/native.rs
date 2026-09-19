use super::{DispatchPlan, ID_BYTES, Installation, MAX_APPLIED};
use crate::{HostError, context_defaults::OwnedContextDefaults};
use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_composition_chromium::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};
use pwright_network_model::extra_headers::EffectiveHeaders;
use serde::{Serialize, Serializer, ser::SerializeMap};
use std::sync::Arc;

// One effective owner is shared by all sessions of a Page. Later Page groups
// materialize this once when their first native future is polled.
struct PageSnapshot(std::sync::Mutex<Option<EffectiveHeaders>>);
impl PageSnapshot {
    fn resolve(
        &self,
        installation: &Installation,
        route: &super::Route,
    ) -> Result<EffectiveHeaders, HostError> {
        let mut snapshot = self.0.lock().map_err(HostError::backend)?;
        if snapshot.is_none() {
            *snapshot = Some(
                installation
                    .settings
                    .lock()
                    .map_err(HostError::backend)?
                    .effective(route.context, route.page)
                    .map_err(super::settings_error)?,
            );
        }
        snapshot
            .as_ref()
            .cloned()
            .ok_or_else(|| HostError::Backend("header Page snapshot was not initialized".into()))
    }
}

pub(super) fn plan_scratch_bytes() -> usize {
    // Two fanouts, each with one snapshot cell/Arc allocation per Page and one
    // group-size queue entry per route. Payloads retain separate pool leases.
    2 * (super::MAX_FAILED_PAGES * (std::mem::size_of::<PageSnapshot>() + 32)
        + super::MAX_ROUTES * std::mem::size_of::<usize>())
}
#[derive(Serialize)]
struct Parameters<'a> {
    headers: Headers<'a>,
}
struct Headers<'a>(&'a EffectiveHeaders);
impl Serialize for Headers<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.iter().len()))?;
        for header in self.0.iter() {
            map.serialize_entry(header.name, header.value)?;
        }
        map.end()
    }
}
impl Installation {
    fn remember(&self, session: &str) -> Result<(), HostError> {
        let mut applied = self.applied.lock().map_err(HostError::backend)?;
        if applied.iter().any(|entry| entry == session) {
            return Ok(());
        }
        if applied.len() == MAX_APPLIED || session.len() > ID_BYTES {
            return Err(HostError::ResourceExhausted(
                "applied header session capacity exceeded".into(),
            ));
        }
        applied.push(session.to_owned());
        Ok(())
    }
    fn forget(&self, session: &str) -> Result<(), HostError> {
        self.applied
            .lock()
            .map_err(HostError::backend)?
            .retain(|entry| entry != session);
        Ok(())
    }
    pub(super) async fn clear_obsolete(&self, session: &str) -> Result<(), HostError> {
        let _permit = self.native.acquire().await.map_err(HostError::backend)?;
        #[derive(Serialize)]
        struct Empty {
            headers: serde_json::Map<String, serde_json::Value>,
        }
        let result = self
            .connection
            .send_command_with_session(
                "Network.setExtraHTTPHeaders",
                serde_json::to_value(Empty {
                    headers: serde_json::Map::new(),
                })
                .map_err(HostError::backend)?,
                Some(session),
            )
            .await;
        match result {
            Ok(_) => {}
            Err(error) if session_gone(&error) => {}
            Err(error) => return Err(HostError::backend(error)),
        }
        self.forget(session)
    }
    async fn enable(&self, session: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let _permit = self.native.acquire().await.map_err(HostError::backend)?;
        self.connection
            .send_command_with_session(
                "Network.enable",
                serde_json::Value::Object(serde_json::Map::new()),
                Some(session),
            )
            // Initialization owns a typed error boundary. Preserve Chrome's
            // receipt for the topology driver's exact-session retirement check;
            // formatting as HostError here erases that evidence too early.
            .await?;
        Ok(())
    }
    async fn apply(
        &self,
        session: &str,
        headers: &EffectiveHeaders,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Validate only final effective values when a native session actually
        // consumes them, never while storing uninstalled desired settings.
        headers
            .validate_http_syntax()
            .map_err(super::syntax_error)?;
        let _permit = self.native.acquire().await.map_err(HostError::backend)?;
        self.remember(session)?;
        // Typed borrowed serialization: only the admitted JSON Map/string copy
        // exists, not a second temporary owned HashMap of headers.
        let params = serde_json::to_value(Parameters {
            headers: Headers(headers),
        })
        .map_err(HostError::backend)?;
        self.connection
            .send_command_with_session("Network.setExtraHTTPHeaders", params, Some(session))
            .await?;
        if headers.iter().len() == 0 {
            self.forget(session)?;
        }
        Ok(())
    }
    pub(super) fn fanout<'a>(
        &'a self,
        plan: &'a DispatchPlan,
        compensation: bool,
    ) -> Result<super::fanout::Fanout<'a>, HostError> {
        let mut fanout = super::fanout::Fanout::new();
        // Chromium Context updates await each Page, while each Page's network
        // manager uses Promise.all across its main/OOPIF sessions. Preserve both
        // levels for primary writes and compensation; failure skips later Pages.
        for (page, headers) in &plan.pages {
            let snapshot = Arc::new(PageSnapshot(std::sync::Mutex::new(headers.clone())));
            for route in plan.routes.iter().filter(|route| route.page == *page) {
                let snapshot = snapshot.clone();
                fanout.push(async move {
                    let headers = snapshot.resolve(self, route)?;
                    tokio::select! {
                        biased;
                        () = route.watch.wait(compensation) => {
                            self.forget(&route.watch.session)?;
                            if route.watch.is_main() { Err(HostError::TargetClosed) } else { Ok(()) }
                        }
                        result = self.apply(&route.watch.session, &headers) => match result {
                            Ok(()) => Ok(()),
                            Err(error) if matches!(error.downcast_ref::<pwright_cdp::connection::CdpError>(),
                                Some(pwright_cdp::connection::CdpError::SessionDetached)) => {
                                self.forget(&route.watch.session)?;
                                if route.watch.is_main() { Err(HostError::TargetClosed) } else { Ok(()) }
                            }
                            Err(error) => Err(header_apply_error(error)),
                        },
                    }
                })?;
            }
            fanout.end_group()?;
        }
        Ok(fanout)
    }
}

fn header_command_error(error: pwright_cdp::connection::CdpError) -> HostError {
    match error {
        pwright_cdp::connection::CdpError::Protocol { message, .. } => HostError::backend(
            format_args!("Protocol error (Network.setExtraHTTPHeaders): {message}"),
        ),
        error => HostError::backend(error),
    }
}

fn header_apply_error(error: Box<dyn std::error::Error + Send + Sync>) -> HostError {
    match error.downcast::<pwright_cdp::connection::CdpError>() {
        Ok(error) => header_command_error(*error),
        Err(error) => match error.downcast::<HostError>() {
            Ok(error) => *error,
            Err(error) => HostError::backend(error),
        },
    }
}

fn session_gone(error: &pwright_cdp::connection::CdpError) -> bool {
    if matches!(error, pwright_cdp::connection::CdpError::SessionDetached) {
        return true;
    }
    matches!(error, pwright_cdp::connection::CdpError::Protocol { code: -32001, message }
        if message == "Session with given id not found.")
}

pub(crate) struct Initializer {
    defaults: OwnedContextDefaults,
    installation: Arc<Installation>,
}
impl Initializer {
    pub(crate) fn new(defaults: OwnedContextDefaults, installation: Arc<Installation>) -> Self {
        Self {
            defaults,
            installation,
        }
    }
}
impl ChromiumSessionInitializer for Initializer {
    fn initialize<'a>(
        &'a self,
        _session: &'a str,
        _target: &'a str,
        _ownership: CdpContextOwnership,
        _page: bool,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async {
            Err(HostError::Backend("headers require scoped native identity".into()).into())
        })
    }
    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            self.defaults.initialize_scoped(target).await?;
            // Enable belongs to attachment, as in the pinned network manager.
            // Per-setter enable ACKs would reorder otherwise synchronous native
            // header dispatch when multiple setters overlap.
            self.installation.enable(target.session_id).await?;
            let headers = self
                .installation
                .settings
                .lock()
                .map_err(HostError::backend)?
                .effective(target.context, target.page)?;
            if let Err(error) = headers.validate_http_syntax() {
                // Expected per-Page header rejection must not seal the shared
                // topology/frame observer. The explicit creation transaction
                // closes its exact Page before rejecting; Context stays usable.
                self.installation
                    .failed_initializations
                    .lock()
                    .map_err(HostError::backend)?
                    .record(target.page, error)?;
                return Ok(());
            }
            // Fresh empty desired settings must not erase another attachment's
            // headers in a borrowed Page. Explicit setters still send empties.
            if headers.iter().len() != 0 {
                self.installation.apply(target.session_id, &headers).await?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
pub(super) fn encode(headers: &EffectiveHeaders) -> serde_json::Value {
    serde_json::to_value(Parameters {
        headers: Headers(headers),
    })
    .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_cdp::connection::CdpError;

    #[test]
    fn native_header_protocol_errors_match_the_public_method_and_keep_byte_bounds() {
        // Arrange
        let message = "界".repeat(4096);
        let errors = [
            CdpError::Protocol {
                code: -32000,
                message: "injected".into(),
            },
            CdpError::Protocol {
                code: -32000,
                message,
            },
            CdpError::Timeout,
        ];
        // Act
        let results = errors
            .map(header_command_error)
            .map(|error| error.to_string());
        // Assert
        assert_eq!(
            results[0],
            "Protocol error (Network.setExtraHTTPHeaders): injected"
        );
        assert!(results[1].len() <= 4096);
        assert!(results[1].ends_with('界'));
        assert_eq!(results[2], "Timeout waiting for response");
    }

    #[test]
    fn only_exact_missing_session_receipt_or_typed_detach_confirms_obsolete_session_retirement() {
        // Arrange
        let errors = [
            CdpError::SessionDetached,
            CdpError::Protocol {
                code: -32001,
                message: "Session with given id not found.".into(),
            },
            CdpError::Protocol {
                code: -32000,
                message: "Session with given id not found.".into(),
            },
            CdpError::Protocol {
                code: -32001,
                message: "other error".into(),
            },
            CdpError::Timeout,
            CdpError::ChannelDropped,
        ];
        // Act
        let confirmed = errors.each_ref().map(session_gone);
        // Assert
        assert_eq!(confirmed, [true, true, false, false, false, false]);
    }
}
