//! Bounded dynamic streaming response: headers commit a document, but load
//! cannot fire until the test explicitly completes the HTTP body.
use super::*;
use pwright_cap_navigation::{BeginNavigationParams, WaitUntil};
use pwright_node_host::HostError;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::oneshot,
    task::JoinHandle,
};

struct Document {
    url: String,
    requested: oneshot::Receiver<()>,
    finish: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<Result<()>>>,
}
impl Drop for Document {
    fn drop(&mut self) {
        if let Some(task) = &self.task {
            task.abort();
        }
    }
}
impl Document {
    async fn new() -> Result<Self> {
        // Explicit Tailscale binding, no loopback/LAN static-file server.
        let listener = TcpListener::bind(("100.78.97.63", 0)).await?;
        let port = listener.local_addr()?.port();
        let (requested, request) = oneshot::channel();
        let (finish, finished) = oneshot::channel();
        let task = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut header = [0; 4096];
            let mut used = 0;
            while !header[..used].windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                if used == header.len() {
                    return Err("request header capacity exceeded".into());
                }
                let read = stream.read(&mut header[used..]).await?;
                if read == 0 {
                    return Err("request closed before headers".into());
                }
                used += read;
            }
            stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").await?;
            let body = b"<!doctype html><title>Pending document</title><link rel=icon href=data:,><body>waiting";
            stream
                .write_all(format!("{:x}\r\n", body.len()).as_bytes())
                .await?;
            stream.write_all(body).await?;
            stream.write_all(b"\r\n").await?;
            requested.send(()).map_err(|_| "request observer dropped")?;
            finished.await?;
            stream.write_all(b"0\r\n\r\n").await?;
            Ok(())
        });
        Ok(Self {
            url: format!("http://macbookpro-yuacx.tail8f3b66.ts.net:{port}/navigation"),
            requested: request,
            finish: Some(finish),
            task: Some(task),
        })
    }
    async fn finish(mut self) -> Result<()> {
        self.finish
            .take()
            .ok_or("missing finish sender")?
            .send(())
            .map_err(|_| "stream task exited early")?;
        self.task.as_mut().ok_or("missing stream task")?.await??;
        self.task.take();
        Ok(())
    }
}

pub(super) async fn exercise(host: &HostSession, page: &PageDescriptor) -> Result<()> {
    let state = host.frame_lifecycle(&page.id).await?;
    let frame = ObjectId::new(page.main_frame_id.parse()?);
    let previous = state
        .snapshot(frame)?
        .ok_or("missing initial document")?
        .document;
    // Arrange: keep the HTTP body open after native dispatch has reached it.
    let mut document = Document::new().await?;
    let mut navigation = Box::pin(host.navigate_frame_native(
        &page.id,
        &page.main_frame_id,
        BeginNavigationParams::new(document.url.clone()),
        WaitUntil::Load,
        10000,
    ));
    tokio::select! {
        result = &mut navigation => { result?; return Err("navigation completed before body release".into()); },
        requested = &mut document.requested => requested?,
    }
    assert_eq!(host.metrics()?.active_operations, 1);
    // Act: abandon just this operation, not the Frame's browser loading.
    drop(navigation);
    // Assert: the original document can still complete when its body arrives.
    assert_eq!(host.metrics()?.active_operations, 0);
    loop {
        let current = state
            .snapshot(frame)?
            .ok_or("dropped navigation detached Frame")?;
        if current.document != previous {
            assert!(
                !current.load,
                "abandonment must not stop loading the open document"
            );
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    document.finish().await?;
    ready(&state, frame).await?;
    assert!(!state.is_sealed());

    let mut document = Document::new().await?;
    let mut navigation = Box::pin(host.navigate_frame_native(
        &page.id,
        &page.main_frame_id,
        BeginNavigationParams::new(document.url.clone()),
        WaitUntil::Load,
        1000,
    ));
    tokio::select! {
        result = &mut navigation => { result?; return Err("navigation ended before dispatch evidence".into()); },
        requested = &mut document.requested => requested?,
    }
    let result = navigation.await;
    assert!(matches!(result, Err(HostError::Timeout(1000))));
    assert_eq!(host.metrics()?.active_operations, 0);
    assert!(!state.snapshot(frame)?.ok_or("timeout detached Frame")?.load);
    document.finish().await?;
    ready(&state, frame).await?;

    let mut document = Document::new().await?;
    let mut navigation = Box::pin(host.navigate_frame_native(
        &page.id,
        &page.main_frame_id,
        BeginNavigationParams::new(document.url.clone()),
        WaitUntil::Load,
        0,
    ));
    tokio::select! {
        result = &mut navigation => { result?; return Err("navigation completed before host close".into()); },
        requested = &mut document.requested => requested?,
    }
    let (result, closed) = tokio::join!(navigation, host.close());
    closed?;
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(host.metrics()?.active_operations, 0);
    assert_eq!(host.metrics()?.pending_calls, 0);
    document.finish().await?;
    println!(
        "PASS host-owned cancellation: dropped call, absolute timeout, host close, original loading not stopped"
    );
    Ok(())
}
