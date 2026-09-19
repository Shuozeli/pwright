use crate::{HostError, HostSession, NavigationResult, ResponseMetadata};
use pwright_cap_navigation::{Reload, ReloadParams, WaitUntil};

#[cfg(test)]
mod boundary_probe;

impl HostSession {
    pub async fn page_reload(
        &self,
        page_id: &str,
        wait_until: &str,
        timeout_ms: u32,
    ) -> Result<NavigationResult, HostError> {
        let wait_until = match wait_until {
            "commit" => WaitUntil::Commit,
            "domcontentloaded" => WaitUntil::DomContentLoaded,
            "load" => WaitUntil::Load,
            "networkidle" => WaitUntil::NetworkIdle,
            _ => {
                return Err(HostError::InvalidArgument(
                    "waitUntil must be commit, domcontentloaded, load, or networkidle".into(),
                ));
            }
        };
        let result = self
            .invoke::<Reload>(page_id, ReloadParams { wait_until }, timeout_ms)
            .await;
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
