use super::*;
use std::future::Future;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Wake, Waker};

#[derive(Default)]
struct WakeCount(AtomicUsize);
impl Wake for WakeCount {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

#[tokio::test]
async fn page_retirement_wakes_every_registered_lifetime_waiter() {
    // Arrange
    let (registry, _) = registry(1, 1);
    let mut f = selected(registry.clone()).await;
    let page = registry.page(ObjectId::new(3)).unwrap().unwrap();
    let mut first = std::pin::pin!(page.closed());
    let mut second = std::pin::pin!(page.closed());
    let first_wakes = Arc::new(WakeCount::default());
    let second_wakes = Arc::new(WakeCount::default());
    let first_waker = Waker::from(first_wakes.clone());
    let second_waker = Waker::from(second_wakes.clone());
    assert!(
        first
            .as_mut()
            .poll(&mut Context::from_waker(&first_waker))
            .is_pending()
    );
    assert!(
        second
            .as_mut()
            .poll(&mut Context::from_waker(&second_waker))
            .is_pending()
    );
    // Act
    close_first(&mut f).await;
    // Assert
    assert!(first_wakes.0.load(Ordering::Relaxed) > 0);
    assert!(second_wakes.0.load(Ordering::Relaxed) > 0);
    assert!(
        first
            .as_mut()
            .poll(&mut Context::from_waker(&first_waker))
            .is_ready()
    );
    assert!(
        second
            .as_mut()
            .poll(&mut Context::from_waker(&second_waker))
            .is_ready()
    );
}

#[tokio::test]
async fn retirement_before_first_poll_is_not_lost() {
    // Arrange
    let (registry, _) = registry(1, 1);
    let mut f = selected(registry.clone()).await;
    let page = registry.page(ObjectId::new(3)).unwrap().unwrap();
    let mut closed = std::pin::pin!(page.closed());
    // Act
    close_first(&mut f).await;
    // Assert
    assert!(
        closed
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_ready()
    );
}

#[tokio::test]
async fn another_pages_retirement_does_not_end_the_live_page_wait() {
    // Arrange
    let (registry, _) = registry(2, 2);
    let mut f = selected(registry.clone()).await;
    add_second_page(&f);
    f.driver.initialize_snapshot(&f.runtime, 0).await.unwrap();
    let page = registry.page(ObjectId::new(12)).unwrap().unwrap();
    let mut closed = std::pin::pin!(page.closed());
    let mut cx = Context::from_waker(Waker::noop());
    assert!(closed.as_mut().poll(&mut cx).is_pending());
    // Act
    close_first(&mut f).await;
    // Assert
    assert!(closed.as_mut().poll(&mut cx).is_pending());
    assert!(!page.is_sealed());
}

#[tokio::test]
async fn registry_sealing_wakes_escaped_page_waiters() {
    // Arrange
    let (registry, _) = registry(1, 1);
    let _f = selected(registry.clone()).await;
    let page = registry.page(ObjectId::new(3)).unwrap().unwrap();
    let mut closed = std::pin::pin!(page.closed());
    let mut cx = Context::from_waker(Waker::noop());
    assert!(closed.as_mut().poll(&mut cx).is_pending());
    // Act
    registry.seal();
    // Assert
    assert!(closed.as_mut().poll(&mut cx).is_ready());
    assert!(page.is_sealed());
}

#[tokio::test]
async fn cancelled_wait_can_be_replaced_without_retaining_an_owner_lease() {
    // Arrange
    let (registry, ledger) = registry(1, 1);
    let mut f = selected(registry.clone()).await;
    let page = registry.page(ObjectId::new(3)).unwrap().unwrap();
    let leases = ledger.metrics().unwrap().active_leases;
    {
        let mut cancelled = std::pin::pin!(page.closed());
        assert!(
            cancelled
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
    }
    let replacement = page.closed();
    assert!(std::mem::size_of_val(&replacement) <= 256);
    assert_eq!(ledger.metrics().unwrap().active_leases, leases);
    // Act
    close_first(&mut f).await;
    replacement.await;
    drop(f.driver);
    drop(registry);
    drop(page);
    // Assert
    assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
}

#[tokio::test]
async fn document_lifecycle_progress_is_not_page_closure() {
    // Arrange
    let (registry, _) = registry(1, 1);
    let mut f = selected(registry.clone()).await;
    let page = registry.page(ObjectId::new(3)).unwrap().unwrap();
    let mut closed = std::pin::pin!(page.closed());
    let mut cx = Context::from_waker(Waker::noop());
    assert!(closed.as_mut().poll(&mut cx).is_pending());
    f.sender
        .send(CdpEvent::new(
            "Page.lifecycleEvent".into(),
            serde_json::json!({
                "frameId":"target-a", "loaderId":"loader-target-a", "name":"load", "timestamp":0
            }),
            Some("session-a".into()),
        ))
        .unwrap();
    // Act
    f.driver.receive_once(&f.runtime).await.unwrap();
    // Assert
    assert!(closed.as_mut().poll(&mut cx).is_pending());
    assert!(page.snapshot(ObjectId::new(4)).unwrap().unwrap().load);
}
