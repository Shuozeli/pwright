//! Raw Page-owned mouse primitives. No selector, task, clock or JS fallback.
use super::{Action, PageState, button_index, button_mask};
use pwright_cdp::{
    MouseButton,
    connection::{CdpError, Result},
    generated::input::DispatchMouseEventParams,
};
use pwright_input_model::MouseMoveSteps;

pub(super) mod button;
pub(super) mod click;
pub(super) mod dispatch;

fn finite_pair(x: f64, y: f64) -> Result<()> {
    if !x.is_finite() || !y.is_finite() {
        return Err(CdpError::Other(
            "mouse coordinates and deltas must be finite".into(),
        ));
    }
    Ok(())
}

fn native_button(button: Option<MouseButton>) -> pwright_cdp::generated::input::MouseButton {
    use pwright_cdp::generated::input::MouseButton as Native;
    match button {
        None => Native::None,
        Some(MouseButton::Left) => Native::Left,
        Some(MouseButton::Right) => Native::Right,
        Some(MouseButton::Middle) => Native::Middle,
    }
}

fn begin_move(page: &PageState, x: f64, y: f64) -> (f64, f64) {
    let mut state = page.state.lock().expect("input state is not poisoned");
    let from = (state.x, state.y);
    state.x = x;
    state.y = y;
    from
}

fn move_event(page: &PageState, x: f64, y: f64) -> DispatchMouseEventParams {
    let state = page.state.lock().expect("input state is not poisoned");
    DispatchMouseEventParams {
        r#type: "mouseMoved".into(),
        x,
        y,
        button: Some(native_button(state.last_button)),
        buttons: Some(i64::from(state.buttons)),
        modifiers: Some(i64::from(state.modifiers.mask())),
        force: Some(if state.buttons != 0 { 0.5 } else { 0.0 }),
        ..Default::default()
    }
}

fn down_event(page: &PageState, button: MouseButton, count: i32) -> DispatchMouseEventParams {
    let mut state = page.state.lock().expect("input state is not poisoned");
    state.last_button = Some(button);
    state.buttons |= button_mask(button);
    state.owned_presses[button_index(button)] = None;
    DispatchMouseEventParams {
        r#type: "mousePressed".into(),
        x: state.x,
        y: state.y,
        button: Some(native_button(Some(button))),
        buttons: Some(i64::from(state.buttons)),
        modifiers: Some(i64::from(state.modifiers.mask())),
        click_count: Some(i64::from(count)),
        force: Some(0.5),
        ..Default::default()
    }
}

fn up_event(page: &PageState, button: MouseButton, count: i32) -> DispatchMouseEventParams {
    let mut state = page.state.lock().expect("input state is not poisoned");
    state.last_button = None;
    state.buttons &= !button_mask(button);
    state.owned_presses[button_index(button)] = None;
    DispatchMouseEventParams {
        r#type: "mouseReleased".into(),
        x: state.x,
        y: state.y,
        button: Some(native_button(Some(button))),
        buttons: Some(i64::from(state.buttons)),
        modifiers: Some(i64::from(state.modifiers.mask())),
        click_count: Some(i64::from(count)),
        ..Default::default()
    }
}

impl Action {
    // Capture/mutate logical state before the first native await. Native raw
    // Mouse calls may be in flight together; do not gate them on prior replies.
    pub async fn raw_mouse_move(&self, x: f64, y: f64, steps: MouseMoveSteps) -> Result<()> {
        finite_pair(x, y)?;
        let (from_x, from_y) = begin_move(&self.page, x, y);
        let mut step = 1_u32;
        while f64::from(step) <= steps.get() {
            let progress = f64::from(step) / steps.get();
            let event = move_event(
                &self.page,
                from_x + (x - from_x) * progress,
                from_y + (y - from_y) * progress,
            );
            self.client.input_dispatch_pointer_event(event).await?;
            step += 1;
        }
        Ok(())
    }

    pub async fn raw_mouse_down(&self, button: MouseButton, count: i32) -> Result<()> {
        self.client
            .input_dispatch_pointer_event(down_event(&self.page, button, count))
            .await
    }

    pub async fn raw_mouse_up(&self, button: MouseButton, count: i32) -> Result<()> {
        self.client
            .input_dispatch_pointer_event(up_event(&self.page, button, count))
            .await
    }

    pub async fn raw_mouse_wheel(&self, delta_x: f64, delta_y: f64) -> Result<()> {
        finite_pair(delta_x, delta_y)?;
        let event = {
            let state = self.page.state.lock().expect("input state is not poisoned");
            DispatchMouseEventParams {
                r#type: "mouseWheel".into(),
                x: state.x,
                y: state.y,
                modifiers: Some(i64::from(state.modifiers.mask())),
                delta_x: Some(delta_x),
                delta_y: Some(delta_y),
                ..Default::default()
            }
        };
        self.client.input_dispatch_pointer_event(event).await
    }
}

#[cfg(test)]
mod tests;
