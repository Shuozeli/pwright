pub mod actions;
pub mod browser;
pub mod clock;
pub mod content;
pub mod cookies;
pub mod evaluate;
mod keys;
pub mod navigate;
pub mod playwright;
pub mod snapshot;
pub mod tab;

pub mod test_utils;

pub use browser::{Browser, BrowserConfig};
pub use snapshot::{A11yNode, RefCache};
pub use tab::Tab;

/// Re-export CdpClient for downstream Rust users.
pub use pwright_cdp::CdpClient;
