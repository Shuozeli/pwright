//! Chromium-only temporary ElementHandle adoption between execution contexts.

use std::fmt::{self, Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use pwright_cap_handle_evaluation::HandleEvaluationErrorDetails;
use pwright_cdp::CdpClient;
use pwright_cdp::connection::CdpError;
use pwright_core::{
    FailureKind, OperationError, ProductionResourceError, ProductionResourceLedger, ResourceClass,
    ResourceError, ResourceLease, ResourceRequest, ResourceRequirements, ResourceScope, Service,
};
use serde_json::Value;

use crate::handle_evaluation::{
    CdpEvaluatedHandle, CdpHandleEvaluationLimits, CdpHandleKind, invalid_response, invalid_value,
    map_cdp_error, parse_response,
};

/// One retained ElementHandle plus the exact Chromium execution context into
/// which a temporary wrapper should be materialized.
pub struct CdpElementHandleAdoptionRequest {
    source: CdpEvaluatedHandle,
    target_client: Arc<dyn CdpClient>,
    target_execution_context_id: i64,
    object_group: Option<Arc<str>>,
}

impl CdpElementHandleAdoptionRequest {
    pub const fn new(
        source: CdpEvaluatedHandle,
        target_client: Arc<dyn CdpClient>,
        target_execution_context_id: i64,
    ) -> Self {
        Self {
            source,
            target_client,
            target_execution_context_id,
            object_group: None,
        }
    }

    pub const fn source(&self) -> &CdpEvaluatedHandle {
        &self.source
    }

    /// Borrows an independently owned native element. This creates no native
    /// object and transfers no source ownership; the caller retains that owner
    /// until adoption and its cleanup complete.
    pub fn from_element_reference(
        source_client: Arc<dyn CdpClient>,
        object_id: &str,
        target_client: Arc<dyn CdpClient>,
        target_execution_context_id: i64,
        max_native_bytes: usize,
    ) -> Result<Self, OperationError<HandleEvaluationErrorDetails>> {
        Ok(Self::new(
            CdpEvaluatedHandle::element_reference(source_client, object_id, max_native_bytes)?,
            target_client,
            target_execution_context_id,
        ))
    }

    /// Opt-in compensation for unparseable/lost native responses. The external
    /// group owner must outlive this request and release the whole group.
    pub fn with_object_group(mut self, group: Arc<str>) -> Self {
        self.object_group = Some(group);
        self
    }

    pub const fn target_execution_context_id(&self) -> i64 {
        self.target_execution_context_id
    }
}

/// Temporary native ElementHandle ownership created only for one target-world
/// operation. It never represents a portable Pwright object and must be
/// explicitly released by the composing transaction.
#[must_use = "a temporary adopted ElementHandle must be released after its operation"]
pub struct CdpAdoptedElementHandle {
    handle: CdpEvaluatedHandle,
}

impl Debug for CdpAdoptedElementHandle {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CdpAdoptedElementHandle")
            .field("handle", &self.handle)
            .finish()
    }
}

impl CdpAdoptedElementHandle {
    pub const fn handle(&self) -> &CdpEvaluatedHandle {
        &self.handle
    }

    pub async fn release(self) -> Result<(), CdpError> {
        self.handle.release().await
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CdpElementHandleAdoptionService {
    limits: CdpHandleEvaluationLimits,
}

impl CdpElementHandleAdoptionService {
    pub const fn new(limits: CdpHandleEvaluationLimits) -> Self {
        Self { limits }
    }

    pub const fn limits(self) -> CdpHandleEvaluationLimits {
        self.limits
    }

    pub fn validate_request(
        &self,
        request: &CdpElementHandleAdoptionRequest,
    ) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
        if request.source.native_argument_bytes() > self.limits.native_id_bytes()
            || request.object_group.as_ref().is_some_and(|group| {
                group.is_empty() || group.len() > self.limits.native_id_bytes()
            })
        {
            return Err(invalid_value(
                "native element or group exceeds the adoption bound".into(),
            ));
        }
        if request.target_execution_context_id <= 0 {
            return Err(invalid_value(
                "target Chromium execution-context ID must be positive".to_owned(),
            ));
        }
        if request.source.kind() != CdpHandleKind::Element {
            return Err(invalid_value(
                "only ElementHandle values can be adopted between execution contexts".to_owned(),
            ));
        }
        if request.source.object_id().is_none() {
            return Err(invalid_response(
                "retained ElementHandle has no native CDP object ID",
            ));
        }
        if !request.source.belongs_to_client(&request.target_client) {
            return Err(invalid_value(
                "ElementHandle adoption cannot cross a CDP target session".to_owned(),
            ));
        }
        Ok(())
    }
}

type CdpElementHandleAdoptionFuture = Pin<
    Box<
        dyn Future<
                Output = Result<
                    CdpAdoptedElementHandle,
                    OperationError<HandleEvaluationErrorDetails>,
                >,
            > + Send,
    >,
>;

impl Service<CdpElementHandleAdoptionRequest> for CdpElementHandleAdoptionService {
    type Response = CdpAdoptedElementHandle;
    type Error = OperationError<HandleEvaluationErrorDetails>;
    type Future<'a> = CdpElementHandleAdoptionFuture;

    fn call(&self, request: CdpElementHandleAdoptionRequest) -> Self::Future<'_> {
        let limits = self.limits;
        if let Err(error) = self.validate_request(&request) {
            return Box::pin(async move { Err(error) });
        }
        Box::pin(async move {
            let source_object_id = request
                .source
                .object_id()
                .expect("validated ElementHandle has one native object ID");

            let backend_node_id = request
                .target_client
                .dom_describe_node_by_object_id(source_object_id)
                .await
                .map_err(|error| map_cdp_error(error, limits.evaluation().error_bytes))?;
            if backend_node_id <= 0 {
                return Err(invalid_response(
                    "CDP DOM.describeNode returned a non-positive backend node ID",
                ));
            }
            let response = match request.object_group.as_ref() {
                Some(group) => {
                    request
                        .target_client
                        .dom_resolve_backend_node_in_execution_context_group(
                            backend_node_id,
                            request.target_execution_context_id,
                            group,
                        )
                        .await
                }
                None => {
                    request
                        .target_client
                        .dom_resolve_backend_node_in_execution_context(
                            backend_node_id,
                            request.target_execution_context_id,
                        )
                        .await
                }
            }
            .map_err(|error| map_cdp_error(error, limits.evaluation().error_bytes))?;
            parse_adopted_response(request.target_client, response, limits).await
        })
    }
}

/// Explicit production budget for temporary Chromium ElementHandle wrappers.
/// This profile is independent from persistent portable Handle routes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpElementHandleAdoptionProductionProfile {
    scope: ResourceScope,
    limits: CdpHandleEvaluationLimits,
    max_concurrent_batches: usize,
    max_handles_per_batch: usize,
    maximum_retained_bytes_per_handle: u64,
    requirements: ResourceRequirements,
}

impl CdpElementHandleAdoptionProductionProfile {
    pub fn new(
        scope: ResourceScope,
        limits: CdpHandleEvaluationLimits,
        max_concurrent_batches: usize,
        max_handles_per_batch: usize,
    ) -> Self {
        let maximum_retained_bytes_per_handle = u64::try_from(limits.native_id_bytes())
            .unwrap_or(u64::MAX)
            .saturating_add(u64::try_from(limits.preview_bytes()).unwrap_or(u64::MAX));
        let handles = u64::try_from(max_concurrent_batches)
            .unwrap_or(u64::MAX)
            .saturating_mul(u64::try_from(max_handles_per_batch).unwrap_or(u64::MAX));
        let bytes = handles.saturating_mul(maximum_retained_bytes_per_handle);
        let requirements = if handles == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::RemoteHandle, handles, handles)
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(
                    1,
                    max_concurrent_batches.saturating_mul(max_handles_per_batch),
                )
        };
        Self {
            scope,
            limits,
            max_concurrent_batches,
            max_handles_per_batch,
            maximum_retained_bytes_per_handle,
            requirements,
        }
    }

    pub const fn scope(self) -> ResourceScope {
        self.scope
    }

    pub const fn limits(self) -> CdpHandleEvaluationLimits {
        self.limits
    }

    pub const fn max_concurrent_batches(self) -> usize {
        self.max_concurrent_batches
    }

    pub const fn max_handles_per_batch(self) -> usize {
        self.max_handles_per_batch
    }

    pub const fn maximum_retained_bytes_per_handle(self) -> u64 {
        self.maximum_retained_bytes_per_handle
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpElementHandleAdoptionSetupError {
    ZeroBatchCapacity,
    ZeroHandleCapacity,
    ZeroNativeIdBytes,
}

impl fmt::Display for CdpElementHandleAdoptionSetupError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroBatchCapacity => {
                formatter.write_str("ElementHandle adoption batch capacity must be non-zero")
            }
            Self::ZeroHandleCapacity => formatter
                .write_str("ElementHandle adoption per-batch Handle capacity must be non-zero"),
            Self::ZeroNativeIdBytes => formatter
                .write_str("ElementHandle adoption native-object ID bound must be non-zero"),
        }
    }
}

impl std::error::Error for CdpElementHandleAdoptionSetupError {}

#[derive(Debug)]
pub enum CdpElementHandleAdoptionInstallError {
    Setup(CdpElementHandleAdoptionSetupError),
    Resources(ProductionResourceError),
}

impl fmt::Display for CdpElementHandleAdoptionInstallError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Setup(error) => fmt::Display::fmt(error, formatter),
            Self::Resources(error) => fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for CdpElementHandleAdoptionInstallError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Setup(error) => Some(error),
            Self::Resources(error) => Some(error),
        }
    }
}

/// A bounded set of adoption requests. Composition is responsible for
/// deduplicating portable Handle IDs before constructing this value.
pub struct CdpElementHandleAdoptionBatchRequest {
    requests: Vec<CdpElementHandleAdoptionRequest>,
}

impl CdpElementHandleAdoptionBatchRequest {
    pub const fn new(requests: Vec<CdpElementHandleAdoptionRequest>) -> Self {
        Self { requests }
    }

    pub fn requests(&self) -> &[CdpElementHandleAdoptionRequest] {
        &self.requests
    }
}

/// Must-use ownership of every temporary adopted RemoteObject and its exact
/// production leases. Explicit release attempts every object before returning.
#[must_use = "an adopted ElementHandle batch must be released after its operation"]
pub struct CdpAdoptedElementHandleBatch {
    handles: Vec<CdpAdoptedElementHandle>,
    _resources: Vec<ResourceLease>,
    _active_batch: AdoptionBatchLease,
}

impl Debug for CdpAdoptedElementHandleBatch {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CdpAdoptedElementHandleBatch")
            .field("handles", &self.handles)
            .finish_non_exhaustive()
    }
}

impl CdpAdoptedElementHandleBatch {
    pub fn handles(&self) -> &[CdpAdoptedElementHandle] {
        &self.handles
    }

    /// Transfers the adopted native owners into an already-admitted persistent
    /// Handle route. The caller must establish that durable ownership before
    /// calling this method; consuming the batch releases only its temporary
    /// adoption accounting, not the Chromium objects.
    pub fn into_handles(self) -> Vec<CdpEvaluatedHandle> {
        let Self {
            handles,
            _resources,
            _active_batch,
        } = self;
        let handles = handles.into_iter().map(|handle| handle.handle).collect();
        drop((_resources, _active_batch));
        handles
    }

    pub async fn release(self) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
        release_batch(self.handles).await
    }
}

struct AdoptionBatchLease {
    active: Arc<AtomicUsize>,
}

impl Drop for AdoptionBatchLease {
    fn drop(&mut self) {
        let previous = self.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

struct AdoptionBatchReservation {
    leases: Vec<ResourceLease>,
    active: AdoptionBatchLease,
}

#[derive(Clone)]
pub struct CdpElementHandleAdoptionBatchService {
    adoption: CdpElementHandleAdoptionService,
    profile: CdpElementHandleAdoptionProductionProfile,
    ledger: pwright_core::ResourceLedger,
    active: Arc<AtomicUsize>,
}

impl CdpElementHandleAdoptionBatchService {
    pub fn new_with_production_resources(
        profile: CdpElementHandleAdoptionProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, CdpElementHandleAdoptionInstallError> {
        if profile.max_concurrent_batches == 0 {
            return Err(CdpElementHandleAdoptionInstallError::Setup(
                CdpElementHandleAdoptionSetupError::ZeroBatchCapacity,
            ));
        }
        if profile.max_handles_per_batch == 0 {
            return Err(CdpElementHandleAdoptionInstallError::Setup(
                CdpElementHandleAdoptionSetupError::ZeroHandleCapacity,
            ));
        }
        if profile.limits.native_id_bytes() == 0 {
            return Err(CdpElementHandleAdoptionInstallError::Setup(
                CdpElementHandleAdoptionSetupError::ZeroNativeIdBytes,
            ));
        }
        let ledger = resources
            .claim(profile.requirements())
            .map_err(CdpElementHandleAdoptionInstallError::Resources)?;
        Ok(Self {
            adoption: CdpElementHandleAdoptionService::new(profile.limits),
            profile,
            ledger,
            active: Arc::new(AtomicUsize::new(0)),
        })
    }

    pub const fn profile(&self) -> CdpElementHandleAdoptionProductionProfile {
        self.profile
    }

    fn reserve_batch(
        &self,
        count: usize,
    ) -> Result<AdoptionBatchReservation, OperationError<HandleEvaluationErrorDetails>> {
        if count == 0 {
            return Err(adoption_resource_error(
                "ElementHandle adoption batch must not be empty",
            ));
        }
        if count > self.profile.max_handles_per_batch {
            return Err(adoption_resource_error(&format!(
                "ElementHandle adoption batch has {count} Handles, limit is {}",
                self.profile.max_handles_per_batch
            )));
        }
        if self
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < self.profile.max_concurrent_batches).then_some(current + 1)
            })
            .is_err()
        {
            return Err(adoption_resource_error(
                "ElementHandle adoption concurrent batch limit was reached",
            ));
        }
        let requests = [
            ResourceRequest::new(ResourceClass::RemoteHandle, 1),
            ResourceRequest::new(
                ResourceClass::BackendStateBytes,
                self.profile.maximum_retained_bytes_per_handle,
            ),
        ];
        let mut leases = Vec::with_capacity(count);
        for _ in 0..count {
            match self.ledger.reserve_many(self.profile.scope, &requests) {
                Ok(lease) => leases.push(lease),
                Err(error) => {
                    drop(leases);
                    let previous = self.active.fetch_sub(1, Ordering::AcqRel);
                    debug_assert!(previous > 0);
                    return Err(map_adoption_resource_error(error));
                }
            }
        }
        Ok(AdoptionBatchReservation {
            leases,
            active: AdoptionBatchLease {
                active: Arc::clone(&self.active),
            },
        })
    }
}

type CdpElementHandleAdoptionBatchFuture = Pin<
    Box<
        dyn Future<
                Output = Result<
                    CdpAdoptedElementHandleBatch,
                    OperationError<HandleEvaluationErrorDetails>,
                >,
            > + Send,
    >,
>;

impl Service<CdpElementHandleAdoptionBatchRequest> for CdpElementHandleAdoptionBatchService {
    type Response = CdpAdoptedElementHandleBatch;
    type Error = OperationError<HandleEvaluationErrorDetails>;
    type Future<'a> = CdpElementHandleAdoptionBatchFuture;

    fn call(&self, request: CdpElementHandleAdoptionBatchRequest) -> Self::Future<'_> {
        for item in &request.requests {
            if let Err(error) = self.adoption.validate_request(item) {
                return Box::pin(async move { Err(error) });
            }
        }
        let mut reservation = match self.reserve_batch(request.requests.len()) {
            Ok(reservation) => reservation,
            Err(error) => return Box::pin(async move { Err(error) }),
        };
        let adoption = self.adoption;
        Box::pin(async move {
            let mut handles = Vec::with_capacity(request.requests.len());
            let mut resources = Vec::with_capacity(request.requests.len());
            for item in request.requests {
                match adoption.call(item).await {
                    Ok(handle) => {
                        let mut lease = reservation
                            .leases
                            .pop()
                            .expect("batch reservation has one lease per request");
                        let retained_bytes = u64::try_from(handle.handle().native_argument_bytes())
                            .unwrap_or(u64::MAX)
                            .saturating_add(
                                u64::try_from(handle.handle().preview().len()).unwrap_or(u64::MAX),
                            );
                        if let Err(error) =
                            lease.try_resize(ResourceClass::BackendStateBytes, retained_bytes)
                        {
                            handles.push(handle);
                            let primary = map_adoption_resource_error(error);
                            return release_after_failure(handles, primary).await;
                        }
                        handles.push(handle);
                        resources.push(lease);
                    }
                    Err(primary) => return release_after_failure(handles, primary).await,
                }
            }
            Ok(CdpAdoptedElementHandleBatch {
                handles,
                _resources: resources,
                _active_batch: reservation.active,
            })
        })
    }
}

async fn release_batch(
    handles: Vec<CdpAdoptedElementHandle>,
) -> Result<(), OperationError<HandleEvaluationErrorDetails>> {
    let mut failures = Vec::new();
    for handle in handles {
        if let Err(error) = handle.release().await {
            failures.push(error.to_string());
        }
    }
    if failures.is_empty() {
        return Ok(());
    }
    let message = format!(
        "failed to release {} adopted ElementHandle object(s): {}",
        failures.len(),
        failures.join("; ")
    );
    Err(OperationError::new(
        FailureKind::Cleanup,
        message.clone(),
        Some(HandleEvaluationErrorDetails::Backend(message)),
    ))
}

async fn release_after_failure(
    handles: Vec<CdpAdoptedElementHandle>,
    primary: OperationError<HandleEvaluationErrorDetails>,
) -> Result<CdpAdoptedElementHandleBatch, OperationError<HandleEvaluationErrorDetails>> {
    match release_batch(handles).await {
        Ok(()) => Err(primary),
        Err(cleanup) => {
            let message = format!(
                "{}; adopted ElementHandle cleanup also failed: {}",
                primary.message(),
                cleanup.message()
            );
            Err(OperationError::new(
                FailureKind::Cleanup,
                message.clone(),
                Some(HandleEvaluationErrorDetails::Backend(message)),
            ))
        }
    }
}

fn adoption_resource_error(message: &str) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(HandleEvaluationErrorDetails::InvalidValue(
            message.to_owned(),
        )),
    )
}

fn map_adoption_resource_error(
    error: ResourceError,
) -> OperationError<HandleEvaluationErrorDetails> {
    adoption_resource_error(&format!(
        "ElementHandle adoption resource admission failed: {error}"
    ))
}

async fn parse_adopted_response(
    client: Arc<dyn CdpClient>,
    response: Value,
    limits: CdpHandleEvaluationLimits,
) -> Result<CdpAdoptedElementHandle, OperationError<HandleEvaluationErrorDetails>> {
    let remote = response
        .get("object")
        .cloned()
        .ok_or_else(|| invalid_response("CDP DOM.resolveNode omitted the adopted RemoteObject"))?;
    let cleanup_id = remote
        .get("objectId")
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);
    if remote.get("subtype").and_then(Value::as_str) == Some("null") {
        return release_raw_after_failure(
            client,
            cleanup_id,
            invalid_value("Unable to adopt element handle from a different document".to_owned()),
        )
        .await;
    }
    let parsed = parse_response(
        Arc::clone(&client),
        serde_json::json!({ "result": remote }),
        limits,
    );
    match parsed {
        Ok(handle) if handle.kind() == CdpHandleKind::Element => {
            Ok(CdpAdoptedElementHandle { handle })
        }
        Ok(handle) => {
            let primary = invalid_response(
                "CDP DOM.resolveNode returned a non-element RemoteObject for adoption",
            );
            release_handle_after_failure(handle, primary).await
        }
        Err(primary) => release_raw_after_failure(client, cleanup_id, primary).await,
    }
}

async fn release_handle_after_failure(
    handle: CdpEvaluatedHandle,
    primary: OperationError<HandleEvaluationErrorDetails>,
) -> Result<CdpAdoptedElementHandle, OperationError<HandleEvaluationErrorDetails>> {
    match handle.release().await {
        Ok(()) => Err(primary),
        Err(cleanup) => Err(cleanup_failure(primary, cleanup)),
    }
}

async fn release_raw_after_failure(
    client: Arc<dyn CdpClient>,
    object_id: Option<String>,
    primary: OperationError<HandleEvaluationErrorDetails>,
) -> Result<CdpAdoptedElementHandle, OperationError<HandleEvaluationErrorDetails>> {
    let Some(object_id) = object_id else {
        return Err(primary);
    };
    match client.runtime_release_object(&object_id).await {
        Ok(()) => Err(primary),
        Err(cleanup) => Err(cleanup_failure(primary, cleanup)),
    }
}

fn cleanup_failure(
    primary: OperationError<HandleEvaluationErrorDetails>,
    cleanup: CdpError,
) -> OperationError<HandleEvaluationErrorDetails> {
    OperationError::new(
        FailureKind::Cleanup,
        "ElementHandle adoption failed and temporary RemoteObject cleanup also failed",
        Some(HandleEvaluationErrorDetails::Cleanup {
            primary: primary.to_string(),
            cleanup: cleanup.to_string(),
        }),
    )
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use pwright_cap_handle_evaluation::EvaluateHandleParams;
    use pwright_core::{
        FailureKind, ProductionResourceLedger, ResourceClass, ResourceLimits, ResourceRequirements,
        ResourceScope, Service,
    };
    use pwright_fake::FakeCdpClient;

    use super::{
        CdpElementHandleAdoptionBatchRequest, CdpElementHandleAdoptionBatchService,
        CdpElementHandleAdoptionProductionProfile, CdpElementHandleAdoptionRequest,
        CdpElementHandleAdoptionService, CdpHandleEvaluationLimits,
    };
    use crate::handle_evaluation::{CdpEvaluatedHandle, CdpHandleEvaluationService};

    fn exact_resources(requirements: ResourceRequirements) -> ProductionResourceLedger {
        let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
        for class in ResourceClass::ALL {
            let required = requirements.class(class);
            limits = limits.with_class(class, required.global, required.per_scope);
        }
        ProductionResourceLedger::new(limits, requirements)
            .expect("exact adoption resource plan should build")
    }

    fn adoption_profile(
        max_concurrent_batches: usize,
        max_handles_per_batch: usize,
    ) -> CdpElementHandleAdoptionProductionProfile {
        CdpElementHandleAdoptionProductionProfile::new(
            ResourceScope::extension(77, 1),
            CdpHandleEvaluationLimits::default(),
            max_concurrent_batches,
            max_handles_per_batch,
        )
    }

    async fn source_handle(
        client: Arc<FakeCdpClient>,
        object_id: &str,
        node: bool,
    ) -> CdpEvaluatedHandle {
        client.set_evaluate_response(if node {
            serde_json::json!({
                "result": {
                    "type": "object",
                    "subtype": "node",
                    "description": object_id,
                    "objectId": object_id
                }
            })
        } else {
            serde_json::json!({
                "result": {
                    "type": "object",
                    "description": object_id,
                    "objectId": object_id
                }
            })
        });
        CdpHandleEvaluationService::new(client)
            .call(EvaluateHandleParams::expression("source"))
            .await
            .expect("source Handle should parse")
    }

    #[tokio::test]
    async fn adopts_one_element_into_the_exact_context_and_releases_it() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(serde_json::json!({
            "result": {
                "type": "object",
                "subtype": "node",
                "description": "body",
                "objectId": "source-node"
            }
        }));
        client.set_object_backend_node("source-node", 71);
        client.set_adopted_node_response(
            71,
            29,
            serde_json::json!({
                "object": {
                    "type": "object",
                    "subtype": "node",
                    "description": "body",
                    "objectId": "adopted-node"
                }
            }),
        );
        let source = CdpHandleEvaluationService::new(client.clone())
            .call(EvaluateHandleParams::expression("document.body"))
            .await
            .unwrap();
        let service = CdpElementHandleAdoptionService::new(CdpHandleEvaluationLimits::default());

        // Act
        let adopted = service
            .call(CdpElementHandleAdoptionRequest::new(
                source,
                client.clone(),
                29,
            ))
            .await
            .unwrap();
        let adopted_id = adopted.handle().object_id().map(str::to_owned);
        adopted.release().await.unwrap();

        // Assert
        assert_eq!(adopted_id.as_deref(), Some("adopted-node"));
        assert_eq!(
            client.method_names(),
            [
                "Runtime.evaluate",
                "DOM.describeNode",
                "DOM.resolveNode",
                "Runtime.releaseObject"
            ]
        );
        let calls = client.calls();
        assert_eq!(calls[1].1[0]["objectId"], "source-node");
        assert_eq!(calls[2].1[0]["backendNodeId"], 71);
        assert_eq!(calls[2].1[0]["executionContextId"], 29);
        assert_eq!(calls[3].1[0], "adopted-node");
    }

    #[tokio::test]
    async fn js_handle_is_rejected_before_dom_io() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(serde_json::json!({
            "result": {
                "type": "object",
                "description": "Object",
                "objectId": "plain-object"
            }
        }));
        let source = CdpHandleEvaluationService::new(client.clone())
            .call(EvaluateHandleParams::expression("({})"))
            .await
            .unwrap();
        let service = CdpElementHandleAdoptionService::new(CdpHandleEvaluationLimits::default());

        // Act
        let error = service
            .call(CdpElementHandleAdoptionRequest::new(
                source,
                client.clone(),
                29,
            ))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(client.method_names(), ["Runtime.evaluate"]);
    }

    #[tokio::test]
    async fn different_cdp_session_is_rejected_before_dom_io() {
        // Arrange
        let source_client = Arc::new(FakeCdpClient::from_html(""));
        source_client.set_evaluate_response(serde_json::json!({
            "result": {
                "type": "object",
                "subtype": "node",
                "description": "body",
                "objectId": "source-node"
            }
        }));
        let source = CdpHandleEvaluationService::new(source_client.clone())
            .call(EvaluateHandleParams::expression("document.body"))
            .await
            .unwrap();
        let target_client = Arc::new(FakeCdpClient::from_html(""));
        let service = CdpElementHandleAdoptionService::new(CdpHandleEvaluationLimits::default());

        // Act
        let error = service
            .call(CdpElementHandleAdoptionRequest::new(
                source,
                target_client.clone(),
                29,
            ))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(source_client.method_names(), ["Runtime.evaluate"]);
        assert!(target_client.method_names().is_empty());
    }

    #[tokio::test]
    async fn invalid_target_context_is_rejected_before_dom_io() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(serde_json::json!({
            "result": {
                "type": "object",
                "subtype": "node",
                "description": "body",
                "objectId": "source-node"
            }
        }));
        let source = CdpHandleEvaluationService::new(client.clone())
            .call(EvaluateHandleParams::expression("document.body"))
            .await
            .unwrap();
        let service = CdpElementHandleAdoptionService::new(CdpHandleEvaluationLimits::default());

        // Act
        let error = service
            .call(CdpElementHandleAdoptionRequest::new(
                source,
                client.clone(),
                0,
            ))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(client.method_names(), ["Runtime.evaluate"]);
    }

    #[tokio::test]
    async fn malformed_adopted_object_is_compensated_once() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_response(serde_json::json!({
            "result": {
                "type": "object",
                "subtype": "node",
                "description": "body",
                "objectId": "source-node"
            }
        }));
        client.set_object_backend_node("source-node", 71);
        client.set_adopted_node_response(
            71,
            29,
            serde_json::json!({
                "object": {
                    "type": "object",
                    "description": "not a node",
                    "objectId": "wrong-object"
                }
            }),
        );
        let source = CdpHandleEvaluationService::new(client.clone())
            .call(EvaluateHandleParams::expression("document.body"))
            .await
            .unwrap();
        let service = CdpElementHandleAdoptionService::new(CdpHandleEvaluationLimits::default());

        // Act
        let error = service
            .call(CdpElementHandleAdoptionRequest::new(
                source,
                client.clone(),
                29,
            ))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(
            client.method_names(),
            [
                "Runtime.evaluate",
                "DOM.describeNode",
                "DOM.resolveNode",
                "Runtime.releaseObject"
            ]
        );
        assert_eq!(client.calls()[3].1[0], "wrong-object");
    }

    #[test]
    fn production_profile_declares_the_complete_temporary_handle_peak() {
        // Arrange
        let profile = adoption_profile(3, 4);
        let per_handle = profile.maximum_retained_bytes_per_handle();

        // Act
        let requirements = profile.requirements();

        // Assert
        assert_eq!(requirements.class(ResourceClass::RemoteHandle).global, 12);
        assert_eq!(
            requirements.class(ResourceClass::RemoteHandle).per_scope,
            12
        );
        assert_eq!(
            requirements.class(ResourceClass::BackendStateBytes).global,
            12 * per_handle
        );
        assert_eq!(requirements.scopes, 1);
        assert_eq!(requirements.leases, 12);
    }

    #[tokio::test]
    async fn production_batch_accounts_every_temporary_object_until_release() {
        // Arrange
        let profile = adoption_profile(1, 2);
        let resources = exact_resources(profile.requirements());
        let service = CdpElementHandleAdoptionBatchService::new_with_production_resources(
            profile, &resources,
        )
        .unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        let first = source_handle(client.clone(), "source-a", true).await;
        let second = source_handle(client.clone(), "source-b", true).await;
        client.set_object_backend_node("source-a", 71);
        client.set_object_backend_node("source-b", 72);
        client.set_adopted_node_response(
            71,
            29,
            serde_json::json!({
                "object": {"type": "object", "subtype": "node", "description": "A", "objectId": "adopted-a"}
            }),
        );
        client.set_adopted_node_response(
            72,
            29,
            serde_json::json!({
                "object": {"type": "object", "subtype": "node", "description": "B", "objectId": "adopted-b"}
            }),
        );

        // Act
        let batch = service
            .call(CdpElementHandleAdoptionBatchRequest::new(vec![
                CdpElementHandleAdoptionRequest::new(first, client.clone(), 29),
                CdpElementHandleAdoptionRequest::new(second, client.clone(), 29),
            ]))
            .await
            .unwrap();
        let retained = resources.metrics().unwrap();
        batch.release().await.unwrap();
        let released = resources.metrics().unwrap();

        // Assert
        assert_eq!(retained.class(ResourceClass::RemoteHandle).current, 2);
        assert_eq!(retained.class(ResourceClass::RemoteHandle).peak, 2);
        assert_eq!(retained.active_leases, 2);
        assert_eq!(released.class(ResourceClass::RemoteHandle).current, 0);
        assert_eq!(released.class(ResourceClass::BackendStateBytes).current, 0);
        assert_eq!(released.active_leases, 0);
        assert_eq!(
            client.method_names(),
            [
                "Runtime.evaluate",
                "Runtime.evaluate",
                "DOM.describeNode",
                "DOM.resolveNode",
                "DOM.describeNode",
                "DOM.resolveNode",
                "Runtime.releaseObject",
                "Runtime.releaseObject",
            ]
        );
    }

    #[tokio::test]
    async fn invalid_later_request_rejects_the_whole_batch_before_dom_io() {
        // Arrange
        let profile = adoption_profile(1, 2);
        let resources = exact_resources(profile.requirements());
        let service = CdpElementHandleAdoptionBatchService::new_with_production_resources(
            profile, &resources,
        )
        .unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        let element = source_handle(client.clone(), "source-node", true).await;
        let javascript = source_handle(client.clone(), "source-object", false).await;
        let calls_before = client.calls().len();

        // Act
        let error = service
            .call(CdpElementHandleAdoptionBatchRequest::new(vec![
                CdpElementHandleAdoptionRequest::new(element, client.clone(), 29),
                CdpElementHandleAdoptionRequest::new(javascript, client.clone(), 29),
            ]))
            .await
            .expect_err("one JSHandle must reject the complete adoption batch");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(client.calls().len(), calls_before);
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
    }

    #[tokio::test]
    async fn partial_batch_failure_releases_prior_adoptions_and_all_resources() {
        // Arrange
        let profile = adoption_profile(1, 2);
        let resources = exact_resources(profile.requirements());
        let service = CdpElementHandleAdoptionBatchService::new_with_production_resources(
            profile, &resources,
        )
        .unwrap();
        let client = Arc::new(FakeCdpClient::from_html(""));
        let first = source_handle(client.clone(), "source-a", true).await;
        let second = source_handle(client.clone(), "source-missing", true).await;
        client.set_object_backend_node("source-a", 71);
        client.set_adopted_node_response(
            71,
            29,
            serde_json::json!({
                "object": {"type": "object", "subtype": "node", "description": "A", "objectId": "adopted-a"}
            }),
        );

        // Act
        let error = service
            .call(CdpElementHandleAdoptionBatchRequest::new(vec![
                CdpElementHandleAdoptionRequest::new(first, client.clone(), 29),
                CdpElementHandleAdoptionRequest::new(second, client.clone(), 29),
            ]))
            .await
            .expect_err("missing second backend node must fail the batch");
        let metrics = resources.metrics().unwrap();

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(metrics.class(ResourceClass::RemoteHandle).current, 0);
        assert_eq!(metrics.class(ResourceClass::BackendStateBytes).current, 0);
        assert_eq!(metrics.active_leases, 0);
        assert_eq!(client.calls().last().unwrap().1[0], "adopted-a");
        assert_eq!(
            client.method_names().last().unwrap(),
            "Runtime.releaseObject"
        );
    }
}
