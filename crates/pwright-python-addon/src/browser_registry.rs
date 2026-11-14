//! Python argument and environment adaptation for the opt-in Rust registry leaf.

use pyo3::exceptions::PyNotImplementedError;
use pyo3::prelude::*;

#[cfg(feature = "browser-registry")]
use pwright_browser_registry::{
    BrowserEngine, RegistryDirectoryInputs, ShortPlatform, executable_path_v1_58_0,
    registry_directory,
};
#[cfg(feature = "browser-registry")]
use pyo3::exceptions::{PyRuntimeError, PyValueError};
#[cfg(feature = "browser-registry")]
use std::path::{Path, PathBuf};

/// Return official v1.58.0 metadata, not proof that a browser is installed.
#[pyfunction]
pub fn browser_executable_path(engine: &str, package_root: &str) -> PyResult<String> {
    #[cfg(feature = "browser-registry")]
    {
        let engine = match engine {
            "chromium" => BrowserEngine::Chromium,
            "firefox" => BrowserEngine::Firefox,
            "webkit" => BrowserEngine::WebKit,
            _ => return Err(PyValueError::new_err("unknown browser engine")),
        };
        let cache_platform = current_platform()?;
        let browsers_path = std::env::var_os("PLAYWRIGHT_BROWSERS_PATH").map(PathBuf::from);
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from);
        let xdg_cache_home = std::env::var_os("XDG_CACHE_HOME").map(PathBuf::from);
        let local_app_data = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
        let init_cwd = std::env::var_os("INIT_CWD").map(PathBuf::from);
        let cwd = std::env::current_dir().map_err(|error| {
            PyRuntimeError::new_err(format!("could not inspect current directory: {error}"))
        })?;
        let inputs = RegistryDirectoryInputs {
            platform: cache_platform,
            browsers_path: browsers_path.as_deref(),
            package_root: Path::new(package_root),
            home: home.as_deref(),
            xdg_cache_home: xdg_cache_home.as_deref(),
            local_app_data: local_app_data.as_deref(),
            init_cwd: init_cwd.as_deref(),
            cwd: &cwd,
        };
        let directory = registry_directory(&inputs)
            .map_err(|error| PyValueError::new_err(error.to_string()))?;
        let host_platform = host_platform(cache_platform);
        let executable = executable_path_v1_58_0(
            &directory,
            engine,
            short_platform(&host_platform)?,
            &host_platform,
        )
        .map_err(|error| PyValueError::new_err(error.to_string()))?;
        executable
            .into_os_string()
            .into_string()
            .map_err(|_| PyValueError::new_err("browser executable path is not UTF-8"))
    }
    #[cfg(not(feature = "browser-registry"))]
    {
        let _ = (engine, package_root);
        Err(PyNotImplementedError::new_err(
            "browser executable metadata requires the browser-registry feature",
        ))
    }
}

#[cfg(feature = "browser-registry")]
fn current_platform() -> PyResult<ShortPlatform> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok(ShortPlatform::MacArm64),
        ("macos", "x86_64") => Ok(ShortPlatform::MacX64),
        ("linux", "aarch64") => Ok(ShortPlatform::LinuxArm64),
        ("linux", "x86_64") => Ok(ShortPlatform::LinuxX64),
        ("windows", "x86_64") => Ok(ShortPlatform::WindowsX64),
        _ => Err(PyNotImplementedError::new_err(
            "official browser executable metadata is unsupported on this host target",
        )),
    }
}

#[cfg(feature = "browser-registry")]
fn host_platform(platform: ShortPlatform) -> String {
    if let Ok(value) = std::env::var("PLAYWRIGHT_HOST_PLATFORM_OVERRIDE")
        && !value.is_empty()
    {
        return value;
    }
    match platform {
        ShortPlatform::MacArm64 => "mac15-arm64".to_owned(),
        ShortPlatform::MacX64 => "mac15".to_owned(),
        ShortPlatform::WindowsX64 => "win64".to_owned(),
        ShortPlatform::LinuxArm64 | ShortPlatform::LinuxX64 => {
            let arch = if platform == ShortPlatform::LinuxArm64 {
                "arm64"
            } else {
                "x64"
            };
            let release = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
            linux_host_platform(&release, arch)
        }
    }
}

#[cfg(feature = "browser-registry")]
fn short_platform(host_platform: &str) -> PyResult<ShortPlatform> {
    if host_platform == "<unknown>" {
        return Err(PyNotImplementedError::new_err(
            "official browser executable metadata is unavailable for an unknown host platform",
        ));
    }
    if host_platform == "win64" {
        return Ok(ShortPlatform::WindowsX64);
    }
    if host_platform.starts_with("mac") {
        return Ok(if host_platform.ends_with("arm64") {
            ShortPlatform::MacArm64
        } else {
            ShortPlatform::MacX64
        });
    }
    Ok(if host_platform.ends_with("arm64") {
        ShortPlatform::LinuxArm64
    } else {
        ShortPlatform::LinuxX64
    })
}

#[cfg(feature = "browser-registry")]
fn linux_host_platform(release: &str, arch: &str) -> String {
    let mut id = "";
    let mut version = "";
    for line in release.lines() {
        if let Some(value) = line.strip_prefix("ID=") {
            id = value.trim_matches('"');
        } else if let Some(value) = line.strip_prefix("VERSION_ID=") {
            version = value.trim_matches('"');
        }
    }
    match (id, version) {
        ("debian" | "raspbian", "11") => format!("debian11-{arch}"),
        ("ubuntu" | "pop" | "neon" | "tuxedo", value)
            if value
                .split('.')
                .next()
                .and_then(|major| major.parse::<u32>().ok())
                .is_some_and(|major| major < 22) =>
        {
            format!("ubuntu20.04-{arch}")
        }
        ("linuxmint", value)
            if value
                .split('.')
                .next()
                .and_then(|major| major.parse::<u32>().ok())
                .is_some_and(|major| major <= 20) =>
        {
            format!("ubuntu20.04-{arch}")
        }
        _ => format!("ubuntu24.04-{arch}"),
    }
}

#[cfg(all(test, feature = "browser-registry"))]
mod tests {
    use super::{linux_host_platform, short_platform};
    use pwright_browser_registry::ShortPlatform;

    #[test]
    fn linux_os_release_selects_pinned_old_webkit_revision_tags() {
        // Arrange
        let debian = "ID=debian\nVERSION_ID=\"11\"\n";
        let ubuntu = "ID=ubuntu\nVERSION_ID=\"20.04\"\n";

        // Act
        let debian_tag = linux_host_platform(debian, "arm64");
        let ubuntu_tag = linux_host_platform(ubuntu, "x64");

        // Assert
        assert_eq!(debian_tag, "debian11-arm64");
        assert_eq!(ubuntu_tag, "ubuntu20.04-x64");
    }

    #[test]
    fn host_override_changes_executable_platform_not_cache_policy() {
        // Arrange
        let host_platform = "mac15-arm64";

        // Act
        let platform = short_platform(host_platform).unwrap();

        // Assert
        assert_eq!(platform, ShortPlatform::MacArm64);
    }
}
