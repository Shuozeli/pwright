#![cfg(feature = "extra-headers")]
//! Native Rust host wire verification, not a public Node compatibility claim.
use pwright_cdp::{CdpConnection, CdpSession};
use pwright_node_host::{
    HostSession, PageDescriptor,
    extra_headers::{HeaderRef, NativeHeaderTarget},
};
use std::{error::Error, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    task::JoinSet,
};

type Result<T> = std::result::Result<T, Box<dyn Error + Send + Sync>>;

async fn respond(mut stream: TcpStream) -> Result<()> {
    let mut bytes = [0; 16 * 1024];
    let mut count = 0;
    loop {
        if count == bytes.len() {
            return Err("fixture request too large".into());
        }
        let read = stream.read(&mut bytes[count..]).await?;
        if read == 0 {
            return Ok(());
        }
        count += read;
        if bytes[..count].windows(4).any(|v| v == b"\r\n\r\n") {
            break;
        }
    }
    let request = std::str::from_utf8(&bytes[..count])?;
    let header = |name: &str| {
        request
            .lines()
            .filter_map(|line| line.split_once(':'))
            .find(|(key, _)| key.eq_ignore_ascii_case(name))
            .map_or("absent", |(_, value)| value.trim())
    };
    let body = "<!doctype html><title>headers</title>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\nX-Observed-Layer: {}\r\nX-Observed-Context: {}\r\n\r\n{body}",
        body.len(),
        header("x-layer"),
        header("x-context")
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}

async fn fixture(listener: TcpListener) -> Result<()> {
    let mut peers = JoinSet::new();
    loop {
        tokio::select! {
            connection = listener.accept(), if peers.len() < 8 => {
                let (stream, _) = connection?;
                peers.spawn(async { tokio::time::timeout(Duration::from_secs(3), respond(stream)).await });
            }
            result = peers.join_next(), if !peers.is_empty() => {
                match result {
                    Some(Ok(Ok(result))) => result?,
                    Some(Ok(Err(_))) => {} // Chrome may preconnect without sending a request.
                    Some(Err(error)) => return Err(error.into()),
                    None => {}
                }
            }
        }
    }
}

async fn observe(
    host: &HostSession,
    page: &PageDescriptor,
    base: &str,
    case: &str,
    layer: &str,
    context: &str,
) -> Result<()> {
    let navigation = host
        .page_goto(&page.id, format!("{base}/{case}"), "load", None, 5000)
        .await?;
    let response = navigation.response.ok_or("missing document response")?;
    for (name, expected) in [("x-observed-layer", layer), ("x-observed-context", context)] {
        let actual = response
            .headers
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str());
        if actual != Some(expected) {
            return Err(format!("{case}: {name}: expected {expected:?}, actual {actual:?}").into());
        }
    }
    println!("PASS {case}: layer={layer}, context={context}");
    Ok(())
}

async fn preparation_order(
    host: &Arc<HostSession>,
    page: &PageDescriptor,
    base: &str,
) -> Result<()> {
    let old = format!("{base}/old");
    let next = format!("{base}/next");
    // Arrange
    host.set_page_extra_http_headers_native(
        &page.id,
        &[HeaderRef {
            name: "Referer",
            value: &old,
        }],
    )
    .await?;
    // Act: do not poll the setter's returned completion future.
    let setter = host.start_set_extra_http_headers_native(
        NativeHeaderTarget::Page(&page.id),
        &[HeaderRef {
            name: "Referer",
            value: &next,
        }],
    );
    let navigation = host
        .start_page_goto(page.id.clone(), base.into(), "load".into(), Some(old), 5000)
        .await;
    // Assert: preparation, not worker scheduling, establishes the conflict.
    assert!(
        matches!(navigation, Err(pwright_node_host::HostError::Backend(message)) if message.contains("already specified"))
    );
    setter.await?;

    // Arrange / Act: capture a conflict, then change metadata before polling it.
    let navigation = host.start_page_goto(
        page.id.clone(),
        base.into(),
        "load".into(),
        Some(format!("{base}/later")),
        5000,
    );
    let setter = host.start_set_extra_http_headers_native(NativeHeaderTarget::Page(&page.id), &[]);
    // Assert
    assert!(
        matches!(navigation.await, Err(pwright_node_host::HostError::Backend(message)) if message.contains("already specified"))
    );
    setter.await?;

    // Arrange: eight prepared but unpolled navigations retain their admission.
    let held: Vec<_> = (0..8)
        .map(|_| host.start_page_goto(page.id.clone(), base.into(), "load".into(), None, 5000))
        .collect();
    // Act
    let excess = host
        .start_page_goto(page.id.clone(), base.into(), "load".into(), None, 5000)
        .await;
    // Assert: dropping unpolled futures releases every preparation slot.
    assert!(matches!(
        excess,
        Err(pwright_node_host::HostError::ResourceExhausted(_))
    ));
    drop(held);
    host.start_page_goto(page.id.clone(), base.into(), "load".into(), None, 5000)
        .await?;
    println!(
        "PASS synchronous preparation: both invocation orders and unpolled navigation reclamation"
    );
    Ok(())
}

async fn exercise(host: &Arc<HostSession>, base: &str, first: PageDescriptor) -> Result<()> {
    let context = first.context_id.clone();
    preparation_order(host, &first, base).await?;
    // Arrange: isolated Context and one previously attached Page.
    let old = [
        HeaderRef {
            name: "X-Layer",
            value: "context-one",
        },
        HeaderRef {
            name: "X-Context",
            value: "one",
        },
    ];
    // Act / Assert: current sessions, later attachment, overrides and clearing.
    host.set_context_extra_http_headers_native(&context, &old)
        .await?;
    observe(host, &first, base, "current-page", "context-one", "one").await?;
    let second = host.new_page(&context).await?;
    observe(
        host,
        &second,
        base,
        "new-page-inherits",
        "context-one",
        "one",
    )
    .await?;
    host.set_page_extra_http_headers_native(
        &first.id,
        &[
            HeaderRef {
                name: "X-Layer",
                value: "first",
            },
            HeaderRef {
                name: "x-layer",
                value: "page",
            },
        ],
    )
    .await?;
    observe(
        host,
        &first,
        base,
        "duplicate-page-last-wins",
        "page",
        "one",
    )
    .await?;
    host.set_context_extra_http_headers_native(
        &context,
        &[
            HeaderRef {
                name: "X-Layer",
                value: "context-two",
            },
            HeaderRef {
                name: "X-Context",
                value: "two",
            },
        ],
    )
    .await?;
    observe(
        host,
        &first,
        base,
        "context-update-keeps-page",
        "page",
        "two",
    )
    .await?;
    observe(
        host,
        &second,
        base,
        "context-update-second-page",
        "context-two",
        "two",
    )
    .await?;
    host.set_page_extra_http_headers_native(&first.id, &[])
        .await?;
    observe(host, &first, base, "clear-page", "context-two", "two").await?;
    host.set_context_extra_http_headers_native(&context, &[])
        .await?;
    observe(
        host,
        &first,
        base,
        "clear-context-first",
        "absent",
        "absent",
    )
    .await?;
    observe(
        host,
        &second,
        base,
        "clear-context-second",
        "absent",
        "absent",
    )
    .await?;
    // Caller syntax rejection must leave the native state and driver usable.
    if host
        .set_page_extra_http_headers_native(
            &first.id,
            &[HeaderRef {
                name: "bad name",
                value: "bad",
            }],
        )
        .await
        .is_ok()
    {
        return Err("invalid name unexpectedly accepted".into());
    }
    observe(
        host,
        &first,
        base,
        "invalid-input-no-mutation",
        "absent",
        "absent",
    )
    .await?;
    if host
        .set_page_extra_http_headers_native(&context, &old)
        .await
        .is_ok()
    {
        return Err("Context ID accepted as Page ID".into());
    }
    // Drive more than the sparse Page metadata capacity through real retirement.
    host.close_page(&first.id).await?;
    host.close_page(&second.id).await?;
    for index in 0..18 {
        let page = host.new_page(&context).await?;
        host.set_page_extra_http_headers_native(&page.id, &old)
            .await?;
        host.close_page(&page.id).await?;
        println!("PASS retired Page slot {index}");
    }
    if host.metrics()?.topology_drivers != 1 {
        return Err("unexpected extra driver".into());
    }
    Ok(())
}

#[tokio::test]
#[ignore = "requires dedicated Chrome/Pwright preflight and explicit Tailscale fixture address"]
async fn selected_host_applies_headers_and_reclaims_retired_page_settings() -> Result<()> {
    let endpoint = std::env::var("PWRIGHT_CDP_WS_URL")?;
    let bind = std::env::var("PWRIGHT_TEST_BIND")?;
    let hostname = std::env::var("PWRIGHT_TEST_HOST")?;
    if !endpoint.starts_with("ws://127.0.0.1:9222/devtools/browser/")
        || !hostname.ends_with(".ts.net")
    {
        return Err("dedicated loopback CDP and full MagicDNS required".into());
    }
    let listener = TcpListener::bind((bind.as_str(), 0)).await?;
    let base = format!("http://{hostname}:{}", listener.local_addr()?.port());
    let server = tokio::spawn(fixture(listener));
    let connection = CdpConnection::connect(&endpoint).await?;
    let browser = CdpSession::browser(connection.clone());
    let host = Arc::new(HostSession::connect_over_cdp(&endpoint, 15000).await?);
    let context = host.new_context().await?;
    // Never use the CDP fallback Context: it can include unrelated shared Pages.
    let mut native_context = None;
    let initial_url = format!("{base}/initial");
    let outcome = tokio::time::timeout(Duration::from_secs(90), async {
        let first = host.new_page(&context.id).await?;
        host.page_goto(&first.id, initial_url.clone(), "load", None, 5000)
            .await?;
        let targets: pwright_cdp::generated::target::GetTargetsReturns = serde_json::from_value(
            browser
                .send(
                    "Target.getTargets",
                    serde_json::to_value(
                        pwright_cdp::generated::target::GetTargetsParams::default(),
                    )?,
                )
                .await?,
        )?;
        native_context = targets
            .target_infos
            .into_iter()
            .find(|target| target.url == initial_url)
            .and_then(|target| target.browser_context_id);
        if native_context.is_none() {
            return Err("native cleanup identity missing".into());
        }
        println!(
            "test-owned native Context {:?}; fixture {base}",
            native_context
        );
        exercise(&host, &base, first).await
    })
    .await;
    let cleanup =
        tokio::time::timeout(Duration::from_secs(15), host.close_context(&context.id)).await;
    if !matches!(cleanup, Ok(Ok(())))
        && let Some(context) = &native_context
    {
        browser.target_dispose_browser_context(context).await?;
    }
    drop(context);
    let close = tokio::time::timeout(Duration::from_secs(15), host.close()).await;
    server.abort();
    let server_result = server.await;
    drop(browser);
    drop(connection);
    if let Ok(result) = server_result {
        result?;
    }
    outcome??;
    cleanup??;
    close??;
    let metrics = host.metrics()?;
    if metrics.active_leases != 0 || metrics.topology_drivers != 0 || metrics.active_operations != 0
    {
        return Err(format!("host retained owners after close: {metrics:?}").into());
    }
    println!("PASS host close: zero leases, operations and topology drivers");
    Ok(())
}
