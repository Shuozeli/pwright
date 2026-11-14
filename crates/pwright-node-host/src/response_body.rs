//! Optional one-command navigation response-body leaf.
//!
//! Routing, request identity, CDP dispatch, decoding and bounded ownership stay
//! in Rust. The leaf retains no Page state, task, queue, timer or browser
//! process. Node receives only a copied Buffer during its admitted settlement.

use base64::{Engine as _, engine::general_purpose::STANDARD};
use pwright_cdp::{CdpSession, domains::network::ResponseBody};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequirements,
    ResourceScope,
};

use crate::{HostError, HostSession};

pub const MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
pub const FFI_ALLOWANCE: u64 = 64 * 1024;
const MAX_OPERATIONS: u64 = 8;
const REQUEST_ID_BYTES: usize = 256;
const SCOPE: ResourceScope = ResourceScope::extension(141, 1);

pub struct ResponseBodyOutput {
    bytes: Vec<u8>,
    _lease: ResourceLease,
}

impl ResponseBodyOutput {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }
}

pub(crate) const fn requirements() -> ResourceRequirements {
    ResourceRequirements::none()
        .with_class(
            ResourceClass::BackendStateBytes,
            MAX_BODY_BYTES as u64 * MAX_OPERATIONS,
            MAX_BODY_BYTES as u64 * MAX_OPERATIONS,
        )
        .with_metadata(1, MAX_OPERATIONS as usize)
}

fn reserve(resources: &ProductionResourceLedger) -> Result<ResourceLease, HostError> {
    resources
        .claim(requirements())
        .and_then(|ledger| {
            Ok(ledger.reserve(
                SCOPE,
                ResourceClass::BackendStateBytes,
                MAX_BODY_BYTES as u64,
            )?)
        })
        .map_err(|error| HostError::ResourceExhausted(error.to_string()))
}

fn parse_id(id: &str) -> Result<ObjectId, HostError> {
    id.parse::<u64>()
        .ok()
        .filter(|id| *id != 0)
        .map(ObjectId::new)
        .ok_or_else(|| HostError::InvalidArgument("invalid portable object ID".into()))
}

fn decode(body: ResponseBody) -> Result<Vec<u8>, HostError> {
    if body.body.len() > MAX_BODY_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "response body exceeds {MAX_BODY_BYTES} encoded bytes"
        )));
    }
    if !body.base64_encoded {
        return Ok(body.body.into_bytes());
    }
    let capacity = body
        .body
        .len()
        .checked_div(4)
        .and_then(|groups| groups.checked_mul(3))
        .and_then(|bytes| bytes.checked_add(3))
        .ok_or_else(|| HostError::ResourceExhausted("response body size overflow".into()))?;
    if capacity > MAX_BODY_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "decoded response body exceeds {MAX_BODY_BYTES} bytes"
        )));
    }
    let mut decoded = Vec::new();
    decoded
        .try_reserve_exact(capacity)
        .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
    STANDARD
        .decode_vec(body.body.as_bytes(), &mut decoded)
        .map_err(|error| HostError::Backend(format!("invalid base64 response body: {error}")))?;
    if decoded.len() > MAX_BODY_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "decoded response body exceeds {MAX_BODY_BYTES} bytes"
        )));
    }
    Ok(decoded)
}

impl HostSession {
    pub async fn frame_response_body(
        &self,
        page_id: &str,
        frame_id: &str,
        request_id: &str,
    ) -> Result<ResponseBodyOutput, HostError> {
        if request_id.is_empty() || request_id.len() > REQUEST_ID_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "response request ID must contain 1 to {REQUEST_ID_BYTES} bytes"
            )));
        }
        let page = parse_id(page_id)?;
        let (frame, _call, _operation) = self.admit_operation(frame_id)?;
        let lease = reserve(&self.resources)?;
        let session = {
            let state = self.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            let route = topology
                .topology()
                .frame_navigation_route(frame)
                .filter(|route| route.page_object_id() == page)
                .ok_or(HostError::TargetClosed)?;
            CdpSession::new(
                live.composition.connection().clone(),
                route.session_id().to_owned(),
                route.target_id().to_owned(),
            )
        };
        let body = tokio::select! {
            biased;
            () = self.stop.cancelled() => return Err(HostError::TargetClosed),
            body = session.network_get_response_body(request_id) => body.map_err(HostError::backend)?,
        };
        Ok(ResponseBodyOutput {
            bytes: decode(body)?,
            _lease: lease,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_body_decoding_preserves_utf8_and_binary_bytes() {
        // Arrange
        let plain = ResponseBody {
            body: "héllo".into(),
            base64_encoded: false,
        };
        let binary = ResponseBody {
            body: "AAH+/w==".into(),
            base64_encoded: true,
        };

        // Act
        let plain = decode(plain).unwrap();
        let binary = decode(binary).unwrap();

        // Assert
        assert_eq!(plain, "héllo".as_bytes());
        assert_eq!(binary, [0, 1, 254, 255]);
    }

    #[test]
    fn response_body_rejects_invalid_base64_and_oversized_input() {
        // Arrange
        let invalid = ResponseBody {
            body: "not base64!".into(),
            base64_encoded: true,
        };
        let oversized = ResponseBody {
            body: "x".repeat(MAX_BODY_BYTES + 1),
            base64_encoded: false,
        };

        // Act
        let invalid = decode(invalid);
        let oversized = decode(oversized);

        // Assert
        assert!(matches!(invalid, Err(HostError::Backend(_))));
        assert!(matches!(oversized, Err(HostError::ResourceExhausted(_))));
    }
}
