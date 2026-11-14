//! Chromium CDP adapters for portable Pwright capabilities.
//!
//! This crate attaches capability services to an existing `pwright-cdp`
//! client. It does not discover, download, launch, or own a browser process.

#[cfg(feature = "full")]
mod context_creation {
    pub use pwright_backend_cdp_context_creation::*;
}
#[cfg(feature = "full")]
mod context_lifecycle {
    pub use pwright_backend_cdp_context_lifecycle::*;
}
#[cfg(feature = "full")]
mod content {
    pub use pwright_backend_cdp_content::{
        CdpScreenshotAdmission, CdpScreenshotProductionProfile, CdpScreenshotService,
        CdpScreenshotServiceFactory,
    };
}
#[cfg(feature = "full")]
mod dom_content {
    pub use pwright_backend_cdp_dom_content::{
        CdpDomContentProductionProfile, CdpDomContentService, CdpDomContentServiceFactory,
    };
}
#[cfg(feature = "full")]
mod evaluation {
    pub use pwright_backend_cdp_evaluation::{
        CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpEvaluationService,
        CdpEvaluationServiceFactory,
    };
}
#[cfg(feature = "full")]
mod handle_evaluation {
    pub use pwright_backend_cdp_handle_evaluation::{
        CdpAdoptedElementHandle, CdpAdoptedElementHandleBatch,
        CdpElementHandleAdoptionBatchRequest, CdpElementHandleAdoptionBatchService,
        CdpElementHandleAdoptionInstallError, CdpElementHandleAdoptionProductionProfile,
        CdpElementHandleAdoptionRequest, CdpElementHandleAdoptionService,
        CdpElementHandleAdoptionSetupError, CdpEvaluatedHandle, CdpHandleEvaluationLimits,
        CdpHandleEvaluationRequest, CdpHandleEvaluationService, CdpHandleKind,
    };
}
#[cfg(feature = "full")]
mod handle_lifecycle {
    pub use pwright_backend_cdp_handle_lifecycle::*;
}
#[cfg(feature = "full")]
mod handle_operations {
    pub use pwright_backend_cdp_handle_evaluation::{
        CdpHandlePropertiesService, CdpHandlePropertyLimitError, CdpHandlePropertyLimits,
        CdpHandleValueEvaluationService,
    };
}
#[cfg(feature = "full")]
mod input {
    pub use pwright_backend_cdp_input::{CdpInputCoordinator, CdpInputTransaction, CdpModifiers};
}
#[cfg(feature = "full")]
mod locator {
    pub use pwright_backend_cdp_locator::*;
}
#[cfg(feature = "full")]
mod locator_screenshot;
#[cfg(feature = "full")]
mod network_events {
    pub use pwright_backend_cdp_network_events::*;
}
#[cfg(feature = "full")]
mod page_creation {
    pub use pwright_backend_cdp_page_creation::*;
}
#[cfg(feature = "full")]
mod page_events {
    pub use pwright_backend_cdp_page_events::*;
}
#[cfg(feature = "full")]
mod page_lifecycle {
    pub use pwright_backend_cdp_page_lifecycle::*;
}
#[cfg(feature = "full")]
mod topology {
    pub use pwright_backend_cdp_topology::*;
}

#[cfg(feature = "full")]
pub use content::{
    CdpScreenshotAdmission, CdpScreenshotProductionProfile, CdpScreenshotService,
    CdpScreenshotServiceFactory,
};
#[cfg(feature = "full")]
pub use context_creation::{
    CdpBrowserContextCreationOptions, CdpBrowserContextCreationService, CdpContextCreationLimits,
    CdpContextCreationSetupError, CdpCreatedBrowserContext,
};
#[cfg(feature = "full")]
pub use context_lifecycle::{
    CdpBrowserContextCloseService, CdpContextCloseLimits, CdpContextCloseOutcome,
    CdpContextCloseSetupError,
};
#[cfg(feature = "full")]
pub use dom_content::{
    CdpDomContentProductionProfile, CdpDomContentService, CdpDomContentServiceFactory,
};
#[cfg(feature = "full")]
pub use evaluation::{
    CdpEvaluationLimits, CdpEvaluationProductionProfile, CdpEvaluationService,
    CdpEvaluationServiceFactory,
};
#[cfg(feature = "full")]
pub use handle_evaluation::{
    CdpAdoptedElementHandle, CdpAdoptedElementHandleBatch, CdpElementHandleAdoptionBatchRequest,
    CdpElementHandleAdoptionBatchService, CdpElementHandleAdoptionInstallError,
    CdpElementHandleAdoptionProductionProfile, CdpElementHandleAdoptionRequest,
    CdpElementHandleAdoptionService, CdpElementHandleAdoptionSetupError, CdpEvaluatedHandle,
    CdpHandleEvaluationLimits, CdpHandleEvaluationRequest, CdpHandleEvaluationService,
    CdpHandleKind,
};
#[cfg(feature = "full")]
pub use handle_lifecycle::{
    CdpHandleDisposeLimits, CdpHandleDisposeService, CdpHandleDisposeSetupError,
};
#[cfg(feature = "full")]
pub use handle_operations::{
    CdpHandlePropertiesService, CdpHandlePropertyLimitError, CdpHandlePropertyLimits,
    CdpHandleValueEvaluationService,
};
#[cfg(feature = "full")]
pub use input::{CdpInputCoordinator, CdpInputTransaction, CdpModifiers};

#[cfg(feature = "full")]
pub use locator::{
    CdpLocatorProductionProfile, CdpLocatorService, CdpLocatorServiceFactory, LocatorInputHandoff,
    SelectorRuntimeMetrics,
};
#[cfg(feature = "full")]
pub use locator_screenshot::CdpLocatorScreenshotResolver;
#[cfg(all(feature = "full", feature = "response-security-details"))]
pub use network_events::CdpResponseSecurityDetails;
#[cfg(feature = "full")]
pub use network_events::{
    CdpNetworkEventAdapter, CdpNetworkEventBatch, CdpNetworkEventEffect, CdpNetworkEventError,
    CdpNetworkEventProductionError, CdpNetworkEventProductionProfile,
};
#[cfg(feature = "full")]
pub use page_creation::{
    CdpCreatedPage, CdpPageCreationLimits, CdpPageCreationRequest, CdpPageCreationService,
    CdpPageCreationSetupError,
};
#[cfg(feature = "full")]
pub use page_events::{
    CdpPageEventAdapter, CdpPageEventError, map_browser_context_page_effect, map_page_popup_effect,
};
#[cfg(feature = "full")]
pub use page_lifecycle::{
    CdpPageCloseLimits, CdpPageClosePoll, CdpPageCloseService, CdpPageCloseSetupError,
    CdpPageRunBeforeUnloadService, ImmediatePageClosePoll,
};
#[cfg(feature = "full")]
pub use pwright_backend_cdp_locator_screenshot::{
    CdpFrameClipTranslator, CdpLocatorScreenshotService, LocatorScreenshotGeometry,
    LocatorScreenshotGeometryError, LocatorScreenshotGeometryResolver,
};
#[cfg(feature = "full")]
pub use topology::{
    CdpBrowserContextRoute, CdpContextOwnership, CdpFrameAttachment, CdpFrameDetachReason,
    CdpFrameExecutionRoute, CdpPageObjectIds, CdpPageSessionRoute, CdpPageState, CdpPageTarget,
    CdpTargetSessionRoute, CdpTopology, CdpTopologyDisposition, CdpTopologyEffect,
    CdpTopologyError, CdpTopologyEventError, CdpTopologyLimits, CdpTopologyMetrics,
    CdpTopologyProductionError, CdpTopologyProductionProfile, CdpTopologyUpdate,
    CdpWorldAttachment, CdpWorldKind, ConnectionEpoch, PreparedCdpBrowserContextClose,
    PreparedCdpPageClose, PreparedCdpTopologyTransition, PreparedOwnedCdpTopologyTransition,
    ProductionCdpTopology,
};

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "full")]
use pwright_browser_backend::BackendFeatureId;
use pwright_browser_backend::{
    BackendDescriptor, BackendEventFeatureDescriptor, BackendFeatureDescriptor, BackendId,
    BackendImplementation, BackendProtocolId, BrowserBackend, BrowserEngine, NativeBackendAccess,
};
#[cfg(feature = "full")]
use pwright_cap_content::{CONTENT_CAPABILITY_ID, PAGE_SCREENSHOT_COMMAND_ID};
#[cfg(feature = "full")]
use pwright_cap_context_creation::{
    BROWSER_NEW_CONTEXT_COMMAND_ID, CONTEXT_CREATION_CAPABILITY_ID,
};
#[cfg(feature = "full")]
use pwright_cap_context_lifecycle::{
    BROWSER_CONTEXT_CLOSE_COMMAND_ID, CONTEXT_LIFECYCLE_CAPABILITY_ID,
};
#[cfg(feature = "full")]
use pwright_cap_dom_content::{DOM_CONTENT_CAPABILITY_ID, FRAME_CONTENT_COMMAND_ID};
#[cfg(feature = "full")]
use pwright_cap_evaluation::{EVALUATE_VALUE_COMMAND_ID, EVALUATION_CAPABILITY_ID};
#[cfg(feature = "full")]
use pwright_cap_handle_evaluation::{
    EVALUATE_HANDLE_COMMAND_ID, HANDLE_EVALUATE_COMMAND_ID, HANDLE_EVALUATE_HANDLE_COMMAND_ID,
    HANDLE_EVALUATION_CAPABILITY_ID, HANDLE_GET_PROPERTIES_COMMAND_ID,
    HANDLE_GET_PROPERTY_COMMAND_ID, HANDLE_JSON_VALUE_COMMAND_ID,
};
#[cfg(feature = "full")]
use pwright_cap_handle_lifecycle::{HANDLE_DISPOSE_COMMAND_ID, HANDLE_LIFECYCLE_CAPABILITY_ID};
#[cfg(feature = "full")]
use pwright_cap_locator::{
    ALL_INNER_TEXTS_COMMAND_ID, ALL_TEXT_CONTENTS_COMMAND_ID, BLUR_COMMAND_ID, CLICK_COMMAND_ID,
    COUNT_COMMAND_ID, FILL_COMMAND_ID, FOCUS_COMMAND_ID, GET_ATTRIBUTE_COMMAND_ID,
    HOVER_COMMAND_ID, INNER_HTML_COMMAND_ID, INNER_TEXT_COMMAND_ID, INPUT_VALUE_COMMAND_ID,
    IS_CHECKED_COMMAND_ID, IS_EDITABLE_COMMAND_ID, IS_ENABLED_COMMAND_ID, IS_VISIBLE_COMMAND_ID,
    LOCATOR_CAPABILITY_ID, PRESS_COMMAND_ID, SCROLL_INTO_VIEW_IF_NEEDED_COMMAND_ID,
    SELECT_OPTION_COMMAND_ID, SET_CHECKED_COMMAND_ID, TEXT_CONTENT_COMMAND_ID, WAIT_FOR_COMMAND_ID,
};
#[cfg(feature = "full")]
use pwright_cap_locator_screenshot::{
    LOCATOR_SCREENSHOT_CAPABILITY_ID, LOCATOR_SCREENSHOT_COMMAND_ID,
};
use pwright_cap_navigation::{
    BEGIN_NAVIGATION_COMMAND_ID, BeginNavigation, BeginNavigationParams, BeginNavigationResult,
    GOTO_COMMAND_ID, Goto, GotoParams, GotoResult, NAVIGATION_CAPABILITY_ID,
    NavigationErrorDetails,
};
#[cfg(feature = "full")]
use pwright_cap_network_events::{
    NETWORK_EVENTS_CAPABILITY_ID, NETWORK_REQUEST_EVENT_ID, NETWORK_REQUEST_FAILED_EVENT_ID,
    NETWORK_REQUEST_FINISHED_EVENT_ID, NETWORK_RESPONSE_EVENT_ID,
};
#[cfg(feature = "full")]
use pwright_cap_page_creation::{BROWSER_CONTEXT_NEW_PAGE_COMMAND_ID, PAGE_CREATION_CAPABILITY_ID};
#[cfg(feature = "full")]
use pwright_cap_page_events::{
    BROWSER_CONTEXT_PAGE_EVENT_ID, PAGE_EVENTS_CAPABILITY_ID, PAGE_LOAD_EVENT_ID,
    PAGE_POPUP_EVENT_ID,
};
#[cfg(feature = "full")]
use pwright_cap_page_lifecycle::{
    PAGE_CLOSE_COMMAND_ID, PAGE_LIFECYCLE_CAPABILITY_ID, PAGE_RUN_BEFORE_UNLOAD_COMMAND_ID,
};
use pwright_cdp::CdpClient;
use pwright_core::{
    CancellationReason, CommandCancellation, OperationContext, OperationError,
    ProductionResourceError, ProductionResourceLedger, ProtocolVersion, Service,
};

pub use pwright_backend_cdp_navigation::{
    CDP_BEGIN_NAVIGATION_FEATURE, CDP_GOTO_FEATURE, CDP_RELOAD_FEATURE, CdpNavigationLimits,
    CdpNavigationProductionProfile,
};
#[cfg(feature = "history-navigation")]
pub use pwright_backend_cdp_navigation::{CDP_GO_BACK_FEATURE, CDP_GO_FORWARD_FEATURE};
use pwright_backend_cdp_navigation::{
    CdpNavigationService as LeafCdpNavigationService,
    CdpNavigationServiceFactory as LeafCdpNavigationServiceFactory, ChromiumCdpNavigationBackend,
};
pub const CHROMIUM_CDP_BACKEND_ID: BackendId = BackendId::new("pwright.backend.chromium-cdp");
pub const CHROMIUM_CDP_PROTOCOL_ID: BackendProtocolId = BackendProtocolId::new("cdp");

#[cfg(feature = "full")]
mod full_backend_features {
    use super::*;

    pub const CDP_LOCATOR_CLICK_FEATURE: BackendFeatureId = BackendFeatureId::new("locator.click");
    pub const CDP_LOCATOR_ALL_INNER_TEXTS_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.all-inner-texts");
    pub const CDP_LOCATOR_ALL_TEXT_CONTENTS_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.all-text-contents");
    pub const CDP_LOCATOR_BLUR_FEATURE: BackendFeatureId = BackendFeatureId::new("locator.blur");
    pub const CDP_LOCATOR_COUNT_FEATURE: BackendFeatureId = BackendFeatureId::new("locator.count");
    pub const CDP_LOCATOR_FILL_FEATURE: BackendFeatureId = BackendFeatureId::new("locator.fill");
    pub const CDP_LOCATOR_FOCUS_FEATURE: BackendFeatureId = BackendFeatureId::new("locator.focus");
    pub const CDP_LOCATOR_PRESS_FEATURE: BackendFeatureId = BackendFeatureId::new("locator.press");
    pub const CDP_LOCATOR_PRESS_SEQUENTIALLY_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.press-sequentially");
    pub const CDP_LOCATOR_SCROLL_INTO_VIEW_IF_NEEDED_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.scroll-into-view-if-needed");
    pub const CDP_LOCATOR_SELECT_OPTION_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.select-option");
    pub const CDP_LOCATOR_SET_CHECKED_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.set-checked");
    pub const CDP_LOCATOR_GET_ATTRIBUTE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.get-attribute");
    pub const CDP_LOCATOR_HOVER_FEATURE: BackendFeatureId = BackendFeatureId::new("locator.hover");
    pub const CDP_LOCATOR_INNER_HTML_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.inner-html");
    pub const CDP_LOCATOR_INNER_TEXT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.inner-text");
    pub const CDP_LOCATOR_INPUT_VALUE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.input-value");
    pub const CDP_LOCATOR_IS_CHECKED_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.is-checked");
    pub const CDP_LOCATOR_IS_EDITABLE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.is-editable");
    pub const CDP_LOCATOR_IS_ENABLED_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.is-enabled");
    pub const CDP_LOCATOR_IS_VISIBLE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.is-visible");
    pub const CDP_LOCATOR_TEXT_CONTENT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.text-content");
    pub const CDP_LOCATOR_WAIT_FOR_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.wait-for");
    pub const CDP_PAGE_LOAD_EVENT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("page-events.load");
    pub const CDP_BROWSER_CONTEXT_PAGE_EVENT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("page-events.browser-context-page");
    pub const CDP_PAGE_POPUP_EVENT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("page-events.popup");
    pub const CDP_NETWORK_REQUEST_EVENT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("network-events.request");
    pub const CDP_NETWORK_RESPONSE_EVENT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("network-events.response");
    pub const CDP_NETWORK_REQUEST_FINISHED_EVENT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("network-events.request-finished");
    pub const CDP_NETWORK_REQUEST_FAILED_EVENT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("network-events.request-failed");
    pub const CDP_BROWSER_CONTEXT_CLOSE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("browser-context.close");
    pub const CDP_BROWSER_NEW_CONTEXT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("browser.new-context");
    pub const CDP_BROWSER_CONTEXT_NEW_PAGE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("browser-context.new-page");
    pub const CDP_PAGE_CLOSE_FEATURE: BackendFeatureId = BackendFeatureId::new("page.close");
    pub const CDP_PAGE_RUN_BEFORE_UNLOAD_FEATURE: BackendFeatureId =
        BackendFeatureId::new("page.run-before-unload");
    pub const CDP_EVALUATE_VALUE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("evaluation.evaluate-value");
    pub const CDP_EVALUATE_HANDLE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("evaluation.evaluate-handle");
    pub const CDP_HANDLE_EVALUATE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("handle.evaluate");
    pub const CDP_HANDLE_EVALUATE_HANDLE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("handle.evaluate-handle");
    pub const CDP_HANDLE_JSON_VALUE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("handle.json-value");
    pub const CDP_HANDLE_GET_PROPERTY_FEATURE: BackendFeatureId =
        BackendFeatureId::new("handle.get-property");
    pub const CDP_HANDLE_GET_PROPERTIES_FEATURE: BackendFeatureId =
        BackendFeatureId::new("handle.get-properties");
    pub const CDP_HANDLE_DISPOSE_FEATURE: BackendFeatureId =
        BackendFeatureId::new("handle.dispose");
    pub const CDP_PAGE_SCREENSHOT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("page.screenshot");
    pub const CDP_LOCATOR_SCREENSHOT_FEATURE: BackendFeatureId =
        BackendFeatureId::new("locator.screenshot");
    pub const CDP_FRAME_CONTENT_FEATURE: BackendFeatureId = BackendFeatureId::new("frame.content");
}

#[cfg(feature = "full")]
pub use full_backend_features::*;

#[cfg(feature = "full")]
pub const CHROMIUM_CDP_FEATURES: &[BackendFeatureDescriptor] = &[
    BackendFeatureDescriptor::new(
        CDP_FRAME_CONTENT_FEATURE,
        DOM_CONTENT_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        FRAME_CONTENT_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_PAGE_SCREENSHOT_FEATURE,
        CONTENT_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        PAGE_SCREENSHOT_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_SCREENSHOT_FEATURE,
        LOCATOR_SCREENSHOT_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        LOCATOR_SCREENSHOT_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_BROWSER_NEW_CONTEXT_FEATURE,
        CONTEXT_CREATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        BROWSER_NEW_CONTEXT_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        CDP_BROWSER_CONTEXT_NEW_PAGE_FEATURE,
        PAGE_CREATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        BROWSER_CONTEXT_NEW_PAGE_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        CDP_BROWSER_CONTEXT_CLOSE_FEATURE,
        CONTEXT_LIFECYCLE_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        BROWSER_CONTEXT_CLOSE_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        CDP_PAGE_CLOSE_FEATURE,
        PAGE_LIFECYCLE_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        PAGE_CLOSE_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        CDP_PAGE_RUN_BEFORE_UNLOAD_FEATURE,
        PAGE_LIFECYCLE_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        PAGE_RUN_BEFORE_UNLOAD_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        CDP_EVALUATE_VALUE_FEATURE,
        EVALUATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        EVALUATE_VALUE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_EVALUATE_HANDLE_FEATURE,
        HANDLE_EVALUATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        EVALUATE_HANDLE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_HANDLE_EVALUATE_FEATURE,
        HANDLE_EVALUATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 2),
        HANDLE_EVALUATE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_HANDLE_EVALUATE_HANDLE_FEATURE,
        HANDLE_EVALUATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 2),
        HANDLE_EVALUATE_HANDLE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_HANDLE_JSON_VALUE_FEATURE,
        HANDLE_EVALUATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 2),
        HANDLE_JSON_VALUE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_HANDLE_GET_PROPERTY_FEATURE,
        HANDLE_EVALUATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 2),
        HANDLE_GET_PROPERTY_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_HANDLE_GET_PROPERTIES_FEATURE,
        HANDLE_EVALUATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 2),
        HANDLE_GET_PROPERTIES_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_HANDLE_DISPOSE_FEATURE,
        HANDLE_LIFECYCLE_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        HANDLE_DISPOSE_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        CDP_BEGIN_NAVIGATION_FEATURE,
        NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        BEGIN_NAVIGATION_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        CDP_GOTO_FEATURE,
        NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        GOTO_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_RELOAD_FEATURE,
        NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 1),
        pwright_cap_navigation::RELOAD_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    #[cfg(feature = "history-navigation")]
    BackendFeatureDescriptor::new(
        CDP_GO_BACK_FEATURE,
        pwright_cap_history_navigation::HISTORY_NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        pwright_cap_history_navigation::GO_BACK_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    #[cfg(feature = "history-navigation")]
    BackendFeatureDescriptor::new(
        CDP_GO_FORWARD_FEATURE,
        pwright_cap_history_navigation::HISTORY_NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 1),
        pwright_cap_history_navigation::GO_FORWARD_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_ALL_INNER_TEXTS_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        ALL_INNER_TEXTS_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_ALL_TEXT_CONTENTS_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        ALL_TEXT_CONTENTS_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_BLUR_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 18),
        BLUR_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_CLICK_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        CLICK_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_COUNT_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        COUNT_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_FILL_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        FILL_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_FOCUS_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 18),
        FOCUS_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_PRESS_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 19),
        PRESS_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_SCROLL_INTO_VIEW_IF_NEEDED_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 22),
        SCROLL_INTO_VIEW_IF_NEEDED_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_PRESS_SEQUENTIALLY_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 23),
        pwright_cap_locator::PRESS_SEQUENTIALLY_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_SELECT_OPTION_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 21),
        SELECT_OPTION_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_SET_CHECKED_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 20),
        SET_CHECKED_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_GET_ATTRIBUTE_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        GET_ATTRIBUTE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_HOVER_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        HOVER_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_INNER_HTML_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        INNER_HTML_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_INNER_TEXT_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        INNER_TEXT_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_INPUT_VALUE_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        INPUT_VALUE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_IS_CHECKED_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        IS_CHECKED_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_IS_EDITABLE_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        IS_EDITABLE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_IS_ENABLED_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        IS_ENABLED_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_IS_VISIBLE_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        IS_VISIBLE_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_TEXT_CONTENT_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        TEXT_CONTENT_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_LOCATOR_WAIT_FOR_FEATURE,
        LOCATOR_CAPABILITY_ID,
        ProtocolVersion::new(1, 17),
        WAIT_FOR_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
];

#[cfg(feature = "full")]
pub const CHROMIUM_CDP_EVENT_FEATURES: &[BackendEventFeatureDescriptor] = &[
    BackendEventFeatureDescriptor::new(
        CDP_PAGE_LOAD_EVENT_FEATURE,
        PAGE_EVENTS_CAPABILITY_ID,
        ProtocolVersion::new(1, 1),
        PAGE_LOAD_EVENT_ID,
        BackendImplementation::Emulated,
    ),
    BackendEventFeatureDescriptor::new(
        CDP_BROWSER_CONTEXT_PAGE_EVENT_FEATURE,
        PAGE_EVENTS_CAPABILITY_ID,
        ProtocolVersion::new(1, 2),
        BROWSER_CONTEXT_PAGE_EVENT_ID,
        BackendImplementation::Emulated,
    ),
    BackendEventFeatureDescriptor::new(
        CDP_PAGE_POPUP_EVENT_FEATURE,
        PAGE_EVENTS_CAPABILITY_ID,
        ProtocolVersion::new(1, 2),
        PAGE_POPUP_EVENT_ID,
        BackendImplementation::Emulated,
    ),
    BackendEventFeatureDescriptor::new(
        CDP_NETWORK_REQUEST_EVENT_FEATURE,
        NETWORK_EVENTS_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        NETWORK_REQUEST_EVENT_ID,
        BackendImplementation::Emulated,
    ),
    BackendEventFeatureDescriptor::new(
        CDP_NETWORK_RESPONSE_EVENT_FEATURE,
        NETWORK_EVENTS_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        NETWORK_RESPONSE_EVENT_ID,
        BackendImplementation::Emulated,
    ),
    BackendEventFeatureDescriptor::new(
        CDP_NETWORK_REQUEST_FINISHED_EVENT_FEATURE,
        NETWORK_EVENTS_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        NETWORK_REQUEST_FINISHED_EVENT_ID,
        BackendImplementation::Emulated,
    ),
    BackendEventFeatureDescriptor::new(
        CDP_NETWORK_REQUEST_FAILED_EVENT_FEATURE,
        NETWORK_EVENTS_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        NETWORK_REQUEST_FAILED_EVENT_ID,
        BackendImplementation::Emulated,
    ),
];

#[cfg(not(feature = "full"))]
pub const CHROMIUM_CDP_FEATURES: &[BackendFeatureDescriptor] = &[
    BackendFeatureDescriptor::new(
        CDP_BEGIN_NAVIGATION_FEATURE,
        NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        BEGIN_NAVIGATION_COMMAND_ID,
        BackendImplementation::Native,
    ),
    BackendFeatureDescriptor::new(
        CDP_GOTO_FEATURE,
        NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        GOTO_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    BackendFeatureDescriptor::new(
        CDP_RELOAD_FEATURE,
        NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 1),
        pwright_cap_navigation::RELOAD_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    #[cfg(feature = "history-navigation")]
    BackendFeatureDescriptor::new(
        CDP_GO_BACK_FEATURE,
        pwright_cap_history_navigation::HISTORY_NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 0),
        pwright_cap_history_navigation::GO_BACK_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
    #[cfg(feature = "history-navigation")]
    BackendFeatureDescriptor::new(
        CDP_GO_FORWARD_FEATURE,
        pwright_cap_history_navigation::HISTORY_NAVIGATION_CAPABILITY_ID,
        ProtocolVersion::new(1, 1),
        pwright_cap_history_navigation::GO_FORWARD_COMMAND_ID,
        BackendImplementation::Emulated,
    ),
];

#[cfg(not(feature = "full"))]
pub const CHROMIUM_CDP_EVENT_FEATURES: &[BackendEventFeatureDescriptor] = &[];

pub enum ChromiumCdpBackend {}

impl BrowserBackend for ChromiumCdpBackend {
    const DESCRIPTOR: BackendDescriptor = BackendDescriptor::new(
        CHROMIUM_CDP_BACKEND_ID,
        "Chromium CDP",
        BrowserEngine::Chromium,
        CHROMIUM_CDP_PROTOCOL_ID,
        CHROMIUM_CDP_FEATURES,
    )
    .with_event_features(CHROMIUM_CDP_EVENT_FEATURES);
}

#[cfg(feature = "full")]
impl NativeBackendAccess<ChromiumCdpBackend> for CdpLocatorService {
    type Native = dyn CdpClient;

    fn native_backend(&self) -> &Self::Native {
        self.native_cdp_client()
    }
}

#[derive(Clone)]
pub struct CdpNavigationService {
    inner: LeafCdpNavigationService,
}

/// Creates per-target navigation services that share one production
/// concurrency counter and resource declaration.
#[derive(Clone)]
pub struct CdpNavigationServiceFactory {
    inner: LeafCdpNavigationServiceFactory,
}

impl CdpNavigationServiceFactory {
    pub fn new_with_production_resources(
        profile: CdpNavigationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        LeafCdpNavigationServiceFactory::new_with_production_resources(profile, resources)
            .map(|inner| Self { inner })
    }

    pub fn with_network_idle_delay_and_production_resources(
        network_idle_delay: Duration,
        profile: CdpNavigationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        LeafCdpNavigationServiceFactory::with_network_idle_delay_and_production_resources(
            network_idle_delay,
            profile,
            resources,
        )
        .map(|inner| Self { inner })
    }

    pub fn bind(&self, client: Arc<dyn CdpClient>) -> CdpNavigationService {
        CdpNavigationService {
            inner: self.inner.bind(client),
        }
    }
}

impl CdpNavigationService {
    /// Delegate explicit Frame initiation to the isolated navigation adapter.
    /// Route lifetime and document/lifecycle completion remain caller-owned.
    pub fn begin_frame_navigation<'a>(
        &'a self,
        frame_id: &'a str,
        params: BeginNavigationParams,
    ) -> Pin<
        Box<
            dyn Future<
                    Output = Result<BeginNavigationResult, OperationError<NavigationErrorDetails>>,
                > + Send
                + 'a,
        >,
    > {
        self.inner.begin_frame_navigation(frame_id, params)
    }

    pub fn new(client: Arc<dyn CdpClient>) -> Self {
        Self {
            inner: LeafCdpNavigationService::new(client),
        }
    }

    pub fn with_network_idle_delay(
        client: Arc<dyn CdpClient>,
        network_idle_delay: Duration,
    ) -> Self {
        Self {
            inner: LeafCdpNavigationService::with_network_idle_delay(client, network_idle_delay),
        }
    }

    pub fn with_limits(client: Arc<dyn CdpClient>, limits: CdpNavigationLimits) -> Self {
        Self {
            inner: LeafCdpNavigationService::with_limits(client, limits),
        }
    }

    pub fn with_network_idle_delay_and_limits(
        client: Arc<dyn CdpClient>,
        network_idle_delay: Duration,
        limits: CdpNavigationLimits,
    ) -> Self {
        Self {
            inner: LeafCdpNavigationService::with_network_idle_delay_and_limits(
                client,
                network_idle_delay,
                limits,
            ),
        }
    }

    pub fn new_with_production_resources(
        client: Arc<dyn CdpClient>,
        profile: CdpNavigationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        LeafCdpNavigationService::new_with_production_resources(client, profile, resources)
            .map(|inner| Self { inner })
    }

    pub fn with_network_idle_delay_and_production_resources(
        client: Arc<dyn CdpClient>,
        network_idle_delay: Duration,
        profile: CdpNavigationProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, ProductionResourceError> {
        LeafCdpNavigationService::with_network_idle_delay_and_production_resources(
            client,
            network_idle_delay,
            profile,
            resources,
        )
        .map(|inner| Self { inner })
    }

    pub const fn limits(&self) -> CdpNavigationLimits {
        self.inner.limits()
    }
}

impl NativeBackendAccess<ChromiumCdpBackend> for CdpNavigationService {
    type Native = dyn CdpClient;

    fn native_backend(&self) -> &Self::Native {
        NativeBackendAccess::<ChromiumCdpNavigationBackend>::native_backend(&self.inner)
    }
}

#[cfg(feature = "history-navigation")]
mod navigation_history;
mod navigation_reload;

impl Service<BeginNavigationParams> for CdpNavigationService {
    type Response = BeginNavigationResult;
    type Error = OperationError<NavigationErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: BeginNavigationParams) -> Self::Future<'_> {
        self.inner.call(params)
    }
}

impl Service<GotoParams> for CdpNavigationService {
    type Response = GotoResult;
    type Error = OperationError<NavigationErrorDetails>;
    type Future<'a>
        = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'a>>
    where
        Self: 'a;

    fn call(&self, params: GotoParams) -> Self::Future<'_> {
        self.inner.call(params)
    }
}

impl CommandCancellation<BeginNavigation> for CdpNavigationService {
    type Future<'a>
        = Pin<
        Box<dyn Future<Output = Result<(), OperationError<NavigationErrorDetails>>> + Send + 'a>,
    >
    where
        Self: 'a;

    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        <LeafCdpNavigationService as CommandCancellation<BeginNavigation>>::cancel(
            &self.inner,
            context,
            reason,
        )
    }
}

impl CommandCancellation<Goto> for CdpNavigationService {
    type Future<'a>
        = Pin<
        Box<dyn Future<Output = Result<(), OperationError<NavigationErrorDetails>>> + Send + 'a>,
    >
    where
        Self: 'a;

    fn cancel(&self, context: OperationContext, reason: CancellationReason) -> Self::Future<'_> {
        <LeafCdpNavigationService as CommandCancellation<Goto>>::cancel(
            &self.inner,
            context,
            reason,
        )
    }
}

#[cfg(all(test, not(feature = "full")))]
mod minimal_tests {
    use pwright_browser_backend::BrowserBackend;

    use super::ChromiumCdpBackend;

    #[test]
    fn minimal_backend_descriptor_contains_only_navigation() {
        // Arrange
        let expected = ["navigation.begin", "navigation.goto", "navigation.reload"];

        // Act
        let descriptor = ChromiumCdpBackend::DESCRIPTOR;
        let actual = descriptor
            .features
            .iter()
            .map(|feature| feature.id.as_str())
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(actual, expected);
        assert!(descriptor.event_features.is_empty());
    }
}

#[cfg(all(test, feature = "full"))]
mod tests {
    use std::sync::Arc;
    use std::time::Duration;

    use pwright_browser_backend::{
        BackendImplementation, BackendManifest, BrowserBackend, BrowserEngine, NativeBackendAccess,
    };
    use pwright_browser_model::BROWSER_OBJECT_KIND_CATALOG;
    use pwright_cap_content::ContentCapability;
    use pwright_cap_context_creation::ContextCreationCapability;
    use pwright_cap_context_lifecycle::ContextLifecycleCapability;
    use pwright_cap_dom_content::DomContentCapability;
    use pwright_cap_evaluation::EvaluationCapability;
    use pwright_cap_handle_evaluation::HandleEvaluationCapability;
    use pwright_cap_handle_lifecycle::HandleLifecycleCapability;
    use pwright_cap_locator::LocatorCapability;
    use pwright_cap_locator_screenshot::LocatorScreenshotCapability;
    use pwright_cap_navigation::{BeginNavigation, NavigationCapability};
    use pwright_cap_navigation::{BeginNavigationParams, GotoParams, WaitUntil};
    use pwright_cap_network_events::NetworkEventsCapability;
    use pwright_cap_page_creation::PageCreationCapability;
    use pwright_cap_page_events::PageEventsCapability;
    use pwright_cap_page_lifecycle::PageLifecycleCapability;
    use pwright_cdp::CdpEvent;
    use pwright_core::{
        CallId, CallMetadata, CancellationReason, Command, CommandCancellation, MethodTag,
        ObjectId, ObjectKindTag, OperationContext, ProductionResourceError,
        ProductionResourceLedger, RegistryBuilder, ResourceClass, ResourceLimits,
        ResourceRequirements, ResourceScope,
    };
    use pwright_core::{FailureKind, Service};
    use pwright_fake::FakeCdpClient;
    use pwright_runtime::{DirectDispatcher, RuntimeLimits};
    use serde_json::json;

    use super::{
        CDP_BEGIN_NAVIGATION_FEATURE, CDP_BROWSER_CONTEXT_CLOSE_FEATURE,
        CDP_BROWSER_CONTEXT_NEW_PAGE_FEATURE, CDP_BROWSER_CONTEXT_PAGE_EVENT_FEATURE,
        CDP_BROWSER_NEW_CONTEXT_FEATURE, CDP_EVALUATE_HANDLE_FEATURE, CDP_EVALUATE_VALUE_FEATURE,
        CDP_FRAME_CONTENT_FEATURE, CDP_GOTO_FEATURE, CDP_HANDLE_DISPOSE_FEATURE,
        CDP_HANDLE_EVALUATE_FEATURE, CDP_HANDLE_EVALUATE_HANDLE_FEATURE,
        CDP_HANDLE_GET_PROPERTIES_FEATURE, CDP_HANDLE_GET_PROPERTY_FEATURE,
        CDP_HANDLE_JSON_VALUE_FEATURE, CDP_LOCATOR_ALL_INNER_TEXTS_FEATURE,
        CDP_LOCATOR_ALL_TEXT_CONTENTS_FEATURE, CDP_LOCATOR_BLUR_FEATURE, CDP_LOCATOR_CLICK_FEATURE,
        CDP_LOCATOR_COUNT_FEATURE, CDP_LOCATOR_FILL_FEATURE, CDP_LOCATOR_FOCUS_FEATURE,
        CDP_LOCATOR_GET_ATTRIBUTE_FEATURE, CDP_LOCATOR_HOVER_FEATURE,
        CDP_LOCATOR_INNER_HTML_FEATURE, CDP_LOCATOR_INNER_TEXT_FEATURE,
        CDP_LOCATOR_INPUT_VALUE_FEATURE, CDP_LOCATOR_IS_CHECKED_FEATURE,
        CDP_LOCATOR_IS_EDITABLE_FEATURE, CDP_LOCATOR_IS_ENABLED_FEATURE,
        CDP_LOCATOR_IS_VISIBLE_FEATURE, CDP_LOCATOR_PRESS_FEATURE, CDP_LOCATOR_SCREENSHOT_FEATURE,
        CDP_LOCATOR_SCROLL_INTO_VIEW_IF_NEEDED_FEATURE, CDP_LOCATOR_SELECT_OPTION_FEATURE,
        CDP_LOCATOR_SET_CHECKED_FEATURE, CDP_LOCATOR_TEXT_CONTENT_FEATURE,
        CDP_LOCATOR_WAIT_FOR_FEATURE, CDP_NETWORK_REQUEST_EVENT_FEATURE,
        CDP_NETWORK_REQUEST_FAILED_EVENT_FEATURE, CDP_NETWORK_REQUEST_FINISHED_EVENT_FEATURE,
        CDP_NETWORK_RESPONSE_EVENT_FEATURE, CDP_PAGE_CLOSE_FEATURE, CDP_PAGE_LOAD_EVENT_FEATURE,
        CDP_PAGE_POPUP_EVENT_FEATURE, CDP_PAGE_RUN_BEFORE_UNLOAD_FEATURE, CHROMIUM_CDP_BACKEND_ID,
        CHROMIUM_CDP_PROTOCOL_ID, CdpNavigationLimits, CdpNavigationProductionProfile,
        CdpNavigationService, CdpNavigationServiceFactory, ChromiumCdpBackend,
    };

    const NAVIGATION_SCOPE: ResourceScope = ResourceScope::Object(ObjectId::new(77));

    fn event(method: &str, params: serde_json::Value) -> CdpEvent {
        CdpEvent::new(method.to_string(), params, None)
    }

    async fn wait_until_navigate_was_sent(client: &FakeCdpClient) {
        for _ in 0..100 {
            if client
                .method_names()
                .iter()
                .any(|method| method == "Page.navigate")
            {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("Page.navigate was not sent");
    }

    fn small_navigation_limits() -> CdpNavigationLimits {
        CdpNavigationLimits::new(256, 64, 4, 4, 256, 64)
    }

    fn navigation_resources(profile: CdpNavigationProductionProfile) -> ProductionResourceLedger {
        let requirement = profile
            .requirements()
            .class(ResourceClass::BackendStateBytes);
        let limits = ResourceLimits::deny_all(1, profile.max_concurrent_operations()).with_class(
            ResourceClass::BackendStateBytes,
            requirement.global,
            requirement.per_scope,
        );
        ProductionResourceLedger::new(limits, profile.requirements())
            .expect("navigation production requirements should fit")
    }

    #[test]
    fn cdp_manifest_validates_all_declared_features_before_browser_interaction() {
        // Arrange
        let mut capabilities = RegistryBuilder::new();
        capabilities
            .register::<ContextLifecycleCapability>()
            .expect("context lifecycle should register");
        capabilities
            .register::<HandleLifecycleCapability>()
            .expect("handle lifecycle should register");
        capabilities
            .register::<HandleEvaluationCapability>()
            .expect("handle evaluation should register");
        capabilities
            .register::<ContextCreationCapability>()
            .expect("context creation should register");
        capabilities
            .register::<NavigationCapability>()
            .expect("navigation should register");
        capabilities
            .register::<EvaluationCapability>()
            .expect("evaluation should register");
        capabilities
            .register::<ContentCapability>()
            .expect("content should register");
        capabilities
            .register::<DomContentCapability>()
            .expect("DOM content should register");
        capabilities
            .register::<LocatorCapability>()
            .expect("locator should register");
        capabilities
            .register::<LocatorScreenshotCapability>()
            .expect("Locator screenshot should register");
        capabilities
            .register::<PageEventsCapability>()
            .expect("page events should register");
        capabilities
            .register::<NetworkEventsCapability>()
            .expect("network events should register");
        capabilities
            .register::<PageLifecycleCapability>()
            .expect("page lifecycle should register");
        capabilities
            .register::<PageCreationCapability>()
            .expect("page creation should register");
        let capabilities = capabilities.build().expect("registry should validate");
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        let service = CdpNavigationService::new(client.clone());

        // Act
        let manifest = BackendManifest::for_backend::<ChromiumCdpBackend>(&capabilities)
            .expect("CDP backend manifest should validate");
        let native = NativeBackendAccess::<ChromiumCdpBackend>::native_backend(&service);

        // Assert
        assert_eq!(manifest.descriptor().id, CHROMIUM_CDP_BACKEND_ID);
        assert_eq!(manifest.descriptor().protocol, CHROMIUM_CDP_PROTOCOL_ID);
        assert_eq!(manifest.descriptor().engine, BrowserEngine::Chromium);
        assert_eq!(manifest.features().len(), 42);
        assert!(manifest.supports(super::CDP_RELOAD_FEATURE));
        assert_eq!(manifest.event_features().len(), 7);
        assert!(manifest.supports(CDP_PAGE_LOAD_EVENT_FEATURE));
        assert!(manifest.supports(CDP_BROWSER_CONTEXT_PAGE_EVENT_FEATURE));
        assert!(manifest.supports(CDP_PAGE_POPUP_EVENT_FEATURE));
        assert!(manifest.supports(CDP_NETWORK_REQUEST_EVENT_FEATURE));
        assert!(manifest.supports(CDP_NETWORK_RESPONSE_EVENT_FEATURE));
        assert!(manifest.supports(CDP_NETWORK_REQUEST_FINISHED_EVENT_FEATURE));
        assert!(manifest.supports(CDP_NETWORK_REQUEST_FAILED_EVENT_FEATURE));
        assert!(manifest.supports(CDP_BROWSER_CONTEXT_CLOSE_FEATURE));
        assert!(manifest.supports(CDP_BROWSER_NEW_CONTEXT_FEATURE));
        assert!(manifest.supports(CDP_BROWSER_CONTEXT_NEW_PAGE_FEATURE));
        assert!(manifest.supports(CDP_PAGE_CLOSE_FEATURE));
        assert!(manifest.supports(CDP_PAGE_RUN_BEFORE_UNLOAD_FEATURE));
        assert!(manifest.supports(CDP_EVALUATE_VALUE_FEATURE));
        assert!(manifest.supports(CDP_EVALUATE_HANDLE_FEATURE));
        assert!(manifest.supports(CDP_HANDLE_EVALUATE_FEATURE));
        assert!(manifest.supports(CDP_HANDLE_EVALUATE_HANDLE_FEATURE));
        assert!(manifest.supports(CDP_HANDLE_JSON_VALUE_FEATURE));
        assert!(manifest.supports(CDP_HANDLE_GET_PROPERTY_FEATURE));
        assert!(manifest.supports(CDP_HANDLE_GET_PROPERTIES_FEATURE));
        assert!(manifest.supports(CDP_HANDLE_DISPOSE_FEATURE));
        assert_eq!(
            manifest
                .require(CDP_FRAME_CONTENT_FEATURE)
                .expect("Frame content should be declared")
                .implementation,
            BackendImplementation::Emulated
        );
        assert_eq!(
            manifest
                .require(CDP_BEGIN_NAVIGATION_FEATURE)
                .expect("begin navigation should be declared")
                .implementation,
            BackendImplementation::Native
        );
        assert_eq!(
            manifest
                .require(CDP_GOTO_FEATURE)
                .expect("goto should be declared")
                .implementation,
            BackendImplementation::Emulated
        );
        assert!(manifest.supports(CDP_LOCATOR_CLICK_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_ALL_INNER_TEXTS_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_ALL_TEXT_CONTENTS_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_BLUR_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_COUNT_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_FILL_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_FOCUS_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_PRESS_FEATURE));
        assert!(manifest.supports(super::CDP_LOCATOR_PRESS_SEQUENTIALLY_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_SCROLL_INTO_VIEW_IF_NEEDED_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_SELECT_OPTION_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_SET_CHECKED_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_GET_ATTRIBUTE_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_HOVER_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_INNER_HTML_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_INNER_TEXT_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_INPUT_VALUE_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_IS_CHECKED_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_IS_EDITABLE_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_IS_ENABLED_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_IS_VISIBLE_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_TEXT_CONTENT_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_WAIT_FOR_FEATURE));
        assert!(manifest.supports(CDP_LOCATOR_SCREENSHOT_FEATURE));
        assert_eq!(
            ChromiumCdpBackend::DESCRIPTOR.features,
            manifest.descriptor().features
        );
        assert_eq!(native.event_session_id(), None);
        assert!(client.method_names().is_empty());
    }

    #[tokio::test]
    async fn service_invokes_page_navigate_through_the_cdp_adapter() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        let service = CdpNavigationService::new(client.clone());

        // Act
        let result = service
            .call(BeginNavigationParams::new("https://example.com"))
            .await
            .expect("fake navigation should succeed");

        // Assert
        assert_eq!(result.frame_id, "fake-frame");
        assert!(result.same_document);
        assert!(!result.is_download);
        assert_eq!(client.method_names(), vec!["Page.navigate"]);
    }

    #[tokio::test]
    async fn typed_command_dispatches_end_to_end_without_a_wire_codec() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        let dispatcher = DirectDispatcher::with_object_kinds(
            CdpNavigationService::new(client.clone()),
            RuntimeLimits::new(1, 1),
            BROWSER_OBJECT_KIND_CATALOG,
        )
        .expect("browser object catalog should be valid");
        let page_id = ObjectId::new(7);
        dispatcher
            .register_object(page_id, None, ObjectKindTag::new(3))
            .expect("page object should register");
        let command = Command::<BeginNavigation>::new(
            CallId::new(11),
            page_id,
            CallMetadata::default(),
            BeginNavigationParams::new("https://example.com"),
        );

        // Act
        let result = dispatcher
            .call(command)
            .await
            .expect("in-process dispatch is infallible");

        // Assert
        assert_eq!(result.call_id(), CallId::new(11));
        assert_eq!(
            result
                .into_outcome()
                .expect("fake CDP navigation should succeed")
                .frame_id,
            "fake-frame"
        );
        assert_eq!(client.method_names(), vec!["Page.navigate"]);
        assert_eq!(dispatcher.pending_call_count(), Ok(0));
    }

    #[tokio::test]
    async fn goto_waits_for_load_and_returns_the_main_resource_response() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        client.set_navigate_response(json!({
            "frameId": "frame-1",
            "loaderId": "loader-1"
        }));
        let service = CdpNavigationService::new(client.clone());
        let task = tokio::spawn(async move {
            service
                .call(GotoParams::new("https://example.com/missing"))
                .await
        });
        wait_until_navigate_was_sent(&client).await;

        // Act
        client.send_event(event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "request-1",
                "loaderId": "loader-1",
                "frameId": "frame-1",
                "type": "Document",
                "request": {"url": "https://example.com/missing"}
            }),
        ));
        client.send_event(event(
            "Network.responseReceived",
            json!({
                "requestId": "request-1",
                "loaderId": "loader-1",
                "frameId": "frame-1",
                "type": "Document",
                "response": {
                    "url": "https://example.com/missing",
                    "status": 404,
                    "statusText": "Not Found",
                    "headers": {"x-z": "last", "content-type": "text/html"}
                }
            }),
        ));
        client.send_event(event(
            "Page.frameNavigated",
            json!({"frame": {
                "id": "frame-1",
                "loaderId": "loader-1",
                "url": "https://example.com/missing"
            }}),
        ));
        assert!(!task.is_finished());
        client.send_event(event(
            "Page.lifecycleEvent",
            json!({
                "frameId": "frame-1",
                "loaderId": "loader-1",
                "name": "load"
            }),
        ));
        let result = task
            .await
            .expect("goto task should join")
            .expect("HTTP status pages should navigate successfully");

        // Assert
        assert_eq!(result.url, "https://example.com/missing");
        assert!(!result.same_document);
        let response = result
            .response
            .expect("HTTP navigation should return response");
        assert_eq!(response.status, 404);
        assert_eq!(response.headers[0].name, "content-type");
        assert_eq!(response.headers[1].name, "x-z");
        assert_eq!(
            client.method_names(),
            vec![
                "Page.enable",
                "Page.setLifecycleEventsEnabled",
                "Network.enable",
                "Page.navigate",
            ]
        );
    }

    #[tokio::test]
    async fn same_document_goto_waits_for_commit_and_returns_no_response() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        client.set_navigate_response(json!({"frameId": "frame-1"}));
        let service = CdpNavigationService::new(client.clone());
        let task = tokio::spawn(async move {
            service
                .call(
                    GotoParams::new("https://example.com/#next")
                        .with_referrer("https://referrer.example"),
                )
                .await
        });
        wait_until_navigate_was_sent(&client).await;

        // Act
        assert!(!task.is_finished());
        client.send_event(event(
            "Page.navigatedWithinDocument",
            json!({"frameId": "frame-1", "url": "https://example.com/#next"}),
        ));
        let result = task
            .await
            .expect("goto task should join")
            .expect("same-document navigation should succeed");

        // Assert
        assert!(result.same_document);
        assert!(result.loader_id.is_none());
        assert!(result.response.is_none());
        assert_eq!(result.url, "https://example.com/#next");
        let navigate = client
            .calls()
            .into_iter()
            .find(|(method, _)| method == "Page.navigate")
            .expect("navigate call should be recorded");
        assert_eq!(
            navigate.1,
            vec![json!({
                "url": "https://example.com/#next",
                "referrer": "https://referrer.example"
            })]
        );
    }

    #[tokio::test]
    async fn goto_distinguishes_dom_content_loaded_from_later_load() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        client.set_navigate_response(json!({
            "frameId": "frame-1",
            "loaderId": "loader-1"
        }));
        let service = CdpNavigationService::new(client.clone());
        let task = tokio::spawn(async move {
            service
                .call(
                    GotoParams::new("data:text/html,ready")
                        .with_wait_until(WaitUntil::DomContentLoaded),
                )
                .await
        });
        wait_until_navigate_was_sent(&client).await;
        client.send_event(event(
            "Page.frameNavigated",
            json!({"frame": {
                "id": "frame-1",
                "loaderId": "loader-1",
                "url": "data:text/html,ready"
            }}),
        ));

        // Act
        client.send_event(event(
            "Page.lifecycleEvent",
            json!({
                "frameId": "frame-1",
                "loaderId": "loader-1",
                "name": "DOMContentLoaded"
            }),
        ));
        let result = task
            .await
            .expect("goto task should join")
            .expect("DOMContentLoaded should satisfy the selected wait state");

        // Assert
        assert_eq!(result.url, "data:text/html,ready");
        assert!(result.response.is_none());
    }

    #[tokio::test]
    async fn network_idle_requires_no_matching_requests_for_the_idle_window() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        client.set_navigate_response(json!({
            "frameId": "frame-1",
            "loaderId": "loader-1"
        }));
        let service = CdpNavigationService::with_network_idle_delay(
            client.clone(),
            Duration::from_millis(10),
        );
        let task = tokio::spawn(async move {
            service
                .call(
                    GotoParams::new("https://example.com").with_wait_until(WaitUntil::NetworkIdle),
                )
                .await
        });
        wait_until_navigate_was_sent(&client).await;
        client.send_event(event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "request-1",
                "loaderId": "loader-1",
                "frameId": "frame-1",
                "type": "Document",
                "request": {"url": "https://example.com"}
            }),
        ));
        client.send_event(event(
            "Network.responseReceived",
            json!({
                "requestId": "request-1",
                "loaderId": "loader-1",
                "frameId": "frame-1",
                "type": "Document",
                "response": {
                    "url": "https://example.com",
                    "status": 200,
                    "statusText": "OK",
                    "headers": {}
                }
            }),
        ));
        client.send_event(event(
            "Page.frameNavigated",
            json!({"frame": {
                "id": "frame-1",
                "loaderId": "loader-1",
                "url": "https://example.com"
            }}),
        ));
        client.send_event(event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "request-2",
                "loaderId": "child-loader",
                "frameId": "child-frame",
                "type": "Script",
                "request": {"url": "https://example.com/app.js"}
            }),
        ));

        // Act
        tokio::time::sleep(Duration::from_millis(15)).await;
        assert!(!task.is_finished());
        client.send_event(event(
            "Network.loadingFinished",
            json!({"requestId": "request-1"}),
        ));
        tokio::time::sleep(Duration::from_millis(15)).await;
        assert!(!task.is_finished());
        client.send_event(event(
            "Network.loadingFinished",
            json!({"requestId": "request-2"}),
        ));
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(!task.is_finished());
        let result = task
            .await
            .expect("goto task should join")
            .expect("idle navigation should succeed");

        // Assert
        assert_eq!(result.response.expect("response should exist").status, 200);
    }

    #[tokio::test]
    async fn started_competing_error_document_interrupts_the_expected_navigation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        client.set_navigate_response(json!({
            "frameId": "frame-1",
            "loaderId": "loader-1"
        }));
        let service = CdpNavigationService::new(client.clone());
        let task = tokio::spawn(async move {
            service
                .call(
                    GotoParams::new("https://example.com/first").with_wait_until(WaitUntil::Commit),
                )
                .await
        });
        wait_until_navigate_was_sent(&client).await;
        client.send_event(event(
            "Page.frameStartedNavigating",
            json!({
                "frameId": "frame-1",
                "loaderId": "loader-2",
                "url": "https://example.com/second"
            }),
        ));

        // Act
        client.send_event(event(
            "Page.frameNavigated",
            json!({"frame": {
                "id": "frame-1",
                "loaderId": "loader-2",
                "url": "chrome-error://chromewebdata/"
            }}),
        ));
        let error = task
            .await
            .expect("goto task should join")
            .expect_err("replacement navigation should interrupt the first");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(error.message().contains("interrupted"));
    }

    #[tokio::test]
    async fn prior_error_document_commit_does_not_interrupt_newer_pending_navigation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        client.set_navigate_response(json!({
            "frameId": "frame-1",
            "loaderId": "loader-new"
        }));
        let service = CdpNavigationService::new(client.clone());
        let task = tokio::spawn(async move {
            service
                .call(GotoParams::new("https://example.com/new").with_wait_until(WaitUntil::Commit))
                .await
        });
        wait_until_navigate_was_sent(&client).await;

        // Act
        client.send_event(event(
            "Page.frameNavigated",
            json!({"frame": {
                "id": "frame-1",
                "loaderId": "loader-old-error",
                "url": "chrome-error://chromewebdata/"
            }}),
        ));
        tokio::task::yield_now().await;
        assert!(!task.is_finished());
        client.send_event(event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "request-new",
                "loaderId": "loader-new",
                "frameId": "frame-1",
                "type": "Document",
                "request": {"url": "https://example.com/new"}
            }),
        ));
        client.send_event(event(
            "Network.responseReceived",
            json!({
                "requestId": "request-new",
                "loaderId": "loader-new",
                "frameId": "frame-1",
                "type": "Document",
                "response": {
                    "url": "https://example.com/new",
                    "status": 200,
                    "statusText": "OK",
                    "headers": {}
                }
            }),
        ));
        client.send_event(event(
            "Page.frameNavigated",
            json!({"frame": {
                "id": "frame-1",
                "loaderId": "loader-new",
                "url": "https://example.com/new"
            }}),
        ));
        let result = task
            .await
            .expect("goto task should join")
            .expect("stale error-page commit should be ignored");

        // Assert
        assert_eq!(result.loader_id.as_deref(), Some("loader-new"));
        assert_eq!(result.url, "https://example.com/new");
        assert_eq!(result.response.expect("response should exist").status, 200);
    }

    #[tokio::test]
    async fn newer_document_request_still_interrupts_expected_navigation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        client.set_navigate_response(json!({
            "frameId": "frame-1",
            "loaderId": "loader-expected"
        }));
        let service = CdpNavigationService::new(client.clone());
        let task = tokio::spawn(async move {
            service
                .call(
                    GotoParams::new("https://example.com/expected")
                        .with_wait_until(WaitUntil::Commit),
                )
                .await
        });
        wait_until_navigate_was_sent(&client).await;
        for (request_id, loader_id, url) in [
            (
                "request-expected",
                "loader-expected",
                "https://example.com/expected",
            ),
            (
                "request-replacement",
                "loader-replacement",
                "https://example.com/replacement",
            ),
        ] {
            client.send_event(event(
                "Network.requestWillBeSent",
                json!({
                    "requestId": request_id,
                    "loaderId": loader_id,
                    "frameId": "frame-1",
                    "type": "Document",
                    "request": {"url": url}
                }),
            ));
        }

        // Act
        client.send_event(event(
            "Page.frameNavigated",
            json!({"frame": {
                "id": "frame-1",
                "loaderId": "loader-replacement",
                "url": "chrome-error://chromewebdata/"
            }}),
        ));
        let error = task
            .await
            .expect("goto task should join")
            .expect_err("newer replacement should interrupt expected navigation");

        // Assert
        assert_eq!(error.kind(), FailureKind::Backend);
        assert!(error.message().contains("interrupted"));
    }

    #[tokio::test]
    async fn begin_navigation_forwards_referrer_to_cdp() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        let service = CdpNavigationService::new(client.clone());
        let params = BeginNavigationParams::new("https://example.com")
            .with_referrer("https://referrer.example");

        // Act
        let result = service
            .call(params)
            .await
            .expect("CDP supports Page.navigate referrer");

        // Assert
        assert!(result.same_document);
        assert_eq!(client.method_names(), vec!["Page.navigate"]);
        assert_eq!(
            client.calls()[0].1,
            vec![json!({
                "url": "https://example.com",
                "referrer": "https://referrer.example"
            })]
        );
    }

    #[tokio::test]
    async fn navigation_cancellation_uses_page_stop_loading() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        let service = CdpNavigationService::new(client.clone());
        let context = OperationContext::new(
            CallId::new(11),
            ObjectId::new(7),
            MethodTag::new(0x0001_0001),
            None,
        );

        // Act
        <CdpNavigationService as CommandCancellation<BeginNavigation>>::cancel(
            &service,
            context,
            CancellationReason::DeadlineExceeded,
        )
        .await
        .expect("fake stop-loading should succeed");

        // Assert
        assert_eq!(client.method_names(), vec!["Page.stopLoading"]);
    }

    #[test]
    fn navigation_production_profile_scales_the_operation_peak_by_local_concurrency() {
        // Arrange
        let limits = small_navigation_limits();
        let profile = CdpNavigationProductionProfile::new(NAVIGATION_SCOPE, limits, 3);

        // Act
        let requirement = profile
            .requirements()
            .class(ResourceClass::BackendStateBytes);

        // Assert
        assert_eq!(profile.resource_scope(), NAVIGATION_SCOPE);
        assert_eq!(profile.limits(), limits);
        assert_eq!(profile.max_concurrent_operations(), 3);
        assert_eq!(
            requirement.global,
            limits.maximum_retained_bytes_per_operation() * 3
        );
        assert_eq!(requirement.per_scope, requirement.global);
        assert_eq!(profile.requirements().scopes, 1);
        assert_eq!(profile.requirements().leases, 3);
    }

    #[tokio::test]
    async fn unpolled_navigation_future_owns_and_drop_releases_production_capacity() {
        // Arrange
        let profile =
            CdpNavigationProductionProfile::new(NAVIGATION_SCOPE, small_navigation_limits(), 1);
        let resources = navigation_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        let service = CdpNavigationService::new_with_production_resources(
            client.clone(),
            profile,
            &resources,
        )
        .expect("declared navigation should construct");

        // Act
        let pending = service.call(BeginNavigationParams::new("https://example.com"));
        let retained = resources
            .metrics()
            .expect("metrics should remain readable")
            .class(ResourceClass::BackendStateBytes)
            .current;
        drop(pending);

        // Assert
        assert_eq!(retained, profile.maximum_retained_bytes_per_operation());
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should remain readable")
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
        assert!(client.method_names().is_empty());
    }

    #[tokio::test]
    async fn factory_bound_pages_share_navigation_concurrency_without_borrowing_extra_capacity() {
        // Arrange
        let profile =
            CdpNavigationProductionProfile::new(NAVIGATION_SCOPE, small_navigation_limits(), 1);
        let per_operation = profile.maximum_retained_bytes_per_operation();
        let resources = ProductionResourceLedger::new(
            ResourceLimits::deny_all(1, 2).with_class(
                ResourceClass::BackendStateBytes,
                per_operation * 2,
                per_operation * 2,
            ),
            profile.requirements(),
        )
        .expect("larger shared ledger should satisfy the profile");
        let first_client = Arc::new(FakeCdpClient::from_html("<title>one</title>"));
        let second_client = Arc::new(FakeCdpClient::from_html("<title>two</title>"));
        let factory =
            CdpNavigationServiceFactory::new_with_production_resources(profile, &resources)
                .expect("declared navigation factory should construct");
        let first_page = factory.bind(first_client.clone());
        let second_page = factory.bind(second_client.clone());
        let first = first_page.call(BeginNavigationParams::new("https://example.com/one"));

        // Act
        let error = second_page
            .call(BeginNavigationParams::new("https://example.com/two"))
            .await
            .expect_err("second operation must hit the profile-local limit");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(error.message().contains("concurrency limit"));
        assert!(first_client.method_names().is_empty());
        assert!(second_client.method_names().is_empty());
        drop(first);
    }

    #[tokio::test]
    async fn request_flood_is_rejected_before_exceeding_the_inflight_set_limit() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        client.set_navigate_response(json!({
            "frameId": "frame-1",
            "loaderId": "loader-1"
        }));
        let limits = CdpNavigationLimits::new(256, 64, 1, 4, 256, 64);
        let service = CdpNavigationService::with_network_idle_delay_and_limits(
            client.clone(),
            Duration::ZERO,
            limits,
        );
        let task = tokio::spawn(async move {
            service
                .call(
                    GotoParams::new("https://example.com").with_wait_until(WaitUntil::NetworkIdle),
                )
                .await
        });
        wait_until_navigate_was_sent(&client).await;
        client.send_event(event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "request-1",
                "loaderId": "loader-1",
                "frameId": "frame-1",
                "type": "Document",
                "request": {"url": "https://example.com"}
            }),
        ));

        // Act
        client.send_event(event(
            "Network.requestWillBeSent",
            json!({
                "requestId": "request-2",
                "loaderId": "child-loader",
                "frameId": "child-frame",
                "type": "Script",
                "request": {"url": "https://example.com/app.js"}
            }),
        ));
        let error = task
            .await
            .expect("goto task should join")
            .expect_err("second retained request should exceed the local limit");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(error.message().contains("in-flight network request count"));
    }

    #[tokio::test]
    async fn oversized_response_headers_are_rejected_before_owned_response_creation() {
        // Arrange
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        client.set_navigate_response(json!({
            "frameId": "frame-1",
            "loaderId": "loader-1"
        }));
        let limits = CdpNavigationLimits::new(256, 64, 4, 1, 256, 64);
        let service = CdpNavigationService::with_limits(client.clone(), limits);
        let task =
            tokio::spawn(async move { service.call(GotoParams::new("https://example.com")).await });
        wait_until_navigate_was_sent(&client).await;

        // Act
        client.send_event(event(
            "Network.responseReceived",
            json!({
                "loaderId": "loader-1",
                "frameId": "frame-1",
                "type": "Document",
                "response": {
                    "url": "https://example.com",
                    "status": 200,
                    "statusText": "OK",
                    "headers": {"a": "one", "b": "two"}
                }
            }),
        ));
        let error = task
            .await
            .expect("goto task should join")
            .expect_err("header count must be checked before copying response state");

        // Assert
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert!(error.message().contains("response header count"));
    }

    #[tokio::test]
    async fn completed_navigation_releases_its_production_operation_lease() {
        // Arrange
        let profile =
            CdpNavigationProductionProfile::new(NAVIGATION_SCOPE, small_navigation_limits(), 1);
        let resources = navigation_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        let service =
            CdpNavigationService::new_with_production_resources(client, profile, &resources)
                .expect("declared navigation should construct");

        // Act
        service
            .call(BeginNavigationParams::new("https://example.com"))
            .await
            .expect("begin navigation should complete");

        // Assert
        let metrics = resources
            .metrics()
            .expect("metrics should remain readable")
            .class(ResourceClass::BackendStateBytes);
        assert_eq!(metrics.current, 0);
        assert_eq!(metrics.peak, profile.maximum_retained_bytes_per_operation());
    }

    #[tokio::test]
    async fn dropping_polled_goto_releases_production_state_capacity() {
        // Arrange
        let profile =
            CdpNavigationProductionProfile::new(NAVIGATION_SCOPE, small_navigation_limits(), 1);
        let resources = navigation_resources(profile);
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));
        client.set_navigate_response(json!({
            "frameId": "frame-1",
            "loaderId": "loader-1"
        }));
        let service = CdpNavigationService::new_with_production_resources(
            client.clone(),
            profile,
            &resources,
        )
        .expect("declared navigation should construct");
        let task =
            tokio::spawn(async move { service.call(GotoParams::new("https://example.com")).await });
        wait_until_navigate_was_sent(&client).await;
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should remain readable")
                .class(ResourceClass::BackendStateBytes)
                .current,
            profile.maximum_retained_bytes_per_operation()
        );

        // Act
        task.abort();
        let join = task.await;

        // Assert
        assert!(
            join.expect_err("aborted navigation should not complete")
                .is_cancelled()
        );
        assert_eq!(
            resources
                .metrics()
                .expect("metrics should remain readable")
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );
    }

    #[test]
    fn omitted_navigation_profile_fails_during_production_construction() {
        // Arrange
        let profile =
            CdpNavigationProductionProfile::new(NAVIGATION_SCOPE, small_navigation_limits(), 1);
        let requirement = profile
            .requirements()
            .class(ResourceClass::BackendStateBytes);
        let limits = ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::BackendStateBytes,
            requirement.global,
            requirement.per_scope,
        );
        let resources = ProductionResourceLedger::new(limits, ResourceRequirements::none())
            .expect("empty plan should be valid");
        let client = Arc::new(FakeCdpClient::from_html("<title>test</title>"));

        // Act
        let error = match CdpNavigationService::new_with_production_resources(
            client, profile, &resources,
        ) {
            Ok(_) => panic!("omitted navigation profile must fail closed"),
            Err(error) => error,
        };

        // Assert
        assert!(matches!(
            error,
            ProductionResourceError::InsufficientGlobal {
                class: ResourceClass::BackendStateBytes,
                ..
            }
        ));
    }
}
