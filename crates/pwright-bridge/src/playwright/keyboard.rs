//! Keyboard input — wraps CDP Input.dispatchKeyEvent.

use std::sync::Arc;

use pwright_cdp::CdpClient;
use pwright_cdp::connection::Result as CdpResult;

use crate::keys::get_key_def;

/// Playwright-compatible Keyboard.
///
/// ```rust,ignore
/// page.keyboard().press("Enter").await?;
/// page.keyboard().type_text("hello").await?;
/// ```
pub struct Keyboard {
    session: Arc<dyn CdpClient>,
}

impl Keyboard {
    pub(crate) fn new(session: Arc<dyn CdpClient>) -> Self {
        Self { session }
    }

    /// Press and release a key. Shorthand for `down(key)` + `up(key)`.
    pub async fn press(&self, key: &str) -> CdpResult<()> {
        self.down(key).await?;
        self.up(key).await?;
        Ok(())
    }

    /// Press a key down (does not release).
    pub async fn down(&self, key: &str) -> CdpResult<()> {
        if let Some(def) = get_key_def(key) {
            self.session
                .input_dispatch_key_event(
                    "rawKeyDown",
                    key,
                    def.code.as_ref(),
                    Some(def.virtual_key),
                )
                .await
        } else {
            // Unknown key — send as raw key name
            self.session
                .input_dispatch_key_event("rawKeyDown", key, key, None)
                .await
        }
    }

    /// Release a key.
    pub async fn up(&self, key: &str) -> CdpResult<()> {
        if let Some(def) = get_key_def(key) {
            self.session
                .input_dispatch_key_event("keyUp", key, def.code.as_ref(), Some(def.virtual_key))
                .await
        } else {
            self.session
                .input_dispatch_key_event("keyUp", key, key, None)
                .await
        }
    }

    /// Type text character by character with `Input.insertText`.
    pub async fn type_text(&self, text: &str) -> CdpResult<()> {
        for ch in text.chars() {
            self.session.input_insert_text(&ch.to_string()).await?;
        }
        Ok(())
    }

    /// Insert text at once (no key events, just inserts).
    pub async fn insert_text(&self, text: &str) -> CdpResult<()> {
        self.session.input_insert_text(text).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::MockCdpClient;

    #[tokio::test]
    async fn test_keyboard_press() {
        let mock = Arc::new(MockCdpClient::new());
        let kb = Keyboard::new(mock.clone());
        kb.press("Enter").await.unwrap();

        let methods = mock.method_names();
        assert_eq!(methods[0], "Input.dispatchKeyEvent");
        assert_eq!(methods[1], "Input.dispatchKeyEvent");
        assert_eq!(methods.len(), 2);
    }

    #[tokio::test]
    async fn test_keyboard_type_text() {
        let mock = Arc::new(MockCdpClient::new());
        let kb = Keyboard::new(mock.clone());
        kb.type_text("hi").await.unwrap();

        let calls = mock.calls_for("Input.insertText");
        assert_eq!(calls.len(), 2);
    }

    #[tokio::test]
    async fn test_keyboard_insert_text() {
        let mock = Arc::new(MockCdpClient::new());
        let kb = Keyboard::new(mock.clone());
        kb.insert_text("hello world").await.unwrap();

        let calls = mock.calls_for("Input.insertText");
        assert_eq!(calls.len(), 1);
    }
}
