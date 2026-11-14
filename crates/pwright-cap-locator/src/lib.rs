//! Portable selector values and locator-action contracts.
//!
//! Locators remain lazy client-side values. A command carries the selector to
//! the backend and resolves it again for every action attempt; no DOM node or
//! browser-protocol identity crosses this boundary.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU8;
use std::num::NonZeroU32;
use std::str::FromStr;
use std::time::Duration;

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability_v2;

/// Locator 2.x command identities and versioned action values.
///
/// The module keeps the logical `pwright.locator` capability ID and stable
/// command/method identities while selecting the 2.x wire schema during the
/// connection handshake.
pub mod v2 {
    pub use super::generated_capability_v2::*;
    pub use super::{ClickOptionsV2 as ClickOptions, ClickParamsV2 as ClickParams};
}

#[cfg(feature = "select-option-timeout-progress")]
mod select_option_progress;
#[cfg(feature = "select-option-timeout-progress")]
pub use select_option_progress::SelectOptionProgress;

pub use pwright_input_model::{INPUT_TEXT_MAX_BYTES, InputText};
pub use pwright_selector_model::{
    AriaRole, AriaRoleParseError, AriaTriState, CssSelector, CssSelectorError, LocatorErrorDetails,
    MAX_TEST_ID_ATTRIBUTES, RegexMatcher, RegexMatcherError, RoleOptions, RolePatternSelector,
    RoleSelector, RoleSelectorError, Selector, TestIdAttributes, TestIdAttributesError,
    TextMatcher, UserFacingAttribute,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
}

/// Keyboard modifiers accepted by Playwright-shaped pointer actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardModifier {
    Alt,
    Control,
    ControlOrMeta,
    Meta,
    Shift,
}

impl KeyboardModifier {
    const fn bit(self) -> u8 {
        1 << self as u8
    }
}

/// Allocation-free, canonical modifier set carried by pointer actions.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct KeyboardModifiers(u8);

impl KeyboardModifiers {
    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn with(mut self, modifier: KeyboardModifier) -> Self {
        self.0 |= modifier.bit();
        self
    }

    pub const fn contains(self, modifier: KeyboardModifier) -> bool {
        self.0 & modifier.bit() != 0
    }

    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn iter(self) -> impl Iterator<Item = KeyboardModifier> {
        [
            KeyboardModifier::Alt,
            KeyboardModifier::Control,
            KeyboardModifier::ControlOrMeta,
            KeyboardModifier::Meta,
            KeyboardModifier::Shift,
        ]
        .into_iter()
        .filter(move |modifier| self.contains(*modifier))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoordinateAxis {
    X,
    Y,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PositionError {
    axis: CoordinateAxis,
}

impl PositionError {
    pub const fn axis(self) -> CoordinateAxis {
        self.axis
    }
}

impl Display for PositionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "pointer position {:?} must be finite", self.axis)
    }
}

impl Error for PositionError {}

/// Point relative to the top-left corner of the target's padding box.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Position {
    x: f64,
    y: f64,
}

impl Position {
    pub fn new(x: f64, y: f64) -> Result<Self, PositionError> {
        if !x.is_finite() {
            return Err(PositionError {
                axis: CoordinateAxis::X,
            });
        }
        if !y.is_finite() {
            return Err(PositionError {
                axis: CoordinateAxis::Y,
            });
        }
        Ok(Self { x, y })
    }

    pub const fn x(self) -> f64 {
        self.x
    }

    pub const fn y(self) -> f64 {
        self.y
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ScrollMode {
    #[default]
    Auto,
    None,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HoverOptions {
    pub force: bool,
    pub scroll: ScrollMode,
    pub modifiers: Option<KeyboardModifiers>,
    pub position: Option<Position>,
    pub trial: bool,
}

impl HoverOptions {
    pub const fn new() -> Self {
        Self {
            force: false,
            scroll: ScrollMode::Auto,
            modifiers: None,
            position: None,
            trial: false,
        }
    }

    pub const fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub const fn with_scroll(mut self, scroll: ScrollMode) -> Self {
        self.scroll = scroll;
        self
    }

    pub const fn with_modifier(mut self, modifier: KeyboardModifier) -> Self {
        let modifiers = match self.modifiers {
            Some(modifiers) => modifiers,
            None => KeyboardModifiers::new(),
        };
        self.modifiers = Some(modifiers.with(modifier));
        self
    }

    /// Ensures exactly this modifier set for the hover and restores the prior
    /// Page keyboard state afterward. Passing an empty set differs from leaving
    /// the option unspecified, which preserves currently pressed modifiers.
    pub const fn with_modifiers(mut self, modifiers: KeyboardModifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    pub const fn with_position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    pub const fn with_trial(mut self, trial: bool) -> Self {
        self.trial = trial;
        self
    }
}

impl Default for HoverOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HoverParams {
    pub selector: Selector,
    pub options: HoverOptions,
}

impl HoverParams {
    pub const fn new(selector: Selector, options: HoverOptions) -> Self {
        Self { selector, options }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ClickOptions {
    pub button: MouseButton,
    pub click_count: NonZeroU8,
    pub force: bool,
    pub trial: bool,
}

impl ClickOptions {
    pub const fn new() -> Self {
        Self {
            button: MouseButton::Left,
            click_count: NonZeroU8::MIN,
            force: false,
            trial: false,
        }
    }

    pub const fn with_button(mut self, button: MouseButton) -> Self {
        self.button = button;
        self
    }

    pub const fn with_click_count(mut self, click_count: NonZeroU8) -> Self {
        self.click_count = click_count;
        self
    }

    pub const fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    pub const fn with_trial(mut self, trial: bool) -> Self {
        self.trial = trial;
        self
    }
}

impl Default for ClickOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClickParams {
    pub selector: Selector,
    pub options: ClickOptions,
}

/// Playwright-aligned Click options for locator protocol 2.x.
///
/// Every field remains optional on the wire so absence preserves Playwright's
/// current defaults and can be distinguished from an explicit `false`, zero
/// delay, or empty modifier set. Timeout and cancellation are carried by
/// `CallMetadata`, not duplicated here.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ClickOptionsV2 {
    pub button: Option<MouseButton>,
    pub click_count: Option<NonZeroU8>,
    pub delay: Option<Duration>,
    pub force: Option<bool>,
    pub modifiers: Option<KeyboardModifiers>,
    pub no_wait_after: Option<bool>,
    pub position: Option<Position>,
    pub scroll: Option<ScrollMode>,
    pub steps: Option<NonZeroU32>,
    pub trial: Option<bool>,
}

impl ClickOptionsV2 {
    pub const fn new() -> Self {
        Self {
            button: None,
            click_count: None,
            delay: None,
            force: None,
            modifiers: None,
            no_wait_after: None,
            position: None,
            scroll: None,
            steps: None,
            trial: None,
        }
    }

    pub const fn with_button(mut self, button: MouseButton) -> Self {
        self.button = Some(button);
        self
    }

    pub const fn with_click_count(mut self, click_count: NonZeroU8) -> Self {
        self.click_count = Some(click_count);
        self
    }

    pub const fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    pub const fn with_force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    pub const fn with_modifier(mut self, modifier: KeyboardModifier) -> Self {
        let modifiers = match self.modifiers {
            Some(modifiers) => modifiers,
            None => KeyboardModifiers::new(),
        };
        self.modifiers = Some(modifiers.with(modifier));
        self
    }

    pub const fn with_modifiers(mut self, modifiers: KeyboardModifiers) -> Self {
        self.modifiers = Some(modifiers);
        self
    }

    pub const fn with_no_wait_after(mut self, no_wait_after: bool) -> Self {
        self.no_wait_after = Some(no_wait_after);
        self
    }

    pub const fn with_position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    pub const fn with_scroll(mut self, scroll: ScrollMode) -> Self {
        self.scroll = Some(scroll);
        self
    }

    pub const fn with_steps(mut self, steps: NonZeroU32) -> Self {
        self.steps = Some(steps);
        self
    }

    pub const fn with_trial(mut self, trial: bool) -> Self {
        self.trial = Some(trial);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClickParamsV2 {
    pub selector: Selector,
    pub options: ClickOptionsV2,
}

impl ClickParamsV2 {
    pub const fn new(selector: Selector, options: ClickOptionsV2) -> Self {
        Self { selector, options }
    }
}

macro_rules! locator_focus_params {
    ($params:ident, $method:literal) => {
        #[doc = concat!("Parameters for Playwright-shaped `Locator.", $method, "()`.")]
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $params {
            pub selector: Selector,
        }

        impl $params {
            pub fn new(selector: impl Into<Selector>) -> Self {
                Self {
                    selector: selector.into(),
                }
            }
        }
    };
}

locator_focus_params!(BlurParams, "blur");
locator_focus_params!(FocusParams, "focus");

pub const PRESS_KEY_MAX_BYTES: usize = 128;
pub const PRESS_KEY_MAX_TOKENS: usize = pwright_input_model::INPUT_CHORD_MAX_KEYS;

/// A bounded Playwright key expression such as `Enter`, `ControlOrMeta+A`, or
/// `Shift++`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PressKey(String);

impl PressKey {
    pub fn new(value: impl Into<String>) -> Result<Self, PressKeyError> {
        let value = value.into();
        let bytes = value.len();
        if bytes > PRESS_KEY_MAX_BYTES {
            return Err(PressKeyError::TooLong {
                bytes,
                maximum: PRESS_KEY_MAX_BYTES,
            });
        }

        let mut tokens = 1_usize;
        let mut building = false;
        for character in value.chars() {
            if character == '+' && building {
                tokens = tokens.saturating_add(1);
                if tokens > PRESS_KEY_MAX_TOKENS {
                    return Err(PressKeyError::TooManyTokens {
                        tokens,
                        maximum: PRESS_KEY_MAX_TOKENS,
                    });
                }
                building = false;
            } else {
                building = true;
            }
        }
        if !building {
            return Err(PressKeyError::EmptyToken {
                token: tokens.saturating_sub(1),
            });
        }

        let mut value = value;
        value.shrink_to_fit();
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn token_count(&self) -> usize {
        let mut tokens = 1_usize;
        let mut building = false;
        for character in self.0.chars() {
            if character == '+' && building {
                tokens += 1;
                building = false;
            } else {
                building = true;
            }
        }
        tokens
    }
}

impl Display for PressKey {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for PressKey {
    type Err = PressKeyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Self::new(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PressKeyError {
    TooLong { bytes: usize, maximum: usize },
    TooManyTokens { tokens: usize, maximum: usize },
    EmptyToken { token: usize },
}

impl Display for PressKeyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLong { bytes, maximum } => write!(
                formatter,
                "key expression uses {bytes} UTF-8 bytes, exceeding the {maximum}-byte limit"
            ),
            Self::TooManyTokens { tokens, maximum } => write!(
                formatter,
                "key expression uses {tokens} tokens, exceeding the {maximum}-token limit"
            ),
            Self::EmptyToken { token } => {
                write!(formatter, "key expression contains empty token {token}")
            }
        }
    }
}

impl Error for PressKeyError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PressOptions {
    pub delay: Option<Duration>,
    pub no_wait_after: bool,
}

impl PressOptions {
    pub const fn new() -> Self {
        Self {
            delay: None,
            no_wait_after: false,
        }
    }

    pub const fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = Some(delay);
        self
    }

    pub const fn with_no_wait_after(mut self, no_wait_after: bool) -> Self {
        self.no_wait_after = no_wait_after;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PressParams {
    pub selector: Selector,
    pub key: PressKey,
    pub options: PressOptions,
}

impl PressParams {
    pub fn new(selector: impl Into<Selector>, key: PressKey, options: PressOptions) -> Self {
        Self {
            selector: selector.into(),
            key,
            options,
        }
    }
}

/// Strict focus followed by literal sequential typing. The overall timeout
/// remains in call metadata; typing does not wait for navigation signals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PressSequentiallyParams {
    pub selector: Selector,
    pub text: String,
    pub delay: Option<Duration>,
}

impl PressSequentiallyParams {
    pub fn new(
        selector: impl Into<Selector>,
        text: impl Into<String>,
        delay: Option<Duration>,
    ) -> Self {
        Self {
            selector: selector.into(),
            text: text.into(),
            delay,
        }
    }
}

/// Strict lazy Locator request for Playwright's scroll-if-needed action.
///
/// Timeout and cancellation remain in [`CallMetadata`](pwright_core::CallMetadata),
/// so the capability does not need an otherwise empty options record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScrollIntoViewIfNeededParams {
    pub selector: Selector,
}

impl ScrollIntoViewIfNeededParams {
    pub fn new(selector: impl Into<Selector>) -> Self {
        Self {
            selector: selector.into(),
        }
    }
}

/// One Playwright-shaped `<option>` matching descriptor.
///
/// Every populated field is matched conjunctively. `value_or_label` is the
/// representation used by the string convenience overload and matches either
/// an exact value or an exact/whitespace-normalized label. An empty descriptor
/// is valid and matches the first option, just as Playwright does.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectOptionValue {
    pub value_or_label: Option<String>,
    pub value: Option<String>,
    pub label: Option<String>,
    pub index: Option<i32>,
}

impl SelectOptionValue {
    pub const fn new() -> Self {
        Self {
            value_or_label: None,
            value: None,
            label: None,
            index: None,
        }
    }

    pub fn value_or_label(value: impl Into<String>) -> Self {
        Self::new().with_value_or_label(value)
    }

    pub fn with_value_or_label(mut self, value: impl Into<String>) -> Self {
        self.value_or_label = Some(value.into());
        self
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub const fn with_index(mut self, index: i32) -> Self {
        self.index = Some(index);
        self
    }
}

impl From<String> for SelectOptionValue {
    fn from(value: String) -> Self {
        Self::value_or_label(value)
    }
}

impl From<&str> for SelectOptionValue {
    fn from(value: &str) -> Self {
        Self::value_or_label(value)
    }
}

/// Requested options for `Locator.select_option()`.
///
/// The empty value is intentional and means deselect all. The wrapper lets the
/// typed client accept one string, one descriptor, or an explicit list without
/// adding a second protocol command.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SelectOptionValues(Vec<SelectOptionValue>);

impl SelectOptionValues {
    pub const fn new() -> Self {
        Self(Vec::new())
    }

    pub fn from_values(values: impl IntoIterator<Item = SelectOptionValue>) -> Self {
        Self(values.into_iter().collect())
    }

    pub fn as_slice(&self) -> &[SelectOptionValue] {
        &self.0
    }

    pub fn into_inner(self) -> Vec<SelectOptionValue> {
        self.0
    }
}

impl From<SelectOptionValue> for SelectOptionValues {
    fn from(value: SelectOptionValue) -> Self {
        Self(vec![value])
    }
}

impl From<Vec<SelectOptionValue>> for SelectOptionValues {
    fn from(values: Vec<SelectOptionValue>) -> Self {
        Self(values)
    }
}

impl From<String> for SelectOptionValues {
    fn from(value: String) -> Self {
        Self(vec![SelectOptionValue::from(value)])
    }
}

impl From<&str> for SelectOptionValues {
    fn from(value: &str) -> Self {
        Self(vec![SelectOptionValue::from(value)])
    }
}

impl<const N: usize> From<[SelectOptionValue; N]> for SelectOptionValues {
    fn from(values: [SelectOptionValue; N]) -> Self {
        Self(Vec::from(values))
    }
}

impl<const N: usize> From<[&str; N]> for SelectOptionValues {
    fn from(values: [&str; N]) -> Self {
        Self(values.into_iter().map(SelectOptionValue::from).collect())
    }
}

/// Playwright-shaped options for selecting native `<option>` elements.
///
/// Force skips visibility and enabled checks on the `<select>` itself. It does
/// not permit selecting a disabled option. Timeout and cancellation remain in
/// [`CallMetadata`](pwright_core::CallMetadata).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectOptionOptions {
    pub force: Option<bool>,
}

impl SelectOptionOptions {
    pub const fn new() -> Self {
        Self { force: None }
    }

    pub const fn with_force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }
}

#[derive(Clone, Debug)]
pub struct SelectOptionParams {
    pub selector: Selector,
    pub values: Vec<SelectOptionValue>,
    pub options: SelectOptionOptions,
    #[cfg(feature = "select-option-timeout-progress")]
    progress: Option<SelectOptionProgress>,
}

impl PartialEq for SelectOptionParams {
    fn eq(&self, other: &Self) -> bool {
        self.selector == other.selector
            && self.values == other.values
            && self.options == other.options
    }
}

impl Eq for SelectOptionParams {}

impl SelectOptionParams {
    pub fn new(
        selector: impl Into<Selector>,
        values: impl Into<SelectOptionValues>,
        options: SelectOptionOptions,
    ) -> Self {
        Self {
            selector: selector.into(),
            values: values.into().into_inner(),
            options,
            #[cfg(feature = "select-option-timeout-progress")]
            progress: None,
        }
    }

    /// Attach a one-call, non-wire diagnostic observer. The default request
    /// never allocates one, and it never changes selector or value equality.
    #[cfg(feature = "select-option-timeout-progress")]
    #[doc(hidden)]
    pub fn with_progress(mut self, progress: SelectOptionProgress) -> Self {
        self.progress = Some(progress);
        self
    }

    #[cfg(feature = "select-option-timeout-progress")]
    #[doc(hidden)]
    pub fn progress(&self) -> Option<&SelectOptionProgress> {
        self.progress.as_ref()
    }
}

/// Selected option values in DOM order.
pub type SelectOptionResult = Vec<String>;

/// Playwright-shaped options for changing the checked state of a checkbox or
/// radio control.
///
/// Presence is preserved independently for every field because the new wire
/// route uses a bounded extensible record. Timeout and cancellation remain in
/// [`CallMetadata`](pwright_core::CallMetadata).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SetCheckedOptions {
    pub force: Option<bool>,
    pub position: Option<Position>,
    pub scroll: Option<ScrollMode>,
    pub trial: Option<bool>,
}

impl SetCheckedOptions {
    pub const fn new() -> Self {
        Self {
            force: None,
            position: None,
            scroll: None,
            trial: None,
        }
    }

    pub const fn with_force(mut self, force: bool) -> Self {
        self.force = Some(force);
        self
    }

    pub const fn with_position(mut self, position: Position) -> Self {
        self.position = Some(position);
        self
    }

    pub const fn with_scroll(mut self, scroll: ScrollMode) -> Self {
        self.scroll = Some(scroll);
        self
    }

    pub const fn with_trial(mut self, trial: bool) -> Self {
        self.trial = Some(trial);
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SetCheckedParams {
    pub selector: Selector,
    pub checked: bool,
    pub options: SetCheckedOptions,
}

impl SetCheckedParams {
    pub fn new(selector: impl Into<Selector>, checked: bool, options: SetCheckedOptions) -> Self {
        Self {
            selector: selector.into(),
            checked,
            options,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CountParams {
    pub selector: Selector,
}

impl CountParams {
    pub fn new(selector: impl Into<Selector>) -> Self {
        Self {
            selector: selector.into(),
        }
    }
}

/// Number of elements matching the Locator at one instant.
///
/// This operation is deliberately non-strict and does not auto-wait for a
/// match, mirroring Playwright's `Locator.count()` snapshot behavior.
pub type CountResult = u32;

/// Parameters for Playwright-shaped `Locator.allTextContents()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllTextContentsParams {
    pub selector: Selector,
}

impl AllTextContentsParams {
    pub fn new(selector: impl Into<Selector>) -> Self {
        Self {
            selector: selector.into(),
        }
    }
}

/// Exact DOM `Node.textContent` values for every current Locator match.
///
/// This is an immediate, non-strict snapshot: zero matches return an empty
/// vector, DOM `null` becomes an empty string, and results retain selector
/// order without exposing element handles. The protocol accepts at most 4096
/// values, one MiB per UTF-8 value, and four MiB in aggregate.
pub type AllTextContentsResult = Vec<String>;

/// Parameters for Playwright-shaped `Locator.allInnerTexts()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllInnerTextsParams {
    pub selector: Selector,
}

impl AllInnerTextsParams {
    pub fn new(selector: impl Into<Selector>) -> Self {
        Self {
            selector: selector.into(),
        }
    }
}

/// Rendered `HTMLElement.innerText` values for every current Locator match.
///
/// This is an immediate, non-strict snapshot: zero matches return an empty
/// vector and results retain selector order without exposing element handles.
/// Unlike JavaScript's unchecked TypeScript cast, the Rust contract rejects a
/// non-HTMLElement match so every successful item is truly a string. The
/// protocol accepts at most 4096 values, one MiB per UTF-8 value, and four MiB
/// in aggregate.
pub type AllInnerTextsResult = Vec<String>;

macro_rules! locator_state_query {
    ($params:ident, $result:ident, $field:literal, $method:literal) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $params {
            pub selector: Selector,
        }

        impl $params {
            pub fn new(selector: impl Into<Selector>) -> Self {
                Self {
                    selector: selector.into(),
                }
            }
        }

        #[doc = concat!("Result of Playwright-shaped `Locator.", $method, "()`.")]
        #[doc = ""]
        #[doc = concat!("The operation waits for one strict target, then reads whether it is ", $field, " without waiting for that state to change.")]
        pub type $result = bool;
    };
}

locator_state_query!(IsCheckedParams, IsCheckedResult, "checked", "isChecked");
locator_state_query!(IsEditableParams, IsEditableResult, "editable", "isEditable");
locator_state_query!(IsEnabledParams, IsEnabledResult, "enabled", "isEnabled");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IsVisibleParams {
    pub selector: Selector,
}

impl IsVisibleParams {
    pub fn new(selector: impl Into<Selector>) -> Self {
        Self {
            selector: selector.into(),
        }
    }
}

/// Whether the strict Locator target is visible at one instant.
///
/// A missing target returns `false`; multiple targets remain a strictness
/// error. This query does not auto-wait, matching Playwright's `isVisible()`.
pub type IsVisibleResult = bool;

/// Parameters for Playwright-shaped `Locator.getAttribute()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetAttributeParams {
    pub selector: Selector,
    pub name: String,
}

impl GetAttributeParams {
    pub fn new(selector: impl Into<Selector>, name: impl Into<String>) -> Self {
        Self {
            selector: selector.into(),
            name: name.into(),
        }
    }
}

/// Exact `Element.getAttribute(name)` result for one strict Locator target.
///
/// The query waits for an element, re-resolves it on retry, preserves a
/// missing attribute as `None`, and introduces no DOM handle. Attribute names
/// are bounded to 64 KiB and present UTF-8 values to 1 MiB on the wire.
pub type GetAttributeResult = Option<String>;

/// Parameters for Playwright-shaped `Locator.textContent()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextContentParams {
    pub selector: Selector,
}

impl TextContentParams {
    pub fn new(selector: impl Into<Selector>) -> Self {
        Self {
            selector: selector.into(),
        }
    }
}

/// Exact DOM `Node.textContent` for one strict Locator target.
///
/// The query waits for a target, re-resolves it on retry, preserves `null`, and
/// performs no whitespace normalization. The wire representation bounds a
/// present UTF-8 value to 1 MiB.
pub type TextContentResult = Option<String>;

/// Parameters for Playwright-shaped `Locator.innerHTML()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InnerHtmlParams {
    pub selector: Selector,
}

impl InnerHtmlParams {
    pub fn new(selector: impl Into<Selector>) -> Self {
        Self {
            selector: selector.into(),
        }
    }
}

/// Serialized child markup from `Element.innerHTML` for one strict target.
///
/// The query waits for an element, re-resolves it on retry, and returns the
/// browser-serialized markup without introducing a DOM handle. The wire
/// representation bounds the UTF-8 value to 1 MiB.
pub type InnerHtmlResult = String;

/// Parameters for Playwright-shaped `Locator.innerText()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InnerTextParams {
    pub selector: Selector,
}

impl InnerTextParams {
    pub fn new(selector: impl Into<Selector>) -> Self {
        Self {
            selector: selector.into(),
        }
    }
}

/// Rendered `HTMLElement.innerText` for one strict Locator target.
///
/// The query waits for an HTML element, re-resolves it on retry, and returns
/// the browser-computed rendered text. The wire representation bounds the
/// UTF-8 value to 1 MiB.
pub type InnerTextResult = String;

/// Parameters for Playwright-shaped `Locator.inputValue()`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputValueParams {
    pub selector: Selector,
}

impl InputValueParams {
    pub fn new(selector: impl Into<Selector>) -> Self {
        Self {
            selector: selector.into(),
        }
    }
}

/// Current value of a strict input, textarea, or select target.
///
/// The query waits and re-resolves, follows an associated label to its control,
/// rejects every other target type, and returns at most 1 MiB of UTF-8 without
/// creating an ElementHandle.
pub type InputValueResult = String;

/// Playwright-compatible states accepted by [`WaitForOptions`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WaitForState {
    Attached,
    Detached,
    #[default]
    Visible,
    Hidden,
}

/// Options for waiting until a Locator reaches one observable DOM state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaitForOptions {
    pub state: WaitForState,
}

impl WaitForOptions {
    pub const fn new() -> Self {
        Self {
            state: WaitForState::Visible,
        }
    }

    pub const fn with_state(mut self, state: WaitForState) -> Self {
        self.state = state;
        self
    }
}

impl Default for WaitForOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Parameters for `Locator.waitFor()`.
///
/// The result is `()` even for attached or visible states. Waiting never
/// creates a retained DOM handle in the portable object graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WaitForParams {
    pub selector: Selector,
    pub options: WaitForOptions,
}

impl WaitForParams {
    pub fn new(selector: impl Into<Selector>, options: WaitForOptions) -> Self {
        Self {
            selector: selector.into(),
            options,
        }
    }
}

impl ClickParams {
    pub fn new(selector: impl Into<Selector>, options: ClickOptions) -> Self {
        Self {
            selector: selector.into(),
            options,
        }
    }
}

pub type LocatorActionResult = ();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FillOptions {
    pub force: bool,
}

impl FillOptions {
    pub const fn new() -> Self {
        Self { force: false }
    }

    pub const fn with_force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }
}

impl Default for FillOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FillParams {
    pub selector: Selector,
    pub value: String,
    pub options: FillOptions,
}

impl FillParams {
    pub fn new(
        selector: impl Into<Selector>,
        value: impl Into<String>,
        options: FillOptions,
    ) -> Self {
        Self {
            selector: selector.into(),
            value: value.into(),
            options,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU8, NonZeroU32};
    use std::time::Duration;

    use pwright_core::{Capability, CapabilityCommandSpec, CommandSpec, RegistryBuilder};

    use super::{
        ALL_INNER_TEXTS_COMMAND_ID, ALL_INNER_TEXTS_METHOD, ALL_TEXT_CONTENTS_COMMAND_ID,
        ALL_TEXT_CONTENTS_METHOD, AllInnerTexts, AllInnerTextsParams, AllTextContents,
        AllTextContentsParams, AriaRole, AriaTriState, BLUR_COMMAND_ID, BLUR_METHOD, Blur,
        CLICK_COMMAND_ID, CLICK_METHOD, COUNT_COMMAND_ID, COUNT_METHOD, Click, ClickOptions, Count,
        CountParams, CssSelector, FILL_COMMAND_ID, FILL_METHOD, FOCUS_COMMAND_ID, FOCUS_METHOD,
        Fill, FillOptions, FillParams, Focus, GET_ATTRIBUTE_COMMAND_ID, GET_ATTRIBUTE_METHOD,
        GetAttribute, GetAttributeParams, GetAttributeResult, HOVER_COMMAND_ID, HOVER_METHOD,
        Hover, HoverOptions, HoverParams, INNER_HTML_COMMAND_ID, INNER_HTML_METHOD,
        INNER_TEXT_COMMAND_ID, INNER_TEXT_METHOD, INPUT_VALUE_COMMAND_ID, INPUT_VALUE_METHOD,
        IS_CHECKED_COMMAND_ID, IS_CHECKED_METHOD, IS_EDITABLE_COMMAND_ID, IS_EDITABLE_METHOD,
        IS_ENABLED_COMMAND_ID, IS_ENABLED_METHOD, IS_VISIBLE_COMMAND_ID, IS_VISIBLE_METHOD,
        InnerHtml, InnerHtmlParams, InnerHtmlResult, InnerText, InnerTextParams, InnerTextResult,
        InputValue, InputValueParams, InputValueResult, IsChecked, IsCheckedParams, IsEditable,
        IsEditableParams, IsEnabled, IsEnabledParams, IsVisible, IsVisibleParams, KeyboardModifier,
        KeyboardModifiers, LOCATOR_CAPABILITY_ID, LOCATOR_COMMAND_DESCRIPTORS, LocatorCapability,
        MAX_TEST_ID_ATTRIBUTES, MouseButton, PRESS_COMMAND_ID, PRESS_KEY_MAX_BYTES,
        PRESS_KEY_MAX_TOKENS, PRESS_METHOD, Position, Press, PressKey, PressKeyError, RegexMatcher,
        RegexMatcherError, RoleOptions, SCROLL_INTO_VIEW_IF_NEEDED_COMMAND_ID,
        SCROLL_INTO_VIEW_IF_NEEDED_METHOD, SELECT_OPTION_COMMAND_ID, SELECT_OPTION_METHOD,
        SET_CHECKED_COMMAND_ID, SET_CHECKED_METHOD, ScrollIntoViewIfNeeded,
        ScrollIntoViewIfNeededParams, ScrollMode, SelectOption, SelectOptionOptions,
        SelectOptionParams, SelectOptionValue, SelectOptionValues, Selector, SetChecked,
        SetCheckedOptions, SetCheckedParams, TEXT_CONTENT_COMMAND_ID, TEXT_CONTENT_METHOD,
        TestIdAttributes, TestIdAttributesError, TextContent, TextContentParams, TextContentResult,
        UserFacingAttribute, WAIT_FOR_COMMAND_ID, WAIT_FOR_METHOD, WaitFor, WaitForOptions,
        WaitForParams, WaitForState,
    };

    #[test]
    fn capability_owns_locator_action_commands() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<LocatorCapability>()
            .expect("locator capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert_eq!(
            registry.owner_of(CLICK_COMMAND_ID),
            Some(LOCATOR_CAPABILITY_ID)
        );
        assert_eq!(
            registry.owner_of(ALL_TEXT_CONTENTS_COMMAND_ID),
            Some(LOCATOR_CAPABILITY_ID)
        );
        assert_eq!(
            registry.owner_of(ALL_INNER_TEXTS_COMMAND_ID),
            Some(LOCATOR_CAPABILITY_ID)
        );
        assert_eq!(
            registry.owner_of(FILL_COMMAND_ID),
            Some(LOCATOR_CAPABILITY_ID)
        );
        assert_eq!(
            registry.owner_of(HOVER_COMMAND_ID),
            Some(LOCATOR_CAPABILITY_ID)
        );
        assert_eq!(
            registry.owner_of(BLUR_COMMAND_ID),
            Some(LOCATOR_CAPABILITY_ID)
        );
        assert_eq!(
            registry.owner_of(FOCUS_COMMAND_ID),
            Some(LOCATOR_CAPABILITY_ID)
        );
        assert_eq!(Click::METHOD, CLICK_METHOD);
        assert_eq!(AllTextContents::METHOD, ALL_TEXT_CONTENTS_METHOD);
        assert_eq!(AllInnerTexts::METHOD, ALL_INNER_TEXTS_METHOD);
        assert_eq!(Blur::METHOD, BLUR_METHOD);
        assert_eq!(Count::METHOD, COUNT_METHOD);
        assert_eq!(Fill::METHOD, FILL_METHOD);
        assert_eq!(Focus::METHOD, FOCUS_METHOD);
        assert_eq!(GetAttribute::METHOD, GET_ATTRIBUTE_METHOD);
        assert_eq!(Hover::METHOD, HOVER_METHOD);
        assert_eq!(InnerHtml::METHOD, INNER_HTML_METHOD);
        assert_eq!(InnerText::METHOD, INNER_TEXT_METHOD);
        assert_eq!(InputValue::METHOD, INPUT_VALUE_METHOD);
        assert_eq!(IsChecked::METHOD, IS_CHECKED_METHOD);
        assert_eq!(IsEditable::METHOD, IS_EDITABLE_METHOD);
        assert_eq!(IsEnabled::METHOD, IS_ENABLED_METHOD);
        assert_eq!(IsVisible::METHOD, IS_VISIBLE_METHOD);
        assert_eq!(
            ScrollIntoViewIfNeeded::METHOD,
            SCROLL_INTO_VIEW_IF_NEEDED_METHOD
        );
        assert_eq!(TextContent::METHOD, TEXT_CONTENT_METHOD);
        assert_eq!(WaitFor::METHOD, WAIT_FOR_METHOD);
        assert_eq!(Click::TARGET_OBJECT_KINDS, Fill::TARGET_OBJECT_KINDS);
        assert_eq!(
            Click::TARGET_OBJECT_KINDS,
            AllTextContents::TARGET_OBJECT_KINDS
        );
        assert_eq!(
            Click::TARGET_OBJECT_KINDS,
            AllInnerTexts::TARGET_OBJECT_KINDS
        );
        assert_eq!(Click::TARGET_OBJECT_KINDS, Count::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, Blur::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, Focus::TARGET_OBJECT_KINDS);
        assert_eq!(
            Click::TARGET_OBJECT_KINDS,
            GetAttribute::TARGET_OBJECT_KINDS
        );
        assert_eq!(Click::TARGET_OBJECT_KINDS, Hover::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, InnerHtml::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, InnerText::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, InputValue::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, IsChecked::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, IsEditable::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, IsEnabled::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, IsVisible::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, Press::TARGET_OBJECT_KINDS);
        assert_eq!(
            Click::TARGET_OBJECT_KINDS,
            ScrollIntoViewIfNeeded::TARGET_OBJECT_KINDS
        );
        assert_eq!(
            Click::TARGET_OBJECT_KINDS,
            SelectOption::TARGET_OBJECT_KINDS
        );
        assert_eq!(Click::TARGET_OBJECT_KINDS, SetChecked::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, TextContent::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS, WaitFor::TARGET_OBJECT_KINDS);
        assert_eq!(Click::TARGET_OBJECT_KINDS.len(), 2);
        assert_eq!(
            Click::TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.frame"
        );
        assert_eq!(
            Click::TARGET_OBJECT_KINDS[1].as_str(),
            "pwright.browser.page"
        );
        assert_eq!(LOCATOR_COMMAND_DESCRIPTORS.len(), 23);
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[0].command(),
            ALL_INNER_TEXTS_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[1].command(),
            ALL_TEXT_CONTENTS_COMMAND_ID
        );
        assert_eq!(LOCATOR_COMMAND_DESCRIPTORS[2].command(), BLUR_COMMAND_ID);
        assert_eq!(LOCATOR_COMMAND_DESCRIPTORS[3].command(), CLICK_COMMAND_ID);
        assert_eq!(LOCATOR_COMMAND_DESCRIPTORS[4].command(), COUNT_COMMAND_ID);
        assert_eq!(LOCATOR_COMMAND_DESCRIPTORS[5].command(), FILL_COMMAND_ID);
        assert_eq!(LOCATOR_COMMAND_DESCRIPTORS[6].command(), FOCUS_COMMAND_ID);
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[7].command(),
            GET_ATTRIBUTE_COMMAND_ID
        );
        assert_eq!(LOCATOR_COMMAND_DESCRIPTORS[8].command(), HOVER_COMMAND_ID);
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[9].command(),
            INNER_HTML_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[10].command(),
            INNER_TEXT_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[11].command(),
            INPUT_VALUE_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[12].command(),
            IS_CHECKED_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[13].command(),
            IS_EDITABLE_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[14].command(),
            IS_ENABLED_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[15].command(),
            IS_VISIBLE_COMMAND_ID
        );
        assert_eq!(LOCATOR_COMMAND_DESCRIPTORS[16].command(), PRESS_COMMAND_ID);
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[17].command(),
            super::PRESS_SEQUENTIALLY_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[18].command(),
            SCROLL_INTO_VIEW_IF_NEEDED_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[19].command(),
            SELECT_OPTION_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[20].command(),
            SET_CHECKED_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[21].command(),
            TEXT_CONTENT_COMMAND_ID
        );
        assert_eq!(
            LOCATOR_COMMAND_DESCRIPTORS[22].command(),
            WAIT_FOR_COMMAND_ID
        );
        assert_eq!(
            LocatorCapability::DESCRIPTOR.plugin.id,
            LOCATOR_CAPABILITY_ID
        );
        assert_eq!(
            LocatorCapability::DESCRIPTOR.plugin.version,
            pwright_core::ProtocolVersion::new(1, 24)
        );
        assert_eq!(Press::COMMAND, PRESS_COMMAND_ID);
        assert_eq!(Press::METHOD, PRESS_METHOD);
        assert_eq!(
            ScrollIntoViewIfNeeded::COMMAND,
            SCROLL_INTO_VIEW_IF_NEEDED_COMMAND_ID
        );
        assert_eq!(
            ScrollIntoViewIfNeeded::METHOD,
            SCROLL_INTO_VIEW_IF_NEEDED_METHOD
        );
        assert_eq!(SelectOption::COMMAND, SELECT_OPTION_COMMAND_ID);
        assert_eq!(SelectOption::METHOD, SELECT_OPTION_METHOD);
        assert_eq!(SetChecked::COMMAND, SET_CHECKED_COMMAND_ID);
        assert_eq!(SetChecked::METHOD, SET_CHECKED_METHOD);
    }

    #[test]
    fn hover_options_preserve_fractional_position_and_canonical_modifiers() {
        // Arrange
        let position = Position::new(12.5, -3.25).expect("finite position should build");

        // Act
        let options = HoverOptions::new()
            .with_force(true)
            .with_scroll(ScrollMode::None)
            .with_modifier(KeyboardModifier::Shift)
            .with_modifier(KeyboardModifier::Alt)
            .with_modifier(KeyboardModifier::Shift)
            .with_position(position)
            .with_trial(true);
        let params = HoverParams::new(Selector::test_id("hover-target"), options);
        let modifiers = options
            .modifiers
            .expect("explicit modifiers should be present")
            .iter()
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(
            modifiers,
            vec![KeyboardModifier::Alt, KeyboardModifier::Shift]
        );
        assert_eq!(options.modifiers.map(KeyboardModifiers::len), Some(2));
        assert_eq!(options.position, Some(position));
        assert_eq!(params.options, options);
        assert_eq!(
            Position::new(f64::NAN, 0.0).map_err(|error| error.axis()),
            Err(super::CoordinateAxis::X)
        );
        assert_eq!(
            Position::new(0.0, f64::INFINITY).map_err(|error| error.axis()),
            Err(super::CoordinateAxis::Y)
        );
        assert_eq!(KeyboardModifiers::new().len(), 0);
    }

    #[test]
    fn hover_distinguishes_unspecified_from_explicitly_empty_modifiers() {
        // Arrange
        let unspecified = HoverOptions::new();

        // Act
        let explicitly_empty = HoverOptions::new().with_modifiers(KeyboardModifiers::new());

        // Assert
        assert_eq!(unspecified.modifiers, None);
        assert_eq!(
            explicitly_empty.modifiers.map(KeyboardModifiers::len),
            Some(0)
        );
    }

    #[test]
    fn all_text_contents_is_an_ordered_non_strict_snapshot_value() {
        // Arrange
        let params = AllTextContentsParams::new(Selector::test_id("row"));

        // Act
        let empty = Vec::<String>::new();
        let ordered = vec!["first".to_string(), String::new(), "third".to_string()];

        // Assert
        assert!(empty.is_empty());
        assert_eq!(ordered, ["first", "", "third"]);
        assert_eq!(params.selector, Selector::test_id("row"));
    }

    #[test]
    fn all_inner_texts_is_an_ordered_non_strict_rendered_text_value() {
        // Arrange
        let params = AllInnerTextsParams::new(Selector::test_id("row"));

        // Act
        let empty = Vec::<String>::new();
        let ordered = vec!["first".to_string(), String::new(), "third".to_string()];

        // Assert
        assert!(empty.is_empty());
        assert_eq!(ordered, ["first", "", "third"]);
        assert_eq!(params.selector, Selector::test_id("row"));
    }

    #[test]
    fn wait_for_defaults_to_visible_and_preserves_explicit_state() {
        // Arrange
        let selector = Selector::test_id("status");

        // Act
        let defaults = WaitForOptions::default();
        let hidden = WaitForOptions::new().with_state(WaitForState::Hidden);
        let params = WaitForParams::new(selector.clone(), hidden);

        // Assert
        assert_eq!(defaults.state, WaitForState::Visible);
        assert_eq!(params.selector, selector);
        assert_eq!(params.options, hidden);
    }

    #[test]
    fn text_content_preserves_lazy_selector_and_nullable_exact_text() {
        // Arrange
        let selector = Selector::test_id("status");

        // Act
        let params = TextContentParams::new(selector.clone());
        let present: TextContentResult = Some("  ready\nnow  ".to_string());
        let absent: TextContentResult = None;

        // Assert
        assert_eq!(params.selector, selector);
        assert_eq!(present.as_deref(), Some("  ready\nnow  "));
        assert_eq!(absent, None);
    }

    #[test]
    fn inner_text_preserves_lazy_selector_and_rendered_text_result() {
        // Arrange
        let selector = Selector::test_id("status");

        // Act
        let params = InnerTextParams::new(selector.clone());
        let rendered: InnerTextResult = "Ready now".to_string();

        // Assert
        assert_eq!(params.selector, selector);
        assert_eq!(rendered, "Ready now");
    }

    #[test]
    fn input_value_preserves_lazy_selector_and_exact_form_value() {
        // Arrange
        let selector = Selector::label("Account", true);

        // Act
        let params = InputValueParams::new(selector.clone());
        let value: InputValueResult = "alice@example.test".to_string();

        // Assert
        assert_eq!(params.selector, selector);
        assert_eq!(value, "alice@example.test");
    }

    #[test]
    fn inner_html_preserves_lazy_selector_and_serialized_markup_result() {
        // Arrange
        let selector = Selector::test_id("container");

        // Act
        let params = InnerHtmlParams::new(selector.clone());
        let markup: InnerHtmlResult = "<span>Ready</span>".to_string();

        // Assert
        assert_eq!(params.selector, selector);
        assert_eq!(markup, "<span>Ready</span>");
    }

    #[test]
    fn get_attribute_preserves_lazy_selector_name_and_nullable_result() {
        // Arrange
        let selector = Selector::test_id("container");
        let missing: GetAttributeResult = None;

        // Act
        let params = GetAttributeParams::new(selector.clone(), "data-state");
        let present: GetAttributeResult = Some("ready".to_string());

        // Assert
        assert_eq!(params.selector, selector);
        assert_eq!(params.name, "data-state");
        assert_eq!(present.as_deref(), Some("ready"));
        assert_eq!(missing, None);
    }

    #[test]
    fn count_params_preserve_a_non_strict_snapshot_query() {
        // Arrange
        let selector = Selector::test_id("row");

        // Act
        let params = CountParams::new(selector.clone());

        // Assert
        assert_eq!(params.selector, selector);
    }

    #[test]
    fn visibility_params_preserve_a_strict_snapshot_query() {
        // Arrange
        let selector = Selector::test_id("status");

        // Act
        let params = IsVisibleParams::new(selector.clone());

        // Assert
        assert_eq!(params.selector, selector);
    }

    #[test]
    fn element_state_params_preserve_the_lazy_strict_query() {
        // Arrange
        let selector = Selector::test_id("control");

        // Act
        let checked = IsCheckedParams::new(selector.clone());
        let editable = IsEditableParams::new(selector.clone());
        let enabled = IsEnabledParams::new(selector.clone());

        // Assert
        assert_eq!(checked.selector, selector);
        assert_eq!(editable.selector, selector);
        assert_eq!(enabled.selector, selector);
    }

    #[test]
    fn fill_options_and_params_are_portable_values() {
        // Arrange
        let selector = CssSelector::new("input[name=email]").expect("valid CSS");

        // Act
        let defaults = FillOptions::default();
        let forced = FillOptions::new().with_force(true);
        let params = FillParams::new(selector.clone(), "user@example.com", forced);

        // Assert
        assert!(!defaults.force);
        assert!(forced.force);
        assert_eq!(params.selector, Selector::Css(selector));
        assert_eq!(params.value, "user@example.com");
        assert_eq!(params.options, forced);
    }

    #[test]
    fn css_selector_rejects_an_empty_query() {
        // Arrange / Act
        let empty = CssSelector::new("  \n");
        let present = CssSelector::new("button[type=submit]");

        // Assert
        assert!(empty.is_err());
        assert_eq!(
            present.expect("selector should build").as_str(),
            "button[type=submit]"
        );
    }

    #[test]
    fn click_options_have_playwright_shaped_defaults_and_explicit_overrides() {
        // Arrange
        let count = NonZeroU8::new(2).expect("two is nonzero");

        // Act
        let defaults = ClickOptions::default();
        let configured = ClickOptions::new()
            .with_button(MouseButton::Right)
            .with_click_count(count)
            .with_force(true)
            .with_trial(true);

        // Assert
        assert_eq!(defaults.button, MouseButton::Left);
        assert_eq!(defaults.click_count, NonZeroU8::MIN);
        assert!(!defaults.force);
        assert!(!defaults.trial);
        assert_eq!(configured.button, MouseButton::Right);
        assert_eq!(configured.click_count, count);
        assert!(configured.force);
        assert!(configured.trial);
    }

    #[test]
    fn locator_v2_click_preserves_absence_and_the_full_playwright_option_surface() {
        // Arrange
        let position = Position::new(12.5, -3.0).expect("finite position should build");
        let modifiers = KeyboardModifiers::new().with(KeyboardModifier::ControlOrMeta);

        // Act
        let defaults = super::ClickOptionsV2::default();
        let configured = super::ClickOptionsV2::new()
            .with_button(MouseButton::Middle)
            .with_click_count(NonZeroU8::new(2).expect("two is nonzero"))
            .with_delay(Duration::from_millis(25))
            .with_force(false)
            .with_modifiers(modifiers)
            .with_no_wait_after(true)
            .with_position(position)
            .with_scroll(ScrollMode::None)
            .with_steps(NonZeroU32::new(4).expect("four is nonzero"))
            .with_trial(false);

        // Assert
        assert_eq!(defaults, super::ClickOptionsV2::new());
        assert_eq!(configured.button, Some(MouseButton::Middle));
        assert_eq!(configured.delay, Some(Duration::from_millis(25)));
        assert_eq!(configured.force, Some(false));
        assert_eq!(configured.modifiers, Some(modifiers));
        assert_eq!(configured.no_wait_after, Some(true));
        assert_eq!(configured.position, Some(position));
        assert_eq!(configured.scroll, Some(ScrollMode::None));
        assert_eq!(configured.steps, NonZeroU32::new(4));
        assert_eq!(configured.trial, Some(false));
        assert_eq!(
            <super::v2::LocatorCapability as Capability>::DESCRIPTOR
                .plugin
                .version,
            pwright_core::ProtocolVersion::new(2, 7)
        );
        assert_eq!(
            super::v2::Click::COMMAND,
            <super::Click as pwright_core::CapabilityCommandSpec>::COMMAND
        );
        assert_eq!(super::v2::Click::METHOD, super::Click::METHOD);
        assert_eq!(super::v2::Blur::COMMAND, super::Blur::COMMAND);
        assert_eq!(super::v2::Blur::METHOD, super::Blur::METHOD);
        assert_eq!(super::v2::Focus::COMMAND, super::Focus::COMMAND);
        assert_eq!(super::v2::Focus::METHOD, super::Focus::METHOD);
        assert_eq!(super::v2::Press::COMMAND, super::Press::COMMAND);
        assert_eq!(super::v2::Press::METHOD, super::Press::METHOD);
        assert_eq!(
            super::v2::ScrollIntoViewIfNeeded::COMMAND,
            super::ScrollIntoViewIfNeeded::COMMAND
        );
        assert_eq!(
            super::v2::ScrollIntoViewIfNeeded::METHOD,
            super::ScrollIntoViewIfNeeded::METHOD
        );
        assert_eq!(
            super::v2::SelectOption::COMMAND,
            super::SelectOption::COMMAND
        );
        assert_eq!(super::v2::SelectOption::METHOD, super::SelectOption::METHOD);
        assert_eq!(super::v2::SetChecked::COMMAND, super::SetChecked::COMMAND);
        assert_eq!(super::v2::SetChecked::METHOD, super::SetChecked::METHOD);
    }

    #[test]
    fn set_checked_options_preserve_absence_and_explicit_values() {
        // Arrange
        let position = Position::new(4.5, -2.0).expect("finite position should build");

        // Act
        let defaults = SetCheckedOptions::default();
        let configured = SetCheckedOptions::new()
            .with_force(false)
            .with_position(position)
            .with_scroll(ScrollMode::None)
            .with_trial(true);
        let params = SetCheckedParams::new(Selector::test_id("accept"), true, configured);

        // Assert
        assert_eq!(defaults, SetCheckedOptions::new());
        assert_eq!(configured.force, Some(false));
        assert_eq!(configured.position, Some(position));
        assert_eq!(configured.scroll, Some(ScrollMode::None));
        assert_eq!(configured.trial, Some(true));
        assert!(params.checked);
    }

    #[test]
    fn scroll_into_view_if_needed_preserves_one_lazy_selector() {
        // Arrange
        let selector = Selector::css("#below-fold").expect("selector should validate");

        // Act
        let params = ScrollIntoViewIfNeededParams::new(selector.clone());

        // Assert
        assert_eq!(params.selector, selector);
    }

    #[test]
    fn select_option_preserves_string_descriptor_and_empty_list_semantics() {
        // Arrange
        let string = SelectOptionValues::from("Blue");
        let descriptor = SelectOptionValue::new()
            .with_value("green")
            .with_label(" Green ")
            .with_index(-1);

        // Act
        let params = SelectOptionParams::new(
            Selector::test_id("color"),
            vec![descriptor.clone()],
            SelectOptionOptions::new().with_force(false),
        );
        let empty = SelectOptionParams::new(
            Selector::test_id("color"),
            SelectOptionValues::new(),
            SelectOptionOptions::new(),
        );

        // Assert
        assert_eq!(
            string.as_slice(),
            &[SelectOptionValue::value_or_label("Blue")]
        );
        assert_eq!(params.values, vec![descriptor]);
        assert_eq!(params.options.force, Some(false));
        assert!(empty.values.is_empty());
    }

    #[cfg(feature = "select-option-timeout-progress")]
    #[test]
    fn select_option_progress_does_not_change_request_value_equality() {
        // Arrange
        let request = SelectOptionParams::new(
            Selector::test_id("color"),
            "Blue",
            SelectOptionOptions::new(),
        );

        // Act
        let observed = request
            .clone()
            .with_progress(crate::SelectOptionProgress::new());

        // Assert
        assert_eq!(request, observed);
    }

    #[test]
    fn press_key_preserves_playwright_plus_grammar_with_explicit_bounds() {
        // Arrange
        let plus = PressKey::new("+").expect("plus should be one key token");
        let shifted_plus = PressKey::new("Shift++").expect("shift plus should be two tokens");
        let maximum = (0..PRESS_KEY_MAX_TOKENS)
            .map(|_| "A")
            .collect::<Vec<_>>()
            .join("+");

        // Act
        let maximum = PressKey::new(maximum).expect("the maximum token count should fit");
        let trailing = PressKey::new("Control+");
        let too_many = PressKey::new(format!("{maximum}+A"));
        let too_long = PressKey::new("x".repeat(PRESS_KEY_MAX_BYTES + 1));

        // Assert
        assert_eq!(plus.token_count(), 1);
        assert_eq!(shifted_plus.token_count(), 2);
        assert_eq!(maximum.token_count(), PRESS_KEY_MAX_TOKENS);
        assert_eq!(trailing, Err(PressKeyError::EmptyToken { token: 1 }));
        assert_eq!(
            too_many,
            Err(PressKeyError::TooManyTokens {
                tokens: PRESS_KEY_MAX_TOKENS + 1,
                maximum: PRESS_KEY_MAX_TOKENS,
            })
        );
        assert_eq!(
            too_long,
            Err(PressKeyError::TooLong {
                bytes: PRESS_KEY_MAX_BYTES + 1,
                maximum: PRESS_KEY_MAX_BYTES,
            })
        );
    }

    #[test]
    fn selector_ast_composes_without_resolving_a_dom_identity() {
        // Arrange
        let buttons = Selector::css("button").expect("valid CSS");
        let enabled = Selector::css(":not([disabled])").expect("valid CSS");

        // Act
        let selector = buttons
            .and(enabled)
            .filter_text("Save", true)
            .or(Selector::css("#fallback").expect("valid CSS"))
            .nth(-1);

        // Assert
        assert!(matches!(selector, Selector::Nth { index: -1, .. }));
    }

    #[test]
    fn selector_filters_preserve_the_base_and_relative_descendant_queries() {
        // Arrange
        let cards = Selector::css(".card").expect("valid CSS");
        let marker = Selector::css(".ready").expect("valid CSS");
        let excluded = Selector::css("[data-excluded]").expect("valid CSS");

        // Act
        let selector = cards
            .filter_has(marker)
            .filter_has_not(excluded)
            .filter_not_text("Blocked", false)
            .filter_visible(true);

        // Assert
        assert!(matches!(
            selector,
            Selector::FilterVisible { visible: true, .. }
        ));
        let display = selector.to_string();
        assert!(display.contains("filter_has"));
        assert!(display.contains("filter_has_not"));
        assert!(display.contains("filter_not_text"));
    }

    #[test]
    fn regex_matchers_require_canonical_ecmascript_flags() {
        // Arrange / Act
        let canonical = RegexMatcher::new("^Save\\s+\\d+$", "gim");
        let duplicate = RegexMatcher::new("Save", "ii");
        let reversed = RegexMatcher::new("Save", "mi");
        let unknown = RegexMatcher::new("Save", "x");
        let conflicting = RegexMatcher::new("Save", "uv");

        // Assert
        assert_eq!(
            canonical.expect("canonical flags should build").flags(),
            "gim"
        );
        assert_eq!(duplicate, Err(RegexMatcherError::DuplicateFlag('i')));
        assert_eq!(reversed, Err(RegexMatcherError::NonCanonicalOrder));
        assert_eq!(unknown, Err(RegexMatcherError::UnsupportedFlag('x')));
        assert_eq!(conflicting, Err(RegexMatcherError::ConflictingUnicodeModes));
    }

    #[test]
    fn regex_selector_variants_remain_lazy_portable_values() {
        // Arrange
        let cards = Selector::css(".card").expect("valid CSS");
        let positive = RegexMatcher::new("Release\\s+\\d+", "i").expect("valid regex");
        let negative = RegexMatcher::new("Blocked", "").expect("valid regex");
        let role_name = RegexMatcher::new("^Deploy", "i").expect("valid regex");

        // Act
        let selector = cards
            .filter_text_regex(positive)
            .filter_not_text_regex(negative)
            .within(
                Selector::role(
                    AriaRole::Button,
                    RoleOptions::new().with_name_regex(role_name),
                )
                .expect("button regex role should build"),
            )
            .or(Selector::label_regex(
                RegexMatcher::new("Release train", "i").expect("valid regex"),
            ))
            .or(Selector::text_regex(
                RegexMatcher::new("^Fallback$", "").expect("valid regex"),
            ));

        // Assert
        assert!(matches!(selector, Selector::Or { .. }));
        let display = selector.to_string();
        assert!(display.contains("filter_text_regex"));
        assert!(display.contains("filter_not_text_regex"));
        assert!(display.contains("get_by_role"));
        assert!(display.contains("get_by_label_regex"));
        assert!(display.contains("get_by_text_regex"));
    }

    #[test]
    fn attribute_and_test_id_selectors_are_immutable_portable_values() {
        // Arrange
        let attributes =
            TestIdAttributes::new("data-pw,data-testid").expect("valid attribute names");

        // Act
        let selector = Selector::attribute_text(UserFacingAttribute::Placeholder, "Search", false)
            .or(Selector::attribute_regex(
                UserFacingAttribute::Alt,
                RegexMatcher::new("^Product \\d+$", "i").expect("valid regex"),
            ))
            .or(Selector::attribute_text(
                UserFacingAttribute::Title,
                "Release notes",
                true,
            ))
            .or(Selector::test_id_with_attributes(
                attributes.clone(),
                "save",
            ))
            .or(Selector::test_id_regex_with_attributes(
                attributes,
                RegexMatcher::new("^custom-", "").expect("valid regex"),
            ));

        // Assert
        let display = selector.to_string();
        assert!(display.contains("get_by_placeholder"));
        assert!(display.contains("get_by_alt_text_regex"));
        assert!(display.contains("get_by_title"));
        assert!(display.contains("data-pw,data-testid"));
        assert!(display.contains("get_by_test_id_regex"));
    }

    #[test]
    fn test_id_attribute_names_are_canonical_and_bounded() {
        // Arrange
        let too_many = (0..=MAX_TEST_ID_ATTRIBUTES)
            .map(|index| format!("data-id-{index}"))
            .collect::<Vec<_>>()
            .join(",");

        // Act
        let standard = TestIdAttributes::standard();
        let multiple = TestIdAttributes::new("data-pw,data-testid");
        let empty = TestIdAttributes::new("data-pw,,data-testid");
        let invalid = TestIdAttributes::new("data testid");
        let excessive = TestIdAttributes::new(too_many);

        // Assert
        assert_eq!(standard.as_csv(), "data-testid");
        assert_eq!(multiple.expect("names should build").names().count(), 2);
        assert_eq!(empty, Err(TestIdAttributesError::EmptyName { index: 1 }));
        assert_eq!(
            invalid,
            Err(TestIdAttributesError::InvalidCharacter {
                index: 0,
                character: ' ',
            })
        );
        assert_eq!(
            excessive,
            Err(TestIdAttributesError::TooMany {
                actual: MAX_TEST_ID_ATTRIBUTES + 1,
                limit: MAX_TEST_ID_ATTRIBUTES,
            })
        );
    }

    #[test]
    fn semantic_selectors_and_descendant_chaining_are_portable_values() {
        // Arrange
        let dialog = Selector::role(
            AriaRole::Dialog,
            RoleOptions::new().with_name("Settings", true),
        )
        .expect("dialog supports accessible-name matching");
        let save = Selector::role(
            AriaRole::Button,
            RoleOptions::new()
                .with_name("Save", false)
                .with_pressed(AriaTriState::False),
        )
        .expect("buttons support pressed matching");

        // Act
        let selector = dialog
            .within(save)
            .and(Selector::label("Primary action", true))
            .or(Selector::text("Fallback", false));

        // Assert
        assert!(matches!(selector, Selector::Or { .. }));
        assert!(selector.to_string().contains("within"));
    }

    #[test]
    fn role_state_filters_are_rejected_for_roles_that_do_not_define_them() {
        // Arrange / Act
        let checked_button = Selector::role(
            AriaRole::Button,
            RoleOptions::new().with_checked(AriaTriState::True),
        );
        let level_button = Selector::role(
            AriaRole::Button,
            RoleOptions::new().with_level(std::num::NonZeroU32::MIN),
        );

        // Assert
        assert_eq!(
            checked_button
                .expect_err("button has no checked state")
                .option,
            "checked"
        );
        assert_eq!(
            level_button.expect_err("button has no level state").option,
            "level"
        );
        assert_eq!(
            "menuitemcheckbox".parse::<AriaRole>(),
            Ok(AriaRole::MenuItemCheckbox)
        );
        assert!("made-up-role".parse::<AriaRole>().is_err());
    }
}
