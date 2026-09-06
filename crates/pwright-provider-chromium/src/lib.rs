//! Optional Chromium process discovery and explicit-path launch provider.
//!
//! This crate never downloads or bundles a browser. Launch accepts a validated
//! executable already present on the machine and either a fresh empty profile
//! or an explicitly marked persistent profile. Chromium's debugging endpoint
//! is forced to loopback and the returned consuming lease is the only
//! process-termination authority.

use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::io;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use pwright_browser_provider::{BrowserProcessLease, LaunchBrowserProvider, LaunchedBrowser};
use pwright_cdp::connection::ConnectionConfig;
use pwright_cdp::{CdpConnection, CdpSession};
use pwright_provider_cdp::{CdpConnector, CdpWebSocketEndpoint, LiveCdpConnector};

const ACTIVE_PORT_FILE: &str = "DevToolsActivePort";
const LOOPBACK_DEBUGGING_ADDRESS: &str = "127.0.0.1";
const PERSISTENT_PROFILE_MARKER_FILE: &str = ".pwright-chromium-profile";
const PERSISTENT_PROFILE_MARKER: &[u8] = b"pwright-chromium-profile-v1\n";
const RESERVED_ARGUMENTS: &[&str] = &[
    "--remote-debugging-address",
    "--remote-debugging-pipe",
    "--remote-debugging-port",
    "--user-data-dir",
];

/// A known Chromium distribution installed outside Pwright.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ChromiumChannel {
    Chrome,
    ChromeBeta,
    ChromeDev,
    ChromeCanary,
    Chromium,
    Edge,
    Custom,
}

impl Display for ChromiumChannel {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Chrome => "chrome",
            Self::ChromeBeta => "chrome-beta",
            Self::ChromeDev => "chrome-dev",
            Self::ChromeCanary => "chrome-canary",
            Self::Chromium => "chromium",
            Self::Edge => "edge",
            Self::Custom => "custom",
        })
    }
}

/// A canonical absolute browser executable path.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ChromiumExecutable {
    path: PathBuf,
    channel: ChromiumChannel,
}

impl ChromiumExecutable {
    /// Validates an explicit browser executable already present on disk.
    pub fn from_path(
        path: impl Into<PathBuf>,
        channel: ChromiumChannel,
    ) -> Result<Self, ChromiumExecutableError> {
        let requested = path.into();
        if !requested.is_absolute() {
            return Err(ChromiumExecutableError::NotAbsolute(requested));
        }
        let path = std::fs::canonicalize(&requested).map_err(|source| {
            ChromiumExecutableError::Inspect {
                path: requested,
                source,
            }
        })?;
        validate_executable_file(&path)?;
        Ok(Self { path, channel })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub const fn channel(&self) -> ChromiumChannel {
        self.channel
    }
}

#[derive(Debug)]
pub enum ChromiumExecutableError {
    NotAbsolute(PathBuf),
    Inspect { path: PathBuf, source: io::Error },
    NotAFile(PathBuf),
    NotExecutable(PathBuf),
}

impl Display for ChromiumExecutableError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAbsolute(path) => {
                write!(
                    formatter,
                    "Chromium executable path is not absolute: {}",
                    path.display()
                )
            }
            Self::Inspect { path, source } => write!(
                formatter,
                "could not inspect Chromium executable {}: {source}",
                path.display()
            ),
            Self::NotAFile(path) => write!(
                formatter,
                "Chromium executable is not a regular file: {}",
                path.display()
            ),
            Self::NotExecutable(path) => write!(
                formatter,
                "Chromium file does not have an executable permission bit: {}",
                path.display()
            ),
        }
    }
}

impl Error for ChromiumExecutableError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Inspect { source, .. } => Some(source),
            Self::NotAbsolute(_) | Self::NotAFile(_) | Self::NotExecutable(_) => None,
        }
    }
}

/// Finds installed Chromium-family executables without downloading anything.
///
/// Results are canonicalized, deduplicated, and returned in deterministic
/// channel/path order. An empty result is not an error; callers decide whether
/// absence should fall back to an explicit path or fail their selected profile.
pub fn discover_system_chromium() -> Vec<ChromiumExecutable> {
    let mut candidates = platform_candidates();
    if let Some(path) = std::env::var_os("PATH") {
        for directory in std::env::split_paths(&path) {
            for (name, channel) in path_candidate_names() {
                candidates.push((directory.join(name), *channel));
            }
        }
    }
    discover_candidates(candidates)
}

fn discover_candidates(
    candidates: impl IntoIterator<Item = (PathBuf, ChromiumChannel)>,
) -> Vec<ChromiumExecutable> {
    let mut found = BTreeMap::new();
    for (path, channel) in candidates {
        if let Ok(executable) = ChromiumExecutable::from_path(path, channel) {
            found.entry(executable.path.clone()).or_insert(executable);
        }
    }
    let mut found = found.into_values().collect::<Vec<_>>();
    found
        .sort_by(|left, right| (left.channel(), left.path()).cmp(&(right.channel(), right.path())));
    found
}

#[cfg(target_os = "macos")]
fn platform_candidates() -> Vec<(PathBuf, ChromiumChannel)> {
    vec![
        (
            PathBuf::from("/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
            ChromiumChannel::Chrome,
        ),
        (
            PathBuf::from("/Applications/Google Chrome Beta.app/Contents/MacOS/Google Chrome Beta"),
            ChromiumChannel::ChromeBeta,
        ),
        (
            PathBuf::from("/Applications/Google Chrome Dev.app/Contents/MacOS/Google Chrome Dev"),
            ChromiumChannel::ChromeDev,
        ),
        (
            PathBuf::from(
                "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary",
            ),
            ChromiumChannel::ChromeCanary,
        ),
        (
            PathBuf::from("/Applications/Chromium.app/Contents/MacOS/Chromium"),
            ChromiumChannel::Chromium,
        ),
        (
            PathBuf::from("/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
            ChromiumChannel::Edge,
        ),
    ]
}

#[cfg(target_os = "linux")]
fn platform_candidates() -> Vec<(PathBuf, ChromiumChannel)> {
    vec![
        (
            PathBuf::from("/usr/bin/google-chrome"),
            ChromiumChannel::Chrome,
        ),
        (
            PathBuf::from("/usr/bin/google-chrome-beta"),
            ChromiumChannel::ChromeBeta,
        ),
        (
            PathBuf::from("/usr/bin/google-chrome-unstable"),
            ChromiumChannel::ChromeDev,
        ),
        (
            PathBuf::from("/usr/bin/chromium"),
            ChromiumChannel::Chromium,
        ),
        (
            PathBuf::from("/usr/bin/chromium-browser"),
            ChromiumChannel::Chromium,
        ),
        (
            PathBuf::from("/usr/bin/microsoft-edge"),
            ChromiumChannel::Edge,
        ),
    ]
}

#[cfg(target_os = "windows")]
fn platform_candidates() -> Vec<(PathBuf, ChromiumChannel)> {
    let mut candidates = Vec::new();
    for root in [
        std::env::var_os("PROGRAMFILES"),
        std::env::var_os("PROGRAMFILES(X86)"),
        std::env::var_os("LOCALAPPDATA"),
    ]
    .into_iter()
    .flatten()
    {
        let root = PathBuf::from(root);
        candidates.push((
            root.join("Google/Chrome/Application/chrome.exe"),
            ChromiumChannel::Chrome,
        ));
        candidates.push((
            root.join("Chromium/Application/chrome.exe"),
            ChromiumChannel::Chromium,
        ));
        candidates.push((
            root.join("Microsoft/Edge/Application/msedge.exe"),
            ChromiumChannel::Edge,
        ));
    }
    candidates
}

#[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
fn platform_candidates() -> Vec<(PathBuf, ChromiumChannel)> {
    Vec::new()
}

#[cfg(target_os = "windows")]
fn path_candidate_names() -> &'static [(&'static str, ChromiumChannel)] {
    &[
        ("chrome.exe", ChromiumChannel::Chrome),
        ("chromium.exe", ChromiumChannel::Chromium),
        ("msedge.exe", ChromiumChannel::Edge),
    ]
}

#[cfg(not(target_os = "windows"))]
fn path_candidate_names() -> &'static [(&'static str, ChromiumChannel)] {
    &[
        ("google-chrome", ChromiumChannel::Chrome),
        ("google-chrome-stable", ChromiumChannel::Chrome),
        ("google-chrome-beta", ChromiumChannel::ChromeBeta),
        ("google-chrome-unstable", ChromiumChannel::ChromeDev),
        ("chromium", ChromiumChannel::Chromium),
        ("chromium-browser", ChromiumChannel::Chromium),
        ("microsoft-edge", ChromiumChannel::Edge),
    ]
}

fn validate_executable_file(path: &Path) -> Result<(), ChromiumExecutableError> {
    let metadata = std::fs::metadata(path).map_err(|source| ChromiumExecutableError::Inspect {
        path: path.to_path_buf(),
        source,
    })?;
    if !metadata.is_file() {
        return Err(ChromiumExecutableError::NotAFile(path.to_path_buf()));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(ChromiumExecutableError::NotExecutable(path.to_path_buf()));
        }
    }
    Ok(())
}

/// Explicit launch-time policy for a caller-owned Chromium profile directory.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumProfilePolicy {
    /// The directory must be empty at every launch request validation.
    Fresh,
    /// The directory must carry Pwright's versioned dedicated-profile marker.
    Persistent,
}

/// A reusable Chromium profile that was explicitly initialized for Pwright.
///
/// This token cannot be constructed from an arbitrary path. Initialization is
/// allowed only for an existing empty absolute directory; reopening requires
/// the exact versioned marker. The directory remains caller-owned and is never
/// recursively deleted by the provider.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChromiumPersistentProfile {
    path: PathBuf,
}

impl ChromiumPersistentProfile {
    pub fn initialize(path: impl Into<PathBuf>) -> Result<Self, ChromiumProfileError> {
        let path = validate_profile_directory(path.into())?;
        if profile_has_entries(&path)? {
            return Err(ChromiumProfileError::NotEmpty(path));
        }
        let marker_path = path.join(PERSISTENT_PROFILE_MARKER_FILE);
        let mut marker = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&marker_path)
            .map_err(|source| ChromiumProfileError::CreateMarker {
                path: marker_path.clone(),
                source,
            })?;
        marker
            .write_all(PERSISTENT_PROFILE_MARKER)
            .map_err(|source| ChromiumProfileError::WriteMarker {
                path: marker_path,
                source,
            })?;
        Ok(Self { path })
    }

    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ChromiumProfileError> {
        let path = validate_profile_directory(path.into())?;
        validate_persistent_profile_marker(&path)?;
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug)]
pub enum ChromiumProfileError {
    NotAbsolute(PathBuf),
    Inspect { path: PathBuf, source: io::Error },
    NotDirectory(PathBuf),
    NotEmpty(PathBuf),
    CreateMarker { path: PathBuf, source: io::Error },
    WriteMarker { path: PathBuf, source: io::Error },
    ReadMarker { path: PathBuf, source: io::Error },
    InvalidMarker(PathBuf),
}

impl Display for ChromiumProfileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAbsolute(path) => write!(
                formatter,
                "Chromium profile directory is not absolute: {}",
                path.display()
            ),
            Self::Inspect { path, source } => write!(
                formatter,
                "could not inspect Chromium profile directory {}: {source}",
                path.display()
            ),
            Self::NotDirectory(path) => write!(
                formatter,
                "Chromium profile path is not a directory: {}",
                path.display()
            ),
            Self::NotEmpty(path) => write!(
                formatter,
                "persistent Chromium profile initialization requires an empty directory: {}",
                path.display()
            ),
            Self::CreateMarker { path, source } => write!(
                formatter,
                "could not create persistent Chromium profile marker {}: {source}",
                path.display()
            ),
            Self::WriteMarker { path, source } => write!(
                formatter,
                "could not write persistent Chromium profile marker {}: {source}",
                path.display()
            ),
            Self::ReadMarker { path, source } => write!(
                formatter,
                "could not read persistent Chromium profile marker {}: {source}",
                path.display()
            ),
            Self::InvalidMarker(path) => write!(
                formatter,
                "persistent Chromium profile marker is invalid: {}",
                path.display()
            ),
        }
    }
}

impl Error for ChromiumProfileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Inspect { source, .. }
            | Self::CreateMarker { source, .. }
            | Self::WriteMarker { source, .. }
            | Self::ReadMarker { source, .. } => Some(source),
            Self::NotAbsolute(_)
            | Self::NotDirectory(_)
            | Self::NotEmpty(_)
            | Self::InvalidMarker(_) => None,
        }
    }
}

fn validate_profile_directory(path: PathBuf) -> Result<PathBuf, ChromiumProfileError> {
    if !path.is_absolute() {
        return Err(ChromiumProfileError::NotAbsolute(path));
    }
    let metadata = std::fs::metadata(&path).map_err(|source| ChromiumProfileError::Inspect {
        path: path.clone(),
        source,
    })?;
    if !metadata.is_dir() {
        return Err(ChromiumProfileError::NotDirectory(path));
    }
    std::fs::canonicalize(&path).map_err(|source| ChromiumProfileError::Inspect { path, source })
}

fn profile_has_entries(path: &Path) -> Result<bool, ChromiumProfileError> {
    let mut entries = std::fs::read_dir(path).map_err(|source| ChromiumProfileError::Inspect {
        path: path.to_path_buf(),
        source,
    })?;
    entries
        .next()
        .transpose()
        .map(|entry| entry.is_some())
        .map_err(|source| ChromiumProfileError::Inspect {
            path: path.to_path_buf(),
            source,
        })
}

fn validate_persistent_profile_marker(path: &Path) -> Result<(), ChromiumProfileError> {
    let marker_path = path.join(PERSISTENT_PROFILE_MARKER_FILE);
    let marker =
        std::fs::read(&marker_path).map_err(|source| ChromiumProfileError::ReadMarker {
            path: marker_path.clone(),
            source,
        })?;
    if marker != PERSISTENT_PROFILE_MARKER {
        return Err(ChromiumProfileError::InvalidMarker(marker_path));
    }
    Ok(())
}

/// An explicit launch request. No constructor performs browser discovery.
#[derive(Clone, Debug)]
pub struct ChromiumLaunchRequest {
    executable: ChromiumExecutable,
    user_data_dir: PathBuf,
    profile_policy: ChromiumProfilePolicy,
    headless: bool,
    additional_arguments: Vec<OsString>,
    startup_timeout: Duration,
    connection_config: ConnectionConfig,
}

impl ChromiumLaunchRequest {
    pub fn new(
        executable: ChromiumExecutable,
        user_data_dir: impl Into<PathBuf>,
        connection_config: ConnectionConfig,
    ) -> Self {
        Self {
            executable,
            user_data_dir: user_data_dir.into(),
            profile_policy: ChromiumProfilePolicy::Fresh,
            headless: true,
            additional_arguments: Vec::new(),
            startup_timeout: Duration::from_secs(10),
            connection_config,
        }
    }

    pub fn new_persistent(
        executable: ChromiumExecutable,
        profile: ChromiumPersistentProfile,
        connection_config: ConnectionConfig,
    ) -> Self {
        Self {
            executable,
            user_data_dir: profile.path,
            profile_policy: ChromiumProfilePolicy::Persistent,
            headless: true,
            additional_arguments: Vec::new(),
            startup_timeout: Duration::from_secs(10),
            connection_config,
        }
    }

    pub const fn executable(&self) -> &ChromiumExecutable {
        &self.executable
    }

    pub fn user_data_dir(&self) -> &Path {
        &self.user_data_dir
    }

    pub const fn profile_policy(&self) -> ChromiumProfilePolicy {
        self.profile_policy
    }

    pub const fn headless(&self) -> bool {
        self.headless
    }

    pub fn additional_arguments(&self) -> &[OsString] {
        &self.additional_arguments
    }

    pub const fn startup_timeout(&self) -> Duration {
        self.startup_timeout
    }

    /// Returns the exact CDP resource configuration that the launched
    /// connection must use. Composition roots validate this before spawning
    /// the owned process so configuration drift cannot fail only after launch.
    pub const fn connection_config(&self) -> &ConnectionConfig {
        &self.connection_config
    }

    pub fn with_headless(mut self, headless: bool) -> Self {
        self.headless = headless;
        self
    }

    pub fn with_startup_timeout(mut self, timeout: Duration) -> Self {
        self.startup_timeout = timeout;
        self
    }

    pub fn with_argument(mut self, argument: impl Into<OsString>) -> Self {
        self.additional_arguments.push(argument.into());
        self
    }

    fn validate(&self) -> Result<(), ChromiumLaunchRequestError> {
        validate_executable_file(self.executable.path()).map_err(|error| {
            ChromiumLaunchRequestError::ExecutableChanged {
                message: error.to_string(),
            }
        })?;
        if !self.user_data_dir.is_absolute() {
            return Err(ChromiumLaunchRequestError::ProfileNotAbsolute(
                self.user_data_dir.clone(),
            ));
        }
        let metadata = std::fs::metadata(&self.user_data_dir).map_err(|source| {
            ChromiumLaunchRequestError::InspectProfile {
                path: self.user_data_dir.clone(),
                source,
            }
        })?;
        if !metadata.is_dir() {
            return Err(ChromiumLaunchRequestError::ProfileNotDirectory(
                self.user_data_dir.clone(),
            ));
        }
        match self.profile_policy {
            ChromiumProfilePolicy::Fresh => {
                if profile_has_entries(&self.user_data_dir)
                    .map_err(ChromiumLaunchRequestError::ProfilePolicy)?
                {
                    return Err(ChromiumLaunchRequestError::ProfileNotEmpty(
                        self.user_data_dir.clone(),
                    ));
                }
            }
            ChromiumProfilePolicy::Persistent => {
                validate_persistent_profile_marker(&self.user_data_dir)
                    .map_err(ChromiumLaunchRequestError::ProfilePolicy)?;
            }
        }
        if self.startup_timeout.is_zero() {
            return Err(ChromiumLaunchRequestError::ZeroStartupTimeout);
        }
        for argument in &self.additional_arguments {
            if let Some(reserved) = reserved_argument(argument) {
                return Err(ChromiumLaunchRequestError::ReservedArgument {
                    argument: argument.clone(),
                    reserved,
                });
            }
        }
        Ok(())
    }

    fn command_arguments(&self) -> Vec<OsString> {
        let mut arguments = Vec::with_capacity(self.additional_arguments.len().saturating_add(7));
        arguments.push(OsString::from("--no-first-run"));
        arguments.push(OsString::from("--no-default-browser-check"));
        arguments.push(OsString::from(format!(
            "--remote-debugging-address={LOOPBACK_DEBUGGING_ADDRESS}"
        )));
        arguments.push(OsString::from("--remote-debugging-port=0"));
        let mut profile = OsString::from("--user-data-dir=");
        profile.push(self.user_data_dir.as_os_str());
        arguments.push(profile);
        if self.headless {
            arguments.push(OsString::from("--headless=new"));
        }
        arguments.extend(self.additional_arguments.iter().cloned());
        arguments.push(OsString::from("about:blank"));
        arguments
    }

    fn prepare_profile_for_launch(&self) -> Result<(), ChromiumLaunchRequestError> {
        if self.profile_policy != ChromiumProfilePolicy::Persistent {
            return Ok(());
        }
        let active_port = self.user_data_dir.join(ACTIVE_PORT_FILE);
        match std::fs::remove_file(&active_port) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(ChromiumLaunchRequestError::ClearStaleActivePort {
                path: active_port,
                source,
            }),
        }
    }
}

fn reserved_argument(argument: &OsStr) -> Option<&'static str> {
    let argument = argument.to_string_lossy();
    RESERVED_ARGUMENTS.iter().copied().find(|reserved| {
        argument == *reserved
            || argument
                .strip_prefix(*reserved)
                .is_some_and(|suffix| suffix.starts_with('='))
    })
}

#[derive(Debug)]
pub enum ChromiumLaunchRequestError {
    ExecutableChanged {
        message: String,
    },
    ProfileNotAbsolute(PathBuf),
    InspectProfile {
        path: PathBuf,
        source: io::Error,
    },
    ProfileNotDirectory(PathBuf),
    ProfileNotEmpty(PathBuf),
    ProfilePolicy(ChromiumProfileError),
    ClearStaleActivePort {
        path: PathBuf,
        source: io::Error,
    },
    ZeroStartupTimeout,
    ReservedArgument {
        argument: OsString,
        reserved: &'static str,
    },
}

impl Display for ChromiumLaunchRequestError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExecutableChanged { message } => {
                write!(
                    formatter,
                    "Chromium executable is no longer valid: {message}"
                )
            }
            Self::ProfileNotAbsolute(path) => write!(
                formatter,
                "Chromium user-data directory is not absolute: {}",
                path.display()
            ),
            Self::InspectProfile { path, source } => write!(
                formatter,
                "could not inspect Chromium user-data directory {}: {source}",
                path.display()
            ),
            Self::ProfileNotDirectory(path) => write!(
                formatter,
                "Chromium user-data path is not a directory: {}",
                path.display()
            ),
            Self::ProfileNotEmpty(path) => write!(
                formatter,
                "Chromium launch requires a dedicated empty user-data directory: {}",
                path.display()
            ),
            Self::ProfilePolicy(error) => {
                write!(
                    formatter,
                    "could not validate Chromium profile policy: {error}"
                )
            }
            Self::ClearStaleActivePort { path, source } => write!(
                formatter,
                "could not clear stale Chromium active-port file {}: {source}",
                path.display()
            ),
            Self::ZeroStartupTimeout => {
                formatter.write_str("Chromium startup timeout must be nonzero")
            }
            Self::ReservedArgument { argument, reserved } => write!(
                formatter,
                "Chromium argument `{}` overrides provider-owned `{reserved}` configuration",
                argument.to_string_lossy()
            ),
        }
    }
}

impl Error for ChromiumLaunchRequestError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InspectProfile { source, .. } | Self::ClearStaleActivePort { source, .. } => {
                Some(source)
            }
            Self::ProfilePolicy(error) => Some(error),
            _ => None,
        }
    }
}

/// Minimal process operations required by the launch provider and its lease.
pub trait ChromiumChildProcess: Send + 'static {
    type WaitFuture<'a>: Future<Output = io::Result<ExitStatus>> + Send
    where
        Self: 'a;

    fn process_id(&self) -> Option<u32>;
    fn set_kill_on_drop(&mut self, enabled: bool);
    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>>;
    fn start_kill(&mut self) -> io::Result<()>;
    fn wait(&mut self) -> Self::WaitFuture<'_>;
}

pub trait ChromiumProcessSpawner: Send + Sync {
    type Child: ChromiumChildProcess;

    fn spawn(&self, executable: &Path, arguments: &[OsString]) -> io::Result<Self::Child>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TokioChromiumProcessSpawner;

pub struct TokioChromiumChild {
    child: tokio::process::Child,
    kill_on_drop: bool,
}

impl ChromiumProcessSpawner for TokioChromiumProcessSpawner {
    type Child = TokioChromiumChild;

    fn spawn(&self, executable: &Path, arguments: &[OsString]) -> io::Result<Self::Child> {
        let mut command = tokio::process::Command::new(executable);
        command
            .args(arguments)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        command.spawn().map(|child| TokioChromiumChild {
            child,
            kill_on_drop: true,
        })
    }
}

impl ChromiumChildProcess for TokioChromiumChild {
    type WaitFuture<'a>
        = Pin<Box<dyn Future<Output = io::Result<ExitStatus>> + Send + 'a>>
    where
        Self: 'a;

    fn process_id(&self) -> Option<u32> {
        self.child.id()
    }

    fn set_kill_on_drop(&mut self, enabled: bool) {
        self.kill_on_drop = enabled;
    }

    fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }

    fn start_kill(&mut self) -> io::Result<()> {
        self.child.start_kill()
    }

    fn wait(&mut self) -> Self::WaitFuture<'_> {
        Box::pin(self.child.wait())
    }
}

impl Drop for TokioChromiumChild {
    fn drop(&mut self) {
        if self.kill_on_drop {
            let _ = self.child.start_kill();
        }
    }
}

#[derive(Debug)]
pub enum ChromiumStartupError {
    ObserveProcess(io::Error),
    ProcessExited { code: Option<i32> },
    ReadActivePort { path: PathBuf, source: io::Error },
    MalformedActivePort { reason: &'static str },
    Timeout { timeout: Duration },
}

impl Display for ChromiumStartupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObserveProcess(error) => {
                write!(
                    formatter,
                    "could not observe Chromium startup process: {error}"
                )
            }
            Self::ProcessExited { code } => {
                write!(
                    formatter,
                    "Chromium exited before CDP became ready (code {code:?})"
                )
            }
            Self::ReadActivePort { path, source } => write!(
                formatter,
                "could not read Chromium active-port file {}: {source}",
                path.display()
            ),
            Self::MalformedActivePort { reason } => {
                write!(
                    formatter,
                    "Chromium active-port file is malformed: {reason}"
                )
            }
            Self::Timeout { timeout } => {
                write!(formatter, "Chromium did not publish CDP within {timeout:?}")
            }
        }
    }
}

impl Error for ChromiumStartupError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ObserveProcess(error) | Self::ReadActivePort { source: error, .. } => Some(error),
            _ => None,
        }
    }
}

pub trait ChromiumEndpointWaiter<Child>: Send + Sync
where
    Child: ChromiumChildProcess,
{
    type Future<'a>: Future<Output = Result<CdpWebSocketEndpoint, ChromiumStartupError>> + Send
    where
        Self: 'a,
        Child: 'a;

    fn wait_for_endpoint<'a>(
        &'a self,
        child: &'a mut Child,
        user_data_dir: &'a Path,
        timeout: Duration,
    ) -> Self::Future<'a>;
}

#[derive(Clone, Copy, Debug)]
pub struct DevToolsActivePortWaiter {
    poll_interval: Duration,
}

impl DevToolsActivePortWaiter {
    pub const fn new(poll_interval: Duration) -> Self {
        Self { poll_interval }
    }

    pub const fn poll_interval(self) -> Duration {
        self.poll_interval
    }
}

impl Default for DevToolsActivePortWaiter {
    fn default() -> Self {
        Self::new(Duration::from_millis(10))
    }
}

impl<Child> ChromiumEndpointWaiter<Child> for DevToolsActivePortWaiter
where
    Child: ChromiumChildProcess,
{
    type Future<'a>
        = Pin<
        Box<dyn Future<Output = Result<CdpWebSocketEndpoint, ChromiumStartupError>> + Send + 'a>,
    >
    where
        Self: 'a,
        Child: 'a;

    fn wait_for_endpoint<'a>(
        &'a self,
        child: &'a mut Child,
        user_data_dir: &'a Path,
        timeout: Duration,
    ) -> Self::Future<'a> {
        Box::pin(async move {
            let started = tokio::time::Instant::now();
            let active_port_path = user_data_dir.join(ACTIVE_PORT_FILE);
            loop {
                if let Some(status) = child
                    .try_wait()
                    .map_err(ChromiumStartupError::ObserveProcess)?
                {
                    return Err(ChromiumStartupError::ProcessExited {
                        code: status.code(),
                    });
                }
                match tokio::fs::read_to_string(&active_port_path).await {
                    Ok(contents) => match parse_active_port(&contents) {
                        Ok(endpoint) => return Ok(endpoint),
                        Err(ChromiumStartupError::MalformedActivePort {
                            reason: "missing port" | "missing WebSocket path",
                        }) => {}
                        Err(error) => return Err(error),
                    },
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(source) => {
                        return Err(ChromiumStartupError::ReadActivePort {
                            path: active_port_path,
                            source,
                        });
                    }
                }
                if started.elapsed() >= timeout {
                    return Err(ChromiumStartupError::Timeout { timeout });
                }
                if self.poll_interval.is_zero() {
                    tokio::task::yield_now().await;
                } else {
                    tokio::time::sleep(self.poll_interval).await;
                }
            }
        })
    }
}

fn parse_active_port(contents: &str) -> Result<CdpWebSocketEndpoint, ChromiumStartupError> {
    let mut lines = contents.lines();
    let port = lines
        .next()
        .ok_or(ChromiumStartupError::MalformedActivePort {
            reason: "missing port",
        })?
        .parse::<u16>()
        .map_err(|_| ChromiumStartupError::MalformedActivePort {
            reason: "port is not a nonzero u16",
        })?;
    if port == 0 {
        return Err(ChromiumStartupError::MalformedActivePort {
            reason: "port is not a nonzero u16",
        });
    }
    let path = lines
        .next()
        .ok_or(ChromiumStartupError::MalformedActivePort {
            reason: "missing WebSocket path",
        })?;
    if !path.starts_with("/devtools/browser/") || path.bytes().any(|byte| !byte.is_ascii_graphic())
    {
        return Err(ChromiumStartupError::MalformedActivePort {
            reason: "invalid WebSocket path",
        });
    }
    if lines.any(|line| !line.is_empty()) {
        return Err(ChromiumStartupError::MalformedActivePort {
            reason: "unexpected extra lines",
        });
    }
    CdpWebSocketEndpoint::parse(format!("ws://{LOOPBACK_DEBUGGING_ADDRESS}:{port}{path}")).map_err(
        |_| ChromiumStartupError::MalformedActivePort {
            reason: "invalid loopback WebSocket endpoint",
        },
    )
}

#[derive(Debug)]
pub enum ChromiumLaunchError {
    InvalidRequest(ChromiumLaunchRequestError),
    Spawn(io::Error),
    Startup(ChromiumStartupError),
    Connect { message: String },
    CleanupAfterFailure { launch: String, cleanup: io::Error },
}

impl Display for ChromiumLaunchError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(error) => Display::fmt(error, formatter),
            Self::Spawn(error) => write!(formatter, "could not spawn Chromium: {error}"),
            Self::Startup(error) => Display::fmt(error, formatter),
            Self::Connect { message } => {
                write!(formatter, "could not connect to Chromium CDP: {message}")
            }
            Self::CleanupAfterFailure { launch, cleanup } => write!(
                formatter,
                "Chromium launch failed ({launch}) and owned-process cleanup also failed: {cleanup}"
            ),
        }
    }
}

impl Error for ChromiumLaunchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidRequest(error) => Some(error),
            Self::Spawn(error) => Some(error),
            Self::Startup(error) => Some(error),
            Self::CleanupAfterFailure { cleanup, .. } => Some(cleanup),
            Self::Connect { .. } => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ChromiumLaunchProvider<
    Spawner = TokioChromiumProcessSpawner,
    Waiter = DevToolsActivePortWaiter,
    Connector = LiveCdpConnector,
> {
    spawner: Spawner,
    waiter: Waiter,
    connector: Connector,
}

impl ChromiumLaunchProvider {
    pub const fn new() -> Self {
        Self {
            spawner: TokioChromiumProcessSpawner,
            waiter: DevToolsActivePortWaiter::new(Duration::from_millis(10)),
            connector: LiveCdpConnector,
        }
    }
}

impl Default for ChromiumLaunchProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl<Spawner, Waiter, Connector> ChromiumLaunchProvider<Spawner, Waiter, Connector> {
    pub const fn with_parts(spawner: Spawner, waiter: Waiter, connector: Connector) -> Self {
        Self {
            spawner,
            waiter,
            connector,
        }
    }
}

impl<Spawner, Waiter, Connector> LaunchBrowserProvider<ChromiumLaunchRequest>
    for ChromiumLaunchProvider<Spawner, Waiter, Connector>
where
    Spawner: ChromiumProcessSpawner,
    Waiter: ChromiumEndpointWaiter<Spawner::Child>,
    Connector: CdpConnector,
    Connector::Connection: Send,
    Connector::Error: Display + Send,
{
    type Connection = Connector::Connection;
    type Lease = ChromiumProcessLease<Spawner::Child>;
    type Error = ChromiumLaunchError;
    type Future<'a>
        = Pin<
        Box<
            dyn Future<Output = Result<LaunchedBrowser<Self::Connection, Self::Lease>, Self::Error>>
                + Send
                + 'a,
        >,
    >
    where
        Self: 'a;

    fn launch(&self, request: ChromiumLaunchRequest) -> Self::Future<'_> {
        Box::pin(async move {
            request
                .validate()
                .map_err(ChromiumLaunchError::InvalidRequest)?;
            request
                .prepare_profile_for_launch()
                .map_err(ChromiumLaunchError::InvalidRequest)?;
            let arguments = request.command_arguments();
            let mut child = self
                .spawner
                .spawn(request.executable.path(), &arguments)
                .map_err(ChromiumLaunchError::Spawn)?;
            child.set_kill_on_drop(true);
            let process_id = child.process_id();
            let endpoint = match self
                .waiter
                .wait_for_endpoint(&mut child, &request.user_data_dir, request.startup_timeout)
                .await
            {
                Ok(endpoint) => endpoint,
                Err(error) => {
                    return Err(
                        cleanup_after_failure(child, ChromiumLaunchError::Startup(error)).await,
                    );
                }
            };
            let connection = match self
                .connector
                .connect(endpoint, request.connection_config)
                .await
            {
                Ok(connection) => connection,
                Err(error) => {
                    return Err(cleanup_after_failure(
                        child,
                        ChromiumLaunchError::Connect {
                            message: error.to_string(),
                        },
                    )
                    .await);
                }
            };
            child.set_kill_on_drop(false);
            let lease = ChromiumProcessLease {
                child,
                process_id,
                user_data_dir: request.user_data_dir,
            };
            Ok(LaunchedBrowser::new(connection, lease))
        })
    }
}

async fn cleanup_after_failure<Child: ChromiumChildProcess>(
    mut child: Child,
    launch: ChromiumLaunchError,
) -> ChromiumLaunchError {
    child.set_kill_on_drop(true);
    match terminate_process(&mut child).await {
        Ok(_) => launch,
        Err(cleanup) => ChromiumLaunchError::CleanupAfterFailure {
            launch: launch.to_string(),
            cleanup,
        },
    }
}

#[derive(Debug)]
pub struct ChromiumProcessLease<Child> {
    child: Child,
    process_id: Option<u32>,
    user_data_dir: PathBuf,
}

impl<Child> ChromiumProcessLease<Child> {
    pub const fn process_id(&self) -> Option<u32> {
        self.process_id
    }

    pub fn user_data_dir(&self) -> &Path {
        &self.user_data_dir
    }
}

impl<Child> ChromiumProcessLease<Child>
where
    Child: ChromiumChildProcess,
{
    /// Observes process exit without consuming termination authority or
    /// blocking for a running browser.
    pub fn try_status(&mut self) -> io::Result<Option<ExitStatus>> {
        self.child.try_wait()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChromiumTermination {
    pub process_id: Option<u32>,
    pub status: ExitStatus,
    pub was_running: bool,
}

/// Time bounds for orderly browser shutdown and forced process cleanup.
///
/// Every phase is explicit so a protocol command timeout cannot silently
/// become the process-exit timeout. Construction rejects zero-duration phases.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChromiumShutdownPolicy {
    close_command_timeout: Duration,
    graceful_exit_timeout: Duration,
    forced_exit_timeout: Duration,
}

impl ChromiumShutdownPolicy {
    pub fn new(
        close_command_timeout: Duration,
        graceful_exit_timeout: Duration,
        forced_exit_timeout: Duration,
    ) -> Result<Self, ChromiumShutdownPolicyError> {
        if close_command_timeout.is_zero() {
            return Err(ChromiumShutdownPolicyError::ZeroCloseCommandTimeout);
        }
        if graceful_exit_timeout.is_zero() {
            return Err(ChromiumShutdownPolicyError::ZeroGracefulExitTimeout);
        }
        if forced_exit_timeout.is_zero() {
            return Err(ChromiumShutdownPolicyError::ZeroForcedExitTimeout);
        }
        Ok(Self {
            close_command_timeout,
            graceful_exit_timeout,
            forced_exit_timeout,
        })
    }

    pub const fn close_command_timeout(self) -> Duration {
        self.close_command_timeout
    }

    pub const fn graceful_exit_timeout(self) -> Duration {
        self.graceful_exit_timeout
    }

    pub const fn forced_exit_timeout(self) -> Duration {
        self.forced_exit_timeout
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumShutdownPolicyError {
    ZeroCloseCommandTimeout,
    ZeroGracefulExitTimeout,
    ZeroForcedExitTimeout,
}

impl Display for ChromiumShutdownPolicyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ZeroCloseCommandTimeout => "Chromium close-command timeout must be nonzero",
            Self::ZeroGracefulExitTimeout => "Chromium graceful-exit timeout must be nonzero",
            Self::ZeroForcedExitTimeout => "Chromium forced-exit timeout must be nonzero",
        })
    }
}

impl Error for ChromiumShutdownPolicyError {}

/// Result of the bounded protocol-level `Browser.close` attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ChromiumCloseAttempt {
    Acknowledged,
    TimedOut,
    Failed { message: String },
}

/// Sends the browser-specific graceful-close request within the supplied bound.
///
/// Returning a classified outcome instead of an error is intentional: a lost
/// acknowledgement does not prove the process stayed alive, so shutdown must
/// still observe the owned process before escalating.
pub trait ChromiumGracefulCloser<Connection>: Send + Sync {
    type Future<'a>: Future<Output = ChromiumCloseAttempt> + Send
    where
        Self: 'a,
        Connection: 'a;

    fn close<'a>(&'a self, connection: &'a Connection, timeout: Duration) -> Self::Future<'a>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LiveCdpBrowserCloser;

impl ChromiumGracefulCloser<std::sync::Arc<CdpConnection>> for LiveCdpBrowserCloser {
    type Future<'a>
        = Pin<Box<dyn Future<Output = ChromiumCloseAttempt> + Send + 'a>>
    where
        Self: 'a;

    fn close<'a>(
        &'a self,
        connection: &'a std::sync::Arc<CdpConnection>,
        timeout: Duration,
    ) -> Self::Future<'a> {
        Box::pin(async move {
            let session = CdpSession::browser(std::sync::Arc::clone(connection));
            match tokio::time::timeout(timeout, session.browser_close()).await {
                Ok(Ok(())) => ChromiumCloseAttempt::Acknowledged,
                Ok(Err(error)) => ChromiumCloseAttempt::Failed {
                    message: error.to_string(),
                },
                Err(_) => ChromiumCloseAttempt::TimedOut,
            }
        })
    }
}

/// Waits for one owned process to exit, returning `None` only on timeout.
pub trait ChromiumProcessExitWaiter<Child>: Send + Sync
where
    Child: ChromiumChildProcess,
{
    type Future<'a>: Future<Output = io::Result<Option<ExitStatus>>> + Send
    where
        Self: 'a,
        Child: 'a;

    fn wait_for_exit<'a>(&'a self, child: &'a mut Child, timeout: Duration) -> Self::Future<'a>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TokioChromiumProcessExitWaiter;

impl<Child> ChromiumProcessExitWaiter<Child> for TokioChromiumProcessExitWaiter
where
    Child: ChromiumChildProcess,
{
    type Future<'a>
        = Pin<Box<dyn Future<Output = io::Result<Option<ExitStatus>>> + Send + 'a>>
    where
        Self: 'a,
        Child: 'a;

    fn wait_for_exit<'a>(&'a self, child: &'a mut Child, timeout: Duration) -> Self::Future<'a> {
        Box::pin(async move {
            match tokio::time::timeout(timeout, child.wait()).await {
                Ok(result) => result.map(Some),
                Err(_) => Ok(None),
            }
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChromiumShutdownMethod {
    AlreadyExited,
    Graceful,
    Forced,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ChromiumShutdownReport {
    pub process_id: Option<u32>,
    pub status: ExitStatus,
    pub method: ChromiumShutdownMethod,
    pub close_attempt: Option<ChromiumCloseAttempt>,
    pub graceful_wait_error: Option<String>,
    pub user_data_dir: PathBuf,
}

#[derive(Debug)]
pub enum ChromiumShutdownError {
    ObserveProcess(io::Error),
    ForceKill(io::Error),
    ObserveAfterFailedKill(io::Error),
    WaitAfterForce(io::Error),
    ForcedExitTimeout { timeout: Duration },
}

impl Display for ChromiumShutdownError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObserveProcess(error) => {
                write!(
                    formatter,
                    "could not observe owned Chromium process: {error}"
                )
            }
            Self::ForceKill(error) => {
                write!(
                    formatter,
                    "could not force-terminate owned Chromium process: {error}"
                )
            }
            Self::ObserveAfterFailedKill(error) => write!(
                formatter,
                "could not observe Chromium after force-termination failed: {error}"
            ),
            Self::WaitAfterForce(error) => write!(
                formatter,
                "could not wait for Chromium after force termination: {error}"
            ),
            Self::ForcedExitTimeout { timeout } => write!(
                formatter,
                "Chromium did not exit within {timeout:?} after force termination"
            ),
        }
    }
}

impl Error for ChromiumShutdownError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ObserveProcess(error)
            | Self::ForceKill(error)
            | Self::ObserveAfterFailedKill(error)
            | Self::WaitAfterForce(error) => Some(error),
            Self::ForcedExitTimeout { .. } => None,
        }
    }
}

/// Coordinates protocol close, process observation, and bounded escalation.
///
/// Only a `LaunchedBrowser` carrying the consuming process lease can enter this
/// path. Dropping the returned future before completion activates the lease's
/// kill-on-drop guard, so cancellation cannot silently orphan an owned child.
#[derive(Clone, Debug)]
pub struct ChromiumBrowserShutdown<
    Closer = LiveCdpBrowserCloser,
    ExitWaiter = TokioChromiumProcessExitWaiter,
> {
    closer: Closer,
    exit_waiter: ExitWaiter,
}

impl ChromiumBrowserShutdown {
    pub const fn new() -> Self {
        Self {
            closer: LiveCdpBrowserCloser,
            exit_waiter: TokioChromiumProcessExitWaiter,
        }
    }
}

impl Default for ChromiumBrowserShutdown {
    fn default() -> Self {
        Self::new()
    }
}

impl<Closer, ExitWaiter> ChromiumBrowserShutdown<Closer, ExitWaiter> {
    pub const fn with_parts(closer: Closer, exit_waiter: ExitWaiter) -> Self {
        Self {
            closer,
            exit_waiter,
        }
    }

    pub fn shutdown<'a, Connection, Child>(
        &'a self,
        browser: LaunchedBrowser<Connection, ChromiumProcessLease<Child>>,
        policy: ChromiumShutdownPolicy,
    ) -> impl Future<Output = Result<ChromiumShutdownReport, ChromiumShutdownError>> + Send + 'a
    where
        Connection: Send + 'a,
        Child: ChromiumChildProcess + 'a,
        Closer: ChromiumGracefulCloser<Connection>,
        ExitWaiter: ChromiumProcessExitWaiter<Child>,
    {
        let (connection, mut lease) = browser.into_connection_and_lease();
        lease.child.set_kill_on_drop(true);
        async move {
            let process_id = lease.process_id;
            let user_data_dir = lease.user_data_dir.clone();
            if let Some(status) = lease
                .child
                .try_wait()
                .map_err(ChromiumShutdownError::ObserveProcess)?
            {
                lease.child.set_kill_on_drop(false);
                return Ok(ChromiumShutdownReport {
                    process_id,
                    status,
                    method: ChromiumShutdownMethod::AlreadyExited,
                    close_attempt: None,
                    graceful_wait_error: None,
                    user_data_dir,
                });
            }

            let close_attempt = self
                .closer
                .close(&connection, policy.close_command_timeout)
                .await;
            drop(connection);
            let mut graceful_wait_error = None;
            match self
                .exit_waiter
                .wait_for_exit(&mut lease.child, policy.graceful_exit_timeout)
                .await
            {
                Ok(Some(status)) => {
                    lease.child.set_kill_on_drop(false);
                    return Ok(ChromiumShutdownReport {
                        process_id,
                        status,
                        method: ChromiumShutdownMethod::Graceful,
                        close_attempt: Some(close_attempt),
                        graceful_wait_error,
                        user_data_dir,
                    });
                }
                Ok(None) => {}
                Err(error) => graceful_wait_error = Some(error.to_string()),
            }

            if let Err(kill_error) = lease.child.start_kill() {
                match lease.child.try_wait() {
                    Ok(Some(status)) => {
                        lease.child.set_kill_on_drop(false);
                        return Ok(ChromiumShutdownReport {
                            process_id,
                            status,
                            method: ChromiumShutdownMethod::Graceful,
                            close_attempt: Some(close_attempt),
                            graceful_wait_error,
                            user_data_dir,
                        });
                    }
                    Ok(None) => return Err(ChromiumShutdownError::ForceKill(kill_error)),
                    Err(error) => {
                        return Err(ChromiumShutdownError::ObserveAfterFailedKill(error));
                    }
                }
            }
            let status = self
                .exit_waiter
                .wait_for_exit(&mut lease.child, policy.forced_exit_timeout)
                .await
                .map_err(ChromiumShutdownError::WaitAfterForce)?
                .ok_or(ChromiumShutdownError::ForcedExitTimeout {
                    timeout: policy.forced_exit_timeout,
                })?;
            lease.child.set_kill_on_drop(false);
            Ok(ChromiumShutdownReport {
                process_id,
                status,
                method: ChromiumShutdownMethod::Forced,
                close_attempt: Some(close_attempt),
                graceful_wait_error,
                user_data_dir,
            })
        }
    }
}

impl<Child> BrowserProcessLease for ChromiumProcessLease<Child>
where
    Child: ChromiumChildProcess,
{
    type Termination = ChromiumTermination;
    type Error = io::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Termination, Self::Error>> + Send>>;

    fn terminate(mut self) -> Self::Future {
        self.child.set_kill_on_drop(true);
        Box::pin(async move {
            let (status, was_running) = terminate_process(&mut self.child).await?;
            Ok(ChromiumTermination {
                process_id: self.process_id,
                status,
                was_running,
            })
        })
    }
}

async fn terminate_process<Child: ChromiumChildProcess>(
    child: &mut Child,
) -> io::Result<(ExitStatus, bool)> {
    if let Some(status) = child.try_wait()? {
        return Ok((status, false));
    }
    if let Err(error) = child.start_kill() {
        if let Some(status) = child.try_wait()? {
            return Ok((status, false));
        }
        return Err(error);
    }
    child.wait().await.map(|status| (status, true))
}

/// Concrete connection returned by the default live provider.
pub type LiveChromiumConnection = std::sync::Arc<CdpConnection>;

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::convert::Infallible;
    use std::fmt::{self, Display, Formatter};
    use std::fs;
    use std::future::{Ready, pending, ready};
    use std::path::{Path, PathBuf};
    use std::pin::Pin;
    use std::process::ExitStatus;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use pwright_browser_provider::{BrowserProcessLease, LaunchBrowserProvider, LaunchedBrowser};
    use pwright_cdp::connection::ConnectionConfig;
    use pwright_provider_cdp::{CdpConnector, CdpWebSocketEndpoint};

    use super::{
        ChromiumBrowserShutdown, ChromiumChannel, ChromiumChildProcess, ChromiumCloseAttempt,
        ChromiumEndpointWaiter, ChromiumExecutable, ChromiumExecutableError,
        ChromiumGracefulCloser, ChromiumLaunchError, ChromiumLaunchProvider, ChromiumLaunchRequest,
        ChromiumLaunchRequestError, ChromiumPersistentProfile, ChromiumProcessExitWaiter,
        ChromiumProcessLease, ChromiumProcessSpawner, ChromiumProfileError, ChromiumProfilePolicy,
        ChromiumShutdownError, ChromiumShutdownMethod, ChromiumShutdownPolicy,
        ChromiumShutdownPolicyError, ChromiumStartupError, DevToolsActivePortWaiter,
        discover_candidates, parse_active_port,
    };

    static TEST_DIRECTORY_ID: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let id = TEST_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "pwright-provider-chromium-{label}-{}-{id}",
                std::process::id()
            ));
            fs::create_dir(&path).expect("test directory should be unique");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }

        fn directory(&self, name: &str) -> PathBuf {
            let path = self.path.join(name);
            fs::create_dir(&path).expect("test child directory should be created");
            path
        }

        fn executable(&self, name: &str) -> PathBuf {
            let path = self.path.join(name);
            fs::write(&path, b"test executable").expect("test executable should be written");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                    .expect("test executable permissions should be set");
            }
            path
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    #[cfg(unix)]
    fn success_status() -> ExitStatus {
        use std::os::unix::process::ExitStatusExt;

        ExitStatus::from_raw(0)
    }

    #[cfg(windows)]
    fn success_status() -> ExitStatus {
        use std::os::windows::process::ExitStatusExt;

        ExitStatus::from_raw(0)
    }

    #[derive(Debug)]
    struct ProcessState {
        executable: PathBuf,
        arguments: Vec<String>,
        kill_on_drop: bool,
        kills: usize,
        waits: usize,
        running: bool,
    }

    impl Default for ProcessState {
        fn default() -> Self {
            Self {
                executable: PathBuf::new(),
                arguments: Vec::new(),
                kill_on_drop: false,
                kills: 0,
                waits: 0,
                running: true,
            }
        }
    }

    #[derive(Clone, Debug)]
    struct RecordingSpawner {
        state: Arc<Mutex<ProcessState>>,
    }

    #[derive(Debug)]
    struct FakeChild {
        state: Arc<Mutex<ProcessState>>,
    }

    impl ChromiumProcessSpawner for RecordingSpawner {
        type Child = FakeChild;

        fn spawn(
            &self,
            executable: &Path,
            arguments: &[std::ffi::OsString],
        ) -> std::io::Result<Self::Child> {
            let mut state = self.state.lock().expect("process state should lock");
            state.executable = executable.to_path_buf();
            state.arguments = arguments
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect();
            state.running = true;
            drop(state);
            Ok(FakeChild {
                state: Arc::clone(&self.state),
            })
        }
    }

    impl ChromiumChildProcess for FakeChild {
        type WaitFuture<'a>
            = Ready<std::io::Result<ExitStatus>>
        where
            Self: 'a;

        fn process_id(&self) -> Option<u32> {
            Some(4242)
        }

        fn set_kill_on_drop(&mut self, enabled: bool) {
            self.state
                .lock()
                .expect("process state should lock")
                .kill_on_drop = enabled;
        }

        fn try_wait(&mut self) -> std::io::Result<Option<ExitStatus>> {
            let state = self.state.lock().expect("process state should lock");
            Ok((!state.running).then(success_status))
        }

        fn start_kill(&mut self) -> std::io::Result<()> {
            let mut state = self.state.lock().expect("process state should lock");
            state.kills += 1;
            state.running = false;
            Ok(())
        }

        fn wait(&mut self) -> Self::WaitFuture<'_> {
            let mut state = self.state.lock().expect("process state should lock");
            state.waits += 1;
            state.running = false;
            ready(Ok(success_status()))
        }
    }

    impl Drop for FakeChild {
        fn drop(&mut self) {
            let mut state = self.state.lock().expect("process state should lock");
            if state.kill_on_drop && state.running {
                state.kills += 1;
                state.running = false;
            }
        }
    }

    #[derive(Clone, Debug)]
    struct ImmediateWaiter {
        requests: Arc<Mutex<Vec<(PathBuf, Duration)>>>,
    }

    impl ChromiumEndpointWaiter<FakeChild> for ImmediateWaiter {
        type Future<'a>
            = Ready<Result<CdpWebSocketEndpoint, ChromiumStartupError>>
        where
            Self: 'a,
            FakeChild: 'a;

        fn wait_for_endpoint<'a>(
            &'a self,
            _child: &'a mut FakeChild,
            user_data_dir: &'a Path,
            timeout: Duration,
        ) -> Self::Future<'a> {
            self.requests
                .lock()
                .expect("waiter requests should lock")
                .push((user_data_dir.to_path_buf(), timeout));
            ready(
                CdpWebSocketEndpoint::parse("ws://127.0.0.1:43117/devtools/browser/generated")
                    .map_err(|_| ChromiumStartupError::MalformedActivePort {
                        reason: "test endpoint",
                    }),
            )
        }
    }

    #[derive(Clone, Debug)]
    struct RecordingConnector {
        requests: Arc<Mutex<Vec<(String, ConnectionConfig)>>>,
    }

    impl CdpConnector for RecordingConnector {
        type Connection = String;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Connection, Self::Error>>;

        fn connect(
            &self,
            endpoint: CdpWebSocketEndpoint,
            config: ConnectionConfig,
        ) -> Self::Future<'_> {
            self.requests
                .lock()
                .expect("connector requests should lock")
                .push((endpoint.as_str().to_string(), config));
            ready(Ok("live-cdp-connection".to_string()))
        }
    }

    #[derive(Clone, Copy, Debug)]
    struct FailingConnector;

    #[derive(Clone, Copy, Debug)]
    struct TestConnectError;

    impl Display for TestConnectError {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("injected connection failure")
        }
    }

    impl CdpConnector for FailingConnector {
        type Connection = String;
        type Error = TestConnectError;
        type Future<'a> = Ready<Result<Self::Connection, Self::Error>>;

        fn connect(
            &self,
            _endpoint: CdpWebSocketEndpoint,
            _config: ConnectionConfig,
        ) -> Self::Future<'_> {
            ready(Err(TestConnectError))
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum CloseBehavior {
        AcknowledgeAndExit,
        TimedOut,
        FailedAndExit,
        Pending,
    }

    #[derive(Clone, Debug)]
    struct ShutdownConnection {
        state: Arc<Mutex<ProcessState>>,
    }

    #[derive(Clone, Debug)]
    struct RecordingCloser {
        behavior: CloseBehavior,
        timeouts: Arc<Mutex<Vec<Duration>>>,
        started: Arc<AtomicUsize>,
    }

    impl ChromiumGracefulCloser<ShutdownConnection> for RecordingCloser {
        type Future<'a>
            = Pin<Box<dyn Future<Output = ChromiumCloseAttempt> + Send + 'a>>
        where
            Self: 'a,
            ShutdownConnection: 'a;

        fn close<'a>(
            &'a self,
            connection: &'a ShutdownConnection,
            timeout: Duration,
        ) -> Self::Future<'a> {
            self.timeouts
                .lock()
                .expect("close timeouts should lock")
                .push(timeout);
            self.started.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move {
                match self.behavior {
                    CloseBehavior::AcknowledgeAndExit => {
                        connection
                            .state
                            .lock()
                            .expect("process state should lock")
                            .running = false;
                        ChromiumCloseAttempt::Acknowledged
                    }
                    CloseBehavior::TimedOut => ChromiumCloseAttempt::TimedOut,
                    CloseBehavior::FailedAndExit => {
                        connection
                            .state
                            .lock()
                            .expect("process state should lock")
                            .running = false;
                        ChromiumCloseAttempt::Failed {
                            message: "connection closed before acknowledgement".to_string(),
                        }
                    }
                    CloseBehavior::Pending => pending().await,
                }
            })
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum ExitWaitOutcome {
        ObserveCurrent,
        Timeout,
        Error,
    }

    #[derive(Clone, Debug)]
    struct ScriptedExitWaiter {
        outcomes: Arc<Mutex<VecDeque<ExitWaitOutcome>>>,
        timeouts: Arc<Mutex<Vec<Duration>>>,
    }

    impl ChromiumProcessExitWaiter<FakeChild> for ScriptedExitWaiter {
        type Future<'a>
            = Ready<std::io::Result<Option<ExitStatus>>>
        where
            Self: 'a,
            FakeChild: 'a;

        fn wait_for_exit<'a>(
            &'a self,
            child: &'a mut FakeChild,
            timeout: Duration,
        ) -> Self::Future<'a> {
            self.timeouts
                .lock()
                .expect("exit timeouts should lock")
                .push(timeout);
            let outcome = self
                .outcomes
                .lock()
                .expect("exit outcomes should lock")
                .pop_front()
                .expect("test should provide one exit outcome per wait");
            ready(match outcome {
                ExitWaitOutcome::ObserveCurrent => child.try_wait(),
                ExitWaitOutcome::Timeout => Ok(None),
                ExitWaitOutcome::Error => Err(std::io::Error::other(
                    "injected process observation failure",
                )),
            })
        }
    }

    fn shutdown_policy() -> ChromiumShutdownPolicy {
        ChromiumShutdownPolicy::new(
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_secs(3),
        )
        .expect("shutdown policy should validate")
    }

    type ShutdownTestFixture = (
        ChromiumBrowserShutdown<RecordingCloser, ScriptedExitWaiter>,
        LaunchedBrowser<ShutdownConnection, ChromiumProcessLease<FakeChild>>,
        Arc<Mutex<ProcessState>>,
        Arc<Mutex<Vec<Duration>>>,
        Arc<Mutex<Vec<Duration>>>,
        Arc<AtomicUsize>,
    );

    fn shutdown_fixture(
        behavior: CloseBehavior,
        outcomes: impl IntoIterator<Item = ExitWaitOutcome>,
    ) -> ShutdownTestFixture {
        let state = Arc::new(Mutex::new(ProcessState::default()));
        let close_timeouts = Arc::new(Mutex::new(Vec::new()));
        let exit_timeouts = Arc::new(Mutex::new(Vec::new()));
        let close_started = Arc::new(AtomicUsize::new(0));
        let shutdown = ChromiumBrowserShutdown::with_parts(
            RecordingCloser {
                behavior,
                timeouts: Arc::clone(&close_timeouts),
                started: Arc::clone(&close_started),
            },
            ScriptedExitWaiter {
                outcomes: Arc::new(Mutex::new(outcomes.into_iter().collect())),
                timeouts: Arc::clone(&exit_timeouts),
            },
        );
        let browser = LaunchedBrowser::new(
            ShutdownConnection {
                state: Arc::clone(&state),
            },
            ChromiumProcessLease {
                child: FakeChild {
                    state: Arc::clone(&state),
                },
                process_id: Some(4242),
                user_data_dir: PathBuf::from("/owned/profile"),
            },
        );
        (
            shutdown,
            browser,
            state,
            close_timeouts,
            exit_timeouts,
            close_started,
        )
    }

    fn executable_and_profile(tree: &TestDirectory) -> (ChromiumExecutable, PathBuf) {
        let executable = ChromiumExecutable::from_path(
            tree.executable("chromium-test"),
            ChromiumChannel::Custom,
        )
        .expect("test executable should validate");
        let profile = tree.directory("profile");
        (executable, profile)
    }

    #[test]
    fn explicit_executable_and_discovery_are_absolute_deduplicated_values() {
        // Arrange
        let tree = TestDirectory::new("discovery");
        let executable_path = tree.executable("chromium-test");
        let missing = tree.path().join("missing");

        // Act
        let explicit =
            ChromiumExecutable::from_path(executable_path.clone(), ChromiumChannel::Custom);
        let relative = ChromiumExecutable::from_path("chromium", ChromiumChannel::Custom);
        let discovered = discover_candidates([
            (executable_path.clone(), ChromiumChannel::Chromium),
            (executable_path, ChromiumChannel::Chrome),
            (missing, ChromiumChannel::Chrome),
        ]);

        // Assert
        assert!(
            explicit
                .expect("absolute executable should validate")
                .path()
                .is_absolute()
        );
        assert!(matches!(
            relative,
            Err(ChromiumExecutableError::NotAbsolute(_))
        ));
        assert_eq!(discovered.len(), 1);
        assert_eq!(discovered[0].channel(), ChromiumChannel::Chromium);
    }

    #[test]
    fn launch_request_rejects_unsafe_profile_and_debugging_overrides() {
        // Arrange
        let tree = TestDirectory::new("request-validation");
        let (executable, profile) = executable_and_profile(&tree);
        fs::write(profile.join("existing-state"), b"state")
            .expect("profile marker should be written");
        let nonempty =
            ChromiumLaunchRequest::new(executable.clone(), profile, ConnectionConfig::default());
        let empty_profile = tree.directory("empty-profile");
        let reserved = ChromiumLaunchRequest::new(
            executable.clone(),
            empty_profile.clone(),
            ConnectionConfig::default(),
        )
        .with_argument("--remote-debugging-port=9222");
        let zero_timeout =
            ChromiumLaunchRequest::new(executable, empty_profile, ConnectionConfig::default())
                .with_startup_timeout(Duration::ZERO);

        // Act
        let nonempty_error = nonempty.validate();
        let reserved_error = reserved.validate();
        let timeout_error = zero_timeout.validate();

        // Assert
        assert!(matches!(
            nonempty_error,
            Err(ChromiumLaunchRequestError::ProfileNotEmpty(_))
        ));
        assert!(matches!(
            reserved_error,
            Err(ChromiumLaunchRequestError::ReservedArgument {
                reserved: "--remote-debugging-port",
                ..
            })
        ));
        assert!(matches!(
            timeout_error,
            Err(ChromiumLaunchRequestError::ZeroStartupTimeout)
        ));
    }

    #[test]
    fn persistent_profile_requires_explicit_initialization_and_versioned_marker() {
        // Arrange
        let tree = TestDirectory::new("persistent-profile");
        let executable = ChromiumExecutable::from_path(
            tree.executable("chromium-test"),
            ChromiumChannel::Custom,
        )
        .expect("test executable should validate");
        let persistent_path = tree.directory("persistent");
        let unmarked_path = tree.directory("unmarked");
        fs::write(unmarked_path.join("Default"), b"browser state")
            .expect("unmarked browser state should be written");
        let invalid_path = tree.directory("invalid");
        fs::write(
            invalid_path.join(super::PERSISTENT_PROFILE_MARKER_FILE),
            b"unknown marker version\n",
        )
        .expect("invalid marker should be written");

        // Act
        let initialized = ChromiumPersistentProfile::initialize(&persistent_path)
            .expect("empty directory should initialize");
        fs::write(persistent_path.join("Default"), b"browser state")
            .expect("persistent browser state should be written");
        fs::write(
            persistent_path.join(super::ACTIVE_PORT_FILE),
            b"9222\n/devtools/browser/stale\n",
        )
        .expect("stale active port should be written");
        let reopened = ChromiumPersistentProfile::open(&persistent_path)
            .expect("marked persistent profile should reopen");
        let request = ChromiumLaunchRequest::new_persistent(
            executable,
            reopened,
            ConnectionConfig::default(),
        );
        let validation = request.validate();
        let preparation = request.prepare_profile_for_launch();
        let unmarked = ChromiumPersistentProfile::open(&unmarked_path);
        let nonempty_initialization = ChromiumPersistentProfile::initialize(&unmarked_path);
        let invalid = ChromiumPersistentProfile::open(&invalid_path);

        // Assert
        assert_eq!(
            initialized.path(),
            fs::canonicalize(&persistent_path).expect("profile path should canonicalize")
        );
        assert_eq!(request.profile_policy(), ChromiumProfilePolicy::Persistent);
        assert!(validation.is_ok());
        assert!(preparation.is_ok());
        assert!(!persistent_path.join(super::ACTIVE_PORT_FILE).exists());
        assert!(
            persistent_path
                .join(super::PERSISTENT_PROFILE_MARKER_FILE)
                .exists()
        );
        assert!(persistent_path.join("Default").exists());
        assert!(matches!(
            unmarked,
            Err(ChromiumProfileError::ReadMarker { .. })
        ));
        assert!(matches!(
            nonempty_initialization,
            Err(ChromiumProfileError::NotEmpty(_))
        ));
        assert!(matches!(
            invalid,
            Err(ChromiumProfileError::InvalidMarker(_))
        ));
    }

    #[test]
    fn active_port_parser_builds_only_a_loopback_browser_endpoint() {
        // Arrange
        let valid = "43117\n/devtools/browser/abc-123\n";

        // Act
        let endpoint = parse_active_port(valid).expect("active port should parse");
        let zero = parse_active_port("0\n/devtools/browser/id\n");
        let foreign_path = parse_active_port("43117\n/devtools/page/id\n");
        let extra = parse_active_port("43117\n/devtools/browser/id\nuntrusted\n");

        // Assert
        assert_eq!(
            endpoint.as_str(),
            "ws://127.0.0.1:43117/devtools/browser/abc-123"
        );
        assert!(matches!(
            zero,
            Err(ChromiumStartupError::MalformedActivePort { .. })
        ));
        assert!(matches!(
            foreign_path,
            Err(ChromiumStartupError::MalformedActivePort { .. })
        ));
        assert!(matches!(
            extra,
            Err(ChromiumStartupError::MalformedActivePort { .. })
        ));
    }

    #[tokio::test]
    async fn explicit_launch_connects_with_provider_owned_flags_and_consuming_lease() {
        // Arrange
        let tree = TestDirectory::new("successful-launch");
        let (executable, profile) = executable_and_profile(&tree);
        let process_state = Arc::new(Mutex::new(ProcessState::default()));
        let waiter_requests = Arc::new(Mutex::new(Vec::new()));
        let connector_requests = Arc::new(Mutex::new(Vec::new()));
        let provider = ChromiumLaunchProvider::with_parts(
            RecordingSpawner {
                state: Arc::clone(&process_state),
            },
            ImmediateWaiter {
                requests: Arc::clone(&waiter_requests),
            },
            RecordingConnector {
                requests: Arc::clone(&connector_requests),
            },
        );
        let config = ConnectionConfig {
            timeout: Duration::from_secs(7),
            write_channel_capacity: 11,
            event_channel_capacity: 13,
            ..ConnectionConfig::default()
        };
        let request = ChromiumLaunchRequest::new(executable.clone(), &profile, config)
            .with_headless(false)
            .with_startup_timeout(Duration::from_secs(3))
            .with_argument("--disable-gpu");

        // Act
        let launched = provider
            .launch(request)
            .await
            .expect("launch should succeed");
        let (connection, mut lease) = launched.into_connection_and_lease();
        let lease_process_id = lease.process_id();
        let lease_profile = lease.user_data_dir().to_path_buf();
        let running_status = lease.try_status().expect("process should be observable");
        let termination = lease
            .terminate()
            .await
            .expect("owned process should terminate");

        // Assert
        assert_eq!(connection, "live-cdp-connection");
        assert_eq!(lease_process_id, Some(4242));
        assert_eq!(lease_profile, profile);
        assert_eq!(running_status, None);
        assert_eq!(termination.process_id, Some(4242));
        assert!(termination.was_running);
        let state = process_state.lock().expect("process state should lock");
        assert_eq!(state.executable, executable.path());
        assert_eq!(state.kills, 1);
        assert_eq!(state.waits, 1);
        assert!(state.arguments.contains(&format!(
            "--remote-debugging-address={}",
            super::LOOPBACK_DEBUGGING_ADDRESS
        )));
        assert!(
            state
                .arguments
                .contains(&"--remote-debugging-port=0".to_string())
        );
        assert!(
            state
                .arguments
                .contains(&format!("--user-data-dir={}", profile.display()))
        );
        assert!(state.arguments.contains(&"--disable-gpu".to_string()));
        assert!(!state.arguments.contains(&"--headless=new".to_string()));
        assert_eq!(
            state.arguments.last().map(String::as_str),
            Some("about:blank")
        );
        drop(state);
        assert_eq!(
            waiter_requests
                .lock()
                .expect("waiter requests should lock")
                .as_slice(),
            &[(profile, Duration::from_secs(3))]
        );
        let connector = connector_requests
            .lock()
            .expect("connector requests should lock");
        assert_eq!(
            connector[0].0,
            "ws://127.0.0.1:43117/devtools/browser/generated"
        );
        assert_eq!(connector[0].1.timeout, Duration::from_secs(7));
        assert_eq!(connector[0].1.write_channel_capacity, 11);
        assert_eq!(connector[0].1.event_channel_capacity, 13);
    }

    #[tokio::test]
    async fn connection_failure_terminates_the_owned_startup_process() {
        // Arrange
        let tree = TestDirectory::new("failed-launch");
        let (executable, profile) = executable_and_profile(&tree);
        let process_state = Arc::new(Mutex::new(ProcessState::default()));
        let provider = ChromiumLaunchProvider::with_parts(
            RecordingSpawner {
                state: Arc::clone(&process_state),
            },
            ImmediateWaiter {
                requests: Arc::new(Mutex::new(Vec::new())),
            },
            FailingConnector,
        );
        let request = ChromiumLaunchRequest::new(executable, profile, ConnectionConfig::default());

        // Act
        let result = provider.launch(request).await;

        // Assert
        assert!(matches!(result, Err(ChromiumLaunchError::Connect { .. })));
        let state = process_state.lock().expect("process state should lock");
        assert_eq!(state.kills, 1);
        assert_eq!(state.waits, 1);
        assert!(!state.running);
    }

    #[tokio::test]
    async fn dropping_a_successful_lease_does_not_implicitly_kill_the_browser() {
        // Arrange
        let tree = TestDirectory::new("lease-drop");
        let (executable, profile) = executable_and_profile(&tree);
        let process_state = Arc::new(Mutex::new(ProcessState::default()));
        let provider = ChromiumLaunchProvider::with_parts(
            RecordingSpawner {
                state: Arc::clone(&process_state),
            },
            ImmediateWaiter {
                requests: Arc::new(Mutex::new(Vec::new())),
            },
            RecordingConnector {
                requests: Arc::new(Mutex::new(Vec::new())),
            },
        );
        let request = ChromiumLaunchRequest::new(executable, profile, ConnectionConfig::default());
        let launched = provider
            .launch(request)
            .await
            .expect("launch should succeed");

        // Act
        drop(launched);

        // Assert
        let state = process_state.lock().expect("process state should lock");
        assert_eq!(state.kills, 0);
        assert!(state.running);
        assert!(!state.kill_on_drop);
    }

    #[tokio::test]
    async fn default_waiter_observes_active_port_and_early_process_exit() {
        // Arrange
        let tree = TestDirectory::new("default-waiter");
        let profile = tree.directory("profile");
        fs::write(
            profile.join(super::ACTIVE_PORT_FILE),
            b"43117\n/devtools/browser/ready\n",
        )
        .expect("active port should be written");
        let ready_state = Arc::new(Mutex::new(ProcessState::default()));
        let mut ready_child = FakeChild { state: ready_state };
        let exited_state = Arc::new(Mutex::new(ProcessState {
            running: false,
            ..ProcessState::default()
        }));
        let mut exited_child = FakeChild {
            state: exited_state,
        };
        let waiter = DevToolsActivePortWaiter::new(Duration::from_millis(1));

        // Act
        let endpoint = waiter
            .wait_for_endpoint(&mut ready_child, &profile, Duration::from_millis(20))
            .await;
        let exit = waiter
            .wait_for_endpoint(&mut exited_child, &profile, Duration::from_millis(20))
            .await;

        // Assert
        assert_eq!(
            endpoint
                .expect("active port should become an endpoint")
                .as_str(),
            "ws://127.0.0.1:43117/devtools/browser/ready"
        );
        assert!(matches!(
            exit,
            Err(ChromiumStartupError::ProcessExited { code: Some(0) })
        ));
    }

    #[tokio::test]
    async fn lease_reports_a_process_that_already_exited_without_killing_it() {
        // Arrange
        let state = Arc::new(Mutex::new(ProcessState {
            running: false,
            ..ProcessState::default()
        }));
        let lease = ChromiumProcessLease {
            child: FakeChild {
                state: Arc::clone(&state),
            },
            process_id: Some(4242),
            user_data_dir: PathBuf::from("/owned/profile"),
        };

        // Act
        let termination = lease
            .terminate()
            .await
            .expect("already-exited process should be observable");

        // Assert
        assert!(!termination.was_running);
        assert_eq!(termination.status.code(), Some(0));
        let state = state.lock().expect("process state should lock");
        assert_eq!(state.kills, 0);
        assert_eq!(state.waits, 0);
    }

    #[test]
    fn shutdown_policy_requires_a_bound_for_every_phase() {
        // Arrange
        let one_second = Duration::from_secs(1);

        // Act
        let close = ChromiumShutdownPolicy::new(Duration::ZERO, one_second, one_second);
        let graceful = ChromiumShutdownPolicy::new(one_second, Duration::ZERO, one_second);
        let forced = ChromiumShutdownPolicy::new(one_second, one_second, Duration::ZERO);

        // Assert
        assert_eq!(
            close,
            Err(ChromiumShutdownPolicyError::ZeroCloseCommandTimeout)
        );
        assert_eq!(
            graceful,
            Err(ChromiumShutdownPolicyError::ZeroGracefulExitTimeout)
        );
        assert_eq!(
            forced,
            Err(ChromiumShutdownPolicyError::ZeroForcedExitTimeout)
        );
    }

    #[tokio::test]
    async fn shutdown_uses_browser_close_before_observing_graceful_exit() {
        // Arrange
        let (shutdown, browser, state, close_timeouts, exit_timeouts, _) = shutdown_fixture(
            CloseBehavior::AcknowledgeAndExit,
            [ExitWaitOutcome::ObserveCurrent],
        );

        // Act
        let report = shutdown
            .shutdown(browser, shutdown_policy())
            .await
            .expect("graceful browser shutdown should succeed");

        // Assert
        assert_eq!(report.method, ChromiumShutdownMethod::Graceful);
        assert_eq!(
            report.close_attempt,
            Some(ChromiumCloseAttempt::Acknowledged)
        );
        assert_eq!(report.process_id, Some(4242));
        assert_eq!(report.status.code(), Some(0));
        assert_eq!(report.user_data_dir, PathBuf::from("/owned/profile"));
        assert_eq!(report.graceful_wait_error, None);
        let state = state.lock().expect("process state should lock");
        assert_eq!(state.kills, 0);
        assert!(!state.running);
        drop(state);
        assert_eq!(
            close_timeouts
                .lock()
                .expect("close timeouts should lock")
                .as_slice(),
            &[Duration::from_secs(1)]
        );
        assert_eq!(
            exit_timeouts
                .lock()
                .expect("exit timeouts should lock")
                .as_slice(),
            &[Duration::from_secs(2)]
        );
    }

    #[tokio::test]
    async fn shutdown_accepts_process_exit_when_close_acknowledgement_is_lost() {
        // Arrange
        let (shutdown, browser, state, _, _, _) = shutdown_fixture(
            CloseBehavior::FailedAndExit,
            [ExitWaitOutcome::ObserveCurrent],
        );

        // Act
        let report = shutdown
            .shutdown(browser, shutdown_policy())
            .await
            .expect("process exit should prove graceful shutdown");

        // Assert
        assert_eq!(report.method, ChromiumShutdownMethod::Graceful);
        assert_eq!(
            report.close_attempt,
            Some(ChromiumCloseAttempt::Failed {
                message: "connection closed before acknowledgement".to_string(),
            })
        );
        assert_eq!(state.lock().expect("process state should lock").kills, 0);
    }

    #[tokio::test]
    async fn shutdown_escalates_after_the_graceful_exit_bound() {
        // Arrange
        let (shutdown, browser, state, _, exit_timeouts, _) = shutdown_fixture(
            CloseBehavior::TimedOut,
            [ExitWaitOutcome::Timeout, ExitWaitOutcome::ObserveCurrent],
        );

        // Act
        let report = shutdown
            .shutdown(browser, shutdown_policy())
            .await
            .expect("forced browser shutdown should succeed");

        // Assert
        assert_eq!(report.method, ChromiumShutdownMethod::Forced);
        assert_eq!(report.close_attempt, Some(ChromiumCloseAttempt::TimedOut));
        assert_eq!(state.lock().expect("process state should lock").kills, 1);
        assert_eq!(
            exit_timeouts
                .lock()
                .expect("exit timeouts should lock")
                .as_slice(),
            &[Duration::from_secs(2), Duration::from_secs(3)]
        );
    }

    #[tokio::test]
    async fn shutdown_records_a_graceful_wait_error_then_still_cleans_up() {
        // Arrange
        let (shutdown, browser, state, _, _, _) = shutdown_fixture(
            CloseBehavior::TimedOut,
            [ExitWaitOutcome::Error, ExitWaitOutcome::ObserveCurrent],
        );

        // Act
        let report = shutdown
            .shutdown(browser, shutdown_policy())
            .await
            .expect("forced cleanup should recover from observation failure");

        // Assert
        assert_eq!(report.method, ChromiumShutdownMethod::Forced);
        assert_eq!(
            report.graceful_wait_error.as_deref(),
            Some("injected process observation failure")
        );
        assert_eq!(state.lock().expect("process state should lock").kills, 1);
    }

    #[tokio::test]
    async fn shutdown_reports_a_forced_exit_timeout_without_orphaning_the_child() {
        // Arrange
        let (shutdown, browser, state, _, _, _) = shutdown_fixture(
            CloseBehavior::TimedOut,
            [ExitWaitOutcome::Timeout, ExitWaitOutcome::Timeout],
        );

        // Act
        let result = shutdown.shutdown(browser, shutdown_policy()).await;

        // Assert
        assert!(matches!(
            result,
            Err(ChromiumShutdownError::ForcedExitTimeout { timeout })
                if timeout == Duration::from_secs(3)
        ));
        let state = state.lock().expect("process state should lock");
        assert_eq!(state.kills, 1);
        assert!(!state.running);
    }

    #[tokio::test]
    async fn cancelling_shutdown_activates_the_owned_process_guard() {
        // Arrange
        let (shutdown, browser, state, _, _, close_started) =
            shutdown_fixture(CloseBehavior::Pending, [ExitWaitOutcome::ObserveCurrent]);
        let task = tokio::spawn(async move { shutdown.shutdown(browser, shutdown_policy()).await });
        while close_started.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }

        // Act
        task.abort();
        let cancellation = task.await;

        // Assert
        assert!(
            cancellation
                .expect_err("task should be cancelled")
                .is_cancelled()
        );
        let state = state.lock().expect("process state should lock");
        assert_eq!(state.kills, 1);
        assert!(!state.running);
    }

    #[test]
    fn dropping_an_unpolled_shutdown_future_does_not_orphan_the_owned_process() {
        // Arrange
        let (shutdown, browser, state, _, _, _) =
            shutdown_fixture(CloseBehavior::Pending, [ExitWaitOutcome::ObserveCurrent]);
        let shutdown_future = shutdown.shutdown(browser, shutdown_policy());

        // Act
        drop(shutdown_future);

        // Assert
        let state = state.lock().expect("process state should lock");
        assert_eq!(state.kills, 1);
        assert!(!state.running);
    }
}
