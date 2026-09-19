use super::*;
use pwright_core::ResourceClass;

#[cfg(feature = "retained-file-input")]
#[test]
fn independent_adoption_and_mutation_cleanup_failures_are_both_reported() {
    // Arrange
    let adoption = Err(HostError::backend("adoption release unconfirmed"));
    let mutation = Err(HostError::backend("upload release unconfirmed"));
    // Act
    let error = combine_native_cleanup(adoption, mutation).unwrap_err();
    // Assert
    assert!(error.to_string().contains("adoption release unconfirmed"));
    assert!(error.to_string().contains("upload release unconfirmed"));
}

#[test]
fn selected_uploads_do_not_widen_ordinary_cdp_command_or_receive_limits() {
    // Arrange
    let timeout = std::time::Duration::from_secs(5);
    // Act
    let factory = crate::profile::attach_factory(timeout).unwrap();
    // Assert
    let config = factory.profile().connection().config();
    assert_eq!(config.max_command_bytes, 4 * 1024 * 1024);
    assert_eq!(config.max_response_bytes, 4 * 1024 * 1024);
    assert_eq!(config.max_websocket_bytes, 4 * 1024 * 1024);
    assert_eq!(profile::large_commands().unwrap().capacity(), 1);
}

#[test]
fn source_and_encoded_transport_have_independent_requirements() {
    // Arrange
    let source = profile::source_requirements();
    // Act
    let wire = profile::large_commands().unwrap().requirements();
    // Assert
    assert_eq!(
        source.class(ResourceClass::ArtifactBytes).global,
        50 * 1024 * 1024
    );
    assert_eq!(source.class(ResourceClass::WireBytes).global, 0);
    assert_eq!(wire.class(ResourceClass::ArtifactBytes).global, 0);
    assert!(wire.class(ResourceClass::WireBytes).global > 72 * 1024 * 1024);
}
