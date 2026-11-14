//! Ignored borrowed-CDP attribution probe. This is test-only and does not
//! expose scope accounting through the Node or production host API.

use std::sync::Arc;

use pwright_core::{ObjectId, ResourceClass, ResourceLedger, ResourceRequirements, ResourceScope};

use crate::evaluation::EvaluationValue;
use crate::locator_query::{QueryBudget, QueryLimits};
use crate::{HostError, HostSession};

#[derive(Debug, Default)]
struct ScopeUsage {
    leases: usize,
    event_items: u64,
    event_bytes: u64,
}

fn scope_usage(ledger: &ResourceLedger, scope: ResourceScope) -> ScopeUsage {
    let usage = ledger
        .scope_metrics(scope)
        .expect("scope accounting is readable");
    usage.map_or_else(ScopeUsage::default, |usage| ScopeUsage {
        leases: usage.active_leases,
        event_items: usage.units(ResourceClass::EventItem),
        event_bytes: usage.units(ResourceClass::EventBytes),
    })
}

#[derive(Debug)]
struct ScopeSnapshot {
    connected: bool,
    leases: usize,
    live_scopes: usize,
    event_items: u64,
    event_bytes: u64,
    connection: ScopeUsage,
    owned_page: Option<ScopeUsage>,
}

fn snapshot(host: &HostSession, owned_page: Option<ObjectId>) -> ScopeSnapshot {
    let session = host.metrics().expect("session metrics are readable");
    // A zero-requirement claim clones the existing ledger without reserving
    // resources or widening the production host/Node metrics surface.
    let ledger = host
        .resources
        .claim(ResourceRequirements::none())
        .expect("read-only ledger claim fits");
    let resources = ledger.metrics().expect("ledger is readable");
    ScopeSnapshot {
        connected: session.connected,
        leases: resources.active_leases,
        live_scopes: resources.live_scopes,
        event_items: resources.class(ResourceClass::EventItem).current,
        event_bytes: resources.class(ResourceClass::EventBytes).current,
        connection: scope_usage(&ledger, ResourceScope::Connection),
        owned_page: owned_page.map(|page| scope_usage(&ledger, ResourceScope::Object(page))),
    }
}

#[tokio::test]
#[ignore = "requires chrome-cdp preflight, an idle owned-context lane, and PWRIGHT_TEST_CDP"]
async fn borrowed_close_attributes_retained_events_to_connection_or_owned_page() {
    // Arrange: attach only to the dedicated loopback CDP Chrome and create an
    // owned Context; never close a foreign Page or the borrowed browser.
    let endpoint = std::env::var("PWRIGHT_TEST_CDP").expect("explicit dedicated Chrome endpoint");
    let url = crate::endpoint::validate_endpoint(&endpoint).expect("valid CDP endpoint");
    assert_eq!(url.host_str(), Some("127.0.0.1"));
    assert_eq!(url.port(), Some(9222));
    let host = Arc::new(
        HostSession::connect_over_cdp_with_lifecycle_events(&endpoint, 5000)
            .await
            .expect("borrowed CDP attachment succeeds"),
    );
    let context = match host.new_context().await {
        Ok(context) => context,
        Err(error) => {
            host.close()
                .await
                .expect("failed fixture attachment retires");
            panic!("owned Context creation failed: {error}");
        }
    };
    let context_id = context.id.clone();
    drop(context);
    let exercise = async {
        let page = host.new_page(&context_id).await?;
        let page_id = page
            .id
            .parse::<u64>()
            .map(ObjectId::new)
            .map_err(|_| HostError::InvalidArgument("invalid fixture Page ID".into()))?;
        let frame_id = page.main_frame_id.clone();
        let page_text_id = page.id.clone();
        drop(page);
        host.page_set_content(
            &page_text_id,
            "<button id='scope-diagnostic'>Click</button>".into(),
            "load",
            5000,
        )
        .await?;
        let queries = QueryBudget::new(QueryLimits {
            recipes: 1,
            bytes: 4096,
        })?;
        let query = queries
            .prepare_leaf("#scope-diagnostic".len())?
            .css("#scope-diagnostic".into())?;
        let locator = host.locator(&frame_id, query)?;
        locator
            .dispatch_event("click".into(), EvaluationValue::Null, 5000)
            .await?;
        Ok::<_, HostError>((page_id, snapshot(&host, Some(page_id))))
    }
    .await;
    let owned_page = exercise.as_ref().ok().map(|(page, _)| *page);

    // Act: retire only this test's Context, then the borrowed CDP attachment.
    let context_close = host.close_context(&context_id).await;
    let after_context = snapshot(&host, owned_page);
    let session_close = host.close().await;
    let after_session = snapshot(&host, owned_page);

    // Assert: retain the per-scope evidence even if a residual lease makes
    // this intermittent diagnostic fail. Other scopes remain unclassified.
    eprintln!(
        "borrowed scope diagnostic: before={:?}; after_context={after_context:?}; after_session={after_session:?}",
        exercise.as_ref().ok().map(|(_, before)| before)
    );
    exercise.expect("static Page dispatch succeeds");
    context_close.expect("owned Context closes");
    session_close.expect("borrowed CDP attachment closes");
    assert!(!after_session.connected, "{after_session:?}");
    assert_eq!(after_session.leases, 0, "{after_session:?}");
    assert_eq!(after_session.live_scopes, 0, "{after_session:?}");
    assert_eq!(after_session.event_items, 0, "{after_session:?}");
    assert_eq!(after_session.event_bytes, 0, "{after_session:?}");
    assert_eq!(after_session.connection.leases, 0, "{after_session:?}");
    assert_eq!(after_session.connection.event_items, 0, "{after_session:?}");
    assert_eq!(after_session.connection.event_bytes, 0, "{after_session:?}");
    let owned_page = after_session
        .owned_page
        .as_ref()
        .expect("Page scope was recorded");
    assert_eq!(owned_page.leases, 0, "{after_session:?}");
    assert_eq!(owned_page.event_items, 0, "{after_session:?}");
    assert_eq!(owned_page.event_bytes, 0, "{after_session:?}");
}
