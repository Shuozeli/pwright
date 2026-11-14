//! Rust-owned Locator drag composition over bounded geometry and Mouse leaves.

use super::{HostLocator, LocatorBoundingBox};
use crate::{HostError, mouse};
use pwright_core::ObjectId;
use std::sync::Arc;

const ACTION_BOX_SOURCE: &str = r#"element => {
  if (!element.isConnected) return null;
  element.scrollIntoView({ block: 'center', inline: 'center', behavior: 'instant' });
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragPosition {
    x: mouse::MouseNumber,
    y: mouse::MouseNumber,
}

impl DragPosition {
    pub fn new(x: f64, y: f64) -> Result<Self, HostError> {
        Ok(Self {
            x: mouse::MouseNumber::try_from(x)
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?,
            y: mouse::MouseNumber::try_from(y)
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DragToOptions {
    source_position: Option<DragPosition>,
    target_position: Option<DragPosition>,
    steps: mouse::MouseMoveSteps,
    trial: bool,
}

impl DragToOptions {
    pub fn new(
        source_position: Option<DragPosition>,
        target_position: Option<DragPosition>,
        steps: f64,
        trial: bool,
    ) -> Result<Self, HostError> {
        Ok(Self {
            source_position,
            target_position,
            steps: mouse::MouseMoveSteps::try_from(steps)
                .map_err(|error| HostError::InvalidArgument(error.to_string()))?,
            trial,
        })
    }
}

fn action_box(value: crate::evaluation::EvaluationValue) -> Result<LocatorBoundingBox, HostError> {
    super::bounding_box_result(value)?
        .ok_or_else(|| HostError::Backend("Element is not visible".into()))
}

fn action_point(
    bounds: LocatorBoundingBox,
    position: Option<DragPosition>,
) -> Result<(mouse::MouseNumber, mouse::MouseNumber), HostError> {
    let (x, y) = position.map_or(
        (
            bounds.x + bounds.width / 2.0,
            bounds.y + bounds.height / 2.0,
        ),
        |position| (bounds.x + position.x.get(), bounds.y + position.y.get()),
    );
    Ok((
        mouse::MouseNumber::try_from(x)
            .map_err(|error| HostError::InvalidArgument(error.to_string()))?,
        mouse::MouseNumber::try_from(y)
            .map_err(|error| HostError::InvalidArgument(error.to_string()))?,
    ))
}

impl HostLocator {
    async fn drag_action_point(
        &self,
        timeout_ms: u32,
        position: Option<DragPosition>,
    ) -> Result<(mouse::MouseNumber, mouse::MouseNumber), HostError> {
        let bounds = self
            .evaluate_with(timeout_ms, || async {
                Ok(crate::evaluation::EvaluateParams::function_without_argument(ACTION_BOX_SOURCE))
            })
            .await
            .and_then(action_box)?;
        action_point(bounds, position)
    }

    async fn drag_page(
        &self,
        target: &Self,
        page_id: &str,
    ) -> Result<(Arc<crate::HostSession>, String), HostError> {
        let host = self.host()?;
        let target_host = target.host()?;
        if !Arc::ptr_eq(&host, &target_host) {
            return Err(HostError::InvalidArgument(
                "Drag source and target must belong to the same browser session".into(),
            ));
        }
        if !self.same_frame_root(target) {
            return Err(HostError::InvalidArgument(
                "Drag source and target must belong to the same Frame".into(),
            ));
        }
        let frame = self.resolve_frame(30_000).await?;
        let target_frame = target.resolve_frame(30_000).await?;
        if frame != target_frame {
            return Err(HostError::InvalidArgument(
                "Drag source and target resolved to different Frames".into(),
            ));
        }
        let page = page_id
            .parse::<u64>()
            .ok()
            .filter(|value| *value != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Page ID".into()))?;
        {
            let state = host.state.lock().map_err(HostError::backend)?;
            let live = state.as_ref().ok_or(HostError::TargetClosed)?;
            let topology = live.composition.topology().map_err(HostError::backend)?;
            if topology.topology().page_object_id_for_frame(frame) != Some(page) {
                return Err(HostError::InvalidArgument(
                    "Drag Frame does not belong to the supplied Page".into(),
                ));
            }
        }
        Ok((host, page.get().to_string()))
    }

    pub async fn drag_to(
        &self,
        page_id: &str,
        target: &Self,
        timeout_ms: u32,
        options: DragToOptions,
    ) -> Result<(), HostError> {
        let (host, page_id) = self.drag_page(target, page_id).await?;
        let source = self
            .drag_action_point(timeout_ms, options.source_position)
            .await?;
        let target = target
            .drag_action_point(timeout_ms, options.target_position)
            .await?;
        if options.trial {
            return Ok(());
        }
        let start = host.admit_input_start()?;
        start
            .run(async move {
                host.mouse_move(
                    &page_id,
                    mouse::MoveParams {
                        x: source.0,
                        y: source.1,
                        steps: mouse::MouseMoveSteps::ONE,
                    },
                )
                .await?;
                host.mouse_down(
                    &page_id,
                    mouse::DownParams {
                        button: mouse::MouseButton::Left,
                        click_count: 1,
                    },
                )
                .await?;
                let movement = host
                    .mouse_move(
                        &page_id,
                        mouse::MoveParams {
                            x: target.0,
                            y: target.1,
                            steps: options.steps,
                        },
                    )
                    .await;
                let release = host
                    .mouse_up(
                        &page_id,
                        mouse::UpParams {
                            button: mouse::MouseButton::Left,
                            click_count: 1,
                        },
                    )
                    .await;
                movement.and(release)
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bounds() -> LocatorBoundingBox {
        LocatorBoundingBox {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 80.0,
        }
    }

    #[test]
    fn drag_point_defaults_to_element_center() {
        // Arrange
        let bounds = bounds();

        // Act
        let point = action_point(bounds, None).unwrap();

        // Assert
        assert_eq!((point.0.get(), point.1.get()), (60.0, 60.0));
    }

    #[test]
    fn drag_point_preserves_position_relative_to_top_left() {
        // Arrange
        let bounds = bounds();
        let position = DragPosition::new(34.0, 7.0).unwrap();

        // Act
        let point = action_point(bounds, Some(position)).unwrap();

        // Assert
        assert_eq!((point.0.get(), point.1.get()), (44.0, 27.0));
    }

    #[test]
    fn drag_options_reject_fractional_or_excessive_event_work() {
        // Arrange
        let fractional = 1.5;
        let excessive = f64::from(pwright_input_model::INPUT_MOUSE_MAX_STEPS) + 1.0;

        // Act
        let fractional_result = DragToOptions::new(None, None, fractional, false);
        let excessive_result = DragToOptions::new(None, None, excessive, false);

        // Assert
        assert!(fractional_result.is_err());
        assert!(excessive_result.is_err());
    }
}
