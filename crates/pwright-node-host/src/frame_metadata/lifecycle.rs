//! One pre-admitted dependent callback group in the existing publication queue.
//! Native producers, not collection diffs, determine which causes to stage.
use super::{
    CollectionMember, CollectionRevision, CollectionView, FrameChange, FrameChangeValue,
    FrameMetadata, FramePublicationDrain, FramePublications, HostError, MetadataState,
    PublicationDrainError, ResourceClass, ResourceLease, SCOPE,
};
#[cfg(any(feature = "frame-detach-events", feature = "frame-events"))]
use crate::frame_tree::FrameNode;
use crate::frame_tree::{FrameClosePublication, FrameTreeFactory, Storage};
use pwright_core::ObjectId;
use std::sync::{Arc, Weak};

mod delivery;
mod terminal;
pub(crate) use delivery::CreationDelivery;
pub(crate) use terminal::TerminalLifecycle;

#[cfg(any(feature = "console-events", feature = "console-history"))]
pub(crate) const MAX_CONSOLE_TYPE_BYTES: usize = 64;
#[cfg(any(feature = "console-events", feature = "console-history"))]
pub(crate) const MAX_CONSOLE_TEXT_BYTES: usize = 16 * 1024;
#[cfg(any(feature = "console-events", feature = "console-history"))]
pub(crate) const MAX_CONSOLE_ARGUMENT_BYTES: usize = 16 * 1024;
#[cfg(any(feature = "console-events", feature = "console-history"))]
pub(crate) const MAX_CONSOLE_LOCATION_URL_BYTES: usize = 16 * 1024;
#[cfg(feature = "page-errors")]
pub(crate) const MAX_PAGE_ERROR_NAME_BYTES: usize = 256;
#[cfg(feature = "page-errors")]
pub(crate) const MAX_PAGE_ERROR_MESSAGE_BYTES: usize = 16 * 1024;
#[cfg(feature = "page-errors")]
pub(crate) const MAX_PAGE_ERROR_STACK_BYTES: usize = 64 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageError {
    name: Box<str>,
    message: Box<str>,
    stack: Box<str>,
}

#[cfg(feature = "page-dialogs")]
pub struct PageDialog {
    id: u64,
    page: ObjectId,
    kind: crate::page_dialogs::DialogKind,
    message: Box<str>,
    default_value: Box<str>,
    action: Arc<crate::page_dialogs::DialogAction>,
}

#[cfg(feature = "file-chooser-events")]
pub struct PageFileChooser {
    id: u64,
    page: ObjectId,
    multiple: bool,
    action: Arc<crate::page_file_chooser::FileChooserAction>,
}

#[cfg(feature = "file-chooser-events")]
impl std::fmt::Debug for PageFileChooser {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PageFileChooser")
            .field("id", &self.id)
            .field("page", &self.page)
            .field("multiple", &self.multiple)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "file-chooser-events")]
impl PartialEq for PageFileChooser {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id && self.page == other.page && self.multiple == other.multiple
    }
}

#[cfg(feature = "file-chooser-events")]
impl Eq for PageFileChooser {}

#[cfg(feature = "file-chooser-events")]
impl PageFileChooser {
    pub(crate) const fn new(
        id: u64,
        page: ObjectId,
        multiple: bool,
        action: Arc<crate::page_file_chooser::FileChooserAction>,
    ) -> Result<Self, HostError> {
        Ok(Self {
            id,
            page,
            multiple,
            action,
        })
    }

    pub(crate) const fn storage_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    pub const fn is_multiple(&self) -> bool {
        self.multiple
    }

    pub fn action(&self) -> Arc<crate::page_file_chooser::FileChooserAction> {
        Arc::clone(&self.action)
    }
}

#[cfg(feature = "page-dialogs")]
impl std::fmt::Debug for PageDialog {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("PageDialog")
            .field("id", &self.id)
            .field("page", &self.page)
            .field("kind", &self.kind)
            .field("message", &self.message)
            .field("default_value", &self.default_value)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "page-dialogs")]
impl PartialEq for PageDialog {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.page == other.page
            && self.kind == other.kind
            && self.message == other.message
            && self.default_value == other.default_value
    }
}

#[cfg(feature = "page-dialogs")]
impl Eq for PageDialog {}

#[cfg(feature = "page-dialogs")]
impl PageDialog {
    pub(crate) fn new(
        id: u64,
        page: ObjectId,
        kind: crate::page_dialogs::DialogKind,
        message: String,
        default_value: String,
        action: Arc<crate::page_dialogs::DialogAction>,
    ) -> Result<Self, HostError> {
        if message.len() > crate::page_dialogs::MAX_DIALOG_MESSAGE_BYTES
            || default_value.len() > crate::page_dialogs::MAX_DIALOG_DEFAULT_BYTES
        {
            return Err(HostError::ResourceExhausted(
                "Page dialog payload exceeds configured bounds".into(),
            ));
        }
        Ok(Self {
            id,
            page,
            kind,
            message: message.into_boxed_str(),
            default_value: default_value.into_boxed_str(),
            action,
        })
    }

    pub(crate) fn storage_bytes(&self) -> Result<usize, HostError> {
        std::mem::size_of::<Self>()
            .checked_add(self.message.len())
            .and_then(|bytes| bytes.checked_add(self.default_value.len()))
            .ok_or_else(|| HostError::ResourceExhausted("Page dialog storage overflow".into()))
    }

    pub const fn kind(&self) -> crate::page_dialogs::DialogKind {
        self.kind
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn default_value(&self) -> &str {
        &self.default_value
    }

    pub fn action(&self) -> Arc<crate::page_dialogs::DialogAction> {
        Arc::clone(&self.action)
    }
}

impl PageError {
    #[cfg(feature = "page-errors")]
    pub const fn maximum_payload_bytes() -> usize {
        MAX_PAGE_ERROR_NAME_BYTES + MAX_PAGE_ERROR_MESSAGE_BYTES + MAX_PAGE_ERROR_STACK_BYTES
    }

    #[cfg(feature = "page-errors")]
    pub(crate) fn storage_bytes(
        name_bytes: usize,
        message_bytes: usize,
        stack_bytes: usize,
    ) -> Result<usize, HostError> {
        std::mem::size_of::<Self>()
            .checked_add(name_bytes)
            .and_then(|bytes| bytes.checked_add(message_bytes))
            .and_then(|bytes| bytes.checked_add(stack_bytes))
            .ok_or_else(|| HostError::ResourceExhausted("Page error storage overflow".into()))
    }

    #[cfg(feature = "page-errors")]
    pub(crate) fn new(name: String, message: String, stack: String) -> Result<Self, HostError> {
        if name.len() > MAX_PAGE_ERROR_NAME_BYTES {
            return Err(HostError::ResourceExhausted(
                "Page error name exceeds byte limit".into(),
            ));
        }
        if message.len() > MAX_PAGE_ERROR_MESSAGE_BYTES {
            return Err(HostError::ResourceExhausted(
                "Page error message exceeds byte limit".into(),
            ));
        }
        if stack.len() > MAX_PAGE_ERROR_STACK_BYTES {
            return Err(HostError::ResourceExhausted(
                "Page error stack exceeds byte limit".into(),
            ));
        }
        Ok(Self {
            name: name.into_boxed_str(),
            message: message.into_boxed_str(),
            stack: stack.into_boxed_str(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn stack(&self) -> &str {
        &self.stack
    }
}

#[derive(Clone)]
pub struct ConsoleMessage {
    message_type: Box<str>,
    text: Box<str>,
    arguments_json: Box<str>,
    location_url: Box<str>,
    line_number: u32,
    column_number: u32,
    timestamp_micros: u64,
    #[cfg(feature = "console-argument-handles")]
    argument_handles: Box<[Arc<crate::handles::PendingHandle>]>,
}

impl std::fmt::Debug for ConsoleMessage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut value = formatter.debug_struct("ConsoleMessage");
        value
            .field("message_type", &self.message_type)
            .field("text", &self.text)
            .field("arguments_json", &self.arguments_json)
            .field("location_url", &self.location_url)
            .field("line_number", &self.line_number)
            .field("column_number", &self.column_number)
            .field("timestamp_micros", &self.timestamp_micros);
        #[cfg(feature = "console-argument-handles")]
        value.field("argument_handles", &self.argument_handles.len());
        value.finish()
    }
}

impl PartialEq for ConsoleMessage {
    fn eq(&self, other: &Self) -> bool {
        self.message_type == other.message_type
            && self.text == other.text
            && self.arguments_json == other.arguments_json
            && self.location_url == other.location_url
            && self.line_number == other.line_number
            && self.column_number == other.column_number
            && self.timestamp_micros == other.timestamp_micros
    }
}

impl Eq for ConsoleMessage {}

impl ConsoleMessage {
    #[cfg(any(feature = "console-events", feature = "console-history"))]
    pub const fn maximum_payload_bytes() -> usize {
        MAX_CONSOLE_TYPE_BYTES
            + MAX_CONSOLE_TEXT_BYTES
            + MAX_CONSOLE_ARGUMENT_BYTES
            + MAX_CONSOLE_LOCATION_URL_BYTES
    }

    #[cfg(any(feature = "console-events", feature = "console-history"))]
    pub(crate) const fn maximum_text_bytes() -> usize {
        MAX_CONSOLE_TEXT_BYTES
    }

    #[cfg(any(feature = "console-events", feature = "console-history"))]
    pub(crate) fn storage_bytes(
        message_type_bytes: usize,
        text_bytes: usize,
        argument_bytes: usize,
        location_url_bytes: usize,
    ) -> Result<usize, HostError> {
        std::mem::size_of::<Self>()
            .checked_add(2 * std::mem::size_of::<usize>())
            .and_then(|bytes| bytes.checked_add(message_type_bytes))
            .and_then(|bytes| bytes.checked_add(text_bytes))
            .and_then(|bytes| bytes.checked_add(argument_bytes))
            .and_then(|bytes| bytes.checked_add(location_url_bytes))
            .ok_or_else(|| HostError::ResourceExhausted("console message storage overflow".into()))
    }

    #[cfg(any(feature = "console-events", feature = "console-history"))]
    pub(crate) fn new(
        message_type: String,
        text: String,
        arguments_json: String,
        location_url: String,
        line_number: u32,
        column_number: u32,
        timestamp_micros: u64,
    ) -> Result<Self, HostError> {
        if message_type.len() > MAX_CONSOLE_TYPE_BYTES {
            return Err(HostError::ResourceExhausted(
                "console message type exceeds byte limit".into(),
            ));
        }
        if text.len() > MAX_CONSOLE_TEXT_BYTES {
            return Err(HostError::ResourceExhausted(
                "console message text exceeds byte limit".into(),
            ));
        }
        if arguments_json.len() > MAX_CONSOLE_ARGUMENT_BYTES {
            return Err(HostError::ResourceExhausted(
                "console message argument snapshot exceeds byte limit".into(),
            ));
        }
        if location_url.len() > MAX_CONSOLE_LOCATION_URL_BYTES {
            return Err(HostError::ResourceExhausted(
                "console message location URL exceeds byte limit".into(),
            ));
        }
        Ok(Self {
            message_type: message_type.into_boxed_str(),
            text: text.into_boxed_str(),
            arguments_json: arguments_json.into_boxed_str(),
            location_url: location_url.into_boxed_str(),
            line_number,
            column_number,
            timestamp_micros,
            #[cfg(feature = "console-argument-handles")]
            argument_handles: Box::new([]),
        })
    }

    #[cfg(feature = "console-argument-handles")]
    pub(crate) fn with_argument_handles(
        mut self,
        handles: Box<[Arc<crate::handles::PendingHandle>]>,
    ) -> Result<Self, HostError> {
        if handles.len() > crate::handles::MAX_EVENT_ARGUMENT_HANDLES {
            return Err(HostError::ResourceExhausted(
                "console argument Handle count exceeds its limit".into(),
            ));
        }
        self.argument_handles = handles;
        Ok(self)
    }

    #[cfg(feature = "console-argument-handles")]
    pub fn argument_handles(&self) -> &[Arc<crate::handles::PendingHandle>] {
        &self.argument_handles
    }

    pub fn message_type(&self) -> &str {
        &self.message_type
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn arguments_json(&self) -> &str {
        &self.arguments_json
    }

    pub fn location_url(&self) -> &str {
        &self.location_url
    }

    pub const fn line_number(&self) -> u32 {
        self.line_number
    }

    pub const fn column_number(&self) -> u32 {
        self.column_number
    }

    pub const fn timestamp_micros(&self) -> u64 {
        self.timestamp_micros
    }
}

/// Authoritative native input, not an inferred difference between snapshots.
/// The producer must reserve before native work, supply chronological views,
/// and avoid also staging a legacy close publication for the same cause.
pub enum LifecycleCause {
    PageReady {
        revision: CollectionRevision,
        page: ObjectId,
        opener: Option<ObjectId>,
    },
    PopupLinked {
        opener: ObjectId,
        popup: ObjectId,
    },
    PageClosed {
        page: ObjectId,
        close: FrameClosePublication,
    },
    ContextClosed {
        context: ObjectId,
    },
    PageDomContentLoaded {
        page: ObjectId,
    },
    PageLoad {
        page: ObjectId,
    },
    PageMainFrameNavigated {
        page: ObjectId,
    },
    #[cfg(feature = "frame-events")]
    PageFrameAttached {
        page: ObjectId,
        frame: FrameNode,
    },
    #[cfg(feature = "frame-events")]
    PageFrameNavigated {
        page: ObjectId,
        frame: FrameNode,
    },
    #[cfg(feature = "frame-detach-events")]
    PageFrameDetached {
        page: ObjectId,
        frame: FrameNode,
    },
    PageConsole {
        page: ObjectId,
        message: Arc<ConsoleMessage>,
    },
    PageError {
        page: ObjectId,
        error: Arc<PageError>,
    },
    #[cfg(feature = "page-dialogs")]
    PageDialog {
        page: ObjectId,
        dialog: Arc<PageDialog>,
    },
    #[cfg(feature = "file-chooser-events")]
    PageFileChooser {
        page: ObjectId,
        chooser: Arc<PageFileChooser>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleNotice {
    ContextPage {
        context: ObjectId,
        page: ObjectId,
    },
    Popup {
        opener: ObjectId,
        popup: ObjectId,
    },
    PageClose {
        context: ObjectId,
        page: ObjectId,
    },
    ContextClose {
        context: ObjectId,
    },
    PageDomContentLoaded {
        context: ObjectId,
        page: ObjectId,
    },
    PageLoad {
        context: ObjectId,
        page: ObjectId,
    },
    PageMainFrameNavigated {
        context: ObjectId,
        page: ObjectId,
    },
    #[cfg(feature = "frame-events")]
    PageFrameAttached {
        context: ObjectId,
        page: ObjectId,
        frame: ObjectId,
    },
    #[cfg(feature = "frame-events")]
    PageFrameNavigated {
        context: ObjectId,
        page: ObjectId,
        frame: ObjectId,
    },
    #[cfg(feature = "frame-detach-events")]
    PageFrameDetached {
        context: ObjectId,
        page: ObjectId,
        frame: ObjectId,
    },
    PageConsole {
        context: ObjectId,
        page: ObjectId,
        message: Arc<ConsoleMessage>,
    },
    PageError {
        context: ObjectId,
        page: ObjectId,
        error: Arc<PageError>,
    },
    #[cfg(feature = "page-dialogs")]
    PageDialog {
        context: ObjectId,
        page: ObjectId,
        dialog: Arc<PageDialog>,
    },
    #[cfg(feature = "file-chooser-events")]
    PageFileChooser {
        context: ObjectId,
        page: ObjectId,
        chooser: Arc<PageFileChooser>,
    },
}

/// Synchronous EventEmitter outcome. A returned Promise is Delivered; its
/// eventual rejection is not awaited and cannot suppress another notice.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleDelivery {
    Delivered,
    ListenerThrew,
}

/// Borrowed metadata for foreign wrapper admission/conversion. It includes
/// retired descriptors, but no host/runtime/transport. No native lock is held.
pub struct LifecycleNoticeView<'a> {
    pub notice: LifecycleNotice,
    pub revision: &'a CollectionRevision,
}

struct PreparedLifecycle {
    before: CollectionView,
    after: CollectionView,
    next: LifecycleNotice,
    dependent: Option<LifecycleNotice>,
    close: Option<FrameClosePublication>,
    awaiting: bool,
    creation: Option<CreationDelivery>,
    #[cfg(feature = "frame-detach-events")]
    detached_frame: Option<FrameNode>,
}

#[cfg(feature = "frame-detach-events")]
impl Drop for PreparedLifecycle {
    fn drop(&mut self) {
        if let Some(frame) = self.detached_frame.take() {
            frame.release_retired_binding();
        }
    }
}

pub(super) struct LifecycleGroup {
    prepared: Option<PreparedLifecycle>,
    _storage: Arc<Storage>,
    creation: Option<CreationDelivery>,
}

impl LifecycleGroup {
    pub(super) fn permits_popup(&self) -> Result<bool, HostError> {
        let prepared = self.prepared.as_ref().expect("staged group");
        if matches!(&prepared.next, LifecycleNotice::Popup { .. })
            && let Some(creation) = &prepared.creation
        {
            return creation.permits_popup();
        }
        Ok(true)
    }

    pub(super) fn abandon_delivery(&self) {
        let prepared = self.prepared.as_ref().expect("staged group");
        if matches!(&prepared.next, LifecycleNotice::ContextPage { .. })
            && let Some(creation) = &prepared.creation
        {
            creation.acknowledge(false);
        }
    }
    pub(super) fn awaiting_acknowledgement(&self) -> bool {
        self.prepared
            .as_ref()
            .expect("only prepared groups are staged")
            .awaiting
    }
}

/// Owns its bounded queue slot and allocated body before a native transaction.
/// Dropping an unused admission releases both budgets. No side queue is created.
pub struct LifecycleReservation {
    book: Weak<FrameMetadata>,
    event: ResourceLease,
    group: Box<LifecycleGroup>,
}

impl LifecycleReservation {
    pub(crate) fn track_creation(mut self, creation: CreationDelivery) -> Self {
        self.group.creation = Some(creation);
        self
    }
}

const STORAGE_BYTES: usize = std::mem::size_of::<LifecycleGroup>()
    + std::mem::size_of::<LifecycleReservation>()
    + std::mem::size_of::<Storage>()
    + 2 * std::mem::size_of::<usize>()
    // One conversion's temporary revision/notice; retained metadata clones
    // continue to share their original projection's lease.
    + std::mem::size_of::<CollectionRevision>()
    + std::mem::size_of::<LifecycleNoticeView<'static>>();

fn invalid(message: &str) -> HostError {
    HostError::InvalidArgument(message.into())
}

fn validate_popup(
    view: &CollectionView,
    opener: ObjectId,
    popup: ObjectId,
) -> Result<(), HostError> {
    let pages = view.pages();
    let opener_page = pages.iter().find(|page| page.id == opener);
    let popup_page = pages.iter().find(|page| page.id == popup);
    match (opener_page, popup_page) {
        (Some(left), Some(right)) if opener != popup && left.context == right.context => Ok(()),
        _ => Err(invalid(
            "popup requires distinct visible Pages in the same Context",
        )),
    }
}

impl PreparedLifecycle {
    fn new(before: CollectionView, cause: LifecycleCause) -> Result<Self, HostError> {
        #[cfg(feature = "frame-detach-events")]
        let detached_frame = match &cause {
            LifecycleCause::PageFrameDetached { frame, .. } => Some(frame.clone()),
            _ => None,
        };
        let (after, next, dependent, close) = match cause {
            LifecycleCause::PageReady {
                revision,
                page,
                opener,
            } => {
                let mut after = before.try_rebase(&revision).map_err(HostError::backend)?;
                let context = revision
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("ready Page is missing from native revision"))?
                    .context;
                if !after.contexts().iter().any(|entry| entry.id == context) {
                    after = after
                        .with_visibility(CollectionMember::Context(context), true)
                        .map_err(HostError::backend)?;
                }
                after = after
                    .with_visibility(CollectionMember::Page(page), true)
                    .map_err(HostError::backend)?;
                if let Some(opener) = opener {
                    validate_popup(&after, opener, page)?;
                }
                (
                    after,
                    LifecycleNotice::ContextPage { context, page },
                    opener.map(|opener| LifecycleNotice::Popup {
                        opener,
                        popup: page,
                    }),
                    None,
                )
            }
            LifecycleCause::PopupLinked { opener, popup } => {
                validate_popup(&before, opener, popup)?;
                (
                    before.clone(),
                    LifecycleNotice::Popup { opener, popup },
                    None,
                    None,
                )
            }
            LifecycleCause::PageClosed { page, close } => {
                let pages = before.pages();
                let descriptor = pages
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("closed Page is not visible"))?;
                if !close.is_for(&descriptor.tree) {
                    return Err(invalid("Page close publication belongs to another tree"));
                }
                let after = before
                    .with_visibility(CollectionMember::Page(page), false)
                    .map_err(HostError::backend)?;
                (
                    after,
                    LifecycleNotice::PageClose {
                        context: descriptor.context,
                        page,
                    },
                    None,
                    Some(close),
                )
            }
            LifecycleCause::ContextClosed { context } => {
                let after = before
                    .with_visibility(CollectionMember::Context(context), false)
                    .map_err(HostError::backend)?;
                (after, LifecycleNotice::ContextClose { context }, None, None)
            }
            LifecycleCause::PageDomContentLoaded { page } => {
                let context = before
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("Page lifecycle event source is not visible"))?
                    .context;
                (
                    before.clone(),
                    LifecycleNotice::PageDomContentLoaded { context, page },
                    None,
                    None,
                )
            }
            LifecycleCause::PageLoad { page } => {
                let context = before
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("Page lifecycle event source is not visible"))?
                    .context;
                (
                    before.clone(),
                    LifecycleNotice::PageLoad { context, page },
                    None,
                    None,
                )
            }
            LifecycleCause::PageMainFrameNavigated { page } => {
                let context = before
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("Page navigation event source is not visible"))?
                    .context;
                (
                    before.clone(),
                    LifecycleNotice::PageMainFrameNavigated { context, page },
                    None,
                    None,
                )
            }
            #[cfg(feature = "frame-events")]
            LifecycleCause::PageFrameAttached { page, frame } => {
                let descriptor = before
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("Frame attach Page is not visible"))?;
                if frame.parent_id().is_none() || !descriptor.tree.contains_native_frame(&frame)? {
                    return Err(invalid("Frame attach does not match a live child identity"));
                }
                (
                    before.clone(),
                    LifecycleNotice::PageFrameAttached {
                        context: descriptor.context,
                        page,
                        frame: frame.id(),
                    },
                    None,
                    None,
                )
            }
            #[cfg(feature = "frame-events")]
            LifecycleCause::PageFrameNavigated { page, frame } => {
                let descriptor = before
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("Frame navigation Page is not visible"))?;
                if !descriptor.tree.contains_native_frame(&frame)? {
                    return Err(invalid(
                        "Frame navigation does not match a live Frame identity",
                    ));
                }
                (
                    before.clone(),
                    LifecycleNotice::PageFrameNavigated {
                        context: descriptor.context,
                        page,
                        frame: frame.id(),
                    },
                    None,
                    None,
                )
            }
            #[cfg(feature = "frame-detach-events")]
            LifecycleCause::PageFrameDetached { page, frame } => {
                let descriptor = before
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("Frame detach Page is not visible"))?;
                if frame.parent_id().is_none() || !descriptor.tree.has_native_retirement(&frame)? {
                    return Err(invalid(
                        "Frame detach does not match a retired child identity",
                    ));
                }
                (
                    before.clone(),
                    LifecycleNotice::PageFrameDetached {
                        context: descriptor.context,
                        page,
                        frame: frame.id(),
                    },
                    None,
                    None,
                )
            }
            LifecycleCause::PageConsole { page, message } => {
                let context = before
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("Page console event source is not visible"))?
                    .context;
                (
                    before.clone(),
                    LifecycleNotice::PageConsole {
                        context,
                        page,
                        message,
                    },
                    None,
                    None,
                )
            }
            LifecycleCause::PageError { page, error } => {
                let context = before
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("Page error event source is not visible"))?
                    .context;
                (
                    before.clone(),
                    LifecycleNotice::PageError {
                        context,
                        page,
                        error,
                    },
                    None,
                    None,
                )
            }
            #[cfg(feature = "page-dialogs")]
            LifecycleCause::PageDialog { page, dialog } => {
                let context = before
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("Page dialog event source is not visible"))?
                    .context;
                (
                    before.clone(),
                    LifecycleNotice::PageDialog {
                        context,
                        page,
                        dialog,
                    },
                    None,
                    None,
                )
            }
            #[cfg(feature = "file-chooser-events")]
            LifecycleCause::PageFileChooser { page, chooser } => {
                let context = before
                    .pages()
                    .iter()
                    .find(|entry| entry.id == page)
                    .ok_or_else(|| invalid("Page file chooser event source is not visible"))?
                    .context;
                (
                    before.clone(),
                    LifecycleNotice::PageFileChooser {
                        context,
                        page,
                        chooser,
                    },
                    None,
                    None,
                )
            }
        };
        Ok(Self {
            before,
            after,
            next,
            dependent,
            close,
            awaiting: false,
            creation: None,
            #[cfg(feature = "frame-detach-events")]
            detached_frame,
        })
    }
}

impl FramePublications {
    pub fn reserve_lifecycle(
        &self,
        factory: &FrameTreeFactory,
    ) -> Result<LifecycleReservation, HostError> {
        self.reserve_lifecycle_payload(factory, 0)
    }

    pub(crate) fn reserve_lifecycle_payload(
        &self,
        factory: &FrameTreeFactory,
        payload_bytes: usize,
    ) -> Result<LifecycleReservation, HostError> {
        let state = self.0.state.lock().map_err(HostError::backend)?;
        if state.discarded || state.sequence == u64::MAX {
            return Err(HostError::TargetClosed);
        }
        if state.queue_storage.is_none() {
            return Err(invalid(
                "lifecycle groups require a pre-admitted publication queue",
            ));
        }
        let event = self
            .0
            .resources
            .reserve(SCOPE, ResourceClass::EventItem, 1)
            .map_err(|error| HostError::ResourceExhausted(error.to_string()))?;
        let storage = factory.reserve_collection_storage(
            STORAGE_BYTES
                .checked_add(payload_bytes)
                .ok_or_else(|| invalid("lifecycle payload storage overflow"))?,
        )?;
        Ok(LifecycleReservation {
            book: Arc::downgrade(&self.0),
            event,
            group: Box::new(LifecycleGroup {
                prepared: None,
                _storage: Arc::new(storage),
                creation: None,
            }),
        })
    }

    /// Stages without publishing. The returned *planned* view sequences the next
    /// native group; it is not the public view. Exact predecessor identity is
    /// checked again at commit. Native source/terminal selection is a separate
    /// integration responsibility; this method cannot stage after teardown.
    pub fn stage_lifecycle(
        &self,
        mut reserved: LifecycleReservation,
        before: CollectionView,
        cause: LifecycleCause,
    ) -> Result<CollectionView, HostError> {
        if !reserved.book.ptr_eq(&Arc::downgrade(&self.0)) || !before.belongs_to(&self.0.resources)
        {
            return Err(invalid(
                "lifecycle reservation/view belongs to another publication queue",
            ));
        }
        if reserved.group.creation.is_some()
            && !matches!(
                cause,
                LifecycleCause::PageReady { .. } | LifecycleCause::PopupLinked { .. }
            )
        {
            return Err(invalid(
                "creation acknowledgement requires a ready or popup cause",
            ));
        }
        let mut prepared = PreparedLifecycle::new(before, cause)?;
        prepared.creation = reserved.group.creation.take();
        let after = prepared.after.clone();
        let mut state = self.0.state.lock().map_err(HostError::backend)?;
        if state.discarded || state.sequence == u64::MAX {
            return Err(HostError::TargetClosed);
        }
        let sequence = state.next_sequence()?;
        reserved.group.prepared = Some(prepared);
        state.pending.push_back(FrameChange {
            sequence,
            value: FrameChangeValue::Lifecycle(reserved.group),
            _event: reserved.event,
        });
        state.sequence = sequence;
        drop(state);
        self.0.changed.notify_one();
        Ok(after)
    }
}

pub(super) fn front_group(state: &MetadataState, sequence: u64) -> Option<&LifecycleGroup> {
    let change = state
        .pending
        .front()
        .filter(|change| change.sequence <= sequence)?;
    match &change.value {
        FrameChangeValue::Lifecycle(group) => Some(group),
        _ => None,
    }
}

fn front_prepared_mut(
    state: &mut MetadataState,
    sequence: u64,
) -> Result<&mut PreparedLifecycle, PublicationDrainError> {
    let change = state
        .pending
        .front_mut()
        .filter(|change| change.sequence <= sequence)
        .ok_or(PublicationDrainError::NoNotice)?;
    match &mut change.value {
        FrameChangeValue::Lifecycle(group) => Ok(group.prepared.as_mut().expect("staged group")),
        _ => Err(PublicationDrainError::NoNotice),
    }
}

impl FramePublicationDrain {
    fn validate_notice_owner(&self, state: &MetadataState) -> Result<(), PublicationDrainError> {
        if state.discarded || state.drain.active != Some(self.owner) {
            return Err(PublicationDrainError::Discarded);
        }
        Ok(())
    }

    /// Prepare/admit the foreign wrapper before commit. The closure runs without
    /// native locks and may fail without consuming the notice or changing state.
    /// Returning does not commit; call commit_notice immediately before emit.
    pub fn with_next_notice<T>(
        &self,
        convert: impl FnOnce(LifecycleNoticeView<'_>) -> T,
    ) -> Result<T, PublicationDrainError> {
        let book = self.book.as_ref().ok_or(PublicationDrainError::Finished)?;
        let state = book.0.state.lock().map_err(HostError::backend)?;
        self.validate_notice_owner(&state)?;
        let group = front_group(&state, self.sequence).ok_or(PublicationDrainError::NoNotice)?;
        if !group.permits_popup()? {
            return Err(PublicationDrainError::NoNotice);
        }
        let prepared = group.prepared.as_ref().expect("staged group");
        if prepared.awaiting {
            return Err(PublicationDrainError::AwaitingAcknowledgement);
        }
        let notice = prepared.next.clone();
        let revision = prepared.after.revision().clone();
        // Conversion may synchronously discard the endpoint. Keep its admitted
        // temporary-body allowance until the closure returns or unwinds.
        let storage = group._storage.clone();
        drop(state);
        let result = convert(LifecycleNoticeView {
            notice,
            revision: &revision,
        });
        drop(storage);
        Ok(result)
    }

    /// Installs callback-visible state and consumes this notice exactly once,
    /// before foreign code executes. A listener throw cannot roll this back.
    pub fn commit_notice(&mut self) -> Result<LifecycleNotice, PublicationDrainError> {
        let book = self.book.as_ref().ok_or(PublicationDrainError::Finished)?;
        let mut state = book.0.state.lock().map_err(HostError::backend)?;
        self.validate_notice_owner(&state)?;
        let current = state.collections.clone();
        let group = front_group(&state, self.sequence).ok_or(PublicationDrainError::NoNotice)?;
        if !group.permits_popup()? {
            return Err(PublicationDrainError::NoNotice);
        }
        let prepared = front_prepared_mut(&mut state, self.sequence)?;
        if prepared.awaiting {
            return Err(PublicationDrainError::AwaitingAcknowledgement);
        }
        let result = if !current
            .as_ref()
            .is_some_and(|view| view.same_state(&prepared.before))
        {
            Err(HostError::Backend(
                "lifecycle publication predecessor changed".into(),
            ))
        } else if let Some(close) = prepared.close.take() {
            close.publish()
        } else {
            Ok(())
        };
        if let Err(error) = result {
            drop(state);
            book.discard();
            self.book = None;
            return Err(PublicationDrainError::Backend(error));
        }
        prepared.awaiting = true;
        let notice = prepared.next.clone();
        state.collections = Some(prepared.after.clone());
        Ok(notice)
    }

    /// Report only the synchronous emit result. ListenerThrew abandons this
    /// group's dependent popup, never independent later close groups. Dropping
    /// a guard while acknowledgement is outstanding has the same skip policy.
    pub fn acknowledge_notice(
        &mut self,
        delivery: LifecycleDelivery,
    ) -> Result<(), PublicationDrainError> {
        let book = self.book.as_ref().ok_or(PublicationDrainError::Finished)?;
        let mut state = book.0.state.lock().map_err(HostError::backend)?;
        self.validate_notice_owner(&state)?;
        let prepared = front_prepared_mut(&mut state, self.sequence)?;
        if !prepared.awaiting {
            return Err(PublicationDrainError::NotCommitted);
        }
        if matches!(&prepared.next, LifecycleNotice::ContextPage { .. })
            && let Some(creation) = &prepared.creation
        {
            creation.acknowledge(delivery == LifecycleDelivery::Delivered);
        }
        if let Some(next) = prepared
            .dependent
            .take()
            .filter(|_| delivery == LifecycleDelivery::Delivered)
        {
            prepared.next = next;
            prepared.before = prepared.after.clone();
            prepared.awaiting = false;
        } else {
            state.pending.pop_front();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
