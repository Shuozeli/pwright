//! Native Page-close confirmation must retire retained world receipts even
//! when no topology driver has consumed a targetDestroyed event.
use super::*;
use crate::{ChromiumFrameWorldAccess, SharedChromiumTopology};
use pwright_backend_cdp_page_lifecycle::CdpPageClosePoll;

struct CloseFixture {
    runtime: PageCloseRuntime,
    topology: SharedChromiumTopology,
    browser: Arc<FakeCdpClient>,
    access: ChromiumFrameWorldAccess,
}

fn close_fixture() -> CloseFixture {
    let profile = ChromiumFrameWorldAccessProfile::new(
        ResourceScope::extension(98, 3),
        NonZeroUsize::new(2).unwrap(),
    )
    .unwrap()
    .with_native_retirement()
    .unwrap();
    let mut fixture = fixture_with_world_access(true, Some(profile));
    for (id, unique, kind) in [
        (11, "main-a", CdpWorldKind::Main),
        (12, "utility-a", CdpWorldKind::Utility),
    ] {
        attach_world(&mut fixture.topology, id, unique, "frame-a", kind);
    }
    fixture.services.synchronize(&fixture.topology).unwrap();
    let access =
        ChromiumFrameWorldAccess::new(&fixture.services, profile, fixture.prepared.resources())
            .unwrap();
    let runtime = runtime(&fixture, NoDeadlineScheduler);
    while runtime.pop_control_message().unwrap().is_some() {}
    CloseFixture {
        runtime,
        topology: Arc::new(Mutex::new(fixture.topology)),
        browser: Arc::new(FakeCdpClient::from_html("")),
        access,
    }
}

impl CloseFixture {
    fn service(&self, poll: Arc<dyn CdpPageClosePoll>) -> ChromiumPageCloseCommandService {
        ChromiumPageCloseCommandService::new(
            self.browser.clone(),
            self.topology.clone(),
            self.runtime.service().clone(),
            self.runtime.lifecycle_handle(),
            CdpPageCloseLimits::new(128, 2),
            poll,
        )
    }

    fn add_other_page(&self) {
        let mut topology = self.topology.lock().unwrap();
        let epoch = ConnectionEpoch::new(1);
        topology
            .attach_page(
                epoch,
                CdpPageTarget::new("target-b", "session-b", "context-a"),
                CdpPageObjectIds::new(ObjectId::new(6), ObjectId::new(7)),
            )
            .unwrap();
        topology
            .attach_frame(
                epoch,
                CdpFrameAttachment::main("target-b", "frame-b"),
                ObjectId::new(7),
            )
            .unwrap();
        for (id, unique, kind) in [
            (21, "main-b", CdpWorldKind::Main),
            (22, "utility-b", CdpWorldKind::Utility),
        ] {
            topology
                .attach_world(
                    epoch,
                    CdpWorldAttachment::new("session-b", id, unique, "frame-b", kind),
                )
                .unwrap();
        }
        self.runtime.service().synchronize(&topology).unwrap();
        for (object, parent, kind) in [(6, 2, PAGE_OBJECT_KIND.tag), (7, 6, FRAME_OBJECT_KIND.tag)]
        {
            self.runtime
                .register_object(ObjectId::new(object), Some(ObjectId::new(parent)), kind)
                .unwrap();
        }
        while self.runtime.pop_control_message().unwrap().is_some() {}
    }
}

fn close_command() -> Command<PageClose> {
    Command::new(
        CallId::new(166),
        ObjectId::new(3),
        CallMetadata::default(),
        (),
    )
}

fn target(target_id: &str) -> pwright_cdp::domains::target::TargetInfo {
    pwright_cdp::domains::target::TargetInfo {
        target_id: target_id.into(),
        target_type: "page".into(),
        title: String::new(),
        url: "about:blank".into(),
        attached: true,
    }
}

#[derive(Default)]
struct ConfirmationGate(tokio::sync::Notify);

impl CdpPageClosePoll for ConfirmationGate {
    fn wait(&self, _completed_checks: usize) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(self.0.notified())
    }
}

#[tokio::test]
async fn confirmed_absence_retires_retained_worlds_without_a_target_destroyed_event() {
    // Arrange
    let fixture = close_fixture();
    let snapshot = fixture.access.snapshot(ObjectId::new(4)).await.unwrap();
    let lifetime = snapshot.native_lifetime().unwrap();
    let service = fixture.service(Arc::new(ImmediatePageClosePoll));
    // Act
    let result = service.call(close_command()).await.unwrap().into_outcome();
    // Assert
    assert_eq!(result, Ok(()));
    assert!(lifetime.both_worlds_destroyed());
    assert!(lifetime.native_objects_retired());
    assert!(matches!(
        snapshot.is_current(),
        Err(crate::FrameWorldAccessError::Topology(
            CdpPageRouteError::WorldTargetClosed { .. }
        ))
    ));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(
        fixture.browser.method_names(),
        ["Target.closeTarget", "Target.getTargets"]
    );
}

#[tokio::test]
async fn confirmed_page_close_does_not_detach_an_already_destroyed_session() {
    // Arrange
    let fixture = close_fixture();
    let snapshot = fixture.access.snapshot(ObjectId::new(4)).await.unwrap();
    let service = fixture.service(Arc::new(ImmediatePageClosePoll));
    service
        .call(close_command())
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    fixture
        .browser
        .set_target_detach_errors([Some("No session with given id".into())]);
    // Act
    let retired = fixture
        .access
        .retire_owned_native_sessions(fixture.browser.as_ref())
        .await;
    // Assert
    assert!(retired.is_ok());
    assert!(snapshot.native_lifetime().unwrap().native_objects_retired());
    assert_eq!(
        fixture.browser.method_names(),
        ["Target.closeTarget", "Target.getTargets"]
    );
}

#[tokio::test]
async fn close_acknowledgement_does_not_retire_worlds_before_target_absence_confirmation() {
    // Arrange
    let fixture = close_fixture();
    let snapshot = fixture.access.snapshot(ObjectId::new(4)).await.unwrap();
    let lifetime = snapshot.native_lifetime().unwrap();
    fixture
        .browser
        .set_target_responses([vec![target("target-a")], vec![]]);
    let gate = Arc::new(ConfirmationGate::default());
    let service = fixture.service(gate.clone());
    let mut closing = service.call(close_command());
    assert!(
        closing
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert!(!lifetime.native_objects_retired());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert_eq!(
        fixture.browser.method_names(),
        ["Target.closeTarget", "Target.getTargets"]
    );
    // Act
    gate.0.notify_waiters();
    let result = closing.await.unwrap().into_outcome();
    // Assert
    assert_eq!(result, Ok(()));
    assert!(lifetime.both_worlds_destroyed());
    assert_eq!(
        fixture.browser.method_names(),
        [
            "Target.closeTarget",
            "Target.getTargets",
            "Target.getTargets"
        ]
    );
}

#[tokio::test]
async fn failed_page_close_does_not_retire_retained_native_worlds() {
    // Arrange
    let fixture = close_fixture();
    let snapshot = fixture.access.snapshot(ObjectId::new(4)).await.unwrap();
    fixture
        .browser
        .set_target_close_error(Some("native close failed".into()));
    let service = fixture.service(Arc::new(ImmediatePageClosePoll));
    // Act
    let result = service.call(close_command()).await.unwrap().into_outcome();
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::Backend);
    assert!(!snapshot.native_lifetime().unwrap().native_objects_retired());
    assert!(snapshot.is_current().unwrap());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert_eq!(fixture.browser.method_names(), ["Target.closeTarget"]);
}

#[tokio::test]
async fn still_present_page_does_not_retire_retained_native_worlds() {
    // Arrange
    let fixture = close_fixture();
    let snapshot = fixture.access.snapshot(ObjectId::new(4)).await.unwrap();
    fixture
        .browser
        .set_target_responses([vec![target("target-a")], vec![target("target-a")]]);
    let service = fixture.service(Arc::new(ImmediatePageClosePoll));
    // Act
    let result = service.call(close_command()).await.unwrap().into_outcome();
    // Assert
    assert_eq!(result.unwrap_err().kind(), FailureKind::Backend);
    assert!(!snapshot.native_lifetime().unwrap().native_objects_retired());
    assert!(snapshot.is_current().unwrap());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert_eq!(
        fixture.browser.method_names(),
        [
            "Target.closeTarget",
            "Target.getTargets",
            "Target.getTargets"
        ]
    );
}

#[tokio::test]
async fn confirmed_page_close_does_not_retire_another_pages_native_worlds() {
    // Arrange
    let fixture = close_fixture();
    fixture.add_other_page();
    let first = fixture.access.snapshot(ObjectId::new(4)).await.unwrap();
    let other = fixture.access.snapshot(ObjectId::new(7)).await.unwrap();
    fixture
        .browser
        .set_target_responses([vec![target("target-b")]]);
    let service = fixture.service(Arc::new(ImmediatePageClosePoll));
    // Act
    let result = service.call(close_command()).await.unwrap().into_outcome();
    // Assert
    assert_eq!(result, Ok(()));
    assert!(first.native_lifetime().unwrap().both_worlds_destroyed());
    assert!(!other.native_lifetime().unwrap().native_objects_retired());
    assert!(other.is_current().unwrap());
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(6)), Ok(true));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(7)), Ok(true));
}
