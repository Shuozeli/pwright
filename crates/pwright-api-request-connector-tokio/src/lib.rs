//! Optional Tokio DNS/TCP connector for Pwright API requests.
//!
//! This leaf supports plain `http:` origins only. TLS, proxies and alternate
//! connection strategies remain separately selectable capabilities.

use std::{
    future::Future,
    io,
    net::{IpAddr, SocketAddr},
    pin::Pin,
    time::Duration,
};

use async_trait::async_trait;
use futures_util::{StreamExt, stream::FuturesUnordered};
use pwright_api_request_transport::{
    ApiConnection, ApiConnectionError, ApiConnectionErrorKind, ApiConnectionTarget, ApiConnector,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt},
    net::{TcpStream, lookup_host},
    time::sleep,
};
use tokio_util::sync::CancellationToken;
use url::{Host, Url};

const MAX_RESOLVED_ADDRESSES: usize = 64;
const CONNECTION_ATTEMPT_DELAY: Duration = Duration::from_millis(300);

type ConnectionAttempt<T> = Pin<Box<dyn Future<Output = io::Result<T>> + Send>>;
type AttemptDelay = Pin<Box<dyn Future<Output = ()> + Send>>;

#[derive(Clone, Copy, Debug, Default)]
pub struct TokioTcpConnector;

#[async_trait]
impl ApiConnector for TokioTcpConnector {
    async fn connect(
        &self,
        target: ApiConnectionTarget<'_>,
        cancellation: &CancellationToken,
    ) -> Result<Box<dyn ApiConnection>, ApiConnectionError> {
        if cancellation.is_cancelled() {
            return Err(cancelled());
        }
        if target.url.scheme() != "http" {
            return Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Tls,
                "Tokio TCP connector supports only http origins",
            ));
        }
        let stream =
            connect_tcp_with_addresses(target.url, target.resolved_addresses, cancellation).await?;
        Ok(Box::new(TokioIoConnection::new(stream)))
    }
}

/// Resolve and connect one bounded Tokio TCP stream for an HTTP-family URL.
///
/// # Errors
///
/// Returns a classified connection error for cancellation, missing targets,
/// excessive/empty DNS answers, connection failure or socket configuration.
pub async fn connect_tcp(
    url: &Url,
    cancellation: &CancellationToken,
) -> Result<TcpStream, ApiConnectionError> {
    connect_tcp_with_addresses(url, None, cancellation).await
}

/// Connect with an optional bounded request-scoped resolver result.
///
/// # Errors
///
/// Returns a classified connection error for invalid injected addresses,
/// cancellation, DNS, connection failure or socket configuration.
pub async fn connect_tcp_with_addresses(
    url: &Url,
    resolved_addresses: Option<&[IpAddr]>,
    cancellation: &CancellationToken,
) -> Result<TcpStream, ApiConnectionError> {
    let host = url.host().ok_or_else(|| {
        ApiConnectionError::new(ApiConnectionErrorKind::Dns, "request URL has no host")
    })?;
    let port = url.port_or_known_default().ok_or_else(|| {
        ApiConnectionError::new(ApiConnectionErrorKind::Dns, "request URL has no port")
    })?;
    let addresses = match resolved_addresses {
        Some([]) => {
            return Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Dns,
                "request-scoped DNS lookup returned no addresses",
            ));
        }
        Some(addresses) if addresses.len() > MAX_RESOLVED_ADDRESSES => {
            return Err(ApiConnectionError::new(
                ApiConnectionErrorKind::Dns,
                format!("request-scoped DNS lookup exceeded {MAX_RESOLVED_ADDRESSES} addresses"),
            ));
        }
        Some(addresses) => interleave_address_families(
            addresses
                .iter()
                .copied()
                .map(|address| SocketAddr::new(address, port))
                .collect(),
        ),
        None => match host {
            Host::Domain(host) => {
                interleave_address_families(resolve(host, port, cancellation).await?)
            }
            Host::Ipv4(address) => vec![SocketAddr::new(address.into(), port)],
            Host::Ipv6(address) => vec![SocketAddr::new(address.into(), port)],
        },
    };
    let stream = connect_addresses(&addresses, cancellation).await?;
    stream
        .set_nodelay(true)
        .map_err(|error| classify_io_error("failed to configure TCP connection", &error))?;
    Ok(stream)
}

async fn resolve(
    host: &str,
    port: u16,
    cancellation: &CancellationToken,
) -> Result<Vec<SocketAddr>, ApiConnectionError> {
    let resolved = tokio::select! {
        biased;
        () = cancellation.cancelled() => return Err(cancelled()),
        result = lookup_host((host, port)) => result.map_err(|error| {
            ApiConnectionError::new(
                ApiConnectionErrorKind::Dns,
                format!("DNS lookup failed for {host}:{port}: {error}"),
            )
        })?,
    };
    let addresses = resolved
        .take(MAX_RESOLVED_ADDRESSES + 1)
        .collect::<Vec<_>>();
    if addresses.is_empty() {
        return Err(ApiConnectionError::new(
            ApiConnectionErrorKind::Dns,
            format!("DNS lookup returned no addresses for {host}:{port}"),
        ));
    }
    if addresses.len() > MAX_RESOLVED_ADDRESSES {
        return Err(ApiConnectionError::new(
            ApiConnectionErrorKind::Dns,
            format!("DNS lookup exceeded {MAX_RESOLVED_ADDRESSES} addresses for {host}:{port}"),
        ));
    }
    Ok(addresses)
}

async fn connect_addresses(
    addresses: &[SocketAddr],
    cancellation: &CancellationToken,
) -> Result<TcpStream, ApiConnectionError> {
    connect_addresses_with(
        addresses,
        cancellation,
        |address| Box::pin(TcpStream::connect(address)),
        || Box::pin(sleep(CONNECTION_ATTEMPT_DELAY)),
    )
    .await
}

fn interleave_address_families(addresses: Vec<SocketAddr>) -> Vec<SocketAddr> {
    let Some(first_address) = addresses.first() else {
        return addresses;
    };
    let first_family_is_ipv6 = first_address.is_ipv6();
    let (first_family, second_family): (Vec<_>, Vec<_>) = addresses
        .into_iter()
        .partition(|address| address.is_ipv6() == first_family_is_ipv6);
    let mut first_family = first_family.into_iter();
    let mut second_family = second_family.into_iter();
    let mut interleaved = Vec::with_capacity(
        first_family
            .size_hint()
            .0
            .saturating_add(second_family.size_hint().0),
    );
    for first in first_family.by_ref() {
        interleaved.push(first);
        if let Some(second) = second_family.next() {
            interleaved.push(second);
        }
    }
    interleaved.extend(second_family);
    interleaved
}

async fn connect_addresses_with<T>(
    addresses: &[SocketAddr],
    cancellation: &CancellationToken,
    mut connect: impl FnMut(SocketAddr) -> ConnectionAttempt<T>,
    mut delay: impl FnMut() -> AttemptDelay,
) -> Result<T, ApiConnectionError> {
    let Some(first_address) = addresses.first().copied() else {
        return Err(ApiConnectionError::new(
            ApiConnectionErrorKind::Dns,
            "no resolved address to connect",
        ));
    };

    let mut pending = FuturesUnordered::new();
    pending.push(connect(first_address));
    let mut next_address = 1;
    let mut launch_delay = (next_address < addresses.len()).then(&mut delay);
    let mut first_error = None;

    loop {
        let has_launch_delay = launch_delay.is_some();
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled()),
            result = pending.next(), if !pending.is_empty() => {
                match result {
                    Some(Ok(stream)) => return Ok(stream),
                    Some(Err(error)) => {
                        first_error.get_or_insert(error);
                        if pending.is_empty() && next_address == addresses.len() {
                            let error = first_error
                                .take()
                                .expect("a failed attempt records an error");
                            return Err(classify_io_error("TCP connect failed", &error));
                        }
                    }
                    None => unreachable!("a non-empty attempt set yields one result"),
                }
            }
            () = async {
                launch_delay
                    .as_mut()
                    .expect("guarded launch delay should exist")
                    .await;
            }, if has_launch_delay => {
                pending.push(connect(addresses[next_address]));
                next_address += 1;
                launch_delay = (next_address < addresses.len()).then(&mut delay);
            }
        }
    }
}

pub struct TokioIoConnection<Io> {
    io: Io,
}

impl<Io> TokioIoConnection<Io> {
    #[must_use]
    pub const fn new(io: Io) -> Self {
        Self { io }
    }
}

#[async_trait]
impl<Io> ApiConnection for TokioIoConnection<Io>
where
    Io: AsyncRead + AsyncWrite + Unpin + Send,
{
    async fn write_all(
        &mut self,
        bytes: &[u8],
        cancellation: &CancellationToken,
    ) -> Result<(), ApiConnectionError> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(cancelled()),
            result = self.io.write_all(bytes) => result.map_err(|error| {
                classify_io_error("TCP write failed", &error)
            }),
        }
    }

    async fn read(
        &mut self,
        output: &mut [u8],
        cancellation: &CancellationToken,
    ) -> Result<usize, ApiConnectionError> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(cancelled()),
            result = self.io.read(output) => result.map_err(|error| {
                classify_io_error("TCP read failed", &error)
            }),
        }
    }
}

fn cancelled() -> ApiConnectionError {
    ApiConnectionError::new(
        ApiConnectionErrorKind::Cancelled,
        "API request was cancelled",
    )
}

fn classify_io_error(stage: &str, error: &io::Error) -> ApiConnectionError {
    let kind = match error.kind() {
        io::ErrorKind::ConnectionReset => ApiConnectionErrorKind::Reset,
        io::ErrorKind::ConnectionRefused => ApiConnectionErrorKind::Refused,
        _ => ApiConnectionErrorKind::Other,
    };
    let message = error
        .to_string()
        .replace("alert: UnknownCA", "alert unknown ca");
    ApiConnectionError::new(kind, format!("{stage}: {message}"))
}

#[cfg(test)]
mod tests {
    use std::{
        future::pending,
        io,
        net::{Ipv4Addr, Ipv6Addr, SocketAddr},
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use pwright_api_request_transport::{
        ApiConnection, ApiConnectionErrorKind, ApiConnectionTarget, ApiConnector,
    };
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt, duplex},
        sync::Notify,
        task::yield_now,
    };
    use tokio_util::sync::CancellationToken;
    use url::Url;

    use super::{
        AttemptDelay, ConnectionAttempt, TokioIoConnection, TokioTcpConnector, classify_io_error,
        connect_addresses_with, interleave_address_families,
    };

    struct ActiveAttempt(Arc<AtomicUsize>);

    impl ActiveAttempt {
        fn new(active: Arc<AtomicUsize>) -> Self {
            active.fetch_add(1, Ordering::SeqCst);
            Self(active)
        }
    }

    impl Drop for ActiveAttempt {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }

    fn ipv4(last: u8) -> SocketAddr {
        SocketAddr::from((Ipv4Addr::new(127, 0, 0, last), 8080))
    }

    fn ipv6(last: u16) -> SocketAddr {
        SocketAddr::from((Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, last), 8080))
    }

    fn notified_delay(notify: Arc<Notify>) -> impl FnMut() -> AttemptDelay {
        move || {
            let notify = notify.clone();
            Box::pin(async move { notify.notified().await })
        }
    }

    #[tokio::test]
    async fn duplex_connection_reads_and_writes_without_a_socket_server() {
        // Arrange
        let (client, mut peer) = duplex(64);
        let mut connection = TokioIoConnection::new(client);
        let cancellation = CancellationToken::new();

        // Act
        connection
            .write_all(b"request", &cancellation)
            .await
            .expect("write should succeed");
        let mut written = [0; 7];
        peer.read_exact(&mut written)
            .await
            .expect("peer should read request");
        peer.write_all(b"response")
            .await
            .expect("peer should write response");
        let mut response = [0; 8];
        let bytes = connection
            .read(&mut response, &cancellation)
            .await
            .expect("read should succeed");

        // Assert
        assert_eq!(&written, b"request");
        assert_eq!(bytes, 8);
        assert_eq!(&response, b"response");
    }

    #[tokio::test]
    async fn cancelled_duplex_operation_is_classified() {
        // Arrange
        let (client, _peer) = duplex(1);
        let mut connection = TokioIoConnection::new(client);
        let cancellation = CancellationToken::new();
        cancellation.cancel();
        let mut output = [0; 1];

        // Act
        let error = connection
            .read(&mut output, &cancellation)
            .await
            .expect_err("pre-cancelled read should fail");

        // Assert
        assert_eq!(error.kind, ApiConnectionErrorKind::Cancelled);
    }

    #[test]
    fn unknown_ca_alert_uses_playwright_compatible_words() {
        // Arrange
        let source = io::Error::other("received fatal alert: UnknownCA");

        // Act
        let error = classify_io_error("TCP read failed", &source);

        // Assert
        assert_eq!(error.kind, ApiConnectionErrorKind::Other);
        assert_eq!(
            error.message,
            "TCP read failed: received fatal alert unknown ca"
        );
    }

    #[tokio::test]
    async fn https_is_rejected_before_dns_or_socket_work() {
        // Arrange
        let connector = TokioTcpConnector;
        let url = Url::parse("https://not-resolved.invalid/").expect("test URL should parse");

        // Act
        let Err(error) = connector
            .connect(
                ApiConnectionTarget {
                    url: &url,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &CancellationToken::new(),
            )
            .await
        else {
            panic!("plain connector should reject HTTPS");
        };

        // Assert
        assert_eq!(error.kind, ApiConnectionErrorKind::Tls);
    }

    #[tokio::test]
    async fn cancelled_connect_is_rejected_before_dns() {
        // Arrange
        let connector = TokioTcpConnector;
        let url = Url::parse("http://not-resolved.invalid/").expect("test URL should parse");
        let cancellation = CancellationToken::new();
        cancellation.cancel();

        // Act
        let Err(error) = connector
            .connect(
                ApiConnectionTarget {
                    url: &url,
                    ignore_https_errors: false,
                    resolved_addresses: None,
                },
                &cancellation,
            )
            .await
        else {
            panic!("cancelled connect should fail");
        };

        // Assert
        assert_eq!(error.kind, ApiConnectionErrorKind::Cancelled);
    }

    #[test]
    fn resolved_addresses_alternate_families_from_the_first_result() {
        // Arrange
        let ipv6_first = vec![ipv6(2), ipv6(1), ipv4(2), ipv4(1), ipv4(3)];
        let ipv4_first = vec![ipv4(2), ipv6(2), ipv6(1), ipv4(1)];

        // Act
        let ipv6_first = interleave_address_families(ipv6_first);
        let ipv4_first = interleave_address_families(ipv4_first);

        // Assert
        assert_eq!(
            ipv6_first,
            vec![ipv6(2), ipv4(2), ipv6(1), ipv4(1), ipv4(3)]
        );
        assert_eq!(ipv4_first, vec![ipv4(2), ipv6(2), ipv4(1), ipv6(1)]);
    }

    #[tokio::test]
    async fn pending_attempt_is_raced_only_after_the_injected_delay() {
        // Arrange
        let addresses = [ipv6(2), ipv4(1)];
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let active = Arc::new(AtomicUsize::new(0));
        let notify = Arc::new(Notify::new());
        let connect = {
            let attempts = attempts.clone();
            let active = active.clone();
            move |address| -> ConnectionAttempt<u8> {
                attempts.lock().expect("attempt log lock").push(address);
                if address == ipv6(2) {
                    let guard = ActiveAttempt::new(active.clone());
                    Box::pin(async move {
                        let _guard = guard;
                        pending::<io::Result<u8>>().await
                    })
                } else {
                    Box::pin(async { Ok(7) })
                }
            }
        };
        let cancellation = CancellationToken::new();
        let task = tokio::spawn({
            let cancellation = cancellation.clone();
            let notify = notify.clone();
            async move {
                connect_addresses_with(&addresses, &cancellation, connect, notified_delay(notify))
                    .await
            }
        });

        // Act
        yield_now().await;
        let before_delay = attempts.lock().expect("attempt log lock").clone();
        notify.notify_one();
        let winner = task.await.expect("scheduler task should join");

        // Assert
        assert_eq!(before_delay, vec![ipv6(2)]);
        assert_eq!(winner.expect("second address should win"), 7);
        assert_eq!(
            *attempts.lock().expect("attempt log lock"),
            vec![ipv6(2), ipv4(1)]
        );
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn failed_attempt_does_not_bypass_the_injected_delay() {
        // Arrange
        let addresses = [ipv6(2), ipv4(1)];
        let attempts = Arc::new(Mutex::new(Vec::new()));
        let notify = Arc::new(Notify::new());
        let connect = {
            let attempts = attempts.clone();
            move |address| -> ConnectionAttempt<u8> {
                attempts.lock().expect("attempt log lock").push(address);
                if address == ipv6(2) {
                    Box::pin(async {
                        Err(io::Error::new(io::ErrorKind::ConnectionRefused, "first"))
                    })
                } else {
                    Box::pin(async { Ok(9) })
                }
            }
        };
        let task = tokio::spawn({
            let notify = notify.clone();
            async move {
                let cancellation = CancellationToken::new();
                connect_addresses_with(&addresses, &cancellation, connect, notified_delay(notify))
                    .await
            }
        });

        // Act
        yield_now().await;
        let before_delay = attempts.lock().expect("attempt log lock").clone();
        let finished_before_delay = task.is_finished();
        notify.notify_one();
        let winner = task.await.expect("scheduler task should join");

        // Assert
        assert_eq!(before_delay, vec![ipv6(2)]);
        assert!(!finished_before_delay);
        assert_eq!(winner.expect("second address should win"), 9);
        assert_eq!(
            *attempts.lock().expect("attempt log lock"),
            vec![ipv6(2), ipv4(1)]
        );
    }

    #[tokio::test]
    async fn all_failures_report_the_first_completed_error() {
        // Arrange
        let addresses = [ipv6(2), ipv4(1)];
        let notify = Arc::new(Notify::new());
        let connect = |address| -> ConnectionAttempt<u8> {
            if address == ipv6(2) {
                Box::pin(async { Err(io::Error::new(io::ErrorKind::ConnectionRefused, "first")) })
            } else {
                Box::pin(async { Err(io::Error::new(io::ErrorKind::BrokenPipe, "second")) })
            }
        };
        let task = tokio::spawn({
            let notify = notify.clone();
            async move {
                let cancellation = CancellationToken::new();
                connect_addresses_with(&addresses, &cancellation, connect, notified_delay(notify))
                    .await
            }
        });

        // Act
        yield_now().await;
        notify.notify_one();
        let error = task
            .await
            .expect("scheduler task should join")
            .expect_err("both attempts should fail");

        // Assert
        assert_eq!(error.kind, ApiConnectionErrorKind::Refused);
        assert!(error.message.contains("first"));
    }

    #[tokio::test]
    async fn cancellation_drops_every_pending_attempt() {
        // Arrange
        let addresses = [ipv6(2)];
        let active = Arc::new(AtomicUsize::new(0));
        let connect = {
            let active = active.clone();
            move |_address| -> ConnectionAttempt<u8> {
                let guard = ActiveAttempt::new(active.clone());
                Box::pin(async move {
                    let _guard = guard;
                    pending::<io::Result<u8>>().await
                })
            }
        };
        let cancellation = CancellationToken::new();
        let task = tokio::spawn({
            let cancellation = cancellation.clone();
            async move {
                connect_addresses_with(&addresses, &cancellation, connect, || Box::pin(pending()))
                    .await
            }
        });

        // Act
        yield_now().await;
        cancellation.cancel();
        let error = task
            .await
            .expect("scheduler task should join")
            .expect_err("cancellation should fail the race");

        // Assert
        assert_eq!(error.kind, ApiConnectionErrorKind::Cancelled);
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn io_error_classifier_retries_only_connection_reset() {
        // Arrange
        let reset = io::Error::new(io::ErrorKind::ConnectionReset, "reset");
        let refused = io::Error::new(io::ErrorKind::ConnectionRefused, "refused");
        let broken_pipe = io::Error::new(io::ErrorKind::BrokenPipe, "pipe");

        // Act
        let reset = classify_io_error("read", &reset);
        let refused = classify_io_error("connect", &refused);
        let broken_pipe = classify_io_error("write", &broken_pipe);

        // Assert
        assert_eq!(reset.kind, ApiConnectionErrorKind::Reset);
        assert_eq!(refused.kind, ApiConnectionErrorKind::Refused);
        assert_eq!(broken_pipe.kind, ApiConnectionErrorKind::Other);
    }
}
