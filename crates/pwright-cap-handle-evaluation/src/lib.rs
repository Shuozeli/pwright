//! Portable opt-in contract for evaluating JavaScript into an owned handle.
//!
//! Handle evaluation is separate from value evaluation: installing the latter
//! never retains browser-side objects. Handle arguments are portable object
//! IDs in this contract; backend-native references never cross the wire.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU32;

use pwright_core::{ObjectId, ObjectKindTag};

#[doc(hidden)]
#[allow(unused_imports)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;
pub use pwright_cap_evaluation::{
    EvaluateParams, EvaluationMode, EvaluationNumber, EvaluationProperty, EvaluationValue,
    EvaluationValueError, EvaluationValueLimits, EvaluationValueMetrics, TypedArrayKind,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluateHandleParams {
    source: String,
    mode: EvaluationMode,
    argument: Option<HandleEvaluationValue>,
}

impl EvaluateHandleParams {
    pub fn expression(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            mode: EvaluationMode::Expression,
            argument: None,
        }
    }

    pub fn function(source: impl Into<String>, argument: impl Into<HandleEvaluationValue>) -> Self {
        Self {
            source: source.into(),
            mode: EvaluationMode::Function,
            argument: Some(argument.into()),
        }
    }

    pub fn function_without_argument(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            mode: EvaluationMode::Function,
            argument: None,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub const fn mode(&self) -> EvaluationMode {
        self.mode
    }

    pub const fn argument(&self) -> Option<&HandleEvaluationValue> {
        self.argument.as_ref()
    }

    pub fn referenced_handles(
        &self,
        limits: EvaluationValueLimits,
    ) -> Result<Vec<ObjectId>, HandleEvaluationValueError> {
        let Some(argument) = &self.argument else {
            return Ok(Vec::new());
        };
        argument.validate(limits)?;
        let mut seen = BTreeSet::new();
        let mut handles = Vec::new();
        collect_handles(argument, &mut seen, &mut handles);
        Ok(handles)
    }

    #[doc(hidden)]
    pub fn value_projection(
        &self,
        limits: EvaluationValueLimits,
    ) -> Result<EvaluateParams, HandleEvaluationValueError> {
        let argument = self
            .argument
            .as_ref()
            .map(|argument| validation_projection(argument, limits, 0))
            .transpose()?;
        match (self.mode, argument) {
            (EvaluationMode::Expression, None) => {
                Ok(EvaluateParams::expression(self.source.clone()))
            }
            (EvaluationMode::Expression, Some(_)) => {
                unreachable!("expression arguments are rejected by protocol validation")
            }
            (EvaluationMode::Function, Some(argument)) => {
                Ok(EvaluateParams::function(self.source.clone(), argument))
            }
            (EvaluationMode::Function, None) => Ok(EvaluateParams::function_without_argument(
                self.source.clone(),
            )),
        }
    }
}

impl From<EvaluateParams> for EvaluateHandleParams {
    fn from(params: EvaluateParams) -> Self {
        let argument = params.argument().cloned().map(HandleEvaluationValue::from);
        Self {
            source: params.source().to_owned(),
            mode: params.mode(),
            argument,
        }
    }
}

/// Function-only evaluation parameters for a Handle-local operation.
///
/// The target Handle is supplied as the first browser-side argument; this
/// optional value is supplied second.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandleEvaluateParams {
    source: String,
    argument: Option<HandleEvaluationValue>,
}

impl HandleEvaluateParams {
    pub fn function(source: impl Into<String>, argument: impl Into<HandleEvaluationValue>) -> Self {
        Self {
            source: source.into(),
            argument: Some(argument.into()),
        }
    }

    pub fn function_without_argument(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            argument: None,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub const fn argument(&self) -> Option<&HandleEvaluationValue> {
        self.argument.as_ref()
    }

    pub fn referenced_handles(
        &self,
        limits: EvaluationValueLimits,
    ) -> Result<Vec<ObjectId>, HandleEvaluationValueError> {
        self.as_frame_params().referenced_handles(limits)
    }

    #[doc(hidden)]
    pub fn value_projection(
        &self,
        limits: EvaluationValueLimits,
    ) -> Result<EvaluateParams, HandleEvaluationValueError> {
        self.as_frame_params().value_projection(limits)
    }

    #[doc(hidden)]
    pub fn as_frame_params(&self) -> EvaluateHandleParams {
        EvaluateHandleParams {
            source: self.source.clone(),
            mode: EvaluationMode::Function,
            argument: self.argument.clone(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct JsonValueOptions;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GetPropertyParams {
    name: String,
}

impl GetPropertyParams {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GetPropertiesOptions;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandleEvaluationProperty {
    pub key: String,
    pub value: HandleEvaluationValue,
}

impl HandleEvaluationProperty {
    pub fn new(key: impl Into<String>, value: impl Into<HandleEvaluationValue>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

/// A bounded evaluation graph whose leaves may reference portable Handles.
///
/// `Ref` is only for cycles within this value graph. `Handle` names a live
/// Pwright object and is resolved to a backend-native argument by the server.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandleEvaluationValue {
    Undefined,
    Null,
    Bool(bool),
    Number(EvaluationNumber),
    Nan,
    Infinity,
    NegativeInfinity,
    NegativeZero,
    String(String),
    Bigint(String),
    Date(String),
    Url(String),
    Regexp {
        pattern: String,
        flags: String,
    },
    Error {
        name: String,
        message: String,
        stack: Option<String>,
    },
    TypedArray {
        kind: TypedArrayKind,
        bytes: Vec<u8>,
    },
    Array {
        id: Option<NonZeroU32>,
        items: Vec<Self>,
    },
    Object {
        id: Option<NonZeroU32>,
        properties: Vec<HandleEvaluationProperty>,
    },
    Ref(NonZeroU32),
    Handle(ObjectId),
}

impl HandleEvaluationValue {
    pub fn number(value: f64) -> Result<Self, pwright_cap_evaluation::EvaluationNumberError> {
        EvaluationNumber::new(value).map(Self::Number)
    }

    pub fn validate(
        &self,
        limits: EvaluationValueLimits,
    ) -> Result<EvaluationValueMetrics, HandleEvaluationValueError> {
        validation_projection(self, limits, 0)?
            .validate(limits)
            .map_err(HandleEvaluationValueError::Value)
    }
}

impl From<EvaluationValue> for HandleEvaluationValue {
    fn from(value: EvaluationValue) -> Self {
        match value {
            EvaluationValue::Undefined => Self::Undefined,
            EvaluationValue::Null => Self::Null,
            EvaluationValue::Bool(value) => Self::Bool(value),
            EvaluationValue::Number(value) => Self::Number(value),
            EvaluationValue::Nan => Self::Nan,
            EvaluationValue::Infinity => Self::Infinity,
            EvaluationValue::NegativeInfinity => Self::NegativeInfinity,
            EvaluationValue::NegativeZero => Self::NegativeZero,
            EvaluationValue::String(value) => Self::String(value),
            EvaluationValue::Bigint(value) => Self::Bigint(value),
            EvaluationValue::Date(value) => Self::Date(value),
            EvaluationValue::Url(value) => Self::Url(value),
            EvaluationValue::Regexp { pattern, flags } => Self::Regexp { pattern, flags },
            EvaluationValue::Error {
                name,
                message,
                stack,
            } => Self::Error {
                name,
                message,
                stack,
            },
            EvaluationValue::TypedArray { kind, bytes } => Self::TypedArray { kind, bytes },
            EvaluationValue::Array { id, items } => Self::Array {
                id,
                items: items.into_iter().map(Self::from).collect(),
            },
            EvaluationValue::Object { id, properties } => Self::Object {
                id,
                properties: properties
                    .into_iter()
                    .map(|property| HandleEvaluationProperty {
                        key: property.key,
                        value: Self::from(property.value),
                    })
                    .collect(),
            },
            EvaluationValue::Ref(id) => Self::Ref(id),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HandleEvaluationValueError {
    Value(EvaluationValueError),
    ZeroHandleObjectId,
    HandleLeafInResult,
}

impl Display for HandleEvaluationValueError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Value(error) => Display::fmt(error, formatter),
            Self::ZeroHandleObjectId => {
                formatter.write_str("Handle evaluation object ID must be nonzero")
            }
            Self::HandleLeafInResult => {
                formatter.write_str("a serialized evaluation result cannot contain a Handle leaf")
            }
        }
    }
}

impl Error for HandleEvaluationValueError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Value(error) => Some(error),
            Self::ZeroHandleObjectId | Self::HandleLeafInResult => None,
        }
    }
}

impl TryFrom<HandleEvaluationValue> for EvaluationValue {
    type Error = HandleEvaluationValueError;

    fn try_from(value: HandleEvaluationValue) -> Result<Self, HandleEvaluationValueError> {
        Ok(match value {
            HandleEvaluationValue::Undefined => Self::Undefined,
            HandleEvaluationValue::Null => Self::Null,
            HandleEvaluationValue::Bool(value) => Self::Bool(value),
            HandleEvaluationValue::Number(value) => Self::Number(value),
            HandleEvaluationValue::Nan => Self::Nan,
            HandleEvaluationValue::Infinity => Self::Infinity,
            HandleEvaluationValue::NegativeInfinity => Self::NegativeInfinity,
            HandleEvaluationValue::NegativeZero => Self::NegativeZero,
            HandleEvaluationValue::String(value) => Self::String(value),
            HandleEvaluationValue::Bigint(value) => Self::Bigint(value),
            HandleEvaluationValue::Date(value) => Self::Date(value),
            HandleEvaluationValue::Url(value) => Self::Url(value),
            HandleEvaluationValue::Regexp { pattern, flags } => Self::Regexp { pattern, flags },
            HandleEvaluationValue::Error {
                name,
                message,
                stack,
            } => Self::Error {
                name,
                message,
                stack,
            },
            HandleEvaluationValue::TypedArray { kind, bytes } => Self::TypedArray { kind, bytes },
            HandleEvaluationValue::Array { id, items } => Self::Array {
                id,
                items: items
                    .into_iter()
                    .map(Self::try_from)
                    .collect::<Result<_, _>>()?,
            },
            HandleEvaluationValue::Object { id, properties } => Self::Object {
                id,
                properties: properties
                    .into_iter()
                    .map(|property| {
                        Ok(EvaluationProperty::new(
                            property.key,
                            Self::try_from(property.value)?,
                        ))
                    })
                    .collect::<Result<_, HandleEvaluationValueError>>()?,
            },
            HandleEvaluationValue::Ref(id) => Self::Ref(id),
            HandleEvaluationValue::Handle(_) => {
                return Err(HandleEvaluationValueError::HandleLeafInResult);
            }
        })
    }
}

fn validation_projection(
    value: &HandleEvaluationValue,
    limits: EvaluationValueLimits,
    depth: usize,
) -> Result<EvaluationValue, HandleEvaluationValueError> {
    if depth > limits.max_depth() {
        return Err(HandleEvaluationValueError::Value(
            EvaluationValueError::DepthExceeded {
                actual: depth,
                limit: limits.max_depth(),
            },
        ));
    }
    let next_depth = depth.saturating_add(1);
    Ok(match value {
        HandleEvaluationValue::Undefined => EvaluationValue::Undefined,
        HandleEvaluationValue::Null => EvaluationValue::Null,
        HandleEvaluationValue::Bool(value) => EvaluationValue::Bool(*value),
        HandleEvaluationValue::Number(value) => EvaluationValue::Number(*value),
        HandleEvaluationValue::Nan => EvaluationValue::Nan,
        HandleEvaluationValue::Infinity => EvaluationValue::Infinity,
        HandleEvaluationValue::NegativeInfinity => EvaluationValue::NegativeInfinity,
        HandleEvaluationValue::NegativeZero => EvaluationValue::NegativeZero,
        HandleEvaluationValue::String(value) => EvaluationValue::String(value.clone()),
        HandleEvaluationValue::Bigint(value) => EvaluationValue::Bigint(value.clone()),
        HandleEvaluationValue::Date(value) => EvaluationValue::Date(value.clone()),
        HandleEvaluationValue::Url(value) => EvaluationValue::Url(value.clone()),
        HandleEvaluationValue::Regexp { pattern, flags } => EvaluationValue::Regexp {
            pattern: pattern.clone(),
            flags: flags.clone(),
        },
        HandleEvaluationValue::Error {
            name,
            message,
            stack,
        } => EvaluationValue::Error {
            name: name.clone(),
            message: message.clone(),
            stack: stack.clone(),
        },
        HandleEvaluationValue::TypedArray { kind, bytes } => EvaluationValue::TypedArray {
            kind: *kind,
            bytes: bytes.clone(),
        },
        HandleEvaluationValue::Array { id, items } => EvaluationValue::Array {
            id: *id,
            items: items
                .iter()
                .map(|item| validation_projection(item, limits, next_depth))
                .collect::<Result<_, _>>()?,
        },
        HandleEvaluationValue::Object { id, properties } => EvaluationValue::Object {
            id: *id,
            properties: properties
                .iter()
                .map(|property| {
                    Ok(EvaluationProperty::new(
                        property.key.clone(),
                        validation_projection(&property.value, limits, next_depth)?,
                    ))
                })
                .collect::<Result<_, HandleEvaluationValueError>>()?,
        },
        HandleEvaluationValue::Ref(id) => EvaluationValue::Ref(*id),
        HandleEvaluationValue::Handle(object_id) => {
            if object_id.get() == 0 {
                return Err(HandleEvaluationValueError::ZeroHandleObjectId);
            }
            EvaluationValue::Null
        }
    })
}

fn collect_handles(
    value: &HandleEvaluationValue,
    seen: &mut BTreeSet<ObjectId>,
    handles: &mut Vec<ObjectId>,
) {
    match value {
        HandleEvaluationValue::Array { items, .. } => {
            for item in items {
                collect_handles(item, seen, handles);
            }
        }
        HandleEvaluationValue::Object { properties, .. } => {
            for property in properties {
                collect_handles(&property.value, seen, handles);
            }
        }
        HandleEvaluationValue::Handle(object_id) if seen.insert(*object_id) => {
            handles.push(*object_id);
        }
        _ => {}
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandleInitializer {
    pub object_id: ObjectId,
    pub parent_id: ObjectId,
    pub object_kind: ObjectKindTag,
    pub preview: String,
}

impl HandleInitializer {
    pub fn new(
        object_id: ObjectId,
        parent_id: ObjectId,
        object_kind: ObjectKindTag,
        preview: impl Into<String>,
    ) -> Self {
        Self {
            object_id,
            parent_id,
            object_kind,
            preview: preview.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandleValueResult {
    pub value: EvaluationValue,
}

impl HandleValueResult {
    pub const fn new(value: EvaluationValue) -> Self {
        Self { value }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HandlePropertyInitializer {
    pub name: String,
    pub handle: HandleInitializer,
}

impl HandlePropertyInitializer {
    pub fn new(name: impl Into<String>, handle: HandleInitializer) -> Self {
        Self {
            name: name.into(),
            handle,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HandlePropertiesResult {
    pub properties: Vec<HandlePropertyInitializer>,
}

impl HandlePropertiesResult {
    pub const fn new(properties: Vec<HandlePropertyInitializer>) -> Self {
        Self { properties }
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::{Capability, CommandSpec, ProtocolVersion};

    use super::{
        EVALUATE_HANDLE_METHOD, EVALUATE_HANDLE_TARGET_OBJECT_KINDS, EvaluateHandle,
        EvaluateHandleParams, HANDLE_EVALUATE_METHOD, HANDLE_EVALUATE_TARGET_OBJECT_KINDS,
        HANDLE_EVALUATION_CAPABILITY_ID, HandleEvaluate, HandleEvaluateParams,
        HandleEvaluationCapability, HandleEvaluationValue,
    };

    #[test]
    fn handle_evaluation_is_opt_in_and_depends_on_explicit_disposal() {
        // Arrange
        let descriptor = HandleEvaluationCapability::DESCRIPTOR;

        // Act
        let dependency = descriptor.dependencies.first().copied();

        // Assert
        assert_eq!(
            HANDLE_EVALUATION_CAPABILITY_ID.as_str(),
            "pwright.handle_evaluation"
        );
        assert_eq!(EvaluateHandle::METHOD, EVALUATE_HANDLE_METHOD);
        assert_eq!(HandleEvaluate::METHOD, HANDLE_EVALUATE_METHOD);
        assert_eq!(descriptor.plugin.version, ProtocolVersion::new(1, 2));
        assert_eq!(descriptor.commands.len(), 6);
        assert_eq!(EVALUATE_HANDLE_TARGET_OBJECT_KINDS.len(), 1);
        assert_eq!(
            EVALUATE_HANDLE_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.frame"
        );
        assert_eq!(HANDLE_EVALUATE_TARGET_OBJECT_KINDS.len(), 2);
        assert_eq!(
            HANDLE_EVALUATE_TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.element_handle"
        );
        assert_eq!(
            HANDLE_EVALUATE_TARGET_OBJECT_KINDS[1].as_str(),
            "pwright.browser.js_handle"
        );
        assert_eq!(
            dependency.map(|value| value.id.as_str()),
            Some("pwright.handle_lifecycle")
        );
        assert_eq!(
            dependency.map(|value| value.minimum_version),
            Some(ProtocolVersion::new(1, 0))
        );
    }

    #[test]
    fn handle_local_arguments_are_function_only_and_collect_handle_leaves() {
        // Arrange
        let limits = super::EvaluationValueLimits::new(4, 8, 8, 64, 0).unwrap();
        let params = HandleEvaluateParams::function(
            "(target, value) => [target, value]",
            HandleEvaluationValue::Handle(pwright_core::ObjectId::new(9)),
        );

        // Act
        let handles = params.referenced_handles(limits).unwrap();
        let projected = params.value_projection(limits).unwrap();

        // Assert
        assert_eq!(handles, [pwright_core::ObjectId::new(9)]);
        assert_eq!(projected.mode(), super::EvaluationMode::Function);
        assert_eq!(projected.source(), "(target, value) => [target, value]");
    }

    #[test]
    fn handle_references_are_deduplicated_in_first_appearance_order() {
        // Arrange
        let limits = super::EvaluationValueLimits::new(4, 8, 8, 64, 0).unwrap();
        let params = EvaluateHandleParams::function(
            "value => value",
            HandleEvaluationValue::Array {
                id: None,
                items: vec![
                    HandleEvaluationValue::Handle(pwright_core::ObjectId::new(7)),
                    HandleEvaluationValue::Handle(pwright_core::ObjectId::new(9)),
                    HandleEvaluationValue::Handle(pwright_core::ObjectId::new(7)),
                ],
            },
        );

        // Act
        let handles = params.referenced_handles(limits).unwrap();

        // Assert
        assert_eq!(
            handles,
            [
                pwright_core::ObjectId::new(7),
                pwright_core::ObjectId::new(9)
            ]
        );
    }

    #[test]
    fn handle_leaf_obeys_value_graph_node_limit() {
        // Arrange
        let limits = super::EvaluationValueLimits::new(4, 1, 8, 64, 0).unwrap();
        let value = HandleEvaluationValue::Array {
            id: None,
            items: vec![HandleEvaluationValue::Handle(pwright_core::ObjectId::new(
                7,
            ))],
        };

        // Act
        let error = value.validate(limits).unwrap_err();

        // Assert
        assert!(matches!(
            error,
            super::HandleEvaluationValueError::Value(super::EvaluationValueError::NodesExceeded {
                actual: 2,
                limit: 1
            })
        ));
    }

    #[test]
    fn serialized_result_conversion_rejects_nested_handle_leaf() {
        // Arrange
        let value = HandleEvaluationValue::Array {
            id: None,
            items: vec![HandleEvaluationValue::Handle(pwright_core::ObjectId::new(
                7,
            ))],
        };

        // Act
        let error = super::EvaluationValue::try_from(value).unwrap_err();

        // Assert
        assert_eq!(error, super::HandleEvaluationValueError::HandleLeafInResult);
    }
}
