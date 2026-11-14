//! Optional, Context-scoped Playwright device emulation.
//!
//! The embedded catalog is immutable package data. Runtime settings are
//! retained in a fixed Context registry and applied by the existing Chromium
//! session-initialization fence before a Page is resumed or published.

use std::error::Error;
use std::sync::{Arc, Mutex};

use pwright_backend_cdp_topology::CdpContextOwnership;
use pwright_cdp::{CdpConnection, CdpSession, generated::browser};
use pwright_core::{
    ObjectId, ProductionResourceLedger, ResourceClass, ResourceLease, ResourceRequest,
    ResourceRequirements, ResourceScope,
};
use pwright_plugin_chromium_session_initialization::{
    ChromiumSessionInitializationFuture, ChromiumSessionInitializationTarget,
    ChromiumSessionInitializer, ChromiumSessionTargetKind,
};
use serde::Serialize;

use crate::HostError;

const MAX_CONTEXTS: usize = 16;
const MAX_USER_AGENT_BYTES: usize = 4096;
const MAX_RETAINED_BYTES: u64 = 64 * 1024;
const MAX_VIEWPORT_DIMENSION: u32 = 10_000_000;
const DEVICE_SCOPE: ResourceScope = ResourceScope::extension(177, 1);

pub const fn device_descriptors_json() -> &'static str {
    include_str!("device_descriptors.json")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeviceViewport {
    width: u32,
    height: u32,
}

impl DeviceViewport {
    pub fn new(width: u32, height: u32) -> Result<Self, HostError> {
        if width == 0
            || height == 0
            || width > MAX_VIEWPORT_DIMENSION
            || height > MAX_VIEWPORT_DIMENSION
        {
            return Err(HostError::InvalidArgument(format!(
                "device viewport dimensions must be between 1 and {MAX_VIEWPORT_DIMENSION}"
            )));
        }
        Ok(Self { width, height })
    }

    #[cfg(feature = "page-viewport")]
    pub(crate) const fn width(self) -> u32 {
        self.width
    }

    #[cfg(feature = "page-viewport")]
    pub(crate) const fn height(self) -> u32 {
        self.height
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeviceContextOptions {
    user_agent: Option<Box<str>>,
    viewport: Option<DeviceViewport>,
    screen: Option<DeviceViewport>,
    device_scale_factor: Option<f64>,
    is_mobile: Option<bool>,
    has_touch: Option<bool>,
}

impl DeviceContextOptions {
    pub fn new(
        user_agent: Option<String>,
        viewport: Option<(u32, u32)>,
        screen: Option<(u32, u32)>,
        device_scale_factor: Option<f64>,
        is_mobile: Option<bool>,
        has_touch: Option<bool>,
    ) -> Result<Self, HostError> {
        let user_agent = user_agent
            .map(|value| {
                if value.len() > MAX_USER_AGENT_BYTES {
                    return Err(HostError::ResourceExhausted(format!(
                        "device user agent exceeds {MAX_USER_AGENT_BYTES} bytes"
                    )));
                }
                Ok(value.into_boxed_str())
            })
            .transpose()?;
        let viewport = viewport
            .map(|(width, height)| DeviceViewport::new(width, height))
            .transpose()?;
        let screen = screen
            .map(|(width, height)| DeviceViewport::new(width, height))
            .transpose()?;
        if device_scale_factor.is_some_and(|value| !value.is_finite() || value < 0.0) {
            return Err(HostError::InvalidArgument(
                "device_scale_factor must be finite and non-negative".into(),
            ));
        }
        Ok(Self {
            user_agent,
            viewport,
            screen,
            device_scale_factor,
            is_mobile,
            has_touch,
        })
    }

    fn retained_bytes(&self) -> Result<u64, HostError> {
        let bytes = std::mem::size_of::<Self>()
            .checked_add(self.user_agent.as_ref().map_or(0, |value| value.len()))
            .ok_or_else(|| HostError::ResourceExhausted("device settings size overflow".into()))?;
        u64::try_from(bytes)
            .map_err(|_| HostError::ResourceExhausted("device settings size overflow".into()))
    }
}

pub(crate) struct StoredSettings {
    options: DeviceContextOptions,
    _lease: ResourceLease,
}

impl StoredSettings {
    pub(crate) fn user_agent(&self) -> Option<&str> {
        self.options.user_agent.as_deref()
    }

    #[cfg(any(feature = "touchscreen-input", test))]
    pub(crate) fn has_touch(&self) -> bool {
        self.options.has_touch.unwrap_or(false)
    }
}

struct ContextEntry {
    context: ObjectId,
    settings: Arc<StoredSettings>,
}

struct State {
    default_settings: Option<Arc<StoredSettings>>,
    entries: Vec<ContextEntry>,
    closed: bool,
}

pub const fn requirements() -> ResourceRequirements {
    let storage_bytes = (std::mem::size_of::<Installation>()
        + std::mem::size_of::<State>()
        + MAX_CONTEXTS * std::mem::size_of::<ContextEntry>()) as u64;
    ResourceRequirements::none()
        .with_class(ResourceClass::EventItem, MAX_CONTEXTS as u64, 1)
        .with_class(
            ResourceClass::EventBytes,
            MAX_RETAINED_BYTES,
            MAX_RETAINED_BYTES,
        )
        .with_class(
            ResourceClass::BackendStateBytes,
            storage_bytes,
            storage_bytes,
        )
        .with_metadata(MAX_CONTEXTS + 1, MAX_CONTEXTS * 3 + 1)
}

pub(crate) struct Installation {
    resources: pwright_core::ResourceLedger,
    state: Mutex<State>,
    _storage: ResourceLease,
}

impl Installation {
    pub(crate) fn new_with_default(
        resources: &ProductionResourceLedger,
        default_options: Option<DeviceContextOptions>,
    ) -> Result<Arc<Self>, HostError> {
        let plan = requirements();
        let ledger = resources.claim(plan).map_err(HostError::backend)?;
        let storage = ledger
            .reserve(
                DEVICE_SCOPE,
                ResourceClass::BackendStateBytes,
                plan.class(ResourceClass::BackendStateBytes).global,
            )
            .map_err(HostError::backend)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(MAX_CONTEXTS)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let default_settings = default_options
            .map(|options| {
                let bytes = options.retained_bytes()?;
                let lease = ledger
                    .reserve_many(
                        DEVICE_SCOPE,
                        &[
                            ResourceRequest::new(ResourceClass::EventItem, 1),
                            ResourceRequest::new(ResourceClass::EventBytes, bytes),
                        ],
                    )
                    .map_err(HostError::backend)?;
                Ok::<_, HostError>(Arc::new(StoredSettings {
                    options,
                    _lease: lease,
                }))
            })
            .transpose()?;
        Ok(Arc::new(Self {
            resources: ledger,
            state: Mutex::new(State {
                default_settings,
                entries,
                closed: false,
            }),
            _storage: storage,
        }))
    }

    pub(crate) fn install(
        &self,
        context: ObjectId,
        options: DeviceContextOptions,
    ) -> Result<(), HostError> {
        let bytes = options.retained_bytes()?;
        let mut state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        if state.entries.iter().any(|entry| entry.context == context) {
            return Err(HostError::InvalidArgument(
                "device settings are already installed for this Context".into(),
            ));
        }
        let retained = state
            .entries
            .len()
            .saturating_add(usize::from(state.default_settings.is_some()));
        if retained >= MAX_CONTEXTS {
            return Err(HostError::ResourceExhausted(format!(
                "device settings exceed {MAX_CONTEXTS} live Contexts"
            )));
        }
        let lease = self
            .resources
            .reserve_many(
                ResourceScope::Object(context),
                &[
                    ResourceRequest::new(ResourceClass::EventItem, 1),
                    ResourceRequest::new(ResourceClass::EventBytes, bytes),
                ],
            )
            .map_err(HostError::backend)?;
        state.entries.push(ContextEntry {
            context,
            settings: Arc::new(StoredSettings {
                options,
                _lease: lease,
            }),
        });
        Ok(())
    }

    pub(crate) fn remove(&self, context: ObjectId) -> Result<(), HostError> {
        self.state
            .lock()
            .map_err(HostError::backend)?
            .entries
            .retain(|entry| entry.context != context);
        Ok(())
    }

    pub(crate) fn settings(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<Option<Arc<StoredSettings>>, HostError> {
        let state = self.state.lock().map_err(HostError::backend)?;
        if state.closed {
            return Err(HostError::TargetClosed);
        }
        Ok(match ownership {
            CdpContextOwnership::Default => state.default_settings.as_ref().map(Arc::clone),
            CdpContextOwnership::Owned => state
                .entries
                .iter()
                .find(|entry| entry.context == context)
                .map(|entry| Arc::clone(&entry.settings)),
            CdpContextOwnership::External => None,
        })
    }

    #[cfg(feature = "page-viewport")]
    pub(crate) fn page_metrics(
        &self,
        context: ObjectId,
        ownership: CdpContextOwnership,
        viewport: DeviceViewport,
    ) -> Result<MetricsOverride, HostError> {
        let mut options = self
            .settings(context, ownership)?
            .map(|settings| settings.options.clone())
            .unwrap_or(DeviceContextOptions {
                user_agent: None,
                viewport: None,
                screen: None,
                device_scale_factor: None,
                is_mobile: None,
                has_touch: None,
            });
        options.viewport = Some(viewport);
        options.screen = Some(viewport);
        Ok(metrics(&options))
    }

    fn close(&self) {
        let Ok(mut state) = self.state.lock() else {
            return;
        };
        state.closed = true;
        state.default_settings = None;
        state.entries.clear();
    }
}

impl Drop for Installation {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MetricsOverride {
    width: u32,
    height: u32,
    screen_width: u32,
    screen_height: u32,
    device_scale_factor: f64,
    mobile: bool,
    screen_orientation: ScreenOrientation,
}

#[derive(Serialize)]
struct ScreenOrientation {
    #[serde(rename = "type")]
    kind: &'static str,
    angle: u16,
}

fn metrics(options: &DeviceContextOptions) -> MetricsOverride {
    let viewport = options.viewport.unwrap_or(DeviceViewport {
        width: 1280,
        height: 720,
    });
    let screen = options.screen.unwrap_or(viewport);
    let mobile = options.is_mobile.unwrap_or(false);
    let landscape = screen.width > screen.height;
    MetricsOverride {
        width: viewport.width,
        height: viewport.height,
        screen_width: screen.width,
        screen_height: screen.height,
        device_scale_factor: options.device_scale_factor.unwrap_or(1.0),
        mobile,
        screen_orientation: if mobile && !landscape {
            ScreenOrientation {
                kind: "portraitPrimary",
                angle: 0,
            }
        } else if mobile {
            ScreenOrientation {
                kind: "landscapePrimary",
                angle: 90,
            }
        } else {
            ScreenOrientation {
                kind: "landscapePrimary",
                angle: 0,
            }
        },
    }
}

pub(crate) fn window_insets(headful: bool, host_platform: &str) -> (u32, u32) {
    if !headful {
        return (0, 0);
    }
    match host_platform {
        "macos" => (2, 80),
        "windows" => (16, 88),
        "linux" => (8, 85),
        _ => (24, 88),
    }
}

pub(crate) struct Initializer<Downstream> {
    downstream: Downstream,
    installation: Arc<Installation>,
    connection: Arc<CdpConnection>,
    headful: bool,
    host_platform: &'static str,
}

impl<Downstream> Initializer<Downstream> {
    pub(crate) fn new(
        downstream: Downstream,
        installation: Arc<Installation>,
        connection: Arc<CdpConnection>,
        headful: bool,
        host_platform: &'static str,
    ) -> Self {
        Self {
            downstream,
            installation,
            connection,
            headful,
            host_platform,
        }
    }
}

impl<Downstream> ChromiumSessionInitializer for Initializer<Downstream>
where
    Downstream: ChromiumSessionInitializer,
{
    fn initialize<'a>(
        &'a self,
        session_id: &'a str,
        target_id: &'a str,
        ownership: CdpContextOwnership,
        is_page: bool,
    ) -> ChromiumSessionInitializationFuture<'a> {
        self.downstream
            .initialize(session_id, target_id, ownership, is_page)
    }

    fn initialize_scoped<'a>(
        &'a self,
        target: ChromiumSessionInitializationTarget<'a>,
    ) -> ChromiumSessionInitializationFuture<'a> {
        Box::pin(async move {
            self.downstream.initialize_scoped(target).await?;
            let Some(settings) = self
                .installation
                .settings(target.context, target.ownership)?
            else {
                return Ok(());
            };
            let session = CdpSession::new(
                Arc::clone(&self.connection),
                target.session_id.to_owned(),
                target.target_id.to_owned(),
            );
            if target.kind == ChromiumSessionTargetKind::Page {
                let values = metrics(&settings.options);
                let (inset_width, inset_height) = window_insets(self.headful, self.host_platform);
                let window = session
                    .send(
                        "Browser.getWindowForTarget",
                        serde_json::to_value(browser::GetWindowForTargetParams::default())?,
                    )
                    .await?;
                let window: browser::GetWindowForTargetReturns = serde_json::from_value(window)?;
                session
                    .send(
                        "Browser.setWindowBounds",
                        serde_json::to_value(browser::SetWindowBoundsParams {
                            window_id: window.window_id,
                            bounds: browser::Bounds {
                                width: Some(i64::from(values.width + inset_width)),
                                height: Some(i64::from(values.height + inset_height)),
                                ..Default::default()
                            },
                        })?,
                    )
                    .await?;
                session
                    .send(
                        "Emulation.setDeviceMetricsOverride",
                        serde_json::to_value(values)?,
                    )
                    .await?;
            }
            if let Some(has_touch) = settings.options.has_touch {
                session
                    .send(
                        "Emulation.setTouchEmulationEnabled",
                        serde_json::json!({
                            "enabled": has_touch,
                            "maxTouchPoints": if has_touch { 1 } else { 0 },
                        }),
                    )
                    .await?;
            }
            Ok::<(), Box<dyn Error + Send + Sync>>(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;

    fn resources() -> ProductionResourceLedger {
        let required = requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let limit = required.class(class);
            limits = limits.with_class(class, limit.global, limit.per_scope);
        }
        ProductionResourceLedger::new(limits, required).expect("valid device resource plan")
    }

    #[test]
    fn device_settings_are_context_scoped_and_release_on_remove() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), None).unwrap();
        let context = ObjectId::new(7);
        let options = DeviceContextOptions::new(
            Some("Pixel 2 user agent".to_owned()),
            Some((411, 731)),
            None,
            Some(2.625),
            Some(true),
            Some(true),
        )
        .unwrap();

        // Act
        installation.install(context, options.clone()).unwrap();
        let observed = installation
            .settings(context, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();
        installation.remove(context).unwrap();

        // Assert
        assert_eq!(observed.options, options);
        assert!(
            installation
                .settings(context, CdpContextOwnership::Owned)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn mobile_metrics_preserve_fractional_scale_and_portrait_screen() {
        // Arrange
        let options = DeviceContextOptions::new(
            None,
            Some((411, 731)),
            None,
            Some(2.625),
            Some(true),
            Some(true),
        )
        .unwrap();

        // Act
        let encoded = serde_json::to_value(metrics(&options)).unwrap();

        // Assert
        assert_eq!(
            encoded,
            serde_json::json!({
                "width": 411,
                "height": 731,
                "screenWidth": 411,
                "screenHeight": 731,
                "deviceScaleFactor": 2.625,
                "mobile": true,
                "screenOrientation": { "type": "portraitPrimary", "angle": 0 },
            })
        );
    }

    #[cfg(feature = "page-viewport")]
    #[test]
    fn page_viewport_resets_screen_but_preserves_context_device_semantics() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), None).unwrap();
        let context = ObjectId::new(9);
        installation
            .install(
                context,
                DeviceContextOptions::new(
                    None,
                    Some((411, 731)),
                    Some((1080, 1920)),
                    Some(2.625),
                    Some(true),
                    Some(true),
                )
                .unwrap(),
            )
            .unwrap();
        let viewport = DeviceViewport::new(500, 400).unwrap();

        // Act
        let encoded = serde_json::to_value(
            installation
                .page_metrics(context, CdpContextOwnership::Owned, viewport)
                .unwrap(),
        )
        .unwrap();

        // Assert
        assert_eq!(
            encoded,
            serde_json::json!({
                "width": 500,
                "height": 400,
                "screenWidth": 500,
                "screenHeight": 400,
                "deviceScaleFactor": 2.625,
                "mobile": true,
                "screenOrientation": { "type": "landscapePrimary", "angle": 90 },
            })
        );
    }

    #[test]
    fn default_settings_are_selected_only_for_the_default_context() {
        // Arrange
        let options = DeviceContextOptions::new(
            Some("persistent user agent".to_owned()),
            Some((456, 789)),
            None,
            Some(3.0),
            Some(false),
            Some(true),
        )
        .unwrap();
        let installation =
            Installation::new_with_default(&resources(), Some(options.clone())).unwrap();
        let context = ObjectId::new(11);

        // Act
        let default_settings = installation
            .settings(context, CdpContextOwnership::Default)
            .unwrap();
        let owned_settings = installation
            .settings(context, CdpContextOwnership::Owned)
            .unwrap();
        let external_settings = installation
            .settings(context, CdpContextOwnership::External)
            .unwrap();

        // Assert
        assert_eq!(default_settings.unwrap().options, options);
        assert!(owned_settings.is_none());
        assert!(external_settings.is_none());
    }

    #[test]
    fn retained_touch_setting_defaults_to_false_and_preserves_true() {
        // Arrange
        let installation = Installation::new_with_default(&resources(), None).unwrap();
        let disabled_context = ObjectId::new(12);
        let enabled_context = ObjectId::new(13);
        installation
            .install(
                disabled_context,
                DeviceContextOptions::new(None, None, None, None, None, None).unwrap(),
            )
            .unwrap();
        installation
            .install(
                enabled_context,
                DeviceContextOptions::new(None, None, None, None, None, Some(true)).unwrap(),
            )
            .unwrap();

        // Act
        let disabled = installation
            .settings(disabled_context, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();
        let enabled = installation
            .settings(enabled_context, CdpContextOwnership::Owned)
            .unwrap()
            .unwrap();

        // Assert
        assert!(!disabled.has_touch());
        assert!(enabled.has_touch());
    }

    #[test]
    fn default_settings_consume_one_fixed_context_slot() {
        // Arrange
        let default_options =
            DeviceContextOptions::new(None, Some((456, 789)), None, None, None, None).unwrap();
        let installation =
            Installation::new_with_default(&resources(), Some(default_options)).unwrap();
        for value in 1..MAX_CONTEXTS {
            let options =
                DeviceContextOptions::new(None, None, None, None, None, Some(true)).unwrap();
            installation
                .install(ObjectId::new(value as u64), options)
                .unwrap();
        }

        // Act
        let error = installation
            .install(
                ObjectId::new(MAX_CONTEXTS as u64),
                DeviceContextOptions::new(None, None, None, None, None, Some(true)).unwrap(),
            )
            .unwrap_err();

        // Assert
        assert!(matches!(error, HostError::ResourceExhausted(_)));
    }
}
