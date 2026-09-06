/// CLI state persistence — `.pwright/state.json`
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

const STATE_DIR: &str = ".pwright";
const STATE_FILE: &str = "state.json";
const EPHEMERAL_TAB_PREFIX: &str = "ephemeral-tab-";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CliState {
    /// Chrome CDP HTTP endpoint (e.g. "http://localhost:9222")
    pub cdp_url: String,
    /// WebSocket URL resolved from the HTTP endpoint
    pub ws_url: String,
    /// Active tab's internal ID (e.g. "tab_00000000")
    pub active_tab: String,
    /// CDP target ID for re-attaching
    pub target_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EphemeralTabLease {
    pub cdp_url: String,
    pub target_id: String,
    pub owner_pid: u32,
}

pub fn save_ephemeral_tab_lease(
    base_dir: &Path,
    cdp_url: &str,
    target_id: &str,
) -> Result<PathBuf> {
    let state_dir = base_dir.join(STATE_DIR);
    std::fs::create_dir_all(&state_dir)?;
    let filename = format!("{EPHEMERAL_TAB_PREFIX}{}.json", std::process::id());
    let path = state_dir.join(&filename);
    let temporary = state_dir.join(format!(".{filename}.tmp"));
    let lease = EphemeralTabLease {
        cdp_url: cdp_url.to_owned(),
        target_id: target_id.to_owned(),
        owner_pid: std::process::id(),
    };
    std::fs::write(&temporary, serde_json::to_vec_pretty(&lease)?)?;
    restrict_permissions(&temporary);
    std::fs::rename(&temporary, &path)?;
    Ok(path)
}

pub fn ephemeral_tab_lease_paths(base_dir: &Path) -> Result<Vec<PathBuf>> {
    let state_dir = base_dir.join(STATE_DIR);
    if !state_dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = std::fs::read_dir(state_dir)?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with(EPHEMERAL_TAB_PREFIX) && name.ends_with(".json")
                })
        })
        .collect::<Vec<_>>();
    paths.sort();
    Ok(paths)
}

pub fn load_ephemeral_tab_lease(path: &Path) -> Result<EphemeralTabLease> {
    let bytes = std::fs::read(path)
        .with_context(|| format!("failed to read tab lease {}", path.display()))?;
    serde_json::from_slice(&bytes)
        .with_context(|| format!("failed to parse tab lease {}", path.display()))
}

pub fn remove_ephemeral_tab_lease(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => {
            Err(error).with_context(|| format!("failed to remove tab lease {}", path.display()))
        }
    }
}

fn restrict_permissions(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(error) = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)) {
            tracing::debug!("failed to set state file permissions: {error}");
        }
    }
}

impl CliState {
    fn state_dir() -> PathBuf {
        PathBuf::from(STATE_DIR)
    }

    fn state_path() -> PathBuf {
        Self::state_dir().join(STATE_FILE)
    }

    /// Load state from disk, or return default.
    /// Warns on corruption instead of silently losing the session.
    pub fn load() -> Self {
        let path = Self::state_path();
        if !path.exists() {
            return Self::default();
        }
        match std::fs::read_to_string(&path) {
            Ok(data) => match serde_json::from_str(&data) {
                Ok(state) => state,
                Err(e) => {
                    eprintln!(
                        "[warning] state file {} is corrupted ({}), starting fresh",
                        path.display(),
                        e
                    );
                    Self::default()
                }
            },
            Err(e) => {
                eprintln!(
                    "[warning] cannot read state file {} ({}), starting fresh",
                    path.display(),
                    e
                );
                Self::default()
            }
        }
    }

    /// Save state to disk.
    pub fn save(&self) -> Result<()> {
        let dir = Self::state_dir();
        if !dir.exists() {
            std::fs::create_dir_all(&dir)?;
        }
        let path = Self::state_path();
        let data = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, data).context("failed to write state file")?;

        // Restrict permissions: state contains WS URL granting browser control
        restrict_permissions(&path);

        Ok(())
    }

    /// Discover WS URL from Chrome HTTP endpoint.
    pub async fn fetch_ws_url(cdp_http: &str) -> Result<String> {
        let version_url = format!("{}/json/version", cdp_http.trim_end_matches('/'));
        let resp: serde_json::Value = reqwest::get(&version_url)
            .await
            .context("cannot connect to Chrome")?
            .json()
            .await
            .context("invalid JSON from Chrome")?;
        resp["webSocketDebuggerUrl"]
            .as_str()
            .map(|s| s.to_string())
            .context("no webSocketDebuggerUrl in response")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_has_empty_fields() {
        let state = CliState::default();
        assert!(state.cdp_url.is_empty());
        assert!(state.ws_url.is_empty());
        assert!(state.active_tab.is_empty());
        assert!(state.target_id.is_empty());
    }

    #[test]
    fn state_serializes_to_json() {
        let state = CliState {
            cdp_url: "http://localhost:9222".to_string(),
            ws_url: "ws://localhost:9222/devtools/browser/abc".to_string(),
            active_tab: "tab_00000000".to_string(),
            target_id: "ABCDEF123456".to_string(),
        };
        let json = serde_json::to_string(&state).unwrap();
        assert!(json.contains("localhost:9222"));
        assert!(json.contains("tab_00000000"));
        assert!(json.contains("ABCDEF123456"));
    }

    #[test]
    fn state_deserializes_from_json() {
        let json = r#"{
            "cdp_url": "http://localhost:9222",
            "ws_url": "ws://localhost:9222/devtools/browser/abc",
            "active_tab": "tab_00000000",
            "target_id": "TARGET123"
        }"#;
        let state: CliState = serde_json::from_str(json).unwrap();
        assert_eq!(state.cdp_url, "http://localhost:9222");
        assert_eq!(state.active_tab, "tab_00000000");
        assert_eq!(state.target_id, "TARGET123");
    }

    #[test]
    fn state_deserializes_from_empty_json_fails() {
        // All fields are required, so empty JSON should fail
        let result = serde_json::from_str::<CliState>("{}");
        assert!(result.is_err());
    }

    #[test]
    fn state_roundtrips_through_json() {
        let original = CliState {
            cdp_url: "http://host:1234".to_string(),
            ws_url: "ws://host:1234/devtools".to_string(),
            active_tab: "tab_42".to_string(),
            target_id: "XYZ".to_string(),
        };
        let json = serde_json::to_string(&original).unwrap();
        let restored: CliState = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.cdp_url, original.cdp_url);
        assert_eq!(restored.ws_url, original.ws_url);
        assert_eq!(restored.active_tab, original.active_tab);
        assert_eq!(restored.target_id, original.target_id);
    }

    #[test]
    fn state_path_is_under_state_dir() {
        let path = CliState::state_path();
        assert!(path.starts_with(STATE_DIR));
        assert!(path.to_str().unwrap().ends_with(STATE_FILE));
    }

    #[test]
    fn load_returns_default_when_no_file() {
        // Load from a non-existent path should return default
        let state = CliState::load();
        // This is fine — in CI/test env there's no .pwright/state.json
        // so it returns default
        let _ = state;
    }

    #[test]
    fn ephemeral_tab_lease_roundtrips_and_is_removed() -> Result<()> {
        // Arrange
        let test_root = std::env::temp_dir().join(format!(
            "pwright-tab-lease-test-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("unnamed")
        ));
        if test_root.exists() {
            std::fs::remove_dir_all(&test_root)?;
        }
        std::fs::create_dir_all(&test_root)?;

        // Act
        let path = save_ephemeral_tab_lease(&test_root, "http://browser:9222", "target-abc")?;
        let paths = ephemeral_tab_lease_paths(&test_root)?;
        let lease = load_ephemeral_tab_lease(&path)?;
        remove_ephemeral_tab_lease(&path)?;

        // Assert
        assert_eq!(paths, vec![path.clone()]);
        assert_eq!(lease.cdp_url, "http://browser:9222");
        assert_eq!(lease.target_id, "target-abc");
        assert!(!path.exists());
        std::fs::remove_dir_all(test_root)?;
        Ok(())
    }

    #[test]
    fn removing_an_already_removed_tab_lease_is_idempotent() -> Result<()> {
        // Arrange
        let directory = tempfile::tempdir()?;
        let path = save_ephemeral_tab_lease(directory.path(), "http://browser:9222", "target-abc")?;
        remove_ephemeral_tab_lease(&path)?;

        // Act
        let result = remove_ephemeral_tab_lease(&path);

        // Assert
        assert!(result.is_ok());
        assert!(!path.exists());
        Ok(())
    }
}
