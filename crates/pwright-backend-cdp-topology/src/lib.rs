//! Command-free, bounded Chromium CDP topology state for Pwright.
//!
//! This crate owns portable Browser/Context/Page/Frame/World identity and
//! lifecycle state. It emits effects for callers to apply but performs no CDP
//! commands, transport I/O, browser discovery, launch, or process ownership.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use pwright_browser_model::{
    BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND, FRAME_OBJECT_KIND, PAGE_OBJECT_KIND,
};
use pwright_core::{
    ControlMessage, ObjectGraph, ObjectGraphError, ObjectId, ProductionResourceError,
    ProductionResourceLedger, ResourceClass, ResourceError, ResourceLease, ResourceLedger,
    ResourceRequirements, ResourceScope,
};

/// Identifies one incarnation of a CDP connection.
///
/// Every browser event must carry the epoch of the reader that produced it.
/// Events from an older reader are ignored so a late detach cannot tear down a
/// target that has already rebound to a new connection.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ConnectionEpoch(u64);

impl ConnectionEpoch {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn checked_next(self) -> Option<Self> {
        match self.0.checked_add(1) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpContextOwnership {
    Default,
    Owned,
    External,
}

/// Borrowed native route for one materialized portable BrowserContext.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpBrowserContextRoute<'a> {
    object_id: ObjectId,
    browser_context_id: &'a str,
    ownership: CdpContextOwnership,
}

impl<'a> CdpBrowserContextRoute<'a> {
    pub const fn object_id(self) -> ObjectId {
        self.object_id
    }

    pub const fn browser_context_id(self) -> &'a str {
        self.browser_context_id
    }

    pub const fn ownership(self) -> CdpContextOwnership {
        self.ownership
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpPageObjectIds {
    pub page: ObjectId,
    pub main_frame: ObjectId,
}

impl CdpPageObjectIds {
    pub const fn new(page: ObjectId, main_frame: ObjectId) -> Self {
        Self { page, main_frame }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpPageTarget {
    pub target_id: String,
    pub session_id: String,
    pub browser_context_id: String,
    pub opener_target_id: Option<String>,
}

/// Borrowed native route for one materialized portable Page.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpPageSessionRoute<'a> {
    object_id: ObjectId,
    browser_context_object_id: ObjectId,
    target_id: &'a str,
    session_id: &'a str,
}

/// Borrowed route from one portable Frame to its current main execution
/// world and the exact CDP session that reported that world.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpFrameExecutionRoute<'a> {
    object_id: ObjectId,
    target_id: &'a str,
    session_id: &'a str,
    execution_context_id: i64,
    unique_context_id: &'a str,
}

/// Borrowed command route for one attached target-local CDP session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpTargetSessionRoute<'a> {
    target_id: &'a str,
    page_target_id: &'a str,
    session_id: &'a str,
}

impl<'a> CdpTargetSessionRoute<'a> {
    pub const fn target_id(self) -> &'a str {
        self.target_id
    }

    pub const fn page_target_id(self) -> &'a str {
        self.page_target_id
    }

    pub const fn session_id(self) -> &'a str {
        self.session_id
    }
}

impl<'a> CdpFrameExecutionRoute<'a> {
    pub const fn object_id(self) -> ObjectId {
        self.object_id
    }

    pub const fn target_id(self) -> &'a str {
        self.target_id
    }

    pub const fn session_id(self) -> &'a str {
        self.session_id
    }

    pub const fn execution_context_id(self) -> i64 {
        self.execution_context_id
    }

    pub const fn unique_context_id(self) -> &'a str {
        self.unique_context_id
    }
}

impl<'a> CdpPageSessionRoute<'a> {
    pub const fn object_id(self) -> ObjectId {
        self.object_id
    }

    pub const fn browser_context_object_id(self) -> ObjectId {
        self.browser_context_object_id
    }

    pub const fn target_id(self) -> &'a str {
        self.target_id
    }

    pub const fn session_id(self) -> &'a str {
        self.session_id
    }
}

impl CdpPageTarget {
    pub fn new(
        target_id: impl Into<String>,
        session_id: impl Into<String>,
        browser_context_id: impl Into<String>,
    ) -> Self {
        Self {
            target_id: target_id.into(),
            session_id: session_id.into(),
            browser_context_id: browser_context_id.into(),
            opener_target_id: None,
        }
    }

    pub fn with_opener(mut self, opener_target_id: impl Into<String>) -> Self {
        self.opener_target_id = Some(opener_target_id.into());
        self
    }

    pub fn from_attached_page(
        event: &pwright_cdp::generated::target::AttachedToTargetEvent,
    ) -> Result<Self, CdpTopologyEventError> {
        if event.target_info.r#type != "page" {
            return Err(CdpTopologyEventError::UnexpectedTargetType {
                expected: "page",
                actual: event.target_info.r#type.clone(),
            });
        }
        let browser_context_id = event.target_info.browser_context_id.clone().ok_or(
            CdpTopologyEventError::MissingBrowserContextId {
                target_id: event.target_info.target_id.clone(),
            },
        )?;
        Ok(Self {
            target_id: event.target_info.target_id.clone(),
            session_id: event.session_id.clone(),
            browser_context_id,
            opener_target_id: event.target_info.opener_id.clone(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpFrameAttachment {
    pub page_target_id: String,
    pub frame_id: String,
    pub parent_frame_id: Option<String>,
    pub session_id: Option<String>,
}

impl CdpFrameAttachment {
    pub fn local(
        page_target_id: impl Into<String>,
        frame_id: impl Into<String>,
        parent_frame_id: impl Into<String>,
    ) -> Self {
        Self {
            page_target_id: page_target_id.into(),
            frame_id: frame_id.into(),
            parent_frame_id: Some(parent_frame_id.into()),
            session_id: None,
        }
    }

    pub fn remote(
        page_target_id: impl Into<String>,
        frame_id: impl Into<String>,
        parent_frame_id: impl Into<String>,
        session_id: impl Into<String>,
    ) -> Self {
        Self {
            page_target_id: page_target_id.into(),
            frame_id: frame_id.into(),
            parent_frame_id: Some(parent_frame_id.into()),
            session_id: Some(session_id.into()),
        }
    }

    /// Describes the root frame reported by `Page.getFrameTree` or a root
    /// `Page.frameNavigated` event. A changed native frame ID retains the
    /// existing public main-frame object.
    pub fn main(page_target_id: impl Into<String>, frame_id: impl Into<String>) -> Self {
        Self {
            page_target_id: page_target_id.into(),
            frame_id: frame_id.into(),
            parent_frame_id: None,
            session_id: None,
        }
    }

    pub fn from_frame_attached(
        page_target_id: impl Into<String>,
        event: &pwright_cdp::generated::page::FrameAttachedEvent,
    ) -> Self {
        Self::local(
            page_target_id,
            event.frame_id.clone(),
            event.parent_frame_id.clone(),
        )
    }

    pub fn from_attached_oopif(
        page_target_id: impl Into<String>,
        event: &pwright_cdp::generated::target::AttachedToTargetEvent,
    ) -> Result<Self, CdpTopologyEventError> {
        if event.target_info.r#type != "iframe" {
            return Err(CdpTopologyEventError::UnexpectedTargetType {
                expected: "iframe",
                actual: event.target_info.r#type.clone(),
            });
        }
        let parent_frame_id = event.target_info.parent_frame_id.clone().ok_or(
            CdpTopologyEventError::MissingParentFrameId {
                target_id: event.target_info.target_id.clone(),
            },
        )?;
        Ok(Self::remote(
            page_target_id,
            event.target_info.target_id.clone(),
            parent_frame_id,
            event.session_id.clone(),
        ))
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpFrameDetachReason {
    Remove,
    Swap,
}

impl CdpFrameDetachReason {
    pub fn from_event(
        event: &pwright_cdp::generated::page::FrameDetachedEvent,
    ) -> Result<Self, CdpTopologyEventError> {
        match event.reason.as_str() {
            "remove" => Ok(Self::Remove),
            "swap" => Ok(Self::Swap),
            actual => Err(CdpTopologyEventError::UnexpectedFrameDetachReason {
                actual: actual.to_owned(),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpWorldKind {
    Main,
    Utility,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpWorldAttachment {
    pub session_id: String,
    pub execution_context_id: i64,
    pub unique_id: String,
    pub frame_id: String,
    pub kind: CdpWorldKind,
}

impl CdpWorldAttachment {
    pub fn new(
        session_id: impl Into<String>,
        execution_context_id: i64,
        unique_id: impl Into<String>,
        frame_id: impl Into<String>,
        kind: CdpWorldKind,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            execution_context_id,
            unique_id: unique_id.into(),
            frame_id: frame_id.into(),
            kind,
        }
    }

    pub fn from_execution_context(
        session_id: impl Into<String>,
        event: &pwright_cdp::generated::runtime::ExecutionContextCreatedEvent,
        utility_world_name: Option<&str>,
    ) -> Result<Option<Self>, CdpTopologyEventError> {
        let Some(aux_data) = &event.context.aux_data else {
            return Ok(None);
        };
        let Some(aux_data) = aux_data.as_object() else {
            return Err(CdpTopologyEventError::MalformedExecutionContextAuxData);
        };
        let Some(frame_id_value) = aux_data.get("frameId") else {
            return Ok(None);
        };
        let Some(frame_id) = frame_id_value.as_str() else {
            return Err(CdpTopologyEventError::MalformedExecutionContextAuxData);
        };
        let is_default = match aux_data.get("isDefault") {
            Some(value) => value
                .as_bool()
                .ok_or(CdpTopologyEventError::MalformedExecutionContextAuxData)?,
            None => false,
        };
        let kind = if is_default {
            CdpWorldKind::Main
        } else if utility_world_name == Some(event.context.name.as_str()) {
            CdpWorldKind::Utility
        } else {
            return Ok(None);
        };
        Ok(Some(Self::new(
            session_id,
            event.context.id,
            event.context.unique_id.clone(),
            frame_id,
            kind,
        )))
    }
}

fn compact_string(value: String) -> String {
    value.into_boxed_str().into_string()
}

fn compact_page_target(mut target: CdpPageTarget) -> CdpPageTarget {
    target.target_id = compact_string(target.target_id);
    target.session_id = compact_string(target.session_id);
    target.browser_context_id = compact_string(target.browser_context_id);
    target.opener_target_id = target.opener_target_id.map(compact_string);
    target
}

fn compact_frame_attachment(mut attachment: CdpFrameAttachment) -> CdpFrameAttachment {
    attachment.page_target_id = compact_string(attachment.page_target_id);
    attachment.frame_id = compact_string(attachment.frame_id);
    attachment.parent_frame_id = attachment.parent_frame_id.map(compact_string);
    attachment.session_id = attachment.session_id.map(compact_string);
    attachment
}

fn compact_world_attachment(mut attachment: CdpWorldAttachment) -> CdpWorldAttachment {
    attachment.session_id = compact_string(attachment.session_id);
    attachment.unique_id = compact_string(attachment.unique_id);
    attachment.frame_id = compact_string(attachment.frame_id);
    attachment
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpPageState {
    Initializing,
    Ready,
    Crashed { status: String, error_code: i64 },
    Reconnecting,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CdpTopologyDisposition {
    Applied,
    Deferred,
    Idempotent,
    IgnoredStaleEpoch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpTopologyEffect {
    Lifecycle(ControlMessage),
    PageReady {
        page: ObjectId,
        context: ObjectId,
    },
    PopupLinked {
        opener: ObjectId,
        popup: ObjectId,
    },
    PageCrashed {
        page: ObjectId,
        status: String,
        error_code: i64,
    },
    MainFrameRebound {
        frame: ObjectId,
    },
    FrameDetachSynchronizationRequired {
        page: ObjectId,
        frame: ObjectId,
    },
    WorldChanged {
        frame: ObjectId,
        kind: CdpWorldKind,
    },
    WorldRemoved {
        frame: ObjectId,
        kind: CdpWorldKind,
    },
    ReconnectStarted {
        epoch: ConnectionEpoch,
    },
    ReconnectFinished {
        epoch: ConnectionEpoch,
        live_pages: usize,
    },
}

#[cfg(test)]
mod tests {
    use pwright_browser_model::{
        BROWSER_CONTEXT_OBJECT_KIND, BROWSER_OBJECT_KIND_CATALOG, FRAME_OBJECT_KIND,
        PAGE_OBJECT_KIND,
    };
    use pwright_core::{ControlMessage, ObjectId};
    use pwright_runtime::{DirectDispatcher, ObjectTopologyMode, RuntimeLimits};
    use serde_json::json;

    use super::{
        CdpContextOwnership, CdpFrameAttachment, CdpFrameDetachReason, CdpPageObjectIds,
        CdpPageState, CdpPageTarget, CdpTopology, CdpTopologyDisposition, CdpTopologyEffect,
        CdpTopologyError, CdpTopologyEventError, CdpTopologyLimits, CdpWorldAttachment,
        CdpWorldKind, ConnectionEpoch,
    };

    const EPOCH_ONE: ConnectionEpoch = ConnectionEpoch::new(1);
    const EPOCH_TWO: ConnectionEpoch = ConnectionEpoch::new(2);

    fn limits() -> CdpTopologyLimits {
        CdpTopologyLimits::new(64, 8, 8, 16, 16, 64)
    }

    fn topology() -> CdpTopology {
        CdpTopology::new(ObjectId::new(1), EPOCH_ONE, limits()).expect("test topology should fit")
    }

    fn register_context(topology: &mut CdpTopology, native_id: &str, object_id: u64) {
        topology
            .register_context(
                EPOCH_ONE,
                native_id,
                ObjectId::new(object_id),
                CdpContextOwnership::Owned,
            )
            .expect("test context should register");
    }

    #[test]
    fn connection_epoch_advances_without_wrapping() {
        // Arrange
        let current = ConnectionEpoch::new(41);
        let exhausted = ConnectionEpoch::new(u64::MAX);

        // Act
        let next = current.checked_next();
        let overflow = exhausted.checked_next();

        // Assert
        assert_eq!(next, Some(ConnectionEpoch::new(42)));
        assert_eq!(overflow, None);
    }

    fn attach_page(
        topology: &mut CdpTopology,
        target_id: &str,
        session_id: &str,
        context_id: &str,
        page_id: u64,
        frame_id: u64,
    ) {
        topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new(target_id, session_id, context_id),
                CdpPageObjectIds::new(ObjectId::new(page_id), ObjectId::new(frame_id)),
            )
            .expect("test page should attach");
    }

    fn apply_lifecycle(dispatcher: &DirectDispatcher<()>, effects: &[CdpTopologyEffect]) {
        for effect in effects {
            let CdpTopologyEffect::Lifecycle(message) = effect else {
                continue;
            };
            match *message {
                ControlMessage::Create {
                    object_id,
                    parent,
                    kind,
                } => dispatcher
                    .register_object(object_id, parent, kind)
                    .expect("topology create should satisfy runtime invariants"),
                ControlMessage::Dispose { object_id } => {
                    dispatcher
                        .dispose_object(object_id)
                        .expect("topology dispose should satisfy runtime invariants");
                }
                ControlMessage::Adopt { .. }
                | ControlMessage::Cancel { .. }
                | ControlMessage::SetSubscription { .. }
                | ControlMessage::Close
                | ControlMessage::CloseAck => {
                    panic!("CDP topology emitted an unrelated control message")
                }
            }
        }
    }

    #[test]
    fn context_and_page_create_the_portable_object_hierarchy() {
        // Arrange
        let mut topology = topology();
        let context = topology
            .register_context(
                EPOCH_ONE,
                "context-a",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should register");

        // Act
        let page = topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("target-a", "session-a", "context-a"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("page should attach");

        // Assert
        assert_eq!(
            context.effects(),
            &[CdpTopologyEffect::Lifecycle(ControlMessage::Create {
                object_id: ObjectId::new(2),
                parent: Some(ObjectId::new(1)),
                kind: BROWSER_CONTEXT_OBJECT_KIND.tag,
            })]
        );
        assert_eq!(
            page.effects(),
            &[
                CdpTopologyEffect::Lifecycle(ControlMessage::Create {
                    object_id: ObjectId::new(3),
                    parent: Some(ObjectId::new(2)),
                    kind: PAGE_OBJECT_KIND.tag,
                }),
                CdpTopologyEffect::Lifecycle(ControlMessage::Create {
                    object_id: ObjectId::new(4),
                    parent: Some(ObjectId::new(3)),
                    kind: FRAME_OBJECT_KIND.tag,
                }),
            ]
        );
        assert_eq!(topology.page_object_id("target-a"), Some(ObjectId::new(3)));
        let route = topology
            .page_session_routes()
            .next()
            .expect("materialized Page should have a session route");
        assert_eq!(route.object_id(), ObjectId::new(3));
        assert_eq!(route.browser_context_object_id(), ObjectId::new(2));
        assert_eq!(
            topology.main_frame_object_id("target-a"),
            Some(ObjectId::new(4))
        );
    }

    #[test]
    fn browser_context_route_resolves_portable_identity_and_ownership() {
        // Arrange
        let mut topology = topology();
        topology
            .register_context(
                EPOCH_ONE,
                "context-a",
                ObjectId::new(2),
                CdpContextOwnership::External,
            )
            .expect("context should register");

        // Act
        let route = topology
            .browser_context_route(ObjectId::new(2))
            .expect("context route should resolve");
        let routes = topology.browser_context_routes().collect::<Vec<_>>();

        // Assert
        assert_eq!(route.object_id(), ObjectId::new(2));
        assert_eq!(route.browser_context_id(), "context-a");
        assert_eq!(route.ownership(), CdpContextOwnership::External);
        assert_eq!(routes, vec![route]);
        assert!(topology.browser_context_route(ObjectId::new(99)).is_none());
    }

    #[test]
    fn default_context_uses_no_invented_native_id_and_accepts_default_pages() {
        // Arrange
        let mut topology = topology();
        topology
            .register_default_context(EPOCH_ONE, ObjectId::new(2))
            .expect("default context should register");

        // Act
        let attached = topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("target-a", "session-a", ""),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("a Page without browserContextId should use the default context");

        // Assert
        let route = topology
            .default_context_route()
            .expect("default context route should resolve");
        assert_eq!(route.browser_context_id(), "");
        assert_eq!(route.object_id(), ObjectId::new(2));
        assert_eq!(route.ownership(), CdpContextOwnership::Default);
        assert_eq!(attached.disposition(), CdpTopologyDisposition::Applied);
        assert_eq!(topology.page_object_id("target-a"), Some(ObjectId::new(3)));
    }

    #[test]
    fn topology_rejects_a_second_default_context_mapping() {
        // Arrange
        let mut topology = topology();
        topology
            .register_default_context(EPOCH_ONE, ObjectId::new(2))
            .expect("first default context should register");

        // Act
        let conflict = topology.register_context(
            EPOCH_ONE,
            "invented-default-id",
            ObjectId::new(3),
            CdpContextOwnership::Default,
        );

        // Assert
        assert!(matches!(
            conflict,
            Err(CdpTopologyError::DefaultContextConflict {
                existing_object_id
            }) if existing_object_id == ObjectId::new(2)
        ));
        assert_eq!(topology.metrics().live_contexts, 1);
    }

    #[test]
    fn topology_effects_apply_to_the_complete_portable_runtime_graph() {
        // Arrange
        let dispatcher = DirectDispatcher::with_object_kinds_and_topology(
            (),
            RuntimeLimits::new(8, 1),
            BROWSER_OBJECT_KIND_CATALOG,
            ObjectTopologyMode::Complete,
        )
        .expect("browser object catalog should be valid");
        dispatcher
            .register_object(
                ObjectId::new(1),
                None,
                pwright_browser_model::BROWSER_OBJECT_KIND.tag,
            )
            .expect("browser root should register");
        let mut topology = topology();
        let context = topology
            .register_context(
                EPOCH_ONE,
                "context-a",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should map");
        let page = topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("target-a", "session-a", "context-a"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("page should map");
        apply_lifecycle(&dispatcher, context.effects());
        apply_lifecycle(&dispatcher, page.effects());

        // Act
        let closed = topology
            .close_context(EPOCH_ONE, "context-a")
            .expect("context should map to one subtree disposal");
        apply_lifecycle(&dispatcher, closed.effects());

        // Assert
        assert_eq!(dispatcher.live_object_count(), Ok(1));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(1)), Ok(true));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(2)), Ok(false));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(3)), Ok(false));
        assert_eq!(dispatcher.is_object_live(ObjectId::new(4)), Ok(false));
    }

    #[test]
    fn page_attachment_waits_for_a_racing_context_registration() {
        // Arrange
        let mut topology = topology();
        let deferred = topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("target-a", "session-a", "late-context"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("unknown context should defer the page");

        // Act
        let admitted = topology
            .register_context(
                EPOCH_ONE,
                "late-context",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context registration should admit its pending page");

        // Assert
        assert_eq!(deferred.disposition(), CdpTopologyDisposition::Deferred);
        assert_eq!(topology.metrics().pending_pages, 0);
        assert_eq!(topology.metrics().live_pages, 1);
        assert_eq!(admitted.effects().len(), 3);
        assert_eq!(topology.page_object_id("target-a"), Some(ObjectId::new(3)));
    }

    #[test]
    fn pending_page_detach_never_creates_a_phantom_page() {
        // Arrange
        let mut topology = topology();
        topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("target-a", "session-a", "late-context"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("unknown context should defer the page");

        // Act
        let detached = topology
            .detach_target(EPOCH_ONE, "session-a", Some("target-a"))
            .expect("pending target detach should be accepted");
        let registered = topology
            .register_context(
                EPOCH_ONE,
                "late-context",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should still register");

        // Assert
        assert!(detached.effects().is_empty());
        assert_eq!(registered.effects().len(), 1);
        assert_eq!(topology.page_object_id("target-a"), None);
        assert_eq!(topology.metrics().pending_pages, 0);
    }

    #[test]
    fn popup_is_reported_only_after_both_pages_are_ready() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "opener", "session-opener", "context-a", 3, 4);
        topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("popup", "session-popup", "context-a").with_opener("opener"),
                CdpPageObjectIds::new(ObjectId::new(5), ObjectId::new(6)),
            )
            .expect("popup target should attach");
        let popup_ready = topology
            .mark_page_ready(EPOCH_ONE, "popup")
            .expect("popup should initialize");

        // Act
        let opener_ready = topology
            .mark_page_ready(EPOCH_ONE, "opener")
            .expect("opener should initialize");

        // Assert
        assert_eq!(popup_ready.effects().len(), 1);
        assert_eq!(
            opener_ready.effects(),
            &[
                CdpTopologyEffect::PageReady {
                    page: ObjectId::new(3),
                    context: ObjectId::new(2),
                },
                CdpTopologyEffect::PopupLinked {
                    opener: ObjectId::new(3),
                    popup: ObjectId::new(5),
                },
            ]
        );
        assert_eq!(
            topology
                .mark_page_ready(EPOCH_ONE, "opener")
                .expect("duplicate readiness should be harmless")
                .disposition(),
            CdpTopologyDisposition::Idempotent
        );
    }

    #[test]
    fn child_frame_waits_for_its_parent_without_losing_object_identity() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 3, 4);
        let child = topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::local("target-a", "child", "parent"),
                ObjectId::new(6),
            )
            .expect("missing frame parent should defer the child");

        // Act
        let parent = topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::local("target-a", "parent", "target-a"),
                ObjectId::new(5),
            )
            .expect("parent should admit itself and its child");

        // Assert
        assert_eq!(child.disposition(), CdpTopologyDisposition::Deferred);
        assert_eq!(parent.effects().len(), 2);
        assert_eq!(topology.frame_object_id("parent"), Some(ObjectId::new(5)));
        assert_eq!(topology.frame_object_id("child"), Some(ObjectId::new(6)));
        assert_eq!(topology.metrics().pending_frames, 0);
    }

    #[test]
    fn oopif_detach_roundtrip_preserves_a_frame_that_swaps_back_to_local() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 3, 4);
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::local("target-a", "frame-a", "target-a"),
                ObjectId::new(5),
            )
            .expect("local frame should attach");
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::remote("target-a", "frame-a", "target-a", "oopif-session"),
                ObjectId::new(99),
            )
            .expect("same frame should swap to a remote session");
        let detached = topology
            .detach_target(EPOCH_ONE, "oopif-session", Some("frame-a"))
            .expect("remote session detach should request synchronization");

        // Act
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::local("target-a", "frame-a", "target-a"),
                ObjectId::new(100),
            )
            .expect("in-flight local attachment should win the race");
        let confirmed = topology
            .confirm_frame_session_detached(EPOCH_ONE, "frame-a")
            .expect("confirmation after local attachment should be harmless");

        // Assert
        assert_eq!(
            detached.effects(),
            &[CdpTopologyEffect::FrameDetachSynchronizationRequired {
                page: ObjectId::new(3),
                frame: ObjectId::new(5),
            }]
        );
        assert_eq!(confirmed.disposition(), CdpTopologyDisposition::Idempotent);
        assert_eq!(topology.frame_object_id("frame-a"), Some(ObjectId::new(5)));
    }

    #[test]
    fn oopif_detach_confirmation_removes_only_the_detached_frame_subtree() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 3, 4);
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::remote("target-a", "frame-a", "target-a", "oopif-session"),
                ObjectId::new(5),
            )
            .expect("remote frame should attach");
        topology
            .detach_target(EPOCH_ONE, "oopif-session", Some("frame-a"))
            .expect("remote frame should enter ambiguous-detach state");

        // Act
        let confirmed = topology
            .confirm_frame_session_detached(EPOCH_ONE, "frame-a")
            .expect("synchronized detach should complete");

        // Assert
        assert_eq!(
            confirmed.effects(),
            &[CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id: ObjectId::new(5),
            })]
        );
        assert_eq!(topology.frame_object_id("frame-a"), None);
        assert_eq!(topology.page_object_id("target-a"), Some(ObjectId::new(3)));
    }

    #[test]
    fn main_frame_process_swap_retains_public_identity_and_cleans_descendants() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 3, 4);
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::local("target-a", "child", "target-a"),
                ObjectId::new(5),
            )
            .expect("child frame should attach");

        // Act
        let rebound = topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::main("target-a", "new-main"),
                ObjectId::new(99),
            )
            .expect("main frame should rebind to its new native ID");

        // Assert
        assert_eq!(topology.frame_object_id("target-a"), None);
        assert_eq!(topology.frame_object_id("new-main"), Some(ObjectId::new(4)));
        assert_eq!(topology.frame_object_id("child"), None);
        assert_eq!(
            rebound.effects(),
            &[
                CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                    object_id: ObjectId::new(5),
                }),
                CdpTopologyEffect::MainFrameRebound {
                    frame: ObjectId::new(4),
                },
            ]
        );
    }

    #[test]
    fn execution_worlds_are_session_scoped_replaced_and_cleared() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 3, 4);
        topology
            .attach_world(
                EPOCH_ONE,
                CdpWorldAttachment::new(
                    "session-a",
                    7,
                    "world-one",
                    "target-a",
                    CdpWorldKind::Main,
                ),
            )
            .expect("first main world should attach");

        // Act
        let replacement = topology
            .attach_world(
                EPOCH_ONE,
                CdpWorldAttachment::new(
                    "session-a",
                    8,
                    "world-two",
                    "target-a",
                    CdpWorldKind::Main,
                ),
            )
            .expect("new main world should replace the old document world");
        let cleared = topology
            .clear_worlds(EPOCH_ONE, "session-a")
            .expect("session clear should remove its worlds");

        // Assert
        assert_eq!(replacement.effects().len(), 1);
        assert_eq!(topology.world_count_for_frame(ObjectId::new(4)), 0);
        assert_eq!(
            cleared.effects(),
            &[CdpTopologyEffect::WorldRemoved {
                frame: ObjectId::new(4),
                kind: CdpWorldKind::Main,
            }]
        );
    }

    #[test]
    fn frame_execution_routes_preserve_local_and_oopif_session_identity() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 3, 4);
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::local("target-a", "local-child", "target-a"),
                ObjectId::new(5),
            )
            .expect("local child should attach");
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::remote("target-a", "remote-child", "target-a", "oopif-session"),
                ObjectId::new(6),
            )
            .expect("OOPIF child should attach");
        for world in [
            CdpWorldAttachment::new(
                "session-a",
                10,
                "local-unique",
                "local-child",
                CdpWorldKind::Main,
            ),
            CdpWorldAttachment::new(
                "oopif-session",
                11,
                "remote-unique",
                "remote-child",
                CdpWorldKind::Main,
            ),
        ] {
            topology
                .attach_world(EPOCH_ONE, world)
                .expect("main world should attach");
        }

        // Act
        let mut routes = topology
            .frame_execution_routes()
            .map(|route| {
                (
                    route.object_id(),
                    route.target_id().to_owned(),
                    route.session_id().to_owned(),
                    route.execution_context_id(),
                    route.unique_context_id().to_owned(),
                )
            })
            .collect::<Vec<_>>();
        routes.sort_by_key(|route| route.0);

        // Assert
        assert_eq!(
            routes,
            vec![
                (
                    ObjectId::new(5),
                    "target-a".to_owned(),
                    "session-a".to_owned(),
                    10,
                    "local-unique".to_owned(),
                ),
                (
                    ObjectId::new(6),
                    "remote-child".to_owned(),
                    "oopif-session".to_owned(),
                    11,
                    "remote-unique".to_owned(),
                ),
            ]
        );
    }

    #[test]
    fn crash_marks_one_page_and_exact_session_detach_disposes_only_that_page() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 3, 4);
        attach_page(&mut topology, "target-b", "session-b", "context-a", 5, 6);

        // Act
        let crashed = topology
            .crash_target(EPOCH_ONE, "target-a", "crashed", 137)
            .expect("target crash should be recorded");
        let detached = topology
            .detach_target(EPOCH_ONE, "session-a", Some("target-a"))
            .expect("exact crashed target should detach");

        // Assert
        assert_eq!(crashed.effects().len(), 1);
        assert_eq!(
            detached.effects(),
            &[CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id: ObjectId::new(3),
            })]
        );
        assert_eq!(topology.page_object_id("target-a"), None);
        assert_eq!(topology.page_object_id("target-b"), Some(ObjectId::new(5)));
        assert_eq!(
            topology.context_object_id("context-a"),
            Some(ObjectId::new(2))
        );
    }

    #[test]
    fn reconnect_ignores_old_detach_and_reconciles_targets_by_native_identity() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 3, 4);
        attach_page(&mut topology, "target-b", "session-b", "context-a", 5, 6);
        topology
            .begin_reconnect(EPOCH_TWO)
            .expect("new connection epoch should begin");
        topology
            .observe_context(EPOCH_TWO, "context-a")
            .expect("authoritative context enumeration should retain the context");
        topology
            .attach_page(
                EPOCH_TWO,
                CdpPageTarget::new("target-a", "new-session-a", "context-a"),
                CdpPageObjectIds::new(ObjectId::new(99), ObjectId::new(100)),
            )
            .expect("known target should rebind without new public IDs");

        // Act
        let stale = topology
            .detach_target(EPOCH_ONE, "session-a", Some("target-a"))
            .expect("old reader event should be ignored");
        let reconciled = topology
            .finish_reconnect()
            .expect("unseen targets should be disposed after reconciliation");

        // Assert
        assert_eq!(
            stale.disposition(),
            CdpTopologyDisposition::IgnoredStaleEpoch
        );
        assert_eq!(topology.page_object_id("target-a"), Some(ObjectId::new(3)));
        assert_eq!(topology.page_object_id("target-b"), None);
        assert!(reconciled.effects().contains(&CdpTopologyEffect::Lifecycle(
            ControlMessage::Dispose {
                object_id: ObjectId::new(5),
            }
        )));
        assert_eq!(
            topology.page_state("target-a"),
            Some(&CdpPageState::Initializing)
        );
    }

    #[test]
    fn reconnect_reconciles_missing_contexts_and_unbound_frames() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        register_context(&mut topology, "context-b", 3);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 4, 5);
        attach_page(&mut topology, "target-b", "session-b", "context-b", 6, 7);
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::local("target-a", "stale-child", "target-a"),
                ObjectId::new(8),
            )
            .expect("local child should attach");
        topology
            .begin_reconnect(EPOCH_TWO)
            .expect("new reconstruction epoch should begin");
        topology
            .observe_context(EPOCH_TWO, "context-a")
            .expect("context-a should be present in the authoritative snapshot");
        topology
            .attach_page(
                EPOCH_TWO,
                CdpPageTarget::new("target-a", "new-session-a", "context-a"),
                CdpPageObjectIds::new(ObjectId::new(99), ObjectId::new(100)),
            )
            .expect("known Page should retain its portable IDs");

        // Act
        let reconciled = topology
            .finish_reconnect()
            .expect("snapshot reconciliation should complete");

        // Assert
        assert_eq!(
            topology.context_object_id("context-a"),
            Some(ObjectId::new(2))
        );
        assert_eq!(topology.context_object_id("context-b"), None);
        assert_eq!(topology.page_object_id("target-a"), Some(ObjectId::new(4)));
        assert_eq!(topology.page_object_id("target-b"), None);
        assert_eq!(topology.frame_object_id("stale-child"), None);
        assert!(reconciled.effects().contains(&CdpTopologyEffect::Lifecycle(
            ControlMessage::Dispose {
                object_id: ObjectId::new(3),
            }
        )));
        assert!(reconciled.effects().contains(&CdpTopologyEffect::Lifecycle(
            ControlMessage::Dispose {
                object_id: ObjectId::new(8),
            }
        )));
        assert_eq!(
            reconciled.effects().last(),
            Some(&CdpTopologyEffect::ReconnectFinished {
                epoch: EPOCH_TWO,
                live_pages: 1,
            })
        );
    }

    #[test]
    fn closing_one_context_removes_exactly_its_owned_subtree() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        register_context(&mut topology, "context-b", 3);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 4, 5);
        attach_page(&mut topology, "target-b", "session-b", "context-b", 6, 7);

        // Act
        let closed = topology
            .close_context(EPOCH_ONE, "context-a")
            .expect("context should close");

        // Assert
        assert_eq!(
            closed.effects(),
            &[CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id: ObjectId::new(2),
            })]
        );
        assert_eq!(topology.context_object_id("context-a"), None);
        assert_eq!(topology.page_object_id("target-a"), None);
        assert_eq!(
            topology.context_object_id("context-b"),
            Some(ObjectId::new(3))
        );
        assert_eq!(topology.page_object_id("target-b"), Some(ObjectId::new(6)));
    }

    #[test]
    fn bounds_and_reserved_object_ids_fail_before_mutating_topology() {
        // Arrange
        let mut topology = CdpTopology::new(
            ObjectId::new(1),
            EPOCH_ONE,
            CdpTopologyLimits::new(4, 1, 1, 1, 0, 8),
        )
        .expect("browser root should fit");
        topology
            .register_context(
                EPOCH_ONE,
                "ctx-one",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("first context should fit");

        // Act
        let capacity = topology
            .register_context(
                EPOCH_ONE,
                "ctx-two",
                ObjectId::new(3),
                CdpContextOwnership::Owned,
            )
            .expect_err("second context should exceed its bound");
        let duplicate = topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("page", "session", "ctx-one"),
                CdpPageObjectIds::new(ObjectId::new(1), ObjectId::new(3)),
            )
            .expect_err("browser object ID must not be reused");

        // Assert
        assert_eq!(
            capacity,
            CdpTopologyError::CapacityExceeded {
                resource: "browser context",
                limit: 1,
            }
        );
        assert_eq!(
            duplicate,
            CdpTopologyError::ObjectIdAlreadyReserved {
                object_id: ObjectId::new(1),
            }
        );
        assert_eq!(topology.metrics().live_objects, 2);
        assert_eq!(topology.metrics().live_pages, 0);
    }

    #[test]
    fn frame_remove_disposes_subtree_while_swap_preserves_the_frame() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 3, 4);
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::local("target-a", "frame-a", "target-a"),
                ObjectId::new(5),
            )
            .expect("frame should attach");
        let swapped = topology
            .detach_frame(EPOCH_ONE, "frame-a", CdpFrameDetachReason::Swap)
            .expect("swap should preserve frame identity");

        // Act
        let removed = topology
            .detach_frame(EPOCH_ONE, "frame-a", CdpFrameDetachReason::Remove)
            .expect("remove should dispose frame identity");

        // Assert
        assert_eq!(swapped.disposition(), CdpTopologyDisposition::Idempotent);
        assert_eq!(
            removed.effects(),
            &[CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id: ObjectId::new(5),
            })]
        );
        assert_eq!(topology.frame_object_id("frame-a"), None);
    }

    #[test]
    fn generated_cdp_events_normalize_into_explicit_topology_inputs() {
        // Arrange
        let page_event = serde_json::from_value::<
            pwright_cdp::generated::target::AttachedToTargetEvent,
        >(json!({
            "sessionId": "page-session",
            "waitingForDebugger": true,
            "targetInfo": {
                "targetId": "popup-target",
                "type": "page",
                "title": "",
                "url": "about:blank",
                "attached": true,
                "openerId": "opener-target",
                "canAccessOpener": true,
                "browserContextId": "context-a"
            }
        }))
        .expect("page event fixture should decode");
        let oopif_event = serde_json::from_value::<
            pwright_cdp::generated::target::AttachedToTargetEvent,
        >(json!({
            "sessionId": "oopif-session",
            "waitingForDebugger": true,
            "targetInfo": {
                "targetId": "frame-a",
                "type": "iframe",
                "title": "",
                "url": "https://frame.example",
                "attached": true,
                "canAccessOpener": false,
                "parentFrameId": "parent-frame",
                "browserContextId": "context-a"
            }
        }))
        .expect("OOPIF event fixture should decode");

        // Act
        let page =
            CdpPageTarget::from_attached_page(&page_event).expect("page target should normalize");
        let frame = CdpFrameAttachment::from_attached_oopif("page-target", &oopif_event)
            .expect("OOPIF target should normalize");

        // Assert
        assert_eq!(
            page,
            CdpPageTarget::new("popup-target", "page-session", "context-a")
                .with_opener("opener-target")
        );
        assert_eq!(
            frame,
            CdpFrameAttachment::remote("page-target", "frame-a", "parent-frame", "oopif-session")
        );
    }

    #[test]
    fn execution_context_normalization_retains_only_frame_main_and_utility_worlds() {
        // Arrange
        let make_event = |id: i64, name: &str, is_default: bool| {
            serde_json::from_value::<pwright_cdp::generated::runtime::ExecutionContextCreatedEvent>(
                json!({
                    "context": {
                        "id": id,
                        "origin": "https://example.test",
                        "name": name,
                        "uniqueId": format!("unique-{id}"),
                        "auxData": {
                            "frameId": "frame-a",
                            "isDefault": is_default,
                            "type": "default"
                        }
                    }
                }),
            )
            .expect("execution context fixture should decode")
        };
        let main = make_event(1, "", true);
        let utility = make_event(2, "__pwright_utility", false);
        let unrelated = make_event(3, "extension-world", false);

        // Act
        let main = CdpWorldAttachment::from_execution_context(
            "session-a",
            &main,
            Some("__pwright_utility"),
        )
        .expect("main world should normalize");
        let utility = CdpWorldAttachment::from_execution_context(
            "session-a",
            &utility,
            Some("__pwright_utility"),
        )
        .expect("utility world should normalize");
        let unrelated = CdpWorldAttachment::from_execution_context(
            "session-a",
            &unrelated,
            Some("__pwright_utility"),
        )
        .expect("unrelated world should be safely classified");

        // Assert
        assert_eq!(
            main.expect("main world should be retained").kind,
            CdpWorldKind::Main
        );
        assert_eq!(
            utility.expect("utility world should be retained").kind,
            CdpWorldKind::Utility
        );
        assert_eq!(unrelated, None);
    }

    #[test]
    fn malformed_generated_events_fail_before_topology_mutation() {
        // Arrange
        let worker_event = serde_json::from_value::<
            pwright_cdp::generated::target::AttachedToTargetEvent,
        >(json!({
            "sessionId": "worker-session",
            "waitingForDebugger": true,
            "targetInfo": {
                "targetId": "worker-a",
                "type": "worker",
                "title": "",
                "url": "worker.js",
                "attached": true,
                "canAccessOpener": false,
                "browserContextId": "context-a"
            }
        }))
        .expect("worker event fixture should decode");

        // Act
        let error = CdpPageTarget::from_attached_page(&worker_event)
            .expect_err("worker must not be admitted as a page");

        // Assert
        assert_eq!(
            error,
            CdpTopologyEventError::UnexpectedTargetType {
                expected: "page",
                actual: "worker".to_owned(),
            }
        );
    }

    #[test]
    fn detached_pending_oopif_releases_its_reserved_session_and_object() {
        // Arrange
        let mut topology = topology();
        register_context(&mut topology, "context-a", 2);
        attach_page(&mut topology, "target-a", "session-a", "context-a", 3, 4);
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::remote(
                    "target-a",
                    "pending-frame",
                    "missing-parent",
                    "pending-session",
                ),
                ObjectId::new(5),
            )
            .expect("missing parent should defer OOPIF");

        // Act
        let detached = topology
            .detach_target(EPOCH_ONE, "pending-session", Some("pending-frame"))
            .expect("pending OOPIF detach should release reservations");
        let reused = topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::local("target-a", "frame-b", "target-a"),
                ObjectId::new(5),
            )
            .expect("released object ID should be reusable");

        // Assert
        assert!(detached.effects().is_empty());
        assert_eq!(reused.disposition(), CdpTopologyDisposition::Applied);
        assert_eq!(topology.frame_object_id("frame-b"), Some(ObjectId::new(5)));
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CdpTopologyUpdate {
    disposition: CdpTopologyDisposition,
    effects: Vec<CdpTopologyEffect>,
}

impl CdpTopologyUpdate {
    fn new(disposition: CdpTopologyDisposition, effects: Vec<CdpTopologyEffect>) -> Self {
        Self {
            disposition,
            effects,
        }
    }

    fn applied(effects: Vec<CdpTopologyEffect>) -> Self {
        Self::new(CdpTopologyDisposition::Applied, effects)
    }

    pub fn idempotent() -> Self {
        Self::new(CdpTopologyDisposition::Idempotent, Vec::new())
    }

    /// Adds caller-owned runtime cleanup to the same staged topology commit.
    /// This is used by optional child capabilities whose portable objects are
    /// descendants of a topology-owned Page but are not stored by CDP topology.
    pub fn lifecycle(messages: Vec<ControlMessage>) -> Self {
        Self::applied(
            messages
                .into_iter()
                .map(CdpTopologyEffect::Lifecycle)
                .collect(),
        )
    }

    /// Combines updates committed during one caller-visible event turn while
    /// preserving effect order.
    pub fn combine(mut self, next: Self) -> Self {
        self.disposition = match (self.disposition, next.disposition) {
            (CdpTopologyDisposition::IgnoredStaleEpoch, _)
            | (_, CdpTopologyDisposition::IgnoredStaleEpoch) => {
                CdpTopologyDisposition::IgnoredStaleEpoch
            }
            (CdpTopologyDisposition::Applied, _) | (_, CdpTopologyDisposition::Applied) => {
                CdpTopologyDisposition::Applied
            }
            (CdpTopologyDisposition::Deferred, _) | (_, CdpTopologyDisposition::Deferred) => {
                CdpTopologyDisposition::Deferred
            }
            _ => CdpTopologyDisposition::Idempotent,
        };
        self.effects.extend(next.effects);
        self
    }

    fn stale() -> Self {
        Self::new(CdpTopologyDisposition::IgnoredStaleEpoch, Vec::new())
    }

    pub const fn disposition(&self) -> CdpTopologyDisposition {
        self.disposition
    }

    pub fn effects(&self) -> &[CdpTopologyEffect] {
        &self.effects
    }

    pub fn into_effects(self) -> Vec<CdpTopologyEffect> {
        self.effects
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpTopologyLimits {
    pub objects: usize,
    pub contexts: usize,
    pub pages: usize,
    pub frames: usize,
    pub worlds: usize,
    pub native_id_bytes: usize,
}

impl CdpTopologyLimits {
    pub const fn new(
        objects: usize,
        contexts: usize,
        pages: usize,
        frames: usize,
        worlds: usize,
        native_id_bytes: usize,
    ) -> Self {
        Self {
            objects,
            contexts,
            pages,
            frames,
            worlds,
            native_id_bytes,
        }
    }

    /// Maximum bytes held by all native strings in a fully populated topology.
    ///
    /// The multiplier includes every retained duplicate: two context IDs,
    /// nine page-related strings, seven frame-related strings, and two world
    /// strings. Page and frame terms include their possible session-map key
    /// and owner value. Pending and materialized records share the same count
    /// limit, so only the larger representation is used.
    pub fn maximum_retained_native_bytes(self) -> Option<u64> {
        let contexts = u64::try_from(self.contexts).ok()?.checked_mul(2)?;
        let pages = u64::try_from(self.pages).ok()?.checked_mul(9)?;
        let frames = u64::try_from(self.frames).ok()?.checked_mul(7)?;
        let worlds = u64::try_from(self.worlds).ok()?.checked_mul(2)?;
        let string_slots = contexts
            .checked_add(pages)?
            .checked_add(frames)?
            .checked_add(worlds)?;
        string_slots.checked_mul(u64::try_from(self.native_id_bytes).ok()?)
    }
}

/// Production resource declaration for a CDP topology.
///
/// A mutation is evaluated against a shadow copy so topology errors and
/// resource errors cannot partially change the live mapping. The requirement
/// therefore covers both the live state and one maximum-size shadow state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CdpTopologyProductionProfile {
    limits: CdpTopologyLimits,
    resource_scope: ResourceScope,
    maximum_retained_native_bytes: u64,
    requirements: ResourceRequirements,
}

impl CdpTopologyProductionProfile {
    pub fn new(
        limits: CdpTopologyLimits,
        resource_scope: ResourceScope,
    ) -> Result<Self, CdpTopologyProductionError> {
        if !matches!(resource_scope, ResourceScope::Extension { .. }) {
            return Err(CdpTopologyProductionError::InvalidResourceScope { resource_scope });
        }
        let maximum_retained_native_bytes = limits
            .maximum_retained_native_bytes()
            .ok_or(CdpTopologyProductionError::CapacityOverflow)?;
        let transactional_bytes = maximum_retained_native_bytes
            .checked_mul(2)
            .ok_or(CdpTopologyProductionError::CapacityOverflow)?;
        let requirements = if transactional_bytes == 0 {
            ResourceRequirements::none()
        } else {
            ResourceRequirements::none()
                .with_class(
                    ResourceClass::BackendStateBytes,
                    transactional_bytes,
                    transactional_bytes,
                )
                .with_metadata(1, 2)
        };
        Ok(Self {
            limits,
            resource_scope,
            maximum_retained_native_bytes,
            requirements,
        })
    }

    pub const fn limits(self) -> CdpTopologyLimits {
        self.limits
    }

    pub const fn resource_scope(self) -> ResourceScope {
        self.resource_scope
    }

    pub const fn maximum_retained_native_bytes(self) -> u64 {
        self.maximum_retained_native_bytes
    }

    pub const fn requirements(self) -> ResourceRequirements {
        self.requirements
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CdpTopologyMetrics {
    pub live_objects: usize,
    pub live_contexts: usize,
    pub live_pages: usize,
    pub live_frames: usize,
    pub live_worlds: usize,
    pub pending_pages: usize,
    pub pending_frames: usize,
    pub peak_objects: usize,
    pub peak_contexts: usize,
    pub peak_pages: usize,
    pub peak_frames: usize,
    pub peak_worlds: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpTopologyEventError {
    UnexpectedTargetType {
        expected: &'static str,
        actual: String,
    },
    MissingBrowserContextId {
        target_id: String,
    },
    MissingParentFrameId {
        target_id: String,
    },
    UnexpectedFrameDetachReason {
        actual: String,
    },
    MalformedExecutionContextAuxData,
}

impl Display for CdpTopologyEventError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedTargetType { expected, actual } => write!(
                formatter,
                "expected CDP target type `{expected}`, got `{actual}`"
            ),
            Self::MissingBrowserContextId { target_id } => write!(
                formatter,
                "CDP page target `{target_id}` has no browser context ID"
            ),
            Self::MissingParentFrameId { target_id } => {
                write!(
                    formatter,
                    "CDP iframe target `{target_id}` has no parent frame ID"
                )
            }
            Self::UnexpectedFrameDetachReason { actual } => {
                write!(formatter, "unsupported CDP frame detach reason `{actual}`")
            }
            Self::MalformedExecutionContextAuxData => {
                formatter.write_str("CDP execution-context auxData has an invalid frame mapping")
            }
        }
    }
}

impl Error for CdpTopologyEventError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CdpTopologyError {
    FutureEpoch {
        current: ConnectionEpoch,
        event: ConnectionEpoch,
    },
    ReconnectEpochNotAdvanced {
        current: ConnectionEpoch,
        requested: ConnectionEpoch,
    },
    EmptyNativeId {
        field: &'static str,
    },
    NativeIdTooLong {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    CapacityExceeded {
        resource: &'static str,
        limit: usize,
    },
    ObjectIdAlreadyReserved {
        object_id: ObjectId,
    },
    ContextConflict {
        browser_context_id: String,
    },
    DefaultContextConflict {
        existing_object_id: ObjectId,
    },
    PageConflict {
        target_id: String,
    },
    FrameConflict {
        frame_id: String,
    },
    SessionInUse {
        session_id: String,
    },
    UnknownContext {
        browser_context_id: String,
    },
    UnknownContextObject {
        object_id: ObjectId,
    },
    UnknownPage {
        target_id: String,
    },
    UnknownPageObject {
        object_id: ObjectId,
    },
    UnknownFrame {
        frame_id: String,
    },
    UnknownSession {
        session_id: String,
    },
    TargetSessionMismatch {
        session_id: String,
        expected_target: String,
        actual_target: String,
    },
    PageCrashed {
        target_id: String,
    },
    ObjectGraph(ObjectGraphError),
}

impl Display for CdpTopologyError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::FutureEpoch { current, event } => write!(
                formatter,
                "event epoch {} is newer than active epoch {}",
                event.get(),
                current.get()
            ),
            Self::ReconnectEpochNotAdvanced { current, requested } => write!(
                formatter,
                "reconnect epoch {} must be newer than active epoch {}",
                requested.get(),
                current.get()
            ),
            Self::EmptyNativeId { field } => write!(formatter, "{field} must not be empty"),
            Self::NativeIdTooLong {
                field,
                actual,
                limit,
            } => write!(
                formatter,
                "{field} is {actual} bytes, exceeding the {limit}-byte limit"
            ),
            Self::CapacityExceeded { resource, limit } => {
                write!(formatter, "{resource} limit of {limit} was reached")
            }
            Self::ObjectIdAlreadyReserved { object_id } => {
                write!(formatter, "object ID `{object_id}` is already reserved")
            }
            Self::ContextConflict { browser_context_id } => write!(
                formatter,
                "CDP browser context `{browser_context_id}` has a conflicting mapping"
            ),
            Self::DefaultContextConflict { existing_object_id } => write!(
                formatter,
                "default CDP browser context is already mapped to portable object `{existing_object_id}`"
            ),
            Self::PageConflict { target_id } => {
                write!(
                    formatter,
                    "CDP target `{target_id}` has a conflicting page mapping"
                )
            }
            Self::FrameConflict { frame_id } => {
                write!(
                    formatter,
                    "CDP frame `{frame_id}` has a conflicting mapping"
                )
            }
            Self::SessionInUse { session_id } => {
                write!(formatter, "CDP session `{session_id}` is already in use")
            }
            Self::UnknownContext { browser_context_id } => write!(
                formatter,
                "CDP browser context `{browser_context_id}` is not mapped"
            ),
            Self::UnknownContextObject { object_id } => write!(
                formatter,
                "portable BrowserContext `{object_id}` has no CDP context mapping"
            ),
            Self::UnknownPage { target_id } => {
                write!(formatter, "CDP page target `{target_id}` is not mapped")
            }
            Self::UnknownPageObject { object_id } => {
                write!(
                    formatter,
                    "portable Page `{object_id}` has no CDP target mapping"
                )
            }
            Self::UnknownFrame { frame_id } => {
                write!(formatter, "CDP frame `{frame_id}` is not mapped")
            }
            Self::UnknownSession { session_id } => {
                write!(formatter, "CDP session `{session_id}` is not mapped")
            }
            Self::TargetSessionMismatch {
                session_id,
                expected_target,
                actual_target,
            } => write!(
                formatter,
                "CDP session `{session_id}` belongs to `{expected_target}`, not `{actual_target}`"
            ),
            Self::PageCrashed { target_id } => {
                write!(
                    formatter,
                    "CDP page target `{target_id}` crashed before becoming ready"
                )
            }
            Self::ObjectGraph(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for CdpTopologyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ObjectGraph(error) => Some(error),
            _ => None,
        }
    }
}

impl From<ObjectGraphError> for CdpTopologyError {
    fn from(error: ObjectGraphError) -> Self {
        Self::ObjectGraph(error)
    }
}

#[derive(Debug)]
pub enum CdpTopologyProductionError {
    CapacityOverflow,
    TransactionInProgress,
    TransactionUnavailable,
    TransactionIdExhausted,
    TransactionOwnerMismatch,
    InvalidResourceScope { resource_scope: ResourceScope },
    RetainedBytesInvariant { actual: u64, maximum: u64 },
    Topology(CdpTopologyError),
    ProductionResource(ProductionResourceError),
    Resource(ResourceError),
}

impl Display for CdpTopologyProductionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CapacityOverflow => formatter.write_str(
                "CDP topology production resource requirement exceeds the resource counter range",
            ),
            Self::TransactionInProgress => {
                formatter.write_str("a CDP topology transaction is already in progress")
            }
            Self::TransactionUnavailable => {
                formatter.write_str("the prepared CDP topology transaction is no longer active")
            }
            Self::TransactionIdExhausted => {
                formatter.write_str("CDP topology transaction IDs are exhausted")
            }
            Self::TransactionOwnerMismatch => {
                formatter.write_str("prepared CDP topology transaction belongs to another owner")
            }
            Self::InvalidResourceScope { resource_scope } => write!(
                formatter,
                "CDP topology production state requires an extension resource scope, got `{resource_scope}`"
            ),
            Self::RetainedBytesInvariant { actual, maximum } => write!(
                formatter,
                "CDP topology retained {actual} native bytes, exceeding its derived {maximum}-byte maximum"
            ),
            Self::Topology(error) => Display::fmt(error, formatter),
            Self::ProductionResource(error) => Display::fmt(error, formatter),
            Self::Resource(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for CdpTopologyProductionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Topology(error) => Some(error),
            Self::ProductionResource(error) => Some(error),
            Self::Resource(error) => Some(error),
            Self::CapacityOverflow
            | Self::TransactionInProgress
            | Self::TransactionUnavailable
            | Self::TransactionIdExhausted
            | Self::TransactionOwnerMismatch
            | Self::InvalidResourceScope { .. }
            | Self::RetainedBytesInvariant { .. } => None,
        }
    }
}

impl From<CdpTopologyError> for CdpTopologyProductionError {
    fn from(error: CdpTopologyError) -> Self {
        Self::Topology(error)
    }
}

impl From<ProductionResourceError> for CdpTopologyProductionError {
    fn from(error: ProductionResourceError) -> Self {
        Self::ProductionResource(error)
    }
}

impl From<ResourceError> for CdpTopologyProductionError {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

#[derive(Clone, Debug)]
struct ContextRecord {
    object_id: ObjectId,
    ownership: CdpContextOwnership,
    seen_epoch: ConnectionEpoch,
}

#[derive(Clone, Debug)]
struct PageRecord {
    object_ids: CdpPageObjectIds,
    browser_context_id: String,
    session_id: Option<String>,
    opener_target_id: Option<String>,
    main_frame_native_id: String,
    state: CdpPageState,
    seen_epoch: ConnectionEpoch,
    ready_reported: bool,
    popup_reported: bool,
}

#[derive(Clone, Debug)]
struct PendingPage {
    target: CdpPageTarget,
    object_ids: CdpPageObjectIds,
    state: CdpPageState,
    seen_epoch: ConnectionEpoch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum FrameHost {
    Main,
    Local,
    Remote { session_id: String },
    AwaitingLocalOrDetach,
    Unbound,
}

#[derive(Clone, Debug)]
struct FrameRecord {
    object_id: ObjectId,
    page_target_id: String,
    parent_frame_id: Option<String>,
    host: FrameHost,
    main: bool,
}

#[derive(Clone, Debug)]
struct PendingFrame {
    attachment: CdpFrameAttachment,
    object_id: ObjectId,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum SessionOwner {
    Page(String),
    PendingPage(String),
    Frame(String),
    PendingFrame(String),
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct WorldKey {
    session_id: String,
    execution_context_id: i64,
}

#[derive(Clone, Debug)]
struct WorldRecord {
    frame_object_id: ObjectId,
    unique_id: String,
    kind: CdpWorldKind,
}

/// Pure, bounded CDP-to-Playwright object mapping state.
///
/// The state machine performs no I/O and allocates no public object IDs. The
/// composition root supplies globally unique IDs, applies returned lifecycle
/// effects to the runtime, and performs requested CDP synchronization before
/// confirming an ambiguous OOPIF detach.
#[derive(Clone, Debug)]
pub struct CdpTopology {
    browser_object_id: ObjectId,
    epoch: ConnectionEpoch,
    limits: CdpTopologyLimits,
    graph: ObjectGraph<pwright_core::ObjectKindTag>,
    reserved_objects: BTreeSet<ObjectId>,
    contexts: BTreeMap<String, ContextRecord>,
    context_by_object: BTreeMap<ObjectId, String>,
    pages: BTreeMap<String, PageRecord>,
    page_by_object: BTreeMap<ObjectId, String>,
    pending_pages: BTreeMap<String, PendingPage>,
    frames: BTreeMap<String, FrameRecord>,
    frame_by_object: BTreeMap<ObjectId, String>,
    pending_frames: BTreeMap<String, PendingFrame>,
    sessions: BTreeMap<String, SessionOwner>,
    worlds: BTreeMap<WorldKey, WorldRecord>,
    metrics: CdpTopologyMetrics,
}

impl CdpTopology {
    pub fn new(
        browser_object_id: ObjectId,
        epoch: ConnectionEpoch,
        limits: CdpTopologyLimits,
    ) -> Result<Self, CdpTopologyError> {
        if limits.objects == 0 {
            return Err(CdpTopologyError::CapacityExceeded {
                resource: "object",
                limit: 0,
            });
        }
        let mut graph = ObjectGraph::new(limits.objects);
        graph.insert(browser_object_id, None, BROWSER_OBJECT_KIND.tag)?;
        let mut reserved_objects = BTreeSet::new();
        reserved_objects.insert(browser_object_id);
        let mut topology = Self {
            browser_object_id,
            epoch,
            limits,
            graph,
            reserved_objects,
            contexts: BTreeMap::new(),
            context_by_object: BTreeMap::new(),
            pages: BTreeMap::new(),
            page_by_object: BTreeMap::new(),
            pending_pages: BTreeMap::new(),
            frames: BTreeMap::new(),
            frame_by_object: BTreeMap::new(),
            pending_frames: BTreeMap::new(),
            sessions: BTreeMap::new(),
            worlds: BTreeMap::new(),
            metrics: CdpTopologyMetrics::default(),
        };
        topology.refresh_metrics();
        Ok(topology)
    }

    pub const fn epoch(&self) -> ConnectionEpoch {
        self.epoch
    }

    pub const fn limits(&self) -> CdpTopologyLimits {
        self.limits
    }

    pub const fn metrics(&self) -> CdpTopologyMetrics {
        self.metrics
    }

    /// Exact variable-size native string allocation retained by this state.
    ///
    /// Inputs are compacted before insertion, and internal copies are cloned
    /// from compact strings. Counting capacity therefore catches regressions
    /// that accidentally retain caller-provided spare capacity.
    pub fn retained_native_bytes(&self) -> usize {
        let mut bytes = 0_usize;
        let mut add = |value: &String| {
            bytes = bytes
                .checked_add(value.capacity())
                .expect("simultaneously allocated strings fit in usize");
        };

        self.contexts.keys().for_each(&mut add);
        self.context_by_object.values().for_each(&mut add);
        for (target_id, page) in &self.pages {
            add(target_id);
            add(&page.browser_context_id);
            if let Some(session_id) = &page.session_id {
                add(session_id);
            }
            if let Some(opener_target_id) = &page.opener_target_id {
                add(opener_target_id);
            }
            add(&page.main_frame_native_id);
            if let CdpPageState::Crashed { status, .. } = &page.state {
                add(status);
            }
        }
        self.page_by_object.values().for_each(&mut add);
        for (target_id, page) in &self.pending_pages {
            add(target_id);
            add(&page.target.target_id);
            add(&page.target.session_id);
            add(&page.target.browser_context_id);
            if let Some(opener_target_id) = &page.target.opener_target_id {
                add(opener_target_id);
            }
            if let CdpPageState::Crashed { status, .. } = &page.state {
                add(status);
            }
        }
        for (frame_id, frame) in &self.frames {
            add(frame_id);
            add(&frame.page_target_id);
            if let Some(parent_frame_id) = &frame.parent_frame_id {
                add(parent_frame_id);
            }
            if let FrameHost::Remote { session_id } = &frame.host {
                add(session_id);
            }
        }
        self.frame_by_object.values().for_each(&mut add);
        for (frame_id, frame) in &self.pending_frames {
            add(frame_id);
            add(&frame.attachment.page_target_id);
            add(&frame.attachment.frame_id);
            if let Some(parent_frame_id) = &frame.attachment.parent_frame_id {
                add(parent_frame_id);
            }
            if let Some(session_id) = &frame.attachment.session_id {
                add(session_id);
            }
        }
        for (session_id, owner) in &self.sessions {
            add(session_id);
            match owner {
                SessionOwner::Page(value)
                | SessionOwner::PendingPage(value)
                | SessionOwner::Frame(value)
                | SessionOwner::PendingFrame(value) => add(value),
            }
        }
        for (key, world) in &self.worlds {
            add(&key.session_id);
            add(&world.unique_id);
        }
        bytes
    }

    pub fn context_object_id(&self, browser_context_id: &str) -> Option<ObjectId> {
        self.contexts
            .get(browser_context_id)
            .map(|record| record.object_id)
    }

    pub fn context_ownership(&self, browser_context_id: &str) -> Option<CdpContextOwnership> {
        self.contexts
            .get(browser_context_id)
            .map(|record| record.ownership)
    }

    /// Resolves one portable BrowserContext without copying its native ID.
    pub fn browser_context_route(&self, object_id: ObjectId) -> Option<CdpBrowserContextRoute<'_>> {
        let browser_context_id = self.context_by_object.get(&object_id)?;
        let context = self.contexts.get(browser_context_id)?;
        Some(CdpBrowserContextRoute {
            object_id,
            browser_context_id,
            ownership: context.ownership,
        })
    }

    /// Resolves the unique default BrowserContext, whose native key is empty
    /// because Chromium does not assign it a browser-context ID.
    pub fn default_context_route(&self) -> Option<CdpBrowserContextRoute<'_>> {
        self.contexts
            .iter()
            .find_map(|(browser_context_id, context)| {
                (context.ownership == CdpContextOwnership::Default).then_some(
                    CdpBrowserContextRoute {
                        object_id: context.object_id,
                        browser_context_id,
                        ownership: context.ownership,
                    },
                )
            })
    }

    /// Enumerates every materialized portable BrowserContext without copying
    /// native IDs. This includes empty Contexts, which still need to retain
    /// Context-scoped listener state for Pages created later.
    pub fn browser_context_routes(&self) -> impl Iterator<Item = CdpBrowserContextRoute<'_>> {
        self.context_by_object
            .iter()
            .filter_map(|(object_id, browser_context_id)| {
                let context = self.contexts.get(browser_context_id)?;
                Some(CdpBrowserContextRoute {
                    object_id: *object_id,
                    browser_context_id,
                    ownership: context.ownership,
                })
            })
    }

    pub fn page_object_id(&self, target_id: &str) -> Option<ObjectId> {
        self.pages
            .get(target_id)
            .map(|record| record.object_ids.page)
    }

    /// Enumerates command-capable Page routes without copying native IDs.
    /// Pages awaiting context materialization or reconnect session rebinding
    /// are intentionally absent.
    pub fn page_session_routes(&self) -> impl Iterator<Item = CdpPageSessionRoute<'_>> {
        self.page_by_object
            .iter()
            .filter_map(|(object_id, target_id)| {
                let page = self.pages.get(target_id)?;
                let session_id = page.session_id.as_deref()?;
                let browser_context_object_id =
                    self.contexts.get(&page.browser_context_id)?.object_id;
                Some(CdpPageSessionRoute {
                    object_id: *object_id,
                    browser_context_object_id,
                    target_id,
                    session_id,
                })
            })
    }

    /// Enumerates Frames whose current main world has been observed.
    ///
    /// The world key supplies the exact session. Its already-validated session
    /// owner supplies the target identity used to construct a target-local CDP
    /// client. Pending owners cannot retain worlds and are excluded.
    pub fn frame_execution_routes(&self) -> impl Iterator<Item = CdpFrameExecutionRoute<'_>> {
        self.frame_execution_routes_for_world(CdpWorldKind::Main)
    }

    /// Enumerates Frames whose current Pwright utility world has been
    /// observed. This is an independent view from user-facing main-world
    /// evaluation routes.
    pub fn frame_utility_execution_routes(
        &self,
    ) -> impl Iterator<Item = CdpFrameExecutionRoute<'_>> {
        self.frame_execution_routes_for_world(CdpWorldKind::Utility)
    }

    fn frame_execution_routes_for_world(
        &self,
        kind: CdpWorldKind,
    ) -> impl Iterator<Item = CdpFrameExecutionRoute<'_>> {
        self.worlds.iter().filter_map(move |(key, world)| {
            if world.kind != kind {
                return None;
            }
            let target_id = match self.sessions.get(&key.session_id)? {
                SessionOwner::Page(target_id) => target_id.as_str(),
                SessionOwner::Frame(frame_id) => frame_id.as_str(),
                SessionOwner::PendingPage(_) | SessionOwner::PendingFrame(_) => return None,
            };
            Some(CdpFrameExecutionRoute {
                object_id: world.frame_object_id,
                target_id,
                session_id: key.session_id.as_str(),
                execution_context_id: key.execution_context_id,
                unique_context_id: world.unique_id.as_str(),
            })
        })
    }

    /// Enumerates every materialized or pending target-local CDP session.
    /// Pending sessions are included because their Page and Runtime domains
    /// must be enabled before their deferred topology records can complete.
    pub fn target_session_routes(&self) -> impl Iterator<Item = CdpTargetSessionRoute<'_>> {
        self.sessions.iter().filter_map(|(session_id, owner)| {
            let (target_id, page_target_id) = match owner {
                SessionOwner::Page(target_id) | SessionOwner::PendingPage(target_id) => {
                    (target_id.as_str(), target_id.as_str())
                }
                SessionOwner::Frame(frame_id) => {
                    let frame = self.frames.get(frame_id)?;
                    (frame_id.as_str(), frame.page_target_id.as_str())
                }
                SessionOwner::PendingFrame(frame_id) => {
                    let frame = self.pending_frames.get(frame_id)?;
                    (frame_id.as_str(), frame.attachment.page_target_id.as_str())
                }
            };
            Some(CdpTargetSessionRoute {
                target_id,
                page_target_id,
                session_id,
            })
        })
    }

    pub fn target_session_route(&self, session_id: &str) -> Option<CdpTargetSessionRoute<'_>> {
        self.target_session_routes()
            .find(|route| route.session_id == session_id)
    }

    /// Returns the current native main-frame identity for one portable Page.
    pub fn main_frame_native_id(&self, page_object_id: ObjectId) -> Option<&str> {
        let target_id = self.page_by_object.get(&page_object_id)?;
        self.pages
            .get(target_id)
            .map(|page| page.main_frame_native_id.as_str())
    }

    pub fn main_frame_object_id(&self, target_id: &str) -> Option<ObjectId> {
        self.pages
            .get(target_id)
            .map(|record| record.object_ids.main_frame)
    }

    pub fn frame_object_id(&self, frame_id: &str) -> Option<ObjectId> {
        self.frames.get(frame_id).map(|record| record.object_id)
    }

    /// Returns the current native Frame identity for one portable Frame.
    pub fn frame_native_id(&self, frame_object_id: ObjectId) -> Option<&str> {
        self.frame_by_object
            .get(&frame_object_id)
            .map(String::as_str)
    }

    /// Returns the portable parent Frame, or `None` for a main Frame.
    pub fn parent_frame_object_id(&self, frame_object_id: ObjectId) -> Option<ObjectId> {
        let frame_id = self.frame_by_object.get(&frame_object_id)?;
        let parent_frame_id = self.frames.get(frame_id)?.parent_frame_id.as_deref()?;
        self.frames
            .get(parent_frame_id)
            .map(|parent| parent.object_id)
    }

    /// Returns the portable Page that owns one materialized portable Frame.
    ///
    /// This lookup follows the topology's native Frame-to-Page relationship
    /// without copying either native identity. It is useful to bind
    /// renderer-owned resources, such as remote JavaScript handles, to both
    /// their immediate Frame and their crash/disconnect lifetime owner.
    pub fn page_object_id_for_frame(&self, frame_object_id: ObjectId) -> Option<ObjectId> {
        let frame_id = self.frame_by_object.get(&frame_object_id)?;
        let page_target_id = &self.frames.get(frame_id)?.page_target_id;
        self.pages
            .get(page_target_id)
            .map(|page| page.object_ids.page)
    }

    pub fn page_state(&self, target_id: &str) -> Option<&CdpPageState> {
        self.pages.get(target_id).map(|record| &record.state)
    }

    pub fn world_count_for_frame(&self, frame_object_id: ObjectId) -> usize {
        self.worlds
            .values()
            .filter(|world| world.frame_object_id == frame_object_id)
            .count()
    }

    pub fn register_context(
        &mut self,
        epoch: ConnectionEpoch,
        browser_context_id: impl Into<String>,
        object_id: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        self.register_context_inner(
            epoch,
            browser_context_id.into(),
            object_id,
            ownership,
            false,
        )
    }

    /// Registers Chromium's default context without inventing a native CDP
    /// context identifier. At most one default context may exist.
    pub fn register_default_context(
        &mut self,
        epoch: ConnectionEpoch,
        object_id: ObjectId,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        self.register_context_inner(
            epoch,
            String::new(),
            object_id,
            CdpContextOwnership::Default,
            true,
        )
    }

    fn register_context_inner(
        &mut self,
        epoch: ConnectionEpoch,
        browser_context_id: String,
        object_id: ObjectId,
        ownership: CdpContextOwnership,
        allow_empty_default: bool,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        let browser_context_id = compact_string(browser_context_id);
        if !(allow_empty_default
            && ownership == CdpContextOwnership::Default
            && browser_context_id.is_empty())
        {
            self.validate_native_id("browser context ID", &browser_context_id)?;
        }
        if let Some(existing) = self.contexts.get(&browser_context_id) {
            if existing.object_id == object_id && existing.ownership == ownership {
                if existing.seen_epoch == self.epoch {
                    return Ok(CdpTopologyUpdate::idempotent());
                }
                self.contexts
                    .get_mut(&browser_context_id)
                    .expect("context existence was checked")
                    .seen_epoch = self.epoch;
                return Ok(CdpTopologyUpdate::applied(Vec::new()));
            }
            return Err(CdpTopologyError::ContextConflict { browser_context_id });
        }
        if ownership == CdpContextOwnership::Default
            && let Some(existing) = self.default_context_route()
        {
            return Err(CdpTopologyError::DefaultContextConflict {
                existing_object_id: existing.object_id(),
            });
        }
        self.ensure_capacity("browser context", self.contexts.len(), self.limits.contexts)?;
        self.reserve_object(object_id)?;
        if let Err(error) = self.graph.insert(
            object_id,
            Some(self.browser_object_id),
            BROWSER_CONTEXT_OBJECT_KIND.tag,
        ) {
            self.reserved_objects.remove(&object_id);
            return Err(error.into());
        }
        self.context_by_object
            .insert(object_id, browser_context_id.clone());
        self.contexts.insert(
            browser_context_id.clone(),
            ContextRecord {
                object_id,
                ownership,
                seen_epoch: self.epoch,
            },
        );

        let mut effects = vec![CdpTopologyEffect::Lifecycle(ControlMessage::Create {
            object_id,
            parent: Some(self.browser_object_id),
            kind: BROWSER_CONTEXT_OBJECT_KIND.tag,
        })];
        let pending_targets = self
            .pending_pages
            .iter()
            .filter(|(_, page)| page.target.browser_context_id == browser_context_id)
            .map(|(target_id, _)| target_id.clone())
            .collect::<Vec<_>>();
        for target_id in pending_targets {
            let page = self
                .pending_pages
                .remove(&target_id)
                .expect("pending page key came from the same map");
            self.materialize_page(page, &mut effects)?;
        }
        self.materialize_ready_pending_frames(&mut effects)?;
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(effects))
    }

    /// Marks a previously registered BrowserContext as present in the current
    /// authoritative enumeration. Unknown native IDs remain unclaimed.
    pub fn observe_context(
        &mut self,
        epoch: ConnectionEpoch,
        browser_context_id: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        let Some(context) = self.contexts.get_mut(browser_context_id) else {
            return Ok(CdpTopologyUpdate::idempotent());
        };
        if context.seen_epoch == self.epoch {
            return Ok(CdpTopologyUpdate::idempotent());
        }
        context.seen_epoch = self.epoch;
        Ok(CdpTopologyUpdate::applied(Vec::new()))
    }

    pub fn attach_page(
        &mut self,
        epoch: ConnectionEpoch,
        target: CdpPageTarget,
        object_ids: CdpPageObjectIds,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        self.validate_page_target(&target)?;
        let target = compact_page_target(target);

        if self.pages.contains_key(&target.target_id) {
            return self.rebind_page(target);
        }
        if self.pending_pages.contains_key(&target.target_id) {
            return self.rebind_pending_page(target);
        }

        self.ensure_capacity(
            "page",
            self.pages.len() + self.pending_pages.len(),
            self.limits.pages,
        )?;
        self.ensure_capacity(
            "frame",
            self.frames.len() + self.pending_frames.len() + self.pending_pages.len(),
            self.limits.frames,
        )?;
        self.ensure_session_available(&target.session_id, None)?;
        self.reserve_object(object_ids.page)?;
        if let Err(error) = self.reserve_object(object_ids.main_frame) {
            self.reserved_objects.remove(&object_ids.page);
            return Err(error);
        }

        let target_id = target.target_id.clone();
        let session_id = target.session_id.clone();
        let page = PendingPage {
            target,
            object_ids,
            state: CdpPageState::Initializing,
            seen_epoch: self.epoch,
        };
        self.sessions
            .insert(session_id, SessionOwner::PendingPage(target_id.clone()));
        if self.contexts.contains_key(&page.target.browser_context_id) {
            let mut effects = Vec::new();
            self.materialize_page(page, &mut effects)?;
            self.materialize_ready_pending_frames(&mut effects)?;
            self.refresh_metrics();
            return Ok(CdpTopologyUpdate::applied(effects));
        }

        self.pending_pages.insert(target_id, page);
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::new(
            CdpTopologyDisposition::Deferred,
            Vec::new(),
        ))
    }

    pub fn mark_page_ready(
        &mut self,
        epoch: ConnectionEpoch,
        target_id: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        let Some(page) = self.pages.get_mut(target_id) else {
            return Err(CdpTopologyError::UnknownPage {
                target_id: target_id.to_owned(),
            });
        };
        if matches!(page.state, CdpPageState::Crashed { .. }) {
            return Err(CdpTopologyError::PageCrashed {
                target_id: target_id.to_owned(),
            });
        }
        let mut effects = Vec::new();
        if !matches!(page.state, CdpPageState::Ready) {
            page.state = CdpPageState::Ready;
        }
        if !page.ready_reported {
            page.ready_reported = true;
            let context = self
                .contexts
                .get(&page.browser_context_id)
                .expect("materialized page has a context")
                .object_id;
            effects.push(CdpTopologyEffect::PageReady {
                page: page.object_ids.page,
                context,
            });
        }
        self.collect_popup_effects(&mut effects);
        self.refresh_metrics();
        if effects.is_empty() {
            Ok(CdpTopologyUpdate::idempotent())
        } else {
            Ok(CdpTopologyUpdate::applied(effects))
        }
    }

    pub fn attach_frame(
        &mut self,
        epoch: ConnectionEpoch,
        attachment: CdpFrameAttachment,
        object_id: ObjectId,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        self.validate_frame_attachment(&attachment)?;
        let attachment = compact_frame_attachment(attachment);
        if attachment.parent_frame_id.is_none() {
            return self.rebind_main_frame(attachment);
        }
        if self.frames.contains_key(&attachment.frame_id) {
            return self.rebind_frame(attachment);
        }
        if let Some(existing) = self.pending_frames.get(&attachment.frame_id) {
            if existing.attachment == attachment {
                return Ok(CdpTopologyUpdate::idempotent());
            }
            return Err(CdpTopologyError::FrameConflict {
                frame_id: attachment.frame_id,
            });
        }
        if !self.pages.contains_key(&attachment.page_target_id)
            && !self.pending_pages.contains_key(&attachment.page_target_id)
        {
            return Err(CdpTopologyError::UnknownPage {
                target_id: attachment.page_target_id,
            });
        }
        self.ensure_capacity(
            "frame",
            self.frames.len() + self.pending_frames.len() + self.pending_pages.len(),
            self.limits.frames,
        )?;
        if let Some(session_id) = &attachment.session_id {
            self.ensure_session_available(session_id, None)?;
        }
        self.reserve_object(object_id)?;

        if self.frame_parent_is_ready(&attachment) {
            let mut effects = Vec::new();
            self.materialize_frame(
                PendingFrame {
                    attachment,
                    object_id,
                },
                &mut effects,
            )?;
            self.materialize_ready_pending_frames(&mut effects)?;
            self.refresh_metrics();
            Ok(CdpTopologyUpdate::applied(effects))
        } else {
            if let Some(session_id) = &attachment.session_id {
                self.sessions.insert(
                    session_id.clone(),
                    SessionOwner::PendingFrame(attachment.frame_id.clone()),
                );
            }
            self.pending_frames.insert(
                attachment.frame_id.clone(),
                PendingFrame {
                    attachment,
                    object_id,
                },
            );
            self.refresh_metrics();
            Ok(CdpTopologyUpdate::new(
                CdpTopologyDisposition::Deferred,
                Vec::new(),
            ))
        }
    }

    pub fn detach_frame(
        &mut self,
        epoch: ConnectionEpoch,
        frame_id: &str,
        reason: CdpFrameDetachReason,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        if let Some(pending) = self.pending_frames.remove(frame_id) {
            if reason == CdpFrameDetachReason::Remove {
                self.release_reserved_object(pending.object_id);
                self.refresh_metrics();
                return Ok(CdpTopologyUpdate::applied(Vec::new()));
            }
            self.pending_frames.insert(frame_id.to_owned(), pending);
            return Ok(CdpTopologyUpdate::idempotent());
        }
        let Some(frame) = self.frames.get(frame_id) else {
            return Ok(CdpTopologyUpdate::idempotent());
        };
        if frame.main {
            return Ok(CdpTopologyUpdate::idempotent());
        }
        if reason == CdpFrameDetachReason::Swap {
            let frame_object_id = frame.object_id;
            let remote = matches!(frame.host, FrameHost::Remote { .. });
            let mut effects = Vec::new();
            self.remove_frame_descendants(frame_object_id, &mut effects);
            if !remote {
                self.frames
                    .get_mut(frame_id)
                    .expect("frame existence was checked")
                    .host = FrameHost::AwaitingLocalOrDetach;
            }
            self.refresh_metrics();
            return if effects.is_empty() {
                Ok(CdpTopologyUpdate::idempotent())
            } else {
                Ok(CdpTopologyUpdate::applied(effects))
            };
        }

        let object_id = frame.object_id;
        self.remove_frame_subtree(frame_id);
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(vec![
            CdpTopologyEffect::Lifecycle(ControlMessage::Dispose { object_id }),
        ]))
    }

    pub fn detach_target(
        &mut self,
        epoch: ConnectionEpoch,
        session_id: &str,
        target_id: Option<&str>,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        let Some(owner) = self.sessions.get(session_id).cloned() else {
            return Ok(CdpTopologyUpdate::idempotent());
        };
        match owner {
            SessionOwner::Page(expected_target) => {
                self.validate_detach_target(session_id, &expected_target, target_id)?;
                let page = self
                    .remove_page(&expected_target)
                    .expect("session owner page is mapped");
                self.refresh_metrics();
                Ok(CdpTopologyUpdate::applied(vec![
                    CdpTopologyEffect::Lifecycle(ControlMessage::Dispose { object_id: page }),
                ]))
            }
            SessionOwner::PendingPage(expected_target) => {
                self.validate_detach_target(session_id, &expected_target, target_id)?;
                self.remove_pending_page(&expected_target);
                self.refresh_metrics();
                Ok(CdpTopologyUpdate::applied(Vec::new()))
            }
            SessionOwner::Frame(expected_frame) => {
                self.validate_detach_target(session_id, &expected_frame, target_id)?;
                self.sessions.remove(session_id);
                self.remove_worlds_for_session(session_id, None);
                let frame = self
                    .frames
                    .get_mut(&expected_frame)
                    .expect("session owner frame is mapped");
                frame.host = FrameHost::AwaitingLocalOrDetach;
                let page = self
                    .pages
                    .get(&frame.page_target_id)
                    .expect("mapped frame belongs to a mapped page")
                    .object_ids
                    .page;
                let frame_object_id = frame.object_id;
                self.refresh_metrics();
                Ok(CdpTopologyUpdate::applied(vec![
                    CdpTopologyEffect::FrameDetachSynchronizationRequired {
                        page,
                        frame: frame_object_id,
                    },
                ]))
            }
            SessionOwner::PendingFrame(expected_frame) => {
                self.validate_detach_target(session_id, &expected_frame, target_id)?;
                self.sessions.remove(session_id);
                if let Some(frame) = self.pending_frames.remove(&expected_frame) {
                    self.release_reserved_object(frame.object_id);
                }
                self.refresh_metrics();
                Ok(CdpTopologyUpdate::applied(Vec::new()))
            }
        }
    }

    pub fn confirm_frame_session_detached(
        &mut self,
        epoch: ConnectionEpoch,
        frame_id: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        let Some(frame) = self.frames.get(frame_id) else {
            return Ok(CdpTopologyUpdate::idempotent());
        };
        if frame.host != FrameHost::AwaitingLocalOrDetach {
            return Ok(CdpTopologyUpdate::idempotent());
        }
        let object_id = frame.object_id;
        self.remove_frame_subtree(frame_id);
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(vec![
            CdpTopologyEffect::Lifecycle(ControlMessage::Dispose { object_id }),
        ]))
    }

    pub fn crash_target(
        &mut self,
        epoch: ConnectionEpoch,
        target_id: &str,
        status: impl Into<String>,
        error_code: i64,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        let status = compact_string(status.into());
        self.validate_native_id("target crash status", &status)?;
        if let Some(page) = self.pages.get_mut(target_id) {
            if page.state
                == (CdpPageState::Crashed {
                    status: status.clone(),
                    error_code,
                })
            {
                return Ok(CdpTopologyUpdate::idempotent());
            }
            page.state = CdpPageState::Crashed {
                status: status.clone(),
                error_code,
            };
            return Ok(CdpTopologyUpdate::applied(vec![
                CdpTopologyEffect::PageCrashed {
                    page: page.object_ids.page,
                    status,
                    error_code,
                },
            ]));
        }
        if let Some(page) = self.pending_pages.get_mut(target_id) {
            page.state = CdpPageState::Crashed { status, error_code };
            return Ok(CdpTopologyUpdate::new(
                CdpTopologyDisposition::Deferred,
                Vec::new(),
            ));
        }
        Err(CdpTopologyError::UnknownPage {
            target_id: target_id.to_owned(),
        })
    }

    pub fn attach_world(
        &mut self,
        epoch: ConnectionEpoch,
        attachment: CdpWorldAttachment,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        self.validate_native_id("session ID", &attachment.session_id)?;
        self.validate_native_id("execution context unique ID", &attachment.unique_id)?;
        self.validate_native_id("frame ID", &attachment.frame_id)?;
        let attachment = compact_world_attachment(attachment);
        let Some(owner) = self.sessions.get(&attachment.session_id) else {
            return Err(CdpTopologyError::UnknownSession {
                session_id: attachment.session_id,
            });
        };
        let Some(frame) = self.frames.get(&attachment.frame_id) else {
            return Err(CdpTopologyError::UnknownFrame {
                frame_id: attachment.frame_id,
            });
        };
        if !self.session_can_reach_frame(owner, frame) {
            return Err(CdpTopologyError::FrameConflict {
                frame_id: attachment.frame_id,
            });
        }
        let key = WorldKey {
            session_id: attachment.session_id,
            execution_context_id: attachment.execution_context_id,
        };
        if let Some(existing) = self.worlds.get(&key) {
            if existing.frame_object_id == frame.object_id
                && existing.unique_id == attachment.unique_id
                && existing.kind == attachment.kind
            {
                return Ok(CdpTopologyUpdate::idempotent());
            }
            return Err(CdpTopologyError::FrameConflict {
                frame_id: attachment.frame_id,
            });
        }
        let replacement = self
            .worlds
            .iter()
            .find(|(_, world)| {
                world.frame_object_id == frame.object_id && world.kind == attachment.kind
            })
            .map(|(key, _)| key.clone());
        if replacement.is_none() {
            self.ensure_capacity("execution world", self.worlds.len(), self.limits.worlds)?;
        }
        if let Some(replacement) = replacement {
            self.worlds.remove(&replacement);
        }
        let frame_object_id = frame.object_id;
        let kind = attachment.kind;
        self.worlds.insert(
            key,
            WorldRecord {
                frame_object_id,
                unique_id: attachment.unique_id,
                kind,
            },
        );
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(vec![
            CdpTopologyEffect::WorldChanged {
                frame: frame_object_id,
                kind,
            },
        ]))
    }

    pub fn destroy_world(
        &mut self,
        epoch: ConnectionEpoch,
        session_id: &str,
        execution_context_id: i64,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        let key = WorldKey {
            session_id: session_id.to_owned(),
            execution_context_id,
        };
        let Some(world) = self.worlds.remove(&key) else {
            return Ok(CdpTopologyUpdate::idempotent());
        };
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(vec![
            CdpTopologyEffect::WorldRemoved {
                frame: world.frame_object_id,
                kind: world.kind,
            },
        ]))
    }

    pub fn clear_worlds(
        &mut self,
        epoch: ConnectionEpoch,
        session_id: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        let mut effects = Vec::new();
        self.remove_worlds_for_session(session_id, Some(&mut effects));
        self.refresh_metrics();
        if effects.is_empty() {
            Ok(CdpTopologyUpdate::idempotent())
        } else {
            Ok(CdpTopologyUpdate::applied(effects))
        }
    }

    pub fn close_context(
        &mut self,
        epoch: ConnectionEpoch,
        browser_context_id: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if let Some(update) = self.check_epoch(epoch)? {
            return Ok(update);
        }
        let Some(context) = self.contexts.get(browser_context_id).cloned() else {
            return Ok(CdpTopologyUpdate::idempotent());
        };
        let page_targets = self
            .pages
            .iter()
            .filter(|(_, page)| page.browser_context_id == browser_context_id)
            .map(|(target_id, _)| target_id.clone())
            .collect::<Vec<_>>();
        for target_id in page_targets {
            self.remove_page(&target_id);
        }
        let pending_targets = self
            .pending_pages
            .iter()
            .filter(|(_, page)| page.target.browser_context_id == browser_context_id)
            .map(|(target_id, _)| target_id.clone())
            .collect::<Vec<_>>();
        for target_id in pending_targets {
            self.remove_pending_page(&target_id);
        }
        self.graph.dispose_subtree(context.object_id);
        self.reserved_objects.remove(&context.object_id);
        self.context_by_object.remove(&context.object_id);
        self.contexts.remove(browser_context_id);
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(vec![
            CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id: context.object_id,
            }),
        ]))
    }

    pub fn begin_reconnect(
        &mut self,
        new_epoch: ConnectionEpoch,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        if new_epoch <= self.epoch {
            return Err(CdpTopologyError::ReconnectEpochNotAdvanced {
                current: self.epoch,
                requested: new_epoch,
            });
        }
        self.epoch = new_epoch;
        self.sessions.clear();
        self.worlds.clear();
        let pending_pages = self.pending_pages.keys().cloned().collect::<Vec<_>>();
        for target_id in pending_pages {
            self.remove_pending_page(&target_id);
        }
        let pending_frames = self.pending_frames.keys().cloned().collect::<Vec<_>>();
        for frame_id in pending_frames {
            if let Some(frame) = self.pending_frames.remove(&frame_id) {
                self.release_reserved_object(frame.object_id);
            }
        }
        for page in self.pages.values_mut() {
            page.session_id = None;
            page.state = CdpPageState::Reconnecting;
            page.ready_reported = false;
            page.popup_reported = false;
        }
        for frame in self.frames.values_mut() {
            frame.host = if frame.main {
                FrameHost::Main
            } else {
                FrameHost::Unbound
            };
        }
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(vec![
            CdpTopologyEffect::ReconnectStarted { epoch: new_epoch },
        ]))
    }

    pub fn finish_reconnect(&mut self) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        let missing_contexts = self
            .contexts
            .iter()
            .filter(|(_, context)| context.seen_epoch != self.epoch)
            .map(|(browser_context_id, _)| browser_context_id.clone())
            .collect::<Vec<_>>();
        let mut effects = Vec::new();
        for browser_context_id in missing_contexts {
            effects.extend(
                self.close_context(self.epoch, &browser_context_id)?
                    .into_effects(),
            );
        }
        let missing_targets = self
            .pages
            .iter()
            .filter(|(_, page)| page.seen_epoch != self.epoch)
            .map(|(target_id, _)| target_id.clone())
            .collect::<Vec<_>>();
        for target_id in missing_targets {
            if let Some(page) = self.remove_page(&target_id) {
                effects.push(CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                    object_id: page,
                }));
            }
        }
        let unbound_frame_roots = self
            .frames
            .iter()
            .filter(|(_, frame)| {
                !frame.main
                    && frame.host == FrameHost::Unbound
                    && frame.parent_frame_id.as_ref().is_none_or(|parent| {
                        self.frames
                            .get(parent)
                            .is_none_or(|parent| parent.host != FrameHost::Unbound)
                    })
            })
            .map(|(frame_id, frame)| (frame_id.clone(), frame.object_id))
            .collect::<Vec<_>>();
        for (frame_id, object_id) in unbound_frame_roots {
            self.remove_frame_subtree(&frame_id);
            effects.push(CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id,
            }));
        }
        effects.push(CdpTopologyEffect::ReconnectFinished {
            epoch: self.epoch,
            live_pages: self.pages.len(),
        });
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(effects))
    }

    fn rebind_page(
        &mut self,
        target: CdpPageTarget,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        let existing = self
            .pages
            .get(&target.target_id)
            .expect("page existence was checked");
        if existing.browser_context_id != target.browser_context_id
            || existing.opener_target_id != target.opener_target_id
        {
            return Err(CdpTopologyError::PageConflict {
                target_id: target.target_id,
            });
        }
        self.ensure_session_available(
            &target.session_id,
            Some(&SessionOwner::Page(target.target_id.clone())),
        )?;
        let previous_session = existing.session_id.clone();
        let exact = previous_session.as_deref() == Some(target.session_id.as_str())
            && existing.seen_epoch == self.epoch
            && !matches!(existing.state, CdpPageState::Reconnecting);
        if exact {
            return Ok(CdpTopologyUpdate::idempotent());
        }
        if let Some(previous_session) = previous_session {
            self.sessions.remove(&previous_session);
            self.remove_worlds_for_session(&previous_session, None);
        }
        self.sessions.insert(
            target.session_id.clone(),
            SessionOwner::Page(target.target_id.clone()),
        );
        let page = self
            .pages
            .get_mut(&target.target_id)
            .expect("page existence was checked");
        page.session_id = Some(target.session_id);
        page.seen_epoch = self.epoch;
        page.state = CdpPageState::Initializing;
        page.ready_reported = false;
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(Vec::new()))
    }

    fn rebind_pending_page(
        &mut self,
        target: CdpPageTarget,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        let existing = self
            .pending_pages
            .get(&target.target_id)
            .expect("pending page existence was checked");
        if existing.target.browser_context_id != target.browser_context_id
            || existing.target.opener_target_id != target.opener_target_id
        {
            return Err(CdpTopologyError::PageConflict {
                target_id: target.target_id,
            });
        }
        self.ensure_session_available(
            &target.session_id,
            Some(&SessionOwner::PendingPage(target.target_id.clone())),
        )?;
        if existing.target.session_id == target.session_id && existing.seen_epoch == self.epoch {
            return Ok(CdpTopologyUpdate::idempotent());
        }
        self.sessions.remove(&existing.target.session_id);
        self.sessions.insert(
            target.session_id.clone(),
            SessionOwner::PendingPage(target.target_id.clone()),
        );
        let existing = self
            .pending_pages
            .get_mut(&target.target_id)
            .expect("pending page existence was checked");
        existing.target.session_id = target.session_id;
        existing.seen_epoch = self.epoch;
        Ok(CdpTopologyUpdate::applied(Vec::new()))
    }

    fn materialize_page(
        &mut self,
        page: PendingPage,
        effects: &mut Vec<CdpTopologyEffect>,
    ) -> Result<(), CdpTopologyError> {
        let context = self
            .contexts
            .get(&page.target.browser_context_id)
            .ok_or_else(|| CdpTopologyError::UnknownContext {
                browser_context_id: page.target.browser_context_id.clone(),
            })?
            .object_id;
        self.graph
            .insert(page.object_ids.page, Some(context), PAGE_OBJECT_KIND.tag)?;
        self.graph.insert(
            page.object_ids.main_frame,
            Some(page.object_ids.page),
            FRAME_OBJECT_KIND.tag,
        )?;
        let target_id = page.target.target_id.clone();
        let session_id = page.target.session_id.clone();
        self.sessions
            .insert(session_id, SessionOwner::Page(target_id.clone()));
        self.page_by_object
            .insert(page.object_ids.page, target_id.clone());
        self.frame_by_object
            .insert(page.object_ids.main_frame, target_id.clone());
        self.frames.insert(
            target_id.clone(),
            FrameRecord {
                object_id: page.object_ids.main_frame,
                page_target_id: target_id.clone(),
                parent_frame_id: None,
                host: FrameHost::Main,
                main: true,
            },
        );
        effects.push(CdpTopologyEffect::Lifecycle(ControlMessage::Create {
            object_id: page.object_ids.page,
            parent: Some(context),
            kind: PAGE_OBJECT_KIND.tag,
        }));
        effects.push(CdpTopologyEffect::Lifecycle(ControlMessage::Create {
            object_id: page.object_ids.main_frame,
            parent: Some(page.object_ids.page),
            kind: FRAME_OBJECT_KIND.tag,
        }));
        let state = page.state.clone();
        self.pages.insert(
            target_id.clone(),
            PageRecord {
                object_ids: page.object_ids,
                browser_context_id: page.target.browser_context_id,
                session_id: Some(page.target.session_id),
                opener_target_id: page.target.opener_target_id,
                main_frame_native_id: target_id.clone(),
                state,
                seen_epoch: page.seen_epoch,
                ready_reported: false,
                popup_reported: false,
            },
        );
        if let CdpPageState::Crashed { status, error_code } = page.state {
            effects.push(CdpTopologyEffect::PageCrashed {
                page: self
                    .pages
                    .get(&target_id)
                    .expect("page was just inserted")
                    .object_ids
                    .page,
                status,
                error_code,
            });
        }
        Ok(())
    }

    fn rebind_main_frame(
        &mut self,
        attachment: CdpFrameAttachment,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        let Some(page) = self.pages.get(&attachment.page_target_id) else {
            return Err(CdpTopologyError::UnknownPage {
                target_id: attachment.page_target_id,
            });
        };
        let old_frame_id = page.main_frame_native_id.clone();
        if old_frame_id == attachment.frame_id {
            return Ok(CdpTopologyUpdate::idempotent());
        }
        if self.frames.contains_key(&attachment.frame_id) {
            return Err(CdpTopologyError::FrameConflict {
                frame_id: attachment.frame_id,
            });
        }
        let main_frame_object = page.object_ids.main_frame;
        let mut effects = Vec::new();
        self.remove_frame_descendants(main_frame_object, &mut effects);
        self.remove_worlds_for_frame(main_frame_object, &mut effects);
        let mut frame = self
            .frames
            .remove(&old_frame_id)
            .expect("mapped page has a main frame");
        frame.host = FrameHost::Main;
        self.frames.insert(attachment.frame_id.clone(), frame);
        self.frame_by_object
            .insert(main_frame_object, attachment.frame_id.clone());
        self.pages
            .get_mut(&attachment.page_target_id)
            .expect("page existence was checked")
            .main_frame_native_id = attachment.frame_id;
        effects.push(CdpTopologyEffect::MainFrameRebound {
            frame: main_frame_object,
        });
        self.materialize_ready_pending_frames(&mut effects)?;
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(effects))
    }

    fn rebind_frame(
        &mut self,
        attachment: CdpFrameAttachment,
    ) -> Result<CdpTopologyUpdate, CdpTopologyError> {
        let frame = self
            .frames
            .get(&attachment.frame_id)
            .expect("frame existence was checked");
        if frame.page_target_id != attachment.page_target_id
            || frame.parent_frame_id != attachment.parent_frame_id
            || frame.main
        {
            return Err(CdpTopologyError::FrameConflict {
                frame_id: attachment.frame_id,
            });
        }
        let desired_host = attachment
            .session_id
            .as_ref()
            .map_or(FrameHost::Local, |session_id| FrameHost::Remote {
                session_id: session_id.clone(),
            });
        if frame.host == desired_host {
            return Ok(CdpTopologyUpdate::idempotent());
        }
        if let Some(session_id) = &attachment.session_id {
            self.ensure_session_available(
                session_id,
                Some(&SessionOwner::Frame(attachment.frame_id.clone())),
            )?;
        }
        let old_session = match &frame.host {
            FrameHost::Remote { session_id } => Some(session_id.clone()),
            _ => None,
        };
        let frame_object_id = frame.object_id;
        if let Some(old_session) = old_session {
            self.sessions.remove(&old_session);
            self.remove_worlds_for_session(&old_session, None);
        }
        if let Some(session_id) = &attachment.session_id {
            self.sessions.insert(
                session_id.clone(),
                SessionOwner::Frame(attachment.frame_id.clone()),
            );
        }
        let mut effects = Vec::new();
        self.remove_frame_descendants(frame_object_id, &mut effects);
        self.frames
            .get_mut(&attachment.frame_id)
            .expect("frame existence was checked")
            .host = desired_host;
        self.refresh_metrics();
        Ok(CdpTopologyUpdate::applied(effects))
    }

    fn materialize_frame(
        &mut self,
        pending: PendingFrame,
        effects: &mut Vec<CdpTopologyEffect>,
    ) -> Result<(), CdpTopologyError> {
        let parent_id = pending
            .attachment
            .parent_frame_id
            .as_deref()
            .and_then(|parent| self.frames.get(parent))
            .ok_or_else(|| CdpTopologyError::UnknownFrame {
                frame_id: pending
                    .attachment
                    .parent_frame_id
                    .clone()
                    .unwrap_or_default(),
            })?
            .object_id;
        self.graph
            .insert(pending.object_id, Some(parent_id), FRAME_OBJECT_KIND.tag)?;
        let host = pending
            .attachment
            .session_id
            .as_ref()
            .map_or(FrameHost::Local, |session_id| FrameHost::Remote {
                session_id: session_id.clone(),
            });
        if let Some(session_id) = &pending.attachment.session_id {
            self.sessions.insert(
                session_id.clone(),
                SessionOwner::Frame(pending.attachment.frame_id.clone()),
            );
        }
        self.frame_by_object
            .insert(pending.object_id, pending.attachment.frame_id.clone());
        self.frames.insert(
            pending.attachment.frame_id.clone(),
            FrameRecord {
                object_id: pending.object_id,
                page_target_id: pending.attachment.page_target_id,
                parent_frame_id: pending.attachment.parent_frame_id,
                host,
                main: false,
            },
        );
        effects.push(CdpTopologyEffect::Lifecycle(ControlMessage::Create {
            object_id: pending.object_id,
            parent: Some(parent_id),
            kind: FRAME_OBJECT_KIND.tag,
        }));
        Ok(())
    }

    fn materialize_ready_pending_frames(
        &mut self,
        effects: &mut Vec<CdpTopologyEffect>,
    ) -> Result<(), CdpTopologyError> {
        loop {
            let ready = self
                .pending_frames
                .iter()
                .find(|(_, frame)| self.frame_parent_is_ready(&frame.attachment))
                .map(|(frame_id, _)| frame_id.clone());
            let Some(frame_id) = ready else {
                return Ok(());
            };
            let frame = self
                .pending_frames
                .remove(&frame_id)
                .expect("ready frame key came from the same map");
            self.materialize_frame(frame, effects)?;
        }
    }

    fn collect_popup_effects(&mut self, effects: &mut Vec<CdpTopologyEffect>) {
        let ready_pages = self
            .pages
            .iter()
            .filter_map(|(target_id, page)| {
                if !matches!(page.state, CdpPageState::Ready) || page.popup_reported {
                    return None;
                }
                let opener_target = page.opener_target_id.as_ref()?;
                let opener = self.pages.get(opener_target)?;
                if !matches!(opener.state, CdpPageState::Ready) {
                    return None;
                }
                Some((
                    target_id.clone(),
                    opener.object_ids.page,
                    page.object_ids.page,
                ))
            })
            .collect::<Vec<_>>();
        for (target_id, opener, popup) in ready_pages {
            self.pages
                .get_mut(&target_id)
                .expect("popup candidate came from the same map")
                .popup_reported = true;
            effects.push(CdpTopologyEffect::PopupLinked { opener, popup });
        }
    }

    fn remove_frame_descendants(
        &mut self,
        frame_object_id: ObjectId,
        effects: &mut Vec<CdpTopologyEffect>,
    ) {
        let children = self.graph.children_of(frame_object_id).collect::<Vec<_>>();
        for child in children {
            let Some(native_id) = self.frame_by_object.get(&child).cloned() else {
                continue;
            };
            self.remove_frame_subtree(&native_id);
            effects.push(CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id: child,
            }));
        }
    }

    fn remove_frame_subtree(&mut self, frame_id: &str) {
        let Some(root) = self.frames.get(frame_id).map(|frame| frame.object_id) else {
            return;
        };
        let object_ids = self.graph.subtree_ids_child_first(root);
        self.graph.dispose_subtree(root);
        for object_id in object_ids {
            let Some(native_id) = self.frame_by_object.remove(&object_id) else {
                continue;
            };
            if let Some(frame) = self.frames.remove(&native_id) {
                if let FrameHost::Remote { session_id } = frame.host {
                    self.sessions.remove(&session_id);
                    self.remove_worlds_for_session(&session_id, None);
                }
                self.remove_worlds_for_frame(object_id, &mut Vec::new());
                self.release_reserved_object(object_id);
            }
        }
        self.remove_pending_frame_descendants(frame_id);
    }

    fn remove_pending_frame_descendants(&mut self, parent_frame_id: &str) {
        let mut parents = vec![parent_frame_id.to_owned()];
        while let Some(parent) = parents.pop() {
            let children = self
                .pending_frames
                .iter()
                .filter(|(_, frame)| frame.attachment.parent_frame_id.as_deref() == Some(&parent))
                .map(|(frame_id, _)| frame_id.clone())
                .collect::<Vec<_>>();
            for child in children {
                if let Some(frame) = self.pending_frames.remove(&child) {
                    if let Some(session_id) = &frame.attachment.session_id {
                        self.sessions.remove(session_id);
                    }
                    self.release_reserved_object(frame.object_id);
                    parents.push(child);
                }
            }
        }
    }

    fn remove_page(&mut self, target_id: &str) -> Option<ObjectId> {
        let page = self.pages.remove(target_id)?;
        if let Some(session_id) = &page.session_id {
            self.sessions.remove(session_id);
            self.remove_worlds_for_session(session_id, None);
        }
        let object_ids = self.graph.subtree_ids_child_first(page.object_ids.page);
        self.graph.dispose_subtree(page.object_ids.page);
        for object_id in object_ids {
            if let Some(native_id) = self.frame_by_object.remove(&object_id)
                && let Some(frame) = self.frames.remove(&native_id)
                && let FrameHost::Remote { session_id } = frame.host
            {
                self.sessions.remove(&session_id);
                self.remove_worlds_for_session(&session_id, None);
            }
            self.reserved_objects.remove(&object_id);
        }
        self.page_by_object.remove(&page.object_ids.page);
        let pending_frames = self
            .pending_frames
            .iter()
            .filter(|(_, frame)| frame.attachment.page_target_id == target_id)
            .map(|(frame_id, _)| frame_id.clone())
            .collect::<Vec<_>>();
        for frame_id in pending_frames {
            if let Some(frame) = self.pending_frames.remove(&frame_id) {
                self.release_reserved_object(frame.object_id);
                if let Some(session_id) = frame.attachment.session_id {
                    self.sessions.remove(&session_id);
                }
            }
        }
        Some(page.object_ids.page)
    }

    fn remove_pending_page(&mut self, target_id: &str) {
        let Some(page) = self.pending_pages.remove(target_id) else {
            return;
        };
        self.sessions.remove(&page.target.session_id);
        self.release_reserved_object(page.object_ids.page);
        self.release_reserved_object(page.object_ids.main_frame);
        let pending_frames = self
            .pending_frames
            .iter()
            .filter(|(_, frame)| frame.attachment.page_target_id == target_id)
            .map(|(frame_id, _)| frame_id.clone())
            .collect::<Vec<_>>();
        for frame_id in pending_frames {
            if let Some(frame) = self.pending_frames.remove(&frame_id) {
                self.release_reserved_object(frame.object_id);
                if let Some(session_id) = frame.attachment.session_id {
                    self.sessions.remove(&session_id);
                }
            }
        }
    }

    fn remove_worlds_for_session(
        &mut self,
        session_id: &str,
        mut effects: Option<&mut Vec<CdpTopologyEffect>>,
    ) {
        let keys = self
            .worlds
            .keys()
            .filter(|key| key.session_id == session_id)
            .cloned()
            .collect::<Vec<_>>();
        for key in keys {
            if let Some(world) = self.worlds.remove(&key)
                && let Some(effects) = effects.as_deref_mut()
            {
                effects.push(CdpTopologyEffect::WorldRemoved {
                    frame: world.frame_object_id,
                    kind: world.kind,
                });
            }
        }
    }

    fn remove_worlds_for_frame(
        &mut self,
        frame_object_id: ObjectId,
        effects: &mut Vec<CdpTopologyEffect>,
    ) {
        let keys = self
            .worlds
            .iter()
            .filter(|(_, world)| world.frame_object_id == frame_object_id)
            .map(|(key, _)| key.clone())
            .collect::<Vec<_>>();
        for key in keys {
            if let Some(world) = self.worlds.remove(&key) {
                effects.push(CdpTopologyEffect::WorldRemoved {
                    frame: world.frame_object_id,
                    kind: world.kind,
                });
            }
        }
    }

    fn frame_parent_is_ready(&self, attachment: &CdpFrameAttachment) -> bool {
        self.pages.contains_key(&attachment.page_target_id)
            && attachment
                .parent_frame_id
                .as_ref()
                .is_some_and(|parent| self.frames.contains_key(parent))
    }

    fn session_can_reach_frame(&self, owner: &SessionOwner, frame: &FrameRecord) -> bool {
        match owner {
            SessionOwner::Page(target_id) => frame.page_target_id == *target_id,
            SessionOwner::Frame(frame_id) => self
                .frames
                .get(frame_id)
                .is_some_and(|owner_frame| owner_frame.page_target_id == frame.page_target_id),
            SessionOwner::PendingPage(_) | SessionOwner::PendingFrame(_) => false,
        }
    }

    fn check_epoch(
        &self,
        event: ConnectionEpoch,
    ) -> Result<Option<CdpTopologyUpdate>, CdpTopologyError> {
        if event < self.epoch {
            return Ok(Some(CdpTopologyUpdate::stale()));
        }
        if event > self.epoch {
            return Err(CdpTopologyError::FutureEpoch {
                current: self.epoch,
                event,
            });
        }
        Ok(None)
    }

    fn validate_page_target(&self, target: &CdpPageTarget) -> Result<(), CdpTopologyError> {
        self.validate_native_id("target ID", &target.target_id)?;
        self.validate_native_id("session ID", &target.session_id)?;
        if target.browser_context_id.is_empty() {
            if self
                .contexts
                .get("")
                .is_none_or(|context| context.ownership != CdpContextOwnership::Default)
            {
                return Err(CdpTopologyError::EmptyNativeId {
                    field: "browser context ID",
                });
            }
        } else {
            self.validate_native_id("browser context ID", &target.browser_context_id)?;
        }
        if let Some(opener) = &target.opener_target_id {
            self.validate_native_id("opener target ID", opener)?;
        }
        Ok(())
    }

    fn validate_frame_attachment(
        &self,
        attachment: &CdpFrameAttachment,
    ) -> Result<(), CdpTopologyError> {
        self.validate_native_id("page target ID", &attachment.page_target_id)?;
        self.validate_native_id("frame ID", &attachment.frame_id)?;
        if let Some(parent) = &attachment.parent_frame_id {
            self.validate_native_id("parent frame ID", parent)?;
        }
        if let Some(session) = &attachment.session_id {
            self.validate_native_id("session ID", session)?;
        }
        Ok(())
    }

    fn validate_native_id(&self, field: &'static str, value: &str) -> Result<(), CdpTopologyError> {
        if value.is_empty() {
            return Err(CdpTopologyError::EmptyNativeId { field });
        }
        if value.len() > self.limits.native_id_bytes {
            return Err(CdpTopologyError::NativeIdTooLong {
                field,
                actual: value.len(),
                limit: self.limits.native_id_bytes,
            });
        }
        Ok(())
    }

    fn validate_detach_target(
        &self,
        session_id: &str,
        expected_target: &str,
        actual_target: Option<&str>,
    ) -> Result<(), CdpTopologyError> {
        if let Some(actual_target) = actual_target
            && actual_target != expected_target
        {
            return Err(CdpTopologyError::TargetSessionMismatch {
                session_id: session_id.to_owned(),
                expected_target: expected_target.to_owned(),
                actual_target: actual_target.to_owned(),
            });
        }
        Ok(())
    }

    fn ensure_session_available(
        &self,
        session_id: &str,
        expected: Option<&SessionOwner>,
    ) -> Result<(), CdpTopologyError> {
        if let Some(owner) = self.sessions.get(session_id)
            && Some(owner) != expected
        {
            return Err(CdpTopologyError::SessionInUse {
                session_id: session_id.to_owned(),
            });
        }
        Ok(())
    }

    fn ensure_capacity(
        &self,
        resource: &'static str,
        current: usize,
        limit: usize,
    ) -> Result<(), CdpTopologyError> {
        if current >= limit {
            Err(CdpTopologyError::CapacityExceeded { resource, limit })
        } else {
            Ok(())
        }
    }

    fn reserve_object(&mut self, object_id: ObjectId) -> Result<(), CdpTopologyError> {
        self.ensure_capacity("object", self.reserved_objects.len(), self.limits.objects)?;
        if !self.reserved_objects.insert(object_id) {
            return Err(CdpTopologyError::ObjectIdAlreadyReserved { object_id });
        }
        Ok(())
    }

    fn release_reserved_object(&mut self, object_id: ObjectId) {
        self.reserved_objects.remove(&object_id);
    }

    fn refresh_metrics(&mut self) {
        self.metrics.live_objects = self.graph.len();
        self.metrics.live_contexts = self.contexts.len();
        self.metrics.live_pages = self.pages.len();
        self.metrics.live_frames = self.frames.len();
        self.metrics.live_worlds = self.worlds.len();
        self.metrics.pending_pages = self.pending_pages.len();
        self.metrics.pending_frames = self.pending_frames.len();
        self.metrics.peak_objects = self.metrics.peak_objects.max(self.reserved_objects.len());
        self.metrics.peak_contexts = self.metrics.peak_contexts.max(self.contexts.len());
        self.metrics.peak_pages = self
            .metrics
            .peak_pages
            .max(self.pages.len() + self.pending_pages.len());
        self.metrics.peak_frames = self
            .metrics
            .peak_frames
            .max(self.frames.len() + self.pending_frames.len() + self.pending_pages.len());
        self.metrics.peak_worlds = self.metrics.peak_worlds.max(self.worlds.len());
    }
}

/// Resource-gated CDP topology for production compositions.
///
/// The wrapped pure topology remains separately available for deterministic
/// unit tests. This type is the production boundary: every mutation reserves
/// space for a maximum-sized shadow before cloning, then transfers an exact
/// `BackendStateBytes` lease to the committed state.
#[derive(Debug)]
pub struct ProductionCdpTopology {
    topology: CdpTopology,
    ledger: ResourceLedger,
    retained_lease: Option<ResourceLease>,
    profile: CdpTopologyProductionProfile,
    transaction_gate: Arc<AtomicU64>,
    next_transaction_id: u64,
}

/// A resource-accounted topology candidate that rolls back on drop.
///
/// The live topology is not changed until [`Self::commit`] is called. This
/// lets a composition validate and commit its runtime and native-session views
/// before publishing the topology transition.
#[must_use = "dropping a prepared topology transition rolls it back"]
pub struct PreparedCdpTopologyTransition<'a, Output> {
    owner: &'a mut ProductionCdpTopology,
    next: CdpTopology,
    next_lease: Option<ResourceLease>,
    output: Output,
}

/// Owned topology candidate that may cross one caller-owned async backend
/// operation without holding a mutex or mutable borrow.
#[must_use = "dropping a prepared topology transaction rolls it back"]
pub struct PreparedOwnedCdpTopologyTransition<Output> {
    transaction_gate: Arc<AtomicU64>,
    transaction_id: u64,
    next: Option<CdpTopology>,
    next_lease: Option<ResourceLease>,
    output: Option<Output>,
    finished: bool,
}

impl<Output> PreparedOwnedCdpTopologyTransition<Output> {
    pub fn topology(&self) -> &CdpTopology {
        self.next
            .as_ref()
            .expect("unfinished topology transaction retains its candidate")
    }

    pub fn output(&self) -> &Output {
        self.output
            .as_ref()
            .expect("unfinished topology transaction retains its output")
    }

    /// Publishes the candidate into its exact owner. The retained lease stays
    /// at the conservative shadow maximum until the next topology mutation;
    /// this keeps commit infallible after a successful backend operation.
    pub fn commit(
        mut self,
        owner: &mut ProductionCdpTopology,
    ) -> Result<Output, CdpTopologyProductionError> {
        if !Arc::ptr_eq(&self.transaction_gate, &owner.transaction_gate) {
            return Err(CdpTopologyProductionError::TransactionOwnerMismatch);
        }
        if self.transaction_gate.load(Ordering::Acquire) != self.transaction_id {
            return Err(CdpTopologyProductionError::TransactionUnavailable);
        }
        let next = self
            .next
            .take()
            .expect("unfinished topology transaction retains its candidate");
        let output = self
            .output
            .take()
            .expect("unfinished topology transaction retains its output");
        let previous_topology = std::mem::replace(&mut owner.topology, next);
        let previous_lease = std::mem::replace(&mut owner.retained_lease, self.next_lease.take());
        drop(previous_topology);
        drop(previous_lease);
        self.transaction_gate.store(0, Ordering::Release);
        self.finished = true;
        Ok(output)
    }
}

#[derive(Clone, Debug)]
struct CdpBrowserContextCloseCandidate {
    browser_context_id: String,
    ownership: CdpContextOwnership,
    update: CdpTopologyUpdate,
}

#[derive(Clone, Debug)]
struct CdpPageCloseCandidate {
    target_id: String,
    session_id: String,
    update: CdpTopologyUpdate,
}

/// Prepared topology half of an explicit BrowserContext close.
#[must_use = "dropping a prepared browser-context close preserves the topology"]
pub struct PreparedCdpBrowserContextClose {
    transaction: PreparedOwnedCdpTopologyTransition<CdpBrowserContextCloseCandidate>,
}

impl PreparedCdpBrowserContextClose {
    pub fn browser_context_id(&self) -> &str {
        &self.transaction.output().browser_context_id
    }

    pub fn ownership(&self) -> CdpContextOwnership {
        self.transaction.output().ownership
    }

    pub fn update(&self) -> &CdpTopologyUpdate {
        &self.transaction.output().update
    }

    pub fn topology(&self) -> &CdpTopology {
        self.transaction.topology()
    }

    pub fn commit(
        self,
        owner: &mut ProductionCdpTopology,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transaction
            .commit(owner)
            .map(|candidate| candidate.update)
    }
}

/// Prepared topology half of an ordinary Page close.
#[must_use = "dropping a prepared Page close preserves the topology"]
pub struct PreparedCdpPageClose {
    transaction: PreparedOwnedCdpTopologyTransition<CdpPageCloseCandidate>,
}

impl PreparedCdpPageClose {
    pub fn target_id(&self) -> &str {
        &self.transaction.output().target_id
    }

    pub fn session_id(&self) -> &str {
        &self.transaction.output().session_id
    }

    pub fn update(&self) -> &CdpTopologyUpdate {
        &self.transaction.output().update
    }

    pub fn topology(&self) -> &CdpTopology {
        self.transaction.topology()
    }

    pub fn commit(
        self,
        owner: &mut ProductionCdpTopology,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transaction
            .commit(owner)
            .map(|candidate| candidate.update)
    }
}

impl<Output> Drop for PreparedOwnedCdpTopologyTransition<Output> {
    fn drop(&mut self) {
        if self.finished {
            return;
        }
        let _ = self.transaction_gate.compare_exchange(
            self.transaction_id,
            0,
            Ordering::AcqRel,
            Ordering::Acquire,
        );
    }
}

impl<Output> PreparedCdpTopologyTransition<'_, Output> {
    pub const fn topology(&self) -> &CdpTopology {
        &self.next
    }

    pub const fn output(&self) -> &Output {
        &self.output
    }

    pub fn commit(self) -> Output {
        let previous_topology = std::mem::replace(&mut self.owner.topology, self.next);
        let previous_lease = std::mem::replace(&mut self.owner.retained_lease, self.next_lease);
        drop(previous_topology);
        drop(previous_lease);
        self.output
    }
}

impl ProductionCdpTopology {
    pub fn new(
        browser_object_id: ObjectId,
        epoch: ConnectionEpoch,
        profile: CdpTopologyProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> Result<Self, CdpTopologyProductionError> {
        let ledger = resources.claim(profile.requirements())?;
        let topology = CdpTopology::new(browser_object_id, epoch, profile.limits())?;
        Ok(Self {
            topology,
            ledger,
            retained_lease: None,
            profile,
            transaction_gate: Arc::new(AtomicU64::new(0)),
            next_transaction_id: 1,
        })
    }

    pub const fn profile(&self) -> CdpTopologyProductionProfile {
        self.profile
    }

    pub fn topology(&self) -> &CdpTopology {
        &self.topology
    }

    pub fn retained_native_bytes(&self) -> usize {
        self.topology.retained_native_bytes()
    }

    pub fn transaction_active(&self) -> bool {
        self.transaction_gate.load(Ordering::Acquire) != 0
    }

    /// Builds a bounded candidate without publishing it.
    ///
    /// Callers may inspect the candidate and operation output, then either
    /// commit it or obtain rollback by simply dropping the returned value.
    pub fn prepare_transition<Output>(
        &mut self,
        apply: impl FnOnce(&mut CdpTopology) -> Result<Output, CdpTopologyError>,
    ) -> Result<PreparedCdpTopologyTransition<'_, Output>, CdpTopologyProductionError> {
        self.ensure_no_transaction()?;
        let maximum = self.profile.maximum_retained_native_bytes();
        let mut next_lease = if maximum == 0 {
            None
        } else {
            Some(self.ledger.reserve(
                self.profile.resource_scope(),
                ResourceClass::BackendStateBytes,
                maximum,
            )?)
        };
        let mut next = self.topology.clone();
        let output = apply(&mut next)?;
        let actual = u64::try_from(next.retained_native_bytes())
            .map_err(|_| CdpTopologyProductionError::CapacityOverflow)?;
        if actual > maximum {
            return Err(CdpTopologyProductionError::RetainedBytesInvariant { actual, maximum });
        }
        if let Some(lease) = &mut next_lease {
            lease.try_resize(ResourceClass::BackendStateBytes, actual)?;
        }
        if actual == 0 {
            next_lease = None;
        }
        Ok(PreparedCdpTopologyTransition {
            owner: self,
            next,
            next_lease,
            output,
        })
    }

    /// Builds an owned, resource-reserved candidate that can cross one async
    /// backend operation. All other topology mutations fail until commit or
    /// rollback-on-drop.
    pub fn prepare_owned_transition<Output>(
        &mut self,
        apply: impl FnOnce(&mut CdpTopology) -> Result<Output, CdpTopologyError>,
    ) -> Result<PreparedOwnedCdpTopologyTransition<Output>, CdpTopologyProductionError> {
        self.ensure_no_transaction()?;
        let maximum = self.profile.maximum_retained_native_bytes();
        let next_lease = if maximum == 0 {
            None
        } else {
            Some(self.ledger.reserve(
                self.profile.resource_scope(),
                ResourceClass::BackendStateBytes,
                maximum,
            )?)
        };
        let mut next = self.topology.clone();
        let output = apply(&mut next)?;
        let actual = u64::try_from(next.retained_native_bytes())
            .map_err(|_| CdpTopologyProductionError::CapacityOverflow)?;
        if actual > maximum {
            return Err(CdpTopologyProductionError::RetainedBytesInvariant { actual, maximum });
        }
        let transaction_id = self.next_transaction_id;
        self.next_transaction_id = self
            .next_transaction_id
            .checked_add(1)
            .ok_or(CdpTopologyProductionError::TransactionIdExhausted)?;
        self.transaction_gate
            .compare_exchange(0, transaction_id, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| CdpTopologyProductionError::TransactionInProgress)?;
        Ok(PreparedOwnedCdpTopologyTransition {
            transaction_gate: Arc::clone(&self.transaction_gate),
            transaction_id,
            next: Some(next),
            next_lease,
            output: Some(output),
            finished: false,
        })
    }

    /// Prepares removal of one portable BrowserContext and returns its exact
    /// native route for a caller-owned asynchronous CDP close.
    pub fn prepare_browser_context_close(
        &mut self,
        object_id: ObjectId,
    ) -> Result<PreparedCdpBrowserContextClose, CdpTopologyProductionError> {
        let epoch = self.topology.epoch();
        let transaction = self.prepare_owned_transition(move |topology| {
            let route = topology
                .browser_context_route(object_id)
                .ok_or(CdpTopologyError::UnknownContextObject { object_id })?;
            let browser_context_id = route.browser_context_id().to_owned();
            let ownership = route.ownership();
            let update = topology.close_context(epoch, &browser_context_id)?;
            Ok(CdpBrowserContextCloseCandidate {
                browser_context_id,
                ownership,
                update,
            })
        })?;
        Ok(PreparedCdpBrowserContextClose { transaction })
    }

    /// Prepares removal of one materialized portable Page and returns its
    /// exact target/session route for caller-owned asynchronous close work.
    pub fn prepare_page_close(
        &mut self,
        object_id: ObjectId,
    ) -> Result<PreparedCdpPageClose, CdpTopologyProductionError> {
        let epoch = self.topology.epoch();
        let transaction = self.prepare_owned_transition(move |topology| {
            let route = topology
                .page_session_routes()
                .find(|route| route.object_id() == object_id)
                .ok_or(CdpTopologyError::UnknownPageObject { object_id })?;
            let target_id = route.target_id().to_owned();
            let session_id = route.session_id().to_owned();
            let update = topology.detach_target(epoch, &session_id, Some(&target_id))?;
            Ok(CdpPageCloseCandidate {
                target_id,
                session_id,
                update,
            })
        })?;
        Ok(PreparedCdpPageClose { transaction })
    }

    fn ensure_no_transaction(&self) -> Result<(), CdpTopologyProductionError> {
        if self.transaction_active() {
            Err(CdpTopologyProductionError::TransactionInProgress)
        } else {
            Ok(())
        }
    }

    pub fn register_context(
        &mut self,
        epoch: ConnectionEpoch,
        browser_context_id: impl Into<String>,
        object_id: ObjectId,
        ownership: CdpContextOwnership,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        let browser_context_id = browser_context_id.into();
        self.transition(move |topology| {
            topology.register_context(epoch, browser_context_id, object_id, ownership)
        })
    }

    pub fn register_default_context(
        &mut self,
        epoch: ConnectionEpoch,
        object_id: ObjectId,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(move |topology| topology.register_default_context(epoch, object_id))
    }

    pub fn observe_context(
        &mut self,
        epoch: ConnectionEpoch,
        browser_context_id: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(|topology| topology.observe_context(epoch, browser_context_id))
    }

    pub fn attach_page(
        &mut self,
        epoch: ConnectionEpoch,
        target: CdpPageTarget,
        object_ids: CdpPageObjectIds,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(move |topology| topology.attach_page(epoch, target, object_ids))
    }

    pub fn mark_page_ready(
        &mut self,
        epoch: ConnectionEpoch,
        target_id: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(|topology| topology.mark_page_ready(epoch, target_id))
    }

    pub fn attach_frame(
        &mut self,
        epoch: ConnectionEpoch,
        attachment: CdpFrameAttachment,
        object_id: ObjectId,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(move |topology| topology.attach_frame(epoch, attachment, object_id))
    }

    pub fn detach_frame(
        &mut self,
        epoch: ConnectionEpoch,
        frame_id: &str,
        reason: CdpFrameDetachReason,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(|topology| topology.detach_frame(epoch, frame_id, reason))
    }

    pub fn detach_target(
        &mut self,
        epoch: ConnectionEpoch,
        session_id: &str,
        target_id: Option<&str>,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(|topology| topology.detach_target(epoch, session_id, target_id))
    }

    pub fn confirm_frame_session_detached(
        &mut self,
        epoch: ConnectionEpoch,
        frame_id: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(|topology| topology.confirm_frame_session_detached(epoch, frame_id))
    }

    pub fn crash_target(
        &mut self,
        epoch: ConnectionEpoch,
        target_id: &str,
        status: impl Into<String>,
        error_code: i64,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        let status = status.into();
        self.transition(move |topology| topology.crash_target(epoch, target_id, status, error_code))
    }

    pub fn attach_world(
        &mut self,
        epoch: ConnectionEpoch,
        attachment: CdpWorldAttachment,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(move |topology| topology.attach_world(epoch, attachment))
    }

    pub fn destroy_world(
        &mut self,
        epoch: ConnectionEpoch,
        session_id: &str,
        execution_context_id: i64,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(|topology| topology.destroy_world(epoch, session_id, execution_context_id))
    }

    pub fn clear_worlds(
        &mut self,
        epoch: ConnectionEpoch,
        session_id: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(|topology| topology.clear_worlds(epoch, session_id))
    }

    pub fn close_context(
        &mut self,
        epoch: ConnectionEpoch,
        browser_context_id: &str,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(|topology| topology.close_context(epoch, browser_context_id))
    }

    pub fn begin_reconnect(
        &mut self,
        new_epoch: ConnectionEpoch,
    ) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(|topology| topology.begin_reconnect(new_epoch))
    }

    pub fn finish_reconnect(&mut self) -> Result<CdpTopologyUpdate, CdpTopologyProductionError> {
        self.transition(CdpTopology::finish_reconnect)
    }

    fn transition<T>(
        &mut self,
        apply: impl FnOnce(&mut CdpTopology) -> Result<T, CdpTopologyError>,
    ) -> Result<T, CdpTopologyProductionError> {
        self.prepare_transition(apply)
            .map(PreparedCdpTopologyTransition::commit)
    }
}

impl std::ops::Deref for ProductionCdpTopology {
    type Target = CdpTopology;

    fn deref(&self) -> &Self::Target {
        &self.topology
    }
}

#[cfg(test)]
mod production_tests {
    use pwright_core::{
        ControlMessage, ProductionResourceLedger, ResourceClass, ResourceError, ResourceLimits,
        ResourceRequirements, ResourceScope,
    };

    use super::{
        CdpContextOwnership, CdpFrameAttachment, CdpPageObjectIds, CdpPageTarget,
        CdpTopologyEffect, CdpTopologyLimits, CdpTopologyProductionError,
        CdpTopologyProductionProfile, CdpWorldAttachment, CdpWorldKind, ConnectionEpoch,
        ProductionCdpTopology,
    };
    use pwright_core::ObjectId;

    const EPOCH_ONE: ConnectionEpoch = ConnectionEpoch::new(1);
    const EPOCH_TWO: ConnectionEpoch = ConnectionEpoch::new(2);
    const TOPOLOGY_SCOPE: ResourceScope = ResourceScope::extension(7, 1);

    fn limits() -> CdpTopologyLimits {
        CdpTopologyLimits::new(32, 4, 4, 8, 8, 32)
    }

    fn resources(
        profile: CdpTopologyProductionProfile,
        extra_leases: usize,
    ) -> ProductionResourceLedger {
        let requirement = profile
            .requirements()
            .class(ResourceClass::BackendStateBytes);
        let limits = ResourceLimits::deny_all(2, 2 + extra_leases).with_class(
            ResourceClass::BackendStateBytes,
            requirement.global,
            requirement.per_scope,
        );
        ProductionResourceLedger::new(limits, profile.requirements())
            .expect("production topology requirements should fit")
    }

    fn production_topology(
        profile: CdpTopologyProductionProfile,
        resources: &ProductionResourceLedger,
    ) -> ProductionCdpTopology {
        ProductionCdpTopology::new(ObjectId::new(1), EPOCH_ONE, profile, resources)
            .expect("production topology should construct")
    }

    fn assert_exact_accounting(
        topology: &ProductionCdpTopology,
        resources: &ProductionResourceLedger,
    ) {
        let expected = u64::try_from(topology.retained_native_bytes())
            .expect("test retained bytes should fit u64");
        let actual = resources
            .metrics()
            .expect("resource metrics should remain readable")
            .class(ResourceClass::BackendStateBytes)
            .current;
        assert_eq!(actual, expected);
    }

    #[test]
    fn production_profile_covers_live_and_shadow_native_string_storage() {
        // Arrange
        let limits = CdpTopologyLimits::new(64, 1, 1, 1, 1, 10);

        // Act
        let profile = CdpTopologyProductionProfile::new(limits, TOPOLOGY_SCOPE)
            .expect("small topology requirements should fit");
        let requirement = profile
            .requirements()
            .class(ResourceClass::BackendStateBytes);

        // Assert
        assert_eq!(profile.maximum_retained_native_bytes(), 200);
        assert_eq!(requirement.global, 400);
        assert_eq!(requirement.per_scope, 400);
        assert_eq!(profile.requirements().scopes, 1);
        assert_eq!(profile.requirements().leases, 2);
        assert_eq!(profile.resource_scope(), TOPOLOGY_SCOPE);
        assert!(matches!(
            CdpTopologyProductionProfile::new(limits, ResourceScope::Connection),
            Err(CdpTopologyProductionError::InvalidResourceScope {
                resource_scope: ResourceScope::Connection
            })
        ));
    }

    #[test]
    fn production_topology_compacts_caller_spare_capacity_and_accounts_exact_bytes() {
        // Arrange
        let profile = CdpTopologyProductionProfile::new(limits(), TOPOLOGY_SCOPE)
            .expect("profile should fit");
        let resources = resources(profile, 0);
        let mut topology = production_topology(profile, &resources);
        let mut context_id = String::with_capacity(4096);
        context_id.push_str("ctx");
        assert!(context_id.capacity() > context_id.len());

        // Act
        topology
            .register_context(
                EPOCH_ONE,
                context_id,
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should register");

        // Assert
        assert_eq!(topology.retained_native_bytes(), 6);
        assert_exact_accounting(&topology, &resources);
    }

    #[test]
    fn prepared_topology_transition_publishes_only_when_committed() {
        // Arrange
        let profile = CdpTopologyProductionProfile::new(limits(), TOPOLOGY_SCOPE)
            .expect("profile should fit");
        let resources = resources(profile, 0);
        let mut topology = production_topology(profile, &resources);

        // Act: stage and drop one valid mutation.
        let prepared = topology
            .prepare_transition(|candidate| {
                candidate.register_context(
                    EPOCH_ONE,
                    "rolled-back",
                    ObjectId::new(2),
                    CdpContextOwnership::Owned,
                )
            })
            .expect("candidate should prepare");
        assert_eq!(
            prepared.topology().context_object_id("rolled-back"),
            Some(ObjectId::new(2))
        );
        drop(prepared);

        // Assert: drop restored both state visibility and retained capacity.
        assert_eq!(topology.context_object_id("rolled-back"), None);
        assert_eq!(
            resources
                .metrics()
                .expect("resource metrics should remain readable")
                .class(ResourceClass::BackendStateBytes)
                .current,
            0
        );

        // Act / Assert: committing publishes the same candidate and its lease.
        topology
            .prepare_transition(|candidate| {
                candidate.register_context(
                    EPOCH_ONE,
                    "committed",
                    ObjectId::new(2),
                    CdpContextOwnership::Owned,
                )
            })
            .expect("candidate should prepare")
            .commit();
        assert_eq!(
            topology.context_object_id("committed"),
            Some(ObjectId::new(2))
        );
        assert_exact_accounting(&topology, &resources);
    }

    #[test]
    fn owned_context_close_transaction_excludes_mutation_and_rolls_back_on_drop() {
        // Arrange
        let profile = CdpTopologyProductionProfile::new(limits(), TOPOLOGY_SCOPE)
            .expect("profile should fit");
        let resources = resources(profile, 0);
        let mut topology = production_topology(profile, &resources);
        topology
            .register_context(
                EPOCH_ONE,
                "context-a",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should register");

        // Act: inspect and then drop the async-spanning candidate.
        let prepared = topology
            .prepare_browser_context_close(ObjectId::new(2))
            .expect("context close should prepare");
        assert_eq!(prepared.browser_context_id(), "context-a");
        assert_eq!(prepared.ownership(), CdpContextOwnership::Owned);
        assert_eq!(prepared.topology().context_object_id("context-a"), None);
        assert!(topology.transaction_active());
        let competing = topology.register_context(
            EPOCH_ONE,
            "context-b",
            ObjectId::new(3),
            CdpContextOwnership::Owned,
        );
        drop(prepared);

        // Assert: competing mutation failed and rollback preserved live state.
        assert!(matches!(
            competing,
            Err(CdpTopologyProductionError::TransactionInProgress)
        ));
        assert!(!topology.transaction_active());
        assert_eq!(
            topology.context_object_id("context-a"),
            Some(ObjectId::new(2))
        );
        assert_exact_accounting(&topology, &resources);
    }

    #[test]
    fn owned_context_close_commit_publishes_exact_candidate() {
        // Arrange
        let profile = CdpTopologyProductionProfile::new(limits(), TOPOLOGY_SCOPE)
            .expect("profile should fit");
        let resources = resources(profile, 0);
        let mut topology = production_topology(profile, &resources);
        topology
            .register_context(
                EPOCH_ONE,
                "context-a",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should register");
        let prepared = topology
            .prepare_browser_context_close(ObjectId::new(2))
            .expect("context close should prepare");

        // Act
        let update = prepared
            .commit(&mut topology)
            .expect("prepared close should commit");

        // Assert
        assert!(!topology.transaction_active());
        assert_eq!(topology.context_object_id("context-a"), None);
        assert_eq!(
            update.effects(),
            &[CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id: ObjectId::new(2),
            })]
        );

        // A subsequent transition replaces the conservative shadow lease with
        // exact retained-state accounting.
        topology
            .close_context(EPOCH_ONE, "already-closed")
            .expect("idempotent close should reconcile accounting");
        assert_exact_accounting(&topology, &resources);
    }

    #[test]
    fn owned_page_close_transaction_rolls_back_and_commits_the_exact_route() {
        // Arrange
        let profile = CdpTopologyProductionProfile::new(limits(), TOPOLOGY_SCOPE)
            .expect("profile should fit");
        let resources = resources(profile, 0);
        let mut topology = production_topology(profile, &resources);
        topology
            .register_context(
                EPOCH_ONE,
                "context-a",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .unwrap();
        topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("target-a", "session-a", "context-a"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .unwrap();

        // Act: first prove rollback, then prepare and publish the same route.
        let prepared = topology.prepare_page_close(ObjectId::new(3)).unwrap();
        assert_eq!(prepared.target_id(), "target-a");
        assert_eq!(prepared.session_id(), "session-a");
        assert_eq!(prepared.topology().page_object_id("target-a"), None);
        drop(prepared);
        assert_eq!(topology.page_object_id("target-a"), Some(ObjectId::new(3)));
        let update = topology
            .prepare_page_close(ObjectId::new(3))
            .unwrap()
            .commit(&mut topology)
            .unwrap();

        // Assert
        assert!(!topology.transaction_active());
        assert_eq!(topology.page_object_id("target-a"), None);
        assert_eq!(
            update.effects(),
            &[CdpTopologyEffect::Lifecycle(ControlMessage::Dispose {
                object_id: ObjectId::new(3),
            })]
        );
    }

    #[test]
    fn deferred_materialized_and_closed_topology_states_resize_one_exact_lease() {
        // Arrange
        let profile = CdpTopologyProductionProfile::new(limits(), TOPOLOGY_SCOPE)
            .expect("profile should fit");
        let resources = resources(profile, 0);
        let mut topology = production_topology(profile, &resources);

        // Act / Assert: the unknown context defers the page and retains its IDs.
        topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("target", "session", "context"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("page should defer");
        assert!(topology.retained_native_bytes() > 0);
        assert_exact_accounting(&topology, &resources);

        // Act / Assert: registering the context materializes the page and main frame.
        topology
            .register_context(
                EPOCH_ONE,
                "context",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should materialize pending page");
        assert_eq!(topology.metrics().pending_pages, 0);
        assert_eq!(topology.metrics().live_pages, 1);
        assert_exact_accounting(&topology, &resources);

        // Act / Assert: context disposal releases every native string lease.
        topology
            .close_context(EPOCH_ONE, "context")
            .expect("context should close");
        assert_eq!(topology.retained_native_bytes(), 0);
        assert_exact_accounting(&topology, &resources);
        assert_eq!(resources.metrics().expect("metrics").active_leases, 0);
    }

    #[test]
    fn reconnect_and_detach_paths_release_sessions_worlds_and_frames_exactly() {
        // Arrange
        let profile = CdpTopologyProductionProfile::new(limits(), TOPOLOGY_SCOPE)
            .expect("profile should fit");
        let resources = resources(profile, 0);
        let mut topology = production_topology(profile, &resources);
        topology
            .register_context(
                EPOCH_ONE,
                "context",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should register");
        topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("target", "session", "context"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("page should attach");
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::remote("target", "child", "target", "child-session"),
                ObjectId::new(5),
            )
            .expect("remote frame should attach");
        topology
            .detach_target(EPOCH_ONE, "child-session", Some("child"))
            .expect("remote session should detach");
        topology
            .confirm_frame_session_detached(EPOCH_ONE, "child")
            .expect("detached frame should be removed after synchronization");
        assert_eq!(topology.frame_object_id("child"), None);
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::remote("target", "child", "target", "child-session-2"),
                ObjectId::new(5),
            )
            .expect("released frame object should be reusable");
        let before_reconnect = topology.retained_native_bytes();

        // Act
        topology
            .begin_reconnect(EPOCH_TWO)
            .expect("reconnect should begin");
        let during_reconnect = topology.retained_native_bytes();
        topology
            .finish_reconnect()
            .expect("unseen context and Page should be removed");

        // Assert
        assert!(during_reconnect < before_reconnect);
        assert_eq!(topology.metrics().live_pages, 0);
        assert_eq!(topology.metrics().live_frames, 0);
        assert_eq!(topology.retained_native_bytes(), 0);
        assert_exact_accounting(&topology, &resources);
    }

    #[test]
    fn crash_opener_remote_frame_and_world_strings_are_all_in_the_exact_inventory() {
        // Arrange
        let profile = CdpTopologyProductionProfile::new(limits(), TOPOLOGY_SCOPE)
            .expect("profile should fit");
        let resources = resources(profile, 0);
        let mut topology = production_topology(profile, &resources);
        topology
            .register_context(
                EPOCH_ONE,
                "context",
                ObjectId::new(2),
                CdpContextOwnership::Owned,
            )
            .expect("context should register");
        topology
            .attach_page(
                EPOCH_ONE,
                CdpPageTarget::new("target", "session", "context").with_opener("opener"),
                CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
            )
            .expect("page should attach");
        topology
            .attach_frame(
                EPOCH_ONE,
                CdpFrameAttachment::remote("target", "child", "target", "child-session"),
                ObjectId::new(5),
            )
            .expect("remote frame should attach");

        // Act
        topology
            .crash_target(EPOCH_ONE, "target", "crashed", -1)
            .expect("crash state should be retained");
        topology
            .attach_world(
                EPOCH_ONE,
                CdpWorldAttachment::new("session", 7, "unique", "child", CdpWorldKind::Main),
            )
            .expect("page session should reach its child frame");

        // Assert
        assert_eq!(topology.retained_native_bytes(), 156);
        assert_exact_accounting(&topology, &resources);
        topology
            .clear_worlds(EPOCH_ONE, "session")
            .expect("world should clear");
        assert_eq!(topology.retained_native_bytes(), 143);
        assert_exact_accounting(&topology, &resources);
    }

    #[test]
    fn shadow_reservation_failure_leaves_live_topology_and_lease_unchanged() {
        // Arrange
        let profile = CdpTopologyProductionProfile::new(limits(), TOPOLOGY_SCOPE)
            .expect("profile should fit");
        let resources = resources(profile, 1);
        let shared = resources
            .claim(profile.requirements())
            .expect("declared component should obtain shared ledger");
        let mut topology = production_topology(profile, &resources);
        topology
            .register_context(EPOCH_ONE, "c", ObjectId::new(2), CdpContextOwnership::Owned)
            .expect("initial context should register");
        let before_bytes = topology.retained_native_bytes();
        let maximum = profile.maximum_retained_native_bytes();
        let _contention = shared
            .reserve(
                ResourceScope::Connection,
                ResourceClass::BackendStateBytes,
                maximum - 1,
            )
            .expect("test contention should fit before the shadow reservation");

        // Act
        let error = topology
            .register_context(EPOCH_ONE, "d", ObjectId::new(3), CdpContextOwnership::Owned)
            .expect_err("full shadow must be admitted before mutation");

        // Assert
        assert!(matches!(
            error,
            CdpTopologyProductionError::Resource(ResourceError::GlobalLimitExceeded {
                class: ResourceClass::BackendStateBytes,
                ..
            })
        ));
        assert_eq!(topology.context_object_id("d"), None);
        assert_eq!(topology.retained_native_bytes(), before_bytes);
    }

    #[test]
    fn undeclared_topology_cannot_obtain_a_production_ledger() {
        // Arrange
        let profile = CdpTopologyProductionProfile::new(limits(), TOPOLOGY_SCOPE)
            .expect("profile should fit");
        let requirement = profile
            .requirements()
            .class(ResourceClass::BackendStateBytes);
        let limits = ResourceLimits::deny_all(1, 2).with_class(
            ResourceClass::BackendStateBytes,
            requirement.global,
            requirement.per_scope,
        );
        let resources = ProductionResourceLedger::new(limits, ResourceRequirements::none())
            .expect("empty production plan should be valid");

        // Act
        let error = ProductionCdpTopology::new(ObjectId::new(1), EPOCH_ONE, profile, &resources)
            .expect_err("undeclared topology must fail closed");

        // Assert
        assert!(matches!(
            error,
            CdpTopologyProductionError::ProductionResource(_)
        ));
    }
}
