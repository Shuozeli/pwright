//! Frame-bound, weak-session Locators dispatch existing typed Rust commands.

pub mod action;
pub(crate) mod keyboard;

use std::{
    str::FromStr,
    sync::{Arc, Weak},
};

use pwright_cap_locator::{
    AllInnerTexts, AllInnerTextsParams, AllTextContents, AllTextContentsParams, Count, CountParams,
    GetAttribute, GetAttributeParams, InnerHtml, InnerHtmlParams, InnerText, InnerTextParams,
    InputValue, InputValueParams, IsChecked, IsCheckedParams, IsEditable, IsEditableParams,
    IsEnabled, IsEnabledParams, IsVisible, IsVisibleParams, TextContent, TextContentParams,
};
use pwright_core::ObjectId;

use crate::{
    HostError, HostSession,
    locator_query::{LocatorQuery, QueryCombination},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LocatorRead {
    Count,
    AllInnerTexts,
    AllTextContents,
    GetAttribute,
    InnerHtml,
    InnerText,
    InputValue,
    IsChecked,
    IsEditable,
    IsEnabled,
    IsDisabled,
    IsVisible,
    IsHidden,
    TextContent,
}

impl FromStr for LocatorRead {
    type Err = HostError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "count" => Self::Count,
            "allInnerTexts" => Self::AllInnerTexts,
            "allTextContents" => Self::AllTextContents,
            "getAttribute" => Self::GetAttribute,
            "innerHTML" => Self::InnerHtml,
            "innerText" => Self::InnerText,
            "inputValue" => Self::InputValue,
            "isChecked" => Self::IsChecked,
            "isEditable" => Self::IsEditable,
            "isEnabled" => Self::IsEnabled,
            "isDisabled" => Self::IsDisabled,
            "isVisible" => Self::IsVisible,
            "isHidden" => Self::IsHidden,
            "textContent" => Self::TextContent,
            _ => {
                return Err(HostError::UnsupportedOperation(
                    "unknown Locator read operation".into(),
                ));
            }
        })
    }
}

pub enum LocatorValue {
    Count(u32),
    Bool(bool),
    Text(Option<String>),
    Texts(Vec<String>),
}

#[derive(Clone)]
pub struct HostLocator {
    host: Weak<HostSession>,
    frame: ObjectId,
    query: LocatorQuery,
}

impl HostSession {
    pub fn locator(
        self: &Arc<Self>,
        frame: &str,
        query: LocatorQuery,
    ) -> Result<HostLocator, HostError> {
        let frame = frame
            .parse::<u64>()
            .ok()
            .filter(|id| *id != 0)
            .map(ObjectId::new)
            .ok_or_else(|| HostError::InvalidArgument("invalid portable Frame ID".into()))?;
        Ok(HostLocator {
            host: Arc::downgrade(self),
            frame,
            query,
        })
    }
}

impl HostLocator {
    #[cfg(feature = "file-input")]
    pub(crate) fn file_input_query(
        &self,
    ) -> Result<
        (
            ObjectId,
            pwright_cap_locator::Selector,
            crate::locator_query::QueryLease,
        ),
        HostError,
    > {
        let (selector, lease) = self.query.command_selector()?;
        Ok((self.frame, selector, lease))
    }
    /// Strictly select/adopt first, then invoke the caller's value-conversion
    /// continuation exactly once. Native ownership stays inside one controlled
    /// Frame command through conversion, evaluation and acknowledged release.
    /// The callback and its future have a 4 KiB inline bound; external retained
    /// Node references/buffers remain the caller's separately admitted budget.
    pub async fn evaluate_with<F, Converted>(
        &self,
        timeout_ms: u32,
        convert: F,
    ) -> Result<crate::evaluation::EvaluationValue, HostError>
    where
        F: FnOnce() -> Converted + Send + 'static,
        Converted: std::future::Future<Output = Result<crate::evaluation::EvaluateParams, HostError>>
            + Send
            + 'static,
    {
        let host = self.host()?;
        let (selector, _query_lease) = self.query.command_selector()?;
        host.locator_evaluate(self.frame, selector, timeout_ms, convert)
            .await
    }

    pub fn copy(&self) -> Result<Self, HostError> {
        Ok(self.derive(self.query.copy()?))
    }

    pub fn host(&self) -> Result<Arc<HostSession>, HostError> {
        self.host
            .upgrade()
            .filter(|host| host.is_connected())
            .ok_or(HostError::TargetClosed)
    }

    fn derive(&self, query: LocatorQuery) -> Self {
        Self {
            host: self.host.clone(),
            frame: self.frame,
            query,
        }
    }

    pub fn nth(&self, index: i32) -> Result<Self, HostError> {
        Ok(self.derive(self.query.nth(index)?))
    }

    pub fn visible(&self, visible: bool) -> Result<Self, HostError> {
        Ok(self.derive(self.query.visible(visible)?))
    }

    pub fn filter_text(&self, text: &str, exclude: bool) -> Result<Self, HostError> {
        Ok(self.derive(self.query.filter_text(text, exclude)?))
    }

    pub fn combine(&self, other: &Self, combination: QueryCombination) -> Result<Self, HostError> {
        if !Weak::ptr_eq(&self.host, &other.host) || self.frame != other.frame {
            return Err(HostError::Backend(
                "Locators must belong to the same frame.".into(),
            ));
        }
        Ok(self.derive(self.query.combine(&other.query, combination)?))
    }

    pub async fn read(
        &self,
        operation: LocatorRead,
        timeout_ms: u32,
        attribute: Option<String>,
    ) -> Result<LocatorValue, HostError> {
        let host = self.host()?;
        if (operation == LocatorRead::GetAttribute) != attribute.is_some() {
            return Err(HostError::InvalidArgument(
                "only getAttribute requires an attribute name".into(),
            ));
        }
        if attribute
            .as_ref()
            .is_some_and(|name| name.capacity() > 64 * 1024)
        {
            return Err(HostError::InvalidArgument(
                "attribute name exceeds 65536 bytes".into(),
            ));
        }
        // This admission covers the command-owned clone, separately from the
        // immutable recipe. Backend execution/cleanup has its own SelectorBytes
        // reservation and the Node boundary separately owns result settlement.
        let (selector, _query_lease) = self.query.command_selector()?;
        let frame = self.frame.get().to_string();
        let outcome = match operation {
            LocatorRead::Count => host
                .invoke::<Count>(&frame, CountParams::new(selector), 0)
                .await
                .map(LocatorValue::Count),
            LocatorRead::AllInnerTexts => host
                .invoke::<AllInnerTexts>(&frame, AllInnerTextsParams::new(selector), 0)
                .await
                .map(LocatorValue::Texts),
            LocatorRead::AllTextContents => host
                .invoke::<AllTextContents>(&frame, AllTextContentsParams::new(selector), 0)
                .await
                .map(LocatorValue::Texts),
            LocatorRead::GetAttribute => host
                .invoke::<GetAttribute>(
                    &frame,
                    GetAttributeParams::new(selector, attribute.expect("validated attribute")),
                    timeout_ms,
                )
                .await
                .map(LocatorValue::Text),
            LocatorRead::InnerHtml => host
                .invoke::<InnerHtml>(&frame, InnerHtmlParams::new(selector), timeout_ms)
                .await
                .map(|value| LocatorValue::Text(Some(value))),
            LocatorRead::InnerText => host
                .invoke::<InnerText>(&frame, InnerTextParams::new(selector), timeout_ms)
                .await
                .map(|value| LocatorValue::Text(Some(value))),
            LocatorRead::InputValue => host
                .invoke::<InputValue>(&frame, InputValueParams::new(selector), timeout_ms)
                .await
                .map(|value| LocatorValue::Text(Some(value))),
            LocatorRead::IsChecked => host
                .invoke::<IsChecked>(&frame, IsCheckedParams::new(selector), timeout_ms)
                .await
                .map(LocatorValue::Bool),
            LocatorRead::IsEditable => host
                .invoke::<IsEditable>(&frame, IsEditableParams::new(selector), timeout_ms)
                .await
                .map(LocatorValue::Bool),
            LocatorRead::IsEnabled | LocatorRead::IsDisabled => host
                .invoke::<IsEnabled>(&frame, IsEnabledParams::new(selector), timeout_ms)
                .await
                .map(|value| {
                    LocatorValue::Bool(if operation == LocatorRead::IsDisabled {
                        !value
                    } else {
                        value
                    })
                }),
            LocatorRead::IsVisible | LocatorRead::IsHidden => host
                .invoke::<IsVisible>(&frame, IsVisibleParams::new(selector), 0)
                .await
                .map(|value| {
                    LocatorValue::Bool(if operation == LocatorRead::IsHidden {
                        !value
                    } else {
                        value
                    })
                }),
            LocatorRead::TextContent => host
                .invoke::<TextContent>(&frame, TextContentParams::new(selector), timeout_ms)
                .await
                .map(LocatorValue::Text),
        };
        crate::completion::publish_command_result(
            outcome,
            host.is_connected(),
            host.flush_topology(),
        )
        .await
    }
}
