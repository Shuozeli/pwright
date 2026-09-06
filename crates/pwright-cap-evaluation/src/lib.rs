//! Portable, value-only JavaScript evaluation contracts.
//!
//! This capability never creates a browser-side handle. `JSHandle` and
//! `ElementHandle` ownership belongs to a separate future capability.

use std::collections::BTreeSet;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU32;

#[doc(hidden)]
#[allow(unused_imports)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

pub const MAX_SAFE_EVALUATION_DEPTH: usize = 128;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationMode {
    Expression,
    Function,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluateParams {
    source: String,
    mode: EvaluationMode,
    argument: Option<EvaluationValue>,
}

impl EvaluateParams {
    pub fn expression(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            mode: EvaluationMode::Expression,
            argument: None,
        }
    }

    pub fn function(source: impl Into<String>, argument: EvaluationValue) -> Self {
        Self {
            source: source.into(),
            mode: EvaluationMode::Function,
            argument: Some(argument),
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

    pub const fn argument(&self) -> Option<&EvaluationValue> {
        self.argument.as_ref()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluateResult {
    pub value: EvaluationValue,
}

impl EvaluateResult {
    pub const fn new(value: EvaluationValue) -> Self {
        Self { value }
    }

    pub fn into_value(self) -> EvaluationValue {
        self.value
    }
}

/// A finite JavaScript number retained by its exact IEEE-754 representation.
///
/// Non-finite values and negative zero have explicit [`EvaluationValue`]
/// variants, avoiding ambiguous equality and wire encodings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvaluationNumber(u64);

impl EvaluationNumber {
    pub fn new(value: f64) -> Result<Self, EvaluationNumberError> {
        if !value.is_finite() {
            return Err(EvaluationNumberError::NonFinite);
        }
        if value == 0.0 && value.is_sign_negative() {
            return Err(EvaluationNumberError::NegativeZero);
        }
        Ok(Self(value.to_bits()))
    }

    pub fn from_bits(bits: u64) -> Result<Self, EvaluationNumberError> {
        Self::new(f64::from_bits(bits))
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationNumberError {
    NonFinite,
    NegativeZero,
}

impl Display for EvaluationNumberError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => {
                formatter.write_str("finite number variant cannot contain NaN or infinity")
            }
            Self::NegativeZero => {
                formatter.write_str("negative zero must use its explicit variant")
            }
        }
    }
}

impl Error for EvaluationNumberError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TypedArrayKind {
    Int8,
    Uint8,
    Uint8Clamped,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
    Bigint64,
    Biguint64,
}

impl TypedArrayKind {
    pub const fn element_bytes(self) -> usize {
        match self {
            Self::Int8 | Self::Uint8 | Self::Uint8Clamped => 1,
            Self::Int16 | Self::Uint16 => 2,
            Self::Int32 | Self::Uint32 | Self::Float32 => 4,
            Self::Float64 | Self::Bigint64 | Self::Biguint64 => 8,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvaluationProperty {
    pub key: String,
    pub value: EvaluationValue,
}

impl EvaluationProperty {
    pub fn new(key: impl Into<String>, value: EvaluationValue) -> Self {
        Self {
            key: key.into(),
            value,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvaluationValue {
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
        properties: Vec<EvaluationProperty>,
    },
    Ref(NonZeroU32),
}

impl EvaluationValue {
    pub fn number(value: f64) -> Result<Self, EvaluationNumberError> {
        EvaluationNumber::new(value).map(Self::Number)
    }

    pub fn validate(
        &self,
        limits: EvaluationValueLimits,
    ) -> Result<EvaluationValueMetrics, EvaluationValueError> {
        let mut state = ValidationState::default();
        validate_value(self, limits, 0, &mut state)?;
        Ok(EvaluationValueMetrics {
            nodes: state.nodes,
            entries: state.entries,
            string_bytes: state.string_bytes,
            binary_bytes: state.binary_bytes,
            reference_ids: state.reference_ids.len(),
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EvaluationValueLimits {
    max_depth: usize,
    max_nodes: usize,
    max_entries: usize,
    max_string_bytes: usize,
    max_binary_bytes: usize,
}

impl EvaluationValueLimits {
    pub fn new(
        max_depth: usize,
        max_nodes: usize,
        max_entries: usize,
        max_string_bytes: usize,
        max_binary_bytes: usize,
    ) -> Result<Self, EvaluationValueLimitError> {
        if max_depth > MAX_SAFE_EVALUATION_DEPTH {
            return Err(EvaluationValueLimitError::DepthTooLarge {
                actual: max_depth,
                maximum: MAX_SAFE_EVALUATION_DEPTH,
            });
        }
        if max_nodes == 0 {
            return Err(EvaluationValueLimitError::ZeroNodes);
        }
        Ok(Self {
            max_depth,
            max_nodes,
            max_entries,
            max_string_bytes,
            max_binary_bytes,
        })
    }

    pub const fn max_depth(self) -> usize {
        self.max_depth
    }

    pub const fn max_nodes(self) -> usize {
        self.max_nodes
    }

    pub const fn max_entries(self) -> usize {
        self.max_entries
    }

    pub const fn max_string_bytes(self) -> usize {
        self.max_string_bytes
    }

    pub const fn max_binary_bytes(self) -> usize {
        self.max_binary_bytes
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationValueLimitError {
    DepthTooLarge { actual: usize, maximum: usize },
    ZeroNodes,
}

impl Display for EvaluationValueLimitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DepthTooLarge { actual, maximum } => write!(
                formatter,
                "evaluation depth limit {actual} exceeds safe maximum {maximum}"
            ),
            Self::ZeroNodes => formatter.write_str("evaluation node limit must be nonzero"),
        }
    }
}

impl Error for EvaluationValueLimitError {}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EvaluationValueMetrics {
    pub nodes: usize,
    pub entries: usize,
    pub string_bytes: usize,
    pub binary_bytes: usize,
    pub reference_ids: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvaluationValueError {
    DepthExceeded {
        actual: usize,
        limit: usize,
    },
    NodesExceeded {
        actual: usize,
        limit: usize,
    },
    EntriesExceeded {
        actual: usize,
        limit: usize,
    },
    StringBytesExceeded {
        actual: usize,
        limit: usize,
    },
    BinaryBytesExceeded {
        actual: usize,
        limit: usize,
    },
    DuplicateReferenceId(NonZeroU32),
    UnknownReferenceId(NonZeroU32),
    DuplicateObjectKey(String),
    NonCanonicalBigint(String),
    MisalignedTypedArray {
        kind: TypedArrayKind,
        bytes: usize,
        element_bytes: usize,
    },
}

impl Display for EvaluationValueError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::DepthExceeded { actual, limit } => {
                write!(formatter, "evaluation depth {actual} exceeds limit {limit}")
            }
            Self::NodesExceeded { actual, limit } => {
                write!(
                    formatter,
                    "evaluation node count {actual} exceeds limit {limit}"
                )
            }
            Self::EntriesExceeded { actual, limit } => {
                write!(
                    formatter,
                    "evaluation entry count {actual} exceeds limit {limit}"
                )
            }
            Self::StringBytesExceeded { actual, limit } => write!(
                formatter,
                "evaluation string bytes {actual} exceeds limit {limit}"
            ),
            Self::BinaryBytesExceeded { actual, limit } => write!(
                formatter,
                "evaluation binary bytes {actual} exceeds limit {limit}"
            ),
            Self::DuplicateReferenceId(id) => {
                write!(formatter, "evaluation reference ID {id} is declared twice")
            }
            Self::UnknownReferenceId(id) => {
                write!(formatter, "evaluation reference ID {id} was not declared")
            }
            Self::DuplicateObjectKey(key) => {
                write!(
                    formatter,
                    "evaluation object contains duplicate key {key:?}"
                )
            }
            Self::NonCanonicalBigint(value) => {
                write!(formatter, "evaluation BigInt {value:?} is not canonical")
            }
            Self::MisalignedTypedArray {
                kind,
                bytes,
                element_bytes,
            } => write!(
                formatter,
                "{kind:?} payload has {bytes} bytes, not a multiple of {element_bytes}"
            ),
        }
    }
}

impl Error for EvaluationValueError {}

#[derive(Default)]
struct ValidationState {
    nodes: usize,
    entries: usize,
    string_bytes: usize,
    binary_bytes: usize,
    reference_ids: BTreeSet<NonZeroU32>,
}

fn validate_value(
    value: &EvaluationValue,
    limits: EvaluationValueLimits,
    depth: usize,
    state: &mut ValidationState,
) -> Result<(), EvaluationValueError> {
    if depth > limits.max_depth {
        return Err(EvaluationValueError::DepthExceeded {
            actual: depth,
            limit: limits.max_depth,
        });
    }
    state.nodes = checked_total(state.nodes, 1, limits.max_nodes).map_err(|actual| {
        EvaluationValueError::NodesExceeded {
            actual,
            limit: limits.max_nodes,
        }
    })?;

    match value {
        EvaluationValue::Undefined
        | EvaluationValue::Null
        | EvaluationValue::Bool(_)
        | EvaluationValue::Number(_)
        | EvaluationValue::Nan
        | EvaluationValue::Infinity
        | EvaluationValue::NegativeInfinity
        | EvaluationValue::NegativeZero => {}
        EvaluationValue::String(value)
        | EvaluationValue::Date(value)
        | EvaluationValue::Url(value) => {
            claim_string(value, limits, state)?;
        }
        EvaluationValue::Bigint(value) => {
            claim_string(value, limits, state)?;
            if !is_canonical_bigint(value) {
                return Err(EvaluationValueError::NonCanonicalBigint(value.clone()));
            }
        }
        EvaluationValue::Regexp { pattern, flags } => {
            claim_string(pattern, limits, state)?;
            claim_string(flags, limits, state)?;
        }
        EvaluationValue::Error {
            name,
            message,
            stack,
        } => {
            claim_string(name, limits, state)?;
            claim_string(message, limits, state)?;
            if let Some(stack) = stack {
                claim_string(stack, limits, state)?;
            }
        }
        EvaluationValue::TypedArray { kind, bytes } => {
            let element_bytes = kind.element_bytes();
            if bytes.len() % element_bytes != 0 {
                return Err(EvaluationValueError::MisalignedTypedArray {
                    kind: *kind,
                    bytes: bytes.len(),
                    element_bytes,
                });
            }
            state.binary_bytes =
                checked_total(state.binary_bytes, bytes.len(), limits.max_binary_bytes).map_err(
                    |actual| EvaluationValueError::BinaryBytesExceeded {
                        actual,
                        limit: limits.max_binary_bytes,
                    },
                )?;
        }
        EvaluationValue::Array { id, items } => {
            declare_reference(*id, state)?;
            claim_entries(items.len(), limits, state)?;
            for item in items {
                validate_value(item, limits, depth.saturating_add(1), state)?;
            }
        }
        EvaluationValue::Object { id, properties } => {
            declare_reference(*id, state)?;
            claim_entries(properties.len(), limits, state)?;
            let mut keys = BTreeSet::new();
            for property in properties {
                if !keys.insert(property.key.as_str()) {
                    return Err(EvaluationValueError::DuplicateObjectKey(
                        property.key.clone(),
                    ));
                }
                claim_string(&property.key, limits, state)?;
                validate_value(&property.value, limits, depth.saturating_add(1), state)?;
            }
        }
        EvaluationValue::Ref(id) => {
            if !state.reference_ids.contains(id) {
                return Err(EvaluationValueError::UnknownReferenceId(*id));
            }
        }
    }
    Ok(())
}

fn declare_reference(
    id: Option<NonZeroU32>,
    state: &mut ValidationState,
) -> Result<(), EvaluationValueError> {
    if let Some(id) = id
        && !state.reference_ids.insert(id)
    {
        return Err(EvaluationValueError::DuplicateReferenceId(id));
    }
    Ok(())
}

fn claim_entries(
    additional: usize,
    limits: EvaluationValueLimits,
    state: &mut ValidationState,
) -> Result<(), EvaluationValueError> {
    state.entries =
        checked_total(state.entries, additional, limits.max_entries).map_err(|actual| {
            EvaluationValueError::EntriesExceeded {
                actual,
                limit: limits.max_entries,
            }
        })?;
    Ok(())
}

fn claim_string(
    value: &str,
    limits: EvaluationValueLimits,
    state: &mut ValidationState,
) -> Result<(), EvaluationValueError> {
    state.string_bytes = checked_total(state.string_bytes, value.len(), limits.max_string_bytes)
        .map_err(|actual| EvaluationValueError::StringBytesExceeded {
            actual,
            limit: limits.max_string_bytes,
        })?;
    Ok(())
}

fn checked_total(current: usize, additional: usize, limit: usize) -> Result<usize, usize> {
    let Some(total) = current.checked_add(additional) else {
        return Err(usize::MAX);
    };
    if total > limit { Err(total) } else { Ok(total) }
}

fn is_canonical_bigint(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    if digits.len() > 1 && digits.starts_with('0') {
        return false;
    }
    value != "-0"
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use pwright_core::{CommandSpec, RegistryBuilder};

    use super::{
        EVALUATE_VALUE_COMMAND_ID, EVALUATE_VALUE_METHOD, EvaluateParams, EvaluateValue,
        EvaluationCapability, EvaluationNumberError, EvaluationValue, EvaluationValueError,
        EvaluationValueLimits, TypedArrayKind,
    };

    fn limits() -> EvaluationValueLimits {
        EvaluationValueLimits::new(8, 32, 32, 256, 256).expect("limits should be valid")
    }

    #[test]
    fn capability_owns_one_frame_targeted_value_command() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<EvaluationCapability>()
            .expect("evaluation capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert_eq!(EvaluateValue::METHOD, EVALUATE_VALUE_METHOD);
        assert_eq!(
            EVALUATE_VALUE_COMMAND_ID.as_str(),
            "evaluation.evaluate_value"
        );
        assert_eq!(
            registry.owner_of(EVALUATE_VALUE_COMMAND_ID),
            Some(super::EVALUATION_CAPABILITY_ID)
        );
        assert_eq!(
            EvaluateValue::TARGET_OBJECT_KINDS[0].as_str(),
            "pwright.browser.frame"
        );
    }

    #[test]
    fn finite_number_and_special_number_variants_are_unambiguous() {
        // Arrange
        let ordinary = EvaluationValue::number(1.5);

        // Act
        let nan = EvaluationValue::number(f64::NAN);
        let negative_zero = EvaluationValue::number(-0.0);

        // Assert
        assert_eq!(ordinary.unwrap().validate(limits()).unwrap().nodes, 1);
        assert_eq!(nan, Err(EvaluationNumberError::NonFinite));
        assert_eq!(negative_zero, Err(EvaluationNumberError::NegativeZero));
    }

    #[test]
    fn a_container_may_refer_to_its_already_declared_identity() {
        // Arrange
        let id = NonZeroU32::new(1).unwrap();
        let value = EvaluationValue::Array {
            id: Some(id),
            items: vec![EvaluationValue::Ref(id)],
        };

        // Act
        let metrics = value.validate(limits()).expect("cycle should validate");

        // Assert
        assert_eq!(metrics.nodes, 2);
        assert_eq!(metrics.entries, 1);
        assert_eq!(metrics.reference_ids, 1);
    }

    #[test]
    fn forward_reference_and_limit_overflow_are_rejected() {
        // Arrange
        let id = NonZeroU32::new(7).unwrap();
        let forward = EvaluationValue::Array {
            id: None,
            items: vec![EvaluationValue::Ref(id)],
        };
        let long = EvaluationValue::String("too long".to_string());
        let small = EvaluationValueLimits::new(2, 4, 4, 3, 4).unwrap();

        // Act
        let forward_error = forward.validate(limits()).unwrap_err();
        let size_error = long.validate(small).unwrap_err();

        // Assert
        assert_eq!(forward_error, EvaluationValueError::UnknownReferenceId(id));
        assert!(matches!(
            size_error,
            EvaluationValueError::StringBytesExceeded {
                actual: 8,
                limit: 3
            }
        ));
    }

    #[test]
    fn typed_array_payload_must_match_its_element_width() {
        // Arrange
        let value = EvaluationValue::TypedArray {
            kind: TypedArrayKind::Float64,
            bytes: vec![0; 7],
        };

        // Act
        let error = value.validate(limits()).unwrap_err();

        // Assert
        assert!(matches!(
            error,
            EvaluationValueError::MisalignedTypedArray {
                kind: TypedArrayKind::Float64,
                bytes: 7,
                element_bytes: 8,
            }
        ));
    }

    #[test]
    fn expression_and_function_parameters_keep_argument_semantics_explicit() {
        // Arrange
        let expression = EvaluateParams::expression("document.title");
        let function = EvaluateParams::function("value => value", EvaluationValue::Null);

        // Act / Assert
        assert_eq!(expression.mode(), super::EvaluationMode::Expression);
        assert!(expression.argument().is_none());
        assert_eq!(function.mode(), super::EvaluationMode::Function);
        assert_eq!(function.argument(), Some(&EvaluationValue::Null));
    }
}
