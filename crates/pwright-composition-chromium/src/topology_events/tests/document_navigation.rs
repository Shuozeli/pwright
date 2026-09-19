use super::*;

async fn tree_fixture() -> DriverFixture {
    let mut f = fixture(16);
    f.factory
        .client("target-a")
        .set_frame_tree(frame_tree(serde_json::json!({
            "frame": frame_value("target-a", None),
            "childFrames": [
                {"frame": frame_value("child", Some("target-a")), "childFrames": [
                    {"frame": frame_value("nested", Some("child"))}
                ]},
                {"frame": frame_value("sibling", Some("target-a"))}
            ]
        })));
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    f
}

fn frame_id(f: &DriverFixture, native: &str) -> ObjectId {
    f.topology
        .lock()
        .unwrap()
        .topology()
        .frame_object_id(native)
        .unwrap()
}

#[tokio::test]
async fn child_document_commit_publishes_descendant_disposal_to_runtime() {
    // Arrange
    let mut f = tree_fixture().await;
    let child = frame_id(&f, "child");
    let nested = frame_id(&f, "nested");
    let sibling = frame_id(&f, "sibling");
    f.sender.send(CdpEvent::new("Page.frameNavigated".into(),
        serde_json::json!({"frame": frame_value("child", Some("target-a")), "type":"Navigation"}),
        Some("session-a".into()))).unwrap();
    // Act
    f.driver.receive_once(&f.runtime).await.unwrap();
    // Assert: disposal is in both production topology and runtime, not URL-only.
    assert_eq!(f.runtime.is_object_live(nested), Ok(false));
    assert_eq!(f.runtime.is_object_live(child), Ok(true));
    assert_eq!(f.runtime.is_object_live(sibling), Ok(true));
    assert!(
        f.topology
            .lock()
            .unwrap()
            .topology()
            .frame_object_id("nested")
            .is_none()
    );
}

#[tokio::test]
async fn unchanged_main_frame_id_still_retires_its_old_document_children() {
    // Arrange
    let mut f = tree_fixture().await;
    let root = frame_id(&f, "target-a");
    let child = frame_id(&f, "child");
    let nested = frame_id(&f, "nested");
    let sibling = frame_id(&f, "sibling");
    f.sender
        .send(CdpEvent::new(
            "Page.frameNavigated".into(),
            serde_json::json!({"frame": frame_value("target-a", None), "type":"Navigation"}),
            Some("session-a".into()),
        ))
        .unwrap();
    // Act
    f.driver.receive_once(&f.runtime).await.unwrap();
    // Assert
    assert_eq!(f.runtime.is_object_live(root), Ok(true));
    for id in [child, nested, sibling] {
        assert_eq!(f.runtime.is_object_live(id), Ok(false));
    }
    assert_eq!(frame_id(&f, "target-a"), root);
}

#[tokio::test]
async fn hash_and_stale_session_events_preserve_descendants() {
    for (session, method, params) in [
        (
            "session-a",
            "Page.navigatedWithinDocument",
            serde_json::json!({
                "frameId":"child", "url":"about:blank#hash", "navigationType":"fragment"
            }),
        ),
        (
            "retired-session",
            "Page.frameNavigated",
            serde_json::json!({
                "frame": frame_value("child", Some("target-a")), "type":"Navigation"
            }),
        ),
    ] {
        // Arrange
        let mut f = tree_fixture().await;
        f.driver = f
            .driver
            .with_frame_url_observer(Arc::new(NavigationProjection::default()));
        let nested = frame_id(&f, "nested");
        f.sender
            .send(CdpEvent::new(method.into(), params, Some(session.into())))
            .unwrap();
        // Act
        f.driver.receive_once(&f.runtime).await.unwrap();
        // Assert
        assert_eq!(f.runtime.is_object_live(nested), Ok(true));
        assert_eq!(frame_id(&f, "nested"), nested);
    }
}
