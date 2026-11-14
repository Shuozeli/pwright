//! Serialized, target-local CDP input state.
//!
//! Playwright models keyboard and mouse state on a Page rather than on an
//! individual Locator call. This coordinator is intentionally independent of
//! locator selectors and protocol commands so later keyboard, mouse, and touch
//! capabilities can share the same bounded state machine.

use std::num::NonZeroU32;
use std::sync::Arc;

use pwright_cdp::connection::{CdpError, Result as CdpResult};
use pwright_cdp::{
    CdpClient, KeyEventType, KeyboardEventParams, MouseButton, MouseEventType, TouchEventType,
};
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

    pub(crate) const fn without(self, modifier: Self) -> Self {
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
    active_touch: Option<ActiveTouch>,
}

#[derive(Clone, Copy)]
struct PressedMouseButton {
    button: MouseButton,
    click_count: i32,
}

#[derive(Clone, Copy)]
struct ActiveTouch {
    x: f64,
    y: f64,
    modifiers: i32,
}

/// One Page-scoped serialization gate and fixed-size input-state record.
///
/// The coordinator contains no unbounded collection. A modifier transaction
/// holds its gate across key transitions and the pointer action, preventing two
/// Frame-scoped Locator calls for the same Page from interleaving input state.
#[derive(Clone)]
pub struct CdpInputCoordinator {
    state: CoordinatorState,
    // Every clone keeps admission alive, including clones injected into services.
    _admission: Option<Arc<crate::production::InputAdmission>>,
}

#[derive(Clone)]
enum CoordinatorState {
    Exclusive(Arc<Mutex<CdpInputState>>),
    Interleaved(Arc<crate::interleaved::PageState>),
}

/// Input ownership is selected once per Page, never independently per action.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CdpInputPolicy {
    /// Whole-action exclusion with acknowledged-state cancellation recovery.
    #[default]
    Exclusive,
    /// Page-owned physical state, event-level mutation and no timeout release-all.
    Interleaved,
}

impl Default for CdpInputCoordinator {
    fn default() -> Self {
        Self::with_policy(CdpInputPolicy::Exclusive)
    }
}

impl CdpInputCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_policy(policy: CdpInputPolicy) -> Self {
        Self {
            state: match policy {
                CdpInputPolicy::Exclusive => CoordinatorState::Exclusive(Arc::default()),
                CdpInputPolicy::Interleaved => CoordinatorState::Interleaved(Arc::default()),
            },
            _admission: None,
        }
    }

    pub(crate) fn with_admission(
        policy: CdpInputPolicy,
        admission: Arc<crate::production::InputAdmission>,
    ) -> Self {
        let mut state = Self::with_policy(policy);
        state._admission = Some(admission);
        state
    }

    pub(crate) const fn state_allocation_bytes(policy: CdpInputPolicy) -> usize {
        let payload = match policy {
            CdpInputPolicy::Exclusive => std::mem::size_of::<Mutex<CdpInputState>>(),
            CdpInputPolicy::Interleaved => std::mem::size_of::<crate::interleaved::PageState>(),
        };
        payload + 2 * std::mem::size_of::<usize>()
    }

    pub fn policy(&self) -> CdpInputPolicy {
        match &self.state {
            CoordinatorState::Exclusive(_) => CdpInputPolicy::Exclusive,
            CoordinatorState::Interleaved(_) => CdpInputPolicy::Interleaved,
        }
    }

    /// Reports whether two routes serialize through the same Page input state.
    pub fn shares_state_with(&self, other: &Self) -> bool {
        match (&self.state, &other.state) {
            (CoordinatorState::Exclusive(a), CoordinatorState::Exclusive(b)) => Arc::ptr_eq(a, b),
            (CoordinatorState::Interleaved(a), CoordinatorState::Interleaved(b)) => {
                Arc::ptr_eq(a, b)
            }
            (CoordinatorState::Exclusive(_), CoordinatorState::Interleaved(_))
            | (CoordinatorState::Interleaved(_), CoordinatorState::Exclusive(_)) => false,
        }
    }

    /// Reserves a Page-bound identity for one opt-in mouse click invocation.
    /// No button is pressed until a transaction selects and dispatches it.
    pub fn new_owned_mouse_press(&self) -> CdpResult<crate::CdpOwnedMousePress> {
        match &self.state {
            CoordinatorState::Interleaved(state) => {
                crate::interleaved::PageState::new_owned_mouse_press(state, self._admission.clone())
            }
            CoordinatorState::Exclusive(_) => Err(CdpError::Other(
                "owned mouse press requires the interleaved input policy".into(),
            )),
        }
    }

    /// Releases only the button still owned by this invocation. A later down
    /// or up on the same button invalidates the claim; other buttons survive.
    pub async fn release_owned_mouse_press(
        &self,
        client: &dyn CdpClient,
        press: &crate::CdpOwnedMousePress,
    ) -> CdpResult<()> {
        match &self.state {
            CoordinatorState::Interleaved(state) => {
                state.release_owned_mouse_press(client, press).await
            }
            CoordinatorState::Exclusive(_) => Err(CdpError::Other(
                "owned mouse press requires the interleaved input policy".into(),
            )),
        }
    }

    /// Creates an effect-free raw input owner without waiting for native replies.
    /// Only explicitly selected Page-owned input can enter this path.
    pub fn begin_raw_mouse_transaction(
        &self,
        client: Arc<dyn CdpClient>,
    ) -> CdpResult<CdpInputTransaction> {
        match &self.state {
            CoordinatorState::Interleaved(state) => Ok(CdpInputTransaction {
                inner: TransactionMode::Interleaved(crate::interleaved::Action::raw(
                    state.clone(),
                    client,
                )),
                _admission: self._admission.clone(),
            }),
            CoordinatorState::Exclusive(_) => Err(CdpError::Other(
                "raw Mouse requires the Page-owned interleaved input policy".into(),
            )),
        }
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
        let state = match &self.state {
            CoordinatorState::Exclusive(state) => state,
            CoordinatorState::Interleaved(state) => {
                return crate::interleaved::Action::begin(Arc::clone(state), client, requested)
                    .await
                    .map(|action| CdpInputTransaction {
                        inner: TransactionMode::Interleaved(action),
                        _admission: self._admission.clone(),
                    });
            }
        };
        let mut state = Arc::clone(state).lock_owned().await;
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
            inner: TransactionMode::Exclusive(ExclusiveInputTransaction {
                client,
                state: Some(state),
            }),
            _admission: self._admission.clone(),
        })
    }

    /// Restores a transaction whose future was dropped after changing keys.
    /// Controlled runtimes call this from command cancellation. A subsequent
    /// input transaction also performs the same recovery before it can start.
    pub async fn cleanup_pending(&self, client: &dyn CdpClient) -> CdpResult<()> {
        match &self.state {
            CoordinatorState::Exclusive(state) => {
                let mut state = state.lock().await;
                cleanup_locked(client, &mut state).await
            }
            // Keyboard and mouse state deliberately survive cancellation, but
            // an acknowledged touchStart must never outlive its Page action.
            CoordinatorState::Interleaved(state) => state.cleanup_touch(client).await,
        }
    }
}

/// An input action using the Page's explicitly selected ownership policy.
pub struct CdpInputTransaction {
    inner: TransactionMode,
    _admission: Option<Arc<crate::production::InputAdmission>>,
}

enum TransactionMode {
    Exclusive(ExclusiveInputTransaction),
    Interleaved(crate::interleaved::Action),
}

macro_rules! dispatch_input {
    ($name:ident($($argument:ident: $kind:ty),*)) => {
        pub async fn $name(&mut self, $($argument: $kind),*) -> CdpResult<()> {
            match &mut self.inner {
                TransactionMode::Exclusive(action) => action.$name($($argument),*).await,
                TransactionMode::Interleaved(action) => action.$name($($argument),*).await,
            }
        }
    };
}

impl CdpInputTransaction {
    /// Selects a Page-bound press owner before this transaction's first down.
    /// Only opt-in retained click should call this; ordinary shared input keeps
    /// its Page-owned lifetime and existing cancellation behavior. This token
    /// owns no keyboard modifiers; cancellation of requested modifiers needs
    /// a separate rollback policy.
    pub fn set_owned_mouse_press(&mut self, press: &crate::CdpOwnedMousePress) -> CdpResult<()> {
        match &mut self.inner {
            TransactionMode::Interleaved(action) => action.set_owned_mouse_press(press),
            TransactionMode::Exclusive(_) => Err(CdpError::Other(
                "owned mouse press requires the interleaved input policy".into(),
            )),
        }
    }

    /// Select once on this transaction only; never mutate shared Page bindings.
    /// The provider and its native retirement driver have separate owners/budgets.
    pub fn set_key_down_interceptor(
        &mut self,
        interceptor: Arc<dyn crate::CdpKeyDownInterceptor>,
    ) -> CdpResult<()> {
        match &mut self.inner {
            TransactionMode::Interleaved(action) if action.key_down_interceptor.is_none() => {
                action.key_down_interceptor = Some(interceptor);
                Ok(())
            }
            _ => Err(CdpError::Other(
                "keydown interception requires an unselected interleaved transaction".into(),
            )),
        }
    }
    /// Creates a caller-owned click execution. After observing its result (or
    /// cancelling its polling), retire it before releasing its native owner.
    pub fn raw_mouse_click<D, W>(
        &mut self,
        options: crate::MouseClickOptions,
        delay: D,
    ) -> CdpResult<crate::MouseClickExecution<D, W>>
    where
        D: FnMut(std::time::Duration) -> W,
        W: std::future::Future<Output = CdpResult<()>>,
    {
        let admission = self._admission.clone();
        let action = self.raw_pointer()?;
        Ok(crate::MouseClickExecution::new(
            action, admission, options, delay, None,
        ))
    }

    /// Opt-in multi-phase dispatch with separate result and retirement polling.
    /// The dispatcher must provide its own admission/storage and Page lifecycle.
    pub fn raw_mouse_click_with_dispatcher<D, W>(
        &mut self,
        options: crate::MouseClickOptions,
        delay: D,
        dispatcher: Arc<dyn crate::CdpMouseDispatcher>,
    ) -> CdpResult<crate::MouseClickExecution<D, W>>
    where
        D: FnMut(std::time::Duration) -> W,
        W: std::future::Future<Output = CdpResult<()>>,
    {
        let admission = self._admission.clone();
        let action = self.raw_pointer()?;
        Ok(crate::MouseClickExecution::new(
            action,
            admission,
            options,
            delay,
            Some(dispatcher),
        ))
    }

    /// Raw Mouse operations require explicitly selected Page-owned state.
    /// Exclusive Locator transactions deliberately retain their old cleanup policy.
    fn raw_pointer(&mut self) -> CdpResult<&mut crate::interleaved::Action> {
        match &mut self.inner {
            TransactionMode::Interleaved(action) => Ok(action),
            TransactionMode::Exclusive(_) => Err(CdpError::Other(
                "raw Mouse requires the Page-owned interleaved input policy".into(),
            )),
        }
    }

    pub async fn raw_mouse_move(
        &mut self,
        x: f64,
        y: f64,
        steps: pwright_input_model::MouseMoveSteps,
    ) -> CdpResult<()> {
        self.raw_pointer()?.raw_mouse_move(x, y, steps).await
    }
    pub async fn raw_mouse_down(&mut self, button: MouseButton, click_count: i32) -> CdpResult<()> {
        self.raw_pointer()?
            .raw_mouse_down(button, click_count)
            .await
    }
    /// Lazy opt-in button dispatch; keep the returned owner through retirement.
    pub fn raw_mouse_down_with_dispatcher(
        &mut self,
        button: MouseButton,
        click_count: i32,
        dispatcher: Arc<dyn crate::CdpMouseDispatcher>,
    ) -> CdpResult<crate::MouseButtonExecution> {
        let admission = self._admission.clone();
        let action = self.raw_pointer()?;
        Ok(crate::MouseButtonExecution::new(
            action,
            admission,
            button,
            click_count,
            crate::MouseDispatchKind::Down,
            dispatcher,
        ))
    }
    /// Lazy opt-in release dispatch; no synthetic release occurs on cancellation.
    pub fn raw_mouse_up_with_dispatcher(
        &mut self,
        button: MouseButton,
        click_count: i32,
        dispatcher: Arc<dyn crate::CdpMouseDispatcher>,
    ) -> CdpResult<crate::MouseButtonExecution> {
        let admission = self._admission.clone();
        let action = self.raw_pointer()?;
        Ok(crate::MouseButtonExecution::new(
            action,
            admission,
            button,
            click_count,
            crate::MouseDispatchKind::Up,
            dispatcher,
        ))
    }
    pub async fn raw_mouse_up(&mut self, button: MouseButton, click_count: i32) -> CdpResult<()> {
        self.raw_pointer()?.raw_mouse_up(button, click_count).await
    }
    pub async fn raw_mouse_wheel(&mut self, delta_x: f64, delta_y: f64) -> CdpResult<()> {
        self.raw_pointer()?.raw_mouse_wheel(delta_x, delta_y).await
    }

    /// Dispatches one atomic Page-scoped touch pair with the currently held
    /// keyboard modifiers. The transaction owner must be retained through
    /// completion so cancellation cannot abandon an acknowledged touchStart.
    pub async fn touchscreen_tap(&mut self, x: f64, y: f64) -> CdpResult<()> {
        match &mut self.inner {
            TransactionMode::Exclusive(action) => action.touchscreen_tap(x, y).await,
            TransactionMode::Interleaved(action) => action.touchscreen_tap(x, y).await,
        }
    }

    /// Resolves a raw key alias at its dispatch boundary, not at call creation.
    pub async fn key_down_named<'a>(
        &mut self,
        name: &'a str,
        is_mac: bool,
    ) -> Result<(), crate::KeyInputError<'a>> {
        match &mut self.inner {
            TransactionMode::Exclusive(action) => {
                let key = crate::resolve_key(
                    name,
                    action.modifiers().contains(CdpModifiers::SHIFT),
                    is_mac,
                )
                .map_err(|error| crate::KeyInputError::UnknownKey(error.0))?;
                action.key_down(key, is_mac).await.map_err(Into::into)
            }
            TransactionMode::Interleaved(action) => action.key_down_named(name, is_mac).await,
        }
    }

    /// Keyup re-resolves physical-code aliases after intervening input changes.
    pub async fn key_up_named<'a>(
        &mut self,
        name: &'a str,
        is_mac: bool,
    ) -> Result<(), crate::KeyInputError<'a>> {
        match &mut self.inner {
            TransactionMode::Exclusive(action) => {
                let key = crate::resolve_key(
                    name,
                    action.modifiers().contains(CdpModifiers::SHIFT),
                    is_mac,
                )
                .map_err(|error| crate::KeyInputError::UnknownKey(error.0))?;
                action.key_up(key).await.map_err(Into::into)
            }
            TransactionMode::Interleaved(action) => action.key_up_named(name, is_mac).await,
        }
    }

    dispatch_input!(insert_text(text: pwright_input_model::InputText<'_>));
    dispatch_input!(key_down(key: ResolvedKey, is_mac: bool));
    dispatch_input!(key_up(key: ResolvedKey));
    dispatch_input!(move_mouse(x: f64, y: f64));
    dispatch_input!(move_mouse_in_steps(x: f64, y: f64, steps: NonZeroU32));
    dispatch_input!(mouse_down(button: MouseButton, click_count: i32));
    dispatch_input!(mouse_up());

    /// Dispatch an opt-in owned press only if its source is still current after
    /// acquiring the Page input gate. Ordinary mouseDown is unchanged.
    pub async fn mouse_down_checked<E>(
        &mut self,
        button: MouseButton,
        click_count: i32,
        check: impl FnOnce() -> std::result::Result<(), E> + Send,
    ) -> std::result::Result<(), crate::CdpCheckedMouseDownError<E>> {
        match &mut self.inner {
            TransactionMode::Interleaved(action) => {
                action.mouse_down_checked(button, click_count, check).await
            }
            TransactionMode::Exclusive(_) => Err(crate::CdpCheckedMouseDownError::Input(
                CdpError::Other("checked mouse down requires interleaved input".into()),
            )),
        }
    }

    pub fn modifiers(&self) -> CdpModifiers {
        match &self.inner {
            TransactionMode::Exclusive(action) => action.modifiers(),
            TransactionMode::Interleaved(action) => action.modifiers(),
        }
    }

    pub async fn finish(self) -> CdpResult<()> {
        let outcome = match self.inner {
            TransactionMode::Exclusive(action) => action.finish().await,
            TransactionMode::Interleaved(action) => action.finish().await,
        };
        drop(self._admission);
        outcome
    }
}

struct ExclusiveInputTransaction {
    client: Arc<dyn CdpClient>,
    state: Option<OwnedMutexGuard<CdpInputState>>,
}

impl ExclusiveInputTransaction {
    /// Inserts validated literal text without generating physical key events.
    /// Uses the same Page serialization owner as keyboard and pointer actions.
    pub async fn insert_text(&mut self, text: pwright_input_model::InputText<'_>) -> CdpResult<()> {
        self.client.input_insert_text(text.as_str()).await
    }

    async fn touchscreen_tap(&mut self, x: f64, y: f64) -> CdpResult<()> {
        let modifiers = self.modifiers().mask();
        self.state
            .as_mut()
            .expect("input transaction owns state")
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
            self.state
                .as_mut()
                .expect("input transaction owns state")
                .active_touch = None;
        }
        start_result?;
        end_result
    }

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
    release_active_touch(client, state).await?;
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

async fn release_active_touch(client: &dyn CdpClient, state: &mut CdpInputState) -> CdpResult<()> {
    let Some(active) = state.active_touch else {
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
    state.active_touch = None;
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
            .input_dispatch_keyboard_event(KeyboardEventParams {
                event_type: if needs_pressed {
                    KeyEventType::RawKeyDown
                } else {
                    KeyEventType::KeyUp
                },
                key: key.key,
                code: key.code,
                windows_virtual_key_code: key.windows_virtual_key_code,
                modifiers: next.mask(),
                text: None,
                unmodified_text: None,
                auto_repeat: false,
                // Temporary modifiers use the left-side aliases, like Keyboard.down.
                location: 1,
                is_keypad: false,
                commands: &[],
            })
            .await?;
        state.pressed_modifiers = next;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::num::NonZeroU32;
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};
    use std::time::Duration;

    use pwright_cdp::{CdpClient, MouseButton};
    use pwright_fake::FakeCdpClient;

    use super::{CdpInputCoordinator, CdpInputPolicy, CdpModifiers};
    use crate::keyboard::resolve_key;

    #[tokio::test]
    async fn temporary_modifier_transitions_preserve_left_key_location() {
        for modifier in [
            CdpModifiers::ALT,
            CdpModifiers::CONTROL,
            CdpModifiers::META,
            CdpModifiers::SHIFT,
        ] {
            // Arrange
            let client = Arc::new(FakeCdpClient::from_html(""));
            let coordinator = CdpInputCoordinator::new();
            // Act
            let transaction = coordinator
                .begin_transaction(client.clone(), Some(modifier))
                .await
                .unwrap();
            transaction.finish().await.unwrap();
            // Assert
            let calls = client.calls();
            assert_eq!(calls.len(), 2);
            for (_, arguments) in &calls {
                assert_eq!(arguments[0]["location"], 1);
                assert!(arguments[0]["code"].as_str().unwrap().ends_with("Left"));
            }
            assert_eq!(calls[0].1[0]["modifiers"], modifier.mask());
            assert_eq!(calls[1].1[0]["modifiers"], 0);
        }
    }

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
    async fn exclusive_touch_with_unacknowledged_end_is_released_by_cleanup() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let stalled_end = Arc::new(tokio::sync::Notify::new());
        client.set_pointer_gates([None, Some(stalled_end), None]);
        let coordinator = CdpInputCoordinator::with_policy(CdpInputPolicy::Exclusive);
        let mut transaction = coordinator
            .begin_transaction(client.clone(), None)
            .await
            .expect("touch transaction should start");
        let mut tap = Box::pin(transaction.touchscreen_tap(12.5, 19.25));
        assert!(matches!(
            tap.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        drop(tap);
        drop(transaction);

        // Act
        coordinator
            .cleanup_pending(client.as_ref())
            .await
            .expect("cleanup should release acknowledged touch state");

        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].1[0]["type"], "touchStart");
        assert_eq!(calls[1].1[0]["type"], "touchEnd");
        assert_eq!(calls[2].1[0]["type"], "touchEnd");
        assert_eq!(calls[2].1[0]["modifiers"], 0);
    }

    #[tokio::test]
    async fn interleaved_touch_with_unacknowledged_end_is_released_by_cleanup() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let stalled_end = Arc::new(tokio::sync::Notify::new());
        client.set_pointer_gates([None, Some(stalled_end), None]);
        let coordinator = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let mut transaction = coordinator
            .begin_transaction(client.clone(), None)
            .await
            .expect("touch transaction should start");
        let mut tap = Box::pin(transaction.touchscreen_tap(40.0, 60.0));
        assert!(matches!(
            tap.as_mut().poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        drop(tap);
        drop(transaction);

        // Act
        coordinator
            .cleanup_pending(client.as_ref())
            .await
            .expect("cleanup should release acknowledged touch state");

        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].1[0]["type"], "touchStart");
        assert_eq!(calls[1].1[0]["type"], "touchEnd");
        assert_eq!(calls[2].1[0]["type"], "touchEnd");
        assert_eq!(calls[2].1[0]["modifiers"], 0);
    }

    #[tokio::test]
    async fn exclusive_touch_submits_end_before_start_acknowledgement() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let stalled_start = Arc::new(tokio::sync::Notify::new());
        client.set_pointer_gates([Some(stalled_start), None, None]);
        let coordinator = CdpInputCoordinator::with_policy(CdpInputPolicy::Exclusive);
        let mut transaction = coordinator
            .begin_transaction(client.clone(), None)
            .await
            .expect("touch transaction should start");
        let mut tap = Box::pin(transaction.touchscreen_tap(12.5, 19.25));

        // Act
        let state = tap.as_mut().poll(&mut Context::from_waker(Waker::noop()));

        // Assert
        assert!(matches!(state, Poll::Pending));
        let calls = client.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1[0]["type"], "touchStart");
        assert_eq!(calls[1].1[0]["type"], "touchEnd");
        drop(tap);
        drop(transaction);
        coordinator.cleanup_pending(client.as_ref()).await.unwrap();
    }

    #[tokio::test]
    async fn interleaved_touch_submits_end_before_start_acknowledgement() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let stalled_start = Arc::new(tokio::sync::Notify::new());
        client.set_pointer_gates([Some(stalled_start), None, None]);
        let coordinator = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let mut transaction = coordinator
            .begin_transaction(client.clone(), None)
            .await
            .expect("touch transaction should start");
        let mut tap = Box::pin(transaction.touchscreen_tap(40.0, 60.0));

        // Act
        let state = tap.as_mut().poll(&mut Context::from_waker(Waker::noop()));

        // Assert
        assert!(matches!(state, Poll::Pending));
        let calls = client.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1[0]["type"], "touchStart");
        assert_eq!(calls[1].1[0]["type"], "touchEnd");
        drop(tap);
        drop(transaction);
        coordinator.cleanup_pending(client.as_ref()).await.unwrap();
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

    #[tokio::test]
    async fn checked_owned_press_dispatches_when_world_is_current() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let coordinator = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let owner = coordinator.new_owned_mouse_press().unwrap();
        let mut transaction = coordinator
            .begin_transaction(client.clone(), None)
            .await
            .unwrap();
        transaction.set_owned_mouse_press(&owner).unwrap();

        // Act
        transaction
            .mouse_down_checked(MouseButton::Left, 1, || Ok::<(), &'static str>(()))
            .await
            .unwrap();
        transaction.mouse_up().await.unwrap();

        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1[0]["type"], "mousePressed");
        assert_eq!(calls[1].1[0]["type"], "mouseReleased");
    }
}
