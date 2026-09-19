//! Command-free, backend-neutral input bounds for Pwright.
//!
//! This crate owns no command, protocol, transport, runtime, browser backend,
//! selector, or browser process API.

/// Maximum physical keys retained by one bounded input chord transaction.
pub const INPUT_CHORD_MAX_KEYS: usize = 16;

/// Maximum UTF-8 byte length accepted by one sequential typing operation.
/// This does not bound the backing owner's capacity or account for its lifetime.
pub const INPUT_TEXT_MAX_BYTES: usize = 1024 * 1024;

/// Explicit event-work bound for a raw mouse move, not a coordinate clamp.
pub const INPUT_MOUSE_MAX_STEPS: u32 = 10_000;

/// Maximum press/release pairs retained by one admitted raw click batch.
pub const INPUT_MOUSE_MAX_CLICKS: u8 = 32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MouseClickCount(u8);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidMouseClickCount {
    NonFinite,
    Fractional,
    WorkLimit,
}

impl std::fmt::Display for InvalidMouseClickCount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFinite => f.write_str("mouse click count must be finite"),
            Self::Fractional => f.write_str("mouse click count must be an integer"),
            Self::WorkLimit => write!(
                f,
                "mouse click count exceeds the {INPUT_MOUSE_MAX_CLICKS} pair-work bound"
            ),
        }
    }
}
impl std::error::Error for InvalidMouseClickCount {}
impl TryFrom<f64> for MouseClickCount {
    type Error = InvalidMouseClickCount;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(InvalidMouseClickCount::NonFinite);
        }
        if value.fract() != 0.0 {
            return Err(InvalidMouseClickCount::Fractional);
        }
        if value > f64::from(INPUT_MOUSE_MAX_CLICKS) {
            return Err(InvalidMouseClickCount::WorkLimit);
        }
        Ok(Self(value.max(0.0) as u8))
    }
}
impl MouseClickCount {
    pub const ONE: Self = Self(1);
    pub const TWO: Self = Self(2);
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Playwright's public integer step semantics: nonpositive values emit no events.
/// An f64 retains the integral numeric range accepted by the JS protocol.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MouseMoveSteps(f64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidMouseMoveSteps {
    NonFinite,
    Fractional,
    WorkLimit,
}

impl std::fmt::Display for InvalidMouseMoveSteps {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NonFinite => formatter.write_str("mouse steps must be finite"),
            Self::Fractional => formatter.write_str("mouse steps must be an integer"),
            Self::WorkLimit => write!(
                formatter,
                "mouse steps exceed the {INPUT_MOUSE_MAX_STEPS} event-work bound"
            ),
        }
    }
}
impl std::error::Error for InvalidMouseMoveSteps {}

impl TryFrom<f64> for MouseMoveSteps {
    type Error = InvalidMouseMoveSteps;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(InvalidMouseMoveSteps::NonFinite);
        }
        if value.fract() != 0.0 {
            return Err(InvalidMouseMoveSteps::Fractional);
        }
        if value > f64::from(INPUT_MOUSE_MAX_STEPS) {
            return Err(InvalidMouseMoveSteps::WorkLimit);
        }
        Ok(Self(value))
    }
}
impl MouseMoveSteps {
    pub const ONE: Self = Self(1.0);
    pub const fn get(self) -> f64 {
        self.0
    }
}

/// Validated borrowed text; ownership and byte accounting stay with the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputText<'a>(&'a str);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InputTextTooLong {
    pub actual: usize,
    pub maximum: usize,
}

impl std::fmt::Display for InputTextTooLong {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "input text is {} bytes, exceeding {}",
            self.actual, self.maximum
        )
    }
}

impl std::error::Error for InputTextTooLong {}

impl<'a> TryFrom<&'a str> for InputText<'a> {
    type Error = InputTextTooLong;

    fn try_from(value: &'a str) -> Result<Self, Self::Error> {
        if value.len() > INPUT_TEXT_MAX_BYTES {
            return Err(InputTextTooLong {
                actual: value.len(),
                maximum: INPUT_TEXT_MAX_BYTES,
            });
        }
        Ok(Self(value))
    }
}

impl<'a> InputText<'a> {
    pub const fn as_str(self) -> &'a str {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn click_counts_bound_work_and_normalize_nonpositive_integers() {
        // Arrange
        let values = [-100.0, -0.0, 1.0, 2.0, 32.0];
        // Act
        let counts = values.map(|value| MouseClickCount::try_from(value).unwrap().get());
        // Assert
        assert_eq!(counts, [0, 0, 1, 2, INPUT_MOUSE_MAX_CLICKS]);
    }

    #[test]
    fn invalid_click_counts_have_distinct_validation_errors() {
        // Arrange
        let values = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.5, 2.5, 33.0];
        // Act
        let errors = values.map(|value| MouseClickCount::try_from(value).unwrap_err());
        // Assert
        use InvalidMouseClickCount::{Fractional, NonFinite, WorkLimit};
        assert_eq!(
            errors,
            [
                NonFinite, NonFinite, NonFinite, Fractional, Fractional, WorkLimit
            ]
        );
    }

    #[test]
    fn numeric_mouse_steps_preserve_integer_and_nonpositive_values() {
        // Arrange
        let values = [0.0, -2.0, 1.0, 2.0, f64::from(INPUT_MOUSE_MAX_STEPS)];
        // Act
        let actual = values.map(|value| MouseMoveSteps::try_from(value).unwrap().get());
        // Assert
        assert_eq!(actual, values);
    }

    #[test]
    fn mouse_steps_reject_nonfinite_or_excess_work_before_dispatch() {
        // Arrange
        let values = [
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::from(INPUT_MOUSE_MAX_STEPS) + 1.0,
        ];
        // Act
        let rejected = values.map(|value| MouseMoveSteps::try_from(value).is_err());
        // Assert
        assert_eq!(rejected, [true; 4]);
    }

    #[test]
    fn fractional_mouse_steps_reject_with_a_distinct_validation_error() {
        // Arrange
        let value = 2.5;
        // Act
        let error = MouseMoveSteps::try_from(value).unwrap_err();
        // Assert
        assert_eq!(error, InvalidMouseMoveSteps::Fractional);
    }

    #[test]
    fn input_text_borrows_at_the_limit_and_rejects_excess_bytes() {
        // Arrange
        let text = "x".repeat(INPUT_TEXT_MAX_BYTES);
        let oversized = format!("{text}x");
        // Act
        let accepted = InputText::try_from(text.as_str()).unwrap();
        let rejected = InputText::try_from(oversized.as_str()).unwrap_err();
        // Assert
        assert_eq!(accepted.as_str().as_ptr(), text.as_ptr());
        assert_eq!(rejected.actual, INPUT_TEXT_MAX_BYTES + 1);
        assert_eq!(rejected.maximum, INPUT_TEXT_MAX_BYTES);
        assert!(!rejected.to_string().contains("xxxxx"));
    }
}
