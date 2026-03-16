//! CDP `Page` domain — generated from protocol JSON.
//!
//! Actions and events related to the inspected page belong to the page domain.

#![allow(clippy::doc_markdown)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Unique frame identifier.
pub type FrameId = String;

/// Indicates whether a frame has been identified as an ad.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdFrameType {
    #[serde(rename = "none")]
    None,
    #[serde(rename = "child")]
    Child,
    #[serde(rename = "root")]
    Root,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AdFrameExplanation {
    ParentIsAd,
    CreatedByAdScript,
    MatchedBlockingRule,
}

/// Indicates whether a frame has been identified as an ad and why.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdFrameStatus {
    pub ad_frame_type: AdFrameType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanations: Option<Vec<AdFrameExplanation>>,
}

/// Identifies the script which caused a script or frame to be labelled as an
/// ad.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdScriptId {
    /// Script Id of the script which caused a script or frame to be labelled as
    /// an ad.
    pub script_id: super::runtime::ScriptId,
    /// Id of scriptId's debugger.
    pub debugger_id: super::runtime::UniqueDebuggerId,
}

/// Encapsulates the script ancestry and the root script filterlist rule that
/// caused the frame to be labelled as an ad. Only created when `ancestryChain`
/// is not empty.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdScriptAncestry {
    /// A chain of `AdScriptId`s representing the ancestry of an ad script that
    /// led to the creation of a frame. The chain is ordered from the script
    /// itself (lower level) up to its root ancestor that was flagged by
    /// filterlist.
    pub ancestry_chain: Vec<AdScriptId>,
    /// The filterlist rule that caused the root (last) script in
    /// `ancestryChain` to be ad-tagged. Only populated if the rule is
    /// available.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_script_filterlist_rule: Option<String>,
}

/// Indicates whether the frame is a secure context and why it is the case.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SecureContextType {
    Secure,
    SecureLocalhost,
    InsecureScheme,
    InsecureAncestor,
}

/// Indicates whether the frame is cross-origin isolated and why it is the case.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CrossOriginIsolatedContextType {
    Isolated,
    NotIsolated,
    NotIsolatedFeatureDisabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GatedAPIFeatures {
    SharedArrayBuffers,
    SharedArrayBuffersTransferAllowed,
    PerformanceMeasureMemory,
    PerformanceProfile,
}

/// All Permissions Policy features. This enum should match the one defined
/// in services/network/public/cpp/permissions_policy/permissions_policy_features.json5.
/// LINT.IfChange(PermissionsPolicyFeature)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionsPolicyFeature {
    #[serde(rename = "accelerometer")]
    Accelerometer,
    #[serde(rename = "all-screens-capture")]
    AllScreensCapture,
    #[serde(rename = "ambient-light-sensor")]
    AmbientLightSensor,
    #[serde(rename = "aria-notify")]
    AriaNotify,
    #[serde(rename = "attribution-reporting")]
    AttributionReporting,
    #[serde(rename = "autofill")]
    Autofill,
    #[serde(rename = "autoplay")]
    Autoplay,
    #[serde(rename = "bluetooth")]
    Bluetooth,
    #[serde(rename = "browsing-topics")]
    BrowsingTopics,
    #[serde(rename = "camera")]
    Camera,
    #[serde(rename = "captured-surface-control")]
    CapturedSurfaceControl,
    #[serde(rename = "ch-dpr")]
    ChDpr,
    #[serde(rename = "ch-device-memory")]
    ChDeviceMemory,
    #[serde(rename = "ch-downlink")]
    ChDownlink,
    #[serde(rename = "ch-ect")]
    ChEct,
    #[serde(rename = "ch-prefers-color-scheme")]
    ChPrefersColorScheme,
    #[serde(rename = "ch-prefers-reduced-motion")]
    ChPrefersReducedMotion,
    #[serde(rename = "ch-prefers-reduced-transparency")]
    ChPrefersReducedTransparency,
    #[serde(rename = "ch-rtt")]
    ChRtt,
    #[serde(rename = "ch-save-data")]
    ChSaveData,
    #[serde(rename = "ch-ua")]
    ChUa,
    #[serde(rename = "ch-ua-arch")]
    ChUaArch,
    #[serde(rename = "ch-ua-bitness")]
    ChUaBitness,
    #[serde(rename = "ch-ua-high-entropy-values")]
    ChUaHighEntropyValues,
    #[serde(rename = "ch-ua-platform")]
    ChUaPlatform,
    #[serde(rename = "ch-ua-model")]
    ChUaModel,
    #[serde(rename = "ch-ua-mobile")]
    ChUaMobile,
    #[serde(rename = "ch-ua-form-factors")]
    ChUaFormFactors,
    #[serde(rename = "ch-ua-full-version")]
    ChUaFullVersion,
    #[serde(rename = "ch-ua-full-version-list")]
    ChUaFullVersionList,
    #[serde(rename = "ch-ua-platform-version")]
    ChUaPlatformVersion,
    #[serde(rename = "ch-ua-wow64")]
    ChUaWow64,
    #[serde(rename = "ch-viewport-height")]
    ChViewportHeight,
    #[serde(rename = "ch-viewport-width")]
    ChViewportWidth,
    #[serde(rename = "ch-width")]
    ChWidth,
    #[serde(rename = "clipboard-read")]
    ClipboardRead,
    #[serde(rename = "clipboard-write")]
    ClipboardWrite,
    #[serde(rename = "compute-pressure")]
    ComputePressure,
    #[serde(rename = "controlled-frame")]
    ControlledFrame,
    #[serde(rename = "cross-origin-isolated")]
    CrossOriginIsolated,
    #[serde(rename = "deferred-fetch")]
    DeferredFetch,
    #[serde(rename = "deferred-fetch-minimal")]
    DeferredFetchMinimal,
    #[serde(rename = "device-attributes")]
    DeviceAttributes,
    #[serde(rename = "digital-credentials-create")]
    DigitalCredentialsCreate,
    #[serde(rename = "digital-credentials-get")]
    DigitalCredentialsGet,
    #[serde(rename = "direct-sockets")]
    DirectSockets,
    #[serde(rename = "direct-sockets-multicast")]
    DirectSocketsMulticast,
    #[serde(rename = "direct-sockets-private")]
    DirectSocketsPrivate,
    #[serde(rename = "display-capture")]
    DisplayCapture,
    #[serde(rename = "document-domain")]
    DocumentDomain,
    #[serde(rename = "encrypted-media")]
    EncryptedMedia,
    #[serde(rename = "execution-while-out-of-viewport")]
    ExecutionWhileOutOfViewport,
    #[serde(rename = "execution-while-not-rendered")]
    ExecutionWhileNotRendered,
    #[serde(rename = "fenced-unpartitioned-storage-read")]
    FencedUnpartitionedStorageRead,
    #[serde(rename = "focus-without-user-activation")]
    FocusWithoutUserActivation,
    #[serde(rename = "fullscreen")]
    Fullscreen,
    #[serde(rename = "frobulate")]
    Frobulate,
    #[serde(rename = "gamepad")]
    Gamepad,
    #[serde(rename = "geolocation")]
    Geolocation,
    #[serde(rename = "gyroscope")]
    Gyroscope,
    #[serde(rename = "hid")]
    Hid,
    #[serde(rename = "identity-credentials-get")]
    IdentityCredentialsGet,
    #[serde(rename = "idle-detection")]
    IdleDetection,
    #[serde(rename = "interest-cohort")]
    InterestCohort,
    #[serde(rename = "join-ad-interest-group")]
    JoinAdInterestGroup,
    #[serde(rename = "keyboard-map")]
    KeyboardMap,
    #[serde(rename = "language-detector")]
    LanguageDetector,
    #[serde(rename = "language-model")]
    LanguageModel,
    #[serde(rename = "local-fonts")]
    LocalFonts,
    #[serde(rename = "local-network-access")]
    LocalNetworkAccess,
    #[serde(rename = "magnetometer")]
    Magnetometer,
    #[serde(rename = "manual-text")]
    ManualText,
    #[serde(rename = "media-playback-while-not-visible")]
    MediaPlaybackWhileNotVisible,
    #[serde(rename = "microphone")]
    Microphone,
    #[serde(rename = "midi")]
    Midi,
    #[serde(rename = "on-device-speech-recognition")]
    OnDeviceSpeechRecognition,
    #[serde(rename = "otp-credentials")]
    OtpCredentials,
    #[serde(rename = "payment")]
    Payment,
    #[serde(rename = "picture-in-picture")]
    PictureInPicture,
    #[serde(rename = "private-aggregation")]
    PrivateAggregation,
    #[serde(rename = "private-state-token-issuance")]
    PrivateStateTokenIssuance,
    #[serde(rename = "private-state-token-redemption")]
    PrivateStateTokenRedemption,
    #[serde(rename = "publickey-credentials-create")]
    PublickeyCredentialsCreate,
    #[serde(rename = "publickey-credentials-get")]
    PublickeyCredentialsGet,
    #[serde(rename = "record-ad-auction-events")]
    RecordAdAuctionEvents,
    #[serde(rename = "rewriter")]
    Rewriter,
    #[serde(rename = "run-ad-auction")]
    RunAdAuction,
    #[serde(rename = "screen-wake-lock")]
    ScreenWakeLock,
    #[serde(rename = "serial")]
    Serial,
    #[serde(rename = "shared-storage")]
    SharedStorage,
    #[serde(rename = "shared-storage-select-url")]
    SharedStorageSelectUrl,
    #[serde(rename = "smart-card")]
    SmartCard,
    #[serde(rename = "speaker-selection")]
    SpeakerSelection,
    #[serde(rename = "storage-access")]
    StorageAccess,
    #[serde(rename = "sub-apps")]
    SubApps,
    #[serde(rename = "summarizer")]
    Summarizer,
    #[serde(rename = "sync-xhr")]
    SyncXhr,
    #[serde(rename = "translator")]
    Translator,
    #[serde(rename = "unload")]
    Unload,
    #[serde(rename = "usb")]
    Usb,
    #[serde(rename = "usb-unrestricted")]
    UsbUnrestricted,
    #[serde(rename = "vertical-scroll")]
    VerticalScroll,
    #[serde(rename = "web-app-installation")]
    WebAppInstallation,
    #[serde(rename = "web-printing")]
    WebPrinting,
    #[serde(rename = "web-share")]
    WebShare,
    #[serde(rename = "window-management")]
    WindowManagement,
    #[serde(rename = "writer")]
    Writer,
    #[serde(rename = "xr-spatial-tracking")]
    XrSpatialTracking,
}

/// Reason for a permissions policy feature to be disabled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PermissionsPolicyBlockReason {
    Header,
    IframeAttribute,
    InFencedFrameTree,
    InIsolatedApp,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsPolicyBlockLocator {
    pub frame_id: FrameId,
    pub block_reason: PermissionsPolicyBlockReason,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionsPolicyFeatureState {
    pub feature: PermissionsPolicyFeature,
    pub allowed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<PermissionsPolicyBlockLocator>,
}

/// Origin Trial(<https://www.chromium.org/blink/origin-trials)> support.
/// Status for an Origin Trial token.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OriginTrialTokenStatus {
    Success,
    NotSupported,
    Insecure,
    Expired,
    WrongOrigin,
    InvalidSignature,
    Malformed,
    WrongVersion,
    FeatureDisabled,
    TokenDisabled,
    FeatureDisabledForUser,
    UnknownTrial,
}

/// Status for an Origin Trial.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OriginTrialStatus {
    Enabled,
    ValidTokenNotProvided,
    OSNotSupported,
    TrialNotAllowed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OriginTrialUsageRestriction {
    None,
    Subset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginTrialToken {
    pub origin: String,
    pub match_sub_domains: bool,
    pub trial_name: String,
    pub expiry_time: super::network::TimeSinceEpoch,
    pub is_third_party: bool,
    pub usage_restriction: OriginTrialUsageRestriction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginTrialTokenWithStatus {
    pub raw_token_text: String,
    /// `parsedToken` is present only when the token is extractable and
    /// parsable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parsed_token: Option<OriginTrialToken>,
    pub status: OriginTrialTokenStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginTrial {
    pub trial_name: String,
    pub status: OriginTrialStatus,
    pub tokens_with_status: Vec<OriginTrialTokenWithStatus>,
}

/// Additional information about the frame document's security origin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SecurityOriginDetails {
    /// Indicates whether the frame document's security origin is one
    /// of the local hostnames (e.g. "localhost") or IP addresses (IPv4
    /// 127.0.0.0/8 or IPv6 ::1).
    pub is_localhost: bool,
}

/// Information about the Frame on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Frame {
    /// Frame unique identifier.
    pub id: FrameId,
    /// Parent frame identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<FrameId>,
    /// Identifier of the loader associated with this frame.
    pub loader_id: super::network::LoaderId,
    /// Frame's name as specified in the tag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Frame document's URL without fragment.
    pub url: String,
    /// Frame document's URL fragment including the '#'.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url_fragment: Option<String>,
    /// Frame document's registered domain, taking the public suffixes list into account.
    /// Extracted from the Frame's url.
    /// Example URLs: <http://www.google.com/file.html> -> "google.com"
    ///               <http://a.b.co.uk/file.html>      -> "b.co.uk"
    pub domain_and_registry: String,
    /// Frame document's security origin.
    pub security_origin: String,
    /// Additional details about the frame document's security origin.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_origin_details: Option<SecurityOriginDetails>,
    /// Frame document's mimeType as determined by the browser.
    pub mime_type: String,
    /// If the frame failed to load, this contains the URL that could not be loaded. Note that unlike url above, this URL may contain a fragment.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unreachable_url: Option<String>,
    /// Indicates whether this frame was tagged as an ad and why.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ad_frame_status: Option<AdFrameStatus>,
    /// Indicates whether the main document is a secure context and explains why that is the case.
    pub secure_context_type: SecureContextType,
    /// Indicates whether this is a cross origin isolated context.
    pub cross_origin_isolated_context_type: CrossOriginIsolatedContextType,
    /// Indicated which gated APIs / features are available.
    #[serde(rename = "gatedAPIFeatures")]
    pub gated_api_features: Vec<GatedAPIFeatures>,
}

/// Information about the Resource on the page.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameResource {
    /// Resource URL.
    pub url: String,
    /// Type of this resource.
    #[serde(rename = "type")]
    pub r#type: super::network::ResourceType,
    /// Resource mimeType as determined by the browser.
    pub mime_type: String,
    /// last-modified timestamp as reported by server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<super::network::TimeSinceEpoch>,
    /// Resource content size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_size: Option<f64>,
    /// True if the resource failed to load.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed: Option<bool>,
    /// True if the resource was canceled during loading.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub canceled: Option<bool>,
}

/// Information about the Frame hierarchy along with their cached resources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameResourceTree {
    /// Frame information for this tree item.
    pub frame: Frame,
    /// Child frames.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_frames: Option<Vec<FrameResourceTree>>,
    /// Information about frame resources.
    pub resources: Vec<FrameResource>,
}

/// Information about the Frame hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameTree {
    /// Frame information for this tree item.
    pub frame: Frame,
    /// Child frames.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_frames: Option<Vec<FrameTree>>,
}

/// Unique script identifier.
pub type ScriptIdentifier = String;

/// Transition type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransitionType {
    #[serde(rename = "link")]
    Link,
    #[serde(rename = "typed")]
    Typed,
    #[serde(rename = "address_bar")]
    AddressBar,
    #[serde(rename = "auto_bookmark")]
    AutoBookmark,
    #[serde(rename = "auto_subframe")]
    AutoSubframe,
    #[serde(rename = "manual_subframe")]
    ManualSubframe,
    #[serde(rename = "generated")]
    Generated,
    #[serde(rename = "auto_toplevel")]
    AutoToplevel,
    #[serde(rename = "form_submit")]
    FormSubmit,
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "keyword")]
    Keyword,
    #[serde(rename = "keyword_generated")]
    KeywordGenerated,
    #[serde(rename = "other")]
    Other,
}

/// Navigation history entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigationEntry {
    /// Unique id of the navigation history entry.
    pub id: i64,
    /// URL of the navigation history entry.
    pub url: String,
    /// URL that the user typed in the url bar.
    #[serde(rename = "userTypedURL")]
    pub user_typed_url: String,
    /// Title of the navigation history entry.
    pub title: String,
    /// Transition type.
    pub transition_type: TransitionType,
}

/// Screencast frame metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastFrameMetadata {
    /// Top offset in DIP.
    pub offset_top: f64,
    /// Page scale factor.
    pub page_scale_factor: f64,
    /// Device screen width in DIP.
    pub device_width: f64,
    /// Device screen height in DIP.
    pub device_height: f64,
    /// Position of horizontal scroll in CSS pixels.
    pub scroll_offset_x: f64,
    /// Position of vertical scroll in CSS pixels.
    pub scroll_offset_y: f64,
    /// Frame swap timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<super::network::TimeSinceEpoch>,
}

/// Javascript dialog type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DialogType {
    #[serde(rename = "alert")]
    Alert,
    #[serde(rename = "confirm")]
    Confirm,
    #[serde(rename = "prompt")]
    Prompt,
    #[serde(rename = "beforeunload")]
    Beforeunload,
}

/// Error while paring app manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManifestError {
    /// Error message.
    pub message: String,
    /// If critical, this is a non-recoverable parse error.
    pub critical: i64,
    /// Error line.
    pub line: i64,
    /// Error column.
    pub column: i64,
}

/// Parsed app manifest properties.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppManifestParsedProperties {
    /// Computed scope value
    pub scope: String,
}

/// Layout viewport position and dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutViewport {
    /// Horizontal offset relative to the document (CSS pixels).
    pub page_x: i64,
    /// Vertical offset relative to the document (CSS pixels).
    pub page_y: i64,
    /// Width (CSS pixels), excludes scrollbar if present.
    pub client_width: i64,
    /// Height (CSS pixels), excludes scrollbar if present.
    pub client_height: i64,
}

/// Visual viewport position, dimensions, and scale.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VisualViewport {
    /// Horizontal offset relative to the layout viewport (CSS pixels).
    pub offset_x: f64,
    /// Vertical offset relative to the layout viewport (CSS pixels).
    pub offset_y: f64,
    /// Horizontal offset relative to the document (CSS pixels).
    pub page_x: f64,
    /// Vertical offset relative to the document (CSS pixels).
    pub page_y: f64,
    /// Width (CSS pixels), excludes scrollbar if present.
    pub client_width: f64,
    /// Height (CSS pixels), excludes scrollbar if present.
    pub client_height: f64,
    /// Scale relative to the ideal viewport (size at width=device-width).
    pub scale: f64,
    /// Page zoom factor (CSS to device independent pixels ratio).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoom: Option<f64>,
}

/// Viewport for capturing screenshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Viewport {
    /// X offset in device independent pixels (dip).
    pub x: f64,
    /// Y offset in device independent pixels (dip).
    pub y: f64,
    /// Rectangle width in device independent pixels (dip).
    pub width: f64,
    /// Rectangle height in device independent pixels (dip).
    pub height: f64,
    /// Page scale factor.
    pub scale: f64,
}

/// Generic font families collection.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FontFamilies {
    /// The standard font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard: Option<String>,
    /// The fixed font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<String>,
    /// The serif font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub serif: Option<String>,
    /// The sansSerif font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sans_serif: Option<String>,
    /// The cursive font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursive: Option<String>,
    /// The fantasy font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fantasy: Option<String>,
    /// The math font-family.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub math: Option<String>,
}

/// Font families collection for a script.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptFontFamilies {
    /// Name of the script which these font families are defined for.
    pub script: String,
    /// Generic font families collection for the script.
    pub font_families: FontFamilies,
}

/// Default font sizes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FontSizes {
    /// Default standard font size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub standard: Option<i64>,
    /// Default fixed font size.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientNavigationReason {
    #[serde(rename = "anchorClick")]
    AnchorClick,
    #[serde(rename = "formSubmissionGet")]
    FormSubmissionGet,
    #[serde(rename = "formSubmissionPost")]
    FormSubmissionPost,
    #[serde(rename = "httpHeaderRefresh")]
    HttpHeaderRefresh,
    #[serde(rename = "initialFrameNavigation")]
    InitialFrameNavigation,
    #[serde(rename = "metaTagRefresh")]
    MetaTagRefresh,
    #[serde(rename = "other")]
    Other,
    #[serde(rename = "pageBlockInterstitial")]
    PageBlockInterstitial,
    #[serde(rename = "reload")]
    Reload,
    #[serde(rename = "scriptInitiated")]
    ScriptInitiated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientNavigationDisposition {
    #[serde(rename = "currentTab")]
    CurrentTab,
    #[serde(rename = "newTab")]
    NewTab,
    #[serde(rename = "newWindow")]
    NewWindow,
    #[serde(rename = "download")]
    Download,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallabilityErrorArgument {
    /// Argument name (e.g. name:'minimum-icon-size-in-pixels').
    pub name: String,
    /// Argument value (e.g. value:'64').
    pub value: String,
}

/// The installability error
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallabilityError {
    /// The error id (e.g. 'manifest-missing-suitable-icon').
    pub error_id: String,
    /// The list of error arguments (e.g. {name:'minimum-icon-size-in-pixels', value:'64'}).
    pub error_arguments: Vec<InstallabilityErrorArgument>,
}

/// The referring-policy used for the navigation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReferrerPolicy {
    #[serde(rename = "noReferrer")]
    NoReferrer,
    #[serde(rename = "noReferrerWhenDowngrade")]
    NoReferrerWhenDowngrade,
    #[serde(rename = "origin")]
    Origin,
    #[serde(rename = "originWhenCrossOrigin")]
    OriginWhenCrossOrigin,
    #[serde(rename = "sameOrigin")]
    SameOrigin,
    #[serde(rename = "strictOrigin")]
    StrictOrigin,
    #[serde(rename = "strictOriginWhenCrossOrigin")]
    StrictOriginWhenCrossOrigin,
    #[serde(rename = "unsafeUrl")]
    UnsafeUrl,
}

/// Per-script compilation cache parameters for `Page.produceCompilationCache`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilationCacheParams {
    /// The URL of the script to produce a compilation cache entry for.
    pub url: String,
    /// A hint to the backend whether eager compilation is recommended.
    /// (the actual compilation mode used is upon backend discretion).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eager: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FileFilter {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepts: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileHandler {
    pub action: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<ImageResource>>,
    /// Mimic a map, name is the key, accepts is the value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepts: Option<Vec<FileFilter>>,
    /// Won't repeat the enums, using string for easy comparison. Same as the
    /// other enums below.
    pub launch_type: String,
}

/// The image definition used in both icon and screenshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageResource {
    /// The src field in the definition, but changing to url in favor of
    /// consistency.
    pub url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sizes: Option<String>,
    #[serde(rename = "type")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchHandler {
    pub client_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolHandler {
    pub protocol: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RelatedApplication {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeExtension {
    /// Instead of using tuple, this field always returns the serialized string
    /// for easy understanding and comparison.
    pub origin: String,
    pub has_origin_wildcard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Screenshot {
    pub image: ImageResource,
    pub form_factor: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareTarget {
    pub action: String,
    pub method: String,
    pub enctype: String,
    /// Embed the ShareTargetParams
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<FileFilter>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shortcut {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WebAppManifest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background_color: Option<String>,
    /// The extra description provided by the manifest.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
    /// The overrided display mode controlled by the user.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_overrides: Option<Vec<String>>,
    /// The handlers to open files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_handlers: Option<Vec<FileHandler>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icons: Option<Vec<ImageResource>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lang: Option<String>,
    /// TODO(crbug.com/1231886): This field is non-standard and part of a Chrome
    /// experiment. See:
    /// <https://github.com/WICG/web-app-launch/blob/main/launch_handler.md>
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub launch_handler: Option<LaunchHandler>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub orientation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefer_related_applications: Option<bool>,
    /// The handlers to open protocols.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_handlers: Option<Vec<ProtocolHandler>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_applications: Option<Vec<RelatedApplication>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Non-standard, see
    /// <https://github.com/WICG/manifest-incubations/blob/gh-pages/scope_extensions-explainer.md>
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_extensions: Option<Vec<ScopeExtension>>,
    /// The screenshots used by chromium.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub screenshots: Option<Vec<Screenshot>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub share_target: Option<ShareTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub short_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shortcuts: Option<Vec<Shortcut>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme_color: Option<String>,
}

/// The type of a frameNavigated event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum NavigationType {
    Navigation,
    BackForwardCacheRestore,
}

/// List of not restored reasons for back-forward cache.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackForwardCacheNotRestoredReason {
    NotPrimaryMainFrame,
    BackForwardCacheDisabled,
    RelatedActiveContentsExist,
    HTTPStatusNotOK,
    SchemeNotHTTPOrHTTPS,
    Loading,
    WasGrantedMediaAccess,
    DisableForRenderFrameHostCalled,
    DomainNotAllowed,
    HTTPMethodNotGET,
    SubframeIsNavigating,
    Timeout,
    CacheLimit,
    JavaScriptExecution,
    RendererProcessKilled,
    RendererProcessCrashed,
    SchedulerTrackedFeatureUsed,
    ConflictingBrowsingInstance,
    CacheFlushed,
    ServiceWorkerVersionActivation,
    SessionRestored,
    ServiceWorkerPostMessage,
    EnteredBackForwardCacheBeforeServiceWorkerHostAdded,
    #[serde(rename = "RenderFrameHostReused_SameSite")]
    RenderFrameHostReusedSameSite,
    #[serde(rename = "RenderFrameHostReused_CrossSite")]
    RenderFrameHostReusedCrossSite,
    ServiceWorkerClaim,
    IgnoreEventAndEvict,
    HaveInnerContents,
    TimeoutPuttingInCache,
    BackForwardCacheDisabledByLowMemory,
    BackForwardCacheDisabledByCommandLine,
    NetworkRequestDatapipeDrainedAsBytesConsumer,
    NetworkRequestRedirected,
    NetworkRequestTimeout,
    NetworkExceedsBufferLimit,
    NavigationCancelledWhileRestoring,
    NotMostRecentNavigationEntry,
    BackForwardCacheDisabledForPrerender,
    UserAgentOverrideDiffers,
    ForegroundCacheLimit,
    BrowsingInstanceNotSwapped,
    BackForwardCacheDisabledForDelegate,
    UnloadHandlerExistsInMainFrame,
    UnloadHandlerExistsInSubFrame,
    ServiceWorkerUnregistration,
    CacheControlNoStore,
    CacheControlNoStoreCookieModified,
    CacheControlNoStoreHTTPOnlyCookieModified,
    NoResponseHead,
    Unknown,
    ActivationNavigationsDisallowedForBug1234857,
    ErrorDocument,
    FencedFramesEmbedder,
    CookieDisabled,
    HTTPAuthRequired,
    CookieFlushed,
    BroadcastChannelOnMessage,
    WebViewSettingsChanged,
    WebViewJavaScriptObjectChanged,
    WebViewMessageListenerInjected,
    WebViewSafeBrowsingAllowlistChanged,
    WebViewDocumentStartJavascriptChanged,
    WebSocket,
    WebTransport,
    WebRTC,
    MainResourceHasCacheControlNoStore,
    MainResourceHasCacheControlNoCache,
    SubresourceHasCacheControlNoStore,
    SubresourceHasCacheControlNoCache,
    ContainsPlugins,
    DocumentLoaded,
    OutstandingNetworkRequestOthers,
    RequestedMIDIPermission,
    RequestedAudioCapturePermission,
    RequestedVideoCapturePermission,
    RequestedBackForwardCacheBlockedSensors,
    RequestedBackgroundWorkPermission,
    BroadcastChannel,
    WebXR,
    SharedWorker,
    SharedWorkerMessage,
    SharedWorkerWithNoActiveClient,
    WebLocks,
    WebHID,
    WebBluetooth,
    WebShare,
    RequestedStorageAccessGrant,
    WebNfc,
    OutstandingNetworkRequestFetch,
    OutstandingNetworkRequestXHR,
    AppBanner,
    Printing,
    WebDatabase,
    PictureInPicture,
    SpeechRecognizer,
    IdleManager,
    PaymentManager,
    SpeechSynthesis,
    KeyboardLock,
    WebOTPService,
    OutstandingNetworkRequestDirectSocket,
    InjectedJavascript,
    InjectedStyleSheet,
    KeepaliveRequest,
    IndexedDBEvent,
    Dummy,
    JsNetworkRequestReceivedCacheControlNoStoreResource,
    WebRTCUsedWithCCNS,
    WebTransportUsedWithCCNS,
    WebSocketUsedWithCCNS,
    SmartCard,
    LiveMediaStreamTrack,
    UnloadHandler,
    ParserAborted,
    ContentSecurityHandler,
    ContentWebAuthenticationAPI,
    ContentFileChooser,
    ContentSerial,
    ContentFileSystemAccess,
    ContentMediaDevicesDispatcherHost,
    ContentWebBluetooth,
    ContentWebUSB,
    ContentMediaSessionService,
    ContentScreenReader,
    ContentDiscarded,
    EmbedderPopupBlockerTabHelper,
    EmbedderSafeBrowsingTriggeredPopupBlocker,
    EmbedderSafeBrowsingThreatDetails,
    EmbedderAppBannerManager,
    EmbedderDomDistillerViewerSource,
    EmbedderDomDistillerSelfDeletingRequestDelegate,
    EmbedderOomInterventionTabHelper,
    EmbedderOfflinePage,
    EmbedderChromePasswordManagerClientBindCredentialManager,
    EmbedderPermissionRequestManager,
    EmbedderModalDialog,
    EmbedderExtensions,
    EmbedderExtensionMessaging,
    EmbedderExtensionMessagingForOpenPort,
    EmbedderExtensionSentMessageToCachedFrame,
    RequestedByWebViewClient,
    PostMessageByWebViewClient,
    CacheControlNoStoreDeviceBoundSessionTerminated,
    CacheLimitPrunedOnModerateMemoryPressure,
    CacheLimitPrunedOnCriticalMemoryPressure,
}

/// Types of not restored reasons for back-forward cache.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum BackForwardCacheNotRestoredReasonType {
    SupportPending,
    PageSupportNeeded,
    Circumstantial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheBlockingDetails {
    /// Url of the file where blockage happened. Optional because of tests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Function name where blockage happened. Optional because of anonymous functions and tests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub function: Option<String>,
    /// Line number in the script (0-based).
    pub line_number: i64,
    /// Column number in the script (0-based).
    pub column_number: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheNotRestoredExplanation {
    /// Type of the reason
    #[serde(rename = "type")]
    pub r#type: BackForwardCacheNotRestoredReasonType,
    /// Not restored reason
    pub reason: BackForwardCacheNotRestoredReason,
    /// Context associated with the reason. The meaning of this context is
    /// dependent on the reason:
    /// - EmbedderExtensionSentMessageToCachedFrame: the extension ID.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<Vec<BackForwardCacheBlockingDetails>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheNotRestoredExplanationTree {
    /// URL of each frame
    pub url: String,
    /// Not restored reasons of each frame
    pub explanations: Vec<BackForwardCacheNotRestoredExplanation>,
    /// Array of children frame
    pub children: Vec<BackForwardCacheNotRestoredExplanationTree>,
}

/// Parameters for `Page.addScriptToEvaluateOnLoad`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddScriptToEvaluateOnLoadParams {
    pub script_source: String,
}

/// Return type for `Page.addScriptToEvaluateOnLoad`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScriptToEvaluateOnLoadReturns {
    /// Identifier of the added script.
    pub identifier: ScriptIdentifier,
}

/// Parameters for `Page.addScriptToEvaluateOnNewDocument`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddScriptToEvaluateOnNewDocumentParams {
    pub source: String,
    /// If specified, creates an isolated world with the given name and evaluates given script in it.
    /// This world name will be used as the ExecutionContextDescription::name when the corresponding
    /// event is emitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_name: Option<String>,
    /// Specifies whether command line API should be available to the script, defaults
    /// to false.
    #[serde(rename = "includeCommandLineAPI")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_command_line_api: Option<bool>,
    /// If true, runs the script immediately on existing execution contexts or worlds.
    /// Default: false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_immediately: Option<bool>,
}

/// Return type for `Page.addScriptToEvaluateOnNewDocument`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddScriptToEvaluateOnNewDocumentReturns {
    /// Identifier of the added script.
    pub identifier: ScriptIdentifier,
}

/// Parameters for `Page.captureScreenshot`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotParams {
    /// Image compression format (defaults to png).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Compression quality from range \[0..100\] (jpeg only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<i64>,
    /// Capture the screenshot of a given region only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub clip: Option<Viewport>,
    /// Capture the screenshot from the surface, rather than the view. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_surface: Option<bool>,
    /// Capture the screenshot beyond the viewport. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capture_beyond_viewport: Option<bool>,
    /// Optimize image encoding for speed, not for resulting size (defaults to false)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optimize_for_speed: Option<bool>,
}

/// Return type for `Page.captureScreenshot`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureScreenshotReturns {
    /// Base64-encoded image data.
    pub data: String,
}

/// Parameters for `Page.captureSnapshot`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshotParams {
    /// Format (defaults to mhtml).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

/// Return type for `Page.captureSnapshot`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSnapshotReturns {
    /// Serialized page data.
    pub data: String,
}

/// Parameters for `Page.createIsolatedWorld`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIsolatedWorldParams {
    /// Id of the frame in which the isolated world should be created.
    pub frame_id: FrameId,
    /// An optional name which is reported in the Execution Context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_name: Option<String>,
    /// Whether or not universal access should be granted to the isolated world. This is a powerful
    /// option, use with caution.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub grant_univeral_access: Option<bool>,
}

/// Return type for `Page.createIsolatedWorld`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateIsolatedWorldReturns {
    /// Execution context of the isolated world.
    pub execution_context_id: super::runtime::ExecutionContextId,
}

/// Parameters for `Page.deleteCookie`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCookieParams {
    /// Name of the cookie to remove.
    pub cookie_name: String,
    /// URL to match cooke domain and path.
    pub url: String,
}

/// Parameters for `Page.enable`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EnableParams {
    /// If true, the `Page.fileChooserOpened` event will be emitted regardless of the state set by
    /// `Page.setInterceptFileChooserDialog` command (default: false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable_file_chooser_opened_event: Option<bool>,
}

/// Parameters for `Page.getAppManifest`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetAppManifestParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_id: Option<String>,
}

/// Return type for `Page.getAppManifest`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAppManifestReturns {
    /// Manifest location.
    pub url: String,
    pub errors: Vec<AppManifestError>,
    /// Manifest content.
    #[serde(default)]
    pub data: Option<String>,
    /// Parsed manifest properties. Deprecated, use manifest instead.
    #[serde(default)]
    pub parsed: Option<AppManifestParsedProperties>,
    pub manifest: WebAppManifest,
}

/// Return type for `Page.getInstallabilityErrors`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetInstallabilityErrorsReturns {
    pub installability_errors: Vec<InstallabilityError>,
}

/// Return type for `Page.getManifestIcons`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetManifestIconsReturns {
    #[serde(default)]
    pub primary_icon: Option<String>,
}

/// Return type for `Page.getAppId`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAppIdReturns {
    /// App id, either from manifest's id attribute or computed from start_url
    #[serde(default)]
    pub app_id: Option<String>,
    /// Recommendation for manifest's id attribute to match current id computed from start_url
    #[serde(default)]
    pub recommended_id: Option<String>,
}

/// Parameters for `Page.getAdScriptAncestry`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAdScriptAncestryParams {
    pub frame_id: FrameId,
}

/// Return type for `Page.getAdScriptAncestry`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAdScriptAncestryReturns {
    /// The ancestry chain of ad script identifiers leading to this frame's
    /// creation, along with the root script's filterlist rule. The ancestry
    /// chain is ordered from the most immediate script (in the frame creation
    /// stack) to more distant ancestors (that created the immediately preceding
    /// script). Only sent if frame is labelled as an ad and ids are available.
    #[serde(default)]
    pub ad_script_ancestry: Option<AdScriptAncestry>,
}

/// Return type for `Page.getFrameTree`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFrameTreeReturns {
    /// Present frame tree structure.
    pub frame_tree: FrameTree,
}

/// Return type for `Page.getLayoutMetrics`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetLayoutMetricsReturns {
    /// Deprecated metrics relating to the layout viewport. Is in device pixels. Use `cssLayoutViewport` instead.
    pub layout_viewport: LayoutViewport,
    /// Deprecated metrics relating to the visual viewport. Is in device pixels. Use `cssVisualViewport` instead.
    pub visual_viewport: VisualViewport,
    /// Deprecated size of scrollable area. Is in DP. Use `cssContentSize` instead.
    pub content_size: super::dom::Rect,
    /// Metrics relating to the layout viewport in CSS pixels.
    pub css_layout_viewport: LayoutViewport,
    /// Metrics relating to the visual viewport in CSS pixels.
    pub css_visual_viewport: VisualViewport,
    /// Size of scrollable area in CSS pixels.
    pub css_content_size: super::dom::Rect,
}

/// Return type for `Page.getNavigationHistory`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNavigationHistoryReturns {
    /// Index of the current navigation history entry.
    pub current_index: i64,
    /// Array of navigation history entries.
    pub entries: Vec<NavigationEntry>,
}

/// Parameters for `Page.getResourceContent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResourceContentParams {
    /// Frame id to get resource for.
    pub frame_id: FrameId,
    /// URL of the resource to get content for.
    pub url: String,
}

/// Return type for `Page.getResourceContent`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResourceContentReturns {
    /// Resource content.
    pub content: String,
    /// True, if content was served as base64.
    pub base64_encoded: bool,
}

/// Return type for `Page.getResourceTree`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResourceTreeReturns {
    /// Present frame / resource tree structure.
    pub frame_tree: FrameResourceTree,
}

/// Parameters for `Page.handleJavaScriptDialog`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct HandleJavaScriptDialogParams {
    /// Whether to accept or dismiss the dialog.
    pub accept: bool,
    /// The text to enter into the dialog prompt before accepting. Used only if this is a prompt
    /// dialog.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_text: Option<String>,
}

/// Parameters for `Page.navigate`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NavigateParams {
    /// URL to navigate the page to.
    pub url: String,
    /// Referrer URL.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer: Option<String>,
    /// Intended transition type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transition_type: Option<TransitionType>,
    /// Frame id to navigate, if not specified navigates the top frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<FrameId>,
    /// Referrer-policy used for the navigation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub referrer_policy: Option<ReferrerPolicy>,
}

/// Return type for `Page.navigate`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigateReturns {
    /// Frame id that has navigated (or failed to navigate)
    pub frame_id: FrameId,
    /// Loader identifier. This is omitted in case of same-document navigation,
    /// as the previously committed loaderId would not change.
    #[serde(default)]
    pub loader_id: Option<super::network::LoaderId>,
    /// User friendly error message, present if and only if navigation has failed.
    #[serde(default)]
    pub error_text: Option<String>,
    /// Whether the navigation resulted in a download.
    #[serde(default)]
    pub is_download: Option<bool>,
}

/// Parameters for `Page.navigateToHistoryEntry`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NavigateToHistoryEntryParams {
    /// Unique id of the entry to navigate to.
    pub entry_id: i64,
}

/// Parameters for `Page.printToPDF`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PrintToPDFParams {
    /// Paper orientation. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub landscape: Option<bool>,
    /// Display header and footer. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_header_footer: Option<bool>,
    /// Print background graphics. Defaults to false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub print_background: Option<bool>,
    /// Scale of the webpage rendering. Defaults to 1.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// Paper width in inches. Defaults to 8.5 inches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_width: Option<f64>,
    /// Paper height in inches. Defaults to 11 inches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub paper_height: Option<f64>,
    /// Top margin in inches. Defaults to 1cm (~0.4 inches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_top: Option<f64>,
    /// Bottom margin in inches. Defaults to 1cm (~0.4 inches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_bottom: Option<f64>,
    /// Left margin in inches. Defaults to 1cm (~0.4 inches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_left: Option<f64>,
    /// Right margin in inches. Defaults to 1cm (~0.4 inches).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub margin_right: Option<f64>,
    /// Paper ranges to print, one based, e.g., '1-5, 8, 11-13'. Pages are
    /// printed in the document order, not in the order specified, and no
    /// more than once.
    /// Defaults to empty string, which implies the entire document is printed.
    /// The page numbers are quietly capped to actual page count of the
    /// document, and ranges beyond the end of the document are ignored.
    /// If this results in no pages to print, an error is reported.
    /// It is an error to specify a range with start greater than end.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_ranges: Option<String>,
    /// HTML template for the print header. Should be valid HTML markup with following
    /// classes used to inject printing values into them:
    /// - `date`: formatted print date
    /// - `title`: document title
    /// - `url`: document location
    /// - `pageNumber`: current page number
    /// - `totalPages`: total pages in the document
    /// 
    /// For example, `<span class=title></span>` would generate span containing the title.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_template: Option<String>,
    /// HTML template for the print footer. Should use the same format as the `headerTemplate`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_template: Option<String>,
    /// Whether or not to prefer page size as defined by css. Defaults to false,
    /// in which case the content will be scaled to fit the paper size.
    #[serde(rename = "preferCSSPageSize")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefer_css_page_size: Option<bool>,
    /// return as stream
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transfer_mode: Option<String>,
    /// Whether or not to generate tagged (accessible) PDF. Defaults to embedder choice.
    #[serde(rename = "generateTaggedPDF")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_tagged_pdf: Option<bool>,
    /// Whether or not to embed the document outline into the PDF.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_document_outline: Option<bool>,
}

/// Return type for `Page.printToPDF`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintToPDFReturns {
    /// Base64-encoded pdf data. Empty if |returnAsStream| is specified.
    pub data: String,
    /// A handle of the stream that holds resulting PDF data.
    #[serde(default)]
    pub stream: Option<Value>,
}

/// Parameters for `Page.reload`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReloadParams {
    /// If true, browser cache is ignored (as if the user pressed Shift+refresh).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_cache: Option<bool>,
    /// If set, the script will be injected into all frames of the inspected page after reload.
    /// Argument will be ignored if reloading dataURL origin.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub script_to_evaluate_on_load: Option<String>,
    /// If set, an error will be thrown if the target page's main frame's
    /// loader id does not match the provided id. This prevents accidentally
    /// reloading an unintended target in case there's a racing navigation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_id: Option<super::network::LoaderId>,
}

/// Parameters for `Page.removeScriptToEvaluateOnLoad`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveScriptToEvaluateOnLoadParams {
    pub identifier: ScriptIdentifier,
}

/// Parameters for `Page.removeScriptToEvaluateOnNewDocument`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveScriptToEvaluateOnNewDocumentParams {
    pub identifier: ScriptIdentifier,
}

/// Parameters for `Page.screencastFrameAck`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastFrameAckParams {
    /// Frame number.
    pub session_id: i64,
}

/// Parameters for `Page.searchInResource`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInResourceParams {
    /// Frame id for resource to search in.
    pub frame_id: FrameId,
    /// URL of the resource to search in.
    pub url: String,
    /// String to search for.
    pub query: String,
    /// If true, search is case sensitive.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub case_sensitive: Option<bool>,
    /// If true, treats string parameter as regex.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_regex: Option<bool>,
}

/// Return type for `Page.searchInResource`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInResourceReturns {
    /// List of search matches.
    pub result: Vec<Value>,
}

/// Parameters for `Page.setAdBlockingEnabled`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetAdBlockingEnabledParams {
    /// Whether to block ads.
    pub enabled: bool,
}

/// Parameters for `Page.setBypassCSP`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetBypassCSPParams {
    /// Whether to bypass page CSP.
    pub enabled: bool,
}

/// Parameters for `Page.getPermissionsPolicyState`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPermissionsPolicyStateParams {
    pub frame_id: FrameId,
}

/// Return type for `Page.getPermissionsPolicyState`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPermissionsPolicyStateReturns {
    pub states: Vec<PermissionsPolicyFeatureState>,
}

/// Parameters for `Page.getOriginTrials`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOriginTrialsParams {
    pub frame_id: FrameId,
}

/// Return type for `Page.getOriginTrials`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOriginTrialsReturns {
    pub origin_trials: Vec<OriginTrial>,
}

/// Parameters for `Page.setDeviceMetricsOverride`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetDeviceMetricsOverrideParams {
    /// Overriding width value in pixels (minimum 0, maximum 10000000). 0 disables the override.
    pub width: i64,
    /// Overriding height value in pixels (minimum 0, maximum 10000000). 0 disables the override.
    pub height: i64,
    /// Overriding device scale factor value. 0 disables the override.
    pub device_scale_factor: f64,
    /// Whether to emulate mobile device. This includes viewport meta tag, overlay scrollbars, text
    /// autosizing and more.
    pub mobile: bool,
    /// Scale to apply to resulting view image.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// Overriding screen width value in pixels (minimum 0, maximum 10000000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_width: Option<i64>,
    /// Overriding screen height value in pixels (minimum 0, maximum 10000000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_height: Option<i64>,
    /// Overriding view X position on screen in pixels (minimum 0, maximum 10000000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_x: Option<i64>,
    /// Overriding view Y position on screen in pixels (minimum 0, maximum 10000000).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position_y: Option<i64>,
    /// Do not set visible view size, rely upon explicit setVisibleSize call.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dont_set_visible_size: Option<bool>,
    /// Screen orientation override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screen_orientation: Option<Value>,
    /// The viewport dimensions and scale. If not set, the override is cleared.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub viewport: Option<Viewport>,
}

/// Parameters for `Page.setDeviceOrientationOverride`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetDeviceOrientationOverrideParams {
    /// Mock alpha
    pub alpha: f64,
    /// Mock beta
    pub beta: f64,
    /// Mock gamma
    pub gamma: f64,
}

/// Parameters for `Page.setFontFamilies`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFontFamiliesParams {
    /// Specifies font families to set. If a font family is not specified, it won't be changed.
    pub font_families: FontFamilies,
    /// Specifies font families to set for individual scripts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub for_scripts: Option<Vec<ScriptFontFamilies>>,
}

/// Parameters for `Page.setFontSizes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFontSizesParams {
    /// Specifies font sizes to set. If a font size is not specified, it won't be changed.
    pub font_sizes: FontSizes,
}

/// Parameters for `Page.setDocumentContent`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDocumentContentParams {
    /// Frame id to set HTML for.
    pub frame_id: FrameId,
    /// HTML content to set.
    pub html: String,
}

/// Parameters for `Page.setDownloadBehavior`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetDownloadBehaviorParams {
    /// Whether to allow all or deny all download requests, or use default Chrome behavior if
    /// available (otherwise deny).
    pub behavior: String,
    /// The default path to save downloaded files to. This is required if behavior is set to 'allow'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub download_path: Option<String>,
}

/// Parameters for `Page.setGeolocationOverride`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetGeolocationOverrideParams {
    /// Mock latitude
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latitude: Option<f64>,
    /// Mock longitude
    #[serde(skip_serializing_if = "Option::is_none")]
    pub longitude: Option<f64>,
    /// Mock accuracy
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accuracy: Option<f64>,
}

/// Parameters for `Page.setLifecycleEventsEnabled`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetLifecycleEventsEnabledParams {
    /// If true, starts emitting lifecycle events.
    pub enabled: bool,
}

/// Parameters for `Page.setTouchEmulationEnabled`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetTouchEmulationEnabledParams {
    /// Whether the touch event emulation should be enabled.
    pub enabled: bool,
    /// Touch/gesture events configuration. Default: current platform.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub configuration: Option<String>,
}

/// Parameters for `Page.startScreencast`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StartScreencastParams {
    /// Image compression format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Compression quality from range \[0..100\].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<i64>,
    /// Maximum screenshot width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_width: Option<i64>,
    /// Maximum screenshot height.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_height: Option<i64>,
    /// Send every n-th frame.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub every_nth_frame: Option<i64>,
}

/// Parameters for `Page.setWebLifecycleState`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetWebLifecycleStateParams {
    /// Target lifecycle state
    pub state: String,
}

/// Parameters for `Page.produceCompilationCache`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ProduceCompilationCacheParams {
    pub scripts: Vec<CompilationCacheParams>,
}

/// Parameters for `Page.addCompilationCache`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AddCompilationCacheParams {
    pub url: String,
    /// Base64-encoded data
    pub data: String,
}

/// Parameters for `Page.setSPCTransactionMode`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetSPCTransactionModeParams {
    pub mode: String,
}

/// Parameters for `Page.setRPHRegistrationMode`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetRPHRegistrationModeParams {
    pub mode: String,
}

/// Parameters for `Page.generateTestReport`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GenerateTestReportParams {
    /// Message to be displayed in the report.
    pub message: String,
    /// Specifies the endpoint group to deliver the report to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
}

/// Parameters for `Page.setInterceptFileChooserDialog`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetInterceptFileChooserDialogParams {
    pub enabled: bool,
    /// If true, cancels the dialog by emitting relevant events (if any)
    /// in addition to not showing it if the interception is enabled
    /// (default: false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel: Option<bool>,
}

/// Parameters for `Page.setPrerenderingAllowed`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetPrerenderingAllowedParams {
    pub is_allowed: bool,
}

/// Parameters for `Page.getAnnotatedPageContent`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetAnnotatedPageContentParams {
    /// Whether to include actionable information. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_actionable_information: Option<bool>,
}

/// Return type for `Page.getAnnotatedPageContent`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAnnotatedPageContentReturns {
    /// The annotated page content as a base64 encoded protobuf.
    /// The format is defined by the `AnnotatedPageContent` message in
    /// components/optimization_guide/proto/features/common_quality_data.proto
    pub content: String,
}

/// Event payload for `Page.domContentEventFired`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomContentEventFiredEvent {
    pub timestamp: super::network::MonotonicTime,
}

/// Event payload for `Page.fileChooserOpened`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChooserOpenedEvent {
    /// Id of the frame containing input node.
    pub frame_id: FrameId,
    /// Input mode.
    pub mode: String,
    /// Input node id. Only present for file choosers opened via an `<input type="file">` element.
    #[serde(default)]
    pub backend_node_id: Option<super::dom::BackendNodeId>,
}

/// Event payload for `Page.frameAttached`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameAttachedEvent {
    /// Id of the frame that has been attached.
    pub frame_id: FrameId,
    /// Parent frame identifier.
    pub parent_frame_id: FrameId,
    /// JavaScript stack trace of when frame was attached, only set if frame initiated from script.
    #[serde(default)]
    pub stack: Option<super::runtime::StackTrace>,
}

/// Event payload for `Page.frameClearedScheduledNavigation`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameClearedScheduledNavigationEvent {
    /// Id of the frame that has cleared its scheduled navigation.
    pub frame_id: FrameId,
}

/// Event payload for `Page.frameDetached`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameDetachedEvent {
    /// Id of the frame that has been detached.
    pub frame_id: FrameId,
    pub reason: String,
}

/// Event payload for `Page.frameSubtreeWillBeDetached`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameSubtreeWillBeDetachedEvent {
    /// Id of the frame that is the root of the subtree that will be detached.
    pub frame_id: FrameId,
}

/// Event payload for `Page.frameNavigated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameNavigatedEvent {
    /// Frame object.
    pub frame: Frame,
    #[serde(rename = "type")]
    pub r#type: NavigationType,
}

/// Event payload for `Page.documentOpened`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentOpenedEvent {
    /// Frame object.
    pub frame: Frame,
}

/// Event payload for `Page.frameStartedNavigating`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStartedNavigatingEvent {
    /// ID of the frame that is being navigated.
    pub frame_id: FrameId,
    /// The URL the navigation started with. The final URL can be different.
    pub url: String,
    /// Loader identifier. Even though it is present in case of same-document
    /// navigation, the previously committed loaderId would not change unless
    /// the navigation changes from a same-document to a cross-document
    /// navigation.
    pub loader_id: super::network::LoaderId,
    pub navigation_type: String,
}

/// Event payload for `Page.frameRequestedNavigation`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameRequestedNavigationEvent {
    /// Id of the frame that is being navigated.
    pub frame_id: FrameId,
    /// The reason for the navigation.
    pub reason: ClientNavigationReason,
    /// The destination URL for the requested navigation.
    pub url: String,
    /// The disposition for the navigation.
    pub disposition: ClientNavigationDisposition,
}

/// Event payload for `Page.frameScheduledNavigation`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameScheduledNavigationEvent {
    /// Id of the frame that has scheduled a navigation.
    pub frame_id: FrameId,
    /// Delay (in seconds) until the navigation is scheduled to begin. The navigation is not
    /// guaranteed to start.
    pub delay: f64,
    /// The reason for the navigation.
    pub reason: ClientNavigationReason,
    /// The destination URL for the scheduled navigation.
    pub url: String,
}

/// Event payload for `Page.frameStartedLoading`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStartedLoadingEvent {
    /// Id of the frame that has started loading.
    pub frame_id: FrameId,
}

/// Event payload for `Page.frameStoppedLoading`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameStoppedLoadingEvent {
    /// Id of the frame that has stopped loading.
    pub frame_id: FrameId,
}

/// Event payload for `Page.downloadWillBegin`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadWillBeginEvent {
    /// Id of the frame that caused download to begin.
    pub frame_id: FrameId,
    /// Global unique identifier of the download.
    pub guid: String,
    /// URL of the resource being downloaded.
    pub url: String,
    /// Suggested file name of the resource (the actual name of the file saved on disk may differ).
    pub suggested_filename: String,
}

/// Event payload for `Page.downloadProgress`.
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
}

/// Event payload for `Page.javascriptDialogClosed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavascriptDialogClosedEvent {
    /// Frame id.
    pub frame_id: FrameId,
    /// Whether dialog was confirmed.
    pub result: bool,
    /// User input in case of prompt.
    pub user_input: String,
}

/// Event payload for `Page.javascriptDialogOpening`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavascriptDialogOpeningEvent {
    /// Frame url.
    pub url: String,
    /// Frame id.
    pub frame_id: FrameId,
    /// Message that will be displayed by the dialog.
    pub message: String,
    /// Dialog type.
    #[serde(rename = "type")]
    pub r#type: DialogType,
    /// True iff browser is capable showing or acting on the given dialog. When browser has no
    /// dialog handler for given target, calling alert while Page domain is engaged will stall
    /// the page execution. Execution can be resumed via calling Page.handleJavaScriptDialog.
    pub has_browser_handler: bool,
    /// Default dialog prompt.
    #[serde(default)]
    pub default_prompt: Option<String>,
}

/// Event payload for `Page.lifecycleEvent`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LifecycleEventEvent {
    /// Id of the frame.
    pub frame_id: FrameId,
    /// Loader identifier. Empty string if the request is fetched from worker.
    pub loader_id: super::network::LoaderId,
    pub name: String,
    pub timestamp: super::network::MonotonicTime,
}

/// Event payload for `Page.backForwardCacheNotUsed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackForwardCacheNotUsedEvent {
    /// The loader id for the associated navigation.
    pub loader_id: super::network::LoaderId,
    /// The frame id of the associated frame.
    pub frame_id: FrameId,
    /// Array of reasons why the page could not be cached. This must not be empty.
    pub not_restored_explanations: Vec<BackForwardCacheNotRestoredExplanation>,
    /// Tree structure of reasons why the page could not be cached for each frame.
    #[serde(default)]
    pub not_restored_explanations_tree: Option<BackForwardCacheNotRestoredExplanationTree>,
}

/// Event payload for `Page.loadEventFired`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadEventFiredEvent {
    pub timestamp: super::network::MonotonicTime,
}

/// Event payload for `Page.navigatedWithinDocument`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NavigatedWithinDocumentEvent {
    /// Id of the frame.
    pub frame_id: FrameId,
    /// Frame's new url.
    pub url: String,
    /// Navigation type
    pub navigation_type: String,
}

/// Event payload for `Page.screencastFrame`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastFrameEvent {
    /// Base64-encoded compressed image.
    pub data: String,
    /// Screencast frame metadata.
    pub metadata: ScreencastFrameMetadata,
    /// Frame number.
    pub session_id: i64,
}

/// Event payload for `Page.screencastVisibilityChanged`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreencastVisibilityChangedEvent {
    /// True if the page is visible.
    pub visible: bool,
}

/// Event payload for `Page.windowOpen`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowOpenEvent {
    /// The URL for the new window.
    pub url: String,
    /// Window name.
    pub window_name: String,
    /// An array of enabled window features.
    pub window_features: Vec<String>,
    /// Whether or not it was triggered by user gesture.
    pub user_gesture: bool,
}

/// Event payload for `Page.compilationCacheProduced`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompilationCacheProducedEvent {
    pub url: String,
    /// Base64-encoded data
    pub data: String,
}

// ── Methods ──
//
// These are the typed method signatures for Page.* commands.
// Integration into CdpSession is done in pwright-cdp.

// Deprecated, please use addScriptToEvaluateOnNewDocument instead.
// async fn page_add_script_to_evaluate_on_load(&self, params: AddScriptToEvaluateOnLoadParams) -> Result<AddScriptToEvaluateOnLoadReturns>
// CDP: "Page.addScriptToEvaluateOnLoad"

// Evaluates given script in every frame upon creation (before loading frame's scripts).
// async fn page_add_script_to_evaluate_on_new_document(&self, params: AddScriptToEvaluateOnNewDocumentParams) -> Result<AddScriptToEvaluateOnNewDocumentReturns>
// CDP: "Page.addScriptToEvaluateOnNewDocument"

// Brings page to front (activates tab).
// async fn page_bring_to_front(&self) -> Result<()>
// CDP: "Page.bringToFront"

// Capture page screenshot.
// async fn page_capture_screenshot(&self, params: CaptureScreenshotParams) -> Result<CaptureScreenshotReturns>
// CDP: "Page.captureScreenshot"

// Returns a snapshot of the page as a string. For MHTML format, the serialization includes
// iframes, shadow DOM, external resources, and element-inline styles.
// async fn page_capture_snapshot(&self, params: CaptureSnapshotParams) -> Result<CaptureSnapshotReturns>
// CDP: "Page.captureSnapshot"

// Clears the overridden device metrics.
// async fn page_clear_device_metrics_override(&self) -> Result<()>
// CDP: "Page.clearDeviceMetricsOverride"

// Clears the overridden Device Orientation.
// async fn page_clear_device_orientation_override(&self) -> Result<()>
// CDP: "Page.clearDeviceOrientationOverride"

// Clears the overridden Geolocation Position and Error.
// async fn page_clear_geolocation_override(&self) -> Result<()>
// CDP: "Page.clearGeolocationOverride"

// Creates an isolated world for the given frame.
// async fn page_create_isolated_world(&self, params: CreateIsolatedWorldParams) -> Result<CreateIsolatedWorldReturns>
// CDP: "Page.createIsolatedWorld"

// Deletes browser cookie with given name, domain and path.
// async fn page_delete_cookie(&self, params: DeleteCookieParams) -> Result<()>
// CDP: "Page.deleteCookie"

// Disables page domain notifications.
// async fn page_disable(&self) -> Result<()>
// CDP: "Page.disable"

// Enables page domain notifications.
// async fn page_enable(&self, params: EnableParams) -> Result<()>
// CDP: "Page.enable"

// Gets the processed manifest for this current document.
//   This API always waits for the manifest to be loaded.
//   If manifestId is provided, and it does not match the manifest of the
//     current document, this API errors out.
//   If there is not a loaded page, this API errors out immediately.
// async fn page_get_app_manifest(&self, params: GetAppManifestParams) -> Result<GetAppManifestReturns>
// CDP: "Page.getAppManifest"

// async fn page_get_installability_errors(&self) -> Result<GetInstallabilityErrorsReturns>
// CDP: "Page.getInstallabilityErrors"

// Deprecated because it's not guaranteed that the returned icon is in fact the one used for PWA installation.
// async fn page_get_manifest_icons(&self) -> Result<GetManifestIconsReturns>
// CDP: "Page.getManifestIcons"

// Returns the unique (PWA) app id.
// Only returns values if the feature flag 'WebAppEnableManifestId' is enabled
// async fn page_get_app_id(&self) -> Result<GetAppIdReturns>
// CDP: "Page.getAppId"

// async fn page_get_ad_script_ancestry(&self, params: GetAdScriptAncestryParams) -> Result<GetAdScriptAncestryReturns>
// CDP: "Page.getAdScriptAncestry"

// Returns present frame tree structure.
// async fn page_get_frame_tree(&self) -> Result<GetFrameTreeReturns>
// CDP: "Page.getFrameTree"

// Returns metrics relating to the layouting of the page, such as viewport bounds/scale.
// async fn page_get_layout_metrics(&self) -> Result<GetLayoutMetricsReturns>
// CDP: "Page.getLayoutMetrics"

// Returns navigation history for the current page.
// async fn page_get_navigation_history(&self) -> Result<GetNavigationHistoryReturns>
// CDP: "Page.getNavigationHistory"

// Resets navigation history for the current page.
// async fn page_reset_navigation_history(&self) -> Result<()>
// CDP: "Page.resetNavigationHistory"

// Returns content of the given resource.
// async fn page_get_resource_content(&self, params: GetResourceContentParams) -> Result<GetResourceContentReturns>
// CDP: "Page.getResourceContent"

// Returns present frame / resource tree structure.
// async fn page_get_resource_tree(&self) -> Result<GetResourceTreeReturns>
// CDP: "Page.getResourceTree"

// Accepts or dismisses a JavaScript initiated dialog (alert, confirm, prompt, or onbeforeunload).
// async fn page_handle_java_script_dialog(&self, params: HandleJavaScriptDialogParams) -> Result<()>
// CDP: "Page.handleJavaScriptDialog"

// Navigates current page to the given URL.
// async fn page_navigate(&self, params: NavigateParams) -> Result<NavigateReturns>
// CDP: "Page.navigate"

// Navigates current page to the given history entry.
// async fn page_navigate_to_history_entry(&self, params: NavigateToHistoryEntryParams) -> Result<()>
// CDP: "Page.navigateToHistoryEntry"

// Print page as PDF.
// async fn page_print_to_pdf(&self, params: PrintToPDFParams) -> Result<PrintToPDFReturns>
// CDP: "Page.printToPDF"

// Reloads given page optionally ignoring the cache.
// async fn page_reload(&self, params: ReloadParams) -> Result<()>
// CDP: "Page.reload"

// Deprecated, please use removeScriptToEvaluateOnNewDocument instead.
// async fn page_remove_script_to_evaluate_on_load(&self, params: RemoveScriptToEvaluateOnLoadParams) -> Result<()>
// CDP: "Page.removeScriptToEvaluateOnLoad"

// Removes given script from the list.
// async fn page_remove_script_to_evaluate_on_new_document(&self, params: RemoveScriptToEvaluateOnNewDocumentParams) -> Result<()>
// CDP: "Page.removeScriptToEvaluateOnNewDocument"

// Acknowledges that a screencast frame has been received by the frontend.
// async fn page_screencast_frame_ack(&self, params: ScreencastFrameAckParams) -> Result<()>
// CDP: "Page.screencastFrameAck"

// Searches for given string in resource content.
// async fn page_search_in_resource(&self, params: SearchInResourceParams) -> Result<SearchInResourceReturns>
// CDP: "Page.searchInResource"

// Enable Chrome's experimental ad filter on all sites.
// async fn page_set_ad_blocking_enabled(&self, params: SetAdBlockingEnabledParams) -> Result<()>
// CDP: "Page.setAdBlockingEnabled"

// Enable page Content Security Policy by-passing.
// async fn page_set_bypass_csp(&self, params: SetBypassCSPParams) -> Result<()>
// CDP: "Page.setBypassCSP"

// Get Permissions Policy state on given frame.
// async fn page_get_permissions_policy_state(&self, params: GetPermissionsPolicyStateParams) -> Result<GetPermissionsPolicyStateReturns>
// CDP: "Page.getPermissionsPolicyState"

// Get Origin Trials on given frame.
// async fn page_get_origin_trials(&self, params: GetOriginTrialsParams) -> Result<GetOriginTrialsReturns>
// CDP: "Page.getOriginTrials"

// Overrides the values of device screen dimensions (window.screen.width, window.screen.height,
// window.innerWidth, window.innerHeight, and "device-width"/"device-height"-related CSS media
// query results).
// async fn page_set_device_metrics_override(&self, params: SetDeviceMetricsOverrideParams) -> Result<()>
// CDP: "Page.setDeviceMetricsOverride"

// Overrides the Device Orientation.
// async fn page_set_device_orientation_override(&self, params: SetDeviceOrientationOverrideParams) -> Result<()>
// CDP: "Page.setDeviceOrientationOverride"

// Set generic font families.
// async fn page_set_font_families(&self, params: SetFontFamiliesParams) -> Result<()>
// CDP: "Page.setFontFamilies"

// Set default font sizes.
// async fn page_set_font_sizes(&self, params: SetFontSizesParams) -> Result<()>
// CDP: "Page.setFontSizes"

// Sets given markup as the document's HTML.
// async fn page_set_document_content(&self, params: SetDocumentContentParams) -> Result<()>
// CDP: "Page.setDocumentContent"

// Set the behavior when downloading a file.
// async fn page_set_download_behavior(&self, params: SetDownloadBehaviorParams) -> Result<()>
// CDP: "Page.setDownloadBehavior"

// Overrides the Geolocation Position or Error. Omitting any of the parameters emulates position
// unavailable.
// async fn page_set_geolocation_override(&self, params: SetGeolocationOverrideParams) -> Result<()>
// CDP: "Page.setGeolocationOverride"

// Controls whether page will emit lifecycle events.
// async fn page_set_lifecycle_events_enabled(&self, params: SetLifecycleEventsEnabledParams) -> Result<()>
// CDP: "Page.setLifecycleEventsEnabled"

// Toggles mouse event-based touch event emulation.
// async fn page_set_touch_emulation_enabled(&self, params: SetTouchEmulationEnabledParams) -> Result<()>
// CDP: "Page.setTouchEmulationEnabled"

// Starts sending each frame using the `screencastFrame` event.
// async fn page_start_screencast(&self, params: StartScreencastParams) -> Result<()>
// CDP: "Page.startScreencast"

// Force the page stop all navigations and pending resource fetches.
// async fn page_stop_loading(&self) -> Result<()>
// CDP: "Page.stopLoading"

// Crashes renderer on the IO thread, generates minidumps.
// async fn page_crash(&self) -> Result<()>
// CDP: "Page.crash"

// Tries to close page, running its beforeunload hooks, if any.
// async fn page_close(&self) -> Result<()>
// CDP: "Page.close"

// Tries to update the web lifecycle state of the page.
// It will transition the page to the given state according to:
// https://github.com/WICG/web-lifecycle/
// async fn page_set_web_lifecycle_state(&self, params: SetWebLifecycleStateParams) -> Result<()>
// CDP: "Page.setWebLifecycleState"

// Stops sending each frame in the `screencastFrame`.
// async fn page_stop_screencast(&self) -> Result<()>
// CDP: "Page.stopScreencast"

// Requests backend to produce compilation cache for the specified scripts.
// `scripts` are appended to the list of scripts for which the cache
// would be produced. The list may be reset during page navigation.
// When script with a matching URL is encountered, the cache is optionally
// produced upon backend discretion, based on internal heuristics.
// See also: `Page.compilationCacheProduced`.
// async fn page_produce_compilation_cache(&self, params: ProduceCompilationCacheParams) -> Result<()>
// CDP: "Page.produceCompilationCache"

// Seeds compilation cache for given url. Compilation cache does not survive
// cross-process navigation.
// async fn page_add_compilation_cache(&self, params: AddCompilationCacheParams) -> Result<()>
// CDP: "Page.addCompilationCache"

// Clears seeded compilation cache.
// async fn page_clear_compilation_cache(&self) -> Result<()>
// CDP: "Page.clearCompilationCache"

// Sets the Secure Payment Confirmation transaction mode.
// https://w3c.github.io/secure-payment-confirmation/#sctn-automation-set-spc-transaction-mode
// async fn page_set_spc_transaction_mode(&self, params: SetSPCTransactionModeParams) -> Result<()>
// CDP: "Page.setSPCTransactionMode"

// Extensions for Custom Handlers API:
// https://html.spec.whatwg.org/multipage/system-state.html#rph-automation
// async fn page_set_rph_registration_mode(&self, params: SetRPHRegistrationModeParams) -> Result<()>
// CDP: "Page.setRPHRegistrationMode"

// Generates a report for testing.
// async fn page_generate_test_report(&self, params: GenerateTestReportParams) -> Result<()>
// CDP: "Page.generateTestReport"

// Pauses page execution. Can be resumed using generic Runtime.runIfWaitingForDebugger.
// async fn page_wait_for_debugger(&self) -> Result<()>
// CDP: "Page.waitForDebugger"

// Intercept file chooser requests and transfer control to protocol clients.
// When file chooser interception is enabled, native file chooser dialog is not shown.
// Instead, a protocol event `Page.fileChooserOpened` is emitted.
// async fn page_set_intercept_file_chooser_dialog(&self, params: SetInterceptFileChooserDialogParams) -> Result<()>
// CDP: "Page.setInterceptFileChooserDialog"

// Enable/disable prerendering manually.
// 
// This command is a short-term solution for https://crbug.com/1440085.
// See https://docs.google.com/document/d/12HVmFxYj5Jc-eJr5OmWsa2bqTJsbgGLKI6ZIyx0_wpA
// for more details.
// 
// TODO(https://crbug.com/1440085): Remove this once Puppeteer supports tab targets.
// async fn page_set_prerendering_allowed(&self, params: SetPrerenderingAllowedParams) -> Result<()>
// CDP: "Page.setPrerenderingAllowed"

// Get the annotated page content for the main frame.
// This is an experimental command that is subject to change.
// async fn page_get_annotated_page_content(&self, params: GetAnnotatedPageContentParams) -> Result<GetAnnotatedPageContentReturns>
// CDP: "Page.getAnnotatedPageContent"

