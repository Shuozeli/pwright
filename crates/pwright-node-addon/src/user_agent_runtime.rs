//! Bounded production observations for the Rust-owned Playwright User-Agent policy.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use napi::{Env, JsString};
use pwright_node_host::HostError;
use pwright_playwright_connect_headers::RuntimeInputs;

use crate::{binding_error, boundary};

const MAX_OS_RELEASE_BYTES: usize = 64 * 1024;
const MAX_MAC_VERSION_BYTES: usize = 128;

fn bounded_env(name: &str, max_bytes: usize) -> napi::Result<Option<String>> {
    let Some(value) = std::env::var_os(name) else {
        return Ok(None);
    };
    let value = value.into_string().map_err(|_| {
        binding_error(HostError::InvalidArgument(format!(
            "{name} must be valid UTF-8"
        )))
    })?;
    if value.len() > max_bytes {
        return Err(binding_error(HostError::ResourceExhausted(format!(
            "{name} exceeds {max_bytes} UTF-8 bytes"
        ))));
    }
    Ok(Some(value))
}

fn mac_product_version() -> Option<String> {
    let mut child = Command::new("sw_vers")
        .arg("-productVersion")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let deadline = Instant::now() + Duration::from_secs(2);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    if !status.success() {
        return None;
    }
    let mut bytes = Vec::with_capacity(MAX_MAC_VERSION_BYTES);
    let stdout = child.stdout.take()?;
    if stdout
        .take((MAX_MAC_VERSION_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() > MAX_MAC_VERSION_BYTES
    {
        return None;
    }
    let version = String::from_utf8(bytes).ok()?;
    let version = version.trim();
    (!version.is_empty()).then(|| version.to_owned())
}

fn linux_os_release() -> Option<String> {
    let file = std::fs::File::open("/etc/os-release").ok()?;
    let mut text = String::new();
    file.take((MAX_OS_RELEASE_BYTES + 1) as u64)
        .read_to_string(&mut text)
        .ok()?;
    (text.len() <= MAX_OS_RELEASE_BYTES).then_some(text)
}

pub fn collect(
    env: &Env,
    platform: JsString<'_>,
    arch: JsString<'_>,
    release: JsString<'_>,
) -> napi::Result<RuntimeInputs> {
    let platform = boundary::bounded_string(platform, 64, "Node platform")?;
    let arch = boundary::bounded_string(arch, 64, "Node architecture")?;
    let release = boundary::bounded_string(release, 128, "Node OS release")?;
    let node_version = env
        .get_node_version()
        .ok()
        .map(|version| (version.major, version.minor));
    let mac_product_version = (platform == "darwin").then(mac_product_version).flatten();
    let linux_os_release = (platform == "linux").then(linux_os_release).flatten();
    Ok(RuntimeInputs {
        platform,
        arch,
        release,
        mac_product_version,
        linux_os_release,
        node_version,
        version_override: bounded_env("PW_VERSION_OVERRIDE", 128)?,
        language_name: bounded_env("PW_LANG_NAME", 32)?,
        language_version: bounded_env("PW_LANG_NAME_VERSION", 128)?,
        ci: bounded_env("CI", 128)?.is_some_and(|value| !value.is_empty()),
    })
}
