pub mod client_trait;
pub mod connection;
pub mod domains;
pub mod event_fanout;
pub mod events;
pub mod generated;
pub mod session;

pub use client_trait::{CdpClient, SessionFactory};
pub use connection::{
    CdpConnection, CdpConnectionProductionProfile, CdpEventSubscriptionError,
    CdpProductionConnectError,
};
pub use domains::browser::DownloadBehavior;
pub use domains::input::{
    KeyEventType, KeyboardEventParams, MouseButton, MouseEventType, TouchEventType,
};
pub use events::{CdpEvent, CdpEventReceiver, CdpEventRecvError, CdpEventTryRecvError};
pub use session::{CdpSession, CdpSessionFactory};
