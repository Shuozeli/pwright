pub mod client_trait;
pub mod connection;
pub mod domains;
pub mod event_admission;
pub mod event_fanout;
pub mod events;
pub mod generated;
pub mod session;

pub use client_trait::{CdpClient, SessionFactory};
#[cfg(feature = "cdp-handshake-headers")]
pub use connection::{
    CDP_HANDSHAKE_HEADER_MAX_BATCH_BYTES, CDP_HANDSHAKE_HEADER_MAX_COUNT,
    CDP_HANDSHAKE_HEADER_MAX_NAME_BYTES, CDP_HANDSHAKE_HEADER_MAX_VALUE_BYTES, CdpHandshakeHeaders,
    CdpHandshakeHeadersError,
};
pub use connection::{
    CdpConnection, CdpConnectionProductionProfile, CdpEventSubscriptionError,
    CdpProductionConnectError,
};
pub use domains::browser::DownloadBehavior;
pub use domains::input::{
    KeyEventType, KeyboardEventParams, MouseButton, MouseEventType, TouchEventType,
};
pub use event_admission::CdpEventAdmissionPolicy;
pub use events::{CdpEvent, CdpEventReceiver, CdpEventRecvError, CdpEventTryRecvError};
pub use session::{CdpSession, CdpSessionFactory};
