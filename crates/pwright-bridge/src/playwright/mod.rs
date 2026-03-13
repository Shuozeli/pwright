//! Playwright-compatible high-level API.
//!
//! Provides `Page`, `Locator`, `Keyboard`, `Mouse`, and `Touchscreen` structs that wrap
//! the low-level bridge functions with a Playwright-compatible interface.
//!
//! ```rust,ignore
//! let page = Page::new(session);
//! page.goto("https://example.com", None).await?;
//! page.locator("button.submit").click(None).await?;
//! page.keyboard().press("Enter").await?;
//! ```

mod keyboard;
mod locator;
mod mouse;
mod page;
mod selectors;
mod touchscreen;

pub use keyboard::Keyboard;
pub use locator::{BoundingBox, Locator};
pub use mouse::{ClickOptions, Mouse};
pub use page::{GotoOptions, Page, ScreenshotOptions};
pub use touchscreen::Touchscreen;
