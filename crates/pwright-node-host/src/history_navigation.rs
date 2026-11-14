use crate::{HostError, HostSession, NavigationResult, ResponseMetadata};
use pwright_cap_history_navigation::{GoBack, GoBackParams, GoForward, GoForwardParams, WaitUntil};

impl HostSession {
    pub async fn page_go_back(
        &self,
        page_id: &str,
        wait_until: &str,
        timeout_ms: u32,
    ) -> Result<Option<NavigationResult>, HostError> {
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
            .invoke::<GoBack>(page_id, GoBackParams { wait_until }, timeout_ms)
            .await;
        let result = crate::completion::publish_command_result(
            result,
            self.is_connected(),
            self.flush_topology(),
        )
        .await?;
        let Some(result) = result.navigation else {
            return Ok(None);
        };
        let request_id = result.loader_id;
        #[cfg(feature = "response-request")]
        let request = if result.response.is_some() {
            match request_id.as_deref() {
                Some(request_id) => Some(self.request_by_native_id(page_id, request_id).await?),
                None => None,
            }
        } else {
            None
        };
        Ok(Some(NavigationResult {
            url: result.url,
            response: result.response.map(|response| ResponseMetadata {
                request_id,
                #[cfg(feature = "response-request")]
                request,
                #[cfg(feature = "navigation-response-finished")]
                finished_owner: None,
                #[cfg(feature = "response-security-details")]
                security_details: None,
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
        }))
    }

    pub async fn page_go_forward(
        &self,
        page_id: &str,
        wait_until: &str,
        timeout_ms: u32,
    ) -> Result<Option<NavigationResult>, HostError> {
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
            .invoke::<GoForward>(page_id, GoForwardParams { wait_until }, timeout_ms)
            .await;
        let result = crate::completion::publish_command_result(
            result,
            self.is_connected(),
            self.flush_topology(),
        )
        .await?;
        let Some(result) = result.navigation else {
            return Ok(None);
        };
        let request_id = result.loader_id;
        #[cfg(feature = "response-request")]
        let request = if result.response.is_some() {
            match request_id.as_deref() {
                Some(request_id) => Some(self.request_by_native_id(page_id, request_id).await?),
                None => None,
            }
        } else {
            None
        };
        Ok(Some(NavigationResult {
            url: result.url,
            response: result.response.map(|response| ResponseMetadata {
                request_id,
                #[cfg(feature = "response-request")]
                request,
                #[cfg(feature = "navigation-response-finished")]
                finished_owner: None,
                #[cfg(feature = "response-security-details")]
                security_details: None,
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
        }))
    }
}
