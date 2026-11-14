//! Playwright 1.58.0 connection headers, independent of CDP and browser ownership.
//! Runtime observations are provided by the binding; this crate owns policy.

#[cfg(feature = "server-ws")]
mod server_ws;
#[cfg(feature = "server-ws")]
pub use server_ws::{
    append_server_connect_defaults, append_server_connect_defaults_for_http_discovery,
    needs_default_user_agent,
};

pub const PLAYWRIGHT_COMPAT_VERSION: &str = "1.58.0";

pub struct RuntimeInputs {
    pub platform: String,
    pub arch: String,
    pub release: String,
    pub mac_product_version: Option<String>,
    pub linux_os_release: Option<String>,
    pub node_version: Option<(u32, u32)>,
    pub version_override: Option<String>,
    pub language_name: Option<String>,
    pub language_version: Option<String>,
    pub ci: bool,
}

fn first_two_version_parts(version: &str) -> String {
    let mut parts = version.split('.');
    let major = parts.next().unwrap_or_default();
    let minor = parts.next().unwrap_or("undefined");
    format!("{major}.{minor}")
}

fn linux_distribution(text: &str) -> (String, String) {
    let mut id = None;
    let mut version = None;
    for line in text.split('\n') {
        let (name, value) = line.split_once('=').unwrap_or((line, ""));
        if name.is_empty() {
            continue;
        }
        let value = value.trim();
        let value = value
            .strip_prefix('"')
            .and_then(|value| value.strip_suffix('"'))
            .unwrap_or(value);
        match name.to_ascii_lowercase().as_str() {
            "id" => id = Some(value.to_owned()),
            "version_id" => version = Some(value.to_owned()),
            _ => {}
        }
    }
    (
        id.filter(|value| !value.is_empty())
            .unwrap_or_else(|| "linux".into()),
        version
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "unknown".into()),
    )
}

pub fn format_user_agent(inputs: &RuntimeInputs) -> String {
    let Some((node_major, node_minor)) = inputs.node_version else {
        return "Playwright/unknown".into();
    };
    let (os_name, os_version) = match inputs.platform.as_str() {
        "win32" => (
            "windows".to_owned(),
            first_two_version_parts(&inputs.release),
        ),
        "darwin" => {
            let Some(version) = inputs.mac_product_version.as_deref() else {
                return "Playwright/unknown".into();
            };
            ("macOS".to_owned(), first_two_version_parts(version))
        }
        "linux" => {
            let (name, version) = inputs
                .linux_os_release
                .as_deref()
                .map(linux_distribution)
                .unwrap_or_else(|| ("linux".into(), "unknown".into()));
            (name, version)
        }
        _ => ("unknown".into(), "unknown".into()),
    };
    let (embedder_name, embedder_version) = match inputs.language_name.as_deref() {
        None | Some("") => ("node", format!("{node_major}.{node_minor}")),
        Some(language @ ("node" | "python" | "java" | "csharp")) => (
            language,
            inputs
                .language_version
                .clone()
                .unwrap_or_else(|| "unknown".into()),
        ),
        Some(_) => ("unknown", "unknown".into()),
    };
    let version = inputs
        .version_override
        .as_deref()
        .filter(|value| !value.is_empty())
        .unwrap_or(PLAYWRIGHT_COMPAT_VERSION);
    let ci = if inputs.ci { " CI/1" } else { "" };
    format!(
        "Playwright/{version} ({}; {os_name} {os_version}) {embedder_name}/{embedder_version}{ci}",
        inputs.arch
    )
}
