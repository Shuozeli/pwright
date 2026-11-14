//! Chromium CDP adapter for retained JavaScript Handle evaluation.
//!
//! The leaf reuses the value-evaluation backend, retains only bounded native
//! Handle state, and owns Handle-local evaluation and property cleanup. It
//! selects no navigation, Locator, client, runtime, provider, or browser.

mod element_adoption;
mod frame_owner_element;
mod handle_evaluation;
mod handle_operations;
mod owner_frame;
mod program;
pub use owner_frame::CdpHandleOwnerFrameService;
pub use frame_owner_element::CdpFrameOwnerElementService;
pub use program::CdpHandleEvaluationProgram;

pub use element_adoption::{
    CdpAdoptedElementHandle, CdpAdoptedElementHandleBatch, CdpElementHandleAdoptionBatchRequest,
    CdpElementHandleAdoptionBatchService, CdpElementHandleAdoptionInstallError,
    CdpElementHandleAdoptionProductionProfile, CdpElementHandleAdoptionRequest,
    CdpElementHandleAdoptionService, CdpElementHandleAdoptionSetupError,
};
pub use handle_evaluation::{
    CdpEvaluatedHandle, CdpHandleEvaluationLimits, CdpHandleEvaluationRequest,
    CdpHandleEvaluationService, CdpHandleKind,
};
pub use handle_operations::{
    CdpHandlePropertiesService, CdpHandlePropertyLimitError, CdpHandlePropertyLimits,
    CdpHandleValueEvaluationService,
};
