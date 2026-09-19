use super::*;

fn world_fixture() -> CdpTopology {
    let mut topology = topology();
    register_context(&mut topology, "context", 2);
    attach_page(&mut topology, "page", "local", "context", 3, 4);
    topology
        .attach_frame(
            EPOCH_ONE,
            CdpFrameAttachment::local("page", "child", "page"),
            ObjectId::new(5),
        )
        .unwrap();
    for (id, name, kind) in [
        (1, "main", CdpWorldKind::Main),
        (2, "utility", CdpWorldKind::Utility),
    ] {
        topology
            .attach_world(
                EPOCH_ONE,
                CdpWorldAttachment::new("local", id, name, "child", kind),
            )
            .unwrap();
    }
    topology
}

#[test]
fn renderer_handoff_hides_old_world_routes_without_claiming_native_destruction() {
    // Arrange
    let mut topology = world_fixture();
    // Act
    topology
        .attach_frame(
            EPOCH_ONE,
            CdpFrameAttachment::remote("page", "child", "page", "remote"),
            ObjectId::new(5),
        )
        .unwrap();
    // Assert
    assert_eq!(topology.frame_session_id(ObjectId::new(5)), Some("remote"));
    assert_eq!(topology.frame_execution_routes().count(), 0);
    assert_eq!(topology.frame_utility_execution_routes().count(), 0);
    assert_eq!(topology.world_count_for_frame(ObjectId::new(5)), 2);
}

#[test]
fn swap_gap_hides_both_world_routes_until_authoritative_session_rebinding() {
    // Arrange
    let mut topology = world_fixture();
    // Act
    topology
        .detach_frame(EPOCH_ONE, "child", CdpFrameDetachReason::Swap)
        .unwrap();
    // Assert
    assert_eq!(topology.frame_session_id(ObjectId::new(5)), None);
    assert_eq!(topology.frame_execution_routes().count(), 0);
    assert_eq!(topology.frame_utility_execution_routes().count(), 0);
    assert_eq!(topology.world_count_for_frame(ObjectId::new(5)), 2);
}

#[test]
fn late_previous_renderer_announcement_cannot_replace_the_current_world() {
    // Arrange
    let mut topology = world_fixture();
    topology
        .attach_frame(
            EPOCH_ONE,
            CdpFrameAttachment::remote("page", "child", "page", "remote"),
            ObjectId::new(5),
        )
        .unwrap();
    topology
        .attach_world(
            EPOCH_ONE,
            CdpWorldAttachment::new("remote", 3, "current-main", "child", CdpWorldKind::Main),
        )
        .unwrap();
    // Act
    let late = topology
        .attach_world(
            EPOCH_ONE,
            CdpWorldAttachment::new("local", 4, "late-main", "child", CdpWorldKind::Main),
        )
        .unwrap();
    // Assert
    assert_eq!(late.disposition(), CdpTopologyDisposition::Idempotent);
    assert!(late.effects().is_empty());
    let current = topology.frame_execution_routes().next().unwrap();
    assert_eq!(current.session_id(), "remote");
    assert_eq!(current.unique_context_id(), "current-main");
}
