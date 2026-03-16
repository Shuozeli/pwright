//! CDP `Target` domain — generated from protocol JSON.
//!
//! Supports additional targets discovery and allows to attach to them.

#![allow(clippy::doc_markdown)]

use serde::{Deserialize, Serialize};

pub type TargetID = String;

/// Unique identifier of attached debugging session.
pub type SessionID = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfo {
    #[serde(alias = "id")]
    pub target_id: TargetID,
    /// List of types: <https://source.chromium.org/chromium/chromium/src/+/main:content/browser/devtools/devtools_agent_host_impl.cc?ss=chromium&q=f:devtools%20-f:out%20%22::kTypeTab%5B%5D%22>
    #[serde(rename = "type")]
    pub r#type: String,
    pub title: String,
    pub url: String,
    /// Whether the target has an attached client.
    pub attached: bool,
    /// Opener target Id
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opener_id: Option<TargetID>,
    /// Whether the target has access to the originating window.
    pub can_access_opener: bool,
    /// Frame id of originating window (is only set if target has an opener).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opener_frame_id: Option<super::page::FrameId>,
    /// Id of the parent frame, only present for the "iframe" targets.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_frame_id: Option<super::page::FrameId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<super::browser::BrowserContextID>,
    /// Provides additional details for specific target types. For example, for
    /// the type of "page", this may be set to "prerender".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subtype: Option<String>,
}

/// A filter used by target query/discovery/auto-attach operations.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FilterEntry {
    /// If set, causes exclusion of matching targets from the list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exclude: Option<bool>,
    /// If not present, matches any type.
    #[serde(rename = "type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

/// The entries in TargetFilter are matched sequentially against targets and
/// the first entry that matches determines if the target is included or not,
/// depending on the value of `exclude` field in the entry.
/// If filter is not specified, the one assumed is
/// \[{type: "browser", exclude: true}, {type: "tab", exclude: true}, {}\]
/// (i.e. include everything but `browser` and `tab`).
pub type TargetFilter = Vec<FilterEntry>;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteLocation {
    pub host: String,
    pub port: i64,
}

/// The state of the target window.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum WindowState {
    #[serde(rename = "normal")]
    Normal,
    #[serde(rename = "minimized")]
    Minimized,
    #[serde(rename = "maximized")]
    Maximized,
    #[serde(rename = "fullscreen")]
    Fullscreen,
}

/// Parameters for `Target.activateTarget`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivateTargetParams {
    pub target_id: TargetID,
}

/// Parameters for `Target.attachToTarget`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetParams {
    pub target_id: TargetID,
    /// Enables "flat" access to the session via specifying sessionId attribute in the commands.
    /// We plan to make this the default, deprecate non-flattened mode,
    /// and eventually retire it. See crbug.com/991325.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatten: Option<bool>,
}

/// Return type for `Target.attachToTarget`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToTargetReturns {
    /// Id assigned to the session.
    pub session_id: SessionID,
}

/// Return type for `Target.attachToBrowserTarget`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachToBrowserTargetReturns {
    /// Id assigned to the session.
    pub session_id: SessionID,
}

/// Parameters for `Target.closeTarget`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseTargetParams {
    pub target_id: TargetID,
}

/// Return type for `Target.closeTarget`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseTargetReturns {
    /// Always set to true. If an error occurs, the response indicates protocol error.
    pub success: bool,
}

/// Parameters for `Target.exposeDevToolsProtocol`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExposeDevToolsProtocolParams {
    pub target_id: TargetID,
    /// Binding name, 'cdp' if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_name: Option<String>,
    /// If true, inherits the current root session's permissions (default: false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherit_permissions: Option<bool>,
}

/// Parameters for `Target.createBrowserContext`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateBrowserContextParams {
    /// If specified, disposes this context when debugging session disconnects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dispose_on_detach: Option<bool>,
    /// Proxy server, similar to the one passed to --proxy-server
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_server: Option<String>,
    /// Proxy bypass list, similar to the one passed to --proxy-bypass-list
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_bypass_list: Option<String>,
    /// An optional list of origins to grant unlimited cross-origin access to.
    /// Parts of the URL other than those constituting origin are ignored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origins_with_universal_network_access: Option<Vec<String>>,
}

/// Return type for `Target.createBrowserContext`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBrowserContextReturns {
    /// The id of the context created.
    pub browser_context_id: super::browser::BrowserContextID,
}

/// Return type for `Target.getBrowserContexts`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBrowserContextsReturns {
    /// An array of browser context ids.
    pub browser_context_ids: Vec<super::browser::BrowserContextID>,
}

/// Parameters for `Target.createTarget`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetParams {
    /// The initial URL the page will be navigated to. An empty string indicates about:blank.
    pub url: String,
    /// Frame left origin in DIP (requires newWindow to be true or headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left: Option<i64>,
    /// Frame top origin in DIP (requires newWindow to be true or headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top: Option<i64>,
    /// Frame width in DIP (requires newWindow to be true or headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    /// Frame height in DIP (requires newWindow to be true or headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
    /// Frame window state (requires newWindow to be true or headless shell).
    /// Default is normal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_state: Option<WindowState>,
    /// The browser context to create the page in.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<super::browser::BrowserContextID>,
    /// Whether BeginFrames for this target will be controlled via DevTools (headless shell only,
    /// not supported on MacOS yet, false by default).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_begin_frame_control: Option<bool>,
    /// Whether to create a new Window or Tab (false by default, not supported by headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_window: Option<bool>,
    /// Whether to create the target in background or foreground (false by default, not supported
    /// by headless shell).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<bool>,
    /// Whether to create the target of type "tab".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_tab: Option<bool>,
    /// Whether to create a hidden target. The hidden target is observable via protocol, but not
    /// present in the tab UI strip. Cannot be created with `forTab: true`, `newWindow: true` or
    /// `background: false`. The life-time of the tab is limited to the life-time of the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hidden: Option<bool>,
}

/// Return type for `Target.createTarget`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTargetReturns {
    /// The id of the page opened.
    pub target_id: TargetID,
}

/// Parameters for `Target.detachFromTarget`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DetachFromTargetParams {
    /// Session to detach.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionID>,
    /// Deprecated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<TargetID>,
}

/// Parameters for `Target.disposeBrowserContext`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisposeBrowserContextParams {
    pub browser_context_id: super::browser::BrowserContextID,
}

/// Parameters for `Target.getTargetInfo`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetTargetInfoParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<TargetID>,
}

/// Return type for `Target.getTargetInfo`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTargetInfoReturns {
    pub target_info: TargetInfo,
}

/// Parameters for `Target.getTargets`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetTargetsParams {
    /// Only targets matching filter will be reported. If filter is not specified
    /// and target discovery is currently enabled, a filter used for target discovery
    /// is used for consistency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<TargetFilter>,
}

/// Return type for `Target.getTargets`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTargetsReturns {
    /// The list of targets.
    pub target_infos: Vec<TargetInfo>,
}

/// Parameters for `Target.sendMessageToTarget`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageToTargetParams {
    pub message: String,
    /// Identifier of the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<SessionID>,
    /// Deprecated.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<TargetID>,
}

/// Parameters for `Target.setAutoAttach`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetAutoAttachParams {
    /// Whether to auto-attach to related targets.
    pub auto_attach: bool,
    /// Whether to pause new targets when attaching to them. Use `Runtime.runIfWaitingForDebugger`
    /// to run paused targets.
    pub wait_for_debugger_on_start: bool,
    /// Enables "flat" access to the session via specifying sessionId attribute in the commands.
    /// We plan to make this the default, deprecate non-flattened mode,
    /// and eventually retire it. See crbug.com/991325.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flatten: Option<bool>,
    /// Only targets matching filter will be attached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<TargetFilter>,
}

/// Parameters for `Target.autoAttachRelated`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoAttachRelatedParams {
    pub target_id: TargetID,
    /// Whether to pause new targets when attaching to them. Use `Runtime.runIfWaitingForDebugger`
    /// to run paused targets.
    pub wait_for_debugger_on_start: bool,
    /// Only targets matching filter will be attached.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<TargetFilter>,
}

/// Parameters for `Target.setDiscoverTargets`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetDiscoverTargetsParams {
    /// Whether to discover available targets.
    pub discover: bool,
    /// Only targets matching filter will be attached. If `discover` is false,
    /// `filter` must be omitted or empty.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<TargetFilter>,
}

/// Parameters for `Target.setRemoteLocations`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetRemoteLocationsParams {
    /// List of remote locations.
    pub locations: Vec<RemoteLocation>,
}

/// Parameters for `Target.getDevToolsTarget`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDevToolsTargetParams {
    /// Page or tab target ID.
    pub target_id: TargetID,
}

/// Return type for `Target.getDevToolsTarget`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDevToolsTargetReturns {
    /// The targetId of DevTools page target if exists.
    #[serde(default)]
    pub target_id: Option<TargetID>,
}

/// Parameters for `Target.openDevTools`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDevToolsParams {
    /// This can be the page or tab target ID.
    pub target_id: TargetID,
    /// The id of the panel we want DevTools to open initially. Currently
    /// supported panels are elements, console, network, sources, resources
    /// and performance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel_id: Option<String>,
}

/// Return type for `Target.openDevTools`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenDevToolsReturns {
    /// The targetId of DevTools page target.
    pub target_id: TargetID,
}

/// Event payload for `Target.attachedToTarget`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachedToTargetEvent {
    /// Identifier assigned to the session used to send/receive messages.
    pub session_id: SessionID,
    pub target_info: TargetInfo,
    pub waiting_for_debugger: bool,
}

/// Event payload for `Target.detachedFromTarget`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachedFromTargetEvent {
    /// Detached session identifier.
    pub session_id: SessionID,
    /// Deprecated.
    #[serde(default)]
    pub target_id: Option<TargetID>,
}

/// Event payload for `Target.receivedMessageFromTarget`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReceivedMessageFromTargetEvent {
    /// Identifier of a session which sends a message.
    pub session_id: SessionID,
    pub message: String,
    /// Deprecated.
    #[serde(default)]
    pub target_id: Option<TargetID>,
}

/// Event payload for `Target.targetCreated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCreatedEvent {
    pub target_info: TargetInfo,
}

/// Event payload for `Target.targetDestroyed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDestroyedEvent {
    pub target_id: TargetID,
}

/// Event payload for `Target.targetCrashed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetCrashedEvent {
    pub target_id: TargetID,
    /// Termination status type.
    pub status: String,
    /// Termination error code.
    pub error_code: i64,
}

/// Event payload for `Target.targetInfoChanged`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetInfoChangedEvent {
    pub target_info: TargetInfo,
}

// ── Methods ──
//
// These are the typed method signatures for Target.* commands.
// Integration into CdpSession is done in pwright-cdp.

// Activates (focuses) the target.
// async fn target_activate_target(&self, params: ActivateTargetParams) -> Result<()>
// CDP: "Target.activateTarget"

// Attaches to the target with given id.
// async fn target_attach_to_target(&self, params: AttachToTargetParams) -> Result<AttachToTargetReturns>
// CDP: "Target.attachToTarget"

// Attaches to the browser target, only uses flat sessionId mode.
// async fn target_attach_to_browser_target(&self) -> Result<AttachToBrowserTargetReturns>
// CDP: "Target.attachToBrowserTarget"

// Closes the target. If the target is a page that gets closed too.
// async fn target_close_target(&self, params: CloseTargetParams) -> Result<CloseTargetReturns>
// CDP: "Target.closeTarget"

// Inject object to the target's main frame that provides a communication
// channel with browser target.
//
// Injected object will be available as `window[bindingName]`.
//
// The object has the following API:
// - `binding.send(json)` - a method to send messages over the remote debugging protocol
// - `binding.onmessage = json => handleMessage(json)` - a callback that will be called for the protocol notifications and command responses.
// async fn target_expose_dev_tools_protocol(&self, params: ExposeDevToolsProtocolParams) -> Result<()>
// CDP: "Target.exposeDevToolsProtocol"

// Creates a new empty BrowserContext. Similar to an incognito profile but you can have more than
// one.
// async fn target_create_browser_context(&self, params: CreateBrowserContextParams) -> Result<CreateBrowserContextReturns>
// CDP: "Target.createBrowserContext"

// Returns all browser contexts created with `Target.createBrowserContext` method.
// async fn target_get_browser_contexts(&self) -> Result<GetBrowserContextsReturns>
// CDP: "Target.getBrowserContexts"

// Creates a new page.
// async fn target_create_target(&self, params: CreateTargetParams) -> Result<CreateTargetReturns>
// CDP: "Target.createTarget"

// Detaches session with given id.
// async fn target_detach_from_target(&self, params: DetachFromTargetParams) -> Result<()>
// CDP: "Target.detachFromTarget"

// Deletes a BrowserContext. All the belonging pages will be closed without calling their
// beforeunload hooks.
// async fn target_dispose_browser_context(&self, params: DisposeBrowserContextParams) -> Result<()>
// CDP: "Target.disposeBrowserContext"

// Returns information about a target.
// async fn target_get_target_info(&self, params: GetTargetInfoParams) -> Result<GetTargetInfoReturns>
// CDP: "Target.getTargetInfo"

// Retrieves a list of available targets.
// async fn target_get_targets(&self, params: GetTargetsParams) -> Result<GetTargetsReturns>
// CDP: "Target.getTargets"

// Sends protocol message over session with given id.
// Consider using flat mode instead; see commands attachToTarget, setAutoAttach,
// and crbug.com/991325.
// async fn target_send_message_to_target(&self, params: SendMessageToTargetParams) -> Result<()>
// CDP: "Target.sendMessageToTarget"

// Controls whether to automatically attach to new targets which are considered
// to be directly related to this one (for example, iframes or workers).
// When turned on, attaches to all existing related targets as well. When turned off,
// automatically detaches from all currently attached targets.
// This also clears all targets added by `autoAttachRelated` from the list of targets to watch
// for creation of related targets.
// You might want to call this recursively for auto-attached targets to attach
// to all available targets.
// async fn target_set_auto_attach(&self, params: SetAutoAttachParams) -> Result<()>
// CDP: "Target.setAutoAttach"

// Adds the specified target to the list of targets that will be monitored for any related target
// creation (such as child frames, child workers and new versions of service worker) and reported
// through `attachedToTarget`. The specified target is also auto-attached.
// This cancels the effect of any previous `setAutoAttach` and is also cancelled by subsequent
// `setAutoAttach`. Only available at the Browser target.
// async fn target_auto_attach_related(&self, params: AutoAttachRelatedParams) -> Result<()>
// CDP: "Target.autoAttachRelated"

// Controls whether to discover available targets and notify via
// `targetCreated/targetInfoChanged/targetDestroyed` events.
// async fn target_set_discover_targets(&self, params: SetDiscoverTargetsParams) -> Result<()>
// CDP: "Target.setDiscoverTargets"

// Enables target discovery for the specified locations, when `setDiscoverTargets` was set to
// `true`.
// async fn target_set_remote_locations(&self, params: SetRemoteLocationsParams) -> Result<()>
// CDP: "Target.setRemoteLocations"

// Gets the targetId of the DevTools page target opened for the given target
// (if any).
// async fn target_get_dev_tools_target(&self, params: GetDevToolsTargetParams) -> Result<GetDevToolsTargetReturns>
// CDP: "Target.getDevToolsTarget"

// Opens a DevTools window for the target.
// async fn target_open_dev_tools(&self, params: OpenDevToolsParams) -> Result<OpenDevToolsReturns>
// CDP: "Target.openDevTools"
