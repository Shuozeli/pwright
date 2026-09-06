pub mod builder;
pub mod client;
pub mod dom;
pub mod pwright;
pub mod selector;

pub use builder::parse_html;
pub use client::FakeCdpClient;
pub use pwright::FakePwright;
