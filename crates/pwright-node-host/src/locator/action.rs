//! Action requests are portable values. Browser behavior and cleanup stay in
//! the existing Locator services and Page-owned input coordinator.

use pwright_cap_locator::{
    Blur, BlurParams, ClickParamsV2, Fill, FillParams, Focus, FocusParams, Hover, HoverParams,
    Press, PressParams, ScrollIntoViewIfNeeded, ScrollIntoViewIfNeededParams, SelectOption,
    SelectOptionParams, SetChecked, SetCheckedParams, TestIdAttributes, v2,
};
pub use pwright_cap_locator::{
    ClickOptionsV2, FillOptions, HoverOptions, KeyboardModifier, KeyboardModifiers, MouseButton,
    Position, PressKey, PressOptions, SelectOptionOptions, SelectOptionValue, SetCheckedOptions,
};
#[cfg(feature = "touchscreen-input")]
pub use pwright_cap_touchscreen::SelectorTapOptions;
#[cfg(feature = "touchscreen-input")]
use pwright_cap_touchscreen::SelectorTapParams;

use super::{HostLocator, playwright_locator_call_log};
#[cfg(feature = "handles")]
use super::{locator_context_was_replaced, remaining_timeout_ms};
use crate::HostError;
#[cfg(feature = "select-option-timeout-progress")]
use pwright_cap_locator::SelectOptionProgress;
use pwright_cap_locator::{PressSequentially, PressSequentiallyParams};
use pwright_core::{FailureKind, OperationError};

#[cfg(feature = "select-option-timeout-progress")]
fn select_option_timeout_error(
    error: HostError,
    timeout_ms: u32,
    progress: &SelectOptionProgress,
) -> HostError {
    if matches!(&error, HostError::Timeout(_)) && progress.option_not_enabled() {
        HostError::ActionTimeout {
            api: "Locator.selectOption",
            timeout_ms,
            locator: String::new(),
            progress: "option being selected is not enabled",
        }
    } else {
        error
    }
}

fn locator_action_error(
    error: OperationError<pwright_cap_locator::LocatorErrorDetails>,
    timeout_ms: u32,
) -> HostError {
    if let Some(pwright_cap_locator::LocatorErrorDetails::InvalidValue(message)) = error.details()
        && let Some(key) = message.strip_prefix("unknown key: ")
    {
        return HostError::Backend(format!("Unknown key: {key}"));
    }
    pointer_action_error(error, timeout_ms, "Locator.click", false)
}

fn pointer_action_error(
    error: OperationError<pwright_cap_locator::LocatorErrorDetails>,
    timeout_ms: u32,
    api: &'static str,
    tap: bool,
) -> HostError {
    if error.kind() == FailureKind::Timeout
        && let Some(pwright_cap_locator::LocatorErrorDetails::Backend(key)) = error.details()
    {
        let progress = match (key.as_str(), tap) {
            ("actionability:missing", true) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element was not found\n\
                 retrying tap action",
            ),
            ("actionability:detached", true) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element was detached from the DOM\n\
                 retrying tap action",
            ),
            ("actionability:not-visible", true) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element is not visible\n\
                 retrying tap action",
            ),
            ("actionability:unstable", true) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element is not stable\n\
                 retrying tap action",
            ),
            ("actionability:disabled", true) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element is not enabled\n\
                 retrying tap action",
            ),
            ("actionability:obscured", true) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element does not receive pointer events\n\
                 retrying tap action",
            ),
            ("actionability:missing", false) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element was not found\n\
                 retrying click action",
            ),
            ("actionability:detached", false) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element was detached from the DOM\n\
                 retrying click action",
            ),
            ("actionability:not-visible", false) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element is not visible\n\
                 retrying click action",
            ),
            ("actionability:unstable", false) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element is not stable\n\
                 retrying click action",
            ),
            ("actionability:disabled", false) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element is not enabled\n\
                 retrying click action",
            ),
            ("actionability:obscured", false) => Some(
                "waiting for element to be visible, enabled and stable\n\
                 element does not receive pointer events\n\
                 retrying click action",
            ),
            _ => None,
        };
        if let Some(progress) = progress {
            return HostError::ActionTimeout {
                api,
                timeout_ms,
                locator: String::new(),
                progress,
            };
        }
    }
    HostError::operation(error, timeout_ms)
}

#[cfg(feature = "touchscreen-input")]
fn tap_action_error(
    error: OperationError<pwright_cap_locator::LocatorErrorDetails>,
    timeout_ms: u32,
) -> HostError {
    pointer_action_error(error, timeout_ms, "Locator.tap", true)
}

pub const MAX_VALUE_BYTES: usize = 1024 * 1024;
pub const MAX_OPTION_BYTES: usize = 64 * 1024;
pub const MAX_OPTIONS: usize = 1024;
const MAX_STRICT_ERROR_PREVIEWS: usize = 5;
const STRICT_ERROR_PREVIEW_SOURCE: &str = r#"(elements, attributes) =>
  elements.slice(0, 5).map(element => {
    let testId = null;
    for (const name of attributes) {
      const value = element.getAttribute(name);
      if (value !== null) {
        testId = String(value).slice(0, 256);
        break;
      }
    }
    return { html: String(element.outerHTML || element.nodeName).slice(0, 512), testId };
  })"#;

#[derive(Debug, Eq, PartialEq)]
struct StrictErrorPreview {
    html: String,
    test_id: Option<String>,
}

fn strict_violation_count(error: &HostError) -> Option<u32> {
    let HostError::Backend(message) = error else {
        return None;
    };
    let message = message.strip_prefix("Backend: ").unwrap_or(message);
    let rest = message
        .strip_prefix("strict selector `")
        .or_else(|| message.strip_prefix("strict CSS selector `"))?;
    let (_, count) = rest.rsplit_once("` resolved to ")?;
    count.strip_suffix(" elements")?.parse().ok()
}

fn strict_error_previews(
    value: crate::evaluation::EvaluationValue,
) -> Result<Vec<StrictErrorPreview>, HostError> {
    let crate::evaluation::EvaluationValue::Array { items, .. } = value else {
        return Err(HostError::Backend(
            "strict-error preview returned a non-array value".into(),
        ));
    };
    if items.len() > MAX_STRICT_ERROR_PREVIEWS {
        return Err(HostError::ResourceExhausted(
            "strict-error preview exceeded its item limit".into(),
        ));
    }
    items
        .into_iter()
        .map(|item| {
            let crate::evaluation::EvaluationValue::Object { properties, .. } = item else {
                return Err(HostError::Backend(
                    "strict-error preview item was not an object".into(),
                ));
            };
            let mut html = None;
            let mut test_id = None;
            for property in properties {
                match (property.key.as_str(), property.value) {
                    ("html", crate::evaluation::EvaluationValue::String(value)) => {
                        html = Some(value)
                    }
                    ("testId", crate::evaluation::EvaluationValue::String(value)) => {
                        test_id = Some(value)
                    }
                    ("testId", crate::evaluation::EvaluationValue::Null) => {}
                    _ => {}
                }
            }
            Ok(StrictErrorPreview {
                html: html.ok_or_else(|| {
                    HostError::Backend("strict-error preview omitted HTML".into())
                })?,
                test_id,
            })
        })
        .collect()
}

#[derive(Clone)]
pub enum LocatorAction {
    PressSequentially {
        text: String,
        delay: Option<std::time::Duration>,
    },
    Click(ClickOptionsV2),
    #[cfg(feature = "touchscreen-input")]
    Tap(SelectorTapOptions),
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
    #[cfg(feature = "handles")]
    const fn api(&self) -> &'static str {
        match self {
            Self::PressSequentially { .. } => "Locator.pressSequentially",
            Self::Click(_) => "Locator.click",
            #[cfg(feature = "touchscreen-input")]
            Self::Tap(_) => "Locator.tap",
            Self::Fill { .. } => "Locator.fill",
            Self::Press { .. } => "Locator.press",
            Self::SetChecked { .. } => "Locator.setChecked",
            Self::SelectOption { .. } => "Locator.selectOption",
            Self::Hover(_) => "Locator.hover",
            Self::Focus => "Locator.focus",
            Self::Blur => "Locator.blur",
            Self::ScrollIntoViewIfNeeded => "Locator.scrollIntoViewIfNeeded",
        }
    }

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
        self.act_with_test_id_attributes(action, timeout_ms, None)
            .await
    }

    /// Execute an action and, on a strictness failure, attach a bounded DOM
    /// preview plus configured test-id suggestion. The setting remains owned
    /// by the caller's Playwright-scoped Rust registry.
    pub async fn act_with_strict_error_test_id_attribute(
        &self,
        action: LocatorAction,
        timeout_ms: u32,
        test_id_attributes: TestIdAttributes,
    ) -> Result<ActionValue, HostError> {
        self.act_with_test_id_attributes(action, timeout_ms, Some(test_id_attributes))
            .await
    }

    async fn act_with_test_id_attributes(
        &self,
        action: LocatorAction,
        timeout_ms: u32,
        test_id_attributes: Option<TestIdAttributes>,
    ) -> Result<ActionValue, HostError> {
        action.validate()?;
        #[cfg(feature = "handles")]
        if self.content_frame_owner.is_some() {
            let api = action.api();
            let locator = self.call_log()?;
            let deadline = (timeout_ms != 0).then(|| {
                tokio::time::Instant::now()
                    + std::time::Duration::from_millis(u64::from(timeout_ms))
            });
            loop {
                let remaining = match remaining_timeout_ms(deadline.as_ref(), timeout_ms) {
                    Ok(remaining) => remaining,
                    Err(HostError::Timeout(_)) => {
                        return Err(HostError::ActionTimeout {
                            api,
                            timeout_ms,
                            locator,
                            progress: "",
                        });
                    }
                    Err(error) => return Err(error),
                };
                match self
                    .act_once(action.clone(), remaining, test_id_attributes.clone())
                    .await
                {
                    Err(error) if locator_context_was_replaced(&error) => {
                        if !self.expectation_owner_is_active().await? {
                            return Err(error);
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    }
                    Err(HostError::Timeout(_)) => {
                        return Err(HostError::ActionTimeout {
                            api,
                            timeout_ms,
                            locator,
                            progress: "",
                        });
                    }
                    outcome => return outcome,
                }
            }
        }
        self.act_once(action, timeout_ms, test_id_attributes).await
    }

    async fn act_once(
        &self,
        action: LocatorAction,
        timeout_ms: u32,
        test_id_attributes: Option<TestIdAttributes>,
    ) -> Result<ActionValue, HostError> {
        // Playwright always executes tap with waitAfter disabled. Topology and
        // Page-binding publication fences can sit behind a non-committed
        // navigation whose response is intentionally held by the application,
        // so neither fence is part of tap's public completion boundary.
        let wait_for_publications = locator_action_waits_for_publication_fences(&action);
        let host = self.host()?;
        let (selector, _query_lease) = self.query.command_selector()?;
        let locator_description = self.call_log()?;
        let frame = self.resolve_frame(timeout_ms).await?.get().to_string();
        let outcome = match action {
            LocatorAction::PressSequentially { text, delay } => host
                .invoke_locator_keyboard::<PressSequentially>(
                    &frame,
                    PressSequentiallyParams::new(selector, text, delay),
                    timeout_ms,
                    HostError::operation,
                )
                .await
                .map(|()| ActionValue::Unit),
            LocatorAction::Click(options) => host
                .invoke_with_error::<v2::Click>(
                    &frame,
                    ClickParamsV2::new(selector, options),
                    timeout_ms,
                    locator_action_error,
                )
                .await
                .map(|()| ActionValue::Unit),
            #[cfg(feature = "touchscreen-input")]
            LocatorAction::Tap(options) => host
                .locator_tap(
                    &frame,
                    SelectorTapParams::new(selector, options),
                    timeout_ms,
                    tap_action_error,
                )
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
                    locator_action_error,
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
            LocatorAction::SelectOption { values, options } => {
                #[cfg(feature = "select-option-timeout-progress")]
                {
                    let progress = SelectOptionProgress::new();
                    host.invoke::<SelectOption>(
                        &frame,
                        SelectOptionParams::new(selector, values, options)
                            .with_progress(progress.clone()),
                        timeout_ms,
                    )
                    .await
                    .map_err(|error| select_option_timeout_error(error, timeout_ms, &progress))
                    .map(ActionValue::Strings)
                }
                #[cfg(not(feature = "select-option-timeout-progress"))]
                {
                    host.invoke::<SelectOption>(
                        &frame,
                        SelectOptionParams::new(selector, values, options),
                        timeout_ms,
                    )
                    .await
                    .map(ActionValue::Strings)
                }
            }
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
        let outcome = if wait_for_publications {
            crate::completion::publish_command_result(
                outcome,
                host.is_connected(),
                host.flush_topology(),
            )
            .await
        } else {
            outcome
        }
        .map_err(|error| match error {
            HostError::ActionTimeout {
                api,
                timeout_ms,
                progress,
                ..
            } => HostError::ActionTimeout {
                api,
                timeout_ms,
                locator: locator_description,
                progress,
            },
            error => error,
        });
        let outcome = match (outcome, test_id_attributes) {
            (Err(error), Some(attributes)) if strict_violation_count(&error).is_some() => {
                Err(self.enrich_strict_error(error, attributes).await)
            }
            (outcome, _) => outcome,
        };
        #[cfg(feature = "page-bindings")]
        if outcome.is_ok() && wait_for_publications {
            host.flush_page_binding_publications(&frame).await?;
        }
        outcome
    }

    async fn enrich_strict_error(
        &self,
        original: HostError,
        attributes: TestIdAttributes,
    ) -> HostError {
        let Some(count) = strict_violation_count(&original) else {
            return original;
        };
        let Ok((selector, _lease)) = self.query.command_selector() else {
            return original;
        };
        let mut message = format!(
            "strict mode violation: {} resolved to {count} elements",
            playwright_locator_call_log(&selector)
        );
        let argument = crate::evaluation::EvaluationValue::Array {
            id: None,
            items: attributes
                .names()
                .map(|name| crate::evaluation::EvaluationValue::String(name.to_owned()))
                .collect(),
        };
        let previews = self
            .evaluate_all_with(move || async move {
                Ok(crate::evaluation::EvaluateParams::function(
                    STRICT_ERROR_PREVIEW_SOURCE,
                    argument,
                ))
            })
            .await
            .and_then(strict_error_previews);
        let Ok(previews) = previews else {
            return HostError::Backend(message);
        };
        if !previews.is_empty() {
            message.push(':');
        }
        for (index, preview) in previews.into_iter().enumerate() {
            message.push_str(&format!("\n    {}) {}", index + 1, preview.html));
            if let Some(test_id) = preview.test_id {
                let quoted =
                    serde_json::to_string(&test_id).unwrap_or_else(|_| format!("{test_id:?}"));
                message.push_str(&format!(" aka get_by_test_id({quoted})"));
            }
        }
        HostError::Backend(message)
    }
}

fn locator_action_waits_for_publication_fences(_action: &LocatorAction) -> bool {
    #[cfg(feature = "touchscreen-input")]
    if matches!(_action, LocatorAction::Tap(_)) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "select-option-timeout-progress")]
    #[test]
    fn disabled_option_progress_enriches_only_its_timeout() {
        // Arrange
        let progress = SelectOptionProgress::new();
        progress.record_retry_option_not_enabled(true);

        // Act
        let error = select_option_timeout_error(HostError::Timeout(1_000), 1_000, &progress);

        // Assert
        assert!(matches!(
            &error,
            HostError::ActionTimeout {
                api: "Locator.selectOption",
                timeout_ms: 1_000,
                progress: "option being selected is not enabled",
                ..
            }
        ));
        assert!(
            error
                .to_string()
                .contains("option being selected is not enabled")
        );
    }

    #[cfg(feature = "select-option-timeout-progress")]
    #[test]
    fn unobserved_select_option_timeout_stays_generic() {
        // Arrange
        let progress = SelectOptionProgress::new();

        // Act
        let error = select_option_timeout_error(HostError::Timeout(1_000), 1_000, &progress);

        // Assert
        assert!(matches!(error, HostError::Timeout(1_000)));
    }

    #[cfg(feature = "select-option-timeout-progress")]
    #[test]
    fn backend_failure_is_never_reclassified_by_option_progress() {
        // Arrange
        let progress = SelectOptionProgress::new();
        progress.record_retry_option_not_enabled(true);

        // Act
        let error =
            select_option_timeout_error(HostError::Backend("CDP failure".into()), 1_000, &progress);

        // Assert
        assert!(matches!(error, HostError::Backend(message) if message == "CDP failure"));
    }

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

    #[test]
    fn strict_error_preview_parser_keeps_bounded_html_and_test_id() {
        // Arrange
        let value = crate::evaluation::EvaluationValue::Array {
            id: None,
            items: vec![crate::evaluation::EvaluationValue::Object {
                id: None,
                properties: vec![
                    crate::evaluation::EvaluationProperty::new(
                        "html",
                        crate::evaluation::EvaluationValue::String(
                            "<div data-custom-id=\"One\"></div>".into(),
                        ),
                    ),
                    crate::evaluation::EvaluationProperty::new(
                        "testId",
                        crate::evaluation::EvaluationValue::String("One".into()),
                    ),
                ],
            }],
        };

        // Act
        let previews = strict_error_previews(value).unwrap();

        // Assert
        assert_eq!(
            previews,
            vec![StrictErrorPreview {
                html: "<div data-custom-id=\"One\"></div>".into(),
                test_id: Some("One".into()),
            }]
        );
    }

    #[test]
    fn strict_violation_count_accepts_typed_css_backend_prefix() {
        // Arrange
        let error =
            HostError::Backend("Backend: strict CSS selector `.foo` resolved to 2 elements".into());

        // Act
        let count = strict_violation_count(&error);

        // Assert
        assert_eq!(count, Some(2));
    }

    #[test]
    fn disabled_click_progress_becomes_a_playwright_shaped_timeout() {
        // Arrange
        let error = OperationError::new(
            FailureKind::Timeout,
            "locator actionability deadline was exceeded",
            Some(pwright_cap_locator::LocatorErrorDetails::Backend(
                "actionability:disabled".to_string(),
            )),
        );

        // Act
        let mapped = locator_action_error(error, 3_000);

        // Assert
        assert!(matches!(
            mapped,
            HostError::ActionTimeout {
                api: "Locator.click",
                timeout_ms: 3_000,
                locator,
                progress
            } if locator.is_empty()
                && progress.contains("element is not enabled")
                && progress.contains("retrying click action")
        ));
    }

    #[cfg(feature = "touchscreen-input")]
    #[test]
    fn disabled_tap_progress_becomes_a_playwright_shaped_timeout() {
        // Arrange
        let error = OperationError::new(
            FailureKind::Timeout,
            "locator tap actionability deadline was exceeded",
            Some(pwright_cap_locator::LocatorErrorDetails::Backend(
                "actionability:disabled".to_string(),
            )),
        );

        // Act
        let mapped = tap_action_error(error, 3_000);

        // Assert
        assert!(matches!(
            mapped,
            HostError::ActionTimeout {
                api: "Locator.tap",
                timeout_ms: 3_000,
                locator,
                progress
            } if locator.is_empty()
                && progress.contains("element is not enabled")
                && progress.contains("retrying tap action")
        ));
    }

    #[cfg(feature = "touchscreen-input")]
    #[test]
    fn tap_completion_does_not_wait_for_publication_fences() {
        // Arrange
        let tap = LocatorAction::Tap(SelectorTapOptions::new());
        let click = LocatorAction::Click(ClickOptionsV2::new());

        // Act
        let tap_waits = locator_action_waits_for_publication_fences(&tap);
        let click_waits = locator_action_waits_for_publication_fences(&click);

        // Assert
        assert!(!tap_waits);
        assert!(click_waits);
    }

    #[test]
    fn unknown_press_key_preserves_playwright_casing_without_backend_prefix() {
        // Arrange
        let error = OperationError::new(
            FailureKind::Backend,
            "unknown key: \"NotARealKey\"",
            Some(pwright_cap_locator::LocatorErrorDetails::InvalidValue(
                "unknown key: \"NotARealKey\"".to_string(),
            )),
        );

        // Act
        let mapped = locator_action_error(error, 30_000);

        // Assert
        assert!(
            matches!(mapped, HostError::Backend(message) if message == "Unknown key: \"NotARealKey\"")
        );
    }

    #[test]
    fn click_timeout_formats_one_api_and_locator_call_log() {
        // Arrange
        let error = HostError::ActionTimeout {
            api: "Locator.click",
            timeout_ms: 42,
            locator: "get_by_role(\"button\", name=\"Hello Python\")".into(),
            progress: "element was not found",
        };

        // Act
        let message = error.to_string();

        // Assert
        assert!(message.starts_with("Locator.click: Timeout 42ms exceeded."));
        assert!(message.contains("waiting for get_by_role(\"button\", name=\"Hello Python\")"));
    }
}
