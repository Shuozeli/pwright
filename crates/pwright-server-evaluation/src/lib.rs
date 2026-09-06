//! Optional composable server binding for value-only evaluation.
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
    use pwright_codec_evaluation::{EvaluationCodec, EvaluationCodecLimits};
    use pwright_core::ProtocolVersion;
    use pwright_server_wire::ServerCapabilitySource;
    use pwright_transport::{
        FrameLimits, NegotiationLimits, VersionPolicy, WireCapabilityId, WireProtocol,
    };

    use super::{EvaluationServerServices, evaluation_server_handlers};

    #[test]
    fn generated_handler_advertises_only_value_evaluation() {
        // Arrange
        let values = EvaluationValueLimits::new(8, 64, 64, 1024, 1024).unwrap();
        let codec = EvaluationCodec::new(EvaluationCodecLimits::new(1024, 512, values).unwrap());
        let version = ProtocolVersion::new(1, 0);
        let frame_limits = FrameLimits::new(16 * 1024, 0, 0, 16 * 1024);
        let protocol = Arc::new(WireProtocol::new_exact_directional(
            codec,
            version,
            frame_limits,
            frame_limits,
        ));
        let handlers = evaluation_server_handlers(
            protocol,
            EvaluationServerServices {
                evaluate_value: Arc::new(()),
            },
        );
        let capability =
            WireCapabilityId::new("pwright.evaluation", 64).expect("capability ID should build");

        // Act
        let manifest = handlers
            .capability_manifest_with_object_kinds(
                BROWSER_OBJECT_KIND_CATALOG,
                NegotiationLimits::new(8, 64),
            )
            .expect("evaluation handler should preflight");

        // Assert
        assert_eq!(
            manifest.support(&capability),
            Some(VersionPolicy::compatible_through(version))
        );
    }
}
