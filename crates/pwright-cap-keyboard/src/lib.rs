//! Optional backend-neutral Page Keyboard contracts. No selector or transport.
use std::{fmt, time::Duration};

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;
pub use generated_capability::*;

pub const KEYBOARD_KEY_MAX_BYTES: usize = 128;
pub const KEYBOARD_TEXT_MAX_BYTES: usize = pwright_input_model::INPUT_TEXT_MAX_BYTES;
pub type KeyboardSuccess = ();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyboardArgumentTooLarge {
    pub capacity: usize,
    pub maximum: usize,
}

impl fmt::Display for KeyboardArgumentTooLarge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "keyboard argument capacity {} exceeds {} bytes",
            self.capacity, self.maximum
        )
    }
}
impl std::error::Error for KeyboardArgumentTooLarge {}

macro_rules! bounded_argument {
    ($name:ident, $limit:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub struct $name(String);

        impl TryFrom<String> for $name {
            type Error = KeyboardArgumentTooLarge;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                if value.capacity() > $limit {
                    return Err(KeyboardArgumentTooLarge {
                        capacity: value.capacity(),
                        maximum: $limit,
                    });
                }
                Ok(Self(value))
            }
        }

        impl $name {
            /// Bounds before copying; this does not supply a resource lease.
            pub fn copy_from_str(value: &str) -> Result<Self, KeyboardArgumentTooLarge> {
                if value.len() > $limit {
                    return Err(KeyboardArgumentTooLarge {
                        capacity: value.len(),
                        maximum: $limit,
                    });
                }
                Self::try_from(value.to_owned())
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
            pub fn capacity(&self) -> usize {
                self.0.capacity()
            }
        }
    };
}
// Key names are intentionally not parsed here: raw input failure timing belongs
// to its backend. Empty/unknown aliases may fail after an earlier key is down.
bounded_argument!(KeyboardKey, KEYBOARD_KEY_MAX_BYTES);
bounded_argument!(KeyboardText, KEYBOARD_TEXT_MAX_BYTES);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DownParams {
    pub key: KeyboardKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpParams {
    pub key: KeyboardKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PressParams {
    pub key: KeyboardKey,
    pub delay: Option<Duration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InsertTextParams {
    pub text: KeyboardText,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypeParams {
    pub text: KeyboardText,
    pub delay: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::{CommandSpec, RegistryBuilder};

    #[test]
    fn keyboard_registers_five_independent_page_only_commands() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder.register::<KeyboardCapability>().unwrap();
        // Act
        let registry = builder.build().unwrap();
        // Assert
        assert!(registry.contains(KEYBOARD_CAPABILITY_ID));
        assert_eq!(KEYBOARD_COMMAND_DESCRIPTORS.len(), 5);
        assert_eq!(KeyboardDown::METHOD, KEYBOARD_DOWN_METHOD);
        for targets in [
            KEYBOARD_DOWN_TARGET_OBJECT_KINDS,
            KEYBOARD_UP_TARGET_OBJECT_KINDS,
            KEYBOARD_PRESS_TARGET_OBJECT_KINDS,
            KEYBOARD_INSERT_TEXT_TARGET_OBJECT_KINDS,
            KEYBOARD_TYPE_TARGET_OBJECT_KINDS,
        ] {
            assert_eq!(targets.len(), 1);
            assert_eq!(targets[0].as_str(), "pwright.browser.page");
        }
    }

    #[test]
    fn raw_key_contract_keeps_unknown_and_empty_tail_for_ordered_backend_errors() {
        // Arrange / Act
        let keys: Vec<_> = ["", "Shift+", "Shift+UnknownKey"]
            .into_iter()
            .map(|value| KeyboardKey::copy_from_str(value).unwrap())
            .collect();
        // Assert
        assert_eq!(keys[0].as_str(), "");
        assert_eq!(keys[1].as_str(), "Shift+");
        assert_eq!(keys[2].as_str(), "Shift+UnknownKey");
    }

    #[test]
    fn ownership_bound_checks_capacity_not_only_visible_length() {
        // Arrange
        let mut key = String::with_capacity(KEYBOARD_KEY_MAX_BYTES + 1);
        key.push('a');
        let mut text = String::with_capacity(KEYBOARD_TEXT_MAX_BYTES + 1);
        text.push('b');
        // Act
        let key_error = KeyboardKey::try_from(key).unwrap_err();
        let text_error = KeyboardText::try_from(text).unwrap_err();
        // Assert
        assert!(key_error.capacity > key_error.maximum);
        assert!(text_error.capacity > text_error.maximum);
        assert!(KeyboardKey::copy_from_str(&"k".repeat(KEYBOARD_KEY_MAX_BYTES)).is_ok());
        assert!(KeyboardText::copy_from_str(&"t".repeat(KEYBOARD_TEXT_MAX_BYTES)).is_ok());
    }
}
