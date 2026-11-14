//! Narrow, opt-in remote Browser operations over one supervised Playwright
//! Server connection. This crate does not launch a browser, implement language
//! bindings, or claim a complete Context/Page API.
//!
//! Only empty `Browser.newContext` options, `BrowserContext.newPage`, and
//! empty-option `BrowserContext.close` are supported by default. The
//! `frame-title` feature adds exactly `Frame.title` through a Page's adopted
//! main Frame. Callers provide checked metadata and own richer option policy.
//! A result becomes a typed handle only after its correlated command reply and
//! a newly-created, correctly parented object in the supervisor's GUID graph.

use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::sync::Arc;

use pwright_playwright_server_client_core::{MAX_GUID_BYTES, RemoteError, ReplyOutcome};
use pwright_playwright_server_session::{
    RemoteBrowserSupervisor, RemoteObjectSnapshot, SupervisorError,
};
use serde_json::{Value, json};

pub const MAX_API_NAME_BYTES: usize = 256;

/// Required, caller-observed metadata; no clock is read inside this crate.
#[derive(Clone, Eq, PartialEq)]
pub struct OperationMetadata {
    api_name: String,
    wall_time_ms: u64,
}

impl Debug for OperationMetadata {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("OperationMetadata")
            .field("api_name", &"<redacted>")
            .field("wall_time_ms", &self.wall_time_ms)
            .finish()
    }
}

impl OperationMetadata {
    pub fn new(api_name: &str, wall_time_ms: u64) -> Result<Self, BrowserOperationError> {
        if api_name.is_empty()
            || api_name.len() > MAX_API_NAME_BYTES
            || !api_name.bytes().all(|byte| (0x21..=0x7e).contains(&byte))
        {
            return Err(BrowserOperationError::InvalidMetadata);
        }
        Ok(Self {
            api_name: api_name.to_owned(),
            wall_time_ms,
        })
    }

    fn wire_value(&self) -> Value {
        json!({
            "wallTime": self.wall_time_ms,
            "apiName": self.api_name,
            "internal": false,
        })
    }
}

/// The only context options this capability currently accepts. There is no
/// silent dropping of unknown user options; richer typed options need a new
/// explicitly composed capability.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EmptyContextOptions;

#[derive(Clone, PartialEq)]
pub enum BrowserOperationError {
    InvalidMetadata,
    Disconnected,
    Supervisor(SupervisorError),
    Remote(RemoteError),
    InvalidResult(&'static str),
}

impl Debug for BrowserOperationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMetadata => formatter.write_str("InvalidMetadata"),
            Self::Disconnected => formatter.write_str("Disconnected"),
            Self::Supervisor(_) => formatter.write_str("Supervisor(<redacted>)"),
            Self::Remote(_) => formatter.write_str("Remote(<redacted>)"),
            Self::InvalidResult(reason) => formatter
                .debug_tuple("InvalidResult")
                .field(reason)
                .finish(),
        }
    }
}

impl Display for BrowserOperationError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMetadata => {
                formatter.write_str("invalid Playwright Server operation metadata")
            }
            Self::Disconnected => formatter.write_str("remote Browser is disconnected"),
            Self::Supervisor(_) => formatter.write_str("remote Browser operation failed"),
            Self::Remote(_) => formatter.write_str("remote Browser rejected operation"),
            Self::InvalidResult(reason) => write!(
                formatter,
                "invalid Playwright Server operation result: {reason}"
            ),
        }
    }
}

impl Error for BrowserOperationError {}

/// A Browser protocol handle tied to exactly one supervised connection.
#[derive(Clone)]
pub struct RemoteBrowser {
    supervisor: Arc<RemoteBrowserSupervisor>,
    created_sequence: u64,
}

impl Debug for RemoteBrowser {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RemoteBrowser")
            .field("guid", &self.guid())
            .finish()
    }
}

impl RemoteBrowser {
    pub fn new(supervisor: Arc<RemoteBrowserSupervisor>) -> Result<Self, BrowserOperationError> {
        let browser = require_live(&supervisor, supervisor.guid(), "Browser", None)?;
        Ok(Self {
            supervisor,
            created_sequence: browser.created_sequence,
        })
    }

    pub fn guid(&self) -> &str {
        self.supervisor.guid()
    }

    pub fn is_connected(&self) -> bool {
        self.supervisor.is_connected()
    }

    pub async fn new_context(
        &self,
        _options: EmptyContextOptions,
        metadata: &OperationMetadata,
    ) -> Result<RemoteContext, BrowserOperationError> {
        require_generation(
            &self.supervisor,
            self.guid(),
            "Browser",
            None,
            self.created_sequence,
            "browser GUID was recycled",
        )?;
        let fence = self.supervisor.last_created_sequence();
        let result = self
            .supervisor
            .call_for_generation(
                self.guid(),
                self.created_sequence,
                "newContext",
                json!({}),
                metadata.wire_value(),
            )
            .await
            .map_err(BrowserOperationError::Supervisor)?;
        let guid = channel_guid(result, "context")?;
        let object = require_new_result(
            &self.supervisor,
            &guid,
            "BrowserContext",
            self.guid(),
            fence,
        )?;
        require_generation(
            &self.supervisor,
            self.guid(),
            "Browser",
            None,
            self.created_sequence,
            "browser GUID was recycled",
        )?;
        Ok(RemoteContext {
            supervisor: Arc::clone(&self.supervisor),
            guid,
            browser_guid: self.guid().to_owned(),
            browser_created_sequence: self.created_sequence,
            created_sequence: object.created_sequence,
        })
    }
}

/// A Context GUID retained with its owning connection and Browser GUID.
#[derive(Clone)]
pub struct RemoteContext {
    supervisor: Arc<RemoteBrowserSupervisor>,
    guid: String,
    browser_guid: String,
    browser_created_sequence: u64,
    created_sequence: u64,
}

impl Debug for RemoteContext {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RemoteContext")
            .field("guid", &self.guid)
            .finish()
    }
}

impl RemoteContext {
    pub fn guid(&self) -> &str {
        &self.guid
    }

    pub fn created_sequence(&self) -> u64 {
        self.created_sequence
    }

    /// Borrow the owning supervisor for a point-in-time graph snapshot.
    pub fn supervisor(&self) -> &RemoteBrowserSupervisor {
        &self.supervisor
    }

    pub fn is_open(&self) -> bool {
        self.supervisor.is_connected()
            && matches_generation(
                &self.supervisor,
                &self.guid,
                "BrowserContext",
                Some(&self.browser_guid),
                self.created_sequence,
            )
            && matches_generation(
                &self.supervisor,
                &self.browser_guid,
                "Browser",
                None,
                self.browser_created_sequence,
            )
    }

    pub async fn new_page(
        &self,
        metadata: &OperationMetadata,
    ) -> Result<RemotePage, BrowserOperationError> {
        require_generation(
            &self.supervisor,
            &self.guid,
            "BrowserContext",
            Some(&self.browser_guid),
            self.created_sequence,
            "context GUID was recycled",
        )?;
        require_generation(
            &self.supervisor,
            &self.browser_guid,
            "Browser",
            None,
            self.browser_created_sequence,
            "browser GUID was recycled",
        )?;
        let fence = self.supervisor.last_created_sequence();
        let result = self
            .supervisor
            .call_for_generation(
                &self.guid,
                self.created_sequence,
                "newPage",
                json!({}),
                metadata.wire_value(),
            )
            .await
            .map_err(BrowserOperationError::Supervisor)?;
        let guid = channel_guid(result, "page")?;
        let object = require_new_result(&self.supervisor, &guid, "Page", &self.guid, fence)?;
        require_generation(
            &self.supervisor,
            &self.guid,
            "BrowserContext",
            Some(&self.browser_guid),
            self.created_sequence,
            "context GUID was recycled",
        )?;
        require_generation(
            &self.supervisor,
            &self.browser_guid,
            "Browser",
            None,
            self.browser_created_sequence,
            "browser GUID was recycled",
        )?;
        Ok(RemotePage {
            supervisor: Arc::clone(&self.supervisor),
            guid,
            context_guid: self.guid.clone(),
            context_created_sequence: self.created_sequence,
            browser_guid: self.browser_guid.clone(),
            browser_created_sequence: self.browser_created_sequence,
            created_sequence: object.created_sequence,
            #[cfg(feature = "frame-title")]
            main_frame: pinned_main_frame(&self.supervisor, &object, fence),
        })
    }

    /// Close exactly this Context generation. The server may dispose the
    /// Context and its descendants before replying, so no post-reply live
    /// check is valid; a correlated successful reply is the close authority.
    pub async fn close(&self, metadata: &OperationMetadata) -> Result<(), BrowserOperationError> {
        require_generation(
            &self.supervisor,
            &self.guid,
            "BrowserContext",
            Some(&self.browser_guid),
            self.created_sequence,
            "context GUID was recycled",
        )?;
        require_generation(
            &self.supervisor,
            &self.browser_guid,
            "Browser",
            None,
            self.browser_created_sequence,
            "browser GUID was recycled",
        )?;
        let outcome = self
            .supervisor
            .call_for_generation(
                &self.guid,
                self.created_sequence,
                "close",
                json!({}),
                metadata.wire_value(),
            )
            .await
            .map_err(BrowserOperationError::Supervisor)?;
        match outcome {
            ReplyOutcome::Result(value)
                if value.as_object().is_some_and(|object| object.is_empty()) =>
            {
                Ok(())
            }
            ReplyOutcome::Result(_) => Err(BrowserOperationError::InvalidResult(
                "context close returned a nonempty result",
            )),
            ReplyOutcome::Error(error) => Err(BrowserOperationError::Remote(error)),
        }
    }
}

/// A Page GUID retained with its owning connection and Context GUID.
#[derive(Clone)]
pub struct RemotePage {
    supervisor: Arc<RemoteBrowserSupervisor>,
    guid: String,
    context_guid: String,
    context_created_sequence: u64,
    browser_guid: String,
    browser_created_sequence: u64,
    created_sequence: u64,
    #[cfg(feature = "frame-title")]
    main_frame: Option<PinnedMainFrame>,
}

#[cfg(feature = "frame-title")]
#[derive(Clone)]
struct PinnedMainFrame {
    guid: String,
    created_sequence: u64,
    adoption_epoch: u64,
}

impl Debug for RemotePage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RemotePage")
            .field("guid", &self.guid)
            .finish()
    }
}

impl RemotePage {
    pub fn guid(&self) -> &str {
        &self.guid
    }

    pub fn created_sequence(&self) -> u64 {
        self.created_sequence
    }

    /// Send the v1.58 `Frame.title` command to the Page's original main Frame.
    /// The main Frame must have been adopted under this Page before the
    /// `newPage` reply. The caller's Page, Context, Browser, and Frame
    /// generations, ancestry, adoption epoch, and closed state are fenced by
    /// the supervisor actor immediately before send and at the ordered reply.
    #[cfg(feature = "frame-title")]
    pub async fn title(
        &self,
        metadata: &OperationMetadata,
    ) -> Result<String, BrowserOperationError> {
        self.require_lineage()?;
        let frame = self
            .main_frame
            .as_ref()
            .ok_or(BrowserOperationError::InvalidResult(
                "page has no adopted main Frame",
            ))?;
        require_generation(
            &self.supervisor,
            &frame.guid,
            "Frame",
            Some(&self.guid),
            frame.created_sequence,
            "main Frame GUID was recycled",
        )?;
        let current_epoch = self
            .supervisor
            .object_with_frame_title_state(&frame.guid)
            .map(|(_, epoch, _)| epoch)
            .ok_or(BrowserOperationError::InvalidResult(
                "target GUID is not live",
            ))?;
        if current_epoch != frame.adoption_epoch {
            return Err(BrowserOperationError::InvalidResult(
                "main Frame adoption changed",
            ));
        }
        let outcome = self
            .supervisor
            .call_for_frame_title(
                [
                    (&frame.guid, frame.created_sequence),
                    (&self.guid, self.created_sequence),
                    (&self.context_guid, self.context_created_sequence),
                    (&self.browser_guid, self.browser_created_sequence),
                ],
                frame.adoption_epoch,
                metadata.wire_value(),
            )
            .await
            .map_err(BrowserOperationError::Supervisor)?;
        let value = match outcome {
            ReplyOutcome::Result(value) => value,
            ReplyOutcome::Error(error) => return Err(BrowserOperationError::Remote(error)),
        };
        let title = match value.as_object() {
            Some(result) if result.len() == 1 => result.get("value").and_then(Value::as_str),
            _ => None,
        }
        .ok_or(BrowserOperationError::InvalidResult(
            "Frame.title did not return exactly {value: string}",
        ))?;
        Ok(title.to_owned())
    }

    #[cfg(feature = "frame-title")]
    fn require_lineage(&self) -> Result<(), BrowserOperationError> {
        require_generation(
            &self.supervisor,
            &self.guid,
            "Page",
            Some(&self.context_guid),
            self.created_sequence,
            "page GUID was recycled",
        )?;
        require_generation(
            &self.supervisor,
            &self.context_guid,
            "BrowserContext",
            Some(&self.browser_guid),
            self.context_created_sequence,
            "context GUID was recycled",
        )?;
        require_generation(
            &self.supervisor,
            &self.browser_guid,
            "Browser",
            None,
            self.browser_created_sequence,
            "browser GUID was recycled",
        )?;
        Ok(())
    }

    /// Install both opt-in Page network subscriptions before publishing this
    /// handle to a language caller. The wire replies are the subscription
    /// fence; a rejected or malformed reply must not be treated as armed.
    #[cfg(feature = "page-events")]
    pub async fn enable_network_events(
        &self,
        metadata: &OperationMetadata,
    ) -> Result<(), BrowserOperationError> {
        for event in ["request", "response"] {
            require_generation(
                &self.supervisor,
                &self.guid,
                "Page",
                Some(&self.context_guid),
                self.created_sequence,
                "page GUID was recycled",
            )?;
            require_generation(
                &self.supervisor,
                &self.context_guid,
                "BrowserContext",
                Some(&self.browser_guid),
                self.context_created_sequence,
                "context GUID was recycled",
            )?;
            require_generation(
                &self.supervisor,
                &self.browser_guid,
                "Browser",
                None,
                self.browser_created_sequence,
                "browser GUID was recycled",
            )?;
            let outcome = self
                .supervisor
                .call_for_generation(
                    &self.guid,
                    self.created_sequence,
                    "updateSubscription",
                    json!({ "event": event, "enabled": true }),
                    metadata.wire_value(),
                )
                .await
                .map_err(BrowserOperationError::Supervisor)?;
            match outcome {
                ReplyOutcome::Result(value)
                    if value.as_object().is_some_and(|object| object.is_empty()) => {}
                ReplyOutcome::Result(_) => {
                    return Err(BrowserOperationError::InvalidResult(
                        "Page subscription returned a nonempty result",
                    ));
                }
                ReplyOutcome::Error(error) => return Err(BrowserOperationError::Remote(error)),
            }
        }
        Ok(())
    }

    pub fn is_open(&self) -> bool {
        self.supervisor.is_connected()
            && matches_generation(
                &self.supervisor,
                &self.guid,
                "Page",
                Some(&self.context_guid),
                self.created_sequence,
            )
            && matches_generation(
                &self.supervisor,
                &self.context_guid,
                "BrowserContext",
                Some(&self.browser_guid),
                self.context_created_sequence,
            )
            && matches_generation(
                &self.supervisor,
                &self.browser_guid,
                "Browser",
                None,
                self.browser_created_sequence,
            )
    }
}

#[cfg(feature = "frame-title")]
fn pinned_main_frame(
    supervisor: &RemoteBrowserSupervisor,
    page: &RemoteObjectSnapshot,
    command_fence: u64,
) -> Option<PinnedMainFrame> {
    let channel = page.initializer.get("mainFrame")?.as_object()?;
    if channel.len() != 1 {
        return None;
    }
    let guid = channel.get("guid")?.as_str()?;
    if guid.is_empty() || guid.len() > MAX_GUID_BYTES || guid.chars().any(char::is_control) {
        return None;
    }
    let (frame, adoption_epoch, closed) = supervisor.object_with_frame_title_state(guid)?;
    if frame.object_type != "Frame"
        || closed
        || frame.parent.as_deref() != Some(page.guid.as_str())
        || frame.created_sequence <= command_fence
        || frame.created_sequence >= page.created_sequence
    {
        return None;
    }
    Some(PinnedMainFrame {
        guid: guid.to_owned(),
        created_sequence: frame.created_sequence,
        adoption_epoch,
    })
}

fn snapshot_with_closed(
    supervisor: &RemoteBrowserSupervisor,
    guid: &str,
) -> Option<(RemoteObjectSnapshot, bool)> {
    #[cfg(feature = "frame-title")]
    {
        supervisor
            .object_with_frame_title_state(guid)
            .map(|(object, _, closed)| (object, closed))
    }
    #[cfg(all(not(feature = "frame-title"), feature = "close-state"))]
    {
        supervisor.object_with_close_state(guid)
    }
    #[cfg(not(any(feature = "frame-title", feature = "close-state")))]
    {
        supervisor.object(guid).map(|object| (object, false))
    }
}

fn matches_generation(
    supervisor: &RemoteBrowserSupervisor,
    guid: &str,
    object_type: &str,
    parent: Option<&str>,
    created_sequence: u64,
) -> bool {
    snapshot_with_closed(supervisor, guid).is_some_and(|(object, closed)| {
        !closed
            && object.object_type == object_type
            && parent.is_none_or(|expected| object.parent.as_deref() == Some(expected))
            && object.created_sequence == created_sequence
    })
}

fn require_generation(
    supervisor: &RemoteBrowserSupervisor,
    guid: &str,
    object_type: &str,
    parent: Option<&str>,
    created_sequence: u64,
    recycle_reason: &'static str,
) -> Result<RemoteObjectSnapshot, BrowserOperationError> {
    let object = require_live(supervisor, guid, object_type, parent)?;
    if object.created_sequence != created_sequence {
        return Err(BrowserOperationError::InvalidResult(recycle_reason));
    }
    Ok(object)
}

fn channel_guid(
    outcome: ReplyOutcome,
    field: &'static str,
) -> Result<String, BrowserOperationError> {
    let value = match outcome {
        ReplyOutcome::Result(value) => value,
        ReplyOutcome::Error(error) => return Err(BrowserOperationError::Remote(error)),
    };
    let guid = value
        .get(field)
        .and_then(Value::as_object)
        .and_then(|channel| channel.get("guid"))
        .and_then(Value::as_str)
        .filter(|guid| {
            !guid.is_empty() && guid.len() <= MAX_GUID_BYTES && !guid.chars().any(char::is_control)
        })
        .ok_or(BrowserOperationError::InvalidResult(
            "required channel GUID missing or invalid",
        ))?;
    Ok(guid.to_owned())
}

fn require_live(
    supervisor: &RemoteBrowserSupervisor,
    guid: &str,
    object_type: &str,
    parent: Option<&str>,
) -> Result<RemoteObjectSnapshot, BrowserOperationError> {
    if !supervisor.is_connected() {
        return Err(BrowserOperationError::Disconnected);
    }
    let (object, closed) = snapshot_with_closed(supervisor, guid).ok_or(
        BrowserOperationError::InvalidResult("target GUID is not live"),
    )?;
    if closed {
        return Err(BrowserOperationError::InvalidResult(
            "target GUID is closed",
        ));
    }
    if object.object_type != object_type {
        return Err(BrowserOperationError::InvalidResult(
            "target GUID has wrong object type",
        ));
    }
    if parent.is_some_and(|expected| object.parent.as_deref() != Some(expected)) {
        return Err(BrowserOperationError::InvalidResult(
            "target GUID has wrong parent",
        ));
    }
    Ok(object)
}

fn require_new_result(
    supervisor: &RemoteBrowserSupervisor,
    guid: &str,
    object_type: &str,
    parent: &str,
    fence: u64,
) -> Result<RemoteObjectSnapshot, BrowserOperationError> {
    let object = require_live(supervisor, guid, object_type, Some(parent))?;
    if object.created_sequence <= fence {
        return Err(BrowserOperationError::InvalidResult(
            "result GUID predates command",
        ));
    }
    Ok(object)
}

#[cfg(test)]
mod tests;
