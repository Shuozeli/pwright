//! Optional typed Touchscreen starter with separate result and native retirement.
//! The caller retains each operation after response cancellation so an
//! acknowledged touchStart is never abandoned before touchEnd is polled.

use pwright_backend_cdp_common::classify_cdp_failure;
use pwright_backend_cdp_input::{CdpInputCoordinator, CdpInputPolicy, CdpModifiers};
use pwright_browser_backend::{
    BackendDescriptor, BackendFeatureDescriptor, BackendFeatureId, BackendId,
    BackendImplementation, BackendProtocolId, BrowserBackend, BrowserEngine, NativeBackendAccess,
};
use pwright_cap_touchscreen::{
    TOUCHSCREEN_CAPABILITY_ID, TOUCHSCREEN_TAP_COMMAND_ID, TapParams, TouchscreenErrorDetails,
};
use pwright_cdp::{CdpClient, connection::CdpError};
use pwright_core::{
    FailureKind, OperationError, ProductionResourceError, ProductionResourceLedger,
    ProtocolVersion, ResourceClass, ResourceLease, ResourceLedger, ResourceRequirements,
    ResourceScope,
};
use std::{
    fmt::{self, Write},
    future::Future,
    num::NonZeroUsize,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

pub const TOUCHSCREEN_ERROR_MAX_BYTES: usize = 4096;
pub type TouchscreenResult = Result<(), OperationError<TouchscreenErrorDetails>>;
type NativeFuture = Pin<Box<dyn Future<Output = Result<(), CdpError>> + Send>>;

pub const CHROMIUM_CDP_TOUCHSCREEN_BACKEND_ID: BackendId =
    BackendId::new("pwright.backend.chromium-cdp.touchscreen");
pub const CHROMIUM_CDP_TOUCHSCREEN_FEATURES: &[BackendFeatureDescriptor] =
    &[BackendFeatureDescriptor::new(
        BackendFeatureId::new("touchscreen.tap"),
        TOUCHSCREEN_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        TOUCHSCREEN_TAP_COMMAND_ID,
        BackendImplementation::Native,
    )];

pub enum ChromiumCdpTouchscreenBackend {}

impl BrowserBackend for ChromiumCdpTouchscreenBackend {
    const DESCRIPTOR: BackendDescriptor = BackendDescriptor::new(
        CHROMIUM_CDP_TOUCHSCREEN_BACKEND_ID,
        "Chromium CDP Touchscreen",
        BrowserEngine::Chromium,
        BackendProtocolId::new("cdp"),
        CHROMIUM_CDP_TOUCHSCREEN_FEATURES,
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpTouchscreenProfileError {
    ByteBudgetOverflow,
}

impl fmt::Display for CdpTouchscreenProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Touchscreen declared byte budget overflowed")
    }
}

impl std::error::Error for CdpTouchscreenProfileError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpTouchscreenProductionProfile {
    scope: ResourceScope,
    operation_bytes: usize,
    operations: NonZeroUsize,
    requirements: ResourceRequirements,
}

impl CdpTouchscreenProductionProfile {
    pub fn new(
        scope: ResourceScope,
        operation_bytes: usize,
        operations: NonZeroUsize,
    ) -> Result<Self, CdpTouchscreenProfileError> {
        let bytes = u64::try_from(operation_bytes)
            .ok()
            .and_then(|bytes| bytes.checked_mul(u64::try_from(operations.get()).ok()?))
            .ok_or(CdpTouchscreenProfileError::ByteBudgetOverflow)?;
        Ok(Self {
            scope,
            operation_bytes,
            operations,
            requirements: ResourceRequirements::none()
                .with_class(ResourceClass::BackendStateBytes, bytes, bytes)
                .with_metadata(1, operations.get()),
        })
    }

    pub const fn operation_bytes(self) -> usize {
        self.operation_bytes
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Clone)]
struct Budget {
    profile: CdpTouchscreenProductionProfile,
    ledger: ResourceLedger,
    active: Arc<AtomicUsize>,
}

impl Budget {
    fn new(
        profile: CdpTouchscreenProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        Ok(Self {
            profile,
            ledger: resources.claim(profile.requirements())?,
            active: Arc::new(AtomicUsize::new(0)),
        })
    }

    fn reserve(&self) -> TouchscreenResultLease {
        if self
            .active
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |active| {
                (active < self.profile.operations.get()).then(|| active + 1)
            })
            .is_err()
        {
            return Err(resource_error(
                "Touchscreen operation concurrency limit was reached",
            ));
        }
        match self.ledger.reserve(
            self.profile.scope,
            ResourceClass::BackendStateBytes,
            u64::try_from(self.profile.operation_bytes).expect("validated byte budget"),
        ) {
            Ok(resource) => Ok(Lease {
                resource: Some(resource),
                active: self.active.clone(),
            }),
            Err(_) => {
                self.active.fetch_sub(1, Ordering::AcqRel);
                Err(resource_error(
                    "Touchscreen backend byte lease could not be reserved",
                ))
            }
        }
    }
}

type TouchscreenResultLease = Result<Lease, OperationError<TouchscreenErrorDetails>>;

struct Lease {
    resource: Option<ResourceLease>,
    active: Arc<AtomicUsize>,
}

impl Drop for Lease {
    fn drop(&mut self) {
        drop(self.resource.take());
        let previous = self.active.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

#[derive(Clone)]
pub struct CdpTouchscreenServiceFactory {
    budget: Budget,
}

impl CdpTouchscreenServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpTouchscreenProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        Ok(Self {
            budget: Budget::new(profile, resources)?,
        })
    }

    pub fn profile(&self) -> CdpTouchscreenProductionProfile {
        self.budget.profile
    }

    pub fn active_operations(&self) -> usize {
        self.budget.active.load(Ordering::Acquire)
    }

    pub fn bind(
        &self,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
    ) -> Result<CdpTouchscreenService, OperationError<TouchscreenErrorDetails>> {
        if input.policy() != CdpInputPolicy::Interleaved {
            return Err(OperationError::new(
                FailureKind::UnsupportedCapability,
                "Touchscreen requires shared Interleaved Page input",
                None,
            ));
        }
        if self.profile().operation_bytes()
            < CdpTouchscreenService::required_operation_bytes(client.clone(), input.clone())
        {
            return Err(resource_error(
                "Touchscreen profile does not cover concrete operation storage",
            ));
        }
        Ok(CdpTouchscreenService {
            client,
            input,
            budget: self.budget.clone(),
        })
    }
}

#[derive(Clone)]
pub struct CdpTouchscreenService {
    client: Arc<dyn CdpClient>,
    input: CdpInputCoordinator,
    budget: Budget,
}

impl CdpTouchscreenService {
    pub fn shares_input_state_with(&self, input: &CdpInputCoordinator) -> bool {
        self.input.shares_state_with(input)
    }

    pub fn required_operation_bytes(
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
    ) -> usize {
        let future = tap_native(
            client,
            input,
            TapParams::new(
                0.0.try_into().expect("zero is finite"),
                0.0.try_into().expect("zero is finite"),
            ),
            None,
        );
        std::mem::size_of::<CdpTouchscreenTapOperation>()
            + std::mem::size_of_val(&future)
            + 2 * TOUCHSCREEN_ERROR_MAX_BYTES
    }

    pub fn start(
        &self,
        params: TapParams,
    ) -> Result<CdpTouchscreenTapOperation, OperationError<TouchscreenErrorDetails>> {
        self.start_with_modifiers(params, None)
    }

    pub fn start_with_modifiers(
        &self,
        params: TapParams,
        modifiers: Option<CdpModifiers>,
    ) -> Result<CdpTouchscreenTapOperation, OperationError<TouchscreenErrorDetails>> {
        let lease = self.budget.reserve()?;
        let future = Box::pin(tap_native(
            self.client.clone(),
            self.input.clone(),
            params,
            modifiers,
        ));
        Ok(CdpTouchscreenTapOperation {
            future: Some(future),
            started: false,
            observed: false,
            _lease: lease,
        })
    }
}

impl NativeBackendAccess<ChromiumCdpTouchscreenBackend> for CdpTouchscreenService {
    type Native = dyn CdpClient;

    fn native_backend(&self) -> &Self::Native {
        self.client.as_ref()
    }
}

async fn tap_native(
    client: Arc<dyn CdpClient>,
    input: CdpInputCoordinator,
    params: TapParams,
    modifiers: Option<CdpModifiers>,
) -> Result<(), CdpError> {
    let mut transaction = input.begin_transaction(client, modifiers).await?;
    let action = transaction
        .touchscreen_tap(params.x.get(), params.y.get())
        .await;
    let cleanup = transaction.finish().await;
    cleanup?;
    action
}

#[must_use = "retain the Touchscreen operation through result and retirement"]
pub struct CdpTouchscreenTapOperation {
    future: Option<NativeFuture>,
    started: bool,
    observed: bool,
    _lease: Lease,
}

impl CdpTouchscreenTapOperation {
    pub fn pending_native_work(&self) -> bool {
        self.future.is_some()
    }

    pub async fn result(&mut self) -> TouchscreenResult {
        if self.observed {
            return Err(OperationError::new(
                FailureKind::ProtocolViolation,
                "Touchscreen result was already observed",
                None,
            ));
        }
        self.started = true;
        let result = self
            .future
            .as_mut()
            .expect("unobserved operation retains native future")
            .await;
        self.future = None;
        self.observed = true;
        result.map_err(native_error)
    }

    /// Continues any started native pair after caller cancellation. An unpolled
    /// operation performs no input; a partially polled pair reaches touchEnd.
    pub async fn retire(&mut self) -> TouchscreenResult {
        self.observed = true;
        let result = match (self.started, self.future.as_mut()) {
            (true, Some(future)) => future.await,
            (true, None) | (false, _) => Ok(()),
        };
        self.future = None;
        result.map_err(|error| native_error(error).reclassify(FailureKind::Cleanup))
    }
}

fn resource_error(message: &str) -> OperationError<TouchscreenErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(TouchscreenErrorDetails::Resource(message.to_owned())),
    )
}

struct BoundedMessage(String);

impl Write for BoundedMessage {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        if value.len() > TOUCHSCREEN_ERROR_MAX_BYTES - self.0.len() {
            return Err(fmt::Error);
        }
        self.0.push_str(value);
        Ok(())
    }
}

fn native_error(error: CdpError) -> OperationError<TouchscreenErrorDetails> {
    let mut message = BoundedMessage(String::with_capacity(TOUCHSCREEN_ERROR_MAX_BYTES));
    if write!(&mut message, "{error}").is_err() {
        return resource_error("Native Touchscreen error exceeds the diagnostic limit");
    }
    OperationError::new(
        classify_cdp_failure(&error),
        message.0.clone(),
        Some(TouchscreenErrorDetails::Backend(message.0)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_core::ResourceLimits;
    use pwright_fake::FakeCdpClient;
    use std::task::{Context, Poll, Waker};

    fn resources(profile: CdpTouchscreenProductionProfile) -> ProductionResourceLedger {
        let required = profile.requirements();
        let mut limits = ResourceLimits::deny_all(required.scopes, required.leases);
        for class in ResourceClass::ALL {
            let bound = required.class(class);
            limits = limits.with_class(class, bound.global, bound.per_scope);
        }
        ProductionResourceLedger::new(limits, required).unwrap()
    }

    fn profile(operation_bytes: usize) -> CdpTouchscreenProductionProfile {
        CdpTouchscreenProductionProfile::new(
            ResourceScope::extension(140, 1),
            operation_bytes,
            NonZeroUsize::new(2).unwrap(),
        )
        .unwrap()
    }

    fn params() -> TapParams {
        TapParams::new(40.0.try_into().unwrap(), 60.0.try_into().unwrap())
    }

    fn touch_calls(client: &FakeCdpClient) -> Vec<pwright_fake::RecordedCdpCall> {
        client
            .calls()
            .into_iter()
            .filter(|(method, _)| method == "Input.dispatchTouchEvent")
            .collect()
    }

    #[tokio::test]
    async fn tap_dispatches_adjacent_start_and_end_with_current_modifiers() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let mut modifier = input.begin_transaction(client.clone(), None).await.unwrap();
        modifier.key_down_named("Alt", false).await.unwrap();
        modifier.finish().await.unwrap();
        let required =
            CdpTouchscreenService::required_operation_bytes(client.clone(), input.clone());
        let profile = profile(required);
        let resources = resources(profile);
        let factory =
            CdpTouchscreenServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind(client.clone(), input).unwrap();
        let mut operation = service.start(params()).unwrap();

        // Act
        operation.result().await.unwrap();

        // Assert
        let calls = touch_calls(&client);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1[0]["modifiers"], 1);
        assert_eq!(calls[1].1[0]["modifiers"], 1);
    }

    #[tokio::test]
    async fn tap_service_records_playwright_touch_payloads() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let required =
            CdpTouchscreenService::required_operation_bytes(client.clone(), input.clone());
        let profile = profile(required);
        let resources = resources(profile);
        let factory =
            CdpTouchscreenServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind(client.clone(), input).unwrap();
        let mut operation = service.start(params()).unwrap();

        // Act
        operation.result().await.unwrap();
        operation.retire().await.unwrap();

        // Assert
        let calls = touch_calls(&client);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1[0]["type"], "touchStart");
        assert_eq!(calls[0].1[0]["touchPoints"][0]["x"], 40.0);
        assert_eq!(calls[0].1[0]["touchPoints"][0]["y"], 60.0);
        assert_eq!(calls[0].1[0]["modifiers"], 0);
        assert_eq!(calls[1].1[0]["type"], "touchEnd");
        assert_eq!(calls[1].1[0]["touchPoints"], serde_json::json!([]));
        assert_eq!(factory.active_operations(), 1);
        drop(operation);
        assert_eq!(factory.active_operations(), 0);
    }

    #[tokio::test]
    async fn dropped_result_future_can_be_resumed_by_retirement() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_pointer_gates([Some(gate.clone()), None]);
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let required =
            CdpTouchscreenService::required_operation_bytes(client.clone(), input.clone());
        let profile = profile(required);
        let resources = resources(profile);
        let factory =
            CdpTouchscreenServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind(client.clone(), input).unwrap();
        let mut operation = service.start(params()).unwrap();
        let mut result = Box::pin(operation.result());
        assert!(matches!(
            result
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        drop(result);

        // Act
        gate.notify_one();
        operation.retire().await.unwrap();

        // Assert
        let calls = touch_calls(&client);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1[0]["type"], "touchStart");
        assert_eq!(calls[1].1[0]["type"], "touchEnd");
    }

    #[tokio::test]
    async fn unpolled_operation_retires_without_dispatching_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let input = CdpInputCoordinator::with_policy(CdpInputPolicy::Interleaved);
        let required =
            CdpTouchscreenService::required_operation_bytes(client.clone(), input.clone());
        let profile = profile(required);
        let resources = resources(profile);
        let factory =
            CdpTouchscreenServiceFactory::new_with_production_resources(profile, &resources)
                .unwrap();
        let service = factory.bind(client.clone(), input).unwrap();
        let mut operation = service.start(params()).unwrap();

        // Act
        operation.retire().await.unwrap();

        // Assert
        assert!(touch_calls(&client).is_empty());
        assert!(!operation.pending_native_work());
    }
}
