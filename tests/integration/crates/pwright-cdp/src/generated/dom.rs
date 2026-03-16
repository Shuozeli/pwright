//! CDP `DOM` domain — generated from protocol JSON.
//!
//! This domain exposes DOM read/write operations. Each DOM Node is represented with its mirror object
//! that has an `id`. This `id` can be used to get additional information on the Node, resolve it into
//! the JavaScript object wrapper, etc. It is important that client receives DOM events only for the
//! nodes that are known to the client. Backend keeps track of the nodes that were sent to the client
//! and never sends the same node twice. It is client's responsibility to collect information about
//! the nodes that were sent to the client. Note that `iframe` owner elements will return
//! corresponding document elements as their child nodes.

#![allow(clippy::doc_markdown)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Unique DOM node identifier.
pub type NodeId = i64;

/// Unique DOM node identifier used to reference a node that may not have been pushed to the
/// front-end.
pub type BackendNodeId = i64;

/// Unique identifier for a CSS stylesheet.
pub type StyleSheetId = String;

/// Backend node with a friendly name.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackendNode {
    /// `Node`'s nodeType.
    pub node_type: i64,
    /// `Node`'s nodeName.
    pub node_name: String,
    pub backend_node_id: BackendNodeId,
}

/// Pseudo element type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PseudoType {
    #[serde(rename = "first-line")]
    FirstLine,
    #[serde(rename = "first-letter")]
    FirstLetter,
    #[serde(rename = "checkmark")]
    Checkmark,
    #[serde(rename = "before")]
    Before,
    #[serde(rename = "after")]
    After,
    #[serde(rename = "picker-icon")]
    PickerIcon,
    #[serde(rename = "interest-hint")]
    InterestHint,
    #[serde(rename = "marker")]
    Marker,
    #[serde(rename = "backdrop")]
    Backdrop,
    #[serde(rename = "column")]
    Column,
    #[serde(rename = "selection")]
    Selection,
    #[serde(rename = "search-text")]
    SearchText,
    #[serde(rename = "target-text")]
    TargetText,
    #[serde(rename = "spelling-error")]
    SpellingError,
    #[serde(rename = "grammar-error")]
    GrammarError,
    #[serde(rename = "highlight")]
    Highlight,
    #[serde(rename = "first-line-inherited")]
    FirstLineInherited,
    #[serde(rename = "scroll-marker")]
    ScrollMarker,
    #[serde(rename = "scroll-marker-group")]
    ScrollMarkerGroup,
    #[serde(rename = "scroll-button")]
    ScrollButton,
    #[serde(rename = "scrollbar")]
    Scrollbar,
    #[serde(rename = "scrollbar-thumb")]
    ScrollbarThumb,
    #[serde(rename = "scrollbar-button")]
    ScrollbarButton,
    #[serde(rename = "scrollbar-track")]
    ScrollbarTrack,
    #[serde(rename = "scrollbar-track-piece")]
    ScrollbarTrackPiece,
    #[serde(rename = "scrollbar-corner")]
    ScrollbarCorner,
    #[serde(rename = "resizer")]
    Resizer,
    #[serde(rename = "input-list-button")]
    InputListButton,
    #[serde(rename = "view-transition")]
    ViewTransition,
    #[serde(rename = "view-transition-group")]
    ViewTransitionGroup,
    #[serde(rename = "view-transition-image-pair")]
    ViewTransitionImagePair,
    #[serde(rename = "view-transition-group-children")]
    ViewTransitionGroupChildren,
    #[serde(rename = "view-transition-old")]
    ViewTransitionOld,
    #[serde(rename = "view-transition-new")]
    ViewTransitionNew,
    #[serde(rename = "placeholder")]
    Placeholder,
    #[serde(rename = "file-selector-button")]
    FileSelectorButton,
    #[serde(rename = "details-content")]
    DetailsContent,
    #[serde(rename = "picker")]
    Picker,
    #[serde(rename = "permission-icon")]
    PermissionIcon,
    #[serde(rename = "overscroll-area-parent")]
    OverscrollAreaParent,
    #[serde(rename = "overscroll-client-area")]
    OverscrollClientArea,
}

/// Shadow root type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ShadowRootType {
    #[serde(rename = "user-agent")]
    UserAgent,
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
}

/// Document compatibility mode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompatibilityMode {
    QuirksMode,
    LimitedQuirksMode,
    NoQuirksMode,
}

/// ContainerSelector physical axes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PhysicalAxes {
    Horizontal,
    Vertical,
    Both,
}

/// ContainerSelector logical axes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum LogicalAxes {
    Inline,
    Block,
    Both,
}

/// Physical scroll orientation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScrollOrientation {
    #[serde(rename = "horizontal")]
    Horizontal,
    #[serde(rename = "vertical")]
    Vertical,
}

/// DOM interaction is implemented in terms of mirror objects that represent the actual DOM nodes.
/// DOMNode is a base node mirror type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Node {
    /// Node identifier that is passed into the rest of the DOM messages as the `nodeId`. Backend
    /// will only push node with given `id` once. It is aware of all requested nodes and will only
    /// fire DOM events for nodes known to the client.
    pub node_id: NodeId,
    /// The id of the parent node if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<NodeId>,
    /// The BackendNodeId for this node.
    pub backend_node_id: BackendNodeId,
    /// `Node`'s nodeType.
    pub node_type: i64,
    /// `Node`'s nodeName.
    pub node_name: String,
    /// `Node`'s localName.
    pub local_name: String,
    /// `Node`'s nodeValue.
    pub node_value: String,
    /// Child count for `Container` nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_node_count: Option<i64>,
    /// Child nodes of this node when requested with children.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<Node>>,
    /// Attributes of the `Element` node in the form of flat array `\[name1, value1, name2, value2\]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<String>>,
    /// Document URL that `Document` or `FrameOwner` node points to.
    #[serde(rename = "documentURL")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_url: Option<String>,
    /// Base URL that `Document` or `FrameOwner` node uses for URL completion.
    #[serde(rename = "baseURL")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    /// `DocumentType`'s publicId.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_id: Option<String>,
    /// `DocumentType`'s systemId.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_id: Option<String>,
    /// `DocumentType`'s internalSubset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub internal_subset: Option<String>,
    /// `Document`'s XML version in case of XML documents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub xml_version: Option<String>,
    /// `Attr`'s name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// `Attr`'s value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// Pseudo element type for this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pseudo_type: Option<PseudoType>,
    /// Pseudo element identifier for this node. Only present if there is a
    /// valid pseudoType.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pseudo_identifier: Option<String>,
    /// Shadow root type.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_root_type: Option<ShadowRootType>,
    /// Frame ID for frame owner elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<super::page::FrameId>,
    /// Content document for frame owner elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_document: Option<Box<Node>>,
    /// Shadow root list for given element host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadow_roots: Option<Vec<Node>>,
    /// Content document fragment for template elements.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub template_content: Option<Box<Node>>,
    /// Pseudo elements associated with this node.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pseudo_elements: Option<Vec<Node>>,
    /// Deprecated, as the HTML Imports API has been removed (crbug.com/937746).
    /// This property used to return the imported document for the HTMLImport links.
    /// The property is always undefined now.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imported_document: Option<Box<Node>>,
    /// Distributed nodes for given insertion point.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distributed_nodes: Option<Vec<BackendNode>>,
    /// Whether the node is SVG.
    #[serde(rename = "isSVG")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_svg: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compatibility_mode: Option<CompatibilityMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_slot: Option<BackendNode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_scrollable: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affected_by_starting_styles: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adopted_style_sheets: Option<Vec<StyleSheetId>>,
}

/// A structure to hold the top-level node of a detached tree and an array of its retained descendants.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DetachedElementInfo {
    pub tree_node: Node,
    pub retained_node_ids: Vec<NodeId>,
}

/// A structure holding an RGBA color.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RGBA {
    /// The red component, in the \[0-255\] range.
    pub r: i64,
    /// The green component, in the \[0-255\] range.
    pub g: i64,
    /// The blue component, in the \[0-255\] range.
    pub b: i64,
    /// The alpha component, in the \[0-1\] range (default: 1).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub a: Option<f64>,
}

/// An array of quad vertices, x immediately followed by y for each point, points clock-wise.
pub type Quad = Vec<f64>;

/// Box model.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BoxModel {
    /// Content box
    pub content: Quad,
    /// Padding box
    pub padding: Quad,
    /// Border box
    pub border: Quad,
    /// Margin box
    pub margin: Quad,
    /// Node width
    pub width: i64,
    /// Node height
    pub height: i64,
    /// Shape outside coordinates
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape_outside: Option<ShapeOutsideInfo>,
}

/// CSS Shape Outside details.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShapeOutsideInfo {
    /// Shape bounds
    pub bounds: Quad,
    /// Shape coordinate details
    pub shape: Vec<Value>,
    /// Margin shape bounds
    pub margin_shape: Vec<Value>,
}

/// Rectangle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    /// X coordinate
    pub x: f64,
    /// Y coordinate
    pub y: f64,
    /// Rectangle width
    pub width: f64,
    /// Rectangle height
    pub height: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CSSComputedStyleProperty {
    /// Computed style property name.
    pub name: String,
    /// Computed style property value.
    pub value: String,
}

/// Parameters for `DOM.collectClassNamesFromSubtree`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectClassNamesFromSubtreeParams {
    /// Id of the node to collect class names.
    pub node_id: NodeId,
}

/// Return type for `DOM.collectClassNamesFromSubtree`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectClassNamesFromSubtreeReturns {
    /// Class name list.
    pub class_names: Vec<String>,
}

/// Parameters for `DOM.copyTo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyToParams {
    /// Id of the node to copy.
    pub node_id: NodeId,
    /// Id of the element to drop the copy into.
    pub target_node_id: NodeId,
    /// Drop the copy before this node (if absent, the copy becomes the last child of
    /// `targetNodeId`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_before_node_id: Option<NodeId>,
}

/// Return type for `DOM.copyTo`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CopyToReturns {
    /// Id of the node clone.
    pub node_id: NodeId,
}

/// Parameters for `DOM.describeNode`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DescribeNodeParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<super::runtime::RemoteObjectId>,
    /// The maximum depth at which children should be retrieved, defaults to 1. Use -1 for the
    /// entire subtree or provide an integer larger than 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
    /// Whether or not iframes and shadow roots should be traversed when returning the subtree
    /// (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Return type for `DOM.describeNode`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DescribeNodeReturns {
    /// Node description.
    pub node: Node,
}

/// Parameters for `DOM.scrollIntoViewIfNeeded`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScrollIntoViewIfNeededParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<super::runtime::RemoteObjectId>,
    /// The rect to be scrolled into view, relative to the node's border box, in CSS pixels.
    /// When omitted, center of the node will be used, similar to Element.scrollIntoView.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rect: Option<Rect>,
}

/// Parameters for `DOM.discardSearchResults`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DiscardSearchResultsParams {
    /// Unique search session identifier.
    pub search_id: String,
}

/// Parameters for `DOM.enable`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EnableParams {
    /// Whether to include whitespaces in the children array of returned Nodes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_whitespace: Option<String>,
}

/// Parameters for `DOM.focus`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FocusParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<super::runtime::RemoteObjectId>,
}

/// Parameters for `DOM.getAttributes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAttributesParams {
    /// Id of the node to retrieve attributes for.
    pub node_id: NodeId,
}

/// Return type for `DOM.getAttributes`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAttributesReturns {
    /// An interleaved array of node attribute names and values.
    pub attributes: Vec<String>,
}

/// Parameters for `DOM.getBoxModel`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetBoxModelParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<super::runtime::RemoteObjectId>,
}

/// Return type for `DOM.getBoxModel`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetBoxModelReturns {
    /// Box model for the node.
    pub model: BoxModel,
}

/// Parameters for `DOM.getContentQuads`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetContentQuadsParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<super::runtime::RemoteObjectId>,
}

/// Return type for `DOM.getContentQuads`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContentQuadsReturns {
    /// Quads that describe node layout relative to viewport.
    pub quads: Vec<Quad>,
}

/// Parameters for `DOM.getDocument`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetDocumentParams {
    /// The maximum depth at which children should be retrieved, defaults to 1. Use -1 for the
    /// entire subtree or provide an integer larger than 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
    /// Whether or not iframes and shadow roots should be traversed when returning the subtree
    /// (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Return type for `DOM.getDocument`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDocumentReturns {
    /// Resulting node.
    pub root: Node,
}

/// Parameters for `DOM.getFlattenedDocument`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetFlattenedDocumentParams {
    /// The maximum depth at which children should be retrieved, defaults to 1. Use -1 for the
    /// entire subtree or provide an integer larger than 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
    /// Whether or not iframes and shadow roots should be traversed when returning the subtree
    /// (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Return type for `DOM.getFlattenedDocument`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFlattenedDocumentReturns {
    /// Resulting node.
    pub nodes: Vec<Node>,
}

/// Parameters for `DOM.getNodesForSubtreeByStyle`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodesForSubtreeByStyleParams {
    /// Node ID pointing to the root of a subtree.
    pub node_id: NodeId,
    /// The style to filter nodes by (includes nodes if any of properties matches).
    pub computed_styles: Vec<CSSComputedStyleProperty>,
    /// Whether or not iframes and shadow roots in the same target should be traversed when returning the
    /// results (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Return type for `DOM.getNodesForSubtreeByStyle`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodesForSubtreeByStyleReturns {
    /// Resulting nodes.
    pub node_ids: Vec<NodeId>,
}

/// Parameters for `DOM.getNodeForLocation`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetNodeForLocationParams {
    /// X coordinate.
    pub x: i64,
    /// Y coordinate.
    pub y: i64,
    /// False to skip to the nearest non-UA shadow root ancestor (default: false).
    #[serde(rename = "includeUserAgentShadowDOM")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_user_agent_shadow_dom: Option<bool>,
    /// Whether to ignore pointer-events: none on elements and hit test them.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_pointer_events_none: Option<bool>,
}

/// Return type for `DOM.getNodeForLocation`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodeForLocationReturns {
    /// Resulting node.
    pub backend_node_id: BackendNodeId,
    /// Frame this node belongs to.
    pub frame_id: super::page::FrameId,
    /// Id of the node at given coordinates, only when enabled and requested document.
    #[serde(default)]
    pub node_id: Option<NodeId>,
}

/// Parameters for `DOM.getOuterHTML`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetOuterHTMLParams {
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<super::runtime::RemoteObjectId>,
    /// Include all shadow roots. Equals to false if not specified.
    #[serde(rename = "includeShadowDOM")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_shadow_dom: Option<bool>,
}

/// Return type for `DOM.getOuterHTML`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetOuterHTMLReturns {
    /// Outer HTML markup.
    #[serde(rename = "outerHTML")]
    pub outer_html: String,
}

/// Parameters for `DOM.getRelayoutBoundary`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRelayoutBoundaryParams {
    /// Id of the node.
    pub node_id: NodeId,
}

/// Return type for `DOM.getRelayoutBoundary`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRelayoutBoundaryReturns {
    /// Relayout boundary node id for the given node.
    pub node_id: NodeId,
}

/// Parameters for `DOM.getSearchResults`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchResultsParams {
    /// Unique search session identifier.
    pub search_id: String,
    /// Start index of the search result to be returned.
    pub from_index: i64,
    /// End index of the search result to be returned.
    pub to_index: i64,
}

/// Return type for `DOM.getSearchResults`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetSearchResultsReturns {
    /// Ids of the search result nodes.
    pub node_ids: Vec<NodeId>,
}

/// Parameters for `DOM.moveTo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveToParams {
    /// Id of the node to move.
    pub node_id: NodeId,
    /// Id of the element to drop the moved node into.
    pub target_node_id: NodeId,
    /// Drop node before this one (if absent, the moved node becomes the last child of
    /// `targetNodeId`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_before_node_id: Option<NodeId>,
}

/// Return type for `DOM.moveTo`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MoveToReturns {
    /// New id of the moved node.
    pub node_id: NodeId,
}

/// Parameters for `DOM.performSearch`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PerformSearchParams {
    /// Plain text or query selector or XPath search query.
    pub query: String,
    /// True to search in user agent shadow DOM.
    #[serde(rename = "includeUserAgentShadowDOM")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_user_agent_shadow_dom: Option<bool>,
}

/// Return type for `DOM.performSearch`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformSearchReturns {
    /// Unique search session identifier.
    pub search_id: String,
    /// Number of search results.
    pub result_count: i64,
}

/// Parameters for `DOM.pushNodeByPathToFrontend`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PushNodeByPathToFrontendParams {
    /// Path to node in the proprietary format.
    pub path: String,
}

/// Return type for `DOM.pushNodeByPathToFrontend`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNodeByPathToFrontendReturns {
    /// Id of the node for given path.
    pub node_id: NodeId,
}

/// Parameters for `DOM.pushNodesByBackendIdsToFrontend`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct PushNodesByBackendIdsToFrontendParams {
    /// The array of backend node ids.
    pub backend_node_ids: Vec<BackendNodeId>,
}

/// Return type for `DOM.pushNodesByBackendIdsToFrontend`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushNodesByBackendIdsToFrontendReturns {
    /// The array of ids of pushed nodes that correspond to the backend ids specified in
    /// backendNodeIds.
    pub node_ids: Vec<NodeId>,
}

/// Parameters for `DOM.querySelector`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorParams {
    /// Id of the node to query upon.
    pub node_id: NodeId,
    /// Selector string.
    pub selector: String,
}

/// Return type for `DOM.querySelector`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorReturns {
    /// Query selector result.
    pub node_id: NodeId,
}

/// Parameters for `DOM.querySelectorAll`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorAllParams {
    /// Id of the node to query upon.
    pub node_id: NodeId,
    /// Selector string.
    pub selector: String,
}

/// Return type for `DOM.querySelectorAll`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuerySelectorAllReturns {
    /// Query selector result.
    pub node_ids: Vec<NodeId>,
}

/// Return type for `DOM.getTopLayerElements`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetTopLayerElementsReturns {
    /// NodeIds of top layer elements
    pub node_ids: Vec<NodeId>,
}

/// Parameters for `DOM.getElementByRelation`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetElementByRelationParams {
    /// Id of the node from which to query the relation.
    pub node_id: NodeId,
    /// Type of relation to get.
    pub relation: String,
}

/// Return type for `DOM.getElementByRelation`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetElementByRelationReturns {
    /// NodeId of the element matching the queried relation.
    pub node_id: NodeId,
}

/// Parameters for `DOM.removeAttribute`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveAttributeParams {
    /// Id of the element to remove attribute from.
    pub node_id: NodeId,
    /// Name of the attribute to remove.
    pub name: String,
}

/// Parameters for `DOM.removeNode`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoveNodeParams {
    /// Id of the node to remove.
    pub node_id: NodeId,
}

/// Parameters for `DOM.requestChildNodes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestChildNodesParams {
    /// Id of the node to get children for.
    pub node_id: NodeId,
    /// The maximum depth at which children should be retrieved, defaults to 1. Use -1 for the
    /// entire subtree or provide an integer larger than 0.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
    /// Whether or not iframes and shadow roots should be traversed when returning the sub-tree
    /// (default is false).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pierce: Option<bool>,
}

/// Parameters for `DOM.requestNode`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestNodeParams {
    /// JavaScript object id to convert into node.
    pub object_id: super::runtime::RemoteObjectId,
}

/// Return type for `DOM.requestNode`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestNodeReturns {
    /// Node id for given object.
    pub node_id: NodeId,
}

/// Parameters for `DOM.resolveNode`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ResolveNodeParams {
    /// Id of the node to resolve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Backend identifier of the node to resolve.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<super::dom::BackendNodeId>,
    /// Symbolic group name that can be used to release multiple objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_group: Option<String>,
    /// Execution context in which to resolve the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_context_id: Option<super::runtime::ExecutionContextId>,
}

/// Return type for `DOM.resolveNode`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolveNodeReturns {
    /// JavaScript object wrapper for given node.
    pub object: super::runtime::RemoteObject,
}

/// Parameters for `DOM.setAttributeValue`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAttributeValueParams {
    /// Id of the element to set attribute for.
    pub node_id: NodeId,
    /// Attribute name.
    pub name: String,
    /// Attribute value.
    pub value: String,
}

/// Parameters for `DOM.setAttributesAsText`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetAttributesAsTextParams {
    /// Id of the element to set attributes for.
    pub node_id: NodeId,
    /// Text with a number of attributes. Will parse this text using HTML parser.
    pub text: String,
    /// Attribute name to replace with new attributes derived from text in case text parsed
    /// successfully.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Parameters for `DOM.setFileInputFiles`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetFileInputFilesParams {
    /// Array of file paths to set.
    pub files: Vec<String>,
    /// Identifier of the node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// Identifier of the backend node.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<BackendNodeId>,
    /// JavaScript object id of the node wrapper.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<super::runtime::RemoteObjectId>,
}

/// Parameters for `DOM.setNodeStackTracesEnabled`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct SetNodeStackTracesEnabledParams {
    /// Enable or disable.
    pub enable: bool,
}

/// Parameters for `DOM.getNodeStackTraces`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodeStackTracesParams {
    /// Id of the node to get stack traces for.
    pub node_id: NodeId,
}

/// Return type for `DOM.getNodeStackTraces`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNodeStackTracesReturns {
    /// Creation stack trace, if available.
    #[serde(default)]
    pub creation: Option<super::runtime::StackTrace>,
}

/// Parameters for `DOM.getFileInfo`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFileInfoParams {
    /// JavaScript object id of the node wrapper.
    pub object_id: super::runtime::RemoteObjectId,
}

/// Return type for `DOM.getFileInfo`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFileInfoReturns {
    pub path: String,
}

/// Return type for `DOM.getDetachedDomNodes`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetDetachedDomNodesReturns {
    /// The list of detached nodes
    pub detached_nodes: Vec<DetachedElementInfo>,
}

/// Parameters for `DOM.setInspectedNode`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetInspectedNodeParams {
    /// DOM node id to be accessible by means of $x command line API.
    pub node_id: NodeId,
}

/// Parameters for `DOM.setNodeName`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNodeNameParams {
    /// Id of the node to set name for.
    pub node_id: NodeId,
    /// New node's name.
    pub name: String,
}

/// Return type for `DOM.setNodeName`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNodeNameReturns {
    /// New node's id.
    pub node_id: NodeId,
}

/// Parameters for `DOM.setNodeValue`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetNodeValueParams {
    /// Id of the node to set value for.
    pub node_id: NodeId,
    /// New node's value.
    pub value: String,
}

/// Parameters for `DOM.setOuterHTML`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetOuterHTMLParams {
    /// Id of the node to set markup for.
    pub node_id: NodeId,
    /// Outer HTML markup to set.
    #[serde(rename = "outerHTML")]
    pub outer_html: String,
}

/// Parameters for `DOM.getFrameOwner`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFrameOwnerParams {
    pub frame_id: super::page::FrameId,
}

/// Return type for `DOM.getFrameOwner`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFrameOwnerReturns {
    /// Resulting node.
    pub backend_node_id: BackendNodeId,
    /// Id of the node at given coordinates, only when enabled and requested document.
    #[serde(default)]
    pub node_id: Option<NodeId>,
}

/// Parameters for `DOM.getContainerForNode`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContainerForNodeParams {
    pub node_id: NodeId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical_axes: Option<PhysicalAxes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logical_axes: Option<LogicalAxes>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries_scroll_state: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub queries_anchored: Option<bool>,
}

/// Return type for `DOM.getContainerForNode`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetContainerForNodeReturns {
    /// The container node for the given node, or null if not found.
    #[serde(default)]
    pub node_id: Option<NodeId>,
}

/// Parameters for `DOM.getQueryingDescendantsForContainer`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetQueryingDescendantsForContainerParams {
    /// Id of the container node to find querying descendants from.
    pub node_id: NodeId,
}

/// Return type for `DOM.getQueryingDescendantsForContainer`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetQueryingDescendantsForContainerReturns {
    /// Descendant nodes with container queries against the given container.
    pub node_ids: Vec<NodeId>,
}

/// Parameters for `DOM.getAnchorElement`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAnchorElementParams {
    /// Id of the positioned element from which to find the anchor.
    pub node_id: NodeId,
    /// An optional anchor specifier, as defined in
    /// <https://www.w3.org/TR/css-anchor-position-1/#anchor-specifier.>
    /// If not provided, it will return the implicit anchor element for
    /// the given positioned element.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anchor_specifier: Option<String>,
}

/// Return type for `DOM.getAnchorElement`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAnchorElementReturns {
    /// The anchor element of the given anchor query.
    pub node_id: NodeId,
}

/// Parameters for `DOM.forceShowPopover`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceShowPopoverParams {
    /// Id of the popover HTMLElement
    pub node_id: NodeId,
    /// If true, opens the popover and keeps it open. If false, closes the
    /// popover if it was previously force-opened.
    pub enable: bool,
}

/// Return type for `DOM.forceShowPopover`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ForceShowPopoverReturns {
    /// List of popovers that were closed in order to respect popover stacking order.
    pub node_ids: Vec<NodeId>,
}

/// Event payload for `DOM.attributeModified`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeModifiedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// Attribute name.
    pub name: String,
    /// Attribute value.
    pub value: String,
}

/// Event payload for `DOM.adoptedStyleSheetsModified`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdoptedStyleSheetsModifiedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// New adoptedStyleSheets array.
    pub adopted_style_sheets: Vec<StyleSheetId>,
}

/// Event payload for `DOM.attributeRemoved`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttributeRemovedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// A ttribute name.
    pub name: String,
}

/// Event payload for `DOM.characterDataModified`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterDataModifiedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// New text value.
    pub character_data: String,
}

/// Event payload for `DOM.childNodeCountUpdated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildNodeCountUpdatedEvent {
    /// Id of the node that has changed.
    pub node_id: NodeId,
    /// New node count.
    pub child_node_count: i64,
}

/// Event payload for `DOM.childNodeInserted`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildNodeInsertedEvent {
    /// Id of the node that has changed.
    pub parent_node_id: NodeId,
    /// Id of the previous sibling.
    pub previous_node_id: NodeId,
    /// Inserted node data.
    pub node: Node,
}

/// Event payload for `DOM.childNodeRemoved`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChildNodeRemovedEvent {
    /// Parent id.
    pub parent_node_id: NodeId,
    /// Id of the node that has been removed.
    pub node_id: NodeId,
}

/// Event payload for `DOM.distributedNodesUpdated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributedNodesUpdatedEvent {
    /// Insertion point where distributed nodes were updated.
    pub insertion_point_id: NodeId,
    /// Distributed nodes for given insertion point.
    pub distributed_nodes: Vec<BackendNode>,
}

/// Event payload for `DOM.inlineStyleInvalidated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InlineStyleInvalidatedEvent {
    /// Ids of the nodes for which the inline styles have been invalidated.
    pub node_ids: Vec<NodeId>,
}

/// Event payload for `DOM.pseudoElementAdded`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseudoElementAddedEvent {
    /// Pseudo element's parent element id.
    pub parent_id: NodeId,
    /// The added pseudo element.
    pub pseudo_element: Node,
}

/// Event payload for `DOM.scrollableFlagUpdated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScrollableFlagUpdatedEvent {
    /// The id of the node.
    pub node_id: super::dom::NodeId,
    /// If the node is scrollable.
    pub is_scrollable: bool,
}

/// Event payload for `DOM.affectedByStartingStylesFlagUpdated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AffectedByStartingStylesFlagUpdatedEvent {
    /// The id of the node.
    pub node_id: super::dom::NodeId,
    /// If the node has starting styles.
    pub affected_by_starting_styles: bool,
}

/// Event payload for `DOM.pseudoElementRemoved`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PseudoElementRemovedEvent {
    /// Pseudo element's parent element id.
    pub parent_id: NodeId,
    /// The removed pseudo element id.
    pub pseudo_element_id: NodeId,
}

/// Event payload for `DOM.setChildNodes`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetChildNodesEvent {
    /// Parent node id to populate with children.
    pub parent_id: NodeId,
    /// Child nodes array.
    pub nodes: Vec<Node>,
}

/// Event payload for `DOM.shadowRootPopped`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowRootPoppedEvent {
    /// Host element id.
    pub host_id: NodeId,
    /// Shadow root id.
    pub root_id: NodeId,
}

/// Event payload for `DOM.shadowRootPushed`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShadowRootPushedEvent {
    /// Host element id.
    pub host_id: NodeId,
    /// Shadow root.
    pub root: Node,
}

// ── Methods ──
//
// These are the typed method signatures for DOM.* commands.
// Integration into CdpSession is done in pwright-cdp.

// Collects class names for the node with given id and all of it's child nodes.
// async fn dom_collect_class_names_from_subtree(&self, params: CollectClassNamesFromSubtreeParams) -> Result<CollectClassNamesFromSubtreeReturns>
// CDP: "DOM.collectClassNamesFromSubtree"

// Creates a deep copy of the specified node and places it into the target container before the
// given anchor.
// async fn dom_copy_to(&self, params: CopyToParams) -> Result<CopyToReturns>
// CDP: "DOM.copyTo"

// Describes node given its id, does not require domain to be enabled. Does not start tracking any
// objects, can be used for automation.
// async fn dom_describe_node(&self, params: DescribeNodeParams) -> Result<DescribeNodeReturns>
// CDP: "DOM.describeNode"

// Scrolls the specified rect of the given node into view if not already visible.
// Note: exactly one between nodeId, backendNodeId and objectId should be passed
// to identify the node.
// async fn dom_scroll_into_view_if_needed(&self, params: ScrollIntoViewIfNeededParams) -> Result<()>
// CDP: "DOM.scrollIntoViewIfNeeded"

// Disables DOM agent for the given page.
// async fn dom_disable(&self) -> Result<()>
// CDP: "DOM.disable"

// Discards search results from the session with the given id. `getSearchResults` should no longer
// be called for that search.
// async fn dom_discard_search_results(&self, params: DiscardSearchResultsParams) -> Result<()>
// CDP: "DOM.discardSearchResults"

// Enables DOM agent for the given page.
// async fn dom_enable(&self, params: EnableParams) -> Result<()>
// CDP: "DOM.enable"

// Focuses the given element.
// async fn dom_focus(&self, params: FocusParams) -> Result<()>
// CDP: "DOM.focus"

// Returns attributes for the specified node.
// async fn dom_get_attributes(&self, params: GetAttributesParams) -> Result<GetAttributesReturns>
// CDP: "DOM.getAttributes"

// Returns boxes for the given node.
// async fn dom_get_box_model(&self, params: GetBoxModelParams) -> Result<GetBoxModelReturns>
// CDP: "DOM.getBoxModel"

// Returns quads that describe node position on the page. This method
// might return multiple quads for inline nodes.
// async fn dom_get_content_quads(&self, params: GetContentQuadsParams) -> Result<GetContentQuadsReturns>
// CDP: "DOM.getContentQuads"

// Returns the root DOM node (and optionally the subtree) to the caller.
// Implicitly enables the DOM domain events for the current target.
// async fn dom_get_document(&self, params: GetDocumentParams) -> Result<GetDocumentReturns>
// CDP: "DOM.getDocument"

// Returns the root DOM node (and optionally the subtree) to the caller.
// Deprecated, as it is not designed to work well with the rest of the DOM agent.
// Use DOMSnapshot.captureSnapshot instead.
// async fn dom_get_flattened_document(&self, params: GetFlattenedDocumentParams) -> Result<GetFlattenedDocumentReturns>
// CDP: "DOM.getFlattenedDocument"

// Finds nodes with a given computed style in a subtree.
// async fn dom_get_nodes_for_subtree_by_style(&self, params: GetNodesForSubtreeByStyleParams) -> Result<GetNodesForSubtreeByStyleReturns>
// CDP: "DOM.getNodesForSubtreeByStyle"

// Returns node id at given location. Depending on whether DOM domain is enabled, nodeId is
// either returned or not.
// async fn dom_get_node_for_location(&self, params: GetNodeForLocationParams) -> Result<GetNodeForLocationReturns>
// CDP: "DOM.getNodeForLocation"

// Returns node's HTML markup.
// async fn dom_get_outer_html(&self, params: GetOuterHTMLParams) -> Result<GetOuterHTMLReturns>
// CDP: "DOM.getOuterHTML"

// Returns the id of the nearest ancestor that is a relayout boundary.
// async fn dom_get_relayout_boundary(&self, params: GetRelayoutBoundaryParams) -> Result<GetRelayoutBoundaryReturns>
// CDP: "DOM.getRelayoutBoundary"

// Returns search results from given `fromIndex` to given `toIndex` from the search with the given
// identifier.
// async fn dom_get_search_results(&self, params: GetSearchResultsParams) -> Result<GetSearchResultsReturns>
// CDP: "DOM.getSearchResults"

// Hides any highlight.
// async fn dom_hide_highlight(&self) -> Result<()>
// CDP: "DOM.hideHighlight"

// Highlights DOM node.
// async fn dom_highlight_node(&self) -> Result<()>
// CDP: "DOM.highlightNode"

// Highlights given rectangle.
// async fn dom_highlight_rect(&self) -> Result<()>
// CDP: "DOM.highlightRect"

// Marks last undoable state.
// async fn dom_mark_undoable_state(&self) -> Result<()>
// CDP: "DOM.markUndoableState"

// Moves node into the new container, places it before the given anchor.
// async fn dom_move_to(&self, params: MoveToParams) -> Result<MoveToReturns>
// CDP: "DOM.moveTo"

// Searches for a given string in the DOM tree. Use `getSearchResults` to access search results or
// `cancelSearch` to end this search session.
// async fn dom_perform_search(&self, params: PerformSearchParams) -> Result<PerformSearchReturns>
// CDP: "DOM.performSearch"

// Requests that the node is sent to the caller given its path. // FIXME, use XPath
// async fn dom_push_node_by_path_to_frontend(&self, params: PushNodeByPathToFrontendParams) -> Result<PushNodeByPathToFrontendReturns>
// CDP: "DOM.pushNodeByPathToFrontend"

// Requests that a batch of nodes is sent to the caller given their backend node ids.
// async fn dom_push_nodes_by_backend_ids_to_frontend(&self, params: PushNodesByBackendIdsToFrontendParams) -> Result<PushNodesByBackendIdsToFrontendReturns>
// CDP: "DOM.pushNodesByBackendIdsToFrontend"

// Executes `querySelector` on a given node.
// async fn dom_query_selector(&self, params: QuerySelectorParams) -> Result<QuerySelectorReturns>
// CDP: "DOM.querySelector"

// Executes `querySelectorAll` on a given node.
// async fn dom_query_selector_all(&self, params: QuerySelectorAllParams) -> Result<QuerySelectorAllReturns>
// CDP: "DOM.querySelectorAll"

// Returns NodeIds of current top layer elements.
// Top layer is rendered closest to the user within a viewport, therefore its elements always
// appear on top of all other content.
// async fn dom_get_top_layer_elements(&self) -> Result<GetTopLayerElementsReturns>
// CDP: "DOM.getTopLayerElements"

// Returns the NodeId of the matched element according to certain relations.
// async fn dom_get_element_by_relation(&self, params: GetElementByRelationParams) -> Result<GetElementByRelationReturns>
// CDP: "DOM.getElementByRelation"

// Re-does the last undone action.
// async fn dom_redo(&self) -> Result<()>
// CDP: "DOM.redo"

// Removes attribute with given name from an element with given id.
// async fn dom_remove_attribute(&self, params: RemoveAttributeParams) -> Result<()>
// CDP: "DOM.removeAttribute"

// Removes node with given id.
// async fn dom_remove_node(&self, params: RemoveNodeParams) -> Result<()>
// CDP: "DOM.removeNode"

// Requests that children of the node with given id are returned to the caller in form of
// `setChildNodes` events where not only immediate children are retrieved, but all children down to
// the specified depth.
// async fn dom_request_child_nodes(&self, params: RequestChildNodesParams) -> Result<()>
// CDP: "DOM.requestChildNodes"

// Requests that the node is sent to the caller given the JavaScript node object reference. All
// nodes that form the path from the node to the root are also sent to the client as a series of
// `setChildNodes` notifications.
// async fn dom_request_node(&self, params: RequestNodeParams) -> Result<RequestNodeReturns>
// CDP: "DOM.requestNode"

// Resolves the JavaScript node object for a given NodeId or BackendNodeId.
// async fn dom_resolve_node(&self, params: ResolveNodeParams) -> Result<ResolveNodeReturns>
// CDP: "DOM.resolveNode"

// Sets attribute for an element with given id.
// async fn dom_set_attribute_value(&self, params: SetAttributeValueParams) -> Result<()>
// CDP: "DOM.setAttributeValue"

// Sets attributes on element with given id. This method is useful when user edits some existing
// attribute value and types in several attribute name/value pairs.
// async fn dom_set_attributes_as_text(&self, params: SetAttributesAsTextParams) -> Result<()>
// CDP: "DOM.setAttributesAsText"

// Sets files for the given file input element.
// async fn dom_set_file_input_files(&self, params: SetFileInputFilesParams) -> Result<()>
// CDP: "DOM.setFileInputFiles"

// Sets if stack traces should be captured for Nodes. See `Node.getNodeStackTraces`. Default is disabled.
// async fn dom_set_node_stack_traces_enabled(&self, params: SetNodeStackTracesEnabledParams) -> Result<()>
// CDP: "DOM.setNodeStackTracesEnabled"

// Gets stack traces associated with a Node. As of now, only provides stack trace for Node creation.
// async fn dom_get_node_stack_traces(&self, params: GetNodeStackTracesParams) -> Result<GetNodeStackTracesReturns>
// CDP: "DOM.getNodeStackTraces"

// Returns file information for the given
// File wrapper.
// async fn dom_get_file_info(&self, params: GetFileInfoParams) -> Result<GetFileInfoReturns>
// CDP: "DOM.getFileInfo"

// Returns list of detached nodes
// async fn dom_get_detached_dom_nodes(&self) -> Result<GetDetachedDomNodesReturns>
// CDP: "DOM.getDetachedDomNodes"

// Enables console to refer to the node with given id via $x (see Command Line API for more details
// $x functions).
// async fn dom_set_inspected_node(&self, params: SetInspectedNodeParams) -> Result<()>
// CDP: "DOM.setInspectedNode"

// Sets node name for a node with given id.
// async fn dom_set_node_name(&self, params: SetNodeNameParams) -> Result<SetNodeNameReturns>
// CDP: "DOM.setNodeName"

// Sets node value for a node with given id.
// async fn dom_set_node_value(&self, params: SetNodeValueParams) -> Result<()>
// CDP: "DOM.setNodeValue"

// Sets node HTML markup, returns new node id.
// async fn dom_set_outer_html(&self, params: SetOuterHTMLParams) -> Result<()>
// CDP: "DOM.setOuterHTML"

// Undoes the last performed action.
// async fn dom_undo(&self) -> Result<()>
// CDP: "DOM.undo"

// Returns iframe node that owns iframe with the given domain.
// async fn dom_get_frame_owner(&self, params: GetFrameOwnerParams) -> Result<GetFrameOwnerReturns>
// CDP: "DOM.getFrameOwner"

// Returns the query container of the given node based on container query
// conditions: containerName, physical and logical axes, and whether it queries
// scroll-state or anchored elements. If no axes are provided and
// queriesScrollState is false, the style container is returned, which is the
// direct parent or the closest element with a matching container-name.
// async fn dom_get_container_for_node(&self, params: GetContainerForNodeParams) -> Result<GetContainerForNodeReturns>
// CDP: "DOM.getContainerForNode"

// Returns the descendants of a container query container that have
// container queries against this container.
// async fn dom_get_querying_descendants_for_container(&self, params: GetQueryingDescendantsForContainerParams) -> Result<GetQueryingDescendantsForContainerReturns>
// CDP: "DOM.getQueryingDescendantsForContainer"

// Returns the target anchor element of the given anchor query according to
// https://www.w3.org/TR/css-anchor-position-1/#target.
// async fn dom_get_anchor_element(&self, params: GetAnchorElementParams) -> Result<GetAnchorElementReturns>
// CDP: "DOM.getAnchorElement"

// When enabling, this API force-opens the popover identified by nodeId
// and keeps it open until disabled.
// async fn dom_force_show_popover(&self, params: ForceShowPopoverParams) -> Result<ForceShowPopoverReturns>
// CDP: "DOM.forceShowPopover"

