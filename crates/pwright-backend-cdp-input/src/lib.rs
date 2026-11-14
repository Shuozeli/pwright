//! Isolated Chromium CDP Page input transaction foundation for Pwright.
//!
//! The leaf owns Playwright-shaped US keyboard resolution and one bounded,
//! serialized Page-local keyboard/mouse state machine. It owns no selector,
//! Locator command, runtime, transport, task, browser session, or process.

mod input;
mod interleaved;
mod key_down;
mod keyboard;
mod press;
mod production;
mod typing;

pub use input::{CdpInputCoordinator, CdpInputPolicy, CdpInputTransaction, CdpModifiers};
pub use interleaved::{CdpCheckedMouseDownError, CdpOwnedMousePress};
pub use interleaved::{
    CdpMouseDispatcher, DirectMouseDispatch, MouseDispatchKind, MouseDispatchOperation,
    MouseDispatchRequest, MouseLiveState, MouseStateReader,
};
pub use interleaved::{MouseButtonExecution, MouseClickExecution, MouseClickOptions};
pub use key_down::{CdpKeyDownInterceptor, KeyDownInterceptionFuture};
pub use keyboard::{KeyChordParseError, KeyTokens, ResolvedKey, UnknownKey, resolve_key};
pub use press::{KeyInputError, press_keys};
pub use production::{CdpInputAdmissionError, CdpInputProductionProfile, CdpInputServiceFactory};
pub use typing::type_text;
