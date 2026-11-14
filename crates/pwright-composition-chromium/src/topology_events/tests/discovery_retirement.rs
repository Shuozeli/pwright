use super::*;
use crate::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer,
};
use std::error::Error;

#[tokio::test]
async fn playwright_fallback_accepts_contexts_absent_from_the_discovery_snapshot() {
    // Arrange: both native Context IDs are absent from getBrowserContexts.
    // The selected fallback must not guess that only the first is the default.
    let mut f = discovery_fixture(8);
    f.driver = f.driver.with_playwright_context_fallback();
    let pages = [
        ("default-page", "default-session", "native-default"),
        ("late-page-a", "late-session-a", "late-context-a"),
    ];
    for (target, session, context) in pages {
        f.factory.set_frame_tree(
            target,
            frame_tree(serde_json::json!({ "frame": frame_value(target, None) })),
        );
        f.sender
            .send(attached_page_event(target, session, Some(context), None))
            .unwrap();
    }
    // Act
    f.driver.initialize_snapshot(&f.runtime, 8).await.unwrap();
    // Assert: borrowed Pages are usable through the default view without
    // registering ownership of the native Contexts or detaching their sessions.
    let topology = f.topology.lock().unwrap();
    let topology = topology.topology();
    let default = topology.default_context_route().unwrap().object_id();
    for (target, _, context) in pages {
        assert!(topology.page_object_id(target).is_some());
        assert!(topology.context_object_id(context).is_none());
        assert!(topology.page_session_routes().any(
            |route| route.target_id() == target && route.browser_context_object_id() == default
        ));
    }
    assert!(
        !f.browser
            .method_names()
            .contains(&"Target.detachFromTarget".to_owned())
    );
    assert_eq!(
        f.browser
            .method_names()
            .iter()
            .filter(|method| method.as_str() == "Target.getBrowserContexts")
            .count(),
        1
    );
}

pub(super) struct RetirementInitializer {
    pub(super) target: &'static str,
    error: Mutex<Option<Box<dyn Error + Send + Sync>>>,
    pub(super) pages: Mutex<Vec<ObjectId>>,
}
impl RetirementInitializer {
    fn new(error: Box<dyn Error + Send + Sync>) -> Self {
        Self {
            target: "page-gone",
            error: Mutex::new(Some(error)),
            pages: Mutex::new(Vec::new()),
        }
    }
    pub(super) fn missing() -> Self {
        Self::new(
            CdpError::Protocol {
                code: -32001,
                message: "Session with given id not found.".into(),
            }
            .into(),
        )
    }
}
impl ChromiumSessionInitializer for RetirementInitializer {
    fn initialize<'a>(
        &'a self,
        _: &'a str,
        _: &'a str,
        _: CdpContextOwnership,
        _: bool,
    ) -> ChromiumSessionInitializationFuture<'a> {
        panic!("scoped identity is required")
    }
    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            self.pages.lock().unwrap().push(target.page);
            if target.target_id == self.target {
                return Err(self
                    .error
                    .lock()
                    .unwrap()
                    .take()
                    .expect("one failed attempt"));
            }
            Ok(())
        })
    }
}

#[tokio::test]
async fn missing_initial_route_retires_runtime_objects_without_retrying_it() {
    // Arrange
    let mut f = fixture(8);
    let initializer = Arc::new(RetirementInitializer {
        target: "target-a",
        ..RetirementInitializer::missing()
    });
    f.driver = f.driver.with_session_initializer(initializer.clone());
    // Act
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    // Assert
    assert!(
        f.topology
            .lock()
            .unwrap()
            .topology()
            .page_object_id("target-a")
            .is_none()
    );
    assert_eq!(f.runtime.live_object_count(), Ok(2));
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    assert_eq!(initializer.pages.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn initial_route_timeout_is_not_a_retirement_receipt() {
    // Arrange
    let mut f = fixture(8);
    f.driver = f
        .driver
        .with_session_initializer(Arc::new(RetirementInitializer {
            target: "target-a",
            ..RetirementInitializer::new(CdpError::Timeout.into())
        }));
    // Act
    let result = f.driver.initialize_snapshot(&f.runtime, 0).await;
    // Assert
    assert!(matches!(
        result,
        Err(ChromiumTopologyEventDriverError::SessionInitialization(_))
    ));
    assert!(
        f.topology
            .lock()
            .unwrap()
            .topology()
            .page_object_id("target-a")
            .is_some()
    );
    assert_eq!(f.runtime.live_object_count(), Ok(4));
}

#[tokio::test]
async fn missing_oopif_initialization_retires_only_the_child_session() {
    // Arrange
    let mut f = fixture(8);
    f.driver = f
        .driver
        .with_session_initializer(Arc::new(RetirementInitializer {
            target: "frame-oopif",
            ..RetirementInitializer::missing()
        }));
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    f.factory.set_frame_tree(
        "frame-oopif",
        frame_tree(serde_json::json!({
            "frame": frame_value("frame-oopif", Some("target-a"))
        })),
    );
    f.sender
        .send(CdpEvent::new(
            "Target.attachedToTarget".into(),
            serde_json::json!({
                "sessionId": "session-oopif",
                "targetInfo": { "targetId": "frame-oopif", "type": "iframe", "title": "",
                    "url": "https://other.example", "attached": true, "canAccessOpener": false,
                    "parentFrameId": "target-a" },
                "waitingForDebugger": true
            }),
            Some("session-a".into()),
        ))
        .unwrap();
    // Act
    f.driver.receive_once(&f.runtime).await.unwrap();
    // Assert
    let topology = f.topology.lock().unwrap();
    assert!(
        topology
            .topology()
            .target_session_route("session-oopif")
            .is_none()
    );
    assert!(
        topology
            .topology()
            .target_session_route("session-a")
            .is_some()
    );
    assert_eq!(
        topology.topology().page_object_id("target-a"),
        Some(ObjectId::new(3))
    );
}

#[tokio::test]
async fn missing_session_during_discovery_keeps_page_unpublished_and_connection_usable() {
    // Arrange
    let mut f = discovery_fixture(8);
    f.driver = f
        .driver
        .with_session_initializer(Arc::new(RetirementInitializer::missing()));
    f.sender
        .send(attached_page_event("page-gone", "session-gone", None, None))
        .unwrap();
    f.sender
        .send(attached_page_event("page-live", "session-live", None, None))
        .unwrap();
    // Act
    f.driver.initialize_snapshot(&f.runtime, 8).await.unwrap();
    // Assert
    let topology = f.topology.lock().unwrap();
    assert!(topology.topology().page_object_id("page-gone").is_none());
    assert!(
        topology
            .topology()
            .target_session_route("session-gone")
            .is_none()
    );
    assert!(topology.topology().page_object_id("page-live").is_some());
    assert!(
        !f.factory
            .client("page-gone")
            .method_names()
            .iter()
            .any(|method| method == "Runtime.runIfWaitingForDebugger")
    );
}

#[tokio::test]
async fn typed_native_detach_abandons_unpublished_page_without_sealing_the_driver() {
    // Arrange
    let mut f = discovery_fixture(8);
    f.driver = f
        .driver
        .with_session_initializer(Arc::new(RetirementInitializer::new(
            CdpError::SessionDetached.into(),
        )));
    f.sender
        .send(attached_page_event("page-gone", "session-gone", None, None))
        .unwrap();
    f.sender
        .send(attached_page_event("page-live", "session-live", None, None))
        .unwrap();
    // Act
    f.driver.initialize_snapshot(&f.runtime, 8).await.unwrap();
    // Assert
    let topology = f.topology.lock().unwrap();
    assert!(topology.topology().page_object_id("page-gone").is_none());
    assert!(topology.topology().page_object_id("page-live").is_some());
    assert!(
        !f.browser
            .method_names()
            .contains(&"Target.detachFromTarget".to_owned())
    );
}

#[tokio::test]
async fn recursive_missing_session_waits_for_ordered_detach_after_reporting_page_ready() {
    // Arrange
    let mut f = discovery_fixture_with_startup(8, false, true);
    let initializer = Arc::new(RetirementInitializer::missing());
    f.driver = f.driver.with_session_initializer(initializer.clone());
    f.sender
        .send(attached_page_event("page-gone", "session-gone", None, None))
        .unwrap();

    // Act
    let (attached, initialization_calls, initialized_before_detach, state_before_detach) = {
        let attached = f.driver.receive_once(&f.runtime).await.unwrap();
        let initialization_calls = initializer.pages.lock().unwrap().len();
        let initialized_before_detach = f.driver.initialized_session_count();
        let state_before_detach = f
            .topology
            .lock()
            .unwrap()
            .topology()
            .page_state("page-gone")
            .cloned();
        f.sender
            .send(CdpEvent::new(
                "Target.detachedFromTarget".into(),
                serde_json::json!({
                    "sessionId": "session-gone",
                    "targetId": "page-gone"
                }),
                None,
            ))
            .unwrap();
        f.driver.receive_once(&f.runtime).await.unwrap();
        (
            attached,
            initialization_calls,
            initialized_before_detach,
            state_before_detach,
        )
    };

    // Assert
    assert!(matches!(
        attached,
        ChromiumTopologyEventDriverOutcome::Updated(_)
    ));
    assert_eq!(initialization_calls, 1);
    assert_eq!(initialized_before_detach, 1);
    assert_eq!(
        state_before_detach,
        Some(pwright_backend_cdp::CdpPageState::Ready)
    );
    assert!(
        f.topology
            .lock()
            .unwrap()
            .topology()
            .page_object_id("page-gone")
            .is_none()
    );
    assert_eq!(f.driver.initialized_session_count(), 0);
}

#[test]
fn compound_missing_session_receipts_are_retirement_only_when_both_sides_are_typed() {
    // Arrange
    let missing = || CdpError::Protocol {
        code: -32001,
        message: "Session with given id not found.".into(),
    };
    let both_missing = ChromiumTopologyEventDriverError::Cdp(CdpError::Compound {
        source: Box::new(CdpError::SessionDetached),
        system: Box::new(missing()),
    });
    let mixed = ChromiumTopologyEventDriverError::Cdp(CdpError::Compound {
        source: Box::new(missing()),
        system: Box::new(CdpError::Closed),
    });

    // Act
    let accepted = super::super::discovered_session::missing_native_session(&both_missing);
    let rejected = super::super::discovered_session::missing_native_session(&mixed);

    // Assert
    assert!(accepted);
    assert!(!rejected);
}

#[tokio::test]
async fn unrelated_initialization_failures_are_not_treated_as_session_retirement() {
    // Arrange
    let message = "Session with given id not found.";
    let cases: Vec<Box<dyn Error + Send + Sync>> = vec![
        CdpError::Protocol {
            code: -32000,
            message: message.into(),
        }
        .into(),
        CdpError::Protocol {
            code: -32001,
            message: "another protocol failure".into(),
        }
        .into(),
        CdpError::Closed.into(),
        CdpError::ChannelDropped.into(),
        CdpError::Timeout.into(),
        CdpError::Other(message.into()).into(),
        CdpError::Other("Native session detached".into()).into(),
        CdpError::Compound {
            source: Box::new(CdpError::Protocol {
                code: -32001,
                message: message.into(),
            }),
            system: Box::new(CdpError::Closed),
        }
        .into(),
        std::io::Error::other("CDP error -32001: Session with given id not found.").into(),
    ];
    for error in cases {
        let expected = error.to_string();
        let mut f = discovery_fixture(8);
        f.driver = f
            .driver
            .with_session_initializer(Arc::new(RetirementInitializer::new(error)));
        f.sender
            .send(attached_page_event("page-gone", "session-gone", None, None))
            .unwrap();
        // Act
        let result = f.driver.receive_once(&f.runtime).await;
        // Assert
        let Err(ChromiumTopologyEventDriverError::SessionInitialization(actual)) = result else {
            panic!("initialization failure was not preserved: {expected}");
        };
        assert_eq!(actual.to_string(), expected);
        assert!(
            f.topology
                .lock()
                .unwrap()
                .topology()
                .page_object_id("page-gone")
                .is_none()
        );
    }
}
