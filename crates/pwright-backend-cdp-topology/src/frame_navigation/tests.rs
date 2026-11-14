use super::*;
use crate::{
    CdpContextOwnership, CdpFrameAttachment, CdpFrameDetachReason, CdpPageObjectIds, CdpPageTarget,
    CdpTopologyLimits,
};

const EPOCH: ConnectionEpoch = ConnectionEpoch::new(1);

#[test]
fn document_commit_retires_only_selected_descendants_and_their_renderer_routes() {
    // Arrange
    let mut topology = topology();
    let selected = topology.frame_navigation_route(ObjectId::new(5)).unwrap();
    let selected_session = selected.session_id().to_owned();
    // Act
    let update = topology
        .frame_document_committed(EPOCH, &selected_session, "local")
        .unwrap();
    // Assert
    assert!(topology.frame_navigation_route(ObjectId::new(4)).is_some());
    assert!(topology.frame_navigation_route(ObjectId::new(5)).is_some());
    assert!(topology.frame_navigation_route(ObjectId::new(6)).is_none());
    assert!(topology.frame_navigation_route(ObjectId::new(7)).is_none());
    assert!(topology.target_session_route("remote-session").is_none());
    let mut expected = Vec::new();
    #[cfg(feature = "frame-detach-events")]
    expected.extend([
        crate::CdpTopologyEffect::FrameRemoved {
            page: ObjectId::new(3),
            frame: ObjectId::new(7),
        },
        crate::CdpTopologyEffect::FrameRemoved {
            page: ObjectId::new(3),
            frame: ObjectId::new(6),
        },
    ]);
    expected.push(crate::CdpTopologyEffect::Lifecycle(
        pwright_core::ControlMessage::Dispose {
            object_id: ObjectId::new(6),
        },
    ));
    assert_eq!(update.effects(), expected.as_slice());
}

#[test]
fn stale_renderer_cannot_commit_another_frames_document() {
    for (epoch, session, native) in [
        (ConnectionEpoch::new(0), "remote-session", "remote"),
        (EPOCH, "main-session", "remote"),
        (EPOCH, "retired-session", "local"),
        (EPOCH, "main-session", "missing"),
    ] {
        // Arrange
        let mut topology = topology();
        // Act
        topology
            .frame_document_committed(epoch, session, native)
            .unwrap();
        // Assert
        assert!(topology.frame_navigation_route(ObjectId::new(7)).is_some());
    }
}

#[test]
fn remote_commit_preserves_remote_frame_and_local_ancestors() {
    // Arrange
    let mut topology = topology();
    // Act
    topology
        .frame_document_committed(EPOCH, "remote-session", "remote")
        .unwrap();
    // Assert
    for id in [4, 5, 6] {
        assert!(topology.frame_navigation_route(ObjectId::new(id)).is_some());
    }
    assert!(topology.frame_navigation_route(ObjectId::new(7)).is_none());
    assert_eq!(
        topology.frame_session_id(ObjectId::new(6)),
        Some("remote-session")
    );
}

fn topology() -> CdpTopology {
    let mut topology = CdpTopology::new(
        ObjectId::new(1),
        EPOCH,
        CdpTopologyLimits::new(64, 8, 8, 16, 16, 64),
    )
    .unwrap();
    topology
        .register_context(
            EPOCH,
            "context",
            ObjectId::new(2),
            CdpContextOwnership::Owned,
        )
        .unwrap();
    topology
        .attach_page(
            EPOCH,
            CdpPageTarget::new("page", "main-session", "context"),
            CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
        )
        .unwrap();
    topology
        .attach_frame(
            EPOCH,
            CdpFrameAttachment::main("page", "main-frame"),
            ObjectId::new(4),
        )
        .unwrap();
    topology
        .attach_frame(
            EPOCH,
            CdpFrameAttachment::local("page", "local", "main-frame"),
            ObjectId::new(5),
        )
        .unwrap();
    topology
        .attach_frame(
            EPOCH,
            CdpFrameAttachment::remote("page", "remote", "local", "remote-session"),
            ObjectId::new(6),
        )
        .unwrap();
    topology
        .attach_frame(
            EPOCH,
            CdpFrameAttachment::local("page", "nested", "remote"),
            ObjectId::new(7),
        )
        .unwrap();
    topology
}

#[test]
fn navigation_routes_preserve_frame_and_nearest_renderer_without_worlds() {
    // Arrange
    let topology = topology();
    assert_eq!(topology.frame_execution_routes().count(), 0);
    assert_eq!(topology.frame_utility_execution_routes().count(), 0);
    // Act
    let routes = [4, 5, 6, 7].map(|id| topology.frame_navigation_route(ObjectId::new(id)).unwrap());
    // Assert
    let expected = [
        ("main-frame", "page", "main-session"),
        ("local", "page", "main-session"),
        ("remote", "remote", "remote-session"),
        ("nested", "remote", "remote-session"),
    ];
    for (index, (route, expected)) in routes.iter().zip(expected).enumerate() {
        assert_eq!(
            (route.frame_id(), route.target_id(), route.session_id()),
            expected
        );
        assert_eq!(
            route.object_id(),
            ObjectId::new(4 + u64::try_from(index).unwrap())
        );
        assert_eq!(route.page_object_id(), ObjectId::new(3));
        assert_eq!(route.epoch(), EPOCH);
        assert!(std::ptr::eq(
            route.frame_id(),
            topology.frame_native_id(route.object_id()).unwrap()
        ));
    }
    assert_eq!(topology.frame_navigation_route(ObjectId::new(3)), None);
    assert_eq!(topology.frame_navigation_route(ObjectId::new(99)), None);
}

#[test]
fn renderer_swap_gap_hides_remote_subtree_without_page_fallback() {
    // Arrange
    let mut topology = topology();
    topology
        .detach_target(EPOCH, "remote-session", Some("remote"))
        .unwrap();
    // Act
    let routes = [4, 5, 6, 7].map(|id| topology.frame_navigation_route(ObjectId::new(id)));
    // Assert
    assert!(routes[0].is_some());
    assert!(routes[1].is_some());
    assert_eq!(routes[2], None);
    assert_eq!(routes[3], None);
}

#[test]
fn reattached_remote_session_replaces_route_but_preserves_frame_identity() {
    // Arrange
    let mut topology = topology();
    topology
        .detach_target(EPOCH, "remote-session", Some("remote"))
        .unwrap();
    topology
        .attach_frame(
            EPOCH,
            CdpFrameAttachment::remote("page", "remote", "local", "new-session"),
            ObjectId::new(99),
        )
        .unwrap();
    // Act
    let route = topology.frame_navigation_route(ObjectId::new(6)).unwrap();
    // Assert
    assert_eq!(route.frame_id(), "remote");
    assert_eq!(route.object_id(), ObjectId::new(6));
    assert_eq!(route.target_id(), "remote");
    assert_eq!(route.session_id(), "new-session");
    assert_eq!(topology.frame_navigation_route(ObjectId::new(99)), None);
    // Rebinding retains the remote Frame, but retires its old document subtree.
    assert_eq!(topology.frame_navigation_route(ObjectId::new(7)), None);
}

#[test]
fn frame_removal_does_not_leave_a_navigation_route() {
    // Arrange
    let mut topology = topology();
    topology
        .detach_frame(EPOCH, "local", CdpFrameDetachReason::Remove)
        .unwrap();
    // Act
    let routes = [4, 5, 6, 7].map(|id| topology.frame_navigation_route(ObjectId::new(id)));
    // Assert
    assert!(routes[0].is_some());
    assert_eq!(routes[1..], [None, None, None]);
}

#[test]
fn crashed_page_has_no_navigation_route_even_with_attached_sessions() {
    // Arrange
    let mut topology = topology();
    topology.crash_target(EPOCH, "page", "crashed", 1).unwrap();
    // Act
    let routes = [4, 5, 6, 7].map(|id| topology.frame_navigation_route(ObjectId::new(id)));
    // Assert
    assert_eq!(routes, [None; 4]);
}

#[test]
fn reconnect_does_not_export_routes_from_the_previous_connection() {
    // Arrange
    let mut topology = topology();
    topology.begin_reconnect(ConnectionEpoch::new(2)).unwrap();
    // Act
    let routes = [4, 5, 6, 7].map(|id| topology.frame_navigation_route(ObjectId::new(id)));
    // Assert
    assert_eq!(routes, [None; 4]);
}
