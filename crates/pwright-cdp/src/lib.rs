pub mod client_trait;
pub mod connection;
pub mod domains;
pub mod events;
pub mod generated;
pub mod session;

pub use client_trait::{CdpClient, SessionFactory};
pub use connection::CdpConnection;
pub use domains::input::{KeyEventType, MouseButton, MouseEventType, TouchEventType};
pub use events::CdpEvent;
pub use session::{CdpSession, CdpSessionFactory};
