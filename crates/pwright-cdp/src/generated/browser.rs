//! CDP `Browser` domain — generated from protocol JSON.
//!
//! The Browser domain defines methods and events for browser managing.

#![allow(clippy::doc_markdown)]

use serde::{Deserialize, Serialize};

pub type BrowserContextID = String;

pub type WindowID = i64;

/// The state of the browser window.
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

/// Browser window bounds information
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    /// The offset from the left edge of the screen to the window in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub left: Option<i64>,
    /// The offset from the top edge of the screen to the window in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top: Option<i64>,
    /// The window width in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    /// The window height in pixels.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
    /// The window state. Default to normal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_state: Option<WindowState>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionType {
    #[serde(rename = "ar")]
    Ar,
    #[serde(rename = "audioCapture")]
    AudioCapture,
    #[serde(rename = "automaticFullscreen")]
    AutomaticFullscreen,
    #[serde(rename = "backgroundFetch")]
    BackgroundFetch,
    #[serde(rename = "backgroundSync")]
    BackgroundSync,
    #[serde(rename = "cameraPanTiltZoom")]
    CameraPanTiltZoom,
    #[serde(rename = "capturedSurfaceControl")]
    CapturedSurfaceControl,
    #[serde(rename = "clipboardReadWrite")]
    ClipboardReadWrite,
    #[serde(rename = "clipboardSanitizedWrite")]
    ClipboardSanitizedWrite,
    #[serde(rename = "displayCapture")]
    DisplayCapture,
    #[serde(rename = "durableStorage")]
    DurableStorage,
    #[serde(rename = "geolocation")]
    Geolocation,
    #[serde(rename = "handTracking")]
    HandTracking,
    #[serde(rename = "idleDetection")]
    IdleDetection,
    #[serde(rename = "keyboardLock")]
    KeyboardLock,
    #[serde(rename = "localFonts")]
    LocalFonts,
    #[serde(rename = "localNetworkAccess")]
    LocalNetworkAccess,
    #[serde(rename = "midi")]
    Midi,
    #[serde(rename = "midiSysex")]
    MidiSysex,
    #[serde(rename = "nfc")]
    Nfc,
    #[serde(rename = "notifications")]
    Notifications,
    #[serde(rename = "paymentHandler")]
    PaymentHandler,
    #[serde(rename = "periodicBackgroundSync")]
    PeriodicBackgroundSync,
    #[serde(rename = "pointerLock")]
    PointerLock,
    #[serde(rename = "protectedMediaIdentifier")]
    ProtectedMediaIdentifier,
    #[serde(rename = "sensors")]
    Sensors,
    #[serde(rename = "smartCard")]
    SmartCard,
    #[serde(rename = "speakerSelection")]
    SpeakerSelection,
    #[serde(rename = "storageAccess")]
    StorageAccess,
    #[serde(rename = "topLevelStorageAccess")]
    TopLevelStorageAccess,
    #[serde(rename = "videoCapture")]
    VideoCapture,
    #[serde(rename = "vr")]
    Vr,
    #[serde(rename = "wakeLockScreen")]
    WakeLockScreen,
    #[serde(rename = "wakeLockSystem")]
    WakeLockSystem,
    #[serde(rename = "webAppInstallation")]
    WebAppInstallation,
    #[serde(rename = "webPrinting")]
    WebPrinting,
    #[serde(rename = "windowManagement")]
    WindowManagement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionSetting {
    #[serde(rename = "granted")]
    Granted,
    #[serde(rename = "denied")]
    Denied,
    #[serde(rename = "prompt")]
    Prompt,
}

/// Definition of PermissionDescriptor defined in the Permissions API:
/// <https://w3c.github.io/permissions/#dom-permissiondescriptor.>
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionDescriptor {
    /// Name of permission.
    /// See <https://cs.chromium.org/chromium/src/third_party/blink/renderer/modules/permissions/permission_descriptor.idl> for valid permission names.
    pub name: String,
    /// For "midi" permission, may also specify sysex control.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sysex: Option<bool>,
    /// For "push" permission, may specify userVisibleOnly.
    /// Note that userVisibleOnly = true is the only currently supported type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_visible_only: Option<bool>,
    /// For "clipboard" permission, may specify allowWithoutSanitization.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_without_sanitization: Option<bool>,
    /// For "fullscreen" permission, must specify allowWithoutGesture:true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_without_gesture: Option<bool>,
    /// For "camera" permission, may specify panTiltZoom.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pan_tilt_zoom: Option<bool>,
}

/// Browser command ids used by executeBrowserCommand.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BrowserCommandId {
    #[serde(rename = "openTabSearch")]
    OpenTabSearch,
    #[serde(rename = "closeTabSearch")]
    CloseTabSearch,
    #[serde(rename = "openGlic")]
    OpenGlic,
}

/// Chrome histogram bucket.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    /// Minimum value (inclusive).
    pub low: i64,
    /// Maximum value (exclusive).
    pub high: i64,
    /// Number of samples.
    pub count: i64,
}

/// Chrome histogram.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Histogram {
    /// Name.
    pub name: String,
    /// Sum of sample values.
    pub sum: i64,
    /// Total number of samples.
    pub count: i64,
    /// Buckets.
    pub buckets: Vec<Bucket>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PrivacySandboxAPI {
    BiddingAndAuctionServices,
    TrustedKeyValue,
}

/// Parameters for `Browser.setPermission`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetPermissionParams {
    /// Descriptor of permission to override.
    pub permission: PermissionDescriptor,
    /// Setting of the permission.
    pub setting: PermissionSetting,
    /// Embedding origin the permission applies to, all origins if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Embedded origin the permission applies to. It is ignored unless the embedding origin is
    /// present and valid. If the embedding origin is provided but the embedded origin isn't, the
    /// embedding origin is used as the embedded origin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedded_origin: Option<String>,
    /// Context to override. When omitted, default browser context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextID>,
}

/// Parameters for `Browser.grantPermissions`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GrantPermissionsParams {
    pub permissions: Vec<PermissionType>,
    /// Origin the permission applies to, all origins if not specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// BrowserContext to override permissions. When omitted, default browser context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextID>,
}

/// Parameters for `Browser.resetPermissions`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResetPermissionsParams {
    /// BrowserContext to reset permissions. When omitted, default browser context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextID>,
}

/// Parameters for `Browser.setDownloadBehavior`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetDownloadBehaviorParams {
    /// Whether to allow all or deny all download requests, or use default Chrome behavior if
    /// available (otherwise deny). |allowAndName| allows download and names files according to
    /// their download guids.
    pub behavior: String,
    /// BrowserContext to set download behavior. When omitted, default browser context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextID>,
    /// The default path to save downloaded files to. This is required if behavior is set to 'allow'
    /// or 'allowAndName'.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_path: Option<String>,
    /// Whether to emit download events (defaults to false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub events_enabled: Option<bool>,
}

/// Parameters for `Browser.cancelDownload`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CancelDownloadParams {
    /// Global unique identifier of the download.
    pub guid: String,
    /// BrowserContext to perform the action in. When omitted, default browser context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextID>,
}

/// Return type for `Browser.getVersion`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetVersionReturns {
    /// Protocol version.
    pub protocol_version: String,
    /// Product name.
    pub product: String,
    /// Product revision.
    pub revision: String,
    /// User-Agent.
    pub user_agent: String,
    /// V8 version.
    pub js_version: String,
}

/// Return type for `Browser.getBrowserCommandLine`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBrowserCommandLineReturns {
    /// Commandline parameters
    pub arguments: Vec<String>,
}

/// Parameters for `Browser.getHistograms`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetHistogramsParams {
    /// Requested substring in name. Only histograms which have query as a
    /// substring in their name are extracted. An empty or absent query returns
    /// all histograms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,
    /// If true, retrieve delta since last delta call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<bool>,
}

/// Return type for `Browser.getHistograms`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHistogramsReturns {
    /// Histograms.
    pub histograms: Vec<Histogram>,
}

/// Parameters for `Browser.getHistogram`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetHistogramParams {
    /// Requested histogram name.
    pub name: String,
    /// If true, retrieve delta since last delta call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delta: Option<bool>,
}

/// Return type for `Browser.getHistogram`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetHistogramReturns {
    /// Histogram.
    pub histogram: Histogram,
}

/// Parameters for `Browser.getWindowBounds`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWindowBoundsParams {
    /// Browser window id.
    pub window_id: WindowID,
}

/// Return type for `Browser.getWindowBounds`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWindowBoundsReturns {
    /// Bounds information of the window. When window state is 'minimized', the restored window
    /// position and size are returned.
    pub bounds: Bounds,
}

/// Parameters for `Browser.getWindowForTarget`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetWindowForTargetParams {
    /// Devtools agent host id. If called as a part of the session, associated targetId is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<super::target::TargetID>,
}

/// Return type for `Browser.getWindowForTarget`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetWindowForTargetReturns {
    /// Browser window id.
    pub window_id: WindowID,
    /// Bounds information of the window. When window state is 'minimized', the restored window
    /// position and size are returned.
    pub bounds: Bounds,
}

/// Parameters for `Browser.setWindowBounds`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetWindowBoundsParams {
    /// Browser window id.
    pub window_id: WindowID,
    /// New window bounds. The 'minimized', 'maximized' and 'fullscreen' states cannot be combined
    /// with 'left', 'top', 'width' or 'height'. Leaves unspecified fields unchanged.
    pub bounds: Bounds,
}

/// Parameters for `Browser.setContentsSize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetContentsSizeParams {
    /// Browser window id.
    pub window_id: WindowID,
    /// The window contents width in DIP. Assumes current width if omitted.
    /// Must be specified if 'height' is omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
    /// The window contents height in DIP. Assumes current height if omitted.
    /// Must be specified if 'width' is omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<i64>,
}

/// Parameters for `Browser.setDockTile`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetDockTileParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge_label: Option<String>,
    /// Png encoded image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// Parameters for `Browser.executeBrowserCommand`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteBrowserCommandParams {
    pub command_id: BrowserCommandId,
}

/// Parameters for `Browser.addPrivacySandboxEnrollmentOverride`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddPrivacySandboxEnrollmentOverrideParams {
    pub url: String,
}

/// Parameters for `Browser.addPrivacySandboxCoordinatorKeyConfig`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddPrivacySandboxCoordinatorKeyConfigParams {
    pub api: PrivacySandboxAPI,
    pub coordinator_origin: String,
    pub key_config: String,
    /// BrowserContext to perform the action in. When omitted, default browser
    /// context is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_context_id: Option<BrowserContextID>,
}

/// Event payload for `Browser.downloadWillBegin`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadWillBeginEvent {
    /// Id of the frame that caused the download to begin.
    pub frame_id: super::page::FrameId,
    /// Global unique identifier of the download.
    pub guid: String,
    /// URL of the resource being downloaded.
    pub url: String,
    /// Suggested file name of the resource (the actual name of the file saved on disk may differ).
    pub suggested_filename: String,
}

/// Event payload for `Browser.downloadProgress`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgressEvent {
    /// Global unique identifier of the download.
    pub guid: String,
    /// Total expected bytes to download.
    pub total_bytes: f64,
    /// Total bytes received.
    pub received_bytes: f64,
    /// Download status.
    pub state: String,
    /// If download is "completed", provides the path of the downloaded file.
    /// Depending on the platform, it is not guaranteed to be set, nor the file
    /// is guaranteed to exist.
    #[serde(default)]
    pub file_path: Option<String>,
}

// ── Methods ──
//
// These are the typed method signatures for Browser.* commands.
// Integration into CdpSession is done in pwright-cdp.

// Set permission settings for given embedding and embedded origins.
// async fn browser_set_permission(&self, params: SetPermissionParams) -> Result<()>
// CDP: "Browser.setPermission"

// Grant specific permissions to the given origin and reject all others. Deprecated. Use
// setPermission instead.
// async fn browser_grant_permissions(&self, params: GrantPermissionsParams) -> Result<()>
// CDP: "Browser.grantPermissions"

// Reset all permission management for all origins.
// async fn browser_reset_permissions(&self, params: ResetPermissionsParams) -> Result<()>
// CDP: "Browser.resetPermissions"

// Set the behavior when downloading a file.
// async fn browser_set_download_behavior(&self, params: SetDownloadBehaviorParams) -> Result<()>
// CDP: "Browser.setDownloadBehavior"

// Cancel a download if in progress
// async fn browser_cancel_download(&self, params: CancelDownloadParams) -> Result<()>
// CDP: "Browser.cancelDownload"

// Close browser gracefully.
// async fn browser_close(&self) -> Result<()>
// CDP: "Browser.close"

// Crashes browser on the main thread.
// async fn browser_crash(&self) -> Result<()>
// CDP: "Browser.crash"

// Crashes GPU process.
// async fn browser_crash_gpu_process(&self) -> Result<()>
// CDP: "Browser.crashGpuProcess"

// Returns version information.
// async fn browser_get_version(&self) -> Result<GetVersionReturns>
// CDP: "Browser.getVersion"

// Returns the command line switches for the browser process if, and only if
// --enable-automation is on the commandline.
// async fn browser_get_browser_command_line(&self) -> Result<GetBrowserCommandLineReturns>
// CDP: "Browser.getBrowserCommandLine"

// Get Chrome histograms.
// async fn browser_get_histograms(&self, params: GetHistogramsParams) -> Result<GetHistogramsReturns>
// CDP: "Browser.getHistograms"

// Get a Chrome histogram by name.
// async fn browser_get_histogram(&self, params: GetHistogramParams) -> Result<GetHistogramReturns>
// CDP: "Browser.getHistogram"

// Get position and size of the browser window.
// async fn browser_get_window_bounds(&self, params: GetWindowBoundsParams) -> Result<GetWindowBoundsReturns>
// CDP: "Browser.getWindowBounds"

// Get the browser window that contains the devtools target.
// async fn browser_get_window_for_target(&self, params: GetWindowForTargetParams) -> Result<GetWindowForTargetReturns>
// CDP: "Browser.getWindowForTarget"

// Set position and/or size of the browser window.
// async fn browser_set_window_bounds(&self, params: SetWindowBoundsParams) -> Result<()>
// CDP: "Browser.setWindowBounds"

// Set size of the browser contents resizing browser window as necessary.
// async fn browser_set_contents_size(&self, params: SetContentsSizeParams) -> Result<()>
// CDP: "Browser.setContentsSize"

// Set dock tile details, platform-specific.
// async fn browser_set_dock_tile(&self, params: SetDockTileParams) -> Result<()>
// CDP: "Browser.setDockTile"

// Invoke custom browser commands used by telemetry.
// async fn browser_execute_browser_command(&self, params: ExecuteBrowserCommandParams) -> Result<()>
// CDP: "Browser.executeBrowserCommand"

// Allows a site to use privacy sandbox features that require enrollment
// without the site actually being enrolled. Only supported on page targets.
// async fn browser_add_privacy_sandbox_enrollment_override(&self, params: AddPrivacySandboxEnrollmentOverrideParams) -> Result<()>
// CDP: "Browser.addPrivacySandboxEnrollmentOverride"

// Configures encryption keys used with a given privacy sandbox API to talk
// to a trusted coordinator.  Since this is intended for test automation only,
// coordinatorOrigin must be a .test domain. No existing coordinator
// configuration for the origin may exist.
// async fn browser_add_privacy_sandbox_coordinator_key_config(&self, params: AddPrivacySandboxCoordinatorKeyConfigParams) -> Result<()>
// CDP: "Browser.addPrivacySandboxCoordinatorKeyConfig"
