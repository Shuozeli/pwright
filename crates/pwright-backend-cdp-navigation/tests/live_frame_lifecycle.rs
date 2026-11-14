//! Native event adapter verification, not public Node Frame.goto conformance.
#![cfg(feature = "frame-lifecycle")]

use pwright_backend_cdp_navigation::{
    CdpNavigationService,
    frame_lifecycle::{FrameLifecycleProfile, LifecycleSnapshot, native::*},
};
use pwright_cap_navigation::BeginNavigationParams;
use pwright_cdp::{CdpConnection, CdpEventReceiver, CdpSession};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
};
use serde_json::json;
use std::{
    error::Error,
    sync::Arc,
    time::{Duration, Instant},
};

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

// Test-owned, single-renderer routing port. Production topology integration and
// cross-renderer initialization are deliberately not simulated by this harness.
struct Routes {
    session: String,
    frames: Vec<(String, ObjectId)>,
    next: u64,
}
impl CdpLifecycleRoutes for Routes {
    fn contains_session(&self, session: &str) -> bool {
        session == self.session
    }
    fn event_frame(&self, session: &str, native: &str) -> Option<ObjectId> {
        self.contains_session(session)
            .then(|| {
                self.frames
                    .iter()
                    .find(|(id, _)| id == native)
                    .map(|(_, id)| *id)
            })
            .flatten()
    }
    fn request_frame(&self, session: &str, native: Option<&str>) -> Option<ObjectId> {
        native.and_then(|id| self.event_frame(session, id))
    }
    fn accepts_document_handoff(&self, _: ObjectId, _: &str) -> bool {
        false
    }
}

struct Observer {
    state: CdpFrameLifecycle,
    routes: Routes,
    events: CdpEventReceiver,
    clock: Instant,
}
impl Observer {
    fn snapshot(&self, frame: ObjectId) -> TestResult<LifecycleSnapshot> {
        self.state
            .snapshot(frame)?
            .ok_or_else(|| "missing observed Frame".into())
    }
    async fn settle(&mut self, frame: ObjectId, loader: Option<&str>) -> TestResult<()> {
        let mut committed = false;
        let mut request = None;
        let mut response = false;
        let mut finished = false;
        loop {
            let wake = self
                .state
                .next_deadline()?
                .map(|d| self.clock + d)
                .unwrap_or_else(|| Instant::now() + Duration::from_secs(5));
            let input = tokio::select! {
                biased;
                input = self.events.recv() => Some(input.map_err(|error| format!("native event loss: {error:?}"))?),
                () = tokio::time::sleep_until(wake.into()) => None,
            };
            let now = self.clock.elapsed();
            self.state.advance(now)?;
            if let Some(input) = input {
                if input.session_id.as_deref() != Some(self.routes.session.as_str()) {
                    continue;
                }
                if input.method == "Page.frameAttached" {
                    let native = input.params["frameId"]
                        .as_str()
                        .ok_or("missing attached id")?;
                    let parent = input.params["parentFrameId"]
                        .as_str()
                        .ok_or("missing parent id")?;
                    let parent = self
                        .routes
                        .event_frame(&self.routes.session, parent)
                        .ok_or("unknown parent")?;
                    if self
                        .routes
                        .event_frame(&self.routes.session, native)
                        .is_none()
                    {
                        if self.routes.frames.len() == 8 || native.len() > 128 {
                            return Err("test route capacity".into());
                        }
                        self.routes.next += 1;
                        let id = ObjectId::new(self.routes.next);
                        self.state.attach(now, id, Some(parent))?;
                        self.routes.frames.push((native.to_owned(), id));
                    }
                } else if input.method == "Page.frameDetached" {
                    if input.params["reason"] != "remove" {
                        return Err("unexpected renderer swap in local fixture".into());
                    }
                    let native = input.params["frameId"]
                        .as_str()
                        .ok_or("missing detached id")?;
                    if let Some(id) = self.routes.event_frame(&self.routes.session, native) {
                        self.state.detach(now, id)?;
                    }
                }
                match self.state.apply(now, 1, &input, &self.routes)? {
                    NativeLifecycleNotice::Committed {
                        frame: id,
                        loader: actual,
                        ..
                    } if id == frame && Some(actual) == loader => committed = true,
                    NativeLifecycleNotice::SameDocument { frame: id, .. }
                        if id == frame && loader.is_none() =>
                    {
                        let state = self.snapshot(frame)?;
                        assert!(state.load && state.dom_content_loaded && state.network_idle);
                        return Ok(());
                    }
                    NativeLifecycleNotice::RequestStarted {
                        frame: id,
                        request: current,
                        document_loader,
                        ..
                    } if id == frame && document_loader == loader => request = Some(current),
                    NativeLifecycleNotice::Response {
                        request: current,
                        response: value,
                        ..
                    } if Some(current) == request => {
                        assert_eq!(value["status"], 200);
                        response = true;
                    }
                    NativeLifecycleNotice::RequestFinished {
                        request: current, ..
                    } if Some(current) == request => finished = true,
                    NativeLifecycleNotice::RequestFailed { error_text, .. } => {
                        return Err(error_text.to_owned().into());
                    }
                    _ => {}
                }
                // Commit/removal prunes descendants. Keep this external test port
                // coherent before accepting another envelope, without fallback.
                for i in (0..self.routes.frames.len()).rev() {
                    if self.state.snapshot(self.routes.frames[i].1)?.is_none() {
                        self.routes.frames.remove(i);
                    }
                }
            }
            let state = self.snapshot(frame)?;
            if committed
                && response
                && finished
                && state.load
                && state.dom_content_loaded
                && state.network_idle
            {
                return Ok(());
            }
        }
    }
}

async fn exercise(client: Arc<CdpSession>, fixture: &str) -> TestResult<()> {
    // Arrange: install the observer before enabling events or navigating.
    let profile = CdpFrameLifecycleProfile::new(
        FrameLifecycleProfile::new(ResourceScope::extension(92, 1), 8, 64)?,
        ResourceScope::extension(92, 2),
        128,
    )?;
    let requirements = profile.requirements();
    let bytes = requirements.class(ResourceClass::BackendStateBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        ),
        requirements,
    )?;
    let events = client.subscribe_composable_events()?;
    let tree = client.page_get_frame_tree().await?;
    let root = ObjectId::new(1);
    let mut state = CdpFrameLifecycle::new(profile, 1, &resources)?;
    state.attach(Duration::ZERO, root, None)?;
    let mut observer = Observer {
        state,
        events,
        clock: Instant::now(),
        routes: Routes {
            session: client.session_id().ok_or("missing session")?.to_owned(),
            frames: vec![(tree.frame.id.clone(), root)],
            next: 1,
        },
    };
    client.page_enable().await?;
    client
        .send("Page.setLifecycleEventsEnabled", json!({"enabled":true}))
        .await?;
    client.network_enable().await?;
    let service = CdpNavigationService::new(client.clone());

    // Act: native root navigation, followed by selected child document and hash.
    let receipt = service
        .begin_frame_navigation(
            &tree.frame.id,
            BeginNavigationParams::new(format!("{fixture}frame-navigation.html?mode=local")),
        )
        .await?;
    observer
        .settle(
            root,
            Some(receipt.loader_id.as_deref().ok_or("missing root loader")?),
        )
        .await?;
    assert_eq!(observer.routes.frames.len(), 4);
    println!("PASS root plus nested/sibling HTTP lifecycle and response completion");
    let tree = client.page_get_frame_tree().await?;
    let children = tree.child_frames.as_ref().ok_or("missing children")?;
    let child = children
        .iter()
        .find(|f| f.frame.name.as_deref() == Some("navigation-target"))
        .ok_or("missing target")?;
    let sibling = children
        .iter()
        .find(|f| f.frame.name.as_deref() == Some("unaffected-sibling"))
        .ok_or("missing sibling")?;
    let selected = observer
        .routes
        .event_frame(&observer.routes.session, &child.frame.id)
        .ok_or("missing portable child")?;
    let sibling_id = observer
        .routes
        .event_frame(&observer.routes.session, &sibling.frame.id)
        .ok_or("missing portable sibling")?;
    let root_before = observer.snapshot(root)?;
    let sibling_before = observer.snapshot(sibling_id)?;
    let previous_document = observer.snapshot(selected)?.document;
    let url = format!("{fixture}empty.html?node067=child");
    let receipt = service
        .begin_frame_navigation(&child.frame.id, BeginNavigationParams::new(&url))
        .await?;
    observer
        .settle(
            selected,
            Some(receipt.loader_id.as_deref().ok_or("missing child loader")?),
        )
        .await?;
    assert_ne!(observer.snapshot(selected)?.document, previous_document);
    assert_eq!(observer.routes.frames.len(), 3);
    assert_eq!(observer.snapshot(root)?, root_before);
    assert_eq!(observer.snapshot(sibling_id)?, sibling_before);
    println!("PASS child document lifecycle, descendant removal, parent/sibling isolation");
    let before_hash = observer.snapshot(selected)?;
    let receipt = service
        .begin_frame_navigation(
            &child.frame.id,
            BeginNavigationParams::new(format!("{url}#same")),
        )
        .await?;
    assert!(receipt.same_document);
    observer.settle(selected, None).await?;
    // Assert: hash retains the current document's lifecycle, not a fresh load.
    assert_eq!(observer.snapshot(selected)?, before_hash);
    assert_eq!(observer.snapshot(root)?, root_before);
    assert_eq!(observer.snapshot(sibling_id)?, sibling_before);
    println!("PASS hash preserves document generation and all lifecycle bits");
    drop(observer);
    assert_eq!(resources.metrics()?.active_leases, 0);
    assert_eq!(
        resources
            .metrics()?
            .class(ResourceClass::BackendStateBytes)
            .current,
        0
    );
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight and serve-lib fixture URL"]
async fn native_frame_events_drive_rust_lifecycle() -> TestResult<()> {
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    let fixture = std::env::var("PWRIGHT_FRAME_LIFECYCLE_FIXTURE")?;
    assert!(endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/"));
    assert!(
        fixture.starts_with("http://macbookpro-yuacx.tail8f3b66.ts.net:") && fixture.ends_with('/')
    );
    let connection = CdpConnection::connect(&endpoint).await?;
    let browser = CdpSession::browser(connection.clone());
    // Explicit recovery of this harness's previously interrupted run only.
    // Validate the exact Context and every target before touching Chrome state.
    if let Ok(recover) = std::env::var("PWRIGHT_FRAME_LIFECYCLE_RECOVER_CONTEXT") {
        let targets = browser.send("Target.getTargets", json!({})).await?;
        let owned: Vec<_> = targets["targetInfos"]
            .as_array()
            .ok_or("missing targets")?
            .iter()
            .filter(|t| t["browserContextId"].as_str() == Some(recover.as_str()))
            .collect();
        if browser
            .target_get_browser_contexts()
            .await?
            .contains(&recover)
        {
            assert_eq!(owned.len(), 1);
            assert_eq!(owned[0]["type"], "page");
            assert_eq!(
                owned[0]["url"],
                format!("{fixture}frame-navigation.html?mode=local")
            );
            browser.target_dispose_browser_context(&recover).await?;
        } else {
            assert!(owned.is_empty());
        }
        assert!(
            !browser
                .target_get_browser_contexts()
                .await?
                .contains(&recover)
        );
        println!("verified recovered lifecycle Context absent {recover}");
    }
    let owner = browser.target_create_browser_context(None, None).await?;
    println!("owned lifecycle Context {owner}");
    let run_owner = owner.clone();
    let mut run = tokio::spawn(async move {
        let browser = CdpSession::browser(connection.clone());
        let target = browser
            .target_create_in_context("about:blank", &run_owner)
            .await?;
        let session = browser.target_attach(&target).await?;
        exercise(
            Arc::new(CdpSession::new(connection, session, target)),
            &fixture,
        )
        .await
    });
    // Join captures assertion panics; timeout aborts the run before disposal.
    let outcome = tokio::time::timeout(Duration::from_secs(25), &mut run).await;
    if outcome.is_err() {
        run.abort();
        assert!(run.await.is_err());
    }
    tokio::time::timeout(
        Duration::from_secs(10),
        browser.target_dispose_browser_context(&owner),
    )
    .await??;
    assert!(
        !browser
            .target_get_browser_contexts()
            .await?
            .contains(&owner)
    );
    println!("disposed lifecycle Context {owner}");
    outcome???;
    Ok(())
}
