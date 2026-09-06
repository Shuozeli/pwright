//! Optional composable server binding for value-only handle evaluation.
//!
//! The caller injects the service and chooses the runtime, backend, transport,
//! and executor. This crate starts none of them.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_server;

pub use generated_server::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_cap_evaluation::EvaluationValueLimits;
    use pwright_codec_handle_evaluation::{HandleEvaluationCodec, HandleEvaluationCodecLimits};
    use pwright_core::ProtocolVersion;
    use pwright_server_wire::ServerCapabilitySource;
    use pwright_transport::{FrameLimits, NegotiationLimits, WireProtocol};

    use super::{HandleEvaluationServerServices, handle_evaluation_server_handlers};

    #[test]
    fn generated_handler_refuses_to_advertise_without_lifecycle_dependency() {
        // Arrange
        let values = EvaluationValueLimits::new(8, 64, 64, 1024, 1024).unwrap();
        let codec = HandleEvaluationCodec::new(
            HandleEvaluationCodecLimits::new(1024, 512, values).unwrap(),
        );
        let version = ProtocolVersion::new(1, 0);
        let frame_limits = FrameLimits::new(16 * 1024, 0, 0, 16 * 1024);
        let protocol = Arc::new(WireProtocol::new_exact_directional(
            codec,
            version,
            frame_limits,
            frame_limits,
        ));
        let handlers = handle_evaluation_server_handlers(
            protocol,
            HandleEvaluationServerServices {
                evaluate_handle: Arc::new(()),
                handle_evaluate: Arc::new(()),
                handle_evaluate_handle: Arc::new(()),
                handle_get_properties: Arc::new(()),
                handle_get_property: Arc::new(()),
                handle_json_value: Arc::new(()),
            },
        );
        // Act
        let error = handlers
            .capability_manifest_with_object_kinds(
                BROWSER_OBJECT_KIND_CATALOG,
                NegotiationLimits::new(8, 64),
            )
            .expect_err("standalone handle evaluation must require lifecycle");

        // Assert
        assert!(error.to_string().contains("pwright.handle_lifecycle"));
    }
}
