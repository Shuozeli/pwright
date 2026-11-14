//! Compose request-affecting session commands before debugger resume.

use std::future::Future;

use pwright_plugin_chromium_session_initialization::ChromiumSessionResumeFence;

/// A fenced command reaches the write queue before polling downstream. Outside
/// the pre-resume path, preserve the ordinary downstream-first initialization.
/// In both cases the command's response remains owned through settlement.
pub(crate) async fn enqueue_before_downstream<
    Command,
    Queue,
    Downstream,
    Settle,
    Response,
    QueueError,
    Error,
>(
    fence: Option<&ChromiumSessionResumeFence>,
    queue: Option<Queue>,
    downstream: Downstream,
    settle: Settle,
) -> Result<(), Error>
where
    Queue: Future<Output = Result<Command, QueueError>>,
    Downstream: Future<Output = Result<(), Error>>,
    Settle: FnOnce(Command) -> Response,
    Response: Future<Output = Result<(), Error>>,
    Error: From<QueueError>,
{
    let Some(queue) = queue else {
        return downstream.await;
    };
    let Some(fence) = fence else {
        downstream.await?;
        let command = queue.await.map_err(Error::from)?;
        return settle(command).await;
    };
    fence.require_enqueue();
    let command = queue.await.map_err(Error::from)?;
    fence.command_enqueued();
    let ((), ()) = tokio::try_join!(downstream, settle(command))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};
    use std::task::Poll;

    use tokio::sync::oneshot;

    use super::*;

    type Events = Arc<Mutex<Vec<&'static str>>>;

    fn record(events: &Events, event: &'static str) {
        events.lock().unwrap().push(event);
    }

    async fn poll_once<T>(mut future: Pin<&mut impl Future<Output = T>>) -> Poll<T> {
        std::future::poll_fn(|cx| Poll::Ready(future.as_mut().poll(cx))).await
    }

    #[tokio::test]
    async fn header_locale_and_user_agent_queue_before_resume_under_backpressure() {
        // Arrange: every queue admission and response has an independent gate.
        let fence = ChromiumSessionResumeFence::default();
        let events: Events = Arc::new(Mutex::new(Vec::new()));
        let (ua_admit, ua_ready) = oneshot::channel::<()>();
        let (locale_admit, locale_ready) = oneshot::channel::<()>();
        let (header_admit, header_ready) = oneshot::channel::<()>();
        let (ua_ack, ua_settled) = oneshot::channel::<()>();
        let (locale_ack, locale_settled) = oneshot::channel::<()>();
        let (header_ack, header_settled) = oneshot::channel::<()>();

        let header_events = events.clone();
        let header = async {
            fence.require_enqueue();
            record(&header_events, "header waiting");
            header_ready.await.unwrap();
            record(&header_events, "header queued");
            fence.command_enqueued();
            header_settled.await.unwrap();
            Ok::<(), &'static str>(())
        };
        let locale_events = events.clone();
        let locale = enqueue_before_downstream(
            Some(&fence),
            Some(async {
                record(&locale_events, "locale waiting");
                locale_ready.await.unwrap();
                record(&locale_events, "locale queued");
                Ok::<(), &'static str>(())
            }),
            header,
            |()| async {
                locale_settled.await.unwrap();
                Ok::<(), &'static str>(())
            },
        );
        let ua_events = events.clone();
        let ua = enqueue_before_downstream(
            Some(&fence),
            Some(async {
                record(&ua_events, "ua waiting");
                ua_ready.await.unwrap();
                record(&ua_events, "ua queued");
                Ok::<(), &'static str>(())
            }),
            locale,
            |()| async {
                ua_settled.await.unwrap();
                Ok::<(), &'static str>(())
            },
        );
        tokio::pin!(ua);

        // Act: the driver polls the composing initializer at each admission.
        assert!(poll_once(ua.as_mut()).await.is_pending());
        assert!(fence.is_waiting());
        ua_admit.send(()).unwrap();
        assert!(poll_once(ua.as_mut()).await.is_pending());
        assert!(fence.is_waiting());
        locale_admit.send(()).unwrap();
        assert!(poll_once(ua.as_mut()).await.is_pending());
        assert!(fence.is_waiting());
        header_admit.send(()).unwrap();
        assert!(poll_once(ua.as_mut()).await.is_pending());

        // Assert: resume is now allowed, with all three commands queued.
        assert!(!fence.is_waiting());
        assert_eq!(
            *events.lock().unwrap(),
            [
                "ua waiting",
                "ua queued",
                "locale waiting",
                "locale queued",
                "header waiting",
                "header queued",
            ]
        );
        ua_ack.send(()).unwrap();
        locale_ack.send(()).unwrap();
        header_ack.send(()).unwrap();
        assert!(matches!(poll_once(ua.as_mut()).await, Poll::Ready(Ok(()))));
    }

    #[tokio::test]
    async fn no_header_locale_and_user_agent_still_queue_before_resume() {
        // Arrange
        let fence = ChromiumSessionResumeFence::default();
        let events: Events = Arc::new(Mutex::new(Vec::new()));
        let (ua_admit, ua_ready) = oneshot::channel::<()>();
        let (locale_admit, locale_ready) = oneshot::channel::<()>();
        let (ua_ack, ua_settled) = oneshot::channel::<()>();
        let (locale_ack, locale_settled) = oneshot::channel::<()>();
        let (network_ack, network_settled) = oneshot::channel::<()>();
        let network_events = events.clone();
        let locale_events = events.clone();
        let locale = enqueue_before_downstream(
            Some(&fence),
            Some(async {
                locale_ready.await.unwrap();
                record(&locale_events, "locale queued");
                Ok::<(), &'static str>(())
            }),
            async {
                record(&network_events, "network.enable queued");
                network_settled.await.unwrap();
                Ok::<(), &'static str>(())
            },
            |()| async {
                locale_settled.await.unwrap();
                Ok::<(), &'static str>(())
            },
        );
        let ua_events = events.clone();
        let ua = enqueue_before_downstream(
            Some(&fence),
            Some(async {
                ua_ready.await.unwrap();
                record(&ua_events, "ua queued");
                Ok::<(), &'static str>(())
            }),
            locale,
            |()| async {
                ua_settled.await.unwrap();
                Ok::<(), &'static str>(())
            },
        );
        tokio::pin!(ua);

        // Act
        assert!(poll_once(ua.as_mut()).await.is_pending());
        assert!(fence.is_waiting());
        ua_admit.send(()).unwrap();
        assert!(poll_once(ua.as_mut()).await.is_pending());
        assert!(fence.is_waiting());
        locale_admit.send(()).unwrap();
        assert!(poll_once(ua.as_mut()).await.is_pending());

        // Assert
        assert!(!fence.is_waiting());
        assert_eq!(
            *events.lock().unwrap(),
            ["ua queued", "locale queued", "network.enable queued"]
        );
        ua_ack.send(()).unwrap();
        locale_ack.send(()).unwrap();
        network_ack.send(()).unwrap();
        assert!(matches!(poll_once(ua.as_mut()).await, Poll::Ready(Ok(()))));
    }

    #[tokio::test]
    async fn unfenced_initialization_keeps_downstream_first_order() {
        // Arrange
        let events: Events = Arc::new(Mutex::new(Vec::new()));
        let queue_events = events.clone();
        let downstream_events = events.clone();
        let response = enqueue_before_downstream(
            None,
            Some(async {
                record(&queue_events, "queued");
                Ok::<(), &'static str>(())
            }),
            async {
                record(&downstream_events, "downstream");
                Ok::<(), &'static str>(())
            },
            |()| async { Ok::<(), &'static str>(()) },
        );

        // Act
        response.await.unwrap();

        // Assert
        assert_eq!(*events.lock().unwrap(), ["downstream", "queued"]);
    }

    #[tokio::test]
    async fn admission_timeout_fails_closed_without_polling_downstream() {
        // Arrange
        let fence = ChromiumSessionResumeFence::default();
        let downstream_polled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let marker = downstream_polled.clone();
        let timeout = enqueue_before_downstream(
            Some(&fence),
            Some(async { Err::<(), _>(pwright_cdp::connection::CdpError::Timeout) }),
            async move {
                marker.store(true, std::sync::atomic::Ordering::SeqCst);
                Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
            },
            |()| async { Ok::<(), Box<dyn std::error::Error + Send + Sync>>(()) },
        )
        .await;

        // Act and assert: a queue error never starts downstream or opens resume.
        assert!(
            matches!(timeout, Err(error) if error.downcast_ref::<pwright_cdp::connection::CdpError>().is_some_and(|error| matches!(error, pwright_cdp::connection::CdpError::Timeout)))
        );
        assert!(fence.is_waiting());
        assert!(!downstream_polled.load(std::sync::atomic::Ordering::SeqCst));
    }

    struct ResponseOwner(Arc<std::sync::atomic::AtomicBool>);

    impl Drop for ResponseOwner {
        fn drop(&mut self) {
            self.0.store(false, std::sync::atomic::Ordering::SeqCst);
        }
    }

    #[tokio::test]
    async fn queued_response_owner_survives_until_protocol_rejection() {
        // Arrange: the queued command's response owner must survive resume.
        let response_fence = ChromiumSessionResumeFence::default();
        let owner_live = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let (ack, settled) = oneshot::channel::<()>();
        let owner = ResponseOwner(owner_live.clone());
        let response = enqueue_before_downstream(
            Some(&response_fence),
            Some(async { Ok::<_, &'static str>(owner) }),
            async { Ok::<(), &'static str>(()) },
            |owner| async move {
                let _owner = owner;
                settled.await.unwrap();
                Err::<(), _>("protocol rejection")
            },
        );
        tokio::pin!(response);

        // Act: queue admission opens the fence before Chrome acknowledges.
        assert!(poll_once(response.as_mut()).await.is_pending());

        // Assert: response ownership remains live until its rejection settles.
        assert!(!response_fence.is_waiting());
        assert!(owner_live.load(std::sync::atomic::Ordering::SeqCst));
        ack.send(()).unwrap();
        assert!(matches!(
            poll_once(response.as_mut()).await,
            Poll::Ready(Err("protocol rejection"))
        ));
        assert!(!owner_live.load(std::sync::atomic::Ordering::SeqCst));
    }
}
