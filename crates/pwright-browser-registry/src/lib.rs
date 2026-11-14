//! Read-only Playwright v1.58.0 browser executable metadata.
//!
//! This leaf does not inspect, install, download, or launch browser binaries.
//! A returned path is not proof that its executable exists. The pinned inputs
//! are the official `browsers.json` SHA-256
//! `6b6e0538d6e46eef71d3e1a0dcb65f8f5ea7914033c8150529d8e207afb6030c`
//! and `lib/server/registry/index.js` SHA-256
//! `15c1a6174cc9d631ffcc908ed8d040f3f5ee543a77711a8062fa3037f7cda75f`.

use std::fmt::{self, Display, Formatter};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserEngine {
    Chromium,
    Firefox,
    WebKit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShortPlatform {
    LinuxX64,
    LinuxArm64,
    MacX64,
    MacArm64,
    WindowsX64,
}

impl ShortPlatform {
    pub const fn is_linux(self) -> bool {
        matches!(self, Self::LinuxX64 | Self::LinuxArm64)
    }

    pub const fn is_mac(self) -> bool {
        matches!(self, Self::MacX64 | Self::MacArm64)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RegistryError {
    RelativeBase(&'static str),
    MissingHome,
}

impl Display for RegistryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::RelativeBase(name) => write!(formatter, "{name} must be an absolute path"),
            Self::MissingHome => formatter.write_str("browser registry requires a home directory"),
        }
    }
}

impl std::error::Error for RegistryError {}

pub struct RegistryDirectoryInputs<'a> {
    pub platform: ShortPlatform,
    pub browsers_path: Option<&'a Path>,
    pub package_root: &'a Path,
    pub home: Option<&'a Path>,
    pub xdg_cache_home: Option<&'a Path>,
    pub local_app_data: Option<&'a Path>,
    pub init_cwd: Option<&'a Path>,
    pub cwd: &'a Path,
}

/// Match Playwright's `PLAYWRIGHT_BROWSERS_PATH`, `INIT_CWD`, and OS cache policy.
pub fn registry_directory(inputs: &RegistryDirectoryInputs<'_>) -> Result<PathBuf, RegistryError> {
    require_absolute(inputs.package_root, "package root")?;
    require_absolute(inputs.cwd, "current directory")?;
    let requested = match inputs.browsers_path {
        Some(path) if path == Path::new("0") => inputs.package_root.join(".local-browsers"),
        Some(path) if !path.as_os_str().is_empty() => path.to_path_buf(),
        _ => {
            let cache_root = match inputs.platform {
                ShortPlatform::LinuxX64 | ShortPlatform::LinuxArm64 => inputs
                    .xdg_cache_home
                    .filter(|path| !path.as_os_str().is_empty())
                    .map(Path::to_path_buf)
                    .or_else(|| inputs.home.map(|path| path.join(".cache")))
                    .ok_or(RegistryError::MissingHome)?,
                ShortPlatform::MacX64 | ShortPlatform::MacArm64 => inputs
                    .home
                    .ok_or(RegistryError::MissingHome)?
                    .join("Library/Caches"),
                ShortPlatform::WindowsX64 => inputs
                    .local_app_data
                    .filter(|path| !path.as_os_str().is_empty())
                    .map(Path::to_path_buf)
                    .or_else(|| inputs.home.map(|path| path.join("AppData/Local")))
                    .ok_or(RegistryError::MissingHome)?,
            };
            cache_root.join("ms-playwright")
        }
    };
    let absolute = if requested.is_absolute() {
        requested
    } else {
        // Node's path.resolve accepts a relative INIT_CWD and resolves it
        // against the cwd captured when the registry module was imported.
        let base = inputs
            .init_cwd
            .filter(|path| !path.as_os_str().is_empty())
            .map_or_else(
                || inputs.cwd.to_path_buf(),
                |path| {
                    if path.is_absolute() {
                        path.to_path_buf()
                    } else {
                        inputs.cwd.join(path)
                    }
                },
            );
        base.join(requested)
    };
    Ok(clean_absolute(&absolute))
}

/// Return the canonical expected path even if no matching browser is installed.
pub fn executable_path_v1_58_0(
    registry_directory: &Path,
    engine: BrowserEngine,
    platform: ShortPlatform,
    host_platform: &str,
) -> Result<PathBuf, RegistryError> {
    require_absolute(registry_directory, "registry directory")?;
    let (directory, executable): (&str, &[&str]) = match (engine, platform) {
        (BrowserEngine::Chromium, ShortPlatform::MacArm64) => (
            "chromium-1208",
            &[
                "chrome-mac-arm64",
                "Google Chrome for Testing.app",
                "Contents",
                "MacOS",
                "Google Chrome for Testing",
            ],
        ),
        (BrowserEngine::Chromium, ShortPlatform::MacX64) => (
            "chromium-1208",
            &[
                "chrome-mac-x64",
                "Google Chrome for Testing.app",
                "Contents",
                "MacOS",
                "Google Chrome for Testing",
            ],
        ),
        (BrowserEngine::Chromium, ShortPlatform::LinuxX64) => {
            ("chromium-1208", &["chrome-linux64", "chrome"])
        }
        (BrowserEngine::Chromium, ShortPlatform::LinuxArm64) => {
            ("chromium-1208", &["chrome-linux", "chrome"])
        }
        (BrowserEngine::Chromium, ShortPlatform::WindowsX64) => {
            ("chromium-1208", &["chrome-win64", "chrome.exe"])
        }
        (BrowserEngine::Firefox, ShortPlatform::MacX64 | ShortPlatform::MacArm64) => (
            "firefox-1509",
            &["firefox", "Nightly.app", "Contents", "MacOS", "firefox"],
        ),
        (BrowserEngine::Firefox, ShortPlatform::LinuxX64 | ShortPlatform::LinuxArm64) => {
            ("firefox-1509", &["firefox", "firefox"])
        }
        (BrowserEngine::Firefox, ShortPlatform::WindowsX64) => {
            ("firefox-1509", &["firefox", "firefox.exe"])
        }
        (BrowserEngine::WebKit, ShortPlatform::WindowsX64) => ("webkit-2248", &["Playwright.exe"]),
        (BrowserEngine::WebKit, _) => ("webkit-2248", &["pw_run.sh"]),
    };
    let directory = if engine == BrowserEngine::WebKit {
        webkit_directory(host_platform)
    } else {
        directory.to_owned()
    };
    let mut path = registry_directory.join(directory);
    for segment in executable {
        path.push(segment);
    }
    Ok(path)
}

fn webkit_directory(host_platform: &str) -> String {
    match host_platform {
        "debian11-x64" | "debian11-arm64" => {
            format!("webkit_{}_special-2105", host_platform.replace('-', "_"))
        }
        "ubuntu20.04-x64" | "ubuntu20.04-arm64" => {
            format!("webkit_{}_special-2092", host_platform.replace('-', "_"))
        }
        _ => "webkit-2248".to_owned(),
    }
}

fn require_absolute(path: &Path, name: &'static str) -> Result<(), RegistryError> {
    if path.is_absolute() {
        Ok(())
    } else {
        Err(RegistryError::RelativeBase(name))
    }
}

fn clean_absolute(path: &Path) -> PathBuf {
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                clean.pop();
            }
            Component::CurDir => {}
            _ => clean.push(component.as_os_str()),
        }
    }
    clean
}

#[cfg(test)]
mod tests {
    use super::{
        BrowserEngine, RegistryDirectoryInputs, RegistryError, ShortPlatform,
        executable_path_v1_58_0, registry_directory,
    };
    use std::path::Path;

    fn mac_inputs<'a>(browsers_path: Option<&'a Path>) -> RegistryDirectoryInputs<'a> {
        RegistryDirectoryInputs {
            platform: ShortPlatform::MacArm64,
            browsers_path,
            package_root: Path::new("/app/pwright"),
            home: Some(Path::new("/Users/alice")),
            xdg_cache_home: None,
            local_app_data: None,
            init_cwd: None,
            cwd: Path::new("/workspace"),
        }
    }

    #[test]
    fn chromium_metadata_matches_official_mac_arm64_without_installing_browser() {
        // Arrange
        let inputs = mac_inputs(None);

        // Act
        let directory = registry_directory(&inputs).unwrap();
        let executable = executable_path_v1_58_0(
            &directory,
            BrowserEngine::Chromium,
            inputs.platform,
            "mac15-arm64",
        )
        .unwrap();

        // Assert
        assert_eq!(
            directory,
            Path::new("/Users/alice/Library/Caches/ms-playwright")
        );
        assert_eq!(
            executable,
            Path::new(
                "/Users/alice/Library/Caches/ms-playwright/chromium-1208/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing"
            )
        );
    }

    #[test]
    fn firefox_and_webkit_metadata_match_official_mac_arm64() {
        // Arrange
        let directory = Path::new("/Users/alice/Library/Caches/ms-playwright");

        // Act
        let firefox = executable_path_v1_58_0(
            directory,
            BrowserEngine::Firefox,
            ShortPlatform::MacArm64,
            "mac15-arm64",
        )
        .unwrap();
        let webkit = executable_path_v1_58_0(
            directory,
            BrowserEngine::WebKit,
            ShortPlatform::MacArm64,
            "mac15-arm64",
        )
        .unwrap();

        // Assert
        assert_eq!(
            firefox,
            directory.join("firefox-1509/firefox/Nightly.app/Contents/MacOS/firefox")
        );
        assert_eq!(webkit, directory.join("webkit-2248/pw_run.sh"));
    }

    #[test]
    fn relative_override_uses_init_cwd_without_filesystem_access() {
        // Arrange
        let mut inputs = mac_inputs(Some(Path::new("browser-cache/../registry")));
        inputs.init_cwd = Some(Path::new("/task"));

        // Act
        let directory = registry_directory(&inputs).unwrap();

        // Assert
        assert_eq!(directory, Path::new("/task/registry"));
    }

    #[test]
    fn relative_init_cwd_is_resolved_against_import_cwd() {
        // Arrange
        let mut inputs = mac_inputs(Some(Path::new("browser-cache")));
        inputs.init_cwd = Some(Path::new("task/../install"));

        // Act
        let directory = registry_directory(&inputs).unwrap();

        // Assert
        assert_eq!(directory, Path::new("/workspace/install/browser-cache"));
    }

    #[test]
    fn absolute_override_does_not_consult_relative_init_cwd() {
        // Arrange
        let mut inputs = mac_inputs(Some(Path::new("/registry")));
        inputs.init_cwd = Some(Path::new("relative-install"));

        // Act
        let directory = registry_directory(&inputs).unwrap();

        // Assert
        assert_eq!(directory, Path::new("/registry"));
    }

    #[test]
    fn absolute_override_clamps_parent_segments_at_root() {
        // Arrange
        let inputs = mac_inputs(Some(Path::new("/../../tmp")));

        // Act
        let directory = registry_directory(&inputs).unwrap();

        // Assert
        assert_eq!(directory, Path::new("/tmp"));
    }

    #[test]
    fn package_local_override_is_explicit_and_does_not_create_it() {
        // Arrange
        let inputs = mac_inputs(Some(Path::new("0")));

        // Act
        let directory = registry_directory(&inputs).unwrap();

        // Assert
        assert_eq!(directory, Path::new("/app/pwright/.local-browsers"));
    }

    #[test]
    fn linux_cache_override_and_old_webkit_revision_are_exact() {
        // Arrange
        let mut inputs = mac_inputs(None);
        inputs.platform = ShortPlatform::LinuxArm64;
        inputs.xdg_cache_home = Some(Path::new("/cache"));

        // Act
        let directory = registry_directory(&inputs).unwrap();
        let webkit = executable_path_v1_58_0(
            &directory,
            BrowserEngine::WebKit,
            inputs.platform,
            "debian11-arm64",
        )
        .unwrap();

        // Assert
        assert_eq!(directory, Path::new("/cache/ms-playwright"));
        assert_eq!(
            webkit,
            Path::new("/cache/ms-playwright/webkit_debian11_arm64_special-2105/pw_run.sh")
        );
    }

    #[test]
    fn relative_registry_root_is_rejected() {
        // Arrange
        let root = Path::new("relative-cache");

        // Act
        let result = executable_path_v1_58_0(
            root,
            BrowserEngine::Chromium,
            ShortPlatform::MacArm64,
            "mac15-arm64",
        );

        // Assert
        assert_eq!(
            result,
            Err(RegistryError::RelativeBase("registry directory"))
        );
    }
}
