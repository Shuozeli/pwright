//! Opt-in Node argument and host-environment adapter for pure registry metadata.

use std::path::{Path, PathBuf};

use napi::JsString;
use napi_derive::napi;
use pwright_browser_registry::{
    BrowserEngine, RegistryDirectoryInputs, ShortPlatform, executable_path_v1_58_0,
    registry_directory,
};
use pwright_node_host::HostError;

use crate::{binding_error, boundary};

#[napi(object)]
pub struct BrowserRegistrySnapshot {
    pub browsers_path: Option<String>,
    pub init_cwd: Option<String>,
    pub xdg_cache_home: Option<String>,
    pub local_app_data: Option<String>,
    pub home: String,
    pub cwd: String,
    pub host_platform_override: Option<String>,
}

/// Report a pinned expected path, not evidence of an installed browser.
#[napi]
pub fn browser_executable_path_snapshot(
    engine: JsString<'_>,
    package_root: JsString<'_>,
    snapshot: BrowserRegistrySnapshot,
) -> napi::Result<String> {
    let engine = boundary::bounded_string(engine, 16, "browser engine")?;
    let engine = match engine.as_str() {
        "chromium" => BrowserEngine::Chromium,
        "firefox" => BrowserEngine::Firefox,
        "webkit" => BrowserEngine::WebKit,
        _ => {
            return Err(binding_error(HostError::InvalidArgument(
                "unknown browser engine".into(),
            )));
        }
    };
    let package_root = PathBuf::from(boundary::bounded_string(
        package_root,
        8192,
        "package root",
    )?);
    validate_snapshot(&snapshot)?;
    let platform = current_platform()?;
    let host_platform = host_platform(platform, snapshot.host_platform_override.as_deref());
    let executable_platform = short_platform(&host_platform)?;
    let inputs = RegistryDirectoryInputs {
        // The cache root follows process.platform, not the host-platform
        // override used for executable layout and revision selection.
        platform,
        browsers_path: snapshot.browsers_path.as_deref().map(Path::new),
        package_root: package_root.as_path(),
        home: Some(Path::new(&snapshot.home)),
        xdg_cache_home: snapshot.xdg_cache_home.as_deref().map(Path::new),
        local_app_data: snapshot.local_app_data.as_deref().map(Path::new),
        init_cwd: snapshot.init_cwd.as_deref().map(Path::new),
        cwd: Path::new(&snapshot.cwd),
    };
    let directory = registry_directory(&inputs)
        .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?;
    executable_path_v1_58_0(&directory, engine, executable_platform, &host_platform)
        .map_err(|error| binding_error(HostError::InvalidArgument(error.to_string())))?
        .into_os_string()
        .into_string()
        .map_err(|_| {
            binding_error(HostError::InvalidArgument(
                "browser executable path is not UTF-8".into(),
            ))
        })
}

fn validate_snapshot(snapshot: &BrowserRegistrySnapshot) -> napi::Result<()> {
    for (name, value) in [
        ("browsers path", snapshot.browsers_path.as_deref()),
        ("INIT_CWD", snapshot.init_cwd.as_deref()),
        ("XDG_CACHE_HOME", snapshot.xdg_cache_home.as_deref()),
        ("LOCALAPPDATA", snapshot.local_app_data.as_deref()),
        ("home", Some(snapshot.home.as_str())),
        ("current directory", Some(snapshot.cwd.as_str())),
        (
            "host platform override",
            snapshot.host_platform_override.as_deref(),
        ),
    ] {
        if value.is_some_and(|value| value.len() > 8192) {
            return Err(binding_error(HostError::InvalidArgument(format!(
                "{name} exceeds 8192 UTF-8 bytes"
            ))));
        }
    }
    Ok(())
}

fn current_platform() -> napi::Result<ShortPlatform> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok(ShortPlatform::MacArm64),
        ("macos", "x86_64") => Ok(ShortPlatform::MacX64),
        ("linux", "aarch64") => Ok(ShortPlatform::LinuxArm64),
        ("linux", "x86_64") => Ok(ShortPlatform::LinuxX64),
        ("windows", "x86_64") => Ok(ShortPlatform::WindowsX64),
        _ => Err(binding_error(HostError::UnsupportedOperation(
            "browser executable metadata is unsupported on this host target".into(),
        ))),
    }
}

fn host_platform(platform: ShortPlatform, platform_override: Option<&str>) -> String {
    if let Some(value) = platform_override.filter(|value| !value.is_empty()) {
        return value.to_owned();
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

fn short_platform(host_platform: &str) -> napi::Result<ShortPlatform> {
    if host_platform == "<unknown>" {
        return Err(binding_error(HostError::UnsupportedOperation(
            "browser executable metadata is unavailable for an unknown host platform".into(),
        )));
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
