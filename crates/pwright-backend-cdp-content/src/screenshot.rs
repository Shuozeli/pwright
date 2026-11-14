use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use pwright_cap_content::{
    ContentErrorDetails, PageScreenshot, ScreenshotClip, ScreenshotData, ScreenshotFormat,
    ScreenshotOptions, ScreenshotScale,
};
use pwright_cdp::CdpClient;
use pwright_cdp::generated::page::{CaptureScreenshotParams, GetLayoutMetricsReturns, Viewport};
use pwright_core::{
    BudgetedBytes, CancellationReason, CommandCancellation, FailureKind, ObjectId,
    OperationContext, OperationError, ProductionResourceError, ProductionResourceLedger,
    ResourceClass, ResourceError, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope, Service,
};

use crate::classify_cdp_failure;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpScreenshotProductionProfile {
    max_screenshot_bytes: usize,
    max_concurrent_captures: usize,
    intersect_viewport_clip: bool,
    requirements: ResourceRequirements,
}

impl CdpScreenshotProductionProfile {
    pub fn new(max_screenshot_bytes: usize, max_concurrent_captures: usize) -> Self {
        let per_capture = u64::try_from(max_screenshot_bytes).unwrap_or(u64::MAX);
        let captures = u64::try_from(max_concurrent_captures).unwrap_or(u64::MAX);
        let total = per_capture.saturating_mul(captures);
        let requirements = if total == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::ScreenshotBytes, total, per_capture)
                .with_metadata(max_concurrent_captures, max_concurrent_captures)
        };
        Self {
            max_screenshot_bytes,
            max_concurrent_captures,
            intersect_viewport_clip: false,
            requirements,
        }
    }

    pub const fn max_screenshot_bytes(self) -> usize {
        self.max_screenshot_bytes
    }

    pub const fn max_concurrent_captures(self) -> usize {
        self.max_concurrent_captures
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }

    /// Opt in to Playwright's viewport intersection for non-full-page clips.
    /// The default retains the 1.0 exact-CDP-clip behavior and resource bounds.
    pub const fn with_viewport_clip_intersection(mut self) -> Self {
        self.intersect_viewport_clip = true;
        self
    }
}

#[derive(Clone)]
struct ScreenshotResources {
    ledger: ResourceLedger,
    profile: CdpScreenshotProductionProfile,
    active: Arc<AtomicUsize>,
}

#[derive(Clone)]
pub struct CdpScreenshotServiceFactory {
    resources: ScreenshotResources,
}

impl CdpScreenshotServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpScreenshotProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let ledger = resources.claim(profile.requirements())?;
        Ok(Self {
            resources: ScreenshotResources {
                ledger,
                profile,
                active: Arc::new(AtomicUsize::new(0)),
            },
        })
    }

    pub fn bind(
        &self,
        client: Arc<dyn CdpClient>,
        page_object_id: ObjectId,
    ) -> CdpScreenshotService {
        CdpScreenshotService {
            client,
            page_object_id,
            resources: self.resources.clone(),
            page_active: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[derive(Clone)]
pub struct CdpScreenshotService {
    client: Arc<dyn CdpClient>,
    page_object_id: ObjectId,
    resources: ScreenshotResources,
    page_active: Arc<AtomicBool>,
}

#[doc(hidden)]
pub struct CdpScreenshotAdmission {
    page_active: Arc<AtomicBool>,
    global_active: Arc<AtomicUsize>,
    bytes: Option<ResourceLease>,
}

impl CdpScreenshotAdmission {
    fn take_bytes(&mut self) -> ResourceLease {
        self.bytes
            .take()
            .expect("capture admission owns byte lease")
    }
}

impl Drop for CdpScreenshotAdmission {
    fn drop(&mut self) {
        self.bytes.take();
        self.page_active.store(false, Ordering::Release);
        let previous = self.global_active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

impl CdpScreenshotService {
    #[doc(hidden)]
    pub fn admit_capture(
        &self,
    ) -> Result<CdpScreenshotAdmission, OperationError<ContentErrorDetails>> {
        if self
            .page_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(resource_error(
                "a screenshot capture is already active for this Page",
            ));
        }
        let limit = self.resources.profile.max_concurrent_captures();
        if self
            .resources
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < limit).then_some(current + 1)
            })
            .is_err()
        {
            self.page_active.store(false, Ordering::Release);
            return Err(resource_error(
                "the screenshot capture concurrency limit was reached",
            ));
        }
        let maximum =
            u64::try_from(self.resources.profile.max_screenshot_bytes()).unwrap_or(u64::MAX);
        let bytes = match self.resources.ledger.reserve(
            ResourceScope::Object(self.page_object_id),
            ResourceClass::ScreenshotBytes,
            maximum,
        ) {
            Ok(bytes) => bytes,
            Err(error) => {
                self.page_active.store(false, Ordering::Release);
                let previous = self.resources.active.fetch_sub(1, Ordering::AcqRel);
                debug_assert!(previous > 0);
                return Err(map_resource_error(error));
            }
        };
        Ok(CdpScreenshotAdmission {
            page_active: Arc::clone(&self.page_active),
            global_active: Arc::clone(&self.resources.active),
            bytes: Some(bytes),
        })
    }

    #[doc(hidden)]
    pub async fn capture_admitted(
        &self,
        options: ScreenshotOptions,
        mut admission: CdpScreenshotAdmission,
    ) -> Result<ScreenshotData, OperationError<ContentErrorDetails>> {
        validate_options(options)?;
        let format = options.format.unwrap_or_default();
        let params = capture_params(
            self.client.as_ref(),
            options,
            format,
            self.resources.profile.intersect_viewport_clip,
        )
        .await?;
        let encoded = self
            .client
            .page_capture_screenshot(params)
            .await
            .map_err(map_cdp_error)?;
        let upper = decoded_length_upper_bound(&encoded).ok_or_else(|| {
            invalid_backend_response("base64 screenshot length arithmetic overflowed")
        })?;
        if upper > self.resources.profile.max_screenshot_bytes() {
            return Err(resource_error(format!(
                "encoded screenshot can decode to {upper} bytes, exceeding limit {}",
                self.resources.profile.max_screenshot_bytes()
            )));
        }
        let decoded = STANDARD.decode(encoded).map_err(|error| {
            invalid_backend_response(format!(
                "CDP returned invalid base64 screenshot data: {error}"
            ))
        })?;
        if decoded.is_empty() {
            return Err(invalid_backend_response("CDP returned an empty screenshot"));
        }
        if decoded.len() > self.resources.profile.max_screenshot_bytes() {
            return Err(resource_error(format!(
                "decoded screenshot has {} bytes, exceeding limit {}",
                decoded.len(),
                self.resources.profile.max_screenshot_bytes()
            )));
        }
        let bytes = BudgetedBytes::from_reserved(
            admission.take_bytes(),
            ResourceClass::ScreenshotBytes,
            decoded,
        )
        .map_err(map_resource_error)?;
        Ok(ScreenshotData::new(bytes, format))
    }
}

impl Service<ScreenshotOptions> for CdpScreenshotService {
    type Response = ScreenshotData;
    type Error = OperationError<ContentErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, options: ScreenshotOptions) -> Self::Future<'_> {
        let admission = match self.admit_capture() {
            Ok(admission) => admission,
            Err(error) => return Box::pin(async move { Err(error) }),
        };
        Box::pin(self.capture_admitted(options, admission))
    }
}

impl CommandCancellation<PageScreenshot> for CdpScreenshotService {
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<(), OperationError<ContentErrorDetails>>> + Send + 'a>>
    where
        Self: 'a;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async { Ok(()) })
    }
}

async fn capture_params(
    client: &dyn CdpClient,
    options: ScreenshotOptions,
    format: ScreenshotFormat,
    intersect_viewport_clip: bool,
) -> Result<CaptureScreenshotParams, OperationError<ContentErrorDetails>> {
    let needs_metrics = options.full_page.unwrap_or(false)
        || options.clip.is_some()
        || options.scale == Some(ScreenshotScale::Css);
    let (clip, capture_beyond_viewport) = if needs_metrics {
        let metrics = client
            .page_get_layout_metrics()
            .await
            .map_err(map_cdp_error)?;
        let clip = viewport_from_metrics(&metrics, options, intersect_viewport_clip)?;
        let visual = &metrics.css_visual_viewport;
        let fits_viewport = clip.x >= visual.page_x
            && clip.y >= visual.page_y
            && clip.x + clip.width <= visual.page_x + visual.client_width
            && clip.y + clip.height <= visual.page_y + visual.client_height;
        (
            Some(clip),
            options.full_page.unwrap_or(false) || !fits_viewport,
        )
    } else {
        (None, false)
    };
    let quality = match (format, options.quality) {
        (ScreenshotFormat::Png, _) => None,
        (ScreenshotFormat::Jpeg, quality) => Some(i64::from(quality.unwrap_or(80))),
        (ScreenshotFormat::Webp, quality) => Some(i64::from(quality.unwrap_or(100))),
    };
    Ok(CaptureScreenshotParams {
        format: Some(screenshot_format_for_cdp(format).to_owned()),
        quality,
        clip,
        capture_beyond_viewport: capture_beyond_viewport.then_some(true),
        ..Default::default()
    })
}

const fn screenshot_format_for_cdp(format: ScreenshotFormat) -> &'static str {
    match format {
        ScreenshotFormat::Png => "png",
        ScreenshotFormat::Jpeg => "jpeg",
        ScreenshotFormat::Webp => "webp",
    }
}

fn viewport_from_metrics(
    metrics: &GetLayoutMetricsReturns,
    options: ScreenshotOptions,
    intersect_viewport_clip: bool,
) -> Result<Viewport, OperationError<ContentErrorDetails>> {
    let (x, y, width, height, mut scale) = if options.full_page.unwrap_or(false) {
        let size = &metrics.css_content_size;
        (size.x, size.y, size.width, size.height, 1.0)
    } else if let Some(ScreenshotClip {
        x,
        y,
        width,
        height,
    }) = options.clip
    {
        let (x, y, width, height) = if options.document_clip {
            (x, y, width, height)
        } else {
            let (x, y, width, height) = if intersect_viewport_clip {
                intersect_clip(
                    x,
                    y,
                    width,
                    height,
                    metrics.css_visual_viewport.client_width,
                    metrics.css_visual_viewport.client_height,
                )?
            } else {
                (x, y, width, height)
            };
            (
                metrics.css_visual_viewport.page_x + x,
                metrics.css_visual_viewport.page_y + y,
                width,
                height,
            )
        };
        (x, y, width, height, metrics.visual_viewport.scale)
    } else {
        let viewport = &metrics.css_visual_viewport;
        (
            viewport.page_x,
            viewport.page_y,
            viewport.client_width,
            viewport.client_height,
            metrics.visual_viewport.scale,
        )
    };
    if options.scale == Some(ScreenshotScale::Css) {
        let device_scale = metrics.content_size.width / metrics.css_content_size.width;
        if !device_scale.is_finite() || device_scale <= 0.0 {
            return Err(invalid_backend_response(
                "CDP layout metrics contain an invalid device scale factor",
            ));
        }
        scale /= device_scale;
    }
    if [x, y, width, height, scale]
        .into_iter()
        .any(|value| !value.is_finite())
        || width <= 0.0
        || height <= 0.0
        || scale <= 0.0
    {
        return Err(invalid_backend_response(
            "CDP layout metrics cannot produce a valid screenshot viewport",
        ));
    }
    Ok(Viewport {
        x,
        y,
        width,
        height,
        scale,
    })
}

fn intersect_clip(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    viewport_width: f64,
    viewport_height: f64,
) -> Result<(f64, f64, f64, f64), OperationError<ContentErrorDetails>> {
    if !viewport_width.is_finite()
        || !viewport_height.is_finite()
        || viewport_width <= 0.0
        || viewport_height <= 0.0
    {
        return Err(invalid_backend_response("invalid viewport dimensions"));
    }
    let left = x.min(viewport_width);
    let top = y.min(viewport_height);
    let right = (x + width).min(viewport_width);
    let bottom = (y + height).min(viewport_height);
    if right <= left || bottom <= top {
        return Err(invalid_options(
            "Clipped area is either empty or outside the resulting image",
        ));
    }
    Ok((left, top, right - left, bottom - top))
}

fn validate_options(options: ScreenshotOptions) -> Result<(), OperationError<ContentErrorDetails>> {
    let format = options.format.unwrap_or_default();
    if options.quality.is_some() && format == ScreenshotFormat::Png {
        return Err(invalid_options(
            "screenshot quality is only valid for JPEG or WebP",
        ));
    }
    if options.full_page.unwrap_or(false) && options.clip.is_some() {
        return Err(invalid_options(
            "full-page screenshot and clip are mutually exclusive",
        ));
    }
    if options.document_clip && options.clip.is_none() {
        return Err(invalid_options(
            "document-space screenshot capture requires a clip",
        ));
    }
    if let Some(clip) = options.clip
        && (!clip.x.is_finite()
            || !clip.y.is_finite()
            || !clip.width.is_finite()
            || !clip.height.is_finite()
            || clip.x < 0.0
            || clip.y < 0.0
            || clip.width <= 0.0
            || clip.height <= 0.0)
    {
        return Err(invalid_options(
            "screenshot clip must be finite, non-negative, and have positive dimensions",
        ));
    }
    Ok(())
}

fn decoded_length_upper_bound(encoded: &str) -> Option<usize> {
    let groups = encoded.len().checked_add(3)?.checked_div(4)?;
    let padded = usize::from(encoded.ends_with('=')) + usize::from(encoded.ends_with("=="));
    groups.checked_mul(3)?.checked_sub(padded)
}

fn invalid_options(message: impl Into<String>) -> OperationError<ContentErrorDetails> {
    let message = message.into();
    OperationError::new(
        FailureKind::ProtocolViolation,
        message.clone(),
        Some(ContentErrorDetails::InvalidOptions(message)),
    )
}

fn invalid_backend_response(message: impl Into<String>) -> OperationError<ContentErrorDetails> {
    let message = message.into();
    OperationError::new(
        FailureKind::ProtocolViolation,
        message.clone(),
        Some(ContentErrorDetails::InvalidBackendResponse(message)),
    )
}

fn resource_error(message: impl Into<String>) -> OperationError<ContentErrorDetails> {
    let message = message.into();
    OperationError::new(
        FailureKind::ResourceExhausted,
        message.clone(),
        Some(ContentErrorDetails::Resource(message)),
    )
}

fn map_resource_error(error: ResourceError) -> OperationError<ContentErrorDetails> {
    resource_error(error.to_string())
}

fn map_cdp_error(error: pwright_cdp::connection::CdpError) -> OperationError<ContentErrorDetails> {
    let kind = classify_cdp_failure(&error);
    let message = error.to_string();
    OperationError::new(
        kind,
        message.clone(),
        Some(ContentErrorDetails::Backend(message)),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::task::{Context, Poll, Waker};

    use pwright_cap_content::{
        ScreenshotClip, ScreenshotFormat, ScreenshotOptions, ScreenshotScale,
    };
    use pwright_core::{
        FailureKind, ObjectId, ProductionResourceLedger, ResourceClass, ResourceLimits, Service,
    };
    use pwright_fake::FakeCdpClient;

    use super::{
        CdpScreenshotProductionProfile, CdpScreenshotServiceFactory, decoded_length_upper_bound,
    };

    #[test]
    fn opted_in_clip_intersection_trims_the_viewport_edge() {
        // Arrange
        let clip = (1200.0, 650.0, 300.0, 300.0);
        // Act
        let actual = super::intersect_clip(clip.0, clip.1, clip.2, clip.3, 1280.0, 720.0).unwrap();
        // Assert
        assert_eq!(actual, (1200.0, 650.0, 80.0, 70.0));
        assert!(!CdpScreenshotProductionProfile::new(32, 1).intersect_viewport_clip);
    }

    #[test]
    fn opted_in_clip_intersection_rejects_empty_intersections() {
        // Arrange
        let clip = (1280.0, 0.0, 50.0, 50.0);
        // Act
        let actual = super::intersect_clip(clip.0, clip.1, clip.2, clip.3, 1280.0, 720.0);
        // Assert
        assert!(actual.is_err());
    }

    #[test]
    fn decoded_bound_accounts_for_standard_base64_padding() {
        // Arrange
        let encoded = "bW9jaw==";

        // Act
        let bound = decoded_length_upper_bound(encoded);

        // Assert
        assert_eq!(bound, Some(4));
    }

    #[tokio::test]
    async fn full_page_css_screenshot_maps_metrics_and_releases_exact_result_budget() {
        // Arrange
        let profile = CdpScreenshotProductionProfile::new(32, 1);
        let requirement = profile.requirements().class(ResourceClass::ScreenshotBytes);
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 1).with_class(
                ResourceClass::ScreenshotBytes,
                requirement.global,
                requirement.per_scope,
            ),
            profile.requirements(),
        )
        .unwrap();
        let ledger = resources.claim(profile.requirements()).unwrap();
        let client = Arc::new(FakeCdpClient::from_html("<main>hello</main>"));
        let factory =
            CdpScreenshotServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind(client.clone(), ObjectId::new(9));

        // Act
        let result = service
            .call(ScreenshotOptions {
                format: Some(ScreenshotFormat::Jpeg),
                full_page: Some(true),
                scale: Some(ScreenshotScale::Css),
                ..ScreenshotOptions::default()
            })
            .await
            .unwrap();

        // Assert
        assert_eq!(result.as_bytes(), b"mock");
        assert_eq!(result.format(), ScreenshotFormat::Jpeg);
        assert_eq!(
            client.method_names(),
            vec!["Page.getLayoutMetrics", "Page.captureScreenshot"]
        );
        let capture = client
            .calls()
            .into_iter()
            .find(|(method, _)| method == "Page.captureScreenshot")
            .expect("capture call should be recorded");
        assert_eq!(capture.1[0]["format"], "jpeg");
        assert_eq!(capture.1[0]["quality"], 80);
        assert_eq!(capture.1[0]["captureBeyondViewport"], true);
        assert_eq!(capture.1[0]["clip"]["height"], 1200.0);
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            4
        );
        drop(result);
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
    async fn document_clip_bypasses_viewport_intersection_and_captures_beyond_it() {
        // Arrange
        let profile = CdpScreenshotProductionProfile::new(32, 1).with_viewport_clip_intersection();
        let requirement = profile.requirements().class(ResourceClass::ScreenshotBytes);
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 1).with_class(
                ResourceClass::ScreenshotBytes,
                requirement.global,
                requirement.per_scope,
            ),
            profile.requirements(),
        )
        .unwrap();
        let client = Arc::new(FakeCdpClient::from_html("<main>tall</main>"));
        let service =
            CdpScreenshotServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap()
                .bind(client.clone(), ObjectId::new(9));

        // Act
        let result = service
            .call(ScreenshotOptions {
                clip: Some(ScreenshotClip {
                    x: 0.0,
                    y: 0.0,
                    width: 500.0,
                    height: 1_000.0,
                }),
                document_clip: true,
                ..ScreenshotOptions::default()
            })
            .await
            .unwrap();

        // Assert
        assert_eq!(result.as_bytes(), b"mock");
        let capture = client
            .calls()
            .into_iter()
            .find(|(method, _)| method == "Page.captureScreenshot")
            .expect("capture call should be recorded");
        assert_eq!(capture.1[0]["captureBeyondViewport"], true);
        assert_eq!(capture.1[0]["clip"]["height"], 1_000.0);
    }

    #[tokio::test]
    async fn encoded_oversize_is_rejected_before_base64_decode() {
        // Arrange
        let profile = CdpScreenshotProductionProfile::new(3, 1);
        let requirement = profile.requirements().class(ResourceClass::ScreenshotBytes);
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 1).with_class(
                ResourceClass::ScreenshotBytes,
                requirement.global,
                requirement.per_scope,
            ),
            profile.requirements(),
        )
        .unwrap();
        let ledger = resources.claim(profile.requirements()).unwrap();
        let client = Arc::new(FakeCdpClient::from_html("<main>hello</main>"));
        client.set_screenshot_response("bW9jaw==");
        let service =
            CdpScreenshotServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap()
                .bind(client, ObjectId::new(9));

        // Act
        let error = service
            .call(ScreenshotOptions::default())
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
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
    async fn dropping_an_in_flight_capture_releases_page_and_byte_admission() {
        // Arrange
        let profile = CdpScreenshotProductionProfile::new(8, 1);
        let requirement = profile.requirements().class(ResourceClass::ScreenshotBytes);
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 1).with_class(
                ResourceClass::ScreenshotBytes,
                requirement.global,
                requirement.per_scope,
            ),
            profile.requirements(),
        )
        .unwrap();
        let ledger = resources.claim(profile.requirements()).unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_screenshot_gate(Some(Arc::new(tokio::sync::Notify::new())));
        let service =
            CdpScreenshotServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap()
                .bind(client.clone(), ObjectId::new(9));
        let mut first = service.call(ScreenshotOptions::default());
        let mut context = Context::from_waker(Waker::noop());

        // Act
        let first_poll = first.as_mut().poll(&mut context);
        let concurrent = service
            .call(ScreenshotOptions::default())
            .await
            .unwrap_err();
        drop(first);
        client.set_screenshot_gate(None);
        let retried = service.call(ScreenshotOptions::default()).await.unwrap();

        // Assert
        assert!(matches!(first_poll, Poll::Pending));
        assert_eq!(concurrent.kind(), FailureKind::ResourceExhausted);
        assert_eq!(retried.as_bytes(), b"mock");
        assert_eq!(
            ledger
                .metrics()
                .unwrap()
                .class(ResourceClass::ScreenshotBytes)
                .current,
            4
        );
        drop(retried);
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
