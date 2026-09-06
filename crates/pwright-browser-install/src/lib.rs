//! Explicit, opt-in managed-browser installation contracts.
//!
//! This crate contains no downloader, HTTP client, archive decoder, async
//! runtime, browser bytes, process launcher, or cache. Applications select and
//! compose each installation stage themselves. Existing Pwright attach and
//! launch providers do not depend on this crate.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::future::Future;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;

const MAX_REVISION_LENGTH: usize = 128;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BrowserEngine {
    Chromium,
    Firefox,
    WebKit,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BrowserRevision(String);

impl BrowserRevision {
    pub fn parse(revision: impl Into<String>) -> Result<Self, BrowserRevisionError> {
        let revision = revision.into();
        if revision.is_empty() {
            return Err(BrowserRevisionError::Empty);
        }
        if revision.len() > MAX_REVISION_LENGTH {
            return Err(BrowserRevisionError::TooLong {
                length: revision.len(),
                maximum: MAX_REVISION_LENGTH,
            });
        }
        if matches!(revision.as_str(), "." | "..") {
            return Err(BrowserRevisionError::Reserved(revision));
        }
        if !revision
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(BrowserRevisionError::InvalidCharacter(revision));
        }
        Ok(Self(revision))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for BrowserRevision {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BrowserRevisionError {
    Empty,
    TooLong { length: usize, maximum: usize },
    Reserved(String),
    InvalidCharacter(String),
}

impl Display for BrowserRevisionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("browser revision must not be empty"),
            Self::TooLong { length, maximum } => write!(
                formatter,
                "browser revision length {length} exceeds maximum {maximum}"
            ),
            Self::Reserved(revision) => {
                write!(formatter, "browser revision `{revision}` is reserved")
            }
            Self::InvalidCharacter(revision) => write!(
                formatter,
                "browser revision `{revision}` contains a character outside [A-Za-z0-9._-]"
            ),
        }
    }
}

impl Error for BrowserRevisionError {}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BrowserOperatingSystem {
    Linux,
    MacOs,
    Windows,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum BrowserArchitecture {
    Aarch64,
    X86_64,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct BrowserTarget {
    operating_system: BrowserOperatingSystem,
    architecture: BrowserArchitecture,
}

impl BrowserTarget {
    pub const fn new(
        operating_system: BrowserOperatingSystem,
        architecture: BrowserArchitecture,
    ) -> Self {
        Self {
            operating_system,
            architecture,
        }
    }

    pub const fn operating_system(self) -> BrowserOperatingSystem {
        self.operating_system
    }

    pub const fn architecture(self) -> BrowserArchitecture {
        self.architecture
    }
}

/// Digest expected from the exact resolved browser archive.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BrowserArtifactIntegrity {
    Sha256([u8; 32]),
    Sha512([u8; 64]),
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct BrowserPackage {
    engine: BrowserEngine,
    revision: BrowserRevision,
    target: BrowserTarget,
    integrity: BrowserArtifactIntegrity,
}

impl BrowserPackage {
    pub const fn new(
        engine: BrowserEngine,
        revision: BrowserRevision,
        target: BrowserTarget,
        integrity: BrowserArtifactIntegrity,
    ) -> Self {
        Self {
            engine,
            revision,
            target,
            integrity,
        }
    }

    pub const fn engine(&self) -> BrowserEngine {
        self.engine
    }

    pub fn revision(&self) -> &BrowserRevision {
        &self.revision
    }

    pub const fn target(&self) -> BrowserTarget {
        self.target
    }

    pub const fn integrity(&self) -> BrowserArtifactIntegrity {
        self.integrity
    }
}

/// Existing canonical empty directory selected for one managed installation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BrowserInstallRoot {
    path: PathBuf,
}

impl BrowserInstallRoot {
    pub fn new(path: impl Into<PathBuf>) -> Result<Self, BrowserInstallRootError> {
        let requested = path.into();
        if !requested.is_absolute() {
            return Err(BrowserInstallRootError::NotAbsolute(requested));
        }
        let metadata =
            std::fs::metadata(&requested).map_err(|source| BrowserInstallRootError::Inspect {
                path: requested.clone(),
                source,
            })?;
        if !metadata.is_dir() {
            return Err(BrowserInstallRootError::NotDirectory(requested));
        }
        let path = std::fs::canonicalize(&requested).map_err(|source| {
            BrowserInstallRootError::Inspect {
                path: requested,
                source,
            }
        })?;
        let root = Self { path };
        root.validate_empty()?;
        Ok(root)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn validate_empty(&self) -> Result<(), BrowserInstallRootError> {
        let mut entries =
            std::fs::read_dir(&self.path).map_err(|source| BrowserInstallRootError::Inspect {
                path: self.path.clone(),
                source,
            })?;
        if entries
            .next()
            .transpose()
            .map_err(|source| BrowserInstallRootError::Inspect {
                path: self.path.clone(),
                source,
            })?
            .is_some()
        {
            return Err(BrowserInstallRootError::NotEmpty(self.path.clone()));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum BrowserInstallRootError {
    NotAbsolute(PathBuf),
    Inspect { path: PathBuf, source: io::Error },
    NotDirectory(PathBuf),
    NotEmpty(PathBuf),
}

impl Display for BrowserInstallRootError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAbsolute(path) => write!(
                formatter,
                "browser installation root is not absolute: {}",
                path.display()
            ),
            Self::Inspect { path, source } => write!(
                formatter,
                "could not inspect browser installation root {}: {source}",
                path.display()
            ),
            Self::NotDirectory(path) => write!(
                formatter,
                "browser installation root is not a directory: {}",
                path.display()
            ),
            Self::NotEmpty(path) => write!(
                formatter,
                "browser installation root must be empty: {}",
                path.display()
            ),
        }
    }
}

impl Error for BrowserInstallRootError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Inspect { source, .. } => Some(source),
            Self::NotAbsolute(_) | Self::NotDirectory(_) | Self::NotEmpty(_) => None,
        }
    }
}

/// A non-default, non-cloneable token authorizing one fetch stage invocation.
#[derive(Debug)]
#[must_use = "download permission has no effect unless consumed by an install request"]
pub struct BrowserDownloadPermission {
    _private: (),
}

impl BrowserDownloadPermission {
    /// Explicitly authorizes one managed browser artifact download.
    pub const fn allow() -> Self {
        Self { _private: () }
    }
}

/// Complete request for one exact managed browser package.
///
/// There is deliberately no two-argument constructor or `Default`; callers
/// must opt into the installation crate and create a download permission.
///
/// ```compile_fail
/// use pwright_browser_install::{BrowserInstallRequest, BrowserInstallRoot, BrowserPackage};
///
/// fn request_without_permission(package: BrowserPackage, root: BrowserInstallRoot) {
///     let _ = BrowserInstallRequest::new(package, root);
/// }
/// ```
#[derive(Debug)]
#[must_use = "browser installation requires explicitly invoking an installer"]
pub struct BrowserInstallRequest {
    package: BrowserPackage,
    destination: BrowserInstallRoot,
    permission: BrowserDownloadPermission,
}

impl BrowserInstallRequest {
    pub const fn new(
        package: BrowserPackage,
        destination: BrowserInstallRoot,
        permission: BrowserDownloadPermission,
    ) -> Self {
        Self {
            package,
            destination,
            permission,
        }
    }

    pub const fn package(&self) -> &BrowserPackage {
        &self.package
    }

    pub const fn destination(&self) -> &BrowserInstallRoot {
        &self.destination
    }
}

pub trait BrowserArtifactResolver: Send + Sync {
    type Artifact: Send;
    type Error: Send;
    type Future<'a>: Future<Output = Result<Self::Artifact, Self::Error>> + Send
    where
        Self: 'a;

    fn resolve<'a>(&'a self, package: &'a BrowserPackage) -> Self::Future<'a>;
}

pub trait BrowserArtifactFetcher<Artifact>: Send + Sync {
    type Fetched: Send;
    type Error: Send;
    type Future<'a>: Future<Output = Result<Self::Fetched, Self::Error>> + Send
    where
        Self: 'a,
        Artifact: 'a;

    fn fetch<'a>(
        &'a self,
        artifact: Artifact,
        destination: &'a BrowserInstallRoot,
        permission: BrowserDownloadPermission,
    ) -> Self::Future<'a>;
}

pub trait BrowserArtifactVerifier<Fetched>: Send + Sync {
    type Verified: Send;
    type Error: Send;
    type Future<'a>: Future<Output = Result<Self::Verified, Self::Error>> + Send
    where
        Self: 'a,
        Fetched: 'a;

    fn verify<'a>(
        &'a self,
        fetched: Fetched,
        expected: BrowserArtifactIntegrity,
    ) -> Self::Future<'a>;
}

pub trait BrowserArtifactInstaller<Verified>: Send + Sync {
    type Installation: Send;
    type Error: Send;
    type Future<'a>: Future<Output = Result<Self::Installation, Self::Error>> + Send
    where
        Self: 'a,
        Verified: 'a;

    fn install<'a>(
        &'a self,
        verified: Verified,
        package: BrowserPackage,
        destination: BrowserInstallRoot,
    ) -> Self::Future<'a>;
}

pub trait BrowserInstaller<Request>: Send + Sync {
    type Installation;
    type Error;
    type Future<'a>: Future<Output = Result<Self::Installation, Self::Error>> + Send
    where
        Self: 'a;

    fn install(&self, request: Request) -> Self::Future<'_>;
}

#[derive(Clone, Debug)]
pub struct BrowserInstallPipeline<Resolver, Fetcher, Verifier, Installer> {
    resolver: Resolver,
    fetcher: Fetcher,
    verifier: Verifier,
    installer: Installer,
}

impl<Resolver, Fetcher, Verifier, Installer>
    BrowserInstallPipeline<Resolver, Fetcher, Verifier, Installer>
{
    pub const fn new(
        resolver: Resolver,
        fetcher: Fetcher,
        verifier: Verifier,
        installer: Installer,
    ) -> Self {
        Self {
            resolver,
            fetcher,
            verifier,
            installer,
        }
    }
}

#[derive(Debug)]
pub enum BrowserInstallPipelineError<Resolve, Fetch, Verify, Install> {
    Request(BrowserInstallRootError),
    Resolve(Resolve),
    Fetch(Fetch),
    Verify(Verify),
    Install(Install),
}

impl<Resolve, Fetch, Verify, Install> Display
    for BrowserInstallPipelineError<Resolve, Fetch, Verify, Install>
where
    Resolve: Display,
    Fetch: Display,
    Verify: Display,
    Install: Display,
{
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Request(error) => Display::fmt(error, formatter),
            Self::Resolve(error) => {
                write!(formatter, "browser artifact resolution failed: {error}")
            }
            Self::Fetch(error) => write!(formatter, "browser artifact fetch failed: {error}"),
            Self::Verify(error) => {
                write!(formatter, "browser artifact verification failed: {error}")
            }
            Self::Install(error) => {
                write!(formatter, "browser artifact installation failed: {error}")
            }
        }
    }
}

impl<Resolve, Fetch, Verify, Install> Error
    for BrowserInstallPipelineError<Resolve, Fetch, Verify, Install>
where
    Resolve: Error + 'static,
    Fetch: Error + 'static,
    Verify: Error + 'static,
    Install: Error + 'static,
{
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Request(error) => Some(error),
            Self::Resolve(error) => Some(error),
            Self::Fetch(error) => Some(error),
            Self::Verify(error) => Some(error),
            Self::Install(error) => Some(error),
        }
    }
}

impl<Resolver, Fetcher, Verifier, Installer> BrowserInstaller<BrowserInstallRequest>
    for BrowserInstallPipeline<Resolver, Fetcher, Verifier, Installer>
where
    Resolver: BrowserArtifactResolver,
    Fetcher: BrowserArtifactFetcher<Resolver::Artifact>,
    Verifier: BrowserArtifactVerifier<Fetcher::Fetched>,
    Installer: BrowserArtifactInstaller<Verifier::Verified>,
{
    type Installation = Installer::Installation;
    type Error = BrowserInstallPipelineError<
        Resolver::Error,
        Fetcher::Error,
        Verifier::Error,
        Installer::Error,
    >;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Installation, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn install(&self, request: BrowserInstallRequest) -> Self::Future<'_> {
        Box::pin(async move {
            request
                .destination
                .validate_empty()
                .map_err(BrowserInstallPipelineError::Request)?;
            let BrowserInstallRequest {
                package,
                destination,
                permission,
            } = request;
            let artifact = self
                .resolver
                .resolve(&package)
                .await
                .map_err(BrowserInstallPipelineError::Resolve)?;
            let fetched = self
                .fetcher
                .fetch(artifact, &destination, permission)
                .await
                .map_err(BrowserInstallPipelineError::Fetch)?;
            let verified = self
                .verifier
                .verify(fetched, package.integrity())
                .await
                .map_err(BrowserInstallPipelineError::Verify)?;
            self.installer
                .install(verified, package, destination)
                .await
                .map_err(BrowserInstallPipelineError::Install)
        })
    }
}

/// Verified installation receipt containing no process-control authority.
///
/// ```compile_fail
/// use pwright_browser_install::InstalledBrowser;
///
/// fn cannot_terminate_an_installation(browser: InstalledBrowser) {
///     browser.terminate();
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use = "an installed browser receipt identifies the verified executable"]
pub struct InstalledBrowser {
    package: BrowserPackage,
    install_root: PathBuf,
    executable: PathBuf,
}

impl InstalledBrowser {
    pub fn from_verified_installation(
        package: BrowserPackage,
        install_root: &BrowserInstallRoot,
        executable: impl Into<PathBuf>,
    ) -> Result<Self, InstalledBrowserError> {
        let requested = executable.into();
        if !requested.is_absolute() {
            return Err(InstalledBrowserError::ExecutableNotAbsolute(requested));
        }
        let executable = std::fs::canonicalize(&requested).map_err(|source| {
            InstalledBrowserError::InspectExecutable {
                path: requested,
                source,
            }
        })?;
        if !executable.starts_with(install_root.path()) {
            return Err(InstalledBrowserError::ExecutableOutsideInstallRoot {
                executable,
                install_root: install_root.path().to_path_buf(),
            });
        }
        let metadata = std::fs::metadata(&executable).map_err(|source| {
            InstalledBrowserError::InspectExecutable {
                path: executable.clone(),
                source,
            }
        })?;
        if !metadata.is_file() {
            return Err(InstalledBrowserError::ExecutableNotFile(executable));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            if metadata.permissions().mode() & 0o111 == 0 {
                return Err(InstalledBrowserError::ExecutableNotRunnable(executable));
            }
        }
        Ok(Self {
            package,
            install_root: install_root.path().to_path_buf(),
            executable,
        })
    }

    pub const fn package(&self) -> &BrowserPackage {
        &self.package
    }

    pub fn install_root(&self) -> &Path {
        &self.install_root
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }
}

#[derive(Debug)]
pub enum InstalledBrowserError {
    ExecutableNotAbsolute(PathBuf),
    InspectExecutable {
        path: PathBuf,
        source: io::Error,
    },
    ExecutableOutsideInstallRoot {
        executable: PathBuf,
        install_root: PathBuf,
    },
    ExecutableNotFile(PathBuf),
    ExecutableNotRunnable(PathBuf),
}

impl Display for InstalledBrowserError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExecutableNotAbsolute(path) => write!(
                formatter,
                "installed browser executable is not absolute: {}",
                path.display()
            ),
            Self::InspectExecutable { path, source } => write!(
                formatter,
                "could not inspect installed browser executable {}: {source}",
                path.display()
            ),
            Self::ExecutableOutsideInstallRoot {
                executable,
                install_root,
            } => write!(
                formatter,
                "installed browser executable {} is outside installation root {}",
                executable.display(),
                install_root.display()
            ),
            Self::ExecutableNotFile(path) => write!(
                formatter,
                "installed browser executable is not a regular file: {}",
                path.display()
            ),
            Self::ExecutableNotRunnable(path) => write!(
                formatter,
                "installed browser file does not have an executable permission bit: {}",
                path.display()
            ),
        }
    }
}

impl Error for InstalledBrowserError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InspectExecutable { source, .. } => Some(source),
            Self::ExecutableNotAbsolute(_)
            | Self::ExecutableOutsideInstallRoot { .. }
            | Self::ExecutableNotFile(_)
            | Self::ExecutableNotRunnable(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::fmt::{self, Display, Formatter};
    use std::fs;
    use std::future::{Ready, ready};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use super::{
        BrowserArchitecture, BrowserArtifactFetcher, BrowserArtifactInstaller,
        BrowserArtifactIntegrity, BrowserArtifactResolver, BrowserArtifactVerifier,
        BrowserDownloadPermission, BrowserEngine, BrowserInstallPipeline,
        BrowserInstallPipelineError, BrowserInstallRequest, BrowserInstallRoot,
        BrowserInstallRootError, BrowserInstaller, BrowserOperatingSystem, BrowserPackage,
        BrowserRevision, BrowserRevisionError, BrowserTarget, InstalledBrowser,
        InstalledBrowserError,
    };

    static TEST_DIRECTORY_ID: AtomicUsize = AtomicUsize::new(0);

    struct TestDirectory {
        path: PathBuf,
    }

    impl TestDirectory {
        fn new(label: &str) -> Self {
            let id = TEST_DIRECTORY_ID.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "pwright-browser-install-{label}-{}-{id}",
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
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }

    fn package() -> BrowserPackage {
        BrowserPackage::new(
            BrowserEngine::Chromium,
            BrowserRevision::parse("1234.5").expect("revision should validate"),
            BrowserTarget::new(BrowserOperatingSystem::MacOs, BrowserArchitecture::Aarch64),
            BrowserArtifactIntegrity::Sha256([7; 32]),
        )
    }

    #[test]
    fn revision_and_install_root_reject_ambiguous_or_destructive_inputs() {
        // Arrange
        let tree = TestDirectory::new("validation");
        let nonempty = tree.directory("nonempty");
        fs::write(nonempty.join("keep"), b"user data").expect("test data should be written");

        // Act
        let empty_revision = BrowserRevision::parse("");
        let traversal_revision = BrowserRevision::parse("../chrome");
        let relative_root = BrowserInstallRoot::new("browser-cache");
        let nonempty_root = BrowserInstallRoot::new(nonempty);

        // Assert
        assert_eq!(empty_revision, Err(BrowserRevisionError::Empty));
        assert!(matches!(
            traversal_revision,
            Err(BrowserRevisionError::InvalidCharacter(_))
        ));
        assert!(matches!(
            relative_root,
            Err(BrowserInstallRootError::NotAbsolute(_))
        ));
        assert!(matches!(
            nonempty_root,
            Err(BrowserInstallRootError::NotEmpty(_))
        ));
    }

    #[derive(Clone, Debug)]
    struct StageRecorder {
        stages: Arc<Mutex<Vec<&'static str>>>,
    }

    impl BrowserArtifactResolver for StageRecorder {
        type Artifact = &'static str;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Artifact, Self::Error>>;

        fn resolve<'a>(&'a self, _package: &'a BrowserPackage) -> Self::Future<'a> {
            self.stages
                .lock()
                .expect("stages should lock")
                .push("resolve");
            ready(Ok("resolved artifact"))
        }
    }

    impl BrowserArtifactFetcher<&'static str> for StageRecorder {
        type Fetched = &'static str;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Fetched, Self::Error>>;

        fn fetch<'a>(
            &'a self,
            _artifact: &'static str,
            destination: &'a BrowserInstallRoot,
            _permission: BrowserDownloadPermission,
        ) -> Self::Future<'a> {
            self.stages
                .lock()
                .expect("stages should lock")
                .push("fetch");
            fs::write(destination.path().join("browser.archive"), b"archive")
                .expect("fake artifact should be written");
            ready(Ok("fetched artifact"))
        }
    }

    impl BrowserArtifactVerifier<&'static str> for StageRecorder {
        type Verified = &'static str;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Verified, Self::Error>>;

        fn verify<'a>(
            &'a self,
            _fetched: &'static str,
            expected: BrowserArtifactIntegrity,
        ) -> Self::Future<'a> {
            assert_eq!(expected, BrowserArtifactIntegrity::Sha256([7; 32]));
            self.stages
                .lock()
                .expect("stages should lock")
                .push("verify");
            ready(Ok("verified artifact"))
        }
    }

    impl BrowserArtifactInstaller<&'static str> for StageRecorder {
        type Installation = InstalledBrowser;
        type Error = InstalledBrowserError;
        type Future<'a> = Ready<Result<Self::Installation, Self::Error>>;

        fn install<'a>(
            &'a self,
            _verified: &'static str,
            package: BrowserPackage,
            destination: BrowserInstallRoot,
        ) -> Self::Future<'a> {
            self.stages
                .lock()
                .expect("stages should lock")
                .push("install");
            let executable = destination.path().join("chromium");
            fs::write(&executable, b"test executable").expect("fake executable should be written");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;

                fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
                    .expect("fake executable permissions should be set");
            }
            ready(InstalledBrowser::from_verified_installation(
                package,
                &destination,
                executable,
            ))
        }
    }

    #[test]
    fn explicit_pipeline_runs_resolve_fetch_verify_install_in_order() {
        // Arrange
        let tree = TestDirectory::new("pipeline");
        let root =
            BrowserInstallRoot::new(tree.directory("install")).expect("empty root should validate");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let recorder = StageRecorder {
            stages: Arc::clone(&stages),
        };
        let pipeline = BrowserInstallPipeline::new(
            recorder.clone(),
            recorder.clone(),
            recorder.clone(),
            recorder,
        );
        let request =
            BrowserInstallRequest::new(package(), root, BrowserDownloadPermission::allow());

        // Act
        let installation = futures_executor(pipeline.install(request))
            .expect("explicit installation pipeline should succeed");

        // Assert
        assert_eq!(
            stages.lock().expect("stages should lock").as_slice(),
            &["resolve", "fetch", "verify", "install"]
        );
        assert_eq!(installation.package().revision().as_str(), "1234.5");
        assert!(
            installation
                .executable()
                .starts_with(installation.install_root())
        );
    }

    #[test]
    fn pipeline_revalidates_the_empty_root_before_resolution() {
        // Arrange
        let tree = TestDirectory::new("root-race");
        let root =
            BrowserInstallRoot::new(tree.directory("install")).expect("empty root should validate");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let recorder = StageRecorder {
            stages: Arc::clone(&stages),
        };
        let pipeline = BrowserInstallPipeline::new(
            recorder.clone(),
            recorder.clone(),
            recorder.clone(),
            recorder,
        );
        let request =
            BrowserInstallRequest::new(package(), root, BrowserDownloadPermission::allow());
        fs::write(
            tree.path().join("install/foreign-file"),
            b"do not overwrite",
        )
        .expect("foreign file should be written");

        // Act
        let result = futures_executor(pipeline.install(request));

        // Assert
        assert!(matches!(
            result,
            Err(BrowserInstallPipelineError::Request(
                BrowserInstallRootError::NotEmpty(_)
            ))
        ));
        assert!(stages.lock().expect("stages should lock").is_empty());
    }

    #[derive(Clone, Copy, Debug)]
    struct StageFailure;

    impl Display for StageFailure {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
            formatter.write_str("injected stage failure")
        }
    }

    impl std::error::Error for StageFailure {}

    #[derive(Clone, Debug)]
    struct FailingResolver {
        stages: Arc<Mutex<Vec<&'static str>>>,
    }

    impl BrowserArtifactResolver for FailingResolver {
        type Artifact = &'static str;
        type Error = StageFailure;
        type Future<'a> = Ready<Result<Self::Artifact, Self::Error>>;

        fn resolve<'a>(&'a self, _package: &'a BrowserPackage) -> Self::Future<'a> {
            self.stages
                .lock()
                .expect("stages should lock")
                .push("resolve");
            ready(Err(StageFailure))
        }
    }

    #[derive(Clone, Debug)]
    struct FailingVerifier {
        stages: Arc<Mutex<Vec<&'static str>>>,
    }

    impl BrowserArtifactVerifier<&'static str> for FailingVerifier {
        type Verified = &'static str;
        type Error = StageFailure;
        type Future<'a> = Ready<Result<Self::Verified, Self::Error>>;

        fn verify<'a>(
            &'a self,
            _fetched: &'static str,
            _expected: BrowserArtifactIntegrity,
        ) -> Self::Future<'a> {
            self.stages
                .lock()
                .expect("stages should lock")
                .push("verify");
            ready(Err(StageFailure))
        }
    }

    #[test]
    fn pipeline_stops_before_fetch_when_resolution_fails() {
        // Arrange
        let tree = TestDirectory::new("resolution-failure");
        let root =
            BrowserInstallRoot::new(tree.directory("install")).expect("empty root should validate");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let recorder = StageRecorder {
            stages: Arc::clone(&stages),
        };
        let pipeline = BrowserInstallPipeline::new(
            FailingResolver {
                stages: Arc::clone(&stages),
            },
            recorder.clone(),
            recorder.clone(),
            recorder,
        );
        let request =
            BrowserInstallRequest::new(package(), root, BrowserDownloadPermission::allow());

        // Act
        let result = futures_executor(pipeline.install(request));

        // Assert
        assert!(matches!(
            result,
            Err(BrowserInstallPipelineError::Resolve(StageFailure))
        ));
        assert_eq!(
            stages.lock().expect("stages should lock").as_slice(),
            &["resolve"]
        );
    }

    #[test]
    fn pipeline_never_installs_an_artifact_that_failed_integrity_verification() {
        // Arrange
        let tree = TestDirectory::new("verification-failure");
        let root =
            BrowserInstallRoot::new(tree.directory("install")).expect("empty root should validate");
        let stages = Arc::new(Mutex::new(Vec::new()));
        let recorder = StageRecorder {
            stages: Arc::clone(&stages),
        };
        let pipeline = BrowserInstallPipeline::new(
            recorder.clone(),
            recorder.clone(),
            FailingVerifier {
                stages: Arc::clone(&stages),
            },
            recorder,
        );
        let request =
            BrowserInstallRequest::new(package(), root, BrowserDownloadPermission::allow());

        // Act
        let result = futures_executor(pipeline.install(request));

        // Assert
        assert!(matches!(
            result,
            Err(BrowserInstallPipelineError::Verify(StageFailure))
        ));
        assert_eq!(
            stages.lock().expect("stages should lock").as_slice(),
            &["resolve", "fetch", "verify"]
        );
        assert!(!tree.path().join("install/chromium").exists());
    }

    #[test]
    fn installed_browser_rejects_an_executable_outside_its_install_root() {
        // Arrange
        let tree = TestDirectory::new("outside-executable");
        let root =
            BrowserInstallRoot::new(tree.directory("install")).expect("empty root should validate");
        let outside = tree.path().join("outside-browser");
        fs::write(&outside, b"test executable").expect("outside executable should be written");

        // Act
        let result = InstalledBrowser::from_verified_installation(package(), &root, outside);

        // Assert
        assert!(matches!(
            result,
            Err(InstalledBrowserError::ExecutableOutsideInstallRoot { .. })
        ));
    }

    fn futures_executor<F: Future>(future: F) -> F::Output {
        use std::task::{Context, Poll, Waker};

        let mut context = Context::from_waker(Waker::noop());
        let mut future = std::pin::pin!(future);
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("test futures must complete immediately"),
        }
    }
}
