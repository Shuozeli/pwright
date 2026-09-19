//! Integration with the actual production paired-world port and event driver.
mod controlled;
use super::*;
use crate::preparation::*;
use anyhow::{Context, Result, ensure};
use pwright_backend_cdp::{
    CdpLocatorProductionProfile, CdpTopologyLimits, CdpTopologyProductionProfile,
};
use pwright_cdp::{CdpConnectionProductionProfile, CdpSession, connection::ConnectionConfig};
use pwright_composition_chromium::*;
use pwright_composition_chromium_locator_resolution::{
    CdpLocatorResolutionService, ContinuableResolutionService, ResolutionProfile, ResolutionRoute,
    ResolutionWaitError, ResolutionWaitProfile, ResolutionWaitStop, StrictResolutionWaiter,
};
use pwright_core::{AtomicObjectIdSource, ObjectId, ProtocolVersion, ResourceLimits};
use pwright_provider_cdp::CdpWebSocketEndpoint;
use pwright_runtime::{CleanupQueue, DeadlineScheduler, DirectDispatcher, RuntimeLimits};
use pwright_selector_model::Selector;
use pwright_server_wire::ServerSupervisorLimits;
use pwright_transport::FrameLimits;
use std::{future::Future, num::NonZeroUsize, pin::Pin, time::Duration};

struct Pair(ChromiumFrameWorldSnapshot);
fn route_error(error: FrameWorldAccessError) -> ResolutionWaitError {
    match error {
        FrameWorldAccessError::Topology(CdpPageRouteError::WorldTargetClosed { .. }) => {
            ResolutionWaitStop::TargetClosed.into()
        }
        FrameWorldAccessError::Topology(CdpPageRouteError::PageCrashed { .. }) => {
            ResolutionWaitStop::BrowserCrashed.into()
        }
        other => ResolutionWaitError::Route(Box::new(other)),
    }
}
impl PairedWorld for Pair {
    fn utility_route(&self) -> ResolutionRoute {
        ResolutionRoute {
            client: self.0.client(),
            utility_world: self.0.utility_unique_id().into(),
            destruction: None,
        }
    }
    fn main_route(&self) -> MainWorldRoute {
        MainWorldRoute {
            client: self.0.client(),
            execution_context_id: self.0.main_execution_context_id(),
            unique_context_id: self.0.main_unique_id().into(),
        }
    }
    fn is_current(&self) -> Result<bool, ResolutionWaitError> {
        self.0.is_current().map_err(route_error)
    }
}
struct Worlds {
    access: ChromiumFrameWorldAccess,
    frame: ObjectId,
}
impl PairedWorldSource for Worlds {
    type Pair = Pair;
    type Current<'a> =
        Pin<Box<dyn Future<Output = Result<Arc<Pair>, ResolutionWaitError>> + Send + 'a>>;
    fn current(&self) -> Self::Current<'_> {
        let future = self.access.snapshot(self.frame);
        Box::pin(async move {
            future
                .await
                .map(|snapshot| Arc::new(Pair(snapshot)))
                .map_err(route_error)
        })
    }
}
struct Delays;
impl DeadlineScheduler for Delays {
    type Sleep<'a> = tokio::time::Sleep;
    fn sleep(&self, delay: Duration) -> Option<Self::Sleep<'_>> {
        Some(tokio::time::sleep(delay))
    }
}
async fn drive<T, E>(
    future: impl Future<Output = Result<T, E>>,
    driver: &mut ChromiumTopologyEventDriver,
    runtime: &DirectDispatcher<CdpPageServices>,
) -> Result<T>
where
    E: std::error::Error + Send + Sync + 'static,
{
    tokio::pin!(future);
    loop {
        tokio::select! {
            result = &mut future => return result.map_err(Into::into),
            event = driver.receive_once(runtime) => { event?; }
        }
    }
}
fn factory(
    extra: ResourceRequirements,
    access: ChromiumFrameWorldAccessProfile,
) -> Result<ChromiumAttachFactory> {
    let connection =
        CdpConnectionProductionProfile::new(ResourceScope::Connection, ConnectionConfig::default());
    let topology = CdpTopologyProductionProfile::new(
        CdpTopologyLimits::new(32, 8, 8, 32, 64, 256),
        ResourceScope::extension(94, 1),
    )?;
    let routes =
        CdpPageRouteProductionProfile::from_topology(ResourceScope::extension(94, 2), topology);
    let events = ChromiumTopologyEventDriverProductionProfile::from_profiles(
        ResourceScope::extension(94, 3),
        &connection,
        topology,
    )?;
    let discovery = ChromiumPageDiscoveryProductionProfile::from_profiles(&connection, topology);
    let wire = CapabilityWireProfile::new(
        ProtocolVersion::new(1, 0),
        FrameLimits::new(2 * 1024 * 1024, 0, 0, 2 * 1024 * 1024),
        FrameLimits::new(2 * 1024 * 1024, 0, 0, 2 * 1024 * 1024),
    );
    let limits = tests::limits();
    let profile = ChromiumAttachProductionProfile::new(
        connection,
        topology,
        routes,
        RuntimeLimits::new(128, 8),
        ServerSupervisorLimits::new(8)?,
    )
    .with_evaluation(EvaluationInstallationProfile::new(
        CdpEvaluationProductionProfile::new(ResourceScope::extension(94, 4), limits, 2),
        pwright_codec_evaluation::EvaluationCodecLimits::new(
            limits.source_bytes,
            limits.error_bytes,
            limits.value,
        )?,
        wire,
    ))
    .with_locator(LocatorInstallationProfile::new(
        CdpLocatorProductionProfile::new(ResourceScope::extension(94, 5), 2),
        pwright_codec_locator::LocatorCodecLimits::new(1024 * 1024),
        wire,
    ))
    .with_topology_events(events)
    .with_page_discovery(discovery)
    .with_utility_world(ChromiumUtilityWorldProfile::playwright_compatible())
    .with_execution_world_readiness(ChromiumExecutionWorldReadinessProfile::from_topology(
        ResourceScope::extension(94, 6),
        topology,
        NonZeroUsize::new(3).unwrap(),
    ))
    .with_extension_requirements(extra.combine_concurrent(access.requirements()));
    let requirements = profile.requirements();
    let mut resource_limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        resource_limits = resource_limits.with_class(class, bound.global, bound.per_scope);
    }
    Ok(ChromiumAttachFactory::new(
        profile,
        resource_limits,
        ObjectId::new(1),
    )?)
}

#[tokio::test]
#[ignore = "requires dedicated Chrome preflight and PWRIGHT_CDP_WS_URL"]
async fn production_pair_preparation_selects_delayed_hidden_label_and_retains_detached_identity()
-> Result<()> {
    // Arrange: real production route observations, one test-owned native
    // Context, and independent resolver/adoption/preparation installations.
    let resolution = ResolutionProfile::new(ResourceScope::extension(95, 1), 2)?;
    let adoption =
        ElementEvaluationProfile::new(ResourceScope::extension(95, 2), 1, tests::limits())?;
    let waiting = ResolutionWaitProfile::new(
        ResourceScope::extension(95, 3),
        1,
        [0, 20, 50, 100, 100, 500].map(Duration::from_millis),
    )?;
    let preparing = PreparationProfile::new(
        ResourceScope::extension(95, 4),
        1,
        Duration::from_millis(20),
    )?;
    let access_profile = ChromiumFrameWorldAccessProfile::new(
        ResourceScope::extension(95, 5),
        NonZeroUsize::new(3).unwrap(),
    )?;
    let required = resolution
        .requirements()
        .combine_concurrent(adoption.requirements())
        .combine_concurrent(waiting.requirements())
        .combine_concurrent(preparing.requirements());
    let endpoint = CdpWebSocketEndpoint::parse(std::env::var("PWRIGHT_CDP_WS_URL")?)?;
    let mut composition = factory(required, access_profile)?.attach(endpoint).await?;
    let runtime = composition.new_page_runtime()?;
    // Match the Node attachment's public attribution policy, without claiming
    // authority to close any discovered Context. Only the exact native Context
    // created below is disposed by this test.
    let mut driver = composition
        .topology_event_driver()?
        .context("missing driver")?
        .with_playwright_context_fallback();
    let access =
        ChromiumFrameWorldAccess::new(runtime.service(), access_profile, composition.resources())?;
    let access_ledger = composition
        .resources()
        .claim(access_profile.requirements())?;
    let access_baseline = access_ledger
        .scope_metrics(ResourceScope::extension(95, 5))?
        .unwrap()
        .units(ResourceClass::BackendStateBytes);
    let queue = CleanupQueue::new(4);
    // This connection installs exactly one resolver sharing this ID source.
    let attempts = ContinuableResolutionService::new(
        Arc::new(CdpLocatorResolutionService::new(
            resolution,
            composition.resources(),
            Arc::new(AtomicObjectIdSource::new(1)?),
        )?),
        queue.clone(),
    );
    let adoption =
        LocatorElementEvaluationService::new(adoption, composition.resources(), queue.clone())?;
    let waiter = StrictResolutionWaiter::new(waiting, composition.resources())?;
    let preparation = LocatorElementPreparationService::new(preparing, composition.resources())?;
    let browser = CdpSession::browser(composition.connection().clone());
    let context = browser.target_create_browser_context(None, None).await?;
    let outcome = tokio::time::timeout(Duration::from_secs(20), async {
        let target = browser.target_create_in_context("about:blank", &context).await?;
        driver.initialize_snapshot(&runtime, 4096).await?;
        let frame = loop {
            if let Some(frame) = composition.topology()?.topology().main_frame_object_id(&target) { break frame; }
            driver.receive_once(&runtime).await?;
        };
        let setup = drive(access.snapshot(frame), &mut driver, &runtime).await?;
        setup.client().runtime_evaluate_in_unique_context("globalThis.calls=0;globalThis.marker=42;document.body.innerHTML='<label for=late>Name</label>';setTimeout(()=>{const input=document.createElement('input');input.id='late';input.value='original';input.hidden=true;document.body.append(input)},50)", setup.main_unique_id()).await?;
        let client = setup.client(); let main = setup.main_unique_id().to_owned(); drop(setup);
        let worlds = Worlds { access: access.clone(), frame };
        let inputs = PreparationInputs { attempts: &attempts, waiter: &waiter, adoption: &adoption, worlds: &worlds, delays: &Delays };
        let stop = async { tokio::time::sleep(Duration::from_secs(3)).await; ResolutionWaitStop::Timeout };
        let prepared = drive(preparation.prepare(Selector::label("Name", true), &inputs, stop), &mut driver, &runtime).await?;
        let utility_id = prepared.data.as_ref().unwrap().source.as_ref().unwrap().element().object_id().to_owned();
        let main_id = prepared.data.as_ref().unwrap().adopted.as_ref().unwrap().handle().object_id().unwrap().to_owned();
        let utility_before = client.runtime_call_function_on(&utility_id, "function(){return this.id}", vec![]).await?;
        let main_before = client.runtime_call_function_on(&main_id, "function(){return this.id}", vec![]).await?;
        ensure!(utility_before["result"]["value"] == "late" && main_before["result"]["value"] == "late");
        ensure!(access_ledger.scope_metrics(ResourceScope::extension(95, 5))?.unwrap().units(ResourceClass::BackendStateBytes) == access_baseline + 8192, "prepared element did not retain its route budget");
        client.runtime_evaluate_in_unique_context("document.getElementById('late').remove();document.body.insertAdjacentHTML('beforeend','<input id=late value=replacement>')", &main).await?;

        // Act: source/argument materialization and one evaluation happen AFTER
        // resolution/adoption. The detached original cannot be reselected.
        let value = prepared.evaluate(&EvaluateParams::function_without_argument("element=>{calls++;return [element.value,element.isConnected,marker,calls]}" )).await?;

        // Assert real values and physical group reclamation, not just counters.
        let EvaluationValue::Array { items, .. } = value else { anyhow::bail!("expected array value"); };
        ensure!(items == vec![EvaluationValue::String("original".into()), EvaluationValue::Bool(false),
            EvaluationValue::Number(pwright_cap_evaluation::EvaluationNumber::new(42.0)?),
            EvaluationValue::Number(pwright_cap_evaluation::EvaluationNumber::new(1.0)?)]);
        let calls = client.runtime_evaluate_in_unique_context("calls", &main).await?;
        ensure!(calls["result"]["value"] == 1);
        ensure!(client.runtime_call_function_on(&utility_id, "function(){return this.id}", vec![]).await.is_err());
        ensure!(client.runtime_call_function_on(&main_id, "function(){return this.id}", vec![]).await.is_err());
        ensure!(access_ledger.scope_metrics(ResourceScope::extension(95, 5))?.unwrap().units(ResourceClass::BackendStateBytes) == access_baseline);
        let timed_out = drive(preparation.prepare(Selector::css("#never")?, &inputs, async { tokio::time::sleep(Duration::from_millis(50)).await; ResolutionWaitStop::Timeout }), &mut driver, &runtime).await;
        ensure!(timed_out.as_ref().is_err_and(|error| error.downcast_ref::<PreparationError>().is_some_and(|error| matches!(error, PreparationError::Waiting(ResolutionWaitError::Stopped(ResolutionWaitStop::Timeout))))));
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
    ensure!(alive && queue.metrics()?.reserved == 0);
    ensure!(
        access_ledger
            .scope_metrics(ResourceScope::extension(95, 5))?
            .unwrap()
            .units(ResourceClass::BackendStateBytes)
            == access_baseline
    );
    Ok(())
}
