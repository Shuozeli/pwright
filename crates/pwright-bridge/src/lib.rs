pub mod actions;
pub mod browser;
pub mod chrome_http;
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

pub use browser::{Browser, BrowserConfig, CdpTabCloser, HttpTabCloser, TabCloser, TabHandle};
pub use chrome_http::ChromeHttpClient;
pub use evaluate::{FromEvalJson, FromEvalResult};
pub use snapshot::{A11yNode, RefCache};
pub use tab::Tab;

/// Re-export CdpClient for downstream Rust users.
pub use pwright_cdp::CdpClient;
