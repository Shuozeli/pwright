//! Opt-in native receipts against the external dedicated Chrome process.
use super::*;
use crate::{
    ChromiumCloseObserver, ChromiumClosePlan, ChromiumCloseReservation, ChromiumCloseTarget,
    ObservedChromiumCloseCommandService,
};
use pwright_core::{OperationError, ResourceLease};

struct Observer {
    ledger: ResourceLedger,
    receipts: Arc<Mutex<Vec<ChromiumCloseTarget>>>,
}
struct Reservation {
    target: ChromiumCloseTarget,
    receipts: Arc<Mutex<Vec<ChromiumCloseTarget>>>,
    _storage: ResourceLease,
}

struct LiveClosePoll;
impl pwright_backend_cdp::CdpPageClosePoll for LiveClosePoll {
    fn wait(&self, _: usize) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(tokio::time::sleep(Duration::from_millis(10)))
    }
}

fn target_id(target: ChromiumCloseTarget) -> ObjectId {
    match target {
        ChromiumCloseTarget::Page(id) | ChromiumCloseTarget::BrowserContext(id) => id,
    }
}

impl ChromiumCloseObserver for Observer {
    fn prepare(
        &self,
        plan: ChromiumClosePlan<'_>,
    ) -> Result<Box<dyn ChromiumCloseReservation>, OperationError<()>> {
        let bytes = u64::try_from(std::mem::size_of::<Reservation>()).unwrap();
        let storage = self
            .ledger
            .reserve(
                ResourceScope::Connection,
                ResourceClass::BackendStateBytes,
                bytes,
            )
            .map_err(|error| {
                OperationError::new(FailureKind::ResourceExhausted, error.to_string(), None)
            })?;
        if !plan
            .effects()
            .contains(&CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id: target_id(plan.target()),
            }))
        {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "missing native close plan",
                None,
            ));
        }
        Ok(Box::new(Reservation {
            target: plan.target(),
            receipts: self.receipts.clone(),
            _storage: storage,
        }))
    }
}

impl ChromiumCloseReservation for Reservation {
    fn committed(self: Box<Self>, effects: &[CdpTopologyEffect]) -> Result<(), OperationError<()>> {
        if !effects.contains(&CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
            object_id: target_id(self.target),
        })) {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "missing committed native close",
                None,
            ));
        }
        let mut receipts = self.receipts.lock().unwrap();
        if receipts.len() == 2 {
            return Err(OperationError::new(
                FailureKind::ResourceExhausted,
                "bounded test receipt overflow",
                None,
            ));
        }
        receipts.push(self.target);
        Ok(())
    }
}

#[tokio::test]
#[ignore = "requires dedicated Chrome CDP and PWRIGHT_CDP_WS_URL; creates only one owned Context"]
async fn live_observed_page_and_context_close_publish_exact_native_receipts() {
    // Arrange: existing browser targets are never closed by this test.
    let endpoint =
        CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL").unwrap()).unwrap();
    assert!(endpoint.as_str().starts_with("ws://127.0.0.1:9222/"));
    // Match the selected Node host: at most 20 checks with 10 ms between them.
    // ImmediatePageClosePoll is for deterministic fakes, not asynchronous Chrome.
    let close_limits = CdpPageCloseLimits::new(128, 20);
    let profile = base_profile_with_runtime(
        RuntimeLimits::new(32, 4)
            .with_active_commands(2)
            .with_control_messages(32),
    )
    .with_context_lifecycle(context_lifecycle_profile())
    .with_context_creation(context_creation_profile())
    .with_page_lifecycle(PageLifecycleInstallationProfile::new(
        close_limits,
        pwright_codec_page_lifecycle::PageLifecycleCodecLimits::new(1024),
        wire_profile(),
    ))
    .with_page_creation(page_creation_profile());
    let factory = ChromiumAttachFactory::new(
        profile.clone(),
        exact_limits(profile.requirements()),
        ObjectId::new(1),
    )
    .unwrap();
    let mut composition = tokio::time::timeout(Duration::from_secs(15), factory.attach(endpoint))
        .await
        .unwrap()
        .unwrap();
    let cleanup = CleanupQueue::new(4);
    let runtime = composition
        .new_controlled_page_runtime(NoDeadlineScheduler, cleanup.clone())
        .unwrap();
    let create_context = composition
        .browser_new_context_service(runtime.lifecycle_handle(), cleanup.clone())
        .unwrap()
        .unwrap();
    let context = create_context
        .call(Command::<BrowserNewContext>::new(
            CallId::new(100),
            ObjectId::new(1),
            CallMetadata::default(),
            (),
        ))
        .await
        .unwrap()
        .into_outcome()
        .unwrap();
    let native_context = composition
        .shared_topology()
        .lock()
        .unwrap()
        .topology()
        .browser_context_route(context.object_id)
        .unwrap()
        .browser_context_id()
        .to_owned();
    let browser = CdpSession::browser(composition.connection().clone());
    let bytes = u64::try_from(std::mem::size_of::<Reservation>()).unwrap();
    let observer = Arc::new(Observer {
        ledger: ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::BackendStateBytes,
            bytes,
            bytes,
        )),
        receipts: Arc::new(Mutex::new(Vec::with_capacity(2))),
    });

    // Act: admission and actual close operations run through the selected port.
    let result = tokio::time::timeout(Duration::from_secs(20), async {
        let create = composition
            .browser_context_new_page_service(&runtime, cleanup.clone())
            .unwrap()
            .unwrap();
        let first = create
            .call(Command::<BrowserContextNewPage>::new(
                CallId::new(101),
                context.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .map_err(|error| error.to_string())?;
        let second = create
            .call(Command::<BrowserContextNewPage>::new(
                CallId::new(102),
                context.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .map_err(|error| error.to_string())?;
        let client: Arc<dyn CdpClient> =
            Arc::new(CdpSession::browser(composition.connection().clone()));
        let raw = ChromiumPageCloseCommandService::new(
            client.clone(),
            composition.shared_topology(),
            runtime.service().clone(),
            runtime.lifecycle_handle(),
            close_limits,
            Arc::new(LiveClosePoll),
        );
        let close = ObservedChromiumCloseCommandService::new(raw, observer.clone());
        close
            .call(Command::<PageClose>::new(
                CallId::new(103),
                first.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .map_err(|error| error.to_string())?;
        let first_retired = !runtime.is_object_live(first.object_id).unwrap();
        let sibling_live = runtime.is_object_live(second.object_id).unwrap();
        let raw = ChromiumBrowserContextCloseCommandService::new(
            client,
            composition.shared_topology(),
            runtime.lifecycle_handle(),
        )
        .with_page_services(runtime.service().clone());
        let close = ObservedChromiumCloseCommandService::new(raw, observer.clone());
        close
            .call(Command::<BrowserContextClose>::new(
                CallId::new(104),
                context.object_id,
                CallMetadata::default(),
                (),
            ))
            .await
            .unwrap()
            .into_outcome()
            .map_err(|error| error.to_string())?;
        Ok::<_, String>((
            first.object_id,
            first_retired,
            sibling_live,
            !runtime.is_object_live(second.object_id).unwrap(),
        ))
    })
    .await;

    // Always clean up the exact native Context before asserting the outcome.
    let mut continuation_error = None;
    while let Some(job) = cleanup.try_take().unwrap() {
        match tokio::time::timeout(Duration::from_secs(10), job).await {
            Ok(Ok(())) => {}
            failure => {
                continuation_error = Some(format!("continuation failed: {failure:?}"));
                break;
            }
        }
    }
    let contexts = browser
        .send("Target.getBrowserContexts", serde_json::json!({}))
        .await
        .unwrap();
    if contexts["browserContextIds"]
        .as_array()
        .unwrap()
        .iter()
        .any(|id| id.as_str() == Some(&native_context))
    {
        browser
            .send(
                "Target.disposeBrowserContext",
                serde_json::json!({"browserContextId": native_context}),
            )
            .await
            .unwrap();
    }

    // Assert
    assert!(continuation_error.is_none(), "{continuation_error:?}");
    let (first, first_retired, sibling_live, sibling_retired) = result.unwrap().unwrap();
    assert!(first_retired && sibling_live && sibling_retired);
    assert_eq!(
        *observer.receipts.lock().unwrap(),
        [
            ChromiumCloseTarget::Page(first),
            ChromiumCloseTarget::BrowserContext(context.object_id)
        ]
    );
    assert_eq!(
        observer
            .ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
    assert!(
        browser
            .send("Browser.getVersion", serde_json::json!({}))
            .await
            .is_ok()
    );
}
