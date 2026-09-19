//! Raw Page keyboard operations, independent of selectors and command ownership.
use std::{fmt, future::Future};

use pwright_cdp::connection::CdpError;

use crate::{CdpInputTransaction, KeyTokens};

/// Input errors do not retain or duplicate the caller's key expression.
#[derive(Debug)]
pub enum KeyInputError<'a> {
    UnknownKey(&'a str),
    Native(CdpError),
}

impl fmt::Display for KeyInputError<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKey(key) => write!(formatter, "Unknown key: \"{key}\""),
            Self::Native(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for KeyInputError<'_> {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnknownKey(_) => None,
            Self::Native(error) => Some(error),
        }
    }
}

impl From<CdpError> for KeyInputError<'_> {
    fn from(error: CdpError) -> Self {
        Self::Native(error)
    }
}

/// Presses a bounded borrowed chord without focusing or selecting an element.
///
/// Each down/up resolves its alias against the current Page modifier state;
/// releases occur in reverse order. The injected delay owns no timer here.
/// Partial errors are not retried or automatically released. The caller must
/// finish the transaction and use its selected cancellation/retirement policy.
/// An unknown later token can therefore leave earlier keys pressed, like the
/// pinned Playwright raw Keyboard operation (distinct from Locator validation).
/// Use [`KeyTokens::parse_raw_keyboard`] to preserve its empty-tail error timing.
pub async fn press_keys<'a, Wait: Future<Output = Result<(), CdpError>>>(
    transaction: &mut CdpInputTransaction,
    keys: &KeyTokens<'a>,
    is_mac: bool,
    delay: impl FnOnce() -> Wait,
) -> Result<(), KeyInputError<'a>> {
    for key in keys.as_slice() {
        transaction.key_down_named(key, is_mac).await?;
    }
    delay().await?;
    for key in keys.as_slice().iter().rev() {
        transaction.key_up_named(key, is_mac).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
