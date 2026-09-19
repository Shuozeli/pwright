#![cfg(feature = "native-release")]

use pwright_cap_handle_lifecycle::HandleLifecycleErrorDetails;
use pwright_composition_chromium_handle_lifecycle::{
    ChromiumHandleDisposeCommandService, ChromiumHandleDisposer, ChromiumHandleRouteProfile,
    ChromiumHandleRouteSpec, ChromiumHandleRoutes,
    native_release::{
        NativeHandleRelease, NativeHandleReleaseCommit, NativeHandleReleaseReceipt,
        ROUTE_OWNER_BYTES,
    },
};
use pwright_core::{
    FailureKind, ObjectId, ObjectKindTag, OperationError, ProductionResourceLedger, ResourceClass,
    ResourceLimits, ResourceScope,
};
use pwright_runtime::{
    ControlledDispatcher, NoDeadlineScheduler,
    disposal_reservations::{DisposalReservationProfile, DisposalReservations},
};
use std::{
    future::{Future, poll_fn},
    num::NonZeroUsize,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

type Runtime = ControlledDispatcher<(), (), NoDeadlineScheduler>;
const HANDLE: ObjectId = ObjectId::new(5);
const FRAME: ObjectId = ObjectId::new(4);

struct Native {
    ready: AtomicBool,
    calls: AtomicUsize,
    fail: bool,
}
impl Native {
    fn new(ready: bool, fail: bool) -> Arc<Self> {
        Arc::new(Self {
            ready: AtomicBool::new(ready),
            calls: AtomicUsize::new(0),
            fail,
        })
    }
}
impl ChromiumHandleDisposer for Native {
    fn dispose(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<(), OperationError<HandleLifecycleErrorDetails>>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            // Tests drive polls explicitly; no hidden worker or clock.
            poll_fn(|_| {
                if self.ready.load(Ordering::SeqCst) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            })
            .await;
            if self.fail {
                Err(OperationError::new(
                    FailureKind::Backend,
                    "native release rejected",
                    None,
                ))
            } else {
                Ok(())
            }
        })
    }
}
struct Fixture {
    runtime: Runtime,
    routes: ChromiumHandleRoutes,
    pool: DisposalReservations,
    resources: ProductionResourceLedger,
    native: Arc<Native>,
    payload: Arc<u8>,
}
impl Fixture {
    fn new(native: Arc<Native>, deferred: bool) -> Self {
        let limits = pwright_runtime::RuntimeLimits::new(8, 2);
        let route_profile =
            ChromiumHandleRouteProfile::new(ResourceScope::extension(156, 1), 3, 64, 64);
        let mut selected = DisposalReservationProfile::new_in_process(
            NonZeroUsize::new(8).unwrap(),
            NonZeroUsize::new(2).unwrap(),
            ResourceScope::extension(156, 2),
        )
        .unwrap();
        if deferred {
            selected = selected
                .with_deferred_commit(NonZeroUsize::new(2).unwrap())
                .unwrap();
        }
        let plan = limits
            .resource_requirements()
            .combine_concurrent(route_profile.requirements())
            .combine_concurrent(selected.requirements());
        let mut budget = ResourceLimits::deny_all(plan.scopes, plan.leases);
        for class in ResourceClass::ALL {
            let amount = plan.class(class);
            budget = budget.with_class(class, amount.global, amount.per_scope);
        }
        let resources = ProductionResourceLedger::new(budget, plan).unwrap();
        let runtime = ControlledDispatcher::new((), (), NoDeadlineScheduler, limits)
            .with_production_resources(&resources)
            .unwrap();
        runtime
            .register_object(FRAME, None, ObjectKindTag::new(1))
            .unwrap();
        runtime
            .register_object(HANDLE, Some(FRAME), ObjectKindTag::new(2))
            .unwrap();
        let routes =
            ChromiumHandleRoutes::new_with_production_resources(route_profile, &resources).unwrap();
        let pool = selected
            .bind(&runtime.lifecycle_handle(), &resources)
            .unwrap();
        let result = Self {
            runtime,
            routes,
            pool,
            resources,
            native,
            payload: Arc::new(7),
        };
        result.publish();
        result
    }
    fn publish(&self) {
        self.routes
            .reserve()
            .unwrap()
            .publish(
                HANDLE,
                ChromiumHandleRouteSpec::new(
                    FRAME,
                    ObjectId::new(3),
                    12,
                    7,
                    Some(self.native.clone()),
                )
                .with_payload(self.payload.clone()),
            )
            .unwrap();
    }
    fn service(&self) -> ChromiumHandleDisposeCommandService {
        ChromiumHandleDisposeCommandService::new(
            self.runtime.lifecycle_handle(),
            self.routes.clone(),
        )
    }
    fn prepare(&self) -> NativeHandleRelease {
        self.service()
            .prepare_native_release(HANDLE, self.pool.reserve().unwrap())
            .unwrap()
            .unwrap()
    }
    fn retained(&self, class: ResourceClass) -> u64 {
        self.resources.metrics().unwrap().class(class).current
    }
    fn retire(&self) {
        self.runtime.dispose_object(HANDLE).unwrap();
        self.routes.retire_parent_disposals(&[HANDLE]).unwrap();
    }
}
fn poll_once<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}
fn commit(receipt: NativeHandleReleaseReceipt) {
    let NativeHandleReleaseCommit::Committed(mut committed) = receipt.commit_or_defer().unwrap()
    else {
        panic!("exact live route must commit");
    };
    committed.notify();
}

#[test]
fn native_wait_does_not_hold_the_runtime_lifecycle_gate() {
    // Arrange
    let fixture = Fixture::new(Native::new(false, false), true);
    let mut release = Box::pin(fixture.prepare().release());

    // Act
    let result = poll_once(release.as_mut());

    // Assert
    assert!(result.is_pending());
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
    let ordinary = fixture.runtime.prepare_object_disposal(FRAME).unwrap();
    assert_eq!(fixture.routes.metrics().unwrap().current, 1);
    assert_eq!(fixture.pool.pending(), 1);
    drop(ordinary);
    drop(release);
    assert_eq!(fixture.pool.pending(), 0);
    assert_eq!(fixture.retained(ResourceClass::RemoteHandle), 1);
}

#[tokio::test]
async fn native_acknowledgement_precedes_local_commit() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);

    // Act
    let receipt = fixture.prepare().release().await.unwrap();

    // Assert
    assert_eq!(receipt.object_id(), HANDLE);
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.runtime.is_object_live(HANDLE), Ok(true));
    assert!(fixture.routes.contains(HANDLE).unwrap());
    assert_eq!(fixture.pool.pending(), 1);
    commit(receipt);
    assert_eq!(fixture.runtime.is_object_live(HANDLE), Ok(false));
    assert!(!fixture.routes.contains(HANDLE).unwrap());
    assert_eq!(fixture.routes.metrics().unwrap().explicitly_disposed, 1);
    assert_eq!(fixture.retained(ResourceClass::RemoteHandle), 0);
    assert_eq!(fixture.pool.pending(), 0);
}

#[tokio::test]
async fn native_failure_preserves_local_ownership_and_releases_commit_admission() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, true), true);

    // Act
    let result = fixture.prepare().release().await;

    // Assert
    assert_eq!(result.err().unwrap().kind(), FailureKind::Backend);
    assert!(fixture.routes.contains(HANDLE).unwrap());
    assert_eq!(fixture.runtime.is_object_live(HANDLE), Ok(true));
    assert_eq!(fixture.pool.pending(), 0);
    assert_eq!(fixture.retained(ResourceClass::RemoteHandle), 1);
}

#[tokio::test]
async fn busy_commit_retains_receipt_and_never_repeats_native_release() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);
    let receipt = fixture.prepare().release().await.unwrap();
    let ordinary = fixture.runtime.prepare_object_disposal(FRAME).unwrap();
    let before = fixture.resources.metrics().unwrap();

    // Act
    let result = receipt.commit_or_defer().unwrap();

    // Assert
    let NativeHandleReleaseCommit::Deferred(receipt) = result else {
        panic!("must defer");
    };
    assert_eq!(fixture.resources.metrics().unwrap(), before);
    assert_eq!(fixture.pool.pending(), 1);
    drop(ordinary);
    commit(receipt);
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.pool.pending(), 0);
}

#[test]
fn parent_retirement_keeps_native_storage_admitted_until_the_pending_release_finishes() {
    // Arrange
    let fixture = Fixture::new(Native::new(false, false), true);
    let mut release = Box::pin(fixture.prepare().release());
    assert!(poll_once(release.as_mut()).is_pending());
    fixture.retire();
    assert_eq!(fixture.routes.metrics().unwrap().current, 0);
    assert_eq!(fixture.retained(ResourceClass::RemoteHandle), 1);
    fixture.native.ready.store(true, Ordering::SeqCst);

    // Act
    let Poll::Ready(Ok(receipt)) = poll_once(release.as_mut()) else {
        panic!("native ack should finish");
    };
    let result = receipt.commit_or_defer().unwrap();

    // Assert
    assert!(matches!(result, NativeHandleReleaseCommit::Superseded));
    assert_eq!(fixture.retained(ResourceClass::RemoteHandle), 0);
    assert_eq!(fixture.pool.pending(), 0);
    assert_eq!(fixture.routes.metrics().unwrap().parent_retired, 1);
}

#[tokio::test]
async fn late_receipt_cannot_retire_a_reused_id_with_the_same_payload_and_disposer() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);
    let receipt = fixture.prepare().release().await.unwrap();
    fixture.retire();
    fixture
        .runtime
        .register_object(HANDLE, Some(FRAME), ObjectKindTag::new(2))
        .unwrap();
    fixture.publish();
    assert_eq!(fixture.retained(ResourceClass::RemoteHandle), 2);

    // Act
    let result = receipt.commit_or_defer().unwrap();

    // Assert
    assert!(matches!(result, NativeHandleReleaseCommit::Superseded));
    assert!(fixture.routes.contains(HANDLE).unwrap());
    assert_eq!(fixture.runtime.is_object_live(HANDLE), Ok(true));
    assert_eq!(fixture.retained(ResourceClass::RemoteHandle), 1);
    assert_eq!(fixture.routes.metrics().unwrap().explicitly_disposed, 0);
}

#[test]
fn unadmitted_deferred_commit_is_rejected_before_native_io() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), false);

    // Act
    let result = fixture
        .service()
        .prepare_native_release(HANDLE, fixture.pool.reserve().unwrap());

    // Assert
    assert_eq!(result.err().unwrap().kind(), FailureKind::ProtocolViolation);
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.pool.pending(), 0);
}

#[test]
fn foreign_runtime_admission_is_rejected_before_native_io() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);
    let foreign = Fixture::new(Native::new(true, false), true);

    // Act
    let result = fixture
        .service()
        .prepare_native_release(HANDLE, foreign.pool.reserve().unwrap());

    // Assert
    assert_eq!(result.err().unwrap().kind(), FailureKind::ProtocolViolation);
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 0);
    assert_eq!(foreign.pool.pending(), 0);
}

#[tokio::test]
async fn closed_transport_rejects_commit_without_claiming_local_retirement() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);
    let receipt = fixture.prepare().release().await.unwrap();
    fixture.runtime.close_transport().unwrap();

    // Act
    let result = receipt.commit_or_defer();

    // Assert
    assert_eq!(result.err().unwrap().kind(), FailureKind::TransportClosed);
    assert_eq!(fixture.routes.metrics().unwrap().explicitly_disposed, 0);
    assert!(fixture.routes.contains(HANDLE).unwrap());
    assert_eq!(fixture.pool.pending(), 0);
}

#[test]
fn selected_route_profile_includes_the_retained_identity_allocation() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);

    // Act
    let retained = fixture.routes.metrics().unwrap().retained_bytes;

    // Assert
    assert_eq!(retained, 12 + 7 + ROUTE_OWNER_BYTES);
    assert_eq!(
        fixture
            .routes
            .profile()
            .requirements()
            .class(ResourceClass::BackendStateBytes)
            .global,
        3 * (64 + 64 + ROUTE_OWNER_BYTES)
    );
}

#[test]
fn dropping_an_unpolled_release_performs_no_native_work() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);
    let release = fixture.prepare().release();

    // Act
    drop(release);

    // Assert
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.pool.pending(), 0);
    assert!(fixture.routes.contains(HANDLE).unwrap());
}

#[tokio::test]
async fn primitive_handle_retirement_needs_no_native_disposer() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);
    fixture.routes.retire_parent_disposals(&[HANDLE]).unwrap();
    fixture
        .routes
        .reserve()
        .unwrap()
        .publish(
            HANDLE,
            ChromiumHandleRouteSpec::new(FRAME, ObjectId::new(3), 0, 1, None),
        )
        .unwrap();

    // Act
    let receipt = fixture.prepare().release().await.unwrap();

    // Assert
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 0);
    commit(receipt);
    assert_eq!(fixture.runtime.is_object_live(HANDLE), Ok(false));
    assert_eq!(fixture.retained(ResourceClass::RemoteHandle), 0);
}

#[tokio::test]
async fn admission_and_native_release_can_overlap_an_ordinary_transaction() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);
    let ordinary = fixture.runtime.prepare_object_disposal(FRAME).unwrap();

    // Act
    let receipt = fixture.prepare().release().await.unwrap();

    // Assert
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(true));
    drop(ordinary);
    commit(receipt);
}

#[test]
fn transport_close_after_reservation_rejects_before_native_io() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);
    let capacity = fixture.pool.reserve().unwrap();
    fixture.runtime.close_transport().unwrap();

    // Act
    let result = fixture.service().prepare_native_release(HANDLE, capacity);

    // Assert
    assert_eq!(result.err().unwrap().kind(), FailureKind::TransportClosed);
    assert_eq!(fixture.native.calls.load(Ordering::SeqCst), 0);
    assert_eq!(fixture.pool.pending(), 0);
}

#[tokio::test]
async fn unexpected_runtime_descendants_are_not_silently_retired() {
    // Arrange
    let fixture = Fixture::new(Native::new(true, false), true);
    let receipt = fixture.prepare().release().await.unwrap();
    fixture
        .runtime
        .register_object(ObjectId::new(6), Some(HANDLE), ObjectKindTag::new(2))
        .unwrap();

    // Act
    let result = receipt.commit_or_defer();

    // Assert
    assert_eq!(result.err().unwrap().kind(), FailureKind::ProtocolViolation);
    assert_eq!(fixture.runtime.is_object_live(HANDLE), Ok(true));
    assert_eq!(fixture.runtime.is_object_live(ObjectId::new(6)), Ok(true));
    assert!(fixture.routes.contains(HANDLE).unwrap());
    assert_eq!(fixture.runtime.lifecycle_transaction_active(), Ok(false));
    assert_eq!(fixture.pool.pending(), 0);
}

#[tokio::test]
#[ignore = "requires dedicated Chrome CDP and PWRIGHT_CDP_WS_URL"]
async fn real_chrome_release_ack_precedes_a_deferred_local_commit() {
    use pwright_browser_provider::AttachBrowserProvider;
    use pwright_cdp::{CdpClient, CdpSession, connection::ConnectionConfig};
    use pwright_composition_chromium_handle_lifecycle::CdpHandleDisposeLimits;
    use pwright_provider_cdp::{CdpAttachProvider, CdpAttachRequest, CdpWebSocketEndpoint};

    // Arrange: the test owns only this new Context, never the Chrome process.
    let endpoint =
        CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL").unwrap()).unwrap();
    let attachment = CdpAttachProvider::new()
        .attach(CdpAttachRequest::new(endpoint, ConnectionConfig::default()))
        .await
        .unwrap();
    let connection = attachment.connection().clone();
    let browser = CdpSession::browser(connection.clone());
    let context_id = browser
        .target_create_browser_context(None, None)
        .await
        .unwrap();
    let observed: Result<_, Box<dyn std::error::Error>> = async {
        let target_id = browser
            .target_create_in_context("about:blank", &context_id)
            .await?;
        let session_id = browser.target_attach(&target_id).await?;
        let page: Arc<dyn CdpClient> =
            Arc::new(CdpSession::new(connection.clone(), session_id, target_id));
        let result = page
            .runtime_evaluate_as_object("({ marker: 'NODE156-native-release' })")
            .await?;
        let native_id = result["result"]["objectId"]
            .as_str()
            .ok_or("missing native object ID")?;
        let before = page.runtime_get_properties(native_id).await?;
        let sibling = page
            .runtime_evaluate_as_object("({ marker: 'NODE156-survivor' })")
            .await?;
        let sibling_id = sibling["result"]["objectId"]
            .as_str()
            .ok_or("missing sibling ID")?;
        let readable_before = before["result"].as_array().is_some_and(|properties| {
            properties
                .iter()
                .any(|property| property["name"] == "marker")
        });
        let fixture = Fixture::new(Native::new(true, false), true);
        fixture.routes.retire_parent_disposals(&[HANDLE])?;
        fixture.routes.reserve()?.publish(
            HANDLE,
            ChromiumHandleRouteSpec::from_cdp(
                FRAME,
                ObjectId::new(3),
                page.clone(),
                Some(native_id),
                0,
                CdpHandleDisposeLimits::new(64),
            )?,
        )?;
        let ordinary = fixture.runtime.prepare_object_disposal(FRAME)?;

        // Act: native release is allowed while a separate local transaction is busy.
        let receipt = fixture.prepare().release().await?;

        // Capture evidence before the owned Context is disposed in all Result paths.
        let released_before_commit = page.runtime_get_properties(native_id).await.is_err();
        let sibling_after = page.runtime_get_properties(sibling_id).await?;
        let sibling_survives = sibling_after["result"]
            .as_array()
            .is_some_and(|properties| {
                properties.iter().any(|property| {
                    property["name"] == "marker" && property["value"]["value"] == "NODE156-survivor"
                })
            });
        let local_still_live =
            fixture.runtime.is_object_live(HANDLE)? && fixture.routes.contains(HANDLE)?;
        let NativeHandleReleaseCommit::Deferred(receipt) = receipt.commit_or_defer()? else {
            return Err("busy commit should preserve receipt".into());
        };
        drop(ordinary);
        commit(receipt);
        let retired =
            !fixture.runtime.is_object_live(HANDLE)? && !fixture.routes.contains(HANDLE)?;
        let returned =
            fixture.pool.pending() == 0 && fixture.retained(ResourceClass::RemoteHandle) == 0;
        Ok((
            readable_before,
            released_before_commit,
            local_still_live,
            retired,
            returned,
            sibling_survives,
        ))
    }
    .await;
    browser
        .target_dispose_browser_context(&context_id)
        .await
        .unwrap();
    let browser_alive = connection
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .is_ok();
    connection.request_disconnect();

    // Assert: actual native invalidation, not only local route removal.
    let evidence = observed.unwrap();
    assert_eq!(evidence, (true, true, true, true, true, true));
    assert!(browser_alive);
    eprintln!(
        "NODE156 Chrome: readable_before=true released_before_local_commit=true local_live_until_commit=true retired=true leases_returned=true sibling_survives=true owned_context_removed=true"
    );
}
