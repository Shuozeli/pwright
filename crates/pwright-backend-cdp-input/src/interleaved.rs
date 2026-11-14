//! Opt-in Page-owned input state. Keyboard and Locator serialize native events;
//! raw Mouse captures state before awaiting replies and permits in-flight overlap.
//! No lock survives a caller delay, selector retry or whole input action.
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use pwright_cdp::connection::{CdpError, Result};
use pwright_cdp::{
    CdpClient, KeyEventType, KeyboardEventParams, MouseButton, MouseEventType, TouchEventType,
};
use pwright_input_model::{INPUT_CHORD_MAX_KEYS, InputText};

use crate::input::CdpModifiers;
use crate::keyboard::{ResolvedKey, mac_editing_commands, resolve_key};

mod pointer;
pub use pointer::button::MouseButtonExecution;
pub use pointer::click::{MouseClickExecution, MouseClickOptions};
pub use pointer::dispatch::{
    CdpMouseDispatcher, DirectMouseDispatch, MouseDispatchKind, MouseDispatchOperation,
    MouseDispatchRequest, MouseLiveState, MouseStateReader,
};

#[derive(Default)]
pub(crate) struct PageState {
    state: Mutex<State>,
    dispatch: tokio::sync::Mutex<()>,
}

#[derive(Default)]
struct State {
    modifiers: CdpModifiers,
    keys: [Option<ResolvedKey>; INPUT_CHORD_MAX_KEYS],
    x: f64,
    y: f64,
    buttons: i32,
    last_button: Option<MouseButton>,
    next_owned_press_generation: u64,
    owned_presses: [Option<OwnedPress>; 3],
    active_touch: Option<ActiveTouch>,
}

#[derive(Clone, Copy)]
struct OwnedPress {
    generation: u64,
    click_count: i32,
}

/// An invocation-local claim on a Page mouse press. Ordinary Mouse and
/// Locator actions do not create one, and dropping it never releases input.
#[derive(Clone)]
pub struct CdpOwnedMousePress {
    page: Arc<PageState>,
    generation: u64,
    _admission: Option<Arc<crate::production::InputAdmission>>,
}

/// A retained pointer press can fail either its Page/world check or CDP input.
/// Keep these distinct so the caller preserves the original failure kind.
#[derive(Debug)]
pub enum CdpCheckedMouseDownError<E> {
    Check(E),
    Input(CdpError),
}

#[derive(Clone, Copy)]
struct ActiveTouch {
    x: f64,
    y: f64,
    modifiers: i32,
}

impl PageState {
    pub(crate) fn new_owned_mouse_press(
        page: &Arc<Self>,
        admission: Option<Arc<crate::production::InputAdmission>>,
    ) -> Result<CdpOwnedMousePress> {
        let mut state = page.state.lock().expect("input state is not poisoned");
        let generation = state
            .next_owned_press_generation
            .checked_add(1)
            .ok_or_else(|| CdpError::Other("Page mouse press generations exhausted".into()))?;
        state.next_owned_press_generation = generation;
        Ok(CdpOwnedMousePress {
            page: Arc::clone(page),
            generation,
            _admission: admission,
        })
    }

    pub(crate) async fn release_owned_mouse_press(
        &self,
        client: &dyn CdpClient,
        press: &CdpOwnedMousePress,
    ) -> Result<()> {
        if !std::ptr::eq(self, press.page.as_ref()) {
            return Err(CdpError::Other(
                "owned mouse press belongs to another Page".into(),
            ));
        }
        // A Locator down/up holds this gate through native acknowledgement.
        // No other Locator dispatch can take over the button between checking
        // the generation and submitting this compensating release.
        let _dispatch = self.dispatch.lock().await;
        for button in [MouseButton::Left, MouseButton::Right, MouseButton::Middle] {
            let release = {
                let mut state = self.state.lock().expect("input state is not poisoned");
                let index = button_index(button);
                let Some(owned) = state.owned_presses[index] else {
                    continue;
                };
                if owned.generation != press.generation {
                    continue;
                }
                state.owned_presses[index] = None;
                state.buttons &= !button_mask(button);
                state.last_button = None;
                (
                    state.x,
                    state.y,
                    state.buttons,
                    state.modifiers.mask(),
                    owned.click_count,
                )
            };
            client
                .input_dispatch_mouse_event(
                    MouseEventType::Released,
                    release.0,
                    release.1,
                    Some(button),
                    Some(release.4),
                    Some(release.2),
                    Some(release.3),
                )
                .await?;
        }
        Ok(())
    }

    pub(crate) async fn cleanup_touch(&self, client: &dyn CdpClient) -> Result<()> {
        let _dispatch = self.dispatch.lock().await;
        let active = self
            .state
            .lock()
            .expect("input state is not poisoned")
            .active_touch;
        let Some(active) = active else {
            return Ok(());
        };
        client
            .input_dispatch_touch_event_with_modifiers(
                TouchEventType::End,
                active.x,
                active.y,
                Some(active.modifiers),
            )
            .await?;
        self.state
            .lock()
            .expect("input state is not poisoned")
            .active_touch = None;
        Ok(())
    }
}

pub(crate) struct Action {
    page: Arc<PageState>,
    client: Arc<dyn CdpClient>,
    restore: Option<CdpModifiers>,
    button: Option<(MouseButton, i32)>,
    owned_press_generation: Option<u64>,
    pub(crate) key_down_interceptor: Option<Arc<dyn crate::CdpKeyDownInterceptor>>,
}

impl Action {
    pub fn raw(page: Arc<PageState>, client: Arc<dyn CdpClient>) -> Self {
        Self {
            page,
            client,
            restore: None,
            button: None,
            owned_press_generation: None,
            key_down_interceptor: None,
        }
    }

    pub async fn begin(
        page: Arc<PageState>,
        client: Arc<dyn CdpClient>,
        requested: Option<CdpModifiers>,
    ) -> Result<Self> {
        let mut action = Self::raw(page, client);
        if let Some(requested) = requested {
            action.restore = Some(action.modifiers());
            action.transition(requested).await?;
        }
        Ok(action)
    }

    pub fn modifiers(&self) -> CdpModifiers {
        self.page
            .state
            .lock()
            .expect("input state is not poisoned")
            .modifiers
    }

    pub fn set_owned_mouse_press(&mut self, press: &CdpOwnedMousePress) -> Result<()> {
        if !Arc::ptr_eq(&self.page, &press.page)
            || self.owned_press_generation.is_some()
            || self.button.is_some()
        {
            return Err(CdpError::Other(
                "owned mouse press is already selected or belongs to another Page".into(),
            ));
        }
        self.owned_press_generation = Some(press.generation);
        Ok(())
    }

    pub async fn insert_text(&mut self, text: InputText<'_>) -> Result<()> {
        let _dispatch = self.page.dispatch.lock().await;
        self.client.input_insert_text(text.as_str()).await
    }

    pub async fn key_down(&mut self, key: ResolvedKey, is_mac: bool) -> Result<()> {
        let _dispatch = self.page.dispatch.lock().await;
        self.dispatch_key_down(key, is_mac).await
    }

    pub async fn key_down_named<'a>(
        &mut self,
        name: &'a str,
        is_mac: bool,
    ) -> std::result::Result<(), crate::KeyInputError<'a>> {
        let _dispatch = self.page.dispatch.lock().await;
        let key = resolve_key(name, self.modifiers().contains(CdpModifiers::SHIFT), is_mac)
            .map_err(|error| crate::KeyInputError::UnknownKey(error.0))?;
        self.dispatch_key_down(key, is_mac)
            .await
            .map_err(Into::into)
    }

    async fn dispatch_key_down(&self, key: ResolvedKey, is_mac: bool) -> Result<()> {
        // Like upstream, Page logical state changes before native dispatch. A
        // cancelled/failed dispatch does not turn into a synthetic key release.
        let (modifiers, repeated) = {
            let mut state = self.page.state.lock().expect("input state is not poisoned");
            let repeated = state
                .keys
                .iter()
                .flatten()
                .any(|held| held.code == key.code);
            if !repeated {
                let slot = state.keys.iter_mut().find(|slot| slot.is_none()).ok_or(
                    CdpError::LimitExceeded {
                        resource: "pressed keyboard keys",
                        actual: INPUT_CHORD_MAX_KEYS + 1,
                        limit: INPUT_CHORD_MAX_KEYS,
                    },
                )?;
                *slot = Some(key);
            }
            if let Some(modifier) = key.modifier {
                state.modifiers = state.modifiers.with(modifier);
            }
            (state.modifiers, repeated)
        };
        let non_shift = modifiers.contains(CdpModifiers::ALT)
            || modifiers.contains(CdpModifiers::CONTROL)
            || modifiers.contains(CdpModifiers::META);
        let text = (!non_shift && !key.text.is_empty()).then_some(key.text);
        if let Some(interceptor) = &self.key_down_interceptor
            && let Some(interception) = interceptor.clone().intercept(self.client.clone(), key.code)
            && interception.await?
        {
            return Ok(());
        }
        self.client
            .input_dispatch_keyboard_event(KeyboardEventParams {
                event_type: if text.is_some() {
                    KeyEventType::KeyDown
                } else {
                    KeyEventType::RawKeyDown
                },
                modifiers: modifiers.mask(),
                key: key.key,
                code: key.code,
                windows_virtual_key_code: key.windows_virtual_key_code,
                text,
                unmodified_text: text,
                auto_repeat: repeated,
                location: key.location,
                is_keypad: key.is_keypad(),
                commands: if is_mac {
                    mac_editing_commands(key.code, modifiers)
                } else {
                    &[]
                },
            })
            .await
    }

    pub async fn key_up(&mut self, key: ResolvedKey) -> Result<()> {
        let _dispatch = self.page.dispatch.lock().await;
        self.dispatch_key_up(key).await
    }

    pub async fn key_up_named<'a>(
        &mut self,
        name: &'a str,
        is_mac: bool,
    ) -> std::result::Result<(), crate::KeyInputError<'a>> {
        let _dispatch = self.page.dispatch.lock().await;
        let key = resolve_key(name, self.modifiers().contains(CdpModifiers::SHIFT), is_mac)
            .map_err(|error| crate::KeyInputError::UnknownKey(error.0))?;
        self.dispatch_key_up(key).await.map_err(Into::into)
    }

    async fn dispatch_key_up(&self, key: ResolvedKey) -> Result<()> {
        let modifiers = {
            let mut state = self.page.state.lock().expect("input state is not poisoned");
            if let Some(modifier) = key.modifier {
                state.modifiers = state.modifiers.without(modifier);
            }
            for slot in &mut state.keys {
                if slot.is_some_and(|held| held.code == key.code) {
                    *slot = None;
                }
            }
            state.modifiers
        };
        self.client
            .input_dispatch_keyboard_event(KeyboardEventParams {
                event_type: KeyEventType::KeyUp,
                modifiers: modifiers.mask(),
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
            .await
    }

    pub async fn move_mouse(&mut self, x: f64, y: f64) -> Result<()> {
        self.move_mouse_in_steps(x, y, NonZeroU32::MIN).await
    }

    pub async fn move_mouse_in_steps(&mut self, x: f64, y: f64, steps: NonZeroU32) -> Result<()> {
        let (start_x, start_y) = {
            let mut state = self.page.state.lock().expect("input state is not poisoned");
            let start = (state.x, state.y);
            state.x = x;
            state.y = y;
            start
        };
        for step in 1..=steps.get() {
            let _dispatch = self.page.dispatch.lock().await;
            let progress = f64::from(step) / f64::from(steps.get());
            let x = start_x + (x - start_x) * progress;
            let y = start_y + (y - start_y) * progress;
            let (buttons, modifiers) = {
                let state = self.page.state.lock().expect("input state is not poisoned");
                (state.buttons, state.modifiers.mask())
            };
            self.client
                .input_dispatch_mouse_event(
                    MouseEventType::Moved,
                    x,
                    y,
                    None,
                    None,
                    Some(buttons),
                    Some(modifiers),
                )
                .await?;
        }
        Ok(())
    }

    pub async fn mouse_down(&mut self, button: MouseButton, count: i32) -> Result<()> {
        let page = Arc::clone(&self.page);
        let _dispatch = page.dispatch.lock().await;
        self.send_mouse_down(button, count).await
    }

    /// An opt-in owned press checks its source after entering the Page's native
    /// dispatch gate, not before an arbitrarily long wait for that gate.
    pub async fn mouse_down_checked<E>(
        &mut self,
        button: MouseButton,
        count: i32,
        check: impl FnOnce() -> std::result::Result<(), E> + Send,
    ) -> std::result::Result<(), CdpCheckedMouseDownError<E>> {
        if self.owned_press_generation.is_none() {
            return Err(CdpCheckedMouseDownError::Input(CdpError::Other(
                "checked mouse down requires an owned Page press".into(),
            )));
        }
        let page = Arc::clone(&self.page);
        let _dispatch = page.dispatch.lock().await;
        check().map_err(CdpCheckedMouseDownError::Check)?;
        self.send_mouse_down(button, count)
            .await
            .map_err(CdpCheckedMouseDownError::Input)
    }

    async fn send_mouse_down(&mut self, button: MouseButton, count: i32) -> Result<()> {
        let (x, y, buttons, modifiers) = {
            let mut state = self.page.state.lock().expect("input state is not poisoned");
            state.buttons |= button_mask(button);
            state.last_button = Some(button);
            state.owned_presses[button_index(button)] =
                self.owned_press_generation.map(|generation| OwnedPress {
                    generation,
                    click_count: count,
                });
            (state.x, state.y, state.buttons, state.modifiers.mask())
        };
        self.button = Some((button, count));
        self.client
            .input_dispatch_mouse_event(
                MouseEventType::Pressed,
                x,
                y,
                Some(button),
                Some(count),
                Some(buttons),
                Some(modifiers),
            )
            .await
    }

    pub async fn mouse_up(&mut self) -> Result<()> {
        let _dispatch = self.page.dispatch.lock().await;
        let Some((button, count)) = self.button.take() else {
            return Ok(());
        };
        let (x, y, buttons, modifiers) = {
            let mut state = self.page.state.lock().expect("input state is not poisoned");
            state.buttons &= !button_mask(button);
            state.last_button = None;
            state.owned_presses[button_index(button)] = None;
            (state.x, state.y, state.buttons, state.modifiers.mask())
        };
        self.client
            .input_dispatch_mouse_event(
                MouseEventType::Released,
                x,
                y,
                Some(button),
                Some(count),
                Some(buttons),
                Some(modifiers),
            )
            .await
    }

    pub async fn touchscreen_tap(&mut self, x: f64, y: f64) -> Result<()> {
        // Keep the two halves adjacent to all Page keyboard, mouse and touch
        // events while retaining independently mutable physical Page state.
        // Chromium Playwright submits both native commands before awaiting
        // either response. In particular, touchEnd must already be on the wire
        // before a synthesized click starts a navigation that can stall the
        // renderer-side acknowledgement path.
        let _dispatch = self.page.dispatch.lock().await;
        let modifiers = self.modifiers().mask();
        self.page
            .state
            .lock()
            .expect("input state is not poisoned")
            .active_touch = Some(ActiveTouch { x, y, modifiers });
        let start = self.client.input_dispatch_touch_event_with_modifiers(
            TouchEventType::Start,
            x,
            y,
            Some(modifiers),
        );
        let end = self.client.input_dispatch_touch_event_with_modifiers(
            TouchEventType::End,
            x,
            y,
            Some(modifiers),
        );
        let (start_result, end_result) = tokio::join!(start, end);
        if end_result.is_ok() {
            self.page
                .state
                .lock()
                .expect("input state is not poisoned")
                .active_touch = None;
        }
        start_result?;
        end_result
    }

    async fn transition(&mut self, requested: CdpModifiers) -> Result<()> {
        for (modifier, name) in [
            (CdpModifiers::ALT, "Alt"),
            (CdpModifiers::CONTROL, "Control"),
            (CdpModifiers::META, "Meta"),
            (CdpModifiers::SHIFT, "Shift"),
        ] {
            let held = self.modifiers().contains(modifier);
            let wanted = requested.contains(modifier);
            if held == wanted {
                continue;
            }
            let key = resolve_key(name, false, false).expect("canonical modifier key");
            if wanted {
                self.key_down(key, false).await?;
            } else {
                self.key_up(key).await?;
            }
        }
        Ok(())
    }

    pub async fn finish(mut self) -> Result<()> {
        if let Some(restore) = self.restore {
            self.transition(restore).await?;
        }
        // No release-all: physical keys/buttons belong to the Page, not this
        // action. Explicit up still releases the code even if another action
        // pressed it too. This is not reference-counted key ownership.
        Ok(())
    }
}

fn button_mask(button: MouseButton) -> i32 {
    match button {
        MouseButton::Left => 1,
        MouseButton::Right => 2,
        MouseButton::Middle => 4,
    }
}

fn button_index(button: MouseButton) -> usize {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CdpInputCoordinator, CdpInputPolicy, type_text};
    use pwright_fake::FakeCdpClient;
    use std::{
        future::{Future, pending, ready},
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
        task::{Context, Waker},
    };

    #[tokio::test]
    async fn checked_owned_press_revalidates_after_page_dispatch_wait() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let page = Arc::new(PageState::default());
        let owner = PageState::new_owned_mouse_press(&page, None).unwrap();
        let mut action = Action::raw(Arc::clone(&page), client.clone());
        action.set_owned_mouse_press(&owner).unwrap();
        let gate = page.dispatch.lock().await;
        let world_current = Arc::new(AtomicBool::new(true));
        let checks = Arc::new(AtomicUsize::new(0));
        let check_world = {
            let world_current = Arc::clone(&world_current);
            let checks = Arc::clone(&checks);
            move || {
                checks.fetch_add(1, Ordering::SeqCst);
                if world_current.load(Ordering::SeqCst) {
                    Ok(())
                } else {
                    Err("observed world retired")
                }
            }
        };

        // Act
        let mut down = Box::pin(action.mouse_down_checked(MouseButton::Left, 1, check_world));
        assert!(
            down.as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        assert_eq!(checks.load(Ordering::SeqCst), 0);
        world_current.store(false, Ordering::SeqCst);
        drop(gate);
        let result = down.await;

        // Assert
        assert!(matches!(
            result,
            Err(CdpCheckedMouseDownError::Check("observed world retired"))
        ));
        assert_eq!(checks.load(Ordering::SeqCst), 1);
        assert!(client.calls().is_empty());
    }

    #[tokio::test]
    async fn second_action_progresses_during_first_delay_without_releasing_first_key() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let mut first = input.begin_transaction(client.clone(), None).await.unwrap();
        let mut typing = Box::pin(type_text(
            &mut first,
            InputText::try_from("ab").unwrap(),
            false,
            pending::<Result<()>>,
        ));
        assert!(
            typing
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
                .is_pending()
        );
        // Act
        let mut second = input.begin_transaction(client.clone(), None).await.unwrap();
        type_text(
            &mut second,
            InputText::try_from("C").unwrap(),
            false,
            || ready(Ok(())),
        )
        .await
        .unwrap();
        second.finish().await.unwrap();
        drop(typing);
        drop(first);
        input.cleanup_pending(client.as_ref()).await.unwrap();
        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].1[0]["key"], "a");
        assert_eq!(calls[1].1[0]["key"], "C");
        assert_eq!(calls[2].1[0]["key"], "C");
        assert_eq!(calls[2].1[0]["type"], "keyUp");
    }

    #[tokio::test]
    async fn same_key_repeat_and_explicit_up_use_page_set_not_per_action_reference_count() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let mut first = input.begin_transaction(client.clone(), None).await.unwrap();
        let mut second = input.begin_transaction(client.clone(), None).await.unwrap();
        let key = resolve_key("a", false, false).unwrap();
        first.key_down(key, false).await.unwrap();
        // Act
        second.key_down(key, false).await.unwrap();
        second.key_up(key).await.unwrap();
        first.key_down(key, false).await.unwrap();
        first.key_up(key).await.unwrap();
        first.finish().await.unwrap();
        second.finish().await.unwrap();
        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 5);
        assert_eq!(calls[1].1[0]["autoRepeat"], true);
        assert_eq!(calls[3].1[0]["autoRepeat"], false);
    }

    #[tokio::test]
    async fn dispatch_failure_retains_logical_key_state_without_automatic_release() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_input_errors([Some("down failed".into()), None]);
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let mut first = input.begin_transaction(client.clone(), None).await.unwrap();
        let shift = resolve_key("Shift", false, false).unwrap();
        // Act
        let error = first.key_down(shift, false).await.unwrap_err();
        drop(first);
        input.cleanup_pending(client.as_ref()).await.unwrap();
        let mut second = input.begin_transaction(client.clone(), None).await.unwrap();
        second.key_down(shift, false).await.unwrap();
        let modifiers = second.modifiers();
        second.finish().await.unwrap();
        // Assert
        assert!(error.to_string().contains("down failed"));
        assert_eq!(modifiers, CdpModifiers::SHIFT);
        let calls = client.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].1[0]["autoRepeat"], true);
    }

    #[tokio::test]
    async fn fixed_page_key_bound_rejects_new_code_but_allows_repeat_and_reuse() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let mut action = input.begin_transaction(client.clone(), None).await.unwrap();
        let names = [
            "a", "b", "c", "d", "e", "f", "g", "h", "i", "j", "k", "l", "m", "n", "o", "p",
        ];
        assert_eq!(names.len(), INPUT_CHORD_MAX_KEYS);
        for name in names {
            action
                .key_down(resolve_key(name, false, false).unwrap(), false)
                .await
                .unwrap();
        }
        // Act
        let overflow = action
            .key_down(resolve_key("q", false, false).unwrap(), false)
            .await;
        action
            .key_down(resolve_key("a", false, false).unwrap(), false)
            .await
            .unwrap();
        action
            .key_up(resolve_key("a", false, false).unwrap())
            .await
            .unwrap();
        action
            .key_down(resolve_key("q", false, false).unwrap(), false)
            .await
            .unwrap();
        action.finish().await.unwrap();
        // Assert
        assert!(matches!(
            overflow,
            Err(CdpError::LimitExceeded {
                actual: 17,
                limit: 16,
                ..
            })
        ));
        let calls = client.calls();
        assert_eq!(calls.len(), 19);
        assert_eq!(calls[16].1[0]["autoRepeat"], true);
        assert_eq!(calls[18].1[0]["key"], "q");
    }

    #[tokio::test]
    async fn pointer_finish_restores_requested_modifiers_without_releasing_another_key() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let mut keyboard = input.begin_transaction(client.clone(), None).await.unwrap();
        keyboard
            .key_down(resolve_key("a", false, false).unwrap(), false)
            .await
            .unwrap();
        // Act
        let mut pointer = input
            .begin_transaction(client.clone(), Some(CdpModifiers::SHIFT))
            .await
            .unwrap();
        pointer.move_mouse(9., 6.).await.unwrap();
        pointer.mouse_down(MouseButton::Left, 1).await.unwrap();
        pointer.mouse_up().await.unwrap();
        pointer.finish().await.unwrap();
        keyboard.finish().await.unwrap();
        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 6);
        assert_eq!(calls[2].1[0]["modifiers"], 8);
        assert_eq!(calls[5].1[0]["key"], "Shift");
        assert_eq!(calls[5].1[0]["type"], "keyUp");
        assert_eq!(calls[5].1[0]["modifiers"], 0);
    }
}
