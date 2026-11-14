//! Fixed Playwright defaults, explicitly selected for this host's owned Contexts.
//! No settings table, timer, task, browser launcher or foreign runtime state.

use std::sync::Arc;

use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cdp::generated::browser;
use pwright_cdp::{CdpConnection, CdpSession};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializer,
};
use serde::Serialize;

pub(crate) struct OwnedContextDefaults {
    connection: Arc<CdpConnection>,
    window_bounds: browser::Bounds,
}

impl OwnedContextDefaults {
    pub fn new(connection: Arc<CdpConnection>, headful: bool, host_platform: &str) -> Self {
        Self {
            connection,
            window_bounds: window_bounds(headful, host_platform),
        }
    }
}

fn window_bounds(headful: bool, host_platform: &str) -> browser::Bounds {
    // Playwright uses the driver platform's insets, not the remote browser OS.
    let (width, height) = if !headful {
        (0, 0)
    } else {
        match host_platform {
            "macos" => (2, 80),
            "windows" => (16, 88),
            "linux" => (8, 85),
            _ => (24, 88),
        }
    };
    browser::Bounds {
        width: Some(1280 + width),
        height: Some(720 + height),
        ..Default::default()
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Viewport {
    width: u16,
    height: u16,
    screen_width: u16,
    screen_height: u16,
    device_scale_factor: u8,
    mobile: bool,
    screen_orientation: ScreenOrientation,
}

#[derive(Serialize)]
struct ScreenOrientation {
    #[serde(rename = "type")]
    kind: &'static str,
    angle: u8,
}

#[derive(Serialize)]
struct Media {
    media: &'static str,
    features: [MediaFeature; 4],
}

#[derive(Serialize)]
struct MediaFeature {
    name: &'static str,
    value: &'static str,
}

#[derive(Serialize)]
struct Enabled {
    enabled: bool,
}

fn viewport() -> Viewport {
    Viewport {
        width: 1280,
        height: 720,
        screen_width: 1280,
        screen_height: 720,
        device_scale_factor: 1,
        mobile: false,
        screen_orientation: ScreenOrientation {
            kind: "landscapePrimary",
            angle: 0,
        },
    }
}

fn media() -> Media {
    Media {
        media: "",
        features: [
            MediaFeature {
                name: "prefers-color-scheme",
                value: "light",
            },
            MediaFeature {
                name: "prefers-reduced-motion",
                value: "no-preference",
            },
            MediaFeature {
                name: "forced-colors",
                value: "none",
            },
            MediaFeature {
                name: "prefers-contrast",
                value: "no-preference",
            },
        ],
    }
}

impl ChromiumSessionInitializer for OwnedContextDefaults {
    fn initialize<'a>(
        &'a self,
        session_id: &'a str,
        target_id: &'a str,
        ownership: CdpContextOwnership,
        is_page: bool,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            if ownership != CdpContextOwnership::Owned {
                return Ok(());
            }
            let session = CdpSession::new(
                self.connection.clone(),
                session_id.to_owned(),
                target_id.to_owned(),
            );
            let media = session.send("Emulation.setEmulatedMedia", serde_json::to_value(media())?);
            let automation = session.send(
                "Emulation.setAutomationOverride",
                serde_json::to_value(Enabled { enabled: true })?,
            );
            if is_page {
                let window = async {
                    let window = session
                        .send(
                            "Browser.getWindowForTarget",
                            serde_json::to_value(browser::GetWindowForTargetParams::default())?,
                        )
                        .await?;
                    let window: browser::GetWindowForTargetReturns =
                        serde_json::from_value(window)?;
                    session
                        .send(
                            "Browser.setWindowBounds",
                            serde_json::to_value(browser::SetWindowBoundsParams {
                                window_id: window.window_id,
                                bounds: self.window_bounds.clone(),
                            })?,
                        )
                        .await
                };
                let metrics = session.send(
                    "Emulation.setDeviceMetricsOverride",
                    serde_json::to_value(viewport())?,
                );
                let focus = session.send(
                    "Emulation.setFocusEmulationEnabled",
                    serde_json::to_value(Enabled { enabled: true })?,
                );
                tokio::try_join!(window, metrics, focus, media, automation)?;
            } else {
                tokio::try_join!(media, automation)?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_window_insets_follow_injected_driver_platform_and_headful_mode() {
        // Arrange
        let inputs = [
            (true, "macos"),
            (true, "windows"),
            (true, "linux"),
            (false, "macos"),
        ];
        // Act
        let sizes = inputs.map(|(headful, platform)| {
            let bounds = window_bounds(headful, platform);
            (bounds.width, bounds.height)
        });
        // Assert
        assert_eq!(
            sizes,
            [
                (Some(1282), Some(800)),
                (Some(1296), Some(808)),
                (Some(1288), Some(805)),
                (Some(1280), Some(720))
            ]
        );
    }

    #[test]
    fn default_viewport_uses_fixed_screen_and_desktop_scale() {
        // Arrange
        let params = viewport();
        // Act
        let encoded = serde_json::to_value(params).unwrap();
        // Assert
        assert_eq!(
            encoded,
            serde_json::json!({
                "width": 1280, "height": 720, "screenWidth": 1280, "screenHeight": 720,
                "deviceScaleFactor": 1, "mobile": false,
                "screenOrientation": { "type": "landscapePrimary", "angle": 0 },
            })
        );
    }

    #[test]
    fn default_media_does_not_inherit_os_dark_or_reduced_motion_preferences() {
        // Arrange
        let params = media();
        // Act
        let encoded = serde_json::to_value(params).unwrap();
        // Assert
        assert_eq!(
            encoded,
            serde_json::json!({ "media": "", "features": [
            { "name": "prefers-color-scheme", "value": "light" },
            { "name": "prefers-reduced-motion", "value": "no-preference" },
            { "name": "forced-colors", "value": "none" },
            { "name": "prefers-contrast", "value": "no-preference" },
        ] })
        );
    }

    #[test]
    fn owned_context_automation_override_is_explicitly_enabled() {
        // Arrange
        let enabled = Enabled { enabled: true };

        // Act
        let encoded = serde_json::to_value(enabled).unwrap();

        // Assert
        assert_eq!(encoded, serde_json::json!({ "enabled": true }));
    }
}
