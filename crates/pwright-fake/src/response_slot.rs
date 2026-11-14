//! Persistent response storage shared by configurable CDP test clients.

use std::sync::Mutex;

/// One optional, persistent response value.
///
/// This type intentionally owns no default, strict-mode policy, error script,
/// one-shot queue, async gate, or protocol-error conversion.
pub struct CdpResponseSlot<T>(Mutex<Option<T>>);

impl<T> CdpResponseSlot<T> {
    pub fn new() -> Self {
        Self(Mutex::new(None))
    }

    pub fn set(&self, response: T) {
        *self.0.lock().unwrap() = Some(response);
    }

    pub fn clear(&self) {
        *self.0.lock().unwrap() = None;
    }
}

impl<T: Clone> CdpResponseSlot<T> {
    pub fn get(&self) -> Option<T> {
        self.0.lock().unwrap().clone()
    }
}

impl<T> Default for CdpResponseSlot<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_response_can_be_replaced() {
        // Arrange
        let slot = CdpResponseSlot::new();
        slot.set(7);

        // Act
        slot.set(9);

        // Assert
        assert_eq!(slot.get(), Some(9));
    }

    #[test]
    fn clearing_removes_the_configured_response() {
        // Arrange
        let slot = CdpResponseSlot::new();
        slot.set(7);

        // Act
        slot.clear();

        // Assert
        assert_eq!(slot.get(), None);
    }
}
