use pwright_browser_backend::{
    BackendDescriptor, BackendFeatureDescriptor, BackendFeatureId, BackendId,
    BackendImplementation, BackendProtocolId, BrowserBackend, BrowserEngine,
};
use pwright_cap_keyboard::*;
use pwright_core::ProtocolVersion;

pub const CHROMIUM_CDP_KEYBOARD_BACKEND_ID: BackendId =
    BackendId::new("pwright.backend.chromium-cdp.keyboard");
pub const CHROMIUM_CDP_KEYBOARD_FEATURES: &[BackendFeatureDescriptor] = &[
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("keyboard.down"),
        KEYBOARD_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        KEYBOARD_DOWN_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("keyboard.up"),
        KEYBOARD_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        KEYBOARD_UP_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("keyboard.press"),
        KEYBOARD_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        KEYBOARD_PRESS_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("keyboard.insert-text"),
        KEYBOARD_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        KEYBOARD_INSERT_TEXT_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        BackendFeatureId::new("keyboard.type"),
        KEYBOARD_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        KEYBOARD_TYPE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
];
pub enum ChromiumCdpKeyboardBackend {}
impl BrowserBackend for ChromiumCdpKeyboardBackend {
    const DESCRIPTOR: BackendDescriptor = BackendDescriptor::new(
        CHROMIUM_CDP_KEYBOARD_BACKEND_ID,
        "Chromium CDP Keyboard",
        BrowserEngine::Chromium,
        BackendProtocolId::new("cdp"),
        CHROMIUM_CDP_KEYBOARD_FEATURES,
    );
}
