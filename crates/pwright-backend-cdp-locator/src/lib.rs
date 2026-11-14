//! Complete Chromium CDP implementation of the portable Locator capability.
//!
//! The service attaches to an existing target-scoped CDP client. It owns
//! bounded selector execution and Page-scoped input coordination, but it does
//! not own a browser process, transport, protocol server, or screenshot
//! capability.

#[cfg(feature = "fill-navigation-commit")]
mod fill_commit;
#[cfg(feature = "retained-element-click")]
mod retained_click_child;
#[cfg(feature = "retained-element-click")]
mod retained_click_diagnostics;
#[cfg(feature = "select-option-navigation-commit")]
mod select_option_commit;
mod sequential_typing;

#[cfg(feature = "fill-navigation-commit")]
use fill_commit::{FillCommitReceipt, is_navigation_context_loss};
pub use pwright_backend_cdp_input::CdpInputPolicy;
#[cfg(feature = "retained-element-click")]
pub use pwright_backend_cdp_input::CdpOwnedMousePress;
#[cfg(feature = "retained-element-click")]
pub use retained_click_diagnostics::RetainedClickDiagnostics;
#[cfg(feature = "select-option-navigation-commit")]
use select_option_commit::{
    SelectOptionCommitReceipt,
    is_navigation_context_loss as is_select_option_navigation_context_loss,
};

#[cfg(test)]
mod keyboard_binding_tests;

use std::future::{Future, Ready, ready};
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use pwright_backend_cdp_common::classify_cdp_failure;
#[cfg(feature = "retained-element-click")]
use pwright_backend_cdp_input::CdpCheckedMouseDownError;
use pwright_backend_cdp_input::{
    CdpInputCoordinator, CdpInputTransaction, CdpModifiers, KeyTokens, ResolvedKey, resolve_key,
};
#[cfg(test)]
use pwright_backend_cdp_selector_runtime::SELECTOR_RUNTIME;
use pwright_backend_cdp_selector_runtime::{
    MAX_COMPILED_SELECTOR_BYTES, MAX_SELECTOR_RUNTIME_INSTALL_BYTES, MAX_SELECTOR_STRING_BYTES,
    SELECTOR_RUNTIME_SYMBOL, SELECTOR_RUNTIME_VERSION, compile_selector, measure_selector,
    selector_runtime_install_script,
};
use pwright_cap_locator::{
    AllInnerTexts, AllInnerTextsParams, AllInnerTextsResult, AllTextContents,
    AllTextContentsParams, AllTextContentsResult, Blur, BlurParams, Click, ClickParams, Count,
    CountParams, CountResult, Fill, FillParams, Focus, FocusParams, GetAttribute,
    GetAttributeParams, GetAttributeResult, Hover, HoverOptions, HoverParams, InnerHtml,
    InnerHtmlParams, InnerHtmlResult, InnerText, InnerTextParams, InnerTextResult, InputValue,
    InputValueParams, InputValueResult, IsChecked, IsCheckedParams, IsEditable, IsEditableParams,
    IsEnabled, IsEnabledParams, IsVisible, IsVisibleParams, IsVisibleResult, KeyboardModifier,
    KeyboardModifiers, LocatorErrorDetails, MouseButton as PortableMouseButton, Press, PressParams,
    ScrollIntoViewIfNeeded, ScrollIntoViewIfNeededParams, ScrollMode, SelectOption,
    SelectOptionOptions, SelectOptionParams, SelectOptionResult, SelectOptionValue, Selector,
    SetChecked, SetCheckedOptions, SetCheckedParams, TextContent, TextContentParams,
    TextContentResult, WaitFor, WaitForParams, WaitForState,
};
use pwright_cap_locator::{ClickOptionsV2, ClickParamsV2, v2 as locator_v2};
#[cfg(feature = "touchscreen-input")]
use pwright_cap_touchscreen::{SelectorTapOptions, SelectorTapParams};
use pwright_cdp::connection::CdpError;
use pwright_cdp::{
    CdpClient, CdpEvent, CdpEventReceiver, CdpEventRecvError, CdpEventSubscriptionError,
    CdpEventTryRecvError, MouseButton,
};
use pwright_core::{
    CancellationReason, CommandCancellation, FailureKind, OperationContext, OperationError,
    ProductionResourceError, ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease,
    ResourceLedger, ResourceRequirements, ResourceScope, Service,
};
use serde_json::Value;

const DEFAULT_RETRY_DELAY: Duration = Duration::from_millis(50);
#[cfg(feature = "touchscreen-input")]
const TAP_TRIAL_INTERCEPTOR_SYMBOL: &str = "pwright.tap.trial-interceptor";
const MAX_CLICK_STEPS: u32 = 10_000;
#[cfg(feature = "retained-element-click")]
const MAX_RETAINED_CLICK_NATIVE_ID_BYTES: usize = 1024;
const MAX_FILL_VALUE_BYTES: usize = 1024 * 1024;
const FILL_COMMIT_BINDING: &str = "__pwrightFillCommit";
const MAX_ATTRIBUTE_NAME_BYTES: usize = 64 * 1024;
const MAX_ATTRIBUTE_RESULT_BYTES: usize = 1024 * 1024;
const MAX_TEXT_CONTENT_BYTES: usize = 1024 * 1024;
const MAX_INNER_HTML_BYTES: usize = 1024 * 1024;
const MAX_INNER_TEXT_BYTES: usize = 1024 * 1024;
const MAX_INPUT_VALUE_BYTES: usize = 1024 * 1024;
const MAX_SELECT_OPTION_ITEMS: usize = 1024;
const MAX_SELECT_OPTION_ITEM_BYTES: usize = 64 * 1024;
const MAX_SELECT_OPTION_TOTAL_BYTES: usize = 1024 * 1024;
// The default Node host admits at most 256 KiB per CDP event. Bound the
// JavaScript string before Runtime.bindingCalled so UTF-8 expansion and the
// protocol envelope still fit without retiring the connection.
#[cfg(feature = "select-option-navigation-commit")]
const MAX_SELECT_OPTION_RECEIPT_CODE_UNITS: usize = 64 * 1024;
#[cfg(feature = "select-option-navigation-commit")]
const MAX_SELECT_OPTION_RECEIPT_BYTES: usize = MAX_SELECT_OPTION_RECEIPT_CODE_UNITS * 3;
const MAX_SELECT_OPTION_INPUT_RETAINED_BYTES: usize = MAX_SELECT_OPTION_TOTAL_BYTES
    + MAX_SELECT_OPTION_ITEMS * std::mem::size_of::<SelectOptionValue>();
const MAX_ALL_TEXT_CONTENTS_ITEMS: usize = 4096;
const MAX_ALL_TEXT_CONTENTS_ITEM_BYTES: usize = 1024 * 1024;
const MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES: usize = 4 * 1024 * 1024;
const MAX_ALL_TEXT_CONTENTS_RETAINED_BYTES: usize = MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES
    + MAX_ALL_TEXT_CONTENTS_ITEMS * (std::mem::size_of::<String>() + std::mem::size_of::<Value>());
const MAX_ALL_INNER_TEXTS_ITEMS: usize = MAX_ALL_TEXT_CONTENTS_ITEMS;
const MAX_ALL_INNER_TEXTS_ITEM_BYTES: usize = MAX_ALL_TEXT_CONTENTS_ITEM_BYTES;
const MAX_ALL_INNER_TEXTS_TOTAL_BYTES: usize = MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES;
const MAX_ALL_INNER_TEXTS_RETAINED_BYTES: usize = MAX_ALL_INNER_TEXTS_TOTAL_BYTES
    + MAX_ALL_INNER_TEXTS_ITEMS * (std::mem::size_of::<String>() + std::mem::size_of::<Value>());
const MAX_COLLECTION_LOCATOR_RESULT_BYTES: usize =
    if MAX_ALL_INNER_TEXTS_RETAINED_BYTES > MAX_ALL_TEXT_CONTENTS_RETAINED_BYTES {
        MAX_ALL_INNER_TEXTS_RETAINED_BYTES
    } else {
        MAX_ALL_TEXT_CONTENTS_RETAINED_BYTES
    };
const MAX_TEXT_RESULT_BYTES: usize = if MAX_TEXT_CONTENT_BYTES > MAX_INNER_TEXT_BYTES {
    MAX_TEXT_CONTENT_BYTES
} else {
    MAX_INNER_TEXT_BYTES
};
const MAX_CONTENT_RESULT_BYTES: usize = if MAX_TEXT_RESULT_BYTES > MAX_INNER_HTML_BYTES {
    if MAX_TEXT_RESULT_BYTES > MAX_ATTRIBUTE_RESULT_BYTES {
        MAX_TEXT_RESULT_BYTES
    } else {
        MAX_ATTRIBUTE_RESULT_BYTES
    }
} else if MAX_INNER_HTML_BYTES > MAX_ATTRIBUTE_RESULT_BYTES {
    MAX_INNER_HTML_BYTES
} else {
    MAX_ATTRIBUTE_RESULT_BYTES
};
const MAX_SCALAR_LOCATOR_RESULT_BYTES: usize = if MAX_CONTENT_RESULT_BYTES > MAX_INPUT_VALUE_BYTES {
    MAX_CONTENT_RESULT_BYTES
} else {
    MAX_INPUT_VALUE_BYTES
};
const MAX_LOCATOR_RESULT_BYTES: usize =
    if MAX_SCALAR_LOCATOR_RESULT_BYTES > MAX_COLLECTION_LOCATOR_RESULT_BYTES {
        MAX_SCALAR_LOCATOR_RESULT_BYTES
    } else {
        MAX_COLLECTION_LOCATOR_RESULT_BYTES
    };
const MAX_SCALAR_LOCATOR_INPUT_BYTES: usize = if MAX_FILL_VALUE_BYTES > MAX_ATTRIBUTE_NAME_BYTES {
    MAX_FILL_VALUE_BYTES
} else {
    MAX_ATTRIBUTE_NAME_BYTES
};
const MAX_LOCATOR_INPUT_BYTES: usize =
    if MAX_SELECT_OPTION_INPUT_RETAINED_BYTES > MAX_SCALAR_LOCATOR_INPUT_BYTES {
        MAX_SELECT_OPTION_INPUT_RETAINED_BYTES
    } else {
        MAX_SCALAR_LOCATOR_INPUT_BYTES
    };
const MAX_ESCAPED_BYTES_PER_INPUT_BYTE: usize = 6;
const MAX_ACTION_WRAPPER_BYTES: usize = 32 * 1024;
const MAX_GENERATED_BYTES_PER_SELECT_OPTION: usize = 96;
const MAX_STRING_ALLOCATION_OVERLAP: usize = 4;

const MAX_SELECTOR_LABEL_BYTES: usize = MAX_COMPILED_SELECTOR_BYTES;
const MAX_FILL_SCRIPT_BYTES: usize = MAX_COMPILED_SELECTOR_BYTES
    + MAX_FILL_VALUE_BYTES * MAX_ESCAPED_BYTES_PER_INPUT_BYTE
    + MAX_ACTION_WRAPPER_BYTES;
const MAX_SELECT_OPTION_SCRIPT_BYTES: usize = MAX_COMPILED_SELECTOR_BYTES
    + MAX_SELECT_OPTION_TOTAL_BYTES * MAX_ESCAPED_BYTES_PER_INPUT_BYTE
    + MAX_SELECT_OPTION_ITEMS * MAX_GENERATED_BYTES_PER_SELECT_OPTION
    + MAX_ACTION_WRAPPER_BYTES;
const MAX_ATTRIBUTE_SCRIPT_BYTES: usize = MAX_COMPILED_SELECTOR_BYTES
    + MAX_ATTRIBUTE_NAME_BYTES * MAX_ESCAPED_BYTES_PER_INPUT_BYTE
    + MAX_ACTION_WRAPPER_BYTES;
const MAX_FOCUS_SCRIPT_BYTES: usize = MAX_COMPILED_SELECTOR_BYTES + MAX_ACTION_WRAPPER_BYTES;
const MAX_NON_FILL_ACTION_SCRIPT_BYTES: usize =
    if MAX_ATTRIBUTE_SCRIPT_BYTES > MAX_FOCUS_SCRIPT_BYTES {
        MAX_ATTRIBUTE_SCRIPT_BYTES
    } else {
        MAX_FOCUS_SCRIPT_BYTES
    };
const MAX_NON_SELECT_ACTION_SCRIPT_BYTES: usize =
    if MAX_FILL_SCRIPT_BYTES > MAX_NON_FILL_ACTION_SCRIPT_BYTES {
        MAX_FILL_SCRIPT_BYTES
    } else {
        MAX_NON_FILL_ACTION_SCRIPT_BYTES
    };
const MAX_ACTION_SCRIPT_BYTES: usize =
    if MAX_SELECT_OPTION_SCRIPT_BYTES > MAX_NON_SELECT_ACTION_SCRIPT_BYTES {
        MAX_SELECT_OPTION_SCRIPT_BYTES
    } else {
        MAX_NON_SELECT_ACTION_SCRIPT_BYTES
    };
const _: () = assert!(MAX_ACTION_SCRIPT_BYTES >= MAX_FOCUS_SCRIPT_BYTES);
const MAX_LOCATOR_ACTION_RETAINED_BYTES: usize = MAX_SELECTOR_STRING_BYTES
    + MAX_LOCATOR_INPUT_BYTES
    + MAX_LOCATOR_RESULT_BYTES
    + MAX_STRING_ALLOCATION_OVERLAP
        * (MAX_SELECTOR_LABEL_BYTES + MAX_ACTION_SCRIPT_BYTES + MAX_SELECTOR_RUNTIME_INSTALL_BYTES);

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SelectorRuntimeMetrics {
    pub installations: u64,
    pub actionability_evaluations: u64,
    pub hover_evaluations: u64,
    pub focus_evaluations: u64,
    pub snapshot_evaluations: u64,
    pub state_evaluations: u64,
    pub content_evaluations: u64,
    pub attribute_evaluations: u64,
    pub input_value_evaluations: u64,
    pub all_inner_texts_evaluations: u64,
    pub all_text_contents_evaluations: u64,
    pub wait_evaluations: u64,
    pub missing_after_navigation: u64,
}

// One utility-world numeric context ID in both the current and atomically
// prepared replacement route. This is separate from retained native strings.
const CONTEXT_ROUTE_NUMERIC_ID_BYTES: u64 = 2 * std::mem::size_of::<i64>() as u64;

/// Production resource declaration for active Chromium locator actions.
///
/// The bound includes compact command strings, selector labels, generated
/// action JavaScript, runtime-install JavaScript, and conservative overlap
/// while nested formatted strings are replaced. Selector nodes themselves are
/// separately limited by count and depth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpLocatorProductionProfile {
    input_policy: CdpInputPolicy,
    resource_scope: ResourceScope,
    max_concurrent_actions: usize,
    max_context_routes: usize,
    max_native_id_bytes: usize,
    maximum_retained_bytes_per_action: u64,
    maximum_route_retained_bytes: u64,
    requirements: ResourceRequirements,
}

impl CdpLocatorProductionProfile {
    pub fn new(resource_scope: ResourceScope, max_concurrent_actions: usize) -> Self {
        let actions = u64::try_from(max_concurrent_actions).unwrap_or(u64::MAX);
        let maximum_retained_bytes_per_action =
            u64::try_from(MAX_LOCATOR_ACTION_RETAINED_BYTES).unwrap_or(u64::MAX);
        let bytes = actions.saturating_mul(maximum_retained_bytes_per_action);
        let requirements = if max_concurrent_actions == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::SelectorBytes, bytes, bytes)
                .with_metadata(1, max_concurrent_actions)
        };
        Self {
            input_policy: CdpInputPolicy::Exclusive,
            resource_scope,
            max_concurrent_actions,
            max_context_routes: 0,
            max_native_id_bytes: 0,
            maximum_retained_bytes_per_action,
            maximum_route_retained_bytes: 0,
            requirements,
        }
    }

    /// Selects Page input scheduling without changing the wire protocol. The
    /// default retains whole-action exclusion and cancellation key recovery.
    pub const fn with_input_policy(mut self, policy: CdpInputPolicy) -> Self {
        self.input_policy = policy;
        self
    }

    pub const fn input_policy(self) -> CdpInputPolicy {
        self.input_policy
    }

    /// Adds persistent storage for Frame-scoped locator routes. Six native
    /// strings cover the route identity, native Frame ID, unique context ID,
    /// and the target-local CDP client's copies. A second set covers atomic
    /// route replacement when an execution world is recreated. Both generations'
    /// utility-world numeric context IDs have a separate size-derived allowance.
    pub fn with_context_routes(
        mut self,
        max_context_routes: usize,
        max_native_id_bytes: usize,
    ) -> Self {
        self.max_context_routes = max_context_routes;
        self.max_native_id_bytes = max_native_id_bytes;
        self.maximum_route_retained_bytes = u64::try_from(max_context_routes)
            .unwrap_or(u64::MAX)
            .saturating_mul(
                u64::try_from(max_native_id_bytes)
                    .unwrap_or(u64::MAX)
                    .saturating_mul(12)
                    .saturating_add(CONTEXT_ROUTE_NUMERIC_ID_BYTES),
            );
        let actions = u64::try_from(self.max_concurrent_actions).unwrap_or(u64::MAX);
        let selector_bytes = self
            .maximum_retained_bytes_per_action
            .saturating_mul(actions);
        let mut requirements = ResourceRequirements::none();
        if selector_bytes > 0 {
            requirements = requirements.with_class(
                ResourceClass::SelectorBytes,
                selector_bytes,
                selector_bytes,
            );
        }
        if self.maximum_route_retained_bytes > 0 {
            requirements = requirements.with_class(
                ResourceClass::BackendStateBytes,
                self.maximum_route_retained_bytes,
                self.maximum_route_retained_bytes,
            );
        }
        self.requirements = if selector_bytes == 0 && self.maximum_route_retained_bytes == 0 {
            ResourceRequirements::none()
        } else {
            requirements.with_metadata(
                1,
                self.max_concurrent_actions
                    .saturating_add(usize::from(self.maximum_route_retained_bytes > 0)),
            )
        };
        self
    }

    /// Removes persistent route storage while retaining the locator action
    /// budget. Composition roots use this when another installed capability
    /// owns the shared utility-world route identity.
    pub fn without_context_routes(mut self) -> Self {
        self.max_context_routes = 0;
        self.max_native_id_bytes = 0;
        self.maximum_route_retained_bytes = 0;
        let actions = u64::try_from(self.max_concurrent_actions).unwrap_or(u64::MAX);
        let selector_bytes = self
            .maximum_retained_bytes_per_action
            .saturating_mul(actions);
        self.requirements = if selector_bytes == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(ResourceClass::SelectorBytes, selector_bytes, selector_bytes)
                .with_metadata(1, self.max_concurrent_actions)
        };
        self
    }

    pub const fn resource_scope(self) -> ResourceScope {
        self.resource_scope
    }

    pub const fn max_concurrent_actions(self) -> usize {
        self.max_concurrent_actions
    }

    pub const fn max_context_routes(self) -> usize {
        self.max_context_routes
    }

    pub const fn max_native_id_bytes(self) -> usize {
        self.max_native_id_bytes
    }

    pub const fn maximum_retained_bytes_per_action(self) -> u64 {
        self.maximum_retained_bytes_per_action
    }

    pub const fn maximum_route_retained_bytes(self) -> u64 {
        self.maximum_route_retained_bytes
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Default)]
struct SelectorRuntimeCounters {
    installations: AtomicU64,
    actionability_evaluations: AtomicU64,
    hover_evaluations: AtomicU64,
    focus_evaluations: AtomicU64,
    snapshot_evaluations: AtomicU64,
    state_evaluations: AtomicU64,
    content_evaluations: AtomicU64,
    attribute_evaluations: AtomicU64,
    input_value_evaluations: AtomicU64,
    all_inner_texts_evaluations: AtomicU64,
    all_text_contents_evaluations: AtomicU64,
    wait_evaluations: AtomicU64,
    missing_after_navigation: AtomicU64,
}

fn selector_runtime_metrics(counters: &SelectorRuntimeCounters) -> SelectorRuntimeMetrics {
    SelectorRuntimeMetrics {
        installations: counters.installations.load(Ordering::Relaxed),
        actionability_evaluations: counters.actionability_evaluations.load(Ordering::Relaxed),
        hover_evaluations: counters.hover_evaluations.load(Ordering::Relaxed),
        focus_evaluations: counters.focus_evaluations.load(Ordering::Relaxed),
        snapshot_evaluations: counters.snapshot_evaluations.load(Ordering::Relaxed),
        state_evaluations: counters.state_evaluations.load(Ordering::Relaxed),
        content_evaluations: counters.content_evaluations.load(Ordering::Relaxed),
        attribute_evaluations: counters.attribute_evaluations.load(Ordering::Relaxed),
        input_value_evaluations: counters.input_value_evaluations.load(Ordering::Relaxed),
        all_inner_texts_evaluations: counters.all_inner_texts_evaluations.load(Ordering::Relaxed),
        all_text_contents_evaluations: counters
            .all_text_contents_evaluations
            .load(Ordering::Relaxed),
        wait_evaluations: counters.wait_evaluations.load(Ordering::Relaxed),
        missing_after_navigation: counters.missing_after_navigation.load(Ordering::Relaxed),
    }
}

#[derive(Clone)]
struct LocatorResourceBudget {
    ledger: ResourceLedger,
    profile: CdpLocatorProductionProfile,
    active_actions: Arc<AtomicUsize>,
}

struct LocatorActionLease {
    resource: Option<ResourceLease>,
    active_actions: Arc<AtomicUsize>,
}

impl Drop for LocatorActionLease {
    fn drop(&mut self) {
        drop(self.resource.take());
        let previous = self.active_actions.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0);
    }
}

/// Chromium implementation of portable locator actions.
///
/// It retains no resolved node between attempts. Every retry evaluates the
/// selector again, matching Locator's lazy behavior across DOM replacement.
#[derive(Clone)]
pub struct CdpLocatorService {
    client: Arc<dyn CdpClient>,
    input: CdpInputCoordinator,
    unique_context_id: Option<Arc<str>>,
    #[cfg(feature = "fill-navigation-commit")]
    fill_commit_execution_context_id: Option<i64>,
    #[cfg(feature = "select-option-navigation-commit")]
    select_option_commit_execution_context_id: Option<i64>,
    #[cfg(feature = "select-option-navigation-commit")]
    select_option_commit_frame_id: Option<Arc<str>>,
    retry_delay: Duration,
    runtime_ready: Arc<tokio::sync::OnceCell<()>>,
    runtime_counters: Arc<SelectorRuntimeCounters>,
    resources: Option<LocatorResourceBudget>,
    keyboard_binding: Option<KeyboardBinding>,
    action_progress: Arc<Mutex<Option<ActionabilityProgress>>>,
}

#[derive(Clone)]
struct KeyboardBinding {
    client: Arc<dyn CdpClient>,
    interceptor: Arc<dyn pwright_backend_cdp_input::CdpKeyDownInterceptor>,
    handoff: Option<Arc<dyn LocatorInputHandoff>>,
}

/// Synchronous, caller-selected lifetime transition after Frame focus succeeds
/// and before Page input mutates logical or native state. No runtime dependency.
pub trait LocatorInputHandoff: Send + Sync {
    fn handoff(&self) -> Result<(), OperationError<LocatorErrorDetails>>;
}

/// Creates per-target locator services with one shared production action
/// counter while keeping selector-runtime installation state target-local.
#[derive(Clone)]
pub struct CdpLocatorServiceFactory {
    retry_delay: Duration,
    resources: LocatorResourceBudget,
    runtime_counters: Arc<SelectorRuntimeCounters>,
}

impl CdpLocatorServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpLocatorProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        Self::with_retry_delay_and_production_resources(DEFAULT_RETRY_DELAY, profile, resources)
    }

    pub fn with_retry_delay_and_production_resources(
        retry_delay: Duration,
        profile: CdpLocatorProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        let ledger = resources.claim(profile.requirements())?;
        Ok(Self {
            retry_delay,
            resources: LocatorResourceBudget {
                ledger,
                profile,
                active_actions: Arc::new(AtomicUsize::new(0)),
            },
            runtime_counters: Arc::new(SelectorRuntimeCounters::default()),
        })
    }

    pub fn bind(&self, client: Arc<dyn CdpClient>) -> CdpLocatorService {
        self.bind_with_input(client, self.create_input_coordinator())
    }

    pub fn create_input_coordinator(&self) -> CdpInputCoordinator {
        CdpInputCoordinator::with_policy(self.resources.profile.input_policy())
    }

    pub fn bind_with_input(
        &self,
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
    ) -> CdpLocatorService {
        CdpLocatorService::from_parts_with_counters(
            client,
            input,
            None,
            self.retry_delay,
            Some(self.resources.clone()),
            Arc::clone(&self.runtime_counters),
        )
    }

    pub fn bind_unique_context(
        &self,
        client: Arc<dyn CdpClient>,
        unique_context_id: String,
    ) -> CdpLocatorService {
        self.bind_unique_context_with_input(
            client,
            unique_context_id,
            self.create_input_coordinator(),
        )
    }

    pub fn bind_unique_context_with_input(
        &self,
        client: Arc<dyn CdpClient>,
        mut unique_context_id: String,
        input: CdpInputCoordinator,
    ) -> CdpLocatorService {
        unique_context_id.shrink_to_fit();
        self.bind_shared_unique_context_with_input(client, Arc::from(unique_context_id), input)
    }

    /// Binds a route to an immutable context identity that may be shared by
    /// multiple internal services without allocating another string copy.
    pub fn bind_shared_unique_context_with_input(
        &self,
        client: Arc<dyn CdpClient>,
        unique_context_id: Arc<str>,
        input: CdpInputCoordinator,
    ) -> CdpLocatorService {
        CdpLocatorService::from_parts_with_counters(
            client,
            input,
            Some(unique_context_id),
            self.retry_delay,
            Some(self.resources.clone()),
            Arc::clone(&self.runtime_counters),
        )
    }

    pub fn selector_runtime_metrics(&self) -> SelectorRuntimeMetrics {
        selector_runtime_metrics(&self.runtime_counters)
    }

    pub const fn profile(&self) -> CdpLocatorProductionProfile {
        self.resources.profile
    }

    pub fn reserve_context_routes(&self) -> Result<Option<ResourceLease>, ResourceError> {
        let bytes = self.resources.profile.maximum_route_retained_bytes();
        if bytes == 0 {
            return Ok(None);
        }
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

impl CdpLocatorService {
    /// Explicit Page keyboard binding, separate from this Frame's selector client.
    /// The caller retains native identity, provider admission and retirement.
    /// Only press/sequential typing select this binding; pointer geometry stays
    /// on its existing route. Input ownership must be the same Page coordinator.
    pub fn with_keyboard_binding(
        mut self,
        client: Arc<dyn CdpClient>,
        input: &CdpInputCoordinator,
        interceptor: Arc<dyn pwright_backend_cdp_input::CdpKeyDownInterceptor>,
    ) -> Result<Self, pwright_cdp::connection::CdpError> {
        if self.keyboard_binding.is_some()
            || !self.input.shares_state_with(input)
            || input.policy() != pwright_backend_cdp_input::CdpInputPolicy::Interleaved
        {
            return Err(pwright_cdp::connection::CdpError::Other(
                "Locator keyboard binding is replaced or belongs to another Page".into(),
            ));
        }
        self.keyboard_binding = Some(KeyboardBinding {
            client,
            interceptor,
            handoff: None,
        });
        Ok(self)
    }

    async fn keyboard_transaction(
        &self,
    ) -> Result<CdpInputTransaction, OperationError<LocatorErrorDetails>> {
        let client = self.keyboard_client();
        let mut transaction = self
            .input
            .begin_transaction(client.clone(), None)
            .await
            .map_err(map_cdp_error)?;
        if let Some(binding) = &self.keyboard_binding {
            transaction
                .set_key_down_interceptor(binding.interceptor.clone())
                .map_err(map_cdp_error)?;
        }
        Ok(transaction)
    }

    fn keyboard_client(&self) -> &Arc<dyn CdpClient> {
        self.keyboard_binding
            .as_ref()
            .map_or(&self.client, |binding| &binding.client)
    }

    pub fn with_keyboard_handoff(
        mut self,
        handoff: Arc<dyn LocatorInputHandoff>,
    ) -> Result<Self, pwright_cdp::connection::CdpError> {
        let binding = self.keyboard_binding.as_mut().ok_or_else(|| {
            pwright_cdp::connection::CdpError::Other(
                "keyboard binding is required before lifetime handoff selection".into(),
            )
        })?;
        if binding.handoff.is_some() {
            return Err(pwright_cdp::connection::CdpError::Other(
                "keyboard lifetime handoff is already selected".into(),
            ));
        }
        binding.handoff = Some(handoff);
        Ok(self)
    }

    fn handoff_keyboard_lifetime(&self) -> Result<(), OperationError<LocatorErrorDetails>> {
        match self
            .keyboard_binding
            .as_ref()
            .and_then(|binding| binding.handoff.as_ref())
        {
            Some(handoff) => handoff.handoff(),
            None => Ok(()),
        }
    }

    pub fn new(client: Arc<dyn CdpClient>) -> Self {
        Self::from_parts(client, DEFAULT_RETRY_DELAY, None)
    }

    pub fn with_retry_delay(client: Arc<dyn CdpClient>, retry_delay: Duration) -> Self {
        Self::from_parts(client, retry_delay, None)
    }

    pub fn new_with_production_resources(
        client: Arc<dyn CdpClient>,
        profile: CdpLocatorProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        Self::with_retry_delay_and_production_resources(
            client,
            DEFAULT_RETRY_DELAY,
            profile,
            resources,
        )
    }

    pub fn with_retry_delay_and_production_resources(
        client: Arc<dyn CdpClient>,
        retry_delay: Duration,
        profile: CdpLocatorProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        CdpLocatorServiceFactory::with_retry_delay_and_production_resources(
            retry_delay,
            profile,
            resources,
        )
        .map(|factory| factory.bind(client))
    }

    pub fn selector_runtime_metrics(&self) -> SelectorRuntimeMetrics {
        selector_runtime_metrics(&self.runtime_counters)
    }

    pub fn unique_context_id(&self) -> Option<&str> {
        self.unique_context_id.as_deref()
    }

    pub fn retained_unique_context_bytes(&self) -> usize {
        self.unique_context_id
            .as_ref()
            .map_or(0, |value| value.len())
    }

    /// Returns the attached target-scoped CDP client for aggregate-backend
    /// compatibility adapters.
    #[doc(hidden)]
    pub fn native_cdp_client(&self) -> &(dyn CdpClient + 'static) {
        self.client.as_ref()
    }

    /// Reports whether two Locator routes coordinate input as members of the
    /// same Page.
    pub fn shares_input_state_with(&self, other: &Self) -> bool {
        self.input.shares_state_with(&other.input)
    }

    /// Reports whether this Locator route uses the supplied Page input owner.
    pub fn uses_input_coordinator(&self, input: &CdpInputCoordinator) -> bool {
        self.input.shares_state_with(input)
    }

    fn from_parts(
        client: Arc<dyn CdpClient>,
        retry_delay: Duration,
        resources: Option<LocatorResourceBudget>,
    ) -> Self {
        Self::from_parts_with_counters(
            client,
            CdpInputCoordinator::new(),
            None,
            retry_delay,
            resources,
            Arc::new(SelectorRuntimeCounters::default()),
        )
    }

    fn from_parts_with_counters(
        client: Arc<dyn CdpClient>,
        input: CdpInputCoordinator,
        unique_context_id: Option<Arc<str>>,
        retry_delay: Duration,
        resources: Option<LocatorResourceBudget>,
        runtime_counters: Arc<SelectorRuntimeCounters>,
    ) -> Self {
        Self {
            client,
            input,
            unique_context_id,
            #[cfg(feature = "fill-navigation-commit")]
            fill_commit_execution_context_id: None,
            #[cfg(feature = "select-option-navigation-commit")]
            select_option_commit_execution_context_id: None,
            #[cfg(feature = "select-option-navigation-commit")]
            select_option_commit_frame_id: None,
            retry_delay,
            runtime_ready: Arc::new(tokio::sync::OnceCell::new()),
            runtime_counters,
            resources,
            keyboard_binding: None,
            action_progress: Arc::new(Mutex::new(None)),
        }
    }

    /// Opt a route into direct-fill commit receipts only when composition has
    /// selected its exact isolated utility execution world.
    #[cfg(feature = "fill-navigation-commit")]
    pub fn with_fill_commit_execution_context(mut self, execution_context_id: i64) -> Self {
        self.fill_commit_execution_context_id = (execution_context_id > 0
            && self.unique_context_id.is_some())
        .then_some(execution_context_id);
        self
    }

    /// Enable select-option recovery only in the explicitly selected utility world.
    #[cfg(feature = "select-option-navigation-commit")]
    pub fn with_select_option_commit_route(
        mut self,
        execution_context_id: i64,
        frame_id: Arc<str>,
    ) -> Self {
        let valid = execution_context_id > 0
            && self.unique_context_id.is_some()
            && !frame_id.is_empty()
            && frame_id.len() <= 1024;
        self.select_option_commit_execution_context_id = valid.then_some(execution_context_id);
        self.select_option_commit_frame_id = valid.then_some(frame_id);
        self
    }

    fn clear_action_progress(&self) {
        if let Ok(mut progress) = self.action_progress.lock() {
            *progress = None;
        }
    }

    fn record_action_progress(&self, progress: ActionabilityProgress) {
        if let Ok(mut current) = self.action_progress.lock() {
            *current = Some(progress);
        }
    }

    fn take_action_progress(&self) -> Option<ActionabilityProgress> {
        self.action_progress.lock().ok()?.take()
    }

    fn reserve_locator_action(
        &self,
        selector: &Selector,
        fill_value: Option<&String>,
    ) -> Result<Option<LocatorActionLease>, OperationError<LocatorErrorDetails>> {
        let Some(resources) = &self.resources else {
            return Ok(None);
        };
        let storage = measure_selector(selector)?;
        if storage.string_capacity() > MAX_SELECTOR_STRING_BYTES {
            return Err(selector_resource_error(
                "selector string capacities exceed 65536 bytes",
            ));
        }
        if let Some(value) = fill_value {
            if value.len() > MAX_FILL_VALUE_BYTES {
                return Err(fill_value_resource_error(
                    "fill value exceeds the 1048576-byte backend limit",
                ));
            }
            if value.capacity() > MAX_FILL_VALUE_BYTES {
                return Err(selector_resource_error(
                    "fill value capacity exceeds the 1048576-byte backend limit",
                ));
            }
        }
        self.reserve_action_resource(resources)
    }

    #[cfg(feature = "retained-element-click")]
    fn reserve_retained_click_action(
        &self,
    ) -> Result<Option<LocatorActionLease>, OperationError<LocatorErrorDetails>> {
        self.resources.as_ref().map_or(Ok(None), |resources| {
            self.reserve_action_resource(resources)
        })
    }

    fn reserve_action_resource(
        &self,
        resources: &LocatorResourceBudget,
    ) -> Result<Option<LocatorActionLease>, OperationError<LocatorErrorDetails>> {
        let limit = resources.profile.max_concurrent_actions();
        if resources
            .active_actions
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < limit).then_some(current + 1)
            })
            .is_err()
        {
            return Err(selector_resource_error(
                "locator action concurrency limit was reached",
            ));
        }
        let resource = match resources.ledger.reserve(
            resources.profile.resource_scope(),
            ResourceClass::SelectorBytes,
            resources.profile.maximum_retained_bytes_per_action(),
        ) {
            Ok(resource) => resource,
            Err(error) => {
                let previous = resources.active_actions.fetch_sub(1, Ordering::AcqRel);
                debug_assert!(previous > 0);
                return Err(map_locator_resource_error(error));
            }
        };
        Ok(Some(LocatorActionLease {
            resource: Some(resource),
            active_actions: resources.active_actions.clone(),
        }))
    }

    async fn execute_click_v2_pointer(
        &self,
        point: ClickPoint,
        options: &ClickOptionsV2,
        steps: std::num::NonZeroU32,
    ) -> Result<(), OperationError<LocatorErrorDetails>> {
        let navigation = self.prepare_click_v2_navigation(options).await?;
        self.dispatch_click_v2_pointer(point, options, steps)
            .await?;
        if let Some(navigation) = navigation {
            navigation.finish(self.client.as_ref()).await?;
        }
        Ok(())
    }

    async fn prepare_click_v2_navigation(
        &self,
        options: &ClickOptionsV2,
    ) -> Result<Option<ClickNavigationBarrier>, OperationError<LocatorErrorDetails>> {
        let wait_after = !options.no_wait_after.unwrap_or(false) && !options.trial.unwrap_or(false);
        if wait_after {
            ClickNavigationBarrier::prepare(self.client.as_ref())
                .await
                .map(Some)
        } else {
            Ok(None)
        }
    }

    async fn dispatch_click_v2_pointer(
        &self,
        point: ClickPoint,
        options: &ClickOptionsV2,
        steps: std::num::NonZeroU32,
    ) -> Result<(), OperationError<LocatorErrorDetails>> {
        let requested_modifiers = options
            .modifiers
            .map(|modifiers| resolve_modifiers(modifiers, point.is_mac));
        let mut transaction = self
            .input
            .begin_transaction(Arc::clone(&self.client), requested_modifiers)
            .await
            .map_err(map_cdp_error)?;
        let action = run_click_v2_pointer_action(&mut transaction, point, options, steps).await;
        let cleanup = transaction.finish().await;
        if let Err(error) = cleanup {
            return Err(map_cdp_error(error).reclassify(FailureKind::Cleanup));
        }
        action.map_err(map_cdp_error)?;
        Ok(())
    }

    #[cfg(feature = "retained-element-click")]
    pub fn new_retained_click_press(
        &self,
    ) -> Result<CdpOwnedMousePress, OperationError<LocatorErrorDetails>> {
        self.input.new_owned_mouse_press().map_err(map_cdp_error)
    }

    #[cfg(feature = "retained-element-click")]
    async fn dispatch_retained_click_v2_pointer<ParentHit, ParentHitFuture>(
        &self,
        route: RetainedClickPointerRoute<'_>,
        options: &ClickOptionsV2,
        identity: RetainedClickIdentity<'_>,
        press: &CdpOwnedMousePress,
        parent_hit: &ParentHit,
        validate_world: &(impl Fn() -> Result<(), OperationError<LocatorErrorDetails>> + Send + Sync),
    ) -> Result<bool, OperationError<LocatorErrorDetails>>
    where
        ParentHit: Fn(f64, f64) -> ParentHitFuture + Send + Sync,
        ParentHitFuture: Future<Output = Result<bool, OperationError<LocatorErrorDetails>>> + Send,
    {
        let RetainedClickPointerRoute {
            local_point,
            dispatch_point,
            child,
            steps,
        } = route;
        let requested_modifiers = options
            .modifiers
            .map(|modifiers| resolve_modifiers(modifiers, local_point.is_mac));
        let mut transaction = self
            .input
            .begin_transaction(Arc::clone(&self.client), requested_modifiers)
            .await
            .map_err(map_cdp_error)?;
        transaction
            .set_owned_mouse_press(press)
            .map_err(map_cdp_error)?;
        let action = async {
            validate_world()?;
            transaction
                .move_mouse_in_steps(dispatch_point.x, dispatch_point.y, steps)
                .await
                .map_err(map_cdp_error)?;
            validate_world()?;
            // Hover can itself move or replace the target. Recheck the same
            // native object in its original world after movement, before any
            // pointer press. A moved point retries; a retired world fails.
            let response = self
                .client
                .runtime_call_function_by_value_in_unique_context(
                    identity.unique_context_id,
                    &retained_click_route_check_script(
                        local_point,
                        options.force.unwrap_or(false),
                        child.is_some(),
                    ),
                    vec![serde_json::json!({"objectId": identity.native_object_id})],
                )
                .await
                .map_err(map_cdp_error)?;
            validate_world()?;
            if !verify_retained_click_route(response)? {
                return Ok(false);
            }
            if let Some((child, viewport)) = child {
                let current = retained_click_child::translate(
                    self.client.as_ref(),
                    child,
                    local_point,
                    viewport,
                )
                .await?;
                validate_world()?;
                if (current.x - dispatch_point.x).abs() > 0.25
                    || (current.y - dispatch_point.y).abs() > 0.25
                {
                    return Ok(false);
                }
                if !options.force.unwrap_or(false) && !parent_hit(current.x, current.y).await? {
                    return Ok(false);
                }
                validate_world()?;
            }
            let button = map_button(options.button.unwrap_or(PortableMouseButton::Left));
            let click_count = options.click_count.map_or(1, |count| count.get());
            let delay = options.delay.unwrap_or(Duration::ZERO);
            for count in 1..=click_count {
                // Each prior press has been released before another world
                // check can fail; never leave a committed press held here.
                validate_world()?;
                transaction
                    .mouse_down_checked(button, i32::from(count), validate_world)
                    .await
                    .map_err(|error| match error {
                        CdpCheckedMouseDownError::Check(error) => error,
                        CdpCheckedMouseDownError::Input(error) => map_cdp_error(error),
                    })?;
                if !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
                transaction.mouse_up().await.map_err(map_cdp_error)?;
                if count < click_count && !delay.is_zero() {
                    tokio::time::sleep(delay).await;
                }
            }
            Ok(true)
        }
        .await;
        let cleanup = transaction.finish().await;
        if let Err(error) = cleanup {
            return Err(map_cdp_error(error).reclassify(FailureKind::Cleanup));
        }
        action
    }

    /// Click an exact retained Element in this Page's top-level Frame.
    ///
    /// The caller must keep its native handle lease alive until this future
    /// settles, supply the handle's original execution-context identity, and
    /// apply a bounded runtime deadline plus the cancellation cleanup hook.
    /// Child Frames, including OOPIFs, fail before input: their local CSS
    /// coordinates cannot be dispatched as top-level Page coordinates.
    #[cfg(feature = "retained-element-click")]
    pub async fn click_retained_top_level_element(
        &self,
        native_object_id: &str,
        unique_context_id: &str,
        options: ClickOptionsV2,
        press: &CdpOwnedMousePress,
        validate_world: impl Fn() -> Result<(), OperationError<LocatorErrorDetails>> + Send + Sync,
    ) -> Result<(), OperationError<LocatorErrorDetails>> {
        let diagnostics = RetainedClickDiagnostics::default();
        self.click_retained_top_level_element_with_diagnostics(
            native_object_id,
            unique_context_id,
            options,
            press,
            &diagnostics,
            validate_world,
        )
        .await
    }

    /// Same exact-element operation with invocation-owned bounded progress.
    /// The caller retains `diagnostics` through cancellation cleanup so a
    /// deadline can report only observations from this invocation.
    #[cfg(feature = "retained-element-click")]
    pub async fn click_retained_top_level_element_with_diagnostics(
        &self,
        native_object_id: &str,
        unique_context_id: &str,
        options: ClickOptionsV2,
        press: &CdpOwnedMousePress,
        diagnostics: &RetainedClickDiagnostics,
        validate_world: impl Fn() -> Result<(), OperationError<LocatorErrorDetails>> + Send + Sync,
    ) -> Result<(), OperationError<LocatorErrorDetails>> {
        self.click_retained_element_with_diagnostics(
            RetainedClickTarget {
                identity: RetainedClickIdentity {
                    native_object_id,
                    unique_context_id,
                },
                child: None,
            },
            options,
            press,
            diagnostics,
            |_, _| async { Ok(true) },
            validate_world,
        )
        .await
    }

    /// Exact retained Element click through one same-session child Frame.
    /// Deeper Frame chains and OOPIF routes fail closed before pointer input.
    #[cfg(feature = "retained-element-click")]
    pub async fn click_retained_one_link_child_element_with_diagnostics<
        ParentHit,
        ParentHitFuture,
    >(
        &self,
        target: RetainedClickChildTarget<'_>,
        options: ClickOptionsV2,
        press: &CdpOwnedMousePress,
        diagnostics: &RetainedClickDiagnostics,
        parent_hit: ParentHit,
        validate_world: impl Fn() -> Result<(), OperationError<LocatorErrorDetails>> + Send + Sync,
    ) -> Result<(), OperationError<LocatorErrorDetails>>
    where
        ParentHit: Fn(f64, f64) -> ParentHitFuture + Send + Sync,
        ParentHitFuture: Future<Output = Result<bool, OperationError<LocatorErrorDetails>>> + Send,
    {
        self.click_retained_element_with_diagnostics(
            RetainedClickTarget {
                identity: RetainedClickIdentity {
                    native_object_id: target.native_object_id,
                    unique_context_id: target.unique_context_id,
                },
                child: Some(retained_click_child::ChildFrame {
                    frame_id: target.child_frame_id,
                    parent_frame_id: target.parent_frame_id,
                }),
            },
            options,
            press,
            diagnostics,
            parent_hit,
            validate_world,
        )
        .await
    }

    #[cfg(feature = "retained-element-click")]
    async fn click_retained_element_with_diagnostics<ParentHit, ParentHitFuture>(
        &self,
        target: RetainedClickTarget<'_>,
        options: ClickOptionsV2,
        press: &CdpOwnedMousePress,
        diagnostics: &RetainedClickDiagnostics,
        parent_hit: ParentHit,
        validate_world: impl Fn() -> Result<(), OperationError<LocatorErrorDetails>> + Send + Sync,
    ) -> Result<(), OperationError<LocatorErrorDetails>>
    where
        ParentHit: Fn(f64, f64) -> ParentHitFuture + Send + Sync,
        ParentHitFuture: Future<Output = Result<bool, OperationError<LocatorErrorDetails>>> + Send,
    {
        let RetainedClickTarget { identity, child } = target;
        let RetainedClickIdentity {
            native_object_id,
            unique_context_id,
        } = identity;
        validate_retained_click_native_id(native_object_id, "object ID")?;
        validate_retained_click_native_id(unique_context_id, "context ID")?;
        if let Some(child) = child {
            validate_retained_click_native_id(child.frame_id, "Frame ID")?;
            validate_retained_click_native_id(child.parent_frame_id, "parent Frame ID")?;
        }
        validate_world()?;
        let steps = options.steps.unwrap_or(std::num::NonZeroU32::MIN);
        if steps.get() > MAX_CLICK_STEPS {
            return Err(locator_resource_error(&format!(
                "click steps exceed the {MAX_CLICK_STEPS}-event backend limit"
            )));
        }
        let _resources = self.reserve_retained_click_action()?;
        async {
            if let Some(child) = child {
                retained_click_child::verify_one_link_route(self.client.as_ref(), child).await?;
            } else {
                let tree = self
                    .client
                    .page_get_frame_tree()
                    .await
                    .map_err(map_cdp_error)?;
                if tree.frame.parent_id.is_some() {
                    return Err(retained_click_target_error(
                        "retained Element click requires a top-level Page target",
                    ));
                }
            }
            let script = retained_click_actionability_script(&options, child.is_some());
            loop {
                validate_world()?;
                self.runtime_counters
                    .actionability_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self
                    .client
                    .runtime_call_function_by_value_in_unique_context(
                        unique_context_id,
                        &script,
                        vec![serde_json::json!({"objectId": native_object_id})],
                    )
                    .await
                    .map_err(map_cdp_error)?;
                validate_world()?;
                let child_viewport =
                    if child.is_some() && retained_click_status(&response)? == "ready" {
                        Some(retained_click_child::Viewport::from_actionability(
                            &response,
                        )?)
                    } else {
                        None
                    };
                let point = match parse_retained_click_attempt(
                    response,
                    options.force.unwrap_or(false),
                    diagnostics,
                )? {
                    ActionAttempt::Ready(point) => {
                        diagnostics.clear();
                        point
                    }
                    ActionAttempt::Retry(_) => {
                        tokio::time::sleep(self.retry_delay).await;
                        continue;
                    }
                    ActionAttempt::RuntimeMissing => {
                        return Err(invalid_response(
                            "retained Element click could not install its selector runtime",
                        ));
                    }
                };
                let dispatch_point = if let Some(child) = child {
                    let viewport = child_viewport.expect("ready child click has viewport");
                    let translated = retained_click_child::translate(
                        self.client.as_ref(),
                        child,
                        point,
                        viewport,
                    )
                    .await?;
                    validate_world()?;
                    if !options.force.unwrap_or(false)
                        && !parent_hit(translated.x, translated.y).await?
                    {
                        diagnostics.record(ActionabilityProgress::Obscured, None);
                        tokio::time::sleep(self.retry_delay).await;
                        continue;
                    }
                    validate_world()?;
                    translated
                } else {
                    point
                };
                if options.trial.unwrap_or(false) {
                    return Ok(());
                }
                let navigation = self.prepare_click_v2_navigation(&options).await?;
                validate_world()?;
                // The point was derived from an exact top-level Element, not
                // a selector. Recheck the same object after the navigation
                // barrier and just before dispatch. Movement retries against
                // this identity; detach or reparenting remains terminal.
                let response = self
                    .client
                    .runtime_call_function_by_value_in_unique_context(
                        unique_context_id,
                        &retained_click_route_check_script(
                            point,
                            options.force.unwrap_or(false),
                            child.is_some(),
                        ),
                        vec![serde_json::json!({"objectId": native_object_id})],
                    )
                    .await
                    .map_err(map_cdp_error)?;
                validate_world()?;
                if !verify_retained_click_route(response)? {
                    diagnostics.record(ActionabilityProgress::Unstable, None);
                    tokio::time::sleep(self.retry_delay).await;
                    continue;
                }
                if !self
                    .dispatch_retained_click_v2_pointer(
                        RetainedClickPointerRoute {
                            local_point: point,
                            dispatch_point,
                            child: child.zip(child_viewport),
                            steps,
                        },
                        &options,
                        RetainedClickIdentity {
                            native_object_id,
                            unique_context_id,
                        },
                        press,
                        &parent_hit,
                        &validate_world,
                    )
                    .await?
                {
                    diagnostics.record(ActionabilityProgress::Unstable, None);
                    tokio::time::sleep(self.retry_delay).await;
                    continue;
                }
                if let Some(navigation) = navigation {
                    navigation.finish(self.client.as_ref()).await?;
                }
                return Ok(());
            }
        }
        .await
    }

    /// Cancellation hook for a retained click dropped after mouse-down.
    #[cfg(feature = "retained-element-click")]
    pub async fn cancel_retained_top_level_click(
        &self,
        press: &CdpOwnedMousePress,
    ) -> Result<(), OperationError<LocatorErrorDetails>> {
        self.input
            .release_owned_mouse_press(self.client.as_ref(), press)
            .await
            .map_err(map_cdp_error)
    }

    async fn ensure_selector_runtime(&self) -> Result<(), OperationError<LocatorErrorDetails>> {
        self.runtime_ready
            .get_or_try_init(|| self.install_selector_runtime())
            .await
            .map(|_| ())
    }

    async fn install_selector_runtime(&self) -> Result<(), OperationError<LocatorErrorDetails>> {
        let response = self
            .evaluate(&selector_runtime_install_script())
            .await
            .map_err(map_cdp_error)?;
        let installed = response
            .get("result")
            .and_then(|result| result.get("value"))
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                invalid_response("selector-runtime installation did not return a boolean")
            })?;
        if !installed {
            return Err(invalid_response(
                "selector-runtime installation reported failure",
            ));
        }
        self.runtime_counters
            .installations
            .fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn evaluate(&self, expression: &str) -> pwright_cdp::connection::Result<Value> {
        match self.unique_context_id.as_deref() {
            Some(unique_context_id) => {
                self.client
                    .runtime_evaluate_in_unique_context(expression, unique_context_id)
                    .await
            }
            None => self.client.runtime_evaluate(expression).await,
        }
    }

    /// Runs one selector tap through the same lazy actionability and Page input
    /// owners as Locator click. This direct optional leaf is used by the Node
    /// FFI composition without installing a second selector runtime.
    #[cfg(feature = "touchscreen-input")]
    pub async fn selector_tap(
        &self,
        params: SelectorTapParams,
        timeout: Option<Duration>,
    ) -> Result<(), OperationError<LocatorErrorDetails>> {
        let trial = params.options.trial.unwrap_or(false);
        let action = self.selector_tap_inner(params);
        let outcome = match timeout {
            Some(timeout) => match tokio::time::timeout(timeout, action).await {
                Ok(outcome) => outcome,
                Err(_) => {
                    self.cancel_selector_tap(trial).await?;
                    let progress = self
                        .take_action_progress()
                        .map(|progress| progress.key().to_owned());
                    Err(OperationError::new(
                        FailureKind::Timeout,
                        "locator tap actionability deadline was exceeded",
                        progress.map(LocatorErrorDetails::Backend),
                    ))
                }
            },
            None => action.await,
        };
        self.clear_action_progress();
        outcome
    }

    /// Cancels a retained selector tap after its action future is dropped.
    /// Both native input and the optional trial interceptor are attempted so
    /// one cleanup failure cannot strand the other resource.
    #[cfg(feature = "touchscreen-input")]
    pub async fn cancel_selector_tap(
        &self,
        trial: bool,
    ) -> Result<(), OperationError<LocatorErrorDetails>> {
        let input_cleanup = self
            .input
            .cleanup_pending(self.client.as_ref())
            .await
            .map_err(|error| map_cdp_error(error).reclassify(FailureKind::Cleanup));
        let interceptor_cleanup = if trial {
            self.evaluate(&tap_trial_interceptor_remove_script())
                .await
                .map(|_| ())
                .map_err(|error| map_cdp_error(error).reclassify(FailureKind::Cleanup))
        } else {
            Ok(())
        };
        input_cleanup?;
        interceptor_cleanup
    }

    #[cfg(feature = "touchscreen-input")]
    async fn selector_tap_inner(
        &self,
        params: SelectorTapParams,
    ) -> Result<(), OperationError<LocatorErrorDetails>> {
        let resources = self.reserve_locator_action(&params.selector, None)?;
        let _resources = resources;
        self.clear_action_progress();
        let selector_label = params.selector.to_string();
        let click_options = tap_click_options(params.options);
        let script = click_v2_actionability_script(&params.selector, &click_options)?;
        self.ensure_selector_runtime().await?;
        let point = loop {
            self.runtime_counters
                .actionability_evaluations
                .fetch_add(1, Ordering::Relaxed);
            let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
            match parse_pointer_attempt(
                response,
                &selector_label,
                true,
                params.options.force.unwrap_or(false),
            )? {
                ActionAttempt::Ready(point) => {
                    self.clear_action_progress();
                    break point;
                }
                ActionAttempt::Retry(progress) => {
                    self.record_action_progress(progress);
                    tokio::time::sleep(self.retry_delay).await;
                }
                ActionAttempt::RuntimeMissing => {
                    self.clear_action_progress();
                    self.runtime_counters
                        .missing_after_navigation
                        .fetch_add(1, Ordering::Relaxed);
                    self.install_selector_runtime().await?;
                }
            }
        };

        let trial = params.options.trial.unwrap_or(false);
        if trial {
            self.evaluate(&tap_trial_interceptor_install_script())
                .await
                .map_err(map_cdp_error)?;
        }
        let requested_modifiers = params
            .options
            .modifiers
            .map(|modifiers| resolve_modifiers(modifiers, point.is_mac));
        let transaction = self
            .input
            .begin_transaction(Arc::clone(&self.client), requested_modifiers)
            .await;
        let mut transaction = match transaction {
            Ok(transaction) => transaction,
            Err(error) => {
                let action_error = map_cdp_error(error);
                if trial {
                    self.evaluate(&tap_trial_interceptor_remove_script())
                        .await
                        .map_err(|error| map_cdp_error(error).reclassify(FailureKind::Cleanup))?;
                }
                return Err(action_error);
            }
        };
        let action = transaction.touchscreen_tap(point.x, point.y).await;
        let input_cleanup = transaction.finish().await;
        let interceptor_cleanup = if trial {
            self.evaluate(&tap_trial_interceptor_remove_script())
                .await
                .map(|_| ())
        } else {
            Ok(())
        };
        if let Err(error) = input_cleanup {
            return Err(map_cdp_error(error).reclassify(FailureKind::Cleanup));
        }
        if let Err(error) = interceptor_cleanup {
            return Err(map_cdp_error(error).reclassify(FailureKind::Cleanup));
        }
        action.map_err(map_cdp_error)
    }

    fn query_element_state(
        &self,
        selector: Selector,
        state: LocatorElementState,
    ) -> Pin<Box<dyn Future<Output = Result<bool, OperationError<LocatorErrorDetails>>> + Send + '_>>
    {
        let resources = match self.reserve_locator_action(&selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(selector);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = selector.to_string();
            let script = element_state_script(&selector, state)?;
            self.ensure_selector_runtime().await?;
            loop {
                self.runtime_counters
                    .state_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_element_state_attempt(response, &selector_label, state)? {
                    ElementStateAttempt::Done(value) => return Ok(value),
                    ElementStateAttempt::Retry => tokio::time::sleep(self.retry_delay).await,
                    ElementStateAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            }
        })
    }

    fn wait_for_selector(
        &self,
        params: WaitForParams,
    ) -> Pin<Box<dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>> + Send + '_>>
    {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = wait_for_script(&params.selector, params.options.state)?;
            self.ensure_selector_runtime().await?;
            loop {
                self.runtime_counters
                    .wait_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_wait_for_attempt(response, &selector_label)? {
                    WaitForAttempt::Done => return Ok(()),
                    WaitForAttempt::Retry => tokio::time::sleep(self.retry_delay).await,
                    WaitForAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            }
        })
    }

    fn text_content(
        &self,
        params: TextContentParams,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<TextContentResult, OperationError<LocatorErrorDetails>>>
                + Send
                + '_,
        >,
    > {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = text_content_script(&params.selector)?;
            self.ensure_selector_runtime().await?;
            loop {
                self.runtime_counters
                    .content_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_text_content_attempt(response, &selector_label)? {
                    TextContentAttempt::Done(value) => return Ok(value),
                    TextContentAttempt::Retry => tokio::time::sleep(self.retry_delay).await,
                    TextContentAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            }
        })
    }

    fn get_attribute(
        &self,
        params: GetAttributeParams,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<GetAttributeResult, OperationError<LocatorErrorDetails>>>
                + Send
                + '_,
        >,
    > {
        if params.name.len() > MAX_ATTRIBUTE_NAME_BYTES {
            drop(params);
            return Box::pin(async {
                Err(attribute_name_resource_error(
                    "attribute name exceeds the 65536-byte backend limit",
                ))
            });
        }
        if self.resources.is_some() && params.name.capacity() > MAX_ATTRIBUTE_NAME_BYTES {
            drop(params);
            return Box::pin(async {
                Err(attribute_name_resource_error(
                    "attribute name capacity exceeds the 65536-byte backend limit",
                ))
            });
        }
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = get_attribute_script(&params.selector, &params.name)?;
            self.ensure_selector_runtime().await?;
            loop {
                self.runtime_counters
                    .attribute_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_get_attribute_attempt(response, &selector_label)? {
                    GetAttributeAttempt::Done(value) => return Ok(value),
                    GetAttributeAttempt::Retry => tokio::time::sleep(self.retry_delay).await,
                    GetAttributeAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            }
        })
    }

    fn inner_text(
        &self,
        params: InnerTextParams,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<InnerTextResult, OperationError<LocatorErrorDetails>>>
                + Send
                + '_,
        >,
    > {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = inner_text_script(&params.selector)?;
            self.ensure_selector_runtime().await?;
            loop {
                self.runtime_counters
                    .content_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_inner_text_attempt(response, &selector_label)? {
                    InnerTextAttempt::Done(value) => return Ok(value),
                    InnerTextAttempt::Retry => tokio::time::sleep(self.retry_delay).await,
                    InnerTextAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            }
        })
    }

    fn input_value(
        &self,
        params: InputValueParams,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<InputValueResult, OperationError<LocatorErrorDetails>>>
                + Send
                + '_,
        >,
    > {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = input_value_script(&params.selector)?;
            self.ensure_selector_runtime().await?;
            loop {
                self.runtime_counters
                    .input_value_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_input_value_attempt(response, &selector_label)? {
                    InputValueAttempt::Done(value) => return Ok(value),
                    InputValueAttempt::Retry => tokio::time::sleep(self.retry_delay).await,
                    InputValueAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            }
        })
    }

    fn focus_operation(
        &self,
        selector: Selector,
        operation: FocusOperation,
    ) -> Pin<Box<dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>> + Send + '_>>
    {
        let resources = match self.reserve_locator_action(&selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(selector);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = selector.to_string();
            let script = focus_operation_script(&selector, operation)?;
            self.ensure_selector_runtime().await?;
            loop {
                self.runtime_counters
                    .focus_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_focus_operation_attempt(response, &selector_label, operation)? {
                    FocusOperationAttempt::Done => return Ok(()),
                    FocusOperationAttempt::Retry => {
                        tokio::time::sleep(self.retry_delay).await;
                    }
                    FocusOperationAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            }
        })
    }

    fn scroll_into_view_if_needed(
        &self,
        selector: Selector,
    ) -> Pin<Box<dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>> + Send + '_>>
    {
        let resources = match self.reserve_locator_action(&selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(selector);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = selector.to_string();
            self.ensure_selector_runtime().await?;
            'resolve: loop {
                let check_script = scroll_into_view_if_needed_check_script(&selector)?;
                loop {
                    self.runtime_counters
                        .actionability_evaluations
                        .fetch_add(1, Ordering::Relaxed);
                    let response = self.evaluate(&check_script).await.map_err(map_cdp_error)?;
                    match parse_scroll_into_view_if_needed_attempt(response, &selector_label)? {
                        ScrollIntoViewIfNeededAttempt::Done => return Ok(()),
                        ScrollIntoViewIfNeededAttempt::ReadyToScroll => break,
                        ScrollIntoViewIfNeededAttempt::Retry => {
                            tokio::time::sleep(self.retry_delay).await;
                        }
                        ScrollIntoViewIfNeededAttempt::RuntimeMissing => {
                            self.runtime_counters
                                .missing_after_navigation
                                .fetch_add(1, Ordering::Relaxed);
                            self.install_selector_runtime().await?;
                        }
                    }
                }
                drop(check_script);

                // Dispatching this synchronous evaluation is the commit
                // point. Cancellation before it cannot mutate the page; after
                // it, the scroll is intentionally not replayed or undone.
                let commit_script = scroll_into_view_if_needed_commit_script(&selector)?;
                self.runtime_counters
                    .actionability_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&commit_script).await.map_err(map_cdp_error)?;
                match parse_scroll_into_view_if_needed_attempt(response, &selector_label)? {
                    ScrollIntoViewIfNeededAttempt::Done => return Ok(()),
                    ScrollIntoViewIfNeededAttempt::Retry => continue 'resolve,
                    ScrollIntoViewIfNeededAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                        continue 'resolve;
                    }
                    ScrollIntoViewIfNeededAttempt::ReadyToScroll => {
                        return Err(invalid_response(
                            "locator scroll commit returned a precommit status",
                        ));
                    }
                }
            }
        })
    }

    /// Resolves actionability and geometry for the aggregate screenshot
    /// compatibility adapter without coupling this crate to screenshot types.
    #[doc(hidden)]
    pub fn prepare_screenshot_geometry(
        &self,
        selector: Selector,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<LocatorScreenshotGeometry, OperationError<LocatorErrorDetails>>,
                > + Send
                + '_,
        >,
    > {
        let resources = match self.reserve_locator_action(&selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(selector);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = selector.to_string();
            let check_script = screenshot_geometry_check_script(&selector)?;
            let commit_script = scroll_into_view_if_needed_commit_script(&selector)?;
            self.ensure_selector_runtime().await?;
            'resolve: loop {
                self.runtime_counters
                    .actionability_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&check_script).await.map_err(map_cdp_error)?;
                match parse_screenshot_geometry_attempt(response, &selector_label)? {
                    ScreenshotGeometryAttempt::Done(geometry) => return Ok(geometry),
                    ScreenshotGeometryAttempt::Retry => {
                        tokio::time::sleep(self.retry_delay).await;
                    }
                    ScreenshotGeometryAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                    ScreenshotGeometryAttempt::ReadyToScroll => {
                        self.runtime_counters
                            .actionability_evaluations
                            .fetch_add(1, Ordering::Relaxed);
                        let response =
                            self.evaluate(&commit_script).await.map_err(map_cdp_error)?;
                        match parse_scroll_into_view_if_needed_attempt(response, &selector_label)? {
                            ScrollIntoViewIfNeededAttempt::Done
                            | ScrollIntoViewIfNeededAttempt::Retry => continue 'resolve,
                            ScrollIntoViewIfNeededAttempt::RuntimeMissing => {
                                self.runtime_counters
                                    .missing_after_navigation
                                    .fetch_add(1, Ordering::Relaxed);
                                self.install_selector_runtime().await?;
                                continue 'resolve;
                            }
                            ScrollIntoViewIfNeededAttempt::ReadyToScroll => {
                                return Err(invalid_response(
                                    "Locator screenshot scroll commit returned a precommit status",
                                ));
                            }
                        }
                    }
                }
            }
        })
    }

    fn inner_html(
        &self,
        params: InnerHtmlParams,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<InnerHtmlResult, OperationError<LocatorErrorDetails>>>
                + Send
                + '_,
        >,
    > {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = inner_html_script(&params.selector)?;
            self.ensure_selector_runtime().await?;
            loop {
                self.runtime_counters
                    .content_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_inner_html_attempt(response, &selector_label)? {
                    InnerHtmlAttempt::Done(value) => return Ok(value),
                    InnerHtmlAttempt::Retry => tokio::time::sleep(self.retry_delay).await,
                    InnerHtmlAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            }
        })
    }
}

#[cfg(feature = "touchscreen-input")]
const fn tap_click_options(options: SelectorTapOptions) -> ClickOptionsV2 {
    ClickOptionsV2 {
        button: None,
        click_count: None,
        delay: None,
        force: options.force,
        modifiers: options.modifiers,
        no_wait_after: options.no_wait_after,
        position: options.position,
        scroll: options.scroll,
        steps: None,
        trial: options.trial,
    }
}

#[cfg(feature = "touchscreen-input")]
fn tap_trial_interceptor_install_script() -> String {
    format!(
        r#"(() => {{
  const key = Symbol.for({TAP_TRIAL_INTERCEPTOR_SYMBOL:?});
  globalThis[key]?.stop?.();
  const types = ["pointerdown", "pointerup", "touchstart", "touchend", "touchcancel"];
  const listener = event => {{
    if (!event.isTrusted) return;
    event.preventDefault();
    event.stopPropagation();
    event.stopImmediatePropagation();
  }};
  for (const type of types) window.addEventListener(type, listener, true);
  globalThis[key] = {{ stop() {{
    for (const type of types) window.removeEventListener(type, listener, true);
    delete globalThis[key];
  }} }};
  return true;
}})()"#,
    )
}

#[cfg(feature = "touchscreen-input")]
fn tap_trial_interceptor_remove_script() -> String {
    format!(
        r#"(() => {{
  const key = Symbol.for({TAP_TRIAL_INTERCEPTOR_SYMBOL:?});
  globalThis[key]?.stop?.();
  return true;
}})()"#,
    )
}

impl Service<ClickParams> for CdpLocatorService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: ClickParams) -> Self::Future<'_> {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = actionability_script(&params.selector, params.options.force)?;
            self.ensure_selector_runtime().await?;
            let point = loop {
                self.runtime_counters
                    .actionability_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_attempt(response, &selector_label, params.options.force)? {
                    ActionAttempt::Ready(point) => break point,
                    ActionAttempt::Retry(_) => tokio::time::sleep(self.retry_delay).await,
                    ActionAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        // A document navigation replaces the JavaScript global.
                        // Reinstall only in the new execution context; no
                        // persistent new-document script is retained in Chrome.
                        self.install_selector_runtime().await?;
                    }
                }
            };

            let mut transaction = self
                .input
                .begin_transaction(Arc::clone(&self.client), None)
                .await
                .map_err(map_cdp_error)?;
            let button = map_button(params.options.button);
            let click_count = i32::from(params.options.click_count.get());
            let action = if params.options.trial {
                Ok(())
            } else if let Err(error) = transaction.move_mouse(point.x, point.y).await {
                Err(error)
            } else if let Err(error) = transaction.mouse_down(button, click_count).await {
                Err(error)
            } else {
                transaction.mouse_up().await
            };
            let cleanup = transaction.finish().await;
            if let Err(error) = cleanup {
                return Err(map_cdp_error(error).reclassify(FailureKind::Cleanup));
            }
            action.map_err(map_cdp_error)
        })
    }
}

impl CommandCancellation<Click> for CdpLocatorService {
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>> + Send + 'a>>
    where
        Self: 'a;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async move {
            self.input
                .cleanup_pending(self.client.as_ref())
                .await
                .map_err(map_cdp_error)
        })
    }
}

impl Service<ClickParamsV2> for CdpLocatorService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: ClickParamsV2) -> Self::Future<'_> {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        self.clear_action_progress();
        Box::pin(async move {
            let outcome = async {
                let _resources = resources;
                let steps = params.options.steps.unwrap_or(std::num::NonZeroU32::MIN);
                if steps.get() > MAX_CLICK_STEPS {
                    return Err(locator_resource_error(&format!(
                        "click steps exceed the {MAX_CLICK_STEPS}-event backend limit"
                    )));
                }
                let selector_label = params.selector.to_string();
                let script = click_v2_actionability_script(&params.selector, &params.options)?;
                self.ensure_selector_runtime().await?;
                let point = loop {
                    self.runtime_counters
                        .actionability_evaluations
                        .fetch_add(1, Ordering::Relaxed);
                    let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                    match parse_pointer_attempt(
                        response,
                        &selector_label,
                        true,
                        params.options.force.unwrap_or(false),
                    )? {
                        ActionAttempt::Ready(point) => {
                            self.clear_action_progress();
                            break point;
                        }
                        ActionAttempt::Retry(progress) => {
                            self.record_action_progress(progress);
                            tokio::time::sleep(self.retry_delay).await;
                        }
                        ActionAttempt::RuntimeMissing => {
                            self.clear_action_progress();
                            self.runtime_counters
                                .missing_after_navigation
                                .fetch_add(1, Ordering::Relaxed);
                            self.install_selector_runtime().await?;
                        }
                    }
                };

                self.execute_click_v2_pointer(point, &params.options, steps)
                    .await
            }
            .await;
            self.clear_action_progress();
            outcome
        })
    }
}

impl CommandCancellation<locator_v2::Click> for CdpLocatorService {
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>> + Send + 'a>>
    where
        Self: 'a;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async move {
            self.input
                .cleanup_pending(self.client.as_ref())
                .await
                .map_err(map_cdp_error)
        })
    }

    fn terminal_error_after_cancel(
        &self,
        _context: OperationContext,
        reason: CancellationReason,
        _timeout: Option<Duration>,
    ) -> Option<OperationError<LocatorErrorDetails>> {
        if reason != CancellationReason::DeadlineExceeded {
            self.clear_action_progress();
            return None;
        }
        let progress = self.take_action_progress()?;
        Some(OperationError::new(
            FailureKind::Timeout,
            "locator actionability deadline was exceeded",
            Some(LocatorErrorDetails::Backend(progress.key().to_owned())),
        ))
    }
}

impl Service<PressParams> for CdpLocatorService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: PressParams) -> Self::Future<'_> {
        let tokens = match KeyTokens::parse(params.key.as_str()) {
            Ok(tokens) => tokens,
            Err(error) => {
                let message = error.to_string();
                drop(params);
                return Box::pin(async move {
                    Err(OperationError::new(
                        FailureKind::Backend,
                        message.clone(),
                        Some(LocatorErrorDetails::InvalidValue(message)),
                    ))
                });
            }
        };
        if let Some(error) = tokens
            .as_slice()
            .iter()
            .find_map(|token| resolve_key(token, false, false).err())
        {
            let message = error.to_string();
            drop(params);
            return Box::pin(async move {
                Err(OperationError::new(
                    FailureKind::Backend,
                    message.clone(),
                    Some(LocatorErrorDetails::InvalidValue(message)),
                ))
            });
        }
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = press_focus_script(&params.selector)?;
            self.ensure_selector_runtime().await?;
            let navigation = if params.options.no_wait_after {
                None
            } else {
                Some(ClickNavigationBarrier::prepare(self.client.as_ref()).await?)
            };
            let mut transaction = self.keyboard_transaction().await?;
            let action: Result<(), OperationError<LocatorErrorDetails>> = async {
                let is_mac = loop {
                    self.runtime_counters
                        .focus_evaluations
                        .fetch_add(1, Ordering::Relaxed);
                    let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                    match parse_press_focus_attempt(response, &selector_label)? {
                        PressFocusAttempt::Done { is_mac } => break is_mac,
                        PressFocusAttempt::Retry => tokio::time::sleep(self.retry_delay).await,
                        PressFocusAttempt::RuntimeMissing => {
                            self.runtime_counters
                                .missing_after_navigation
                                .fetch_add(1, Ordering::Relaxed);
                            self.install_selector_runtime().await?;
                        }
                    }
                };
                self.handoff_keyboard_lifetime()?;
                let tokens = KeyTokens::parse(params.key.as_str())
                    .expect("portable PressKey validates its bounded token sequence");
                let mut pressed = [None::<ResolvedKey>; pwright_cap_locator::PRESS_KEY_MAX_TOKENS];
                let mut pressed_count = 0;
                for token in tokens.as_slice() {
                    let key = resolve_key(
                        token,
                        transaction.modifiers().contains(CdpModifiers::SHIFT),
                        is_mac,
                    )
                    .map_err(|error| invalid_press_key(error.to_string()))?;
                    transaction
                        .key_down(key, is_mac)
                        .await
                        .map_err(map_cdp_error)?;
                    pressed[pressed_count] = Some(key);
                    pressed_count += 1;
                }
                if let Some(delay) = params.options.delay.filter(|delay| !delay.is_zero()) {
                    tokio::time::sleep(delay).await;
                }
                while pressed_count > 0 {
                    pressed_count -= 1;
                    transaction
                        .key_up(pressed[pressed_count].expect("press records a dense key prefix"))
                        .await
                        .map_err(map_cdp_error)?;
                }
                Ok(())
            }
            .await;
            let cleanup = transaction.finish().await;
            if let Err(error) = cleanup {
                return Err(map_cdp_error(error).reclassify(FailureKind::Cleanup));
            }
            action?;
            if let Some(navigation) = navigation {
                navigation.finish(self.keyboard_client().as_ref()).await?;
            }
            Ok(())
        })
    }
}

macro_rules! impl_press_cancellation {
    ($($command:ty),+ $(,)?) => {
        $(
            impl CommandCancellation<$command> for CdpLocatorService {
                type Future<'a> = Pin<Box<dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>> + Send + 'a>> where Self: 'a;

                fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
                    Box::pin(async move {
                        self.input.cleanup_pending(self.client.as_ref()).await.map_err(map_cdp_error)
                    })
                }
            }
        )+
    };
}

impl_press_cancellation!(Press, locator_v2::Press, Fill, locator_v2::Fill);

impl Service<SelectOptionParams> for CdpLocatorService {
    type Response = SelectOptionResult;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: SelectOptionParams) -> Self::Future<'_> {
        if params.values.capacity() > MAX_SELECT_OPTION_ITEMS {
            drop(params);
            return Box::pin(async move {
                Err(select_option_resource_error(
                    "select-option descriptor capacity exceeds the 1024-item backend limit",
                ))
            });
        }
        if let Err(error) = validate_select_option_input(&params.values) {
            drop(params);
            return Box::pin(async move { Err(error) });
        }
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            #[cfg(feature = "select-option-timeout-progress")]
            let progress = params.progress().cloned();
            let selector_label = params.selector.to_string();
            #[cfg(feature = "select-option-navigation-commit")]
            let commit_token = match (
                self.select_option_commit_execution_context_id,
                self.unique_context_id.as_deref(),
                self.select_option_commit_frame_id.as_deref(),
            ) {
                (Some(_), Some(unique_context_id), Some(_)) => {
                    Some(SelectOptionCommitReceipt::next_token(unique_context_id)?)
                }
                _ => None,
            };
            #[cfg(not(feature = "select-option-navigation-commit"))]
            let commit_token: Option<String> = None;
            let script = select_option_script(
                &params.selector,
                &params.values,
                params.options,
                commit_token.as_deref(),
            )?;
            self.ensure_selector_runtime().await?;
            loop {
                #[cfg(feature = "select-option-navigation-commit")]
                let receipt = if let (
                    Some(execution_context_id),
                    Some(unique_context_id),
                    Some(frame_id),
                    Some(commit_token),
                ) = (
                    self.select_option_commit_execution_context_id,
                    self.unique_context_id.as_ref(),
                    self.select_option_commit_frame_id.as_ref(),
                    commit_token.as_deref(),
                ) {
                    Some(
                        SelectOptionCommitReceipt::prepare(
                            self.client.as_ref(),
                            execution_context_id,
                            Arc::clone(unique_context_id),
                            Arc::clone(frame_id),
                            commit_token,
                        )
                        .await?,
                    )
                } else {
                    None
                };
                self.runtime_counters
                    .actionability_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = match self.evaluate(&script).await {
                    Ok(response) => response,
                    Err(error) => {
                        #[cfg(feature = "select-option-navigation-commit")]
                        if is_select_option_navigation_context_loss(&error)
                            && let Some(receipt) = receipt
                            && let Some(values) = receipt.confirm_after_context_loss().await
                        {
                            return Ok(values);
                        }
                        return Err(map_cdp_error(error));
                    }
                };
                match parse_select_option_attempt(response, &selector_label)? {
                    SelectOptionAttempt::Done(values) => {
                        #[cfg(feature = "select-option-timeout-progress")]
                        if let Some(progress) = &progress {
                            progress.record_retry_option_not_enabled(false);
                        }
                        return Ok(values);
                    }
                    SelectOptionAttempt::Retry(option_not_enabled) => {
                        #[cfg(feature = "select-option-timeout-progress")]
                        if let Some(progress) = &progress {
                            progress.record_retry_option_not_enabled(option_not_enabled);
                        }
                        #[cfg(not(feature = "select-option-timeout-progress"))]
                        let _ = option_not_enabled;
                        tokio::time::sleep(self.retry_delay).await;
                    }
                    SelectOptionAttempt::RuntimeMissing => {
                        #[cfg(feature = "select-option-timeout-progress")]
                        if let Some(progress) = &progress {
                            progress.record_retry_option_not_enabled(false);
                        }
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            }
        })
    }
}

macro_rules! impl_select_option_cancellation {
    ($($command:ty),+ $(,)?) => {
        $(
            impl CommandCancellation<$command> for CdpLocatorService {
                type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

                fn cancel(
                    &self,
                    _context: OperationContext,
                    _reason: CancellationReason,
                ) -> Self::Future<'_> {
                    // No native handle or input state survives a retry. A
                    // successful renderer evaluation is the only commit point
                    // and this cancellation path never replays it.
                    ready(Ok(()))
                }
            }
        )+
    };
}

impl_select_option_cancellation!(SelectOption, locator_v2::SelectOption);

impl Service<SetCheckedParams> for CdpLocatorService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: SetCheckedParams) -> Self::Future<'_> {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let state_script = set_checked_state_script(&params.selector)?;
            self.ensure_selector_runtime().await?;
            let current = loop {
                self.runtime_counters
                    .state_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&state_script).await.map_err(map_cdp_error)?;
                match parse_set_checked_state_attempt(response, &selector_label)? {
                    SetCheckedStateAttempt::Done { checked, is_radio } => {
                        break (checked, is_radio);
                    }
                    SetCheckedStateAttempt::Retry => {
                        tokio::time::sleep(self.retry_delay).await;
                    }
                    SetCheckedStateAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            };
            if current.0 == params.checked {
                return Ok(());
            }
            if !params.checked && current.1 {
                return Err(set_checked_target_error(
                    "Cannot uncheck radio button. Radio buttons can only be unchecked by selecting another radio button in the same group.",
                ));
            }

            let click_options = set_checked_click_options(params.options);
            let actionability = click_v2_actionability_script(&params.selector, &click_options)?;
            let point = loop {
                self.runtime_counters
                    .actionability_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&actionability).await.map_err(map_cdp_error)?;
                match parse_pointer_attempt(
                    response,
                    &selector_label,
                    false,
                    params.options.force.unwrap_or(false),
                )? {
                    ActionAttempt::Ready(point) => break point,
                    ActionAttempt::Retry(_) => tokio::time::sleep(self.retry_delay).await,
                    ActionAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            };

            if params.options.trial.unwrap_or(false) {
                return Ok(());
            }

            let mut transaction = self
                .input
                .begin_transaction(Arc::clone(&self.client), None)
                .await
                .map_err(map_cdp_error)?;
            let action = run_click_v2_pointer_action(
                &mut transaction,
                point,
                &click_options,
                std::num::NonZeroU32::MIN,
            )
            .await;
            let cleanup = transaction.finish().await;
            if let Err(error) = cleanup {
                return Err(map_cdp_error(error).reclassify(FailureKind::Cleanup));
            }
            action.map_err(map_cdp_error)?;

            self.runtime_counters
                .state_evaluations
                .fetch_add(1, Ordering::Relaxed);
            let response = self.evaluate(&state_script).await.map_err(map_cdp_error)?;
            match parse_set_checked_state_attempt(response, &selector_label)? {
                SetCheckedStateAttempt::Done { checked, .. } if checked == params.checked => Ok(()),
                SetCheckedStateAttempt::Done { .. } => Err(set_checked_target_error(
                    "Clicking the checkbox did not change its state",
                )),
                SetCheckedStateAttempt::Retry => Err(set_checked_target_error(
                    "Checkbox or radio button was disconnected after the committed click",
                )),
                SetCheckedStateAttempt::RuntimeMissing => Err(set_checked_target_error(
                    "Selector runtime disappeared after the committed click",
                )),
            }
        })
    }
}

macro_rules! impl_set_checked_cancellation {
    ($($command:ty),+ $(,)?) => {
        $(
            impl CommandCancellation<$command> for CdpLocatorService {
                type Future<'a> = Pin<Box<dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>> + Send + 'a>> where Self: 'a;

                fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
                    Box::pin(async move {
                        self.input.cleanup_pending(self.client.as_ref()).await.map_err(map_cdp_error)
                    })
                }
            }
        )+
    };
}

impl_set_checked_cancellation!(SetChecked, locator_v2::SetChecked);

impl CommandCancellation<locator_v2::Hover> for CdpLocatorService {
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>> + Send + 'a>>
    where
        Self: 'a;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async move {
            self.input
                .cleanup_pending(self.client.as_ref())
                .await
                .map_err(map_cdp_error)
        })
    }
}

macro_rules! impl_ready_locator_v2_cancellation {
    ($($command:ty),+ $(,)?) => {
        $(
            impl CommandCancellation<$command> for CdpLocatorService {
                type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

                fn cancel(
                    &self,
                    _context: OperationContext,
                    _reason: CancellationReason,
                ) -> Self::Future<'_> {
                    ready(Ok(()))
                }
            }
        )+
    };
}

impl_ready_locator_v2_cancellation!(
    locator_v2::AllInnerTexts,
    locator_v2::AllTextContents,
    locator_v2::Blur,
    locator_v2::Count,
    locator_v2::Focus,
    locator_v2::GetAttribute,
    locator_v2::InnerHtml,
    locator_v2::InnerText,
    locator_v2::InputValue,
    locator_v2::IsChecked,
    locator_v2::IsEditable,
    locator_v2::IsEnabled,
    locator_v2::IsVisible,
    locator_v2::TextContent,
    locator_v2::WaitFor,
);

macro_rules! impl_focus_operation_service {
    ($command:ty, $params:ty, $operation:expr) => {
        impl Service<$params> for CdpLocatorService {
            type Response = ();
            type Error = OperationError<LocatorErrorDetails>;
            type Future<'a>
                = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
            where
                Self: 'a;

            fn call(&self, params: $params) -> Self::Future<'_> {
                self.focus_operation(params.selector, $operation)
            }
        }

        impl CommandCancellation<$command> for CdpLocatorService {
            type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

            fn cancel(
                &self,
                _context: OperationContext,
                _reason: CancellationReason,
            ) -> Self::Future<'_> {
                ready(Ok(()))
            }
        }
    };
}

impl_focus_operation_service!(Focus, FocusParams, FocusOperation::Focus);
impl_focus_operation_service!(Blur, BlurParams, FocusOperation::Blur);

impl Service<ScrollIntoViewIfNeededParams> for CdpLocatorService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: ScrollIntoViewIfNeededParams) -> Self::Future<'_> {
        self.scroll_into_view_if_needed(params.selector)
    }
}

macro_rules! impl_scroll_into_view_if_needed_cancellation {
    ($($command:ty),+ $(,)?) => {
        $(
            impl CommandCancellation<$command> for CdpLocatorService {
                type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

                fn cancel(
                    &self,
                    _context: OperationContext,
                    _reason: CancellationReason,
                ) -> Self::Future<'_> {
                    // There is no retained DOM handle or native input state.
                    // A completed scroll is the sole, non-replayable commit.
                    ready(Ok(()))
                }
            }
        )+
    };
}

impl_scroll_into_view_if_needed_cancellation!(
    ScrollIntoViewIfNeeded,
    locator_v2::ScrollIntoViewIfNeeded,
);

impl Service<HoverParams> for CdpLocatorService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: HoverParams) -> Self::Future<'_> {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = hover_actionability_script(&params.selector, params.options)?;
            self.ensure_selector_runtime().await?;
            let point = loop {
                self.runtime_counters
                    .hover_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_hover_attempt(response, &selector_label, params.options.force)? {
                    ActionAttempt::Ready(point) => break point,
                    ActionAttempt::Retry(_) => tokio::time::sleep(self.retry_delay).await,
                    ActionAttempt::RuntimeMissing => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            };

            let requested_modifiers = params
                .options
                .modifiers
                .map(|modifiers| resolve_modifiers(modifiers, point.is_mac));
            let mut transaction = self
                .input
                .begin_transaction(Arc::clone(&self.client), requested_modifiers)
                .await
                .map_err(map_cdp_error)?;
            let action = if params.options.trial {
                Ok(())
            } else {
                transaction.move_mouse(point.x, point.y).await
            };
            let cleanup = transaction.finish().await;
            if let Err(error) = cleanup {
                return Err(map_cdp_error(error).reclassify(FailureKind::Cleanup));
            }
            action.map_err(map_cdp_error)
        })
    }
}

impl CommandCancellation<Hover> for CdpLocatorService {
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<(), OperationError<LocatorErrorDetails>>> + Send + 'a>>
    where
        Self: 'a;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        Box::pin(async move {
            self.input
                .cleanup_pending(self.client.as_ref())
                .await
                .map_err(map_cdp_error)
        })
    }
}

impl Service<CountParams> for CdpLocatorService {
    type Response = CountResult;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: CountParams) -> Self::Future<'_> {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = count_script(&params.selector)?;
            self.ensure_selector_runtime().await?;

            // Playwright's count is a non-strict snapshot query. It evaluates
            // once even when the result is zero; only a replaced JavaScript
            // global gets one bounded runtime-reinstallation attempt.
            for attempt in 0..=1 {
                self.runtime_counters
                    .snapshot_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_count_attempt(response, &selector_label)? {
                    CountAttempt::Done(count) => return Ok(count),
                    CountAttempt::RuntimeMissing if attempt == 0 => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                    CountAttempt::RuntimeMissing => {
                        return Err(invalid_response(
                            "selector runtime remained missing after reinstallation",
                        ));
                    }
                }
            }
            unreachable!("bounded count attempts always return")
        })
    }
}

impl CommandCancellation<Count> for CdpLocatorService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // The snapshot owns no persistent browser-side operation or node.
        ready(Ok(()))
    }
}

impl Service<AllInnerTextsParams> for CdpLocatorService {
    type Response = AllInnerTextsResult;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: AllInnerTextsParams) -> Self::Future<'_> {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = all_inner_texts_script(&params.selector)?;
            self.ensure_selector_runtime().await?;

            for attempt in 0..=1 {
                self.runtime_counters
                    .all_inner_texts_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_all_inner_texts_attempt(response, &selector_label)? {
                    AllInnerTextsAttempt::Done(values) => return Ok(values),
                    AllInnerTextsAttempt::RuntimeMissing if attempt == 0 => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                    AllInnerTextsAttempt::RuntimeMissing => {
                        return Err(invalid_response(
                            "selector runtime remained missing after reinstallation",
                        ));
                    }
                }
            }
            unreachable!("bounded all-inner-texts attempts always return")
        })
    }
}

impl CommandCancellation<AllInnerTexts> for CdpLocatorService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // The snapshot owns no persistent browser-side operation or node.
        ready(Ok(()))
    }
}

impl Service<AllTextContentsParams> for CdpLocatorService {
    type Response = AllTextContentsResult;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: AllTextContentsParams) -> Self::Future<'_> {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = all_text_contents_script(&params.selector)?;
            self.ensure_selector_runtime().await?;

            for attempt in 0..=1 {
                self.runtime_counters
                    .all_text_contents_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_all_text_contents_attempt(response, &selector_label)? {
                    AllTextContentsAttempt::Done(values) => return Ok(values),
                    AllTextContentsAttempt::RuntimeMissing if attempt == 0 => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                    AllTextContentsAttempt::RuntimeMissing => {
                        return Err(invalid_response(
                            "selector runtime remained missing after reinstallation",
                        ));
                    }
                }
            }
            unreachable!("bounded all-text-contents attempts always return")
        })
    }
}

impl CommandCancellation<AllTextContents> for CdpLocatorService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // The snapshot owns no persistent browser-side operation or node.
        ready(Ok(()))
    }
}

impl Service<IsVisibleParams> for CdpLocatorService {
    type Response = IsVisibleResult;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: IsVisibleParams) -> Self::Future<'_> {
        let resources = match self.reserve_locator_action(&params.selector, None) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            let selector_label = params.selector.to_string();
            let script = visibility_script(&params.selector)?;
            self.ensure_selector_runtime().await?;

            for attempt in 0..=1 {
                self.runtime_counters
                    .snapshot_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = self.evaluate(&script).await.map_err(map_cdp_error)?;
                match parse_visibility_attempt(response, &selector_label)? {
                    VisibilityAttempt::Done(visible) => return Ok(visible),
                    VisibilityAttempt::RuntimeMissing if attempt == 0 => {
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                    VisibilityAttempt::RuntimeMissing => {
                        return Err(invalid_response(
                            "selector runtime remained missing after reinstallation",
                        ));
                    }
                }
            }
            unreachable!("bounded visibility attempts always return")
        })
    }
}

impl CommandCancellation<IsVisible> for CdpLocatorService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // The snapshot owns no persistent browser-side operation or node.
        ready(Ok(()))
    }
}

impl Service<GetAttributeParams> for CdpLocatorService {
    type Response = GetAttributeResult;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: GetAttributeParams) -> Self::Future<'_> {
        self.get_attribute(params)
    }
}

impl CommandCancellation<GetAttribute> for CdpLocatorService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // Dropping the retry future stops polling; no DOM identity or
        // browser-side operation remains to clean up.
        ready(Ok(()))
    }
}

impl Service<TextContentParams> for CdpLocatorService {
    type Response = TextContentResult;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: TextContentParams) -> Self::Future<'_> {
        self.text_content(params)
    }
}

impl CommandCancellation<TextContent> for CdpLocatorService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // Dropping the retry future stops polling; no DOM identity or
        // browser-side operation remains to clean up.
        ready(Ok(()))
    }
}

impl Service<InnerHtmlParams> for CdpLocatorService {
    type Response = InnerHtmlResult;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: InnerHtmlParams) -> Self::Future<'_> {
        self.inner_html(params)
    }
}

impl CommandCancellation<InnerHtml> for CdpLocatorService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // Dropping the retry future stops polling; no DOM identity or
        // browser-side operation remains to clean up.
        ready(Ok(()))
    }
}

impl Service<InnerTextParams> for CdpLocatorService {
    type Response = InnerTextResult;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: InnerTextParams) -> Self::Future<'_> {
        self.inner_text(params)
    }
}

impl CommandCancellation<InnerText> for CdpLocatorService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // Dropping the retry future stops polling; no DOM identity or
        // browser-side operation remains to clean up.
        ready(Ok(()))
    }
}

impl Service<InputValueParams> for CdpLocatorService {
    type Response = InputValueResult;
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: InputValueParams) -> Self::Future<'_> {
        self.input_value(params)
    }
}

impl CommandCancellation<InputValue> for CdpLocatorService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // Dropping the retry future stops polling; no DOM identity or
        // browser-side operation remains to clean up.
        ready(Ok(()))
    }
}

macro_rules! impl_element_state_service {
    ($command:ty, $params:ty, $state:expr) => {
        impl Service<$params> for CdpLocatorService {
            type Response = bool;
            type Error = OperationError<LocatorErrorDetails>;
            type Future<'a>
                = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
            where
                Self: 'a;

            fn call(&self, params: $params) -> Self::Future<'_> {
                self.query_element_state(params.selector, $state)
            }
        }

        impl CommandCancellation<$command> for CdpLocatorService {
            type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

            fn cancel(
                &self,
                _context: OperationContext,
                _reason: CancellationReason,
            ) -> Self::Future<'_> {
                // State queries retain no browser-side operation or DOM node.
                ready(Ok(()))
            }
        }
    };
}

impl_element_state_service!(IsChecked, IsCheckedParams, LocatorElementState::Checked);
impl_element_state_service!(IsEditable, IsEditableParams, LocatorElementState::Editable);
impl_element_state_service!(IsEnabled, IsEnabledParams, LocatorElementState::Enabled);

impl Service<WaitForParams> for CdpLocatorService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: WaitForParams) -> Self::Future<'_> {
        self.wait_for_selector(params)
    }
}

impl CommandCancellation<WaitFor> for CdpLocatorService {
    type Future<'a> = Ready<Result<(), OperationError<LocatorErrorDetails>>>;

    fn cancel(&self, _context: OperationContext, _reason: CancellationReason) -> Self::Future<'_> {
        // Dropping the caller-driven retry future stops future polling; no DOM
        // identity or persistent browser-side wait needs separate cleanup.
        ready(Ok(()))
    }
}

impl Service<FillParams> for CdpLocatorService {
    type Response = ();
    type Error = OperationError<LocatorErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: FillParams) -> Self::Future<'_> {
        let resources = match self.reserve_locator_action(&params.selector, Some(&params.value)) {
            Ok(resources) => resources,
            Err(error) => {
                drop(params);
                return Box::pin(async move { Err(error) });
            }
        };
        Box::pin(async move {
            let _resources = resources;
            if params.value.len() > MAX_FILL_VALUE_BYTES {
                return Err(fill_value_resource_error(
                    "fill value exceeds the 1048576-byte backend limit",
                ));
            }

            let selector_label = params.selector.to_string();
            #[cfg(feature = "fill-navigation-commit")]
            let commit_token = match (
                self.fill_commit_execution_context_id,
                self.unique_context_id.as_deref(),
            ) {
                (Some(_), Some(unique_context_id)) => {
                    Some(FillCommitReceipt::next_token(unique_context_id)?)
                }
                _ => None,
            };
            #[cfg(not(feature = "fill-navigation-commit"))]
            let commit_token: Option<String> = None;
            let script = fill_actionability_script(
                &params.selector,
                &params.value,
                params.options.force,
                commit_token.as_deref(),
            )?;
            self.ensure_selector_runtime().await?;
            loop {
                // Selection/focus and the final insert are one Page-owned
                // transaction. Release it before retry delays so a missing
                // target does not retain the input gate between attempts.
                let mut transaction = self
                    .input
                    .begin_transaction(Arc::clone(&self.client), None)
                    .await
                    .map_err(map_cdp_error)?;
                #[cfg(feature = "fill-navigation-commit")]
                let receipt = if let (Some(execution_context_id), Some(commit_token)) = (
                    self.fill_commit_execution_context_id,
                    commit_token.as_deref(),
                ) {
                    Some(
                        FillCommitReceipt::prepare(
                            self.client.as_ref(),
                            execution_context_id,
                            commit_token,
                        )
                        .await?,
                    )
                } else {
                    None
                };
                self.runtime_counters
                    .actionability_evaluations
                    .fetch_add(1, Ordering::Relaxed);
                let response = match self.evaluate(&script).await {
                    Ok(response) => response,
                    Err(error) => {
                        #[cfg(feature = "fill-navigation-commit")]
                        if is_navigation_context_loss(&error)
                            && let Some(receipt) = receipt
                            && receipt.confirm_after_context_loss().await
                        {
                            return transaction.finish().await.map_err(map_cdp_error);
                        }
                        return Err(map_cdp_error(error));
                    }
                };
                match parse_fill_attempt(response, &selector_label)? {
                    FillAttempt::Done => return transaction.finish().await.map_err(map_cdp_error),
                    FillAttempt::NeedsInput { is_mac } => {
                        let action = async {
                            if params.value.is_empty() {
                                let key = resolve_key("Delete", false, is_mac)
                                    .expect("the static layout includes Delete");
                                transaction.key_down(key, is_mac).await?;
                                transaction.key_up(key).await
                            } else {
                                self.client.input_insert_text(&params.value).await
                            }
                        }
                        .await;
                        transaction.finish().await.map_err(|error| {
                            map_cdp_error(error).reclassify(FailureKind::Cleanup)
                        })?;
                        return action.map_err(map_cdp_error);
                    }
                    FillAttempt::Retry => {
                        drop(transaction);
                        tokio::time::sleep(self.retry_delay).await;
                    }
                    FillAttempt::RuntimeMissing => {
                        drop(transaction);
                        self.runtime_counters
                            .missing_after_navigation
                            .fetch_add(1, Ordering::Relaxed);
                        self.install_selector_runtime().await?;
                    }
                }
            }
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ClickPoint {
    x: f64,
    y: f64,
    is_mac: bool,
}

#[cfg(feature = "retained-element-click")]
struct RetainedClickIdentity<'a> {
    native_object_id: &'a str,
    unique_context_id: &'a str,
}

#[cfg(feature = "retained-element-click")]
struct RetainedClickTarget<'a> {
    identity: RetainedClickIdentity<'a>,
    child: Option<retained_click_child::ChildFrame<'a>>,
}

/// Exact retained Element and its one-link child Frame route.
#[cfg(feature = "retained-element-click")]
#[derive(Clone, Copy)]
pub struct RetainedClickChildTarget<'a> {
    native_object_id: &'a str,
    unique_context_id: &'a str,
    child_frame_id: &'a str,
    parent_frame_id: &'a str,
}

#[cfg(feature = "retained-element-click")]
impl<'a> RetainedClickChildTarget<'a> {
    pub fn new(
        native_object_id: &'a str,
        unique_context_id: &'a str,
        child_frame_id: &'a str,
        parent_frame_id: &'a str,
    ) -> Self {
        Self {
            native_object_id,
            unique_context_id,
            child_frame_id,
            parent_frame_id,
        }
    }
}

#[cfg(feature = "retained-element-click")]
struct RetainedClickPointerRoute<'a> {
    local_point: ClickPoint,
    dispatch_point: ClickPoint,
    child: Option<(
        retained_click_child::ChildFrame<'a>,
        retained_click_child::Viewport,
    )>,
    steps: std::num::NonZeroU32,
}

struct ClickNavigationBarrier {
    events: CdpEventReceiver,
    expected_session: Option<String>,
    main_frame_id: String,
}

impl ClickNavigationBarrier {
    async fn prepare(client: &dyn CdpClient) -> Result<Self, OperationError<LocatorErrorDetails>> {
        let mut events = client
            .subscribe_composable_events()
            .map_err(click_navigation_subscription_error)?;
        client.page_enable().await.map_err(map_cdp_error)?;
        // A renderer-session root can be an OOPIF and legitimately have a
        // parent. Page creation's parentless-root assertion does not apply to
        // this Frame-local navigation observation.
        let main_frame_id = client
            .page_get_frame_tree()
            .await
            .map_err(map_cdp_error)?
            .frame
            .id;
        loop {
            match events.try_recv() {
                Ok(_) => {}
                Err(CdpEventTryRecvError::Empty) => break,
                Err(CdpEventTryRecvError::Lagged(skipped)) => {
                    return Err(click_navigation_stream_error(skipped));
                }
                Err(CdpEventTryRecvError::Closed) => {
                    return Err(click_navigation_stream_closed());
                }
            }
        }
        Ok(Self {
            events,
            expected_session: client.event_session_id().map(ToOwned::to_owned),
            main_frame_id,
        })
    }

    async fn finish(
        mut self,
        client: &dyn CdpClient,
    ) -> Result<(), OperationError<LocatorErrorDetails>> {
        // Matches Chromium Playwright's input-action epilogue. The response is
        // an ordering barrier for Page events caused by the preceding input.
        client.page_enable().await.map_err(map_cdp_error)?;
        let mut requested = false;
        loop {
            match self.events.try_recv() {
                Ok(event) => {
                    if self.observe(&event, &mut requested)? {
                        return Ok(());
                    }
                }
                Err(CdpEventTryRecvError::Empty) => break,
                Err(CdpEventTryRecvError::Lagged(skipped)) => {
                    return Err(click_navigation_stream_error(skipped));
                }
                Err(CdpEventTryRecvError::Closed) => {
                    return Err(click_navigation_stream_closed());
                }
            }
        }
        if !requested {
            return Ok(());
        }
        loop {
            let event = match self.events.recv().await {
                Ok(event) => event,
                Err(CdpEventRecvError::Lagged(skipped)) => {
                    return Err(click_navigation_stream_error(skipped));
                }
                Err(CdpEventRecvError::Closed) => {
                    return Err(click_navigation_stream_closed());
                }
            };
            if self.observe(&event, &mut requested)? {
                return Ok(());
            }
        }
    }

    fn observe(
        &self,
        event: &CdpEvent,
        requested: &mut bool,
    ) -> Result<bool, OperationError<LocatorErrorDetails>> {
        if event.method == "Browser.downloadWillBegin"
            && event.params.get("frameId").and_then(Value::as_str)
                == Some(self.main_frame_id.as_str())
        {
            return Ok(true);
        }
        if event.session_id.as_deref() != self.expected_session.as_deref() {
            return Ok(false);
        }
        match event.method.as_str() {
            "Page.frameRequestedNavigation"
                if event.params.get("frameId").and_then(Value::as_str)
                    == Some(self.main_frame_id.as_str())
                    && event.params.get("disposition").and_then(Value::as_str)
                        == Some("currentTab") =>
            {
                *requested = true;
                Ok(false)
            }
            "Page.frameNavigated" if *requested => Ok(event
                .params
                .get("frame")
                .and_then(|frame| frame.get("id"))
                .and_then(Value::as_str)
                == Some(self.main_frame_id.as_str())),
            "Page.navigatedWithinDocument" if *requested => {
                Ok(event.params.get("frameId").and_then(Value::as_str)
                    == Some(self.main_frame_id.as_str()))
            }
            "Page.frameDetached"
                if *requested
                    && event.params.get("frameId").and_then(Value::as_str)
                        == Some(self.main_frame_id.as_str()) =>
            {
                Err(OperationError::new(
                    FailureKind::TargetClosed,
                    "main frame detached while waiting for click navigation",
                    Some(LocatorErrorDetails::Backend(
                        "main frame detached during click navigation".to_string(),
                    )),
                ))
            }
            _ => Ok(false),
        }
    }
}

async fn run_click_v2_pointer_action(
    transaction: &mut CdpInputTransaction,
    point: ClickPoint,
    options: &ClickOptionsV2,
    steps: std::num::NonZeroU32,
) -> Result<(), CdpError> {
    if options.trial.unwrap_or(false) {
        return Ok(());
    }
    transaction
        .move_mouse_in_steps(point.x, point.y, steps)
        .await?;
    let button = map_button(options.button.unwrap_or(PortableMouseButton::Left));
    let click_count = options.click_count.map_or(1, |count| count.get());
    let delay = options.delay.unwrap_or(Duration::ZERO);
    for count in 1..=click_count {
        transaction.mouse_down(button, i32::from(count)).await?;
        if !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
        transaction.mouse_up().await?;
        if count < click_count && !delay.is_zero() {
            tokio::time::sleep(delay).await;
        }
    }
    Ok(())
}

const fn set_checked_click_options(options: SetCheckedOptions) -> ClickOptionsV2 {
    ClickOptionsV2 {
        button: None,
        click_count: None,
        delay: None,
        force: options.force,
        modifiers: None,
        no_wait_after: None,
        position: options.position,
        scroll: options.scroll,
        steps: None,
        trial: options.trial,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActionabilityProgress {
    Missing,
    Detached,
    NotVisible,
    Unstable,
    Disabled,
    Obscured,
}

impl ActionabilityProgress {
    const fn key(self) -> &'static str {
        match self {
            Self::Missing => "actionability:missing",
            Self::Detached => "actionability:detached",
            Self::NotVisible => "actionability:not-visible",
            Self::Unstable => "actionability:unstable",
            Self::Disabled => "actionability:disabled",
            Self::Obscured => "actionability:obscured",
        }
    }
}

enum ActionAttempt {
    Ready(ClickPoint),
    Retry(ActionabilityProgress),
    RuntimeMissing,
}

enum FillAttempt {
    Done,
    NeedsInput { is_mac: bool },
    Retry,
    RuntimeMissing,
}

enum CountAttempt {
    Done(CountResult),
    RuntimeMissing,
}

enum AllInnerTextsAttempt {
    Done(AllInnerTextsResult),
    RuntimeMissing,
}

enum AllTextContentsAttempt {
    Done(AllTextContentsResult),
    RuntimeMissing,
}

enum VisibilityAttempt {
    Done(IsVisibleResult),
    RuntimeMissing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LocatorElementState {
    Checked,
    Editable,
    Enabled,
}

impl LocatorElementState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Checked => "checked",
            Self::Editable => "editable",
            Self::Enabled => "enabled",
        }
    }
}

enum ElementStateAttempt {
    Done(bool),
    Retry,
    RuntimeMissing,
}

enum SetCheckedStateAttempt {
    Done { checked: bool, is_radio: bool },
    Retry,
    RuntimeMissing,
}

enum SelectOptionAttempt {
    Done(SelectOptionResult),
    Retry(bool),
    RuntimeMissing,
}

enum ScrollIntoViewIfNeededAttempt {
    Done,
    ReadyToScroll,
    Retry,
    RuntimeMissing,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[doc(hidden)]
pub struct LocatorScreenshotGeometry {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

enum ScreenshotGeometryAttempt {
    Done(LocatorScreenshotGeometry),
    ReadyToScroll,
    Retry,
    RuntimeMissing,
}

enum WaitForAttempt {
    Done,
    Retry,
    RuntimeMissing,
}

enum TextContentAttempt {
    Done(TextContentResult),
    Retry,
    RuntimeMissing,
}

enum GetAttributeAttempt {
    Done(GetAttributeResult),
    Retry,
    RuntimeMissing,
}

enum InnerHtmlAttempt {
    Done(InnerHtmlResult),
    Retry,
    RuntimeMissing,
}

enum InnerTextAttempt {
    Done(InnerTextResult),
    Retry,
    RuntimeMissing,
}

enum InputValueAttempt {
    Done(InputValueResult),
    Retry,
    RuntimeMissing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FocusOperation {
    Focus,
    Blur,
}

impl FocusOperation {
    const fn name(self) -> &'static str {
        match self {
            Self::Focus => "focus",
            Self::Blur => "blur",
        }
    }
}

enum FocusOperationAttempt {
    Done,
    Retry,
    RuntimeMissing,
}

enum PressFocusAttempt {
    Done { is_mac: bool },
    Retry,
    RuntimeMissing,
}

fn actionability_script(
    selector: &Selector,
    force: bool,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    let enabled_check = enabled_actionability_check(force);
    Ok(format!(
        r#"(async () => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "missing" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element.isConnected) return {{ status: "detached" }};
  const style = getComputedStyle(element);
  let rect = element.getBoundingClientRect();
  if (style.display === "none" || rect.width <= 0 || rect.height <= 0 || (!{force} && style.visibility === "hidden"))
    return {{ status: "notvisible" }};
{enabled_check}
  element.scrollIntoView({{ block: "center", inline: "center", behavior: "instant" }});
  await new Promise(resolve => {{
    if (document.visibilityState !== "visible") {{ setTimeout(resolve, 0); return; }}
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }});
  if (!element.isConnected) return {{ status: "detached" }};
  const next = element.getBoundingClientRect();
  if (!{force} && (Math.abs(rect.x - next.x) > 0.25 || Math.abs(rect.y - next.y) > 0.25 ||
      Math.abs(rect.width - next.width) > 0.25 || Math.abs(rect.height - next.height) > 0.25))
    return {{ status: "unstable" }};
  rect = next;
  const left = Math.max(0, rect.left);
  const top = Math.max(0, rect.top);
  const right = Math.min(window.innerWidth, rect.right);
  const bottom = Math.min(window.innerHeight, rect.bottom);
  if (right <= left || bottom <= top) return {{ status: "out-of-viewport" }};
  const x = (left + right) / 2;
  const y = (top + bottom) / 2;
  if (!{force}) {{
    const hit = document.elementFromPoint(x, y);
    const target = !element.matches('input, textarea, select') && !element.isContentEditable
      ? (element.closest('button, [role=button], a, [role=link]') || element) : element;
    if (!hit || (hit !== target && !target.contains(hit))) return {{ status: "obscured" }};
  }}
  return {{ status: "ready", x, y }};
}})()"#
    ))
}

fn click_v2_actionability_script(
    selector: &Selector,
    options: &ClickOptionsV2,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    let element_setup = format!(
        r#"  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "missing" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];"#
    );
    Ok(format!(
        "(async () => {{\n{}\n}})()",
        click_v2_actionability_body(&element_setup, options, false)
    ))
}

fn click_v2_actionability_body(
    element_setup: &str,
    options: &ClickOptionsV2,
    retained_blocker_preview: bool,
) -> String {
    let position = options.position.map_or_else(
        || "null".to_string(),
        |position| format!(r#"{{ x: {}, y: {} }}"#, position.x(), position.y()),
    );
    let force = options.force.unwrap_or(false);
    let enabled_check = enabled_actionability_check(force);
    let scroll = matches!(options.scroll.unwrap_or(ScrollMode::Auto), ScrollMode::Auto);
    let obscured_result = if retained_blocker_preview {
        r#"{
      const blockerTag = hit ? String(hit.localName || "").slice(0, 25) : "";
      const blockerId = hit ? String(hit.id || "").slice(0, 65) : "";
      return { status: "obscured", blockerTag, blockerId };
    }"#
    } else {
        r#"return { status: "obscured" };"#
    };
    let viewport_fields = if retained_blocker_preview {
        ", viewportWidth: window.innerWidth, viewportHeight: window.innerHeight"
    } else {
        ""
    };
    format!(
        r#"
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
{element_setup}
  if (!element.isConnected) return {{ status: "detached" }};
  const style = getComputedStyle(element);
  let rect = element.getBoundingClientRect();
  if (style.display === "none" || rect.width <= 0 || rect.height <= 0 || (!{force} && style.visibility === "hidden"))
    return {{ status: "notvisible" }};
{enabled_check}
  const position = {position};
  if ({scroll}) {{
    if (position) {{
      const positionPoint = () => {{
        const box = element.getBoundingClientRect();
        return {{
          x: box.left + (Number.parseFloat(style.borderLeftWidth) || 0) + position.x,
          y: box.top + (Number.parseFloat(style.borderTopWidth) || 0) + position.y,
        }};
      }};
      let ancestor = element.parentElement || element.getRootNode()?.host || null;
      while (ancestor) {{
        const overflow = getComputedStyle(ancestor);
        const scrollX = /^(auto|scroll|hidden)$/.test(overflow.overflowX) && ancestor.scrollWidth > ancestor.clientWidth;
        const scrollY = /^(auto|scroll|hidden)$/.test(overflow.overflowY) && ancestor.scrollHeight > ancestor.clientHeight;
        if (scrollX || scrollY) {{
          const box = ancestor.getBoundingClientRect();
          const left = box.left + ancestor.clientLeft;
          const top = box.top + ancestor.clientTop;
          const right = left + ancestor.clientWidth;
          const bottom = top + ancestor.clientHeight;
          const point = positionPoint();
          const dx = scrollX && (point.x < left || point.x >= right) ? point.x - (left + right) / 2 : 0;
          const dy = scrollY && (point.y < top || point.y >= bottom) ? point.y - (top + bottom) / 2 : 0;
          if (dx || dy) ancestor.scrollTo({{ left: ancestor.scrollLeft + dx, top: ancestor.scrollTop + dy, behavior: "instant" }});
        }}
        ancestor = ancestor.parentElement || ancestor.getRootNode()?.host || null;
      }}
      const point = positionPoint();
      const dx = point.x < 0 || point.x >= window.innerWidth ? point.x - window.innerWidth / 2 : 0;
      const dy = point.y < 0 || point.y >= window.innerHeight ? point.y - window.innerHeight / 2 : 0;
      if (dx || dy) window.scrollBy({{ left: dx, top: dy, behavior: "instant" }});
    }} else {{
      element.scrollIntoView({{ block: "center", inline: "center", behavior: "instant" }});
    }}
  }}
  await new Promise(resolve => {{
    if (document.visibilityState !== "visible") {{ setTimeout(resolve, 0); return; }}
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }});
  if (!element.isConnected) return {{ status: "detached" }};
  const next = element.getBoundingClientRect();
  if (!{force} && (Math.abs(rect.x - next.x) > 0.25 || Math.abs(rect.y - next.y) > 0.25 ||
      Math.abs(rect.width - next.width) > 0.25 || Math.abs(rect.height - next.height) > 0.25))
    return {{ status: "unstable" }};
  rect = next;
  let x;
  let y;
  if (position) {{
    x = rect.left + (Number.parseFloat(style.borderLeftWidth) || 0) + position.x;
    y = rect.top + (Number.parseFloat(style.borderTopWidth) || 0) + position.y;
  }} else {{
    const clientRect = Array.from(element.getClientRects(), candidate => ({{
      left: Math.max(0, candidate.left),
      top: Math.max(0, candidate.top),
      right: Math.min(window.innerWidth, candidate.right),
      bottom: Math.min(window.innerHeight, candidate.bottom),
    }})).find(candidate =>
      (candidate.right - candidate.left) * (candidate.bottom - candidate.top) > 0.99
    );
    if (!clientRect) return {{ status: "out-of-viewport" }};
    const {{ left, top, right, bottom }} = clientRect;
    if (right <= left || bottom <= top) return {{ status: "out-of-viewport" }};
    x = (left + right) / 2;
    y = (top + bottom) / 2;
  }}
  if (x < 0 || y < 0 || x >= window.innerWidth || y >= window.innerHeight)
    return {{ status: "out-of-viewport" }};
  if (!{force}) {{
    const hit = document.elementFromPoint(x, y);
    const target = !element.matches('input, textarea, select') && !element.isContentEditable
      ? (element.closest('button, [role=button], a, [role=link]') || element) : element;
    if (!hit || (hit !== target && !target.contains(hit))) {obscured_result}
  }}
  return {{ status: "ready", x, y, isMac: /Mac|iPhone|iPad|iPod/.test(navigator.platform){viewport_fields} }};"#,
    )
}

#[cfg(feature = "retained-element-click")]
const RETAINED_CLICK_TARGET_SETUP: &str = r#"  const ELEMENT_NODE = 1;
  if (!node || typeof node.nodeType !== "number") return { status: "invalid-target" };
  const element = node.nodeType === ELEMENT_NODE ? node : node.parentElement;
  if (!element || !node.isConnected || !element.isConnected) return { status: "detached" };
  if (element.nodeType !== ELEMENT_NODE) return { status: "invalid-target" };
  if (element.ownerDocument?.defaultView !== globalThis) return { status: "frame-mismatch" };"#;

#[cfg(feature = "retained-element-click")]
fn retained_click_actionability_script(options: &ClickOptionsV2, allow_child: bool) -> String {
    let frame_check = if allow_child {
        ""
    } else {
        "  if (globalThis !== globalThis.top) return { status: \"non-top-level\" };"
    };
    let setup = format!("{RETAINED_CLICK_TARGET_SETUP}\n{frame_check}");
    let body = click_v2_actionability_body(&setup, options, true);
    format!(
        "async function(node) {{\n{};\n{}\n}}",
        selector_runtime_install_script(),
        body
    )
}

#[cfg(feature = "retained-element-click")]
fn retained_click_route_check_script(point: ClickPoint, force: bool, allow_child: bool) -> String {
    let frame_check = if allow_child {
        ""
    } else {
        "if (globalThis !== globalThis.top) return { status: \"non-top-level\" };"
    };
    format!(
        r#"function(node) {{
  {RETAINED_CLICK_TARGET_SETUP}
  {frame_check}
  const x = {x};
  const y = {y};
  if (x < 0 || y < 0 || x >= window.innerWidth || y >= window.innerHeight)
    return {{ status: "moved" }};
  const containsPoint = Array.from(element.getClientRects()).some(rect =>
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom);
  if (!containsPoint) return {{ status: "moved" }};
  if (!{force}) {{
    const hit = document.elementFromPoint(x, y);
    const target = !element.matches('input, textarea, select') && !element.isContentEditable
      ? (element.closest('button, [role=button], a, [role=link]') || element) : element;
    if (!hit || (hit !== target && !target.contains(hit))) return {{ status: "moved" }};
  }}
  return {{ status: "ready" }};
}}"#,
        x = point.x,
        y = point.y
    )
}

fn enabled_actionability_check(force: bool) -> String {
    format!(
        r#"  if (!{force}) {{
    const enabled = __pw.elementState(element, "enabled");
    if (enabled.status === "retry") return {{ status: "detached" }};
    if (enabled.status !== "done")
      return {{ status: "invalid", message: enabled.message || "Unable to determine enabled state" }};
    if (!enabled.value) return {{ status: "disabled" }};
  }}"#
    )
}

fn hover_actionability_script(
    selector: &Selector,
    options: HoverOptions,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    let position = options.position.map_or_else(
        || "null".to_string(),
        |position| format!(r#"{{ x: {}, y: {} }}"#, position.x(), position.y()),
    );
    let scroll = matches!(options.scroll, ScrollMode::Auto);
    Ok(format!(
        r#"(async () => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "missing" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element.isConnected) return {{ status: "detached" }};
  const style = getComputedStyle(element);
  let rect = element.getBoundingClientRect();
  if (!{force} && (style.visibility === "hidden" || style.display === "none" || rect.width <= 0 || rect.height <= 0))
    return {{ status: "notvisible" }};
  if ({scroll}) element.scrollIntoView({{ block: "center", inline: "center", behavior: "instant" }});
  await new Promise(resolve => {{
    if (document.visibilityState !== "visible") {{ setTimeout(resolve, 0); return; }}
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }});
  if (!element.isConnected) return {{ status: "detached" }};
  const next = element.getBoundingClientRect();
  if (!{force} && (Math.abs(rect.x - next.x) > 0.25 || Math.abs(rect.y - next.y) > 0.25 ||
      Math.abs(rect.width - next.width) > 0.25 || Math.abs(rect.height - next.height) > 0.25))
    return {{ status: "unstable" }};
  rect = next;
  const position = {position};
  let x;
  let y;
  if (position) {{
    x = rect.left + (Number.parseFloat(style.borderLeftWidth) || 0) + position.x;
    y = rect.top + (Number.parseFloat(style.borderTopWidth) || 0) + position.y;
  }} else {{
    const left = Math.max(0, rect.left);
    const top = Math.max(0, rect.top);
    const right = Math.min(window.innerWidth, rect.right);
    const bottom = Math.min(window.innerHeight, rect.bottom);
    if (right <= left || bottom <= top) return {{ status: "out-of-viewport" }};
    x = (left + right) / 2;
    y = (top + bottom) / 2;
  }}
  if (x < 0 || y < 0 || x >= window.innerWidth || y >= window.innerHeight)
    return {{ status: "out-of-viewport" }};
  if (!{force}) {{
    const hit = document.elementFromPoint(x, y);
    if (!hit || (hit !== element && !element.contains(hit))) return {{ status: "obscured" }};
  }}
  return {{ status: "ready", x, y, isMac: /Mac|iPhone|iPad|iPod/.test(navigator.platform) }};
}})()"#,
        force = options.force,
    ))
}

fn count_script(selector: &Selector) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  return {{ status: "done", count: matches.length }};
}})()"#
    ))
}

fn all_inner_texts_script(
    selector: &Selector,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (!matches.every(element => element instanceof HTMLElement))
    return {{ status: "invalid-target", message: "Node is not an HTMLElement" }};
  return {{ status: "done", values: matches.map(element => element.innerText) }};
}})()"#
    ))
}

fn all_text_contents_script(
    selector: &Selector,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  return {{ status: "done", values: matches.map(element => element.textContent || '') }};
}})()"#
    ))
}

fn visibility_script(selector: &Selector) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "missing" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  return {{ status: "done", visible: __pw.visible(matches[0]) }};
}})()"#
    ))
}

fn element_state_script(
    selector: &Selector,
    state: LocatorElementState,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    let state = state.as_str();
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "retry" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  return __pw.elementState(matches[0], "{state}");
}})()"#
    ))
}

fn set_checked_state_script(
    selector: &Selector,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "retry" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const state = __pw.elementState(matches[0], "checked");
  if (state.status !== "done") return state;
  const element = __pw.retarget(matches[0]);
  return {{ status: "done", checked: state.value,
    isRadio: element?.tagName === "INPUT" && element.type === "radio" }};
}})()"#
    ))
}

fn select_option_script(
    selector: &Selector,
    values: &[SelectOptionValue],
    options: SelectOptionOptions,
    commit_token: Option<&str>,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    validate_select_option_input(values)?;
    let selector = compile_selector(selector, "document")?;
    let requested = select_option_values_json(values)?;
    let force = options.force.unwrap_or(false);
    #[cfg(feature = "select-option-navigation-commit")]
    let commit_call = commit_token
        .map(|token| {
            let token = serde_json::to_string(token).map_err(|error| {
                invalid_response(&format!(
                    "could not encode select-option commit receipt: {error}"
                ))
            })?;
            Ok::<_, OperationError<LocatorErrorDetails>>(format!(
                r#"if (committedValues.length <= {MAX_SELECT_OPTION_ITEMS} &&
      committedValues.every(value => value.length <= {MAX_SELECT_OPTION_ITEM_BYTES}) &&
      committedValues.reduce((size, value) => size + value.length, 0) <= {MAX_SELECT_OPTION_TOTAL_BYTES}) {{
    const receipt = JSON.stringify({{ token: {token}, values: committedValues }});
    if (receipt.length <= {MAX_SELECT_OPTION_RECEIPT_CODE_UNITS})
      globalThis[{FILL_COMMIT_BINDING:?}](receipt);
  }}"#,
            ))
        })
        .transpose()?
        .unwrap_or_default();
    #[cfg(not(feature = "select-option-navigation-commit"))]
    let commit_call = {
        let _ = commit_token;
        String::new()
    };
    let script = format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "retry" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = __pw.retarget(matches[0]);
  if (!element || !element.isConnected) return {{ status: "retry" }};
  if (element.tagName !== "SELECT")
    return {{ status: "invalid-target", message: "Element is not a <select> element" }};
  if (!{force}) {{
    if (!__pw.visible(element)) return {{ status: "notvisible" }};
    const state = __pw.elementState(element, "enabled");
    if (state.status !== "done" || !state.value) return {{ status: "disabled" }};
  }}
  const requested = {requested};
  const normalize = value => String(value ?? '').replace(/\s+/g, ' ').trim();
  const options = Array.from(element.options);
  const selected = [];
  let remaining = requested.slice();
  for (let index = 0; index < options.length; ++index) {{
    const option = options[index];
    const normalizedLabel = normalize(option.label);
    const filter = descriptor => {{
      const matchesLabel = label => label === option.label || normalize(label) === normalizedLabel;
      let matches = true;
      if (descriptor.valueOrLabel !== undefined)
        matches = matches && (descriptor.valueOrLabel === option.value || matchesLabel(descriptor.valueOrLabel));
      if (descriptor.value !== undefined) matches = matches && descriptor.value === option.value;
      if (descriptor.label !== undefined) matches = matches && matchesLabel(descriptor.label);
      if (descriptor.index !== undefined) matches = matches && descriptor.index === index;
      return matches;
    }};
    if (!remaining.some(filter)) continue;
    const optionState = __pw.elementState(option, "enabled");
    if (optionState.status !== "done" || !optionState.value)
      return {{ status: "option-not-enabled" }};
    selected.push(option);
    if (element.multiple) remaining = remaining.filter(value => !filter(value));
    else {{ remaining = []; break; }}
  }}
  if (remaining.length) return {{ status: "options-not-found" }};
  element.value = undefined;
  selected.forEach(option => option.selected = true);
  const committedValues = selected.map(option => option.value);
  {commit_call}
  element.dispatchEvent(new Event("input", {{ bubbles: true, composed: true }}));
  element.dispatchEvent(new Event("change", {{ bubbles: true }}));
  const values = selected.map(option => option.value);
  return {{ status: "done", values }};
}})()"#,
    );
    if script.len() > MAX_SELECT_OPTION_SCRIPT_BYTES {
        return Err(select_option_resource_error(
            "select-option script exceeds its declared backend bound",
        ));
    }
    Ok(script)
}

fn scroll_into_view_if_needed_check_script(
    selector: &Selector,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    let script = format!(
        r#"(async () => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "missing" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element || !element.isConnected) return {{ status: "detached" }};
  if (element.nodeType !== 1)
    return {{ status: "invalid-target", message: "Node is not an Element" }};
  if (element.getClientRects().length === 0) return {{ status: "no-layout" }};
  const before = element.getBoundingClientRect();
  await new Promise(resolve => {{
    if (document.visibilityState !== "visible") {{ setTimeout(resolve, 0); return; }}
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }});
  if (!element.isConnected) return {{ status: "detached" }};
  if (element.getClientRects().length === 0) return {{ status: "no-layout" }};
  const after = element.getBoundingClientRect();
  const stable = Math.abs(before.x - after.x) <= 0.25 &&
    Math.abs(before.y - after.y) <= 0.25 &&
    Math.abs(before.width - after.width) <= 0.25 &&
    Math.abs(before.height - after.height) <= 0.25;
  if (!stable) return {{ status: "unstable" }};
  let ratio;
  try {{
    ratio = await new Promise(resolve => {{
      const observer = new IntersectionObserver(entries => {{
        observer.disconnect();
        resolve(entries[0]?.intersectionRatio ?? 0);
      }});
      observer.observe(element);
    }});
  }} catch (error) {{ return {{ status: "backend", message: String(error) }}; }}
  if (!element.isConnected) return {{ status: "detached" }};
  return {{ status: ratio === 1 ? "done" : "ready-to-scroll" }};
}})()"#,
    );
    if script.len() > MAX_FOCUS_SCRIPT_BYTES {
        return Err(locator_resource_error(
            "scroll-into-view script exceeds its declared backend bound",
        ));
    }
    Ok(script)
}

fn screenshot_geometry_check_script(
    selector: &Selector,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    let script = format!(
        r#"(async () => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "missing" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element || !element.isConnected) return {{ status: "detached" }};
  if (element.nodeType !== 1)
    return {{ status: "invalid-target", message: "Node is not an Element" }};
  if (!__pw.visible(element) || element.getClientRects().length === 0)
    return {{ status: "notvisible" }};
  const before = element.getBoundingClientRect();
  await new Promise(resolve => {{
    if (document.visibilityState !== "visible") {{ setTimeout(resolve, 0); return; }}
    requestAnimationFrame(() => requestAnimationFrame(resolve));
  }});
  if (!element.isConnected) return {{ status: "detached" }};
  if (!__pw.visible(element) || element.getClientRects().length === 0)
    return {{ status: "notvisible" }};
  const after = element.getBoundingClientRect();
  const stable = Math.abs(before.x - after.x) <= 0.25 &&
    Math.abs(before.y - after.y) <= 0.25 &&
    Math.abs(before.width - after.width) <= 0.25 &&
    Math.abs(before.height - after.height) <= 0.25;
  if (!stable) return {{ status: "unstable" }};
  let ratio;
  try {{
    ratio = await new Promise(resolve => {{
      const observer = new IntersectionObserver(entries => {{
        observer.disconnect();
        resolve(entries[0]?.intersectionRatio ?? 0);
      }});
      observer.observe(element);
    }});
  }} catch (error) {{ return {{ status: "backend", message: String(error) }}; }}
  if (!element.isConnected) return {{ status: "detached" }};
  // Oversized elements can never reach an intersection ratio of one, but
  // Playwright still captures their complete layout box. Scroll only when the
  // element is entirely outside the viewport.
  if (ratio <= 0) return {{ status: "ready-to-scroll" }};
  const final = element.getBoundingClientRect();
  const finalStable = Math.abs(after.x - final.x) <= 0.25 &&
    Math.abs(after.y - final.y) <= 0.25 &&
    Math.abs(after.width - final.width) <= 0.25 &&
    Math.abs(after.height - final.height) <= 0.25;
  if (!finalStable) return {{ status: "unstable" }};
  const x = Math.floor(final.left);
  const y = Math.floor(final.top);
  const right = Math.ceil(final.right);
  const bottom = Math.ceil(final.bottom);
  if (![x, y, right, bottom].every(Number.isFinite) || right <= x || bottom <= y)
    return {{ status: "no-layout" }};
  return {{ status: "done", x, y, width: right - x, height: bottom - y }};
}})()"#,
    );
    if script.len() > MAX_FOCUS_SCRIPT_BYTES {
        return Err(locator_resource_error(
            "Locator screenshot geometry script exceeds its declared backend bound",
        ));
    }
    Ok(script)
}

fn scroll_into_view_if_needed_commit_script(
    selector: &Selector,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    let script = format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "missing" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element || !element.isConnected) return {{ status: "detached" }};
  if (element.nodeType !== 1)
    return {{ status: "invalid-target", message: "Node is not an Element" }};
  if (element.getClientRects().length === 0) return {{ status: "no-layout" }};
  try {{
    element.scrollIntoView({{ behavior: "instant", block: "center", inline: "center" }});
  }} catch (error) {{ return {{ status: "backend", message: String(error) }}; }}
  return {{ status: "done" }};
}})()"#,
    );
    if script.len() > MAX_FOCUS_SCRIPT_BYTES {
        return Err(locator_resource_error(
            "scroll-into-view commit script exceeds its declared backend bound",
        ));
    }
    Ok(script)
}

fn wait_for_script(
    selector: &Selector,
    state: WaitForState,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    let state = match state {
        WaitForState::Attached => "attached",
        WaitForState::Detached => "detached",
        WaitForState::Visible => "visible",
        WaitForState::Hidden => "hidden",
    };
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  const attached = !!element && element.isConnected;
  const visible = attached && __pw.visible(element);
  const success = {{ attached, detached: !attached, visible, hidden: !visible }}["{state}"];
  return {{ status: success ? "done" : "retry" }};
}})()"#
    ))
}

fn text_content_script(selector: &Selector) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "retry" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element.isConnected) return {{ status: "retry" }};
  return {{ status: "done", value: element.textContent }};
}})()"#
    ))
}

fn get_attribute_script(
    selector: &Selector,
    name: &str,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    if name.len() > MAX_ATTRIBUTE_NAME_BYTES {
        return Err(attribute_name_resource_error(
            "attribute name exceeds the 65536-byte backend limit",
        ));
    }
    let selector = compile_selector(selector, "document")?;
    let name = serde_json::to_string(name)
        .map_err(|error| invalid_response(&format!("could not encode attribute name: {error}")))?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "retry" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element.isConnected) return {{ status: "retry" }};
  try {{ return {{ status: "done", value: element.getAttribute({name}) }}; }}
  catch (error) {{ return {{ status: "backend", message: String(error) }}; }}
}})()"#
    ))
}

fn inner_text_script(selector: &Selector) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "retry" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element.isConnected) return {{ status: "retry" }};
  if (element.namespaceURI !== "http://www.w3.org/1999/xhtml")
    return {{ status: "invalid-target", message: "Node is not an HTMLElement" }};
  return {{ status: "done", value: element.innerText }};
}})()"#
    ))
}

fn input_value_script(selector: &Selector) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "retry" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = __pw.retarget(matches[0]);
  if (!element || !element.isConnected) return {{ status: "retry" }};
  if (!['INPUT', 'TEXTAREA', 'SELECT'].includes(element.nodeName))
    return {{ status: "invalid-target", message: "Node is not an <input>, <textarea> or <select> element" }};
  return {{ status: "done", value: element.value }};
}})()"#
    ))
}

fn focus_operation_script(
    selector: &Selector,
    operation: FocusOperation,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    let invocation = match operation {
        FocusOperation::Focus => "element.focus(); element.focus();",
        FocusOperation::Blur => "element.blur();",
    };
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "retry" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element.isConnected) return {{ status: "retry" }};
  if (element.nodeType !== 1)
    return {{ status: "invalid-target", message: "Node is not an element" }};
  try {{ {invocation} }}
  catch (error) {{ return {{ status: "backend", message: String(error) }}; }}
  return {{ status: "done" }};
}})()"#
    ))
}

fn press_focus_script(selector: &Selector) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "retry" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element.isConnected) return {{ status: "retry" }};
  if (element.nodeType !== 1)
    return {{ status: "invalid-target", message: "Node is not an element" }};
  try {{
    const wasFocused = element.ownerDocument.activeElement === element;
    element.focus();
    element.focus();
    if (!wasFocused && element.nodeName === "INPUT" && typeof element.setSelectionRange === "function") {{
      try {{ element.setSelectionRange(0, 0); }} catch (_) {{}}
    }}
  }} catch (error) {{ return {{ status: "backend", message: String(error) }}; }}
  return {{ status: "done", isMac: navigator.platform.includes("Mac") }};
}})()"#
    ))
}

fn inner_html_script(selector: &Selector) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "retry" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  const element = matches[0];
  if (!element.isConnected) return {{ status: "retry" }};
  return {{ status: "done", value: element.innerHTML }};
}})()"#
    ))
}

fn fill_actionability_script(
    selector: &Selector,
    value: &str,
    force: bool,
    commit_token: Option<&str>,
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let selector = compile_selector(selector, "document")?;
    let value = serde_json::to_string(value)
        .map_err(|error| invalid_response(&format!("could not encode fill value: {error}")))?;
    let commit_call = commit_token
        .map(|token| {
            let token = serde_json::to_string(token).map_err(|error| {
                invalid_response(&format!("could not encode fill commit receipt: {error}"))
            })?;
            Ok::<_, OperationError<LocatorErrorDetails>>(format!(
                "globalThis[{FILL_COMMIT_BINDING:?}]({token});"
            ))
        })
        .transpose()?
        .unwrap_or_default();
    Ok(format!(
        r#"(() => {{
  const __pw = globalThis[Symbol.for({SELECTOR_RUNTIME_SYMBOL:?})];
  if (!__pw || __pw.version !== {SELECTOR_RUNTIME_VERSION}) return {{ status: "runtime-missing" }};
  let matches;
  try {{ matches = {selector}; }}
  catch (error) {{ return {{ status: "invalid", message: String(error) }}; }}
  if (matches.length === 0) return {{ status: "missing" }};
  if (matches.length > 1) return {{ status: "strict", count: matches.length }};
  let element = matches[0];
  if (element instanceof HTMLLabelElement && element.control) element = element.control;
  if (!element.isConnected) return {{ status: "detached" }};
  const style = getComputedStyle(element);
  const rect = element.getBoundingClientRect();
  if (!{force} && (style.visibility === "hidden" || style.display === "none" || rect.width <= 0 || rect.height <= 0))
    return {{ status: "notvisible" }};
  if (!{force} && (element.matches(':disabled') || element.getAttribute("aria-disabled") === "true"))
    return {{ status: "disabled" }};
  const value = {value};
  const tag = element.tagName;
  if (tag === 'INPUT') {{
    const type = (element.type || 'text').toLowerCase();
    const direct = new Set(['color','date','time','datetime-local','month','range','week']);
    const textual = new Set(['','email','number','password','search','tel','text','url']);
    if (!direct.has(type) && !textual.has(type))
      return {{ status: "invalid-target", message: `Input of type "${{type}}" cannot be filled` }};
    if (!{force} && (element.readOnly || element.getAttribute("aria-readonly") === "true"))
      return {{ status: "noteditable" }};
    if (type === 'number' && value !== '' && Number.isNaN(Number(value)))
      return {{ status: "invalid-value", message: `Cannot type text into input[type=number]` }};
    if (direct.has(type)) {{
      // Color values are ASCII hex and the DOM value setter canonicalizes
      // valid mixed-case input to lowercase before the validation readback.
      const expected = type === 'color' ? value.trim().toLowerCase() : value.trim();
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
      setter.call(element, expected);
      if (element.value !== expected)
        return {{ status: "invalid-value", message: `Malformed value for input[type=${{type}}]` }};
      element.dispatchEvent(new Event('input', {{ bubbles: true, composed: true }}));
      element.dispatchEvent(new Event('change', {{ bubbles: true }}));
      {commit_call}
      return {{ status: "done" }};
    }}
    element.focus();
    element.select();
    return {{ status: "needs-input", isMac: navigator.platform.includes("Mac") }};
  }}
  if (tag === 'TEXTAREA') {{
    if (!{force} && (element.readOnly || element.getAttribute("aria-readonly") === "true"))
      return {{ status: "noteditable" }};
    element.focus();
    element.select();
    return {{ status: "needs-input", isMac: navigator.platform.includes("Mac") }};
  }}
  if (element.isContentEditable) {{
    if (!{force} && element.getAttribute("aria-readonly") === "true")
      return {{ status: "noteditable" }};
    element.focus();
    const range = element.ownerDocument.createRange();
    range.selectNodeContents(element);
    const selection = element.ownerDocument.getSelection();
    selection.removeAllRanges();
    selection.addRange(range);
    return {{ status: "needs-input", isMac: navigator.platform.includes("Mac") }};
  }}
  return {{ status: "invalid-target", message: "Element is not an <input>, <textarea> or [contenteditable] element" }};
}})()"#
    ))
}

fn validate_select_option_input(
    values: &[SelectOptionValue],
) -> Result<(), OperationError<LocatorErrorDetails>> {
    if values.len() > MAX_SELECT_OPTION_ITEMS {
        return Err(select_option_resource_error(
            "select-option descriptors exceed the 1024-item backend limit",
        ));
    }
    let mut aggregate_length = 0usize;
    let mut aggregate_capacity = 0usize;
    for value in values {
        for text in [
            value.value_or_label.as_ref(),
            value.value.as_ref(),
            value.label.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            if text.len() > MAX_SELECT_OPTION_ITEM_BYTES
                || text.capacity() > MAX_SELECT_OPTION_ITEM_BYTES
            {
                return Err(select_option_resource_error(
                    "select-option descriptor string exceeds the 65536-byte backend limit",
                ));
            }
            aggregate_length = aggregate_length.saturating_add(text.len());
            aggregate_capacity = aggregate_capacity.saturating_add(text.capacity());
        }
    }
    if aggregate_length > MAX_SELECT_OPTION_TOTAL_BYTES
        || aggregate_capacity > MAX_SELECT_OPTION_TOTAL_BYTES
    {
        return Err(select_option_resource_error(
            "select-option descriptor strings exceed the 1048576-byte aggregate backend limit",
        ));
    }
    Ok(())
}

fn select_option_values_json(
    values: &[SelectOptionValue],
) -> Result<String, OperationError<LocatorErrorDetails>> {
    let values = values
        .iter()
        .map(|value| {
            let mut object = serde_json::Map::new();
            if let Some(value) = &value.value_or_label {
                object.insert("valueOrLabel".to_owned(), Value::String(value.clone()));
            }
            if let Some(value) = &value.value {
                object.insert("value".to_owned(), Value::String(value.clone()));
            }
            if let Some(label) = &value.label {
                object.insert("label".to_owned(), Value::String(label.clone()));
            }
            if let Some(index) = value.index {
                object.insert("index".to_owned(), Value::from(index));
            }
            Value::Object(object)
        })
        .collect::<Vec<_>>();
    serde_json::to_string(&values)
        .map_err(|error| invalid_response(&format!("could not encode select options: {error}")))
}

fn selector_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::Backend(message.to_string())),
    )
}

fn locator_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::InvalidValue(message.to_string())),
    )
}

fn invalid_press_key(message: String) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::Backend,
        message.clone(),
        Some(LocatorErrorDetails::InvalidValue(message)),
    )
}

fn set_checked_target_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::Backend,
        message,
        Some(LocatorErrorDetails::InvalidTarget(message.to_string())),
    )
}

fn select_option_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::InvalidValue(message.to_string())),
    )
}

fn select_option_target_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::Backend,
        message,
        Some(LocatorErrorDetails::InvalidTarget(message.to_string())),
    )
}

fn click_navigation_stream_error(skipped: u64) -> OperationError<LocatorErrorDetails> {
    let message = format!("click navigation event stream lagged by {skipped} events");
    OperationError::new(
        FailureKind::ResourceExhausted,
        message.clone(),
        Some(LocatorErrorDetails::Backend(message)),
    )
}

fn click_navigation_subscription_error(
    error: CdpEventSubscriptionError,
) -> OperationError<LocatorErrorDetails> {
    let message = error.to_string();
    OperationError::new(
        FailureKind::ResourceExhausted,
        message.clone(),
        Some(LocatorErrorDetails::Backend(message)),
    )
}

fn click_navigation_stream_closed() -> OperationError<LocatorErrorDetails> {
    let message = "click navigation event stream closed";
    OperationError::new(
        FailureKind::TransportClosed,
        message,
        Some(LocatorErrorDetails::Backend(message.to_string())),
    )
}

fn fill_value_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::InvalidValue(message.to_string())),
    )
}

fn attribute_name_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::InvalidValue(message.to_string())),
    )
}

fn attribute_result_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::Backend(message.to_string())),
    )
}

fn text_content_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::Backend(message.to_string())),
    )
}

fn inner_html_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::Backend(message.to_string())),
    )
}

fn inner_text_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::Backend(message.to_string())),
    )
}

fn input_value_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::Backend(message.to_string())),
    )
}

fn all_inner_texts_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::Backend(message.to_string())),
    )
}

fn all_text_contents_resource_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ResourceExhausted,
        message,
        Some(LocatorErrorDetails::Backend(message.to_string())),
    )
}

fn map_locator_resource_error(error: ResourceError) -> OperationError<LocatorErrorDetails> {
    let message = error.to_string();
    OperationError::new(
        FailureKind::ResourceExhausted,
        message.clone(),
        Some(LocatorErrorDetails::Backend(message)),
    )
}

fn parse_attempt(
    response: Value,
    selector: &str,
    force: bool,
) -> Result<ActionAttempt, OperationError<LocatorErrorDetails>> {
    parse_pointer_attempt(response, selector, false, force)
}

fn parse_hover_attempt(
    response: Value,
    selector: &str,
    force: bool,
) -> Result<ActionAttempt, OperationError<LocatorErrorDetails>> {
    parse_pointer_attempt(response, selector, true, force)
}

fn parse_pointer_attempt(
    response: Value,
    selector: &str,
    require_platform: bool,
    force: bool,
) -> Result<ActionAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator actionability result is missing status"))?;
    match status {
        "ready" => {
            let x = coordinate(value, "x")?;
            let y = coordinate(value, "y")?;
            let is_mac = match value.get("isMac") {
                Some(value) => value.as_bool().ok_or_else(|| {
                    invalid_response("locator actionability returned an invalid platform marker")
                })?,
                None if !require_platform => false,
                None => {
                    return Err(invalid_response(
                        "locator hover actionability is missing platform marker",
                    ));
                }
            };
            Ok(ActionAttempt::Ready(ClickPoint { x, y, is_mac }))
        }
        "missing" => Ok(ActionAttempt::Retry(ActionabilityProgress::Missing)),
        "detached" => Ok(ActionAttempt::Retry(ActionabilityProgress::Detached)),
        "notvisible" if force => Err(OperationError::new(
            FailureKind::Backend,
            "Element is not visible",
            Some(LocatorErrorDetails::Backend(
                "Element is not visible".to_owned(),
            )),
        )),
        "notvisible" => Ok(ActionAttempt::Retry(ActionabilityProgress::NotVisible)),
        "out-of-viewport" if force => Err(OperationError::new(
            FailureKind::Backend,
            "Element is outside of the viewport",
            Some(LocatorErrorDetails::Backend(
                "Element is outside of the viewport".to_owned(),
            )),
        )),
        "out-of-viewport" => Ok(ActionAttempt::Retry(ActionabilityProgress::NotVisible)),
        "unstable" => Ok(ActionAttempt::Retry(ActionabilityProgress::Unstable)),
        "disabled" => Ok(ActionAttempt::Retry(ActionabilityProgress::Disabled)),
        "obscured" => Ok(ActionAttempt::Retry(ActionabilityProgress::Obscured)),
        "runtime-missing" => Ok(ActionAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "CSS selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict CSS selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("invalid CSS selector")
                .to_string();
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid CSS selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator actionability returned unknown status `{other}`"
        ))),
    }
}

#[cfg(feature = "retained-element-click")]
fn validate_retained_click_native_id(
    value: &str,
    label: &str,
) -> Result<(), OperationError<LocatorErrorDetails>> {
    if value.is_empty() || value.len() > MAX_RETAINED_CLICK_NATIVE_ID_BYTES {
        return Err(locator_resource_error(&format!(
            "retained Element {label} must contain between 1 and {MAX_RETAINED_CLICK_NATIVE_ID_BYTES} bytes"
        )));
    }
    Ok(())
}

#[cfg(feature = "retained-element-click")]
fn retained_click_target_error(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::Backend,
        message,
        Some(LocatorErrorDetails::InvalidTarget(message.to_owned())),
    )
}

#[cfg(feature = "retained-element-click")]
fn retained_click_status(response: &Value) -> Result<&str, OperationError<LocatorErrorDetails>> {
    response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(|value| value.get("status"))
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("retained Element click response is missing status"))
}

#[cfg(feature = "retained-element-click")]
fn retained_click_status_error(status: &str) -> Option<OperationError<LocatorErrorDetails>> {
    match status {
        "detached" => Some(OperationError::new(
            FailureKind::Backend,
            "Element is not attached to the DOM",
            Some(LocatorErrorDetails::InvalidTarget(status.to_owned())),
        )),
        "frame-mismatch" => Some(OperationError::new(
            FailureKind::TargetClosed,
            "retained Element moved to another Frame",
            Some(LocatorErrorDetails::InvalidTarget(status.to_owned())),
        )),
        "non-top-level" => Some(retained_click_target_error(
            "retained Element click requires a top-level Frame; child-frame coordinate translation is unavailable",
        )),
        "invalid-target" => Some(retained_click_target_error(
            "retained click target is not an Element",
        )),
        _ => None,
    }
}

#[cfg(feature = "retained-element-click")]
fn parse_retained_click_attempt(
    response: Value,
    force: bool,
    diagnostics: &RetainedClickDiagnostics,
) -> Result<ActionAttempt, OperationError<LocatorErrorDetails>> {
    if let Some(error) = retained_click_status_error(retained_click_status(&response)?) {
        return Err(error);
    }
    let blocker = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .and_then(|value| {
            Some((
                value.get("blockerTag")?.as_str()?,
                value.get("blockerId")?.as_str()?,
            ))
        })
        .and_then(|(tag, id)| RetainedClickDiagnostics::blocker_preview(tag, id));
    let attempt = parse_pointer_attempt(response, "retained Element", true, force)?;
    if let ActionAttempt::Retry(progress) = attempt {
        diagnostics.record(progress, blocker);
    }
    Ok(attempt)
}

#[cfg(feature = "retained-element-click")]
fn verify_retained_click_route(
    response: Value,
) -> Result<bool, OperationError<LocatorErrorDetails>> {
    let status = retained_click_status(&response)?;
    if let Some(error) = retained_click_status_error(status) {
        return Err(error);
    }
    if status == "ready" {
        Ok(true)
    } else if status == "moved" {
        Ok(false)
    } else {
        Err(invalid_response(&format!(
            "retained Element route check returned unknown status `{status}`"
        )))
    }
}

fn parse_fill_attempt(
    response: Value,
    selector: &str,
) -> Result<FillAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator fill result is missing status"))?;
    match status {
        "done" => Ok(FillAttempt::Done),
        "needs-input" => Ok(FillAttempt::NeedsInput {
            is_mac: value
                .get("isMac")
                .and_then(Value::as_bool)
                .ok_or_else(|| invalid_response("locator fill result is missing isMac"))?,
        }),
        "missing" | "detached" | "notvisible" | "disabled" | "noteditable" => {
            Ok(FillAttempt::Retry)
        }
        "runtime-missing" => Ok(FillAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => {
            let message = status_message(value, "element cannot be filled");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::InvalidTarget(message)),
            ))
        }
        "invalid-value" => {
            let message = status_message(value, "value cannot be filled");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::InvalidValue(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator fill returned unknown status `{other}`"
        ))),
    }
}

fn parse_count_attempt(
    response: Value,
    selector: &str,
) -> Result<CountAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator count result is missing status"))?;
    match status {
        "done" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("locator count result is missing count"))?;
            let count = CountResult::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "locator match count exceeds the portable u32 limit",
                    Some(LocatorErrorDetails::Backend(
                        "locator match count exceeds the portable u32 limit".to_string(),
                    )),
                )
            })?;
            Ok(CountAttempt::Done(count))
        }
        "runtime-missing" => Ok(CountAttempt::RuntimeMissing),
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator count returned unknown status `{other}`"
        ))),
    }
}

fn parse_all_inner_texts_attempt(
    mut response: Value,
    selector: &str,
) -> Result<AllInnerTextsAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get_mut("result")
        .and_then(|result| result.get_mut("value"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator all-inner-texts result is missing status"))?
        .to_string();
    match status.as_str() {
        "done" => {
            let values = match value.remove("values") {
                Some(Value::Array(values)) => values,
                _ => {
                    return Err(invalid_response(
                        "locator all-inner-texts result values is not an array",
                    ));
                }
            };
            if values.len() > MAX_ALL_INNER_TEXTS_ITEMS {
                return Err(all_inner_texts_resource_error(
                    "all-inner-texts result exceeds the 4096-item backend limit",
                ));
            }

            let mut aggregate_bytes = 0usize;
            for value in &values {
                let text = value.as_str().ok_or_else(|| {
                    invalid_response("locator all-inner-texts result contains a non-string")
                })?;
                if text.len() > MAX_ALL_INNER_TEXTS_ITEM_BYTES {
                    return Err(all_inner_texts_resource_error(
                        "all-inner-texts item exceeds the 1048576-byte backend limit",
                    ));
                }
                aggregate_bytes = aggregate_bytes.saturating_add(text.len());
                if aggregate_bytes > MAX_ALL_INNER_TEXTS_TOTAL_BYTES {
                    return Err(all_inner_texts_resource_error(
                        "all-inner-texts strings exceed the 4194304-byte aggregate backend limit",
                    ));
                }
            }

            let mut output = Vec::with_capacity(values.len());
            for value in values {
                let Value::String(mut text) = value else {
                    unreachable!("all collection elements were validated as strings")
                };
                text.shrink_to_fit();
                output.push(text);
            }
            output.shrink_to_fit();
            Ok(AllInnerTextsAttempt::Done(output))
        }
        "runtime-missing" => Ok(AllInnerTextsAttempt::RuntimeMissing),
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => {
            let message = status_message(value, "Node is not an HTMLElement");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("locator `{selector}` points to a non-HTMLElement: {message}"),
                Some(LocatorErrorDetails::InvalidTarget(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator all-inner-texts returned unknown status `{other}`"
        ))),
    }
}

fn parse_all_text_contents_attempt(
    mut response: Value,
    selector: &str,
) -> Result<AllTextContentsAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get_mut("result")
        .and_then(|result| result.get_mut("value"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator all-text-contents result is missing status"))?
        .to_string();
    match status.as_str() {
        "done" => {
            let values = match value.remove("values") {
                Some(Value::Array(values)) => values,
                _ => {
                    return Err(invalid_response(
                        "locator all-text-contents result values is not an array",
                    ));
                }
            };
            if values.len() > MAX_ALL_TEXT_CONTENTS_ITEMS {
                return Err(all_text_contents_resource_error(
                    "all-text-contents result exceeds the 4096-item backend limit",
                ));
            }

            let mut aggregate_bytes = 0usize;
            for value in &values {
                let text = value.as_str().ok_or_else(|| {
                    invalid_response("locator all-text-contents result contains a non-string")
                })?;
                if text.len() > MAX_ALL_TEXT_CONTENTS_ITEM_BYTES {
                    return Err(all_text_contents_resource_error(
                        "all-text-contents item exceeds the 1048576-byte backend limit",
                    ));
                }
                aggregate_bytes = aggregate_bytes.saturating_add(text.len());
                if aggregate_bytes > MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES {
                    return Err(all_text_contents_resource_error(
                        "all-text-contents strings exceed the 4194304-byte aggregate backend limit",
                    ));
                }
            }

            let mut output = Vec::with_capacity(values.len());
            for value in values {
                let Value::String(mut text) = value else {
                    unreachable!("all collection elements were validated as strings")
                };
                text.shrink_to_fit();
                output.push(text);
            }
            output.shrink_to_fit();
            Ok(AllTextContentsAttempt::Done(output))
        }
        "runtime-missing" => Ok(AllTextContentsAttempt::RuntimeMissing),
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator all-text-contents returned unknown status `{other}`"
        ))),
    }
}

fn parse_visibility_attempt(
    response: Value,
    selector: &str,
) -> Result<VisibilityAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator visibility result is missing status"))?;
    match status {
        "done" => value
            .get("visible")
            .and_then(Value::as_bool)
            .map(VisibilityAttempt::Done)
            .ok_or_else(|| invalid_response("locator visibility result is missing visible")),
        "missing" => Ok(VisibilityAttempt::Done(false)),
        "runtime-missing" => Ok(VisibilityAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator visibility returned unknown status `{other}`"
        ))),
    }
}

fn parse_element_state_attempt(
    response: Value,
    selector: &str,
    state: LocatorElementState,
) -> Result<ElementStateAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator element-state result is missing status"))?;
    match status {
        "done" => value
            .get("value")
            .and_then(Value::as_bool)
            .map(ElementStateAttempt::Done)
            .ok_or_else(|| invalid_response("locator element-state result is missing value")),
        "retry" => Ok(ElementStateAttempt::Retry),
        "runtime-missing" => Ok(ElementStateAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => {
            let message = status_message(value, "invalid element-state target");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("cannot read {} state: {message}", state.as_str()),
                Some(LocatorErrorDetails::InvalidTarget(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator {} state returned unknown status `{other}`",
            state.as_str()
        ))),
    }
}

fn parse_set_checked_state_attempt(
    response: Value,
    selector: &str,
) -> Result<SetCheckedStateAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator set-checked state result is missing status"))?;
    match status {
        "done" => {
            let checked = value
                .get("checked")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    invalid_response("locator set-checked state result is missing checked")
                })?;
            let is_radio = value
                .get("isRadio")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    invalid_response("locator set-checked state result is missing isRadio")
                })?;
            Ok(SetCheckedStateAttempt::Done { checked, is_radio })
        }
        "retry" => Ok(SetCheckedStateAttempt::Retry),
        "runtime-missing" => Ok(SetCheckedStateAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => Err(set_checked_target_error(&status_message(
            value,
            "Not a checkbox or radio button",
        ))),
        other => Err(invalid_response(&format!(
            "locator set-checked state returned unknown status `{other}`"
        ))),
    }
}

fn parse_select_option_attempt(
    mut response: Value,
    selector: &str,
) -> Result<SelectOptionAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get_mut("result")
        .and_then(|result| result.get_mut("value"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator select-option result is missing status"))?
        .to_owned();
    match status.as_str() {
        "done" => {
            let values = match value.remove("values") {
                Some(Value::Array(values)) => values,
                _ => {
                    return Err(invalid_response(
                        "locator select-option result values is not an array",
                    ));
                }
            };
            if values.len() > MAX_SELECT_OPTION_ITEMS {
                return Err(select_option_resource_error(
                    "select-option result exceeds the 1024-item backend limit",
                ));
            }
            let mut aggregate_bytes = 0usize;
            for value in &values {
                let text = value.as_str().ok_or_else(|| {
                    invalid_response("locator select-option result contains a non-string")
                })?;
                if text.len() > MAX_SELECT_OPTION_ITEM_BYTES {
                    return Err(select_option_resource_error(
                        "select-option result item exceeds the 65536-byte backend limit",
                    ));
                }
                aggregate_bytes = aggregate_bytes.saturating_add(text.len());
                if aggregate_bytes > MAX_SELECT_OPTION_TOTAL_BYTES {
                    return Err(select_option_resource_error(
                        "select-option result strings exceed the 1048576-byte aggregate backend limit",
                    ));
                }
            }
            let mut output = Vec::with_capacity(values.len());
            for value in values {
                let Value::String(mut text) = value else {
                    unreachable!("select-option result elements were validated as strings")
                };
                text.shrink_to_fit();
                output.push(text);
            }
            output.shrink_to_fit();
            Ok(SelectOptionAttempt::Done(output))
        }
        "retry" | "notvisible" | "disabled" | "options-not-found" | "option-not-enabled" => {
            Ok(SelectOptionAttempt::Retry(status == "option-not-enabled"))
        }
        "runtime-missing" => Ok(SelectOptionAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => Err(select_option_target_error(&status_message(
            value,
            "Element is not a <select> element",
        ))),
        other => Err(invalid_response(&format!(
            "locator select-option returned unknown status `{other}`"
        ))),
    }
}

fn parse_scroll_into_view_if_needed_attempt(
    response: Value,
    selector: &str,
) -> Result<ScrollIntoViewIfNeededAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator scroll-into-view result is missing status"))?;
    match status {
        "done" => Ok(ScrollIntoViewIfNeededAttempt::Done),
        "ready-to-scroll" => Ok(ScrollIntoViewIfNeededAttempt::ReadyToScroll),
        "missing" | "detached" | "no-layout" | "unstable" | "retry" => {
            Ok(ScrollIntoViewIfNeededAttempt::Retry)
        }
        "runtime-missing" => Ok(ScrollIntoViewIfNeededAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => {
            let message = status_message(value, "Node is not an Element");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::InvalidTarget(message)),
            ))
        }
        "backend" => {
            let message = status_message(value, "native scroll operation failed");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::Backend(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator scroll-into-view returned unknown status `{other}`"
        ))),
    }
}

fn parse_screenshot_geometry_attempt(
    response: Value,
    selector: &str,
) -> Result<ScreenshotGeometryAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("Locator screenshot geometry result is missing status"))?;
    match status {
        "done" => {
            let field = |name| {
                value.get(name).and_then(Value::as_f64).ok_or_else(|| {
                    invalid_response(&format!(
                        "Locator screenshot geometry result is missing finite `{name}`"
                    ))
                })
            };
            let geometry = LocatorScreenshotGeometry {
                x: field("x")?,
                y: field("y")?,
                width: field("width")?,
                height: field("height")?,
            };
            if [geometry.x, geometry.y, geometry.width, geometry.height]
                .into_iter()
                .any(|value| !value.is_finite())
                || geometry.x < 0.0
                || geometry.y < 0.0
                || geometry.width <= 0.0
                || geometry.height <= 0.0
            {
                return Err(invalid_response(
                    "Locator screenshot geometry is outside the valid viewport clip domain",
                ));
            }
            Ok(ScreenshotGeometryAttempt::Done(geometry))
        }
        "ready-to-scroll" => Ok(ScreenshotGeometryAttempt::ReadyToScroll),
        "missing" | "detached" | "notvisible" | "no-layout" | "unstable" | "retry" => {
            Ok(ScreenshotGeometryAttempt::Retry)
        }
        "runtime-missing" => Ok(ScreenshotGeometryAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => {
            let message = status_message(value, "Node is not an Element");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::InvalidTarget(message)),
            ))
        }
        "backend" => {
            let message = status_message(value, "Locator screenshot geometry failed");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::Backend(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "Locator screenshot geometry returned unknown status `{other}`"
        ))),
    }
}

fn parse_wait_for_attempt(
    response: Value,
    selector: &str,
) -> Result<WaitForAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator wait result is missing status"))?;
    match status {
        "done" => Ok(WaitForAttempt::Done),
        "retry" => Ok(WaitForAttempt::Retry),
        "runtime-missing" => Ok(WaitForAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator wait returned unknown status `{other}`"
        ))),
    }
}

fn parse_text_content_attempt(
    mut response: Value,
    selector: &str,
) -> Result<TextContentAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get_mut("result")
        .and_then(|result| result.get_mut("value"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator text-content result is missing status"))?
        .to_string();
    match status.as_str() {
        "done" => match value.remove("value") {
            Some(Value::Null) => Ok(TextContentAttempt::Done(None)),
            Some(Value::String(mut text)) => {
                if text.len() > MAX_TEXT_CONTENT_BYTES {
                    return Err(text_content_resource_error(
                        "text content exceeds the 1048576-byte backend limit",
                    ));
                }
                text.shrink_to_fit();
                Ok(TextContentAttempt::Done(Some(text)))
            }
            Some(_) => Err(invalid_response(
                "locator text-content result value is neither string nor null",
            )),
            None => Err(invalid_response(
                "locator text-content result is missing value",
            )),
        },
        "retry" => Ok(TextContentAttempt::Retry),
        "runtime-missing" => Ok(TextContentAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator text-content returned unknown status `{other}`"
        ))),
    }
}

fn parse_get_attribute_attempt(
    mut response: Value,
    selector: &str,
) -> Result<GetAttributeAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get_mut("result")
        .and_then(|result| result.get_mut("value"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator get-attribute result is missing status"))?
        .to_string();
    match status.as_str() {
        "done" => match value.remove("value") {
            Some(Value::Null) => Ok(GetAttributeAttempt::Done(None)),
            Some(Value::String(mut attribute)) => {
                if attribute.len() > MAX_ATTRIBUTE_RESULT_BYTES {
                    return Err(attribute_result_resource_error(
                        "attribute value exceeds the 1048576-byte backend limit",
                    ));
                }
                attribute.shrink_to_fit();
                Ok(GetAttributeAttempt::Done(Some(attribute)))
            }
            Some(_) => Err(invalid_response(
                "locator get-attribute result value is neither string nor null",
            )),
            None => Err(invalid_response(
                "locator get-attribute result is missing value",
            )),
        },
        "retry" => Ok(GetAttributeAttempt::Retry),
        "runtime-missing" => Ok(GetAttributeAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "backend" => {
            let message = status_message(value, "getAttribute failed");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::Backend(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator get-attribute returned unknown status `{other}`"
        ))),
    }
}

fn parse_inner_html_attempt(
    mut response: Value,
    selector: &str,
) -> Result<InnerHtmlAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get_mut("result")
        .and_then(|result| result.get_mut("value"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator inner-html result is missing status"))?
        .to_string();
    match status.as_str() {
        "done" => match value.remove("value") {
            Some(Value::String(mut html)) => {
                if html.len() > MAX_INNER_HTML_BYTES {
                    return Err(inner_html_resource_error(
                        "inner HTML exceeds the 1048576-byte backend limit",
                    ));
                }
                html.shrink_to_fit();
                Ok(InnerHtmlAttempt::Done(html))
            }
            Some(_) => Err(invalid_response(
                "locator inner-html result value is not a string",
            )),
            None => Err(invalid_response(
                "locator inner-html result is missing value",
            )),
        },
        "retry" => Ok(InnerHtmlAttempt::Retry),
        "runtime-missing" => Ok(InnerHtmlAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator inner-html returned unknown status `{other}`"
        ))),
    }
}

fn parse_inner_text_attempt(
    mut response: Value,
    selector: &str,
) -> Result<InnerTextAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get_mut("result")
        .and_then(|result| result.get_mut("value"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator inner-text result is missing status"))?
        .to_string();
    match status.as_str() {
        "done" => match value.remove("value") {
            Some(Value::String(mut text)) => {
                if text.len() > MAX_INNER_TEXT_BYTES {
                    return Err(inner_text_resource_error(
                        "inner text exceeds the 1048576-byte backend limit",
                    ));
                }
                text.shrink_to_fit();
                Ok(InnerTextAttempt::Done(text))
            }
            Some(_) => Err(invalid_response(
                "locator inner-text result value is not a string",
            )),
            None => Err(invalid_response(
                "locator inner-text result is missing value",
            )),
        },
        "retry" => Ok(InnerTextAttempt::Retry),
        "runtime-missing" => Ok(InnerTextAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => {
            let message = status_message(value, "Node is not an HTMLElement");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::InvalidTarget(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator inner-text returned unknown status `{other}`"
        ))),
    }
}

fn parse_input_value_attempt(
    mut response: Value,
    selector: &str,
) -> Result<InputValueAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get_mut("result")
        .and_then(|result| result.get_mut("value"))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator input-value result is missing status"))?
        .to_string();
    match status.as_str() {
        "done" => match value.remove("value") {
            Some(Value::String(mut input_value)) => {
                if input_value.len() > MAX_INPUT_VALUE_BYTES {
                    return Err(input_value_resource_error(
                        "input value exceeds the 1048576-byte backend limit",
                    ));
                }
                input_value.shrink_to_fit();
                Ok(InputValueAttempt::Done(input_value))
            }
            Some(_) => Err(invalid_response(
                "locator input-value result value is not a string",
            )),
            None => Err(invalid_response(
                "locator input-value result is missing value",
            )),
        },
        "retry" => Ok(InputValueAttempt::Retry),
        "runtime-missing" => Ok(InputValueAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => {
            let message = status_message(
                value,
                "Node is not an <input>, <textarea> or <select> element",
            );
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::InvalidTarget(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator input-value returned unknown status `{other}`"
        ))),
    }
}

fn parse_focus_operation_attempt(
    response: Value,
    selector: &str,
    operation: FocusOperation,
) -> Result<FocusOperationAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value.get("status").and_then(Value::as_str).ok_or_else(|| {
        invalid_response(&format!(
            "locator {} result is missing status",
            operation.name()
        ))
    })?;
    match status {
        "done" => Ok(FocusOperationAttempt::Done),
        "retry" => Ok(FocusOperationAttempt::Retry),
        "runtime-missing" => Ok(FocusOperationAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => {
            let message = status_message(value, "Node is not an element");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::InvalidTarget(message)),
            ))
        }
        "backend" => {
            let message = status_message(value, "native focus operation failed");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::Backend(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator {} returned unknown status `{other}`",
            operation.name()
        ))),
    }
}

fn parse_press_focus_attempt(
    response: Value,
    selector: &str,
) -> Result<PressFocusAttempt, OperationError<LocatorErrorDetails>> {
    let value = response
        .get("result")
        .and_then(|result| result.get("value"))
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_response("Runtime.evaluate is missing object result.value"))?;
    let status = value
        .get("status")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_response("locator press focus result is missing status"))?;
    match status {
        "done" => {
            let is_mac = value
                .get("isMac")
                .and_then(Value::as_bool)
                .ok_or_else(|| invalid_response("locator press focus result is missing isMac"))?;
            Ok(PressFocusAttempt::Done { is_mac })
        }
        "retry" => Ok(PressFocusAttempt::Retry),
        "runtime-missing" => Ok(PressFocusAttempt::RuntimeMissing),
        "strict" => {
            let count = value
                .get("count")
                .and_then(Value::as_u64)
                .ok_or_else(|| invalid_response("strict result is missing match count"))?;
            let count = u32::try_from(count).map_err(|_| {
                OperationError::new(
                    FailureKind::ResourceExhausted,
                    "selector match count exceeds the portable limit",
                    Some(LocatorErrorDetails::StrictViolation {
                        selector: selector.to_string(),
                        matches: u32::MAX,
                    }),
                )
            })?;
            Err(OperationError::new(
                FailureKind::Backend,
                format!("strict selector `{selector}` resolved to {count} elements"),
                Some(LocatorErrorDetails::StrictViolation {
                    selector: selector.to_string(),
                    matches: count,
                }),
            ))
        }
        "invalid" => {
            let message = status_message(value, "invalid selector");
            Err(OperationError::new(
                FailureKind::Backend,
                format!("invalid selector `{selector}`: {message}"),
                Some(LocatorErrorDetails::InvalidSelector(message)),
            ))
        }
        "invalid-target" => {
            let message = status_message(value, "Node is not an element");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::InvalidTarget(message)),
            ))
        }
        "backend" => {
            let message = status_message(value, "native focus operation failed");
            Err(OperationError::new(
                FailureKind::Backend,
                message.clone(),
                Some(LocatorErrorDetails::Backend(message)),
            ))
        }
        other => Err(invalid_response(&format!(
            "locator press focus returned unknown status `{other}`"
        ))),
    }
}

fn status_message(value: &serde_json::Map<String, Value>, fallback: &str) -> String {
    value
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn coordinate(
    value: &serde_json::Map<String, Value>,
    field: &'static str,
) -> Result<f64, OperationError<LocatorErrorDetails>> {
    let coordinate = value
        .get(field)
        .and_then(Value::as_f64)
        .filter(|value| value.is_finite())
        .ok_or_else(|| invalid_response("locator actionability returned an invalid coordinate"))?;
    Ok(coordinate)
}

fn invalid_response(message: &str) -> OperationError<LocatorErrorDetails> {
    OperationError::new(
        FailureKind::ProtocolViolation,
        message,
        Some(LocatorErrorDetails::InvalidBackendResponse(
            message.to_string(),
        )),
    )
}

fn map_button(button: PortableMouseButton) -> MouseButton {
    match button {
        PortableMouseButton::Left => MouseButton::Left,
        PortableMouseButton::Right => MouseButton::Right,
        PortableMouseButton::Middle => MouseButton::Middle,
    }
}

fn resolve_modifiers(modifiers: KeyboardModifiers, is_mac: bool) -> CdpModifiers {
    let mut resolved = CdpModifiers::NONE;
    for modifier in modifiers.iter() {
        resolved = resolved.with(match modifier {
            KeyboardModifier::Alt => CdpModifiers::ALT,
            KeyboardModifier::Control => CdpModifiers::CONTROL,
            KeyboardModifier::ControlOrMeta if is_mac => CdpModifiers::META,
            KeyboardModifier::ControlOrMeta => CdpModifiers::CONTROL,
            KeyboardModifier::Meta => CdpModifiers::META,
            KeyboardModifier::Shift => CdpModifiers::SHIFT,
        });
    }
    resolved
}

fn map_cdp_error(error: CdpError) -> OperationError<LocatorErrorDetails> {
    let kind = classify_cdp_failure(&error);
    let message = error.to_string();
    OperationError::new(
        kind,
        message.clone(),
        Some(LocatorErrorDetails::Backend(message)),
    )
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU8, NonZeroU32};
    use std::sync::Arc;
    #[cfg(feature = "retained-element-click")]
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use pwright_backend_cdp_input::resolve_key;
    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_cap_locator::{
        AllInnerTexts, AllInnerTextsParams, AllTextContentsParams, AriaRole, BlurParams, Click,
        ClickOptions, ClickParams, CountParams, CssSelector, FillOptions, FillParams, FocusParams,
        GetAttribute, GetAttributeParams, HoverOptions, HoverParams, InnerHtml, InnerHtmlParams,
        InnerText, InnerTextParams, InputValue, InputValueParams, IsCheckedParams,
        IsEditableParams, IsEnabled, IsEnabledParams, IsVisibleParams, KeyboardModifier,
        LocatorErrorDetails, MouseButton, Position, Press, PressKey, PressOptions, PressParams,
        RegexMatcher, RoleOptions, ScrollIntoViewIfNeeded, ScrollIntoViewIfNeededParams,
        ScrollMode, SelectOption, SelectOptionOptions, SelectOptionParams, SelectOptionValue,
        Selector, SetChecked, SetCheckedOptions, SetCheckedParams, TestIdAttributes, TextContent,
        TextContentParams, UserFacingAttribute, WaitFor, WaitForOptions, WaitForParams,
        WaitForState,
    };
    use pwright_cap_locator::{ClickOptionsV2, ClickParamsV2};
    #[cfg(feature = "touchscreen-input")]
    use pwright_cap_touchscreen::{KeyboardModifiers, SelectorTapOptions, SelectorTapParams};
    use pwright_cdp::CdpEvent;
    use pwright_core::{
        CallId, CallMetadata, CancellationReason, Command, CommandCancellation, CommandSpec,
        FailureKind, ObjectId, ObjectKindTag, OperationContext, ProductionResourceLedger,
        ResourceClass, ResourceLimits, ResourceRequirements, ResourceScope, Service,
    };
    use pwright_fake::FakeCdpClient;
    use pwright_runtime::{ControlledDispatcher, RuntimeLimits};
    use pwright_runtime_tokio::TokioDeadlineScheduler;
    use serde_json::json;

    use super::{
        CdpLocatorProductionProfile, CdpLocatorService, CdpLocatorServiceFactory,
        LocatorElementState, MAX_FOCUS_SCRIPT_BYTES, MAX_SELECTOR_STRING_BYTES,
        actionability_script, click_v2_actionability_script, element_state_script,
        fill_actionability_script, parse_pointer_attempt, screenshot_geometry_check_script,
        scroll_into_view_if_needed_check_script, scroll_into_view_if_needed_commit_script,
        select_option_script, wait_for_script,
    };

    const LOCATOR_SCOPE: ResourceScope = ResourceScope::Object(ObjectId::new(42));

    fn selector(value: &str) -> CssSelector {
        CssSelector::new(value).expect("test selector should be valid")
    }

    fn runtime_installed() -> serde_json::Value {
        json!({"result": {"value": true}})
    }

    #[cfg(feature = "retained-element-click")]
    fn retained_click_service(
        client: Arc<FakeCdpClient>,
        retry_delay: Duration,
    ) -> CdpLocatorService {
        CdpLocatorService::from_parts_with_counters(
            client,
            super::CdpInputCoordinator::with_policy(super::CdpInputPolicy::Interleaved),
            None,
            retry_delay,
            None,
            Arc::new(super::SelectorRuntimeCounters::default()),
        )
    }

    #[cfg(feature = "retained-element-click")]
    fn one_link_frame_tree() -> pwright_cdp::generated::page::FrameTree {
        serde_json::from_value(json!({
            "frame": {
                "id": "main-frame", "loaderId": "main-loader", "url": "about:blank",
                "domainAndRegistry": "", "securityOrigin": "://", "mimeType": "text/html",
                "secureContextType": "InsecureScheme",
                "crossOriginIsolatedContextType": "NotIsolated", "gatedAPIFeatures": []
            },
            "childFrames": [{"frame": {
                "id": "child-frame", "parentId": "main-frame", "loaderId": "child-loader",
                "url": "about:blank", "domainAndRegistry": "", "securityOrigin": "://",
                "mimeType": "text/html", "secureContextType": "InsecureScheme",
                "crossOriginIsolatedContextType": "NotIsolated", "gatedAPIFeatures": []
            }}]
        }))
        .expect("one-link Frame tree must satisfy CDP schema")
    }

    #[cfg(feature = "retained-element-click")]
    fn child_ready_response() -> serde_json::Value {
        json!({"result": {"value": {
            "status": "ready", "x": 20.0, "y": 30.0, "isMac": false,
            "viewportWidth": 200.0, "viewportHeight": 150.0
        }}})
    }

    fn production_resources(
        profile: CdpLocatorProductionProfile,
        extra_leases: usize,
    ) -> ProductionResourceLedger {
        let requirements = profile.requirements();
        let selector = requirements.class(ResourceClass::SelectorBytes);
        let backend = requirements.class(ResourceClass::BackendStateBytes);
        let limits = ResourceLimits::deny_all(
            requirements.scopes,
            requirements.leases.saturating_add(extra_leases),
        )
        .with_class(
            ResourceClass::SelectorBytes,
            selector.global,
            selector.per_scope,
        )
        .with_class(
            ResourceClass::BackendStateBytes,
            backend.global,
            backend.per_scope,
        );
        ProductionResourceLedger::new(limits, requirements)
            .expect("locator production requirements should fit")
    }

    #[tokio::test]
    async fn click_waits_for_actionability_then_dispatches_the_requested_mouse_pair() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Submit</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "missing"}}}),
            json!({"result": {"value": {"status": "unstable"}}}),
            json!({"result": {"value": {"status": "ready", "x": 12.5, "y": 19.0}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);
        let options = ClickOptions::new()
            .with_button(MouseButton::Right)
            .with_click_count(NonZeroU8::new(2).expect("two is nonzero"));

        // Act
        service
            .call(ClickParams::new(selector("button"), options))
            .await
            .expect("actionable click should succeed");

        // Assert
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
            ]
        );
        let calls = client.calls();
        assert_eq!(calls[4].1[0]["type"], "mouseMoved");
        assert_eq!(calls[4].1[0]["x"], 12.5);
        assert_eq!(calls[4].1[0]["y"], 19.0);
        assert_eq!(calls[4].1[0]["buttons"], 0);
        assert_eq!(calls[5].1[0]["button"], "right");
        assert_eq!(calls[5].1[0]["clickCount"], 2);
        assert_eq!(calls[5].1[0]["buttons"], 2);
        assert_eq!(calls[6].1[0]["buttons"], 0);
    }

    #[tokio::test]
    async fn trial_runs_actionability_without_dispatching_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Submit</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "ready", "x": 1.0, "y": 2.0}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        service
            .call(ClickParams::new(
                selector("button"),
                ClickOptions::new().with_trial(true),
            ))
            .await
            .expect("trial should succeed");

        // Assert
        assert_eq!(
            client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
    }

    #[cfg(feature = "touchscreen-input")]
    #[tokio::test]
    async fn selector_tap_dispatches_touch_pair_with_temporary_modifiers() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Submit</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "ready", "x": 12.5, "y": 19.0, "isMac": false
            }}}),
        ]);
        let service = CdpLocatorService::new(client.clone());
        let options = SelectorTapOptions::new()
            .with_modifiers(KeyboardModifiers::new().with(KeyboardModifier::Alt));

        // Act
        service
            .selector_tap(
                SelectorTapParams::new(Selector::Css(selector("button")), options),
                None,
            )
            .await
            .expect("actionable tap should succeed");

        // Assert
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Input.dispatchKeyEvent",
                "Input.dispatchTouchEvent",
                "Input.dispatchTouchEvent",
                "Input.dispatchKeyEvent",
            ]
        );
        let calls = client.calls();
        assert_eq!(calls[2].1[0]["key"], "Alt");
        assert_eq!(calls[3].1[0]["type"], "touchStart");
        assert_eq!(calls[3].1[0]["touchPoints"][0]["x"], 12.5);
        assert_eq!(calls[3].1[0]["modifiers"], 1);
        assert_eq!(calls[4].1[0]["type"], "touchEnd");
        assert_eq!(calls[5].1[0]["type"], "keyUp");
    }

    #[cfg(feature = "touchscreen-input")]
    #[tokio::test]
    async fn selector_tap_trial_blocks_dom_commit_while_still_dispatching_touch() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Submit</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "ready", "x": 1.0, "y": 2.0, "isMac": false
            }}}),
            json!({"result": {"value": true}}),
            json!({"result": {"value": true}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        service
            .selector_tap(
                SelectorTapParams::new(
                    Selector::Css(selector("button")),
                    SelectorTapOptions::new().with_trial(true),
                ),
                None,
            )
            .await
            .expect("trial tap should finish");

        // Assert
        let calls = client.calls();
        assert_eq!(
            calls
                .iter()
                .filter(|(method, _)| method == "Input.dispatchTouchEvent")
                .count(),
            2
        );
        let methods = client.method_names();
        assert_eq!(methods.last().map(String::as_str), Some("Runtime.evaluate"));
        let install = calls[2].1[0]
            .as_str()
            .expect("trial installation should be JavaScript");
        let cleanup = calls[5].1[0]
            .as_str()
            .expect("trial cleanup should be JavaScript");
        assert!(install.contains("touchstart"));
        assert!(cleanup.contains("stop"));
    }

    #[tokio::test]
    async fn click_mouse_up_failure_is_retried_for_cleanup_before_error_returns() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Submit</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "ready", "x": 1.0, "y": 2.0}}}),
        ]);
        client.set_input_errors([None, None, Some("mouse up failed".to_string()), None]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let error = service
            .call(ClickParams::new(selector("button"), ClickOptions::new()))
            .await
            .expect_err("the original mouse-up failure should fail click");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(error.message().contains("mouse up failed"));
        let input_calls = client
            .calls()
            .into_iter()
            .filter(|(method, _)| method.starts_with("Input."))
            .collect::<Vec<_>>();
        assert_eq!(input_calls.len(), 4);
        assert_eq!(input_calls[0].1[0]["type"], "mouseMoved");
        assert_eq!(input_calls[1].1[0]["type"], "mousePressed");
        assert_eq!(input_calls[2].1[0]["type"], "mouseReleased");
        assert_eq!(input_calls[3].1[0]["type"], "mouseReleased");
    }

    #[tokio::test]
    async fn click_cancellation_releases_a_button_from_a_dropped_transaction() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Submit</button>"));
        let service = CdpLocatorService::new(client.clone());
        let mut transaction = service
            .input
            .begin_transaction(client.clone(), None)
            .await
            .expect("input transaction should start");
        transaction
            .move_mouse(3.0, 4.0)
            .await
            .expect("pointer should move");
        transaction
            .mouse_down(pwright_cdp::MouseButton::Left, 1)
            .await
            .expect("button should press");
        drop(transaction);
        let context = OperationContext::new(CallId::new(47), ObjectId::new(3), Click::METHOD, None);

        // Act
        let result = <CdpLocatorService as CommandCancellation<Click>>::cancel(
            &service,
            context,
            CancellationReason::Caller,
        )
        .await;

        // Assert
        assert_eq!(result, Ok(()));
        let calls = client.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].1[0]["type"], "mouseMoved");
        assert_eq!(calls[1].1[0]["type"], "mousePressed");
        assert_eq!(calls[2].1[0]["type"], "mouseReleased");
        assert_eq!(calls[2].1[0]["buttons"], 0);
    }

    #[tokio::test(start_paused = true)]
    async fn locator_v2_click_applies_every_pointer_option_with_bounded_steps() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Submit</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "ready", "x": 9.0, "y": 6.0, "isMac": true
            }}}),
        ]);
        let service = CdpLocatorService::new(client.clone());
        let options = ClickOptionsV2::new()
            .with_button(MouseButton::Right)
            .with_click_count(NonZeroU8::new(2).expect("two is nonzero"))
            .with_delay(Duration::from_millis(5))
            .with_force(true)
            .with_modifier(KeyboardModifier::ControlOrMeta)
            .with_no_wait_after(true)
            .with_position(Position::new(2.5, 3.25).expect("finite position should build"))
            .with_scroll(ScrollMode::None)
            .with_steps(NonZeroU32::new(3).expect("three is nonzero"));

        // Act
        service
            .call(ClickParamsV2::new(
                Selector::Css(selector("button")),
                options,
            ))
            .await
            .expect("v2 click should succeed");

        // Assert
        let calls = client.calls();
        assert_eq!(calls.len(), 11);
        assert_eq!(calls[2].1[0]["key"], "Meta");
        assert_eq!(calls[3].1[0]["x"], 3.0);
        assert_eq!(calls[4].1[0]["x"], 6.0);
        assert_eq!(calls[5].1[0]["x"], 9.0);
        assert_eq!(calls[6].1[0]["type"], "mousePressed");
        assert_eq!(calls[6].1[0]["button"], "right");
        assert_eq!(calls[6].1[0]["clickCount"], 1);
        assert_eq!(calls[8].1[0]["clickCount"], 2);
        assert_eq!(calls[10].1[0]["type"], "keyUp");
        let script = calls[1].1[0]
            .as_str()
            .expect("evaluation should carry script");
        assert!(script.contains("const position = { x: 2.5, y: 3.25 }"));
        assert!(script.contains("if (false) {\n    if (position)"));
        assert!(script.contains("if (!true"));
        assert!(!script.contains("blockerTag"));
    }

    #[cfg(feature = "retained-element-click")]
    #[test]
    fn retained_element_click_scripts_do_not_read_mutable_page_node_global() {
        // Arrange
        let point = super::ClickPoint {
            x: 10.0,
            y: 20.0,
            is_mac: false,
        };
        let options = ClickOptionsV2::new();

        // Act
        let scripts = [
            super::retained_click_actionability_script(&options, false),
            super::retained_click_actionability_script(&options, true),
            super::retained_click_route_check_script(point, false, false),
            super::retained_click_route_check_script(point, true, true),
        ];

        // Assert
        for script in scripts {
            assert!(script.contains("const ELEMENT_NODE = 1;"));
            assert!(script.contains("node.nodeType === ELEMENT_NODE ? node : node.parentElement"));
            assert!(script.contains("!node.isConnected || !element.isConnected"));
            assert!(script.contains("element.nodeType !== ELEMENT_NODE"));
            assert!(script.contains("element.ownerDocument?.defaultView !== globalThis"));
            assert!(!script.contains("Node."));
        }
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_text_node_click_retargets_each_phase_without_requerying() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Text</button>"));
        client.set_evaluate_responses([
            json!({"result": {"value": {
                "status": "ready", "x": 13.5, "y": 21.25, "isMac": false
            }}}),
            json!({"result": {"value": {"status": "ready"}}}),
            json!({"result": {"value": {"status": "ready"}}}),
        ]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();

        // Act
        service
            .click_retained_top_level_element(
                "text-node-7",
                "world-3",
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                || Ok(()),
            )
            .await
            .expect("the exact retained Text node should click through its parent Element");

        // Assert
        let evaluations: Vec<_> = client
            .calls()
            .into_iter()
            .filter(|call| call.0 == "Runtime.callFunctionOn")
            .collect();
        assert_eq!(evaluations.len(), 3);
        for (index, call) in evaluations.iter().enumerate() {
            assert_eq!(call.1[0]["uniqueContextId"], "world-3");
            assert_eq!(call.1[0]["arguments"][0]["objectId"], "text-node-7");
            let script = call.1[0]["functionDeclaration"].as_str().unwrap();
            assert!(script.contains("node.nodeType === ELEMENT_NODE ? node : node.parentElement"));
            assert!(script.contains("!node.isConnected || !element.isConnected"));
            assert!(!script.contains("const element = matches[0]"));
            assert!(script.starts_with(if index == 0 {
                "async function(node)"
            } else {
                "function(node)"
            }));
        }
        assert!(client.calls().iter().any(|call| {
            call.0 == "Input.dispatchMouseEvent" && call.1[0]["type"] == "mousePressed"
        }));
    }

    #[cfg(feature = "retained-element-click")]
    #[test]
    fn retained_obscured_response_records_only_sanitized_blocker_preview() {
        // Arrange
        let diagnostics = super::RetainedClickDiagnostics::default();
        let response = json!({"result": {"value": {
            "status": "obscured", "blockerTag": "div", "blockerId": "blocker"
        }}});

        // Act
        let attempt = super::parse_retained_click_attempt(response, false, &diagnostics);
        let log = diagnostics.timeout_log().unwrap();

        // Assert
        assert!(matches!(
            attempt,
            Ok(super::ActionAttempt::Retry(
                super::ActionabilityProgress::Obscured
            ))
        ));
        assert!(log.contains("<div id=\"blocker\"></div> intercepts pointer events"));
        assert!(log.contains("retrying click action"));
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_top_level_click_uses_the_exact_object_and_shared_pointer_transaction() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>First</button>"));
        client.set_evaluate_responses([
            json!({"result": {"value": {
                "status": "ready", "x": 13.5, "y": 21.25, "isMac": false
            }}}),
            json!({"result": {"value": {"status": "ready"}}}),
            json!({"result": {"value": {"status": "ready"}}}),
        ]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();
        let options = ClickOptionsV2::new()
            .with_button(MouseButton::Right)
            .with_no_wait_after(true);

        // Act
        service
            .click_retained_top_level_element("remote-object-7", "world-3", options, &press, || {
                Ok(())
            })
            .await
            .expect("the exact top-level Element should click");

        // Assert
        assert_eq!(
            client.method_names(),
            [
                "Page.getFrameTree",
                "Runtime.callFunctionOn",
                "Runtime.callFunctionOn",
                "Input.dispatchMouseEvent",
                "Runtime.callFunctionOn",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
            ]
        );
        let calls = client.calls();
        for call in &calls[1..=2] {
            assert_eq!(call.1[0]["uniqueContextId"], "world-3");
            assert_eq!(call.1[0]["arguments"][0]["objectId"], "remote-object-7");
        }
        let script = calls[1].1[0]["functionDeclaration"]
            .as_str()
            .expect("actionability should use a function declaration");
        assert!(script.contains("async function(node)"));
        assert!(script.contains("globalThis !== globalThis.top"));
        assert!(script.contains("blockerTag"));
        assert!(!script.contains("const element = matches[0]"));
        let route_script = calls[2].1[0]["functionDeclaration"]
            .as_str()
            .expect("route check should use a function declaration");
        assert!(route_script.contains("const x = 13.5"));
        assert!(route_script.contains("element.getClientRects()"));
        assert_eq!(calls[3].1[0]["x"], 13.5);
        assert_eq!(calls[3].1[0]["y"], 21.25);
        assert_eq!(calls[4].1[0]["arguments"][0]["objectId"], "remote-object-7");
        assert_eq!(calls[5].1[0]["button"], "right");
        assert_eq!(calls[6].1[0]["type"], "mouseReleased");
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_top_level_trial_checks_exact_element_without_pointer_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>First</button>"));
        client.set_evaluate_responses([json!({"result": {"value": {
            "status": "ready", "x": 13.5, "y": 21.25, "isMac": false
        }}})]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();

        // Act
        service
            .click_retained_top_level_element(
                "remote-object-7",
                "world-3",
                ClickOptionsV2::new().with_trial(true),
                &press,
                || Ok(()),
            )
            .await
            .expect("trial should finish after exact-element actionability");

        // Assert
        assert_eq!(
            client.method_names(),
            ["Page.getFrameTree", "Runtime.callFunctionOn"]
        );
        assert_eq!(
            client.calls()[1].1[0]["arguments"][0]["objectId"],
            "remote-object-7"
        );
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn detached_retained_element_fails_without_requery_or_pointer_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>First</button>"));
        client.set_evaluate_responses([json!({"result": {"value": {"status": "detached"}}})]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();

        // Act
        let error = service
            .click_retained_top_level_element(
                "removed-object",
                "world-3",
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                || Ok(()),
            )
            .await
            .expect_err("detached retained Element must fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(error.message(), "Element is not attached to the DOM");
        assert_eq!(
            client.method_names(),
            ["Page.getFrameTree", "Runtime.callFunctionOn"]
        );
        assert_eq!(
            client.calls()[1].1[0]["arguments"][0]["objectId"],
            "removed-object"
        );
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn child_frame_target_is_rejected_before_retained_evaluation_or_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Nested</button>"));
        let tree = serde_json::from_value(json!({
            "frame": {
                "id": "child-frame",
                "parentId": "main-frame",
                "loaderId": "fake-loader",
                "url": "about:blank",
                "domainAndRegistry": "",
                "securityOrigin": "://",
                "mimeType": "text/html",
                "secureContextType": "InsecureScheme",
                "crossOriginIsolatedContextType": "NotIsolated",
                "gatedAPIFeatures": []
            }
        }))
        .expect("child Frame tree should match CDP schema");
        client.set_frame_tree(tree);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();

        // Act
        let error = service
            .click_retained_top_level_element(
                "nested-object",
                "world-3",
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                || Ok(()),
            )
            .await
            .expect_err("child Frame coordinates must not be used as Page coordinates");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(client.method_names(), ["Page.getFrameTree"]);
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_child_click_translates_content_quad_to_page_css_coordinates() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<iframe><button>Nested</button></iframe>",
        ));
        client.set_frame_tree(one_link_frame_tree());
        client.set_frame_owner("child-frame", 91);
        client.set_backend_box_model(
            91,
            json!({"model": {
                "content": [108.0, 106.0, 308.0, 106.0, 308.0, 256.0, 108.0, 256.0],
                "border": [100.0, 98.0, 316.0, 98.0, 316.0, 264.0, 100.0, 264.0]
            }}),
        );
        client.set_evaluate_responses([
            child_ready_response(),
            json!({"result": {"value": {"status": "ready"}}}),
            json!({"result": {"value": {"status": "ready"}}}),
        ]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();
        let diagnostics = super::RetainedClickDiagnostics::default();

        // Act
        service
            .click_retained_one_link_child_element_with_diagnostics(
                super::RetainedClickChildTarget::new(
                    "nested-object",
                    "world-3",
                    "child-frame",
                    "main-frame",
                ),
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                &diagnostics,
                |_, _| async { Ok(true) },
                || Ok(()),
            )
            .await
            .expect("one same-session child click should use Page CSS coordinates");

        // Assert
        let calls = client.calls();
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.0 == "DOM.getFrameOwner")
                .count(),
            2
        );
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.0 == "DOM.getBoxModel")
                .count(),
            2
        );
        let move_call = calls
            .iter()
            .find(|call| call.0 == "Input.dispatchMouseEvent" && call.1[0]["type"] == "mouseMoved")
            .unwrap();
        assert_eq!(move_call.1[0]["x"], 128.0);
        assert_eq!(move_call.1[0]["y"], 136.0);
        assert!(calls.iter().any(
            |call| call.0 == "Input.dispatchMouseEvent" && call.1[0]["type"] == "mousePressed"
        ));
        assert!(
            calls
                .iter()
                .filter(|call| call.0 == "Runtime.callFunctionOn")
                .all(|call| {
                    call.1[0]["arguments"].as_array().is_none_or(|args| {
                        args.is_empty() || args[0]["objectId"] == "nested-object"
                    })
                })
        );
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_child_click_rejects_malformed_content_quad_without_mouse_down() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<iframe></iframe>"));
        client.set_frame_tree(one_link_frame_tree());
        client.set_frame_owner("child-frame", 91);
        client.set_backend_box_model(
            91,
            json!({"model": {
                "content": [108.0, 106.0, 308.0, 106.0, 400.0, 256.0, 108.0, 256.0]
            }}),
        );
        client.set_evaluate_responses([child_ready_response()]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();
        let diagnostics = super::RetainedClickDiagnostics::default();

        // Act
        let error = service
            .click_retained_one_link_child_element_with_diagnostics(
                super::RetainedClickChildTarget::new(
                    "nested-object",
                    "world-3",
                    "child-frame",
                    "main-frame",
                ),
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                &diagnostics,
                |_, _| async { Ok(true) },
                || Ok(()),
            )
            .await
            .expect_err("non-affine iframe geometry must fail closed");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(
            !client
                .method_names()
                .contains(&"Input.dispatchMouseEvent".to_owned())
        );
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_child_click_retries_transient_parent_overlay_before_mouse_down() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<iframe></iframe>"));
        client.set_frame_tree(one_link_frame_tree());
        client.set_frame_owner("child-frame", 91);
        client.set_backend_box_model(
            91,
            json!({"model": {
                "content": [108.0, 106.0, 308.0, 106.0, 308.0, 256.0, 108.0, 256.0]
            }}),
        );
        client.set_evaluate_responses([
            child_ready_response(),
            child_ready_response(),
            json!({"result": {"value": {"status": "ready"}}}),
            json!({"result": {"value": {"status": "ready"}}}),
        ]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();
        let diagnostics = super::RetainedClickDiagnostics::default();
        let attempts = Arc::new(AtomicUsize::new(0));
        let hit_client = client.clone();
        let hit_attempts = attempts.clone();

        // Act
        service
            .click_retained_one_link_child_element_with_diagnostics(
                super::RetainedClickChildTarget::new(
                    "nested-object",
                    "world-3",
                    "child-frame",
                    "main-frame",
                ),
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                &diagnostics,
                move |_, _| {
                    let attempt = hit_attempts.fetch_add(1, Ordering::Relaxed);
                    if attempt == 0 {
                        assert!(
                            !hit_client
                                .method_names()
                                .contains(&"Input.dispatchMouseEvent".to_owned())
                        );
                    }
                    async move { Ok(attempt != 0) }
                },
                || Ok(()),
            )
            .await
            .expect("a transient parent overlay should retry the same element");

        // Assert
        assert_eq!(attempts.load(Ordering::Relaxed), 3);
        assert_eq!(
            client
                .calls()
                .iter()
                .filter(|call| call.0 == "Input.dispatchMouseEvent"
                    && call.1[0]["type"] == "mousePressed")
                .count(),
            1
        );
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_child_force_click_skips_parent_hit_without_skipping_frame_route() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<iframe></iframe>"));
        client.set_frame_tree(one_link_frame_tree());
        client.set_frame_owner("child-frame", 91);
        client.set_backend_box_model(
            91,
            json!({"model": {
                "content": [108.0, 106.0, 308.0, 106.0, 308.0, 256.0, 108.0, 256.0]
            }}),
        );
        client.set_evaluate_responses([
            child_ready_response(),
            json!({"result": {"value": {"status": "ready"}}}),
            json!({"result": {"value": {"status": "ready"}}}),
        ]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();
        let diagnostics = super::RetainedClickDiagnostics::default();
        let hit_calls = Arc::new(AtomicUsize::new(0));
        let hit_calls_for_proof = hit_calls.clone();

        // Act
        service
            .click_retained_one_link_child_element_with_diagnostics(
                super::RetainedClickChildTarget::new(
                    "nested-object",
                    "world-3",
                    "child-frame",
                    "main-frame",
                ),
                ClickOptionsV2::new()
                    .with_force(true)
                    .with_no_wait_after(true),
                &press,
                &diagnostics,
                move |_, _| {
                    hit_calls_for_proof.fetch_add(1, Ordering::Relaxed);
                    async { Ok(false) }
                },
                || Ok(()),
            )
            .await
            .expect("force should skip hit proof, not native route and geometry");

        // Assert
        let methods = client.method_names();
        assert_eq!(hit_calls.load(Ordering::Relaxed), 0);
        assert!(methods.contains(&"Page.getFrameTree".to_owned()));
        assert!(methods.contains(&"DOM.getBoxModel".to_owned()));
        assert!(client.calls().iter().any(|call| {
            call.0 == "Input.dispatchMouseEvent" && call.1[0]["type"] == "mousePressed"
        }));
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_child_click_rejects_retired_parent_edge_without_mouse_down() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<iframe></iframe>"));
        client.set_frame_trees([
            one_link_frame_tree(),
            serde_json::from_value(json!({"frame": {
                "id": "main-frame", "loaderId": "main-loader", "url": "about:blank",
                "domainAndRegistry": "", "securityOrigin": "://", "mimeType": "text/html",
                "secureContextType": "InsecureScheme",
                "crossOriginIsolatedContextType": "NotIsolated", "gatedAPIFeatures": []
            }}))
            .unwrap(),
        ]);
        client.set_evaluate_responses([child_ready_response()]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();
        let diagnostics = super::RetainedClickDiagnostics::default();

        // Act
        let error = service
            .click_retained_one_link_child_element_with_diagnostics(
                super::RetainedClickChildTarget::new(
                    "nested-object",
                    "world-3",
                    "child-frame",
                    "main-frame",
                ),
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                &diagnostics,
                |_, _| async { Ok(true) },
                || Ok(()),
            )
            .await
            .expect_err("retired parent edge must fail before geometry or input");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(
            !client
                .method_names()
                .contains(&"Input.dispatchMouseEvent".to_owned())
        );
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_child_click_rejects_nested_route_before_parent_proof_or_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<iframe></iframe>"));
        client.set_frame_tree(one_link_frame_tree());
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();
        let diagnostics = super::RetainedClickDiagnostics::default();

        // Act
        let error = service
            .click_retained_one_link_child_element_with_diagnostics(
                super::RetainedClickChildTarget::new(
                    "nested-object",
                    "world-3",
                    "grandchild-frame",
                    "child-frame",
                ),
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                &diagnostics,
                |_, _| async { Ok(true) },
                || Ok(()),
            )
            .await
            .expect_err("a grandchild must not inherit one-link coordinates");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(client.method_names(), ["Page.getFrameTree"]);
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_child_click_rejects_oopif_root_before_parent_proof_or_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<iframe></iframe>"));
        let remote_root = serde_json::from_value(json!({"frame": {
            "id": "child-frame", "parentId": "main-frame", "loaderId": "child-loader",
            "url": "about:blank", "domainAndRegistry": "", "securityOrigin": "://",
            "mimeType": "text/html", "secureContextType": "InsecureScheme",
            "crossOriginIsolatedContextType": "NotIsolated", "gatedAPIFeatures": []
        }}))
        .unwrap();
        client.set_frame_tree(remote_root);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();
        let diagnostics = super::RetainedClickDiagnostics::default();

        // Act
        let error = service
            .click_retained_one_link_child_element_with_diagnostics(
                super::RetainedClickChildTarget::new(
                    "nested-object",
                    "world-3",
                    "child-frame",
                    "main-frame",
                ),
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                &diagnostics,
                |_, _| async { Ok(true) },
                || Ok(()),
            )
            .await
            .expect_err("an OOPIF-local root must not dispatch Page input");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(client.method_names(), ["Page.getFrameTree"]);
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_route_change_after_actionability_prevents_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>First</button>"));
        client.set_evaluate_responses([
            json!({"result": {"value": {
                "status": "ready", "x": 13.5, "y": 21.25, "isMac": false
            }}}),
            json!({"result": {"value": {"status": "frame-mismatch"}}}),
        ]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();

        // Act
        let error = service
            .click_retained_top_level_element(
                "moved-object",
                "world-3",
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                || Ok(()),
            )
            .await
            .expect_err("moving the exact Element after actionability must fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::TargetClosed);
        assert_eq!(
            client.method_names(),
            [
                "Page.getFrameTree",
                "Runtime.callFunctionOn",
                "Runtime.callFunctionOn",
            ]
        );
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_world_retirement_after_mouse_move_prevents_mouse_down() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>First</button>"));
        client.set_evaluate_responses([
            json!({"result": {"value": {
                "status": "ready", "x": 13.5, "y": 21.25, "isMac": false
            }}}),
            json!({"result": {"value": {"status": "ready"}}}),
        ]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();
        let guard_client = client.clone();
        let validate_world = move || {
            if guard_client
                .method_names()
                .iter()
                .any(|method| method == "Input.dispatchMouseEvent")
            {
                return Err(super::OperationError::new(
                    FailureKind::TargetClosed,
                    "creation world retired after hover",
                    None,
                ));
            }
            Ok(())
        };

        // Act
        let error = service
            .click_retained_top_level_element(
                "object-7",
                "world-3",
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                validate_world,
            )
            .await
            .expect_err("a retired world cannot receive a pointer press");

        // Assert
        assert_eq!(error.kind(), FailureKind::TargetClosed);
        assert_eq!(
            client.method_names(),
            [
                "Page.getFrameTree",
                "Runtime.callFunctionOn",
                "Runtime.callFunctionOn",
                "Input.dispatchMouseEvent",
            ]
        );
        assert_eq!(client.calls()[3].1[0]["type"], "mouseMoved");
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn retained_route_recheck_follows_navigation_barrier_before_pointer_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>First</button>"));
        client.set_evaluate_responses([
            json!({"result": {"value": {
                "status": "ready", "x": 13.5, "y": 21.25, "isMac": false
            }}}),
            json!({"result": {"value": {"status": "ready"}}}),
            json!({"result": {"value": {"status": "ready"}}}),
        ]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();

        // Act
        service
            .click_retained_top_level_element(
                "object-7",
                "world-3",
                ClickOptionsV2::new(),
                &press,
                || Ok(()),
            )
            .await
            .expect("top-level click should finish without requested navigation");

        // Assert
        assert_eq!(
            client.method_names(),
            [
                "Page.getFrameTree",
                "Runtime.callFunctionOn",
                "Page.enable",
                "Page.getFrameTree",
                "Runtime.callFunctionOn",
                "Input.dispatchMouseEvent",
                "Runtime.callFunctionOn",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
                "Page.enable",
            ]
        );
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn moved_retained_click_point_retries_the_same_object_before_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>First</button>"));
        client.set_evaluate_responses([
            json!({"result": {"value": {
                "status": "ready", "x": 13.5, "y": 21.25, "isMac": false
            }}}),
            json!({"result": {"value": {"status": "moved"}}}),
            json!({"result": {"value": {
                "status": "ready", "x": 14.0, "y": 22.0, "isMac": false
            }}}),
            json!({"result": {"value": {"status": "ready"}}}),
            json!({"result": {"value": {"status": "ready"}}}),
        ]);
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();

        // Act
        service
            .click_retained_top_level_element(
                "object-7",
                "world-3",
                ClickOptionsV2::new().with_no_wait_after(true),
                &press,
                || Ok(()),
            )
            .await
            .expect("moved point should retry against the exact retained object");

        // Assert
        assert_eq!(
            client.method_names(),
            [
                "Page.getFrameTree",
                "Runtime.callFunctionOn",
                "Runtime.callFunctionOn",
                "Runtime.callFunctionOn",
                "Runtime.callFunctionOn",
                "Input.dispatchMouseEvent",
                "Runtime.callFunctionOn",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
            ]
        );
        let calls = client.calls();
        for call in &calls[1..=4] {
            assert_eq!(call.1[0]["arguments"][0]["objectId"], "object-7");
        }
        assert_eq!(calls[5].1[0]["x"], 14.0);
        assert_eq!(calls[6].1[0]["arguments"][0]["objectId"], "object-7");
    }

    #[cfg(feature = "retained-element-click")]
    #[tokio::test]
    async fn invalid_retained_native_id_fails_before_browser_io() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>First</button>"));
        let service = retained_click_service(client.clone(), Duration::ZERO);
        let press = service.new_retained_click_press().unwrap();

        // Act
        let error = service
            .click_retained_top_level_element("", "world-3", ClickOptionsV2::new(), &press, || {
                Ok(())
            })
            .await
            .expect_err("empty native object ID must fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(client.calls().is_empty());
    }

    #[test]
    fn locator_v2_click_scrolls_large_position_through_overflow_ancestors() {
        // Arrange
        let selector = Selector::Css(selector("button"));
        let options = ClickOptionsV2::new()
            .with_position(Position::new(1900.0, 1910.0).expect("finite position should build"));

        // Act
        let script = click_v2_actionability_script(&selector, &options)
            .expect("positioned click actionability should compile");

        // Assert
        assert!(script.contains("const position = { x: 1900, y: 1910 }"));
        assert!(
            script.contains("let ancestor = element.parentElement || element.getRootNode()?.host")
        );
        assert!(script.contains("ancestor.scrollTo({ left: ancestor.scrollLeft + dx"));
        assert!(script.contains("window.scrollBy({ left: dx, top: dy"));
        assert!(
            script.find("ancestor.scrollTo").unwrap() < script.find("window.scrollBy").unwrap()
        );
        assert!(
            script.find("window.scrollBy").unwrap() < script.find("await new Promise").unwrap()
        );
    }

    #[tokio::test]
    async fn forced_locator_v2_click_returns_not_visible_without_retry_or_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<button style=\"display:none\">Submit</button>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "notvisible"}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let error = service
            .call(ClickParamsV2::new(
                Selector::Css(selector("button")),
                ClickOptionsV2::new().with_force(true),
            ))
            .await
            .expect_err("forced click should fail immediately for a no-layout element");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(error.message(), "Element is not visible");
        assert_eq!(
            client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
        let script = client.calls()[1].1[0]
            .as_str()
            .expect("actionability expression should be a string")
            .to_owned();
        assert!(script.contains("if (style.display === \"none\" || rect.width <= 0"));
        assert_eq!(
            service.selector_runtime_metrics().actionability_evaluations,
            1
        );
    }

    #[test]
    fn forced_click_reports_outside_viewport_separately_from_no_layout() {
        // Arrange
        let response = json!({"result": {"value": {"status": "out-of-viewport"}}});

        // Act
        let result = parse_pointer_attempt(response, "button", true, true);

        // Assert
        let error = result.err().expect("forced offscreen click should fail");
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(error.message(), "Element is outside of the viewport");
    }

    #[test]
    fn ordinary_click_retries_a_point_outside_viewport() {
        // Arrange
        let response = json!({"result": {"value": {"status": "out-of-viewport"}}});

        // Act
        let result = parse_pointer_attempt(response, "button", true, false);

        // Assert
        assert!(matches!(
            result,
            Ok(super::ActionAttempt::Retry(
                super::ActionabilityProgress::NotVisible
            ))
        ));
    }

    #[tokio::test]
    async fn locator_v2_click_waits_for_a_requested_main_frame_navigation_commit() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<a>Next</a>"));
        client.set_main_frame_id("main-frame");
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "ready", "x": 1.0, "y": 2.0, "isMac": false
            }}}),
        ]);
        client.set_page_enable_events([
            vec![],
            vec![
                CdpEvent::new(
                    "Page.frameRequestedNavigation".to_string(),
                    json!({"frameId": "main-frame", "disposition": "currentTab"}),
                    None,
                ),
                CdpEvent::new(
                    "Page.frameNavigated".to_string(),
                    json!({"frame": {"id": "main-frame", "url": "https://example.test/next"}}),
                    None,
                ),
            ],
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let result = service
            .call(ClickParamsV2::new(
                Selector::Css(selector("a")),
                ClickOptionsV2::new(),
            ))
            .await;

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Page.enable",
                "Page.getFrameTree",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
                "Page.enable",
            ]
        );
    }

    #[tokio::test]
    async fn locator_v2_click_finishes_when_requested_navigation_becomes_a_download() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<a>Download</a>"));
        client.set_main_frame_id("main-frame");
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "ready", "x": 1.0, "y": 2.0, "isMac": false
            }}}),
        ]);
        client.set_page_enable_events([
            vec![],
            vec![
                CdpEvent::new(
                    "Page.frameRequestedNavigation".to_string(),
                    json!({"frameId": "main-frame", "disposition": "currentTab"}),
                    None,
                ),
                CdpEvent::new(
                    "Browser.downloadWillBegin".to_string(),
                    json!({
                        "frameId": "main-frame",
                        "guid": "download-guid",
                        "url": "https://example.test/file",
                        "suggestedFilename": "file.txt"
                    }),
                    None,
                ),
            ],
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let result = service
            .call(ClickParamsV2::new(
                Selector::Css(selector("a")),
                ClickOptionsV2::new(),
            ))
            .await;

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Page.enable",
                "Page.getFrameTree",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
                "Page.enable",
            ]
        );
    }

    #[tokio::test]
    async fn locator_v2_click_rejects_unbounded_pointer_work_before_browser_io() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Submit</button>"));
        let service = CdpLocatorService::new(client.clone());
        let options = ClickOptionsV2::new().with_steps(
            NonZeroU32::new(super::MAX_CLICK_STEPS + 1).expect("limit plus one is nonzero"),
        );

        // Act
        let error = service
            .call(ClickParamsV2::new(
                Selector::Css(selector("button")),
                options,
            ))
            .await
            .expect_err("unbounded step count must fail closed");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(client.calls().is_empty());
    }

    #[tokio::test]
    async fn focus_uses_native_focus_twice_without_input_dispatch() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Target</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done"}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        service
            .call(FocusParams::new(Selector::Css(selector("button"))))
            .await
            .expect("strict target should focus");

        // Assert
        assert_eq!(
            client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
        let calls = client.calls();
        let script = calls[1].1[0]
            .as_str()
            .expect("evaluation should carry script");
        assert!(script.contains("element.focus(); element.focus();"));
        assert!(!script.contains("scrollIntoView"));
        assert_eq!(service.selector_runtime_metrics().focus_evaluations, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn scroll_into_view_retries_precommit_states_without_native_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div id='target'></div>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "missing"}}}),
            json!({"result": {"value": {"status": "runtime-missing"}}}),
            runtime_installed(),
            json!({"result": {"value": {"status": "detached"}}}),
            json!({"result": {"value": {"status": "no-layout"}}}),
            json!({"result": {"value": {"status": "unstable"}}}),
            json!({"result": {"value": {"status": "ready-to-scroll"}}}),
            json!({"result": {"value": {"status": "done"}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        service
            .call(ScrollIntoViewIfNeededParams::new(Selector::Css(selector(
                "#target",
            ))))
            .await
            .expect("all precommit states should re-resolve and retry");

        // Assert
        assert_eq!(client.method_names(), vec!["Runtime.evaluate"; 9]);
        assert!(
            client
                .method_names()
                .iter()
                .all(|method| !method.starts_with("Input."))
        );
        assert_eq!(
            service.selector_runtime_metrics().actionability_evaluations,
            7
        );
        assert_eq!(service.selector_runtime_metrics().installations, 2);
        assert_eq!(
            service.selector_runtime_metrics().missing_after_navigation,
            1
        );
    }

    #[test]
    fn scroll_into_view_script_encodes_stability_and_if_needed_semantics() {
        // Arrange
        let selector = Selector::Css(selector("#below-fold"));

        // Act
        let check_script = scroll_into_view_if_needed_check_script(&selector)
            .expect("a bounded strict selector should compile");
        let commit_script = scroll_into_view_if_needed_commit_script(&selector)
            .expect("a bounded strict selector should compile for commit");

        // Assert
        assert!(check_script.len() <= MAX_FOCUS_SCRIPT_BYTES);
        assert!(commit_script.len() <= MAX_FOCUS_SCRIPT_BYTES);
        assert!(
            check_script.contains("requestAnimationFrame(() => requestAnimationFrame(resolve))")
        );
        assert!(check_script.contains(
            "document.visibilityState !== \"visible\") { setTimeout(resolve, 0); return; }"
        ));
        assert!(check_script.contains("Math.abs(before.x - after.x) <= 0.25"));
        assert!(check_script.contains("new IntersectionObserver"));
        assert!(check_script.contains("ratio === 1 ? \"done\" : \"ready-to-scroll\""));
        assert!(!check_script.contains("element.scrollIntoView"));
        assert!(commit_script.contains(
            "element.scrollIntoView({ behavior: \"instant\", block: \"center\", inline: \"center\" })"
        ));
        assert!(!commit_script.contains("requestAnimationFrame"));
        assert!(!check_script.contains("__pw.visible"));
        assert!(!check_script.contains("elementState"));
        assert!(!check_script.contains("elementFromPoint"));
    }

    #[test]
    fn screenshot_geometry_accepts_oversized_partially_visible_elements() {
        // Arrange
        let selector = Selector::Css(selector("body"));

        // Act
        let check_script = screenshot_geometry_check_script(&selector)
            .expect("a bounded strict selector should compile");

        // Assert
        assert!(check_script.len() <= MAX_FOCUS_SCRIPT_BYTES);
        assert!(check_script.contains("if (ratio <= 0)"));
        assert!(!check_script.contains("if (ratio !== 1)"));
        assert!(!check_script.contains("element.scrollIntoView"));
    }

    #[tokio::test]
    async fn scroll_into_view_strict_failure_is_terminal_and_cancellation_is_stateless() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div></div><div></div>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());
        let context = OperationContext::new(
            CallId::new(7),
            ObjectId::new(8),
            ScrollIntoViewIfNeeded::METHOD,
            None,
        );

        // Act
        let error = service
            .call(ScrollIntoViewIfNeededParams::new(Selector::Css(selector(
                "div",
            ))))
            .await
            .expect_err("strict resolution should fail without retrying");
        let cancellation =
            <CdpLocatorService as CommandCancellation<ScrollIntoViewIfNeeded>>::cancel(
                &service,
                context,
                CancellationReason::Caller,
            )
            .await;

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(
            error.details(),
            Some(&LocatorErrorDetails::StrictViolation {
                selector: "div".to_string(),
                matches: 2,
            })
        );
        assert_eq!(cancellation, Ok(()));
        assert_eq!(client.method_names(), vec!["Runtime.evaluate"; 2]);
    }

    #[tokio::test]
    async fn scroll_into_view_rejects_non_elements_and_malformed_results_without_retry() {
        // Arrange
        let invalid_client = Arc::new(FakeCdpClient::from_html("<body></body>"));
        invalid_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "invalid-target",
                "message": "Node is not an Element"
            }}}),
        ]);
        let invalid_service = CdpLocatorService::new(invalid_client.clone());
        let malformed_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        malformed_client
            .set_evaluate_responses([runtime_installed(), json!({"result": {"value": true}})]);
        let malformed_service = CdpLocatorService::new(malformed_client.clone());

        // Act
        let invalid = invalid_service
            .call(ScrollIntoViewIfNeededParams::new(Selector::Css(selector(
                "body",
            ))))
            .await
            .expect_err("a non-Element target must fail terminally");
        let malformed = malformed_service
            .call(ScrollIntoViewIfNeededParams::new(Selector::Css(selector(
                "div",
            ))))
            .await
            .expect_err("a non-object response must fail terminally");

        // Assert
        assert_eq!(
            invalid.details(),
            Some(&LocatorErrorDetails::InvalidTarget(
                "Node is not an Element".to_string()
            ))
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
        assert_eq!(invalid_client.method_names(), vec!["Runtime.evaluate"; 2]);
        assert_eq!(malformed_client.method_names(), vec!["Runtime.evaluate"; 2]);
    }

    #[tokio::test]
    async fn blur_uses_native_blur_once_without_actionability_checks() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done"}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        service
            .call(BlurParams::new(Selector::Css(selector("input"))))
            .await
            .expect("strict target should blur");

        // Assert
        assert_eq!(
            client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
        let calls = client.calls();
        let script = calls[1].1[0]
            .as_str()
            .expect("evaluation should carry script");
        assert!(script.contains("element.blur();"));
        assert!(!script.contains("element.focus()"));
        assert!(!script.contains("scrollIntoView"));
        assert_eq!(service.selector_runtime_metrics().focus_evaluations, 1);
    }

    #[tokio::test(start_paused = true)]
    async fn focus_reinstalls_a_missing_runtime_and_retries_a_detached_target() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>Target</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "runtime-missing"}}}),
            runtime_installed(),
            json!({"result": {"value": {"status": "retry"}}}),
            json!({"result": {"value": {"status": "done"}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        service
            .call(FocusParams::new(Selector::Css(selector("button"))))
            .await
            .expect("focus should recover and re-resolve the target");

        // Assert
        assert_eq!(client.method_names(), vec!["Runtime.evaluate"; 5]);
        assert_eq!(
            service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics {
                installations: 2,
                actionability_evaluations: 0,
                hover_evaluations: 0,
                focus_evaluations: 3,
                snapshot_evaluations: 0,
                state_evaluations: 0,
                content_evaluations: 0,
                attribute_evaluations: 0,
                input_value_evaluations: 0,
                all_inner_texts_evaluations: 0,
                all_text_contents_evaluations: 0,
                wait_evaluations: 0,
                missing_after_navigation: 1,
            }
        );
    }

    #[tokio::test]
    async fn press_focuses_resets_selection_and_dispatches_a_shifted_chord() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input value='abc'>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "isMac": true}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());
        let params = PressParams::new(
            Selector::Css(selector("input")),
            PressKey::new("Shift+A").expect("key expression should parse"),
            PressOptions::new().with_no_wait_after(true),
        );

        // Act
        service.call(params).await.expect("press should succeed");

        // Assert
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Input.dispatchKeyEvent",
                "Input.dispatchKeyEvent",
                "Input.dispatchKeyEvent",
                "Input.dispatchKeyEvent",
            ]
        );
        let calls = client.calls();
        let script = calls[1].1[0].as_str().expect("focus script should exist");
        assert!(script.contains("element.focus();"));
        assert!(script.contains("element.setSelectionRange(0, 0)"));
        assert!(!script.contains("scrollIntoView"));
        assert_eq!(
            (
                calls[2].1[0]["type"].as_str(),
                calls[2].1[0]["code"].as_str(),
                calls[2].1[0]["modifiers"].as_i64()
            ),
            (Some("rawKeyDown"), Some("ShiftLeft"), Some(8))
        );
        assert_eq!(
            (
                calls[3].1[0]["type"].as_str(),
                calls[3].1[0]["key"].as_str(),
                calls[3].1[0]["text"].as_str()
            ),
            (Some("keyDown"), Some("A"), Some("A"))
        );
        assert_eq!(
            (
                calls[4].1[0]["type"].as_str(),
                calls[4].1[0]["code"].as_str()
            ),
            (Some("keyUp"), Some("KeyA"))
        );
        assert_eq!(
            (
                calls[5].1[0]["type"].as_str(),
                calls[5].1[0]["code"].as_str(),
                calls[5].1[0]["modifiers"].as_i64()
            ),
            (Some("keyUp"), Some("ShiftLeft"), Some(0))
        );
    }

    #[tokio::test]
    async fn press_rejects_an_unknown_key_before_browser_io() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input>"));
        let service = CdpLocatorService::new(client.clone());
        let params = PressParams::new(
            Selector::Css(selector("input")),
            PressKey::new("HyperSpace").expect("bounded expression should parse"),
            PressOptions::new(),
        );

        // Act
        let error = service
            .call(params)
            .await
            .expect_err("unknown key should fail");

        // Assert
        assert!(matches!(
            error.details(),
            Some(LocatorErrorDetails::InvalidValue(message)) if message.contains("HyperSpace")
        ));
        assert!(client.calls().is_empty());
    }

    #[tokio::test]
    async fn press_keyup_failure_is_cleaned_in_reverse_before_returning() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "isMac": false}}}),
        ]);
        client.set_input_errors([
            None,
            None,
            Some("main key up failed".to_string()),
            None,
            None,
        ]);
        let service = CdpLocatorService::new(client.clone());
        let params = PressParams::new(
            Selector::Css(selector("input")),
            PressKey::new("Control+A").expect("key expression should parse"),
            PressOptions::new().with_no_wait_after(true),
        );

        // Act
        let error = service
            .call(params)
            .await
            .expect_err("key up should fail press");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(error.message().contains("main key up failed"));
        let input = client
            .calls()
            .into_iter()
            .filter(|(method, _)| method == "Input.dispatchKeyEvent")
            .collect::<Vec<_>>();
        assert_eq!(input.len(), 5);
        assert_eq!(input[2].1[0]["code"], "KeyA");
        assert_eq!(input[3].1[0]["code"], "KeyA");
        assert_eq!(input[4].1[0]["code"], "ControlLeft");
        assert_eq!(input[4].1[0]["modifiers"], 0);
    }

    #[tokio::test]
    async fn press_cancellation_releases_every_acknowledged_key_in_reverse() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input>"));
        let service = CdpLocatorService::new(client.clone());
        let mut transaction = service
            .input
            .begin_transaction(client.clone(), None)
            .await
            .expect("input transaction should start");
        let shift = resolve_key("Shift", false, true).expect("Shift");
        let key = resolve_key("KeyA", true, true).expect("KeyA");
        transaction.key_down(shift, true).await.expect("Shift down");
        transaction.key_down(key, true).await.expect("A down");
        drop(transaction);
        let context = OperationContext::new(CallId::new(91), ObjectId::new(3), Press::METHOD, None);

        // Act
        let result = <CdpLocatorService as CommandCancellation<Press>>::cancel(
            &service,
            context,
            CancellationReason::DeadlineExceeded,
        )
        .await;

        // Assert
        assert_eq!(result, Ok(()));
        let calls = client.calls();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[2].1[0]["code"], "KeyA");
        assert_eq!(calls[3].1[0]["code"], "ShiftLeft");
        assert_eq!(calls[3].1[0]["modifiers"], 0);
    }

    #[test]
    fn click_deadline_projects_and_consumes_the_last_bounded_progress() {
        // Arrange
        let service = CdpLocatorService::new(Arc::new(FakeCdpClient::from_html("")));
        service.record_action_progress(super::ActionabilityProgress::Disabled);
        let context = OperationContext::new(
            CallId::new(92),
            ObjectId::new(3),
            pwright_cap_locator::v2::Click::METHOD,
            None,
        );

        // Act
        let first = <CdpLocatorService as CommandCancellation<
            pwright_cap_locator::v2::Click,
        >>::terminal_error_after_cancel(
            &service,
            context,
            CancellationReason::DeadlineExceeded,
            Some(Duration::from_millis(250)),
        );
        let second = <CdpLocatorService as CommandCancellation<
            pwright_cap_locator::v2::Click,
        >>::terminal_error_after_cancel(
            &service,
            context,
            CancellationReason::DeadlineExceeded,
            Some(Duration::from_millis(250)),
        );

        // Assert
        let first = first.expect("recorded progress should enrich the deadline");
        assert_eq!(first.kind(), FailureKind::Timeout);
        assert_eq!(
            first.details(),
            Some(&LocatorErrorDetails::Backend(
                "actionability:disabled".to_string()
            ))
        );
        assert!(second.is_none());
    }

    #[tokio::test]
    async fn select_option_retries_precommit_states_and_returns_dom_order_values() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<select multiple></select>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "retry"}}}),
            json!({"result": {"value": {"status": "notvisible"}}}),
            json!({"result": {"value": {"status": "disabled"}}}),
            json!({"result": {"value": {"status": "options-not-found"}}}),
            json!({"result": {"value": {"status": "option-not-enabled"}}}),
            json!({"result": {"value": {"status": "done", "values": ["green", "blue"]}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);

        // Act
        let values = service
            .call(SelectOptionParams::new(
                Selector::Css(selector("select")),
                [
                    SelectOptionValue::value_or_label("Blue"),
                    SelectOptionValue::new().with_value("green").with_index(2),
                ],
                SelectOptionOptions::new().with_force(false),
            ))
            .await
            .expect("all precommit states should retry");

        // Assert
        assert_eq!(values, ["green", "blue"]);
        assert_eq!(client.method_names(), vec!["Runtime.evaluate"; 7]);
        assert!(
            client
                .method_names()
                .iter()
                .all(|method| method != "Input.dispatchMouseEvent")
        );
    }

    #[cfg(feature = "select-option-timeout-progress")]
    #[tokio::test(start_paused = true)]
    async fn disabled_option_retry_is_retained_for_only_this_timed_out_call() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<select><option disabled>two</option></select>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "option-not-enabled"}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::from_secs(1));
        let progress = pwright_cap_locator::SelectOptionProgress::new();
        let params = SelectOptionParams::new(
            Selector::Css(selector("select")),
            "two",
            SelectOptionOptions::new(),
        )
        .with_progress(progress.clone());

        // Act
        let deadline = tokio::time::timeout(Duration::from_millis(10), service.call(params)).await;

        // Assert
        assert!(deadline.is_err());
        assert!(progress.option_not_enabled());
        assert_eq!(client.method_names(), vec!["Runtime.evaluate"; 2]);
    }

    #[cfg(feature = "select-option-timeout-progress")]
    #[tokio::test(start_paused = true)]
    async fn other_select_option_retry_does_not_claim_disabled_option() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<select></select>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "options-not-found"}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client, Duration::from_secs(1));
        let progress = pwright_cap_locator::SelectOptionProgress::new();
        let params = SelectOptionParams::new(
            Selector::Css(selector("select")),
            "two",
            SelectOptionOptions::new(),
        )
        .with_progress(progress.clone());

        // Act
        let deadline = tokio::time::timeout(Duration::from_millis(10), service.call(params)).await;

        // Assert
        assert!(deadline.is_err());
        assert!(!progress.option_not_enabled());
    }

    #[cfg(feature = "select-option-timeout-progress")]
    #[tokio::test]
    async fn successful_selection_clears_an_earlier_disabled_option_retry() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<select><option>two</option></select>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "option-not-enabled"}}}),
            json!({"result": {"value": {"status": "done", "values": ["two"]}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client, Duration::ZERO);
        let progress = pwright_cap_locator::SelectOptionProgress::new();
        let params = SelectOptionParams::new(
            Selector::Css(selector("select")),
            "two",
            SelectOptionOptions::new(),
        )
        .with_progress(progress.clone());

        // Act
        let values = service.call(params).await.unwrap();

        // Assert
        assert_eq!(values, ["two"]);
        assert!(!progress.option_not_enabled());
    }

    #[test]
    fn select_option_script_preserves_matching_event_and_force_semantics() {
        // Arrange
        let values = vec![
            SelectOptionValue::value_or_label("Blue"),
            SelectOptionValue::new()
                .with_value("green")
                .with_label(" Green ")
                .with_index(-1),
        ];

        // Act
        let script = select_option_script(
            &Selector::Css(selector("select")),
            &values,
            SelectOptionOptions::new().with_force(true),
            None,
        )
        .expect("bounded descriptors should compile");
        let empty = select_option_script(
            &Selector::Css(selector("select")),
            &[],
            SelectOptionOptions::new(),
            None,
        )
        .expect("empty selection should compile");

        // Assert
        assert!(script.contains(r#""valueOrLabel":"Blue""#));
        assert!(script.contains(r#""value":"green""#));
        assert!(script.contains(r#""label":" Green ""#));
        assert!(script.contains(r#""index":-1"#));
        assert!(script.contains("if (!true)"));
        assert!(script.contains("descriptor.valueOrLabel === option.value"));
        assert!(script.contains("if (element.multiple)"));
        assert!(script.contains("__pw.elementState(option, \"enabled\")"));
        assert!(script.contains(r#"status: "option-not-enabled""#));
        assert!(script.contains("element.value = undefined"));
        assert!(script.contains(r#"new Event("input", { bubbles: true, composed: true })"#));
        assert!(script.contains(r#"new Event("change", { bubbles: true })"#));
        assert!(empty.contains("const requested = [];"));
        assert!(!script.contains("__pwrightFillCommit"));
    }

    #[cfg(feature = "select-option-navigation-commit")]
    #[test]
    fn select_option_receipt_follows_dom_commit_and_precedes_input_listener() {
        // Arrange
        let selector = Selector::Css(selector("select"));

        // Act
        let script = select_option_script(
            &selector,
            &[SelectOptionValue::from("blue")],
            SelectOptionOptions::new(),
            Some("select-option:world:1"),
        )
        .unwrap();

        // Assert
        let selected = script
            .find("selected.forEach(option => option.selected = true)")
            .unwrap();
        let committed_values = script
            .find("const committedValues = selected.map(option => option.value)")
            .unwrap();
        let input = script.find("dispatchEvent(new Event(\"input\"").unwrap();
        let change = script.find("dispatchEvent(new Event(\"change\"").unwrap();
        let receipt = script.find("__pwrightFillCommit").unwrap();
        let result_values = script
            .rfind("const values = selected.map(option => option.value)")
            .unwrap();
        assert!(script.contains("if (receipt.length <= 65536)"));
        assert!(
            selected < committed_values
                && committed_values < receipt
                && receipt < input
                && input < change
                && change < result_values
        );
    }

    #[cfg(feature = "select-option-navigation-commit")]
    #[tokio::test]
    async fn select_option_recovers_values_only_from_receipt_and_frame_navigation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<select><option value='blue'>Blue</option></select>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "count": 1}}}),
        ]);
        let mut service = CdpLocatorService::new(client.clone());
        service.unique_context_id = Some(Arc::from("utility-context-1"));
        let service = service.with_select_option_commit_route(7, Arc::from("frame-a"));
        service
            .call(CountParams::new(selector("select")))
            .await
            .unwrap();
        client.set_evaluate_errors(["Cannot find context with specified id".to_owned()]);
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_evaluate_gate(Some(gate.clone()));

        // Act
        let selection = tokio::spawn(async move {
            service
                .call(SelectOptionParams::new(
                    selector("select"),
                    "blue",
                    SelectOptionOptions::new(),
                ))
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if client.method_names().len() >= 4 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let calls = client.calls();
        let script = calls[3].1[0]["expression"].as_str().unwrap();
        let prefix = "token: ";
        let start = script.find(prefix).unwrap() + prefix.len();
        let end = start + script[start..].find(", values").unwrap();
        let token: String = serde_json::from_str(&script[start..end]).unwrap();
        client.send_event(CdpEvent::new(
            "Runtime.bindingCalled".to_owned(),
            json!({
                "name": super::FILL_COMMIT_BINDING,
                "executionContextId": 7,
                "payload": json!({"token": token, "values": ["blue"]}).to_string(),
            }),
            None,
        ));
        client.send_event(CdpEvent::new(
            "Page.frameNavigated".to_owned(),
            json!({"frame": {"id": "frame-a"}}),
            None,
        ));
        gate.notify_one();
        let result = selection.await.unwrap();

        // Assert
        assert_eq!(result.unwrap(), ["blue"]);
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.addBinding",
                "Runtime.evaluate",
            ]
        );
    }

    #[cfg(feature = "select-option-navigation-commit")]
    #[tokio::test]
    async fn select_option_recovers_through_exact_session_receipt_and_frame_navigation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<select><option value='blue'>Blue</option></select>",
        ));
        client.set_event_session_id("selected-session").unwrap();
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "count": 1}}}),
        ]);
        let mut service = CdpLocatorService::new(client.clone());
        service.unique_context_id = Some(Arc::from("utility-context-1"));
        let service = service.with_select_option_commit_route(7, Arc::from("frame-a"));
        service
            .call(CountParams::new(selector("select")))
            .await
            .unwrap();
        client.set_evaluate_errors(["Cannot find context with specified id".to_owned()]);
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_evaluate_gate(Some(gate.clone()));

        // Act
        let selection = tokio::spawn(async move {
            service
                .call(SelectOptionParams::new(
                    selector("select"),
                    "blue",
                    SelectOptionOptions::new(),
                ))
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if client.method_names().len() >= 4 {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let calls = client.calls();
        let script = calls[3].1[0]["expression"].as_str().unwrap();
        let start = script.find("token: ").unwrap() + "token: ".len();
        let end = start + script[start..].find(", values").unwrap();
        let token: String = serde_json::from_str(&script[start..end]).unwrap();
        client.send_event(CdpEvent::new(
            "Runtime.bindingCalled".to_owned(),
            json!({
                "name": super::FILL_COMMIT_BINDING,
                "executionContextId": 7,
                "payload": json!({"token": token, "values": ["blue"]}).to_string(),
            }),
            Some("selected-session".to_owned()),
        ));
        client.send_event(CdpEvent::new(
            "Page.frameNavigated".to_owned(),
            json!({"frame": {"id": "frame-a"}}),
            Some("selected-session".to_owned()),
        ));
        gate.notify_one();
        let result = selection.await.unwrap();

        // Assert
        assert_eq!(result.unwrap(), ["blue"]);
        assert_eq!(
            client.method_names(),
            [
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.addBinding",
                "Runtime.evaluate"
            ]
        );
        assert!(
            script.find(super::FILL_COMMIT_BINDING).unwrap()
                < script.find("dispatchEvent(new Event(\"input\"").unwrap()
        );
    }

    #[cfg(feature = "select-option-navigation-commit")]
    #[tokio::test]
    async fn select_option_preserves_context_loss_without_commit_receipt() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<select><option value='blue'>Blue</option></select>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "count": 1}}}),
        ]);
        let mut service = CdpLocatorService::new(client.clone());
        service.unique_context_id = Some(Arc::from("utility-context-1"));
        let service = service.with_select_option_commit_route(7, Arc::from("frame-a"));
        service
            .call(CountParams::new(selector("select")))
            .await
            .unwrap();
        client.set_evaluate_errors(["Cannot find context with specified id".to_owned()]);

        // Act
        let error = service
            .call(SelectOptionParams::new(
                selector("select"),
                "blue",
                SelectOptionOptions::new(),
            ))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(
            error
                .message()
                .contains("Cannot find context with specified id")
        );
    }

    #[tokio::test]
    async fn select_option_rejects_invalid_target_and_unbounded_input_without_native_io() {
        // Arrange
        let invalid_client = Arc::new(FakeCdpClient::from_html("<body></body>"));
        invalid_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "invalid-target",
                "message": "Element is not a <select> element"
            }}}),
        ]);
        let invalid_service = CdpLocatorService::new(invalid_client.clone());
        let oversized_client = Arc::new(FakeCdpClient::from_html("<select></select>"));
        let oversized_service = CdpLocatorService::new(oversized_client.clone());
        let oversized = "x".repeat(super::MAX_SELECT_OPTION_ITEM_BYTES + 1);

        // Act
        let invalid = invalid_service
            .call(SelectOptionParams::new(
                Selector::Css(selector("body")),
                "x",
                SelectOptionOptions::new(),
            ))
            .await
            .expect_err("non-select target must fail terminally");
        let oversized = oversized_service
            .call(SelectOptionParams::new(
                Selector::Css(selector("select")),
                oversized,
                SelectOptionOptions::new(),
            ))
            .await
            .expect_err("oversized descriptor must fail before browser IO");

        // Assert
        assert!(matches!(
            invalid.details(),
            Some(LocatorErrorDetails::InvalidTarget(message))
                if message == "Element is not a <select> element"
        ));
        assert_eq!(invalid_client.method_names(), vec!["Runtime.evaluate"; 2]);
        assert_eq!(oversized.kind(), FailureKind::ResourceExhausted);
        assert!(oversized_client.calls().is_empty());
    }

    #[tokio::test]
    async fn select_option_cancellation_owns_no_input_or_handle_cleanup() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<select></select>"));
        let service = CdpLocatorService::new(client.clone());
        let context = OperationContext::new(
            CallId::new(92),
            ObjectId::new(3),
            SelectOption::METHOD,
            None,
        );

        // Act
        let result = <CdpLocatorService as CommandCancellation<SelectOption>>::cancel(
            &service,
            context,
            CancellationReason::DeadlineExceeded,
        )
        .await;

        // Assert
        assert_eq!(result, Ok(()));
        assert!(client.calls().is_empty());
    }

    #[tokio::test]
    async fn set_checked_is_a_noop_when_the_requested_state_already_matches() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input type='checkbox' checked>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "checked": true, "isRadio": false}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        service
            .call(SetCheckedParams::new(
                selector("input"),
                true,
                SetCheckedOptions::new(),
            ))
            .await
            .expect("matching state should succeed without pointer work");

        // Assert
        assert_eq!(
            client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
    }

    #[tokio::test]
    async fn set_checked_retries_missing_and_runtime_replaced_targets_before_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input type='checkbox'>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "retry"}}}),
            json!({"result": {"value": {"status": "runtime-missing"}}}),
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "checked": true, "isRadio": false}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);

        // Act
        service
            .call(SetCheckedParams::new(
                selector("input"),
                true,
                SetCheckedOptions::new(),
            ))
            .await
            .expect("a matching target should succeed after pre-input retries");

        // Assert
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.evaluate",
            ]
        );
        assert_eq!(service.selector_runtime_metrics().state_evaluations, 3);
        assert_eq!(
            service.selector_runtime_metrics().missing_after_navigation,
            1
        );
    }

    #[tokio::test]
    async fn set_checked_clicks_once_and_verifies_the_post_state() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input type='checkbox'>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "checked": false, "isRadio": false}}}),
            json!({"result": {"value": {"status": "ready", "x": 4.5, "y": 8.0, "isMac": true}}}),
            json!({"result": {"value": {"status": "done", "checked": true, "isRadio": false}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());
        let options = SetCheckedOptions::new()
            .with_force(false)
            .with_position(Position::new(2.0, 3.0).expect("finite position"))
            .with_scroll(ScrollMode::None);

        // Act
        service
            .call(SetCheckedParams::new(selector("input"), true, options))
            .await
            .expect("checked state should be changed and verified");

        // Assert
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
                "Input.dispatchMouseEvent",
                "Runtime.evaluate",
            ]
        );
        let calls = client.calls();
        let actionability = calls[2].1[0]
            .as_str()
            .expect("actionability expression should be a string");
        assert!(actionability.contains("const position = { x: 2, y: 3 }"));
        assert!(actionability.contains("if (false) {\n    if (position)"));
        assert_eq!(calls[3].1[0]["type"], "mouseMoved");
        assert_eq!(calls[4].1[0]["type"], "mousePressed");
        assert_eq!(calls[4].1[0]["button"], "left");
        assert_eq!(calls[5].1[0]["type"], "mouseReleased");
    }

    #[tokio::test]
    async fn set_checked_mouse_up_failure_cleans_the_button_without_postcheck() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input type='checkbox'>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "checked": false, "isRadio": false}}}),
            json!({"result": {"value": {"status": "ready", "x": 1.0, "y": 2.0, "isMac": false}}}),
        ]);
        client.set_input_errors([None, None, Some("mouse up failed".to_string()), None]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let error = service
            .call(SetCheckedParams::new(
                selector("input"),
                true,
                SetCheckedOptions::new(),
            ))
            .await
            .expect_err("the failed click must fail SetChecked");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(error.message().contains("mouse up failed"));
        let input = client
            .calls()
            .into_iter()
            .filter(|(method, _)| method == "Input.dispatchMouseEvent")
            .collect::<Vec<_>>();
        assert_eq!(input.len(), 4);
        assert_eq!(input[0].1[0]["type"], "mouseMoved");
        assert_eq!(input[1].1[0]["type"], "mousePressed");
        assert_eq!(input[2].1[0]["type"], "mouseReleased");
        assert_eq!(input[3].1[0]["type"], "mouseReleased");
        assert_eq!(service.selector_runtime_metrics().state_evaluations, 1);
    }

    #[tokio::test]
    async fn set_checked_rejects_unchecking_a_checked_radio() {
        // Arrange
        let radio_client = Arc::new(FakeCdpClient::from_html("<input type='radio' checked>"));
        radio_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "checked": true, "isRadio": true}}}),
        ]);
        let radio_service = CdpLocatorService::new(radio_client.clone());

        // Act
        let radio_error = radio_service
            .call(SetCheckedParams::new(
                selector("input"),
                false,
                SetCheckedOptions::new(),
            ))
            .await
            .expect_err("a checked radio cannot be unchecked directly");

        // Assert
        assert!(
            radio_error
                .message()
                .contains("Cannot uncheck radio button")
        );
        assert_eq!(
            radio_client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
    }

    #[tokio::test]
    async fn set_checked_reports_a_failed_postcondition_without_replaying_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input type='checkbox'>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "checked": false, "isRadio": false}}}),
            json!({"result": {"value": {"status": "ready", "x": 1.0, "y": 2.0, "isMac": false}}}),
            json!({"result": {"value": {"status": "done", "checked": false, "isRadio": false}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let error = service
            .call(SetCheckedParams::new(
                selector("input"),
                true,
                SetCheckedOptions::new(),
            ))
            .await
            .expect_err("unchanged checkbox state must not report success");

        // Assert
        assert!(error.message().contains("did not change its state"));
        assert_eq!(
            client
                .method_names()
                .iter()
                .filter(|method| method.as_str() == "Input.dispatchMouseEvent")
                .count(),
            3
        );
    }

    #[tokio::test]
    async fn set_checked_trial_runs_actionability_without_input_or_postcheck() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input type='checkbox'>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "checked": false, "isRadio": false}}}),
            json!({"result": {"value": {"status": "ready", "x": 1.0, "y": 2.0, "isMac": false}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        service
            .call(SetCheckedParams::new(
                selector("input"),
                true,
                SetCheckedOptions::new().with_trial(true),
            ))
            .await
            .expect("trial should finish after actionability");

        // Assert
        assert_eq!(
            client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate", "Runtime.evaluate"]
        );
    }

    #[tokio::test]
    async fn set_checked_cancellation_releases_a_button_from_a_dropped_transaction() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input type='checkbox'>"));
        let service = CdpLocatorService::new(client.clone());
        let mut transaction = service
            .input
            .begin_transaction(client.clone(), None)
            .await
            .expect("input transaction should start");
        transaction
            .move_mouse(3.0, 4.0)
            .await
            .expect("pointer should move");
        transaction
            .mouse_down(pwright_cdp::MouseButton::Left, 1)
            .await
            .expect("button should press");
        drop(transaction);
        let context =
            OperationContext::new(CallId::new(97), ObjectId::new(3), SetChecked::METHOD, None);

        // Act
        let result = <CdpLocatorService as CommandCancellation<SetChecked>>::cancel(
            &service,
            context,
            CancellationReason::Caller,
        )
        .await;

        // Assert
        assert_eq!(result, Ok(()));
        let calls = client.calls();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].1[0]["type"], "mouseMoved");
        assert_eq!(calls[1].1[0]["type"], "mousePressed");
        assert_eq!(calls[2].1[0]["type"], "mouseReleased");
        assert_eq!(calls[2].1[0]["buttons"], 0);
    }

    #[tokio::test]
    async fn hover_dispatches_one_move_with_fractional_position_and_platform_modifiers() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button disabled>Target</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "ready", "x": 12.5, "y": 19.25, "isMac": true
            }}}),
        ]);
        let service = CdpLocatorService::new(client.clone());
        let options = HoverOptions::new()
            .with_scroll(ScrollMode::None)
            .with_modifier(KeyboardModifier::ControlOrMeta)
            .with_modifier(KeyboardModifier::Shift)
            .with_position(Position::new(2.5, 3.25).expect("finite position should build"));

        // Act
        service
            .call(HoverParams::new(Selector::Css(selector("button")), options))
            .await
            .expect("visible disabled target should be hoverable");

        // Assert
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Input.dispatchKeyEvent",
                "Input.dispatchKeyEvent",
                "Input.dispatchMouseEvent",
                "Input.dispatchKeyEvent",
                "Input.dispatchKeyEvent",
            ]
        );
        let calls = client.calls();
        assert_eq!(calls[2].1[0]["type"], "rawKeyDown");
        assert_eq!(calls[2].1[0]["key"], "Meta");
        assert_eq!(calls[2].1[0]["modifiers"], 4);
        assert_eq!(calls[3].1[0]["key"], "Shift");
        assert_eq!(calls[3].1[0]["modifiers"], 12);
        assert_eq!(calls[4].1[0]["type"], "mouseMoved");
        assert_eq!(calls[4].1[0]["x"], 12.5);
        assert_eq!(calls[4].1[0]["y"], 19.25);
        assert_eq!(calls[4].1[0]["buttons"], 0);
        assert_eq!(calls[4].1[0]["modifiers"], 12);
        assert_eq!(calls[5].1[0]["type"], "keyUp");
        assert_eq!(calls[5].1[0]["key"], "Meta");
        assert_eq!(calls[5].1[0]["modifiers"], 8);
        assert_eq!(calls[6].1[0]["key"], "Shift");
        assert_eq!(calls[6].1[0]["modifiers"], 0);
        let script = calls[1].1[0]
            .as_str()
            .expect("evaluation should carry script");
        assert!(script.contains("const position = { x: 2.5, y: 3.25 }"));
        assert!(script.contains("if (false) element.scrollIntoView"));
        assert!(!script.contains("aria-disabled"));
        assert_eq!(service.selector_runtime_metrics().hover_evaluations, 1);
    }

    #[tokio::test]
    async fn hover_trial_presses_and_restores_modifiers_without_moving_mouse() {
        // Arrange
        let trial_client = Arc::new(FakeCdpClient::from_html("<div>Target</div>"));
        trial_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "ready", "x": 1.0, "y": 2.0, "isMac": false
            }}}),
        ]);
        let trial_service = CdpLocatorService::new(trial_client.clone());
        let invalid_client = Arc::new(FakeCdpClient::from_html("<div>Target</div>"));
        invalid_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "ready", "x": 1.0, "y": 2.0}}}),
        ]);
        let invalid_service = CdpLocatorService::new(invalid_client);

        // Act
        let trial = trial_service
            .call(HoverParams::new(
                Selector::Css(selector("div")),
                HoverOptions::new()
                    .with_modifier(KeyboardModifier::Shift)
                    .with_trial(true),
            ))
            .await;
        let invalid = invalid_service
            .call(HoverParams::new(
                Selector::Css(selector("div")),
                HoverOptions::new(),
            ))
            .await
            .expect_err("platform-less result should fail");

        // Assert
        assert_eq!(trial, Ok(()));
        assert_eq!(
            trial_client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Input.dispatchKeyEvent",
                "Input.dispatchKeyEvent",
            ]
        );
        let calls = trial_client.calls();
        assert_eq!(calls[2].1[0]["type"], "rawKeyDown");
        assert_eq!(calls[2].1[0]["key"], "Shift");
        assert_eq!(calls[3].1[0]["type"], "keyUp");
        assert_eq!(calls[3].1[0]["modifiers"], 0);
        assert_eq!(invalid.kind(), FailureKind::ProtocolViolation);
    }

    #[tokio::test]
    async fn hover_mouse_failure_restores_modifiers_before_returning_error() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div>Target</div>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "ready", "x": 1.0, "y": 2.0, "isMac": false
            }}}),
        ]);
        client.set_input_errors([None, Some("mouse move failed".to_string()), None]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let error = service
            .call(HoverParams::new(
                Selector::Css(selector("div")),
                HoverOptions::new().with_modifier(KeyboardModifier::Shift),
            ))
            .await
            .expect_err("mouse failure should fail hover");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(error.message().contains("mouse move failed"));
        let calls = client.calls();
        assert_eq!(calls[2].1[0]["type"], "rawKeyDown");
        assert_eq!(calls[3].0, "Input.dispatchMouseEvent");
        assert_eq!(calls[4].1[0]["type"], "keyUp");
        assert_eq!(calls[4].1[0]["modifiers"], 0);
    }

    #[tokio::test]
    async fn hover_cancellation_restores_a_dropped_modifier_transaction() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div>Target</div>"));
        let service = CdpLocatorService::new(client.clone());
        let transaction = service
            .input
            .begin_transaction(client.clone(), Some(super::CdpModifiers::ALT))
            .await
            .expect("modifier transaction should start");
        drop(transaction);
        let context = OperationContext::new(
            CallId::new(46),
            ObjectId::new(3),
            pwright_cap_locator::Hover::METHOD,
            None,
        );

        // Act
        let result =
            <CdpLocatorService as CommandCancellation<pwright_cap_locator::Hover>>::cancel(
                &service,
                context,
                CancellationReason::Caller,
            )
            .await;

        // Assert
        assert_eq!(result, Ok(()));
        let calls = client.calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1[0]["type"], "rawKeyDown");
        assert_eq!(calls[1].1[0]["type"], "keyUp");
        assert_eq!(calls[1].1[0]["modifiers"], 0);
    }

    #[tokio::test]
    async fn count_returns_zero_or_many_without_strictness_or_actionability_waiting() {
        // Arrange
        let zero_client = Arc::new(FakeCdpClient::from_html(""));
        zero_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "count": 0}}}),
        ]);
        let zero_service = CdpLocatorService::new(zero_client.clone());
        let many_client = Arc::new(FakeCdpClient::from_html(
            "<div></div><div></div><div></div>",
        ));
        many_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "count": 3}}}),
        ]);
        let many_service = CdpLocatorService::new(many_client.clone());

        // Act
        let zero = zero_service
            .call(CountParams::new(selector(".missing")))
            .await;
        let many = many_service.call(CountParams::new(selector("div"))).await;

        // Assert
        assert_eq!(zero, Ok(0));
        assert_eq!(many, Ok(3));
        assert_eq!(
            zero_client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
        assert_eq!(
            many_client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
        assert_eq!(
            many_service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics {
                installations: 1,
                actionability_evaluations: 0,
                hover_evaluations: 0,
                focus_evaluations: 0,
                snapshot_evaluations: 1,
                state_evaluations: 0,
                content_evaluations: 0,
                attribute_evaluations: 0,
                input_value_evaluations: 0,
                all_inner_texts_evaluations: 0,
                all_text_contents_evaluations: 0,
                wait_evaluations: 0,
                missing_after_navigation: 0,
            }
        );
    }

    #[tokio::test]
    async fn count_reinstalls_a_missing_runtime_once_and_rejects_bad_results() {
        // Arrange
        let recovered_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        recovered_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "runtime-missing"}}}),
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "count": 1}}}),
        ]);
        let recovered_service = CdpLocatorService::new(recovered_client.clone());
        let invalid_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        invalid_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "invalid", "message": "SyntaxError"}}}),
        ]);
        let invalid_service = CdpLocatorService::new(invalid_client);
        let overflow_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        overflow_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "count": u64::from(u32::MAX) + 1}}}),
        ]);
        let overflow_service = CdpLocatorService::new(overflow_client);

        // Act
        let recovered = recovered_service
            .call(CountParams::new(selector("div")))
            .await;
        let invalid = invalid_service
            .call(CountParams::new(selector("[")))
            .await
            .expect_err("invalid selector should fail");
        let overflow = overflow_service
            .call(CountParams::new(selector("div")))
            .await
            .expect_err("overflow count should fail");

        // Assert
        assert_eq!(recovered, Ok(1));
        assert_eq!(invalid.kind(), FailureKind::Backend);
        assert_eq!(
            invalid.details(),
            Some(&LocatorErrorDetails::InvalidSelector(
                "SyntaxError".to_string()
            ))
        );
        assert_eq!(overflow.kind(), FailureKind::ResourceExhausted);
        assert_eq!(recovered_client.method_names().len(), 4);
        assert_eq!(
            recovered_service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics {
                installations: 2,
                actionability_evaluations: 0,
                hover_evaluations: 0,
                focus_evaluations: 0,
                snapshot_evaluations: 2,
                state_evaluations: 0,
                content_evaluations: 0,
                attribute_evaluations: 0,
                input_value_evaluations: 0,
                all_inner_texts_evaluations: 0,
                all_text_contents_evaluations: 0,
                wait_evaluations: 0,
                missing_after_navigation: 1,
            }
        );
    }

    #[tokio::test]
    async fn all_inner_texts_returns_an_ordered_non_strict_rendered_snapshot() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<div>first</div><div></div><div>third</div>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "done",
                "values": ["first", "", "third"]
            }}}),
            json!({"result": {"value": {"status": "done", "values": []}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let ordered = service
            .call(AllInnerTextsParams::new(selector("div")))
            .await;
        let empty = service
            .call(AllInnerTextsParams::new(selector(".missing")))
            .await;

        // Assert
        assert_eq!(ordered, Ok(vec!["first".into(), "".into(), "third".into()]));
        assert_eq!(empty, Ok(Vec::new()));
        assert_eq!(client.method_names().len(), 3);
        let metrics = service.selector_runtime_metrics();
        assert_eq!(metrics.snapshot_evaluations, 0);
        assert_eq!(metrics.all_inner_texts_evaluations, 2);
        assert_eq!(metrics.all_text_contents_evaluations, 0);
        let calls = client.calls();
        let expression = calls[1].1[0]
            .as_str()
            .expect("snapshot expression should be a string");
        assert!(expression.contains("matches.map(element => element.innerText)"));
        assert!(expression.contains("element instanceof HTMLElement"));
        assert!(!expression.contains("matches.length > 1"));
    }

    #[tokio::test]
    async fn all_inner_texts_reinstalls_runtime_once_without_polling_for_matches() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div>ready</div>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "runtime-missing"}}}),
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "values": ["ready"]}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let result = service
            .call(AllInnerTextsParams::new(selector("div")))
            .await;

        // Assert
        assert_eq!(result, Ok(vec!["ready".into()]));
        assert_eq!(client.method_names().len(), 4);
        let metrics = service.selector_runtime_metrics();
        assert_eq!(metrics.installations, 2);
        assert_eq!(metrics.all_inner_texts_evaluations, 2);
        assert_eq!(metrics.missing_after_navigation, 1);
    }

    #[tokio::test]
    async fn all_inner_texts_cancellation_owns_no_persistent_browser_work() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div>ready</div>"));
        let service = CdpLocatorService::new(client.clone());
        let context = OperationContext::new(
            CallId::new(45),
            ObjectId::new(3),
            AllInnerTexts::METHOD,
            None,
        );

        // Act
        let result = <CdpLocatorService as CommandCancellation<AllInnerTexts>>::cancel(
            &service,
            context,
            CancellationReason::Caller,
        )
        .await;

        // Assert
        assert_eq!(result, Ok(()));
        assert!(client.method_names().is_empty());
    }

    #[tokio::test]
    async fn all_inner_texts_preserves_invalid_target_and_malformed_failures() {
        // Arrange
        let invalid_target_client = Arc::new(FakeCdpClient::from_html("<svg></svg>"));
        invalid_target_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "invalid-target",
                "message": "Node is not an HTMLElement"
            }}}),
        ]);
        let invalid_target_service = CdpLocatorService::new(invalid_target_client);
        let malformed_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "done",
                "values": ["valid", 7]
            }}}),
        ]);
        let malformed_service = CdpLocatorService::new(malformed_client);

        // Act
        let invalid_target = invalid_target_service
            .call(AllInnerTextsParams::new(selector("svg")))
            .await
            .expect_err("a non-HTMLElement should fail");
        let malformed = malformed_service
            .call(AllInnerTextsParams::new(selector("div")))
            .await
            .expect_err("a non-string collection item should fail");

        // Assert
        assert_eq!(invalid_target.kind(), FailureKind::Backend);
        assert_eq!(
            invalid_target.details(),
            Some(&LocatorErrorDetails::InvalidTarget(
                "Node is not an HTMLElement".to_string()
            ))
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
        assert!(matches!(
            malformed.details(),
            Some(LocatorErrorDetails::InvalidBackendResponse(_))
        ));
    }

    #[tokio::test]
    async fn all_inner_texts_rejects_item_count_item_size_and_aggregate_size() {
        // Arrange
        let too_many_client = Arc::new(FakeCdpClient::from_html(""));
        too_many_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "done",
                "values": vec![""; super::MAX_ALL_INNER_TEXTS_ITEMS + 1]
            }}}),
        ]);
        let too_many_service = CdpLocatorService::new(too_many_client);
        let large_item_client = Arc::new(FakeCdpClient::from_html(""));
        large_item_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "done",
                "values": ["x".repeat(super::MAX_ALL_INNER_TEXTS_ITEM_BYTES + 1)]
            }}}),
        ]);
        let large_item_service = CdpLocatorService::new(large_item_client);
        let aggregate_client = Arc::new(FakeCdpClient::from_html(""));
        aggregate_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "done",
                "values": vec!["x".repeat(900_000); 5]
            }}}),
        ]);
        let aggregate_service = CdpLocatorService::new(aggregate_client);

        // Act
        let too_many = too_many_service
            .call(AllInnerTextsParams::new(selector("div")))
            .await
            .expect_err("too many values should fail");
        let large_item = large_item_service
            .call(AllInnerTextsParams::new(selector("div")))
            .await
            .expect_err("an oversized item should fail");
        let aggregate = aggregate_service
            .call(AllInnerTextsParams::new(selector("div")))
            .await
            .expect_err("an oversized aggregate should fail");

        // Assert
        assert_eq!(too_many.kind(), FailureKind::ResourceExhausted);
        assert_eq!(large_item.kind(), FailureKind::ResourceExhausted);
        assert_eq!(aggregate.kind(), FailureKind::ResourceExhausted);
    }

    #[tokio::test]
    async fn all_text_contents_returns_an_ordered_non_strict_snapshot() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<div>first</div><div></div><div>third</div>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "done",
                "values": ["first", "", "third"]
            }}}),
            json!({"result": {"value": {"status": "done", "values": []}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let ordered = service
            .call(AllTextContentsParams::new(selector("div")))
            .await;
        let empty = service
            .call(AllTextContentsParams::new(selector(".missing")))
            .await;

        // Assert
        assert_eq!(ordered, Ok(vec!["first".into(), "".into(), "third".into()]));
        assert_eq!(empty, Ok(Vec::new()));
        assert_eq!(client.method_names().len(), 3);
        assert_eq!(service.selector_runtime_metrics().snapshot_evaluations, 0);
        assert_eq!(
            service
                .selector_runtime_metrics()
                .all_text_contents_evaluations,
            2
        );
        let calls = client.calls();
        let expression = calls[1].1[0]
            .as_str()
            .expect("snapshot expression should be a string");
        assert!(expression.contains("matches.map(element => element.textContent || '')"));
        assert!(!expression.contains("matches.length > 1"));
    }

    #[tokio::test]
    async fn all_text_contents_reinstalls_runtime_once_without_polling_for_matches() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div>ready</div>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "runtime-missing"}}}),
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "values": ["ready"]}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let result = service
            .call(AllTextContentsParams::new(selector("div")))
            .await;

        // Assert
        assert_eq!(result, Ok(vec!["ready".into()]));
        assert_eq!(client.method_names().len(), 4);
        let metrics = service.selector_runtime_metrics();
        assert_eq!(metrics.installations, 2);
        assert_eq!(metrics.all_text_contents_evaluations, 2);
        assert_eq!(metrics.missing_after_navigation, 1);
    }

    #[tokio::test]
    async fn all_text_contents_rejects_item_count_item_size_and_aggregate_size() {
        // Arrange
        let too_many_client = Arc::new(FakeCdpClient::from_html(""));
        too_many_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "done",
                "values": vec![""; super::MAX_ALL_TEXT_CONTENTS_ITEMS + 1]
            }}}),
        ]);
        let too_many_service = CdpLocatorService::new(too_many_client);
        let large_item_client = Arc::new(FakeCdpClient::from_html(""));
        large_item_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "done",
                "values": ["x".repeat(super::MAX_ALL_TEXT_CONTENTS_ITEM_BYTES + 1)]
            }}}),
        ]);
        let large_item_service = CdpLocatorService::new(large_item_client);
        let aggregate_client = Arc::new(FakeCdpClient::from_html(""));
        aggregate_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "done",
                "values": vec!["x".repeat(900_000); 5]
            }}}),
        ]);
        let aggregate_service = CdpLocatorService::new(aggregate_client);
        let malformed_client = Arc::new(FakeCdpClient::from_html(""));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "done",
                "values": ["valid", 7]
            }}}),
        ]);
        let malformed_service = CdpLocatorService::new(malformed_client);

        // Act
        let too_many = too_many_service
            .call(AllTextContentsParams::new(selector("div")))
            .await
            .expect_err("too many values should fail");
        let large_item = large_item_service
            .call(AllTextContentsParams::new(selector("div")))
            .await
            .expect_err("an oversized item should fail");
        let aggregate = aggregate_service
            .call(AllTextContentsParams::new(selector("div")))
            .await
            .expect_err("an oversized aggregate should fail");
        let malformed = malformed_service
            .call(AllTextContentsParams::new(selector("div")))
            .await
            .expect_err("a non-string collection item should fail");

        // Assert
        assert_eq!(too_many.kind(), FailureKind::ResourceExhausted);
        assert_eq!(large_item.kind(), FailureKind::ResourceExhausted);
        assert_eq!(aggregate.kind(), FailureKind::ResourceExhausted);
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
        assert!(matches!(
            malformed.details(),
            Some(LocatorErrorDetails::InvalidBackendResponse(_))
        ));
    }

    #[tokio::test]
    async fn visibility_is_a_strict_immediate_snapshot_and_missing_is_false() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<button>Visible</button><button hidden>Hidden</button>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "visible": true}}}),
            json!({"result": {"value": {"status": "done", "visible": false}}}),
            json!({"result": {"value": {"status": "missing"}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        let visible = service
            .call(IsVisibleParams::new(selector("button:first-child")))
            .await;
        let hidden = service
            .call(IsVisibleParams::new(selector("button[hidden]")))
            .await;
        let missing = service
            .call(IsVisibleParams::new(selector(".missing")))
            .await;

        // Assert
        assert_eq!(visible, Ok(true));
        assert_eq!(hidden, Ok(false));
        assert_eq!(missing, Ok(false));
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.evaluate",
            ]
        );
        assert!(
            client.calls()[1].1[0]
                .as_str()
                .unwrap_or_default()
                .contains("__pw.visible(matches[0])")
        );
        assert_eq!(
            service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics {
                installations: 1,
                actionability_evaluations: 0,
                hover_evaluations: 0,
                focus_evaluations: 0,
                snapshot_evaluations: 3,
                state_evaluations: 0,
                content_evaluations: 0,
                attribute_evaluations: 0,
                input_value_evaluations: 0,
                all_inner_texts_evaluations: 0,
                all_text_contents_evaluations: 0,
                wait_evaluations: 0,
                missing_after_navigation: 0,
            }
        );
    }

    #[tokio::test]
    async fn visibility_preserves_strictness_and_rejects_malformed_boolean_results() {
        // Arrange
        let strict_client = Arc::new(FakeCdpClient::from_html(
            "<button>A</button><button>B</button>",
        ));
        strict_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let strict_service = CdpLocatorService::new(strict_client);
        let malformed_client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "visible": "yes"}}}),
        ]);
        let malformed_service = CdpLocatorService::new(malformed_client);

        // Act
        let strict = strict_service
            .call(IsVisibleParams::new(selector("button")))
            .await
            .expect_err("multiple matches should preserve strictness");
        let malformed = malformed_service
            .call(IsVisibleParams::new(selector("button")))
            .await
            .expect_err("non-boolean visibility should fail");

        // Assert
        assert_eq!(strict.kind(), FailureKind::Backend);
        assert_eq!(
            strict.details(),
            Some(&LocatorErrorDetails::StrictViolation {
                selector: "button".to_string(),
                matches: 2,
            })
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
    }

    #[tokio::test]
    async fn element_state_getters_wait_for_a_target_then_read_current_state() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<button disabled>Submit</button><input><input type=checkbox checked>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "retry"}}}),
            json!({"result": {"value": {"status": "done", "value": false}}}),
            json!({"result": {"value": {"status": "done", "value": true}}}),
            json!({"result": {"value": {"status": "done", "value": true}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);

        // Act
        let enabled = service.call(IsEnabledParams::new(selector("button"))).await;
        let editable = service
            .call(IsEditableParams::new(selector("input:not([type])")))
            .await;
        let checked = service
            .call(IsCheckedParams::new(selector("input[type=checkbox]")))
            .await;

        // Assert
        assert_eq!(enabled, Ok(false));
        assert_eq!(editable, Ok(true));
        assert_eq!(checked, Ok(true));
        assert_eq!(
            service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics {
                installations: 1,
                actionability_evaluations: 0,
                hover_evaluations: 0,
                focus_evaluations: 0,
                snapshot_evaluations: 0,
                state_evaluations: 4,
                content_evaluations: 0,
                attribute_evaluations: 0,
                input_value_evaluations: 0,
                all_inner_texts_evaluations: 0,
                all_text_contents_evaluations: 0,
                wait_evaluations: 0,
                missing_after_navigation: 0,
            }
        );
        assert!(
            client.calls()[1].1[0]
                .as_str()
                .unwrap_or_default()
                .contains("__pw.elementState(matches[0], \"enabled\")")
        );
    }

    #[tokio::test]
    async fn element_state_getters_preserve_strict_invalid_target_and_boolean_failures() {
        // Arrange
        let strict_client = Arc::new(FakeCdpClient::from_html(
            "<button>A</button><button>B</button>",
        ));
        strict_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let strict_service = CdpLocatorService::new(strict_client);
        let target_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        target_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "invalid-target", "message": "Not a checkbox or radio button"}}}),
        ]);
        let target_service = CdpLocatorService::new(target_client);
        let malformed_client = Arc::new(FakeCdpClient::from_html("<button></button>"));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": "yes"}}}),
        ]);
        let malformed_service = CdpLocatorService::new(malformed_client);

        // Act
        let strict = strict_service
            .call(IsEnabledParams::new(selector("button")))
            .await
            .expect_err("multiple targets should fail strictness");
        let invalid_target = target_service
            .call(IsCheckedParams::new(selector("div")))
            .await
            .expect_err("non-checkable target should fail");
        let malformed = malformed_service
            .call(IsEnabledParams::new(selector("button")))
            .await
            .expect_err("non-boolean state should fail");

        // Assert
        assert_eq!(strict.kind(), FailureKind::Backend);
        assert_eq!(
            strict.details(),
            Some(&LocatorErrorDetails::StrictViolation {
                selector: "button".to_string(),
                matches: 2,
            })
        );
        assert_eq!(invalid_target.kind(), FailureKind::Backend);
        assert_eq!(
            invalid_target.details(),
            Some(&LocatorErrorDetails::InvalidTarget(
                "Not a checkbox or radio button".to_string()
            ))
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
    }

    #[tokio::test]
    async fn selector_runtime_exposes_shared_state_and_retarget_primitives() {
        // Arrange
        let scripts = [
            element_state_script(
                &Selector::css("input").expect("valid CSS"),
                LocatorElementState::Checked,
            )
            .expect("checked script should compile"),
            element_state_script(
                &Selector::css("input").expect("valid CSS"),
                LocatorElementState::Editable,
            )
            .expect("editable script should compile"),
            element_state_script(
                &Selector::css("input").expect("valid CSS"),
                LocatorElementState::Enabled,
            )
            .expect("enabled script should compile"),
        ];

        // Act
        let state_calls = scripts
            .iter()
            .map(|script| {
                script.contains(super::SELECTOR_RUNTIME_SYMBOL)
                    && script.contains(&format!(
                        "__pw.version !== {}",
                        super::SELECTOR_RUNTIME_VERSION
                    ))
                    && script.contains("__pw.elementState")
            })
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(state_calls, vec![true, true, true]);
        assert!(
            super::SELECTOR_RUNTIME
                .contains(&format!("version: {}", super::SELECTOR_RUNTIME_VERSION))
        );
        assert!(super::SELECTOR_RUNTIME.contains("elementState"));
        assert!(super::SELECTOR_RUNTIME.contains("retarget"));
    }

    #[tokio::test]
    async fn text_content_reports_a_closed_native_session_as_target_closed() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_errors(["Session with given id not found.".to_owned()]);
        let service = CdpLocatorService::new(client.clone());
        // Act
        let error = service
            .call(TextContentParams::new(selector("#missing")))
            .await
            .unwrap_err();
        // Assert
        assert_eq!(error.kind(), FailureKind::TargetClosed);
        assert_eq!(client.calls().len(), 1);
    }

    #[tokio::test]
    async fn text_content_does_not_misclassify_navigation_context_loss_as_page_close() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        client.set_evaluate_errors(["Cannot find context with specified id".to_owned()]);
        let service = CdpLocatorService::new(client.clone());
        // Act
        let error = service
            .call(TextContentParams::new(selector("#missing")))
            .await
            .unwrap_err();
        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(client.calls().len(), 1);
    }

    #[tokio::test]
    async fn text_content_waits_then_preserves_exact_text_and_null() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "retry"}}}),
            json!({"result": {"value": {"status": "done", "value": "  Ready\nnow  "}}}),
            json!({"result": {"value": {"status": "done", "value": null}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);

        // Act
        let present = service
            .call(TextContentParams::new(selector("#status")))
            .await;
        let absent = service
            .call(TextContentParams::new(selector("#nullable")))
            .await;

        // Assert
        assert_eq!(
            present
                .expect("text-content query should succeed")
                .as_deref(),
            Some("  Ready\nnow  ")
        );
        assert_eq!(absent, Ok(None));
        assert_eq!(
            service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics {
                installations: 1,
                actionability_evaluations: 0,
                hover_evaluations: 0,
                focus_evaluations: 0,
                snapshot_evaluations: 0,
                state_evaluations: 0,
                content_evaluations: 3,
                attribute_evaluations: 0,
                input_value_evaluations: 0,
                all_inner_texts_evaluations: 0,
                all_text_contents_evaluations: 0,
                wait_evaluations: 0,
                missing_after_navigation: 0,
            }
        );
        assert!(
            client.calls()[1].1[0]
                .as_str()
                .unwrap_or_default()
                .contains("value: element.textContent")
        );
    }

    #[tokio::test]
    async fn text_content_preserves_strict_malformed_and_size_failures() {
        // Arrange
        let strict_client = Arc::new(FakeCdpClient::from_html("<div></div><div></div>"));
        strict_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let malformed_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": true}}}),
        ]);
        let oversized_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        oversized_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": "x".repeat(super::MAX_TEXT_CONTENT_BYTES + 1)}}}),
        ]);

        // Act
        let strict = CdpLocatorService::new(strict_client)
            .call(TextContentParams::new(selector("div")))
            .await
            .expect_err("multiple matches should fail strictness");
        let malformed = CdpLocatorService::new(malformed_client)
            .call(TextContentParams::new(selector("div")))
            .await
            .expect_err("non-string text content should fail");
        let oversized = CdpLocatorService::new(oversized_client)
            .call(TextContentParams::new(selector("div")))
            .await
            .expect_err("oversized text content should fail");

        // Assert
        assert_eq!(
            strict.details(),
            Some(&LocatorErrorDetails::StrictViolation {
                selector: "div".to_string(),
                matches: 2,
            })
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
        assert_eq!(oversized.kind(), FailureKind::ResourceExhausted);
    }

    #[tokio::test]
    async fn get_attribute_waits_then_preserves_exact_value_null_and_element_namespace() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<svg></svg>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "retry"}}}),
            json!({"result": {"value": {"status": "done", "value": "ready & exact"}}}),
            json!({"result": {"value": {"status": "done", "value": null}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);

        // Act
        let present = service
            .call(GetAttributeParams::new(
                selector("#graphic"),
                "data-\"state",
            ))
            .await;
        let absent = service
            .call(GetAttributeParams::new(selector("#graphic"), "missing"))
            .await;

        // Assert
        assert_eq!(
            present.as_ref().map(|value| value.as_deref()),
            Ok(Some("ready & exact"))
        );
        assert_eq!(absent, Ok(None));
        assert_eq!(service.selector_runtime_metrics().attribute_evaluations, 3);
        let calls = client.calls();
        let script = calls[1].1[0].as_str().unwrap_or_default();
        assert!(script.contains("element.getAttribute(\"data-\\\"state\")"));
        assert!(!script.contains("Node is not an HTMLElement"));
    }

    #[tokio::test]
    async fn get_attribute_preserves_strict_backend_malformed_and_resource_failures() {
        // Arrange
        let strict_client = Arc::new(FakeCdpClient::from_html("<div></div><div></div>"));
        strict_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let backend_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        backend_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "backend", "message": "DOM failure"}}}),
        ]);
        let malformed_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": true}}}),
        ]);
        let oversized_result_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        oversized_result_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": "x".repeat(super::MAX_ATTRIBUTE_RESULT_BYTES + 1)}}}),
        ]);
        let oversized_name_client = Arc::new(FakeCdpClient::from_html("<div></div>"));

        // Act
        let strict = CdpLocatorService::new(strict_client)
            .call(GetAttributeParams::new(selector("div"), "id"))
            .await
            .expect_err("multiple matches should fail strictness");
        let backend = CdpLocatorService::new(backend_client)
            .call(GetAttributeParams::new(selector("div"), "id"))
            .await
            .expect_err("browser getAttribute error should remain typed");
        let malformed = CdpLocatorService::new(malformed_client)
            .call(GetAttributeParams::new(selector("div"), "id"))
            .await
            .expect_err("non-null non-string result should fail");
        let oversized_result = CdpLocatorService::new(oversized_result_client)
            .call(GetAttributeParams::new(selector("div"), "id"))
            .await
            .expect_err("oversized attribute value should fail");
        let oversized_name = CdpLocatorService::new(oversized_name_client.clone())
            .call(GetAttributeParams::new(
                selector("div"),
                "n".repeat(super::MAX_ATTRIBUTE_NAME_BYTES + 1),
            ))
            .await
            .expect_err("oversized attribute name should fail before CDP");

        // Assert
        assert_eq!(
            strict.details(),
            Some(&LocatorErrorDetails::StrictViolation {
                selector: "div".to_string(),
                matches: 2,
            })
        );
        assert_eq!(backend.kind(), FailureKind::Backend);
        assert_eq!(
            backend.details(),
            Some(&LocatorErrorDetails::Backend("DOM failure".to_string()))
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
        assert_eq!(oversized_result.kind(), FailureKind::ResourceExhausted);
        assert_eq!(oversized_name.kind(), FailureKind::ResourceExhausted);
        assert!(oversized_name_client.method_names().is_empty());
    }

    #[tokio::test]
    async fn inner_html_waits_then_returns_exact_serialized_markup_for_any_element() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<svg></svg>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "retry"}}}),
            json!({"result": {"value": {"status": "done", "value": "<text>SVG &amp; text</text>"}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);

        // Act
        let markup = service
            .call(InnerHtmlParams::new(selector("#graphic")))
            .await;

        // Assert
        assert_eq!(markup.as_deref(), Ok("<text>SVG &amp; text</text>"));
        assert_eq!(service.selector_runtime_metrics().content_evaluations, 2);
        let calls = client.calls();
        let script = calls[1].1[0].as_str().unwrap_or_default();
        assert!(script.contains("element.innerHTML"));
        assert!(!script.contains("Node is not an HTMLElement"));
    }

    #[tokio::test]
    async fn inner_html_preserves_strict_malformed_and_size_failures() {
        // Arrange
        let strict_client = Arc::new(FakeCdpClient::from_html("<div></div><div></div>"));
        strict_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let malformed_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": null}}}),
        ]);
        let oversized_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        oversized_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": "x".repeat(super::MAX_INNER_HTML_BYTES + 1)}}}),
        ]);

        // Act
        let strict = CdpLocatorService::new(strict_client)
            .call(InnerHtmlParams::new(selector("div")))
            .await
            .expect_err("multiple matches should fail strictness");
        let malformed = CdpLocatorService::new(malformed_client)
            .call(InnerHtmlParams::new(selector("div")))
            .await
            .expect_err("null inner HTML should fail");
        let oversized = CdpLocatorService::new(oversized_client)
            .call(InnerHtmlParams::new(selector("div")))
            .await
            .expect_err("oversized inner HTML should fail");

        // Assert
        assert_eq!(
            strict.details(),
            Some(&LocatorErrorDetails::StrictViolation {
                selector: "div".to_string(),
                matches: 2,
            })
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
        assert_eq!(oversized.kind(), FailureKind::ResourceExhausted);
    }

    #[tokio::test]
    async fn inner_text_waits_then_returns_rendered_html_text() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "retry"}}}),
            json!({"result": {"value": {"status": "done", "value": "Visible text"}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);

        // Act
        let rendered = service
            .call(InnerTextParams::new(selector("#status")))
            .await;

        // Assert
        assert_eq!(rendered.as_deref(), Ok("Visible text"));
        assert_eq!(service.selector_runtime_metrics().content_evaluations, 2);
        let calls = client.calls();
        let script = calls[1].1[0].as_str().unwrap_or_default();
        assert!(script.contains("element.innerText"));
        assert!(script.contains("http://www.w3.org/1999/xhtml"));
        assert!(script.contains("Node is not an HTMLElement"));
    }

    #[tokio::test]
    async fn inner_text_preserves_strict_invalid_target_malformed_and_size_failures() {
        // Arrange
        let strict_client = Arc::new(FakeCdpClient::from_html("<div></div><div></div>"));
        strict_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let target_client = Arc::new(FakeCdpClient::from_html("<svg></svg>"));
        target_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "invalid-target", "message": "Node is not an HTMLElement"}}}),
        ]);
        let malformed_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": null}}}),
        ]);
        let oversized_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        oversized_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": "x".repeat(super::MAX_INNER_TEXT_BYTES + 1)}}}),
        ]);

        // Act
        let strict = CdpLocatorService::new(strict_client)
            .call(InnerTextParams::new(selector("div")))
            .await
            .expect_err("multiple matches should fail strictness");
        let invalid_target = CdpLocatorService::new(target_client)
            .call(InnerTextParams::new(selector("svg")))
            .await
            .expect_err("SVG target should not expose HTMLElement.innerText");
        let malformed = CdpLocatorService::new(malformed_client)
            .call(InnerTextParams::new(selector("div")))
            .await
            .expect_err("null inner text should fail");
        let oversized = CdpLocatorService::new(oversized_client)
            .call(InnerTextParams::new(selector("div")))
            .await
            .expect_err("oversized inner text should fail");

        // Assert
        assert_eq!(
            strict.details(),
            Some(&LocatorErrorDetails::StrictViolation {
                selector: "div".to_string(),
                matches: 2,
            })
        );
        assert_eq!(
            invalid_target.details(),
            Some(&LocatorErrorDetails::InvalidTarget(
                "Node is not an HTMLElement".to_string()
            ))
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
        assert_eq!(oversized.kind(), FailureKind::ResourceExhausted);
    }

    #[tokio::test]
    async fn input_value_waits_then_returns_exact_value_through_shared_retargeting() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(
            "<label>Account <input value='alice@example.test'></label>",
        ));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "retry"}}}),
            json!({"result": {"value": {"status": "done", "value": "alice@example.test"}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);

        // Act
        let value = service
            .call(InputValueParams::new(Selector::label("Account", true)))
            .await;

        // Assert
        assert_eq!(value.as_deref(), Ok("alice@example.test"));
        assert_eq!(
            service.selector_runtime_metrics().input_value_evaluations,
            2
        );
        let calls = client.calls();
        let script = calls[1].1[0].as_str().unwrap_or_default();
        assert!(script.contains("__pw.retarget(matches[0])"));
        assert!(script.contains("'INPUT', 'TEXTAREA', 'SELECT'"));
        assert!(script.contains("value: element.value"));
    }

    #[tokio::test]
    async fn input_value_preserves_strict_invalid_target_malformed_and_size_failures() {
        // Arrange
        let strict_client = Arc::new(FakeCdpClient::from_html("<input><input>"));
        strict_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let target_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        target_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "invalid-target", "message": "Node is not an <input>, <textarea> or <select> element"}}}),
        ]);
        let malformed_client = Arc::new(FakeCdpClient::from_html("<input>"));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": null}}}),
        ]);
        let oversized_client = Arc::new(FakeCdpClient::from_html("<textarea></textarea>"));
        oversized_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "value": "x".repeat(super::MAX_INPUT_VALUE_BYTES + 1)}}}),
        ]);

        // Act
        let strict = CdpLocatorService::new(strict_client)
            .call(InputValueParams::new(selector("input")))
            .await
            .expect_err("multiple controls should fail strictness");
        let invalid_target = CdpLocatorService::new(target_client)
            .call(InputValueParams::new(selector("div")))
            .await
            .expect_err("non-control target should fail");
        let malformed = CdpLocatorService::new(malformed_client)
            .call(InputValueParams::new(selector("input")))
            .await
            .expect_err("non-string value should fail");
        let oversized = CdpLocatorService::new(oversized_client)
            .call(InputValueParams::new(selector("textarea")))
            .await
            .expect_err("oversized value should fail");

        // Assert
        assert_eq!(
            strict.details(),
            Some(&LocatorErrorDetails::StrictViolation {
                selector: "input".to_string(),
                matches: 2,
            })
        );
        assert_eq!(
            invalid_target.details(),
            Some(&LocatorErrorDetails::InvalidTarget(
                "Node is not an <input>, <textarea> or <select> element".to_string()
            ))
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
        assert_eq!(oversized.kind(), FailureKind::ResourceExhausted);
    }

    #[tokio::test]
    async fn wait_for_retries_without_retaining_a_handle_and_supports_every_state() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "retry"}}}),
            json!({"result": {"value": {"status": "done"}}}),
            json!({"result": {"value": {"status": "done"}}}),
            json!({"result": {"value": {"status": "done"}}}),
            json!({"result": {"value": {"status": "done"}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);
        let states = [
            WaitForState::Visible,
            WaitForState::Attached,
            WaitForState::Detached,
            WaitForState::Hidden,
        ];

        // Act
        let mut results = Vec::new();
        for state in states {
            results.push(
                service
                    .call(WaitForParams::new(
                        selector("div"),
                        WaitForOptions::new().with_state(state),
                    ))
                    .await,
            );
        }

        // Assert
        assert_eq!(results, vec![Ok(()), Ok(()), Ok(()), Ok(())]);
        assert_eq!(
            service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics {
                installations: 1,
                actionability_evaluations: 0,
                hover_evaluations: 0,
                focus_evaluations: 0,
                snapshot_evaluations: 0,
                state_evaluations: 0,
                content_evaluations: 0,
                attribute_evaluations: 0,
                input_value_evaluations: 0,
                all_inner_texts_evaluations: 0,
                all_text_contents_evaluations: 0,
                wait_evaluations: 5,
                missing_after_navigation: 0,
            }
        );
        let scripts = client.calls();
        assert!(
            scripts[1].1[0]
                .as_str()
                .unwrap_or_default()
                .contains("[\"visible\"]")
        );
        assert!(
            scripts[3].1[0]
                .as_str()
                .unwrap_or_default()
                .contains("[\"attached\"]")
        );
        assert!(
            scripts[4].1[0]
                .as_str()
                .unwrap_or_default()
                .contains("[\"detached\"]")
        );
        assert!(
            scripts[5].1[0]
                .as_str()
                .unwrap_or_default()
                .contains("[\"hidden\"]")
        );
    }

    #[tokio::test]
    async fn wait_for_preserves_strict_and_malformed_backend_failures() {
        // Arrange
        let strict_client = Arc::new(FakeCdpClient::from_html("<div></div><div></div>"));
        strict_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let malformed_client = Arc::new(FakeCdpClient::from_html("<div></div>"));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "surprise"}}}),
        ]);

        // Act
        let strict = CdpLocatorService::new(strict_client)
            .call(WaitForParams::new(selector("div"), WaitForOptions::new()))
            .await
            .expect_err("multiple matches should fail strictness");
        let malformed = CdpLocatorService::new(malformed_client)
            .call(WaitForParams::new(selector("div"), WaitForOptions::new()))
            .await
            .expect_err("unknown status should fail the protocol boundary");

        // Assert
        assert_eq!(
            strict.details(),
            Some(&LocatorErrorDetails::StrictViolation {
                selector: "div".to_string(),
                matches: 2,
            })
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
    }

    #[test]
    fn wait_for_script_maps_zero_match_to_hidden_or_detached_success() {
        // Arrange
        let selector = Selector::css(".status").expect("valid CSS");

        // Act
        let hidden =
            wait_for_script(&selector, WaitForState::Hidden).expect("hidden wait should compile");
        let attached = wait_for_script(&selector, WaitForState::Attached)
            .expect("attached wait should compile");

        // Assert
        assert!(hidden.contains("const attached = !!element && element.isConnected"));
        assert!(hidden.contains("hidden: !visible"));
        assert!(hidden.contains("[\"hidden\"]"));
        assert!(attached.contains("[\"attached\"]"));
    }

    #[tokio::test(start_paused = true)]
    async fn runtime_deadline_bounds_element_state_wait_for_target() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let waiting = json!({"result": {"value": {"status": "retry"}}});
        client.set_evaluate_responses(
            std::iter::once(runtime_installed()).chain(std::iter::repeat_n(waiting, 10)),
        );
        let service =
            CdpLocatorService::with_retry_delay(client.clone(), Duration::from_millis(50));
        let dispatcher = ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(3);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("page should register");
        let command = Command::<IsEnabled>::new(
            CallId::new(4),
            page_id,
            CallMetadata::new(Some(Duration::from_millis(125)), None),
            IsEnabledParams::new(selector("button")),
        );

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch should be infallible");

        // Assert
        assert_eq!(
            result
                .into_outcome()
                .expect_err("missing state target should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
        assert!(
            client
                .method_names()
                .iter()
                .all(|method| method == "Runtime.evaluate")
        );
    }

    #[tokio::test(start_paused = true)]
    async fn runtime_deadline_bounds_locator_wait_for() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let waiting = json!({"result": {"value": {"status": "retry"}}});
        client.set_evaluate_responses(
            std::iter::once(runtime_installed()).chain(std::iter::repeat_n(waiting, 10)),
        );
        let service = CdpLocatorService::with_retry_delay(client, Duration::from_millis(50));
        let dispatcher = ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(3);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("page should register");
        let command = Command::<WaitFor>::new(
            CallId::new(5),
            page_id,
            CallMetadata::new(Some(Duration::from_millis(125)), None),
            WaitForParams::new(selector(".missing"), WaitForOptions::new()),
        );

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch should be infallible");

        // Assert
        assert_eq!(
            result
                .into_outcome()
                .expect_err("missing locator should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[tokio::test(start_paused = true)]
    async fn runtime_deadline_bounds_text_content_target_wait() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let waiting = json!({"result": {"value": {"status": "retry"}}});
        client.set_evaluate_responses(
            std::iter::once(runtime_installed()).chain(std::iter::repeat_n(waiting, 10)),
        );
        let service = CdpLocatorService::with_retry_delay(client, Duration::from_millis(50));
        let dispatcher = ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(3);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("page should register");
        let command = Command::<TextContent>::new(
            CallId::new(6),
            page_id,
            CallMetadata::new(Some(Duration::from_millis(125)), None),
            TextContentParams::new(selector(".missing")),
        );

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch should be infallible");

        // Assert
        assert_eq!(
            result
                .into_outcome()
                .expect_err("missing text-content target should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[tokio::test(start_paused = true)]
    async fn runtime_deadline_bounds_get_attribute_target_wait() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let waiting = json!({"result": {"value": {"status": "retry"}}});
        client.set_evaluate_responses(
            std::iter::once(runtime_installed()).chain(std::iter::repeat_n(waiting, 10)),
        );
        let service = CdpLocatorService::with_retry_delay(client, Duration::from_millis(50));
        let dispatcher = ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(3);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("page should register");
        let command = Command::<GetAttribute>::new(
            CallId::new(9),
            page_id,
            CallMetadata::new(Some(Duration::from_millis(125)), None),
            GetAttributeParams::new(selector(".missing"), "data-state"),
        );

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch should be infallible");

        // Assert
        assert_eq!(
            result
                .into_outcome()
                .expect_err("missing get-attribute target should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[tokio::test(start_paused = true)]
    async fn runtime_deadline_bounds_inner_html_target_wait() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let waiting = json!({"result": {"value": {"status": "retry"}}});
        client.set_evaluate_responses(
            std::iter::once(runtime_installed()).chain(std::iter::repeat_n(waiting, 10)),
        );
        let service = CdpLocatorService::with_retry_delay(client, Duration::from_millis(50));
        let dispatcher = ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(3);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("page should register");
        let command = Command::<InnerHtml>::new(
            CallId::new(8),
            page_id,
            CallMetadata::new(Some(Duration::from_millis(125)), None),
            InnerHtmlParams::new(selector(".missing")),
        );

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch should be infallible");

        // Assert
        assert_eq!(
            result
                .into_outcome()
                .expect_err("missing inner-HTML target should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[tokio::test(start_paused = true)]
    async fn runtime_deadline_bounds_inner_text_target_wait() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let waiting = json!({"result": {"value": {"status": "retry"}}});
        client.set_evaluate_responses(
            std::iter::once(runtime_installed()).chain(std::iter::repeat_n(waiting, 10)),
        );
        let service = CdpLocatorService::with_retry_delay(client, Duration::from_millis(50));
        let dispatcher = ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(3);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("page should register");
        let command = Command::<InnerText>::new(
            CallId::new(7),
            page_id,
            CallMetadata::new(Some(Duration::from_millis(125)), None),
            InnerTextParams::new(selector(".missing")),
        );

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch should be infallible");

        // Assert
        assert_eq!(
            result
                .into_outcome()
                .expect_err("missing inner-text target should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[tokio::test(start_paused = true)]
    async fn runtime_deadline_bounds_input_value_target_wait() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let waiting = json!({"result": {"value": {"status": "retry"}}});
        client.set_evaluate_responses(
            std::iter::once(runtime_installed()).chain(std::iter::repeat_n(waiting, 10)),
        );
        let service = CdpLocatorService::with_retry_delay(client, Duration::from_millis(50));
        let dispatcher = ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(3);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("page should register");
        let command = Command::<InputValue>::new(
            CallId::new(8),
            page_id,
            CallMetadata::new(Some(Duration::from_millis(125)), None),
            InputValueParams::new(selector(".missing")),
        );

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch should be infallible");

        // Assert
        assert_eq!(
            result
                .into_outcome()
                .expect_err("missing input-value target should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[tokio::test]
    async fn strict_and_malformed_results_fail_without_retrying_or_clicking() {
        // Arrange
        let strict_client = Arc::new(FakeCdpClient::from_html(
            "<button>A</button><button>B</button>",
        ));
        strict_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "strict", "count": 2}}}),
        ]);
        let strict_service = CdpLocatorService::new(strict_client.clone());
        let malformed_client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        malformed_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"x": 1.0}}}),
        ]);
        let malformed_service = CdpLocatorService::new(malformed_client.clone());

        // Act
        let strict = strict_service
            .call(ClickParams::new(selector("button"), ClickOptions::new()))
            .await
            .expect_err("strict selector should fail");
        let malformed = malformed_service
            .call(ClickParams::new(selector("button"), ClickOptions::new()))
            .await
            .expect_err("malformed backend result should fail");

        // Assert
        assert_eq!(strict.kind(), FailureKind::Backend);
        assert_eq!(
            strict.details(),
            Some(&LocatorErrorDetails::StrictViolation {
                selector: "button".to_string(),
                matches: 2,
            })
        );
        assert_eq!(malformed.kind(), FailureKind::ProtocolViolation);
        assert_eq!(
            strict_client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
        assert_eq!(
            malformed_client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
    }

    #[tokio::test]
    async fn invalid_selector_status_is_a_typed_failure() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({
                "result": {"value": {"status": "invalid", "message": "SyntaxError"}}
            }),
        ]);
        let service = CdpLocatorService::new(client);

        // Act
        let error = service
            .call(ClickParams::new(selector("["), ClickOptions::new()))
            .await
            .expect_err("invalid selector should fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert_eq!(
            error.details(),
            Some(&LocatorErrorDetails::InvalidSelector(
                "SyntaxError".to_string()
            ))
        );
    }

    #[tokio::test(start_paused = true)]
    async fn runtime_deadline_bounds_an_actionability_retry_loop() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html(""));
        let missing = json!({"result": {"value": {"status": "missing"}}});
        client.set_evaluate_responses(
            std::iter::once(runtime_installed()).chain(std::iter::repeat_n(missing, 10)),
        );
        let service =
            CdpLocatorService::with_retry_delay(client.clone(), Duration::from_millis(50));
        let dispatcher = ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(3);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("page should register");
        let command = Command::<Click>::new(
            CallId::new(1),
            page_id,
            CallMetadata::new(Some(Duration::from_millis(125)), None),
            ClickParams::new(selector("button"), ClickOptions::new()),
        );

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch should be infallible");

        // Assert
        assert_eq!(
            result
                .into_outcome()
                .expect_err("missing locator should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
        assert!(
            client
                .method_names()
                .iter()
                .all(|method| method == "Runtime.evaluate")
        );
    }

    #[tokio::test(start_paused = true)]
    async fn runtime_deadline_bounds_a_fill_editability_retry_loop() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input readonly>"));
        let waiting = json!({"result": {"value": {"status": "noteditable"}}});
        client.set_evaluate_responses(
            std::iter::once(runtime_installed()).chain(std::iter::repeat_n(waiting, 10)),
        );
        let service =
            CdpLocatorService::with_retry_delay(client.clone(), Duration::from_millis(50));
        let dispatcher = ControlledDispatcher::with_object_kinds(
            service.clone(),
            service,
            TokioDeadlineScheduler,
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(3);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("page should register");
        let command = Command::<pwright_cap_locator::Fill>::new(
            CallId::new(2),
            page_id,
            CallMetadata::new(Some(Duration::from_millis(125)), None),
            FillParams::new(selector("input"), "new", FillOptions::new()),
        );

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch should be infallible");

        // Assert
        assert_eq!(
            result
                .into_outcome()
                .expect_err("readonly locator should time out")
                .kind(),
            FailureKind::Timeout
        );
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
        assert!(
            !client
                .method_names()
                .contains(&"Input.insertText".to_string())
        );
    }

    #[tokio::test]
    async fn selector_complexity_is_bounded_before_javascript_evaluation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        let service = CdpLocatorService::new(client.clone());
        let mut selector = Selector::css("button").expect("valid CSS");
        for index in 0..32 {
            selector = selector.nth(index);
        }

        // Act
        let error = service
            .call(ClickParams::new(selector, ClickOptions::new()))
            .await
            .expect_err("overly deep selector should fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(client.method_names().is_empty());
    }

    #[tokio::test]
    async fn concurrent_clones_install_one_runtime_and_keep_retry_scripts_compact() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "ready", "x": 1.0, "y": 2.0}}}),
            json!({"result": {"value": {"status": "ready", "x": 1.0, "y": 2.0}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());
        let clone = service.clone();
        let params = ClickParams::new(selector("button"), ClickOptions::new().with_trial(true));

        // Act
        let (first, second) = tokio::join!(service.call(params.clone()), clone.call(params));

        // Assert
        assert_eq!(first, Ok(()));
        assert_eq!(second, Ok(()));
        let calls = client.calls();
        assert_eq!(calls.len(), 3);
        assert!(
            calls[0].1[0]
                .as_str()
                .unwrap_or_default()
                .contains("validRoles")
        );
        assert!(
            calls[1..]
                .iter()
                .all(|(_, args)| !args[0].as_str().unwrap_or_default().contains("validRoles"))
        );
        assert_eq!(
            service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics {
                installations: 1,
                actionability_evaluations: 2,
                hover_evaluations: 0,
                focus_evaluations: 0,
                snapshot_evaluations: 0,
                state_evaluations: 0,
                content_evaluations: 0,
                attribute_evaluations: 0,
                input_value_evaluations: 0,
                all_inner_texts_evaluations: 0,
                all_text_contents_evaluations: 0,
                wait_evaluations: 0,
                missing_after_navigation: 0,
            }
        );
    }

    #[tokio::test]
    async fn missing_runtime_after_navigation_is_reinstalled_without_persistent_script() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        let ready = json!({"result": {"value": {"status": "ready", "x": 1.0, "y": 2.0}}});
        client.set_evaluate_responses([
            runtime_installed(),
            ready.clone(),
            json!({"result": {"value": {"status": "runtime-missing"}}}),
            runtime_installed(),
            ready,
        ]);
        let service = CdpLocatorService::new(client.clone());
        let params = ClickParams::new(selector("button"), ClickOptions::new().with_trial(true));

        // Act
        let before_navigation = service.call(params.clone()).await;
        let after_navigation = service.call(params).await;

        // Assert
        assert_eq!(before_navigation, Ok(()));
        assert_eq!(after_navigation, Ok(()));
        let calls = client.calls();
        assert_eq!(calls.len(), 5);
        assert_eq!(
            calls
                .iter()
                .filter(|(_, args)| args[0].as_str().unwrap_or_default().contains("validRoles"))
                .count(),
            2
        );
        assert!(
            client
                .method_names()
                .iter()
                .all(|method| method == "Runtime.evaluate")
        );
        assert_eq!(
            service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics {
                installations: 2,
                actionability_evaluations: 3,
                hover_evaluations: 0,
                focus_evaluations: 0,
                snapshot_evaluations: 0,
                state_evaluations: 0,
                content_evaluations: 0,
                attribute_evaluations: 0,
                input_value_evaluations: 0,
                all_inner_texts_evaluations: 0,
                all_text_contents_evaluations: 0,
                wait_evaluations: 0,
                missing_after_navigation: 1,
            }
        );
    }

    #[tokio::test]
    async fn malformed_runtime_installation_fails_before_actionability() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        client.set_evaluate_response(json!({"result": {"value": "not-a-boolean"}}));
        let service = CdpLocatorService::new(client.clone());

        // Act
        let error = service
            .call(ClickParams::new(
                selector("button"),
                ClickOptions::new().with_trial(true),
            ))
            .await
            .expect_err("malformed installation acknowledgement should fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ProtocolViolation);
        assert_eq!(client.method_names(), vec!["Runtime.evaluate"]);
        assert_eq!(
            service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics::default()
        );
    }

    #[test]
    fn semantic_descendant_selectors_compile_into_one_backend_owned_runtime() {
        // Arrange
        let selector = Selector::role(
            AriaRole::Dialog,
            RoleOptions::new().with_name("Settings", true),
        )
        .expect("valid dialog role")
        .within(
            Selector::role(
                AriaRole::Button,
                RoleOptions::new().with_name("Save", false),
            )
            .expect("valid button role"),
        );

        // Act
        let script = actionability_script(&selector, false)
            .expect("bounded semantic selector should compile");

        // Assert
        assert!(script.contains("__pw.role(document, \"dialog\""));
        assert!(script.contains("__pw.role(__root, \"button\""));
        assert!(script.contains("__pw.within"));
        assert!(script.contains(super::SELECTOR_RUNTIME_SYMBOL));
        assert!(!script.contains("validRoles"));
        assert!(script.len() < super::SELECTOR_RUNTIME.len());
    }

    #[test]
    fn legacy_click_enabled_check_uses_shared_retargeting_state() {
        // Arrange
        let selector = Selector::text("Click target", false);

        // Act
        let script =
            actionability_script(&selector, false).expect("click actionability should compile");

        // Assert
        assert!(script.contains("__pw.elementState(element, \"enabled\")"));
        assert!(script.contains("element.closest('button, [role=button], a, [role=link]')"));
        assert!(script.contains("hit !== target && !target.contains(hit)"));
        assert!(!script.contains("element.disabled"));
        assert!(!script.contains("element.getAttribute(\"aria-disabled\")"));
    }

    #[test]
    fn locator_v2_click_enabled_check_uses_shared_retargeting_state() {
        // Arrange
        let selector = Selector::text("Click target", false);

        // Act
        let script = click_v2_actionability_script(&selector, &ClickOptionsV2::new())
            .expect("locator v2 click actionability should compile");

        // Assert
        assert!(script.contains("__pw.elementState(element, \"enabled\")"));
        assert!(script.contains("Array.from(element.getClientRects()"));
        assert!(script.contains("const { left, top, right, bottom } = clientRect"));
        assert!(!script.contains("element.disabled"));
        assert!(!script.contains("element.getAttribute(\"aria-disabled\")"));
    }

    #[test]
    fn locator_v2_click_hit_check_accepts_button_and_role_button_ancestors() {
        // Arrange
        let selector = Selector::text("Click target", false);

        // Act
        let script = click_v2_actionability_script(&selector, &ClickOptionsV2::new())
            .expect("v2 click actionability");

        // Assert
        assert!(script.contains("element.closest('button, [role=button], a, [role=link]')"));
        assert!(script.contains("hit !== target && !target.contains(hit)"));
    }

    #[test]
    fn playwright_filter_variants_compile_as_base_preserving_relative_queries() {
        // Arrange
        let selector = Selector::css(".card")
            .expect("valid CSS")
            .filter_has(Selector::css(".ready").expect("valid CSS"))
            .filter_has_not(Selector::css("[data-excluded]").expect("valid CSS"))
            .filter_not_text("Blocked", false)
            .filter_visible(true);

        // Act
        let script = actionability_script(&selector, false)
            .expect("bounded Playwright-style filters should compile");

        // Assert
        assert!(script.contains("__pw.css(__candidate, \".ready\")"));
        assert!(script.contains("__pw.css(__candidate, \"[data-excluded]\")"));
        assert!(script.contains(".length > 0"));
        assert!(script.contains(".length === 0"));
        assert!(script.contains("!__pw.matchesText"));
        assert!(script.contains("__pw.visible(element) === true"));
    }

    #[test]
    fn playwright_regex_variants_compile_without_embedding_the_runtime() {
        // Arrange
        let selector = Selector::css(".card")
            .expect("valid CSS")
            .filter_text_regex(RegexMatcher::new("Release\\s+\\d+", "i").expect("valid regex"))
            .filter_not_text_regex(RegexMatcher::new("Blocked", "").expect("valid regex"))
            .within(
                Selector::role(
                    AriaRole::Button,
                    RoleOptions::new().with_name_regex(
                        RegexMatcher::new("^Deploy\\s+build", "i").expect("valid regex"),
                    ),
                )
                .expect("valid role regex"),
            )
            .or(Selector::label_regex(
                RegexMatcher::new("Release train", "i").expect("valid regex"),
            ))
            .or(Selector::text_regex(
                RegexMatcher::new("^Fallback$", "m").expect("valid regex"),
            ));

        // Act
        let script = actionability_script(&selector, false)
            .expect("bounded Playwright regex selectors should compile");

        // Assert
        assert!(script.contains("new RegExp(\"Release\\\\s+\\\\d+\", \"i\")"));
        assert!(script.contains("new RegExp(\"Blocked\", \"\")"));
        assert!(script.contains("\"kind\":\"regex\""));
        assert!(script.contains("__pw.labelRegex"));
        assert!(script.contains("__pw.textRegex"));
        assert!(!script.contains("validRoles"));
    }

    #[test]
    fn user_facing_attribute_and_test_id_variants_compile_to_runtime_calls() {
        // Arrange
        let attributes =
            TestIdAttributes::new("data-pw,data-testid").expect("valid test-id attributes");
        let selector =
            Selector::attribute_text(UserFacingAttribute::Placeholder, "USER  NAME", false)
                .or(Selector::attribute_regex(
                    UserFacingAttribute::Alt,
                    RegexMatcher::new("^Product Hero \\d+$", "i").expect("valid regex"),
                ))
                .or(Selector::attribute_text(
                    UserFacingAttribute::Title,
                    "Release Notes",
                    true,
                ))
                .or(Selector::test_id_with_attributes(
                    attributes.clone(),
                    "save",
                ))
                .or(Selector::test_id_regex_with_attributes(
                    attributes,
                    RegexMatcher::new("^custom-\\d+$", "").expect("valid regex"),
                ));

        // Act
        let script = actionability_script(&selector, false)
            .expect("bounded attribute selectors should compile");

        // Assert
        assert!(
            script.contains("__pw.attribute(document, \"placeholder\", \"USER  NAME\", false)")
        );
        assert!(
            script.contains(
                "__pw.attributeRegex(document, \"alt\", \"^Product Hero \\\\d+$\", \"i\")"
            )
        );
        assert!(script.contains("__pw.attribute(document, \"title\", \"Release Notes\", true)"));
        assert!(script.contains("__pw.testId(document, [\"data-pw\",\"data-testid\"], \"save\")"));
        assert!(script.contains(
            "__pw.testIdRegex(document, [\"data-pw\",\"data-testid\"], \"^custom-\\\\d+$\", \"\")"
        ));
        assert!(!script.contains("matchesAttribute ="));
    }

    #[tokio::test]
    async fn aggregate_selector_string_bytes_are_bounded_before_evaluation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        let service = CdpLocatorService::new(client.clone());
        let selector = Selector::text("x".repeat(65_537), false);

        // Act
        let error = service
            .call(ClickParams::new(selector, ClickOptions::new()))
            .await
            .expect_err("oversized local selector should fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(client.method_names().is_empty());
    }

    #[tokio::test]
    async fn regex_source_bytes_are_bounded_before_evaluation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        let service = CdpLocatorService::new(client.clone());
        let selector = Selector::text_regex(
            RegexMatcher::new("x".repeat(MAX_SELECTOR_STRING_BYTES + 1), "")
                .expect("large source still has valid flags"),
        );

        // Act
        let error = service
            .call(ClickParams::new(selector, ClickOptions::new()))
            .await
            .expect_err("oversized regex source should fail");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(client.method_names().is_empty());
    }

    #[tokio::test]
    async fn fill_retries_until_editable_then_inserts_text_through_cdp() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input readonly>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "noteditable"}}}),
            json!({"result": {"value": {"status": "needs-input", "isMac": false}}}),
        ]);
        let service = CdpLocatorService::with_retry_delay(client.clone(), Duration::ZERO);

        // Act
        service
            .call(FillParams::new(
                selector("input"),
                "Ada Lovelace",
                FillOptions::new(),
            ))
            .await
            .expect("editable input should be filled");

        // Assert
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Input.insertText",
            ]
        );
        assert_eq!(client.calls()[3].1, vec![json!("Ada Lovelace")]);
        assert_eq!(
            service.selector_runtime_metrics(),
            super::SelectorRuntimeMetrics {
                installations: 1,
                actionability_evaluations: 2,
                hover_evaluations: 0,
                focus_evaluations: 0,
                snapshot_evaluations: 0,
                state_evaluations: 0,
                content_evaluations: 0,
                attribute_evaluations: 0,
                input_value_evaluations: 0,
                all_inner_texts_evaluations: 0,
                all_text_contents_evaluations: 0,
                wait_evaluations: 0,
                missing_after_navigation: 0,
            }
        );
    }

    #[tokio::test]
    async fn direct_fill_completion_does_not_dispatch_keyboard_input() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input type=date>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done"}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        service
            .call(FillParams::new(
                selector("input"),
                "2026-09-02",
                FillOptions::new(),
            ))
            .await
            .expect("date input should fill directly");

        // Assert
        assert_eq!(
            client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate"]
        );
    }

    #[cfg(feature = "fill-navigation-commit")]
    #[tokio::test]
    async fn direct_fill_accepts_context_loss_only_after_exact_commit_receipt() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input type=date>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "count": 1}}}),
        ]);
        let mut service = CdpLocatorService::new(client.clone());
        service.unique_context_id = Some(Arc::from("utility-context-1"));
        let service = service.with_fill_commit_execution_context(7);
        service
            .call(CountParams::new(selector("input")))
            .await
            .unwrap();
        client.set_evaluate_errors(["Cannot find context with specified id".to_owned()]);
        let gate = Arc::new(tokio::sync::Notify::new());
        client.set_evaluate_gate(Some(gate.clone()));

        // Act
        let fill = tokio::spawn(async move {
            service
                .call(FillParams::new(
                    selector("input"),
                    "2020-03-02",
                    FillOptions::new(),
                ))
                .await
        });
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if client
                    .method_names()
                    .last()
                    .is_some_and(|name| name == "Runtime.evaluate")
                    && client.method_names().len() >= 4
                {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let calls = client.calls();
        let script = calls[3].1[0]["expression"].as_str().unwrap();
        let binding = "globalThis[\"__pwrightFillCommit\"](";
        let payload_start = script.find(binding).unwrap() + binding.len();
        let payload_end = payload_start + script[payload_start..].find(");").unwrap();
        let payload: String = serde_json::from_str(&script[payload_start..payload_end]).unwrap();
        client.send_event(CdpEvent::new(
            "Runtime.bindingCalled".to_owned(),
            json!({
                "name": super::FILL_COMMIT_BINDING,
                "executionContextId": 7,
                "payload": payload,
            }),
            None,
        ));
        gate.notify_one();
        let result = fill.await.unwrap();

        // Assert
        assert!(result.is_ok());
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Runtime.addBinding",
                "Runtime.evaluate",
            ]
        );
    }

    #[cfg(feature = "fill-navigation-commit")]
    #[tokio::test]
    async fn direct_fill_preserves_context_loss_without_commit_receipt() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input type=date>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "done", "count": 1}}}),
        ]);
        let mut service = CdpLocatorService::new(client.clone());
        service.unique_context_id = Some(Arc::from("utility-context-1"));
        let service = service.with_fill_commit_execution_context(7);
        service
            .call(CountParams::new(selector("input")))
            .await
            .unwrap();
        client.set_evaluate_errors(["Cannot find context with specified id".to_owned()]);

        // Act
        let error = service
            .call(FillParams::new(
                selector("input"),
                "2020-03-02",
                FillOptions::new(),
            ))
            .await
            .unwrap_err();

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(
            error
                .message()
                .contains("Cannot find context with specified id")
        );
    }

    #[cfg(feature = "fill-navigation-commit")]
    #[test]
    fn direct_fill_receipt_runs_only_after_both_dom_events() {
        // Arrange
        let selector = Selector::Css(selector("input[type=date]"));

        // Act
        let script =
            fill_actionability_script(&selector, "2020-03-02", false, Some("world:1")).unwrap();

        // Assert
        let input = script
            .find("element.dispatchEvent(new Event('input'")
            .unwrap();
        let change = script
            .find("element.dispatchEvent(new Event('change'")
            .unwrap();
        let receipt = script
            .find("globalThis[\"__pwrightFillCommit\"](\"world:1\")")
            .unwrap();
        assert!(input < change && change < receipt);
        assert!(receipt < script.find("return { status: \"done\" }").unwrap());
    }

    #[tokio::test]
    async fn empty_fill_deletes_the_selected_editable_value() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<textarea>old</textarea>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "needs-input", "isMac": true}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());

        // Act
        service
            .call(FillParams::new(
                selector("textarea"),
                "",
                FillOptions::new(),
            ))
            .await
            .expect("empty fill should clear the selected value");

        // Assert
        assert_eq!(
            client.method_names(),
            vec![
                "Runtime.evaluate",
                "Runtime.evaluate",
                "Input.dispatchKeyEvent",
                "Input.dispatchKeyEvent",
            ]
        );
        let calls = client.calls();
        assert_eq!(calls[2].1[0]["type"], "rawKeyDown");
        assert_eq!(calls[3].1[0]["type"], "keyUp");
    }

    #[tokio::test]
    async fn fill_cannot_focus_or_insert_while_another_page_input_transaction_is_held() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "needs-input", "isMac": false}}}),
        ]);
        let service = CdpLocatorService::new(client.clone());
        let held = service
            .input
            .begin_transaction(client.clone(), None)
            .await
            .unwrap();
        let mut future = service.call(FillParams::new(
            selector("input"),
            "value",
            FillOptions::new(),
        ));
        let mut task = std::task::Context::from_waker(std::task::Waker::noop());
        assert!(future.as_mut().poll(&mut task).is_pending());
        assert_eq!(client.method_names(), vec!["Runtime.evaluate"]);
        // Act
        drop(held);
        let result = future.await;
        // Assert
        assert!(result.is_ok());
        assert_eq!(
            client.method_names(),
            vec!["Runtime.evaluate", "Runtime.evaluate", "Input.insertText"]
        );
    }

    #[tokio::test]
    async fn clear_repairs_an_acknowledged_delete_after_key_up_failure() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "needs-input", "isMac": true}}}),
        ]);
        client.set_input_errors([None, Some("key up failed".into()), None]);
        let service = CdpLocatorService::new(client.clone());
        // Act
        let result = service
            .call(FillParams::new(selector("input"), "", FillOptions::new()))
            .await;
        // Assert
        assert_eq!(result.unwrap_err().kind(), FailureKind::Backend);
        let calls = client.calls();
        assert_eq!(calls.len(), 5);
        assert_eq!(calls[2].1[0]["code"], "Delete");
        assert_eq!(calls[3].1[0]["type"], "keyUp");
        assert_eq!(calls[4].1[0]["type"], "keyUp");
        let transaction = service
            .input
            .begin_transaction(client.clone(), None)
            .await
            .unwrap();
        drop(transaction);
        assert_eq!(
            client.calls().len(),
            5,
            "the repaired key must not remain pressed"
        );
    }

    #[tokio::test]
    async fn fill_reports_invalid_target_and_value_as_typed_terminal_failures() {
        // Arrange
        let target_client = Arc::new(FakeCdpClient::from_html("<button>Save</button>"));
        target_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "invalid-target",
                "message": "Element is not editable"
            }}}),
        ]);
        let value_client = Arc::new(FakeCdpClient::from_html("<input type=number>"));
        value_client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {
                "status": "invalid-value",
                "message": "Cannot type text into input[type=number]"
            }}}),
        ]);

        // Act
        let target = CdpLocatorService::new(target_client)
            .call(FillParams::new(selector("button"), "x", FillOptions::new()))
            .await
            .expect_err("button should not be fillable");
        let value = CdpLocatorService::new(value_client)
            .call(FillParams::new(
                selector("input"),
                "NaN",
                FillOptions::new(),
            ))
            .await
            .expect_err("number input should reject text");

        // Assert
        assert_eq!(
            target.details(),
            Some(&LocatorErrorDetails::InvalidTarget(
                "Element is not editable".to_string()
            ))
        );
        assert_eq!(
            value.details(),
            Some(&LocatorErrorDetails::InvalidValue(
                "Cannot type text into input[type=number]".to_string()
            ))
        );
    }

    #[test]
    fn fill_script_uses_playwright_compatible_invalid_target_words() {
        // Arrange
        let selector = Selector::Css(selector("input"));

        // Act
        let script = fill_actionability_script(&selector, "value", false, None)
            .expect("fill script should compile");

        // Assert
        assert!(script.contains("Input of type"));
        assert!(
            script.contains("Element is not an <input>, <textarea> or [contenteditable] element")
        );
    }

    #[test]
    fn fill_script_canonicalizes_color_case_without_changing_other_direct_inputs() {
        // Arrange
        let selector = Selector::Css(selector("input"));

        // Act
        let script = fill_actionability_script(&selector, "#AbCd00", false, None)
            .expect("fill script should compile");

        // Assert
        assert!(script.contains("type === 'color' ? value.trim().toLowerCase() : value.trim()"));
        assert!(script.contains("if (element.value !== expected)"));
    }

    #[tokio::test]
    async fn fill_value_is_bounded_before_javascript_evaluation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<input>"));
        let service = CdpLocatorService::new(client.clone());

        // Act
        let error = service
            .call(FillParams::new(
                selector("input"),
                "x".repeat(1024 * 1024 + 1),
                FillOptions::new(),
            ))
            .await
            .expect_err("oversized fill value should fail locally");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(matches!(
            error.details(),
            Some(LocatorErrorDetails::InvalidValue(_))
        ));
        assert!(client.method_names().is_empty());
    }

    #[test]
    fn production_profile_covers_every_concurrent_action_at_its_transient_peak() {
        // Arrange
        let profile = CdpLocatorProductionProfile::new(LOCATOR_SCOPE, 3);

        // Act
        let requirement = profile.requirements().class(ResourceClass::SelectorBytes);

        // Assert
        assert_eq!(profile.resource_scope(), LOCATOR_SCOPE);
        assert_eq!(super::MAX_CONTENT_RESULT_BYTES, 1024 * 1024);
        assert_eq!(
            super::MAX_LOCATOR_RESULT_BYTES,
            super::MAX_COLLECTION_LOCATOR_RESULT_BYTES
        );
        assert_eq!(
            super::MAX_COLLECTION_LOCATOR_RESULT_BYTES,
            super::MAX_ALL_TEXT_CONTENTS_RETAINED_BYTES
        );
        assert_eq!(
            super::MAX_COLLECTION_LOCATOR_RESULT_BYTES,
            super::MAX_ALL_INNER_TEXTS_RETAINED_BYTES
        );
        assert_ne!(
            super::MAX_COLLECTION_LOCATOR_RESULT_BYTES,
            super::MAX_ALL_TEXT_CONTENTS_RETAINED_BYTES + super::MAX_ALL_INNER_TEXTS_RETAINED_BYTES
        );
        assert_eq!(super::MAX_SCALAR_LOCATOR_RESULT_BYTES, 1024 * 1024);
        assert_eq!(
            super::MAX_LOCATOR_INPUT_BYTES,
            super::MAX_SELECT_OPTION_INPUT_RETAINED_BYTES
        );
        assert_eq!(
            super::MAX_ACTION_SCRIPT_BYTES,
            super::MAX_SELECT_OPTION_SCRIPT_BYTES
        );
        assert!(profile.maximum_retained_bytes_per_action() > 8 * 1024 * 1024);
        assert_eq!(
            requirement.global,
            profile.maximum_retained_bytes_per_action() * 3
        );
        assert_eq!(requirement.per_scope, requirement.global);
        assert_eq!(profile.requirements().scopes, 1);
        assert_eq!(profile.requirements().leases, 3);
    }

    #[test]
    fn production_profile_reserves_atomic_frame_route_replacement() {
        // Arrange
        let ordinary = CdpLocatorProductionProfile::new(LOCATOR_SCOPE, 1);
        let profile = ordinary.with_context_routes(3, 64);
        let expected_route_bytes = 3 * (12 * 64 + 2 * std::mem::size_of::<i64>() as u64);
        let resources = production_resources(profile, 0);
        let factory = CdpLocatorServiceFactory::new_with_production_resources(profile, &resources)
            .expect("locator factory should claim actions and routes");

        // Act
        let route_lease = factory
            .reserve_context_routes()
            .expect("route storage should reserve")
            .expect("configured routes need one lease");

        // Assert
        assert_eq!(profile.max_context_routes(), 3);
        assert_eq!(profile.max_native_id_bytes(), 64);
        assert_eq!(profile.maximum_route_retained_bytes(), expected_route_bytes);
        assert_eq!(
            profile.requirements().leases,
            ordinary.requirements().leases + 1
        );
        assert_eq!(
            profile.requirements().class(ResourceClass::SelectorBytes),
            ordinary.requirements().class(ResourceClass::SelectorBytes)
        );
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should remain readable")
                .class(ResourceClass::BackendStateBytes)
                .current,
            profile.maximum_route_retained_bytes()
        );
        drop(route_lease);
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should remain readable")
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
    }

    #[tokio::test]
    async fn unpolled_production_action_owns_capacity_and_drop_releases_it() {
        // Arrange
        let profile = CdpLocatorProductionProfile::new(LOCATOR_SCOPE, 1);
        let resources = production_resources(profile, 0);
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        let service =
            CdpLocatorService::new_with_production_resources(client.clone(), profile, &resources)
                .expect("declared locator should construct");

        // Act
        let pending = service.call(ClickParams::new(
            selector("button"),
            ClickOptions::new().with_trial(true),
        ));
        let retained = resources
            .metrics()
            .expect("metrics should remain readable")
            .class(ResourceClass::SelectorBytes)
            .current;
        drop(pending);

        // Assert
        assert_eq!(retained, profile.maximum_retained_bytes_per_action());
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should remain readable")
                .class(ResourceClass::SelectorBytes)
                .current,
            0
        );
        assert!(client.method_names().is_empty());
    }

    #[tokio::test]
    async fn factory_bound_pages_share_locator_concurrency_before_backend_or_future_retention() {
        // Arrange
        let profile = CdpLocatorProductionProfile::new(LOCATOR_SCOPE, 1);
        let per_action = profile.maximum_retained_bytes_per_action();
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 2).with_class(
                ResourceClass::SelectorBytes,
                per_action * 2,
                per_action * 2,
            ),
            profile.requirements(),
        )
        .expect("overprovisioned shared ledger should still preserve the local limit");
        let first_client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        let second_client = Arc::new(FakeCdpClient::from_html("<button>B</button>"));
        let factory = CdpLocatorServiceFactory::new_with_production_resources(profile, &resources)
            .expect("declared locator factory should construct");
        let first_page = factory.bind(first_client.clone());
        let second_page = factory.bind(second_client.clone());
        let first = first_page.call(ClickParams::new(
            selector("button"),
            ClickOptions::new().with_trial(true),
        ));

        // Act
        let error = second_page
            .call(ClickParams::new(
                selector("button"),
                ClickOptions::new().with_trial(true),
            ))
            .await
            .expect_err("second concurrent action should exhaust its local profile");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(error.message().contains("concurrency limit"));
        assert!(first_client.method_names().is_empty());
        assert!(second_client.method_names().is_empty());
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should remain readable")
                .class(ResourceClass::SelectorBytes)
                .current,
            profile.maximum_retained_bytes_per_action()
        );
        drop(first);
    }

    #[tokio::test]
    async fn completed_production_action_releases_selector_capacity() {
        // Arrange
        let profile = CdpLocatorProductionProfile::new(LOCATOR_SCOPE, 1);
        let resources = production_resources(profile, 0);
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        client.set_evaluate_responses([
            runtime_installed(),
            json!({"result": {"value": {"status": "ready", "x": 1.0, "y": 2.0}}}),
        ]);
        let service = CdpLocatorService::new_with_production_resources(client, profile, &resources)
            .expect("declared locator should construct");

        // Act
        service
            .call(ClickParams::new(
                selector("button"),
                ClickOptions::new().with_trial(true),
            ))
            .await
            .expect("production click should complete");

        // Assert
        let metrics = resources
            .metrics()
            .expect("metrics should remain readable")
            .class(ResourceClass::SelectorBytes);
        assert_eq!(metrics.current, 0);
        assert_eq!(metrics.peak, profile.maximum_retained_bytes_per_action());
    }

    #[tokio::test]
    async fn production_rejects_spare_string_capacity_before_browser_interaction() {
        // Arrange
        let profile = CdpLocatorProductionProfile::new(LOCATOR_SCOPE, 1);
        let resources = production_resources(profile, 0);
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        let service =
            CdpLocatorService::new_with_production_resources(client.clone(), profile, &resources)
                .expect("declared locator should construct");
        let mut oversized_capacity = String::with_capacity(MAX_SELECTOR_STRING_BYTES + 1);
        oversized_capacity.push_str("button");

        // Act
        let error = service
            .call(ClickParams::new(
                CssSelector::new(oversized_capacity).expect("selector content is valid"),
                ClickOptions::new(),
            ))
            .await
            .expect_err("spare capacity must not bypass the production memory contract");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(client.method_names().is_empty());
        assert_eq!(resources.metrics().expect("metrics").active_leases, 0);
    }

    #[tokio::test]
    async fn production_rejects_spare_attribute_name_capacity_before_browser_interaction() {
        // Arrange
        let profile = CdpLocatorProductionProfile::new(LOCATOR_SCOPE, 1);
        let resources = production_resources(profile, 0);
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));
        let service =
            CdpLocatorService::new_with_production_resources(client.clone(), profile, &resources)
                .expect("declared locator should construct");
        let mut oversized_capacity = String::with_capacity(super::MAX_ATTRIBUTE_NAME_BYTES + 1);
        oversized_capacity.push_str("data-state");

        // Act
        let error = service
            .call(GetAttributeParams::new(
                selector("button"),
                oversized_capacity,
            ))
            .await
            .expect_err("attribute name spare capacity must not escape the production contract");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(client.method_names().is_empty());
        assert_eq!(resources.metrics().expect("metrics").active_leases, 0);
    }

    #[test]
    fn omitted_locator_profile_fails_during_production_construction() {
        // Arrange
        let profile = CdpLocatorProductionProfile::new(LOCATOR_SCOPE, 1);
        let requirement = profile.requirements().class(ResourceClass::SelectorBytes);
        let limits = ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::SelectorBytes,
            requirement.global,
            requirement.per_scope,
        );
        let resources = ProductionResourceLedger::new(limits, ResourceRequirements::none())
            .expect("empty plan should be valid");
        let client = Arc::new(FakeCdpClient::from_html("<button>A</button>"));

        // Act
        let error =
            match CdpLocatorService::new_with_production_resources(client, profile, &resources) {
                Ok(_) => panic!("omitted locator profile must fail closed"),
                Err(error) => error,
            };

        // Assert
        assert!(matches!(
            error,
            pwright_core::ProductionResourceError::InsufficientGlobal {
                class: ResourceClass::SelectorBytes,
                ..
            }
        ));
    }
}
