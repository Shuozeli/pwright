//! Native transaction receipts, never foreign callback dispatch or retention.
use super::*;
use crate::{
    ChromiumCommittedFrameOwnership, ChromiumFrameOwnershipView, ChromiumFrameUrlObserver,
};
use pwright_backend_cdp::{CdpTopologyEffect, CdpTopologyUpdate};

#[derive(Debug)]
struct Receipt {
    effects: Vec<CdpTopologyEffect>,
    source_address: usize,
    pages: Vec<ObjectId>,
}

#[derive(Default)]
struct Observer {
    receipts: Mutex<Vec<Receipt>>,
    refreshes: AtomicUsize,
    reject_ready: bool,
}

impl ChromiumFrameUrlObserver for Observer {
    fn update(
        &self,
        _: ObjectId,
        _: &str,
        _: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn retain_live(
        &self,
        _: &dyn Fn(ObjectId) -> bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        Ok(())
    }

    fn reconcile_ownership(
        &self,
        _: ChromiumFrameOwnershipView<'_>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.refreshes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn reconcile_committed(
        &self,
        publication: ChromiumCommittedFrameOwnership<'_>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        assert!(publication.effects().len() <= 32, "bounded test receipt");
        let receipt = Receipt {
            effects: publication.effects().to_vec(),
            source_address: publication.effects().as_ptr().addr(),
            pages: publication
                .ownership()
                .collection_pages()
                .map(|page| page.id)
                .collect(),
        };
        let reject = self.reject_ready
            && receipt
                .effects
                .iter()
                .any(|effect| matches!(effect, CdpTopologyEffect::PageReady { .. }));
        let mut receipts = self.receipts.lock().unwrap();
        assert!(receipts.len() < 32, "bounded test recorder");
        receipts.push(receipt);
        if reject {
            return Err(std::io::Error::other("committed observer rejected").into());
        }
        Ok(())
    }
}

async fn attach(
    fixture: &mut DriverFixture,
    target: &str,
    opener: Option<&str>,
) -> CdpTopologyUpdate {
    fixture
        .sender
        .send(attached_page_event(
            target,
            &format!("session-{target}"),
            None,
            opener,
        ))
        .unwrap();
    let outcome = fixture.driver.receive_once(&fixture.runtime).await.unwrap();
    let ChromiumTopologyEventDriverOutcome::Updated(update) = outcome else {
        panic!("expected committed update");
    };
    update
}

#[tokio::test]
async fn popup_receipt_preserves_committed_effects_with_matching_membership() {
    // Arrange
    let mut fixture = discovery_fixture(8);
    let observer = Arc::new(Observer::default());
    fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
    attach(&mut fixture, "opener", None).await;
    // Act
    let update = attach(&mut fixture, "popup", Some("opener")).await;
    // Assert
    let receipts = observer.receipts.lock().unwrap();
    let receipt = receipts.last().unwrap();
    assert_eq!(receipt.effects, update.effects());
    assert_eq!(receipt.pages, [ObjectId::new(3), ObjectId::new(5)]);
    let ordered: Vec<_> = receipt
        .effects
        .iter()
        .filter(|effect| {
            matches!(
                effect,
                CdpTopologyEffect::PageReady { .. } | CdpTopologyEffect::PopupLinked { .. }
            )
        })
        .cloned()
        .collect();
    assert_eq!(
        ordered,
        [
            CdpTopologyEffect::PageReady {
                page: ObjectId::new(5),
                context: ObjectId::new(2)
            },
            CdpTopologyEffect::PopupLinked {
                opener: ObjectId::new(3),
                popup: ObjectId::new(5)
            }
        ]
    );
}

#[tokio::test]
async fn deferred_popup_link_is_not_rewritten_as_a_second_popup_readiness() {
    // Arrange
    let mut fixture = discovery_fixture(8);
    let observer = Arc::new(Observer::default());
    fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
    attach(&mut fixture, "popup", Some("opener")).await;
    // Act
    let update = attach(&mut fixture, "opener", None).await;
    // Assert
    let ordered: Vec<_> = update
        .effects()
        .iter()
        .filter(|effect| {
            matches!(
                effect,
                CdpTopologyEffect::PageReady { .. } | CdpTopologyEffect::PopupLinked { .. }
            )
        })
        .cloned()
        .collect();
    assert_eq!(
        ordered,
        [
            CdpTopologyEffect::PageReady {
                page: ObjectId::new(5),
                context: ObjectId::new(2)
            },
            CdpTopologyEffect::PopupLinked {
                opener: ObjectId::new(5),
                popup: ObjectId::new(3)
            }
        ]
    );
    let receipts = observer.receipts.lock().unwrap();
    assert_eq!(receipts.last().unwrap().effects, update.effects());
    let popup_ready_count = receipts
        .iter()
        .flat_map(|receipt| &receipt.effects)
        .filter(|effect| {
            matches!(effect, CdpTopologyEffect::PageReady { page, .. } if *page == ObjectId::new(3))
        })
        .count();
    assert_eq!(popup_ready_count, 1);
}

#[tokio::test]
async fn disposal_receipt_keeps_native_order_and_the_post_commit_view() {
    // Arrange
    let mut fixture = discovery_fixture(8);
    let observer = Arc::new(Observer::default());
    fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
    attach(&mut fixture, "page", None).await;
    fixture
        .sender
        .send(CdpEvent::new(
            "Target.detachedFromTarget".into(),
            serde_json::json!({"targetId":"page", "sessionId":"session-page"}),
            None,
        ))
        .unwrap();
    // Act
    let outcome = fixture.driver.receive_once(&fixture.runtime).await.unwrap();
    // Assert
    let ChromiumTopologyEventDriverOutcome::Updated(update) = outcome else {
        panic!("expected disposal update");
    };
    let receipts = observer.receipts.lock().unwrap();
    let receipt = receipts.last().unwrap();
    assert_eq!(receipt.effects, update.effects());
    assert!(!receipt.effects.is_empty());
    assert!(receipt.pages.is_empty());
    assert!(!fixture.runtime.is_object_live(ObjectId::new(3)).unwrap());
}

#[tokio::test]
async fn snapshot_refresh_does_not_replay_committed_effects() {
    // Arrange
    let mut fixture = discovery_fixture(8);
    let observer = Arc::new(Observer::default());
    fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
    attach(&mut fixture, "page", None).await;
    let count = observer.receipts.lock().unwrap().len();
    let refreshes = observer.refreshes.load(Ordering::SeqCst);
    // Act
    fixture.driver.refresh_frame_ownership().unwrap();
    // Assert
    assert_eq!(observer.receipts.lock().unwrap().len(), count);
    assert_eq!(observer.refreshes.load(Ordering::SeqCst), refreshes + 1);
}

#[tokio::test]
async fn initialization_failure_produces_no_ready_or_popup_receipt() {
    // Arrange
    let mut fixture = discovery_fixture(8);
    let observer = Arc::new(Observer::default());
    let initializer = scoped_initializer(&fixture, true);
    fixture.driver = fixture
        .driver
        .with_frame_url_observer(observer.clone())
        .with_session_initializer(initializer);
    fixture
        .sender
        .send(attached_page_event(
            "page",
            "session-page",
            None,
            Some("opener"),
        ))
        .unwrap();
    // Act
    let result = fixture.driver.receive_once(&fixture.runtime).await;
    // Assert
    assert!(matches!(
        result,
        Err(ChromiumTopologyEventDriverError::SessionInitialization(_))
    ));
    assert!(
        observer
            .receipts
            .lock()
            .unwrap()
            .iter()
            .flat_map(|receipt| &receipt.effects)
            .all(|effect| !matches!(
                effect,
                CdpTopologyEffect::PageReady { .. } | CdpTopologyEffect::PopupLinked { .. }
            ))
    );
    assert!(
        fixture
            .topology
            .lock()
            .unwrap()
            .topology()
            .page_object_id("page")
            .is_none()
    );
}

#[tokio::test]
async fn observer_failure_propagates_without_claiming_native_commit_rollback() {
    // Arrange
    let mut fixture = discovery_fixture(8);
    let observer = Arc::new(Observer {
        reject_ready: true,
        ..Observer::default()
    });
    fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
    fixture
        .sender
        .send(attached_page_event("page", "session-page", None, None))
        .unwrap();
    // Act
    let result = fixture.driver.receive_once(&fixture.runtime).await;
    // Assert
    assert!(matches!(
        result,
        Err(ChromiumTopologyEventDriverError::FrameUrlObserver(_))
    ));
    let receipts = observer.receipts.lock().unwrap();
    assert!(
        receipts
            .last()
            .unwrap()
            .effects
            .iter()
            .any(|effect| matches!(effect, CdpTopologyEffect::PageReady { .. }))
    );
    assert!(
        fixture
            .topology
            .lock()
            .unwrap()
            .topology()
            .page_object_id("page")
            .is_some()
    );
}

#[test]
fn legacy_observer_receives_one_snapshot_callback_for_a_committed_update() {
    struct Legacy(AtomicUsize);
    impl ChromiumFrameUrlObserver for Legacy {
        fn update(
            &self,
            _: ObjectId,
            _: &str,
            _: Option<&str>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
        fn retain_live(
            &self,
            _: &dyn Fn(ObjectId) -> bool,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
        fn reconcile_ownership(
            &self,
            view: ChromiumFrameOwnershipView<'_>,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            assert_eq!(view.collection_pages().count(), 1);
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    // Arrange
    let mut fixture = fixture(8);
    let observer = Arc::new(Legacy(AtomicUsize::new(0)));
    fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
    // Act
    fixture
        .driver
        .transact_update(&fixture.runtime, |topology| {
            topology.mark_page_ready(ConnectionEpoch::new(1), "target-a")
        })
        .unwrap();
    // Assert
    assert_eq!(observer.0.load(Ordering::SeqCst), 1);
}

#[test]
fn idempotent_ready_transaction_does_not_invent_another_ready_notice() {
    // Arrange
    let mut fixture = fixture(8);
    let observer = Arc::new(Observer::default());
    fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
    fixture
        .driver
        .transact_update(&fixture.runtime, |topology| {
            topology.mark_page_ready(ConnectionEpoch::new(1), "target-a")
        })
        .unwrap();
    // Act
    let update = fixture
        .driver
        .transact_update(&fixture.runtime, |topology| {
            topology.mark_page_ready(ConnectionEpoch::new(1), "target-a")
        })
        .unwrap();
    // Assert
    assert!(update.effects().is_empty());
    let receipts = observer.receipts.lock().unwrap();
    assert_eq!(receipts.len(), 2);
    assert!(receipts[1].effects.is_empty());
    assert_eq!(
        receipts
            .iter()
            .flat_map(|receipt| &receipt.effects)
            .filter(|effect| matches!(effect, CdpTopologyEffect::PageReady { .. }))
            .count(),
        1
    );
}

#[test]
fn effect_slice_is_borrowed_before_the_outer_driver_aggregates_transactions() {
    // Arrange
    let mut fixture = fixture(8);
    let observer = Arc::new(Observer::default());
    fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
    // Act: direct transaction output, before receive_once combines its batches.
    let update = fixture
        .driver
        .transact_update(&fixture.runtime, |topology| {
            topology.mark_page_ready(ConnectionEpoch::new(1), "target-a")
        })
        .unwrap();
    // Assert
    let receipts = observer.receipts.lock().unwrap();
    assert_eq!(receipts.len(), 1);
    assert_eq!(receipts[0].source_address, update.effects().as_ptr().addr());
    assert_eq!(receipts[0].effects, update.effects());
}

#[tokio::test]
async fn pre_registered_page_readiness_is_observed_after_synchronized_initialization() {
    // Arrange: the route already exists, as after explicit native Page creation.
    let mut fixture = fixture(8);
    let observer = Arc::new(Observer::default());
    fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
    // Act
    fixture
        .driver
        .initialize_snapshot(&fixture.runtime, 0)
        .await
        .unwrap();
    // Assert
    let receipts = observer.receipts.lock().unwrap();
    let ready: Vec<_> = receipts
        .iter()
        .filter(|receipt| {
            receipt.effects.iter().any(|effect| {
                matches!(effect, CdpTopologyEffect::PageReady { page, .. } if *page == ObjectId::new(3))
            })
        })
        .collect();
    assert_eq!(ready.len(), 1);
    assert_eq!(ready[0].pages, [ObjectId::new(3)]);
    assert!(
        fixture
            .factory
            .client("target-a")
            .method_names()
            .iter()
            .any(|method| method == "Page.enable")
    );
}

#[tokio::test]
async fn ignored_destroy_notification_does_not_synthesize_a_disposal_transaction() {
    // Arrange: this driver retires Page ownership from exact session detach.
    let mut fixture = discovery_fixture(8);
    let observer = Arc::new(Observer::default());
    fixture.driver = fixture.driver.with_frame_url_observer(observer.clone());
    attach(&mut fixture, "page", None).await;
    let count = observer.receipts.lock().unwrap().len();
    fixture
        .sender
        .send(CdpEvent::new(
            "Target.targetDestroyed".into(),
            serde_json::json!({"targetId":"page"}),
            None,
        ))
        .unwrap();
    // Act
    let result = fixture.driver.receive_once(&fixture.runtime).await.unwrap();
    // Assert
    assert!(matches!(
        result,
        ChromiumTopologyEventDriverOutcome::Ignored
    ));
    assert_eq!(observer.receipts.lock().unwrap().len(), count);
}
