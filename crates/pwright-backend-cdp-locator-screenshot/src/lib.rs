use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::Arc;

use pwright_cap_content::ContentErrorDetails;
use pwright_cap_locator_screenshot::{
    LocatorScreenshot, LocatorScreenshotErrorDetails, LocatorScreenshotParams, ScreenshotClip,
    ScreenshotData, ScreenshotFormat, ScreenshotOptions, Selector,
};
use pwright_cdp::CdpClient;
use pwright_core::{
    CancellationReason, CommandCancellation, FailureKind, OperationContext, OperationError, Service,
};

use pwright_backend_cdp_common::classify_cdp_failure;
use pwright_backend_cdp_content::CdpScreenshotService;

/// Valid viewport geometry returned by an installed selector-resolution plugin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocatorScreenshotGeometry {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
}

impl LocatorScreenshotGeometry {
    pub fn new(
        x: f64,
        y: f64,
        width: f64,
        height: f64,
    ) -> Result<Self, LocatorScreenshotGeometryError> {
        if [x, y, width, height]
            .into_iter()
            .any(|value| !value.is_finite())
            || x < 0.0
            || y < 0.0
            || width <= 0.0
            || height <= 0.0
        {
            return Err(LocatorScreenshotGeometryError);
        }
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub const fn x(self) -> f64 {
        self.x
    }

    pub const fn y(self) -> f64 {
        self.y
    }

    pub const fn width(self) -> f64 {
        self.width
    }

    pub const fn height(self) -> f64 {
        self.height
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocatorScreenshotGeometryError;

impl std::fmt::Display for LocatorScreenshotGeometryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Locator screenshot geometry must be finite, nonnegative, and nonempty")
    }
}

impl std::error::Error for LocatorScreenshotGeometryError {}

/// Pluggable selector-to-geometry boundary used by Locator screenshot.
///
/// An implementation may use Pwright's full Locator backend or a smaller
/// selector engine. It must keep resolution lazy and return no DOM/protocol
/// handle, so the screenshot capability does not acquire public Locator
/// command authority merely to compute a clip.
pub trait LocatorScreenshotGeometryResolver: Send + Sync {
    fn resolve(
        &self,
        selector: Selector,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<
                        LocatorScreenshotGeometry,
                        OperationError<LocatorScreenshotErrorDetails>,
                    >,
                > + Send
                + '_,
        >,
    >;
}

#[derive(Clone, Default)]
pub struct CdpFrameClipTranslator {
    leaf: Option<Arc<CdpFrameClipTranslationNode>>,
}

struct CdpFrameClipTranslationNode {
    parent_client: Arc<dyn CdpClient>,
    child_frame_id: Arc<str>,
    parent: Option<Arc<Self>>,
}

impl CdpFrameClipTranslator {
    pub fn child(
        parent: &Self,
        parent_client: Arc<dyn CdpClient>,
        child_frame_id: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            leaf: Some(Arc::new(CdpFrameClipTranslationNode {
                parent_client,
                child_frame_id: child_frame_id.into(),
                parent: parent.leaf.clone(),
            })),
        }
    }

    async fn translate(
        &self,
        geometry: LocatorScreenshotGeometry,
    ) -> Result<ScreenshotClip, OperationError<LocatorScreenshotErrorDetails>> {
        let mut x = geometry.x;
        let mut y = geometry.y;
        let mut current = self.leaf.as_deref();
        while let Some(node) = current {
            let backend_node_id = node
                .parent_client
                .dom_get_frame_owner(&node.child_frame_id)
                .await
                .map_err(map_translation_cdp_error)?;
            let model = node
                .parent_client
                .dom_get_box_model_by_backend_node(backend_node_id)
                .await
                .map_err(map_translation_cdp_error)?;
            let (frame_x, frame_y) = parse_border_origin(&model)?;
            x += frame_x;
            y += frame_y;
            current = node.parent.as_deref();
        }
        if !x.is_finite() || !y.is_finite() || x < 0.0 || y < 0.0 {
            return Err(invalid_backend_response(
                "translated Locator screenshot geometry is outside the top-level viewport",
            ));
        }
        Ok(ScreenshotClip {
            x,
            y,
            width: geometry.width,
            height: geometry.height,
        })
    }
}

/// One Page/Frame-scoped Locator capture service.
///
/// The screenshot admission lease is acquired before selector resolution and
/// remains live through stability, optional scroll, final geometry, CDP
/// capture, decode, and exact-byte shrink. No DOM handle escapes the call.
#[derive(Clone)]
pub struct CdpLocatorScreenshotService {
    resolver: Arc<dyn LocatorScreenshotGeometryResolver>,
    screenshot: CdpScreenshotService,
    clip_translator: CdpFrameClipTranslator,
}

impl CdpLocatorScreenshotService {
    pub fn new(
        resolver: Arc<dyn LocatorScreenshotGeometryResolver>,
        screenshot: CdpScreenshotService,
    ) -> Self {
        Self {
            resolver,
            screenshot,
            clip_translator: CdpFrameClipTranslator { leaf: None },
        }
    }

    pub fn with_clip_translator(mut self, clip_translator: CdpFrameClipTranslator) -> Self {
        self.clip_translator = clip_translator;
        self
    }
}

impl Service<LocatorScreenshotParams> for CdpLocatorScreenshotService {
    type Response = ScreenshotData;
    type Error = OperationError<LocatorScreenshotErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: LocatorScreenshotParams) -> Self::Future<'_> {
        if let Err(error) = validate_options(params.options) {
            return Box::pin(async move { Err(error) });
        }
        let admission = match self.screenshot.admit_capture() {
            Ok(admission) => admission,
            Err(error) => return Box::pin(async move { Err(map_content_error(error)) }),
        };
        Box::pin(async move {
            let geometry = self.resolver.resolve(params.selector).await?;
            let clip = self.clip_translator.translate(geometry).await?;
            let capture_options = ScreenshotOptions {
                format: params.options.format,
                quality: params.options.quality,
                full_page: None,
                clip: Some(clip),
                scale: params.options.scale,
            };
            self.screenshot
                .capture_admitted(capture_options, admission)
                .await
                .map_err(map_content_error)
        })
    }
}

fn parse_border_origin(
    response: &serde_json::Value,
) -> Result<(f64, f64), OperationError<LocatorScreenshotErrorDetails>> {
    let border = response
        .get("model")
        .and_then(|model| model.get("border"))
        .and_then(serde_json::Value::as_array)
        .filter(|border| border.len() == 8)
        .ok_or_else(|| {
            invalid_backend_response("DOM.getBoxModel is missing an eight-number border quad")
        })?;
    let coordinate = |index: usize| {
        border[index]
            .as_f64()
            .filter(|coordinate| coordinate.is_finite())
            .ok_or_else(|| {
                invalid_backend_response("DOM.getBoxModel returned a non-finite border quad")
            })
    };
    let x = coordinate(0)?
        .min(coordinate(2)?)
        .min(coordinate(4)?)
        .min(coordinate(6)?);
    let y = coordinate(1)?
        .min(coordinate(3)?)
        .min(coordinate(5)?)
        .min(coordinate(7)?);
    Ok((x, y))
}

fn map_translation_cdp_error(
    error: pwright_cdp::connection::CdpError,
) -> OperationError<LocatorScreenshotErrorDetails> {
    let kind = classify_cdp_failure(&error);
    let message = error.to_string();
    OperationError::new(
        kind,
        message.clone(),
        Some(LocatorScreenshotErrorDetails::Backend(message)),
    )
}

fn invalid_backend_response(
    message: impl Into<String>,
) -> OperationError<LocatorScreenshotErrorDetails> {
    let message = message.into();
    OperationError::new(
        FailureKind::ProtocolViolation,
        message.clone(),
        Some(LocatorScreenshotErrorDetails::InvalidBackendResponse(
            message,
        )),
    )
}

impl CommandCancellation<LocatorScreenshot> for CdpLocatorScreenshotService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorScreenshotErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // All selector and capture state is call-scoped RAII state. Chromium
        // cannot cancel an already-dispatched capture, but no late result is
        // retained or published after the owning future is dropped.
        ready(Ok(()))
    }
}

fn validate_options(
    options: ScreenshotOptions,
) -> Result<(), OperationError<LocatorScreenshotErrorDetails>> {
    if options.full_page.is_some() {
        return Err(invalid_options(
            "full_page is not valid for Locator screenshots",
        ));
    }
    if options.clip.is_some() {
        return Err(invalid_options("clip is not valid for Locator screenshots"));
    }
    if options.quality.is_some() && options.format.unwrap_or_default() == ScreenshotFormat::Png {
        return Err(invalid_options(
            "screenshot quality is only valid for JPEG or WebP",
        ));
    }
    Ok(())
}

fn invalid_options(message: impl Into<String>) -> OperationError<LocatorScreenshotErrorDetails> {
    let message = message.into();
    OperationError::new(
        FailureKind::ProtocolViolation,
        message.clone(),
        Some(LocatorScreenshotErrorDetails::InvalidOptions(message)),
    )
}

fn map_content_error(
    error: OperationError<ContentErrorDetails>,
) -> OperationError<LocatorScreenshotErrorDetails> {
    error.map_details(|details| match details {
        ContentErrorDetails::InvalidOptions(message) => {
            LocatorScreenshotErrorDetails::InvalidOptions(message)
        }
        ContentErrorDetails::InvalidBackendResponse(message) => {
            LocatorScreenshotErrorDetails::InvalidBackendResponse(message)
        }
        ContentErrorDetails::Resource(message) => LocatorScreenshotErrorDetails::Resource(message),
        ContentErrorDetails::Backend(message) => LocatorScreenshotErrorDetails::Backend(message),
    })
}

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};

    use pwright_backend_cdp_content::{
        CdpScreenshotProductionProfile, CdpScreenshotServiceFactory,
    };
    use pwright_cap_locator_screenshot::{
        LocatorScreenshotErrorDetails, LocatorScreenshotParams, ScreenshotOptions, Selector,
    };
    use pwright_cdp::CdpClient;
    use pwright_core::{
        FailureKind, ObjectId, OperationError, ProductionResourceLedger, ResourceClass,
        ResourceLimits, Service,
    };
    use pwright_fake::FakeCdpClient;
    use serde_json::json;

    use super::{
        CdpFrameClipTranslator, CdpLocatorScreenshotService, LocatorScreenshotGeometry,
        LocatorScreenshotGeometryResolver,
    };

    #[derive(Clone)]
    struct TestResolver {
        client: Arc<FakeCdpClient>,
        runtime_ready: Arc<tokio::sync::OnceCell<()>>,
    }

    impl LocatorScreenshotGeometryResolver for TestResolver {
        fn resolve(
            &self,
            selector: Selector,
        ) -> Pin<
            Box<
                dyn Future<
                        Output = Result<
                            LocatorScreenshotGeometry,
                            OperationError<LocatorScreenshotErrorDetails>,
                        >,
                    > + Send
                    + '_,
            >,
        > {
            Box::pin(async move {
                self.runtime_ready
                    .get_or_try_init(|| async {
                        self.client
                            .runtime_evaluate("install selector runtime")
                            .await
                            .map(|_| ())
                            .map_err(test_backend_error)
                    })
                    .await?;
                loop {
                    let response = self
                        .client
                        .runtime_evaluate("getBoundingClientRect")
                        .await
                        .map_err(test_backend_error)?;
                    let value = response["result"]["value"]
                        .as_object()
                        .expect("test response");
                    match value["status"].as_str().expect("test status") {
                        "done" if value.contains_key("x") => {
                            return LocatorScreenshotGeometry::new(
                                value["x"].as_f64().expect("x"),
                                value["y"].as_f64().expect("y"),
                                value["width"].as_f64().expect("width"),
                                value["height"].as_f64().expect("height"),
                            )
                            .map_err(|error| test_protocol_error(error.to_string()));
                        }
                        "done" => continue,
                        "ready-to-scroll" => {
                            self.client
                                .runtime_evaluate("scrollIntoView")
                                .await
                                .map_err(test_backend_error)?;
                        }
                        "strict" => {
                            let matches = u32::try_from(value["count"].as_u64().expect("count"))
                                .expect("test count");
                            return Err(OperationError::new(
                                FailureKind::Backend,
                                "strict selector",
                                Some(LocatorScreenshotErrorDetails::StrictViolation {
                                    selector: selector.to_string(),
                                    matches,
                                }),
                            ));
                        }
                        status => panic!("unexpected test resolver status {status}"),
                    }
                }
            })
        }
    }

    fn test_backend_error(
        error: pwright_cdp::connection::CdpError,
    ) -> OperationError<LocatorScreenshotErrorDetails> {
        OperationError::new(
            FailureKind::Backend,
            error.to_string(),
            Some(LocatorScreenshotErrorDetails::Backend(error.to_string())),
        )
    }

    fn test_protocol_error(message: String) -> OperationError<LocatorScreenshotErrorDetails> {
        OperationError::new(
            FailureKind::ProtocolViolation,
            message.clone(),
            Some(LocatorScreenshotErrorDetails::InvalidBackendResponse(
                message,
            )),
        )
    }

    fn services(
        client: Arc<FakeCdpClient>,
    ) -> (CdpLocatorScreenshotService, pwright_core::ResourceLedger) {
        let screenshot_profile = CdpScreenshotProductionProfile::new(32, 1);
        let screenshot_limit = screenshot_profile
            .requirements()
            .class(ResourceClass::ScreenshotBytes);
        let requirements = screenshot_profile.requirements();
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
                ResourceClass::ScreenshotBytes,
                screenshot_limit.global,
                screenshot_limit.per_scope,
            ),
            requirements,
        )
        .expect("combined resource plan should validate");
        let ledger = resources
            .claim(screenshot_profile.requirements())
            .expect("screenshot claim should exist");
        let screenshot_client: Arc<dyn CdpClient> = client.clone();
        let screenshot = CdpScreenshotServiceFactory::new_with_production_resources(
            screenshot_profile,
            &resources,
        )
        .expect("screenshot factory should build")
        .bind(screenshot_client, ObjectId::new(3));
        (
            CdpLocatorScreenshotService::new(
                Arc::new(TestResolver {
                    client,
                    runtime_ready: Arc::new(tokio::sync::OnceCell::new()),
                }),
                screenshot,
            ),
            ledger,
        )
    }

    #[test]
    fn geometry_constructor_rejects_untrusted_invalid_clips() {
        // Arrange
        let invalid = [
            (f64::NAN, 0.0, 1.0, 1.0),
            (-1.0, 0.0, 1.0, 1.0),
            (0.0, 0.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, f64::INFINITY),
        ];

        // Act
        let results = invalid
            .map(|(x, y, width, height)| LocatorScreenshotGeometry::new(x, y, width, height));

        // Assert
        assert!(results.into_iter().all(|result| result.is_err()));
    }

    #[tokio::test]
    async fn visible_element_geometry_becomes_the_capture_clip() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Save</button>"));
        client.set_evaluate_responses([
            json!({"result": {"value": true}}),
            json!({"result": {"value": {"status": "done", "x": 10.0, "y": 20.0, "width": 31.0, "height": 42.0}}}),
        ]);
        let (service, ledger) = services(client.clone());

        // Act
        let screenshot = service
            .call(LocatorScreenshotParams::new(
                Selector::css("button").expect("selector should validate"),
                ScreenshotOptions::default(),
            ))
            .await
            .expect("capture should succeed");

        // Assert
        assert_eq!(screenshot.as_bytes(), b"mock");
        let capture = client
            .calls()
            .into_iter()
            .find(|(method, _)| method == "Page.captureScreenshot")
            .expect("capture should be dispatched");
        assert_eq!(capture.1[0]["clip"]["x"], 10.0);
        assert_eq!(capture.1[0]["clip"]["y"], 20.0);
        assert_eq!(capture.1[0]["clip"]["width"], 31.0);
        assert_eq!(capture.1[0]["clip"]["height"], 42.0);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            4
        );
        drop(screenshot);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn below_fold_element_scrolls_once_then_revalidates_geometry() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div id='below'></div>"));
        client.set_evaluate_responses([
            json!({"result": {"value": true}}),
            json!({"result": {"value": {"status": "ready-to-scroll"}}}),
            json!({"result": {"value": {"status": "done"}}}),
            json!({"result": {"value": {"status": "done", "x": 2.0, "y": 3.0, "width": 40.0, "height": 50.0}}}),
        ]);
        let (service, _) = services(client.clone());

        // Act
        service
            .call(LocatorScreenshotParams::new(
                Selector::css("#below").expect("selector should validate"),
                ScreenshotOptions::default(),
            ))
            .await
            .expect("capture should succeed");

        // Assert
        let evaluations: Vec<_> = client
            .calls()
            .into_iter()
            .filter(|(method, _)| method == "Runtime.evaluate")
            .collect();
        assert_eq!(evaluations.len(), 4);
        assert!(
            evaluations[2].1[0]
                .as_str()
                .unwrap()
                .contains("scrollIntoView")
        );
        assert!(
            evaluations[3].1[0]
                .as_str()
                .unwrap()
                .contains("getBoundingClientRect")
        );
    }

    #[tokio::test]
    async fn strict_failure_releases_capture_admission_for_retry() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<button></button><button></button>",
        ));
        client.set_evaluate_responses([
            json!({"result": {"value": true}}),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let (service, ledger) = services(client.clone());

        // Act
        let error = service
            .call(LocatorScreenshotParams::new(
                Selector::css("button").expect("selector should validate"),
                ScreenshotOptions::default(),
            ))
            .await
            .expect_err("strict resolution should fail");
        client.set_evaluate_responses([json!({"result": {"value": {"status": "done", "x": 1.0, "y": 1.0, "width": 10.0, "height": 10.0}}})]);
        let retry = service
            .call(LocatorScreenshotParams::new(
                Selector::css("#one").expect("selector should validate"),
                ScreenshotOptions::default(),
            ))
            .await
            .expect("released admission should allow retry");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(matches!(
            error.details(),
            Some(LocatorScreenshotErrorDetails::StrictViolation { matches: 2, .. })
        ));
        assert_eq!(retry.as_bytes(), b"mock");
        drop(retry);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn dropping_during_resolution_releases_capture_admission() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Save</button>"));
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_evaluate_gate(Some(gate));
        let (service, ledger) = services(client.clone());
        let mut pending = service.call(LocatorScreenshotParams::new(
            Selector::css("button").expect("selector should validate"),
            ScreenshotOptions::default(),
        ));
        let mut context = Context::from_waker(Waker::noop());

        // Act
        assert!(matches!(pending.as_mut().poll(&mut context), Poll::Pending));
        drop(pending);
        client.set_evaluate_gate(None);
        client.set_evaluate_responses([
            json!({"result": {"value": true}}),
            json!({"result": {"value": {"status": "done", "x": 1.0, "y": 1.0, "width": 10.0, "height": 10.0}}}),
        ]);
        let retry = service
            .call(LocatorScreenshotParams::new(
                Selector::css("button").expect("selector should validate"),
                ScreenshotOptions::default(),
            ))
            .await
            .expect("drop should release all admission");

        // Assert
        assert_eq!(retry.as_bytes(), b"mock");
        drop(retry);
        let metrics = ledger.metrics().unwrap();
        assert_eq!(metrics.class(ResourceClass::ScreenshotBytes).current, 0);
    }

    #[tokio::test]
    async fn malformed_frame_owner_geometry_releases_capture_admission_for_retry() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Save</button>"));
        client.set_evaluate_responses([
            json!({"result": {"value": true}}),
            json!({"result": {"value": {"status": "done", "x": 1.0, "y": 2.0, "width": 10.0, "height": 11.0}}}),
        ]);
        client.set_frame_owner("child", 91);
        client.set_backend_box_model(91, json!({"model": {"border": [1.0, 2.0]}}));
        let (service, ledger) = services(client.clone());
        let translator = CdpFrameClipTranslator::child(
            &CdpFrameClipTranslator::default(),
            client.clone(),
            "child",
        );
        let service = service.with_clip_translator(translator);

        // Act
        let error = service
            .call(LocatorScreenshotParams::new(
                Selector::css("button").expect("selector should validate"),
                ScreenshotOptions::default(),
            ))
            .await
            .expect_err("malformed owner geometry should fail closed");
        client.set_backend_box_model(
            91,
            json!({
                "model": {
                    "border": [20.0, 30.0, 40.0, 30.0, 40.0, 50.0, 20.0, 50.0]
                }
            }),
        );
        client.set_evaluate_responses([json!({"result": {"value": {"status": "done", "x": 1.0, "y": 2.0, "width": 10.0, "height": 11.0}}})]);
        let retry = service
            .call(LocatorScreenshotParams::new(
                Selector::css("button").expect("selector should validate"),
                ScreenshotOptions::default(),
            ))
            .await
            .expect("failed translation should release capture admission");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(retry.as_bytes(), b"mock");
        drop(retry);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            0
        );
    }
}
