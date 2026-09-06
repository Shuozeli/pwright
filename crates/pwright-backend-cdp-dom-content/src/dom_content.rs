use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use pwright_browser_backend::NativeBackendAccess;
use pwright_cap_dom_content::{DomContentData, DomContentErrorDetails, DomContentParams};
use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_core::{
    BudgetedUtf8, FailureKind, OperationError, ProductionResourceError, ProductionResourceLedger,
    ResourceClass, ResourceError, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope, Service,
};
use serde_json::Value;

/// Fixed browser-side serializer matching Playwright's Frame.content order.
const DOM_CONTENT_SERIALIZER: &str = r#"(() => {
  let value = '';
  if (document.doctype)
    value = new XMLSerializer().serializeToString(document.doctype);
  if (document.documentElement)
    value += document.documentElement.outerHTML;
  return value;
})()"#;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpDomContentProductionProfile {
    scope: ResourceScope,
    max_content_bytes: usize,
    max_concurrent_reads: usize,
    max_context_routes: usize,
    max_native_id_bytes: usize,
    maximum_route_retained_bytes: u64,
    requirements: ResourceRequirements,
}

impl CdpDomContentProductionProfile {
    pub fn new(
        scope: ResourceScope,
        max_content_bytes: usize,
        max_concurrent_reads: usize,
    ) -> Self {
        let bytes = u64::try_from(max_content_bytes)
            .unwrap_or(u64::MAX)
            .saturating_mul(u64::try_from(max_concurrent_reads).unwrap_or(u64::MAX));
        let requirements = if max_content_bytes == 0 || max_concurrent_reads == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::DomContentBytes, bytes, bytes)
                .with_metadata(1, max_concurrent_reads)
        };
        Self {
            scope,
            max_content_bytes,
            max_concurrent_reads,
            max_context_routes: 0,
            max_native_id_bytes: 0,
            maximum_route_retained_bytes: 0,
            requirements,
        }
    }

    /// Adds persistent storage for shared Frame utility-world routes. Five
    /// native strings cover the route identity, unique context ID, and the
    /// target-local CDP client's copies. A second set covers atomic route
    /// replacement when an execution world is recreated.
    pub fn with_context_routes(
        mut self,
        max_context_routes: usize,
        max_native_id_bytes: usize,
    ) -> Self {
        self.max_context_routes = max_context_routes;
        self.max_native_id_bytes = max_native_id_bytes;
        self.maximum_route_retained_bytes = u64::try_from(max_context_routes)
            .unwrap_or(u64::MAX)
            .saturating_mul(10)
            .saturating_mul(u64::try_from(max_native_id_bytes).unwrap_or(u64::MAX));
        let content_bytes = u64::try_from(self.max_content_bytes)
            .unwrap_or(u64::MAX)
            .saturating_mul(u64::try_from(self.max_concurrent_reads).unwrap_or(u64::MAX));
        let mut requirements = ResourceRequirements::none();
        if content_bytes > 0 {
            requirements = requirements.with_class(
                ResourceClass::DomContentBytes,
                content_bytes,
                content_bytes,
            );
        }
        if self.maximum_route_retained_bytes > 0 {
            requirements = requirements.with_class(
                ResourceClass::BackendStateBytes,
                self.maximum_route_retained_bytes,
                self.maximum_route_retained_bytes,
            );
        }
        self.requirements = if content_bytes == 0 && self.maximum_route_retained_bytes == 0 {
            ResourceRequirements::none()
        } else {
            requirements.with_metadata(
                1,
                self.max_concurrent_reads
                    .saturating_add(usize::from(self.maximum_route_retained_bytes > 0)),
            )
        };
        self
    }

    pub const fn resource_scope(self) -> ResourceScope {
        self.scope
    }

    pub const fn max_content_bytes(self) -> usize {
        self.max_content_bytes
    }

    pub const fn max_concurrent_reads(self) -> usize {
        self.max_concurrent_reads
    }

    pub const fn max_context_routes(self) -> usize {
        self.max_context_routes
    }

    pub const fn max_native_id_bytes(self) -> usize {
        self.max_native_id_bytes
    }

    pub const fn maximum_route_retained_bytes(self) -> u64 {
        self.maximum_route_retained_bytes
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Clone)]
struct DomContentResources {
    ledger: ResourceLedger,
    profile: CdpDomContentProductionProfile,
    active: Arc<AtomicUsize>,
}

struct DomContentAdmission {
    lease: Option<ResourceLease>,
    active: Arc<AtomicUsize>,
    frame_active: Arc<AtomicBool>,
}

impl DomContentAdmission {
    fn take_lease(&mut self) -> ResourceLease {
        self.lease
            .take()
            .expect("DOM-content admission owns its result lease")
    }
}

impl Drop for DomContentAdmission {
    fn drop(&mut self) {
        drop(self.lease.take());
        self.frame_active.store(false, Ordering::Release);
        let previous = self.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

#[derive(Clone)]
pub struct CdpDomContentService {
    client: Arc<dyn CdpClient>,
    unique_context_id: Arc<str>,
    resources: DomContentResources,
    frame_active: Arc<AtomicBool>,
}

#[derive(Clone)]
pub struct CdpDomContentServiceFactory {
    resources: DomContentResources,
}

impl CdpDomContentServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpDomContentProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let ledger = resources.claim(profile.requirements())?;
        Ok(Self {
            resources: DomContentResources {
                ledger,
                profile,
                active: Arc::new(AtomicUsize::new(0)),
            },
        })
    }

    pub fn bind_shared_unique_context(
        &self,
        client: Arc<dyn CdpClient>,
        unique_context_id: Arc<str>,
    ) -> CdpDomContentService {
        CdpDomContentService {
            client,
            unique_context_id,
            resources: self.resources.clone(),
            frame_active: Arc::new(AtomicBool::new(false)),
        }
    }

    pub const fn profile(&self) -> CdpDomContentProductionProfile {
        self.resources.profile
    }

    pub fn reserve_context_routes(&self) -> Result<Option<ResourceLease>, ResourceError> {
        let bytes = self.resources.profile.maximum_route_retained_bytes();
        if bytes == 0 {
            Ok(None)
        } else {
            self.resources
                .ledger
                .reserve(
                    self.resources.profile.resource_scope(),
                    ResourceClass::BackendStateBytes,
                    bytes,
                )
                .map(Some)
        }
    }
}

impl CdpDomContentService {
    pub fn unique_context_id(&self) -> &str {
        &self.unique_context_id
    }

    fn reserve(&self) -> Result<DomContentAdmission, OperationError<DomContentErrorDetails>> {
        let profile = self.resources.profile;
        if profile.max_content_bytes() == 0 || profile.max_concurrent_reads() == 0 {
            return Err(resource_error(
                "DOM-content production profile has zero result or concurrency capacity",
            ));
        }
        if self
            .frame_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(resource_error(
                "a DOM-content read is already active for this Frame",
            ));
        }
        if self
            .resources
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < profile.max_concurrent_reads()).then_some(current + 1)
            })
            .is_err()
        {
            self.frame_active.store(false, Ordering::Release);
            return Err(resource_error(
                "DOM-content connection concurrency limit was reached",
            ));
        }
        let maximum = u64::try_from(profile.max_content_bytes()).unwrap_or(u64::MAX);
        match self.resources.ledger.reserve(
            profile.resource_scope(),
            ResourceClass::DomContentBytes,
            maximum,
        ) {
            Ok(lease) => Ok(DomContentAdmission {
                lease: Some(lease),
                active: Arc::clone(&self.resources.active),
                frame_active: Arc::clone(&self.frame_active),
            }),
            Err(error) => {
                self.frame_active.store(false, Ordering::Release);
                let previous = self.resources.active.fetch_sub(1, Ordering::AcqRel);
                debug_assert!(previous > 0);
                Err(map_resource_error(error))
            }
        }
    }
}

impl NativeBackendAccess<super::ChromiumCdpDomContentBackend> for CdpDomContentService {
    type Native = dyn CdpClient;

    fn native_backend(&self) -> &Self::Native {
        self.client.as_ref()
    }
}

impl Service<DomContentParams> for CdpDomContentService {
    type Response = DomContentData;
    type Error = OperationError<DomContentErrorDetails>;
    type Future<'a> =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>;

    fn call(&self, _params: DomContentParams) -> Self::Future<'_> {
        let mut admission = match self.reserve() {
            Ok(admission) => admission,
            Err(error) => return Box::pin(async move { Err(error) }),
        };
        Box::pin(async move {
            let response = self
                .client
                .runtime_evaluate_in_unique_context(
                    DOM_CONTENT_SERIALIZER,
                    self.unique_context_id.as_ref(),
                )
                .await
                .map_err(map_cdp_error)?;
            let text = take_string_result(response)?;
            if text.len() > self.resources.profile.max_content_bytes() {
                return Err(invalid_response(format!(
                    "DOM content has {} bytes, exceeding limit {}",
                    text.len(),
                    self.resources.profile.max_content_bytes()
                )));
            }
            let lease = admission.take_lease();
            let text = BudgetedUtf8::from_reserved(lease, ResourceClass::DomContentBytes, text)
                .map_err(map_resource_error)?;
            drop(admission);
            Ok(DomContentData::new(text))
        })
    }
}

fn take_string_result(
    mut response: Value,
) -> Result<String, OperationError<DomContentErrorDetails>> {
    let result = response
        .get_mut("result")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid_response("Runtime.evaluate result is not an object".to_owned()))?;
    match result.remove("value") {
        Some(Value::String(value)) => Ok(value),
        _ => Err(invalid_response(
            "Runtime.evaluate result value is not a string".to_owned(),
        )),
    }
}

fn map_cdp_error(error: CdpError) -> OperationError<DomContentErrorDetails> {
    let kind = super::classify_cdp_failure(&error);
    let message = error.to_string();
    let lowered = message.to_ascii_lowercase();
    let details = if kind == FailureKind::Backend
        && [
            "execution context was destroyed",
            "cannot find context",
            "context with specified id",
            "unique context id",
            "uniquecontextid not found",
        ]
        .iter()
        .any(|needle| lowered.contains(needle))
    {
        DomContentErrorDetails::ContentChangedDuringNavigation(
            "Unable to retrieve content because the page is navigating and changing the content."
                .to_owned(),
        )
    } else {
        DomContentErrorDetails::Backend(message.clone())
    };
    OperationError::new(kind, message, Some(details))
}

fn invalid_response(message: String) -> OperationError<DomContentErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        message.clone(),
        Some(DomContentErrorDetails::InvalidBackendResponse(message)),
    )
}

fn resource_error(message: &str) -> OperationError<DomContentErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(DomContentErrorDetails::Resource(message.to_owned())),
    )
}

fn map_resource_error(error: ResourceError) -> OperationError<DomContentErrorDetails> {
    resource_error(&error.to_string())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_core::{
        FailureKind, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceScope,
        Service,
    };
    use pwright_fake::FakeCdpClient;
    use serde_json::json;

    use super::{
        CdpDomContentProductionProfile, CdpDomContentServiceFactory, DomContentErrorDetails,
        DomContentParams,
    };

    fn profile(max_content_bytes: usize, concurrent: usize) -> CdpDomContentProductionProfile {
        CdpDomContentProductionProfile::new(
            ResourceScope::extension(92, 1),
            max_content_bytes,
            concurrent,
        )
    }

    fn resources(profile: CdpDomContentProductionProfile) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        ProductionResourceLedger::new(limits, requirements).unwrap()
    }

    #[test]
    fn shared_context_route_storage_is_explicit_and_reservable() {
        // Arrange
        let profile = profile(128, 1).with_context_routes(3, 64);
        let resources = resources(profile);
        let factory =
            CdpDomContentServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();

        // Act
        let lease = factory
            .reserve_context_routes()
            .expect("route reservation should fit")
            .expect("non-zero route storage should own one lease");

        // Assert
        assert_eq!(profile.max_context_routes(), 3);
        assert_eq!(profile.max_native_id_bytes(), 64);
        assert_eq!(profile.maximum_route_retained_bytes(), 3 * 10 * 64);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::BackendStateBytes)
                .current,
            3 * 10 * 64
        );
        drop(lease);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn reads_exact_serialized_text_in_the_bound_unique_context() {
        // Arrange
        let profile = profile(128, 1);
        let resources = resources(profile);
        let factory =
            CdpDomContentServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(json!({"result":{"value":"<!DOCTYPE html><html></html>"}}));
        let service = factory.bind_shared_unique_context(client.clone(), Arc::from("utility-7"));

        // Act
        let result = service.call(DomContentParams).await.unwrap();

        // Assert
        assert_eq!(result.as_str(), "<!DOCTYPE html><html></html>");
        assert_eq!(client.calls()[0].0, "Runtime.evaluate");
        assert_eq!(client.calls()[0].1[0]["uniqueContextId"], "utility-7");
        assert!(
            client.calls()[0].1[0]["expression"]
                .as_str()
                .unwrap()
                .contains("XMLSerializer")
        );
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            28
        );
        drop(result);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn malformed_and_oversized_results_fail_without_a_second_evaluation() {
        // Arrange
        let profile = profile(4, 1);
        let resources = resources(profile);
        let factory =
            CdpDomContentServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        let service = factory.bind_shared_unique_context(client.clone(), Arc::from("utility-7"));
        client.set_evaluate_response(json!({"result":{"value":"12345"}}));

        // Act
        let error = service.call(DomContentParams).await.unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(client.calls().len(), 1);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn navigation_context_destruction_is_typed_once_without_hidden_retry() {
        // Arrange
        let profile = profile(128, 1);
        let resources = resources(profile);
        let factory =
            CdpDomContentServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_errors(["Execution context was destroyed.".to_owned()]);
        let service = factory.bind_shared_unique_context(client.clone(), Arc::from("utility-7"));

        // Act
        let error = service.call(DomContentParams).await.unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(matches!(
            error.details(),
            Some(DomContentErrorDetails::ContentChangedDuringNavigation(_))
        ));
        assert_eq!(client.calls().len(), 1, "the adapter must not retry");
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
        client.set_evaluate_response(json!({"result":{"value":"<html></html>"}}));
        let recovered = service.call(DomContentParams).await.unwrap();
        assert_eq!(recovered.as_str(), "<html></html>");
        assert_eq!(client.calls().len(), 2);
    }

    #[tokio::test]
    async fn chromium_missing_unique_context_wording_maps_to_navigation_race() {
        // Arrange
        let profile = profile(128, 1);
        let resources = resources(profile);
        let factory =
            CdpDomContentServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_errors(["uniqueContextId not found".to_owned()]);
        let service = factory.bind_shared_unique_context(client.clone(), Arc::from("utility-7"));

        // Act
        let error = service.call(DomContentParams).await.unwrap_err();

        // Assert
        assert!(matches!(
            error.details(),
            Some(DomContentErrorDetails::ContentChangedDuringNavigation(_))
        ));
        assert_eq!(client.calls().len(), 1);
    }

    #[tokio::test]
    async fn closed_target_wording_preserves_target_closed_instead_of_navigation_race() {
        // Arrange
        let profile = profile(128, 1);
        let resources = resources(profile);
        let factory =
            CdpDomContentServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_errors(["Session with given id not found.".to_owned()]);
        let service = factory.bind_shared_unique_context(client.clone(), Arc::from("utility-7"));

        // Act
        let error = service.call(DomContentParams).await.unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::TargetClosed);
        assert!(matches!(
            error.details(),
            Some(DomContentErrorDetails::Backend(_))
        ));
        assert_eq!(client.calls().len(), 1);
        assert_eq!(
            resources
                .metrics()
                .unwrap()
                .class(ResourceClass::DomContentBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn cancelled_native_read_releases_frame_global_and_byte_admission() {
        // Arrange
        let profile = profile(64, 1);
        let resources = resources(profile);
        let factory =
            CdpDomContentServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_evaluate_gate(Some(Arc::clone(&gate)));
        let service = factory.bind_shared_unique_context(client.clone(), Arc::from("utility-7"));
        let pending_service = service.clone();
        let pending = tokio::spawn(async move { pending_service.call(DomContentParams).await });
        for _ in 0..100 {
            if !client.calls().is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
        let second = service.call(DomContentParams).await.unwrap_err();
        assert_eq!(second.kind(), FailureKind::ResourceExhausted);

        // Act
        pending.abort();
        let _ = pending.await;
        client.set_evaluate_gate(None);
        client.set_evaluate_response(json!({"result":{"value":"<html></html>"}}));
        let retry = service.call(DomContentParams).await.unwrap();

        // Assert
        assert_eq!(retry.as_str(), "<html></html>");
        assert_eq!(client.calls().len(), 2);
    }
}
