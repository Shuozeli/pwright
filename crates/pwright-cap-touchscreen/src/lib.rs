//! Optional backend-neutral Page Touchscreen contracts; no browser or input state.

pub use pwright_cap_locator::{
    KeyboardModifier, KeyboardModifiers, Position, ScrollMode, Selector,
};
use pwright_core::{
    Capability, CapabilityCommandDescriptor, CapabilityCommandSpec, CapabilityDescriptor,
    CapabilityId, CommandId, CommandSpec, MethodTag, ObjectKindId, PluginInfo, ProtocolVersion,
};
use std::fmt;

pub type TouchscreenSuccess = ();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchscreenArgumentError {
    NonFinite,
}

impl fmt::Display for TouchscreenArgumentError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => formatter.write_str("touchscreen coordinates must be finite"),
        }
    }
}

impl std::error::Error for TouchscreenArgumentError {}

/// One finite CSS-pixel coordinate. Fractional and negative values are
/// preserved; this value does not imply viewport clamping or target lookup.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TouchscreenCoordinate(f64);

impl TryFrom<f64> for TouchscreenCoordinate {
    type Error = TouchscreenArgumentError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(TouchscreenArgumentError::NonFinite);
        }
        Ok(Self(value))
    }
}

impl TouchscreenCoordinate {
    pub const fn get(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TapParams {
    pub x: TouchscreenCoordinate,
    pub y: TouchscreenCoordinate,
}

impl TapParams {
    pub const fn new(x: TouchscreenCoordinate, y: TouchscreenCoordinate) -> Self {
        Self { x, y }
    }
}

/// Playwright-shaped selector tap options. Timeout is carried by the caller's
/// operation boundary so it cannot diverge from cancellation ownership.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SelectorTapOptions {
    pub force: Option<bool>,
    pub modifiers: Option<KeyboardModifiers>,
    pub no_wait_after: Option<bool>,
    pub position: Option<Position>,
    pub scroll: Option<ScrollMode>,
    pub trial: Option<bool>,
}

impl SelectorTapOptions {
    pub const fn new() -> Self {
        Self {
            force: None,
            modifiers: None,
            no_wait_after: None,
            position: None,
            scroll: None,
            trial: None,
        }
    }

    pub const fn with_force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    pub const fn with_modifiers(mut self, modifiers: KeyboardModifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    pub const fn with_position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    pub const fn with_trial(mut self, trial: bool) -> Self {
        self.trial = Some(trial);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SelectorTapParams {
    pub selector: Selector,
    pub options: SelectorTapOptions,
}

impl SelectorTapParams {
    pub const fn new(selector: Selector, options: SelectorTapOptions) -> Self {
        Self { selector, options }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TouchscreenErrorDetails {
    InvalidArgument(String),
    Resource(String),
    Backend(String),
}

pub const TOUCHSCREEN_CAPABILITY_ID: CapabilityId = CapabilityId::new("pwright.touchscreen");
pub const TOUCHSCREEN_TAP_COMMAND_ID: CommandId = CommandId::new("touchscreen.tap");
pub const TOUCHSCREEN_TAP_METHOD: MethodTag = MethodTag::new(917511);
pub const TOUCHSCREEN_TAP_TARGET_OBJECT_KINDS: &[ObjectKindId] =
    &[ObjectKindId::new("pwright.browser.page")];
pub const TOUCHSCREEN_COMMAND_DESCRIPTORS: &[CapabilityCommandDescriptor] =
    &[CapabilityCommandDescriptor::new(
        TOUCHSCREEN_TAP_COMMAND_ID,
        TOUCHSCREEN_TAP_METHOD,
        TOUCHSCREEN_TAP_TARGET_OBJECT_KINDS,
    )];

pub struct TouchscreenCapability;

impl Capability for TouchscreenCapability {
    const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new(
        PluginInfo::new(
            TOUCHSCREEN_CAPABILITY_ID,
            "Touchscreen",
            ProtocolVersion::new(1, 0),
        ),
        &[],
        &[TOUCHSCREEN_TAP_COMMAND_ID],
    );
}

pub struct TouchscreenTap;

impl CommandSpec for TouchscreenTap {
    const METHOD: MethodTag = TOUCHSCREEN_TAP_METHOD;
    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = TOUCHSCREEN_TAP_TARGET_OBJECT_KINDS;
    type Params = TapParams;
    type Result = TouchscreenSuccess;
    type ErrorDetails = TouchscreenErrorDetails;
}

impl CapabilityCommandSpec for TouchscreenTap {
    type Capability = TouchscreenCapability;
    const COMMAND: CommandId = TOUCHSCREEN_TAP_COMMAND_ID;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_accepts_finite_fractional_and_negative_values() {
        // Arrange
        let values = [-12.5, 0.0, 42.25];

        // Act
        let coordinates = values.map(TouchscreenCoordinate::try_from);

        // Assert
        assert_eq!(coordinates[0].unwrap().get(), -12.5);
        assert_eq!(coordinates[1].unwrap().get(), 0.0);
        assert_eq!(coordinates[2].unwrap().get(), 42.25);
    }

    #[test]
    fn coordinate_rejects_every_non_finite_value() {
        // Arrange
        let values = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

        // Act
        let errors = values.map(TouchscreenCoordinate::try_from);

        // Assert
        assert!(
            errors
                .into_iter()
                .all(|result| { result == Err(TouchscreenArgumentError::NonFinite) })
        );
    }

    #[test]
    fn descriptor_is_a_single_page_scoped_tap_command() {
        // Arrange
        let descriptor = TOUCHSCREEN_COMMAND_DESCRIPTORS[0];

        // Act
        let target = descriptor.targets()[0];

        // Assert
        assert_eq!(TOUCHSCREEN_COMMAND_DESCRIPTORS.len(), 1);
        assert_eq!(descriptor.command(), TOUCHSCREEN_TAP_COMMAND_ID);
        assert_eq!(descriptor.method(), TOUCHSCREEN_TAP_METHOD);
        assert_eq!(target, ObjectKindId::new("pwright.browser.page"));
    }

    #[test]
    fn selector_tap_preserves_optional_action_fields() {
        // Arrange
        let position = Position::new(12.5, 8.25).unwrap();
        let modifiers = KeyboardModifiers::new().with(KeyboardModifier::Alt);

        // Act
        let params = SelectorTapParams::new(
            Selector::test_id("tap-target"),
            SelectorTapOptions::new()
                .with_force(true)
                .with_modifiers(modifiers)
                .with_position(position)
                .with_trial(true),
        );

        // Assert
        assert_eq!(params.options.force, Some(true));
        assert_eq!(params.options.modifiers, Some(modifiers));
        assert_eq!(params.options.position, Some(position));
        assert_eq!(params.options.trial, Some(true));
    }
}
