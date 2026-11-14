//! Exercises actual SessionTeardown -> FrameTrees -> terminal publication.
use crate::{
    HostSession, LifecycleDelivery, LifecycleNotice, PublicationDrainError, PublicationProgress,
};
use pwright_cdp::{CdpConnection, CdpSession, connection::ConnectionConfig};
use pwright_core::ObjectId;
use std::{error::Error, time::Duration};

#[tokio::test]
#[ignore = "requires PWRIGHT_CDP_WS_URL for dedicated loopback Chrome; owns one Context only"]
async fn native_session_teardown_publishes_independent_page_and_context_groups()
-> Result<(), Box<dyn Error>> {
    // Arrange
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    if !endpoint.starts_with("ws://127.0.0.1:9222/") {
        return Err("dedicated loopback Chrome endpoint required".into());
    }
    let connection = CdpConnection::connect_with_config(
        &endpoint,
        ConnectionConfig {
            timeout: Duration::from_secs(5),
            ..ConnectionConfig::default()
        },
    )
    .await?;
    let browser = CdpSession::browser(connection);
    let host = HostSession::connect_over_cdp_with_terminal_events(&endpoint, 15_000).await?;
    let context = tokio::time::timeout(Duration::from_secs(10), host.new_context()).await??;
    let context_id = ObjectId::new(context.id.parse()?);
    // Read the exact native route only for cleanup ownership; behavioral
    // assertions below use public metadata and drain APIs, not native internals.
    let native_context = {
        let state = host.state.lock().unwrap();
        let topology = state.as_ref().unwrap().composition.shared_topology();
        topology
            .lock()
            .unwrap()
            .topology()
            .browser_context_route(context_id)
            .ok_or("owned Context route missing")?
            .browser_context_id()
            .to_owned()
    };
    let result = tokio::time::timeout(Duration::from_secs(25), async {
        let first = host.new_page(&context.id).await?;
        let second = host.new_page(&context.id).await?;
        let publications = host.frame_publications();
        publications.fence()?.publish()?;
        let retained = publications.published_collections()?.ok_or("no initial collection")?;
        let old = publications.fence()?;

        // Act: this is actual native host shutdown, not a manually built group.
        host.close().await?;
        old.publish()?;
        if first.frame_tree.is_closed()? || second.frame_tree.is_closed()? {
            return Err("terminal state escaped its publication watermark".into());
        }
        let mut owner = publications.fence()?.try_begin()?;
        let mut owned_pages = 0;
        let mut owned_contexts = 0;
        let mut total = 0;
        loop {
            match owner.publish_next() {
                Ok(PublicationProgress::Published) => {},
                Ok(PublicationProgress::Complete) => break,
                Err(PublicationDrainError::NoticePending) => {
                    let notice = owner.with_next_notice(|value| value.notice)?;
                    let committed = owner.commit_notice()?;
                    if notice != committed { return Err("notice identity changed".into()); }
                    let visible = publications.published_collections()?.ok_or("missing callback view")?;
                    let mut delivery = LifecycleDelivery::Delivered;
                    match notice {
                        LifecycleNotice::PageClose { context, page } if context == context_id => {
                            owned_pages += 1;
                            let closed = usize::from(first.frame_tree.is_closed()?) + usize::from(second.frame_tree.is_closed()?);
                            if closed != owned_pages
                                || visible.pages().iter().any(|entry| entry.id == page)
                                || !visible.contexts().iter().any(|entry| entry.id == context_id)
                            { return Err("Page callback state is not incremental".into()); }
                            if owned_pages == 1 { delivery = LifecycleDelivery::ListenerThrew; }
                        }
                        LifecycleNotice::ContextClose { context } if context == context_id => {
                            owned_contexts += 1;
                            if owned_pages != 2 || visible.contexts().iter().any(|entry| entry.id == context_id) {
                                return Err("Context closed before its Page groups".into());
                            }
                        }
                        LifecycleNotice::PageClose { .. } | LifecycleNotice::ContextClose { .. } => {},
                        LifecycleNotice::ContextPage { .. } | LifecycleNotice::Popup { .. } => return Err("creation during terminal publication".into()),
                        LifecycleNotice::PageDomContentLoaded { .. }
                        | LifecycleNotice::PageLoad { .. }
                        | LifecycleNotice::PageMainFrameNavigated { .. }
                        | LifecycleNotice::PageConsole { .. }
                        | LifecycleNotice::PageError { .. } => return Err("Page event during terminal publication".into()),
                        #[cfg(feature = "frame-events")]
                        LifecycleNotice::PageFrameAttached { .. }
                        | LifecycleNotice::PageFrameNavigated { .. } => return Err("Page event during terminal publication".into()),
                        #[cfg(feature = "frame-detach-events")]
                        LifecycleNotice::PageFrameDetached { .. } => return Err("Page event during terminal publication".into()),
                        #[cfg(feature = "page-dialogs")]
                        LifecycleNotice::PageDialog { .. } => return Err("Page event during terminal publication".into()),
                        #[cfg(feature = "file-chooser-events")]
                        LifecycleNotice::PageFileChooser { .. } => return Err("Page event during terminal publication".into()),
                    }
                    if !publications.connection_is_published_open()? { return Err("early disconnected state".into()); }
                    owner.acknowledge_notice(delivery)?;
                    total += 1;
                    if total > 32 { return Err("terminal group bound exceeded".into()); }
                }
                Err(error) => return Err(Box::<dyn Error>::from(error)),
            }
        }
        // Assert: throwing the first owned Page event did not suppress others.
        if owned_pages != 2 || owned_contexts != 1 { return Err("owned terminal groups missing".into()); }
        if retained.pages().iter().filter(|entry| entry.context == context_id).count() != 2 {
            return Err("retained membership was mutated".into());
        }
        if !publications.publish_disconnect()? || publications.publish_disconnect()? {
            return Err("disconnect was not published exactly once".into());
        }
        // A retained host can be asked to close again without enqueuing duplicates.
        host.close().await?;
        if !publications.is_drained()? { return Err("second close replayed terminal groups".into()); }
        eprintln!("Native terminal source: owned Pages={owned_pages}, owned Contexts={owned_contexts}, total groups={total}");
        Ok::<(), Box<dyn Error>>(())
    }).await;
    // Always terminate the same host and remove only its exact native Context,
    // including on observation timeout. Never restart or close external Chrome.
    let shutdown = tokio::time::timeout(Duration::from_secs(10), host.close()).await;
    if browser
        .target_get_browser_contexts()
        .await?
        .contains(&native_context)
    {
        browser
            .target_dispose_browser_context(&native_context)
            .await?;
    }
    browser.target_get_targets().await?;
    shutdown??;
    result??;
    Ok(())
}
