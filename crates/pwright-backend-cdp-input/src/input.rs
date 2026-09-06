//! Serialized, target-local CDP input state.
//!
//! Playwright models keyboard and mouse state on a Page rather than on an
//! individual Locator call. This coordinator is intentionally independent of
//! locator selectors and protocol commands so later keyboard, mouse, and touch
//! capabilities can share the same bounded state machine.

use std::num::NonZeroU32;
use std::sync::Arc;

use pwright_cdp::connection::{CdpError, Result as CdpResult};
use pwright_cdp::{CdpClient, KeyEventType, KeyboardEventParams, MouseButton, MouseEventType};
use pwright_input_model::INPUT_CHORD_MAX_KEYS;
use tokio::sync::{Mutex, OwnedMutexGuard};

use crate::keyboard::{ResolvedKey, mac_editing_commands};

/// Resolved CDP modifier state. Smart platform aliases must be resolved before
/// entering this backend boundary.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CdpModifiers(u8);

impl CdpModifiers {
    pub const NONE: Self = Self(0);
    pub const ALT: Self = Self(1);
    pub const CONTROL: Self = Self(2);
    pub const META: Self = Self(4);
    pub const SHIFT: Self = Self(8);

    pub const fn with(self, modifier: Self) -> Self {
        Self(self.0 | modifier.0)
    }

    pub const fn contains(self, modifier: Self) -> bool {
        self.0 & modifier.0 != 0
    }

    pub const fn mask(self) -> i32 {
        self.0 as i32
    }

    const fn without(self, modifier: Self) -> Self {
        Self(self.0 & !modifier.0)
    }
}

#[derive(Clone, Copy)]
struct ModifierKey {
    modifier: CdpModifiers,
    key: &'static str,
    code: &'static str,
    windows_virtual_key_code: i64,
}

const MODIFIER_KEYS: [ModifierKey; 4] = [
    ModifierKey {
        modifier: CdpModifiers::ALT,
        key: "Alt",
        code: "AltLeft",
        windows_virtual_key_code: 18,
    },
    ModifierKey {
        modifier: CdpModifiers::CONTROL,
        key: "Control",
        code: "ControlLeft",
        windows_virtual_key_code: 17,
    },
    ModifierKey {
        modifier: CdpModifiers::META,
        key: "Meta",
        code: "MetaLeft",
        windows_virtual_key_code: 91,
    },
    ModifierKey {
        modifier: CdpModifiers::SHIFT,
        key: "Shift",
        code: "ShiftLeft",
        windows_virtual_key_code: 16,
    },
];

#[derive(Default)]
struct CdpInputState {
    pressed_modifiers: CdpModifiers,
    pending_restore: Option<CdpModifiers>,
    pressed_keys: [Option<ResolvedKey>; INPUT_CHORD_MAX_KEYS],
    pressed_key_count: usize,
    pointer_x: f64,
    pointer_y: f64,
    pressed_button: Option<PressedMouseButton>,
}

#[derive(Clone, Copy)]
struct PressedMouseButton {
    button: MouseButton,
    click_count: i32,
}

/// One Page-scoped serialization gate and fixed-size input-state record.
///
/// The coordinator contains no unbounded collection. A modifier transaction
/// holds its gate across key transitions and the pointer action, preventing two
/// Frame-scoped Locator calls for the same Page from interleaving input state.
#[derive(Clone, Default)]
pub struct CdpInputCoordinator {
    state: Arc<Mutex<CdpInputState>>,
}

impl CdpInputCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// Reports whether two routes serialize through the same Page input state.
    pub fn shares_state_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.state, &other.state)
    }

    /// Starts one serialized input transaction.
    ///
    /// `None` preserves the currently pressed modifiers. `Some(NONE)` ensures
    /// no modifiers are pressed and restores the prior state afterward.
    pub async fn begin_transaction(
        &self,
        client: Arc<dyn CdpClient>,
        requested: Option<CdpModifiers>,
    ) -> CdpResult<CdpInputTransaction> {
        let mut state = Arc::clone(&self.state).lock_owned().await;
        cleanup_locked(client.as_ref(), &mut state).await?;

        if let Some(requested) = requested {
            let restore = state.pressed_modifiers;
            state.pending_restore = Some(restore);
            if let Err(error) = transition_modifiers(client.as_ref(), &mut state, requested).await {
                let _ = cleanup_locked(client.as_ref(), &mut state).await;
                return Err(error);
            }
        }

        Ok(CdpInputTransaction {
            client,
            state: Some(state),
        })
    }

    /// Restores a transaction whose future was dropped after changing keys.
    /// Controlled runtimes call this from command cancellation. A subsequent
    /// input transaction also performs the same recovery before it can start.
    pub async fn cleanup_pending(&self, client: &dyn CdpClient) -> CdpResult<()> {
        let mut state = self.state.lock().await;
        cleanup_locked(client, &mut state).await
    }
}

/// Exclusive Page input transaction. Call `finish` after the pointer action to
/// restore an explicitly requested temporary modifier set.
pub struct CdpInputTransaction {
    client: Arc<dyn CdpClient>,
    state: Option<OwnedMutexGuard<CdpInputState>>,
}

impl CdpInputTransaction {
    pub fn modifiers(&self) -> CdpModifiers {
        self.state
            .as_ref()
            .map_or(CdpModifiers::NONE, |state| state.pressed_modifiers)
    }

    /// Dispatches one Playwright-shaped key down and records acknowledged
    /// physical-key state for repeat detection and cancellation cleanup.
    pub async fn key_down(&mut self, key: ResolvedKey, is_mac: bool) -> CdpResult<()> {
        let state = self.state.as_mut().expect("input transaction owns state");
        let repeated = state.pressed_keys[..state.pressed_key_count]
            .iter()
            .flatten()
            .any(|pressed| pressed.code == key.code);
        if !repeated && state.pressed_key_count == INPUT_CHORD_MAX_KEYS {
            return Err(CdpError::LimitExceeded {
                resource: "pressed keyboard keys",
                actual: state.pressed_key_count.saturating_add(1),
                limit: INPUT_CHORD_MAX_KEYS,
            });
        }
        let next_modifiers = key.modifier.map_or(state.pressed_modifiers, |modifier| {
            state.pressed_modifiers.with(modifier)
        });
        let has_non_shift_modifier = next_modifiers.contains(CdpModifiers::ALT)
            || next_modifiers.contains(CdpModifiers::CONTROL)
            || next_modifiers.contains(CdpModifiers::META);
        let text = (!has_non_shift_modifier && !key.text.is_empty()).then_some(key.text);
        let commands = if is_mac {
            mac_editing_commands(key.code, next_modifiers)
        } else {
            &[]
        };
        self.client
            .input_dispatch_keyboard_event(KeyboardEventParams {
                event_type: if text.is_some() {
                    KeyEventType::KeyDown
                } else {
                    KeyEventType::RawKeyDown
                },
                modifiers: next_modifiers.mask(),
                key: key.key,
                code: key.code,
                windows_virtual_key_code: key.windows_virtual_key_code,
                text,
                unmodified_text: text,
                auto_repeat: repeated,
                location: key.location,
                is_keypad: key.is_keypad(),
                commands,
            })
            .await?;
        state.pressed_modifiers = next_modifiers;
        if !repeated {
            let index = state.pressed_key_count;
            state.pressed_keys[index] = Some(key);
            state.pressed_key_count += 1;
        }
        Ok(())
    }

    /// Dispatches one key up. State is removed only after Chromium
    /// acknowledges the event, leaving failed releases visible to cleanup.
    pub async fn key_up(&mut self, key: ResolvedKey) -> CdpResult<()> {
        let state = self.state.as_mut().expect("input transaction owns state");
        release_key(self.client.as_ref(), state, key).await
    }

    /// Moves the Page pointer and records the position for subsequent button
    /// transitions and cancellation cleanup.
    pub async fn move_mouse(&mut self, x: f64, y: f64) -> CdpResult<()> {
        self.move_mouse_in_steps(x, y, NonZeroU32::MIN).await
    }

    /// Moves the pointer through exactly `steps` linearly interpolated CDP
    /// events. The final event always lands on `(x, y)`.
    pub async fn move_mouse_in_steps(
        &mut self,
        x: f64,
        y: f64,
        steps: NonZeroU32,
    ) -> CdpResult<()> {
        let state = self.state.as_mut().expect("input transaction owns state");
        let start_x = state.pointer_x;
        let start_y = state.pointer_y;
        let steps = steps.get();
        for step in 1..=steps {
            let progress = f64::from(step) / f64::from(steps);
            let next_x = start_x + (x - start_x) * progress;
            let next_y = start_y + (y - start_y) * progress;
            self.client
                .input_dispatch_mouse_event(
                    MouseEventType::Moved,
                    next_x,
                    next_y,
                    None,
                    None,
                    Some(buttons_mask(state.pressed_button)),
                    Some(state.pressed_modifiers.mask()),
                )
                .await?;
            state.pointer_x = next_x;
            state.pointer_y = next_y;
        }
        Ok(())
    }

    /// Presses one mouse button at the transaction's current pointer.
    pub async fn mouse_down(&mut self, button: MouseButton, click_count: i32) -> CdpResult<()> {
        let state = self.state.as_mut().expect("input transaction owns state");
        if state.pressed_button.is_some() {
            return Err(CdpError::Other(
                "Page input transaction already owns a pressed mouse button".to_string(),
            ));
        }
        let pressed = PressedMouseButton {
            button,
            click_count,
        };
        self.client
            .input_dispatch_mouse_event(
                MouseEventType::Pressed,
                state.pointer_x,
                state.pointer_y,
                Some(button),
                Some(click_count),
                Some(buttons_mask(Some(pressed))),
                Some(state.pressed_modifiers.mask()),
            )
            .await?;
        state.pressed_button = Some(pressed);
        Ok(())
    }

    /// Releases the transaction's pressed mouse button, if any.
    pub async fn mouse_up(&mut self) -> CdpResult<()> {
        let state = self.state.as_mut().expect("input transaction owns state");
        release_pressed_button(self.client.as_ref(), state).await
    }

    pub async fn finish(mut self) -> CdpResult<()> {
        let state = self.state.as_mut().expect("input transaction owns state");
        cleanup_locked(self.client.as_ref(), state).await?;
        self.state.take();
        Ok(())
    }
}

async fn cleanup_locked(client: &dyn CdpClient, state: &mut CdpInputState) -> CdpResult<()> {
    while state.pressed_key_count > 0 {
        let key = state.pressed_keys[state.pressed_key_count - 1]
            .expect("pressed key slots form a dense prefix");
        release_key(client, state, key).await?;
    }
    release_pressed_button(client, state).await?;
    let Some(restore) = state.pending_restore else {
        return Ok(());
    };
    transition_modifiers(client, state, restore).await?;
    state.pending_restore = None;
    Ok(())
}

async fn release_key(
    client: &dyn CdpClient,
    state: &mut CdpInputState,
    key: ResolvedKey,
) -> CdpResult<()> {
    let next_modifiers = key.modifier.map_or(state.pressed_modifiers, |modifier| {
        state.pressed_modifiers.without(modifier)
    });
    client
        .input_dispatch_keyboard_event(KeyboardEventParams {
            event_type: KeyEventType::KeyUp,
            modifiers: next_modifiers.mask(),
            key: key.key,
            code: key.code,
            windows_virtual_key_code: key.windows_virtual_key_code,
            text: None,
            unmodified_text: None,
            auto_repeat: false,
            location: key.location,
            is_keypad: key.is_keypad(),
            commands: &[],
        })
        .await?;
    state.pressed_modifiers = next_modifiers;
    if let Some(index) = state.pressed_keys[..state.pressed_key_count]
        .iter()
        .position(|pressed| pressed.is_some_and(|pressed| pressed.code == key.code))
    {
        for offset in index..state.pressed_key_count - 1 {
            state.pressed_keys[offset] = state.pressed_keys[offset + 1];
        }
        state.pressed_key_count -= 1;
        state.pressed_keys[state.pressed_key_count] = None;
    }
    Ok(())
}

async fn release_pressed_button(
    client: &dyn CdpClient,
    state: &mut CdpInputState,
) -> CdpResult<()> {
    let Some(pressed) = state.pressed_button else {
        return Ok(());
    };
    client
        .input_dispatch_mouse_event(
            MouseEventType::Released,
            state.pointer_x,
            state.pointer_y,
            Some(pressed.button),
            Some(pressed.click_count),
            Some(0),
            Some(state.pressed_modifiers.mask()),
        )
        .await?;
    state.pressed_button = None;
    Ok(())
}

const fn buttons_mask(pressed: Option<PressedMouseButton>) -> i32 {
    match pressed {
        None => 0,
        Some(PressedMouseButton {
            button: MouseButton::Left,
            ..
        }) => 1,
        Some(PressedMouseButton {
            button: MouseButton::Right,
            ..
        }) => 2,
        Some(PressedMouseButton {
            button: MouseButton::Middle,
            ..
        }) => 4,
    }
}

async fn transition_modifiers(
    client: &dyn CdpClient,
    state: &mut CdpInputState,
    requested: CdpModifiers,
) -> CdpResult<()> {
    for key in MODIFIER_KEYS {
        let is_pressed = state.pressed_modifiers.contains(key.modifier);
        let needs_pressed = requested.contains(key.modifier);
        if is_pressed == needs_pressed {
            continue;
        }
        let next = if needs_pressed {
            state.pressed_modifiers.with(key.modifier)
        } else {
            state.pressed_modifiers.without(key.modifier)
        };
        client
            .input_dispatch_key_event_with_modifiers(
                if needs_pressed {
                    KeyEventType::RawKeyDown
                } else {
                    KeyEventType::KeyUp
                },
                key.key,
                key.code,
                Some(key.windows_virtual_key_code),
                Some(next.mask()),
            )
            .await?;
        state.pressed_modifiers = next;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use std::time::Duration;

    use pwright_cdp::{CdpClient, MouseButton};
    use pwright_fake::FakeCdpClient;

    use super::{CdpInputCoordinator, CdpModifiers};
    use crate::keyboard::resolve_key;

    #[tokio::test]
    async fn keyboard_transaction_dispatches_shifted_text_and_cleans_up_in_reverse_order() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let coordinator = CdpInputCoordinator::new();
        let mut transaction = coordinator
            .begin_transaction(client.clone(), None)
            .await
            .expect("keyboard transaction should start");
        let shift = resolve_key("Shift", false, true).expect("Shift should resolve");
        let key_a = resolve_key("KeyA", true, true).expect("KeyA should resolve");
        transaction
            .key_down(shift, true)
            .await
            .expect("Shift should press");
        transaction
            .key_down(key_a, true)
            .await
            .expect("A should press");

        // Act
        drop(transaction);
        coordinator
            .cleanup_pending(client.as_ref())
            .await
            .expect("cleanup should release both keys");

        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 4);
        assert_eq!(
            (
                calls[0].1[0]["type"].as_str(),
                calls[0].1[0]["modifiers"].as_i64()
            ),
            (Some("rawKeyDown"), Some(8))
        );
        assert_eq!(
            (
                calls[1].1[0]["type"].as_str(),
                calls[1].1[0]["key"].as_str(),
                calls[1].1[0]["text"].as_str()
            ),
            (Some("keyDown"), Some("A"), Some("A"))
        );
        assert_eq!(
            (
                calls[2].1[0]["type"].as_str(),
                calls[2].1[0]["code"].as_str()
            ),
            (Some("keyUp"), Some("KeyA"))
        );
        assert_eq!(
            (
                calls[3].1[0]["type"].as_str(),
                calls[3].1[0]["code"].as_str(),
                calls[3].1[0]["modifiers"].as_i64()
            ),
            (Some("keyUp"), Some("ShiftLeft"), Some(0))
        );
    }

    #[tokio::test]
    async fn repeated_physical_down_sets_auto_repeat_without_growing_cleanup_state() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let coordinator = CdpInputCoordinator::new();
        let mut transaction = coordinator
            .begin_transaction(client.clone(), None)
            .await
            .expect("keyboard transaction should start");
        let key = resolve_key("a", false, false).expect("a should resolve");

        // Act
        transaction.key_down(key, false).await.expect("first down");
        transaction.key_down(key, false).await.expect("repeat down");
        transaction
            .finish()
            .await
            .expect("cleanup should release once");

        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].1[0]["autoRepeat"], false);
        assert_eq!(calls[1].1[0]["autoRepeat"], true);
        assert_eq!(calls[2].1[0]["type"], "keyUp");
    }

    #[tokio::test]
    async fn modifier_transaction_transitions_in_canonical_order_and_restores() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let coordinator = CdpInputCoordinator::new();
        let desired = CdpModifiers::META.with(CdpModifiers::SHIFT);

        // Act
        let transaction = coordinator
            .begin_transaction(client.clone(), Some(desired))
            .await
            .expect("modifier transaction should start");
        let active = transaction.modifiers();
        transaction
            .finish()
            .await
            .expect("modifier transaction should restore");

        // Assert
        assert_eq!(active, desired);
        let calls = client.calls();
        assert_eq!(client.method_names(), vec!["Input.dispatchKeyEvent"; 4]);
        assert_eq!(calls[0].1[0]["type"], "rawKeyDown");
        assert_eq!(calls[0].1[0]["key"], "Meta");
        assert_eq!(calls[0].1[0]["modifiers"], 4);
        assert_eq!(calls[1].1[0]["key"], "Shift");
        assert_eq!(calls[1].1[0]["modifiers"], 12);
        assert_eq!(calls[2].1[0]["type"], "keyUp");
        assert_eq!(calls[2].1[0]["key"], "Meta");
        assert_eq!(calls[2].1[0]["modifiers"], 8);
        assert_eq!(calls[3].1[0]["key"], "Shift");
        assert_eq!(calls[3].1[0]["modifiers"], 0);
    }

    #[tokio::test]
    async fn dropped_transaction_is_recovered_before_later_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let coordinator = CdpInputCoordinator::new();
        let transaction = coordinator
            .begin_transaction(client.clone(), Some(CdpModifiers::ALT))
            .await
            .expect("modifier transaction should start");

        // Act
        drop(transaction);
        coordinator
            .cleanup_pending(client.as_ref())
            .await
            .expect("dropped transaction should restore");

        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1[0]["type"], "rawKeyDown");
        assert_eq!(calls[1].1[0]["type"], "keyUp");
        assert_eq!(calls[1].1[0]["modifiers"], 0);
    }

    #[tokio::test]
    async fn partial_modifier_transition_failure_restores_already_pressed_keys() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_input_errors([None, Some("Shift key-down failed".to_string()), None]);
        let coordinator = CdpInputCoordinator::new();
        let desired = CdpModifiers::META.with(CdpModifiers::SHIFT);

        // Act
        let error = coordinator
            .begin_transaction(client.clone(), Some(desired))
            .await
            .err()
            .expect("partial transition should fail");

        // Assert
        assert!(error.to_string().contains("Shift key-down failed"));
        let calls = client.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].1[0]["key"], "Meta");
        assert_eq!(calls[0].1[0]["type"], "rawKeyDown");
        assert_eq!(calls[1].1[0]["key"], "Shift");
        assert_eq!(calls[2].1[0]["key"], "Meta");
        assert_eq!(calls[2].1[0]["type"], "keyUp");
        assert_eq!(calls[2].1[0]["modifiers"], 0);
    }

    #[tokio::test]
    async fn dropped_mouse_down_is_released_at_the_recorded_pointer_before_modifier_restore() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let coordinator = CdpInputCoordinator::new();
        let mut transaction = coordinator
            .begin_transaction(client.clone(), Some(CdpModifiers::SHIFT))
            .await
            .expect("input transaction should start");
        transaction
            .move_mouse(12.5, 19.25)
            .await
            .expect("pointer should move");
        transaction
            .mouse_down(MouseButton::Right, 2)
            .await
            .expect("mouse button should press");

        // Act
        drop(transaction);
        coordinator
            .cleanup_pending(client.as_ref())
            .await
            .expect("cleanup should release mouse and modifier state");

        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 5);
        assert_eq!(calls[1].1[0]["type"], "mouseMoved");
        assert_eq!(calls[2].1[0]["type"], "mousePressed");
        assert_eq!(calls[2].1[0]["buttons"], 2);
        assert_eq!(calls[3].1[0]["type"], "mouseReleased");
        assert_eq!(calls[3].1[0]["x"], 12.5);
        assert_eq!(calls[3].1[0]["y"], 19.25);
        assert_eq!(calls[3].1[0]["buttons"], 0);
        assert_eq!(calls[3].1[0]["modifiers"], 8);
        assert_eq!(calls[4].1[0]["type"], "keyUp");
        assert_eq!(calls[4].1[0]["modifiers"], 0);
    }

    #[tokio::test]
    async fn stepped_mouse_move_interpolates_from_the_page_cursor_and_lands_exactly() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let coordinator = CdpInputCoordinator::new();
        let mut transaction = coordinator
            .begin_transaction(client.clone(), None)
            .await
            .expect("input transaction should start");

        // Act
        transaction
            .move_mouse_in_steps(9.0, 6.0, NonZeroU32::new(3).expect("three is nonzero"))
            .await
            .expect("stepped pointer move should succeed");
        transaction
            .mouse_down(MouseButton::Left, 1)
            .await
            .expect("button should press at the final point");
        transaction.finish().await.expect("input should clean up");

        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 5);
        assert_eq!(
            (calls[0].1[0]["x"].as_f64(), calls[0].1[0]["y"].as_f64()),
            (Some(3.0), Some(2.0))
        );
        assert_eq!(
            (calls[1].1[0]["x"].as_f64(), calls[1].1[0]["y"].as_f64()),
            (Some(6.0), Some(4.0))
        );
        assert_eq!(
            (calls[2].1[0]["x"].as_f64(), calls[2].1[0]["y"].as_f64()),
            (Some(9.0), Some(6.0))
        );
        assert_eq!(
            (calls[3].1[0]["x"].as_f64(), calls[3].1[0]["y"].as_f64()),
            (Some(9.0), Some(6.0))
        );
        assert_eq!(calls[4].1[0]["type"], "mouseReleased");
    }

    #[tokio::test]
    async fn page_gate_prevents_modifier_transactions_from_interleaving() {
        // Arrange
        let client: Arc<dyn CdpClient> = Arc::new(FakeCdpClient::from_html(""));
        let coordinator = CdpInputCoordinator::new();
        let first = coordinator
            .begin_transaction(client.clone(), Some(CdpModifiers::SHIFT))
            .await
            .expect("first transaction should start");
        let waiting = coordinator.begin_transaction(client.clone(), None);

        // Act
        let blocked = tokio::time::timeout(Duration::from_millis(1), waiting).await;
        first
            .finish()
            .await
            .expect("first transaction should finish");
        let second = coordinator
            .begin_transaction(client, None)
            .await
            .expect("second transaction should start after release");

        // Assert
        assert!(blocked.is_err());
        assert_eq!(second.modifiers(), CdpModifiers::NONE);
        second
            .finish()
            .await
            .expect("second transaction should finish");
    }
}
