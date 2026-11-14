//! Bounded Page capture through the existing optional Rust content command.
pub(crate) mod queue;
use std::sync::atomic::{AtomicU64, Ordering};

use pwright_cap_content::PageScreenshot;
pub use pwright_cap_content::{
    ScreenshotClip, ScreenshotData, ScreenshotFormat, ScreenshotOptions, ScreenshotScale,
};
pub(crate) use queue::requirements;

use crate::{HostError, HostSession, locator::HostLocator};

// The transport already bounds a response to 4 MiB. A 2 MiB binary image's
// base64 representation fits that budget without increasing transport limits.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_CONCURRENT_CAPTURES: usize = 2;
pub const MAX_MASK_LOCATORS: usize = 64;
pub const MAX_MASK_ELEMENTS_PER_LOCATOR: usize = 256;
// Native result and Node-owned copy coexist during settlement. Admission is
// held through conversion; user-owned Buffers outlive the ticket independently.
pub const FFI_ALLOWANCE: u64 = 2 * MAX_BYTES as u64 + 4096;

static NEXT_MASK_TRANSACTION: AtomicU64 = AtomicU64::new(1);

const INSTALL_MASK_SOURCE: &str = r#"(elements, token) => {
  if (elements.length > 256)
    throw new Error('screenshot mask Locator exceeds 256 matched elements');
  const stateKey = Symbol.for('pwright.screenshot.masks.v1');
  const states = globalThis[stateKey] || (globalThis[stateKey] = new Map());
  let state = states.get(token);
  const document = elements[0]?.ownerDocument || globalThis.document;
  if (!document?.documentElement) return;
  if (!state) {
    const pane = document.createElement('x-pw-glass');
    Object.assign(pane.style, {
      position: 'fixed', top: '0', right: '0', bottom: '0', left: '0',
      zIndex: '2147483647', pointerEvents: 'none', display: 'flex',
      backgroundColor: 'transparent'
    });
    const root = pane.attachShadow({ mode: 'closed' });
    const style = document.createElement('style');
    style.textContent = 'x-pw-mask { position: absolute; display: block; background: #F0F; }';
    root.appendChild(style);
    document.documentElement.appendChild(pane);
    state = { pane, root };
    states.set(token, state);
  }
  for (const element of elements) {
    if (!element?.isConnected) continue;
    const box = element.getBoundingClientRect();
    const mask = document.createElement('x-pw-mask');
    Object.assign(mask.style, {
      left: box.x + 'px', top: box.y + 'px',
      width: box.width + 'px', height: box.height + 'px'
    });
    state.root.appendChild(mask);
  }
}"#;

const REMOVE_MASK_SOURCE: &str = r#"(_elements, token) => {
  const states = globalThis[Symbol.for('pwright.screenshot.masks.v1')];
  const state = states?.get(token);
  if (!state) return;
  state.pane.remove();
  states.delete(token);
}"#;

#[cfg(feature = "handles")]
const ELEMENT_SCREENSHOT_CLIP_SOURCE: &str = r#"element => {
  if (!(element instanceof Element) || !element.isConnected)
    throw new Error('ElementHandle is not attached to the DOM');
  if (!element.getClientRects().length)
    throw new Error('ElementHandle has no layout box');
  let box = element.getBoundingClientRect();
  const outside = box.right <= 0 || box.bottom <= 0 ||
    box.left >= innerWidth || box.top >= innerHeight;
  if (outside) {
    element.scrollIntoView({ behavior: 'instant', block: 'center', inline: 'center' });
    box = element.getBoundingClientRect();
  }
  const x = Math.floor(box.left + scrollX);
  const y = Math.floor(box.top + scrollY);
  const right = Math.ceil(box.right + scrollX);
  const bottom = Math.ceil(box.bottom + scrollY);
  if (![x, y, right, bottom].every(Number.isFinite) || x < 0 || y < 0 || right <= x || bottom <= y)
    throw new Error('ElementHandle screenshot geometry is outside the viewport clip domain');
  return { x, y, width: right - x, height: bottom - y };
}"#;

/// Bounded caller-owned result at the host boundary. The FFI caller admits its
/// result allowance before capture and retains it through Node Buffer copying.
/// The core screenshot lease is released before the next queued capture starts.
pub struct ScreenshotOutput(Vec<u8>);

impl ScreenshotOutput {
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn from_capture(data: ScreenshotData) -> Self {
        Self(data.as_bytes().to_vec())
    }
}

impl HostSession {
    pub async fn page_screenshot(
        &self,
        page: &str,
        options: ScreenshotOptions,
        timeout_ms: u32,
    ) -> Result<ScreenshotOutput, HostError> {
        self.page_screenshot_with_masks(page, options, Vec::new(), timeout_ms)
            .await
    }

    pub async fn page_screenshot_with_masks(
        &self,
        page: &str,
        options: ScreenshotOptions,
        masks: Vec<HostLocator>,
        timeout_ms: u32,
    ) -> Result<ScreenshotOutput, HostError> {
        validate(options)?;
        validate_masks(&masks)?;
        let (object, _, _operation) = self.admit_operation(page)?;
        let queue = self
            .state
            .lock()
            .map_err(HostError::backend)?
            .as_ref()
            .ok_or(HostError::TargetClosed)?
            .screenshot_queue
            .clone();
        // One deadline includes queue wait; the controlled command must not
        // restart it. Dropping that command still invokes runtime cancellation.
        queue
            .run(object, timeout_ms, &self.stop, async {
                let token = install_masks(&masks).await?;
                let capture = self
                    .invoke::<PageScreenshot>(page, options, 0)
                    .await
                    .map(ScreenshotOutput::from_capture);
                finish_masks(&masks, token.as_deref(), capture).await
            })
            .await
    }

    #[cfg(feature = "handles")]
    pub async fn element_screenshot_with_masks(
        &self,
        element: &crate::handles::PendingHandle,
        mut options: ScreenshotOptions,
        masks: Vec<HostLocator>,
        timeout_ms: u32,
    ) -> Result<ScreenshotOutput, HostError> {
        use crate::{evaluation::EvaluationValue, handles::OwnedHandleArgument};

        validate(options)?;
        validate_masks(&masks)?;
        if !element.is_element() {
            return Err(HostError::InvalidArgument(
                "ElementHandle.screenshot requires an ElementHandle".into(),
            ));
        }
        let page = element
            .source_world()
            .map(|world| world.page_id())
            .ok_or(HostError::TargetClosed)?;
        let argument = OwnedHandleArgument::from_slots(EvaluationValue::Undefined, Vec::new())?;
        let geometry = self
            .handle_evaluate_owned(element, ELEMENT_SCREENSHOT_CLIP_SOURCE.to_owned(), argument)
            .await?;
        options.full_page = None;
        options.clip = Some(clip_from_evaluation(geometry)?);
        options.document_clip = true;
        self.page_screenshot_with_masks(&page.to_string(), options, masks, timeout_ms)
            .await
    }
}

fn validate_masks(masks: &[HostLocator]) -> Result<(), HostError> {
    if masks.len() > MAX_MASK_LOCATORS {
        return Err(HostError::ResourceExhausted(format!(
            "screenshot mask list exceeds {MAX_MASK_LOCATORS} Locators"
        )));
    }
    Ok(())
}

async fn install_masks(masks: &[HostLocator]) -> Result<Option<String>, HostError> {
    if masks.is_empty() {
        return Ok(None);
    }
    let token = format!(
        "pwright-mask-{}",
        NEXT_MASK_TRANSACTION.fetch_add(1, Ordering::Relaxed)
    );
    for mask in masks {
        let argument = crate::evaluation::EvaluationValue::String(token.clone());
        if let Err(error) = mask
            .evaluate_all_with(move || async move {
                Ok(crate::evaluation::EvaluateParams::function(
                    INSTALL_MASK_SOURCE,
                    argument,
                ))
            })
            .await
        {
            let _ = remove_masks(masks, &token).await;
            return Err(error);
        }
    }
    Ok(Some(token))
}

async fn remove_masks(masks: &[HostLocator], token: &str) -> Result<(), HostError> {
    let mut first_error = None;
    for mask in masks.iter().rev() {
        let argument = crate::evaluation::EvaluationValue::String(token.to_owned());
        let result = mask
            .evaluate_all_with(move || async move {
                Ok(crate::evaluation::EvaluateParams::function(
                    REMOVE_MASK_SOURCE,
                    argument,
                ))
            })
            .await;
        if result.is_err() && first_error.is_none() {
            first_error = result.err();
        }
    }
    first_error.map_or(Ok(()), Err)
}

pub(crate) async fn finish_masks<T>(
    masks: &[HostLocator],
    token: Option<&str>,
    outcome: Result<T, HostError>,
) -> Result<T, HostError> {
    let cleanup = match token {
        Some(token) => remove_masks(masks, token).await,
        None => Ok(()),
    };
    match (outcome, cleanup) {
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
        (Ok(value), Ok(())) => Ok(value),
    }
}

#[cfg(feature = "locator-screenshot")]
pub(crate) async fn begin_masks(masks: &[HostLocator]) -> Result<Option<String>, HostError> {
    validate_masks(masks)?;
    install_masks(masks).await
}

#[cfg(feature = "handles")]
fn clip_from_evaluation(
    value: crate::evaluation::EvaluationValue,
) -> Result<ScreenshotClip, HostError> {
    let crate::evaluation::EvaluationValue::Object { properties, .. } = value else {
        return Err(HostError::Backend(
            "ElementHandle screenshot geometry returned a non-object value".into(),
        ));
    };
    let number = |name: &str| {
        properties
            .iter()
            .find(|property| property.key == name)
            .and_then(|property| match property.value {
                crate::evaluation::EvaluationValue::Number(value) => Some(value.get()),
                _ => None,
            })
            .ok_or_else(|| {
                HostError::Backend(format!(
                    "ElementHandle screenshot geometry omitted finite `{name}`"
                ))
            })
    };
    let clip = ScreenshotClip {
        x: number("x")?,
        y: number("y")?,
        width: number("width")?,
        height: number("height")?,
    };
    if [clip.x, clip.y, clip.width, clip.height]
        .into_iter()
        .any(|value| !value.is_finite())
        || clip.x < 0.0
        || clip.y < 0.0
        || clip.width <= 0.0
        || clip.height <= 0.0
    {
        return Err(HostError::Backend(
            "ElementHandle screenshot geometry is invalid".into(),
        ));
    }
    Ok(clip)
}

fn validate(options: ScreenshotOptions) -> Result<(), HostError> {
    if options.format == Some(ScreenshotFormat::Webp) {
        return Err(HostError::InvalidArgument(
            "screenshot type must be png or jpeg".into(),
        ));
    }
    if let Some(quality) = options.quality
        && (quality > 100 || options.format != Some(ScreenshotFormat::Jpeg))
    {
        return Err(HostError::InvalidArgument(
            "quality requires jpeg and an integer in [0, 100]".into(),
        ));
    }
    if let Some(clip) = options.clip {
        if [clip.x, clip.y, clip.width, clip.height]
            .into_iter()
            .any(|value| !value.is_finite())
            || clip.x < 0.0
            || clip.y < 0.0
            || clip.width <= 0.0
            || clip.height <= 0.0
        {
            return Err(HostError::InvalidArgument(
                "clip requires finite non-negative coordinates and positive dimensions".into(),
            ));
        }
        if options.full_page == Some(true) {
            return Err(HostError::UnsupportedOperation(
                "combined fullPage and clip capture".into(),
            ));
        }
    }
    if options.document_clip && options.clip.is_none() {
        return Err(HostError::InvalidArgument(
            "document-space screenshot capture requires a clip".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_handoff_releases_core_lease_before_next_capture() {
        use pwright_core::{
            BudgetedBytes, ObjectId, ResourceClass, ResourceLedger, ResourceLimits, ResourceScope,
        };
        // Arrange
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::ScreenshotBytes,
            8,
            8,
        ));
        let data = ScreenshotData::new(
            BudgetedBytes::copy_from_slice(
                &ledger,
                ResourceScope::Object(ObjectId::new(1)),
                ResourceClass::ScreenshotBytes,
                b"image",
            )
            .unwrap(),
            ScreenshotFormat::Png,
        );
        // Act
        let output = ScreenshotOutput::from_capture(data);
        // Assert
        assert_eq!(output.as_bytes(), b"image");
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[test]
    fn screenshot_accepts_jpeg_quality_endpoints() {
        // Arrange
        let options = [0, 100].map(|quality| ScreenshotOptions {
            format: Some(ScreenshotFormat::Jpeg),
            quality: Some(quality),
            ..Default::default()
        });
        // Act
        let results = options.map(validate);
        // Assert
        assert!(results.into_iter().all(|result| result.is_ok()));
    }

    #[test]
    fn screenshot_rejects_quality_on_default_png() {
        // Arrange
        let options = ScreenshotOptions {
            quality: Some(80),
            ..Default::default()
        };
        // Act
        let result = validate(options);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn screenshot_rejects_nonfinite_geometry_before_dispatch() {
        // Arrange
        let options = ScreenshotOptions {
            clip: Some(ScreenshotClip {
                x: 0.0,
                y: 0.0,
                width: f64::NAN,
                height: 10.0,
            }),
            ..Default::default()
        };
        // Act
        let result = validate(options);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }

    #[test]
    fn screenshot_combined_full_page_clip_remains_explicitly_unsupported() {
        // Arrange
        let options = ScreenshotOptions {
            full_page: Some(true),
            clip: Some(ScreenshotClip {
                x: 0.0,
                y: 0.0,
                width: 10.0,
                height: 10.0,
            }),
            ..Default::default()
        };
        // Act
        let result = validate(options);
        // Assert
        assert!(matches!(result, Err(HostError::UnsupportedOperation(_))));
    }

    #[test]
    fn mask_program_is_bounded_isolated_and_cleanup_addressable() {
        // Arrange
        let install = INSTALL_MASK_SOURCE;
        let remove = REMOVE_MASK_SOURCE;

        // Act
        let uses_declared_element_bound = install.contains("elements.length > 256");

        // Assert
        assert_eq!(MAX_MASK_ELEMENTS_PER_LOCATOR, 256);
        assert!(uses_declared_element_bound);
        assert!(install.contains("attachShadow({ mode: 'closed' })"));
        assert!(install.contains("background: #F0F"));
        assert!(install.contains("pointerEvents: 'none'"));
        assert!(remove.contains("pwright.screenshot.masks.v1"));
        assert!(remove.contains("state.pane.remove()"));
    }

    #[cfg(feature = "handles")]
    #[test]
    fn element_geometry_program_returns_document_coordinates() {
        // Arrange
        let source = ELEMENT_SCREENSHOT_CLIP_SOURCE;

        // Act
        let uses_document_x = source.contains("box.left + scrollX");
        let uses_document_y = source.contains("box.top + scrollY");

        // Assert
        assert!(uses_document_x);
        assert!(uses_document_y);
        assert!(source.contains("box.right + scrollX"));
        assert!(source.contains("box.bottom + scrollY"));
    }

    #[test]
    fn document_clip_requires_explicit_geometry() {
        // Arrange
        let options = ScreenshotOptions {
            document_clip: true,
            ..ScreenshotOptions::default()
        };

        // Act
        let result = validate(options);

        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }
}
