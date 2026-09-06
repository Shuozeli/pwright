//! Isolated Chromium CDP Page input transaction foundation for Pwright.
//!
//! The leaf owns Playwright-shaped US keyboard resolution and one bounded,
//! serialized Page-local keyboard/mouse state machine. It owns no selector,
//! Locator command, runtime, transport, task, browser session, or process.

mod input;
mod keyboard;

pub use input::{CdpInputCoordinator, CdpInputTransaction, CdpModifiers};
pub use keyboard::{KeyChordParseError, KeyTokens, ResolvedKey, UnknownKey, resolve_key};
