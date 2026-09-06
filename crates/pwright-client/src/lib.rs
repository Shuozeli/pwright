//! Typed client-side handles for Pwright capabilities.
//!
//! Handles dispatch typed commands directly through `Service`; this crate
//! selects no codec, transport, async runtime, or browser backend.

mod call_id;
mod lifecycle;
mod locator;
mod remote;

pub use call_id::{AtomicCallIdSource, CallIdSource, CallIdSourceError};
pub use lifecycle::{
    ClientLimits, ClientObjectMetrics, ClientStateError, ClientTerminalReasonError,
    DEFAULT_LIVE_OBJECT_LIMIT, DEFAULT_TERMINAL_REASON_BYTES,
};
pub use locator::{Locator, LocatorScopeKind, LocatorScopeMismatch};
pub use remote::{
    AnyHandle, Artifact, ArtifactKind, Browser, BrowserContext, BrowserContextKind, BrowserKind,
    Client, CommandFuture, ElementHandle, ElementHandleKind, Frame, FrameKind, HandleCreationError,
    JsHandle, JsHandleKind, Page, PageKind, Remote, RemoteAccessError, RemoteInitializer,
    RemoteKind, Request, RequestKind, Response, ResponseKind, Stream, StreamKind,
};
