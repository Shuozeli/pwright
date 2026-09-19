//! Action requests are portable values. Browser behavior and cleanup stay in
//! the existing Locator services and Page-owned input coordinator.

use pwright_cap_locator::{
    Blur, BlurParams, ClickParamsV2, Fill, FillParams, Focus, FocusParams, Hover, HoverParams,
    Press, PressParams, ScrollIntoViewIfNeeded, ScrollIntoViewIfNeededParams, SelectOption,
    SelectOptionParams, SetChecked, SetCheckedParams, v2,
};
pub use pwright_cap_locator::{
    ClickOptionsV2, FillOptions, HoverOptions, KeyboardModifier, KeyboardModifiers, MouseButton,
    Position, PressKey, PressOptions, SelectOptionOptions, SelectOptionValue, SetCheckedOptions,
};

use super::HostLocator;
use crate::HostError;
use pwright_cap_locator::{PressSequentially, PressSequentiallyParams};

pub const MAX_VALUE_BYTES: usize = 1024 * 1024;
pub const MAX_OPTION_BYTES: usize = 64 * 1024;
pub const MAX_OPTIONS: usize = 1024;

pub enum LocatorAction {
    PressSequentially {
        text: String,
        delay: Option<std::time::Duration>,
    },
    Click(ClickOptionsV2),
    Fill {
        value: String,
        options: FillOptions,
    },
    Press {
        key: PressKey,
        options: PressOptions,
    },
    SetChecked {
        checked: bool,
        options: SetCheckedOptions,
    },
    SelectOption {
        values: Vec<SelectOptionValue>,
        options: SelectOptionOptions,
    },
    Hover(HoverOptions),
    Focus,
    Blur,
    ScrollIntoViewIfNeeded,
}

pub enum ActionValue {
    Unit,
    Strings(Vec<String>),
}

impl LocatorAction {
    /// Safe Rust callers are subject to the same input limits as Node callers.
    pub fn validate(&self) -> Result<(), HostError> {
        match self {
            Self::PressSequentially { text, .. } if text.capacity() > MAX_VALUE_BYTES => {
                return Err(HostError::InvalidArgument(
                    "sequential text exceeds 1 MiB".into(),
                ));
            }
            Self::Fill { value, .. } if value.capacity() > MAX_VALUE_BYTES => {
                return Err(HostError::InvalidArgument(
                    "fill value exceeds 1 MiB".into(),
                ));
            }
            Self::SelectOption { values, .. } => {
                if values.capacity() > MAX_OPTIONS {
                    return Err(HostError::InvalidArgument(
                        "selectOption exceeds 1024 descriptors".into(),
                    ));
                }
                let mut total = 0usize;
                for value in values {
                    for text in [&value.value_or_label, &value.value, &value.label]
                        .into_iter()
                        .flatten()
                    {
                        if text.capacity() > MAX_OPTION_BYTES {
                            return Err(HostError::InvalidArgument(
                                "selectOption string exceeds 64 KiB".into(),
                            ));
                        }
                        total = total.checked_add(text.capacity()).ok_or_else(|| {
                            HostError::InvalidArgument("selectOption byte overflow".into())
                        })?;
                    }
                }
                if total > MAX_VALUE_BYTES {
                    return Err(HostError::InvalidArgument(
                        "selectOption strings exceed 1 MiB".into(),
                    ));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl HostLocator {
    pub async fn act(
        &self,
        action: LocatorAction,
        timeout_ms: u32,
    ) -> Result<ActionValue, HostError> {
        action.validate()?;
        let host = self.host()?;
        let (selector, _query_lease) = self.query.command_selector()?;
        let frame = self.frame.get().to_string();
        let outcome = match action {
            LocatorAction::PressSequentially { text, delay } => host
                .invoke_locator_keyboard::<PressSequentially>(
                    &frame,
                    PressSequentiallyParams::new(selector, text, delay),
                    timeout_ms,
                )
                .await
                .map(|()| ActionValue::Unit),
            LocatorAction::Click(options) => host
                .invoke::<v2::Click>(&frame, ClickParamsV2::new(selector, options), timeout_ms)
                .await
                .map(|()| ActionValue::Unit),
            LocatorAction::Fill { value, options } => host
                .invoke::<Fill>(
                    &frame,
                    FillParams::new(selector, value, options),
                    timeout_ms,
                )
                .await
                .map(|()| ActionValue::Unit),
            LocatorAction::Press { key, options } => host
                .invoke_locator_keyboard::<Press>(
                    &frame,
                    PressParams::new(selector, key, options),
                    timeout_ms,
                )
                .await
                .map(|()| ActionValue::Unit),
            LocatorAction::SetChecked { checked, options } => host
                .invoke::<SetChecked>(
                    &frame,
                    SetCheckedParams::new(selector, checked, options),
                    timeout_ms,
                )
                .await
                .map(|()| ActionValue::Unit),
            LocatorAction::SelectOption { values, options } => host
                .invoke::<SelectOption>(
                    &frame,
                    SelectOptionParams::new(selector, values, options),
                    timeout_ms,
                )
                .await
                .map(ActionValue::Strings),
            LocatorAction::Hover(options) => host
                .invoke::<Hover>(&frame, HoverParams::new(selector, options), timeout_ms)
                .await
                .map(|()| ActionValue::Unit),
            LocatorAction::Focus => host
                .invoke::<Focus>(&frame, FocusParams::new(selector), timeout_ms)
                .await
                .map(|()| ActionValue::Unit),
            LocatorAction::Blur => host
                .invoke::<Blur>(&frame, BlurParams::new(selector), timeout_ms)
                .await
                .map(|()| ActionValue::Unit),
            LocatorAction::ScrollIntoViewIfNeeded => host
                .invoke::<ScrollIntoViewIfNeeded>(
                    &frame,
                    ScrollIntoViewIfNeededParams::new(selector),
                    timeout_ms,
                )
                .await
                .map(|()| ActionValue::Unit),
        };
        crate::completion::publish_command_result(
            outcome,
            host.is_connected(),
            host.flush_topology(),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequential_typing_rejects_excess_retained_capacity_before_dispatch() {
        // Arrange
        let action = LocatorAction::PressSequentially {
            text: String::with_capacity(MAX_VALUE_BYTES + 1),
            delay: None,
        };
        // Act
        let outcome = action.validate();
        // Assert
        assert!(matches!(outcome, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn fill_rejects_excess_retained_capacity_before_dispatch() {
        // Arrange
        let action = LocatorAction::Fill {
            value: String::with_capacity(MAX_VALUE_BYTES + 1),
            options: FillOptions::new(),
        };
        // Act
        let outcome = action.validate();
        // Assert
        assert!(matches!(outcome, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn selection_counts_all_descriptor_string_capacities() {
        // Arrange
        let values = (0..17)
            .map(|_| SelectOptionValue::new().with_label("x".repeat(MAX_OPTION_BYTES)))
            .collect();
        let action = LocatorAction::SelectOption {
            values,
            options: SelectOptionOptions::new(),
        };
        // Act
        let outcome = action.validate();
        // Assert
        assert!(matches!(outcome, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn empty_selection_preserves_deselection() {
        // Arrange
        let action = LocatorAction::SelectOption {
            values: Vec::new(),
            options: SelectOptionOptions::new(),
        };
        // Act
        let outcome = action.validate();
        // Assert
        assert!(outcome.is_ok());
    }
}
