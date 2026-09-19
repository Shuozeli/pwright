//! Development-only command assembly: production topology and one controlled
//! runtime, without installing or advertising a public Node/wire capability.
use super::*;
use pwright_browser_model::FRAME_OBJECT_KIND_ID;
use pwright_cap_evaluation::EvaluateValue;
use pwright_core::{
    CallId, CallMetadata, CancellationReason, Command, CommandCancellation, CommandResult,
    CommandSpec, FailureKind, MethodTag, ObjectKindId, OperationContext,
};
use pwright_runtime::{CleanupQueueReservation, ControlledDispatcher, TargetedService};
use std::convert::Infallible;
use std::future::{Ready, pending, ready};
use std::sync::atomic::{AtomicUsize, Ordering};

type Runtime = ControlledDispatcher<CdpPageServices, CdpPageServices, Delays, CleanupQueue>;
type Prepared = MainWorldElement<CleanupQueueReservation, CleanupQueueReservation>;
struct Prepare;
impl CommandSpec for Prepare {
    const METHOD: MethodTag = MethodTag::new(65000);
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = &[FRAME_OBJECT_KIND_ID];
    type Params = Selector;
    type Result = Prepared;
    type ErrorDetails = ();
}

struct PreparationHandler {
    attempts: ContinuableResolutionService<CdpLocatorResolutionService, CleanupQueue>,
    adoption: LocatorElementEvaluationService<CleanupQueue>,
    waiter: StrictResolutionWaiter,
    preparation: LocatorElementPreparationService,
    access: ChromiumFrameWorldAccess,
    started: AtomicUsize,
    cancelled: AtomicUsize,
}
impl TargetedService<Selector> for PreparationHandler {
    type Response = Prepared;
    type Error = OperationError<()>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Prepared, OperationError<()>>> + Send + 'a>>;
    fn call_for(&self, frame: ObjectId, selector: Selector) -> Self::Future<'_> {
        self.started.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            let worlds = Worlds {
                access: self.access.clone(),
                frame,
            };
            let inputs = PreparationInputs {
                attempts: &self.attempts,
                waiter: &self.waiter,
                adoption: &self.adoption,
                worlds: &worlds,
                delays: &Delays,
            };
            self.preparation
                .prepare(selector, &inputs, pending())
                .await
                .map_err(|error| {
                    // Test-only adapter: error taxonomy parity belongs to the
                    // eventual production command. No public mapping is claimed.
                    OperationError::new(FailureKind::Backend, error.to_string(), None)
                })
        })
    }
}
impl CommandCancellation<Prepare> for PreparationHandler {
    type Future<'a> = Ready<Result<(), OperationError<()>>>;
    fn cancel(&self, _: OperationContext, _: CancellationReason) -> Self::Future<'_> {
        // Dropping preparation transfers its exact in-flight native phase to
        // its pre-reserved cleanup slot. This callback does not claim native
        // release; the test explicitly drains and checks that separate owner.
        self.cancelled.fetch_add(1, Ordering::SeqCst);
        ready(Ok(()))
    }
}

async fn drive_controlled<T, E>(
    future: impl Future<Output = Result<T, E>>,
    driver: &mut ChromiumTopologyEventDriver,
    runtime: &Runtime,
) -> Result<T>
where
    E: Into<anyhow::Error>,
{
    tokio::pin!(future);
    loop {
        tokio::select! {
            result = &mut future => return result.map_err(Into::into),
            event = driver.receive_once(runtime) => { event?; }
        }
    }
}
fn prepare_command(id: u64, frame: ObjectId, selector: Selector, timeout: u64) -> Command<Prepare> {
    Command::new(
        CallId::new(id),
        frame,
        CallMetadata::new(Some(Duration::from_millis(timeout)), None),
        selector,
    )
}
fn unwrap_dispatch<C: CommandSpec>(
    result: Result<CommandResult<C>, Infallible>,
) -> Result<C::Result, OperationError<C::ErrorDetails>> {
    match result {
        Ok(result) => result.into_outcome(),
        Err(never) => match never {},
    }
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn preparation_shares_real_frame_queue_deadline_and_native_cleanup() -> Result<()> {
    // Arrange: exactly one production runtime, one test-owned native Context,
    // independent, explicitly accounted preparation leaves and one cleanup pool.
    let resolution = ResolutionProfile::new(ResourceScope::extension(96, 1), 2)?;
    let adoption =
        ElementEvaluationProfile::new(ResourceScope::extension(96, 2), 1, tests::limits())?;
    let waiting = ResolutionWaitProfile::new(
        ResourceScope::extension(96, 3),
        1,
        [0, 20, 50, 100, 100, 500].map(Duration::from_millis),
    )?;
    let preparing = PreparationProfile::new(
        ResourceScope::extension(96, 4),
        1,
        Duration::from_millis(20),
    )?;
    let access_profile = ChromiumFrameWorldAccessProfile::new(
        ResourceScope::extension(96, 5),
        NonZeroUsize::new(3).unwrap(),
    )?;
    let requirements = resolution
        .requirements()
        .combine_concurrent(adoption.requirements())
        .combine_concurrent(waiting.requirements())
        .combine_concurrent(preparing.requirements());
    let endpoint = CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL")?)?;
    let mut composition = factory(requirements, access_profile)?
        .attach(endpoint)
        .await?;
    let queue = CleanupQueue::new(8);
    let runtime = composition.new_controlled_page_runtime(Delays, queue.clone())?;
    let mut driver = composition
        .topology_event_driver()?
        .context("missing topology driver")?
        .with_playwright_context_fallback();
    let access =
        ChromiumFrameWorldAccess::new(runtime.service(), access_profile, composition.resources())?;
    let ledger = composition
        .resources()
        .claim(access_profile.requirements())?;
    let baseline = ledger
        .scope_metrics(ResourceScope::extension(96, 5))?
        .unwrap()
        .units(ResourceClass::BackendStateBytes);
    let resolver = Arc::new(CdpLocatorResolutionService::new(
        resolution,
        composition.resources(),
        Arc::new(AtomicObjectIdSource::new(1)?),
    )?);
    let handler = Arc::new(PreparationHandler {
        attempts: ContinuableResolutionService::new(resolver.clone(), queue.clone()),
        adoption: LocatorElementEvaluationService::new(
            adoption,
            composition.resources(),
            queue.clone(),
        )?,
        waiter: StrictResolutionWaiter::new(waiting, composition.resources())?,
        preparation: LocatorElementPreparationService::new(preparing, composition.resources())?,
        access: access.clone(),
        started: AtomicUsize::new(0),
        cancelled: AtomicUsize::new(0),
    });
    let browser = CdpSession::browser(composition.connection().clone());
    let context = browser.target_create_browser_context(None, None).await?;
    let outcome = tokio::time::timeout(Duration::from_secs(20), async {
        let target = browser.target_create_in_context("about:blank", &context).await?;
        driver.initialize_snapshot(&runtime, 4096).await?;
        let frame = loop {
            if let Some(frame) = composition.topology()?.topology().main_frame_object_id(&target) { break frame; }
            driver.receive_once(&runtime).await?;
        };
        let setup = drive_controlled(access.snapshot(frame), &mut driver, &runtime).await?;
        let client = setup.client();
        let main = setup.main_unique_id().to_owned();
        client.runtime_evaluate_in_unique_context("globalThis.fixtureReady=true; document.body.innerHTML='<input id=original value=kept>'", &main).await?;
        drop(setup);

        // Act: dispatch preparation through the SAME typed controlled runtime.
        let selected = drive_controlled(runtime.dispatch_with(prepare_command(1, frame, Selector::css("#original")?, 2000), handler.as_ref(), handler.clone()), &mut driver, &runtime).await?;
        let selected = selected.into_outcome()?;

        // Assert: both wrappers exist; an escaped test-owned prepared value
        // retains its snapshot until explicit physical group release.
        ensure!(runtime.pending_call_count()? == 0);
        let utility_id = selected.data.as_ref().unwrap().source.as_ref().unwrap().element().object_id().to_owned();
        let main_id = selected.data.as_ref().unwrap().adopted.as_ref().unwrap().handle().object_id().unwrap().to_owned();
        for id in [&utility_id, &main_id] {
            ensure!(client.runtime_call_function_on(id, "function(){return this.value}", vec![]).await?["result"]["value"] == "kept");
        }
        selected.release().await?;
        for id in [&utility_id, &main_id] {
            ensure!(client.runtime_call_function_on(id, "function(){return this.value}", vec![]).await.is_err());
        }

        // Arrange: installed value evaluation occupies this exact Frame lane.
        let existing = Command::<EvaluateValue>::new(CallId::new(2), frame, CallMetadata::default(), EvaluateParams::function_without_argument("()=>new Promise(resolve=>{globalThis.baseStarted=true;globalThis.completeBase=()=>resolve(7)})"));
        let mut existing = runtime.call(existing);
        drive_controlled(async {
            loop {
                tokio::select! {
                    result = &mut existing => { unwrap_dispatch(result)?; anyhow::bail!("base completed before queue probe"); },
                    value = client.runtime_evaluate_in_unique_context("globalThis.baseStarted===true", &main) => {
                        if value?["result"]["value"] == true { break; }
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        }, &mut driver, &runtime).await?;
        let starts = handler.started.load(Ordering::SeqCst);
        let queued = runtime.dispatch_with(prepare_command(3, frame, Selector::css("#original")?, 30), handler.as_ref(), handler.clone());
        // Act: the one original queue deadline expires before backend entry.
        let queued = drive_controlled(queued, &mut driver, &runtime).await?.into_outcome();
        // Assert: no native preparation/adoption or extra route allocation.
        ensure!(queued.is_err_and(|error| error.kind() == FailureKind::Timeout));
        ensure!(handler.started.load(Ordering::SeqCst) == starts);
        ensure!(handler.cancelled.load(Ordering::SeqCst) == 0);
        ensure!(runtime.pending_call_count()? == 1);
        ensure!(ledger.scope_metrics(ResourceScope::extension(96, 5))?.unwrap().units(ResourceClass::BackendStateBytes) == baseline);
        client.runtime_evaluate_in_unique_context("globalThis.completeBase()", &main).await?;
        let existing = drive_controlled(existing, &mut driver, &runtime).await?;
        ensure!(existing.into_outcome()?.into_value() == EvaluationValue::Number(pwright_cap_evaluation::EvaluationNumber::new(7.0)?));

        // Arrange/Act: a started strict wait uses runtime deadline cancellation.
        let missing = drive_controlled(runtime.dispatch_with(prepare_command(4, frame, Selector::css("#missing")?, 50), handler.as_ref(), handler.clone()), &mut driver, &runtime).await?.into_outcome();
        // Assert only typed timeout now; native cleanup is checked AFTER drain.
        ensure!(missing.is_err_and(|error| error.kind() == FailureKind::Timeout));
        ensure!(handler.cancelled.load(Ordering::SeqCst) == 1);
        ensure!(runtime.pending_call_count()? == 0 && runtime.connection_metrics()?.active_commands == 0);
        Ok::<_, anyhow::Error>(())
    }).await;
    let drain = tokio::time::timeout(Duration::from_secs(3), async {
        while let Some(job) = queue.try_take()? {
            job.await?;
        }
        Ok::<_, anyhow::Error>(())
    })
    .await;
    let cleanup = tokio::time::timeout(
        Duration::from_secs(3),
        browser.target_dispose_browser_context(&context),
    )
    .await;
    let alive = composition
        .connection()
        .send_command("Browser.getVersion", serde_json::json!({}))
        .await
        .is_ok();
    cleanup??;
    drain??;
    outcome??;
    ensure!(alive && resolver.is_cleanup_healthy() && handler.adoption.is_cleanup_healthy());
    ensure!(queue.metrics()?.reserved == 0 && queue.metrics()?.queued == 0);
    ensure!(
        ledger
            .scope_metrics(ResourceScope::extension(96, 5))?
            .unwrap()
            .units(ResourceClass::BackendStateBytes)
            == baseline
    );
    Ok(())
}
