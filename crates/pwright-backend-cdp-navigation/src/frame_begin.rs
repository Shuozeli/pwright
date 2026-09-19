//! Frame-targeted native initiation, not the complete portable Frame.goto API.
use super::*;

impl CdpNavigationService {
    /// Begin navigation in an explicit native Frame on the injected session.
    ///
    /// The caller owns Frame/session routing, lifetime and lifecycle waiting.
    /// This primitive never falls back to a Page, creates an execution world,
    /// retries a side effect, subscribes to events, or calls Page.stopLoading.
    /// Its result is a native initiation receipt, not document-load completion.
    pub fn begin_frame_navigation<'a>(
        &'a self,
        frame_id: &'a str,
        params: BeginNavigationParams,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<BeginNavigationResult, OperationError<NavigationErrorDetails>>,
                > + Send
                + 'a,
        >,
    > {
        let resources = validate_native_id(
            "navigation target Frame ID",
            frame_id,
            self.limits.native_id_bytes,
        )
        .and_then(|()| self.reserve_operation(&params.url, params.referrer.as_ref()));
        let resources = match resources {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        // Existing begin-navigation allowance is conservative for this path:
        // <=4 native-ID copies overlap (serialized request + frame/loader
        // response), within its 8-ID allowance. There is no event/request index.
        // The caller lends frame_id; no lasting route/string owner is added.
        Box::pin(async move {
            let _resources = resources;
            let response = self
                .client
                .page_navigate_in_frame(frame_id, &params.url, params.referrer.as_deref())
                .await
                .map_err(map_cdp_error)?;
            // The legacy Page primitive allows this CDP error through. Frame
            // dispatch must preserve it as an initiation failure, as Chromium's
            // Playwright delegate does. This is not an HTTP response status:
            // an ordinary 404/500 response has no Page.navigate errorText.
            if response
                .get("errorText")
                .and_then(serde_json::Value::as_str)
                == Some(HTTP_STATUS_FAILURE)
            {
                validate_navigation_value(
                    "navigation error text",
                    HTTP_STATUS_FAILURE,
                    self.limits.status_text_bytes,
                )?;
                return Err(OperationError::new(
                    FailureKind::Backend,
                    format!("document navigation failed: {HTTP_STATUS_FAILURE}"),
                    Some(NavigationErrorDetails::Network(
                        HTTP_STATUS_FAILURE.to_owned(),
                    )),
                ));
            }
            let navigation = parse_navigation_result(response, self.limits)?;
            if navigation.frame_id != frame_id {
                return Err(invalid_event(
                    "Page.navigate returned a different Frame identity",
                ));
            }
            Ok(navigation)
        })
    }
}

#[cfg(test)]
mod tests;
