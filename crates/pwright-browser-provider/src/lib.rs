//! Browser acquisition and process-ownership contracts for Pwright.
//!
//! Attachment and launch are deliberately different traits with different
//! result types. An attached browser has no process-control capability. Only a
//! launched browser carries the lease that may terminate its owned process.
//! This crate does not discover, install, download, or launch a browser.

use std::future::Future;

/// Attaches to a browser process that Pwright does not own.
pub trait AttachBrowserProvider<Request>: Send + Sync {
    type Connection;
    type Error;
    type Future<'a>: Future<Output = Result<AttachedBrowser<Self::Connection>, Self::Error>> + Send
    where
        Self: 'a;

    fn attach(&self, request: Request) -> Self::Future<'_>;
}

/// Launches a browser process and returns explicit ownership of that process.
pub trait LaunchBrowserProvider<Request>: Send + Sync {
    type Connection;
    type Lease: BrowserProcessLease;
    type Error;
    type Future<'a>: Future<Output = Result<LaunchedBrowser<Self::Connection, Self::Lease>, Self::Error>>
        + Send
    where
        Self: 'a;

    fn launch(&self, request: Request) -> Self::Future<'_>;
}

/// A connection to an externally owned browser.
///
/// Dropping the connection may disconnect its transport. This value contains
/// no authority to terminate the browser process.
///
/// ```compile_fail
/// use pwright_browser_provider::AttachedBrowser;
///
/// let browser = AttachedBrowser::new("external connection");
/// browser.terminate();
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[must_use = "dropping an attachment disconnects it without terminating the browser"]
pub struct AttachedBrowser<Connection> {
    connection: Connection,
}

impl<Connection> AttachedBrowser<Connection> {
    pub const fn new(connection: Connection) -> Self {
        Self { connection }
    }

    pub const fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn into_connection(self) -> Connection {
        self.connection
    }
}

/// A connection paired with the exclusive capability to terminate its process.
#[derive(Debug, Eq, PartialEq)]
#[must_use = "an owned browser should be shut down explicitly"]
pub struct LaunchedBrowser<Connection, Lease> {
    connection: Connection,
    lease: Lease,
}

impl<Connection, Lease> LaunchedBrowser<Connection, Lease>
where
    Lease: BrowserProcessLease,
{
    pub const fn new(connection: Connection, lease: Lease) -> Self {
        Self { connection, lease }
    }

    pub const fn connection(&self) -> &Connection {
        &self.connection
    }

    pub fn into_connection_and_lease(self) -> (Connection, Lease) {
        (self.connection, self.lease)
    }
}

/// Exclusive authority to request termination of one owned browser process.
///
/// `terminate` consumes the lease value, preventing repeated termination
/// through that value. Implementations must not rely on `Drop` as the only
/// termination mechanism or manufacture duplicate authority.
pub trait BrowserProcessLease: Send + 'static {
    type Termination;
    type Error;
    type Future: Future<Output = Result<Self::Termination, Self::Error>> + Send;

    fn terminate(self) -> Self::Future;
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::{AttachedBrowser, BrowserProcessLease, LaunchedBrowser};

    struct RecordingConnection {
        drops: Arc<AtomicUsize>,
    }

    impl Drop for RecordingConnection {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct RecordingLease {
        terminations: Arc<AtomicUsize>,
    }

    impl BrowserProcessLease for RecordingLease {
        type Termination = ();
        type Error = Infallible;
        type Future = Ready<Result<Self::Termination, Self::Error>>;

        fn terminate(self) -> Self::Future {
            self.terminations.fetch_add(1, Ordering::SeqCst);
            ready(Ok(()))
        }
    }

    #[test]
    fn dropping_an_attachment_releases_only_its_connection() {
        // Arrange
        let connection_drops = Arc::new(AtomicUsize::new(0));
        let attachment = AttachedBrowser::new(RecordingConnection {
            drops: Arc::clone(&connection_drops),
        });

        // Act
        drop(attachment);

        // Assert
        assert_eq!(connection_drops.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn launched_browser_separates_connection_from_termination_authority() {
        // Arrange
        let connection_drops = Arc::new(AtomicUsize::new(0));
        let terminations = Arc::new(AtomicUsize::new(0));
        let launched = LaunchedBrowser::new(
            RecordingConnection {
                drops: Arc::clone(&connection_drops),
            },
            RecordingLease {
                terminations: Arc::clone(&terminations),
            },
        );

        // Act
        let (connection, lease) = launched.into_connection_and_lease();
        let _termination = lease.terminate();
        drop(connection);

        // Assert
        assert_eq!(terminations.load(Ordering::SeqCst), 1);
        assert_eq!(connection_drops.load(Ordering::SeqCst), 1);
    }
}
