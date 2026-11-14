//! Frame-bound, weak-session Locators dispatch existing typed Rust commands.

pub mod action;
#[cfg(feature = "drag-and-drop")]
pub mod drag;
pub(crate) mod keyboard;

use std::{
    str::FromStr,
    sync::{Arc, Weak},
    time::Duration,
};

use pwright_cap_locator::{
    AllInnerTexts, AllInnerTextsParams, AllTextContents, AllTextContentsParams, Count, CountParams,
    GetAttribute, GetAttributeParams, InnerHtml, InnerHtmlParams, InnerText, InnerTextParams,
    InputValue, InputValueParams, IsChecked, IsCheckedParams, IsEditable, IsEditableParams,
    IsEnabled, IsEnabledParams, IsVisible, IsVisibleParams, RegexMatcher, Selector, TextContent,
    TextContentParams, WaitFor, WaitForOptions, WaitForParams, WaitForState,
};
#[cfg(feature = "locator-screenshot")]
use pwright_cap_locator_screenshot::{LocatorScreenshot, LocatorScreenshotParams};
use pwright_core::ObjectId;

#[cfg(feature = "handles")]
use pwright_backend_cdp_selector_runtime::{
    MAX_SELECTOR_DEPTH, SELECTOR_RUNTIME_SYMBOL, SELECTOR_RUNTIME_VERSION, compile_selector,
    selector_runtime_install_script,
};

#[cfg(any(
    feature = "selector-registration",
    feature = "layout-selectors",
    feature = "handles"
))]
use crate::locator_query::CommandPlan;
use crate::{
    HostError, HostSession,
    locator_query::{LocatorQuery, QueryCombination},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorRead {
    Count,
    AllInnerTexts,
    AllTextContents,
    GetAttribute,
    InnerHtml,
    InnerText,
    InputValue,
    IsChecked,
    IsEditable,
    IsEnabled,
    IsDisabled,
    IsVisible,
    IsHidden,
    TextContent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorWaitState {
    Attached,
    Detached,
    Visible,
    Hidden,
}

#[cfg(feature = "handles")]
enum SelectorWaitAttempt {
    Pending,
    Complete(Option<crate::handles::PendingHandle>),
}

impl From<LocatorWaitState> for WaitForState {
    fn from(value: LocatorWaitState) -> Self {
        match value {
            LocatorWaitState::Attached => Self::Attached,
            LocatorWaitState::Detached => Self::Detached,
            LocatorWaitState::Visible => Self::Visible,
            LocatorWaitState::Hidden => Self::Hidden,
        }
    }
}

#[cfg(feature = "handles")]
fn remaining_timeout_ms(
    deadline: Option<&tokio::time::Instant>,
    original_timeout_ms: u32,
) -> Result<u32, HostError> {
    let Some(deadline) = deadline else {
        return Ok(0);
    };
    let now = tokio::time::Instant::now();
    if now >= *deadline {
        return Err(HostError::Timeout(original_timeout_ms));
    }
    Ok(
        u32::try_from(deadline.saturating_duration_since(now).as_millis())
            .unwrap_or(u32::MAX)
            .max(1),
    )
}

#[cfg(feature = "handles")]
fn selector_wait_error(error: HostError, original_timeout_ms: u32) -> HostError {
    match error {
        HostError::Timeout(_) => HostError::Timeout(original_timeout_ms),
        error => error,
    }
}

impl FromStr for LocatorRead {
    type Err = HostError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "count" => Self::Count,
            "allInnerTexts" => Self::AllInnerTexts,
            "allTextContents" => Self::AllTextContents,
            "getAttribute" => Self::GetAttribute,
            "innerHTML" => Self::InnerHtml,
            "innerText" => Self::InnerText,
            "inputValue" => Self::InputValue,
            "isChecked" => Self::IsChecked,
            "isEditable" => Self::IsEditable,
            "isEnabled" => Self::IsEnabled,
            "isDisabled" => Self::IsDisabled,
            "isVisible" => Self::IsVisible,
            "isHidden" => Self::IsHidden,
            "textContent" => Self::TextContent,
            _ => {
                return Err(HostError::UnsupportedOperation(
                    "unknown Locator read operation".into(),
                ));
            }
        })
    }
}

pub enum LocatorValue {
    Count(u32),
    Bool(bool),
    Text(Option<String>),
    Texts(Vec<String>),
}

fn playwright_locator_call_log(selector: &Selector) -> String {
    match selector {
        Selector::Css(css) => format!(
            "locator('{}')",
            css.as_str().replace('\\', "\\\\").replace('\'', "\\'")
        ),
        Selector::Within { base, descendant } => {
            let descendant = match &**descendant {
                Selector::Css(css) => format!(
                    "locator('{}')",
                    css.as_str().replace('\\', "\\\\").replace('\'', "\\'")
                ),
                descendant => format!("locator({})", playwright_locator_call_log(descendant)),
            };
            format!("{}.{}", playwright_locator_call_log(base), descendant)
        }
        Selector::Nth { base, index } => {
            format!("{}.nth({index})", playwright_locator_call_log(base))
        }
        selector => selector.to_string(),
    }
}

impl HostLocator {
    fn call_log(&self) -> Result<String, HostError> {
        let (selector, _lease) = self.query.command_selector()?;
        let current = playwright_locator_call_log(&selector);
        #[cfg(feature = "handles")]
        if let Some(owner) = &self.content_frame_owner {
            return Ok(format!("{}.contentFrame().{current}", owner.call_log()?));
        }
        Ok(current)
    }
}

fn rewrite_strict_selector_error<T>(
    query: &LocatorQuery,
    outcome: Result<T, HostError>,
) -> Result<T, HostError> {
    let message = match outcome {
        Err(HostError::Backend(message)) => message,
        outcome => return outcome,
    };
    let normalized = message.strip_prefix("Backend: ").unwrap_or(&message);
    if normalized.starts_with("invalid selector `") && normalized.contains("not a valid selector") {
        let (selector, _lease) = query.command_selector()?;
        if let Selector::Css(css) = selector
            && let Some(token) = unexpected_css_closing_token(css.as_str())
        {
            return Err(HostError::Backend(format!(
                "Unexpected token {:?} while parsing css selector {:?}",
                token.to_string(),
                css.as_str()
            )));
        }
    }
    let Some(rest) = normalized.strip_prefix("strict selector `") else {
        return Err(HostError::Backend(message));
    };
    let Some((_selector, count)) = rest.rsplit_once("` resolved to ") else {
        return Err(HostError::Backend(message));
    };
    let Some(count) = count.strip_suffix(" elements") else {
        return Err(HostError::Backend(message));
    };
    if count.parse::<u32>().is_err() {
        return Err(HostError::Backend(message));
    }
    let (selector, _lease) = query.command_selector()?;
    Err(HostError::Backend(format!(
        "strict mode violation: {} resolved to {count} elements",
        playwright_locator_call_log(&selector)
    )))
}

#[cfg(feature = "handles")]
fn rewrite_frame_owner_error(error: HostError) -> HostError {
    match error {
        HostError::Backend(message) if message.starts_with("strict mode violation:") => {
            HostError::Backend(format!("Error: {message}"))
        }
        error => error,
    }
}

fn unexpected_css_closing_token(selector: &str) -> Option<char> {
    let mut quote = None;
    let mut escaped = false;
    let mut square = 0_u32;
    let mut parenthesis = 0_u32;
    let mut curly = 0_u32;
    for token in selector.chars() {
        if escaped {
            escaped = false;
            continue;
        }
        if token == '\\' {
            escaped = true;
            continue;
        }
        if let Some(active) = quote {
            if token == active {
                quote = None;
            }
            continue;
        }
        if token == '\'' || token == '"' {
            quote = Some(token);
            continue;
        }
        match token {
            '[' => square = square.saturating_add(1),
            ']' if square == 0 => return Some(token),
            ']' => square -= 1,
            '(' => parenthesis = parenthesis.saturating_add(1),
            ')' if parenthesis == 0 => return Some(token),
            ')' => parenthesis -= 1,
            '{' => curly = curly.saturating_add(1),
            '}' if curly == 0 => return Some(token),
            '}' => curly -= 1,
            _ => {}
        }
    }
    None
}

fn recover_visibility_context_loss(
    operation: LocatorRead,
    outcome: Result<LocatorValue, HostError>,
) -> Result<LocatorValue, HostError> {
    match outcome {
        Err(HostError::Backend(message))
            if message.starts_with("strict selector `")
                || message.starts_with("Backend: strict selector `") =>
        {
            let message = message.strip_prefix("Backend: ").unwrap_or(&message);
            Err(HostError::Backend(format!(
                "strict mode violation: {message}"
            )))
        }
        Err(HostError::Backend(message))
            if matches!(operation, LocatorRead::IsVisible | LocatorRead::IsHidden)
                && (message.contains("uniqueContextId not found")
                    || message.contains("Execution context was destroyed")
                    || message.contains("Cannot find context with specified id")) =>
        {
            Ok(LocatorValue::Bool(operation == LocatorRead::IsHidden))
        }
        outcome => outcome,
    }
}

pub struct LocatorExpectation {
    pub matches: bool,
    pub received: Option<String>,
    pub preview: Option<String>,
    pub missing: bool,
    pub timed_out: bool,
}

pub struct LocatorValueExpectation {
    pub matches: bool,
    pub received: Option<crate::evaluation::EvaluationValue>,
    pub missing: bool,
    pub timed_out: bool,
}

pub struct LocatorCountExpectation {
    pub matches: bool,
    pub received: Option<u32>,
    pub timed_out: bool,
}

pub struct LocatorTextsExpectation {
    pub matches: bool,
    pub received: Vec<String>,
    pub timed_out: bool,
}

#[cfg(feature = "handles")]
pub struct LocatorHandleResult {
    pub handle: crate::handles::PendingHandle,
    pub preview: Option<String>,
}

fn count_expectation_matches(received: u32, expected: u32) -> bool {
    received == expected
}

fn state_expectation_attempt_timeout(
    first_attempt: bool,
    has_content_frame_owner: bool,
    timeout_ms: u32,
    remaining: Duration,
) -> u32 {
    // Playwright always completes one immediate check without racing the
    // public timeout. Rust's command timeout value `0` means unbounded rather
    // than one-shot, so give the plain-Locator probe a small independent
    // budget. A nested FrameLocator remains bounded by the public deadline
    // because resolving a missing iframe owner can itself wait.
    if first_attempt && !has_content_frame_owner {
        return 100;
    }
    if timeout_ms == 0 {
        1
    } else {
        u32::try_from(remaining.as_millis().clamp(1, 100)).unwrap_or(100)
    }
}

const EXPECTATION_PREVIEW_SOURCE: &str = r#"element => {
  const trim = (value, cap) => {
    const chars = Array.from(value);
    return chars.length <= cap ? value : chars.slice(0, cap - 1).join('') + '\u2026';
  };
  const oneLine = value => value.replace(/\n/g, '\u21b5').replace(/\t/g, '\u21c6');
  if (element.nodeType === Node.TEXT_NODE)
    return oneLine(`#text=${element.nodeValue || ''}`);
  if (element.nodeType !== Node.ELEMENT_NODE)
    return oneLine(`<${element.nodeName.toLowerCase()} />`);
  const booleanAttributes = new Set(['checked', 'selected', 'disabled', 'readonly', 'multiple']);
  const autoClosingTags = new Set(['AREA', 'BASE', 'BR', 'COL', 'COMMAND', 'EMBED', 'HR', 'IMG',
    'INPUT', 'KEYGEN', 'LINK', 'MENUITEM', 'META', 'PARAM', 'SOURCE', 'TRACK', 'WBR']);
  const attrs = [];
  for (const { name, value } of element.attributes) {
    if (name === 'style') continue;
    attrs.push(!value && booleanAttributes.has(name) ? ` ${name}` : ` ${name}="${value}"`);
  }
  attrs.sort((left, right) => left.length - right.length);
  const attrText = trim(attrs.join(''), 500);
  const name = element.nodeName.toLowerCase();
  if (autoClosingTags.has(element.nodeName))
    return oneLine(`<${name}${attrText}/>`);
  const children = element.childNodes;
  const onlyText = children.length <= 5 && Array.from(children).every(child => child.nodeType === Node.TEXT_NODE);
  const text = onlyText ? (element.textContent || '') : (children.length ? '\u2026' : '');
  return oneLine(`<${name}${attrText}>${trim(text, 50)}</${name}>`);
}"#;

#[cfg(feature = "handles")]
const FRAME_OWNER_DIAGNOSTIC_SOURCE: &str = r#"element => {
  if (element.nodeName === 'IFRAME' || element.nodeName === 'FRAME') return '';
  const preview = String(element.outerHTML || element.nodeName || 'element');
  return Array.from(preview).slice(0, 512).join('');
}"#;

#[cfg(feature = "handles")]
fn non_frame_owner_preview(
    value: crate::evaluation::EvaluationValue,
) -> Result<Option<String>, HostError> {
    match value {
        crate::evaluation::EvaluationValue::String(value) if value.is_empty() => Ok(None),
        crate::evaluation::EvaluationValue::String(value) => Ok(Some(value)),
        _ => Err(HostError::Backend(
            "FrameLocator owner diagnostic returned a non-string value".into(),
        )),
    }
}

#[cfg(feature = "locator-bounding-box")]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocatorBoundingBox {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[cfg(feature = "locator-bounding-box")]
pub(crate) const BOUNDING_BOX_SOURCE: &str = r#"element => {
  if (!element.isConnected) return null;
  const rect = element.getBoundingClientRect();
  if (!element.getClientRects().length) return null;
  const style = element.ownerDocument.defaultView.getComputedStyle(element);
  if (style.visibility === 'hidden') return null;
  let x = rect.x;
  let y = rect.y;
  let current = element.ownerDocument.defaultView;
  while (current && current !== current.top) {
    let frameElement;
    try { frameElement = current.frameElement; } catch { return null; }
    if (!frameElement) return null;
    const frameRect = frameElement.getBoundingClientRect();
    x += frameRect.x;
    y += frameRect.y;
    current = current.parent;
  }
  return { x, y, width: rect.width, height: rect.height };
}"#;

#[cfg(feature = "locator-highlight")]
const HIGHLIGHT_SOURCE: &str = r#"(elements, argument) => {
  const stateKey = Symbol.for('pwright.locator.highlight.v1');
  const previous = globalThis[stateKey];
  if (previous) {
    if (previous.raf) cancelAnimationFrame(previous.raf);
    previous.pane.remove();
  }
  const document = elements[0]?.ownerDocument || globalThis.document;
  if (!document?.documentElement) return;
  const pane = document.createElement('x-pw-glass');
  Object.assign(pane.style, {
    position: 'fixed', top: '0', right: '0', bottom: '0', left: '0',
    zIndex: '2147483647', pointerEvents: 'none', display: 'flex',
    backgroundColor: 'transparent'
  });
  const root = pane.attachShadow({ mode: argument.underTest ? 'open' : 'closed' });
  const style = document.createElement('style');
  style.textContent = `
    :host { font: 13px system-ui, sans-serif; color: #333; }
    x-pw-highlight { position: absolute; display: block; background: #6fa8dc7f; }
    x-pw-tooltip { position: absolute; display: flex; flex-direction: column;
      background: white; border-radius: 6px; box-shadow: 0 .5rem 1.2rem rgba(0,0,0,.3);
      font-size: 12.8px; line-height: 1.5; max-width: 600px; overflow: hidden; }
    x-pw-tooltip-line { display: flex; max-width: 600px; padding: 6px; }
  `;
  root.appendChild(style);
  const rendered = elements.map((element, index) => {
    const highlight = document.createElement('x-pw-highlight');
    const tooltip = document.createElement('x-pw-tooltip');
    const line = document.createElement('x-pw-tooltip-line');
    const suffix = elements.length > 1 ? ` [${index + 1} of ${elements.length}]` : '';
    line.textContent = argument.description + suffix;
    tooltip.appendChild(line);
    root.append(highlight, tooltip);
    return { element, highlight, tooltip };
  });
  document.documentElement.appendChild(pane);
  const state = { pane, raf: 0 };
  globalThis[stateKey] = state;
  const update = () => {
    for (const entry of rendered) {
      if (!entry.element.isConnected) {
        entry.highlight.style.display = 'none';
        entry.tooltip.style.display = 'none';
        continue;
      }
      const box = entry.element.getBoundingClientRect();
      Object.assign(entry.highlight.style, {
        display: 'block', left: box.x + 'px', top: box.y + 'px',
        width: box.width + 'px', height: box.height + 'px'
      });
      let left = Math.max(5, box.left);
      let top = Math.max(0, box.bottom) + 5;
      const tooltipWidth = entry.tooltip.offsetWidth;
      const tooltipHeight = entry.tooltip.offsetHeight;
      if (left + tooltipWidth > pane.offsetWidth - 5)
        left = pane.offsetWidth - tooltipWidth - 5;
      if (top + tooltipHeight > pane.offsetHeight - 5)
        top = Math.max(0, box.top) > tooltipHeight + 5
          ? Math.max(0, box.top) - tooltipHeight - 5
          : pane.offsetHeight - tooltipHeight - 5;
      Object.assign(entry.tooltip.style, { display: 'flex', left: left + 'px', top: top + 'px' });
    }
    state.raf = requestAnimationFrame(update);
  };
  update();
}"#;

#[cfg(feature = "locator-dom-actions")]
pub const MAX_EVENT_TYPE_BYTES: usize = 1024;

#[cfg(feature = "locator-dom-actions")]
pub(crate) const DISPATCH_EVENT_SOURCE: &str = r#"(element, argument) => {
  const defaults = { bubbles: true, cancelable: true, composed: true };
  const init = { ...defaults, ...(argument.eventInit || {}) };
  const constructors = {
    auxclick: 'MouseEvent', click: 'MouseEvent', contextmenu: 'MouseEvent', dblclick: 'MouseEvent',
    mousedown: 'MouseEvent', mouseenter: 'MouseEvent', mouseleave: 'MouseEvent',
    mousemove: 'MouseEvent', mouseout: 'MouseEvent', mouseover: 'MouseEvent', mouseup: 'MouseEvent',
    drag: 'DragEvent', dragend: 'DragEvent', dragenter: 'DragEvent', dragexit: 'DragEvent',
    dragleave: 'DragEvent', dragover: 'DragEvent', dragstart: 'DragEvent', drop: 'DragEvent',
    keydown: 'KeyboardEvent', keypress: 'KeyboardEvent', keyup: 'KeyboardEvent',
    pointercancel: 'PointerEvent', pointerdown: 'PointerEvent', pointerenter: 'PointerEvent',
    pointerleave: 'PointerEvent', pointermove: 'PointerEvent', pointerout: 'PointerEvent',
    pointerover: 'PointerEvent', pointerup: 'PointerEvent',
    touchcancel: 'TouchEvent', touchend: 'TouchEvent', touchmove: 'TouchEvent', touchstart: 'TouchEvent',
    wheel: 'WheelEvent', focus: 'FocusEvent', blur: 'FocusEvent', focusin: 'FocusEvent',
    focusout: 'FocusEvent',
    deviceorientation: 'DeviceOrientationEvent', deviceorientationabsolute: 'DeviceOrientationEvent',
    devicemotion: 'DeviceMotionEvent'
  };
  const constructorName = constructors[argument.type];
  if (constructorName === 'TouchEvent') {
    init.target ??= element;
    const TouchConstructor = globalThis.Touch;
    const createTouches = touches => !touches || !TouchConstructor ? touches : touches.map(touch =>
      touch instanceof TouchConstructor ? touch : new TouchConstructor({
        ...touch,
        target: touch.target ?? element
      })
    );
    init.touches = createTouches(init.touches);
    init.targetTouches = createTouches(init.targetTouches);
    init.changedTouches = createTouches(init.changedTouches);
  }
  if (constructorName === 'DeviceOrientationEvent') {
    let event;
    try {
      event = new DeviceOrientationEvent(argument.type, init);
    } catch {
      const { bubbles, cancelable, alpha, beta, gamma, absolute } = init;
      event = document.createEvent('DeviceOrientationEvent');
      event.initDeviceOrientationEvent(argument.type, bubbles, cancelable, alpha, beta, gamma, absolute);
    }
    element.dispatchEvent(event);
    return;
  }
  if (constructorName === 'DeviceMotionEvent') {
    let event;
    try {
      event = new DeviceMotionEvent(argument.type, init);
    } catch {
      const { bubbles, cancelable, acceleration, accelerationIncludingGravity, rotationRate, interval } = init;
      event = document.createEvent('DeviceMotionEvent');
      event.initDeviceMotionEvent(argument.type, bubbles, cancelable,
        acceleration, accelerationIncludingGravity, rotationRate, interval);
    }
    element.dispatchEvent(event);
    return;
  }
  const EventConstructor = globalThis[constructorName] || Event;
  element.dispatchEvent(new EventConstructor(argument.type, init));
}"#;

#[cfg(feature = "locator-dom-actions")]
pub(crate) fn dispatch_event_argument(
    event_type: String,
    event_init: crate::evaluation::EvaluationValue,
) -> Result<crate::evaluation::EvaluationValue, HostError> {
    if event_type.is_empty() || event_type.len() > MAX_EVENT_TYPE_BYTES {
        return Err(HostError::InvalidArgument(format!(
            "event type must contain between 1 and {MAX_EVENT_TYPE_BYTES} UTF-8 bytes"
        )));
    }
    if !matches!(
        event_init,
        crate::evaluation::EvaluationValue::Null
            | crate::evaluation::EvaluationValue::Object { .. }
    ) {
        return Err(HostError::InvalidArgument(
            "event initializer must be an object or null".into(),
        ));
    }
    Ok(crate::evaluation::EvaluationValue::Object {
        id: None,
        properties: vec![
            crate::evaluation::EvaluationProperty::new(
                "type",
                crate::evaluation::EvaluationValue::String(event_type),
            ),
            crate::evaluation::EvaluationProperty::new("eventInit", event_init),
        ],
    })
}

#[cfg(feature = "locator-dom-actions")]
const SELECT_TEXT_SOURCE: &str = r#"element => {
  const target = element.nodeName === 'LABEL' && element.control ? element.control : element;
  target.focus();
  if (target.nodeName === 'INPUT' || target.nodeName === 'TEXTAREA') {
    target.select();
    return;
  }
  const selection = target.ownerDocument.defaultView.getSelection();
  if (!selection) return;
  const range = target.ownerDocument.createRange();
  range.selectNodeContents(target);
  selection.removeAllRanges();
  selection.addRange(range);
}"#;

#[cfg(feature = "locator-bounding-box")]
pub(crate) fn bounding_box_result(
    value: crate::evaluation::EvaluationValue,
) -> Result<Option<LocatorBoundingBox>, HostError> {
    use crate::evaluation::EvaluationValue;

    let EvaluationValue::Object { properties, .. } = value else {
        return match value {
            EvaluationValue::Null => Ok(None),
            _ => Err(HostError::Backend(
                "bounding box received a non-object result".into(),
            )),
        };
    };
    let mut x = None;
    let mut y = None;
    let mut width = None;
    let mut height = None;
    for property in properties {
        let EvaluationValue::Number(value) = property.value else {
            continue;
        };
        match property.key.as_str() {
            "x" => x = Some(value.get()),
            "y" => y = Some(value.get()),
            "width" => width = Some(value.get()),
            "height" => height = Some(value.get()),
            _ => {}
        }
    }
    Ok(Some(LocatorBoundingBox {
        x: x.ok_or_else(|| HostError::Backend("bounding box omitted x".into()))?,
        y: y.ok_or_else(|| HostError::Backend("bounding box omitted y".into()))?,
        width: width.ok_or_else(|| HostError::Backend("bounding box omitted width".into()))?,
        height: height.ok_or_else(|| HostError::Backend("bounding box omitted height".into()))?,
    }))
}

const PROPERTY_EXPECTATION_SOURCE: &str = r#"(element, argument) => {
  let received = element;
  for (const property of argument.name.split('.')) {
    if (typeof received !== 'object' || received === null || !(property in received)) {
      received = undefined;
      break;
    }
    received = received[property];
  }
  const equal = (left, right, seen = new WeakMap()) => {
    if (Object.is(left, right)) return true;
    if (typeof left !== 'object' || left === null || typeof right !== 'object' || right === null)
      return false;
    if (seen.get(left) === right) return true;
    seen.set(left, right);
    const leftTag = Object.prototype.toString.call(left);
    if (leftTag !== Object.prototype.toString.call(right)) return false;
    if (leftTag === '[object Date]') return left.getTime() === right.getTime();
    if (leftTag === '[object RegExp]')
      return left.source === right.source && left.flags === right.flags;
    if (leftTag === '[object URL]') return left.href === right.href;
    if (leftTag === '[object Error]')
      return left.name === right.name && left.message === right.message;
    if (ArrayBuffer.isView(left)) {
      if (left.byteLength !== right.byteLength) return false;
      const leftBytes = new Uint8Array(left.buffer, left.byteOffset, left.byteLength);
      const rightBytes = new Uint8Array(right.buffer, right.byteOffset, right.byteLength);
      return leftBytes.every((value, index) => value === rightBytes[index]);
    }
    const leftKeys = Object.keys(left);
    const rightKeys = Object.keys(right);
    if (leftKeys.length !== rightKeys.length || leftKeys.some(key => !rightKeys.includes(key)))
      return false;
    return leftKeys.every(key => equal(left[key], right[key], seen));
  };
  return { matches: equal(received, argument.expected), received };
}"#;

const ATTRIBUTE_EXPECTATION_SOURCE: &str = r#"(element, argument) => {
  const received = element.getAttribute(argument.name);
  if (received === null)
    return { matches: false, received };
  if (argument.flags !== undefined) {
    const flags = new Set(argument.flags.split(''));
    if (argument.ignoreCase === false) flags.delete('i');
    if (argument.ignoreCase === true) flags.add('i');
    const expression = new RegExp(argument.expected, [...flags].join(''));
    return { matches: expression.test(received), received };
  }
  const expected = argument.ignoreCase === true ? argument.expected.toLocaleLowerCase() : argument.expected;
  const actual = argument.ignoreCase === true ? received.toLocaleLowerCase() : received;
  return { matches: actual === expected, received };
}"#;

const CSS_EXPECTATION_SOURCE: &str = r#"(element, argument) => {
  const received = window.getComputedStyle(element).getPropertyValue(argument.name);
  if (argument.flags !== undefined) {
    const expression = new RegExp(argument.expected, argument.flags);
    return { matches: expression.test(received), received };
  }
  return { matches: received === argument.expected, received };
}"#;

const TEXT_REGEX_EXPECTATION_SOURCE: &str = r#"(element, argument) => {
  const received = argument.useInnerText ? element.innerText : (element.textContent ?? '');
  const flags = new Set(argument.flags.split(''));
  if (argument.ignoreCase === false) flags.delete('i');
  if (argument.ignoreCase === true) flags.add('i');
  const expression = new RegExp(argument.source, [...flags].join(''));
  return { matches: expression.test(received), received };
}"#;

const STRING_EXPECTATION_SOURCE: &str = r#"(element, argument) => {
  let received;
  if (argument.kind === 'class') {
    received = element.classList.toString();
  } else if (argument.kind === 'value') {
    if (element.nodeName === 'LABEL' && element.control)
      element = element.control;
    if (element.nodeName !== 'INPUT' && element.nodeName !== 'TEXTAREA' && element.nodeName !== 'SELECT')
      throw new Error('Not an input element');
    received = element.value;
  } else {
    throw new Error('Unknown string expectation kind');
  }
  const matches = argument.flags === undefined
    ? received === argument.expected
    : new RegExp(argument.expected, argument.flags).test(received);
  return { matches, received };
}"#;

const CLASS_NAMES_EXPECTATION_SOURCE: &str = r#"(elements, argument) => {
  const received = elements.map(element => element.classList.toString());
  const matches = received.length === argument.expected.length &&
    received.every((value, index) => {
      const matcher = argument.expected[index];
      return matcher.flags === undefined
        ? value === matcher.expected
        : new RegExp(matcher.expected, matcher.flags).test(value);
    });
  return { matches, received };
}"#;

fn value_expectation_result(
    value: crate::evaluation::EvaluationValue,
) -> Result<(bool, crate::evaluation::EvaluationValue), HostError> {
    let crate::evaluation::EvaluationValue::Object { properties, .. } = value else {
        return Err(HostError::Backend(
            "value expectation received a non-object result".into(),
        ));
    };
    let mut matches = None;
    let mut received = None;
    for property in properties {
        match (property.key.as_str(), property.value) {
            ("matches", crate::evaluation::EvaluationValue::Bool(value)) => matches = Some(value),
            ("received", value) => received = Some(value),
            _ => {}
        }
    }
    Ok((
        matches
            .ok_or_else(|| HostError::Backend("value expectation result omitted matches".into()))?,
        received.ok_or_else(|| {
            HostError::Backend("value expectation result omitted received".into())
        })?,
    ))
}

fn texts_value_expectation_result(
    value: crate::evaluation::EvaluationValue,
) -> Result<(bool, Vec<String>), HostError> {
    let (matches, received) = value_expectation_result(value)?;
    let crate::evaluation::EvaluationValue::Array { items, .. } = received else {
        return Err(HostError::Backend(
            "text collection expectation received a non-array result".into(),
        ));
    };
    if items.len() > 1024 {
        return Err(HostError::ResourceExhausted(
            "text collection expectation result exceeds 1024 items".into(),
        ));
    }
    let mut received = Vec::with_capacity(items.len());
    let mut bytes = 0usize;
    for item in items {
        let crate::evaluation::EvaluationValue::String(item) = item else {
            return Err(HostError::Backend(
                "text collection expectation result contains a non-string value".into(),
            ));
        };
        bytes = bytes.checked_add(item.len()).ok_or_else(|| {
            HostError::ResourceExhausted(
                "text collection expectation result byte count overflowed".into(),
            )
        })?;
        if bytes > 1024 * 1024 {
            return Err(HostError::ResourceExhausted(
                "text collection expectation result exceeds 1048576 UTF-8 bytes".into(),
            ));
        }
        received.push(item);
    }
    Ok((matches, received))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorExpectationState {
    Attached,
    Detached,
    Empty,
    Checked,
    Unchecked,
    Indeterminate,
    Editable,
    Readonly,
    Enabled,
    Disabled,
    Focused,
}

impl LocatorExpectationState {
    const fn read_operation(self) -> Option<LocatorRead> {
        match self {
            Self::Attached | Self::Detached => Some(LocatorRead::Count),
            Self::Checked | Self::Unchecked => Some(LocatorRead::IsChecked),
            Self::Editable | Self::Readonly => Some(LocatorRead::IsEditable),
            Self::Enabled | Self::Disabled => Some(LocatorRead::IsEnabled),
            Self::Empty | Self::Indeterminate | Self::Focused => None,
        }
    }

    fn received_from_read(self, value: LocatorValue) -> Result<&'static str, HostError> {
        match (self, value) {
            (Self::Attached | Self::Detached, LocatorValue::Count(0)) => Ok("detached"),
            (Self::Attached | Self::Detached, LocatorValue::Count(1)) => Ok("attached"),
            (Self::Attached | Self::Detached, LocatorValue::Count(_)) => Err(HostError::Backend(
                "strict mode violation: attachment expectation resolved to multiple elements"
                    .into(),
            )),
            (Self::Checked | Self::Unchecked, LocatorValue::Bool(value)) => {
                Ok(if value { "checked" } else { "unchecked" })
            }
            (Self::Editable | Self::Readonly, LocatorValue::Bool(value)) => {
                Ok(if value { "editable" } else { "readOnly" })
            }
            (Self::Enabled | Self::Disabled, LocatorValue::Bool(value)) => {
                Ok(if value { "enabled" } else { "disabled" })
            }
            _ => Err(HostError::Backend(
                "state expectation read returned an incompatible value".into(),
            )),
        }
    }

    fn source(self) -> Option<&'static str> {
        match self {
            Self::Empty => Some(
                "element => { const value = element.nodeName === 'INPUT' || element.nodeName === 'TEXTAREA' ? element.value : (element.textContent ?? '').trim(); return value ? 'notEmpty' : 'empty'; }",
            ),
            Self::Indeterminate => Some(
                r#"element => {
  let target = element?.nodeType === Node.ELEMENT_NODE ? element : element?.parentElement;
  if (target && !target.matches('input, textarea, select') && !target.isContentEditable)
    target = target.closest('button, [role=button], [role=checkbox], [role=radio]') || target;
  if (target && !target.matches('a, input, textarea, button, select, [role=link], [role=button], [role=checkbox], [role=radio]') && !target.isContentEditable) {
    const label = target.closest('label');
    if (label?.control) target = label.control;
  }
  if (!target || !target.isConnected) return 'retry';
  if (target.nodeName === 'INPUT' && (target.type === 'checkbox' || target.type === 'radio'))
    return target.indeterminate ? 'mixed' : target.checked ? 'checked' : 'unchecked';
  const role = target.nodeName === 'OPTION' ? 'option' :
    (target.getAttribute('role') || '').toLowerCase().split(/\s+/).find(value => value);
  const checkable = new Set(['checkbox', 'menuitemcheckbox', 'option', 'radio',
    'switch', 'menuitemradio', 'treeitem']);
  if (!checkable.has(role)) throw new Error('Not a checkbox or radio button');
  const value = (target.getAttribute('aria-checked') || '').toLowerCase();
  return value === 'mixed' ? 'mixed' : value === 'true' ? 'checked' : 'unchecked';
}"#,
            ),
            Self::Focused => Some(
                r#"element => {
  if (!element || !element.isConnected) return 'retry';
  const root = element.getRootNode();
  const focused = root?.activeElement === element && !!element.ownerDocument && element.ownerDocument.hasFocus();
  return focused ? 'focused' : 'inactive';
}"#,
            ),
            Self::Attached
            | Self::Detached
            | Self::Checked
            | Self::Unchecked
            | Self::Editable
            | Self::Readonly
            | Self::Enabled
            | Self::Disabled => None,
        }
    }

    fn matches(self, received: Option<&str>, is_not: bool) -> Result<bool, HostError> {
        match (self, received) {
            (Self::Attached, None) => Ok(false),
            (Self::Detached, None) => Ok(true),
            (_, None) => Ok(is_not),
            (Self::Attached, Some("attached")) => Ok(true),
            (Self::Attached, Some("detached")) => Ok(false),
            (Self::Detached, Some("attached")) => Ok(false),
            (Self::Detached, Some("detached")) => Ok(true),
            (Self::Empty, Some("empty")) => Ok(true),
            (Self::Empty, Some("notEmpty")) => Ok(false),
            (Self::Checked, Some("checked")) => Ok(true),
            (Self::Checked, Some("unchecked" | "mixed")) => Ok(false),
            (Self::Unchecked, Some("unchecked")) => Ok(true),
            (Self::Unchecked, Some("checked" | "mixed")) => Ok(false),
            (Self::Indeterminate, Some("mixed")) => Ok(true),
            (Self::Indeterminate, Some("checked" | "unchecked")) => Ok(false),
            (Self::Editable, Some("editable")) => Ok(true),
            (Self::Editable, Some("readOnly" | "disabled")) => Ok(false),
            (Self::Readonly, Some("editable")) => Ok(false),
            (Self::Readonly, Some("readOnly" | "disabled")) => Ok(true),
            (Self::Enabled, Some("enabled")) => Ok(true),
            (Self::Enabled, Some("disabled")) => Ok(false),
            (Self::Disabled, Some("enabled")) => Ok(false),
            (Self::Disabled, Some("disabled")) => Ok(true),
            (Self::Focused, Some("focused")) => Ok(true),
            (Self::Focused, Some("inactive")) => Ok(false),
            (_, Some(value)) => Err(HostError::Backend(format!(
                "state expectation received an unknown value: {value}"
            ))),
        }
    }
}

pub struct LocatorVisibilityExpectation {
    pub matches: bool,
    pub received: bool,
    pub preview: Option<String>,
    pub missing: bool,
    pub timed_out: bool,
}

#[derive(Clone, Copy)]
pub struct TextExpectation<'a> {
    pub expected: &'a str,
    pub substring: bool,
    pub normalize_whitespace: bool,
    pub ignore_case: bool,
    pub use_inner_text: bool,
    pub is_not: bool,
}

pub struct TextsExpectation<'a> {
    pub expected: &'a [String],
    pub substring: bool,
    pub normalize_whitespace: bool,
    pub ignore_case: bool,
    pub use_inner_text: bool,
    pub is_not: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorStringExpectationKind {
    Class,
    Value,
}

impl LocatorStringExpectationKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Class => "class",
            Self::Value => "value",
        }
    }
}

#[derive(Clone, Copy)]
pub struct StringMatcher<'a> {
    pub expected: &'a str,
    pub flags: Option<&'a str>,
}

#[derive(Clone, Copy)]
pub struct AttributeExpectation<'a> {
    pub name: &'a str,
    pub expected: &'a str,
    pub flags: Option<&'a str>,
    pub ignore_case: Option<bool>,
    pub is_not: bool,
}

#[derive(Clone, Copy)]
pub struct CssExpectation<'a> {
    pub name: &'a str,
    pub expected: &'a str,
    pub flags: Option<&'a str>,
    pub is_not: bool,
}

fn normalized(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn text_matches(actual: &str, expectation: &TextExpectation<'_>) -> bool {
    let actual = if expectation.normalize_whitespace {
        normalized(actual)
    } else {
        actual.to_owned()
    };
    let expected = if expectation.normalize_whitespace {
        normalized(expectation.expected)
    } else {
        expectation.expected.to_owned()
    };
    let (actual, expected) = if expectation.ignore_case {
        (actual.to_lowercase(), expected.to_lowercase())
    } else {
        (actual, expected)
    };
    if expectation.substring {
        actual.contains(&expected)
    } else {
        actual == expected
    }
}

fn text_expectation_matches(
    received: Option<&str>,
    missing: bool,
    expectation: &TextExpectation<'_>,
) -> bool {
    if missing {
        return expectation.is_not;
    }
    received.is_some_and(|actual| text_matches(actual, expectation))
}

fn texts_expectation_matches(received: &[String], expectation: &TextsExpectation<'_>) -> bool {
    received.len() == expectation.expected.len()
        && received
            .iter()
            .zip(expectation.expected)
            .all(|(actual, expected)| {
                text_matches(
                    actual,
                    &TextExpectation {
                        expected,
                        substring: expectation.substring,
                        normalize_whitespace: expectation.normalize_whitespace,
                        ignore_case: expectation.ignore_case,
                        use_inner_text: expectation.use_inner_text,
                        is_not: false,
                    },
                )
            })
}

#[derive(Clone)]
pub struct HostLocator {
    host: Weak<HostSession>,
    frame: ObjectId,
    #[cfg(feature = "handles")]
    content_frame_owner: Option<Arc<HostLocator>>,
    #[cfg(feature = "handles")]
    deferred_frame_combination: Option<Arc<DeferredFrameCombination>>,
    query: LocatorQuery,
}

#[cfg(feature = "handles")]
struct DeferredFrameCombination {
    left: HostLocator,
    right: HostLocator,
    combination: QueryCombination,
}

#[cfg(feature = "handles")]
impl DeferredFrameCombination {
    fn error(&self) -> Result<HostError, HostError> {
        let method = match self.combination {
            QueryCombination::And => "and",
            QueryCombination::Or => "or",
            _ => {
                return Err(HostError::Backend(
                    "invalid deferred frame combination".into(),
                ));
            }
        };
        Ok(HostError::Backend(format!(
            "Frame locators are not allowed inside composite locators, while querying \"{}.{method}({})\"",
            self.left.call_log()?,
            self.right.call_log()?,
        )))
    }
}

/// Lazy FrameLocator recipe. The owner iframe Locator and every nested frame
/// hop stay in Rust; bindings retain only this bounded immutable value.
#[cfg(feature = "handles")]
#[derive(Clone)]
pub struct HostFrameLocator {
    owner: Arc<HostLocator>,
}

impl HostSession {
    pub fn locator(
        self: &Arc<Self>,
        frame: &str,
        query: LocatorQuery,
    ) -> Result<HostLocator, HostError> {
        let frame = frame
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Frame ID".into()))?;
        self.require_visible_frame_id(frame)?;
        Ok(HostLocator {
            host: Arc::downgrade(self),
            frame,
            #[cfg(feature = "handles")]
            content_frame_owner: None,
            #[cfg(feature = "handles")]
            deferred_frame_combination: None,
            query,
        })
    }
}

impl HostLocator {
    /// Validate the immutable selector recipe without issuing a browser
    /// command. This surfaces deferred parser issues at the public API
    /// boundary while retaining no additional query ownership.
    pub fn validate(&self) -> Result<(), HostError> {
        #[cfg(any(
            feature = "selector-registration",
            feature = "layout-selectors",
            feature = "react-selectors"
        ))]
        {
            let (_plan, _lease) = self.query.command_plan()?;
            Ok(())
        }
        #[cfg(not(any(
            feature = "selector-registration",
            feature = "layout-selectors",
            feature = "react-selectors"
        )))]
        {
            let (_selector, _lease) = self.query.command_selector()?;
            Ok(())
        }
    }

    /// Return `None` for an ordinary selector, or the current match count for
    /// a browser-program selector in its Playwright execution world. This lets
    /// a thin binding preserve `query_selector(...)->None` without attempting
    /// to parse or execute custom or layout engines itself.
    #[cfg(any(feature = "selector-registration", feature = "layout-selectors"))]
    pub async fn browser_program_match_count(&self) -> Result<Option<u32>, HostError> {
        let host = self.host()?;
        let frame = self.resolve_frame(30_000).await?;
        let (plan, _query_lease) = self.query.command_plan()?;
        let value = match plan {
            CommandPlan::Selector(_) => return Ok(None),
            CommandPlan::Browser(_) => {
                return Err(HostError::UnsupportedOperation(
                    "browser selector count is not installed".into(),
                ));
            }
            #[cfg(feature = "selector-registration")]
            CommandPlan::Registered(program) => {
                host.locator_query_all_registered(frame, program, || async {
                    Ok(
                        crate::evaluation::EvaluateParams::function_without_argument(
                            "elements => elements.length",
                        ),
                    )
                })
                .await?
            }
            #[cfg(feature = "layout-selectors")]
            CommandPlan::Layout(program) => {
                host.locator_query_all_layout(frame, program, || async {
                    Ok(
                        crate::evaluation::EvaluateParams::function_without_argument(
                            "elements => elements.length",
                        ),
                    )
                })
                .await?
            }
        };
        let crate::evaluation::EvaluationValue::Number(value) = value else {
            return Err(HostError::Backend(
                "browser-program selector count returned a non-number".into(),
            ));
        };
        let value = value.get();
        if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > f64::from(u32::MAX)
        {
            return Err(HostError::Backend(
                "browser-program selector count returned an invalid number".into(),
            ));
        }
        Ok(Some(value as u32))
    }

    #[cfg(feature = "file-input")]
    pub(crate) fn file_input_query(
        &self,
    ) -> Result<
        (
            ObjectId,
            pwright_cap_locator::Selector,
            crate::locator_query::QueryLease,
        ),
        HostError,
    > {
        #[cfg(feature = "handles")]
        if self.content_frame_owner.is_some() {
            return Err(HostError::UnsupportedOperation(
                "file input through FrameLocator requires async Frame resolution".into(),
            ));
        }
        let (selector, lease) = self.query.command_selector()?;
        Ok((self.frame, selector, lease))
    }
    /// Strictly select/adopt first, then invoke the caller's value-conversion
    /// continuation exactly once. Native ownership stays inside one controlled
    /// Frame command through conversion, evaluation and acknowledged release.
    /// The callback and its future have a 4 KiB inline bound; external retained
    /// Node references/buffers remain the caller's separately admitted budget.
    pub async fn evaluate_with<F, Converted>(
        &self,
        timeout_ms: u32,
        convert: F,
    ) -> Result<crate::evaluation::EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: std::future::Future<Output = Result<crate::evaluation::EvaluateParams, HostError>>
            + Send
            + 'static,
    {
        let host = self.host()?;
        let frame = self.resolve_frame(timeout_ms).await?;
        #[cfg(any(feature = "selector-registration", feature = "layout-selectors"))]
        {
            let (plan, _query_lease) = self.query.command_plan()?;
            match plan {
                CommandPlan::Selector(selector) => {
                    host.locator_evaluate(frame, selector, timeout_ms, convert)
                        .await
                }
                CommandPlan::Browser(_) => Err(HostError::UnsupportedOperation(
                    "browser selector Locator evaluation is not installed".into(),
                )),
                #[cfg(feature = "selector-registration")]
                CommandPlan::Registered(program) => {
                    host.locator_evaluate_registered(frame, program, timeout_ms, convert)
                        .await
                }
                #[cfg(feature = "layout-selectors")]
                CommandPlan::Layout(program) => {
                    host.locator_evaluate_layout(frame, program, timeout_ms, convert)
                        .await
                }
            }
        }
        #[cfg(not(any(feature = "selector-registration", feature = "layout-selectors")))]
        {
            let (selector, _query_lease) = self.query.command_selector()?;
            host.locator_evaluate(frame, selector, timeout_ms, convert)
                .await
        }
    }

    /// Resolve one strict Element under the caller's timeout, evaluate on that
    /// exact unpublished owner, and return only the resulting Handle. Dropping
    /// the temporary Element owner schedules its native release on every
    /// success, conversion-error and cancellation path.
    #[cfg(feature = "handles")]
    pub async fn evaluate_handle_with<F, Converted>(
        &self,
        timeout_ms: u32,
        convert: F,
    ) -> Result<crate::handles::PendingHandle, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: std::future::Future<Output = Result<crate::evaluation::EvaluateParams, HostError>>
            + Send
            + 'static,
    {
        if std::mem::size_of::<F>() > crate::locator_evaluation::CONVERSION_STATE_BYTES
            || std::mem::size_of::<Converted>() > crate::locator_evaluation::CONVERSION_STATE_BYTES
        {
            return Err(HostError::ResourceExhausted(
                "Locator Handle conversion state exceeds 4096 bytes".into(),
            ));
        }
        let element = self.element_handle(timeout_ms).await?;
        let params: crate::handles::EvaluateHandleParams = convert().await?.into();
        let host = self.host()?;
        let handle = host.handle_prepare_program(&element, params.into()).await?;
        if !handle.is_element() {
            return Ok(handle);
        }
        let argument = crate::handles::OwnedHandleArgument::from_slots(
            crate::evaluation::EvaluationValue::Undefined,
            Vec::new(),
        )?;
        match host
            .handle_evaluate_owned(&handle, EXPECTATION_PREVIEW_SOURCE.to_owned(), argument)
            .await?
        {
            crate::evaluation::EvaluationValue::String(value) => {
                Ok(handle.with_preview(format!("JSHandle@{value}")))
            }
            _ => Err(HostError::Backend(
                "Locator Handle preview returned a non-string value".into(),
            )),
        }
    }

    /// Select the first current match without Locator strictness, then invoke
    /// the caller's bounded value-conversion continuation exactly once. This
    /// is the Page `eval_on_selector` path; Locator `evaluate` continues to use
    /// strict resolution through [`Self::evaluate_with`].
    pub async fn evaluate_first_with<F, Converted>(
        &self,
        timeout_ms: u32,
        convert: F,
    ) -> Result<crate::evaluation::EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: std::future::Future<Output = Result<crate::evaluation::EvaluateParams, HostError>>
            + Send
            + 'static,
    {
        let host = self.host()?;
        let frame = self.resolve_frame(timeout_ms).await?;
        #[cfg(any(feature = "selector-registration", feature = "layout-selectors"))]
        {
            let (plan, _query_lease) = self.query.command_plan()?;
            match plan {
                CommandPlan::Selector(selector) => {
                    host.locator_evaluate_first(frame, selector, timeout_ms, convert)
                        .await
                }
                CommandPlan::Browser(_) => Err(HostError::UnsupportedOperation(
                    "browser selector first-match evaluation is not installed".into(),
                )),
                #[cfg(feature = "selector-registration")]
                CommandPlan::Registered(program) => {
                    host.locator_evaluate_registered(frame, program, timeout_ms, convert)
                        .await
                }
                #[cfg(feature = "layout-selectors")]
                CommandPlan::Layout(program) => {
                    host.locator_evaluate_layout(frame, program, timeout_ms, convert)
                        .await
                }
            }
        }
        #[cfg(not(any(feature = "selector-registration", feature = "layout-selectors")))]
        {
            let (selector, _query_lease) = self.query.command_selector()?;
            host.locator_evaluate_first(frame, selector, timeout_ms, convert)
                .await
        }
    }

    /// Resolve every current match in the Frame's utility world and evaluate
    /// one function against the exact collection. The selector recipe and DOM
    /// identities remain Rust-owned; Node supplies only the bounded evaluation
    /// value conversion.
    pub async fn evaluate_all_with<F, Converted>(
        &self,
        convert: F,
    ) -> Result<crate::evaluation::EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: std::future::Future<Output = Result<crate::evaluation::EvaluateParams, HostError>>
            + Send
            + 'static,
    {
        self.evaluate_all_with_optional_timeout(None, convert).await
    }

    /// Resolve every current match and evaluate one Rust-selected collection
    /// function within the caller's total deadline. This is used by polling
    /// expectations so a single selector/evaluation attempt cannot outlive
    /// the public matcher timeout.
    async fn evaluate_all_with_timeout<F, Converted>(
        &self,
        timeout_ms: u32,
        convert: F,
    ) -> Result<crate::evaluation::EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: std::future::Future<Output = Result<crate::evaluation::EvaluateParams, HostError>>
            + Send
            + 'static,
    {
        self.evaluate_all_with_optional_timeout(Some(timeout_ms), convert)
            .await
    }

    async fn evaluate_all_with_optional_timeout<F, Converted>(
        &self,
        timeout_ms: Option<u32>,
        convert: F,
    ) -> Result<crate::evaluation::EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: std::future::Future<Output = Result<crate::evaluation::EvaluateParams, HostError>>
            + Send
            + 'static,
    {
        let host = self.host()?;
        let frame = self.resolve_frame(timeout_ms.unwrap_or(30_000)).await?;
        #[cfg(any(
            feature = "selector-registration",
            feature = "layout-selectors",
            feature = "react-selectors"
        ))]
        {
            let (plan, _query_lease) = self.query.command_plan()?;
            match plan {
                CommandPlan::Selector(selector) => {
                    host.locator_evaluate_all(frame, selector, timeout_ms, convert)
                        .await
                }
                CommandPlan::Browser(_program) => {
                    #[cfg(feature = "react-selectors")]
                    if _program.contains_react() {
                        return host
                            .locator_evaluate_all_react(frame, _program, timeout_ms, convert)
                            .await;
                    }
                    Err(HostError::UnsupportedOperation(
                        "browser selector all-match evaluation is not installed".into(),
                    ))
                }
                #[cfg(feature = "selector-registration")]
                CommandPlan::Registered(program) => {
                    host.locator_evaluate_all_registered(frame, program, timeout_ms, convert)
                        .await
                }
                #[cfg(feature = "layout-selectors")]
                CommandPlan::Layout(program) => {
                    host.locator_evaluate_all_layout(frame, program, timeout_ms, convert)
                        .await
                }
            }
        }
        #[cfg(not(any(
            feature = "selector-registration",
            feature = "layout-selectors",
            feature = "react-selectors"
        )))]
        {
            let (selector, _query_lease) = self.query.command_selector()?;
            host.locator_evaluate_all(frame, selector, timeout_ms, convert)
                .await
        }
    }

    /// Resolve this immutable recipe to one strict attached ElementHandle.
    /// The binding receives only the owned handle publication; selector
    /// compilation, runtime installation and strictness remain in Rust.
    #[cfg(feature = "handles")]
    pub async fn element_handle(
        &self,
        timeout_ms: u32,
    ) -> Result<crate::handles::PendingHandle, HostError> {
        let frame = self.resolve_frame(timeout_ms).await?;
        self.element_handle_in_frame(frame, timeout_ms).await
    }

    /// Run one non-waiting selector query and atomically return its first
    /// ElementHandle or `None`. The browser result is classified before any
    /// portable Handle publication, so absence never creates a temporary
    /// Handle and DOM mutation cannot split a count/read pair.
    #[cfg(feature = "handles")]
    pub async fn query_element_handle(
        &self,
    ) -> Result<Option<crate::handles::PendingHandle>, HostError> {
        let frame = self.resolve_frame(0).await?;
        self.query_element_handle_in_frame(frame).await
    }

    /// Run one non-waiting selector snapshot and atomically retain every
    /// matching ElementHandle. Collection construction, indexed validation,
    /// batch publication and temporary cleanup stay below the FFI boundary.
    #[cfg(feature = "element-handle-dom")]
    pub async fn query_element_handles(
        &self,
    ) -> Result<Vec<crate::handles::PendingHandle>, HostError> {
        let frame = self.resolve_frame(0).await?;
        self.query_element_handles_in_frame(frame).await
    }

    /// Apply Page.wait_for_selector semantics to this immutable query. Each
    /// attempt checks the state and returns the selected Element in one browser
    /// evaluation, so a later DOM mutation cannot replace the observed Handle.
    /// Negative states complete without creating a Handle.
    #[cfg(feature = "handles")]
    pub async fn wait_for_element(
        &self,
        state: LocatorWaitState,
        timeout_ms: u32,
        strict: bool,
    ) -> Result<Option<crate::handles::PendingHandle>, HostError> {
        let deadline = (timeout_ms != 0)
            .then(|| tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms)));
        let frame = self
            .resolve_frame(timeout_ms)
            .await
            .map_err(|error| selector_wait_error(error, timeout_ms))?;
        let host = self.host()?;
        let (plan, _query_lease) = self.query.command_plan()?;
        let matches = match plan {
            CommandPlan::Selector(selector) => {
                compile_selector(&selector, "document").map_err(HostError::backend)?
            }
            CommandPlan::Browser(program) => program.compile("document")?,
            #[cfg(feature = "selector-registration")]
            CommandPlan::Registered(_) => {
                return Err(HostError::UnsupportedOperation(
                    "registered selector Page wait is not installed".into(),
                ));
            }
            #[cfg(feature = "layout-selectors")]
            CommandPlan::Layout(_) => {
                return Err(HostError::UnsupportedOperation(
                    "layout selector Page wait is not installed".into(),
                ));
            }
        };
        let source = selector_wait_source(matches, state, strict)?;
        let frame_id = frame.get().to_string();
        let mut poll_count = 0usize;
        loop {
            let remaining = remaining_timeout_ms(deadline.as_ref(), timeout_ms)?;
            let attempt = async {
                match state {
                    LocatorWaitState::Attached | LocatorWaitState::Visible => host
                        .frame_prepare_optional_element_program(
                            &frame_id,
                            crate::handles::EvaluateHandleParams::expression(source.clone()).into(),
                        )
                        .await
                        .map(|handle| match handle {
                            Some(handle) => SelectorWaitAttempt::Complete(Some(handle)),
                            None => SelectorWaitAttempt::Pending,
                        }),
                    LocatorWaitState::Detached | LocatorWaitState::Hidden => {
                        let value = host
                            .frame_evaluate_with_timeout(
                                &frame_id,
                                crate::evaluation::EvaluateParams::expression(source.clone()),
                                remaining,
                            )
                            .await?;
                        match value {
                            crate::evaluation::EvaluationValue::Bool(true) => {
                                Ok(SelectorWaitAttempt::Complete(None))
                            }
                            crate::evaluation::EvaluationValue::Bool(false) => {
                                Ok(SelectorWaitAttempt::Pending)
                            }
                            _ => Err(HostError::Backend(
                                "selector wait returned a non-boolean state".into(),
                            )),
                        }
                    }
                }
            };
            let outcome = if let Some(deadline) = deadline {
                tokio::time::timeout_at(deadline, attempt)
                    .await
                    .map_err(|_| HostError::Timeout(timeout_ms))?
            } else {
                attempt.await
            };
            match outcome {
                Ok(SelectorWaitAttempt::Complete(handle)) => return Ok(handle),
                Ok(SelectorWaitAttempt::Pending) => {}
                Err(error) if selector_wait_context_was_replaced(&error) => {
                    self.retry_selector_wait_after_context_loss(
                        frame,
                        error,
                        deadline.as_ref(),
                        timeout_ms,
                    )
                    .await?;
                    continue;
                }
                Err(error) => return Err(selector_wait_error(error, timeout_ms)),
            }
            let delay_ms = [20, 50, 100, 100, 500]
                .get(poll_count)
                .copied()
                .unwrap_or(500);
            poll_count = poll_count.saturating_add(1);
            let delay = deadline.map_or(Duration::from_millis(delay_ms), |at| {
                at.saturating_duration_since(tokio::time::Instant::now())
                    .min(Duration::from_millis(delay_ms))
            });
            if delay.is_zero() {
                return Err(HostError::Timeout(timeout_ms));
            }
            tokio::time::sleep(delay).await;
        }
    }

    #[cfg(feature = "handles")]
    async fn retry_selector_wait_after_context_loss(
        &self,
        frame: ObjectId,
        error: HostError,
        deadline: Option<&tokio::time::Instant>,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        let same_frame_active = self.host()?.frame_metadata.is_active(frame)?;
        let delay = selector_wait_retry_delay(error, same_frame_active, deadline, timeout_ms)?;
        tokio::time::sleep(delay).await;
        Ok(())
    }

    /// Resolve this Locator to one temporary ElementHandle, execute the
    /// caller's bounded function through the selected Handle capability, and
    /// return an owned result Handle. Node previewing is performed by a static
    /// Rust-owned program before the temporary element owner is released.
    #[cfg(feature = "handles")]
    pub async fn evaluate_handle(
        &self,
        source: String,
        is_function: bool,
        argument: crate::evaluation::EvaluationValue,
        timeout_ms: u32,
    ) -> Result<LocatorHandleResult, HostError> {
        let element = self.element_handle(timeout_ms).await?;
        let host = self.host()?;
        let argument = crate::handles::OwnedHandleArgument::from_slots(argument, Vec::new())?;
        let handle = host
            .handle_prepare_owned(&element, source, is_function, argument)
            .await?;
        let preview = if handle.is_element() {
            let argument = crate::handles::OwnedHandleArgument::from_slots(
                crate::evaluation::EvaluationValue::Undefined,
                Vec::new(),
            )?;
            match host
                .handle_evaluate_owned(&handle, EXPECTATION_PREVIEW_SOURCE.to_owned(), argument)
                .await?
            {
                crate::evaluation::EvaluationValue::String(value) => {
                    Some(format!("JSHandle@{value}"))
                }
                _ => {
                    return Err(HostError::Backend(
                        "Locator Handle preview returned a non-string value".into(),
                    ));
                }
            }
        } else {
            None
        };
        Ok(LocatorHandleResult { handle, preview })
    }

    /// Return one bounded, Playwright-compatible element preview for matcher
    /// diagnostics. Selection and serialization stay in the Rust-owned
    /// Locator lane; bindings never receive a DOM handle.
    async fn expectation_preview(
        &self,
        timeout_ms: u32,
    ) -> Result<(Option<String>, bool), HostError> {
        match self.read(LocatorRead::Count, 0, None).await {
            Ok(LocatorValue::Count(0)) => Ok((None, true)),
            Ok(LocatorValue::Count(1)) => match self
                .evaluate_with(timeout_ms.clamp(1, 100), || async {
                    Ok(
                        crate::evaluation::EvaluateParams::function_without_argument(
                            EXPECTATION_PREVIEW_SOURCE,
                        ),
                    )
                })
                .await
            {
                Ok(crate::evaluation::EvaluationValue::String(value)) => Ok((Some(value), false)),
                Ok(_) => Err(HostError::Backend(
                    "expectation preview returned a non-string value".into(),
                )),
                Err(HostError::Timeout(_)) => Ok((None, false)),
                Err(error) if locator_context_was_replaced(&error) => Ok((None, false)),
                Err(error) => Err(error),
            },
            Ok(LocatorValue::Count(_)) => Err(HostError::Backend(
                "strict mode violation: expectation resolved to multiple elements".into(),
            )),
            Ok(_) => Err(HostError::Backend(
                "expectation preview count returned an incompatible value".into(),
            )),
            Err(HostError::Timeout(_)) => Ok((None, false)),
            Err(error) if locator_context_was_replaced(&error) => Ok((None, false)),
            Err(error) => Err(error),
        }
    }

    #[cfg(feature = "locator-bounding-box")]
    pub async fn bounding_box(
        &self,
        timeout_ms: u32,
    ) -> Result<Option<LocatorBoundingBox>, HostError> {
        self.evaluate_with(timeout_ms, || async {
            Ok(crate::evaluation::EvaluateParams::function_without_argument(BOUNDING_BOX_SOURCE))
        })
        .await
        .and_then(bounding_box_result)
    }

    #[cfg(feature = "locator-screenshot")]
    pub async fn screenshot(
        &self,
        options: crate::screenshot::ScreenshotOptions,
        timeout_ms: u32,
    ) -> Result<crate::screenshot::ScreenshotOutput, HostError> {
        self.screenshot_with_masks(options, Vec::new(), timeout_ms)
            .await
    }

    #[cfg(feature = "locator-screenshot")]
    pub async fn screenshot_with_masks(
        &self,
        options: crate::screenshot::ScreenshotOptions,
        masks: Vec<HostLocator>,
        timeout_ms: u32,
    ) -> Result<crate::screenshot::ScreenshotOutput, HostError> {
        let (selector, _query_lease) = self.query.command_selector()?;
        let host = self.host()?;
        let frame = self.resolve_frame(timeout_ms).await?;
        let token = crate::screenshot::begin_masks(&masks).await?;
        let capture = host
            .invoke::<LocatorScreenshot>(
                &frame.get().to_string(),
                LocatorScreenshotParams::new(selector, options),
                timeout_ms,
            )
            .await
            .map(crate::screenshot::ScreenshotOutput::from_capture);
        crate::screenshot::finish_masks(&masks, token.as_deref(), capture).await
    }

    /// Install one browser-resident overlay for this Frame. Repeated calls
    /// replace the previous overlay and animation-frame loop in the same
    /// utility world; navigation discards that world. Rust selects the static
    /// program and exact Locator collection while Node supplies only the
    /// bounded public description used by the tooltip.
    #[cfg(feature = "locator-highlight")]
    pub async fn highlight(&self, description: String) -> Result<(), HostError> {
        if description.len() > 64 * 1024 {
            return Err(HostError::InvalidArgument(
                "Locator description exceeds 65536 bytes".into(),
            ));
        }
        let under_test = std::env::var_os("PWTEST_UNDER_TEST").is_some_and(|value| {
            value != "0" && value.to_string_lossy().to_ascii_lowercase() != "false"
        });
        let argument = crate::evaluation::EvaluationValue::Object {
            id: None,
            properties: vec![
                crate::evaluation::EvaluationProperty::new(
                    "description",
                    crate::evaluation::EvaluationValue::String(description),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "underTest",
                    crate::evaluation::EvaluationValue::Bool(under_test),
                ),
            ],
        };
        self.evaluate_all_with(move || async move {
            Ok(crate::evaluation::EvaluateParams::function(
                HIGHLIGHT_SOURCE,
                argument,
            ))
        })
        .await
        .map(|_| ())
    }

    /// Dispatch one bounded DOM event through a Rust-selected static program.
    /// The binding provides only the event type and serializable initializer;
    /// selector resolution and executable source remain owned by Rust.
    #[cfg(feature = "locator-dom-actions")]
    pub async fn dispatch_event(
        &self,
        event_type: String,
        event_init: crate::evaluation::EvaluationValue,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        let argument = dispatch_event_argument(event_type, event_init)?;
        self.evaluate_with(timeout_ms, move || async move {
            Ok(crate::evaluation::EvaluateParams::function(
                DISPATCH_EVENT_SOURCE,
                argument,
            ))
        })
        .await
        .map(|_| ())
    }

    /// Event initializers containing live Handles must execute in the exact
    /// target world. Resolve one temporary element owner, evaluate with the
    /// retained argument owners, then let the temporary owner release. Plain
    /// values continue through the atomic selector/evaluation command above.
    #[cfg(all(
        feature = "locator-dom-actions",
        feature = "handles",
        feature = "element-handle-dom"
    ))]
    pub async fn dispatch_event_owned(
        &self,
        event_type: String,
        event_init: crate::handles::OwnedHandleArgument,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        let element = self.element_handle(timeout_ms).await?;
        self.host()?
            .element_handle_dispatch_event_owned(&element, event_type, event_init)
            .await
    }

    /// Focus and select text with one static Rust-owned DOM program. This leaf
    /// is independently selectable from click, keyboard, Handle and file-input
    /// capabilities.
    #[cfg(feature = "locator-dom-actions")]
    pub async fn select_text(&self, timeout_ms: u32, _force: bool) -> Result<(), HostError> {
        self.evaluate_with(timeout_ms, || async {
            Ok(crate::evaluation::EvaluateParams::function_without_argument(SELECT_TEXT_SOURCE))
        })
        .await
        .map(|_| ())
    }

    pub fn copy(&self) -> Result<Self, HostError> {
        Ok(self.derive(self.query.copy()?))
    }

    pub fn host(&self) -> Result<Arc<HostSession>, HostError> {
        #[cfg(feature = "handles")]
        if let Some(error) = &self.deferred_frame_combination {
            return Err(error.error()?);
        }
        self.host
            .upgrade()
            .filter(|host| host.is_connected())
            .ok_or(HostError::TargetClosed)
    }

    fn derive(&self, query: LocatorQuery) -> Self {
        Self {
            host: self.host.clone(),
            frame: self.frame,
            #[cfg(feature = "handles")]
            content_frame_owner: self.content_frame_owner.clone(),
            #[cfg(feature = "handles")]
            deferred_frame_combination: self.deferred_frame_combination.clone(),
            query,
        }
    }

    #[cfg(feature = "handles")]
    fn same_frame_root(&self, other: &Self) -> bool {
        if self.frame != other.frame {
            return false;
        }
        match (&self.content_frame_owner, &other.content_frame_owner) {
            (None, None) => true,
            (Some(left), Some(right)) => same_frame_owner(left, right),
            _ => false,
        }
    }

    #[cfg(not(feature = "handles"))]
    fn same_frame_root(&self, other: &Self) -> bool {
        self.frame == other.frame
    }

    /// Resolve every retained iframe owner at operation time. No DOM identity
    /// survives the call: each hop strictly selects a temporary Handle, asks
    /// the Handle capability for its current content Frame, then acknowledges
    /// native release before continuing.
    #[cfg(feature = "handles")]
    async fn resolve_frame(&self, timeout_ms: u32) -> Result<ObjectId, HostError> {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        let mut owners = Vec::new();
        let mut cursor = self.content_frame_owner.clone();
        while let Some(owner) = cursor {
            if owners.len() >= MAX_SELECTOR_DEPTH {
                return Err(HostError::ResourceExhausted(
                    "FrameLocator nesting exceeds the selector depth limit".into(),
                ));
            }
            cursor = owner.content_frame_owner.clone();
            owners.push(owner);
        }

        let mut frame = self.frame;
        for owner in owners.into_iter().rev() {
            loop {
                let now = tokio::time::Instant::now();
                if timeout_ms != 0 && now >= deadline {
                    return Err(HostError::Timeout(timeout_ms));
                }
                let remaining_ms = if timeout_ms == 0 {
                    0
                } else {
                    u32::try_from(deadline.saturating_duration_since(now).as_millis())
                        .unwrap_or(u32::MAX)
                        .max(1)
                };
                let fixed = HostLocator {
                    host: owner.host.clone(),
                    frame,
                    content_frame_owner: None,
                    deferred_frame_combination: owner.deferred_frame_combination.clone(),
                    query: owner.query.clone(),
                };
                let handle = fixed
                    .element_handle_in_frame(frame, remaining_ms)
                    .await
                    .map_err(rewrite_frame_owner_error)?;
                let host = fixed.host()?;
                let relation = host.handle_content_frame(&handle).await;
                let diagnostic = if matches!(&relation, Ok(None)) {
                    let argument = crate::handles::OwnedHandleArgument::from_slots(
                        crate::evaluation::EvaluationValue::Undefined,
                        Vec::new(),
                    )?;
                    host.handle_evaluate_owned(
                        &handle,
                        FRAME_OWNER_DIAGNOSTIC_SOURCE.to_owned(),
                        argument,
                    )
                    .await
                    .and_then(non_frame_owner_preview)
                } else {
                    Ok(None)
                };
                let cleanup = handle.dispose().await;
                let content = match (relation, diagnostic, cleanup) {
                    (Err(error), _, _) => return Err(error),
                    (Ok(_), Err(error), _) => return Err(error),
                    (Ok(_), Ok(_), Err(error)) => return Err(error),
                    (Ok(_), Ok(Some(preview)), Ok(())) => {
                        return Err(HostError::Backend(format!(
                            "Selector {} resolved to {preview}, <iframe> was expected",
                            fixed.call_log()?
                        )));
                    }
                    (Ok(content), Ok(None), Ok(())) => content,
                };
                if let Some(content) = content {
                    frame = content
                        .parse::<u64>()
                        .ok()
                        .filter(|id| *id != 0)
                        .map(ObjectId::new)
                        .ok_or_else(|| {
                            HostError::Backend(
                                "FrameLocator resolved an invalid portable Frame ID".into(),
                            )
                        })?;
                    break;
                }
                if timeout_ms == 0 {
                    return Err(HostError::Timeout(0));
                }
                tokio::time::sleep(
                    deadline
                        .saturating_duration_since(tokio::time::Instant::now())
                        .min(Duration::from_millis(10)),
                )
                .await;
            }
        }
        Ok(frame)
    }

    #[cfg(not(feature = "handles"))]
    async fn resolve_frame(&self, _timeout_ms: u32) -> Result<ObjectId, HostError> {
        Ok(self.frame)
    }

    async fn resolved_frame_is_active(&self) -> Result<bool, HostError> {
        let frame = match self.resolve_frame(1).await {
            Ok(frame) => frame,
            Err(HostError::Timeout(_) | HostError::TargetClosed) => return Ok(false),
            Err(error) => return Err(error),
        };
        self.host()?.frame_metadata.is_active(frame)
    }

    /// Decide whether an expectation may retry after its current execution
    /// context disappears. A FrameLocator deliberately resolves its retained
    /// iframe-owner recipe again on every attempt, so replacing that iframe
    /// retires the old child Frame without closing the owner Frame. Plain
    /// Locators continue to require their resolved Frame itself to be active.
    async fn expectation_owner_is_active(&self) -> Result<bool, HostError> {
        #[cfg(feature = "handles")]
        if self.content_frame_owner.is_some() {
            return self.host()?.frame_metadata.is_active(self.frame);
        }
        self.resolved_frame_is_active().await
    }

    #[cfg(feature = "handles")]
    pub fn content_frame(&self) -> Result<HostFrameLocator, HostError> {
        self.validate()?;
        Ok(HostFrameLocator {
            owner: Arc::new(self.copy()?),
        })
    }

    async fn wait_for_in_frame(
        &self,
        frame: ObjectId,
        state: LocatorWaitState,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        let (selector, _query_lease) = self.query.command_selector()?;
        self.wait_for_selector_in_frame(frame, selector, state, timeout_ms)
            .await
    }

    async fn wait_for_selector_in_frame(
        &self,
        frame: ObjectId,
        selector: Selector,
        state: LocatorWaitState,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        let host = self.host()?;
        let outcome = host
            .invoke::<WaitFor>(
                &frame.get().to_string(),
                WaitForParams::new(selector, WaitForOptions::new().with_state(state.into())),
                timeout_ms,
            )
            .await;
        rewrite_strict_selector_error(
            &self.query,
            crate::completion::publish_command_result(
                outcome,
                host.is_connected(),
                host.flush_topology(),
            )
            .await,
        )
    }

    #[cfg(feature = "handles")]
    async fn element_handle_in_frame(
        &self,
        frame: ObjectId,
        timeout_ms: u32,
    ) -> Result<crate::handles::PendingHandle, HostError> {
        let host = self.host()?;
        let (plan, _query_lease) = self.query.command_plan()?;
        match plan {
            CommandPlan::Selector(selector) => {
                let source = element_handle_expression(&selector)?;
                self.wait_for_selector_in_frame(
                    frame,
                    selector,
                    LocatorWaitState::Attached,
                    timeout_ms,
                )
                .await?;
                host.frame_prepare_handle(
                    &frame.get().to_string(),
                    crate::handles::EvaluateHandleParams::expression(source),
                )
                .await
            }
            CommandPlan::Browser(program) => {
                let deadline = (timeout_ms != 0).then(|| {
                    tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms))
                });
                let matches = program.compile("document")?;
                let source = polling_element_handle_expression_from_matches(matches)?;
                loop {
                    let handle = rewrite_strict_selector_error(
                        &self.query,
                        host.frame_prepare_optional_element_program(
                            &frame.get().to_string(),
                            crate::handles::EvaluateHandleParams::expression(source.clone()).into(),
                        )
                        .await,
                    )?;
                    if let Some(handle) = handle {
                        return Ok(handle);
                    }
                    if deadline.is_some_and(|at| tokio::time::Instant::now() >= at) {
                        return Err(HostError::Timeout(timeout_ms));
                    }
                    let delay = deadline
                        .map(|at| {
                            at.saturating_duration_since(tokio::time::Instant::now())
                                .min(Duration::from_millis(10))
                        })
                        .unwrap_or(Duration::from_millis(10));
                    tokio::time::sleep(delay).await;
                }
            }
            #[cfg(feature = "selector-registration")]
            CommandPlan::Registered(program) => {
                host.locator_prepare_registered_element_handle(frame, program)
                    .await
            }
            #[cfg(feature = "layout-selectors")]
            CommandPlan::Layout(_) => Err(HostError::UnsupportedOperation(
                "layout selector ElementHandle queries are not installed".into(),
            )),
        }
    }

    #[cfg(feature = "handles")]
    async fn query_element_handle_in_frame(
        &self,
        frame: ObjectId,
    ) -> Result<Option<crate::handles::PendingHandle>, HostError> {
        let host = self.host()?;
        let (plan, _query_lease) = self.query.command_plan()?;
        let source = match plan {
            CommandPlan::Selector(selector) => optional_element_handle_expression(&selector)?,
            CommandPlan::Browser(program) => {
                let matches = program.compile("document")?;
                optional_element_handle_expression_from_matches(matches)?
            }
            #[cfg(feature = "selector-registration")]
            CommandPlan::Registered(program) => {
                return host
                    .locator_prepare_registered_optional_element_handle(frame, program)
                    .await;
            }
            #[cfg(feature = "layout-selectors")]
            CommandPlan::Layout(_) => {
                return Err(HostError::UnsupportedOperation(
                    "layout selector ElementHandle queries are not installed".into(),
                ));
            }
        };
        host.frame_prepare_optional_element_program(
            &frame.get().to_string(),
            crate::handles::EvaluateHandleParams::expression(source).into(),
        )
        .await
    }

    #[cfg(feature = "element-handle-dom")]
    async fn query_element_handles_in_frame(
        &self,
        frame: ObjectId,
    ) -> Result<Vec<crate::handles::PendingHandle>, HostError> {
        let host = self.host()?;
        let (plan, _query_lease) = self.query.command_plan()?;
        match plan {
            CommandPlan::Selector(selector) => {
                host.frame_query_selector_all(&frame.get().to_string(), &selector)
                    .await
            }
            CommandPlan::Browser(program) => {
                let matches = program.compile("document")?;
                host.frame_query_compiled_all(&frame.get().to_string(), matches)
                    .await
            }
            #[cfg(feature = "selector-registration")]
            CommandPlan::Registered(_) => Err(HostError::UnsupportedOperation(
                "registered selector ElementHandle batches are not installed".into(),
            )),
            #[cfg(feature = "layout-selectors")]
            CommandPlan::Layout(_) => Err(HostError::UnsupportedOperation(
                "layout selector ElementHandle batches are not installed".into(),
            )),
        }
    }

    pub fn nth(&self, index: i32) -> Result<Self, HostError> {
        Ok(self.derive(self.query.nth(index)?))
    }

    pub fn visible(&self, visible: bool) -> Result<Self, HostError> {
        Ok(self.derive(self.query.visible(visible)?))
    }

    pub fn filter_text(&self, text: &str, exclude: bool) -> Result<Self, HostError> {
        Ok(self.derive(self.query.filter_text(text, exclude)?))
    }

    pub fn filter_regex(
        &self,
        source: &str,
        flags: &str,
        exclude: bool,
    ) -> Result<Self, HostError> {
        Ok(self.derive(self.query.filter_regex(source, flags, exclude)?))
    }

    pub fn combine(&self, other: &Self, combination: QueryCombination) -> Result<Self, HostError> {
        #[cfg(feature = "handles")]
        let rebase_root_locator = matches!(combination, QueryCombination::Within)
            && self.frame == other.frame
            && self.content_frame_owner.is_some()
            && other.content_frame_owner.is_none();
        #[cfg(not(feature = "handles"))]
        let rebase_root_locator = false;
        if !Weak::ptr_eq(&self.host, &other.host) {
            return Err(HostError::Backend(
                locator_combination_frame_error(combination).into(),
            ));
        }
        if !self.same_frame_root(other) && !rebase_root_locator {
            #[cfg(feature = "handles")]
            if matches!(combination, QueryCombination::And | QueryCombination::Or)
                && self.content_frame_owner.is_some()
                && other.content_frame_owner.is_some()
            {
                let query = self.query.combine(&other.query, combination)?;
                let mut locator = self.derive(query);
                locator.deferred_frame_combination = Some(Arc::new(DeferredFrameCombination {
                    left: self.clone(),
                    right: other.clone(),
                    combination,
                }));
                return Ok(locator);
            }
            return Err(HostError::Backend(
                locator_combination_frame_error(combination).into(),
            ));
        }
        Ok(self.derive(self.query.combine(&other.query, combination)?))
    }

    /// Compose a newly admitted immutable query within this Locator's existing
    /// host and Frame identity. Language bindings use this instead of encoding
    /// Playwright selector-chain syntax themselves.
    pub fn combine_query(
        &self,
        other: LocatorQuery,
        combination: QueryCombination,
    ) -> Result<Self, HostError> {
        let other = self.derive(other);
        self.combine(&other, combination)
    }

    pub async fn read(
        &self,
        operation: LocatorRead,
        timeout_ms: u32,
        attribute: Option<String>,
    ) -> Result<LocatorValue, HostError> {
        if (operation == LocatorRead::GetAttribute) != attribute.is_some() {
            return Err(HostError::InvalidArgument(
                "only getAttribute requires an attribute name".into(),
            ));
        }
        if attribute
            .as_ref()
            .is_some_and(|name| name.capacity() > 64 * 1024)
        {
            return Err(HostError::InvalidArgument(
                "attribute name exceeds 65536 bytes".into(),
            ));
        }
        #[cfg(feature = "handles")]
        if self.content_frame_owner.is_some() && timeout_ms != 0 {
            let deadline =
                tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
            loop {
                let remaining = remaining_timeout_ms(Some(&deadline), timeout_ms)?;
                match self
                    .read_once(operation, remaining, attribute.clone())
                    .await
                {
                    Err(error) if locator_context_was_replaced(&error) => {
                        if !self.expectation_owner_is_active().await? {
                            return Err(error);
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Err(HostError::Timeout(_)) => return Err(HostError::Timeout(timeout_ms)),
                    outcome => return outcome,
                }
            }
        }
        self.read_once(operation, timeout_ms, attribute).await
    }

    /// Run Playwright's private count test hook after fixing the current
    /// execution-world snapshot but before evaluating the query. Navigation
    /// invalidates that snapshot, and queryCount compatibility reports zero
    /// for the resulting retriable context-loss error.
    pub async fn count_with_before_query<F, Hooked>(&self, hook: F) -> Result<u32, HostError>
    where
        F: FnOnce() -> Hooked + Send + 'static,
        Hooked: std::future::Future<Output = Result<(), HostError>> + Send + 'static,
    {
        let outcome = self
            .evaluate_all_with(move || async move {
                hook().await?;
                Ok(
                    crate::evaluation::EvaluateParams::function_without_argument(
                        "elements => elements.length",
                    ),
                )
            })
            .await;
        let value = match outcome {
            Err(error) if locator_context_was_replaced(&error) => return Ok(0),
            outcome => outcome?,
        };
        let crate::evaluation::EvaluationValue::Number(value) = value else {
            return Err(HostError::Backend(
                "Locator count hook returned a non-number".into(),
            ));
        };
        let value = value.get();
        if !value.is_finite() || value < 0.0 || value.fract() != 0.0 || value > f64::from(u32::MAX)
        {
            return Err(HostError::Backend(
                "Locator count hook returned an invalid number".into(),
            ));
        }
        Ok(value as u32)
    }

    async fn read_once(
        &self,
        operation: LocatorRead,
        timeout_ms: u32,
        attribute: Option<String>,
    ) -> Result<LocatorValue, HostError> {
        #[cfg(feature = "react-selectors")]
        if matches!(operation, LocatorRead::Count) && self.query.contains_react() {
            let value = self
                .evaluate_all_with_timeout(timeout_ms, || async {
                    Ok(
                        crate::evaluation::EvaluateParams::function_without_argument(
                            "elements => elements.length",
                        ),
                    )
                })
                .await?;
            let crate::evaluation::EvaluationValue::Number(number) = value else {
                return Err(HostError::Backend(
                    "React selector count returned a non-number".into(),
                ));
            };
            let number = number.get();
            if !number.is_finite()
                || number < 0.0
                || number.fract() != 0.0
                || number > f64::from(u32::MAX)
            {
                return Err(HostError::Backend(
                    "React selector count returned an invalid number".into(),
                ));
            }
            return Ok(LocatorValue::Count(number as u32));
        }
        let host = self.host()?;
        // This admission covers the command-owned clone, separately from the
        // immutable recipe. Backend execution/cleanup has its own SelectorBytes
        // reservation and the Node boundary separately owns result settlement.
        let (selector, _query_lease) = self.query.command_selector()?;
        #[cfg(feature = "handles")]
        let missing_frame_default =
            self.content_frame_owner
                .as_ref()
                .and_then(|_| match operation {
                    LocatorRead::Count => Some(LocatorValue::Count(0)),
                    LocatorRead::AllInnerTexts | LocatorRead::AllTextContents => {
                        Some(LocatorValue::Texts(Vec::new()))
                    }
                    LocatorRead::IsVisible => Some(LocatorValue::Bool(false)),
                    LocatorRead::IsHidden => Some(LocatorValue::Bool(true)),
                    _ => None,
                });
        #[cfg(not(feature = "handles"))]
        let missing_frame_default: Option<LocatorValue> = None;
        let resolve_timeout = if missing_frame_default.is_some() {
            1
        } else {
            timeout_ms
        };
        let frame = match self.resolve_frame(resolve_timeout).await {
            Ok(frame) => frame.get().to_string(),
            Err(error @ HostError::Timeout(_)) => {
                return match missing_frame_default {
                    Some(default) => Ok(default),
                    None => Err(error),
                };
            }
            Err(error) => return Err(error),
        };
        let outcome = match operation {
            LocatorRead::Count => host
                .invoke::<Count>(&frame, CountParams::new(selector), 0)
                .await
                .map(LocatorValue::Count),
            LocatorRead::AllInnerTexts => host
                .invoke::<AllInnerTexts>(&frame, AllInnerTextsParams::new(selector), 0)
                .await
                .map(LocatorValue::Texts),
            LocatorRead::AllTextContents => host
                .invoke::<AllTextContents>(&frame, AllTextContentsParams::new(selector), 0)
                .await
                .map(LocatorValue::Texts),
            LocatorRead::GetAttribute => host
                .invoke::<GetAttribute>(
                    &frame,
                    GetAttributeParams::new(selector, attribute.expect("validated attribute")),
                    timeout_ms,
                )
                .await
                .map(LocatorValue::Text),
            LocatorRead::InnerHtml => host
                .invoke::<InnerHtml>(&frame, InnerHtmlParams::new(selector), timeout_ms)
                .await
                .map(|value| LocatorValue::Text(Some(value))),
            LocatorRead::InnerText => host
                .invoke::<InnerText>(&frame, InnerTextParams::new(selector), timeout_ms)
                .await
                .map(|value| LocatorValue::Text(Some(value))),
            LocatorRead::InputValue => host
                .invoke::<InputValue>(&frame, InputValueParams::new(selector), timeout_ms)
                .await
                .map(|value| LocatorValue::Text(Some(value))),
            LocatorRead::IsChecked => host
                .invoke::<IsChecked>(&frame, IsCheckedParams::new(selector), timeout_ms)
                .await
                .map(LocatorValue::Bool),
            LocatorRead::IsEditable => host
                .invoke::<IsEditable>(&frame, IsEditableParams::new(selector), timeout_ms)
                .await
                .map(LocatorValue::Bool),
            LocatorRead::IsEnabled | LocatorRead::IsDisabled => host
                .invoke::<IsEnabled>(&frame, IsEnabledParams::new(selector), timeout_ms)
                .await
                .map(|value| {
                    LocatorValue::Bool(if operation == LocatorRead::IsDisabled {
                        !value
                    } else {
                        value
                    })
                }),
            LocatorRead::IsVisible | LocatorRead::IsHidden => host
                .invoke::<IsVisible>(&frame, IsVisibleParams::new(selector), 0)
                .await
                .map(|value| {
                    LocatorValue::Bool(if operation == LocatorRead::IsHidden {
                        !value
                    } else {
                        value
                    })
                }),
            LocatorRead::TextContent => host
                .invoke::<TextContent>(&frame, TextContentParams::new(selector), timeout_ms)
                .await
                .map(LocatorValue::Text),
        };
        let outcome = crate::completion::publish_command_result(
            outcome,
            host.is_connected(),
            host.flush_topology(),
        )
        .await;
        recover_visibility_context_loss(
            operation,
            rewrite_strict_selector_error(&self.query, outcome),
        )
    }

    pub async fn wait_for(
        &self,
        state: LocatorWaitState,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        #[cfg(feature = "handles")]
        if self.content_frame_owner.is_some() {
            let deadline = (timeout_ms != 0).then(|| {
                tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms))
            });
            loop {
                let remaining = remaining_timeout_ms(deadline.as_ref(), timeout_ms)?;
                let missing_succeeds =
                    matches!(state, LocatorWaitState::Detached | LocatorWaitState::Hidden);
                let resolve_timeout = if missing_succeeds { 1 } else { remaining };
                let frame = match self.resolve_frame(resolve_timeout).await {
                    Ok(frame) => frame,
                    Err(HostError::Timeout(_)) if missing_succeeds => return Ok(()),
                    Err(HostError::Timeout(_)) => return Err(HostError::Timeout(timeout_ms)),
                    Err(error) if locator_context_was_replaced(&error) => {
                        if !self.expectation_owner_is_active().await? {
                            return Err(error);
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                match self.wait_for_in_frame(frame, state, remaining).await {
                    Err(error) if locator_context_was_replaced(&error) => {
                        if !self.expectation_owner_is_active().await? {
                            return Err(error);
                        }
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Err(HostError::Timeout(_)) => return Err(HostError::Timeout(timeout_ms)),
                    outcome => return outcome,
                }
            }
        }
        let frame = self.resolve_frame(timeout_ms).await?;
        self.wait_for_in_frame(frame, state, timeout_ms).await
    }

    pub async fn expect_visibility(
        &self,
        timeout_ms: u32,
        visible: bool,
        is_not: bool,
    ) -> Result<LocatorVisibilityExpectation, HostError> {
        let started = tokio::time::Instant::now();
        let timeout = Duration::from_millis(u64::from(timeout_ms));
        let deadline = started + timeout;
        let mut preview = None;
        let mut missing = false;
        let mut preview_attempted = false;
        loop {
            let operation = if visible {
                LocatorRead::IsVisible
            } else {
                LocatorRead::IsHidden
            };
            let received = match self.read(operation, 0, None).await {
                Ok(LocatorValue::Bool(value)) => value,
                Ok(_) => {
                    return Err(HostError::Backend(
                        "visibility expectation received a non-boolean Locator value".into(),
                    ));
                }
                Err(error) if locator_context_was_replaced(&error) => {
                    if self.expectation_owner_is_active().await? {
                        operation == LocatorRead::IsHidden
                    } else {
                        return Err(error);
                    }
                }
                Err(error) => return Err(error),
            };
            if received != is_not {
                return Ok(LocatorVisibilityExpectation {
                    matches: received,
                    received,
                    preview: None,
                    missing: false,
                    timed_out: false,
                });
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if !preview_attempted && timeout_ms != 0 && !remaining.is_zero() {
                let preview_timeout =
                    u32::try_from(remaining.as_millis().clamp(1, 100)).unwrap_or(100);
                (preview, missing) = self.expectation_preview(preview_timeout).await?;
                preview_attempted = true;
            }
            if timeout_ms == 0 || tokio::time::Instant::now() >= deadline {
                return Ok(LocatorVisibilityExpectation {
                    matches: received,
                    received,
                    preview,
                    missing,
                    timed_out: true,
                });
            }
            tokio::time::sleep(
                deadline
                    .saturating_duration_since(tokio::time::Instant::now())
                    .min(Duration::from_millis(10)),
            )
            .await;
        }
    }

    /// Poll the typed collection count while Rust owns the total deadline and
    /// every selector-resolution attempt. Bindings receive only the bounded
    /// public integers and the final structured outcome.
    pub async fn expect_count(
        &self,
        timeout_ms: u32,
        expected: u32,
        is_not: bool,
    ) -> Result<LocatorCountExpectation, HostError> {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        let mut last_received = None;
        loop {
            let outcome = self.read(LocatorRead::Count, 0, None).await;
            let received = match outcome {
                Ok(LocatorValue::Count(value)) => Some(value),
                Ok(_) => {
                    return Err(HostError::Backend(
                        "count expectation received an incompatible Locator value".into(),
                    ));
                }
                Err(error) if locator_context_was_replaced(&error) => {
                    if self.expectation_owner_is_active().await? {
                        last_received
                    } else {
                        return Err(error);
                    }
                }
                Err(error) => return Err(error),
            };
            if received.is_some() {
                last_received = received;
            }
            let matches = received.is_some_and(|value| count_expectation_matches(value, expected));
            if matches != is_not {
                return Ok(LocatorCountExpectation {
                    matches,
                    received,
                    timed_out: false,
                });
            }
            if timeout_ms == 0 || tokio::time::Instant::now() >= deadline {
                return Ok(LocatorCountExpectation {
                    matches,
                    received,
                    timed_out: true,
                });
            }
            tokio::time::sleep(
                deadline
                    .saturating_duration_since(tokio::time::Instant::now())
                    .min(Duration::from_millis(10)),
            )
            .await;
        }
    }

    /// Poll the ordered text contents for a bounded Locator collection. Rust
    /// owns the complete deadline and selector-resolution lifecycle; language
    /// bindings only supply the immutable expected strings.
    pub async fn expect_texts(
        &self,
        timeout_ms: u32,
        expectation: TextsExpectation<'_>,
    ) -> Result<LocatorTextsExpectation, HostError> {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        let mut last_received = Vec::new();
        let operation = if expectation.use_inner_text {
            LocatorRead::AllInnerTexts
        } else {
            LocatorRead::AllTextContents
        };
        loop {
            let received = match self.read(operation, 0, None).await {
                Ok(LocatorValue::Texts(value)) => value,
                Ok(_) => {
                    return Err(HostError::Backend(
                        "text collection expectation received an incompatible Locator value".into(),
                    ));
                }
                Err(error) if locator_context_was_replaced(&error) => {
                    if self.expectation_owner_is_active().await? {
                        last_received.clone()
                    } else {
                        return Err(error);
                    }
                }
                Err(error) => return Err(error),
            };
            last_received.clone_from(&received);
            let matches = texts_expectation_matches(&received, &expectation);
            if matches != expectation.is_not {
                return Ok(LocatorTextsExpectation {
                    matches,
                    received,
                    timed_out: false,
                });
            }
            if timeout_ms == 0 || tokio::time::Instant::now() >= deadline {
                return Ok(LocatorTextsExpectation {
                    matches,
                    received,
                    timed_out: true,
                });
            }
            tokio::time::sleep(
                deadline
                    .saturating_duration_since(tokio::time::Instant::now())
                    .min(Duration::from_millis(10)),
            )
            .await;
        }
    }

    /// Compare one Playwright string surface through a Rust-selected static
    /// browser program. This preserves class-list/SVG behavior and value
    /// follow-label validation instead of treating either surface as an
    /// arbitrary JavaScript property.
    pub async fn expect_string(
        &self,
        timeout_ms: u32,
        kind: LocatorStringExpectationKind,
        matcher: StringMatcher<'_>,
        is_not: bool,
    ) -> Result<LocatorValueExpectation, HostError> {
        if matcher.expected.len() > crate::evaluation::STRING_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "string expectation exceeds {} bytes",
                crate::evaluation::STRING_BYTES
            )));
        }
        if let Some(flags) = matcher.flags {
            if flags.len() > 64 {
                return Err(HostError::InvalidArgument(
                    "string expectation RegExp flags exceed 64 bytes".into(),
                ));
            }
            RegexMatcher::new(matcher.expected.to_owned(), flags.to_owned())
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        }
        let argument = crate::evaluation::EvaluationValue::Object {
            id: None,
            properties: vec![
                crate::evaluation::EvaluationProperty::new(
                    "kind",
                    crate::evaluation::EvaluationValue::String(kind.as_str().to_owned()),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "expected",
                    crate::evaluation::EvaluationValue::String(matcher.expected.to_owned()),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "flags",
                    matcher
                        .flags
                        .map_or(crate::evaluation::EvaluationValue::Undefined, |flags| {
                            crate::evaluation::EvaluationValue::String(flags.to_owned())
                        }),
                ),
            ],
        };
        self.expect_value(timeout_ms, STRING_EXPECTATION_SOURCE, argument, is_not)
            .await
    }

    /// Compare exact class attributes for a bounded Locator collection.
    /// This compatibility entry point delegates to the typed mixed-matcher
    /// implementation used by Playwright's string-or-RegExp arrays.
    pub async fn expect_class_names(
        &self,
        timeout_ms: u32,
        expected: &[String],
        is_not: bool,
    ) -> Result<LocatorTextsExpectation, HostError> {
        let matchers = expected
            .iter()
            .map(|expected| StringMatcher {
                expected,
                flags: None,
            })
            .collect::<Vec<_>>();
        self.expect_class_matchers(timeout_ms, &matchers, is_not)
            .await
    }

    /// Compare a bounded Locator collection with ordered exact/RegExp class
    /// matchers. Rust validates every RegExp before browser access and owns
    /// the immutable selector recipe, retries and total deadline.
    pub async fn expect_class_matchers(
        &self,
        timeout_ms: u32,
        expected: &[StringMatcher<'_>],
        is_not: bool,
    ) -> Result<LocatorTextsExpectation, HostError> {
        let expected_bytes = expected.iter().try_fold(0usize, |total, matcher| {
            let matcher_bytes = matcher
                .expected
                .len()
                .checked_add(matcher.flags.map_or(0, str::len))
                .ok_or_else(|| {
                    HostError::ResourceExhausted(
                        "expected class collection byte count overflowed".into(),
                    )
                })?;
            total.checked_add(matcher_bytes).ok_or_else(|| {
                HostError::ResourceExhausted(
                    "expected class collection byte count overflowed".into(),
                )
            })
        })?;
        if expected.len() > 1024 || expected_bytes > 1024 * 1024 {
            return Err(HostError::ResourceExhausted(
                "expected class collection exceeds 1024 items or 1048576 UTF-8 bytes".into(),
            ));
        }
        for matcher in expected {
            if let Some(flags) = matcher.flags {
                if flags.len() > 64 {
                    return Err(HostError::InvalidArgument(
                        "class expectation RegExp flags exceed 64 bytes".into(),
                    ));
                }
                RegexMatcher::new(matcher.expected.to_owned(), flags.to_owned())
                    .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
            }
        }
        let argument = crate::evaluation::EvaluationValue::Object {
            id: None,
            properties: vec![crate::evaluation::EvaluationProperty::new(
                "expected",
                crate::evaluation::EvaluationValue::Array {
                    id: None,
                    items: expected
                        .iter()
                        .map(|matcher| crate::evaluation::EvaluationValue::Object {
                            id: None,
                            properties: vec![
                                crate::evaluation::EvaluationProperty::new(
                                    "expected",
                                    crate::evaluation::EvaluationValue::String(
                                        matcher.expected.to_owned(),
                                    ),
                                ),
                                crate::evaluation::EvaluationProperty::new(
                                    "flags",
                                    matcher.flags.map_or(
                                        crate::evaluation::EvaluationValue::Undefined,
                                        |flags| {
                                            crate::evaluation::EvaluationValue::String(
                                                flags.to_owned(),
                                            )
                                        },
                                    ),
                                ),
                            ],
                        })
                        .collect(),
                },
            )],
        };
        let deadline = tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        let mut last_received = Vec::new();
        let mut first_attempt = true;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let attempt_ms = if first_attempt {
                100
            } else if timeout_ms == 0 {
                1
            } else {
                u32::try_from(remaining.as_millis().clamp(1, 100)).unwrap_or(100)
            };
            first_attempt = false;
            let attempt_argument = argument.clone();
            let outcome = self
                .evaluate_all_with_timeout(attempt_ms, move || async move {
                    Ok(crate::evaluation::EvaluateParams::function(
                        CLASS_NAMES_EXPECTATION_SOURCE,
                        attempt_argument,
                    ))
                })
                .await;
            let (matches, received) = match outcome {
                Ok(value) => texts_value_expectation_result(value)?,
                Err(HostError::Timeout(_)) => (is_not, last_received.clone()),
                Err(error) if locator_context_was_replaced(&error) => {
                    if self.expectation_owner_is_active().await? {
                        (is_not, last_received.clone())
                    } else {
                        return Err(error);
                    }
                }
                Err(error) => return Err(error),
            };
            last_received.clone_from(&received);
            if matches != is_not {
                return Ok(LocatorTextsExpectation {
                    matches,
                    received,
                    timed_out: false,
                });
            }
            if timeout_ms == 0 || tokio::time::Instant::now() >= deadline {
                return Ok(LocatorTextsExpectation {
                    matches,
                    received,
                    timed_out: true,
                });
            }
            tokio::time::sleep(
                deadline
                    .saturating_duration_since(tokio::time::Instant::now())
                    .min(Duration::from_millis(10)),
            )
            .await;
        }
    }

    /// Poll one immutable Locator recipe through the existing Rust evaluation
    /// owner. Each attempt finishes selection, evaluation and native cleanup
    /// before the next starts; no browser Promise or persistent matcher table
    /// survives an attempt.
    pub async fn expect_state(
        &self,
        timeout_ms: u32,
        state: LocatorExpectationState,
        is_not: bool,
    ) -> Result<LocatorExpectation, HostError> {
        let started = tokio::time::Instant::now();
        let deadline = started + Duration::from_millis(u64::from(timeout_ms));
        let mut last_received = None;
        let mut preview = None;
        let mut missing = false;
        let mut preview_attempted = false;
        let mut first_attempt = true;
        #[cfg(feature = "handles")]
        let has_content_frame_owner = self.content_frame_owner.is_some();
        #[cfg(not(feature = "handles"))]
        let has_content_frame_owner = false;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let attempt_ms = state_expectation_attempt_timeout(
                first_attempt,
                has_content_frame_owner,
                timeout_ms,
                remaining,
            );
            first_attempt = false;
            let outcome = if let Some(operation) = state.read_operation() {
                self.read(operation, attempt_ms, None)
                    .await
                    .and_then(|value| {
                        Ok(crate::evaluation::EvaluationValue::String(
                            state.received_from_read(value)?.into(),
                        ))
                    })
            } else {
                let source = state
                    .source()
                    .ok_or_else(|| HostError::Backend("state expectation omitted source".into()))?;
                self.evaluate_with(attempt_ms, move || async move {
                    Ok(crate::evaluation::EvaluateParams::function_without_argument(source))
                })
                .await
            };
            let received = match outcome {
                Ok(crate::evaluation::EvaluationValue::String(value)) if value == "retry" => None,
                Ok(crate::evaluation::EvaluationValue::String(value)) => Some(value),
                Ok(_) => {
                    return Err(HostError::Backend(
                        "state expectation received a non-string value".into(),
                    ));
                }
                Err(HostError::Timeout(_)) => None,
                Err(error) if locator_context_was_replaced(&error) => {
                    if self.expectation_owner_is_active().await? {
                        None
                    } else {
                        return Err(error);
                    }
                }
                Err(error) => return Err(error),
            };
            if received.is_some() {
                last_received.clone_from(&received);
            }
            let matches = state.matches(received.as_deref(), is_not)?;
            if matches != is_not {
                return Ok(LocatorExpectation {
                    matches,
                    received,
                    preview: None,
                    missing: false,
                    timed_out: false,
                });
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if !preview_attempted && received.is_some() && timeout_ms != 0 && !remaining.is_zero() {
                let preview_timeout =
                    u32::try_from(remaining.as_millis().clamp(1, 100)).unwrap_or(100);
                (preview, missing) = self.expectation_preview(preview_timeout).await?;
                preview_attempted = true;
            }
            if timeout_ms == 0 || tokio::time::Instant::now() >= deadline {
                let missing = missing
                    || matches!(state, LocatorExpectationState::Attached)
                        && received.as_deref() == Some("detached")
                    || received.is_none() && last_received.is_none();
                return Ok(LocatorExpectation {
                    matches,
                    received: received.or(last_received),
                    preview,
                    missing,
                    timed_out: true,
                });
            }
            tokio::time::sleep(
                deadline
                    .saturating_duration_since(tokio::time::Instant::now())
                    .min(Duration::from_millis(10)),
            )
            .await;
        }
    }

    /// Resolve a dotted property and compare one bounded serialized value in
    /// the browser using a Rust-selected static program. Rust owns retries,
    /// deadline and missing-element semantics; Node only converts language
    /// values before and after this operation.
    pub async fn expect_property(
        &self,
        timeout_ms: u32,
        name: &str,
        expected: crate::evaluation::EvaluationValue,
        is_not: bool,
    ) -> Result<LocatorValueExpectation, HostError> {
        if name.len() > crate::evaluation::STRING_BYTES {
            return Err(HostError::InvalidArgument(format!(
                "property name exceeds {} bytes",
                crate::evaluation::STRING_BYTES
            )));
        }
        let argument = crate::evaluation::EvaluationValue::Object {
            id: None,
            properties: vec![
                crate::evaluation::EvaluationProperty::new(
                    "name",
                    crate::evaluation::EvaluationValue::String(name.to_owned()),
                ),
                crate::evaluation::EvaluationProperty::new("expected", expected),
            ],
        };
        self.expect_value(timeout_ms, PROPERTY_EXPECTATION_SOURCE, argument, is_not)
            .await
    }

    /// Compare one DOM attribute through a Rust-selected browser program.
    /// The caller supplies either an exact string or a validated JavaScript
    /// RegExp source/flag pair while Rust owns retries and the total deadline.
    pub async fn expect_attribute(
        &self,
        timeout_ms: u32,
        expectation: AttributeExpectation<'_>,
    ) -> Result<LocatorValueExpectation, HostError> {
        if expectation.name.len() > crate::evaluation::STRING_BYTES
            || expectation.expected.len() > crate::evaluation::STRING_BYTES
        {
            return Err(HostError::InvalidArgument(format!(
                "attribute expectation name or value exceeds {} bytes",
                crate::evaluation::STRING_BYTES
            )));
        }
        if let Some(flags) = expectation.flags {
            RegexMatcher::new(expectation.expected.to_owned(), flags.to_owned())
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        }
        let argument = crate::evaluation::EvaluationValue::Object {
            id: None,
            properties: vec![
                crate::evaluation::EvaluationProperty::new(
                    "name",
                    crate::evaluation::EvaluationValue::String(expectation.name.to_owned()),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "expected",
                    crate::evaluation::EvaluationValue::String(expectation.expected.to_owned()),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "flags",
                    expectation
                        .flags
                        .map_or(crate::evaluation::EvaluationValue::Undefined, |flags| {
                            crate::evaluation::EvaluationValue::String(flags.to_owned())
                        }),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "ignoreCase",
                    expectation.ignore_case.map_or(
                        crate::evaluation::EvaluationValue::Undefined,
                        crate::evaluation::EvaluationValue::Bool,
                    ),
                ),
            ],
        };
        self.expect_value(
            timeout_ms,
            ATTRIBUTE_EXPECTATION_SOURCE,
            argument,
            expectation.is_not,
        )
        .await
    }

    /// Compare one computed CSS property through a Rust-selected browser
    /// program. The binding supplies only bounded strings while Rust owns
    /// regular-expression validation, retries and the total deadline.
    pub async fn expect_css(
        &self,
        timeout_ms: u32,
        expectation: CssExpectation<'_>,
    ) -> Result<LocatorValueExpectation, HostError> {
        if expectation.name.len() > crate::evaluation::STRING_BYTES
            || expectation.expected.len() > crate::evaluation::STRING_BYTES
        {
            return Err(HostError::InvalidArgument(format!(
                "CSS expectation name or value exceeds {} bytes",
                crate::evaluation::STRING_BYTES
            )));
        }
        if let Some(flags) = expectation.flags {
            RegexMatcher::new(expectation.expected.to_owned(), flags.to_owned())
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?;
        }
        let argument = crate::evaluation::EvaluationValue::Object {
            id: None,
            properties: vec![
                crate::evaluation::EvaluationProperty::new(
                    "name",
                    crate::evaluation::EvaluationValue::String(expectation.name.to_owned()),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "expected",
                    crate::evaluation::EvaluationValue::String(expectation.expected.to_owned()),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "flags",
                    expectation
                        .flags
                        .map_or(crate::evaluation::EvaluationValue::Undefined, |flags| {
                            crate::evaluation::EvaluationValue::String(flags.to_owned())
                        }),
                ),
            ],
        };
        self.expect_value(
            timeout_ms,
            CSS_EXPECTATION_SOURCE,
            argument,
            expectation.is_not,
        )
        .await
    }

    async fn expect_value(
        &self,
        timeout_ms: u32,
        source: &'static str,
        argument: crate::evaluation::EvaluationValue,
        is_not: bool,
    ) -> Result<LocatorValueExpectation, HostError> {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let attempt_ms = if timeout_ms == 0 {
                1
            } else {
                u32::try_from(remaining.as_millis().clamp(1, 100)).unwrap_or(100)
            };
            let attempt_argument = argument.clone();
            let outcome = self
                .evaluate_with(attempt_ms, move || async move {
                    Ok(crate::evaluation::EvaluateParams::function(
                        source,
                        attempt_argument,
                    ))
                })
                .await;
            let (matches, received, missing) = match outcome {
                Ok(value) => {
                    let (matches, received) = value_expectation_result(value)?;
                    (matches, Some(received), false)
                }
                Err(HostError::Timeout(_)) => (is_not, None, true),
                Err(error) if locator_context_was_replaced(&error) => {
                    if self.expectation_owner_is_active().await? {
                        (is_not, None, true)
                    } else {
                        return Err(error);
                    }
                }
                Err(error) => return Err(error),
            };
            if matches != is_not {
                return Ok(LocatorValueExpectation {
                    matches,
                    received,
                    missing,
                    timed_out: false,
                });
            }
            if timeout_ms == 0 || tokio::time::Instant::now() >= deadline {
                return Ok(LocatorValueExpectation {
                    matches,
                    received,
                    missing,
                    timed_out: true,
                });
            }
            tokio::time::sleep(
                deadline
                    .saturating_duration_since(tokio::time::Instant::now())
                    .min(Duration::from_millis(10)),
            )
            .await;
        }
    }

    /// Evaluate JavaScript RegExp semantics in the browser while Rust owns the
    /// immutable Locator recipe, total deadline and serial retry lifecycle.
    pub async fn expect_text_regex(
        &self,
        timeout_ms: u32,
        source: &str,
        flags: &str,
        ignore_case: Option<bool>,
        use_inner_text: bool,
        is_not: bool,
    ) -> Result<LocatorExpectation, HostError> {
        if source.len() > crate::evaluation::STRING_BYTES || flags.len() > 64 {
            return Err(HostError::InvalidArgument(
                "text RegExp exceeds its bounded source or flags allowance".into(),
            ));
        }
        let argument = crate::evaluation::EvaluationValue::Object {
            id: None,
            properties: vec![
                crate::evaluation::EvaluationProperty::new(
                    "source",
                    crate::evaluation::EvaluationValue::String(source.to_owned()),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "flags",
                    crate::evaluation::EvaluationValue::String(flags.to_owned()),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "ignoreCase",
                    ignore_case.map_or(crate::evaluation::EvaluationValue::Undefined, |value| {
                        crate::evaluation::EvaluationValue::Bool(value)
                    }),
                ),
                crate::evaluation::EvaluationProperty::new(
                    "useInnerText",
                    crate::evaluation::EvaluationValue::Bool(use_inner_text),
                ),
            ],
        };
        let deadline = tokio::time::Instant::now() + Duration::from_millis(u64::from(timeout_ms));
        let mut preview = None;
        let mut preview_attempted = false;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let attempt_ms = if timeout_ms == 0 {
                1
            } else {
                u32::try_from(remaining.as_millis().clamp(1, 100)).unwrap_or(100)
            };
            let attempt_argument = argument.clone();
            let outcome = self
                .evaluate_with(attempt_ms, move || async move {
                    Ok(crate::evaluation::EvaluateParams::function(
                        TEXT_REGEX_EXPECTATION_SOURCE,
                        attempt_argument,
                    ))
                })
                .await;
            let (matches, received, missing) = match outcome {
                Ok(value) => {
                    let (matches, received) = value_expectation_result(value)?;
                    let crate::evaluation::EvaluationValue::String(received) = received else {
                        return Err(HostError::Backend(
                            "text RegExp expectation received a non-string value".into(),
                        ));
                    };
                    (matches, Some(received), false)
                }
                Err(HostError::Timeout(_)) => (is_not, None, true),
                Err(error) if locator_context_was_replaced(&error) => {
                    if self.expectation_owner_is_active().await? {
                        (is_not, None, true)
                    } else {
                        return Err(error);
                    }
                }
                Err(error) => return Err(error),
            };
            if matches != is_not {
                return Ok(LocatorExpectation {
                    matches,
                    received,
                    preview: None,
                    missing: false,
                    timed_out: false,
                });
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if !preview_attempted && received.is_some() && timeout_ms != 0 && !remaining.is_zero() {
                let preview_timeout =
                    u32::try_from(remaining.as_millis().clamp(1, 100)).unwrap_or(100);
                (preview, _) = self.expectation_preview(preview_timeout).await?;
                preview_attempted = true;
            }
            if timeout_ms == 0 || tokio::time::Instant::now() >= deadline {
                return Ok(LocatorExpectation {
                    matches,
                    received,
                    preview,
                    missing,
                    timed_out: true,
                });
            }
            tokio::time::sleep(
                deadline
                    .saturating_duration_since(tokio::time::Instant::now())
                    .min(Duration::from_millis(10)),
            )
            .await;
        }
    }

    /// Poll one immutable Locator recipe through the existing Rust command
    /// path. The operation owns no browser-side promise or persistent matcher
    /// table; each bounded read completes before the next attempt starts.
    pub async fn expect_text(
        &self,
        timeout_ms: u32,
        expectation: TextExpectation<'_>,
    ) -> Result<LocatorExpectation, HostError> {
        let started = tokio::time::Instant::now();
        let timeout = Duration::from_millis(u64::from(timeout_ms));
        let deadline = started + timeout;
        let operation = if expectation.use_inner_text {
            LocatorRead::InnerText
        } else {
            LocatorRead::TextContent
        };
        let mut preview = None;
        let mut preview_attempted = false;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            let attempt_ms = if timeout_ms == 0 {
                1
            } else {
                u32::try_from(remaining.as_millis().clamp(1, 100)).unwrap_or(100)
            };
            let (received, missing) = match self.read(operation, attempt_ms, None).await {
                Ok(LocatorValue::Text(value)) => (value, false),
                Ok(_) => {
                    return Err(HostError::Backend(
                        "text expectation received a non-text Locator value".into(),
                    ));
                }
                Err(HostError::Timeout(_)) => (None, true),
                Err(error) if locator_context_was_replaced(&error) => {
                    if self.expectation_owner_is_active().await? {
                        (None, true)
                    } else {
                        return Err(error);
                    }
                }
                Err(error) => return Err(error),
            };
            let matches = text_expectation_matches(received.as_deref(), missing, &expectation);
            if matches != expectation.is_not {
                return Ok(LocatorExpectation {
                    matches,
                    received,
                    preview: None,
                    missing: false,
                    timed_out: false,
                });
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if !preview_attempted && received.is_some() && timeout_ms != 0 && !remaining.is_zero() {
                let preview_timeout =
                    u32::try_from(remaining.as_millis().clamp(1, 100)).unwrap_or(100);
                (preview, _) = self.expectation_preview(preview_timeout).await?;
                preview_attempted = true;
            }
            if timeout_ms == 0 || tokio::time::Instant::now() >= deadline {
                return Ok(LocatorExpectation {
                    matches,
                    received,
                    preview,
                    missing,
                    timed_out: true,
                });
            }
            tokio::time::sleep(
                deadline
                    .saturating_duration_since(tokio::time::Instant::now())
                    .min(Duration::from_millis(10)),
            )
            .await;
        }
    }
}

#[cfg(feature = "handles")]
impl HostFrameLocator {
    pub fn owner(&self) -> Result<HostLocator, HostError> {
        self.owner.copy()
    }

    pub fn locator(&self, query: LocatorQuery) -> Result<HostLocator, HostError> {
        query.command_selector()?;
        Ok(HostLocator {
            host: self.owner.host.clone(),
            frame: self.owner.frame,
            content_frame_owner: Some(Arc::clone(&self.owner)),
            deferred_frame_combination: None,
            query,
        })
    }

    pub fn locator_from(&self, locator: &HostLocator) -> Result<HostLocator, HostError> {
        if !Weak::ptr_eq(&self.owner.host, &locator.host) {
            return Err(HostError::Backend(
                "FrameLocator and Locator must belong to the same browser session".into(),
            ));
        }
        self.locator(locator.query.copy()?)
    }

    pub fn frame_locator(&self, query: LocatorQuery) -> Result<Self, HostError> {
        self.locator(query)?.content_frame()
    }

    pub fn nth(&self, index: i32) -> Result<Self, HostError> {
        self.owner.nth(index)?.content_frame()
    }
}

#[cfg(feature = "handles")]
fn element_handle_expression(
    selector: &pwright_cap_locator::Selector,
) -> Result<String, HostError> {
    let matches = compile_selector(selector, "document").map_err(HostError::backend)?;
    element_handle_expression_from_matches(matches)
}

#[cfg(feature = "handles")]
fn element_handle_expression_from_matches(matches: String) -> Result<String, HostError> {
    let install = selector_runtime_install_script();
    let source = format!(
        r#"(() => {{
  {install};
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error("Pwright selector runtime is unavailable");
  const __pwMatches = {matches};
  if (__pwMatches.length === 0)
    throw new Error("failed to find element matching selector");
  if (__pwMatches.length !== 1)
    throw new Error(`strict mode violation: locator resolved to ${{__pwMatches.length}} elements`);
  return __pwMatches[0];
}})()"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    );
    if source.len() > crate::evaluation::SOURCE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "Locator ElementHandle source exceeds {} UTF-8 bytes",
            crate::evaluation::SOURCE_BYTES
        )));
    }
    Ok(source)
}

#[cfg(feature = "handles")]
fn polling_element_handle_expression_from_matches(matches: String) -> Result<String, HostError> {
    let install = selector_runtime_install_script();
    let source = format!(
        r#"(() => {{
  {install};
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error("Pwright selector runtime is unavailable");
  const __pwMatches = {matches};
  if (__pwMatches.length === 0)
    return null;
  if (__pwMatches.length !== 1)
    throw new Error(`strict mode violation: locator resolved to ${{__pwMatches.length}} elements`);
  return __pwMatches[0];
}})()"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    );
    if source.len() > crate::evaluation::SOURCE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "Locator polling ElementHandle source exceeds {} UTF-8 bytes",
            crate::evaluation::SOURCE_BYTES
        )));
    }
    Ok(source)
}

#[cfg(feature = "handles")]
fn selector_wait_source(
    matches: String,
    state: LocatorWaitState,
    strict: bool,
) -> Result<String, HostError> {
    let install = selector_runtime_install_script();
    let strict_check = if strict {
        "if (__pwMatches.length > 1) throw new Error(`strict mode violation: locator resolved to ${__pwMatches.length} elements`);"
    } else {
        ""
    };
    let result = match state {
        LocatorWaitState::Attached => "return element || null;",
        LocatorWaitState::Visible => "return element && __pw.visible(element) ? element : null;",
        LocatorWaitState::Detached => "return !element;",
        LocatorWaitState::Hidden => "return !element || !__pw.visible(element);",
    };
    let source = format!(
        r#"(() => {{
  {install};
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error("Pwright selector runtime is unavailable");
  const __pwMatches = {matches};
  {strict_check}
  const element = __pwMatches[0];
  {result}
}})()"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    );
    if source.len() > crate::evaluation::SOURCE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "selector wait source exceeds {} UTF-8 bytes",
            crate::evaluation::SOURCE_BYTES
        )));
    }
    Ok(source)
}

#[cfg(feature = "handles")]
fn optional_element_handle_expression(
    selector: &pwright_cap_locator::Selector,
) -> Result<String, HostError> {
    let matches = compile_selector(selector, "document").map_err(HostError::backend)?;
    optional_element_handle_expression_from_matches(matches)
}

#[cfg(feature = "handles")]
fn optional_element_handle_expression_from_matches(matches: String) -> Result<String, HostError> {
    let install = selector_runtime_install_script();
    let source = format!(
        r#"(() => {{
  {install};
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error("Pwright selector runtime is unavailable");
  const __pwMatches = {matches};
  return __pwMatches.length === 0 ? null : __pwMatches[0];
}})()"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    );
    if source.len() > crate::evaluation::SOURCE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "Locator optional ElementHandle source exceeds {} UTF-8 bytes",
            crate::evaluation::SOURCE_BYTES
        )));
    }
    Ok(source)
}

fn locator_combination_frame_error(combination: QueryCombination) -> &'static str {
    match combination {
        QueryCombination::Has => "Inner \"has\" locator must belong to the same frame.",
        QueryCombination::HasNot => "Inner \"has_not\" locator must belong to the same frame.",
        QueryCombination::And | QueryCombination::Or | QueryCombination::Within => {
            "Locators must belong to the same frame."
        }
    }
}

#[cfg(feature = "handles")]
fn same_frame_owner(left: &HostLocator, right: &HostLocator) -> bool {
    left.frame == right.frame
        && left.query.same_recipe(&right.query)
        && match (&left.content_frame_owner, &right.content_frame_owner) {
            (None, None) => true,
            (Some(left), Some(right)) => same_frame_owner(left, right),
            _ => false,
        }
}

fn locator_context_was_replaced(error: &HostError) -> bool {
    match error {
        HostError::TargetClosed => true,
        HostError::Backend(message) => {
            message.contains("uniqueContextId not found")
                || message.contains("Execution context was destroyed")
                || message.contains("Cannot find context with specified id")
        }
        _ => false,
    }
}

#[cfg(feature = "handles")]
fn selector_wait_context_was_replaced(error: &HostError) -> bool {
    locator_context_was_replaced(error)
        || matches!(error, HostError::Backend(message) if message.contains("CDP error -32000: Inspected target navigated or closed"))
}

#[cfg(feature = "handles")]
fn selector_wait_retry_delay(
    error: HostError,
    same_frame_active: bool,
    deadline: Option<&tokio::time::Instant>,
    timeout_ms: u32,
) -> Result<Duration, HostError> {
    if !same_frame_active {
        return Err(error);
    }
    let delay = deadline.map_or(Duration::from_millis(10), |deadline| {
        deadline
            .saturating_duration_since(tokio::time::Instant::now())
            .min(Duration::from_millis(10))
    });
    if delay.is_zero() {
        return Err(HostError::Timeout(timeout_ms));
    }
    Ok(delay)
}

#[cfg(test)]
mod expectation_tests {
    #[cfg(feature = "handles")]
    use std::sync::{Arc, Weak};
    use std::time::Duration;

    use pwright_cap_locator::WaitForState;
    #[cfg(feature = "handles")]
    use pwright_core::ObjectId;

    use super::{
        ATTRIBUTE_EXPECTATION_SOURCE, CLASS_NAMES_EXPECTATION_SOURCE, CSS_EXPECTATION_SOURCE,
        HostError, LocatorExpectationState, LocatorRead, LocatorValue, LocatorWaitState,
        STRING_EXPECTATION_SOURCE, TextExpectation, TextsExpectation, count_expectation_matches,
        locator_combination_frame_error, locator_context_was_replaced,
        recover_visibility_context_loss, state_expectation_attempt_timeout,
        text_expectation_matches, text_matches, texts_expectation_matches,
        texts_value_expectation_result, value_expectation_result,
    };
    #[cfg(feature = "handles")]
    use super::{
        AttributeExpectation, CssExpectation, HostLocator, LocatorStringExpectationKind,
        StringMatcher, element_handle_expression, non_frame_owner_preview,
        optional_element_handle_expression, polling_element_handle_expression_from_matches,
        remaining_timeout_ms, rewrite_frame_owner_error, rewrite_strict_selector_error,
        selector_wait_context_was_replaced, selector_wait_error, selector_wait_retry_delay,
        selector_wait_source,
    };
    #[cfg(feature = "locator-dom-actions")]
    use super::{DISPATCH_EVENT_SOURCE, dispatch_event_argument};
    #[cfg(feature = "locator-bounding-box")]
    use super::{LocatorBoundingBox, bounding_box_result};
    use crate::evaluation::{EvaluationProperty, EvaluationValue};
    use crate::locator_query::QueryCombination;
    #[cfg(feature = "handles")]
    use crate::locator_query::{QueryBudget, QueryLimits};

    #[cfg(feature = "handles")]
    fn locator_fixture(selector: &str) -> (HostLocator, QueryBudget) {
        let budget = QueryBudget::new(QueryLimits {
            recipes: 16,
            bytes: 64 * 1024,
        })
        .unwrap();
        let query = budget
            .prepare_leaf(selector.len())
            .unwrap()
            .css(selector.to_owned())
            .unwrap();
        (
            HostLocator {
                host: Weak::new(),
                frame: ObjectId::new(7),
                content_frame_owner: None,
                deferred_frame_combination: None,
                query,
            },
            budget,
        )
    }

    #[cfg(feature = "locator-dom-actions")]
    #[test]
    fn dispatch_event_program_materializes_plain_touch_records() {
        // Arrange
        let source = DISPATCH_EVENT_SOURCE;

        // Act
        let defaults_target = source.contains("init.target ??= element");
        let creates_touches = source.contains("new TouchConstructor({");
        let converts_all_lists = ["init.touches", "init.targetTouches", "init.changedTouches"]
            .into_iter()
            .all(|field| source.contains(&format!("{field} = createTouches({field})")));

        // Assert
        assert!(defaults_target);
        assert!(creates_touches);
        assert!(converts_all_lists);
    }

    #[cfg(feature = "locator-dom-actions")]
    #[test]
    fn dispatch_event_program_uses_device_orientation_constructor_for_both_event_names() {
        // Arrange
        let source = DISPATCH_EVENT_SOURCE;

        // Act
        let maps_relative = source.contains("deviceorientation: 'DeviceOrientationEvent'");
        let maps_absolute = source.contains("deviceorientationabsolute: 'DeviceOrientationEvent'");
        let constructs_event = source.contains("new DeviceOrientationEvent(argument.type, init)");
        let preserves_fields =
            source.contains("{ bubbles, cancelable, alpha, beta, gamma, absolute }");
        let has_legacy_fallback = source.contains(
            "event.initDeviceOrientationEvent(argument.type, bubbles, cancelable, alpha, beta, gamma, absolute)",
        );

        // Assert
        assert!(maps_relative);
        assert!(maps_absolute);
        assert!(constructs_event);
        assert!(preserves_fields);
        assert!(has_legacy_fallback);
    }

    #[cfg(feature = "locator-dom-actions")]
    #[test]
    fn dispatch_event_program_uses_device_motion_constructor_with_nested_fields() {
        // Arrange
        let source = DISPATCH_EVENT_SOURCE;

        // Act
        let maps_motion = source.contains("devicemotion: 'DeviceMotionEvent'");
        let constructs_event = source.contains("new DeviceMotionEvent(argument.type, init)");
        let preserves_fields = source.contains(
            "{ bubbles, cancelable, acceleration, accelerationIncludingGravity, rotationRate, interval }",
        );
        let has_legacy_fallback = source
            .contains("event.initDeviceMotionEvent(argument.type, bubbles, cancelable,")
            && source
                .contains("acceleration, accelerationIncludingGravity, rotationRate, interval);");

        // Assert
        assert!(maps_motion);
        assert!(constructs_event);
        assert!(preserves_fields);
        assert!(has_legacy_fallback);
    }

    #[cfg(feature = "locator-dom-actions")]
    #[test]
    fn dispatch_event_argument_preserves_the_bounded_initializer_tree() {
        // Arrange
        let event_init = EvaluationValue::Object {
            id: None,
            properties: vec![EvaluationProperty::new(
                "detail",
                EvaluationValue::number(2.0).unwrap(),
            )],
        };

        // Act
        let argument = dispatch_event_argument("custom".into(), event_init.clone());

        // Assert
        assert_eq!(
            argument.unwrap(),
            EvaluationValue::Object {
                id: None,
                properties: vec![
                    EvaluationProperty::new("type", EvaluationValue::String("custom".into())),
                    EvaluationProperty::new("eventInit", event_init),
                ],
            }
        );
    }

    #[cfg(feature = "locator-dom-actions")]
    #[test]
    fn dispatch_event_argument_preserves_nested_device_motion_values() {
        // Arrange
        let acceleration = EvaluationValue::Object {
            id: None,
            properties: vec![EvaluationProperty::new(
                "x",
                EvaluationValue::number(10.0).unwrap(),
            )],
        };
        let event_init = EvaluationValue::Object {
            id: None,
            properties: vec![
                EvaluationProperty::new("acceleration", acceleration),
                EvaluationProperty::new("interval", EvaluationValue::number(16.0).unwrap()),
            ],
        };

        // Act
        let argument = dispatch_event_argument("devicemotion".into(), event_init.clone());

        // Assert
        assert_eq!(
            argument.unwrap(),
            EvaluationValue::Object {
                id: None,
                properties: vec![
                    EvaluationProperty::new("type", EvaluationValue::String("devicemotion".into())),
                    EvaluationProperty::new("eventInit", event_init),
                ],
            }
        );
    }

    #[cfg(feature = "locator-dom-actions")]
    #[test]
    fn dispatch_event_argument_rejects_a_non_object_initializer() {
        // Arrange
        let event_init = EvaluationValue::String("not an object".into());

        // Act
        let result = dispatch_event_argument("click".into(), event_init);

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[cfg(feature = "locator-dom-actions")]
    #[test]
    fn dispatch_event_argument_rejects_an_empty_type() {
        // Arrange
        let event_init = EvaluationValue::Null;

        // Act
        let result = dispatch_event_argument(String::new(), event_init);

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn frame_locator_re_roots_child_query_under_one_bounded_owner_recipe() {
        // Arrange
        let (owner, budget) = locator_fixture("iframe");
        let frame_locator = owner.content_frame().unwrap();
        let child_query = budget
            .prepare_leaf("button".len())
            .unwrap()
            .css("button".to_owned())
            .unwrap();

        // Act
        let child = frame_locator.locator(child_query).unwrap();

        // Assert
        assert_eq!(child.frame, ObjectId::new(7));
        assert!(
            child
                .content_frame_owner
                .as_ref()
                .is_some_and(|value| Arc::ptr_eq(value, &frame_locator.owner))
        );
        assert_eq!(
            child.query.command_selector().unwrap().0.to_string(),
            "button"
        );
    }

    #[cfg(feature = "handles")]
    #[test]
    fn frame_locator_nth_derives_the_owner_before_entering_the_frame() {
        // Arrange
        let (owner, _budget) = locator_fixture("iframe");
        let frame_locator = owner.content_frame().unwrap();

        // Act
        let second = frame_locator.nth(1).unwrap();
        let owner = second.owner().unwrap();

        // Assert
        assert_eq!(
            owner.query.command_selector().unwrap().0.to_string(),
            "iframe.nth(1)"
        );
        assert!(owner.content_frame_owner.is_none());
    }

    #[cfg(feature = "handles")]
    #[test]
    fn equivalent_lazy_frame_recipes_can_be_composed() {
        // Arrange
        let (first_owner, budget) = locator_fixture("iframe");
        let second_owner = first_owner.derive(
            budget
                .prepare_leaf("iframe".len())
                .unwrap()
                .css("iframe".to_owned())
                .unwrap(),
        );
        let child_query = || {
            budget
                .prepare_leaf("span".len())
                .unwrap()
                .css("span".to_owned())
                .unwrap()
        };
        let left = first_owner
            .content_frame()
            .unwrap()
            .locator(child_query())
            .unwrap();
        let right = second_owner
            .content_frame()
            .unwrap()
            .locator(child_query())
            .unwrap();

        // Act
        let combined = left.combine(&right, QueryCombination::Or);

        // Assert
        assert!(combined.is_ok());
    }

    #[cfg(feature = "handles")]
    #[test]
    fn different_lazy_frame_recipes_report_composite_locator_context() {
        // Arrange
        let (iframe, budget) = locator_fixture("iframe");
        let different_iframe = iframe.derive(
            budget
                .prepare_leaf("#iframe".len())
                .unwrap()
                .css("#iframe".to_owned())
                .unwrap(),
        );
        let div = iframe
            .content_frame()
            .unwrap()
            .locator(
                budget
                    .prepare_leaf("div".len())
                    .unwrap()
                    .css("div".to_owned())
                    .unwrap(),
            )
            .unwrap();
        let span = different_iframe
            .content_frame()
            .unwrap()
            .locator(
                budget
                    .prepare_leaf("span".len())
                    .unwrap()
                    .css("span".to_owned())
                    .unwrap(),
            )
            .unwrap();

        // Act
        let combined = div.combine(&span, QueryCombination::And).unwrap();
        let error = match combined.host() {
            Ok(_) => panic!("different iframe owner recipes must fail when used"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            HostError::Backend(message)
                if message.contains(
                    "Frame locators are not allowed inside composite locators, while querying \"locator('iframe').contentFrame().locator('div').and(locator('#iframe').contentFrame().locator('span'))\""
                )
        ));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn locator_within_dynamic_frame_rebases_a_root_locator_recipe() {
        // Arrange
        let (owner, budget) = locator_fixture("iframe");
        let frame_locator = owner.content_frame().unwrap();
        let div = frame_locator
            .locator(
                budget
                    .prepare_leaf("div".len())
                    .unwrap()
                    .css("div".to_owned())
                    .unwrap(),
            )
            .unwrap();
        let root_input = owner.derive(
            budget
                .prepare_leaf("input".len())
                .unwrap()
                .css("input".to_owned())
                .unwrap(),
        );

        // Act
        let nested = div.combine(&root_input, QueryCombination::Within).unwrap();

        // Assert
        assert_eq!(
            nested.query.command_selector().unwrap().0.to_string(),
            "(div).within(input)"
        );
        assert!(nested.content_frame_owner.is_some());
    }

    #[cfg(feature = "handles")]
    #[test]
    fn strict_selector_errors_use_the_playwright_locator_call_log() {
        // Arrange
        let (body, budget) = locator_fixture("body");
        let iframe = budget
            .prepare_leaf("iframe".len())
            .unwrap()
            .css("iframe".to_owned())
            .unwrap();
        let nested = body
            .combine_query(iframe, QueryCombination::Within)
            .unwrap();
        let backend = Err::<(), _>(HostError::Backend(
            "Backend: strict selector `(body).within(iframe)` resolved to 3 elements".into(),
        ));

        // Act
        let error = rewrite_strict_selector_error(&nested.query, backend).unwrap_err();

        // Assert
        assert!(matches!(
            error,
            HostError::Backend(message)
                if message == "strict mode violation: locator('body').locator('iframe') resolved to 3 elements"
        ));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn frame_owner_strictness_preserves_the_stackless_error_prefix() {
        // Arrange
        let error = HostError::Backend(
            "strict mode violation: locator('iframe') resolved to 3 elements".into(),
        );

        // Act
        let error = rewrite_frame_owner_error(error);

        // Assert
        assert!(matches!(
            error,
            HostError::Backend(message)
                if message == "Error: strict mode violation: locator('iframe') resolved to 3 elements"
        ));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn invalid_css_reports_the_unmatched_closing_token() {
        // Arrange
        let (locator, _budget) = locator_fixture("row]");
        let backend = Err::<(), _>(HostError::Backend(
            "Backend: invalid selector `row]`: SyntaxError: Failed to execute \
             'querySelectorAll' on 'Document': 'row]' is not a valid selector."
                .into(),
        ));

        // Act
        let error = rewrite_strict_selector_error(&locator.query, backend).unwrap_err();

        // Assert
        assert!(matches!(
            error,
            HostError::Backend(message)
                if message == "Unexpected token \"]\" while parsing css selector \"row]\""
        ));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn element_handle_source_installs_runtime_and_enforces_strict_resolution() {
        // Arrange
        let selector = pwright_cap_locator::Selector::css("button".to_owned()).unwrap();

        // Act
        let source = element_handle_expression(&selector).unwrap();

        // Assert
        assert!(source.contains(pwright_backend_cdp_selector_runtime::SELECTOR_RUNTIME_SYMBOL));
        assert!(source.contains("__pw.css(document, \"button\")"));
        assert!(source.contains("__pwMatches.length === 0"));
        assert!(source.contains("__pwMatches.length !== 1"));
        assert!(source.len() <= crate::evaluation::SOURCE_BYTES);
    }

    #[cfg(feature = "handles")]
    #[test]
    fn optional_element_handle_source_returns_first_match_or_null_without_waiting() {
        // Arrange
        let selector = pwright_cap_locator::Selector::css("button".to_owned()).unwrap();

        // Act
        let source = optional_element_handle_expression(&selector).unwrap();

        // Assert
        assert!(source.contains(pwright_backend_cdp_selector_runtime::SELECTOR_RUNTIME_SYMBOL));
        assert!(source.contains("__pw.css(document, \"button\")"));
        assert!(source.contains("__pwMatches.length === 0 ? null : __pwMatches[0]"));
        assert!(!source.contains("strict mode violation"));
        assert!(!source.contains("failed to find element"));
        assert!(source.len() <= crate::evaluation::SOURCE_BYTES);
    }

    #[cfg(feature = "handles")]
    #[test]
    fn polling_element_handle_source_waits_on_absence_but_rejects_multiple_matches() {
        // Arrange
        let matches = "__pw.xpath(document, \"//button\")".to_owned();

        // Act
        let source = polling_element_handle_expression_from_matches(matches).unwrap();

        // Assert
        assert!(source.contains("if (__pwMatches.length === 0)\n    return null"));
        assert!(source.contains("if (__pwMatches.length !== 1)"));
        assert!(source.contains("strict mode violation"));
        assert!(source.contains("return __pwMatches[0]"));
        assert!(source.len() <= crate::evaluation::SOURCE_BYTES);
    }

    #[cfg(feature = "locator-bounding-box")]
    #[test]
    fn bounding_box_extracts_exact_finite_coordinates() {
        // Arrange
        let number = |value| EvaluationValue::number(value).unwrap();
        let value = EvaluationValue::Object {
            id: None,
            properties: vec![
                EvaluationProperty::new("x", number(1.25)),
                EvaluationProperty::new("y", number(2.5)),
                EvaluationProperty::new("width", number(30.0)),
                EvaluationProperty::new("height", number(40.0)),
            ],
        };

        // Act
        let result = bounding_box_result(value);

        // Assert
        assert_eq!(
            result.unwrap(),
            Some(LocatorBoundingBox {
                x: 1.25,
                y: 2.5,
                width: 30.0,
                height: 40.0,
            })
        );
    }

    #[cfg(feature = "locator-bounding-box")]
    #[test]
    fn bounding_box_preserves_browser_null_for_invisible_elements() {
        // Arrange
        let value = EvaluationValue::Null;

        // Act
        let result = bounding_box_result(value);

        // Assert
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn state_expectations_preserve_playwright_missing_element_semantics() {
        // Arrange
        let positive = false;
        let negative = true;

        // Act
        let attached = LocatorExpectationState::Attached.matches(None, positive);
        let not_attached = LocatorExpectationState::Attached.matches(None, negative);
        let detached = LocatorExpectationState::Detached.matches(None, positive);
        let not_detached = LocatorExpectationState::Detached.matches(None, negative);
        let empty = LocatorExpectationState::Empty.matches(None, positive);
        let not_empty = LocatorExpectationState::Empty.matches(None, negative);
        let checked = LocatorExpectationState::Checked.matches(None, positive);
        let not_checked = LocatorExpectationState::Checked.matches(None, negative);
        let editable = LocatorExpectationState::Editable.matches(None, positive);
        let not_editable = LocatorExpectationState::Editable.matches(None, negative);
        let focused = LocatorExpectationState::Focused.matches(None, positive);
        let not_focused = LocatorExpectationState::Focused.matches(None, negative);

        // Assert
        assert!(!attached.unwrap());
        assert!(!not_attached.unwrap());
        assert!(detached.unwrap());
        assert!(not_detached.unwrap());
        assert!(!empty.unwrap());
        assert!(not_empty.unwrap());
        assert!(!checked.unwrap());
        assert!(not_checked.unwrap());
        assert!(!editable.unwrap());
        assert!(not_editable.unwrap());
        assert!(!focused.unwrap());
        assert!(not_focused.unwrap());
    }

    #[test]
    fn state_expectations_map_bounded_browser_values() {
        // Arrange
        let cases = [
            (LocatorExpectationState::Attached, "attached", true),
            (LocatorExpectationState::Attached, "detached", false),
            (LocatorExpectationState::Detached, "attached", false),
            (LocatorExpectationState::Detached, "detached", true),
            (LocatorExpectationState::Empty, "empty", true),
            (LocatorExpectationState::Empty, "notEmpty", false),
            (LocatorExpectationState::Checked, "checked", true),
            (LocatorExpectationState::Checked, "mixed", false),
            (LocatorExpectationState::Unchecked, "unchecked", true),
            (LocatorExpectationState::Unchecked, "checked", false),
            (LocatorExpectationState::Indeterminate, "mixed", true),
            (LocatorExpectationState::Indeterminate, "unchecked", false),
            (LocatorExpectationState::Editable, "editable", true),
            (LocatorExpectationState::Editable, "readOnly", false),
            (LocatorExpectationState::Readonly, "readOnly", true),
            (LocatorExpectationState::Readonly, "disabled", true),
            (LocatorExpectationState::Enabled, "enabled", true),
            (LocatorExpectationState::Enabled, "disabled", false),
            (LocatorExpectationState::Disabled, "disabled", true),
            (LocatorExpectationState::Disabled, "enabled", false),
            (LocatorExpectationState::Focused, "focused", true),
            (LocatorExpectationState::Focused, "inactive", false),
        ];

        // Act
        let actual = cases.map(|(state, received, _)| state.matches(Some(received), false));

        // Assert
        for ((_, _, expected), actual) in cases.into_iter().zip(actual) {
            assert_eq!(actual.unwrap(), expected);
        }
    }

    #[test]
    fn state_expectations_keep_browser_semantics_in_rust_selected_programs() {
        // Arrange
        let checked_state = LocatorExpectationState::Checked;
        let editable_state = LocatorExpectationState::Editable;
        let enabled_state = LocatorExpectationState::Enabled;
        let attached_state = LocatorExpectationState::Attached;

        // Act
        let attached = attached_state.read_operation();
        let checked = checked_state.read_operation();
        let editable = editable_state.read_operation();
        let enabled = enabled_state.read_operation();
        let indeterminate = LocatorExpectationState::Indeterminate
            .source()
            .expect("indeterminate uses a Rust-selected browser program");
        let focused = LocatorExpectationState::Focused
            .source()
            .expect("focused uses a Rust-selected browser program");

        // Assert
        assert_eq!(attached, Some(LocatorRead::Count));
        assert_eq!(checked, Some(LocatorRead::IsChecked));
        assert_eq!(editable, Some(LocatorRead::IsEditable));
        assert_eq!(enabled, Some(LocatorRead::IsEnabled));
        assert!(indeterminate.contains("target.indeterminate"));
        assert!(indeterminate.contains("aria-checked"));
        assert!(focused.contains("activeElement === element"));
        assert!(focused.contains("ownerDocument.hasFocus()"));
    }

    #[test]
    fn state_expectations_map_immediate_counts_without_a_strict_wait() {
        // Arrange
        let attached = LocatorExpectationState::Attached;
        let detached = LocatorExpectationState::Detached;

        // Act
        let present = attached.received_from_read(LocatorValue::Count(1));
        let missing = detached.received_from_read(LocatorValue::Count(0));
        let duplicate = attached.received_from_read(LocatorValue::Count(2));

        // Assert
        assert_eq!(present.unwrap(), "attached");
        assert_eq!(missing.unwrap(), "detached");
        assert!(
            duplicate
                .unwrap_err()
                .to_string()
                .contains("strict mode violation")
        );
    }

    #[test]
    fn state_expectations_complete_one_plain_locator_probe_before_timeout() {
        // Arrange
        let remaining = Duration::from_millis(1);

        // Act
        let plain_first = state_expectation_attempt_timeout(true, false, 1, remaining);
        let nested_first = state_expectation_attempt_timeout(true, true, 1, remaining);
        let retry = state_expectation_attempt_timeout(false, false, 1, remaining);

        // Assert
        assert_eq!(plain_first, 100);
        assert_eq!(nested_first, 1);
        assert_eq!(retry, 1);
    }

    #[test]
    fn count_expectations_compare_typed_collection_sizes() {
        // Arrange
        let received = 2;

        // Act
        let equal = count_expectation_matches(received, 2);
        let different = count_expectation_matches(received, 1);

        // Assert
        assert!(equal);
        assert!(!different);
    }

    #[test]
    fn state_expectations_retry_only_known_replaced_context_diagnostics() {
        // Arrange
        let replaced = [
            HostError::TargetClosed,
            HostError::Backend("CDP error: uniqueContextId not found".into()),
            HostError::Backend("Execution context was destroyed".into()),
            HostError::Backend("Cannot find context with specified id".into()),
        ];

        // Act
        let retryable = replaced.each_ref().map(locator_context_was_replaced);

        // Assert
        assert!(retryable.into_iter().all(|value| value));
        assert!(!locator_context_was_replaced(&HostError::Backend(
            "boom".into()
        )));
    }

    #[test]
    fn property_expectation_extracts_typed_result_fields() {
        // Arrange
        let value = EvaluationValue::Object {
            id: None,
            properties: vec![
                EvaluationProperty::new("received", EvaluationValue::String("abc".into())),
                EvaluationProperty::new("matches", EvaluationValue::Bool(true)),
            ],
        };

        // Act
        let result = value_expectation_result(value);

        // Assert
        assert_eq!(
            result.unwrap(),
            (true, EvaluationValue::String("abc".into()))
        );
    }

    #[test]
    fn class_collection_expectation_extracts_bounded_string_results() {
        // Arrange
        let value = EvaluationValue::Object {
            id: None,
            properties: vec![
                EvaluationProperty::new("matches", EvaluationValue::Bool(true)),
                EvaluationProperty::new(
                    "received",
                    EvaluationValue::Array {
                        id: None,
                        items: vec![
                            EvaluationValue::String("completed".into()),
                            EvaluationValue::String(String::new()),
                        ],
                    },
                ),
            ],
        };

        // Act
        let result = texts_value_expectation_result(value);

        // Assert
        assert_eq!(
            result.unwrap(),
            (true, vec!["completed".to_owned(), String::new()])
        );
    }

    #[test]
    fn class_collection_expectation_rejects_non_string_results() {
        // Arrange
        let value = EvaluationValue::Object {
            id: None,
            properties: vec![
                EvaluationProperty::new("matches", EvaluationValue::Bool(false)),
                EvaluationProperty::new(
                    "received",
                    EvaluationValue::Array {
                        id: None,
                        items: vec![EvaluationValue::Bool(false)],
                    },
                ),
            ],
        };

        // Act
        let result = texts_value_expectation_result(value);

        // Assert
        assert!(matches!(result, Err(HostError::Backend(_))));
    }

    #[test]
    fn class_collection_program_reads_svg_safe_class_lists_and_mixed_matchers() {
        // Arrange
        let source = CLASS_NAMES_EXPECTATION_SOURCE;

        // Act
        let reads_class_list = source.contains("element.classList.toString()");
        let preserves_collection_order = source.contains("received.every((value, index)");
        let supports_regex = source.contains("new RegExp(matcher.expected, matcher.flags)");

        // Assert
        assert!(reads_class_list);
        assert!(preserves_collection_order);
        assert!(supports_regex);
    }

    #[test]
    fn string_expectation_program_preserves_class_and_follow_label_value_semantics() {
        // Arrange
        let source = STRING_EXPECTATION_SOURCE;

        // Act
        let class_list = source.contains("element.classList.toString()");
        let follows_label = source.contains("element.nodeName === 'LABEL' && element.control");
        let validates_control = source.contains("element.nodeName !== 'INPUT'")
            && source.contains("element.nodeName !== 'TEXTAREA'")
            && source.contains("element.nodeName !== 'SELECT'");
        let supports_regex = source.contains("new RegExp(argument.expected, argument.flags)");

        // Assert
        assert!(class_list);
        assert!(follows_label);
        assert!(validates_control);
        assert!(supports_regex);
    }

    #[cfg(feature = "handles")]
    #[tokio::test]
    async fn string_expectation_rejects_invalid_regex_before_browser_access() {
        // Arrange
        let (locator, _budget) = locator_fixture("input");
        let matcher = StringMatcher {
            expected: "value",
            flags: Some("ii"),
        };

        // Act
        let result = locator
            .expect_string(5_000, LocatorStringExpectationKind::Value, matcher, false)
            .await;

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[cfg(feature = "handles")]
    #[tokio::test]
    async fn attribute_expectation_rejects_invalid_regex_flags_before_browser_access() {
        // Arrange
        let (locator, _budget) = locator_fixture("input");
        let expectation = AttributeExpectation {
            name: "id",
            expected: "target",
            flags: Some("ii"),
            ignore_case: None,
            is_not: false,
        };

        // Act
        let result = locator.expect_attribute(5_000, expectation).await;

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[cfg(feature = "handles")]
    #[tokio::test]
    async fn css_expectation_rejects_invalid_regex_flags_before_browser_access() {
        // Arrange
        let (locator, _budget) = locator_fixture("body");
        let expectation = CssExpectation {
            name: "background-color",
            expected: "rgb\\(",
            flags: Some("ii"),
            is_not: false,
        };

        // Act
        let result = locator.expect_css(5_000, expectation).await;

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn attribute_expectation_program_uses_dom_attributes_and_regex_flag_overrides() {
        // Arrange
        let source = ATTRIBUTE_EXPECTATION_SOURCE;

        // Act
        let uses_dom_attribute = source.contains("element.getAttribute(argument.name)");
        let can_add_ignore_case = source.contains("flags.add('i')");
        let can_remove_ignore_case = source.contains("flags.delete('i')");

        // Assert
        assert!(uses_dom_attribute);
        assert!(can_add_ignore_case);
        assert!(can_remove_ignore_case);
    }

    #[test]
    fn css_expectation_program_reads_computed_style_and_supports_regex() {
        // Arrange
        let source = CSS_EXPECTATION_SOURCE;

        // Act
        let reads_computed_style = source.contains("window.getComputedStyle(element)");
        let reads_named_property = source.contains("getPropertyValue(argument.name)");
        let supports_regex = source.contains("new RegExp(argument.expected, argument.flags)");

        // Assert
        assert!(reads_computed_style);
        assert!(reads_named_property);
        assert!(supports_regex);
    }

    #[test]
    fn text_expectation_missing_element_never_satisfies_either_polarity() {
        // Arrange
        let positive = TextExpectation {
            expected: "abc",
            substring: false,
            normalize_whitespace: false,
            ignore_case: false,
            use_inner_text: false,
            is_not: false,
        };
        let negative = TextExpectation {
            is_not: true,
            ..positive
        };

        // Act
        let positive_matches = text_expectation_matches(None, true, &positive);
        let negative_matches = text_expectation_matches(None, true, &negative);

        // Assert
        assert_eq!(positive_matches, positive.is_not);
        assert_eq!(negative_matches, negative.is_not);
    }

    #[test]
    fn visibility_reads_treat_navigation_context_loss_as_not_visible() {
        // Arrange
        let errors = [
            "CDP error -32602: uniqueContextId not found",
            "Execution context was destroyed",
            "Cannot find context with specified id",
        ];

        // Act
        let visible = errors.map(|message| {
            recover_visibility_context_loss(
                LocatorRead::IsVisible,
                Err(HostError::Backend(message.into())),
            )
        });
        let hidden = recover_visibility_context_loss(
            LocatorRead::IsHidden,
            Err(HostError::Backend(errors[0].into())),
        );

        // Assert
        assert!(
            visible
                .into_iter()
                .all(|result| { matches!(result, Ok(LocatorValue::Bool(false))) })
        );
        assert!(matches!(hidden, Ok(LocatorValue::Bool(true))));
    }

    #[test]
    fn visibility_reads_preserve_non_navigation_backend_errors() {
        // Arrange
        let error = HostError::Backend("invalid selector".into());

        // Act
        let result = recover_visibility_context_loss(LocatorRead::IsVisible, Err(error));

        // Assert
        assert!(
            matches!(result, Err(HostError::Backend(message)) if message == "invalid selector")
        );
    }

    #[test]
    fn locator_reads_map_duplicate_results_to_playwright_strictness() {
        // Arrange
        let errors = [
            "strict selector `div` resolved to 2 elements",
            "Backend: strict selector `div` resolved to 2 elements",
        ];

        // Act
        let results = errors.map(|message| {
            recover_visibility_context_loss(
                LocatorRead::InnerText,
                Err(HostError::Backend(message.into())),
            )
        });

        // Assert
        assert!(results.into_iter().all(|result| matches!(
            result,
            Err(HostError::Backend(message))
                if message == "strict mode violation: strict selector `div` resolved to 2 elements"
        )));
    }

    #[test]
    fn locator_wait_states_map_to_the_typed_capability_states() {
        // Arrange
        let states = [
            (LocatorWaitState::Attached, WaitForState::Attached),
            (LocatorWaitState::Detached, WaitForState::Detached),
            (LocatorWaitState::Visible, WaitForState::Visible),
            (LocatorWaitState::Hidden, WaitForState::Hidden),
        ];

        // Act
        let mapped = states.map(|(state, _)| WaitForState::from(state));

        // Assert
        assert_eq!(
            mapped,
            states.map(|(_, expected)| expected),
            "every public Locator wait state must remain a lossless typed mapping"
        );
    }

    #[cfg(feature = "handles")]
    #[test]
    fn frame_owner_diagnostic_distinguishes_frames_from_non_frames() {
        // Arrange
        let iframe = crate::evaluation::EvaluationValue::String(String::new());
        let div = crate::evaluation::EvaluationValue::String("<div></div>".into());

        // Act
        let iframe_preview = non_frame_owner_preview(iframe).unwrap();
        let div_preview = non_frame_owner_preview(div).unwrap();

        // Assert
        assert_eq!(iframe_preview, None);
        assert_eq!(div_preview.as_deref(), Some("<div></div>"));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_deadline_preserves_unbounded_and_original_timeout() {
        // Arrange
        let expired = tokio::time::Instant::now() - std::time::Duration::from_millis(1);

        // Act
        let unbounded = remaining_timeout_ms(None, 0);
        let timed_out = remaining_timeout_ms(Some(&expired), 71);

        // Assert
        assert_eq!(unbounded.unwrap(), 0);
        assert!(matches!(timed_out, Err(HostError::Timeout(71))));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_checks_xpath_visibility_and_returns_the_observed_element() {
        // Arrange
        let matches = "__pw.xpath(document, \"//div\")".to_owned();

        // Act
        let source = selector_wait_source(matches, LocatorWaitState::Visible, false).unwrap();

        // Assert
        assert!(source.contains("const __pwMatches = __pw.xpath(document, \"//div\")"));
        assert!(source.contains("const element = __pwMatches[0]"));
        assert!(source.contains("return element && __pw.visible(element) ? element : null"));
        assert!(!source.contains("strict mode violation"));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_checks_strictness_before_all_four_states() {
        // Arrange
        let states = [
            LocatorWaitState::Attached,
            LocatorWaitState::Detached,
            LocatorWaitState::Visible,
            LocatorWaitState::Hidden,
        ];

        // Act
        let sources = states.map(|state| selector_wait_source("[]".into(), state, true).unwrap());

        // Assert
        for source in &sources {
            let strict = source.find("strict mode violation").unwrap();
            let chosen = source.find("const element = __pwMatches[0]").unwrap();
            assert!(strict < chosen);
        }
        assert!(sources[0].contains("return element || null"));
        assert!(sources[1].contains("return !element"));
        assert!(sources[2].contains("return element && __pw.visible(element) ? element : null"));
        assert!(sources[3].contains("return !element || !__pw.visible(element)"));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_rejects_source_beyond_the_evaluation_budget() {
        // Arrange
        let matches = "x".repeat(crate::evaluation::SOURCE_BYTES);

        // Act
        let result = selector_wait_source(matches, LocatorWaitState::Attached, false);

        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_reports_original_timeout_after_a_nested_wait_expires() {
        // Arrange
        let narrowed_timeout = HostError::Timeout(68);
        let backend_error = HostError::Backend("selector failed".into());

        // Act
        let timeout = selector_wait_error(narrowed_timeout, 71);
        let backend = selector_wait_error(backend_error, 71);

        // Assert
        assert!(matches!(timeout, HostError::Timeout(71)));
        assert!(matches!(backend, HostError::Backend(message) if message == "selector failed"));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_retries_the_exact_cdp_navigation_diagnostic() {
        // Arrange
        let navigated =
            HostError::Backend("CDP error -32000: Inspected target navigated or closed".into());

        // Act
        let navigation_retryable = selector_wait_context_was_replaced(&navigated);

        // Assert
        assert!(navigation_retryable);
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_navigation_retry_does_not_widen_other_locator_retries() {
        // Arrange
        let navigated =
            HostError::Backend("CDP error -32000: Inspected target navigated or closed".into());

        // Act
        let retryable = locator_context_was_replaced(&navigated);

        // Assert
        assert!(!retryable);
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_does_not_retry_an_unrelated_cdp_error() {
        // Arrange
        let unrelated = HostError::Backend("CDP error -32000: Target closed".into());

        // Act
        let retryable = selector_wait_context_was_replaced(&unrelated);

        // Assert
        assert!(!retryable);
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_retry_preserves_navigation_error_for_inactive_frame() {
        // Arrange
        let navigated =
            HostError::Backend("CDP error -32000: Inspected target navigated or closed".into());
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);

        // Act
        let outcome = selector_wait_retry_delay(navigated, false, Some(&deadline), 71);

        // Assert
        assert!(
            matches!(outcome, Err(HostError::Backend(message)) if message.contains("Inspected target navigated or closed"))
        );
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_retry_reports_original_timeout_after_deadline() {
        // Arrange
        let deadline = tokio::time::Instant::now() - Duration::from_millis(1);

        // Act
        let outcome = selector_wait_retry_delay(
            HostError::Backend("CDP error -32000: Inspected target navigated or closed".into()),
            true,
            Some(&deadline),
            71,
        );

        // Assert
        assert!(matches!(outcome, Err(HostError::Timeout(71))));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn selector_wait_retry_uses_a_bounded_delay_on_the_same_active_frame() {
        // Arrange
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);

        // Act
        let outcome = selector_wait_retry_delay(
            HostError::Backend("CDP error -32000: Inspected target navigated or closed".into()),
            true,
            Some(&deadline),
            71,
        );

        // Assert
        assert!(
            matches!(outcome, Ok(delay) if delay > Duration::ZERO && delay <= Duration::from_millis(10))
        );
    }

    #[test]
    fn nested_locator_frame_errors_preserve_the_official_combination_context() {
        // Arrange
        let combinations = [
            (
                QueryCombination::Has,
                "Inner \"has\" locator must belong to the same frame.",
            ),
            (
                QueryCombination::HasNot,
                "Inner \"has_not\" locator must belong to the same frame.",
            ),
            (
                QueryCombination::And,
                "Locators must belong to the same frame.",
            ),
        ];

        // Act
        let messages =
            combinations.map(|(combination, _)| locator_combination_frame_error(combination));

        // Assert
        assert_eq!(messages, combinations.map(|(_, expected)| expected));
    }

    #[test]
    fn text_expectation_normalizes_whitespace_and_matches_substring() {
        // Arrange
        let expectation = TextExpectation {
            expected: "request count: 1",
            substring: true,
            normalize_whitespace: true,
            ignore_case: false,
            use_inner_text: false,
            is_not: false,
        };

        // Act
        let matches = text_matches("  request\n count:   1 ", &expectation);

        // Assert
        assert!(matches);
    }

    #[test]
    fn text_expectation_can_compare_case_insensitively_without_substring_mode() {
        // Arrange
        let expectation = TextExpectation {
            expected: "READY",
            substring: false,
            normalize_whitespace: false,
            ignore_case: true,
            use_inner_text: false,
            is_not: false,
        };

        // Act
        let matches = text_matches("ready", &expectation);

        // Assert
        assert!(matches);
    }

    #[test]
    fn text_collection_expectation_preserves_order_and_normalizes_each_item() {
        // Arrange
        let received = vec!["  first\nitem ".to_owned(), "second item".to_owned()];
        let expected = vec!["first item".to_owned(), "second item".to_owned()];
        let reversed = vec!["second item".to_owned(), "first item".to_owned()];
        let expectation = TextsExpectation {
            expected: &expected,
            substring: false,
            normalize_whitespace: true,
            ignore_case: false,
            use_inner_text: false,
            is_not: false,
        };

        // Act
        let ordered_matches = texts_expectation_matches(&received, &expectation);
        let reversed_matches = texts_expectation_matches(
            &received,
            &TextsExpectation {
                expected: &reversed,
                ..expectation
            },
        );

        // Assert
        assert!(ordered_matches);
        assert!(!reversed_matches);
    }
}
