//! CDP `Accessibility` domain — generated from protocol JSON.

#![allow(clippy::doc_markdown)]

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Unique accessibility node identifier.
pub type AXNodeId = String;

/// Enum of possible property types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AXValueType {
    #[serde(rename = "boolean")]
    Boolean,
    #[serde(rename = "tristate")]
    Tristate,
    #[serde(rename = "booleanOrUndefined")]
    BooleanOrUndefined,
    #[serde(rename = "idref")]
    Idref,
    #[serde(rename = "idrefList")]
    IdrefList,
    #[serde(rename = "integer")]
    Integer,
    #[serde(rename = "node")]
    Node,
    #[serde(rename = "nodeList")]
    NodeList,
    #[serde(rename = "number")]
    Number,
    #[serde(rename = "string")]
    String,
    #[serde(rename = "computedString")]
    ComputedString,
    #[serde(rename = "token")]
    Token,
    #[serde(rename = "tokenList")]
    TokenList,
    #[serde(rename = "domRelation")]
    DomRelation,
    #[serde(rename = "role")]
    Role,
    #[serde(rename = "internalRole")]
    InternalRole,
    #[serde(rename = "valueUndefined")]
    ValueUndefined,
}

/// Enum of possible property sources.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AXValueSourceType {
    #[serde(rename = "attribute")]
    Attribute,
    #[serde(rename = "implicit")]
    Implicit,
    #[serde(rename = "style")]
    Style,
    #[serde(rename = "contents")]
    Contents,
    #[serde(rename = "placeholder")]
    Placeholder,
    #[serde(rename = "relatedElement")]
    RelatedElement,
}

/// Enum of possible native property sources (as a subtype of a particular AXValueSourceType).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AXValueNativeSourceType {
    #[serde(rename = "description")]
    Description,
    #[serde(rename = "figcaption")]
    Figcaption,
    #[serde(rename = "label")]
    Label,
    #[serde(rename = "labelfor")]
    Labelfor,
    #[serde(rename = "labelwrapped")]
    Labelwrapped,
    #[serde(rename = "legend")]
    Legend,
    #[serde(rename = "rubyannotation")]
    Rubyannotation,
    #[serde(rename = "tablecaption")]
    Tablecaption,
    #[serde(rename = "title")]
    Title,
    #[serde(rename = "other")]
    Other,
}

/// A single source for a computed AX property.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXValueSource {
    /// What type of source this is.
    #[serde(rename = "type")]
    pub r#type: AXValueSourceType,
    /// The value of this property source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<AXValue>,
    /// The name of the relevant attribute, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute: Option<String>,
    /// The value of the relevant attribute, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attribute_value: Option<AXValue>,
    /// Whether this source is superseded by a higher priority source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded: Option<bool>,
    /// The native markup source for this value, e.g. a `<label>` element.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_source: Option<AXValueNativeSourceType>,
    /// The value, such as a node or node list, of the native source.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_source_value: Option<AXValue>,
    /// Whether the value for this property is invalid.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invalid: Option<bool>,
    /// Reason for the value being invalid, if it is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invalid_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXRelatedNode {
    /// The BackendNodeId of the related DOM node.
    #[serde(rename = "backendDOMNodeId")]
    pub backend_dom_node_id: super::dom::BackendNodeId,
    /// The IDRef value provided, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idref: Option<String>,
    /// The text alternative of this node in the current context.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXProperty {
    /// The name of this property.
    pub name: AXPropertyName,
    /// The value of this property.
    pub value: AXValue,
}

/// A single computed AX property.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXValue {
    /// The type of this value.
    #[serde(rename = "type")]
    pub r#type: AXValueType,
    /// The computed value of this property.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    /// One or more related nodes, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub related_nodes: Option<Vec<AXRelatedNode>>,
    /// The sources which contributed to the computation of this property.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<Vec<AXValueSource>>,
}

/// Values of AXProperty name:
/// - from 'busy' to 'roledescription': states which apply to every AX node
/// - from 'live' to 'root': attributes which apply to nodes in live regions
/// - from 'autocomplete' to 'valuetext': attributes which apply to widgets
/// - from 'checked' to 'selected': states which apply to widgets
/// - from 'activedescendant' to 'owns': relationships between elements other than parent/child/sibling
/// - from 'activeFullscreenElement' to 'uninteresting': reasons why this noode is hidden
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AXPropertyName {
    #[serde(rename = "actions")]
    Actions,
    #[serde(rename = "busy")]
    Busy,
    #[serde(rename = "disabled")]
    Disabled,
    #[serde(rename = "editable")]
    Editable,
    #[serde(rename = "focusable")]
    Focusable,
    #[serde(rename = "focused")]
    Focused,
    #[serde(rename = "hidden")]
    Hidden,
    #[serde(rename = "hiddenRoot")]
    HiddenRoot,
    #[serde(rename = "invalid")]
    Invalid,
    #[serde(rename = "keyshortcuts")]
    Keyshortcuts,
    #[serde(rename = "settable")]
    Settable,
    #[serde(rename = "roledescription")]
    Roledescription,
    #[serde(rename = "live")]
    Live,
    #[serde(rename = "atomic")]
    Atomic,
    #[serde(rename = "relevant")]
    Relevant,
    #[serde(rename = "root")]
    Root,
    #[serde(rename = "autocomplete")]
    Autocomplete,
    #[serde(rename = "hasPopup")]
    HasPopup,
    #[serde(rename = "level")]
    Level,
    #[serde(rename = "multiselectable")]
    Multiselectable,
    #[serde(rename = "orientation")]
    Orientation,
    #[serde(rename = "multiline")]
    Multiline,
    #[serde(rename = "readonly")]
    Readonly,
    #[serde(rename = "required")]
    Required,
    #[serde(rename = "valuemin")]
    Valuemin,
    #[serde(rename = "valuemax")]
    Valuemax,
    #[serde(rename = "valuetext")]
    Valuetext,
    #[serde(rename = "checked")]
    Checked,
    #[serde(rename = "expanded")]
    Expanded,
    #[serde(rename = "modal")]
    Modal,
    #[serde(rename = "pressed")]
    Pressed,
    #[serde(rename = "selected")]
    Selected,
    #[serde(rename = "activedescendant")]
    Activedescendant,
    #[serde(rename = "controls")]
    Controls,
    #[serde(rename = "describedby")]
    Describedby,
    #[serde(rename = "details")]
    Details,
    #[serde(rename = "errormessage")]
    Errormessage,
    #[serde(rename = "flowto")]
    Flowto,
    #[serde(rename = "labelledby")]
    Labelledby,
    #[serde(rename = "owns")]
    Owns,
    #[serde(rename = "url")]
    Url,
    #[serde(rename = "activeFullscreenElement")]
    ActiveFullscreenElement,
    #[serde(rename = "activeModalDialog")]
    ActiveModalDialog,
    #[serde(rename = "activeAriaModalDialog")]
    ActiveAriaModalDialog,
    #[serde(rename = "ariaHiddenElement")]
    AriaHiddenElement,
    #[serde(rename = "ariaHiddenSubtree")]
    AriaHiddenSubtree,
    #[serde(rename = "emptyAlt")]
    EmptyAlt,
    #[serde(rename = "emptyText")]
    EmptyText,
    #[serde(rename = "inertElement")]
    InertElement,
    #[serde(rename = "inertSubtree")]
    InertSubtree,
    #[serde(rename = "labelContainer")]
    LabelContainer,
    #[serde(rename = "labelFor")]
    LabelFor,
    #[serde(rename = "notRendered")]
    NotRendered,
    #[serde(rename = "notVisible")]
    NotVisible,
    #[serde(rename = "presentationalRole")]
    PresentationalRole,
    #[serde(rename = "probablyPresentational")]
    ProbablyPresentational,
    #[serde(rename = "inactiveCarouselTabContent")]
    InactiveCarouselTabContent,
    #[serde(rename = "uninteresting")]
    Uninteresting,
}

/// A node in the accessibility tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AXNode {
    /// Unique identifier for this node.
    pub node_id: AXNodeId,
    /// Whether this node is ignored for accessibility
    pub ignored: bool,
    /// Collection of reasons why this node is hidden.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignored_reasons: Option<Vec<AXProperty>>,
    /// This `Node`'s role, whether explicit or implicit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<AXValue>,
    /// This `Node`'s Chrome raw role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chrome_role: Option<AXValue>,
    /// The accessible name for this `Node`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<AXValue>,
    /// The accessible description for this `Node`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<AXValue>,
    /// The value for this `Node`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<AXValue>,
    /// All other properties
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<AXProperty>>,
    /// ID for this node's parent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<AXNodeId>,
    /// IDs for each of this node's child nodes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_ids: Option<Vec<AXNodeId>>,
    /// The backend ID for the associated DOM node, if any.
    #[serde(rename = "backendDOMNodeId")]
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backend_dom_node_id: Option<super::dom::BackendNodeId>,
    /// The frame ID for the frame associated with this nodes document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<super::page::FrameId>,
}

/// Parameters for `Accessibility.getPartialAXTree`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetPartialAXTreeParams {
    /// Identifier of the node to get the partial accessibility tree for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<super::dom::NodeId>,
    /// Identifier of the backend node to get the partial accessibility tree for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<super::dom::BackendNodeId>,
    /// JavaScript object id of the node wrapper to get the partial accessibility tree for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<super::runtime::RemoteObjectId>,
    /// Whether to fetch this node's ancestors, siblings and children. Defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetch_relatives: Option<bool>,
}

/// Return type for `Accessibility.getPartialAXTree`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPartialAXTreeReturns {
    /// The `Accessibility.AXNode` for this DOM node, if it exists, plus its ancestors, siblings and
    /// children, if requested.
    pub nodes: Vec<AXNode>,
}

/// Parameters for `Accessibility.getFullAXTree`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetFullAXTreeParams {
    /// The maximum depth at which descendants of the root node should be retrieved.
    /// If omitted, the full tree is returned.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth: Option<i64>,
    /// The frame for whose document the AX tree should be retrieved.
    /// If omitted, the root frame is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<super::page::FrameId>,
}

/// Return type for `Accessibility.getFullAXTree`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetFullAXTreeReturns {
    pub nodes: Vec<AXNode>,
}

/// Parameters for `Accessibility.getRootAXNode`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetRootAXNodeParams {
    /// The frame in whose document the node resides.
    /// If omitted, the root frame is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<super::page::FrameId>,
}

/// Return type for `Accessibility.getRootAXNode`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetRootAXNodeReturns {
    pub node: AXNode,
}

/// Parameters for `Accessibility.getAXNodeAndAncestors`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct GetAXNodeAndAncestorsParams {
    /// Identifier of the node to get.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<super::dom::NodeId>,
    /// Identifier of the backend node to get.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<super::dom::BackendNodeId>,
    /// JavaScript object id of the node wrapper to get.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<super::runtime::RemoteObjectId>,
}

/// Return type for `Accessibility.getAXNodeAndAncestors`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetAXNodeAndAncestorsReturns {
    pub nodes: Vec<AXNode>,
}

/// Parameters for `Accessibility.getChildAXNodes`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetChildAXNodesParams {
    pub id: AXNodeId,
    /// The frame in whose document the node resides.
    /// If omitted, the root frame is used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<super::page::FrameId>,
}

/// Return type for `Accessibility.getChildAXNodes`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetChildAXNodesReturns {
    pub nodes: Vec<AXNode>,
}

/// Parameters for `Accessibility.queryAXTree`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct QueryAXTreeParams {
    /// Identifier of the node for the root to query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node_id: Option<super::dom::NodeId>,
    /// Identifier of the backend node for the root to query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backend_node_id: Option<super::dom::BackendNodeId>,
    /// JavaScript object id of the node wrapper for the root to query.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<super::runtime::RemoteObjectId>,
    /// Find nodes with this computed name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessible_name: Option<String>,
    /// Find nodes with this computed role.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
}

/// Return type for `Accessibility.queryAXTree`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryAXTreeReturns {
    /// A list of `Accessibility.AXNode` matching the specified attributes,
    /// including nodes that are ignored for accessibility.
    pub nodes: Vec<AXNode>,
}

/// Event payload for `Accessibility.loadComplete`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCompleteEvent {
    /// New document root node.
    pub root: AXNode,
}

/// Event payload for `Accessibility.nodesUpdated`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NodesUpdatedEvent {
    /// Updated node data.
    pub nodes: Vec<AXNode>,
}

// ── Methods ──
//
// These are the typed method signatures for Accessibility.* commands.
// Integration into CdpSession is done in pwright-cdp.

// Disables the accessibility domain.
// async fn accessibility_disable(&self) -> Result<()>
// CDP: "Accessibility.disable"

// Enables the accessibility domain which causes `AXNodeId`s to remain consistent between method calls.
// This turns on accessibility for the page, which can impact performance until accessibility is disabled.
// async fn accessibility_enable(&self) -> Result<()>
// CDP: "Accessibility.enable"

// Fetches the accessibility node and partial accessibility tree for this DOM node, if it exists.
// async fn accessibility_get_partial_ax_tree(&self, params: GetPartialAXTreeParams) -> Result<GetPartialAXTreeReturns>
// CDP: "Accessibility.getPartialAXTree"

// Fetches the entire accessibility tree for the root Document
// async fn accessibility_get_full_ax_tree(&self, params: GetFullAXTreeParams) -> Result<GetFullAXTreeReturns>
// CDP: "Accessibility.getFullAXTree"

// Fetches the root node.
// Requires `enable()` to have been called previously.
// async fn accessibility_get_root_ax_node(&self, params: GetRootAXNodeParams) -> Result<GetRootAXNodeReturns>
// CDP: "Accessibility.getRootAXNode"

// Fetches a node and all ancestors up to and including the root.
// Requires `enable()` to have been called previously.
// async fn accessibility_get_ax_node_and_ancestors(&self, params: GetAXNodeAndAncestorsParams) -> Result<GetAXNodeAndAncestorsReturns>
// CDP: "Accessibility.getAXNodeAndAncestors"

// Fetches a particular accessibility node by AXNodeId.
// Requires `enable()` to have been called previously.
// async fn accessibility_get_child_ax_nodes(&self, params: GetChildAXNodesParams) -> Result<GetChildAXNodesReturns>
// CDP: "Accessibility.getChildAXNodes"

// Query a DOM node's accessibility subtree for accessible name and role.
// This command computes the name and role for all nodes in the subtree, including those that are
// ignored for accessibility, and returns those that match the specified name and role. If no DOM
// node is specified, or the DOM node does not exist, the command returns an error. If neither
// `accessibleName` or `role` is specified, it returns all the accessibility nodes in the subtree.
// async fn accessibility_query_ax_tree(&self, params: QueryAXTreeParams) -> Result<QueryAXTreeReturns>
// CDP: "Accessibility.queryAXTree"
