//! Optional ElementHandle DOM reads rooted in an existing Rust-owned handle.

#[cfg(feature = "touchscreen-input")]
use pwright_backend_cdp_input::CdpModifiers;
use pwright_backend_cdp_selector_runtime::{
    SELECTOR_RUNTIME_SYMBOL, SELECTOR_RUNTIME_VERSION, compile_selector,
    selector_runtime_install_script,
};
use pwright_cap_locator::Selector;
#[cfg(feature = "touchscreen-input")]
use pwright_cap_locator::{KeyboardModifier, KeyboardModifiers, ScrollMode};
#[cfg(feature = "touchscreen-input")]
use pwright_cap_touchscreen::{SelectorTapOptions, TapParams, TouchscreenCoordinate};
use std::str::FromStr;

#[cfg(feature = "locator-dom-actions")]
use crate::locator::DISPATCH_EVENT_SOURCE;
#[cfg(feature = "locator-bounding-box")]
use crate::locator::{BOUNDING_BOX_SOURCE, LocatorBoundingBox, bounding_box_result};
use crate::{
    HostError, HostSession,
    evaluation::{self, EvaluationValue},
    handles::{MAX_PREVIEW_BYTES, OwnedHandleArgument, PendingHandle},
    locator_query::{ElementHandleQuery, ElementHandleQueryPlan},
};

const INNER_TEXT_SOURCE: &str = r#"(element) => {
  if (!(element instanceof HTMLElement))
    throw new Error("Node is not an HTMLElement");
  return element.innerText;
}"#;
const TEXT_CONTENT_SOURCE: &str = "(element) => element.textContent";
const INNER_HTML_SOURCE: &str = "(element) => element.innerHTML";
const GET_ATTRIBUTE_SOURCE: &str = "(element, name) => element.getAttribute(name)";
const ELEMENT_PREVIEW_SOURCE: &str = r#"(element) => {
  const ELEMENT_NODE = 1;
  const TEXT_NODE = 3;
  const trimWithEllipsis = (input, cap) => {
    if (input.length <= cap)
      return input;
    const chars = [...input];
    if (chars.length > cap)
      return chars.slice(0, cap - 1).join('') + '\u2026';
    return chars.join('');
  };
  const oneLine = value => value.replace(/\n/g, '\u21b5').replace(/\t/g, '\u21c6');
  if (element.nodeType === TEXT_NODE)
    return 'JSHandle@' + oneLine(`#text=${element.nodeValue || ''}`);
  if (element.nodeType !== ELEMENT_NODE)
    return 'JSHandle@' + oneLine(`<${element.nodeName.toLowerCase()} />`);

  const booleanAttributes = new Set(['checked', 'selected', 'disabled', 'readonly', 'multiple']);
  const autoClosingTags = new Set([
    'AREA', 'BASE', 'BR', 'COL', 'COMMAND', 'EMBED', 'HR', 'IMG', 'INPUT',
    'KEYGEN', 'LINK', 'MENUITEM', 'META', 'PARAM', 'SOURCE', 'TRACK', 'WBR',
  ]);
  const attrs = [];
  for (let i = 0; i < element.attributes.length; i++) {
    const { name, value } = element.attributes[i];
    if (name === 'style')
      continue;
    if (!value && booleanAttributes.has(name))
      attrs.push(` ${name}`);
    else
      attrs.push(` ${name}="${value}"`);
  }
  attrs.sort((a, b) => a.length - b.length);
  const attrText = trimWithEllipsis(attrs.join(''), 500);
  const tagName = element.nodeName.toLowerCase();
  if (autoClosingTags.has(element.nodeName))
    return 'JSHandle@' + oneLine(`<${tagName}${attrText}/>`);

  const children = element.childNodes;
  let onlyText = false;
  if (children.length <= 5) {
    onlyText = true;
    for (let i = 0; i < children.length; i++)
      onlyText = onlyText && children[i].nodeType === TEXT_NODE;
  }
  const text = onlyText ? (element.textContent || '') : (children.length ? '\u2026' : '');
  return 'JSHandle@' + oneLine(`<${tagName}${attrText}>${trimWithEllipsis(text, 50)}</${tagName}>`);
}"#;
const FOCUS_SOURCE: &str = r#"(element) => {
  if (!(element instanceof HTMLElement))
    throw new Error("Node is not an HTMLElement");
  element.focus();
}"#;
#[cfg(feature = "element-handle-state-wait")]
const STABLE_WAIT_SOURCE: &str = r#"(element) => new Promise((resolve, reject) => {
  let previous = null;
  const sample = () => {
    try {
      if (!element.isConnected)
        throw new Error('Element is not attached to the DOM');
      const rect = element.getBoundingClientRect();
      const current = { x: rect.x, y: rect.y, width: rect.width, height: rect.height };
      if (previous !== null) {
        resolve(current.x === previous.x && current.y === previous.y &&
          current.width === previous.width && current.height === previous.height);
        return;
      }
      previous = current;
      requestAnimationFrame(sample);
    } catch (error) {
      reject(error);
    }
  };
  requestAnimationFrame(sample);
})"#;
#[cfg(feature = "touchscreen-input")]
const TAP_TRIAL_INSTALL_SOURCE: &str = r#"(element) => {
  const key = Symbol.for('pwright.element-handle.tap.trial');
  globalThis[key]?.stop?.();
  const types = ['pointerdown', 'pointerup', 'touchstart', 'touchend', 'touchcancel'];
  const listener = event => {
    if (!event.isTrusted) return;
    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();
  };
  for (const type of types) window.addEventListener(type, listener, true);
  globalThis[key] = { stop() {
    for (const type of types) window.removeEventListener(type, listener, true);
    delete globalThis[key];
  } };
}"#;
#[cfg(feature = "touchscreen-input")]
const TAP_TRIAL_REMOVE_SOURCE: &str = r#"(element) => {
  const key = Symbol.for('pwright.element-handle.tap.trial');
  globalThis[key]?.stop?.();
}"#;

const ATTRIBUTE_BYTES: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementHandleDomRead {
    GetAttribute,
    InnerHtml,
    InputValue,
    IsChecked,
    IsDisabled,
    IsEditable,
    IsEnabled,
    IsHidden,
    IsVisible,
}

impl FromStr for ElementHandleDomRead {
    type Err = HostError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "getAttribute" => Self::GetAttribute,
            "innerHTML" => Self::InnerHtml,
            "inputValue" => Self::InputValue,
            "isChecked" => Self::IsChecked,
            "isDisabled" => Self::IsDisabled,
            "isEditable" => Self::IsEditable,
            "isEnabled" => Self::IsEnabled,
            "isHidden" => Self::IsHidden,
            "isVisible" => Self::IsVisible,
            _ => {
                return Err(HostError::UnsupportedOperation(
                    "unknown ElementHandle DOM read operation".into(),
                ));
            }
        })
    }
}

#[cfg(feature = "element-handle-state-predicates")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElementHandleStatePredicate {
    Disabled,
    Editable,
    Enabled,
    Hidden,
    Visible,
}

#[cfg(feature = "element-handle-state-predicates")]
impl FromStr for ElementHandleStatePredicate {
    type Err = HostError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "isDisabled" => Ok(Self::Disabled),
            "isEditable" => Ok(Self::Editable),
            "isEnabled" => Ok(Self::Enabled),
            "isHidden" => Ok(Self::Hidden),
            "isVisible" => Ok(Self::Visible),
            _ => Err(HostError::InvalidArgument(
                "unknown ElementHandle state predicate".into(),
            )),
        }
    }
}

#[cfg(feature = "element-handle-state-predicates")]
impl ElementHandleStatePredicate {
    fn dom_read(self) -> ElementHandleDomRead {
        match self {
            Self::Disabled => ElementHandleDomRead::IsDisabled,
            Self::Editable => ElementHandleDomRead::IsEditable,
            Self::Enabled => ElementHandleDomRead::IsEnabled,
            Self::Hidden => ElementHandleDomRead::IsHidden,
            Self::Visible => ElementHandleDomRead::IsVisible,
        }
    }
}

pub enum ElementHandleDomValue {
    Bool(bool),
    Text(Option<String>),
}

#[derive(Clone, Copy)]
enum QueryOutput {
    First,
    IndexedObject,
    Array,
}

impl HostSession {
    /// Snapshot every match in one Frame evaluation, then publish the retained
    /// elements as one Rust-owned batch. The temporary indexed carrier never
    /// crosses the language boundary and is settled before success is returned.
    pub(crate) async fn frame_query_selector_all(
        &self,
        frame_id: &str,
        selector: &Selector,
    ) -> Result<Vec<PendingHandle>, HostError> {
        let matches = compile_selector(selector, "document").map_err(HostError::backend)?;
        self.frame_query_compiled_all(frame_id, matches).await
    }

    pub(crate) async fn frame_query_compiled_all(
        &self,
        frame_id: &str,
        matches: String,
    ) -> Result<Vec<PendingHandle>, HostError> {
        let source = frame_query_all_source_from_matches(matches)?;
        let collection = self
            .frame_prepare_handle(
                frame_id,
                crate::handles::EvaluateHandleParams::expression(source),
            )
            .await?;
        let properties = settle_temporary(
            self.handle_get_properties(&collection).await,
            &collection,
            "Frame query collection",
        )
        .await?;
        indexed_elements(properties).await
    }

    /// Focus the exact retained ElementHandle before a separately admitted
    /// Page-owned input command. The language binding never recreates a
    /// selector or owns browser-side focus semantics.
    pub async fn element_handle_focus(&self, target: &PendingHandle) -> Result<(), HostError> {
        require_element(target)?;
        self.handle_evaluate_owned(target, FOCUS_SOURCE.into(), undefined_argument()?)
            .await?;
        Ok(())
    }

    /// Tap the exact retained ElementHandle. Actionability is evaluated
    /// against its native object identity; no selector reconstruction can
    /// silently redirect the action to a replacement element.
    #[cfg(feature = "touchscreen-input")]
    pub async fn element_handle_tap(
        &self,
        target: &PendingHandle,
        options: SelectorTapOptions,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        require_element(target)?;
        let page_id = target
            .source_world()
            .map(|world| world.page_id())
            .ok_or_else(|| HostError::backend("Handle creation-world identity is missing"))?;
        let source = element_tap_actionability_source(options)?;
        let deadline = (timeout_ms != 0).then(|| {
            tokio::time::Instant::now() + std::time::Duration::from_millis(u64::from(timeout_ms))
        });
        let mut last_progress = "waiting for element to be visible, enabled and stable";
        let point = loop {
            let evaluation =
                self.handle_evaluate_owned(target, source.clone(), undefined_argument()?);
            let value = match deadline {
                Some(deadline) => {
                    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                    if remaining.is_zero() {
                        return Err(element_tap_timeout(target, timeout_ms, last_progress));
                    }
                    tokio::time::timeout(remaining, evaluation)
                        .await
                        .map_err(|_| element_tap_timeout(target, timeout_ms, last_progress))??
                }
                None => evaluation.await?,
            };
            match parse_element_tap_attempt(value)? {
                ElementTapAttempt::Ready { x, y, is_mac } => break (x, y, is_mac),
                ElementTapAttempt::Retry(progress) => {
                    last_progress = progress;
                    let delay = std::time::Duration::from_millis(50);
                    if let Some(deadline) = deadline {
                        let remaining =
                            deadline.saturating_duration_since(tokio::time::Instant::now());
                        if remaining.is_zero() {
                            return Err(element_tap_timeout(target, timeout_ms, last_progress));
                        }
                        tokio::time::sleep(delay.min(remaining)).await;
                    } else {
                        tokio::time::sleep(delay).await;
                    }
                }
            }
        };

        let trial = options.trial.unwrap_or(false);
        if trial {
            self.handle_evaluate_owned(
                target,
                TAP_TRIAL_INSTALL_SOURCE.into(),
                undefined_argument()?,
            )
            .await?;
        }
        let modifiers = options
            .modifiers
            .map(|modifiers| element_tap_modifiers(modifiers, point.2));
        let params = TapParams::new(
            TouchscreenCoordinate::try_from(point.0).map_err(HostError::backend)?,
            TouchscreenCoordinate::try_from(point.1).map_err(HostError::backend)?,
        );
        let action = self.element_handle_tap_at(page_id, params, modifiers).await;
        let cleanup = if trial {
            self.handle_evaluate_owned(
                target,
                TAP_TRIAL_REMOVE_SOURCE.into(),
                undefined_argument()?,
            )
            .await
            .map(|_| ())
        } else {
            Ok(())
        };
        cleanup?;
        action
    }

    /// Read HTMLElement.innerText without delegating browser semantics to a
    /// language binding.
    pub async fn element_handle_inner_text(
        &self,
        target: &PendingHandle,
    ) -> Result<String, HostError> {
        require_element(target)?;
        let value = self
            .handle_evaluate_owned(target, INNER_TEXT_SOURCE.into(), undefined_argument()?)
            .await?;
        match value {
            EvaluationValue::String(value) => Ok(value),
            _ => Err(HostError::Backend(
                "ElementHandle.inner_text returned a non-string value".into(),
            )),
        }
    }

    /// Read Node.textContent from the exact retained DOM owner.
    pub async fn element_handle_text_content(
        &self,
        target: &PendingHandle,
    ) -> Result<Option<String>, HostError> {
        require_element(target)?;
        let value = self
            .handle_evaluate_owned(target, TEXT_CONTENT_SOURCE.into(), undefined_argument()?)
            .await?;
        match value {
            EvaluationValue::Null => Ok(None),
            EvaluationValue::String(value) => Ok(Some(value)),
            _ => Err(HostError::Backend(
                "ElementHandle.text_content returned a non-string value".into(),
            )),
        }
    }

    /// Read the existing geometry capability against this retained DOM owner.
    /// No selector is reconstructed, so a later matching replacement cannot
    /// silently become the target of the measurement.
    #[cfg(feature = "locator-bounding-box")]
    pub async fn element_handle_bounding_box(
        &self,
        target: &PendingHandle,
    ) -> Result<Option<LocatorBoundingBox>, HostError> {
        require_element(target)?;
        let value = self
            .handle_evaluate_owned(target, BOUNDING_BOX_SOURCE.into(), undefined_argument()?)
            .await?;
        bounding_box_result(value)
    }

    /// Dispatch a static Rust-selected DOM event against the exact retained
    /// ElementHandle. The initializer is an already validated value tree;
    /// language Handle references are rejected before this host call.
    #[cfg(feature = "locator-dom-actions")]
    pub async fn element_handle_dispatch_event(
        &self,
        target: &PendingHandle,
        event_type: String,
        event_init: EvaluationValue,
    ) -> Result<(), HostError> {
        let argument = OwnedHandleArgument::from_slots(event_init, Vec::new())?;
        self.element_handle_dispatch_event_owned(target, event_type, argument)
            .await
    }

    /// Preserve Handle leaves in eventInit and use the retained target's exact
    /// execution world. Cross-world references are rejected by the Handle
    /// evaluation route rather than serialized into impostor objects.
    #[cfg(feature = "locator-dom-actions")]
    pub async fn element_handle_dispatch_event_owned(
        &self,
        target: &PendingHandle,
        event_type: String,
        event_init: OwnedHandleArgument,
    ) -> Result<(), HostError> {
        require_element(target)?;
        let argument = event_init.for_dispatch_event(event_type)?;
        self.handle_evaluate_owned(target, DISPATCH_EVENT_SOURCE.into(), argument)
            .await?;
        Ok(())
    }

    /// Compute Playwright's asynchronously refreshed, bounded Node preview
    /// from the exact retained Handle. The browser algorithm is fixed by this
    /// Rust capability; language wrappers only cache the returned string.
    pub async fn element_handle_preview(
        &self,
        target: &PendingHandle,
    ) -> Result<String, HostError> {
        require_element(target)?;
        let value = self
            .handle_evaluate_owned(target, ELEMENT_PREVIEW_SOURCE.into(), undefined_argument()?)
            .await?;
        match value {
            EvaluationValue::String(value) if value.len() <= MAX_PREVIEW_BYTES => Ok(value),
            EvaluationValue::String(_) => Err(HostError::ResourceExhausted(format!(
                "ElementHandle preview exceeds {MAX_PREVIEW_BYTES} UTF-8 bytes"
            ))),
            _ => Err(HostError::Backend(
                "ElementHandle preview returned a non-string value".into(),
            )),
        }
    }

    /// Read a DOM property or state from the exact retained ElementHandle.
    /// Selector-free reads stay attached to the original browser object; the
    /// shared Rust-owned selector runtime supplies Playwright-style retarget
    /// and element-state semantics where those are required.
    pub async fn element_handle_dom_read(
        &self,
        target: &PendingHandle,
        operation: ElementHandleDomRead,
        attribute: Option<String>,
    ) -> Result<ElementHandleDomValue, HostError> {
        require_element(target)?;
        if (operation == ElementHandleDomRead::GetAttribute) != attribute.is_some() {
            return Err(HostError::InvalidArgument(
                "only getAttribute requires an attribute name".into(),
            ));
        }
        if attribute
            .as_ref()
            .is_some_and(|name| name.capacity() > ATTRIBUTE_BYTES)
        {
            return Err(HostError::InvalidArgument(format!(
                "attribute name exceeds {ATTRIBUTE_BYTES} bytes"
            )));
        }

        let (source, argument) = match operation {
            ElementHandleDomRead::GetAttribute => (
                GET_ATTRIBUTE_SOURCE.to_owned(),
                OwnedHandleArgument::from_slots(
                    EvaluationValue::String(attribute.expect("validated attribute")),
                    Vec::new(),
                )?,
            ),
            ElementHandleDomRead::InnerHtml => {
                (INNER_HTML_SOURCE.to_owned(), undefined_argument()?)
            }
            ElementHandleDomRead::InputValue => (input_value_source()?, undefined_argument()?),
            ElementHandleDomRead::IsChecked => (
                element_state_source("checked", false)?,
                undefined_argument()?,
            ),
            ElementHandleDomRead::IsDisabled => (
                element_state_source("enabled", true)?,
                undefined_argument()?,
            ),
            ElementHandleDomRead::IsEditable => (
                element_state_source("editable", false)?,
                undefined_argument()?,
            ),
            ElementHandleDomRead::IsEnabled => (
                element_state_source("enabled", false)?,
                undefined_argument()?,
            ),
            ElementHandleDomRead::IsHidden => (visibility_source(true)?, undefined_argument()?),
            ElementHandleDomRead::IsVisible => (visibility_source(false)?, undefined_argument()?),
        };
        let value = self.handle_evaluate_owned(target, source, argument).await?;
        match operation {
            ElementHandleDomRead::GetAttribute => match value {
                EvaluationValue::Null => Ok(ElementHandleDomValue::Text(None)),
                EvaluationValue::String(value) => Ok(ElementHandleDomValue::Text(Some(value))),
                _ => Err(HostError::Backend(
                    "ElementHandle.get_attribute returned a non-string value".into(),
                )),
            },
            ElementHandleDomRead::InnerHtml | ElementHandleDomRead::InputValue => match value {
                EvaluationValue::String(value) => Ok(ElementHandleDomValue::Text(Some(value))),
                _ => Err(HostError::Backend(
                    "ElementHandle DOM read returned a non-string value".into(),
                )),
            },
            ElementHandleDomRead::IsChecked
            | ElementHandleDomRead::IsDisabled
            | ElementHandleDomRead::IsEditable
            | ElementHandleDomRead::IsEnabled
            | ElementHandleDomRead::IsHidden
            | ElementHandleDomRead::IsVisible => match value {
                EvaluationValue::Bool(value) => Ok(ElementHandleDomValue::Bool(value)),
                _ => Err(HostError::Backend(
                    "ElementHandle state read returned a non-boolean value".into(),
                )),
            },
        }
    }

    /// Exact retained-node predicates share the existing DOM state evaluator.
    #[cfg(feature = "element-handle-state-predicates")]
    pub async fn element_handle_state_predicate(
        &self,
        target: &PendingHandle,
        predicate: ElementHandleStatePredicate,
    ) -> Result<bool, HostError> {
        let value = self
            .element_handle_dom_read(target, predicate.dom_read(), None)
            .await?;
        let ElementHandleDomValue::Bool(value) = value else {
            return Err(HostError::Backend(
                "ElementHandle state predicate returned non-boolean value".into(),
            ));
        };
        Ok(value)
    }

    /// Wait against the original retained node. A selector-based wait could
    /// silently follow a replacement node after detachment.
    #[cfg(feature = "element-handle-state-wait")]
    pub async fn element_handle_wait_for_element_state(
        &self,
        target: &PendingHandle,
        state: &str,
        timeout_ms: u32,
    ) -> Result<(), HostError> {
        require_element(target)?;
        let source = wait_state_source(state)?;
        let deadline = (timeout_ms != 0).then(|| {
            tokio::time::Instant::now() + std::time::Duration::from_millis(u64::from(timeout_ms))
        });
        loop {
            let evaluation =
                self.handle_evaluate_owned(target, source.clone(), undefined_argument()?);
            let value = match deadline {
                Some(deadline) => tokio::time::timeout_at(deadline, evaluation)
                    .await
                    .map_err(|_| HostError::Timeout(timeout_ms))??,
                None => evaluation.await?,
            };
            match value {
                EvaluationValue::Bool(true) => return Ok(()),
                EvaluationValue::Bool(false) => {}
                _ => {
                    return Err(HostError::Backend(
                        "ElementHandle state wait returned a non-boolean value".into(),
                    ));
                }
            }
            let delay = std::time::Duration::from_millis(50);
            match deadline {
                Some(deadline) => {
                    let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                    if remaining.is_zero() {
                        return Err(HostError::Timeout(timeout_ms));
                    }
                    tokio::time::sleep(delay.min(remaining)).await;
                }
                None => tokio::time::sleep(delay).await,
            }
        }
    }

    /// Resolve the first descendant match from an existing ElementHandle. A
    /// non-match is released natively before returning `None`.
    pub async fn element_handle_query_selector(
        &self,
        target: &PendingHandle,
        query: ElementHandleQuery,
    ) -> Result<Option<PendingHandle>, HostError> {
        require_element(target)?;
        let (plan, _query_lease) = query.command()?;
        let source = query_source(&plan, QueryOutput::First)?;
        let result = self
            .handle_prepare_owned(target, source, true, undefined_argument()?)
            .await?;
        if result.is_element() {
            return Ok(Some(result));
        }
        result.dispose().await?;
        Ok(None)
    }

    /// Resolve every descendant match and return only exact element owners.
    /// The temporary collection never crosses the Rust boundary.
    pub async fn element_handle_query_selector_all(
        &self,
        target: &PendingHandle,
        query: ElementHandleQuery,
    ) -> Result<Vec<PendingHandle>, HostError> {
        require_element(target)?;
        let (plan, _query_lease) = query.command()?;
        let source = query_source(&plan, QueryOutput::IndexedObject)?;
        let collection = self
            .handle_prepare_owned(target, source, true, undefined_argument()?)
            .await?;
        let properties = settle_temporary(
            self.handle_get_properties(&collection).await,
            &collection,
            "ElementHandle query collection",
        )
        .await?;
        indexed_elements(properties).await
    }

    /// Evaluate a user function against one Rust-created array Handle and
    /// release that temporary array before publishing the result.
    pub async fn element_handle_eval_on_selector_all(
        &self,
        target: &PendingHandle,
        query: ElementHandleQuery,
        source: String,
        is_function: bool,
        argument: OwnedHandleArgument,
    ) -> Result<EvaluationValue, HostError> {
        require_element(target)?;
        let (plan, _query_lease) = query.command()?;
        let query_source = query_source(&plan, QueryOutput::Array)?;
        let collection = self
            .handle_prepare_owned(target, query_source, true, undefined_argument()?)
            .await?;
        settle_temporary(
            self.handle_evaluate_program_owned(&collection, source, is_function, argument)
                .await,
            &collection,
            "ElementHandle query array",
        )
        .await
    }
}

#[cfg(feature = "touchscreen-input")]
enum ElementTapAttempt {
    Ready { x: f64, y: f64, is_mac: bool },
    Retry(&'static str),
}

#[cfg(feature = "touchscreen-input")]
fn element_tap_timeout(
    target: &PendingHandle,
    timeout_ms: u32,
    progress: &'static str,
) -> HostError {
    HostError::ActionTimeout {
        api: "ElementHandle.tap",
        timeout_ms,
        locator: target.initializer().preview.clone(),
        progress,
    }
}

#[cfg(feature = "touchscreen-input")]
fn element_tap_modifiers(modifiers: KeyboardModifiers, is_mac: bool) -> CdpModifiers {
    modifiers
        .iter()
        .fold(CdpModifiers::NONE, |resolved, modifier| {
            resolved.with(match modifier {
                KeyboardModifier::Alt => CdpModifiers::ALT,
                KeyboardModifier::Control => CdpModifiers::CONTROL,
                KeyboardModifier::ControlOrMeta if is_mac => CdpModifiers::META,
                KeyboardModifier::ControlOrMeta => CdpModifiers::CONTROL,
                KeyboardModifier::Meta => CdpModifiers::META,
                KeyboardModifier::Shift => CdpModifiers::SHIFT,
            })
        })
}

#[cfg(feature = "touchscreen-input")]
fn element_tap_actionability_source(options: SelectorTapOptions) -> Result<String, HostError> {
    let force = options.force.unwrap_or(false);
    let scroll = matches!(options.scroll.unwrap_or(ScrollMode::Auto), ScrollMode::Auto);
    let position = options.position.map_or_else(
        || "null".to_owned(),
        |point| format!("{{ x: {}, y: {} }}", point.x(), point.y()),
    );
    selector_runtime_source(&format!(
        r#"return (async () => {{
  if (!element || !element.isConnected) return {{ status: 'detached' }};
  if (element.nodeType !== Node.ELEMENT_NODE)
    return {{ status: 'invalid', message: 'Node is not an Element' }};
  const style = getComputedStyle(element);
  let rect = element.getBoundingClientRect();
  if (!{force} && (style.visibility === 'hidden' || style.display === 'none' || rect.width <= 0 || rect.height <= 0))
    return {{ status: 'notvisible' }};
  if (!{force}) {{
    const enabled = __pw.elementState(element, 'enabled');
    if (enabled.status === 'retry') return {{ status: 'detached' }};
    if (enabled.status !== 'done') return {{ status: 'invalid', message: enabled.message }};
    if (!enabled.value) return {{ status: 'disabled' }};
  }}
  if ({scroll}) element.scrollIntoView({{ block: 'center', inline: 'center', behavior: 'instant' }});
  await new Promise(resolve => {{
    if (document.visibilityState !== 'visible') {{ setTimeout(resolve, 0); return; }}
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }});
  if (!element.isConnected) return {{ status: 'detached' }};
  const next = element.getBoundingClientRect();
  if (!{force} && (Math.abs(rect.x - next.x) > 0.25 || Math.abs(rect.y - next.y) > 0.25 ||
      Math.abs(rect.width - next.width) > 0.25 || Math.abs(rect.height - next.height) > 0.25))
    return {{ status: 'unstable' }};
  rect = next;
  const position = {position};
  let x;
  let y;
  if (position) {{
    x = rect.left + (Number.parseFloat(style.borderLeftWidth) || 0) + position.x;
    y = rect.top + (Number.parseFloat(style.borderTopWidth) || 0) + position.y;
  }} else {{
    const visible = Array.from(element.getClientRects(), candidate => ({{
      left: Math.max(0, candidate.left), top: Math.max(0, candidate.top),
      right: Math.min(window.innerWidth, candidate.right), bottom: Math.min(window.innerHeight, candidate.bottom),
    }})).find(candidate => (candidate.right - candidate.left) * (candidate.bottom - candidate.top) > 0.99);
    if (!visible) return {{ status: 'notvisible' }};
    x = (visible.left + visible.right) / 2;
    y = (visible.top + visible.bottom) / 2;
  }}
  if (x < 0 || y < 0 || x >= window.innerWidth || y >= window.innerHeight)
    return {{ status: 'notvisible' }};
  if (!{force}) {{
    const hit = document.elementFromPoint(x, y);
    if (!hit || (hit !== element && !element.contains(hit))) return {{ status: 'obscured' }};
  }}
  return {{ status: 'ready', x, y, isMac: /Mac|iPhone|iPad|iPod/.test(navigator.platform) }};
}})();"#,
    ))
}

#[cfg(feature = "touchscreen-input")]
fn parse_element_tap_attempt(value: EvaluationValue) -> Result<ElementTapAttempt, HostError> {
    let EvaluationValue::Object { properties, .. } = value else {
        return Err(HostError::backend(
            "ElementHandle.tap actionability returned a non-object value",
        ));
    };
    let property = |name: &str| {
        properties
            .iter()
            .find(|property| property.key == name)
            .map(|property| &property.value)
    };
    let status = match property("status") {
        Some(EvaluationValue::String(status)) => status.as_str(),
        _ => {
            return Err(HostError::backend(
                "ElementHandle.tap actionability omitted status",
            ));
        }
    };
    match status {
        "ready" => {
            let number = |name| match property(name) {
                Some(EvaluationValue::Number(value)) => Ok((*value).get()),
                _ => Err(HostError::backend(format!(
                    "ElementHandle.tap actionability omitted finite {name}"
                ))),
            };
            let is_mac = match property("isMac") {
                Some(EvaluationValue::Bool(value)) => *value,
                _ => {
                    return Err(HostError::backend(
                        "ElementHandle.tap actionability omitted platform marker",
                    ));
                }
            };
            Ok(ElementTapAttempt::Ready {
                x: number("x")?,
                y: number("y")?,
                is_mac,
            })
        }
        "notvisible" => Ok(ElementTapAttempt::Retry("element is not visible")),
        "unstable" => Ok(ElementTapAttempt::Retry("element is not stable")),
        "disabled" => Ok(ElementTapAttempt::Retry("element is not enabled")),
        "obscured" => Ok(ElementTapAttempt::Retry(
            "element does not receive pointer events",
        )),
        "detached" => Err(HostError::TargetClosed),
        "invalid" => match property("message") {
            Some(EvaluationValue::String(message)) => Err(HostError::Backend(message.clone())),
            _ => Err(HostError::backend(
                "ElementHandle.tap actionability returned an invalid target",
            )),
        },
        other => Err(HostError::backend(format!(
            "ElementHandle.tap actionability returned unknown status `{other}`"
        ))),
    }
}

#[cfg(test)]
fn frame_query_all_source(selector: &Selector) -> Result<String, HostError> {
    let matches = compile_selector(selector, "document").map_err(HostError::backend)?;
    frame_query_all_source_from_matches(matches)
}

fn frame_query_all_source_from_matches(matches: String) -> Result<String, HostError> {
    let install = selector_runtime_install_script();
    let result = query_result_source(QueryOutput::IndexedObject);
    let source = format!(
        r#"(() => {{
  {install};
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error("Pwright selector runtime is unavailable");
  const __pwMatches = {matches};
  {result}
}})()"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    );
    if source.len() > evaluation::SOURCE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "Frame query-all source exceeds {} UTF-8 bytes",
            evaluation::SOURCE_BYTES
        )));
    }
    Ok(source)
}

fn require_element(target: &PendingHandle) -> Result<(), HostError> {
    if target.is_element() {
        Ok(())
    } else {
        Err(HostError::InvalidArgument(
            "ElementHandle DOM operation requires an ElementHandle".into(),
        ))
    }
}

fn undefined_argument() -> Result<OwnedHandleArgument, HostError> {
    OwnedHandleArgument::from_slots(EvaluationValue::Undefined, Vec::new())
}

fn selector_runtime_source(body: &str) -> Result<String, HostError> {
    let install = selector_runtime_install_script();
    let source = format!(
        r#"(element) => {{
  {install};
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error("Pwright selector runtime is unavailable");
  {body}
}}"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    );
    if source.len() > evaluation::SOURCE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "ElementHandle DOM source exceeds {} UTF-8 bytes",
            evaluation::SOURCE_BYTES
        )));
    }
    Ok(source)
}

fn input_value_source() -> Result<String, HostError> {
    selector_runtime_source(
        r#"const target = __pw.retarget(element);
  if (!target || (target.nodeName !== 'INPUT' && target.nodeName !== 'TEXTAREA' && target.nodeName !== 'SELECT'))
    throw new Error('Node is not an <input>, <textarea> or <select> element');
  return target.value;"#,
    )
}

fn visibility_source(invert: bool) -> Result<String, HostError> {
    selector_runtime_source(if invert {
        "return !element.isConnected || !__pw.visible(element);"
    } else {
        "return element.isConnected && __pw.visible(element);"
    })
}

#[cfg(feature = "element-handle-state-wait")]
fn wait_state_source(state: &str) -> Result<String, HostError> {
    match state {
        "visible" => selector_runtime_source(
            "if (!element.isConnected) throw new Error('Element is not attached to the DOM'); return __pw.visible(element);",
        ),
        "hidden" => visibility_source(true),
        "enabled" => element_state_source("enabled", false),
        "disabled" => element_state_source("enabled", true),
        "editable" => element_state_source("editable", false),
        "stable" => Ok(STABLE_WAIT_SOURCE.to_owned()),
        _ => Err(HostError::InvalidArgument(format!(
            "unsupported ElementHandle state: {state}"
        ))),
    }
}

fn element_state_source(state: &str, invert: bool) -> Result<String, HostError> {
    let state = serde_json::to_string(state)
        .map_err(|error| HostError::backend(format!("could not encode element state: {error}")))?;
    selector_runtime_source(&format!(
        r#"const result = __pw.elementState(element, {state});
  if (result.status === 'retry')
    throw new Error('Element is not attached to the DOM');
  if (result.status === 'invalid-target')
    throw new Error(result.message);
  return {invert} ? !result.value : result.value;"#
    ))
}

fn query_source(plan: &ElementHandleQueryPlan, output: QueryOutput) -> Result<String, HostError> {
    let matches = match plan {
        ElementHandleQueryPlan::Selector(selector) => {
            compile_selector(selector, "element").map_err(HostError::backend)?
        }
        ElementHandleQueryPlan::XPath(expression) => {
            let expression = serde_json::to_string(expression)
                .map_err(|error| HostError::backend(format!("could not encode XPath: {error}")))?;
            format!("__pw.xpath(element, {expression})")
        }
    };
    let install = selector_runtime_install_script();
    let result = query_result_source(output);
    let source = format!(
        r#"(element) => {{
  {install};
  const __pw = globalThis[Symbol.for({runtime_symbol:?})];
  if (!__pw || __pw.version !== {runtime_version})
    throw new Error("Pwright selector runtime is unavailable");
  const __pwMatches = {matches};
  {result}
}}"#,
        runtime_symbol = SELECTOR_RUNTIME_SYMBOL,
        runtime_version = SELECTOR_RUNTIME_VERSION,
    );
    if source.len() > evaluation::SOURCE_BYTES {
        return Err(HostError::ResourceExhausted(format!(
            "ElementHandle query source exceeds {} UTF-8 bytes",
            evaluation::SOURCE_BYTES
        )));
    }
    Ok(source)
}

fn query_result_source(output: QueryOutput) -> &'static str {
    match output {
        QueryOutput::First => "return __pwMatches.length === 0 ? null : __pwMatches[0];",
        QueryOutput::IndexedObject => {
            "const __pwResult = {}; for (let __pwIndex = 0; __pwIndex < __pwMatches.length; ++__pwIndex) __pwResult[__pwIndex] = __pwMatches[__pwIndex]; return __pwResult;"
        }
        QueryOutput::Array => "return __pwMatches;",
    }
}

async fn settle_temporary<T>(
    result: Result<T, HostError>,
    temporary: &PendingHandle,
    label: &str,
) -> Result<T, HostError> {
    let cleanup = temporary.dispose().await;
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(cleanup)) => Err(HostError::Backend(format!(
            "{label} cleanup failed: {cleanup}"
        ))),
        (Err(primary), Err(cleanup)) => Err(HostError::Backend(format!(
            "{primary}; {label} cleanup failed: {cleanup}"
        ))),
    }
}

async fn indexed_elements(
    properties: Vec<crate::handles::NamedPendingHandle>,
) -> Result<Vec<PendingHandle>, HostError> {
    let mut entries = Vec::with_capacity(properties.len());
    let mut invalid = false;
    for property in properties {
        let index = property.name().parse::<usize>().ok();
        let canonical = index
            .map(|index| property.name() == index.to_string())
            .unwrap_or(false);
        let handle = property.into_handle();
        invalid |= !canonical || !handle.is_element();
        entries.push((index.unwrap_or(usize::MAX), handle));
    }
    entries.sort_by_key(|(index, _)| *index);
    invalid |= entries
        .iter()
        .enumerate()
        .any(|(expected, (actual, _))| expected != *actual);
    if invalid {
        let mut cleanup_failures = Vec::new();
        for (_, handle) in &entries {
            if let Err(error) = handle.dispose().await {
                cleanup_failures.push(error.to_string());
            }
        }
        let primary = "Element query returned a non-element or non-index property";
        return Err(if cleanup_failures.is_empty() {
            HostError::Backend(primary.into())
        } else {
            HostError::Backend(format!(
                "{primary}; batch cleanup failed: {}",
                cleanup_failures.join("; ")
            ))
        });
    }
    Ok(entries.into_iter().map(|(_, handle)| handle).collect())
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "locator-bounding-box")]
    use super::BOUNDING_BOX_SOURCE;
    #[cfg(feature = "locator-dom-actions")]
    use super::DISPATCH_EVENT_SOURCE;
    #[cfg(feature = "touchscreen-input")]
    use super::element_tap_actionability_source;
    #[cfg(feature = "element-handle-state-wait")]
    use super::wait_state_source;
    use super::{
        ELEMENT_PREVIEW_SOURCE, FOCUS_SOURCE, GET_ATTRIBUTE_SOURCE, INNER_HTML_SOURCE,
        INNER_TEXT_SOURCE, QueryOutput, TEXT_CONTENT_SOURCE, element_state_source,
        frame_query_all_source, input_value_source, query_source, visibility_source,
    };
    #[cfg(feature = "element-handle-state-predicates")]
    use super::{ElementHandleDomRead, ElementHandleStatePredicate};
    use crate::locator_query::{QueryBudget, QueryLimits};

    fn query(source: &str) -> crate::locator_query::ElementHandleQuery {
        let budget = QueryBudget::new(QueryLimits {
            recipes: 4,
            bytes: 1024 * 1024,
        })
        .expect("budget should be valid");
        budget
            .prepare_leaf(source.len())
            .expect("query should be admitted")
            .element_handle(source.to_owned())
            .expect("query should parse")
    }

    #[test]
    fn query_selector_source_is_rooted_at_the_element() {
        // Arrange
        let (plan, _lease) = query(".child").command().expect("query should compile");

        // Act
        let source = query_source(&plan, QueryOutput::First).expect("source should compile");

        // Assert
        assert!(source.contains("__pw.css(element, \".child\")"));
        assert!(source.contains("return __pwMatches.length === 0 ? null : __pwMatches[0]"));
        assert!(!source.contains("__pw.css(document"));
    }

    #[test]
    fn frame_query_all_source_returns_an_indexed_snapshot_without_array_from() {
        // Arrange
        let selector = pwright_cap_locator::Selector::css("div".to_owned())
            .expect("CSS selector should validate");

        // Act
        let source = frame_query_all_source(&selector).expect("source should compile");

        // Assert
        assert!(source.contains("__pw.css(document, \"div\")"));
        assert!(source.contains("__pwResult[__pwIndex] = __pwMatches[__pwIndex]"));
        assert!(!source.contains("Array.from"));
    }

    #[test]
    fn query_all_xpath_source_is_rooted_and_returns_an_array() {
        // Arrange
        let (plan, _lease) = query("xpath=./div[contains(@class, 'child')]")
            .command()
            .expect("XPath should compile");

        // Act
        let source = query_source(&plan, QueryOutput::Array).expect("source should compile");

        // Assert
        assert!(source.contains("const xpath = (root, expression) =>"));
        assert!(source.contains("__pw.xpath(element"));
        assert!(source.contains("./div[contains(@class, 'child')]"));
        assert!(source.contains("value.startsWith('/')"));
        assert!(source.contains("value = '.' + value"));
        assert!(source.contains("node.nodeType === Node.ELEMENT_NODE"));
        assert!(source.contains("return __pwMatches;"));
    }

    #[test]
    fn inner_text_source_preserves_playwright_html_element_error() {
        // Arrange
        let expected = "Node is not an HTMLElement";

        // Act
        let source = INNER_TEXT_SOURCE;

        // Assert
        assert!(source.contains("instanceof HTMLElement"));
        assert!(source.contains(expected));
    }

    #[test]
    fn text_content_source_reads_the_exact_dom_owner() {
        // Arrange
        let expected = "element.textContent";

        // Act
        let source = TEXT_CONTENT_SOURCE;

        // Assert
        assert!(source.contains(expected));
    }

    #[cfg(feature = "element-handle-state-wait")]
    #[test]
    fn state_wait_sources_keep_the_retained_node_and_detachment_contract() {
        // Arrange
        let states = [
            "visible", "hidden", "enabled", "disabled", "editable", "stable",
        ];

        // Act
        let sources = states.map(|state| wait_state_source(state).expect("valid state"));

        // Assert
        for source in &sources {
            assert!(source.contains("(element) =>"));
            assert!(!source.contains("__pw.css(document"));
        }
        assert!(sources[0].contains("Element is not attached to the DOM"));
        assert!(sources[1].contains("!element.isConnected || !__pw.visible(element)"));
        assert!(sources[2].contains("__pw.elementState(element, \"enabled\")"));
        assert!(sources[3].contains("return true ? !result.value : result.value"));
        assert!(sources[4].contains("__pw.elementState(element, \"editable\")"));
        assert!(sources[5].contains("requestAnimationFrame(sample)"));
    }

    #[cfg(feature = "element-handle-state-wait")]
    #[test]
    fn state_wait_rejects_unselected_states() {
        // Arrange
        let state = "checked";

        // Act
        let outcome = wait_state_source(state);

        // Assert
        assert!(matches!(outcome, Err(crate::HostError::InvalidArgument(_))));
    }

    #[cfg(feature = "element-handle-state-predicates")]
    #[test]
    fn state_predicates_use_existing_retained_dom_observations() {
        // Arrange
        let cases = [
            ("isDisabled", ElementHandleDomRead::IsDisabled),
            ("isEditable", ElementHandleDomRead::IsEditable),
            ("isEnabled", ElementHandleDomRead::IsEnabled),
            ("isHidden", ElementHandleDomRead::IsHidden),
            ("isVisible", ElementHandleDomRead::IsVisible),
        ];

        // Act
        let mapped = cases.map(|(name, _)| {
            name.parse::<ElementHandleStatePredicate>()
                .expect("valid predicate")
                .dom_read()
        });

        // Assert
        assert_eq!(mapped, cases.map(|(_, operation)| operation));
        assert!("isChecked".parse::<ElementHandleStatePredicate>().is_err());
    }

    #[cfg(feature = "element-handle-state-wait")]
    #[test]
    fn stable_state_wait_samples_exact_node_on_two_animation_frames() {
        // Arrange
        let state = "stable";

        // Act
        let source = wait_state_source(state).expect("stable state should compile");

        // Assert
        assert!(source.contains("(element) => new Promise"));
        assert!(source.contains("if (!element.isConnected)"));
        assert!(source.contains("Element is not attached to the DOM"));
        assert!(source.contains("const rect = element.getBoundingClientRect()"));
        assert!(source.contains("current.x === previous.x && current.y === previous.y"));
        assert!(source.contains("current.width === previous.width"));
        assert!(source.contains("current.height === previous.height"));
        assert_eq!(source.matches("requestAnimationFrame(sample)").count(), 2);
        assert!(!source.contains("document.querySelector"));
    }

    #[test]
    fn preview_source_keeps_playwright_node_formatting_in_the_rust_capability() {
        // Arrange
        let expected = [
            "JSHandle@",
            "#text=",
            "trimWithEllipsis(attrs.join(''), 500)",
            "trimWithEllipsis(text, 50)",
            "attrs.sort((a, b) => a.length - b.length)",
            "name === 'style'",
        ];

        // Act
        let source = ELEMENT_PREVIEW_SOURCE;

        // Assert
        for fragment in expected {
            assert!(source.contains(fragment));
        }
        assert!(source.contains("replace(/\\n/g, '\\u21b5')"));
        assert!(source.contains("replace(/\\t/g, '\\u21c6')"));
        assert!(source.contains("const ELEMENT_NODE = 1"));
        assert!(source.contains("const TEXT_NODE = 3"));
        assert!(!source.contains("Node."));
        assert!(!source.contains("__pw.css(document"));
        assert!(!source.contains("__pw.css(element"));
    }

    #[test]
    fn focus_source_targets_the_exact_retained_element() {
        // Arrange
        let expected = "element.focus()";

        // Act
        let source = FOCUS_SOURCE;

        // Assert
        assert!(source.contains("instanceof HTMLElement"));
        assert!(source.contains(expected));
        assert!(!source.contains("querySelector"));
    }

    #[cfg(feature = "locator-bounding-box")]
    #[test]
    fn bounding_box_source_measures_the_retained_element_without_selector_requery() {
        // Arrange
        let source = BOUNDING_BOX_SOURCE;

        // Act
        let measures_retained_element = source.starts_with("element => {")
            && source.contains("element.getBoundingClientRect()")
            && source.contains("element.getClientRects()");

        // Assert
        assert!(measures_retained_element);
        assert!(!source.contains("querySelector"));
        assert!(!source.contains("__pw.css"));
        assert!(source.len() <= crate::evaluation::SOURCE_BYTES);
    }

    #[cfg(feature = "locator-dom-actions")]
    #[test]
    fn dispatch_event_source_targets_the_retained_element_without_selector_requery() {
        // Arrange
        let source = DISPATCH_EVENT_SOURCE;

        // Act
        let dispatches_retained_element = source.starts_with("(element, argument) => {")
            && source.contains("element.dispatchEvent(new EventConstructor(argument.type, init))");

        // Assert
        assert!(dispatches_retained_element);
        assert!(!source.contains("querySelector"));
        assert!(!source.contains("__pw.css"));
        assert!(source.len() <= crate::evaluation::SOURCE_BYTES);
    }

    #[cfg(feature = "touchscreen-input")]
    #[test]
    fn tap_actionability_source_keeps_exact_handle_identity_and_position() {
        // Arrange
        let options = pwright_cap_touchscreen::SelectorTapOptions {
            position: Some(
                pwright_cap_touchscreen::Position::new(2.5, 3.25)
                    .expect("finite position should build"),
            ),
            scroll: Some(pwright_cap_touchscreen::ScrollMode::None),
            ..pwright_cap_touchscreen::SelectorTapOptions::new()
        };

        // Act
        let source = element_tap_actionability_source(options).expect("source should compile");

        // Assert
        assert!(source.contains("element.isConnected"));
        assert!(source.contains("document.elementFromPoint(x, y)"));
        assert!(source.contains("{ x: 2.5, y: 3.25 }"));
        assert!(source.contains("if (false) element.scrollIntoView"));
        assert!(!source.contains("__pw.css(document"));
        assert!(!source.contains("__pw.css(element"));
    }

    #[test]
    fn property_sources_read_the_exact_retained_element() {
        // Arrange
        let expected = ["element.innerHTML", "element.getAttribute(name)"];

        // Act
        let sources = [INNER_HTML_SOURCE, GET_ATTRIBUTE_SOURCE];

        // Assert
        for (source, expected) in sources.into_iter().zip(expected) {
            assert!(source.contains(expected));
            assert!(!source.contains("querySelector"));
        }
    }

    #[test]
    fn input_value_source_reuses_runtime_retarget_and_preserves_the_error() {
        // Arrange
        let expected = "Node is not an <input>, <textarea> or <select> element";

        // Act
        let source = input_value_source().expect("source should compile");

        // Assert
        assert!(source.contains("__pw.retarget(element)"));
        assert!(source.contains(expected));
        assert!(source.contains("target.value"));
    }

    #[test]
    fn state_sources_reuse_runtime_and_keep_visibility_detachment_total() {
        // Arrange
        let state = "checked";

        // Act
        let checked = element_state_source(state, false).expect("state source should compile");
        let hidden = visibility_source(true).expect("visibility source should compile");

        // Assert
        assert!(checked.contains("__pw.elementState(element, \"checked\")"));
        assert!(
            checked.contains("Not a checkbox or radio button")
                || checked.contains("result.message")
        );
        assert!(checked.contains("Element is not attached to the DOM"));
        assert!(hidden.contains("!element.isConnected || !__pw.visible(element)"));
    }
}
