use pwright_node_host::HostSession;

#[tokio::test]
#[ignore = "requires an explicitly selected dedicated Chrome endpoint"]
async fn explicit_close_drains_native_session_and_transport() {
    // Arrange
    let _ = tracing_subscriber::fmt()
        .with_max_level(tracing_subscriber::filter::LevelFilter::DEBUG)
        .try_init();
    let endpoint =
        std::env::var("PWRIGHT_TEST_CDP").expect("explicit dedicated CDP endpoint required");
    let session = HostSession::connect_over_cdp(&endpoint, 5000)
        .await
        .expect("attach should initialize");
    assert!(
        !session
            .contexts()
            .expect("contexts should be available")
            .is_empty()
    );
    // Act
    session.close().await.expect("explicit close should finish");
    // Assert
    let metrics = session.metrics().expect("metrics should remain available");
    assert!(!metrics.connected);
    assert_eq!(metrics.active_leases, 0);
    assert_eq!(metrics.active_operations, 0);
    assert_eq!(metrics.cleanup_in_flight, 0);
}
