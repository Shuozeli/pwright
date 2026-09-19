//! Real close services with an admitted, bounded test observer.
mod confirmation;
use super::*;
use crate::{
    ChromiumCloseObserver, ChromiumClosePlan, ChromiumCloseReservation, ChromiumCloseTarget,
    ChromiumNetworkPageRegistrationHandle, ObservedChromiumCloseCommandService,
};
use pwright_core::{OperationError, ResourceLease};
use pwright_runtime::ControlledLifecycleHandle;

const EFFECTS: usize = 8;
const SCOPE: ResourceScope = ResourceScope::Connection;

#[derive(Default)]
struct Record {
    attempts: usize,
    commits: usize,
    abandoned: usize,
    target: Option<ChromiumCloseTarget>,
}

// Execution owners are held only by this test diagnostic, never recommended
// retention for a production metadata journal.
#[derive(Clone)]
struct Observer {
    record: Arc<Mutex<Record>>,
    ledger: ResourceLedger,
    topology: Arc<Mutex<ProductionCdpTopology>>,
    runtime: ControlledLifecycleHandle,
    services: Option<CdpPageServices>,
    browser: Arc<FakeCdpClient>,
    native_calls_before: usize,
    fail_commit: bool,
    network: Option<ChromiumNetworkPageRegistrationHandle>,
}

struct Reservation {
    observer: Observer,
    target: ObjectId,
    expected: Vec<CdpTopologyEffect>,
    source_address: usize,
    finished: bool,
    _storage: ResourceLease,
}

fn reservation_bytes() -> u64 {
    u64::try_from(
        std::mem::size_of::<Reservation>() + EFFECTS * std::mem::size_of::<CdpTopologyEffect>(),
    )
    .unwrap()
}

impl Observer {
    fn new(
        topology: Arc<Mutex<ProductionCdpTopology>>,
        runtime: ControlledLifecycleHandle,
        services: Option<CdpPageServices>,
        browser: Arc<FakeCdpClient>,
        capacity: u64,
    ) -> Self {
        Self {
            record: Arc::new(Mutex::new(Record::default())),
            ledger: ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
                ResourceClass::BackendStateBytes,
                capacity,
                capacity,
            )),
            topology,
            runtime,
            services,
            native_calls_before: browser.method_names().len(),
            browser,
            fail_commit: false,
            network: None,
        }
    }

    fn retained(&self) -> u64 {
        self.ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current
    }

    fn with_network(mut self, registration: ChromiumNetworkPageRegistrationHandle) -> Self {
        self.network = Some(registration);
        self
    }
}

impl ChromiumCloseObserver for Observer {
    fn prepare(
        &self,
        plan: ChromiumClosePlan<'_>,
    ) -> Result<Box<dyn ChromiumCloseReservation>, OperationError<()>> {
        self.record.lock().unwrap().attempts += 1;
        assert_eq!(self.browser.method_names().len(), self.native_calls_before);
        assert!(self.topology.try_lock().unwrap().transaction_active());
        assert!(self.runtime.lifecycle_transaction_active().unwrap());
        let target = match plan.target() {
            ChromiumCloseTarget::Page(id) | ChromiumCloseTarget::BrowserContext(id) => id,
        };
        assert!(self.runtime.is_object_live(target).unwrap());
        assert!(plan.effects().len() <= EFFECTS);
        let storage = self
            .ledger
            .reserve(SCOPE, ResourceClass::BackendStateBytes, reservation_bytes())
            .map_err(|error| {
                OperationError::new(FailureKind::ResourceExhausted, error.to_string(), None)
            })?;
        // Both box and fixed vector capacity were admitted before allocation.
        let mut expected = Vec::with_capacity(EFFECTS);
        expected.extend_from_slice(plan.effects());
        self.record.lock().unwrap().target = Some(plan.target());
        Ok(Box::new(Reservation {
            observer: self.clone(),
            target,
            expected,
            source_address: plan.effects().as_ptr().addr(),
            finished: false,
            _storage: storage,
        }))
    }
}

impl ChromiumCloseReservation for Reservation {
    fn committed(
        mut self: Box<Self>,
        effects: &[CdpTopologyEffect],
    ) -> Result<(), OperationError<()>> {
        assert_eq!(effects, self.expected);
        assert_eq!(effects.as_ptr().addr(), self.source_address);
        assert!(
            effects.contains(&CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id: self.target
            }))
        );
        assert!(
            !self
                .observer
                .topology
                .try_lock()
                .unwrap()
                .transaction_active()
        );
        assert!(
            !self
                .observer
                .runtime
                .lifecycle_transaction_active()
                .unwrap()
        );
        assert!(!self.observer.runtime.is_object_live(self.target).unwrap());
        if let Some(registration) = &self.observer.network {
            let driver = registration.driver().unwrap();
            assert!(driver.state.try_lock().unwrap().adapters.is_empty());
        }
        if let Some(services) = &self.observer.services {
            assert_eq!(services.route_count(), Ok(0));
            if let Some(registry) = services.network_subscription_registry() {
                assert_eq!(registry.metrics().unwrap().registered_page_lanes(), 0);
            }
        }
        self.finished = true;
        self.observer.record.lock().unwrap().commits += 1;
        if self.observer.fail_commit {
            return Err(OperationError::new(
                FailureKind::Cleanup,
                "observer commit rejected",
                None,
            ));
        }
        Ok(())
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if !self.finished {
            self.observer.record.lock().unwrap().abandoned += 1;
        }
    }
}

fn page_command() -> Command<PageClose> {
    Command::new(
        CallId::new(31),
        ObjectId::new(3),
        CallMetadata::default(),
        (),
    )
}

#[tokio::test]
async fn context_admission_failure_prevents_native_disposal() {
    // Arrange
    let (runtime, topology) = context_close_state(CdpContextOwnership::Owned);
    let browser = Arc::new(FakeCdpClient::from_html(""));
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        None,
        browser.clone(),
        0,
    ));
    let raw = ChromiumBrowserContextCloseCommandService::new(
        browser.clone(),
        topology.clone(),
        runtime.lifecycle_handle(),
    );
    let service = ObservedChromiumCloseCommandService::new(raw, observer.clone());
    // Act
    let error = service
        .call(context_command())
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(browser.method_names().is_empty());
    assert_eq!(runtime.is_object_live(ObjectId::new(2)), Ok(true));
    assert!(!runtime.lifecycle_transaction_active().unwrap());
    assert!(!topology.lock().unwrap().transaction_active());
    assert_eq!(observer.record.lock().unwrap().commits, 0);
    assert_eq!(observer.retained(), 0);
}

#[tokio::test]
async fn unconfirmed_page_close_releases_receipt_without_claiming_retirement() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    let target = pwright_cdp::domains::target::TargetInfo {
        target_id: "target-a".into(),
        target_type: "page".into(),
        title: String::new(),
        url: "about:blank".into(),
        attached: true,
    };
    browser.set_target_responses([vec![target.clone()], vec![target]]);
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        None,
        browser.clone(),
        reservation_bytes(),
    ));
    let service = ObservedChromiumCloseCommandService::new(
        page_service(&runtime, topology, browser),
        observer.clone(),
    );
    // Act
    let error = service
        .call(page_command())
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert_eq!(observer.record.lock().unwrap().abandoned, 1);
    assert_eq!(observer.record.lock().unwrap().commits, 0);
    assert_eq!(observer.retained(), 0);
}

#[tokio::test]
async fn already_closed_page_does_not_repeat_observer_admission() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        None,
        browser.clone(),
        reservation_bytes(),
    ));
    let service = ObservedChromiumCloseCommandService::new(
        page_service(&runtime, topology, browser),
        observer.clone(),
    );
    service
        .call(page_command())
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    // Act
    let error = service
        .call(page_command())
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::TargetClosed);
    assert_eq!(observer.record.lock().unwrap().attempts, 1);
    assert_eq!(observer.record.lock().unwrap().commits, 1);
    assert_eq!(observer.retained(), 0);
}

#[test]
fn context_continuation_keeps_one_reservation_until_native_completion() {
    // Arrange
    let (runtime, topology) = context_close_state(CdpContextOwnership::Owned);
    let browser = Arc::new(FakeCdpClient::from_html(""));
    browser.pause_dispose_browser_context();
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        None,
        browser.clone(),
        reservation_bytes(),
    ));
    let raw = ChromiumBrowserContextCloseCommandService::new(
        browser.clone(),
        topology,
        runtime.lifecycle_handle(),
    );
    let observed = ObservedChromiumCloseCommandService::new(raw, observer.clone());
    let cleanup = CleanupQueue::new(1);
    let service =
        ContinuableBrowserContextCloseCommandService::new(Arc::new(observed), cleanup.clone());
    let mut future = Box::pin(service.call(context_command()));
    assert!(poll_now(future.as_mut()).is_pending());
    // Act
    drop(future);
    // Assert
    assert_eq!(observer.retained(), reservation_bytes());
    assert_eq!(observer.record.lock().unwrap().attempts, 1);
    assert_eq!(observer.record.lock().unwrap().abandoned, 0);
    browser.resume_dispose_browser_context();
    let mut continuation = cleanup.try_take().unwrap().unwrap();
    assert!(matches!(
        poll_now(continuation.as_mut()),
        Poll::Ready(Ok(()))
    ));
    assert_eq!(observer.record.lock().unwrap().commits, 1);
    assert_eq!(observer.retained(), 0);
}

fn context_command() -> Command<BrowserContextClose> {
    Command::new(
        CallId::new(32),
        ObjectId::new(2),
        CallMetadata::default(),
        (),
    )
}

fn page_service(
    runtime: &PageCloseRuntime,
    topology: Arc<Mutex<ProductionCdpTopology>>,
    browser: Arc<FakeCdpClient>,
) -> ChromiumPageCloseCommandService {
    ChromiumPageCloseCommandService::new(
        browser,
        topology,
        runtime.service().clone(),
        runtime.lifecycle_handle(),
        page_lifecycle_profile().backend(),
        Arc::new(ImmediatePageClosePoll),
    )
}

#[tokio::test]
async fn page_receipt_follows_every_local_commit_and_releases_admission() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        Some(runtime.service().clone()),
        browser.clone(),
        reservation_bytes(),
    ));
    let service = ObservedChromiumCloseCommandService::new(
        page_service(&runtime, topology, browser),
        observer.clone(),
    );
    // Act
    let outcome = service.call(page_command()).await.unwrap().into_outcome();
    // Assert
    assert_eq!(outcome, Ok(()));
    assert_eq!(observer.record.lock().unwrap().commits, 1);
    assert_eq!(
        observer.record.lock().unwrap().target,
        Some(ChromiumCloseTarget::Page(ObjectId::new(3)))
    );
    assert_eq!(observer.retained(), 0);
}

#[tokio::test]
async fn context_receipt_follows_subtree_retirement() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        Some(runtime.service().clone()),
        browser.clone(),
        reservation_bytes(),
    ));
    let raw = ChromiumBrowserContextCloseCommandService::new(
        browser,
        topology,
        runtime.lifecycle_handle(),
    )
    .with_page_services(runtime.service().clone());
    let service = ObservedChromiumCloseCommandService::new(raw, observer.clone());
    // Act
    let outcome = service
        .call(context_command())
        .await
        .unwrap()
        .into_outcome();
    // Assert
    assert_eq!(outcome, Ok(()));
    assert_eq!(observer.record.lock().unwrap().commits, 1);
    assert_eq!(
        observer.record.lock().unwrap().target,
        Some(ChromiumCloseTarget::BrowserContext(ObjectId::new(2)))
    );
    assert_eq!(runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(observer.retained(), 0);
}

#[tokio::test]
async fn admission_failure_prevents_native_page_close_and_releases_transactions() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        None,
        browser.clone(),
        0,
    ));
    let service = ObservedChromiumCloseCommandService::new(
        page_service(&runtime, topology.clone(), browser.clone()),
        observer.clone(),
    );
    // Act
    let error = service
        .call(page_command())
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::ResourceExhausted);
    assert!(browser.method_names().is_empty());
    assert_eq!(runtime.is_object_live(ObjectId::new(3)), Ok(true));
    assert!(!runtime.lifecycle_transaction_active().unwrap());
    assert!(!topology.lock().unwrap().transaction_active());
    assert_eq!(observer.record.lock().unwrap().commits, 0);
    assert_eq!(observer.retained(), 0);
}

#[tokio::test]
async fn native_context_failure_drops_reservation_without_a_commit_receipt() {
    // Arrange
    let (runtime, topology) = context_close_state(CdpContextOwnership::Owned);
    let browser = Arc::new(FakeCdpClient::from_html(""));
    browser.set_dispose_browser_context_error(Some("close refused".into()));
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        None,
        browser.clone(),
        reservation_bytes(),
    ));
    let raw = ChromiumBrowserContextCloseCommandService::new(
        browser,
        topology,
        runtime.lifecycle_handle(),
    );
    let service = ObservedChromiumCloseCommandService::new(raw, observer.clone());
    // Act
    let error = service
        .call(context_command())
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Backend);
    assert_eq!(runtime.is_object_live(ObjectId::new(2)), Ok(true));
    assert_eq!(observer.record.lock().unwrap().commits, 0);
    assert_eq!(observer.record.lock().unwrap().abandoned, 1);
    assert_eq!(observer.retained(), 0);
}

#[tokio::test]
async fn borrowed_context_does_not_prepare_a_destruction_receipt() {
    // Arrange
    let (runtime, topology) = context_close_state(CdpContextOwnership::External);
    let browser = Arc::new(FakeCdpClient::from_html(""));
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        None,
        browser.clone(),
        reservation_bytes(),
    ));
    let raw = ChromiumBrowserContextCloseCommandService::new(
        browser.clone(),
        topology,
        runtime.lifecycle_handle(),
    );
    let service = ObservedChromiumCloseCommandService::new(raw, observer.clone());
    // Act
    let outcome = service
        .call(context_command())
        .await
        .unwrap()
        .into_outcome();
    // Assert
    assert!(outcome.is_err());
    assert!(browser.method_names().is_empty());
    assert_eq!(observer.record.lock().unwrap().attempts, 0);
    assert_eq!(observer.retained(), 0);
}

#[tokio::test]
async fn observer_failure_after_commit_does_not_roll_back_closed_owners() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    let mut observer = Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        Some(runtime.service().clone()),
        browser.clone(),
        reservation_bytes(),
    );
    observer.fail_commit = true;
    let observer = Arc::new(observer);
    let service = ObservedChromiumCloseCommandService::new(
        page_service(&runtime, topology, browser),
        observer.clone(),
    );
    // Act
    let error = service
        .call(page_command())
        .await
        .unwrap()
        .into_outcome()
        .unwrap_err();
    // Assert
    assert_eq!(error.kind(), FailureKind::Cleanup);
    assert_eq!(runtime.is_object_live(ObjectId::new(3)), Ok(false));
    assert_eq!(observer.record.lock().unwrap().commits, 1);
    assert_eq!(observer.record.lock().unwrap().abandoned, 0);
    assert_eq!(observer.retained(), 0);
}

#[test]
fn unpolled_close_has_no_observer_admission() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        None,
        browser.clone(),
        reservation_bytes(),
    ));
    let service = ObservedChromiumCloseCommandService::new(
        page_service(&runtime, topology, browser.clone()),
        observer.clone(),
    );
    let future = service.call(page_command());
    // Act
    drop(future);
    // Assert
    assert_eq!(observer.record.lock().unwrap().attempts, 0);
    assert!(browser.method_names().is_empty());
    assert_eq!(observer.retained(), 0);
}

#[test]
fn started_close_moves_the_same_admitted_receipt_into_cleanup_continuation() {
    // Arrange
    let (runtime, topology, browser, _) = page_close_state();
    browser.pause_target_close();
    let observer = Arc::new(Observer::new(
        topology.clone(),
        runtime.lifecycle_handle(),
        Some(runtime.service().clone()),
        browser.clone(),
        reservation_bytes(),
    ));
    let raw = ObservedChromiumCloseCommandService::new(
        page_service(&runtime, topology, browser.clone()),
        observer.clone(),
    );
    let cleanup = CleanupQueue::new(1);
    let service = ContinuablePageCloseCommandService::new(Arc::new(raw), cleanup.clone());
    let mut future = Box::pin(service.call(page_command()));
    assert!(poll_now(future.as_mut()).is_pending());
    assert_eq!(observer.retained(), reservation_bytes());
    // Act
    drop(future);
    // Assert: the same admission stays alive, without a second prepare call.
    assert_eq!(observer.record.lock().unwrap().attempts, 1);
    assert_eq!(observer.record.lock().unwrap().abandoned, 0);
    assert_eq!(observer.retained(), reservation_bytes());
    browser.resume_target_close();
    let mut continuation = cleanup.try_take().unwrap().unwrap();
    assert!(matches!(
        poll_now(continuation.as_mut()),
        Poll::Ready(Ok(()))
    ));
    assert_eq!(observer.record.lock().unwrap().commits, 1);
    assert_eq!(observer.retained(), 0);
}

#[tokio::test]
async fn observed_network_page_close_notifies_after_the_lane_is_retired() {
    // Arrange
    let (runtime, topology, browser, _, services, driver, creation) = network_page_creation_state();
    let page = creation
        .call(Command::<BrowserContextNewPage>::new(
            CallId::new(40),
            ObjectId::new(2),
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let observer = Arc::new(
        Observer::new(
            topology.clone(),
            runtime.lifecycle_handle(),
            Some(services.clone()),
            browser.clone(),
            reservation_bytes(),
        )
        .with_network(driver.registration_handle()),
    );
    let raw = ChromiumPageCloseCommandService::new(
        browser,
        topology,
        services,
        runtime.lifecycle_handle(),
        page_lifecycle_profile().backend(),
        Arc::new(ImmediatePageClosePoll),
    )
    .with_network_registration(driver.registration_handle());
    let service = ObservedChromiumCloseCommandService::new(raw, observer.clone());
    // Act
    let outcome = service
        .call(Command::<PageClose>::new(
            CallId::new(41),
            page.object_id,
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome();
    // Assert
    assert_eq!(outcome, Ok(()));
    assert_eq!(driver.page_count(), 0);
    assert_eq!(observer.record.lock().unwrap().commits, 1);
    assert_eq!(observer.retained(), 0);
}

#[tokio::test]
async fn observed_network_context_close_notifies_after_every_lane_is_retired() {
    // Arrange
    let (runtime, topology, browser, _, services, driver, creation) = network_page_creation_state();
    creation
        .call(Command::<BrowserContextNewPage>::new(
            CallId::new(40),
            ObjectId::new(2),
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let observer = Arc::new(
        Observer::new(
            topology.clone(),
            runtime.lifecycle_handle(),
            Some(services.clone()),
            browser.clone(),
            reservation_bytes(),
        )
        .with_network(driver.registration_handle()),
    );

    let raw = ChromiumBrowserContextCloseCommandService::new(
        browser,
        topology,
        runtime.lifecycle_handle(),
    )
    .with_network_registration(services, driver.registration_handle());
    let service = ObservedChromiumCloseCommandService::new(raw, observer.clone());
    // Act
    let outcome = service
        .call(context_command())
        .await
        .unwrap()
        .into_outcome();
    // Assert
    assert_eq!(outcome, Ok(()));
    assert_eq!(driver.page_count(), 0);
    assert_eq!(observer.record.lock().unwrap().commits, 1);
    assert_eq!(observer.retained(), 0);
}
