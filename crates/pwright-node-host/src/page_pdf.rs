//! Optional bounded Chromium Page PDF export.
//!
//! Page/session routing, CDP dispatch, response decoding and retained output
//! accounting stay in Rust. Language bindings may copy the bounded result or
//! persist it, but they do not own browser or protocol semantics.

use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use pwright_cdp::CdpSession;
use pwright_core::{
    BudgetedBytes, ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease,
    ResourceLedger, ResourceRequirements, ResourceScope,
};
use serde::Deserialize;

use crate::{HostError, HostSession};

const PRINT_METHOD: &str = "Page.printToPDF";
const READ_METHOD: &str = "IO.read";
const CLOSE_METHOD: &str = "IO.close";
const READ_CHUNK_BYTES: usize = 64 * 1024;
const MAX_STREAM_HANDLE_BYTES: usize = 1024;
const MAX_STREAM_READS: usize = MAX_BYTES / READ_CHUNK_BYTES + 64;
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_CONCURRENT_CAPTURES: usize = 2;
pub const FFI_ALLOWANCE: u64 = 2 * MAX_BYTES as u64 + 4096;

pub(crate) const fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::ArtifactBytes,
            (MAX_BYTES * MAX_CONCURRENT_CAPTURES) as u64,
            MAX_BYTES as u64,
        )
        .with_metadata(MAX_CONCURRENT_CAPTURES, MAX_CONCURRENT_CAPTURES)
}

pub(crate) struct Installation {
    ledger: ResourceLedger,
}

impl Installation {
    pub(crate) fn new(resources: &ProductionResourceLedger) -> Result<Arc<Self>, HostError> {
        Ok(Arc::new(Self {
            ledger: resources
                .claim(requirements())
                .map_err(HostError::backend)?,
        }))
    }

    fn accumulator(&self, page: ObjectId) -> Result<StreamAccumulator, HostError> {
        let lease = self
            .ledger
            .reserve(
                ResourceScope::Object(page),
                ResourceClass::ArtifactBytes,
                MAX_BYTES as u64,
            )
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        Ok(StreamAccumulator {
            bytes: Vec::new(),
            lease,
        })
    }
}

struct StreamAccumulator {
    bytes: Vec<u8>,
    lease: ResourceLease,
}

impl StreamAccumulator {
    fn push(&mut self, data: &str, base64_encoded: bool) -> Result<(), HostError> {
        if data.is_empty() {
            return Ok(());
        }
        let remaining = MAX_BYTES.saturating_sub(self.bytes.len());
        if !base64_encoded {
            if data.len() > remaining {
                return Err(HostError::ResourceExhausted(format!(
                    "PDF output exceeds {MAX_BYTES} bytes"
                )));
            }
            self.bytes
                .try_reserve_exact(data.len())
                .map_err(|_| HostError::ResourceExhausted("PDF output allocation failed".into()))?;
            self.bytes.extend_from_slice(data.as_bytes());
            return Ok(());
        }
        if !data.len().is_multiple_of(4) {
            return Err(HostError::Backend(
                "IO.read returned malformed base64 PDF data".into(),
            ));
        }
        let padding = data
            .as_bytes()
            .iter()
            .rev()
            .take_while(|byte| **byte == b'=')
            .count();
        if padding > 2 {
            return Err(HostError::Backend(
                "IO.read returned malformed base64 PDF padding".into(),
            ));
        }
        let decoded_bound = data
            .len()
            .checked_div(4)
            .and_then(|groups| groups.checked_mul(3))
            .and_then(|bytes| bytes.checked_sub(padding))
            .ok_or_else(|| HostError::ResourceExhausted("PDF size arithmetic overflow".into()))?;
        if decoded_bound > remaining {
            return Err(HostError::ResourceExhausted(format!(
                "PDF output exceeds {MAX_BYTES} bytes"
            )));
        }
        let decoded = STANDARD
            .decode(data)
            .map_err(|_| HostError::Backend("IO.read returned invalid base64 PDF data".into()))?;
        self.bytes
            .try_reserve_exact(decoded.len())
            .map_err(|_| HostError::ResourceExhausted("PDF output allocation failed".into()))?;
        self.bytes.extend_from_slice(&decoded);
        Ok(())
    }

    fn finish(self) -> Result<PdfOutput, HostError> {
        if self.bytes.is_empty() {
            return Err(HostError::Backend(
                "Page.printToPDF returned an empty document".into(),
            ));
        }
        BudgetedBytes::from_reserved(self.lease, ResourceClass::ArtifactBytes, self.bytes)
            .map(PdfOutput)
            .map_err(HostError::backend)
    }
}

#[derive(Debug)]
pub struct PdfOutput(BudgetedBytes);

impl PdfOutput {
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PrintToPdfReturns {
    stream: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct IoReadReturns {
    data: String,
    #[serde(default)]
    base64_encoded: bool,
    #[serde(default)]
    eof: bool,
}

struct StreamCloseGuard {
    stream: Option<(CdpSession, String)>,
}

impl StreamCloseGuard {
    fn new(session: CdpSession, handle: String) -> Self {
        Self {
            stream: Some((session, handle)),
        }
    }

    fn session(&self) -> &CdpSession {
        &self.stream.as_ref().expect("live PDF stream").0
    }

    fn handle(&self) -> &str {
        &self.stream.as_ref().expect("live PDF stream").1
    }

    async fn close(&mut self) -> Result<(), HostError> {
        let params = serde_json::json!({ "handle": self.handle() });
        let result = self
            .session()
            .send(CLOSE_METHOD, params)
            .await
            .map(|_| ())
            .map_err(HostError::backend);
        self.stream = None;
        result
    }
}

impl Drop for StreamCloseGuard {
    fn drop(&mut self) {
        let Some((session, handle)) = self.stream.take() else {
            return;
        };
        if let Ok(runtime) = tokio::runtime::Handle::try_current() {
            std::mem::drop(runtime.spawn(async move {
                let _ = session
                    .send(CLOSE_METHOD, serde_json::json!({ "handle": handle }))
                    .await;
            }));
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PdfOptions {
    pub tagged: bool,
    pub outline: bool,
}

fn print_params(options: PdfOptions) -> serde_json::Value {
    serde_json::json!({
        "landscape": false,
        "displayHeaderFooter": false,
        "headerTemplate": "",
        "footerTemplate": "",
        "printBackground": false,
        "scale": 1,
        "paperWidth": 8.5,
        "paperHeight": 11,
        "marginTop": 0,
        "marginBottom": 0,
        "marginLeft": 0,
        "marginRight": 0,
        "pageRanges": "",
        "preferCSSPageSize": false,
        "generateTaggedPDF": options.tagged,
        "generateDocumentOutline": options.outline,
        "transferMode": "ReturnAsStream"
    })
}

async fn read_stream(
    installation: &Installation,
    page: ObjectId,
    guard: &StreamCloseGuard,
) -> Result<StreamAccumulator, HostError> {
    let mut output = installation.accumulator(page)?;
    for _ in 0..MAX_STREAM_READS {
        let response = guard
            .session()
            .send(
                READ_METHOD,
                serde_json::json!({
                    "handle": guard.handle(),
                    "size": READ_CHUNK_BYTES
                }),
            )
            .await
            .map_err(HostError::backend)?;
        let response: IoReadReturns =
            serde_json::from_value(response).map_err(HostError::backend)?;
        output.push(&response.data, response.base64_encoded)?;
        if response.eof {
            return Ok(output);
        }
    }
    Err(HostError::ResourceExhausted(format!(
        "PDF stream exceeds {MAX_STREAM_READS} reads"
    )))
}

impl HostSession {
    pub async fn page_pdf(
        &self,
        page_id: &str,
        options: PdfOptions,
        timeout_ms: u32,
    ) -> Result<PdfOutput, HostError> {
        let (page, _call, _operation) = self.admit_operation(page_id)?;
        let (session, installation, queue) = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let route = topology
                .topology()
                .page_session_routes()
                .find(|route| route.object_id() == page)
                .ok_or(HostError::TargetClosed)?;
            (
                CdpSession::new(
                    live.composition.connection().clone(),
                    route.session_id().to_owned(),
                    route.target_id().to_owned(),
                ),
                Arc::clone(&live.page_pdf),
                Arc::clone(&live.screenshot_queue),
            )
        };
        queue
            .run(page, timeout_ms, &self.stop, async move {
                let response = session
                    .send(PRINT_METHOD, print_params(options))
                    .await
                    .map_err(HostError::backend)?;
                let response: PrintToPdfReturns =
                    serde_json::from_value(response).map_err(HostError::backend)?;
                let mut guard = StreamCloseGuard::new(session, response.stream);
                if guard.handle().len() > MAX_STREAM_HANDLE_BYTES {
                    let _ = guard.close().await;
                    return Err(HostError::ResourceExhausted(format!(
                        "PDF stream handle exceeds {MAX_STREAM_HANDLE_BYTES} bytes"
                    )));
                }
                let read = read_stream(&installation, page, &guard).await;
                let close = guard.close().await;
                match read {
                    Err(error) => Err(error),
                    Ok(output) => {
                        close?;
                        output.finish()
                    }
                }
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::{ResourceLimits, ResourceMetrics};

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        ProductionResourceLedger::new(
            ResourceLimits::deny_all(required.scopes, required.leases).with_class(
                ResourceClass::ArtifactBytes,
                required.class(ResourceClass::ArtifactBytes).global,
                required.class(ResourceClass::ArtifactBytes).per_scope,
            ),
            required,
        )
        .unwrap()
    }

    fn artifact_bytes(metrics: &ResourceMetrics) -> u64 {
        metrics.class(ResourceClass::ArtifactBytes).current
    }

    #[test]
    fn default_payload_matches_playwright_letter_defaults() {
        // Arrange
        let expected = serde_json::json!({
            "landscape": false,
            "displayHeaderFooter": false,
            "headerTemplate": "",
            "footerTemplate": "",
            "printBackground": false,
            "scale": 1,
            "paperWidth": 8.5,
            "paperHeight": 11,
            "marginTop": 0,
            "marginBottom": 0,
            "marginLeft": 0,
            "marginRight": 0,
            "pageRanges": "",
            "preferCSSPageSize": false,
            "generateTaggedPDF": false,
            "generateDocumentOutline": false,
            "transferMode": "ReturnAsStream"
        });

        // Act
        let payload = print_params(PdfOptions::default());

        // Assert
        assert_eq!(PRINT_METHOD, "Page.printToPDF");
        assert_eq!(payload, expected);
    }

    #[test]
    fn explicit_tagged_and_outline_options_map_to_chromium_fields() {
        // Arrange
        let options = PdfOptions {
            tagged: true,
            outline: true,
        };

        // Act
        let payload = print_params(options);

        // Assert
        assert_eq!(payload["generateTaggedPDF"], true);
        assert_eq!(payload["generateDocumentOutline"], true);
        assert_eq!(payload["transferMode"], "ReturnAsStream");
    }

    #[test]
    fn streamed_output_holds_and_releases_the_exact_artifact_budget() {
        // Arrange
        let resources = resources();
        let installation = Installation::new(&resources).unwrap();

        // Act
        let mut streamed = installation.accumulator(ObjectId::new(7)).unwrap();
        streamed.push("%PDF", false).unwrap();
        streamed.push(&STANDARD.encode(b"-1.7\n"), true).unwrap();
        let output = streamed.finish().unwrap();
        let retained = artifact_bytes(&resources.metrics().unwrap());
        drop(output);
        let released = artifact_bytes(&resources.metrics().unwrap());

        // Assert
        assert_eq!(retained, 9);
        assert_eq!(released, 0);
    }

    #[test]
    fn oversized_stream_is_rejected_and_releases_its_reserved_budget() {
        // Arrange
        let resources = resources();
        let installation = Installation::new(&resources).unwrap();
        let mut streamed = installation.accumulator(ObjectId::new(7)).unwrap();
        streamed.bytes.resize(MAX_BYTES, b'A');

        // Act
        let error = streamed.push("B", false).unwrap_err();
        drop(streamed);

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
        assert_eq!(artifact_bytes(&resources.metrics().unwrap()), 0);
    }
}
