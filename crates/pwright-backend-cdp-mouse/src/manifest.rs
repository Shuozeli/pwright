use pwright_browser_backend::{
    BackendDescriptor, BackendFeatureDescriptor, BackendFeatureId, BackendId,
    BackendImplementation, BackendProtocolId, BrowserBackend, BrowserEngine,
};
use pwright_cap_mouse::*;
use pwright_core::ProtocolVersion;

pub const CHROMIUM_CDP_MOUSE_BACKEND_ID: BackendId =
    BackendId::new("pwright.backend.chromium-cdp.mouse");
pub const CHROMIUM_CDP_MOUSE_FEATURES: &[BackendFeatureDescriptor] = &[
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("mouse.move"),
        MOUSE_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        MOUSE_MOVE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("mouse.down"),
        MOUSE_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        MOUSE_DOWN_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("mouse.up"),
        MOUSE_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        MOUSE_UP_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("mouse.click"),
        MOUSE_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        MOUSE_CLICK_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("mouse.dblclick"),
        MOUSE_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        MOUSE_DBLCLICK_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("mouse.wheel"),
        MOUSE_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        MOUSE_WHEEL_COMMAND_ID,
        BackendImplementation::Native,
    ),
];
pub enum ChromiumCdpMouseBackend {}
impl BrowserBackend for ChromiumCdpMouseBackend {
    const DESCRIPTOR: BackendDescriptor = BackendDescriptor::new(
        CHROMIUM_CDP_MOUSE_BACKEND_ID,
        "Chromium CDP Mouse",
        BrowserEngine::Chromium,
        BackendProtocolId::new("cdp"),
        CHROMIUM_CDP_MOUSE_FEATURES,
    );
}
