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

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils;

pub use browser::{
    Browser, BrowserConfig, CdpTabCloser, SUPPORTED_SCHEMES, TabCloser, TabHandle,
    is_supported_scheme, rewrite_ws_url,
};
pub use evaluate::{FromEvalJson, FromEvalResult};
pub use snapshot::{A11yNode, RefCache};
pub use tab::Tab;

/// Re-export CdpClient for downstream Rust users.
pub use pwright_cdp::CdpClient;
