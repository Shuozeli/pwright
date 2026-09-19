//! Parser, validator, and deterministic Rust generator for Pwright schemas.
//!
//! This is a development tool. Generated capability crates do not depend on
//! YAML, Serde, this crate, or a build script at compile time.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt::{self, Display, Formatter, Write};

use serde::Deserialize;

const SCHEMA_FORMAT: u16 = 1;
const MAX_TYPES: usize = 1_024;
const MAX_VARIANTS_PER_TYPE: usize = 256;
const MAX_FIELDS_PER_SHAPE: usize = 1_024;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProtocolSchema {
    format: u16,
    capability: Option<String>,
    display_name: Option<String>,
    library: Option<String>,
    protocol: Option<String>,
    version: Version,
    /// Development-only binding placement. Wire identity/version is unchanged.
    /// Allows an opt-in minor revision to coexist with a retained root API.
    #[serde(default)]
    rust_minor_module: bool,
    root: Option<String>,
    #[serde(default)]
    imports: Vec<String>,
    #[serde(default)]
    dependencies: Vec<CapabilityDependencyDefinition>,
    #[serde(default)]
    commands: Vec<CommandDefinition>,
    types: Vec<TypeDefinition>,
}

impl ProtocolSchema {
    fn rust_minor_namespace(&self) -> Option<String> {
        self.rust_minor_module
            .then(|| format!("v{}_{}", self.version.major, self.version.minor))
    }
    fn namespace(&self) -> Option<&str> {
        self.capability
            .as_deref()
            .or(self.library.as_deref())
            .or(self.protocol.as_deref())
    }

    fn root(&self) -> Option<&str> {
        self.root.as_deref()
    }

    const fn identity_kind(&self) -> Option<SchemaIdentityKind> {
        if self.capability.is_some() {
            Some(SchemaIdentityKind::Capability)
        } else if self.library.is_some() {
            Some(SchemaIdentityKind::Library)
        } else if self.protocol.is_some() {
            Some(SchemaIdentityKind::Protocol)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SchemaIdentityKind {
    Capability,
    Library,
    Protocol,
}

impl SchemaIdentityKind {
    const fn display_name(self) -> &'static str {
        match self {
            Self::Capability => "Capability",
            Self::Library => "Library",
            Self::Protocol => "Protocol",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct Version {
    major: u16,
    minor: u16,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct CapabilityDependencyDefinition {
    id: String,
    minimum_version: Version,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct CommandDefinition {
    name: String,
    id: String,
    method: u32,
    #[serde(default)]
    targets: Vec<String>,
    command: String,
    result: String,
    params_type: String,
    result_type: String,
    error_details_type: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum TypeDefinition {
    #[serde(rename = "enum")]
    Enumeration {
        id: String,
        public_name: Option<String>,
        #[serde(default)]
        public_external: bool,
        default: Option<String>,
        variants: Vec<EnumVariant>,
    },
    Record {
        id: String,
        public_name: Option<String>,
        #[serde(default)]
        public_external: bool,
        #[serde(default)]
        encoding: RecordEncoding,
        encoded_size: Option<SizeConstraints>,
        fields: Vec<Field>,
    },
    TaggedUnion {
        id: String,
        public_name: Option<String>,
        #[serde(default)]
        public_external: bool,
        variants: Vec<Variant>,
    },
}

impl TypeDefinition {
    fn id(&self) -> &str {
        match self {
            Self::Enumeration { id, .. }
            | Self::Record { id, .. }
            | Self::TaggedUnion { id, .. } => id,
        }
    }

    fn public_name(&self) -> Option<&str> {
        match self {
            Self::Enumeration { public_name, .. }
            | Self::Record { public_name, .. }
            | Self::TaggedUnion { public_name, .. } => public_name.as_deref(),
        }
    }

    fn enum_default(&self) -> Option<&str> {
        match self {
            Self::Enumeration { default, .. } => default.as_deref(),
            Self::Record { .. } | Self::TaggedUnion { .. } => None,
        }
    }

    const fn public_external(&self) -> bool {
        match self {
            Self::Enumeration {
                public_external, ..
            }
            | Self::Record {
                public_external, ..
            }
            | Self::TaggedUnion {
                public_external, ..
            } => *public_external,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct EnumVariant {
    tag: u8,
    name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct Variant {
    tag: u8,
    name: String,
    #[serde(default)]
    fields: Vec<Field>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct Field {
    tag: Option<u16>,
    name: String,
    #[serde(rename = "type")]
    field_type: FieldType,
    #[serde(default)]
    cardinality: Cardinality,
    target: Option<String>,
    counted_by: Option<String>,
    constraints: Option<FieldConstraints>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct FieldConstraints {
    minimum: Option<NumericBound>,
    maximum: Option<NumericBound>,
    minimum_length: Option<u32>,
    maximum_length: Option<u32>,
    minimum_items: Option<u32>,
    maximum_items: Option<u32>,
}

impl FieldConstraints {
    const fn numeric(self) -> Option<NumericConstraints> {
        if self.minimum.is_none() && self.maximum.is_none() {
            None
        } else {
            Some(NumericConstraints {
                minimum: self.minimum,
                maximum: self.maximum,
            })
        }
    }

    const fn length(self) -> Option<SizeConstraints> {
        if self.minimum_length.is_none() && self.maximum_length.is_none() {
            None
        } else {
            Some(SizeConstraints {
                minimum: self.minimum_length,
                maximum: self.maximum_length,
            })
        }
    }

    const fn items(self) -> Option<SizeConstraints> {
        if self.minimum_items.is_none() && self.maximum_items.is_none() {
            None
        } else {
            Some(SizeConstraints {
                minimum: self.minimum_items,
                maximum: self.maximum_items,
            })
        }
    }

    const fn is_empty(self) -> bool {
        self.numeric().is_none() && self.length().is_none() && self.items().is_none()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct NumericConstraints {
    minimum: Option<NumericBound>,
    maximum: Option<NumericBound>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct SizeConstraints {
    minimum: Option<u32>,
    maximum: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum RecordEncoding {
    #[default]
    Positional,
    Extensible,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(untagged)]
enum NumericBound {
    Signed(i64),
    Unsigned(u64),
}

impl NumericBound {
    const fn as_i128(self) -> i128 {
        match self {
            Self::Signed(value) => value as i128,
            Self::Unsigned(value) => value as i128,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum FieldType {
    U8,
    U16,
    U32,
    U64,
    I32,
    I64,
    F64,
    Bytes,
    Bool,
    #[serde(rename = "enum")]
    Enumeration,
    Utf8,
    Message,
}

impl FieldType {
    const fn schema_variant(self) -> &'static str {
        match self {
            Self::U8 => "U8",
            Self::U16 => "U16",
            Self::U32 => "U32",
            Self::U64 => "U64",
            Self::I32 => "I32",
            Self::I64 => "I64",
            Self::F64 => "F64",
            Self::Bytes => "Bytes",
            Self::Bool => "Bool",
            Self::Enumeration => "Enumeration",
            Self::Utf8 => "Utf8",
            Self::Message => "Message",
        }
    }

    const fn rust_type(self) -> Option<&'static str> {
        match self {
            Self::U8 => Some("u8"),
            Self::U16 => Some("u16"),
            Self::U32 => Some("u32"),
            Self::U64 => Some("u64"),
            Self::I32 => Some("i32"),
            Self::I64 => Some("i64"),
            Self::F64 => Some("f64"),
            Self::Bytes => Some("Vec<u8>"),
            Self::Bool => Some("bool"),
            Self::Enumeration => None,
            Self::Utf8 => Some("String"),
            Self::Message => None,
        }
    }

    const fn integer_bounds(self) -> Option<(i128, i128)> {
        match self {
            Self::U8 => Some((0, u8::MAX as i128)),
            Self::U16 => Some((0, u16::MAX as i128)),
            Self::U32 => Some((0, u32::MAX as i128)),
            Self::U64 => Some((0, u64::MAX as i128)),
            Self::I32 => Some((i32::MIN as i128, i32::MAX as i128)),
            Self::I64 => Some((i64::MIN as i128, i64::MAX as i128)),
            Self::F64
            | Self::Bytes
            | Self::Bool
            | Self::Enumeration
            | Self::Utf8
            | Self::Message => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Cardinality {
    #[default]
    Required,
    Optional,
    Repeated,
}

impl Cardinality {
    const fn schema_variant(self) -> &'static str {
        match self {
            Self::Required => "Required",
            Self::Optional => "Optional",
            Self::Repeated => "Repeated",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumericConstraintRange {
    minimum: i128,
    maximum: i128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SizeConstraintRange {
    minimum: Option<u32>,
    maximum: Option<u32>,
}

impl SizeConstraintRange {
    pub const fn new(minimum: Option<u32>, maximum: Option<u32>) -> Self {
        Self { minimum, maximum }
    }

    pub const fn minimum(&self) -> Option<u32> {
        self.minimum
    }

    pub const fn maximum(&self) -> Option<u32> {
        self.maximum
    }
}

impl NumericConstraintRange {
    pub const fn new(minimum: Option<i128>, maximum: Option<i128>) -> Self {
        Self {
            minimum: match minimum {
                Some(value) => value,
                None => i128::MIN,
            },
            maximum: match maximum {
                Some(value) => value,
                None => i128::MAX,
            },
        }
    }

    pub const fn minimum(&self) -> Option<i128> {
        if self.minimum == i128::MIN {
            None
        } else {
            Some(self.minimum)
        }
    }

    pub const fn maximum(&self) -> Option<i128> {
        if self.maximum == i128::MAX {
            None
        } else {
            Some(self.maximum)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaError {
    Yaml(String),
    UnsupportedFormat {
        actual: u16,
        supported: u16,
    },
    EmptyCapability,
    EmptyRoot,
    MissingSchemaIdentity,
    ConflictingSchemaIdentity,
    CapabilityRootMissing,
    ProtocolRootMissing,
    LibraryHasRoot,
    LibraryHasCommands,
    ProtocolHasCommands,
    LibraryHasDependencies,
    ProtocolHasDependencies,
    GeneratedPublicModelsRequireCapability,
    RustMinorModuleRequiresCapability,
    ServerOutputRequiresCapability,
    ServerOutputRequiresCommands,
    ImportsRequireResolution,
    InvalidDependencyId {
        value: String,
    },
    NonCanonicalDependencyOrder {
        previous: String,
        current: String,
    },
    SelfDependency {
        capability: String,
    },
    InvalidImport {
        value: String,
    },
    NonCanonicalImportOrder {
        previous: String,
        current: String,
    },
    MissingImport {
        namespace: String,
    },
    UnexpectedImport {
        namespace: String,
    },
    ImportMustBeLibrary {
        namespace: String,
    },
    ImportedTypeConflict {
        id: String,
    },
    InvalidCommandId {
        value: String,
    },
    NonCanonicalCommandOrder {
        previous: String,
        current: String,
    },
    DuplicateCommandMethod {
        method: u32,
    },
    EmptyCommandTargets {
        command: String,
    },
    InvalidCommandTarget {
        command: String,
        target: String,
    },
    NonCanonicalCommandTargetOrder {
        command: String,
        previous: String,
        current: String,
    },
    UnknownCommandVariant {
        command: String,
        role: &'static str,
        variant: String,
    },
    ReusedCommandVariant {
        variant: String,
    },
    InvalidPublicTypeName {
        type_id: String,
        name: String,
    },
    DuplicatePublicTypeName {
        name: String,
    },
    ExternalPublicModelMissingName {
        type_id: String,
    },
    UnknownEnumDefault {
        type_id: String,
        variant: String,
    },
    UnknownCommandModel {
        command: String,
        role: &'static str,
        type_id: String,
    },
    WrongCommandModelKind {
        command: String,
        role: &'static str,
        type_id: String,
    },
    CommandModelMissingPublicName {
        command: String,
        role: &'static str,
        type_id: String,
    },
    PublicModelTargetMissingName {
        owner: String,
        target: String,
    },
    TooManyTypes {
        actual: usize,
        limit: usize,
    },
    EmptyTypes,
    NonCanonicalTypeOrder {
        previous: String,
        current: String,
    },
    InvalidIdentifier {
        kind: &'static str,
        value: String,
    },
    DuplicateRustTypeName {
        name: String,
    },
    RootMissing {
        id: String,
    },
    RootMustBeTaggedUnion {
        id: String,
    },
    TooManyVariants {
        type_id: String,
        actual: usize,
        limit: usize,
    },
    EmptyVariants {
        type_id: String,
    },
    NonCanonicalVariantOrder {
        type_id: String,
        previous: u8,
        current: u8,
    },
    UnexpectedRecordEncodedSize {
        type_id: String,
    },
    MissingOrInvalidExtensibleRecordEncodedSize {
        type_id: String,
        range: Option<SizeConstraintRange>,
    },
    UnexpectedExtensibleFieldTag {
        owner: String,
        field: String,
        tag: u16,
    },
    MissingExtensibleFieldTag {
        owner: String,
        field: String,
    },
    NonCanonicalExtensibleFieldTag {
        type_id: String,
        previous: Option<u16>,
        current: u16,
    },
    ExtensibleFieldUsesExternalCount {
        owner: String,
        field: String,
    },
    TooManyFields {
        owner: String,
        actual: usize,
        limit: usize,
    },
    DuplicateField {
        owner: String,
        name: String,
    },
    MessageTargetMissing {
        owner: String,
        field: String,
    },
    UnexpectedTarget {
        owner: String,
        field: String,
    },
    CountedByOnNonRepeatedField {
        owner: String,
        field: String,
    },
    CountFieldMissingOrNotEarlier {
        owner: String,
        field: String,
        count_field: String,
    },
    InvalidCountField {
        owner: String,
        field: String,
        count_field: String,
    },
    UnexpectedNumericConstraints {
        owner: String,
        field: String,
    },
    InvalidNumericConstraints {
        owner: String,
        field: String,
        range: NumericConstraintRange,
    },
    EmptyConstraints {
        owner: String,
        field: String,
    },
    UnexpectedLengthConstraints {
        owner: String,
        field: String,
    },
    InvalidLengthConstraints {
        owner: String,
        field: String,
        range: SizeConstraintRange,
    },
    UnexpectedItemConstraints {
        owner: String,
        field: String,
    },
    InvalidItemConstraints {
        owner: String,
        field: String,
        range: SizeConstraintRange,
    },
    UnknownTarget {
        owner: String,
        field: String,
        target: String,
    },
    WrongTargetKind {
        owner: String,
        field: String,
        target: String,
    },
    UninhabitedRecursiveModel {
        type_id: String,
    },
    Formatting,
}

impl Display for SchemaError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Yaml(error) => write!(formatter, "invalid schema YAML: {error}"),
            Self::UnsupportedFormat { actual, supported } => write!(
                formatter,
                "schema format {actual} is unsupported; this generator supports {supported}"
            ),
            Self::EmptyCapability => formatter.write_str("schema identity is empty"),
            Self::EmptyRoot => formatter.write_str("schema root is empty"),
            Self::MissingSchemaIdentity => formatter.write_str(
                "schema must declare exactly one of `capability`, `library`, or `protocol`",
            ),
            Self::ConflictingSchemaIdentity => formatter.write_str(
                "schema cannot declare more than one of `capability`, `library`, or `protocol`",
            ),
            Self::CapabilityRootMissing => {
                formatter.write_str("capability schema must declare a tagged-union root")
            }
            Self::ProtocolRootMissing => {
                formatter.write_str("protocol schema must declare a tagged-union root")
            }
            Self::LibraryHasRoot => {
                formatter.write_str("library schema cannot declare a protocol root")
            }
            Self::LibraryHasCommands => {
                formatter.write_str("library schema cannot declare capability commands")
            }
            Self::ProtocolHasCommands => {
                formatter.write_str("protocol schema cannot declare capability commands")
            }
            Self::LibraryHasDependencies => {
                formatter.write_str("library schema cannot declare capability dependencies")
            }
            Self::ProtocolHasDependencies => {
                formatter.write_str("protocol schema cannot declare capability dependencies")
            }
            Self::GeneratedPublicModelsRequireCapability => {
                formatter.write_str("generated public models require a capability schema")
            }
            Self::RustMinorModuleRequiresCapability => formatter
                .write_str("minor-version Rust module selection requires a capability schema"),
            Self::ServerOutputRequiresCapability => {
                formatter.write_str("server binding output requires a capability schema")
            }
            Self::ServerOutputRequiresCommands => {
                formatter.write_str("server binding output requires at least one command")
            }
            Self::ImportsRequireResolution => formatter
                .write_str("schema declares imports; use the import-aware parser and generator"),
            Self::InvalidDependencyId { value } => {
                write!(
                    formatter,
                    "capability dependency `{value}` is not canonical"
                )
            }
            Self::NonCanonicalDependencyOrder { previous, current } => write!(
                formatter,
                "capability dependency `{current}` does not follow `{previous}`"
            ),
            Self::SelfDependency { capability } => {
                write!(formatter, "capability `{capability}` depends on itself")
            }
            Self::InvalidImport { value } => {
                write!(
                    formatter,
                    "schema import `{value}` is not a valid namespace"
                )
            }
            Self::NonCanonicalImportOrder { previous, current } => write!(
                formatter,
                "schema import `{current}` does not follow `{previous}`"
            ),
            Self::MissingImport { namespace } => {
                write!(
                    formatter,
                    "declared schema import `{namespace}` was not provided"
                )
            }
            Self::UnexpectedImport { namespace } => write!(
                formatter,
                "provided schema library `{namespace}` was not declared as an import"
            ),
            Self::ImportMustBeLibrary { namespace } => write!(
                formatter,
                "schema import `{namespace}` is not a wire library"
            ),
            Self::ImportedTypeConflict { id } => {
                write!(formatter, "schema imports conflicting type ID `{id}`")
            }
            Self::InvalidCommandId { value } => {
                write!(formatter, "command ID `{value}` is not canonical")
            }
            Self::NonCanonicalCommandOrder { previous, current } => write!(
                formatter,
                "schema command `{current}` does not follow `{previous}`"
            ),
            Self::DuplicateCommandMethod { method } => {
                write!(formatter, "schema repeats command method {method}")
            }
            Self::EmptyCommandTargets { command } => {
                write!(
                    formatter,
                    "command `{command}` declares no target object kinds"
                )
            }
            Self::InvalidCommandTarget { command, target } => write!(
                formatter,
                "command `{command}` has invalid target object kind `{target}`"
            ),
            Self::NonCanonicalCommandTargetOrder {
                command,
                previous,
                current,
            } => write!(
                formatter,
                "command `{command}` target `{current}` does not follow `{previous}`"
            ),
            Self::UnknownCommandVariant {
                command,
                role,
                variant,
            } => write!(
                formatter,
                "command `{command}` references unknown {role} root variant `{variant}`"
            ),
            Self::ReusedCommandVariant { variant } => {
                write!(
                    formatter,
                    "root variant `{variant}` is reused by multiple commands"
                )
            }
            Self::InvalidPublicTypeName { type_id, name } => write!(
                formatter,
                "schema type `{type_id}` has invalid public name `{name}`"
            ),
            Self::DuplicatePublicTypeName { name } => {
                write!(formatter, "schema repeats public type name `{name}`")
            }
            Self::ExternalPublicModelMissingName { type_id } => write!(
                formatter,
                "external public model `{type_id}` must declare `public_name`"
            ),
            Self::UnknownEnumDefault { type_id, variant } => write!(
                formatter,
                "enum `{type_id}` declares unknown default variant `{variant}`"
            ),
            Self::UnknownCommandModel {
                command,
                role,
                type_id,
            } => write!(
                formatter,
                "command `{command}` references unknown {role} model `{type_id}`"
            ),
            Self::WrongCommandModelKind {
                command,
                role,
                type_id,
            } => write!(
                formatter,
                "command `{command}` references {role} model `{type_id}` with an incompatible kind"
            ),
            Self::CommandModelMissingPublicName {
                command,
                role,
                type_id,
            } => write!(
                formatter,
                "command `{command}` {role} model `{type_id}` has no public name"
            ),
            Self::PublicModelTargetMissingName { owner, target } => write!(
                formatter,
                "public model `{owner}` references `{target}` without a public name"
            ),
            Self::TooManyTypes { actual, limit } => {
                write!(
                    formatter,
                    "schema has {actual} types, exceeding limit {limit}"
                )
            }
            Self::EmptyTypes => formatter.write_str("schema has no types"),
            Self::NonCanonicalTypeOrder { previous, current } => write!(
                formatter,
                "schema type `{current}` does not follow `{previous}`"
            ),
            Self::InvalidIdentifier { kind, value } => {
                write!(
                    formatter,
                    "{kind} `{value}` is not a lower_snake_case identifier"
                )
            }
            Self::DuplicateRustTypeName { name } => {
                write!(formatter, "multiple schema IDs generate Rust type `{name}`")
            }
            Self::RootMissing { id } => write!(formatter, "root type `{id}` is missing"),
            Self::RootMustBeTaggedUnion { id } => {
                write!(formatter, "root type `{id}` is not a tagged union")
            }
            Self::TooManyVariants {
                type_id,
                actual,
                limit,
            } => write!(
                formatter,
                "type `{type_id}` has {actual} variants, exceeding limit {limit}"
            ),
            Self::EmptyVariants { type_id } => {
                write!(formatter, "tagged union `{type_id}` has no variants")
            }
            Self::NonCanonicalVariantOrder {
                type_id,
                previous,
                current,
            } => write!(
                formatter,
                "variant tag {current} in `{type_id}` does not follow tag {previous}"
            ),
            Self::UnexpectedRecordEncodedSize { type_id } => write!(
                formatter,
                "positional record `{type_id}` cannot declare `encoded_size`"
            ),
            Self::MissingOrInvalidExtensibleRecordEncodedSize { type_id, range } => write!(
                formatter,
                "extensible record `{type_id}` requires a valid bounded `encoded_size`, got {range:?}"
            ),
            Self::UnexpectedExtensibleFieldTag { owner, field, tag } => write!(
                formatter,
                "positional field `{owner}.{field}` unexpectedly declares extensible tag {tag}"
            ),
            Self::MissingExtensibleFieldTag { owner, field } => {
                write!(formatter, "extensible field `{owner}.{field}` has no tag")
            }
            Self::NonCanonicalExtensibleFieldTag {
                type_id,
                previous,
                current,
            } => match previous {
                Some(previous) => write!(
                    formatter,
                    "extensible field tag {current} in `{type_id}` does not follow tag {previous}"
                ),
                None => write!(
                    formatter,
                    "extensible record `{type_id}` uses reserved field tag {current}"
                ),
            },
            Self::ExtensibleFieldUsesExternalCount { owner, field } => write!(
                formatter,
                "extensible field `{owner}.{field}` cannot depend on `counted_by` from another field"
            ),
            Self::TooManyFields {
                owner,
                actual,
                limit,
            } => write!(
                formatter,
                "shape `{owner}` has {actual} fields, exceeding limit {limit}"
            ),
            Self::DuplicateField { owner, name } => {
                write!(formatter, "shape `{owner}` repeats field `{name}`")
            }
            Self::MessageTargetMissing { owner, field } => {
                write!(formatter, "message field `{owner}.{field}` has no target")
            }
            Self::UnexpectedTarget { owner, field } => {
                write!(
                    formatter,
                    "scalar field `{owner}.{field}` unexpectedly has a target"
                )
            }
            Self::CountedByOnNonRepeatedField { owner, field } => write!(
                formatter,
                "non-repeated field `{owner}.{field}` declares `counted_by`"
            ),
            Self::CountFieldMissingOrNotEarlier {
                owner,
                field,
                count_field,
            } => write!(
                formatter,
                "field `{owner}.{field}` references missing or non-earlier count field `{count_field}`"
            ),
            Self::InvalidCountField {
                owner,
                field,
                count_field,
            } => write!(
                formatter,
                "field `{owner}.{field}` requires `{count_field}` to be a required u32 count"
            ),
            Self::UnexpectedNumericConstraints { owner, field } => write!(
                formatter,
                "non-integer field `{owner}.{field}` has numeric constraints"
            ),
            Self::InvalidNumericConstraints {
                owner,
                field,
                range,
            } => write!(
                formatter,
                "integer field `{owner}.{field}` has invalid range {:?}..={:?}",
                range.minimum(),
                range.maximum()
            ),
            Self::EmptyConstraints { owner, field } => {
                write!(formatter, "field `{owner}.{field}` has empty constraints")
            }
            Self::UnexpectedLengthConstraints { owner, field } => write!(
                formatter,
                "field `{owner}.{field}` cannot carry string or byte length constraints"
            ),
            Self::InvalidLengthConstraints {
                owner,
                field,
                range,
            } => write!(
                formatter,
                "field `{owner}.{field}` has invalid length range {:?}..={:?}",
                range.minimum(),
                range.maximum()
            ),
            Self::UnexpectedItemConstraints { owner, field } => write!(
                formatter,
                "non-repeated field `{owner}.{field}` has item-count constraints"
            ),
            Self::InvalidItemConstraints {
                owner,
                field,
                range,
            } => write!(
                formatter,
                "field `{owner}.{field}` has invalid item-count range {:?}..={:?}",
                range.minimum(),
                range.maximum()
            ),
            Self::UnknownTarget {
                owner,
                field,
                target,
            } => write!(
                formatter,
                "message field `{owner}.{field}` references unknown type `{target}`"
            ),
            Self::WrongTargetKind {
                owner,
                field,
                target,
            } => write!(
                formatter,
                "field `{owner}.{field}` references `{target}` with an incompatible type kind"
            ),
            Self::UninhabitedRecursiveModel { type_id } => write!(
                formatter,
                "recursive model `{type_id}` has no finite wire value"
            ),
            Self::Formatting => formatter.write_str("could not format generated Rust source"),
        }
    }
}

impl Error for SchemaError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompatibilityError {
    InvalidPrevious(SchemaError),
    InvalidNext(SchemaError),
    CapabilityChanged {
        previous: String,
        next: String,
    },
    IdentityKindChanged {
        previous: &'static str,
        next: &'static str,
    },
    RootChanged {
        previous: String,
        next: String,
    },
    ImportsChanged,
    DependenciesChanged,
    MajorChanged {
        previous: u16,
        next: u16,
    },
    VersionRegressed {
        previous: (u16, u16),
        next: (u16, u16),
    },
    CommandRemoved {
        id: String,
    },
    CommandChanged {
        id: String,
    },
    TypeRemoved {
        id: String,
    },
    TypeKindChanged {
        id: String,
    },
    PositionalLayoutChanged {
        id: String,
    },
    ExtensibleRecordEncodedSizeChanged {
        id: String,
    },
    ExtensibleFieldRemoved {
        id: String,
        tag: u16,
    },
    ExtensibleFieldChanged {
        id: String,
        tag: u16,
    },
    NonOptionalExtensibleFieldAdded {
        id: String,
        tag: u16,
    },
}

impl Display for CompatibilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrevious(error) => {
                write!(formatter, "previous schema is invalid: {error}")
            }
            Self::InvalidNext(error) => write!(formatter, "next schema is invalid: {error}"),
            Self::CapabilityChanged { previous, next } => write!(
                formatter,
                "schema capability changed from `{previous}` to `{next}`"
            ),
            Self::IdentityKindChanged { previous, next } => write!(
                formatter,
                "schema identity kind changed from {previous} to {next}"
            ),
            Self::RootChanged { previous, next } => {
                write!(
                    formatter,
                    "schema root changed from `{previous}` to `{next}`"
                )
            }
            Self::ImportsChanged => {
                formatter.write_str("schema imports changed within a compatible major version")
            }
            Self::DependenciesChanged => formatter.write_str(
                "schema capability dependencies changed within a compatible major version",
            ),
            Self::MajorChanged { previous, next } => {
                write!(formatter, "schema major changed from {previous} to {next}")
            }
            Self::VersionRegressed { previous, next } => write!(
                formatter,
                "schema version regressed from {}.{} to {}.{}",
                previous.0, previous.1, next.0, next.1
            ),
            Self::CommandRemoved { id } => {
                write!(formatter, "schema command `{id}` was removed")
            }
            Self::CommandChanged { id } => {
                write!(formatter, "schema command `{id}` changed its wire mapping")
            }
            Self::TypeRemoved { id } => write!(formatter, "schema type `{id}` was removed"),
            Self::TypeKindChanged { id } => {
                write!(formatter, "schema type `{id}` changed kind")
            }
            Self::PositionalLayoutChanged { id } => write!(
                formatter,
                "schema type `{id}` changed its published positional layout"
            ),
            Self::ExtensibleRecordEncodedSizeChanged { id } => write!(
                formatter,
                "extensible record `{id}` changed its published encoded-size bound"
            ),
            Self::ExtensibleFieldRemoved { id, tag } => {
                write!(
                    formatter,
                    "extensible record `{id}` removed field tag {tag}"
                )
            }
            Self::ExtensibleFieldChanged { id, tag } => write!(
                formatter,
                "extensible record `{id}` changed field tag {tag}"
            ),
            Self::NonOptionalExtensibleFieldAdded { id, tag } => write!(
                formatter,
                "extensible record `{id}` added non-optional field tag {tag}"
            ),
        }
    }
}

impl Error for CompatibilityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPrevious(error) | Self::InvalidNext(error) => Some(error),
            _ => None,
        }
    }
}

pub fn parse_and_validate(source: &str) -> Result<ProtocolSchema, SchemaError> {
    let schema = parse_source(source)?;
    if !schema.imports.is_empty() {
        return Err(SchemaError::ImportsRequireResolution);
    }
    validate(&schema)?;
    Ok(schema)
}

pub fn parse_and_validate_with_imports(
    source: &str,
    imports: &[ProtocolSchema],
) -> Result<ProtocolSchema, SchemaError> {
    let schema = parse_source(source)?;
    resolve_schema(&schema, imports)?;
    Ok(schema)
}

fn parse_source(source: &str) -> Result<ProtocolSchema, SchemaError> {
    serde_yaml::from_str(source).map_err(|error| SchemaError::Yaml(error.to_string()))
}

fn resolve_schema(
    schema: &ProtocolSchema,
    imports: &[ProtocolSchema],
) -> Result<ProtocolSchema, SchemaError> {
    validate_import_names(&schema.imports)?;
    validate_type_order(&schema.types)?;
    for imported in imports {
        if !imported.imports.is_empty() {
            return Err(SchemaError::ImportsRequireResolution);
        }
        validate(imported)?;
        let namespace = imported.namespace().unwrap_or_default();
        if imported.library.is_none() {
            return Err(SchemaError::ImportMustBeLibrary {
                namespace: namespace.to_owned(),
            });
        }
        if schema
            .imports
            .binary_search_by(|candidate| candidate.as_str().cmp(namespace))
            .is_err()
        {
            return Err(SchemaError::UnexpectedImport {
                namespace: namespace.to_owned(),
            });
        }
    }
    for namespace in &schema.imports {
        let count = imports
            .iter()
            .filter(|imported| imported.namespace() == Some(namespace.as_str()))
            .count();
        if count == 0 {
            return Err(SchemaError::MissingImport {
                namespace: namespace.clone(),
            });
        }
        if count > 1 {
            return Err(SchemaError::UnexpectedImport {
                namespace: namespace.clone(),
            });
        }
    }
    let mut resolved = schema.clone();
    resolved.imports.clear();
    for imported in imports {
        resolved.types.extend(imported.types.iter().cloned());
    }
    resolved
        .types
        .sort_by(|left, right| left.id().cmp(right.id()));
    if let Some(pair) = resolved
        .types
        .windows(2)
        .find(|pair| pair[0].id() == pair[1].id())
    {
        return Err(SchemaError::ImportedTypeConflict {
            id: pair[0].id().to_owned(),
        });
    }
    validate(&resolved)?;
    Ok(resolved)
}

fn validate_import_names(imports: &[String]) -> Result<(), SchemaError> {
    let mut previous: Option<&str> = None;
    for namespace in imports {
        if rust_type_name(namespace).is_err() {
            return Err(SchemaError::InvalidImport {
                value: namespace.clone(),
            });
        }
        if let Some(previous) = previous
            && previous >= namespace
        {
            return Err(SchemaError::NonCanonicalImportOrder {
                previous: previous.to_owned(),
                current: namespace.clone(),
            });
        }
        previous = Some(namespace);
    }
    Ok(())
}

fn validate_dependencies(
    capability: Option<&str>,
    dependencies: &[CapabilityDependencyDefinition],
) -> Result<(), SchemaError> {
    let Some(capability) = capability else {
        return if dependencies.is_empty() {
            Ok(())
        } else {
            Err(SchemaError::LibraryHasDependencies)
        };
    };
    let mut previous: Option<&str> = None;
    for dependency in dependencies {
        if rust_type_name(&dependency.id).is_err() {
            return Err(SchemaError::InvalidDependencyId {
                value: dependency.id.clone(),
            });
        }
        if dependency.id == capability {
            return Err(SchemaError::SelfDependency {
                capability: capability.to_owned(),
            });
        }
        if let Some(previous) = previous
            && previous >= dependency.id.as_str()
        {
            return Err(SchemaError::NonCanonicalDependencyOrder {
                previous: previous.to_owned(),
                current: dependency.id.clone(),
            });
        }
        previous = Some(&dependency.id);
    }
    Ok(())
}

fn validate_type_order(types: &[TypeDefinition]) -> Result<(), SchemaError> {
    for pair in types.windows(2) {
        if pair[0].id() >= pair[1].id() {
            return Err(SchemaError::NonCanonicalTypeOrder {
                previous: pair[0].id().to_owned(),
                current: pair[1].id().to_owned(),
            });
        }
    }
    Ok(())
}

/// Checks a candidate source schema against a previously published minor
/// version. New types and appended union variants are permitted; every
/// previously decodable positional shape must remain exact.
pub fn check_minor_compatibility(
    previous: &ProtocolSchema,
    next: &ProtocolSchema,
) -> Result<(), CompatibilityError> {
    if !previous.imports.is_empty() {
        return Err(CompatibilityError::InvalidPrevious(
            SchemaError::ImportsRequireResolution,
        ));
    }
    if !next.imports.is_empty() {
        return Err(CompatibilityError::InvalidNext(
            SchemaError::ImportsRequireResolution,
        ));
    }
    validate(previous).map_err(CompatibilityError::InvalidPrevious)?;
    validate(next).map_err(CompatibilityError::InvalidNext)?;
    check_minor_compatibility_resolved(previous, next)
}

pub fn check_minor_compatibility_with_imports(
    previous: &ProtocolSchema,
    next: &ProtocolSchema,
    previous_imports: &[ProtocolSchema],
    next_imports: &[ProtocolSchema],
) -> Result<(), CompatibilityError> {
    if previous.imports != next.imports {
        return Err(CompatibilityError::ImportsChanged);
    }
    let previous =
        resolve_schema(previous, previous_imports).map_err(CompatibilityError::InvalidPrevious)?;
    let next = resolve_schema(next, next_imports).map_err(CompatibilityError::InvalidNext)?;
    check_minor_compatibility_resolved(&previous, &next)
}

fn check_minor_compatibility_resolved(
    previous: &ProtocolSchema,
    next: &ProtocolSchema,
) -> Result<(), CompatibilityError> {
    if previous.identity_kind() != next.identity_kind() {
        return Err(CompatibilityError::IdentityKindChanged {
            previous: previous
                .identity_kind()
                .expect("validated previous identity")
                .display_name(),
            next: next
                .identity_kind()
                .expect("validated next identity")
                .display_name(),
        });
    }
    if previous.namespace() != next.namespace() {
        return Err(CompatibilityError::CapabilityChanged {
            previous: previous.namespace().unwrap_or_default().to_owned(),
            next: next.namespace().unwrap_or_default().to_owned(),
        });
    }
    if previous.root != next.root {
        return Err(CompatibilityError::RootChanged {
            previous: previous.root().unwrap_or("<library>").to_owned(),
            next: next.root().unwrap_or("<library>").to_owned(),
        });
    }
    if previous.dependencies != next.dependencies {
        return Err(CompatibilityError::DependenciesChanged);
    }
    if previous.version.major != next.version.major {
        return Err(CompatibilityError::MajorChanged {
            previous: previous.version.major,
            next: next.version.major,
        });
    }
    if next.version.minor < previous.version.minor {
        return Err(CompatibilityError::VersionRegressed {
            previous: (previous.version.major, previous.version.minor),
            next: (next.version.major, next.version.minor),
        });
    }
    for previous_command in &previous.commands {
        let candidate = next
            .commands
            .binary_search_by(|command| command.id.cmp(&previous_command.id))
            .ok()
            .map(|index| &next.commands[index])
            .ok_or_else(|| CompatibilityError::CommandRemoved {
                id: previous_command.id.clone(),
            })?;
        if candidate != previous_command {
            return Err(CompatibilityError::CommandChanged {
                id: previous_command.id.clone(),
            });
        }
    }
    for previous_type in &previous.types {
        let candidate = next
            .types
            .binary_search_by(|definition| definition.id().cmp(previous_type.id()))
            .ok()
            .map(|index| &next.types[index])
            .ok_or_else(|| CompatibilityError::TypeRemoved {
                id: previous_type.id().to_owned(),
            })?;
        if previous_type.public_name() != candidate.public_name()
            || previous_type.public_external() != candidate.public_external()
        {
            return Err(CompatibilityError::PositionalLayoutChanged {
                id: previous_type.id().to_owned(),
            });
        }
        if previous_type.enum_default() != candidate.enum_default() {
            return Err(CompatibilityError::PositionalLayoutChanged {
                id: previous_type.id().to_owned(),
            });
        }
        match (previous_type, candidate) {
            (
                TypeDefinition::Enumeration {
                    id,
                    variants: previous_variants,
                    ..
                },
                TypeDefinition::Enumeration {
                    variants: next_variants,
                    ..
                },
            ) if next_variants.len() < previous_variants.len()
                || !next_variants.starts_with(previous_variants) =>
            {
                return Err(CompatibilityError::PositionalLayoutChanged { id: id.clone() });
            }
            (
                TypeDefinition::Record {
                    id,
                    encoding: previous_encoding,
                    encoded_size: previous_encoded_size,
                    fields: previous_fields,
                    ..
                },
                TypeDefinition::Record {
                    encoding: next_encoding,
                    encoded_size: next_encoded_size,
                    fields: next_fields,
                    ..
                },
            ) => {
                if previous_encoding != next_encoding {
                    return Err(CompatibilityError::TypeKindChanged { id: id.clone() });
                }
                match previous_encoding {
                    RecordEncoding::Positional if previous_fields != next_fields => {
                        return Err(CompatibilityError::PositionalLayoutChanged { id: id.clone() });
                    }
                    RecordEncoding::Extensible => check_extensible_record_compatibility(
                        id,
                        *previous_encoded_size,
                        previous_fields,
                        *next_encoded_size,
                        next_fields,
                    )?,
                    RecordEncoding::Positional => {}
                }
            }
            (
                TypeDefinition::TaggedUnion {
                    id,
                    variants: previous_variants,
                    ..
                },
                TypeDefinition::TaggedUnion {
                    variants: next_variants,
                    ..
                },
            ) if next_variants.len() < previous_variants.len()
                || !next_variants.starts_with(previous_variants) =>
            {
                return Err(CompatibilityError::PositionalLayoutChanged { id: id.clone() });
            }
            (TypeDefinition::Enumeration { .. }, TypeDefinition::Enumeration { .. })
            | (TypeDefinition::TaggedUnion { .. }, TypeDefinition::TaggedUnion { .. }) => {}
            _ => {
                return Err(CompatibilityError::TypeKindChanged {
                    id: previous_type.id().to_owned(),
                });
            }
        }
    }
    Ok(())
}

fn check_extensible_record_compatibility(
    id: &str,
    previous_encoded_size: Option<SizeConstraints>,
    previous_fields: &[Field],
    next_encoded_size: Option<SizeConstraints>,
    next_fields: &[Field],
) -> Result<(), CompatibilityError> {
    if previous_encoded_size != next_encoded_size {
        return Err(CompatibilityError::ExtensibleRecordEncodedSizeChanged { id: id.to_owned() });
    }
    for previous in previous_fields {
        let tag = previous.tag.expect("validated extensible field tag");
        let candidate = next_fields
            .binary_search_by_key(&tag, |field| {
                field.tag.expect("validated extensible field tag")
            })
            .ok()
            .map(|index| &next_fields[index])
            .ok_or_else(|| CompatibilityError::ExtensibleFieldRemoved {
                id: id.to_owned(),
                tag,
            })?;
        if previous != candidate {
            return Err(CompatibilityError::ExtensibleFieldChanged {
                id: id.to_owned(),
                tag,
            });
        }
    }
    for candidate in next_fields {
        let tag = candidate.tag.expect("validated extensible field tag");
        if previous_fields
            .binary_search_by_key(&tag, |field| {
                field.tag.expect("validated extensible field tag")
            })
            .is_err()
            && candidate.cardinality != Cardinality::Optional
        {
            return Err(CompatibilityError::NonOptionalExtensibleFieldAdded {
                id: id.to_owned(),
                tag,
            });
        }
    }
    Ok(())
}

pub fn generate_rust(schema: &ProtocolSchema) -> Result<String, SchemaError> {
    if !schema.imports.is_empty() {
        return Err(SchemaError::ImportsRequireResolution);
    }
    generate_rust_resolved(schema, schema, &[])
}

pub fn generate_rust_with_imports(
    schema: &ProtocolSchema,
    imports: &[ProtocolSchema],
) -> Result<String, SchemaError> {
    let resolved = resolve_schema(schema, imports)?;
    generate_rust_resolved(schema, &resolved, imports)
}

/// Generates the core-only capability identity and command metadata surface.
pub fn generate_capability_rust(schema: &ProtocolSchema) -> Result<String, SchemaError> {
    if !schema.imports.is_empty() {
        return Err(SchemaError::ImportsRequireResolution);
    }
    generate_capability_rust_resolved(schema, schema)
}

/// Generates a capability surface after resolving its declared wire libraries.
pub fn generate_capability_rust_with_imports(
    schema: &ProtocolSchema,
    imports: &[ProtocolSchema],
) -> Result<String, SchemaError> {
    let resolved = resolve_schema(schema, imports)?;
    generate_capability_rust_resolved(schema, &resolved)
}

/// Generates an optional capability-specific concurrent server binding.
pub fn generate_server_rust(schema: &ProtocolSchema) -> Result<String, SchemaError> {
    if !schema.imports.is_empty() {
        return Err(SchemaError::ImportsRequireResolution);
    }
    generate_server_rust_resolved(schema, schema)
}

/// Generates a server binding after resolving the capability's libraries.
pub fn generate_server_rust_with_imports(
    schema: &ProtocolSchema,
    imports: &[ProtocolSchema],
) -> Result<String, SchemaError> {
    let resolved = resolve_schema(schema, imports)?;
    generate_server_rust_resolved(schema, &resolved)
}

fn generate_server_rust_resolved(
    schema: &ProtocolSchema,
    resolved: &ProtocolSchema,
) -> Result<String, SchemaError> {
    validate(resolved)?;
    let capability = schema
        .capability
        .as_deref()
        .ok_or(SchemaError::ServerOutputRequiresCapability)?;
    if schema.commands.is_empty() {
        return Err(SchemaError::ServerOutputRequiresCommands);
    }
    let short_name = capability.strip_prefix("pwright.").unwrap_or(capability);
    let crate_suffix = short_name.replace('.', "_");
    let rust_name = rust_type_name(short_name)?;
    let major_suffix = (schema.version.major != 1).then(|| format!("V{}", schema.version.major));
    let codec_type = format!(
        "{rust_name}{}Codec",
        major_suffix.as_deref().unwrap_or_default()
    );
    let alias = format!(
        "{rust_name}{}ServerHandlers",
        major_suffix.as_deref().unwrap_or_default()
    );
    let service_bundle = format!(
        "{rust_name}{}ServerServices",
        major_suffix.as_deref().unwrap_or_default()
    );
    let constructor = if schema.version.major == 1 {
        format!("{}_server_handlers", short_name.replace('.', "_"))
    } else {
        format!(
            "{}_v{}_server_handlers",
            short_name.replace('.', "_"),
            schema.version.major
        )
    };
    let services = schema
        .commands
        .iter()
        .map(|command| format!("{}Service", pascal_case(&command.name)))
        .collect::<Vec<_>>();
    let handlers = schema
        .commands
        .iter()
        .zip(&services)
        .map(|(command, service)| {
            format!(
                "ConcurrentTypedCommandHandler<{}, {codec_type}, {service}>",
                pascal_case(&command.name)
            )
        })
        .collect::<Vec<_>>();
    let handler_chain = balanced_server_handler_type(&handlers);
    let markers = schema
        .commands
        .iter()
        .map(|command| pascal_case(&command.name))
        .collect::<Vec<_>>()
        .join(", ");

    let mut output = String::new();
    writeln!(
        output,
        "// @generated by pwright-schema-gen; do not edit manually."
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "use std::sync::Arc;\n").map_err(|_| SchemaError::Formatting)?;
    if let Some(module) = schema.rust_minor_namespace() {
        writeln!(
            output,
            "use pwright_cap_{crate_suffix}::{module}::{{{markers}}};"
        )
        .map_err(|_| SchemaError::Formatting)?;
    } else if schema.version.major == 1 {
        writeln!(output, "use pwright_cap_{crate_suffix}::{{{markers}}};")
            .map_err(|_| SchemaError::Formatting)?;
    } else {
        writeln!(
            output,
            "use pwright_cap_{crate_suffix}::v{}::{{{markers}}};",
            schema.version.major
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    let codec_module = schema
        .rust_minor_namespace()
        .map_or_else(String::new, |module| format!("{module}::"));
    writeln!(
        output,
        "use pwright_codec_{crate_suffix}::{codec_module}{codec_type};"
    )
    .map_err(|_| SchemaError::Formatting)?;
    if schema.commands.len() == 1 {
        writeln!(
            output,
            "use pwright_server_wire::ConcurrentTypedCommandHandler;"
        )
        .map_err(|_| SchemaError::Formatting)?;
    } else {
        writeln!(
            output,
            "use pwright_server_wire::{{ConcurrentServerHandlerChain, ConcurrentTypedCommandHandler}};"
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "use pwright_transport::WireProtocol;\n")
        .map_err(|_| SchemaError::Formatting)?;
    let grouped = services.len() > 16;
    let middle = services.len() / 2;
    let (left_services, right_services) = services.split_at(middle);
    let (left_handlers, right_handlers) = handlers.split_at(middle);
    let left_alias = format!("{alias}Left");
    let right_alias = format!("{alias}Right");
    let left_bundle = format!("{service_bundle}Left");
    let right_bundle = format!("{service_bundle}Right");
    if grouped {
        writeln!(
            output,
            "pub type {left_alias}<{}> = {};",
            left_services.join(", "),
            balanced_server_handler_type(left_handlers)
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "pub type {right_alias}<{}> = {};",
            right_services.join(", "),
            balanced_server_handler_type(right_handlers)
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "pub type {alias}<Left, Right> = ConcurrentServerHandlerChain<Left, Right>;\n"
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "pub struct {left_bundle}<{}> {{",
            left_services.join(", ")
        )
        .map_err(|_| SchemaError::Formatting)?;
        for (command, service) in schema.commands[..middle].iter().zip(left_services) {
            writeln!(output, "    pub {}: Arc<{service}>,", command.name)
                .map_err(|_| SchemaError::Formatting)?;
        }
        writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "pub struct {right_bundle}<{}> {{",
            right_services.join(", ")
        )
        .map_err(|_| SchemaError::Formatting)?;
        for (command, service) in schema.commands[middle..].iter().zip(right_services) {
            writeln!(output, "    pub {}: Arc<{service}>,", command.name)
                .map_err(|_| SchemaError::Formatting)?;
        }
        writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "pub struct {service_bundle}<Left, Right> {{")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "    pub left: Left,").map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "    pub right: Right,").map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        let service_group = format!("{service_bundle}Group");
        writeln!(output, "pub trait {service_group} {{").map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "    type Handlers;").map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "    fn into_handlers(self, protocol: Arc<WireProtocol<{codec_type}>>) -> Self::Handlers;"
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        emit_server_service_group_impl(
            &mut output,
            ServerServiceGroup {
                service_group: &service_group,
                bundle: &left_bundle,
                alias: &left_alias,
                services: left_services,
                commands: &schema.commands[..middle],
                handlers: left_handlers,
                codec_type: &codec_type,
            },
        )?;
        emit_server_service_group_impl(
            &mut output,
            ServerServiceGroup {
                service_group: &service_group,
                bundle: &right_bundle,
                alias: &right_alias,
                services: right_services,
                commands: &schema.commands[middle..],
                handlers: right_handlers,
                codec_type: &codec_type,
            },
        )?;
        writeln!(output, "pub fn {constructor}<Left, Right>(")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "    protocol: Arc<WireProtocol<{codec_type}>>,")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "    services: {service_bundle}<Left, Right>,")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, ") -> {alias}<Left::Handlers, Right::Handlers>")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "where").map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "    Left: {service_group},").map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "    Right: {service_group},").map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "{{").map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "    let left = services.left.into_handlers(Arc::clone(&protocol));"
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "    let right = services.right.into_handlers(protocol);"
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "    ConcurrentServerHandlerChain::new(left, right)")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "}}").map_err(|_| SchemaError::Formatting)?;
        return Ok(output);
    } else {
        writeln!(
            output,
            "pub type {alias}<{}> = {handler_chain};\n",
            services.join(", ")
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "pub struct {service_bundle}<{}> {{",
            services.join(", ")
        )
        .map_err(|_| SchemaError::Formatting)?;
        for (command, service) in schema.commands.iter().zip(&services) {
            writeln!(output, "    pub {}: Arc<{service}>,", command.name)
                .map_err(|_| SchemaError::Formatting)?;
        }
        writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "pub fn {constructor}<{}>(", services.join(", "))
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    protocol: Arc<WireProtocol<{codec_type}>>,")
        .map_err(|_| SchemaError::Formatting)?;
    if grouped {
        writeln!(
            output,
            "    services: {service_bundle}<{left_bundle}<{}>, {right_bundle}<{}>>,",
            left_services.join(", "),
            right_services.join(", ")
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            ") -> {alias}<{left_alias}<{}>, {right_alias}<{}>> {{",
            left_services.join(", "),
            right_services.join(", ")
        )
        .map_err(|_| SchemaError::Formatting)?;
    } else {
        writeln!(
            output,
            "    services: {service_bundle}<{}>,",
            services.join(", ")
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, ") -> {alias}<{}> {{", services.join(", "))
            .map_err(|_| SchemaError::Formatting)?;
    }
    for (index, (command, handler)) in schema.commands.iter().zip(&handlers).enumerate() {
        writeln!(
            output,
            "    let {}: {handler} = ConcurrentTypedCommandHandler::from_shared(",
            command.name
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "        Arc::clone(&protocol),").map_err(|_| SchemaError::Formatting)?;
        let group = if grouped {
            if index < middle { "left." } else { "right." }
        } else {
            ""
        };
        writeln!(output, "        services.{group}{},", command.name)
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "    );").map_err(|_| SchemaError::Formatting)?;
    }
    let handler_names = schema
        .commands
        .iter()
        .map(|command| command.name.clone())
        .collect::<Vec<_>>();
    let chain_expression = if grouped {
        let left = balanced_server_handler_expression(&handler_names[..middle]);
        let right = balanced_server_handler_expression(&handler_names[middle..]);
        format!("ConcurrentServerHandlerChain::new({left}, {right})")
    } else {
        balanced_server_handler_expression(&handler_names)
    };
    writeln!(output, "    {chain_expression}\n}}").map_err(|_| SchemaError::Formatting)?;
    Ok(output)
}

struct ServerServiceGroup<'a> {
    service_group: &'a str,
    bundle: &'a str,
    alias: &'a str,
    services: &'a [String],
    commands: &'a [CommandDefinition],
    handlers: &'a [String],
    codec_type: &'a str,
}

fn emit_server_service_group_impl(
    output: &mut String,
    group: ServerServiceGroup<'_>,
) -> Result<(), SchemaError> {
    let ServerServiceGroup {
        service_group,
        bundle,
        alias,
        services,
        commands,
        handlers,
        codec_type,
    } = group;
    writeln!(
        output,
        "impl<{}> {service_group} for {bundle}<{}> {{",
        services.join(", "),
        services.join(", ")
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "    type Handlers = {alias}<{}>;",
        services.join(", ")
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "    fn into_handlers(self, protocol: Arc<WireProtocol<{codec_type}>>) -> Self::Handlers {{"
    )
    .map_err(|_| SchemaError::Formatting)?;
    for (command, handler) in commands.iter().zip(handlers) {
        writeln!(
            output,
            "        let {}: {handler} = ConcurrentTypedCommandHandler::from_shared(",
            command.name
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "            Arc::clone(&protocol),")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "            self.{},", command.name)
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "        );").map_err(|_| SchemaError::Formatting)?;
    }
    let handler_names = commands
        .iter()
        .map(|command| command.name.clone())
        .collect::<Vec<_>>();
    let expression = balanced_server_handler_expression(&handler_names);
    writeln!(output, "        {expression}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    }}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
    Ok(())
}

fn balanced_server_handler_type(handlers: &[String]) -> String {
    if handlers.len() == 1 {
        return handlers[0].clone();
    }
    let middle = handlers.len() / 2;
    let left = balanced_server_handler_type(&handlers[..middle]);
    let right = balanced_server_handler_type(&handlers[middle..]);
    format!("ConcurrentServerHandlerChain<{left}, {right}>")
}

fn balanced_server_handler_expression(handlers: &[String]) -> String {
    if handlers.len() == 1 {
        return handlers[0].clone();
    }
    let middle = handlers.len() / 2;
    let left = balanced_server_handler_expression(&handlers[..middle]);
    let right = balanced_server_handler_expression(&handlers[middle..]);
    format!("ConcurrentServerHandlerChain::new({left}, {right})")
}

/// Generates a deterministic human-readable reference for one schema.
pub fn generate_markdown_reference(schema: &ProtocolSchema) -> Result<String, SchemaError> {
    if !schema.imports.is_empty() {
        return Err(SchemaError::ImportsRequireResolution);
    }
    generate_markdown_reference_resolved(schema, schema)
}

/// Generates a deterministic reference after resolving declared libraries.
pub fn generate_markdown_reference_with_imports(
    schema: &ProtocolSchema,
    imports: &[ProtocolSchema],
) -> Result<String, SchemaError> {
    let resolved = resolve_schema(schema, imports)?;
    generate_markdown_reference_resolved(schema, &resolved)
}

fn generate_markdown_reference_resolved(
    schema: &ProtocolSchema,
    resolved: &ProtocolSchema,
) -> Result<String, SchemaError> {
    validate(resolved)?;
    let namespace = schema
        .namespace()
        .ok_or(SchemaError::MissingSchemaIdentity)?;
    let identity_kind = schema
        .identity_kind()
        .expect("validated schema identity")
        .display_name();
    let mut output = String::new();
    writeln!(output, "<!-- agent-updated: 2026-09-03T22:08:32Z -->\n")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "# `{namespace}` protocol reference\n")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "> Generated by `pwright-schema-gen`; do not edit manually. The YAML schema is canonical.\n"
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "| Property | Value |\n| --- | --- |").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "| Kind | {identity_kind} |").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "| Schema format | `{}` |", schema.format)
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "| Protocol version | `{}.{}` |",
        schema.version.major, schema.version.minor
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "| Root type | {} |",
        schema
            .root
            .as_deref()
            .map_or_else(|| "—".to_string(), |root| format!("`{root}`"))
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "| Imports | {} |",
        markdown_code_list(schema.imports.iter().map(String::as_str))
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "| Capability dependencies | {} |\n",
        if schema.dependencies.is_empty() {
            "—".to_string()
        } else {
            schema
                .dependencies
                .iter()
                .map(|dependency| {
                    format!(
                        "`{}` ≥ `{}.{}`",
                        dependency.id,
                        dependency.minimum_version.major,
                        dependency.minimum_version.minor
                    )
                })
                .collect::<Vec<_>>()
                .join(", ")
        }
    )
    .map_err(|_| SchemaError::Formatting)?;

    if !schema.commands.is_empty() {
        writeln!(output, "## Commands\n").map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "| Command | ID | Method | Targets | Params | Result | Error details | Wire variants |\n| --- | --- | ---: | --- | --- | --- | --- | --- |"
        )
        .map_err(|_| SchemaError::Formatting)?;
        for command in &schema.commands {
            writeln!(
                output,
                "| `{}` | `{}` | `{}` | {} | `{}` | `{}` | `{}` | `{}` / `{}` |",
                command.name,
                command.id,
                command.method,
                markdown_code_list(command.targets.iter().map(String::as_str)),
                command.params_type,
                command.result_type,
                command.error_details_type,
                command.command,
                command.result
            )
            .map_err(|_| SchemaError::Formatting)?;
        }
        writeln!(output).map_err(|_| SchemaError::Formatting)?;
    }

    writeln!(output, "## Types\n").map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "| Type ID | Kind | Public Rust model |\n| --- | --- | --- |"
    )
    .map_err(|_| SchemaError::Formatting)?;
    for definition in &schema.types {
        let kind = match definition {
            TypeDefinition::Enumeration { .. } => "enum",
            TypeDefinition::Record {
                encoding: RecordEncoding::Positional,
                ..
            } => "record",
            TypeDefinition::Record {
                encoding: RecordEncoding::Extensible,
                ..
            } => "extensible record",
            TypeDefinition::TaggedUnion { .. } => "tagged union",
        };
        let public_name = definition
            .public_name()
            .map(pascal_case)
            .map_or_else(|| "—".to_string(), |name| format!("`{name}`"));
        writeln!(output, "| `{}` | {kind} | {public_name} |", definition.id())
            .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output).map_err(|_| SchemaError::Formatting)?;

    for definition in &schema.types {
        emit_markdown_type(&mut output, definition)?;
    }
    // Keep generated documents whitespace-clean when the final type is added
    // or replaced; section separators are not extra blank lines at EOF.
    while output.ends_with("\n\n") {
        output.pop();
    }
    Ok(output)
}

fn markdown_code_list<'a>(values: impl Iterator<Item = &'a str>) -> String {
    let values = values.map(|value| format!("`{value}`")).collect::<Vec<_>>();
    if values.is_empty() {
        "—".to_string()
    } else {
        values.join(", ")
    }
}

fn emit_markdown_type(output: &mut String, definition: &TypeDefinition) -> Result<(), SchemaError> {
    writeln!(output, "### `{}`\n", definition.id()).map_err(|_| SchemaError::Formatting)?;
    match definition {
        TypeDefinition::Enumeration {
            default, variants, ..
        } => {
            writeln!(output, "| Tag | Variant | Default |\n| ---: | --- | --- |")
                .map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                let is_default = default.as_deref() == Some(variant.name.as_str());
                writeln!(
                    output,
                    "| `{}` | `{}` | {} |",
                    variant.tag,
                    variant.name,
                    if is_default { "yes" } else { "" }
                )
                .map_err(|_| SchemaError::Formatting)?;
            }
        }
        TypeDefinition::Record {
            encoding, fields, ..
        } => match encoding {
            RecordEncoding::Positional => emit_markdown_fields(output, fields)?,
            RecordEncoding::Extensible => emit_markdown_extensible_fields(output, fields)?,
        },
        TypeDefinition::TaggedUnion { variants, .. } => {
            writeln!(output, "| Tag | Variant | Fields |\n| ---: | --- | --- |")
                .map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                let fields = if variant.fields.is_empty() {
                    "—".to_string()
                } else {
                    variant
                        .fields
                        .iter()
                        .map(markdown_inline_field)
                        .collect::<Vec<_>>()
                        .join("; ")
                };
                writeln!(
                    output,
                    "| `{}` | `{}` | {fields} |",
                    variant.tag, variant.name
                )
                .map_err(|_| SchemaError::Formatting)?;
            }
        }
    }
    writeln!(output).map_err(|_| SchemaError::Formatting)
}

fn emit_markdown_fields(output: &mut String, fields: &[Field]) -> Result<(), SchemaError> {
    writeln!(
        output,
        "| Position | Field | Type | Cardinality | Constraints |\n| ---: | --- | --- | --- | --- |"
    )
    .map_err(|_| SchemaError::Formatting)?;
    for (position, field) in fields.iter().enumerate() {
        writeln!(
            output,
            "| {} | `{}` | {} | {} | {} |",
            position + 1,
            field.name,
            markdown_field_type(field),
            markdown_cardinality(field.cardinality),
            markdown_constraints(field)
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    Ok(())
}

fn emit_markdown_extensible_fields(
    output: &mut String,
    fields: &[Field],
) -> Result<(), SchemaError> {
    writeln!(
        output,
        "| Tag | Field | Type | Cardinality | Constraints |\n| ---: | --- | --- | --- | --- |"
    )
    .map_err(|_| SchemaError::Formatting)?;
    for field in fields {
        writeln!(
            output,
            "| `{}` | `{}` | {} | {} | {} |",
            field.tag.expect("validated extensible field tag"),
            field.name,
            markdown_field_type(field),
            markdown_cardinality(field.cardinality),
            markdown_constraints(field)
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    Ok(())
}

fn markdown_inline_field(field: &Field) -> String {
    format!(
        "`{}`: {} ({}){}",
        field.name,
        markdown_field_type(field),
        markdown_cardinality(field.cardinality),
        match markdown_constraints(field) {
            constraints if constraints != "—" => format!(", {constraints}"),
            _ => String::new(),
        }
    )
}

fn markdown_field_type(field: &Field) -> String {
    match field.field_type {
        FieldType::U8 => "`u8`".to_string(),
        FieldType::U16 => "`u16`".to_string(),
        FieldType::U32 => "`u32`".to_string(),
        FieldType::U64 => "`u64`".to_string(),
        FieldType::I32 => "`i32`".to_string(),
        FieldType::I64 => "`i64`".to_string(),
        FieldType::F64 => "`f64`".to_string(),
        FieldType::Bytes => "`bytes`".to_string(),
        FieldType::Bool => "`bool`".to_string(),
        FieldType::Utf8 => "`utf8`".to_string(),
        FieldType::Enumeration | FieldType::Message => field
            .target
            .as_deref()
            .map_or_else(|| "—".to_string(), |target| format!("`{target}`")),
    }
}

const fn markdown_cardinality(cardinality: Cardinality) -> &'static str {
    match cardinality {
        Cardinality::Required => "required",
        Cardinality::Optional => "optional",
        Cardinality::Repeated => "repeated",
    }
}

fn markdown_constraints(field: &Field) -> String {
    let mut values = Vec::new();
    if let Some(constraints) = field.constraints {
        if let Some(numeric) = constraints.numeric() {
            values.push(format!(
                "value {}..={}",
                markdown_numeric_bound(numeric.minimum),
                markdown_numeric_bound(numeric.maximum)
            ));
        }
        if let Some(length) = constraints.length() {
            values.push(format!(
                "bytes {}..={}",
                markdown_size_bound(length.minimum),
                markdown_size_bound(length.maximum)
            ));
        }
        if let Some(items) = constraints.items() {
            values.push(format!(
                "items {}..={}",
                markdown_size_bound(items.minimum),
                markdown_size_bound(items.maximum)
            ));
        }
    }
    if let Some(count_field) = field.counted_by.as_deref() {
        values.push(format!("counted by `{count_field}`"));
    }
    if values.is_empty() {
        "—".to_string()
    } else {
        values.join(", ")
    }
}

fn markdown_numeric_bound(bound: Option<NumericBound>) -> String {
    bound.map_or_else(|| "*".to_string(), |bound| bound.as_i128().to_string())
}

fn markdown_size_bound(bound: Option<u32>) -> String {
    bound.map_or_else(|| "*".to_string(), |bound| bound.to_string())
}

fn generate_capability_rust_resolved(
    schema: &ProtocolSchema,
    resolved: &ProtocolSchema,
) -> Result<String, SchemaError> {
    validate(resolved)?;
    let capability = schema
        .capability
        .as_deref()
        .ok_or(SchemaError::LibraryHasCommands)?;
    let short_name = capability.strip_prefix("pwright.").unwrap_or(capability);
    let rust_name = rust_type_name(short_name)?;
    let display_name = schema.display_name.as_deref().unwrap_or(&rust_name);
    let constant_prefix = screaming_snake_case(short_name);
    let capability_id = format!("{constant_prefix}_CAPABILITY_ID");
    let command_ids = format!("{constant_prefix}_COMMAND_IDS");
    let capability_type = format!("{rust_name}Capability");
    let mut output = String::new();
    writeln!(
        output,
        "// @generated by pwright-schema-gen; do not edit manually."
    )
    .map_err(|_| SchemaError::Formatting)?;
    let dependency_import = if schema.dependencies.is_empty() {
        ""
    } else {
        "CapabilityDependency, "
    };
    writeln!(
        output,
        "use pwright_core::{{Capability, CapabilityCommandDescriptor, CapabilityCommandSpec, {dependency_import}CapabilityDescriptor, CapabilityId, CommandId, CommandSpec, MethodTag, ObjectKindId, PluginInfo, ProtocolVersion}};\n"
    )
    .map_err(|_| SchemaError::Formatting)?;
    let public_names = public_rust_names(resolved);
    let recursive = recursive_model_edges(resolved);
    let non_eq_types = non_eq_type_ids(resolved);
    let command_models = schema
        .commands
        .iter()
        .flat_map(|command| {
            [
                command.params_type.as_str(),
                command.result_type.as_str(),
                command.error_details_type.as_str(),
            ]
        })
        .collect::<BTreeSet<_>>();
    let external_models = schema
        .types
        .iter()
        .filter(|definition| definition.public_external())
        .filter(|definition| command_models.contains(definition.id()))
        .filter_map(TypeDefinition::public_name)
        .map(pascal_case)
        .collect::<Vec<_>>();
    if !external_models.is_empty() {
        writeln!(output, "use super::{{{}}};\n", external_models.join(", "))
            .map_err(|_| SchemaError::Formatting)?;
    }
    for definition in &schema.types {
        if definition.public_name().is_some() && !definition.public_external() {
            emit_public_model(
                &mut output,
                definition,
                &public_names,
                &recursive,
                &non_eq_types,
            )?;
        }
    }
    writeln!(
        output,
        "pub const {capability_id}: CapabilityId = CapabilityId::new({capability:?});"
    )
    .map_err(|_| SchemaError::Formatting)?;
    for command in &schema.commands {
        let command_prefix = screaming_snake_case(&command.name);
        writeln!(
            output,
            "pub const {command_prefix}_COMMAND_ID: CommandId = CommandId::new({:?});",
            command.id
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "pub const {command_prefix}_METHOD: MethodTag = MethodTag::new({});",
            command.method
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "pub const {command_prefix}_TARGET_OBJECT_KINDS: &[ObjectKindId] = &["
        )
        .map_err(|_| SchemaError::Formatting)?;
        for target in &command.targets {
            writeln!(output, "    ObjectKindId::new({target:?}),")
                .map_err(|_| SchemaError::Formatting)?;
        }
        writeln!(output, "];").map_err(|_| SchemaError::Formatting)?;
    }
    let command_descriptors = format!("{constant_prefix}_COMMAND_DESCRIPTORS");
    writeln!(
        output,
        "\npub const {command_descriptors}: &[CapabilityCommandDescriptor] = &["
    )
    .map_err(|_| SchemaError::Formatting)?;
    for command in &schema.commands {
        let command_prefix = screaming_snake_case(&command.name);
        writeln!(output, "    CapabilityCommandDescriptor::new(")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "        {command_prefix}_COMMAND_ID,")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "        {command_prefix}_METHOD,")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "        {command_prefix}_TARGET_OBJECT_KINDS,")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "    ),").map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "];\n").map_err(|_| SchemaError::Formatting)?;
    let dependency_ids = format!("{constant_prefix}_DEPENDENCIES");
    if !schema.dependencies.is_empty() {
        writeln!(
            output,
            "\nconst {dependency_ids}: &[CapabilityDependency] = &["
        )
        .map_err(|_| SchemaError::Formatting)?;
        for dependency in &schema.dependencies {
            writeln!(
                output,
                "    CapabilityDependency::new(CapabilityId::new({:?}), ProtocolVersion::new({}, {})),",
                dependency.id, dependency.minimum_version.major, dependency.minimum_version.minor
            )
            .map_err(|_| SchemaError::Formatting)?;
        }
        writeln!(output, "];\n").map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "\nconst {command_ids}: &[CommandId] = &[")
        .map_err(|_| SchemaError::Formatting)?;
    for command in &schema.commands {
        writeln!(
            output,
            "    {}_COMMAND_ID,",
            screaming_snake_case(&command.name)
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "];\n").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "pub struct {capability_type};\n").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "impl Capability for {capability_type} {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "    const DESCRIPTOR: CapabilityDescriptor = CapabilityDescriptor::new("
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "        PluginInfo::new({capability_id}, {display_name:?}, ProtocolVersion::new({}, {})),",
        schema.version.major, schema.version.minor
    )
    .map_err(|_| SchemaError::Formatting)?;
    if schema.dependencies.is_empty() {
        writeln!(output, "        &[],").map_err(|_| SchemaError::Formatting)?;
    } else {
        writeln!(output, "        {dependency_ids},").map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "        {command_ids},").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    );\n}}").map_err(|_| SchemaError::Formatting)?;
    for command in &schema.commands {
        emit_public_command_spec(&mut output, command, &public_names, &capability_type)?;
    }
    Ok(output)
}

fn public_rust_names(schema: &ProtocolSchema) -> BTreeMap<&str, String> {
    schema
        .types
        .iter()
        .filter_map(|definition| {
            definition
                .public_name()
                .map(|name| (definition.id(), pascal_case(name)))
        })
        .collect()
}

fn emit_public_model(
    output: &mut String,
    definition: &TypeDefinition,
    public_names: &BTreeMap<&str, String>,
    recursive: &RecursiveModelEdges,
    non_eq_types: &BTreeSet<&str>,
) -> Result<(), SchemaError> {
    let name = &public_names[definition.id()];
    match definition {
        TypeDefinition::Enumeration {
            default, variants, ..
        } => {
            let default_derive = if default.is_some() { ", Default" } else { "" };
            writeln!(
                output,
                "#[derive(Clone, Copy, Debug, Eq, PartialEq{default_derive})]"
            )
            .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "pub enum {name} {{").map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                if default.as_deref() == Some(&variant.name) {
                    writeln!(output, "    #[default]").map_err(|_| SchemaError::Formatting)?;
                }
                writeln!(output, "    {},", pascal_case(&variant.name))
                    .map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::Record { fields, .. } => {
            let eq_derive = if non_eq_types.contains(definition.id()) {
                ""
            } else {
                ", Eq"
            };
            writeln!(output, "#[derive(Clone, Debug{eq_derive}, PartialEq)]")
                .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "pub struct {name} {{").map_err(|_| SchemaError::Formatting)?;
            emit_public_fields(
                output,
                definition.id(),
                fields,
                public_names,
                recursive,
                "    pub ",
            )?;
            writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::TaggedUnion { variants, .. } => {
            let eq_derive = if non_eq_types.contains(definition.id()) {
                ""
            } else {
                ", Eq"
            };
            writeln!(output, "#[derive(Clone, Debug{eq_derive}, PartialEq)]")
                .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "pub enum {name} {{").map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                let variant_name = pascal_case(&variant.name);
                match variant.fields.as_slice() {
                    [] => writeln!(output, "    {variant_name},")
                        .map_err(|_| SchemaError::Formatting)?,
                    [field] => writeln!(
                        output,
                        "    {variant_name}({}),",
                        public_field_type(definition.id(), field, public_names, recursive)
                    )
                    .map_err(|_| SchemaError::Formatting)?,
                    fields => {
                        writeln!(output, "    {variant_name} {{")
                            .map_err(|_| SchemaError::Formatting)?;
                        emit_public_fields(
                            output,
                            definition.id(),
                            fields,
                            public_names,
                            recursive,
                            "        ",
                        )?;
                        writeln!(output, "    }},").map_err(|_| SchemaError::Formatting)?;
                    }
                }
            }
            writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        }
    }
    Ok(())
}

fn emit_public_fields(
    output: &mut String,
    owner: &str,
    fields: &[Field],
    public_names: &BTreeMap<&str, String>,
    recursive: &RecursiveModelEdges,
    prefix: &str,
) -> Result<(), SchemaError> {
    for field in fields {
        writeln!(
            output,
            "{prefix}{}: {},",
            field.name,
            public_field_type(owner, field, public_names, recursive)
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    Ok(())
}

fn public_field_type(
    owner: &str,
    field: &Field,
    public_names: &BTreeMap<&str, String>,
    recursive: &RecursiveModelEdges,
) -> String {
    let base = match field.field_type {
        FieldType::U8 => "u8".to_owned(),
        FieldType::U16 => "u16".to_owned(),
        FieldType::U32 => "u32".to_owned(),
        FieldType::U64 => "u64".to_owned(),
        FieldType::I32 => "i32".to_owned(),
        FieldType::I64 => "i64".to_owned(),
        FieldType::F64 => "f64".to_owned(),
        FieldType::Bytes => "Vec<u8>".to_owned(),
        FieldType::Bool => "bool".to_owned(),
        FieldType::Utf8 => "String".to_owned(),
        FieldType::Enumeration | FieldType::Message => {
            public_names[field.target.as_deref().expect("validated public target")].clone()
        }
    };
    let base = if field_requires_box(owner, field, recursive) {
        format!("Box<{base}>")
    } else {
        base
    };
    match field.cardinality {
        Cardinality::Required => base,
        Cardinality::Optional => format!("Option<{base}>"),
        Cardinality::Repeated => format!("Vec<{base}>"),
    }
}

fn emit_public_command_spec(
    output: &mut String,
    command: &CommandDefinition,
    public_names: &BTreeMap<&str, String>,
    capability_type: &str,
) -> Result<(), SchemaError> {
    let command_type = pascal_case(&command.name);
    let prefix = screaming_snake_case(&command.name);
    writeln!(output, "\npub struct {command_type};\n").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "impl CommandSpec for {command_type} {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    const METHOD: MethodTag = {prefix}_METHOD;")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "    const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = {prefix}_TARGET_OBJECT_KINDS;"
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "    type Params = {};",
        public_names[command.params_type.as_str()]
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "    type Result = {};",
        public_names[command.result_type.as_str()]
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "    type ErrorDetails = {};\n}}",
        public_names[command.error_details_type.as_str()]
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "\nimpl CapabilityCommandSpec for {command_type} {{\n    type Capability = {capability_type};\n    const COMMAND: CommandId = {prefix}_COMMAND_ID;\n}}"
    )
    .map_err(|_| SchemaError::Formatting)
}

fn emit_public_conversions(
    output: &mut String,
    definition: &TypeDefinition,
    rust_names: &BTreeMap<&str, String>,
    public_names: &BTreeMap<&str, String>,
    recursive: &RecursiveModelEdges,
) -> Result<(), SchemaError> {
    let wire = &rust_names[definition.id()];
    let public = format!("Public{}", public_names[definition.id()]);
    match definition {
        TypeDefinition::Enumeration { variants, .. } => {
            writeln!(output, "impl From<{wire}> for {public} {{")
                .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "    fn from(value: {wire}) -> Self {{")
                .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "        match value {{").map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                let name = pascal_case(&variant.name);
                writeln!(output, "            {wire}::{name} => Self::{name},")
                    .map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "        }}\n    }}\n}}\n").map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "impl From<{public}> for {wire} {{")
                .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "    fn from(value: {public}) -> Self {{")
                .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "        match value {{").map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                let name = pascal_case(&variant.name);
                writeln!(output, "            {public}::{name} => Self::{name},")
                    .map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "        }}\n    }}\n}}\n").map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::Record { fields, .. } => {
            emit_record_conversion(output, definition.id(), wire, &public, fields, recursive)?;
            emit_record_conversion(output, definition.id(), &public, wire, fields, recursive)?;
        }
        TypeDefinition::TaggedUnion { variants, .. } => {
            emit_union_conversion(
                output,
                definition.id(),
                wire,
                &public,
                variants,
                true,
                recursive,
            )?;
            emit_union_conversion(
                output,
                definition.id(),
                &public,
                wire,
                variants,
                false,
                recursive,
            )?;
        }
    }
    Ok(())
}

fn emit_record_conversion(
    output: &mut String,
    owner: &str,
    source: &str,
    target: &str,
    fields: &[Field],
    recursive: &RecursiveModelEdges,
) -> Result<(), SchemaError> {
    writeln!(output, "impl From<{source}> for {target} {{").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    fn from(value: {source}) -> Self {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "        Self {{").map_err(|_| SchemaError::Formatting)?;
    for field in fields {
        writeln!(
            output,
            "            {}: {},",
            field.name,
            public_conversion_expression(owner, field, &format!("value.{}", field.name), recursive,)
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "        }}\n    }}\n}}\n").map_err(|_| SchemaError::Formatting)
}

fn emit_union_conversion(
    output: &mut String,
    owner: &str,
    source: &str,
    target: &str,
    variants: &[Variant],
    wire_to_public: bool,
    recursive: &RecursiveModelEdges,
) -> Result<(), SchemaError> {
    writeln!(output, "impl From<{source}> for {target} {{").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    fn from(value: {source}) -> Self {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "        match value {{").map_err(|_| SchemaError::Formatting)?;
    for variant in variants {
        let name = pascal_case(&variant.name);
        match variant.fields.as_slice() {
            [] => writeln!(output, "            {source}::{name} => Self::{name},")
                .map_err(|_| SchemaError::Formatting)?,
            [field] if wire_to_public => writeln!(
                output,
                "            {source}::{name} {{ {} }} => Self::{name}({}),",
                field.name,
                public_conversion_expression(owner, field, &field.name, recursive)
            )
            .map_err(|_| SchemaError::Formatting)?,
            [field] => writeln!(
                output,
                "            {source}::{name}({}) => Self::{name} {{ {} }},",
                field.name,
                union_field_initializer(owner, field, recursive)
            )
            .map_err(|_| SchemaError::Formatting)?,
            fields => {
                let bindings = fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                writeln!(
                    output,
                    "            {source}::{name} {{ {bindings} }} => Self::{name} {{"
                )
                .map_err(|_| SchemaError::Formatting)?;
                for field in fields {
                    writeln!(
                        output,
                        "                {},",
                        union_field_initializer(owner, field, recursive)
                    )
                    .map_err(|_| SchemaError::Formatting)?;
                }
                writeln!(output, "            }},").map_err(|_| SchemaError::Formatting)?;
            }
        }
    }
    writeln!(output, "        }}\n    }}\n}}\n").map_err(|_| SchemaError::Formatting)
}

fn union_field_initializer(owner: &str, field: &Field, recursive: &RecursiveModelEdges) -> String {
    let converted = public_conversion_expression(owner, field, &field.name, recursive);
    if converted == field.name {
        field.name.clone()
    } else {
        format!("{}: {converted}", field.name)
    }
}

fn public_conversion_expression(
    owner: &str,
    field: &Field,
    value: &str,
    recursive: &RecursiveModelEdges,
) -> String {
    if matches!(
        field.field_type,
        FieldType::Enumeration | FieldType::Message
    ) {
        if field_requires_box(owner, field, recursive) {
            return match field.cardinality {
                Cardinality::Required => format!("Box::new((*{value}).into())"),
                Cardinality::Optional => {
                    format!("{value}.map(|value| Box::new((*value).into()))")
                }
                Cardinality::Repeated => unreachable!("recursive repeated fields use Vec"),
            };
        }
        match field.cardinality {
            Cardinality::Required => format!("{value}.into()"),
            Cardinality::Optional => format!("{value}.map(Into::into)"),
            Cardinality::Repeated => {
                format!("{value}.into_iter().map(Into::into).collect()")
            }
        }
    } else {
        value.to_owned()
    }
}

fn generate_rust_resolved(
    schema: &ProtocolSchema,
    resolved: &ProtocolSchema,
    imports: &[ProtocolSchema],
) -> Result<String, SchemaError> {
    validate(resolved)?;
    let rust_names = rust_type_names(resolved)?;
    let recursive = recursive_model_edges(resolved);
    let non_eq_types = non_eq_type_ids(resolved);
    let borrowed_types = borrowed_type_ids(resolved);
    let borrowed_rust_names = borrowed_rust_type_names(&rust_names, &borrowed_types);
    let minimum_sizes = minimum_wire_sizes(resolved);
    let mut output = String::new();
    writeln!(
        output,
        "// @generated by pwright-schema-gen; do not edit manually."
    )
    .map_err(|_| SchemaError::Formatting)?;
    emit_rust_imports(&mut output, schema)?;
    if schema.commands.is_empty() {
        writeln!(output, "use pwright_core::ProtocolVersion;\n")
            .map_err(|_| SchemaError::Formatting)?;
    } else {
        writeln!(
            output,
            "use pwright_core::{{CommandId, MethodTag, ProtocolVersion}};\n"
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    emit_imported_rust_uses(&mut output, imports, &borrowed_types)?;
    emit_public_rust_uses(&mut output, schema)?;

    for definition in &schema.types {
        emit_model(
            &mut output,
            definition,
            &rust_names,
            &recursive,
            &non_eq_types,
        )?;
    }
    for definition in &schema.types {
        if borrowed_types.contains(definition.id()) {
            emit_borrowed_model(
                &mut output,
                definition,
                &rust_names,
                &borrowed_rust_names,
                &recursive,
            )?;
        }
    }
    for definition in &schema.types {
        emit_codec_impl(
            &mut output,
            definition,
            &rust_names,
            &minimum_sizes,
            &recursive,
        )?;
    }
    for definition in &schema.types {
        if borrowed_types.contains(definition.id()) {
            emit_borrowed_codec_impl(&mut output, definition, &borrowed_rust_names)?;
        }
    }
    let public_names = public_rust_names(resolved);
    for definition in &schema.types {
        if definition.public_name().is_some() && !definition.public_external() {
            emit_public_conversions(
                &mut output,
                definition,
                &rust_names,
                &public_names,
                &recursive,
            )?;
        }
    }
    for (type_index, definition) in schema.types.iter().enumerate() {
        emit_schema_constants(&mut output, schema.version, type_index, definition)?;
    }
    emit_command_constants(&mut output, schema)?;
    writeln!(output, "const TYPES: &[BinaryTypeSchema] = &[")
        .map_err(|_| SchemaError::Formatting)?;
    for definition in &resolved.types {
        let schema_name = schema_constant_name(definition.id());
        match definition {
            TypeDefinition::Enumeration { .. } => {
                writeln!(output, "    BinaryTypeSchema::Enumeration({schema_name}),")
            }
            TypeDefinition::Record {
                encoding: RecordEncoding::Positional,
                ..
            } => {
                writeln!(output, "    BinaryTypeSchema::Record({schema_name}),")
            }
            TypeDefinition::Record {
                encoding: RecordEncoding::Extensible,
                ..
            } => writeln!(
                output,
                "    BinaryTypeSchema::ExtensibleRecord({schema_name}),"
            ),
            TypeDefinition::TaggedUnion { .. } => {
                writeln!(output, "    BinaryTypeSchema::TaggedUnion({schema_name}),")
            }
        }
        .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "];\n").map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "pub const CATALOG: BinarySchemaCatalog = BinarySchemaCatalog::new(TYPES);"
    )
    .map_err(|_| SchemaError::Formatting)?;
    if let Some(root) = schema.root() {
        let root_index = schema
            .types
            .binary_search_by(|definition| definition.id().cmp(root))
            .map_err(|_| SchemaError::RootMissing {
                id: root.to_owned(),
            })?;
        writeln!(
            output,
            "pub const ROOT_SCHEMA: BinarySchema = {};",
            schema_constant_name(schema.types[root_index].id())
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    Ok(output)
}

fn emit_imported_rust_uses(
    output: &mut String,
    imports: &[ProtocolSchema],
    borrowed_types: &BTreeSet<&str>,
) -> Result<(), SchemaError> {
    for imported in imports {
        let namespace = imported.namespace().expect("validated import namespace");
        let crate_name = rust_import_crate_name(namespace);
        let mut names = Vec::new();
        for definition in &imported.types {
            let rust_name = rust_type_name(definition.id())?;
            names.push(rust_name.clone());
            if borrowed_types.contains(definition.id()) {
                names.push(format!("{rust_name}Ref"));
            }
            names.push(schema_constant_name(definition.id()));
        }
        writeln!(output, "pub use {crate_name}::{{{}}};", names.join(", "))
            .map_err(|_| SchemaError::Formatting)?;
    }
    if !imports.is_empty() {
        writeln!(output).map_err(|_| SchemaError::Formatting)?;
    }
    Ok(())
}

fn emit_public_rust_uses(output: &mut String, schema: &ProtocolSchema) -> Result<(), SchemaError> {
    let public_names = schema
        .types
        .iter()
        .filter(|definition| !definition.public_external())
        .filter_map(|definition| {
            definition
                .public_name()
                .map(|name| (definition.id(), pascal_case(name)))
        })
        .collect::<BTreeMap<_, _>>();
    if public_names.is_empty() {
        return Ok(());
    }
    let capability = schema
        .capability
        .as_deref()
        .expect("validated public capability models");
    let crate_name = match capability.strip_prefix("pwright.") {
        Some(suffix) => format!("pwright_cap_{}", suffix.replace('.', "_")),
        None => format!("{}_cap", capability.replace('.', "_")),
    };
    let imports = public_names
        .values()
        .map(|name| format!("{name} as Public{name}"))
        .collect::<Vec<_>>()
        .join(", ");
    let module = schema
        .rust_minor_namespace()
        .map_or_else(String::new, |module| format!("{module}::"));
    writeln!(output, "use {crate_name}::{module}{{{imports}}};\n")
        .map_err(|_| SchemaError::Formatting)
}

fn rust_import_crate_name(namespace: &str) -> String {
    match namespace.strip_prefix("pwright.") {
        Some(suffix) => format!("pwright_codec_{}", suffix.replace('.', "_")),
        None => namespace.replace('.', "_"),
    }
}

fn emit_rust_imports(output: &mut String, schema: &ProtocolSchema) -> Result<(), SchemaError> {
    let mut imports = BTreeSet::from([
        "BinaryDecode",
        "BinaryEncode",
        "BinaryReader",
        "BinarySchemaCatalog",
        "BinaryTypeSchema",
        "BinaryValueContext",
        "BinaryValueDecodeError",
        "BinaryValueEncodeError",
        "BinaryWriter",
    ]);
    let fields = schema
        .types
        .iter()
        .flat_map(definition_fields)
        .collect::<Vec<_>>();
    if !fields.is_empty() {
        imports.extend(["BinaryCardinality", "BinaryFieldKind", "BinaryFieldSchema"]);
    }
    if fields.iter().any(|field| {
        field
            .constraints
            .is_some_and(|constraints| constraints.numeric().is_some())
    }) {
        imports.insert("BinaryNumericRange");
    }
    if fields.iter().any(|field| {
        field.constraints.is_some_and(|constraints| {
            constraints.length().is_some() || constraints.items().is_some()
        }) || field.counted_by.is_some()
    }) || schema.types.iter().any(|definition| {
        matches!(
            definition,
            TypeDefinition::Record {
                encoding: RecordEncoding::Extensible,
                ..
            }
        )
    }) {
        imports.insert("BinarySizeRange");
    }
    if fields.iter().any(|field| field.counted_by.is_some()) {
        imports.insert("BinaryDecodeError");
    }
    if fields
        .iter()
        .any(|field| field.cardinality == Cardinality::Repeated)
    {
        imports.insert("BinaryEncodeSequenceRef");
    }
    if !schema.commands.is_empty() {
        imports.insert("BinaryCommandSchema");
    }
    if schema
        .types
        .iter()
        .any(|definition| matches!(definition, TypeDefinition::Enumeration { .. }))
    {
        imports.extend(["BinaryEnumSchema", "BinaryEnumVariantSchema"]);
    }
    if schema.types.iter().any(|definition| {
        matches!(
            definition,
            TypeDefinition::Record {
                encoding: RecordEncoding::Positional,
                ..
            }
        )
    }) {
        imports.insert("BinaryRecordSchema");
    }
    if schema.types.iter().any(|definition| {
        matches!(
            definition,
            TypeDefinition::Record {
                encoding: RecordEncoding::Extensible,
                ..
            }
        )
    }) {
        imports.extend([
            "BinaryExtensibleFieldSchema",
            "BinaryExtensibleRecordSchema",
        ]);
    }
    if schema
        .types
        .iter()
        .any(|definition| matches!(definition, TypeDefinition::TaggedUnion { .. }))
    {
        imports.extend(["BinarySchema", "BinaryVariantSchema"]);
    }
    writeln!(
        output,
        "use pwright_codec_binary::{{{}}};",
        imports.into_iter().collect::<Vec<_>>().join(", ")
    )
    .map_err(|_| SchemaError::Formatting)
}

fn validate(schema: &ProtocolSchema) -> Result<(), SchemaError> {
    if schema.rust_minor_module && schema.capability.is_none() {
        return Err(SchemaError::RustMinorModuleRequiresCapability);
    }
    if schema.format != SCHEMA_FORMAT {
        return Err(SchemaError::UnsupportedFormat {
            actual: schema.format,
            supported: SCHEMA_FORMAT,
        });
    }
    let identity_count = usize::from(schema.capability.is_some())
        + usize::from(schema.library.is_some())
        + usize::from(schema.protocol.is_some());
    if identity_count == 0 {
        return Err(SchemaError::MissingSchemaIdentity);
    }
    if identity_count != 1 {
        return Err(SchemaError::ConflictingSchemaIdentity);
    }
    let namespace = schema.namespace().expect("exactly one schema identity");
    if namespace.is_empty() {
        return Err(SchemaError::EmptyCapability);
    }
    match schema
        .identity_kind()
        .expect("exactly one schema identity kind")
    {
        SchemaIdentityKind::Capability => {
            let root = schema.root().ok_or(SchemaError::CapabilityRootMissing)?;
            if root.is_empty() {
                return Err(SchemaError::EmptyRoot);
            }
        }
        SchemaIdentityKind::Library => {
            if schema.root().is_some() {
                return Err(SchemaError::LibraryHasRoot);
            }
            if !schema.commands.is_empty() {
                return Err(SchemaError::LibraryHasCommands);
            }
        }
        SchemaIdentityKind::Protocol => {
            let root = schema.root().ok_or(SchemaError::ProtocolRootMissing)?;
            if root.is_empty() {
                return Err(SchemaError::EmptyRoot);
            }
            if !schema.commands.is_empty() {
                return Err(SchemaError::ProtocolHasCommands);
            }
        }
    }
    if schema.protocol.is_some() && !schema.dependencies.is_empty() {
        return Err(SchemaError::ProtocolHasDependencies);
    }
    validate_dependencies(schema.capability.as_deref(), &schema.dependencies)?;
    if schema.types.is_empty() {
        return Err(SchemaError::EmptyTypes);
    }
    if schema.types.len() > MAX_TYPES {
        return Err(SchemaError::TooManyTypes {
            actual: schema.types.len(),
            limit: MAX_TYPES,
        });
    }
    let mut previous_id: Option<&str> = None;
    let ids = schema
        .types
        .iter()
        .map(TypeDefinition::id)
        .collect::<BTreeSet<_>>();
    let mut rust_names = BTreeSet::new();
    let mut public_names = BTreeSet::new();
    for definition in &schema.types {
        let id = definition.id();
        if let Some(previous) = previous_id
            && previous >= id
        {
            return Err(SchemaError::NonCanonicalTypeOrder {
                previous: previous.to_owned(),
                current: id.to_owned(),
            });
        }
        previous_id = Some(id);
        let rust_name = rust_type_name(id)?;
        if !rust_names.insert(rust_name.clone()) {
            return Err(SchemaError::DuplicateRustTypeName { name: rust_name });
        }
        if let Some(public_name) = definition.public_name() {
            if schema.capability.is_none() && !definition.public_external() {
                return Err(SchemaError::GeneratedPublicModelsRequireCapability);
            }
            validate_identifier("public type", public_name).map_err(|_| {
                SchemaError::InvalidPublicTypeName {
                    type_id: id.to_owned(),
                    name: public_name.to_owned(),
                }
            })?;
            if !public_names.insert(public_name) {
                return Err(SchemaError::DuplicatePublicTypeName {
                    name: public_name.to_owned(),
                });
            }
        } else if definition.public_external() {
            return Err(SchemaError::ExternalPublicModelMissingName {
                type_id: id.to_owned(),
            });
        }
        match definition {
            TypeDefinition::Enumeration {
                default, variants, ..
            } => {
                validate_enum_variants(id, variants, default.as_deref())?;
            }
            TypeDefinition::Record {
                encoding,
                encoded_size,
                fields,
                ..
            } => {
                validate_record_encoding(id, *encoding, *encoded_size, fields)?;
                validate_fields(id, fields, &ids)?;
            }
            TypeDefinition::TaggedUnion { variants, .. } => {
                if variants.is_empty() {
                    return Err(SchemaError::EmptyVariants {
                        type_id: id.to_owned(),
                    });
                }
                if variants.len() > MAX_VARIANTS_PER_TYPE {
                    return Err(SchemaError::TooManyVariants {
                        type_id: id.to_owned(),
                        actual: variants.len(),
                        limit: MAX_VARIANTS_PER_TYPE,
                    });
                }
                let mut previous_tag = None;
                let mut names = BTreeSet::new();
                for variant in variants {
                    validate_identifier("variant", &variant.name)?;
                    if !names.insert(variant.name.as_str()) {
                        return Err(SchemaError::InvalidIdentifier {
                            kind: "duplicate variant",
                            value: variant.name.clone(),
                        });
                    }
                    if let Some(previous) = previous_tag
                        && previous >= variant.tag
                    {
                        return Err(SchemaError::NonCanonicalVariantOrder {
                            type_id: id.to_owned(),
                            previous,
                            current: variant.tag,
                        });
                    }
                    previous_tag = Some(variant.tag);
                    validate_positional_field_tags(
                        &format!("{id}.{}", variant.name),
                        &variant.fields,
                    )?;
                    validate_fields(&format!("{id}.{}", variant.name), &variant.fields, &ids)?;
                }
            }
        }
    }
    for definition in &schema.types {
        for field in definition_fields(definition) {
            let Some(target) = field.target.as_deref() else {
                continue;
            };
            let target_definition = &schema.types[schema
                .types
                .binary_search_by(|definition| definition.id().cmp(target))
                .expect("validated type target")];
            let valid = matches!(
                (field.field_type, target_definition),
                (FieldType::Message, TypeDefinition::Record { .. })
                    | (FieldType::Message, TypeDefinition::TaggedUnion { .. })
                    | (FieldType::Enumeration, TypeDefinition::Enumeration { .. })
            );
            if !valid {
                return Err(SchemaError::WrongTargetKind {
                    owner: definition.id().to_owned(),
                    field: field.name.clone(),
                    target: target.to_owned(),
                });
            }
            if definition.public_name().is_some()
                && !definition.public_external()
                && target_definition.public_name().is_none()
            {
                return Err(SchemaError::PublicModelTargetMissingName {
                    owner: definition.id().to_owned(),
                    target: target.to_owned(),
                });
            }
        }
    }
    if let Some(root) = schema.root() {
        let root_variants = match schema
            .types
            .binary_search_by(|definition| definition.id().cmp(root))
        {
            Ok(index) => match &schema.types[index] {
                TypeDefinition::TaggedUnion { variants, .. } => variants,
                _ => {
                    return Err(SchemaError::RootMustBeTaggedUnion {
                        id: root.to_owned(),
                    });
                }
            },
            Err(_) => {
                return Err(SchemaError::RootMissing {
                    id: root.to_owned(),
                });
            }
        };
        validate_commands(&schema.commands, root_variants, &schema.types)?;
    }
    validate_model_inhabitance(schema)?;
    Ok(())
}

fn validate_commands(
    commands: &[CommandDefinition],
    root_variants: &[Variant],
    types: &[TypeDefinition],
) -> Result<(), SchemaError> {
    let mut previous: Option<&str> = None;
    let mut methods = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut used_variants = BTreeSet::new();
    for command in commands {
        validate_identifier("command", &command.name)?;
        if !names.insert(command.name.as_str()) {
            return Err(SchemaError::InvalidIdentifier {
                kind: "duplicate command",
                value: command.name.clone(),
            });
        }
        if !valid_command_id(&command.id) {
            return Err(SchemaError::InvalidCommandId {
                value: command.id.clone(),
            });
        }
        if let Some(previous) = previous
            && previous >= command.id.as_str()
        {
            return Err(SchemaError::NonCanonicalCommandOrder {
                previous: previous.to_owned(),
                current: command.id.clone(),
            });
        }
        previous = Some(&command.id);
        if !methods.insert(command.method) {
            return Err(SchemaError::DuplicateCommandMethod {
                method: command.method,
            });
        }
        if command.targets.is_empty() {
            return Err(SchemaError::EmptyCommandTargets {
                command: command.id.clone(),
            });
        }
        let mut previous_target: Option<&str> = None;
        for target in &command.targets {
            if rust_type_name(target).is_err() {
                return Err(SchemaError::InvalidCommandTarget {
                    command: command.id.clone(),
                    target: target.clone(),
                });
            }
            if let Some(previous) = previous_target
                && previous >= target
            {
                return Err(SchemaError::NonCanonicalCommandTargetOrder {
                    command: command.id.clone(),
                    previous: previous.to_owned(),
                    current: target.clone(),
                });
            }
            previous_target = Some(target);
        }
        for (role, variant) in [
            ("command", command.command.as_str()),
            ("result", command.result.as_str()),
        ] {
            if !root_variants
                .iter()
                .any(|candidate| candidate.name == variant)
            {
                return Err(SchemaError::UnknownCommandVariant {
                    command: command.id.clone(),
                    role,
                    variant: variant.to_owned(),
                });
            }
            if !used_variants.insert(variant) {
                return Err(SchemaError::ReusedCommandVariant {
                    variant: variant.to_owned(),
                });
            }
        }
        for (role, type_id, expected) in [
            ("params", command.params_type.as_str(), "record"),
            ("result", command.result_type.as_str(), "record"),
            (
                "error details",
                command.error_details_type.as_str(),
                "error",
            ),
        ] {
            let definition = types
                .binary_search_by(|definition| definition.id().cmp(type_id))
                .ok()
                .map(|index| &types[index])
                .ok_or_else(|| SchemaError::UnknownCommandModel {
                    command: command.id.clone(),
                    role,
                    type_id: type_id.to_owned(),
                })?;
            let compatible = match expected {
                "record" => matches!(definition, TypeDefinition::Record { .. }),
                "error" => matches!(
                    definition,
                    TypeDefinition::Enumeration { .. } | TypeDefinition::TaggedUnion { .. }
                ),
                _ => unreachable!(),
            };
            if !compatible {
                return Err(SchemaError::WrongCommandModelKind {
                    command: command.id.clone(),
                    role,
                    type_id: type_id.to_owned(),
                });
            }
            if definition.public_name().is_none() {
                return Err(SchemaError::CommandModelMissingPublicName {
                    command: command.id.clone(),
                    role,
                    type_id: type_id.to_owned(),
                });
            }
        }
    }
    Ok(())
}

fn valid_command_id(value: &str) -> bool {
    !value.is_empty()
        && !matches!(value.as_bytes().first(), Some(b'.' | b'-'))
        && !matches!(value.as_bytes().last(), Some(b'.' | b'-'))
        && !value.contains("..")
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
}

fn validate_record_encoding(
    type_id: &str,
    encoding: RecordEncoding,
    encoded_size: Option<SizeConstraints>,
    fields: &[Field],
) -> Result<(), SchemaError> {
    match encoding {
        RecordEncoding::Positional => {
            if encoded_size.is_some() {
                return Err(SchemaError::UnexpectedRecordEncodedSize {
                    type_id: type_id.to_owned(),
                });
            }
            validate_positional_field_tags(type_id, fields)
        }
        RecordEncoding::Extensible => {
            let valid_range = encoded_size
                .is_some_and(|range| range.maximum.is_some() && !invalid_size_constraints(range));
            if !valid_range {
                return Err(SchemaError::MissingOrInvalidExtensibleRecordEncodedSize {
                    type_id: type_id.to_owned(),
                    range: encoded_size
                        .map(|range| SizeConstraintRange::new(range.minimum, range.maximum)),
                });
            }
            let mut previous_tag = None;
            for field in fields {
                let tag = field
                    .tag
                    .ok_or_else(|| SchemaError::MissingExtensibleFieldTag {
                        owner: type_id.to_owned(),
                        field: field.name.clone(),
                    })?;
                if tag == 0 || previous_tag.is_some_and(|previous| previous >= tag) {
                    return Err(SchemaError::NonCanonicalExtensibleFieldTag {
                        type_id: type_id.to_owned(),
                        previous: previous_tag,
                        current: tag,
                    });
                }
                previous_tag = Some(tag);
                if field.counted_by.is_some() {
                    return Err(SchemaError::ExtensibleFieldUsesExternalCount {
                        owner: type_id.to_owned(),
                        field: field.name.clone(),
                    });
                }
            }
            Ok(())
        }
    }
}

fn validate_positional_field_tags(owner: &str, fields: &[Field]) -> Result<(), SchemaError> {
    if let Some(field) = fields.iter().find(|field| field.tag.is_some()) {
        return Err(SchemaError::UnexpectedExtensibleFieldTag {
            owner: owner.to_owned(),
            field: field.name.clone(),
            tag: field.tag.expect("matched present tag"),
        });
    }
    Ok(())
}

fn validate_fields(owner: &str, fields: &[Field], ids: &BTreeSet<&str>) -> Result<(), SchemaError> {
    if fields.len() > MAX_FIELDS_PER_SHAPE {
        return Err(SchemaError::TooManyFields {
            owner: owner.to_owned(),
            actual: fields.len(),
            limit: MAX_FIELDS_PER_SHAPE,
        });
    }
    let mut names = BTreeSet::new();
    for (index, field) in fields.iter().enumerate() {
        validate_identifier("field", &field.name)?;
        if !names.insert(field.name.as_str()) {
            return Err(SchemaError::DuplicateField {
                owner: owner.to_owned(),
                name: field.name.clone(),
            });
        }
        validate_constraints(owner, field)?;
        if let Some(count_field) = field.counted_by.as_deref() {
            validate_identifier("count field", count_field)?;
            if field.cardinality != Cardinality::Repeated {
                return Err(SchemaError::CountedByOnNonRepeatedField {
                    owner: owner.to_owned(),
                    field: field.name.clone(),
                });
            }
            let count = fields[..index]
                .iter()
                .find(|candidate| candidate.name == count_field)
                .ok_or_else(|| SchemaError::CountFieldMissingOrNotEarlier {
                    owner: owner.to_owned(),
                    field: field.name.clone(),
                    count_field: count_field.to_owned(),
                })?;
            if count.field_type != FieldType::U32 || count.cardinality != Cardinality::Required {
                return Err(SchemaError::InvalidCountField {
                    owner: owner.to_owned(),
                    field: field.name.clone(),
                    count_field: count_field.to_owned(),
                });
            }
        }
        match (field.field_type, field.target.as_deref()) {
            (FieldType::Message | FieldType::Enumeration, None) => {
                return Err(SchemaError::MessageTargetMissing {
                    owner: owner.to_owned(),
                    field: field.name.clone(),
                });
            }
            (FieldType::Message | FieldType::Enumeration, Some(target))
                if !ids.contains(target) =>
            {
                return Err(SchemaError::UnknownTarget {
                    owner: owner.to_owned(),
                    field: field.name.clone(),
                    target: target.to_owned(),
                });
            }
            (FieldType::Message | FieldType::Enumeration, Some(_)) | (_, None) => {}
            (_, Some(_)) => {
                return Err(SchemaError::UnexpectedTarget {
                    owner: owner.to_owned(),
                    field: field.name.clone(),
                });
            }
        }
    }
    Ok(())
}

fn validate_constraints(owner: &str, field: &Field) -> Result<(), SchemaError> {
    let Some(constraints) = field.constraints else {
        return Ok(());
    };
    if constraints.is_empty() {
        return Err(SchemaError::EmptyConstraints {
            owner: owner.to_owned(),
            field: field.name.clone(),
        });
    }
    if let Some(numeric) = constraints.numeric() {
        let Some((type_minimum, type_maximum)) = field.field_type.integer_bounds() else {
            return Err(SchemaError::UnexpectedNumericConstraints {
                owner: owner.to_owned(),
                field: field.name.clone(),
            });
        };
        let minimum = numeric.minimum.map(NumericBound::as_i128);
        let maximum = numeric.maximum.map(NumericBound::as_i128);
        let invalid = minimum
            .zip(maximum)
            .is_some_and(|(minimum, maximum)| minimum > maximum)
            || minimum.is_some_and(|minimum| minimum < type_minimum || minimum > type_maximum)
            || maximum.is_some_and(|maximum| maximum < type_minimum || maximum > type_maximum);
        if invalid {
            return Err(SchemaError::InvalidNumericConstraints {
                owner: owner.to_owned(),
                field: field.name.clone(),
                range: NumericConstraintRange::new(minimum, maximum),
            });
        }
    }
    if let Some(length) = constraints.length() {
        if !matches!(field.field_type, FieldType::Bytes | FieldType::Utf8)
            || field.cardinality == Cardinality::Repeated
        {
            return Err(SchemaError::UnexpectedLengthConstraints {
                owner: owner.to_owned(),
                field: field.name.clone(),
            });
        }
        if invalid_size_constraints(length) {
            return Err(SchemaError::InvalidLengthConstraints {
                owner: owner.to_owned(),
                field: field.name.clone(),
                range: SizeConstraintRange::new(length.minimum, length.maximum),
            });
        }
    }
    if let Some(items) = constraints.items() {
        if field.cardinality != Cardinality::Repeated {
            return Err(SchemaError::UnexpectedItemConstraints {
                owner: owner.to_owned(),
                field: field.name.clone(),
            });
        }
        if invalid_size_constraints(items) {
            return Err(SchemaError::InvalidItemConstraints {
                owner: owner.to_owned(),
                field: field.name.clone(),
                range: SizeConstraintRange::new(items.minimum, items.maximum),
            });
        }
    }
    Ok(())
}

fn invalid_size_constraints(constraints: SizeConstraints) -> bool {
    constraints
        .minimum
        .zip(constraints.maximum)
        .is_some_and(|(minimum, maximum)| minimum > maximum)
}

fn validate_enum_variants(
    type_id: &str,
    variants: &[EnumVariant],
    default: Option<&str>,
) -> Result<(), SchemaError> {
    if variants.is_empty() {
        return Err(SchemaError::EmptyVariants {
            type_id: type_id.to_owned(),
        });
    }
    if variants.len() > MAX_VARIANTS_PER_TYPE {
        return Err(SchemaError::TooManyVariants {
            type_id: type_id.to_owned(),
            actual: variants.len(),
            limit: MAX_VARIANTS_PER_TYPE,
        });
    }
    let mut previous_tag = None;
    let mut names = BTreeSet::new();
    for variant in variants {
        validate_identifier("enum variant", &variant.name)?;
        if !names.insert(variant.name.as_str()) {
            return Err(SchemaError::InvalidIdentifier {
                kind: "duplicate enum variant",
                value: variant.name.clone(),
            });
        }
        if let Some(previous) = previous_tag
            && previous >= variant.tag
        {
            return Err(SchemaError::NonCanonicalVariantOrder {
                type_id: type_id.to_owned(),
                previous,
                current: variant.tag,
            });
        }
        previous_tag = Some(variant.tag);
    }
    if let Some(default) = default
        && !names.contains(default)
    {
        return Err(SchemaError::UnknownEnumDefault {
            type_id: type_id.to_owned(),
            variant: default.to_owned(),
        });
    }
    Ok(())
}

type RecursiveModelEdges = BTreeSet<(String, String)>;

fn recursive_model_edges(schema: &ProtocolSchema) -> RecursiveModelEdges {
    let graph = schema
        .types
        .iter()
        .map(|definition| {
            let targets = definition_fields(definition)
                .into_iter()
                .filter(|field| {
                    field.field_type == FieldType::Message
                        && field.cardinality != Cardinality::Repeated
                })
                .map(|field| field.target.as_deref().expect("validated message target"))
                .collect::<Vec<_>>();
            (definition.id(), targets)
        })
        .collect::<BTreeMap<_, _>>();
    let mut recursive = BTreeSet::new();
    for (owner, targets) in &graph {
        for target in targets {
            let mut visited = BTreeSet::new();
            if model_reaches(target, owner, &graph, &mut visited) {
                recursive.insert(((*owner).to_owned(), (*target).to_owned()));
            }
        }
    }
    recursive
}

fn model_reaches<'a>(
    current: &'a str,
    goal: &str,
    graph: &BTreeMap<&'a str, Vec<&'a str>>,
    visited: &mut BTreeSet<&'a str>,
) -> bool {
    if current == goal {
        return true;
    }
    if !visited.insert(current) {
        return false;
    }
    graph.get(current).is_some_and(|targets| {
        targets
            .iter()
            .any(|target| model_reaches(target, goal, graph, visited))
    })
}

fn field_requires_box(owner: &str, field: &Field, recursive: &RecursiveModelEdges) -> bool {
    field.field_type == FieldType::Message
        && field.cardinality != Cardinality::Repeated
        && recursive.contains(&(
            owner.to_owned(),
            field
                .target
                .as_deref()
                .expect("validated message target")
                .to_owned(),
        ))
}

fn validate_model_inhabitance(schema: &ProtocolSchema) -> Result<(), SchemaError> {
    let sizes = minimum_wire_sizes(schema);
    if let Some(definition) = schema
        .types
        .iter()
        .find(|definition| !sizes.contains_key(definition.id()))
    {
        return Err(SchemaError::UninhabitedRecursiveModel {
            type_id: definition.id().to_owned(),
        });
    }
    Ok(())
}

fn definition_fields(definition: &TypeDefinition) -> Vec<&Field> {
    match definition {
        TypeDefinition::Enumeration { .. } => Vec::new(),
        TypeDefinition::Record { fields, .. } => fields.iter().collect(),
        TypeDefinition::TaggedUnion { variants, .. } => variants
            .iter()
            .flat_map(|variant| &variant.fields)
            .collect(),
    }
}

fn non_eq_type_ids(schema: &ProtocolSchema) -> BTreeSet<&str> {
    let mut result = schema
        .types
        .iter()
        .filter(|definition| {
            definition_fields(definition)
                .iter()
                .any(|field| field.field_type == FieldType::F64)
        })
        .map(TypeDefinition::id)
        .collect::<BTreeSet<_>>();
    loop {
        let before = result.len();
        for definition in &schema.types {
            if definition_fields(definition).iter().any(|field| {
                field.field_type == FieldType::Message
                    && field
                        .target
                        .as_deref()
                        .is_some_and(|target| result.contains(target))
            }) {
                result.insert(definition.id());
            }
        }
        if result.len() == before {
            return result;
        }
    }
}

fn emit_model(
    output: &mut String,
    definition: &TypeDefinition,
    rust_names: &BTreeMap<&str, String>,
    recursive: &RecursiveModelEdges,
    non_eq_types: &BTreeSet<&str>,
) -> Result<(), SchemaError> {
    let name = &rust_names[definition.id()];
    let default_derive = if definition.enum_default().is_some() {
        ", Default"
    } else {
        ""
    };
    let eq_derive = if non_eq_types.contains(definition.id()) {
        ""
    } else {
        ", Eq"
    };
    writeln!(
        output,
        "#[derive(Clone, Debug{eq_derive}, PartialEq{default_derive})]"
    )
    .map_err(|_| SchemaError::Formatting)?;
    match definition {
        TypeDefinition::Enumeration {
            default, variants, ..
        } => {
            writeln!(output, "pub enum {name} {{").map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                if default.as_deref() == Some(&variant.name) {
                    writeln!(output, "    #[default]").map_err(|_| SchemaError::Formatting)?;
                }
                writeln!(output, "    {},", pascal_case(&variant.name))
                    .map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::Record { fields, .. } => {
            writeln!(output, "pub struct {name} {{").map_err(|_| SchemaError::Formatting)?;
            emit_model_fields(
                output,
                definition.id(),
                fields,
                rust_names,
                recursive,
                "    pub ",
            )?;
            writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::TaggedUnion { variants, .. } => {
            writeln!(output, "pub enum {name} {{").map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                let variant_name = pascal_case(&variant.name);
                if variant.fields.is_empty() {
                    writeln!(output, "    {variant_name},").map_err(|_| SchemaError::Formatting)?;
                } else {
                    writeln!(output, "    {variant_name} {{")
                        .map_err(|_| SchemaError::Formatting)?;
                    emit_model_fields(
                        output,
                        definition.id(),
                        &variant.fields,
                        rust_names,
                        recursive,
                        "        ",
                    )?;
                    writeln!(output, "    }},").map_err(|_| SchemaError::Formatting)?;
                }
            }
            writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        }
    }
    Ok(())
}

fn emit_model_fields(
    output: &mut String,
    owner: &str,
    fields: &[Field],
    rust_names: &BTreeMap<&str, String>,
    recursive: &RecursiveModelEdges,
    prefix: &str,
) -> Result<(), SchemaError> {
    for field in fields {
        let base = match field.field_type.rust_type() {
            Some(scalar) => scalar.to_owned(),
            None => rust_names[field.target.as_deref().expect("validated message target")].clone(),
        };
        let base = if field_requires_box(owner, field, recursive) {
            format!("Box<{base}>")
        } else {
            base
        };
        let rust_type = match field.cardinality {
            Cardinality::Required => base,
            Cardinality::Optional => format!("Option<{base}>"),
            Cardinality::Repeated => format!("Vec<{base}>"),
        };
        writeln!(output, "{prefix}{}: {rust_type},", field.name)
            .map_err(|_| SchemaError::Formatting)?;
    }
    Ok(())
}

fn emit_borrowed_model(
    output: &mut String,
    definition: &TypeDefinition,
    rust_names: &BTreeMap<&str, String>,
    borrowed_rust_names: &BTreeMap<&str, String>,
    recursive: &RecursiveModelEdges,
) -> Result<(), SchemaError> {
    let name = &borrowed_rust_names[definition.id()];
    match definition {
        TypeDefinition::Enumeration { .. } => {
            unreachable!("enumerations never need borrowed models")
        }
        TypeDefinition::Record { fields, .. } => {
            writeln!(output, "pub struct {name}<'a> {{").map_err(|_| SchemaError::Formatting)?;
            emit_borrowed_model_fields(
                output,
                definition.id(),
                fields,
                rust_names,
                borrowed_rust_names,
                recursive,
                "    pub ",
            )?;
            writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::TaggedUnion { variants, .. } => {
            writeln!(output, "pub enum {name}<'a> {{").map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                let variant_name = pascal_case(&variant.name);
                if variant.fields.is_empty() {
                    writeln!(output, "    {variant_name},").map_err(|_| SchemaError::Formatting)?;
                } else {
                    writeln!(output, "    {variant_name} {{")
                        .map_err(|_| SchemaError::Formatting)?;
                    emit_borrowed_model_fields(
                        output,
                        definition.id(),
                        &variant.fields,
                        rust_names,
                        borrowed_rust_names,
                        recursive,
                        "        ",
                    )?;
                    writeln!(output, "    }},").map_err(|_| SchemaError::Formatting)?;
                }
            }
            writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;
        }
    }
    Ok(())
}

fn emit_borrowed_model_fields(
    output: &mut String,
    owner: &str,
    fields: &[Field],
    rust_names: &BTreeMap<&str, String>,
    borrowed_rust_names: &BTreeMap<&str, String>,
    recursive: &RecursiveModelEdges,
    prefix: &str,
) -> Result<(), SchemaError> {
    for field in fields {
        let rust_type = if field.cardinality == Cardinality::Repeated {
            "BinaryEncodeSequenceRef<'a>".to_owned()
        } else {
            let base = match field.field_type {
                FieldType::Utf8 => "&'a str".to_owned(),
                FieldType::Bytes => "&'a [u8]".to_owned(),
                FieldType::Enumeration | FieldType::Message => {
                    let target = field.target.as_deref().expect("validated type target");
                    let borrowed_name = &borrowed_rust_names[target];
                    if borrowed_name == &rust_names[target] {
                        rust_names[target].clone()
                    } else {
                        format!("{borrowed_name}<'a>")
                    }
                }
                _ => field
                    .field_type
                    .rust_type()
                    .expect("scalar Rust type")
                    .to_owned(),
            };
            let base = if field_requires_box(owner, field, recursive) {
                format!("Box<{base}>")
            } else {
                base
            };
            match field.cardinality {
                Cardinality::Required => base,
                Cardinality::Optional => format!("Option<{base}>"),
                Cardinality::Repeated => unreachable!("handled above"),
            }
        };
        writeln!(output, "{prefix}{}: {rust_type},", field.name)
            .map_err(|_| SchemaError::Formatting)?;
    }
    Ok(())
}

fn emit_codec_impl(
    output: &mut String,
    definition: &TypeDefinition,
    rust_names: &BTreeMap<&str, String>,
    minimum_sizes: &BTreeMap<&str, usize>,
    recursive: &RecursiveModelEdges,
) -> Result<(), SchemaError> {
    let type_id = definition.id();
    let rust_name = &rust_names[type_id];
    if let TypeDefinition::Enumeration { variants, .. } = definition {
        return emit_enum_codec_impl(output, type_id, rust_name, variants);
    }
    let context_name = if definition_fields(definition)
        .iter()
        .any(|field| field_uses_context(field))
    {
        "context"
    } else {
        "_context"
    };
    let has_fields = !definition_fields(definition).is_empty();
    let has_envelope = matches!(
        definition,
        TypeDefinition::Record {
            encoding: RecordEncoding::Extensible,
            ..
        }
    );
    let writer_name = if has_fields || has_envelope {
        "writer"
    } else {
        "_writer"
    };
    let reader_name = if has_fields || has_envelope {
        "reader"
    } else {
        "_reader"
    };
    writeln!(output, "impl BinaryEncode for {rust_name} {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    fn encode_binary(&self, {writer_name}: &mut BinaryWriter, context: &mut BinaryValueContext) -> Result<(), BinaryValueEncodeError> {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "        context.within_message({type_id:?}, |{context_name}| {{"
    )
    .map_err(|_| SchemaError::Formatting)?;
    match definition {
        TypeDefinition::Enumeration { .. } => unreachable!("enumerations return above"),
        TypeDefinition::Record {
            encoding: RecordEncoding::Positional,
            fields,
            ..
        } => {
            for field in fields {
                emit_encode_field(
                    output,
                    field,
                    &format!("self.{}", field.name),
                    &format!("{type_id}.{}", field.name),
                    rust_names,
                    12,
                    false,
                )?;
            }
            writeln!(output, "            Ok(())").map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::Record {
            encoding: RecordEncoding::Extensible,
            encoded_size,
            fields,
            ..
        } => emit_extensible_encode_fields(
            output,
            type_id,
            encoded_size.expect("validated extensible encoded size"),
            fields,
            rust_names,
            false,
        )?,
        TypeDefinition::TaggedUnion { variants, .. } => {
            writeln!(output, "            match self {{").map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                let variant_name = pascal_case(&variant.name);
                if variant.fields.is_empty() {
                    writeln!(output, "                Self::{variant_name} => {{")
                        .map_err(|_| SchemaError::Formatting)?;
                } else {
                    let bindings = variant
                        .fields
                        .iter()
                        .map(|field| field.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    writeln!(
                        output,
                        "                Self::{variant_name} {{ {bindings} }} => {{"
                    )
                    .map_err(|_| SchemaError::Formatting)?;
                }
                writeln!(output, "                    writer.u8({})?;", variant.tag)
                    .map_err(|_| SchemaError::Formatting)?;
                for field in &variant.fields {
                    emit_encode_field(
                        output,
                        field,
                        &field.name,
                        &format!("{type_id}.{}.{}", variant.name, field.name),
                        rust_names,
                        20,
                        true,
                    )?;
                }
                writeln!(output, "                    Ok(())")
                    .map_err(|_| SchemaError::Formatting)?;
                writeln!(output, "                }}").map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "            }}").map_err(|_| SchemaError::Formatting)?;
        }
    }
    writeln!(output, "        }})").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    }}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;

    writeln!(output, "impl BinaryDecode for {rust_name} {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    fn decode_binary({reader_name}: &mut BinaryReader<'_>, context: &mut BinaryValueContext) -> Result<Self, BinaryValueDecodeError> {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "        context.within_message({type_id:?}, |{context_name}| {{"
    )
    .map_err(|_| SchemaError::Formatting)?;
    match definition {
        TypeDefinition::Enumeration { .. } => unreachable!("enumerations return above"),
        TypeDefinition::Record {
            encoding: RecordEncoding::Positional,
            fields,
            ..
        } => {
            for field in fields {
                let expression = decode_field_expression(
                    type_id,
                    field,
                    &format!("{type_id}.{}", field.name),
                    rust_names,
                    minimum_sizes,
                    recursive,
                    12,
                );
                writeln!(output, "            let {} = {expression};", field.name)
                    .map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "            Ok(Self {{").map_err(|_| SchemaError::Formatting)?;
            for field in fields {
                writeln!(output, "                {},", field.name)
                    .map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "            }})").map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::Record {
            encoding: RecordEncoding::Extensible,
            encoded_size,
            fields,
            ..
        } => emit_extensible_decode_fields(
            output,
            type_id,
            encoded_size.expect("validated extensible encoded size"),
            fields,
            rust_names,
            minimum_sizes,
            recursive,
        )?,
        TypeDefinition::TaggedUnion { variants, .. } => {
            writeln!(output, "            match reader.u8()? {{")
                .map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                let variant_name = pascal_case(&variant.name);
                if variant.fields.is_empty() {
                    writeln!(
                        output,
                        "                {} => Ok(Self::{variant_name}),",
                        variant.tag
                    )
                    .map_err(|_| SchemaError::Formatting)?;
                } else {
                    writeln!(output, "                {} => {{", variant.tag)
                        .map_err(|_| SchemaError::Formatting)?;
                    for field in &variant.fields {
                        let expression = decode_field_expression(
                            type_id,
                            field,
                            &format!("{type_id}.{}.{}", variant.name, field.name),
                            rust_names,
                            minimum_sizes,
                            recursive,
                            20,
                        );
                        writeln!(
                            output,
                            "                    let {} = {expression};",
                            field.name
                        )
                        .map_err(|_| SchemaError::Formatting)?;
                    }
                    writeln!(output, "                    Ok(Self::{variant_name} {{")
                        .map_err(|_| SchemaError::Formatting)?;
                    for field in &variant.fields {
                        writeln!(output, "                        {},", field.name)
                            .map_err(|_| SchemaError::Formatting)?;
                    }
                    writeln!(output, "                    }})")
                        .map_err(|_| SchemaError::Formatting)?;
                    writeln!(output, "                }},").map_err(|_| SchemaError::Formatting)?;
                }
            }
            writeln!(output, "                value => Err(BinaryValueDecodeError::InvalidVariantTag {{ type_id: {type_id:?}, value }}),")
                .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "            }}").map_err(|_| SchemaError::Formatting)?;
        }
    }
    writeln!(output, "        }})").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    }}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)
}

fn emit_extensible_encode_fields(
    output: &mut String,
    type_id: &str,
    encoded_size: SizeConstraints,
    fields: &[Field],
    rust_names: &BTreeMap<&str, String>,
    borrowed_model: bool,
) -> Result<(), SchemaError> {
    writeln!(
        output,
        "            pwright_codec_binary::encode_extensible_record(writer, {type_id:?}, {}, |record| {{",
        size_range_expression(encoded_size)
    )
    .map_err(|_| SchemaError::Formatting)?;
    for field in fields {
        let tag = field.tag.expect("validated extensible field tag");
        let path = format!("{type_id}.{}", field.name);
        if field.cardinality == Cardinality::Optional {
            writeln!(
                output,
                "                if let Some(value) = &self.{} {{",
                field.name
            )
            .map_err(|_| SchemaError::Formatting)?;
            writeln!(
                output,
                "                    record.field({tag}, {path:?}, |writer| {{"
            )
            .map_err(|_| SchemaError::Formatting)?;
            let mut required = field.clone();
            required.cardinality = Cardinality::Required;
            emit_extensible_encode_field_body(
                output,
                &required,
                "value",
                &path,
                rust_names,
                24,
                borrowed_model,
                true,
            )?;
            writeln!(output, "                        Ok(())")
                .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "                    }})?;").map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "                }}").map_err(|_| SchemaError::Formatting)?;
        } else {
            writeln!(
                output,
                "                record.field({tag}, {path:?}, |writer| {{"
            )
            .map_err(|_| SchemaError::Formatting)?;
            emit_extensible_encode_field_body(
                output,
                field,
                &format!("self.{}", field.name),
                &path,
                rust_names,
                20,
                borrowed_model,
                false,
            )?;
            writeln!(output, "                    Ok(())").map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "                }})?;").map_err(|_| SchemaError::Formatting)?;
        }
    }
    writeln!(output, "                Ok(())").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "            }})?;").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "            Ok(())").map_err(|_| SchemaError::Formatting)
}

#[allow(clippy::too_many_arguments)]
fn emit_extensible_encode_field_body(
    output: &mut String,
    field: &Field,
    value: &str,
    path: &str,
    rust_names: &BTreeMap<&str, String>,
    indent: usize,
    borrowed_model: bool,
    value_is_borrowed: bool,
) -> Result<(), SchemaError> {
    if borrowed_model {
        emit_borrowed_encode_field(
            output,
            field,
            value,
            path,
            rust_names,
            indent,
            value_is_borrowed,
        )
    } else {
        emit_encode_field(
            output,
            field,
            value,
            path,
            rust_names,
            indent,
            value_is_borrowed,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_extensible_decode_fields(
    output: &mut String,
    type_id: &str,
    encoded_size: SizeConstraints,
    fields: &[Field],
    rust_names: &BTreeMap<&str, String>,
    minimum_sizes: &BTreeMap<&str, usize>,
    recursive: &RecursiveModelEdges,
) -> Result<(), SchemaError> {
    for field in fields {
        writeln!(output, "            let mut {} = None;", field.name)
            .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(
        output,
        "            let mut record = pwright_codec_binary::decode_extensible_record(reader, {type_id:?}, {})?;",
        size_range_expression(encoded_size)
    )
    .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "            while let Some(mut field) = record.next_field()? {{"
    )
    .map_err(|_| SchemaError::Formatting)?;
    if fields.len() == 1 {
        writeln!(output, "                #[allow(clippy::single_match)]")
            .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "                match field.tag() {{")
        .map_err(|_| SchemaError::Formatting)?;
    for field in fields {
        let tag = field.tag.expect("validated extensible field tag");
        let path = format!("{type_id}.{}", field.name);
        let expression = decode_extensible_field_expression(
            type_id,
            field,
            &path,
            rust_names,
            minimum_sizes,
            recursive,
            24,
        );
        writeln!(output, "                    {tag} => {{").map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "                        let reader = field.reader();"
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "                        let decoded_value = {expression};"
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "                        field.finish()?;")
            .map_err(|_| SchemaError::Formatting)?;
        writeln!(
            output,
            "                        {} = Some(decoded_value);",
            field.name
        )
        .map_err(|_| SchemaError::Formatting)?;
        writeln!(output, "                    }}").map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "                    _ => {{}}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "                }}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "            }}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "            record.finish()?;").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "            Ok(Self {{").map_err(|_| SchemaError::Formatting)?;
    for field in fields {
        if field.cardinality == Cardinality::Optional {
            writeln!(output, "                {},", field.name)
                .map_err(|_| SchemaError::Formatting)?;
        } else {
            writeln!(
                output,
                "                {}: {}.ok_or(BinaryValueDecodeError::MissingExtensibleField {{ record: {type_id:?}, field: {:?}, tag: {} }})?,",
                field.name,
                field.name,
                field.name,
                field.tag.expect("validated extensible field tag")
            )
            .map_err(|_| SchemaError::Formatting)?;
        }
    }
    writeln!(output, "            }})").map_err(|_| SchemaError::Formatting)
}

#[allow(clippy::too_many_arguments)]
fn decode_extensible_field_expression(
    owner: &str,
    field: &Field,
    path: &str,
    rust_names: &BTreeMap<&str, String>,
    minimum_sizes: &BTreeMap<&str, usize>,
    recursive: &RecursiveModelEdges,
    indent: usize,
) -> String {
    if field.cardinality == Cardinality::Repeated {
        return decode_field_expression(
            owner,
            field,
            path,
            rust_names,
            minimum_sizes,
            recursive,
            indent,
        );
    }
    let decoded = decode_required_expression(field, path, rust_names);
    if field_requires_box(owner, field, recursive) {
        format!("Box::new({decoded})")
    } else {
        decoded
    }
}

fn emit_enum_codec_impl(
    output: &mut String,
    type_id: &str,
    rust_name: &str,
    variants: &[EnumVariant],
) -> Result<(), SchemaError> {
    writeln!(output, "impl BinaryEncode for {rust_name} {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    fn encode_binary(&self, writer: &mut BinaryWriter, _context: &mut BinaryValueContext) -> Result<(), BinaryValueEncodeError> {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "        writer.u8(match self {{").map_err(|_| SchemaError::Formatting)?;
    for variant in variants {
        writeln!(
            output,
            "            Self::{} => {},",
            pascal_case(&variant.name),
            variant.tag
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "        }})?;").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "        Ok(())").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    }}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)?;

    writeln!(output, "impl BinaryDecode for {rust_name} {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    fn decode_binary(reader: &mut BinaryReader<'_>, _context: &mut BinaryValueContext) -> Result<Self, BinaryValueDecodeError> {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "        match reader.u8()? {{").map_err(|_| SchemaError::Formatting)?;
    for variant in variants {
        writeln!(
            output,
            "            {} => Ok(Self::{}),",
            variant.tag,
            pascal_case(&variant.name)
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "            value => Err(BinaryValueDecodeError::InvalidVariantTag {{ type_id: {type_id:?}, value }}),")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "        }}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    }}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)
}

fn emit_borrowed_codec_impl(
    output: &mut String,
    definition: &TypeDefinition,
    borrowed_rust_names: &BTreeMap<&str, String>,
) -> Result<(), SchemaError> {
    let type_id = definition.id();
    let rust_name = &borrowed_rust_names[type_id];
    let context_name = if definition_fields(definition)
        .iter()
        .any(|field| field_uses_context(field))
    {
        "context"
    } else {
        "_context"
    };
    writeln!(output, "impl BinaryEncode for {rust_name}<'_> {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    fn encode_binary(&self, writer: &mut BinaryWriter, context: &mut BinaryValueContext) -> Result<(), BinaryValueEncodeError> {{")
        .map_err(|_| SchemaError::Formatting)?;
    writeln!(
        output,
        "        context.within_message({type_id:?}, |{context_name}| {{"
    )
    .map_err(|_| SchemaError::Formatting)?;
    match definition {
        TypeDefinition::Enumeration { .. } => {
            unreachable!("enumerations never need borrowed models")
        }
        TypeDefinition::Record {
            encoding: RecordEncoding::Positional,
            fields,
            ..
        } => {
            for field in fields {
                emit_borrowed_encode_field(
                    output,
                    field,
                    &format!("self.{}", field.name),
                    &format!("{type_id}.{}", field.name),
                    borrowed_rust_names,
                    12,
                    false,
                )?;
            }
            writeln!(output, "            Ok(())").map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::Record {
            encoding: RecordEncoding::Extensible,
            encoded_size,
            fields,
            ..
        } => emit_extensible_encode_fields(
            output,
            type_id,
            encoded_size.expect("validated extensible encoded size"),
            fields,
            borrowed_rust_names,
            true,
        )?,
        TypeDefinition::TaggedUnion { variants, .. } => {
            writeln!(output, "            match self {{").map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                let variant_name = pascal_case(&variant.name);
                if variant.fields.is_empty() {
                    writeln!(output, "                Self::{variant_name} => {{")
                        .map_err(|_| SchemaError::Formatting)?;
                } else {
                    let bindings = variant
                        .fields
                        .iter()
                        .map(|field| field.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ");
                    writeln!(
                        output,
                        "                Self::{variant_name} {{ {bindings} }} => {{"
                    )
                    .map_err(|_| SchemaError::Formatting)?;
                }
                writeln!(output, "                    writer.u8({})?;", variant.tag)
                    .map_err(|_| SchemaError::Formatting)?;
                for field in &variant.fields {
                    emit_borrowed_encode_field(
                        output,
                        field,
                        &field.name,
                        &format!("{type_id}.{}.{}", variant.name, field.name),
                        borrowed_rust_names,
                        20,
                        true,
                    )?;
                }
                writeln!(output, "                    Ok(())")
                    .map_err(|_| SchemaError::Formatting)?;
                writeln!(output, "                }}").map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "            }}").map_err(|_| SchemaError::Formatting)?;
        }
    }
    writeln!(output, "        }})").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "    }}").map_err(|_| SchemaError::Formatting)?;
    writeln!(output, "}}\n").map_err(|_| SchemaError::Formatting)
}

fn emit_borrowed_encode_field(
    output: &mut String,
    field: &Field,
    value: &str,
    path: &str,
    borrowed_rust_names: &BTreeMap<&str, String>,
    indent: usize,
    value_is_borrowed: bool,
) -> Result<(), SchemaError> {
    let spaces = " ".repeat(indent);
    if field.cardinality == Cardinality::Repeated {
        if let Some(items) = field.constraints.and_then(FieldConstraints::items) {
            writeln!(
                output,
                "{spaces}pwright_codec_binary::check_size_range({path:?}, {value}.len(), {})?;",
                size_range_expression(items)
            )
            .map_err(|_| SchemaError::Formatting)?;
        }
        if let Some(count_field) = field.counted_by.as_deref() {
            let count = if value_is_borrowed {
                format!("*{count_field}")
            } else {
                format!("self.{count_field}")
            };
            writeln!(
                output,
                "{spaces}context.check_repeated({path:?}, {value}.len())?;"
            )
            .map_err(|_| SchemaError::Formatting)?;
            writeln!(
                output,
                "{spaces}pwright_codec_binary::check_size_range({path:?}, {value}.len(), BinarySizeRange::new(Some({count}), Some({count})))?;"
            )
            .map_err(|_| SchemaError::Formatting)?;
        } else {
            writeln!(
                output,
                "{spaces}pwright_codec_binary::encode_repeated_len(writer, context, {path:?}, {value}.len())?;"
            )
            .map_err(|_| SchemaError::Formatting)?;
        }
        writeln!(output, "{spaces}{value}.encode_items(writer, context)?;")
            .map_err(|_| SchemaError::Formatting)?;
        return Ok(());
    }
    match field.cardinality {
        Cardinality::Required => emit_encode_required(
            output,
            field,
            value,
            path,
            borrowed_rust_names,
            &spaces,
            value_is_borrowed || matches!(field.field_type, FieldType::Bytes | FieldType::Utf8),
        ),
        Cardinality::Optional => {
            let option = if value_is_borrowed {
                value.to_owned()
            } else {
                format!("&{value}")
            };
            writeln!(output, "{spaces}writer.u8(u8::from({value}.is_some()))?;")
                .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "{spaces}if let Some(value) = {option} {{")
                .map_err(|_| SchemaError::Formatting)?;
            emit_encode_required(
                output,
                field,
                "value",
                path,
                borrowed_rust_names,
                &format!("{spaces}    "),
                true,
            )?;
            writeln!(output, "{spaces}}}").map_err(|_| SchemaError::Formatting)
        }
        Cardinality::Repeated => unreachable!("handled above"),
    }
}

fn emit_encode_field(
    output: &mut String,
    field: &Field,
    value: &str,
    path: &str,
    rust_names: &BTreeMap<&str, String>,
    indent: usize,
    borrowed: bool,
) -> Result<(), SchemaError> {
    let spaces = " ".repeat(indent);
    match field.cardinality {
        Cardinality::Required => {
            emit_encode_required(output, field, value, path, rust_names, &spaces, borrowed)?;
        }
        Cardinality::Optional => {
            let option = if borrowed {
                value.to_owned()
            } else {
                format!("&{value}")
            };
            writeln!(output, "{spaces}writer.u8(u8::from({value}.is_some()))?;")
                .map_err(|_| SchemaError::Formatting)?;
            writeln!(output, "{spaces}if let Some(value) = {option} {{")
                .map_err(|_| SchemaError::Formatting)?;
            emit_encode_required(
                output,
                field,
                "value",
                path,
                rust_names,
                &format!("{spaces}    "),
                true,
            )?;
            writeln!(output, "{spaces}}}").map_err(|_| SchemaError::Formatting)?;
        }
        Cardinality::Repeated => {
            let values = if borrowed {
                value.to_owned()
            } else {
                format!("&{value}")
            };
            if let Some(items) = field.constraints.and_then(FieldConstraints::items) {
                writeln!(
                    output,
                    "{spaces}pwright_codec_binary::check_size_range({path:?}, {value}.len(), {})?;",
                    size_range_expression(items)
                )
                .map_err(|_| SchemaError::Formatting)?;
            }
            if let Some(count_field) = field.counted_by.as_deref() {
                let count = if borrowed {
                    format!("*{count_field}")
                } else {
                    format!("self.{count_field}")
                };
                writeln!(
                    output,
                    "{spaces}context.check_repeated({path:?}, {value}.len())?;"
                )
                .map_err(|_| SchemaError::Formatting)?;
                writeln!(
                    output,
                    "{spaces}pwright_codec_binary::check_size_range({path:?}, {value}.len(), BinarySizeRange::new(Some({count}), Some({count})))?;"
                )
                .map_err(|_| SchemaError::Formatting)?;
            } else {
                writeln!(
                    output,
                    "{spaces}pwright_codec_binary::encode_repeated_len(writer, context, {path:?}, {value}.len())?;"
                )
                .map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "{spaces}for value in {values} {{")
                .map_err(|_| SchemaError::Formatting)?;
            emit_encode_required(
                output,
                field,
                "value",
                path,
                rust_names,
                &format!("{spaces}    "),
                true,
            )?;
            writeln!(output, "{spaces}}}").map_err(|_| SchemaError::Formatting)?;
        }
    }
    Ok(())
}

fn emit_encode_required(
    output: &mut String,
    field: &Field,
    value: &str,
    path: &str,
    rust_names: &BTreeMap<&str, String>,
    spaces: &str,
    borrowed: bool,
) -> Result<(), SchemaError> {
    let copy_value = if borrowed {
        format!("*{value}")
    } else {
        value.to_owned()
    };
    let reference_value = if borrowed {
        value.to_owned()
    } else {
        format!("&{value}")
    };
    if let Some(constraints) = field.constraints.and_then(FieldConstraints::numeric) {
        writeln!(
            output,
            "{spaces}pwright_codec_binary::check_numeric_range({path:?}, i128::from({copy_value}), {})?;",
            numeric_range_expression(constraints)
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    let statement = match field.field_type {
        FieldType::U8 => format!("writer.u8({copy_value})?;"),
        FieldType::U16 => format!("writer.u16({copy_value})?;"),
        FieldType::U32 => format!("writer.u32({copy_value})?;"),
        FieldType::U64 => format!("writer.u64({copy_value})?;"),
        FieldType::I32 => format!("writer.i32({copy_value})?;"),
        FieldType::I64 => format!("writer.i64({copy_value})?;"),
        FieldType::F64 => format!("writer.f64({copy_value})?;"),
        FieldType::Bytes => match field.constraints.and_then(FieldConstraints::length) {
            Some(constraints) => format!(
                "pwright_codec_binary::encode_bytes_constrained(writer, context, {path:?}, {reference_value}, {})?;",
                size_range_expression(constraints)
            ),
            None => format!(
                "pwright_codec_binary::encode_bytes(writer, context, {path:?}, {reference_value})?;"
            ),
        },
        FieldType::Bool => format!("writer.u8(u8::from({copy_value}))?;"),
        FieldType::Utf8 => match field.constraints.and_then(FieldConstraints::length) {
            Some(constraints) => format!(
                "pwright_codec_binary::encode_utf8_constrained(writer, context, {path:?}, {reference_value}, {})?;",
                size_range_expression(constraints)
            ),
            None => format!(
                "pwright_codec_binary::encode_utf8(writer, context, {path:?}, {reference_value})?;"
            ),
        },
        FieldType::Enumeration | FieldType::Message => {
            let target = field.target.as_deref().expect("validated message target");
            let rust_name = &rust_names[target];
            format!(
                "<{rust_name} as BinaryEncode>::encode_binary({reference_value}, writer, context)?;"
            )
        }
    };
    writeln!(output, "{spaces}{statement}").map_err(|_| SchemaError::Formatting)
}

fn decode_field_expression(
    owner: &str,
    field: &Field,
    path: &str,
    rust_names: &BTreeMap<&str, String>,
    minimum_sizes: &BTreeMap<&str, usize>,
    recursive: &RecursiveModelEdges,
    indent: usize,
) -> String {
    let required = decode_required_expression(field, path, rust_names);
    let required = if field_requires_box(owner, field, recursive) {
        format!("Box::new({required})")
    } else {
        required
    };
    match field.cardinality {
        Cardinality::Required => required,
        Cardinality::Optional => format!(
            "match reader.u8()? {{ 0 => None, 1 => Some({required}), value => return Err(BinaryValueDecodeError::InvalidOptionFlag {{ field: {path:?}, value }}) }}"
        ),
        Cardinality::Repeated => {
            let spaces = " ".repeat(indent);
            let minimum_item_bytes = minimum_field_value_size(field, minimum_sizes);
            let length = field.counted_by.as_deref().map_or_else(
                || {
                    format!(
                        "let length = pwright_codec_binary::decode_repeated_len(reader, context, {path:?})?;"
                    )
                },
                |count_field| {
                    let count_path = format!("{owner}.{count_field}");
                    format!(
                        "let length = usize::try_from(u64::from({count_field})).map_err(|_| BinaryDecodeError::IntegerOutOfRange {{ field: {count_path:?}, value: u64::from({count_field}) }})?;\n{spaces}    context.check_repeated({path:?}, length)?;"
                    )
                },
            );
            let item_constraint = field
                .constraints
                .and_then(FieldConstraints::items)
                .map(|constraints| {
                    format!(
                        "\n{spaces}    pwright_codec_binary::check_size_range({path:?}, length, {})?;",
                        size_range_expression(constraints)
                    )
                })
                .unwrap_or_default();
            format!(
                "{{\n{spaces}    {length}{item_constraint}\n{spaces}    reader.ensure_count(length, {minimum_item_bytes})?;\n{spaces}    let mut values = pwright_codec_binary::try_vec_with_capacity({path:?}, length)?;\n{spaces}    for _ in 0..length {{\n{spaces}        values.push({required});\n{spaces}    }}\n{spaces}    values\n{spaces}}}"
            )
        }
    }
}

fn decode_required_expression(
    field: &Field,
    path: &str,
    rust_names: &BTreeMap<&str, String>,
) -> String {
    let decoded = match field.field_type {
        FieldType::U8 => "reader.u8()?".to_owned(),
        FieldType::U16 => "reader.u16()?".to_owned(),
        FieldType::U32 => "reader.u32()?".to_owned(),
        FieldType::U64 => "reader.u64()?".to_owned(),
        FieldType::I32 => "reader.i32()?".to_owned(),
        FieldType::I64 => "reader.i64()?".to_owned(),
        FieldType::F64 => "reader.f64()?".to_owned(),
        FieldType::Bytes => match field.constraints.and_then(FieldConstraints::length) {
            Some(constraints) => format!(
                "pwright_codec_binary::decode_bytes_constrained(reader, context, {path:?}, {})?",
                size_range_expression(constraints)
            ),
            None => format!("pwright_codec_binary::decode_bytes(reader, context, {path:?})?"),
        },
        FieldType::Bool => format!(
            "match reader.u8()? {{ 0 => false, 1 => true, value => return Err(BinaryValueDecodeError::InvalidBoolean {{ field: {path:?}, value }}) }}"
        ),
        FieldType::Utf8 => match field.constraints.and_then(FieldConstraints::length) {
            Some(constraints) => format!(
                "pwright_codec_binary::decode_utf8_constrained(reader, context, {path:?}, {})?",
                size_range_expression(constraints)
            ),
            None => format!("pwright_codec_binary::decode_utf8(reader, context, {path:?})?"),
        },
        FieldType::Enumeration | FieldType::Message => {
            let target = field.target.as_deref().expect("validated message target");
            let rust_name = &rust_names[target];
            format!("<{rust_name} as BinaryDecode>::decode_binary(reader, context)?")
        }
    };
    match field.constraints.and_then(FieldConstraints::numeric) {
        Some(constraints) => format!(
            "{{ let value = {decoded}; pwright_codec_binary::check_numeric_range({path:?}, i128::from(value), {})?; value }}",
            numeric_range_expression(constraints)
        ),
        None => decoded,
    }
}

fn numeric_range_expression(constraints: NumericConstraints) -> String {
    format!(
        "BinaryNumericRange::new({}, {})",
        i128_option_expression(constraints.minimum.map(NumericBound::as_i128)),
        i128_option_expression(constraints.maximum.map(NumericBound::as_i128))
    )
}

fn size_range_expression(constraints: SizeConstraints) -> String {
    format!(
        "BinarySizeRange::new({}, {})",
        u32_option_expression(constraints.minimum),
        u32_option_expression(constraints.maximum)
    )
}

fn u32_option_expression(value: Option<u32>) -> String {
    value.map_or_else(|| "None".to_owned(), |value| format!("Some({value}_u32)"))
}

fn i128_option_expression(value: Option<i128>) -> String {
    value.map_or_else(|| "None".to_owned(), |value| format!("Some({value}_i128)"))
}

fn field_uses_context(field: &Field) -> bool {
    field.cardinality == Cardinality::Repeated
        || matches!(
            field.field_type,
            FieldType::Bytes | FieldType::Enumeration | FieldType::Utf8 | FieldType::Message
        )
}

fn minimum_wire_sizes(schema: &ProtocolSchema) -> BTreeMap<&str, usize> {
    let mut sizes = BTreeMap::new();
    loop {
        let mut changed = false;
        for definition in &schema.types {
            let Some(size) = minimum_type_size(definition, &sizes) else {
                continue;
            };
            match sizes.get(definition.id()) {
                Some(previous) if *previous <= size => {}
                _ => {
                    sizes.insert(definition.id(), size);
                    changed = true;
                }
            }
        }
        if !changed {
            return sizes;
        }
    }
}

fn minimum_type_size(definition: &TypeDefinition, sizes: &BTreeMap<&str, usize>) -> Option<usize> {
    match definition {
        TypeDefinition::Enumeration { .. } => Some(1),
        TypeDefinition::Record {
            encoding: RecordEncoding::Positional,
            fields,
            ..
        } => minimum_fields_size(fields, sizes),
        TypeDefinition::Record {
            encoding: RecordEncoding::Extensible,
            fields,
            ..
        } => minimum_extensible_record_size(fields, sizes),
        TypeDefinition::TaggedUnion { variants, .. } => variants
            .iter()
            .filter_map(|variant| minimum_fields_size(&variant.fields, sizes))
            .min()
            .and_then(|fields| 1_usize.checked_add(fields)),
    }
}

fn minimum_extensible_record_size(
    fields: &[Field],
    sizes: &BTreeMap<&str, usize>,
) -> Option<usize> {
    fields
        .iter()
        .filter(|field| field.cardinality != Cardinality::Optional)
        .try_fold(4_usize, |total, field| {
            total
                .checked_add(6)?
                .checked_add(minimum_field_size(field, sizes)?)
        })
}

fn minimum_fields_size(fields: &[Field], sizes: &BTreeMap<&str, usize>) -> Option<usize> {
    fields.iter().try_fold(0_usize, |total, field| {
        total.checked_add(minimum_field_size(field, sizes)?)
    })
}

fn minimum_field_size(field: &Field, sizes: &BTreeMap<&str, usize>) -> Option<usize> {
    match field.cardinality {
        Cardinality::Required => match field.field_type {
            FieldType::Enumeration | FieldType::Message => sizes
                .get(field.target.as_deref().expect("validated message target"))
                .copied(),
            _ => Some(minimum_scalar_size(field.field_type)),
        },
        Cardinality::Optional => Some(1),
        Cardinality::Repeated => Some(if field.counted_by.is_some() { 0 } else { 4 }),
    }
}

fn minimum_field_value_size(field: &Field, minimum_sizes: &BTreeMap<&str, usize>) -> usize {
    match field.field_type {
        FieldType::Enumeration | FieldType::Message => {
            minimum_sizes[field.target.as_deref().expect("validated message target")]
        }
        _ => minimum_scalar_size(field.field_type),
    }
}

const fn minimum_scalar_size(field_type: FieldType) -> usize {
    match field_type {
        FieldType::U8 => 1,
        FieldType::U16 => 2,
        FieldType::U32 | FieldType::I32 => 4,
        FieldType::U64 | FieldType::I64 | FieldType::F64 => 8,
        FieldType::Bytes | FieldType::Utf8 => 4,
        FieldType::Bool => 1,
        FieldType::Enumeration | FieldType::Message => unreachable!(),
    }
}

fn emit_schema_constants(
    output: &mut String,
    version: Version,
    type_index: usize,
    definition: &TypeDefinition,
) -> Result<(), SchemaError> {
    let schema_name = schema_constant_name(definition.id());
    match definition {
        TypeDefinition::Enumeration { id, variants, .. } => {
            writeln!(
                output,
                "const TYPE_{type_index}_VARIANTS: &[BinaryEnumVariantSchema] = &["
            )
            .map_err(|_| SchemaError::Formatting)?;
            for variant in variants {
                writeln!(
                    output,
                    "    BinaryEnumVariantSchema::new({}, {:?}),",
                    variant.tag, variant.name
                )
                .map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "];\n").map_err(|_| SchemaError::Formatting)?;
            writeln!(
                output,
                "pub const {schema_name}: BinaryEnumSchema = BinaryEnumSchema::new({id:?}, ProtocolVersion::new({}, {}), TYPE_{type_index}_VARIANTS);\n",
                version.major,
                version.minor
            )
            .map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::Record {
            id,
            encoding: RecordEncoding::Positional,
            fields,
            ..
        } => {
            emit_field_constant(output, &format!("TYPE_{type_index}_FIELDS"), fields)?;
            writeln!(
                output,
                "pub const {schema_name}: BinaryRecordSchema = BinaryRecordSchema::new({id:?}, ProtocolVersion::new({}, {}), TYPE_{type_index}_FIELDS);\n",
                version.major,
                version.minor
            )
            .map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::Record {
            id,
            encoding: RecordEncoding::Extensible,
            encoded_size,
            fields,
            ..
        } => {
            emit_extensible_field_constant(output, &format!("TYPE_{type_index}_FIELDS"), fields)?;
            writeln!(
                output,
                "pub const {schema_name}: BinaryExtensibleRecordSchema = BinaryExtensibleRecordSchema::new({id:?}, ProtocolVersion::new({}, {}), {}, TYPE_{type_index}_FIELDS);\n",
                version.major,
                version.minor,
                size_range_expression(encoded_size.expect("validated extensible encoded size"))
            )
            .map_err(|_| SchemaError::Formatting)?;
        }
        TypeDefinition::TaggedUnion { id, variants, .. } => {
            for (variant_index, variant) in variants.iter().enumerate() {
                emit_field_constant(
                    output,
                    &format!("TYPE_{type_index}_VARIANT_{variant_index}_FIELDS"),
                    &variant.fields,
                )?;
            }
            writeln!(
                output,
                "const TYPE_{type_index}_VARIANTS: &[BinaryVariantSchema] = &["
            )
            .map_err(|_| SchemaError::Formatting)?;
            for (variant_index, variant) in variants.iter().enumerate() {
                writeln!(
                    output,
                    "    BinaryVariantSchema::new({}, {:?}, TYPE_{type_index}_VARIANT_{variant_index}_FIELDS),",
                    variant.tag,
                    variant.name
                )
                .map_err(|_| SchemaError::Formatting)?;
            }
            writeln!(output, "];\n").map_err(|_| SchemaError::Formatting)?;
            writeln!(
                output,
                "pub const {schema_name}: BinarySchema = BinarySchema::new({id:?}, ProtocolVersion::new({}, {}), TYPE_{type_index}_VARIANTS);\n",
                version.major,
                version.minor
            )
            .map_err(|_| SchemaError::Formatting)?;
        }
    }
    Ok(())
}

fn schema_constant_name(id: &str) -> String {
    let mut name = id
        .bytes()
        .map(|byte| match byte {
            b'a'..=b'z' => char::from(byte.to_ascii_uppercase()),
            b'0'..=b'9' => char::from(byte),
            b'.' | b'_' => '_',
            _ => unreachable!("validated schema type ID"),
        })
        .collect::<String>();
    name.push_str("_SCHEMA");
    name
}

fn command_schema_constant_name(id: &str) -> String {
    let mut name = String::from("COMMAND_");
    name.extend(id.bytes().map(|byte| match byte {
        b'a'..=b'z' => char::from(byte.to_ascii_uppercase()),
        b'0'..=b'9' => char::from(byte),
        b'.' | b'_' | b'-' => '_',
        _ => unreachable!("validated command ID"),
    }));
    name.push_str("_SCHEMA");
    name
}

fn emit_command_constants(output: &mut String, schema: &ProtocolSchema) -> Result<(), SchemaError> {
    if schema.commands.is_empty() {
        return Ok(());
    }
    let root = schema.root().expect("validated capability command root");
    let root_definition = &schema.types[schema
        .types
        .binary_search_by(|definition| definition.id().cmp(root))
        .expect("validated root")];
    let TypeDefinition::TaggedUnion { variants, .. } = root_definition else {
        unreachable!("validated tagged-union root")
    };
    for command in &schema.commands {
        let command_tag = variants
            .iter()
            .find(|variant| variant.name == command.command)
            .expect("validated command variant")
            .tag;
        let result_tag = variants
            .iter()
            .find(|variant| variant.name == command.result)
            .expect("validated result variant")
            .tag;
        let name = command_schema_constant_name(&command.id);
        writeln!(
            output,
            "pub const {name}: BinaryCommandSchema = BinaryCommandSchema::new(CommandId::new({:?}), MethodTag::new({}), {command_tag}, {result_tag});",
            command.id, command.method
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "\npub const COMMANDS: &[BinaryCommandSchema] = &[")
        .map_err(|_| SchemaError::Formatting)?;
    for command in &schema.commands {
        writeln!(output, "    {},", command_schema_constant_name(&command.id))
            .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "];\n").map_err(|_| SchemaError::Formatting)
}

fn emit_field_constant(
    output: &mut String,
    name: &str,
    fields: &[Field],
) -> Result<(), SchemaError> {
    writeln!(output, "const {name}: &[BinaryFieldSchema] = &[")
        .map_err(|_| SchemaError::Formatting)?;
    for field in fields {
        writeln!(output, "    {},", field_schema_expression(field)?)
            .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "];\n").map_err(|_| SchemaError::Formatting)
}

fn field_schema_expression(field: &Field) -> Result<String, SchemaError> {
    let mut expression = if matches!(
        field.field_type,
        FieldType::Enumeration | FieldType::Message
    ) {
        let constructor = match field.field_type {
            FieldType::Enumeration => "enumeration",
            FieldType::Message => "message",
            _ => unreachable!(),
        };
        format!(
            "BinaryFieldSchema::{constructor}({:?}, {:?}, BinaryCardinality::{})",
            field.name,
            field.target.as_deref().expect("validated message target"),
            field.cardinality.schema_variant()
        )
    } else {
        format!(
            "BinaryFieldSchema::new({:?}, BinaryFieldKind::{}, BinaryCardinality::{})",
            field.name,
            field.field_type.schema_variant(),
            field.cardinality.schema_variant()
        )
    };
    if let Some(constraints) = field.constraints.and_then(FieldConstraints::numeric) {
        write!(
            expression,
            ".with_numeric_range({})",
            numeric_range_expression(constraints)
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    if let Some(constraints) = field.constraints.and_then(FieldConstraints::length) {
        write!(
            expression,
            ".with_length_range({})",
            size_range_expression(constraints)
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    if let Some(constraints) = field.constraints.and_then(FieldConstraints::items) {
        write!(
            expression,
            ".with_item_count_range({})",
            size_range_expression(constraints)
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    if let Some(count_field) = field.counted_by.as_deref() {
        write!(expression, ".with_external_count({count_field:?})")
            .map_err(|_| SchemaError::Formatting)?;
    }
    Ok(expression)
}

fn emit_extensible_field_constant(
    output: &mut String,
    name: &str,
    fields: &[Field],
) -> Result<(), SchemaError> {
    writeln!(output, "const {name}: &[BinaryExtensibleFieldSchema] = &[")
        .map_err(|_| SchemaError::Formatting)?;
    for field in fields {
        writeln!(
            output,
            "    BinaryExtensibleFieldSchema::new({}, {}),",
            field.tag.expect("validated extensible field tag"),
            field_schema_expression(field)?
        )
        .map_err(|_| SchemaError::Formatting)?;
    }
    writeln!(output, "];\n").map_err(|_| SchemaError::Formatting)
}

fn rust_type_names(schema: &ProtocolSchema) -> Result<BTreeMap<&str, String>, SchemaError> {
    schema
        .types
        .iter()
        .map(|definition| Ok((definition.id(), rust_type_name(definition.id())?)))
        .collect()
}

fn borrowed_type_ids(schema: &ProtocolSchema) -> BTreeSet<&str> {
    let mut borrowed = BTreeSet::new();
    loop {
        let mut changed = false;
        for definition in &schema.types {
            if matches!(definition, TypeDefinition::Enumeration { .. })
                || borrowed.contains(definition.id())
            {
                continue;
            }
            let needs_borrow = definition_fields(definition).into_iter().any(|field| {
                field.cardinality == Cardinality::Repeated
                    || matches!(field.field_type, FieldType::Bytes | FieldType::Utf8)
                    || (field.field_type == FieldType::Message
                        && borrowed
                            .contains(field.target.as_deref().expect("validated message target")))
            });
            if needs_borrow {
                borrowed.insert(definition.id());
                changed = true;
            }
        }
        if !changed {
            return borrowed;
        }
    }
}

fn borrowed_rust_type_names<'a>(
    rust_names: &BTreeMap<&'a str, String>,
    borrowed_types: &BTreeSet<&str>,
) -> BTreeMap<&'a str, String> {
    rust_names
        .iter()
        .map(|(id, name)| {
            let name = if borrowed_types.contains(id) {
                format!("{name}Ref")
            } else {
                name.clone()
            };
            (*id, name)
        })
        .collect()
}

fn rust_type_name(id: &str) -> Result<String, SchemaError> {
    if id.split('.').any(|segment| {
        segment.is_empty()
            || !segment
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    }) {
        return Err(SchemaError::InvalidIdentifier {
            kind: "type ID",
            value: id.to_owned(),
        });
    }
    Ok(id.split('.').map(pascal_case).collect())
}

fn validate_identifier(kind: &'static str, value: &str) -> Result<(), SchemaError> {
    let mut bytes = value.bytes();
    let valid_first = bytes.next().is_some_and(|byte| byte.is_ascii_lowercase());
    if !valid_first
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(SchemaError::InvalidIdentifier {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn pascal_case(value: &str) -> String {
    let mut result = String::new();
    let mut uppercase = true;
    for character in value.chars() {
        if character == '_' {
            uppercase = true;
        } else if uppercase {
            result.extend(character.to_uppercase());
            uppercase = false;
        } else {
            result.push(character);
        }
    }
    result
}

fn screaming_snake_case(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'a'..=b'z' => char::from(byte.to_ascii_uppercase()),
            b'0'..=b'9' => char::from(byte),
            b'.' | b'_' => '_',
            _ => unreachable!("validated schema identifier"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use pwright_codec_binary::{
        BinaryDecode, BinaryEncode, BinaryReader, BinaryValueContext, BinaryValueLimits,
        BinaryWriter,
    };

    use super::{
        CompatibilityError, NumericConstraintRange, SchemaError, SizeConstraintRange,
        balanced_server_handler_expression, balanced_server_handler_type,
        check_minor_compatibility, check_minor_compatibility_with_imports,
        generate_capability_rust, generate_markdown_reference, generate_rust,
        generate_rust_with_imports, generate_server_rust, parse_and_validate,
        parse_and_validate_with_imports,
    };

    #[allow(dead_code)]
    mod generated_extensible_record {
        include!("fixtures/generated_extensible_record.rs");
    }

    const REPRESENTATIVE: &str = r#"
format: 1
capability: pwright.test
version: { major: 1, minor: 2 }
root: pwright.test.message
types:
  - kind: record
    id: pwright.test.child
    fields:
      - { name: value, type: utf8 }
  - kind: tagged_union
    id: pwright.test.message
    variants:
      - tag: 1
        name: command
        fields:
          - { name: call_id, type: u64 }
          - { name: child, type: message, target: pwright.test.child, cardinality: optional }
"#;

    #[test]
    fn selector_library_types_exactly_match_both_published_locator_tracks() {
        // Arrange
        let library = parse_and_validate(include_str!(
            "../../pwright-codec-selector/schema/selector.yml"
        ))
        .expect("selector library should validate");
        let locator_v1 = parse_and_validate_with_imports(
            include_str!("../../pwright-cap-locator/schema/locator.yml"),
            &[
                parse_and_validate(include_str!("../../pwright-codec-common/schema/common.yml"))
                    .expect("common library should validate"),
            ],
        )
        .expect("locator 1.x should validate");
        let locator_v2 = parse_and_validate_with_imports(
            include_str!("../../pwright-cap-locator/schema/locator-v2.yml"),
            &[
                parse_and_validate(include_str!("../../pwright-codec-common/schema/common.yml"))
                    .expect("common library should validate"),
            ],
        )
        .expect("locator 2.x should validate");

        // Act
        let matching_types = |locator: &super::ProtocolSchema| {
            library.types.iter().all(|selector_type| {
                locator
                    .types
                    .iter()
                    .find(|candidate| candidate.id() == selector_type.id())
                    == Some(selector_type)
            })
        };

        // Assert
        assert_eq!(library.types.len(), 9);
        assert!(matching_types(&locator_v1));
        assert!(matching_types(&locator_v2));
    }

    #[test]
    fn selector_library_can_be_imported_without_importing_locator_commands() {
        // Arrange
        let library = parse_and_validate(include_str!(
            "../../pwright-codec-selector/schema/selector.yml"
        ))
        .expect("selector library should validate");
        let source = r#"
format: 1
protocol: pwright.test_selector_consumer
version: { major: 1, minor: 0 }
root: pwright.test_selector_consumer
imports:
  - pwright.selector
types:
  - kind: tagged_union
    id: pwright.test_selector_consumer
    variants:
      - tag: 0
        name: select
        fields:
          - { name: selector, type: message, target: pwright.locator.selector }
"#;

        // Act
        let schema = parse_and_validate_with_imports(source, std::slice::from_ref(&library))
            .expect("consumer should resolve the selector library");
        let generated = generate_rust_with_imports(&schema, &[library])
            .expect("consumer should generate from imported selector types");

        // Assert
        assert!(generated.contains("pub use pwright_codec_selector::{"));
        assert!(generated.contains("PwrightLocatorSelector"));
        assert!(!generated.contains("pub enum PwrightLocatorSelector"));
    }

    const REPRESENTATIVE_LIBRARY: &str = r#"
format: 1
library: pwright.common
version: { major: 1, minor: 0 }
types:
  - kind: record
    id: pwright.common.duration
    fields:
      - { name: seconds, type: u64 }
"#;

    const EXTENSIBLE_LIBRARY: &str = r#"
format: 1
library: pwright.options
version: { major: 1, minor: 0 }
types:
  - kind: record
    id: pwright.options.click
    encoding: extensible
    encoded_size: { minimum: 0, maximum: 256 }
    fields:
      - { tag: 1, name: force, type: bool, cardinality: optional }
      - { tag: 10, name: label, type: utf8, cardinality: optional, constraints: { maximum_length: 64 } }
"#;

    const REPRESENTATIVE_PROTOCOL: &str = r#"
format: 1
protocol: pwright.control
version: { major: 1, minor: 0 }
root: pwright.control
types:
  - kind: tagged_union
    id: pwright.control
    variants:
      - tag: 2
        name: dispose
        fields:
          - { name: object_id, type: u64 }
"#;

    const IMPORTING_CAPABILITY: &str = r#"
format: 1
capability: pwright.test
version: { major: 1, minor: 0 }
root: pwright.test.message
imports:
  - pwright.common
types:
  - kind: tagged_union
    id: pwright.test.message
    variants:
      - tag: 1
        name: command
        fields:
          - { name: duration, type: message, target: pwright.common.duration }
"#;

    const COMMAND_CAPABILITY: &str = r#"
format: 1
capability: pwright.test
version: { major: 1, minor: 0 }
root: pwright.test.message
commands:
  - id: test.execute-command
    name: execute
    method: 17
    targets: [pwright.browser.page]
    command: execute_command
    result: execute_result
    params_type: pwright.test.params
    result_type: pwright.test.result
    error_details_type: pwright.test.error
types:
  - kind: enum
    id: pwright.test.error
    public_name: test_error
    variants:
      - { tag: 0, name: failed }
  - kind: tagged_union
    id: pwright.test.message
    variants:
      - tag: 1
        name: execute_command
        fields:
          - { name: params, type: message, target: pwright.test.params }
      - tag: 2
        name: execute_result
        fields:
          - { name: result, type: message, target: pwright.test.result }
          - { name: error, type: enum, target: pwright.test.error, cardinality: optional }
  - kind: record
    id: pwright.test.params
    public_name: execute_params
    fields:
      - { name: value, type: utf8 }
  - kind: record
    id: pwright.test.result
    public_name: execute_result
    fields:
      - { name: value, type: utf8 }
"#;

    #[test]
    fn representative_schema_generates_models_and_catalog() {
        // Arrange
        let schema = parse_and_validate(REPRESENTATIVE).expect("schema should parse");

        // Act
        let generated = generate_rust(&schema).expect("schema should generate");

        // Assert
        assert!(generated.contains("pub struct PwrightTestChild"));
        assert!(generated.contains("pub enum PwrightTestMessage"));
        assert!(generated.contains("pub struct PwrightTestChildRef<'a>"));
        assert!(generated.contains("pub enum PwrightTestMessageRef<'a>"));
        assert!(generated.contains("impl BinaryEncode for PwrightTestMessageRef<'_>"));
        assert!(generated.contains("pub const CATALOG: BinarySchemaCatalog"));
        assert!(generated.contains("pub const ROOT_SCHEMA: BinarySchema"));
        assert!(generated.contains("BinaryFieldSchema::message"));
    }

    #[test]
    fn extensible_record_generates_bounded_tagged_codec_and_catalog_metadata() {
        // Arrange
        let schema =
            parse_and_validate(EXTENSIBLE_LIBRARY).expect("extensible schema should parse");

        // Act
        let generated = generate_rust(&schema).expect("extensible schema should generate");
        let reference = generate_markdown_reference(&schema)
            .expect("extensible schema reference should generate");

        // Assert
        assert!(generated.contains(
            "encode_extensible_record(writer, \"pwright.options.click\", BinarySizeRange::new(Some(0_u32), Some(256_u32))"
        ));
        assert!(generated.contains("record.field(1, \"pwright.options.click.force\", |writer|"));
        assert!(generated.contains(
            "decode_extensible_record(reader, \"pwright.options.click\", BinarySizeRange::new(Some(0_u32), Some(256_u32))"
        ));
        assert!(
            generated
                .contains("BinaryExtensibleFieldSchema::new(10, BinaryFieldSchema::new(\"label\"")
        );
        assert!(
            generated.contains("BinaryTypeSchema::ExtensibleRecord(PWRIGHT_OPTIONS_CLICK_SCHEMA)")
        );
        assert!(reference.contains("| `pwright.options.click` | extensible record |"));
        assert!(reference.contains("| `10` | `label` | `utf8` | optional |"));
        assert_eq!(
            generated,
            include_str!("fixtures/generated_extensible_record.rs")
        );
    }

    #[test]
    fn extensible_record_field_named_value_does_not_shadow_its_slot() {
        // Arrange
        let source = EXTENSIBLE_LIBRARY.replace("tag: 10, name: label", "tag: 10, name: value");
        let schema = parse_and_validate(&source).expect("schema should parse");

        // Act
        let generated = generate_rust(&schema).expect("schema should generate");

        // Assert
        assert!(generated.contains("let decoded_value = pwright_codec_binary::decode_utf8"));
        assert!(generated.contains("value = Some(decoded_value);"));
    }

    #[test]
    fn one_field_extensible_record_keeps_generated_clippy_clean() {
        // Arrange
        let source = EXTENSIBLE_LIBRARY.replace(
            "      - { tag: 10, name: label, type: utf8, cardinality: optional, constraints: { maximum_length: 64 } }\n",
            "",
        );
        let schema = parse_and_validate(&source).expect("schema should parse");

        // Act
        let generated = generate_rust(&schema).expect("schema should generate");

        // Assert
        assert!(
            generated.contains("#[allow(clippy::single_match)]\n                match field.tag()")
        );
    }

    #[test]
    fn compiled_extensible_fixture_round_trips_while_skipping_an_unknown_tag() {
        // Arrange
        let value = generated_extensible_record::PwrightOptionsClick {
            force: Some(true),
            label: Some("submit".to_owned()),
        };
        let limits = BinaryValueLimits::new(64, 0, 0, 4, 4);
        let mut writer = BinaryWriter::new();
        let mut context = BinaryValueContext::new(limits);
        value
            .encode_binary(&mut writer, &mut context)
            .expect("generated value should encode");
        let encoded = writer.into_bytes();
        let first_field_end = 4 + 2 + 4 + 1;
        let unknown = [0, 5, 0, 0, 0, 1, 99];
        let mut with_unknown = Vec::new();
        with_unknown.extend_from_slice(&encoded[..first_field_end]);
        with_unknown.extend_from_slice(&unknown);
        with_unknown.extend_from_slice(&encoded[first_field_end..]);
        let body_length = u32::from_be_bytes(with_unknown[..4].try_into().expect("length prefix"));
        with_unknown[..4].copy_from_slice(&(body_length + unknown.len() as u32).to_be_bytes());
        let mut reader = BinaryReader::new(&with_unknown);
        let mut context = BinaryValueContext::new(limits);

        // Act
        let decoded = generated_extensible_record::PwrightOptionsClick::decode_binary(
            &mut reader,
            &mut context,
        )
        .expect("generated decoder should skip unknown tag");

        // Assert
        assert_eq!(decoded, value);
        assert_eq!(reader.finish(), Ok(()));
        assert_eq!(generated_extensible_record::CATALOG.validate(), Ok(()));
    }

    #[test]
    fn extensible_record_minor_compatibility_allows_only_new_optional_tags() {
        // Arrange
        let previous =
            parse_and_validate(EXTENSIBLE_LIBRARY).expect("previous schema should parse");
        let added_optional_source = EXTENSIBLE_LIBRARY
            .replace("minor: 0", "minor: 1")
            .replace(
                "      - { tag: 10, name: label",
                "      - { tag: 5, name: trial, type: bool, cardinality: optional }\n      - { tag: 10, name: label",
            );
        let added_required_source = added_optional_source.replace(
            "tag: 5, name: trial, type: bool, cardinality: optional",
            "tag: 5, name: trial, type: bool, cardinality: required",
        );
        let resized_source = EXTENSIBLE_LIBRARY
            .replace("minor: 0", "minor: 1")
            .replace("maximum: 256", "maximum: 512");
        let added_optional =
            parse_and_validate(&added_optional_source).expect("optional tag should parse");
        let added_required =
            parse_and_validate(&added_required_source).expect("required tag should parse");
        let resized = parse_and_validate(&resized_source).expect("resized schema should parse");

        // Act
        let compatible = check_minor_compatibility(&previous, &added_optional);
        let required_error = check_minor_compatibility(&previous, &added_required);
        let resized_error = check_minor_compatibility(&previous, &resized);

        // Assert
        assert_eq!(compatible, Ok(()));
        assert_eq!(
            required_error,
            Err(CompatibilityError::NonOptionalExtensibleFieldAdded {
                id: "pwright.options.click".to_owned(),
                tag: 5,
            })
        );
        assert_eq!(
            resized_error,
            Err(CompatibilityError::ExtensibleRecordEncodedSizeChanged {
                id: "pwright.options.click".to_owned(),
            })
        );
    }

    #[test]
    fn extensible_record_requires_bounded_size_and_canonical_field_tags() {
        // Arrange
        let missing_size =
            EXTENSIBLE_LIBRARY.replace("    encoded_size: { minimum: 0, maximum: 256 }\n", "");
        let unbounded = EXTENSIBLE_LIBRARY.replace(
            "encoded_size: { minimum: 0, maximum: 256 }",
            "encoded_size: { minimum: 0 }",
        );
        let reserved = EXTENSIBLE_LIBRARY.replacen("tag: 1", "tag: 0", 1);
        let reversed = EXTENSIBLE_LIBRARY.replacen("tag: 1", "tag: 11", 1);

        // Act
        let missing_size_error = parse_and_validate(&missing_size);
        let unbounded_error = parse_and_validate(&unbounded);
        let reserved_error = parse_and_validate(&reserved);
        let reversed_error = parse_and_validate(&reversed);

        // Assert
        for error in [missing_size_error, unbounded_error] {
            assert!(matches!(
                error,
                Err(SchemaError::MissingOrInvalidExtensibleRecordEncodedSize { .. })
            ));
        }
        assert!(matches!(
            reserved_error,
            Err(SchemaError::NonCanonicalExtensibleFieldTag {
                previous: None,
                current: 0,
                ..
            })
        ));
        assert!(matches!(
            reversed_error,
            Err(SchemaError::NonCanonicalExtensibleFieldTag {
                previous: Some(11),
                current: 10,
                ..
            })
        ));
    }

    #[test]
    fn protocol_family_generates_a_root_catalog_without_capability_metadata() {
        // Arrange
        let schema =
            parse_and_validate(REPRESENTATIVE_PROTOCOL).expect("protocol family should parse");

        // Act
        let generated = generate_rust(&schema).expect("protocol family should generate");
        let reference =
            generate_markdown_reference(&schema).expect("protocol reference should generate");

        // Assert
        assert!(generated.contains("pub enum PwrightControl"));
        assert!(generated.contains("pub const ROOT_SCHEMA: BinarySchema"));
        assert!(!generated.contains("BinaryCommandSchema"));
        assert!(reference.contains("| Kind | Protocol |"));
    }

    #[test]
    fn protocol_family_requires_a_root_and_forbids_capability_metadata() {
        // Arrange
        let missing_root = REPRESENTATIVE_PROTOCOL.replace("root: pwright.control\n", "");
        let commands =
            COMMAND_CAPABILITY.replace("capability: pwright.test", "protocol: pwright.test");
        let dependencies = REPRESENTATIVE_PROTOCOL.replace(
            "root: pwright.control\n",
            "root: pwright.control\ndependencies:\n  - id: pwright.base\n    minimum_version: { major: 1, minor: 0 }\n",
        );

        // Act
        let missing_root_error =
            parse_and_validate(&missing_root).expect_err("protocol root is required");
        let command_error =
            parse_and_validate(&commands).expect_err("protocol commands must be rejected");
        let dependency_error = parse_and_validate(&dependencies)
            .expect_err("protocol capability dependencies must be rejected");

        // Assert
        assert_eq!(missing_root_error, SchemaError::ProtocolRootMissing);
        assert_eq!(command_error, SchemaError::ProtocolHasCommands);
        assert_eq!(dependency_error, SchemaError::ProtocolHasDependencies);
    }

    #[test]
    fn protocol_family_cannot_become_a_capability_in_a_minor_release() {
        // Arrange
        let previous =
            parse_and_validate(REPRESENTATIVE_PROTOCOL).expect("protocol family should parse");
        let next_source = REPRESENTATIVE_PROTOCOL
            .replace("protocol: pwright.control", "capability: pwright.control")
            .replace(
                "version: { major: 1, minor: 0 }",
                "version: { major: 1, minor: 1 }",
            );
        let next = parse_and_validate(&next_source).expect("capability schema should parse");

        // Act
        let error = check_minor_compatibility(&previous, &next)
            .expect_err("schema identity kind must remain exact");

        // Assert
        assert_eq!(
            error,
            CompatibilityError::IdentityKindChanged {
                previous: "Protocol",
                next: "Capability",
            }
        );
    }

    #[test]
    fn externally_counted_segments_generate_without_inline_length_prefixes() {
        // Arrange
        let source = r#"
format: 1
protocol: pwright.segmented
version: { major: 1, minor: 0 }
root: pwright.segmented
types:
  - kind: tagged_union
    id: pwright.segmented
    variants:
      - tag: 1
        name: batch
        fields:
          - { name: item_count, type: u32 }
          - { name: items, type: u16, cardinality: repeated, counted_by: item_count }
"#;
        let schema = parse_and_validate(source).expect("external count should validate");

        // Act
        let generated = generate_rust(&schema).expect("external count should generate");
        let reference =
            generate_markdown_reference(&schema).expect("external count should be documented");

        // Assert
        assert!(
            generated.contains(
                "context.check_repeated(\"pwright.segmented.batch.items\", items.len())?;"
            )
        );
        assert!(generated.contains("BinarySizeRange::new(Some(*item_count), Some(*item_count))"));
        assert!(generated.contains("let length = usize::try_from(u64::from(item_count))"));
        assert!(generated.contains(".with_external_count(\"item_count\")"));
        assert!(
            !generated
                .contains("encode_repeated_len(writer, context, \"pwright.segmented.batch.items\"")
        );
        assert!(reference.contains("counted by `item_count`"));
    }

    #[test]
    fn counted_by_requires_an_earlier_required_u32_field() {
        // Arrange
        let base = r#"
format: 1
protocol: pwright.segmented
version: { major: 1, minor: 0 }
root: pwright.segmented
types:
  - kind: tagged_union
    id: pwright.segmented
    variants:
      - tag: 1
        name: batch
        fields:
          - { name: item_count, type: u32 }
          - { name: items, type: u16, cardinality: repeated, counted_by: item_count }
"#;
        let missing = base.replace("counted_by: item_count", "counted_by: missing");
        let wrong_kind = base.replace("name: item_count, type: u32", "name: item_count, type: u16");
        let not_repeated = base.replace("type: u16, cardinality: repeated", "type: u16");

        // Act
        let missing_error = parse_and_validate(&missing).expect_err("missing count must fail");
        let kind_error = parse_and_validate(&wrong_kind).expect_err("wrong count kind must fail");
        let cardinality_error =
            parse_and_validate(&not_repeated).expect_err("non-repeated count use must fail");

        // Assert
        assert!(matches!(
            missing_error,
            SchemaError::CountFieldMissingOrNotEarlier { .. }
        ));
        assert!(matches!(kind_error, SchemaError::InvalidCountField { .. }));
        assert!(matches!(
            cardinality_error,
            SchemaError::CountedByOnNonRepeatedField { .. }
        ));
    }

    #[test]
    fn command_metadata_generates_stable_ids_methods_and_root_tags() {
        // Arrange
        let schema = parse_and_validate(COMMAND_CAPABILITY).expect("command schema should parse");

        // Act
        let generated = generate_rust(&schema).expect("command schema should generate");

        // Assert
        assert!(generated.contains(
            "pub const COMMAND_TEST_EXECUTE_COMMAND_SCHEMA: BinaryCommandSchema = BinaryCommandSchema::new(CommandId::new(\"test.execute-command\"), MethodTag::new(17), 1, 2);"
        ));
        assert!(generated.contains(
            "pub const COMMANDS: &[BinaryCommandSchema] = &[\n    COMMAND_TEST_EXECUTE_COMMAND_SCHEMA,\n];"
        ));
    }

    #[test]
    fn capability_surface_generation_has_only_core_metadata() {
        // Arrange
        let schema = parse_and_validate(COMMAND_CAPABILITY).expect("command schema should parse");

        // Act
        let generated =
            generate_capability_rust(&schema).expect("capability surface should generate");

        // Assert
        assert!(generated.contains(
            "pub const TEST_CAPABILITY_ID: CapabilityId = CapabilityId::new(\"pwright.test\");"
        ));
        assert!(generated.contains(
            "pub const EXECUTE_COMMAND_ID: CommandId = CommandId::new(\"test.execute-command\");"
        ));
        assert!(generated.contains("pub const EXECUTE_METHOD: MethodTag = MethodTag::new(17);"));
        assert!(generated.contains(
            "pub const EXECUTE_TARGET_OBJECT_KINDS: &[ObjectKindId] = &[\n    ObjectKindId::new(\"pwright.browser.page\"),\n];"
        ));
        assert!(generated.contains(
            "pub const TEST_COMMAND_DESCRIPTORS: &[CapabilityCommandDescriptor] = &[\n    CapabilityCommandDescriptor::new(\n        EXECUTE_COMMAND_ID,\n        EXECUTE_METHOD,\n        EXECUTE_TARGET_OBJECT_KINDS,\n    ),\n];"
        ));
        assert!(generated.contains("pub struct TestCapability;"));
        assert!(generated.contains("impl Capability for TestCapability"));
        assert!(generated.contains("pub struct ExecuteParams"));
        assert!(generated.contains("pub struct ExecuteResult"));
        assert!(generated.contains("pub enum TestError"));
        assert!(generated.contains("pub struct Execute;"));
        assert!(generated.contains("type Params = ExecuteParams;"));
        assert!(generated.contains("type Result = ExecuteResult;"));
        assert!(generated.contains("type ErrorDetails = TestError;"));
        assert!(generated.contains("impl CapabilityCommandSpec for Execute"));
        assert!(generated.contains(
            "const TARGET_OBJECT_KINDS: &'static [ObjectKindId] = EXECUTE_TARGET_OBJECT_KINDS;"
        ));
        assert!(!generated.contains("pwright_codec"));
        assert!(!generated.contains("serde"));
    }

    #[test]
    fn policy_owned_public_models_can_remain_external_to_generated_capability_code() {
        // Arrange
        let source = COMMAND_CAPABILITY
            .replace(
                "    public_name: test_error\n",
                "    public_name: test_error\n    public_external: true\n",
            )
            .replace(
                "    public_name: execute_params\n",
                "    public_name: execute_params\n    public_external: true\n",
            )
            .replace(
                "    public_name: execute_result\n",
                "    public_name: execute_result\n    public_external: true\n",
            )
            .replace(
                "  - kind: record\n    id: pwright.test.params\n",
                "  - kind: record\n    id: pwright.test.options\n    public_name: test_options\n    public_external: true\n    fields:\n      - { name: enabled, type: bool }\n  - kind: record\n    id: pwright.test.params\n",
            )
            .replace(
                "    fields:\n      - { name: value, type: utf8 }\n  - kind: record\n    id: pwright.test.result\n",
                "    fields:\n      - { name: value, type: utf8 }\n      - { name: options, type: message, target: pwright.test.options }\n  - kind: record\n    id: pwright.test.result\n",
            );
        let schema = parse_and_validate(&source).expect("external public models should validate");

        // Act
        let capability =
            generate_capability_rust(&schema).expect("capability metadata should generate");
        let codec = generate_rust(&schema).expect("wire models should still generate");

        // Assert
        assert!(capability.contains("use super::{TestError, ExecuteParams, ExecuteResult};"));
        assert!(!capability.contains("TestOptions"));
        assert!(!capability.contains("pub struct ExecuteParams"));
        assert!(capability.contains("type Params = ExecuteParams;"));
        assert!(capability.contains("type Result = ExecuteResult;"));
        assert!(!codec.contains("PublicExecuteParams"));
        assert!(!codec.contains("pwright_cap_test"));
    }

    #[test]
    fn protocol_reference_is_deterministic_and_covers_commands_models_and_constraints() {
        // Arrange
        let source = COMMAND_CAPABILITY
            .replace(
                "    public_name: test_error\n",
                "    public_name: test_error\n    default: failed\n",
            )
            .replace(
                "      - { name: value, type: utf8 }",
                "      - name: value\n        type: utf8\n        constraints: { minimum_length: 1, maximum_length: 64 }",
            );
        let schema = parse_and_validate(&source).expect("command schema should parse");

        // Act
        let first =
            generate_markdown_reference(&schema).expect("protocol reference should generate");
        let second =
            generate_markdown_reference(&schema).expect("protocol reference should repeat");

        // Assert
        assert_eq!(first, second);
        assert!(first.starts_with("<!-- agent-updated: 2026-09-03T22:08:32Z -->"));
        assert!(first.ends_with('\n'));
        assert!(!first.ends_with("\n\n"));
        assert!(first.contains("# `pwright.test` protocol reference"));
        assert!(
            first
                .contains("| `execute` | `test.execute-command` | `17` | `pwright.browser.page` |")
        );
        assert!(first.contains("| `pwright.test.params` | record | `ExecuteParams` |"));
        assert!(first.contains("| 1 | `value` | `utf8` | required | bytes 1..=64 |"));
        assert!(first.contains("| `0` | `failed` | yes |"));
    }

    #[test]
    fn server_binding_generates_one_complete_concurrent_typed_handler_chain() {
        // Arrange
        let two_commands = COMMAND_CAPABILITY
            .replace(
                "types:\n",
                "  - id: test.inspect-command\n    name: inspect\n    method: 18\n    targets: [pwright.browser.page]\n    command: inspect_command\n    result: inspect_result\n    params_type: pwright.test.params\n    result_type: pwright.test.result\n    error_details_type: pwright.test.error\ntypes:\n",
            )
            .replace(
                "          - { name: error, type: enum, target: pwright.test.error, cardinality: optional }\n  - kind: record\n",
                "          - { name: error, type: enum, target: pwright.test.error, cardinality: optional }\n      - tag: 3\n        name: inspect_command\n        fields:\n          - { name: params, type: message, target: pwright.test.params }\n      - tag: 4\n        name: inspect_result\n        fields:\n          - { name: result, type: message, target: pwright.test.result }\n          - { name: error, type: enum, target: pwright.test.error, cardinality: optional }\n  - kind: record\n",
            );
        let schema = parse_and_validate(&two_commands).expect("two commands should parse");
        let singleton_schema =
            parse_and_validate(COMMAND_CAPABILITY).expect("one command should parse");
        let library = parse_and_validate(REPRESENTATIVE_LIBRARY).expect("library should parse");

        // Act
        let generated = generate_server_rust(&schema).expect("server binding should generate");
        let singleton =
            generate_server_rust(&singleton_schema).expect("singleton binding should generate");
        let library_error =
            generate_server_rust(&library).expect_err("a library has no server handlers");

        // Assert
        assert!(generated.contains("use pwright_cap_test::{Execute, Inspect};"));
        assert!(generated.contains("use pwright_codec_test::TestCodec;"));
        assert!(generated.contains(
            "pub type TestServerHandlers<ExecuteService, InspectService> = ConcurrentServerHandlerChain<"
        ));
        assert!(
            generated.contains("pub struct TestServerServices<ExecuteService, InspectService> {")
        );
        assert!(generated.contains("pub execute: Arc<ExecuteService>"));
        assert!(generated.contains("pub inspect: Arc<InspectService>"));
        assert!(!generated.contains("#[allow(clippy::type_complexity)]"));
        assert!(generated.contains("pub fn test_server_handlers<ExecuteService, InspectService>("));
        assert!(generated.contains("services: TestServerServices<ExecuteService, InspectService>"));
        assert!(generated.contains("ConcurrentTypedCommandHandler::from_shared("));
        assert!(generated.contains("ConcurrentServerHandlerChain::new(execute, inspect)"));
        assert!(!generated.contains("pwright_runtime"));
        assert!(!generated.contains("pwright_backend"));
        assert!(!generated.contains("tokio"));
        assert!(singleton.contains("use pwright_server_wire::ConcurrentTypedCommandHandler;"));
        assert!(!singleton.contains("ConcurrentServerHandlerChain"));
        assert!(singleton.contains(
            "pub type TestServerHandlers<ExecuteService> = ConcurrentTypedCommandHandler<Execute, TestCodec, ExecuteService>;"
        ));
        assert!(singleton.contains("pub struct TestServerServices<ExecuteService>"));
        assert_eq!(library_error, SchemaError::ServerOutputRequiresCapability);
    }

    #[test]
    fn server_handler_generation_balances_large_type_trees() {
        // Arrange
        let handlers = ["A", "B", "C", "D"].map(str::to_owned);

        // Act
        let handler_type = balanced_server_handler_type(&handlers);
        let expression = balanced_server_handler_expression(&handlers);

        // Assert
        assert_eq!(
            handler_type,
            "ConcurrentServerHandlerChain<ConcurrentServerHandlerChain<A, B>, ConcurrentServerHandlerChain<C, D>>"
        );
        assert_eq!(
            expression,
            "ConcurrentServerHandlerChain::new(ConcurrentServerHandlerChain::new(A, B), ConcurrentServerHandlerChain::new(C, D))"
        );
    }

    #[test]
    fn server_binding_scopes_nondefault_major_markers_codec_and_names() {
        // Arrange
        let source = COMMAND_CAPABILITY.replace(
            "version: { major: 1, minor: 0 }",
            "version: { major: 2, minor: 0 }",
        );
        let schema = parse_and_validate(&source).expect("major 2 capability should parse");

        // Act
        let generated =
            generate_server_rust(&schema).expect("major 2 server binding should generate");

        // Assert
        assert!(generated.contains("use pwright_cap_test::v2::{Execute};"));
        assert!(generated.contains("use pwright_codec_test::TestV2Codec;"));
        assert!(generated.contains("pub type TestV2ServerHandlers<ExecuteService>"));
        assert!(generated.contains("pub struct TestV2ServerServices<ExecuteService>"));
        assert!(generated.contains("pub fn test_v2_server_handlers<ExecuteService>("));
        assert!(generated.contains("WireProtocol<TestV2Codec>"));
    }

    #[test]
    fn explicit_minor_binding_scopes_public_conversions_and_server_without_changing_wire_identity()
    {
        // Arrange
        let root = parse_and_validate(COMMAND_CAPABILITY).unwrap();
        let source = COMMAND_CAPABILITY.replace(
            "root: pwright.test.message",
            "rust_minor_module: true\nroot: pwright.test.message",
        );
        let selected = parse_and_validate(&source).unwrap();
        // Act
        let codec = generate_rust(&selected).unwrap();
        let server = generate_server_rust(&selected).unwrap();
        // Assert
        assert!(codec.contains("use pwright_cap_test::v1_0::{"));
        assert!(server.contains("use pwright_cap_test::v1_0::{Execute};"));
        assert!(server.contains("use pwright_codec_test::v1_0::TestCodec;"));
        assert_eq!(
            generate_capability_rust(&root).unwrap(),
            generate_capability_rust(&selected).unwrap()
        );
        assert_eq!(
            codec.replace("pwright_cap_test::v1_0::", "pwright_cap_test::"),
            generate_rust(&root).unwrap()
        );
        check_minor_compatibility(&root, &selected).unwrap();
    }

    #[test]
    fn minor_binding_selection_rejects_noncapability_schemas() {
        // Arrange
        let source = include_str!("../../pwright-codec-common/schema/common.yml")
            .replace("format: 1", "format: 1\nrust_minor_module: true");
        // Act
        let result = parse_and_validate(&source);
        // Assert
        assert_eq!(
            result.unwrap_err(),
            SchemaError::RustMinorModuleRequiresCapability
        );
    }

    #[test]
    fn navigation_minor_revision_adds_frame_command_without_changing_published_page_contracts() {
        // Arrange
        let imports =
            [
                parse_and_validate(include_str!("../../pwright-codec-common/schema/common.yml"))
                    .unwrap(),
            ];
        let previous = parse_and_validate_with_imports(
            include_str!("../../pwright-cap-navigation/schema/baselines/1.1/navigation.yml"),
            &imports,
        )
        .unwrap();
        let selected = parse_and_validate_with_imports(
            include_str!("../../pwright-cap-navigation/schema/navigation-1.2.yml"),
            &imports,
        )
        .unwrap();
        // Act
        let result =
            check_minor_compatibility_with_imports(&previous, &selected, &imports, &imports);
        // Assert
        result.unwrap();
        assert!(selected.rust_minor_module);
        assert_eq!(selected.commands.len(), 4);
        for command in &previous.commands {
            assert!(selected.commands.contains(command));
        }
        let frame = selected
            .commands
            .iter()
            .find(|command| command.name == "frame_goto")
            .unwrap();
        assert_eq!(frame.method, 65540);
        assert_eq!(frame.targets, ["pwright.browser.frame"]);
        assert_eq!(
            include_str!("../../pwright-cap-navigation/schema/navigation.yml"),
            include_str!("../../pwright-cap-navigation/schema/baselines/1.1/navigation.yml")
        );
    }

    #[test]
    fn capability_surface_generates_declared_dependency_minimums() {
        // Arrange
        let source = COMMAND_CAPABILITY.replace(
            "root: pwright.test.message\n",
            "root: pwright.test.message\ndependencies:\n  - id: pwright.base\n    minimum_version: { major: 1, minor: 2 }\n",
        );
        let schema = parse_and_validate(&source).expect("dependency should be valid");

        // Act
        let generated =
            generate_capability_rust(&schema).expect("capability surface should generate");

        // Assert
        assert!(generated.contains("CapabilityDependency"));
        assert!(generated.contains(
            "CapabilityDependency::new(CapabilityId::new(\"pwright.base\"), ProtocolVersion::new(1, 2))"
        ));
        assert!(generated.contains("        TEST_DEPENDENCIES,"));
    }

    #[test]
    fn capability_cannot_depend_on_itself() {
        // Arrange
        let source = COMMAND_CAPABILITY.replace(
            "root: pwright.test.message\n",
            "root: pwright.test.message\ndependencies:\n  - id: pwright.test\n    minimum_version: { major: 1, minor: 0 }\n",
        );

        // Act
        let error = parse_and_validate(&source).expect_err("self dependency must fail");

        // Assert
        assert_eq!(
            error,
            SchemaError::SelfDependency {
                capability: "pwright.test".to_owned(),
            }
        );
    }

    #[test]
    fn capability_dependencies_must_be_canonically_ordered() {
        // Arrange
        let source = COMMAND_CAPABILITY.replace(
            "root: pwright.test.message\n",
            "root: pwright.test.message\ndependencies:\n  - id: pwright.zeta\n    minimum_version: { major: 1, minor: 0 }\n  - id: pwright.alpha\n    minimum_version: { major: 1, minor: 0 }\n",
        );

        // Act
        let error = parse_and_validate(&source).expect_err("unordered dependencies must fail");

        // Assert
        assert_eq!(
            error,
            SchemaError::NonCanonicalDependencyOrder {
                previous: "pwright.zeta".to_owned(),
                current: "pwright.alpha".to_owned(),
            }
        );
    }

    #[test]
    fn wire_libraries_cannot_declare_capability_dependencies() {
        // Arrange
        let source = REPRESENTATIVE_LIBRARY.replace(
            "version: { major: 1, minor: 0 }\n",
            "version: { major: 1, minor: 0 }\ndependencies:\n  - id: pwright.base\n    minimum_version: { major: 1, minor: 0 }\n",
        );

        // Act
        let error = parse_and_validate(&source).expect_err("library dependency must fail");

        // Assert
        assert_eq!(error, SchemaError::LibraryHasDependencies);
    }

    #[test]
    fn dependency_changes_require_a_new_major_version() {
        // Arrange
        let source = COMMAND_CAPABILITY.replace(
            "root: pwright.test.message\n",
            "root: pwright.test.message\ndependencies:\n  - id: pwright.base\n    minimum_version: { major: 1, minor: 0 }\n",
        );
        let previous = parse_and_validate(&source).expect("previous should parse");
        let next_source = source
            .replacen(
                "version: { major: 1, minor: 0 }",
                "version: { major: 1, minor: 1 }",
                1,
            )
            .replace(
                "minimum_version: { major: 1, minor: 0 }",
                "minimum_version: { major: 1, minor: 1 }",
            );
        let next = parse_and_validate(&next_source).expect("next should parse");

        // Act
        let error = check_minor_compatibility(&previous, &next)
            .expect_err("dependency minimum change must fail");

        // Assert
        assert_eq!(error, CompatibilityError::DependenciesChanged);
    }

    #[test]
    fn codec_generation_owns_bidirectional_semantic_model_conversions() {
        // Arrange
        let schema = parse_and_validate(COMMAND_CAPABILITY).expect("command schema should parse");

        // Act
        let generated = generate_rust(&schema).expect("codec surface should generate");

        // Assert
        assert!(generated.contains("use pwright_cap_test::{"));
        assert!(generated.contains("ExecuteParams as PublicExecuteParams"));
        assert!(generated.contains("ExecuteResult as PublicExecuteResult"));
        assert!(generated.contains("TestError as PublicTestError"));
        assert!(generated.contains("impl From<PwrightTestParams> for PublicExecuteParams"));
        assert!(generated.contains("impl From<PublicExecuteParams> for PwrightTestParams"));
        assert!(generated.contains("impl From<PwrightTestError> for PublicTestError"));
        assert!(generated.contains("impl From<PublicTestError> for PwrightTestError"));
    }

    #[test]
    fn multi_field_union_conversions_use_shorthand_for_unchanged_fields() {
        // Arrange
        let source = r#"
format: 1
capability: pwright.test
version: { major: 1, minor: 0 }
root: pwright.test.message
types:
  - kind: tagged_union
    id: pwright.test.message
    public_name: test_message
    variants:
      - tag: 1
        name: cleanup
        fields:
          - { name: primary, type: utf8 }
          - { name: cleanup, type: utf8 }
"#;
        let schema = parse_and_validate(source).expect("union schema should parse");

        // Act
        let generated = generate_rust(&schema).expect("codec surface should generate");

        // Assert
        assert!(
            generated
                .contains("Self::Cleanup {\n                primary,\n                cleanup,")
        );
        assert!(!generated.contains("primary: primary"));
        assert!(!generated.contains("cleanup: cleanup"));
    }

    #[test]
    fn enum_defaults_generate_for_public_and_wire_models() {
        // Arrange
        let source = COMMAND_CAPABILITY.replace(
            "    public_name: test_error\n",
            "    public_name: test_error\n    default: failed\n",
        );
        let schema = parse_and_validate(&source).expect("enum default should be valid");

        // Act
        let capability =
            generate_capability_rust(&schema).expect("capability surface should generate");
        let codec = generate_rust(&schema).expect("codec surface should generate");

        // Assert
        assert!(capability.contains(
            "#[derive(Clone, Copy, Debug, Eq, PartialEq, Default)]\npub enum TestError {\n    #[default]\n    Failed,"
        ));
        assert!(codec.contains(
            "#[derive(Clone, Debug, Eq, PartialEq, Default)]\npub enum PwrightTestError {\n    #[default]\n    Failed,"
        ));
    }

    #[test]
    fn enum_default_must_name_an_existing_variant() {
        // Arrange
        let source = COMMAND_CAPABILITY.replace(
            "    public_name: test_error\n",
            "    public_name: test_error\n    default: missing\n",
        );

        // Act
        let error = parse_and_validate(&source).expect_err("unknown default must fail");

        // Assert
        assert_eq!(
            error,
            SchemaError::UnknownEnumDefault {
                type_id: "pwright.test.error".to_owned(),
                variant: "missing".to_owned(),
            }
        );
    }

    #[test]
    fn enum_default_changes_require_a_new_major_version() {
        // Arrange
        let previous_source = COMMAND_CAPABILITY.replace(
            "    public_name: test_error\n",
            "    public_name: test_error\n    default: failed\n",
        );
        let next_source = previous_source
            .replacen(
                "version: { major: 1, minor: 0 }",
                "version: { major: 1, minor: 1 }",
                1,
            )
            .replace("    default: failed\n", "");
        let previous = parse_and_validate(&previous_source).expect("previous should parse");
        let next = parse_and_validate(&next_source).expect("next should parse");

        // Act
        let error =
            check_minor_compatibility(&previous, &next).expect_err("default removal must fail");

        // Assert
        assert_eq!(
            error,
            CompatibilityError::PositionalLayoutChanged {
                id: "pwright.test.error".to_owned(),
            }
        );
    }

    #[test]
    fn commands_must_reference_distinct_root_variants() {
        // Arrange
        let unknown = COMMAND_CAPABILITY.replace("result: execute_result", "result: missing");
        let reused =
            COMMAND_CAPABILITY.replace("result: execute_result", "result: execute_command");

        // Act
        let unknown_error =
            parse_and_validate(&unknown).expect_err("unknown result variant must fail");
        let reused_error = parse_and_validate(&reused).expect_err("reused root variant must fail");

        // Assert
        assert_eq!(
            unknown_error,
            SchemaError::UnknownCommandVariant {
                command: "test.execute-command".to_owned(),
                role: "result",
                variant: "missing".to_owned(),
            }
        );
        assert_eq!(
            reused_error,
            SchemaError::ReusedCommandVariant {
                variant: "execute_command".to_owned(),
            }
        );
    }

    #[test]
    fn commands_must_declare_at_least_one_target_object_kind() {
        // Arrange
        let source = COMMAND_CAPABILITY.replace("    targets: [pwright.browser.page]\n", "");

        // Act
        let error = parse_and_validate(&source).expect_err("empty target list must fail");

        // Assert
        assert_eq!(
            error,
            SchemaError::EmptyCommandTargets {
                command: "test.execute-command".to_owned(),
            }
        );
    }

    #[test]
    fn command_target_object_kinds_must_be_canonically_ordered() {
        // Arrange
        let source = COMMAND_CAPABILITY.replace(
            "    targets: [pwright.browser.page]\n",
            "    targets: [pwright.browser.page, pwright.browser.frame]\n",
        );

        // Act
        let error = parse_and_validate(&source).expect_err("unordered targets must fail");

        // Assert
        assert_eq!(
            error,
            SchemaError::NonCanonicalCommandTargetOrder {
                command: "test.execute-command".to_owned(),
                previous: "pwright.browser.page".to_owned(),
                current: "pwright.browser.frame".to_owned(),
            }
        );
    }

    #[test]
    fn command_target_changes_require_a_new_major_version() {
        // Arrange
        let previous = parse_and_validate(COMMAND_CAPABILITY).expect("previous should parse");
        let next_source = COMMAND_CAPABILITY
            .replacen(
                "version: { major: 1, minor: 0 }",
                "version: { major: 1, minor: 1 }",
                1,
            )
            .replace(
                "    targets: [pwright.browser.page]\n",
                "    targets: [pwright.browser.frame]\n",
            );
        let next = parse_and_validate(&next_source).expect("next should parse");

        // Act
        let error =
            check_minor_compatibility(&previous, &next).expect_err("target changes must fail");

        // Assert
        assert_eq!(
            error,
            CompatibilityError::CommandChanged {
                id: "test.execute-command".to_owned(),
            }
        );
    }

    #[test]
    fn command_semantic_models_require_published_public_names() {
        // Arrange
        let source = COMMAND_CAPABILITY.replace("    public_name: execute_params\n", "");

        // Act
        let error = parse_and_validate(&source)
            .expect_err("command params without a public name must fail");

        // Assert
        assert_eq!(
            error,
            SchemaError::CommandModelMissingPublicName {
                command: "test.execute-command".to_owned(),
                role: "params",
                type_id: "pwright.test.params".to_owned(),
            }
        );
    }

    #[test]
    fn published_command_wire_mappings_are_exact_in_a_minor_release() {
        // Arrange
        let previous = parse_and_validate(COMMAND_CAPABILITY).expect("previous should parse");
        let next_source = COMMAND_CAPABILITY
            .replace("minor: 0", "minor: 1")
            .replace("method: 17", "method: 18");
        let next = parse_and_validate(&next_source).expect("next should parse");

        // Act
        let error = check_minor_compatibility(&previous, &next)
            .expect_err("published command mapping changes must fail");

        // Assert
        assert_eq!(
            error,
            CompatibilityError::CommandChanged {
                id: "test.execute-command".to_owned(),
            }
        );
    }

    #[test]
    fn library_schema_generates_models_and_catalog_without_a_root() {
        // Arrange
        let schema =
            parse_and_validate(REPRESENTATIVE_LIBRARY).expect("library schema should parse");

        // Act
        let generated = generate_rust(&schema).expect("library schema should generate");

        // Assert
        assert!(generated.contains("pub struct PwrightCommonDuration"));
        assert!(generated.contains("pub const PWRIGHT_COMMON_DURATION_SCHEMA"));
        assert!(generated.contains("pub const CATALOG: BinarySchemaCatalog"));
        assert!(!generated.contains("ROOT_SCHEMA"));
        assert!(!generated.contains("BinaryEncodeSequenceRef"));
        assert!(!generated.contains("BinarySchema,"));
    }

    #[test]
    fn capability_import_reuses_library_models_and_composes_its_catalog() {
        // Arrange
        let library =
            parse_and_validate(REPRESENTATIVE_LIBRARY).expect("library schema should parse");
        let schema =
            parse_and_validate_with_imports(IMPORTING_CAPABILITY, std::slice::from_ref(&library))
                .expect("declared import should resolve");

        // Act
        let generated =
            generate_rust_with_imports(&schema, &[library]).expect("bundle should generate");

        // Assert
        assert!(generated.contains(
            "pub use pwright_codec_common::{PwrightCommonDuration, PWRIGHT_COMMON_DURATION_SCHEMA};"
        ));
        assert!(!generated.contains("pub struct PwrightCommonDuration"));
        assert!(generated.contains("duration: PwrightCommonDuration"));
        assert!(generated.contains("BinaryTypeSchema::Record(PWRIGHT_COMMON_DURATION_SCHEMA)"));
        assert!(generated.contains("BinaryTypeSchema::TaggedUnion(PWRIGHT_TEST_MESSAGE_SCHEMA)"));
    }

    #[test]
    fn declared_import_must_be_provided() {
        // Arrange
        let imports = [];

        // Act
        let error = parse_and_validate_with_imports(IMPORTING_CAPABILITY, &imports)
            .expect_err("missing imported library must fail");

        // Assert
        assert_eq!(
            error,
            SchemaError::MissingImport {
                namespace: "pwright.common".to_owned(),
            }
        );
    }

    #[test]
    fn imported_capability_compatibility_includes_library_types() {
        // Arrange
        let previous_library =
            parse_and_validate(REPRESENTATIVE_LIBRARY).expect("previous library should parse");
        let next_library_source = REPRESENTATIVE_LIBRARY
            .replace("minor: 0", "minor: 1")
            .replace("name: seconds, type: u64", "name: seconds, type: u32");
        let next_library =
            parse_and_validate(&next_library_source).expect("next library should parse");
        let previous = parse_and_validate_with_imports(
            IMPORTING_CAPABILITY,
            std::slice::from_ref(&previous_library),
        )
        .expect("previous bundle should resolve");
        let next = parse_and_validate_with_imports(
            IMPORTING_CAPABILITY,
            std::slice::from_ref(&next_library),
        )
        .expect("next bundle should resolve");

        // Act
        let error = check_minor_compatibility_with_imports(
            &previous,
            &next,
            &[previous_library],
            &[next_library],
        )
        .expect_err("imported layout changes must fail");

        // Assert
        assert_eq!(
            error,
            CompatibilityError::PositionalLayoutChanged {
                id: "pwright.common.duration".to_owned(),
            }
        );
    }

    #[test]
    fn repeated_fields_generate_a_borrowed_sequence_boundary() {
        // Arrange
        let source = r#"
format: 1
capability: pwright.test
version: { major: 1, minor: 0 }
root: pwright.test.message
types:
  - kind: tagged_union
    id: pwright.test.message
    public_name: test_message
    variants:
      - tag: 1
        name: values
        fields:
          - { name: values, type: utf8, cardinality: repeated }
"#;
        let schema = parse_and_validate(source).expect("schema should parse");

        // Act
        let generated = generate_rust(&schema).expect("schema should generate");

        // Assert
        assert!(generated.contains("values: BinaryEncodeSequenceRef<'a>"));
        assert!(generated.contains(
            "pwright_codec_binary::encode_repeated_len(writer, context, \"pwright.test.message.values.values\", values.len())?;"
        ));
        assert!(generated.contains("values.encode_items(writer, context)?;"));
    }

    #[test]
    fn numeric_constraints_generate_catalog_and_bidirectional_value_checks() {
        // Arrange
        let source = REPRESENTATIVE.replace(
            "{ name: call_id, type: u64 }",
            "{ name: call_id, type: u64, constraints: { minimum: 1, maximum: 9 } }",
        );
        let schema = parse_and_validate(&source).expect("numeric range should validate");

        // Act
        let generated = generate_rust(&schema).expect("schema should generate");

        // Assert
        assert!(generated.contains(
            "check_numeric_range(\"pwright.test.message.command.call_id\", i128::from(*call_id), BinaryNumericRange::new(Some(1_i128), Some(9_i128)))?;"
        ));
        assert!(generated.contains(
            "check_numeric_range(\"pwright.test.message.command.call_id\", i128::from(value), BinaryNumericRange::new(Some(1_i128), Some(9_i128)))?;"
        ));
        assert!(
            generated.contains(
                ".with_numeric_range(BinaryNumericRange::new(Some(1_i128), Some(9_i128)))"
            )
        );
    }

    #[test]
    fn length_and_item_constraints_generate_preallocation_checks_and_catalog_metadata() {
        // Arrange
        let source = r#"
format: 1
capability: pwright.test
version: { major: 1, minor: 0 }
root: pwright.test.message
types:
  - kind: record
    id: pwright.test.child
    fields:
      - { name: payload, type: bytes, cardinality: optional, constraints: { maximum_length: 16 } }
      - { name: value, type: utf8, constraints: { minimum_length: 1, maximum_length: 8 } }
  - kind: tagged_union
    id: pwright.test.message
    variants:
      - tag: 1
        name: children
        fields:
          - { name: children, type: message, target: pwright.test.child, cardinality: repeated, constraints: { minimum_items: 1, maximum_items: 4 } }
"#;
        let schema = parse_and_validate(source).expect("size constraints should validate");

        // Act
        let generated = generate_rust(&schema).expect("schema should generate");

        // Assert
        assert!(generated.contains(
            "encode_utf8_constrained(writer, context, \"pwright.test.child.value\", &self.value, BinarySizeRange::new(Some(1_u32), Some(8_u32)))?;"
        ));
        assert!(generated.contains(
            "decode_utf8_constrained(reader, context, \"pwright.test.child.value\", BinarySizeRange::new(Some(1_u32), Some(8_u32)))?"
        ));
        assert!(generated.contains(
            "encode_bytes_constrained(writer, context, \"pwright.test.child.payload\", value, BinarySizeRange::new(None, Some(16_u32)))?;"
        ));
        assert!(generated.contains(
            "decode_bytes_constrained(reader, context, \"pwright.test.child.payload\", BinarySizeRange::new(None, Some(16_u32)))?"
        ));
        assert!(generated.contains(
            "check_size_range(\"pwright.test.message.children.children\", children.len(), BinarySizeRange::new(Some(1_u32), Some(4_u32)))?;"
        ));
        assert!(
            generated
                .contains(".with_length_range(BinarySizeRange::new(Some(1_u32), Some(8_u32)))")
        );
        assert!(
            generated
                .contains(".with_item_count_range(BinarySizeRange::new(Some(1_u32), Some(4_u32)))")
        );
    }

    #[test]
    fn size_constraints_are_restricted_to_their_wire_shapes() {
        // Arrange
        let length_on_integer = REPRESENTATIVE.replace(
            "{ name: call_id, type: u64 }",
            "{ name: call_id, type: u64, constraints: { maximum_length: 4 } }",
        );
        let items_on_scalar = REPRESENTATIVE.replace(
            "{ name: value, type: utf8 }",
            "{ name: value, type: utf8, constraints: { maximum_items: 4 } }",
        );

        // Act
        let length_error = parse_and_validate(&length_on_integer)
            .expect_err("integer length constraints must fail");
        let items_error =
            parse_and_validate(&items_on_scalar).expect_err("scalar item constraints must fail");

        // Assert
        assert_eq!(
            length_error,
            SchemaError::UnexpectedLengthConstraints {
                owner: "pwright.test.message.command".to_owned(),
                field: "call_id".to_owned(),
            }
        );
        assert_eq!(
            items_error,
            SchemaError::UnexpectedItemConstraints {
                owner: "pwright.test.child".to_owned(),
                field: "value".to_owned(),
            }
        );
    }

    #[test]
    fn reversed_size_constraints_are_rejected() {
        // Arrange
        let source = REPRESENTATIVE.replace(
            "{ name: value, type: utf8 }",
            "{ name: value, type: utf8, constraints: { minimum_length: 5, maximum_length: 4 } }",
        );

        // Act
        let error = parse_and_validate(&source).expect_err("reversed size range must fail");

        // Assert
        assert_eq!(
            error,
            SchemaError::InvalidLengthConstraints {
                owner: "pwright.test.child".to_owned(),
                field: "value".to_owned(),
                range: SizeConstraintRange::new(Some(5), Some(4)),
            }
        );
    }

    #[test]
    fn numeric_constraints_reject_non_integer_fields() {
        // Arrange
        let non_integer = REPRESENTATIVE.replace(
            "{ name: value, type: utf8 }",
            "{ name: value, type: utf8, constraints: { maximum: 4 } }",
        );

        // Act
        let non_integer_error =
            parse_and_validate(&non_integer).expect_err("UTF-8 range must fail");

        // Assert
        assert_eq!(
            non_integer_error,
            SchemaError::UnexpectedNumericConstraints {
                owner: "pwright.test.child".to_owned(),
                field: "value".to_owned(),
            }
        );
    }

    #[test]
    fn numeric_constraints_must_fit_the_declared_integer_type() {
        // Arrange
        let out_of_bounds = REPRESENTATIVE.replace(
            "{ name: call_id, type: u64 }",
            "{ name: call_id, type: u8, constraints: { maximum: 256 } }",
        );

        // Act
        let error = parse_and_validate(&out_of_bounds).expect_err("u8 range must fit u8");

        // Assert
        assert_eq!(
            error,
            SchemaError::InvalidNumericConstraints {
                owner: "pwright.test.message.command".to_owned(),
                field: "call_id".to_owned(),
                range: NumericConstraintRange::new(None, Some(256)),
            }
        );
    }

    #[test]
    fn numeric_constraints_preserve_the_full_unsigned_wire_domain() {
        // Arrange
        let source = REPRESENTATIVE.replace(
            "{ name: call_id, type: u64 }",
            "{ name: call_id, type: u64, constraints: { maximum: 18446744073709551615 } }",
        );

        // Act
        let generated = generate_rust(
            &parse_and_validate(&source).expect("u64 maximum should parse without narrowing"),
        )
        .expect("schema should generate");

        // Assert
        assert!(generated.contains("Some(18446744073709551615_i128)"));
    }

    #[test]
    fn float64_fields_use_network_order_and_remove_eq_transitively() {
        // Arrange
        let source = r#"
format: 1
capability: pwright.test
version: { major: 1, minor: 0 }
root: pwright.test.message
types:
  - kind: tagged_union
    id: pwright.test.message
    public_name: test_message
    variants:
      - tag: 1
        name: point
        fields:
          - { name: value, type: message, target: pwright.test.point }
  - kind: record
    id: pwright.test.point
    public_name: test_point
    fields:
      - { name: x, type: f64 }
      - { name: y, type: f64 }
"#;
        let schema = parse_and_validate(source).expect("f64 schema should validate");

        // Act
        let generated = generate_rust(&schema).expect("f64 wire model should generate");
        let capability =
            generate_capability_rust(&schema).expect("f64 public model should generate");

        // Assert
        assert!(
            generated.contains("#[derive(Clone, Debug, PartialEq)]\npub enum PwrightTestMessage")
        );
        assert!(
            generated.contains("#[derive(Clone, Debug, PartialEq)]\npub struct PwrightTestPoint")
        );
        assert!(generated.contains("writer.f64("));
        assert!(generated.contains("let x = reader.f64()?"));
        assert!(generated.contains("BinaryFieldKind::F64"));
        assert!(capability.contains("#[derive(Clone, Debug, PartialEq)]\npub enum TestMessage"));
        assert!(capability.contains("#[derive(Clone, Debug, PartialEq)]\npub struct TestPoint"));
        assert!(capability.contains("pub x: f64"));
    }

    #[test]
    fn changing_numeric_constraints_is_not_minor_compatible() {
        // Arrange
        let previous_source = REPRESENTATIVE.replace(
            "{ name: call_id, type: u64 }",
            "{ name: call_id, type: u64, constraints: { maximum: 9 } }",
        );
        let next_source = previous_source
            .replace("minor: 2", "minor: 3")
            .replace("maximum: 9", "maximum: 10");
        let previous = parse_and_validate(&previous_source).expect("previous should parse");
        let next = parse_and_validate(&next_source).expect("candidate should parse");

        // Act
        let error = check_minor_compatibility(&previous, &next)
            .expect_err("range changes must require a new major");

        // Assert
        assert_eq!(
            error,
            CompatibilityError::PositionalLayoutChanged {
                id: "pwright.test.message".to_owned(),
            }
        );
    }

    #[test]
    fn unknown_keys_fail_instead_of_being_ignored() {
        // Arrange
        let source = REPRESENTATIVE.replace("format: 1", "format: 1\nsurprise: true");

        // Act
        let error = parse_and_validate(&source).expect_err("unknown keys must fail");

        // Assert
        assert!(matches!(error, SchemaError::Yaml(_)));
    }

    #[test]
    fn unknown_message_targets_fail_before_generation() {
        // Arrange
        let source = REPRESENTATIVE.replace(
            "pwright.test.child, cardinality",
            "pwright.missing, cardinality",
        );

        // Act
        let error = parse_and_validate(&source).expect_err("unknown target must fail");

        // Assert
        assert_eq!(
            error,
            SchemaError::UnknownTarget {
                owner: "pwright.test.message.command".to_owned(),
                field: "child".to_owned(),
                target: "pwright.missing".to_owned(),
            }
        );
    }

    #[test]
    fn recursive_models_are_boxed_and_require_a_finite_wire_value() {
        // Arrange
        let source = r#"
format: 1
capability: pwright.test
version: { major: 1, minor: 0 }
root: pwright.test.message
types:
  - kind: tagged_union
    id: pwright.test.message
    public_name: test_message
    variants:
      - tag: 1
        name: child
        fields:
          - { name: next, type: message, target: pwright.test.message, cardinality: optional }
          - { name: label, type: utf8 }
"#;

        let schema = parse_and_validate(source).expect("optional recursion should be inhabited");
        let uninhabited = source.replace(", cardinality: optional", "");

        // Act
        let generated = generate_rust(&schema).expect("recursive model should generate");
        let capability =
            generate_capability_rust(&schema).expect("recursive public model should generate");
        let error = parse_and_validate(&uninhabited)
            .expect_err("required recursion without a terminal variant must fail");

        // Assert
        assert!(generated.contains("next: Option<Box<PwrightTestMessage>>"));
        assert!(generated.contains("next: Option<Box<PwrightTestMessageRef<'a>>>"));
        assert!(generated.contains(
            "let next = match reader.u8()? { 0 => None, 1 => Some(Box::new(<PwrightTestMessage as BinaryDecode>::decode_binary(reader, context)?))"
        ));
        assert!(generated.contains("next: next.map(|value| Box::new((*value).into()))"));
        assert!(capability.contains("next: Option<Box<TestMessage>>"));
        assert_eq!(
            error,
            SchemaError::UninhabitedRecursiveModel {
                type_id: "pwright.test.message".to_owned(),
            }
        );
    }

    #[test]
    fn source_compatibility_allows_new_types_and_appended_union_variants() {
        // Arrange
        let previous = parse_and_validate(REPRESENTATIVE).expect("previous should parse");
        let next_source = REPRESENTATIVE
            .replace("minor: 2", "minor: 3")
            .replace(
                "          - { name: child, type: message, target: pwright.test.child, cardinality: optional }",
                "          - { name: child, type: message, target: pwright.test.child, cardinality: optional }\n      - tag: 2\n        name: event\n        fields: []\n  - kind: record\n    id: pwright.test.new_type\n    fields: []",
            );
        let next = parse_and_validate(&next_source).expect("candidate should parse");

        // Act
        let compatibility = check_minor_compatibility(&previous, &next);

        // Assert
        assert_eq!(compatibility, Ok(()));
    }

    #[test]
    fn source_compatibility_rejects_changed_record_fields() {
        // Arrange
        let previous = parse_and_validate(REPRESENTATIVE).expect("previous should parse");
        let next_source = REPRESENTATIVE
            .replace("minor: 2", "minor: 3")
            .replace("name: value, type: utf8", "name: value, type: bytes");
        let next = parse_and_validate(&next_source).expect("candidate should parse");

        // Act
        let error = check_minor_compatibility(&previous, &next)
            .expect_err("record layout changes must fail");

        // Assert
        assert_eq!(
            error,
            CompatibilityError::PositionalLayoutChanged {
                id: "pwright.test.child".to_owned(),
            }
        );
    }

    #[test]
    fn source_compatibility_rejects_changed_published_union_variant() {
        // Arrange
        let previous = parse_and_validate(REPRESENTATIVE).expect("previous should parse");
        let next_source = REPRESENTATIVE
            .replace("minor: 2", "minor: 3")
            .replace("name: command", "name: renamed_command");
        let next = parse_and_validate(&next_source).expect("candidate should parse");

        // Act
        let error = check_minor_compatibility(&previous, &next)
            .expect_err("published union variants must remain exact");

        // Assert
        assert_eq!(
            error,
            CompatibilityError::PositionalLayoutChanged {
                id: "pwright.test.message".to_owned(),
            }
        );
    }
}
