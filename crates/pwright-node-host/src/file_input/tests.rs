use super::*;
use pwright_core::ResourceClass;

#[cfg(feature = "file-chooser-native-paths")]
#[tokio::test]
async fn colocated_file_chooser_paths_target_the_utility_world_object() {
    // Arrange
    let client = pwright_fake::FakeCdpClient::from_html("<input type=file>");
    client.set_adopted_node_response(
        47,
        73,
        serde_json::json!({
            "object": {
                "type": "object",
                "subtype": "node",
                "objectId": "utility-input",
            },
        }),
    );
    let files = vec!["/tmp/first.txt".to_owned(), "/tmp/second.txt".to_owned()];

    // Act
    dispatch_colocated_paths(&client, 47, 73, "chooser-group", &files)
        .await
        .expect("native path dispatch");

    // Assert
    assert_eq!(
        client.calls(),
        [
            (
                "DOM.resolveNode".into(),
                vec![serde_json::json!({
                    "backendNodeId": 47,
                    "executionContextId": 73,
                    "objectGroup": "chooser-group",
                })],
            ),
            (
                "DOM.setFileInputFiles".into(),
                vec![serde_json::json!({
                    "objectId": "utility-input",
                    "files": ["/tmp/first.txt", "/tmp/second.txt"],
                })],
            ),
        ]
    );
}

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
