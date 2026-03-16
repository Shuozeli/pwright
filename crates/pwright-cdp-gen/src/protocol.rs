//! Typed representation of the CDP protocol JSON schema.

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ProtocolFile {
    pub version: Version,
    pub domains: Vec<Domain>,
}

#[derive(Debug, Deserialize)]
pub struct Version {
    pub major: String,
    pub minor: String,
}

#[derive(Debug, Deserialize)]
pub struct Domain {
    pub domain: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub types: Vec<TypeDef>,
    #[serde(default)]
    pub commands: Vec<Command>,
    #[serde(default)]
    pub events: Vec<Event>,
}

#[derive(Debug, Deserialize)]
pub struct TypeDef {
    pub id: String,
    #[serde(rename = "type")]
    pub type_kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub properties: Vec<Property>,
    #[serde(default, rename = "enum")]
    pub enum_values: Vec<String>,
    /// For array types: describes the element type.
    #[serde(default)]
    pub items: Option<ItemRef>,
}

#[derive(Debug, Deserialize)]
pub struct Command {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub parameters: Vec<Property>,
    #[serde(default)]
    pub returns: Vec<Property>,
}

#[derive(Debug, Deserialize)]
pub struct Event {
    pub name: String,
    #[serde(default)]
    pub parameters: Vec<Property>,
}

#[derive(Debug, Deserialize)]
pub struct Property {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub optional: bool,
    /// Primitive type: "string", "number", "integer", "boolean", "object", "any", "array", "binary"
    #[serde(rename = "type")]
    pub type_kind: Option<String>,
    /// Reference to another type: "DomainName.TypeId" or "TypeId"
    #[serde(rename = "$ref")]
    pub ref_type: Option<String>,
    /// For array types: element type description.
    #[serde(default)]
    pub items: Option<ItemRef>,
    /// For string types with fixed values.
    #[serde(default, rename = "enum")]
    pub enum_values: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ItemRef {
    #[serde(rename = "type")]
    pub type_kind: Option<String>,
    #[serde(rename = "$ref")]
    pub ref_type: Option<String>,
}
