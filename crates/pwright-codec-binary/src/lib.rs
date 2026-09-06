//! Canonical, allocation-aware binary primitives and schema metadata.
//!
//! Pwright capability payload codecs stay independent crates, but they share
//! these exact integer, length, truncation, and positional-layout rules. This
//! crate uses no Serde, executor, socket, browser protocol, or code generator.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::str::Utf8Error;

use pwright_core::{CommandId, MethodTag, ProtocolVersion};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntegerWidth {
    U32,
    U64,
}

impl Display for IntegerWidth {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::U32 => "u32",
            Self::U64 => "u64",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinaryEncodeError {
    IntegerOutOfRange {
        field: &'static str,
        value: u128,
        target: IntegerWidth,
    },
    AllocationFailed {
        current: usize,
        additional: usize,
    },
}

impl Display for BinaryEncodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntegerOutOfRange {
                field,
                value,
                target,
            } => write!(
                formatter,
                "value {value} for `{field}` does not fit canonical {target} encoding"
            ),
            Self::AllocationFailed {
                current,
                additional,
            } => write!(
                formatter,
                "could not extend binary payload of {current} bytes by {additional} bytes"
            ),
        }
    }
}

impl Error for BinaryEncodeError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinaryDecodeError {
    IntegerOutOfRange {
        field: &'static str,
        value: u64,
    },
    Truncated {
        offset: usize,
        needed: usize,
        remaining: usize,
    },
    TrailingBytes {
        remaining: usize,
    },
}

impl Display for BinaryDecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntegerOutOfRange { field, value } => {
                write!(
                    formatter,
                    "wire value {value} does not fit host field `{field}`"
                )
            }
            Self::Truncated {
                offset,
                needed,
                remaining,
            } => write!(
                formatter,
                "binary payload at byte {offset} needs {needed} bytes but only {remaining} remain"
            ),
            Self::TrailingBytes { remaining } => {
                write!(formatter, "binary payload has {remaining} trailing bytes")
            }
        }
    }
}

impl Error for BinaryDecodeError {}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BinaryWriter {
    bytes: Vec<u8>,
}

impl BinaryWriter {
    pub const fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    pub fn try_with_capacity(capacity: usize) -> Result<Self, BinaryEncodeError> {
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(capacity)
            .map_err(|_| BinaryEncodeError::AllocationFailed {
                current: 0,
                additional: capacity,
            })?;
        Ok(Self { bytes })
    }

    pub fn u8(&mut self, value: u8) -> Result<(), BinaryEncodeError> {
        self.reserve(1)?;
        self.bytes.push(value);
        Ok(())
    }

    pub fn u16(&mut self, value: u16) -> Result<(), BinaryEncodeError> {
        self.raw(&value.to_be_bytes())
    }

    pub fn u32(&mut self, value: u32) -> Result<(), BinaryEncodeError> {
        self.raw(&value.to_be_bytes())
    }

    pub fn i32(&mut self, value: i32) -> Result<(), BinaryEncodeError> {
        self.raw(&value.to_be_bytes())
    }

    pub fn u64(&mut self, value: u64) -> Result<(), BinaryEncodeError> {
        self.raw(&value.to_be_bytes())
    }

    pub fn i64(&mut self, value: i64) -> Result<(), BinaryEncodeError> {
        self.raw(&value.to_be_bytes())
    }

    /// Writes one IEEE-754 binary64 value in canonical network byte order.
    pub fn f64(&mut self, value: f64) -> Result<(), BinaryEncodeError> {
        self.raw(&value.to_be_bytes())
    }

    pub fn count_u32(
        &mut self,
        field: &'static str,
        value: usize,
    ) -> Result<(), BinaryEncodeError> {
        let encoded = u32::try_from(value).map_err(|_| BinaryEncodeError::IntegerOutOfRange {
            field,
            value: value as u128,
            target: IntegerWidth::U32,
        })?;
        self.u32(encoded)
    }

    pub fn usize_u64(
        &mut self,
        field: &'static str,
        value: usize,
    ) -> Result<(), BinaryEncodeError> {
        let encoded = u64::try_from(value).map_err(|_| BinaryEncodeError::IntegerOutOfRange {
            field,
            value: value as u128,
            target: IntegerWidth::U64,
        })?;
        self.u64(encoded)
    }

    pub fn raw(&mut self, value: &[u8]) -> Result<(), BinaryEncodeError> {
        self.reserve(value.len())?;
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.bytes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bytes.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.bytes
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }

    fn reserve(&mut self, additional: usize) -> Result<(), BinaryEncodeError> {
        self.bytes
            .try_reserve_exact(additional)
            .map_err(|_| BinaryEncodeError::AllocationFailed {
                current: self.bytes.len(),
                additional,
            })
    }

    fn patch_u32(&mut self, offset: usize, value: u32) {
        self.bytes[offset..offset + std::mem::size_of::<u32>()]
            .copy_from_slice(&value.to_be_bytes());
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryReader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> BinaryReader<'a> {
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    pub const fn offset(self) -> usize {
        self.offset
    }

    pub const fn remaining(self) -> usize {
        self.bytes.len().saturating_sub(self.offset)
    }

    pub fn ensure(&self, needed: usize) -> Result<(), BinaryDecodeError> {
        let remaining = self.remaining();
        if remaining < needed {
            return Err(BinaryDecodeError::Truncated {
                offset: self.offset,
                needed,
                remaining,
            });
        }
        Ok(())
    }

    pub fn ensure_count(
        &self,
        count: usize,
        minimum_entry_bytes: usize,
    ) -> Result<(), BinaryDecodeError> {
        self.ensure(count.saturating_mul(minimum_entry_bytes))
    }

    pub fn take(&mut self, length: usize) -> Result<&'a [u8], BinaryDecodeError> {
        self.ensure(length)?;
        let start = self.offset;
        self.offset += length;
        Ok(&self.bytes[start..self.offset])
    }

    pub fn array<const N: usize>(&mut self) -> Result<[u8; N], BinaryDecodeError> {
        let bytes = self.take(N)?;
        let mut value = [0; N];
        value.copy_from_slice(bytes);
        Ok(value)
    }

    pub fn u8(&mut self) -> Result<u8, BinaryDecodeError> {
        Ok(self.array::<1>()?[0])
    }

    pub fn u16(&mut self) -> Result<u16, BinaryDecodeError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    pub fn u32(&mut self) -> Result<u32, BinaryDecodeError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    pub fn i32(&mut self) -> Result<i32, BinaryDecodeError> {
        Ok(i32::from_be_bytes(self.array()?))
    }

    pub fn u64(&mut self) -> Result<u64, BinaryDecodeError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    pub fn i64(&mut self) -> Result<i64, BinaryDecodeError> {
        Ok(i64::from_be_bytes(self.array()?))
    }

    /// Reads one IEEE-754 binary64 value in canonical network byte order.
    pub fn f64(&mut self) -> Result<f64, BinaryDecodeError> {
        Ok(f64::from_be_bytes(self.array()?))
    }

    pub fn count_u32(&mut self, field: &'static str) -> Result<usize, BinaryDecodeError> {
        let value = u64::from(self.u32()?);
        usize::try_from(value).map_err(|_| BinaryDecodeError::IntegerOutOfRange { field, value })
    }

    pub fn usize_u64(&mut self, field: &'static str) -> Result<usize, BinaryDecodeError> {
        let value = self.u64()?;
        usize::try_from(value).map_err(|_| BinaryDecodeError::IntegerOutOfRange { field, value })
    }

    pub fn finish(self) -> Result<(), BinaryDecodeError> {
        let remaining = self.remaining();
        if remaining == 0 {
            Ok(())
        } else {
            Err(BinaryDecodeError::TrailingBytes { remaining })
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryValueLimits {
    pub string_bytes: usize,
    pub binary_bytes: usize,
    pub repeated_items: usize,
    pub nesting_depth: usize,
    pub message_nodes: usize,
}

impl BinaryValueLimits {
    pub const fn new(
        string_bytes: usize,
        binary_bytes: usize,
        repeated_items: usize,
        nesting_depth: usize,
        message_nodes: usize,
    ) -> Self {
        Self {
            string_bytes,
            binary_bytes,
            repeated_items,
            nesting_depth,
            message_nodes,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BinaryValueResource {
    StringBytes,
    BinaryBytes,
    RepeatedItems,
    NestingDepth,
    MessageNodes,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinaryNumericRange {
    minimum: i128,
    maximum: i128,
}

impl BinaryNumericRange {
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

    pub const fn minimum(self) -> Option<i128> {
        if self.minimum == i128::MIN {
            None
        } else {
            Some(self.minimum)
        }
    }

    pub const fn maximum(self) -> Option<i128> {
        if self.maximum == i128::MAX {
            None
        } else {
            Some(self.maximum)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryNumericRangeError {
    pub field: &'static str,
    pub value: i128,
    pub range: BinaryNumericRange,
}

impl Display for BinaryNumericRangeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "binary field `{}` value {} is outside",
            self.field, self.value
        )?;
        match (self.range.minimum(), self.range.maximum()) {
            (Some(minimum), Some(maximum)) => write!(formatter, " {minimum}..={maximum}"),
            (Some(minimum), None) => write!(formatter, " {minimum}.."),
            (None, Some(maximum)) => write!(formatter, " ..={maximum}"),
            (None, None) => formatter.write_str(" an empty numeric range"),
        }
    }
}

impl Error for BinaryNumericRangeError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinarySizeRange {
    minimum: Option<u32>,
    maximum: Option<u32>,
}

impl BinarySizeRange {
    pub const fn new(minimum: Option<u32>, maximum: Option<u32>) -> Self {
        Self { minimum, maximum }
    }

    pub const fn minimum(self) -> Option<u32> {
        self.minimum
    }

    pub const fn maximum(self) -> Option<u32> {
        self.maximum
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinarySizeRangeError {
    pub field: &'static str,
    pub actual: usize,
    pub range: BinarySizeRange,
}

impl Display for BinarySizeRangeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "binary field `{}` size {} is outside",
            self.field, self.actual
        )?;
        match (self.range.minimum(), self.range.maximum()) {
            (Some(minimum), Some(maximum)) => write!(formatter, " {minimum}..={maximum}"),
            (Some(minimum), None) => write!(formatter, " {minimum}.."),
            (None, Some(maximum)) => write!(formatter, " ..={maximum}"),
            (None, None) => formatter.write_str(" an empty size range"),
        }
    }
}

impl Error for BinarySizeRangeError {}

pub fn check_size_range(
    field: &'static str,
    actual: usize,
    range: BinarySizeRange,
) -> Result<(), BinarySizeRangeError> {
    let below = range
        .minimum
        .is_some_and(|minimum| actual < minimum as usize);
    let above = range
        .maximum
        .is_some_and(|maximum| actual > maximum as usize);
    if below || above {
        Err(BinarySizeRangeError {
            field,
            actual,
            range,
        })
    } else {
        Ok(())
    }
}

pub fn check_numeric_range(
    field: &'static str,
    value: i128,
    range: BinaryNumericRange,
) -> Result<(), BinaryNumericRangeError> {
    if value < range.minimum || value > range.maximum {
        Err(BinaryNumericRangeError {
            field,
            value,
            range,
        })
    } else {
        Ok(())
    }
}

impl Display for BinaryValueResource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::StringBytes => "string bytes",
            Self::BinaryBytes => "binary bytes",
            Self::RepeatedItems => "repeated items",
            Self::NestingDepth => "nesting depth",
            Self::MessageNodes => "message nodes",
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BinaryValueLimitError {
    pub resource: BinaryValueResource,
    pub field: &'static str,
    pub actual: usize,
    pub limit: usize,
}

impl Display for BinaryValueLimitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "binary field `{}` uses {} {}, exceeding limit {}",
            self.field, self.actual, self.resource, self.limit
        )
    }
}

impl Error for BinaryValueLimitError {}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryValueContext {
    limits: BinaryValueLimits,
    depth: usize,
    nodes: usize,
}

impl BinaryValueContext {
    pub const fn new(limits: BinaryValueLimits) -> Self {
        Self {
            limits,
            depth: 0,
            nodes: 0,
        }
    }

    pub const fn limits(self) -> BinaryValueLimits {
        self.limits
    }

    pub const fn depth(self) -> usize {
        self.depth
    }

    pub const fn nodes(self) -> usize {
        self.nodes
    }

    pub fn within_message<T, E>(
        &mut self,
        type_id: &'static str,
        operation: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<T, E>
    where
        E: From<BinaryValueLimitError>,
    {
        let next_depth = self.depth.saturating_add(1);
        self.check(
            BinaryValueResource::NestingDepth,
            type_id,
            next_depth,
            self.limits.nesting_depth,
        )?;
        let next_nodes = self.nodes.saturating_add(1);
        self.check(
            BinaryValueResource::MessageNodes,
            type_id,
            next_nodes,
            self.limits.message_nodes,
        )?;
        self.depth = next_depth;
        self.nodes = next_nodes;
        let result = operation(self);
        self.depth -= 1;
        result
    }

    pub fn check_string(
        &self,
        field: &'static str,
        actual: usize,
    ) -> Result<(), BinaryValueLimitError> {
        self.check(
            BinaryValueResource::StringBytes,
            field,
            actual,
            self.limits.string_bytes,
        )
    }

    pub fn check_binary(
        &self,
        field: &'static str,
        actual: usize,
    ) -> Result<(), BinaryValueLimitError> {
        self.check(
            BinaryValueResource::BinaryBytes,
            field,
            actual,
            self.limits.binary_bytes,
        )
    }

    pub fn check_repeated(
        &self,
        field: &'static str,
        actual: usize,
    ) -> Result<(), BinaryValueLimitError> {
        self.check(
            BinaryValueResource::RepeatedItems,
            field,
            actual,
            self.limits.repeated_items,
        )
    }

    fn check(
        &self,
        resource: BinaryValueResource,
        field: &'static str,
        actual: usize,
        limit: usize,
    ) -> Result<(), BinaryValueLimitError> {
        if actual > limit {
            Err(BinaryValueLimitError {
                resource,
                field,
                actual,
                limit,
            })
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinaryValueEncodeError {
    Limit(BinaryValueLimitError),
    NumericRange(BinaryNumericRangeError),
    SizeRange(BinarySizeRangeError),
    Primitive(BinaryEncodeError),
    NonCanonicalExtensibleFieldTag {
        record: &'static str,
        previous: Option<u16>,
        current: u16,
    },
}

impl Display for BinaryValueEncodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit(error) => Display::fmt(error, formatter),
            Self::NumericRange(error) => Display::fmt(error, formatter),
            Self::SizeRange(error) => Display::fmt(error, formatter),
            Self::Primitive(error) => Display::fmt(error, formatter),
            Self::NonCanonicalExtensibleFieldTag {
                record,
                previous,
                current,
            } => match previous {
                Some(previous) => write!(
                    formatter,
                    "extensible record `{record}` field tag {current} does not follow tag {previous}"
                ),
                None => write!(
                    formatter,
                    "extensible record `{record}` uses reserved field tag {current}"
                ),
            },
        }
    }
}

impl Error for BinaryValueEncodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Limit(error) => Some(error),
            Self::NumericRange(error) => Some(error),
            Self::SizeRange(error) => Some(error),
            Self::Primitive(error) => Some(error),
            Self::NonCanonicalExtensibleFieldTag { .. } => None,
        }
    }
}

impl From<BinaryValueLimitError> for BinaryValueEncodeError {
    fn from(error: BinaryValueLimitError) -> Self {
        Self::Limit(error)
    }
}

impl From<BinaryEncodeError> for BinaryValueEncodeError {
    fn from(error: BinaryEncodeError) -> Self {
        Self::Primitive(error)
    }
}

impl From<BinaryNumericRangeError> for BinaryValueEncodeError {
    fn from(error: BinaryNumericRangeError) -> Self {
        Self::NumericRange(error)
    }
}

impl From<BinarySizeRangeError> for BinaryValueEncodeError {
    fn from(error: BinarySizeRangeError) -> Self {
        Self::SizeRange(error)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinaryValueDecodeError {
    Limit(BinaryValueLimitError),
    NumericRange(BinaryNumericRangeError),
    SizeRange(BinarySizeRangeError),
    Primitive(BinaryDecodeError),
    InvalidUtf8 {
        field: &'static str,
        source: Utf8Error,
    },
    InvalidBoolean {
        field: &'static str,
        value: u8,
    },
    InvalidOptionFlag {
        field: &'static str,
        value: u8,
    },
    InvalidVariantTag {
        type_id: &'static str,
        value: u8,
    },
    NonCanonicalExtensibleFieldTag {
        record: &'static str,
        previous: Option<u16>,
        current: u16,
    },
    MissingExtensibleField {
        record: &'static str,
        field: &'static str,
        tag: u16,
    },
    AllocationFailed {
        field: &'static str,
        entries: usize,
    },
}

impl Display for BinaryValueDecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Limit(error) => Display::fmt(error, formatter),
            Self::NumericRange(error) => Display::fmt(error, formatter),
            Self::SizeRange(error) => Display::fmt(error, formatter),
            Self::Primitive(error) => Display::fmt(error, formatter),
            Self::InvalidUtf8 { field, source } => {
                write!(formatter, "binary field `{field}` is not UTF-8: {source}")
            }
            Self::InvalidBoolean { field, value } => {
                write!(
                    formatter,
                    "binary field `{field}` has invalid boolean {value}"
                )
            }
            Self::InvalidOptionFlag { field, value } => {
                write!(
                    formatter,
                    "binary field `{field}` has invalid option flag {value}"
                )
            }
            Self::InvalidVariantTag { type_id, value } => {
                write!(
                    formatter,
                    "binary type `{type_id}` has unknown variant tag {value}"
                )
            }
            Self::NonCanonicalExtensibleFieldTag {
                record,
                previous,
                current,
            } => match previous {
                Some(previous) => write!(
                    formatter,
                    "extensible record `{record}` field tag {current} does not follow tag {previous}"
                ),
                None => write!(
                    formatter,
                    "extensible record `{record}` uses reserved field tag {current}"
                ),
            },
            Self::MissingExtensibleField { record, field, tag } => write!(
                formatter,
                "extensible record `{record}` is missing required field `{field}` with tag {tag}"
            ),
            Self::AllocationFailed { field, entries } => write!(
                formatter,
                "could not allocate {entries} entries for binary field `{field}`"
            ),
        }
    }
}

impl Error for BinaryValueDecodeError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Limit(error) => Some(error),
            Self::NumericRange(error) => Some(error),
            Self::SizeRange(error) => Some(error),
            Self::Primitive(error) => Some(error),
            Self::InvalidUtf8 { source, .. } => Some(source),
            Self::InvalidBoolean { .. }
            | Self::InvalidOptionFlag { .. }
            | Self::InvalidVariantTag { .. }
            | Self::NonCanonicalExtensibleFieldTag { .. }
            | Self::MissingExtensibleField { .. }
            | Self::AllocationFailed { .. } => None,
        }
    }
}

impl From<BinaryValueLimitError> for BinaryValueDecodeError {
    fn from(error: BinaryValueLimitError) -> Self {
        Self::Limit(error)
    }
}

impl From<BinaryDecodeError> for BinaryValueDecodeError {
    fn from(error: BinaryDecodeError) -> Self {
        Self::Primitive(error)
    }
}

impl From<BinaryNumericRangeError> for BinaryValueDecodeError {
    fn from(error: BinaryNumericRangeError) -> Self {
        Self::NumericRange(error)
    }
}

impl From<BinarySizeRangeError> for BinaryValueDecodeError {
    fn from(error: BinarySizeRangeError) -> Self {
        Self::SizeRange(error)
    }
}

pub trait BinaryEncode {
    fn encode_binary(
        &self,
        writer: &mut BinaryWriter,
        context: &mut BinaryValueContext,
    ) -> Result<(), BinaryValueEncodeError>;
}

/// A borrowed sequence whose items can be encoded without first collecting an
/// owned wire-model vector.
///
/// Generated borrowed models use this object-safe boundary for repeated
/// fields. Implementations must emit exactly [`Self::len`] items; the generated
/// owner writes and bounds the canonical count before calling `encode_items`.
pub trait BinaryEncodeSequence {
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn encode_items(
        &self,
        writer: &mut BinaryWriter,
        context: &mut BinaryValueContext,
    ) -> Result<(), BinaryValueEncodeError>;
}

/// A copyable reference to a borrowed repeated-field encoder.
#[derive(Clone, Copy)]
pub struct BinaryEncodeSequenceRef<'a> {
    sequence: &'a dyn BinaryEncodeSequence,
}

impl<'a> BinaryEncodeSequenceRef<'a> {
    pub const fn new(sequence: &'a dyn BinaryEncodeSequence) -> Self {
        Self { sequence }
    }

    pub fn len(self) -> usize {
        self.sequence.len()
    }

    pub fn is_empty(self) -> bool {
        self.sequence.is_empty()
    }

    pub fn encode_items(
        self,
        writer: &mut BinaryWriter,
        context: &mut BinaryValueContext,
    ) -> Result<(), BinaryValueEncodeError> {
        self.sequence.encode_items(writer, context)
    }
}

impl fmt::Debug for BinaryEncodeSequenceRef<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("BinaryEncodeSequenceRef")
            .field("len", &self.sequence.len())
            .finish_non_exhaustive()
    }
}

/// Adapts a borrowed slice of generated values to a repeated-field encoder.
#[derive(Clone, Copy, Debug)]
pub struct BinaryEncodeSlice<'a, T> {
    values: &'a [T],
}

impl<'a, T> BinaryEncodeSlice<'a, T> {
    pub const fn new(values: &'a [T]) -> Self {
        Self { values }
    }

    pub const fn values(self) -> &'a [T] {
        self.values
    }
}

impl<T: BinaryEncode> BinaryEncodeSequence for BinaryEncodeSlice<'_, T> {
    fn len(&self) -> usize {
        self.values.len()
    }

    fn encode_items(
        &self,
        writer: &mut BinaryWriter,
        context: &mut BinaryValueContext,
    ) -> Result<(), BinaryValueEncodeError> {
        for value in self.values {
            value.encode_binary(writer, context)?;
        }
        Ok(())
    }
}

pub trait BinaryDecode: Sized {
    fn decode_binary(
        reader: &mut BinaryReader<'_>,
        context: &mut BinaryValueContext,
    ) -> Result<Self, BinaryValueDecodeError>;
}

pub fn encode_value<T: BinaryEncode>(
    value: &T,
    limits: BinaryValueLimits,
) -> Result<Vec<u8>, BinaryValueEncodeError> {
    let mut writer = BinaryWriter::new();
    let mut context = BinaryValueContext::new(limits);
    value.encode_binary(&mut writer, &mut context)?;
    Ok(writer.into_bytes())
}

pub fn decode_value<T: BinaryDecode>(
    bytes: &[u8],
    limits: BinaryValueLimits,
) -> Result<T, BinaryValueDecodeError> {
    let mut reader = BinaryReader::new(bytes);
    let mut context = BinaryValueContext::new(limits);
    let value = T::decode_binary(&mut reader, &mut context)?;
    reader.finish()?;
    Ok(value)
}

pub fn encode_utf8(
    writer: &mut BinaryWriter,
    context: &BinaryValueContext,
    field: &'static str,
    value: &str,
) -> Result<(), BinaryValueEncodeError> {
    context.check_string(field, value.len())?;
    writer.count_u32(field, value.len())?;
    writer.raw(value.as_bytes())?;
    Ok(())
}

pub fn encode_utf8_constrained(
    writer: &mut BinaryWriter,
    context: &BinaryValueContext,
    field: &'static str,
    value: &str,
    range: BinarySizeRange,
) -> Result<(), BinaryValueEncodeError> {
    check_size_range(field, value.len(), range)?;
    encode_utf8(writer, context, field, value)
}

pub fn decode_utf8(
    reader: &mut BinaryReader<'_>,
    context: &BinaryValueContext,
    field: &'static str,
) -> Result<String, BinaryValueDecodeError> {
    let length = reader.count_u32(field)?;
    context.check_string(field, length)?;
    let decoded = std::str::from_utf8(reader.take(length)?)
        .map_err(|source| BinaryValueDecodeError::InvalidUtf8 { field, source })?;
    let mut value = String::new();
    value
        .try_reserve_exact(length)
        .map_err(|_| BinaryValueDecodeError::AllocationFailed {
            field,
            entries: length,
        })?;
    value.push_str(decoded);
    Ok(value)
}

pub fn decode_utf8_constrained(
    reader: &mut BinaryReader<'_>,
    context: &BinaryValueContext,
    field: &'static str,
    range: BinarySizeRange,
) -> Result<String, BinaryValueDecodeError> {
    let length = reader.count_u32(field)?;
    context.check_string(field, length)?;
    check_size_range(field, length, range)?;
    let decoded = std::str::from_utf8(reader.take(length)?)
        .map_err(|source| BinaryValueDecodeError::InvalidUtf8 { field, source })?;
    let mut value = String::new();
    value
        .try_reserve_exact(length)
        .map_err(|_| BinaryValueDecodeError::AllocationFailed {
            field,
            entries: length,
        })?;
    value.push_str(decoded);
    Ok(value)
}

pub fn encode_bytes(
    writer: &mut BinaryWriter,
    context: &BinaryValueContext,
    field: &'static str,
    value: &[u8],
) -> Result<(), BinaryValueEncodeError> {
    context.check_binary(field, value.len())?;
    writer.count_u32(field, value.len())?;
    writer.raw(value)?;
    Ok(())
}

pub fn encode_bytes_constrained(
    writer: &mut BinaryWriter,
    context: &BinaryValueContext,
    field: &'static str,
    value: &[u8],
    range: BinarySizeRange,
) -> Result<(), BinaryValueEncodeError> {
    check_size_range(field, value.len(), range)?;
    encode_bytes(writer, context, field, value)
}

pub fn decode_bytes(
    reader: &mut BinaryReader<'_>,
    context: &BinaryValueContext,
    field: &'static str,
) -> Result<Vec<u8>, BinaryValueDecodeError> {
    let length = reader.count_u32(field)?;
    context.check_binary(field, length)?;
    let encoded = reader.take(length)?;
    let mut value = Vec::new();
    value
        .try_reserve_exact(length)
        .map_err(|_| BinaryValueDecodeError::AllocationFailed {
            field,
            entries: length,
        })?;
    value.extend_from_slice(encoded);
    Ok(value)
}

pub fn decode_bytes_constrained(
    reader: &mut BinaryReader<'_>,
    context: &BinaryValueContext,
    field: &'static str,
    range: BinarySizeRange,
) -> Result<Vec<u8>, BinaryValueDecodeError> {
    let length = reader.count_u32(field)?;
    context.check_binary(field, length)?;
    check_size_range(field, length, range)?;
    let encoded = reader.take(length)?;
    let mut value = Vec::new();
    value
        .try_reserve_exact(length)
        .map_err(|_| BinaryValueDecodeError::AllocationFailed {
            field,
            entries: length,
        })?;
    value.extend_from_slice(encoded);
    Ok(value)
}

pub fn encode_repeated_len(
    writer: &mut BinaryWriter,
    context: &BinaryValueContext,
    field: &'static str,
    length: usize,
) -> Result<(), BinaryValueEncodeError> {
    context.check_repeated(field, length)?;
    writer.count_u32(field, length)?;
    Ok(())
}

pub fn decode_repeated_len(
    reader: &mut BinaryReader<'_>,
    context: &BinaryValueContext,
    field: &'static str,
) -> Result<usize, BinaryValueDecodeError> {
    let length = reader.count_u32(field)?;
    context.check_repeated(field, length)?;
    Ok(length)
}

pub fn try_vec_with_capacity<T>(
    field: &'static str,
    entries: usize,
) -> Result<Vec<T>, BinaryValueDecodeError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(entries)
        .map_err(|_| BinaryValueDecodeError::AllocationFailed { field, entries })?;
    Ok(values)
}

/// Encodes one length-delimited record whose fields are independently tagged
/// and length-delimited.
///
/// Field tag zero is reserved. Emitted tags must be strictly increasing, which
/// gives every logical record one canonical byte representation. Lengths are
/// patched into the destination writer in place, so neither the record nor an
/// individual field needs a temporary buffer.
pub fn encode_extensible_record(
    writer: &mut BinaryWriter,
    record: &'static str,
    encoded_size_range: BinarySizeRange,
    encode: impl FnOnce(&mut BinaryExtensibleWriter<'_>) -> Result<(), BinaryValueEncodeError>,
) -> Result<(), BinaryValueEncodeError> {
    let length_offset = writer.len();
    writer.u32(0)?;
    let body_offset = writer.len();
    {
        let mut record_writer = BinaryExtensibleWriter {
            writer,
            record,
            previous_tag: None,
        };
        encode(&mut record_writer)?;
    }
    let body_length = writer.len() - body_offset;
    check_size_range(record, body_length, encoded_size_range)?;
    let encoded_length =
        u32::try_from(body_length).map_err(|_| BinaryEncodeError::IntegerOutOfRange {
            field: record,
            value: body_length as u128,
            target: IntegerWidth::U32,
        })?;
    writer.patch_u32(length_offset, encoded_length);
    Ok(())
}

/// Writes canonical fields inside one [`encode_extensible_record`] envelope.
pub struct BinaryExtensibleWriter<'a> {
    writer: &'a mut BinaryWriter,
    record: &'static str,
    previous_tag: Option<u16>,
}

impl BinaryExtensibleWriter<'_> {
    /// Writes one present field. Optional fields are omitted by not calling
    /// this method; calling it with an empty body represents present-but-empty.
    pub fn field(
        &mut self,
        tag: u16,
        field: &'static str,
        encode: impl FnOnce(&mut BinaryWriter) -> Result<(), BinaryValueEncodeError>,
    ) -> Result<(), BinaryValueEncodeError> {
        if tag == 0 || self.previous_tag.is_some_and(|previous| previous >= tag) {
            return Err(BinaryValueEncodeError::NonCanonicalExtensibleFieldTag {
                record: self.record,
                previous: self.previous_tag,
                current: tag,
            });
        }

        self.writer.u16(tag)?;
        let length_offset = self.writer.len();
        self.writer.u32(0)?;
        let body_offset = self.writer.len();
        encode(self.writer)?;
        let body_length = self.writer.len() - body_offset;
        let encoded_length =
            u32::try_from(body_length).map_err(|_| BinaryEncodeError::IntegerOutOfRange {
                field,
                value: body_length as u128,
                target: IntegerWidth::U32,
            })?;
        self.writer.patch_u32(length_offset, encoded_length);
        self.previous_tag = Some(tag);
        Ok(())
    }
}

/// Opens one bounded extensible-record envelope without allocating.
///
/// The declared record size is checked before its bytes are consumed. Unknown
/// fields can then be skipped by dropping the borrowed field returned from
/// [`BinaryExtensibleReader::next_field`].
pub fn decode_extensible_record<'a>(
    reader: &mut BinaryReader<'a>,
    record: &'static str,
    encoded_size_range: BinarySizeRange,
) -> Result<BinaryExtensibleReader<'a>, BinaryValueDecodeError> {
    let body_length = reader.count_u32(record)?;
    check_size_range(record, body_length, encoded_size_range)?;
    let body = reader.take(body_length)?;
    Ok(BinaryExtensibleReader {
        reader: BinaryReader::new(body),
        record,
        previous_tag: None,
    })
}

/// Iterates the canonical fields of a borrowed extensible-record envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryExtensibleReader<'a> {
    reader: BinaryReader<'a>,
    record: &'static str,
    previous_tag: Option<u16>,
}

impl<'a> BinaryExtensibleReader<'a> {
    pub fn next_field(
        &mut self,
    ) -> Result<Option<BinaryExtensibleField<'a>>, BinaryValueDecodeError> {
        if self.reader.remaining() == 0 {
            return Ok(None);
        }

        let tag = self.reader.u16()?;
        if tag == 0 || self.previous_tag.is_some_and(|previous| previous >= tag) {
            return Err(BinaryValueDecodeError::NonCanonicalExtensibleFieldTag {
                record: self.record,
                previous: self.previous_tag,
                current: tag,
            });
        }
        let body_length = self.reader.count_u32(self.record)?;
        let body = self.reader.take(body_length)?;
        self.previous_tag = Some(tag);
        Ok(Some(BinaryExtensibleField {
            tag,
            reader: BinaryReader::new(body),
        }))
    }

    pub fn finish(self) -> Result<(), BinaryValueDecodeError> {
        self.reader.finish()?;
        Ok(())
    }
}

/// A single borrowed field body from an extensible record.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BinaryExtensibleField<'a> {
    tag: u16,
    reader: BinaryReader<'a>,
}

impl<'a> BinaryExtensibleField<'a> {
    pub const fn tag(&self) -> u16 {
        self.tag
    }

    pub fn reader(&mut self) -> &mut BinaryReader<'a> {
        &mut self.reader
    }

    /// Known fields must consume their complete body. Unknown fields may be
    /// dropped without calling `finish`, which skips them without allocation.
    pub fn finish(self) -> Result<(), BinaryValueDecodeError> {
        self.reader.finish()?;
        Ok(())
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BinaryFieldKind {
    U8,
    U16,
    U32,
    U64,
    Bytes,
    Utf8,
    Message,
    I32,
    I64,
    Bool,
    Enumeration,
    F64,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BinaryCardinality {
    Required,
    Optional,
    Repeated,
}

/// Maps one typed capability command to its canonical binary root variants.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinaryCommandSchema {
    id: CommandId,
    method: MethodTag,
    command_tag: u8,
    result_tag: u8,
}

impl BinaryCommandSchema {
    pub const fn new(id: CommandId, method: MethodTag, command_tag: u8, result_tag: u8) -> Self {
        Self {
            id,
            method,
            command_tag,
            result_tag,
        }
    }

    pub const fn id(self) -> CommandId {
        self.id
    }

    pub const fn method(self) -> MethodTag {
        self.method
    }

    pub const fn command_tag(self) -> u8 {
        self.command_tag
    }

    pub const fn result_tag(self) -> u8 {
        self.result_tag
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinaryFieldSchema {
    name: &'static str,
    kind: BinaryFieldKind,
    cardinality: BinaryCardinality,
    message_type: Option<&'static str>,
    numeric_range: Option<BinaryNumericRange>,
    length_range: Option<BinarySizeRange>,
    item_count_range: Option<BinarySizeRange>,
    external_count_field: Option<&'static str>,
}

impl BinaryFieldSchema {
    pub const fn new(
        name: &'static str,
        kind: BinaryFieldKind,
        cardinality: BinaryCardinality,
    ) -> Self {
        Self {
            name,
            kind,
            cardinality,
            message_type: None,
            numeric_range: None,
            length_range: None,
            item_count_range: None,
            external_count_field: None,
        }
    }

    pub const fn message(
        name: &'static str,
        message_type: &'static str,
        cardinality: BinaryCardinality,
    ) -> Self {
        Self {
            name,
            kind: BinaryFieldKind::Message,
            cardinality,
            message_type: Some(message_type),
            numeric_range: None,
            length_range: None,
            item_count_range: None,
            external_count_field: None,
        }
    }

    pub const fn enumeration(
        name: &'static str,
        enumeration_type: &'static str,
        cardinality: BinaryCardinality,
    ) -> Self {
        Self {
            name,
            kind: BinaryFieldKind::Enumeration,
            cardinality,
            message_type: Some(enumeration_type),
            numeric_range: None,
            length_range: None,
            item_count_range: None,
            external_count_field: None,
        }
    }

    pub const fn with_numeric_range(mut self, numeric_range: BinaryNumericRange) -> Self {
        self.numeric_range = Some(numeric_range);
        self
    }

    pub const fn with_length_range(mut self, length_range: BinarySizeRange) -> Self {
        self.length_range = Some(length_range);
        self
    }

    pub const fn with_item_count_range(mut self, item_count_range: BinarySizeRange) -> Self {
        self.item_count_range = Some(item_count_range);
        self
    }

    pub const fn with_external_count(mut self, count_field: &'static str) -> Self {
        self.external_count_field = Some(count_field);
        self
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn kind(self) -> BinaryFieldKind {
        self.kind
    }

    pub const fn cardinality(self) -> BinaryCardinality {
        self.cardinality
    }

    pub const fn message_type(self) -> Option<&'static str> {
        self.message_type
    }

    pub const fn referenced_type(self) -> Option<&'static str> {
        self.message_type
    }

    pub const fn numeric_range(self) -> Option<BinaryNumericRange> {
        self.numeric_range
    }

    pub const fn length_range(self) -> Option<BinarySizeRange> {
        self.length_range
    }

    pub const fn item_count_range(self) -> Option<BinarySizeRange> {
        self.item_count_range
    }

    pub const fn external_count_field(self) -> Option<&'static str> {
        self.external_count_field
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinaryVariantSchema {
    tag: u8,
    name: &'static str,
    fields: &'static [BinaryFieldSchema],
}

impl BinaryVariantSchema {
    pub const fn new(tag: u8, name: &'static str, fields: &'static [BinaryFieldSchema]) -> Self {
        Self { tag, name, fields }
    }

    pub const fn tag(self) -> u8 {
        self.tag
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn fields(self) -> &'static [BinaryFieldSchema] {
        self.fields
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinarySchema {
    id: &'static str,
    version: ProtocolVersion,
    variants: &'static [BinaryVariantSchema],
}

impl BinarySchema {
    pub const fn new(
        id: &'static str,
        version: ProtocolVersion,
        variants: &'static [BinaryVariantSchema],
    ) -> Self {
        Self {
            id,
            version,
            variants,
        }
    }

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub const fn version(self) -> ProtocolVersion {
        self.version
    }

    pub const fn variants(self) -> &'static [BinaryVariantSchema] {
        self.variants
    }

    pub fn validate(self) -> Result<(), BinarySchemaError> {
        validate_name("schema", self.id)?;
        if self.variants.is_empty() {
            return Err(BinarySchemaError::NoVariants);
        }
        let mut previous_tag = None;
        for variant in self.variants {
            if let Some(previous) = previous_tag
                && previous >= variant.tag
            {
                return Err(BinarySchemaError::NonCanonicalVariantOrder {
                    previous,
                    current: variant.tag,
                });
            }
            previous_tag = Some(variant.tag);
            validate_name("variant", variant.name)?;
            validate_fields(self.id, variant.name, variant.fields)?;
        }
        Ok(())
    }

    /// Returns a deterministic layout fingerprint. The semantic minor version
    /// is excluded: unchanged positional layouts retain the same fingerprint.
    pub fn layout_fingerprint(self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        hash_bytes(&mut hash, self.id.as_bytes());
        hash_bytes(&mut hash, &self.version.major.to_be_bytes());
        for variant in self.variants {
            hash_bytes(&mut hash, &[variant.tag]);
            hash_bytes(&mut hash, variant.name.as_bytes());
            hash_fields(&mut hash, variant.fields);
        }
        hash
    }

    /// Existing positional variants must remain byte-for-byte identical in a
    /// minor release. New higher-tagged variants may be appended because
    /// capability negotiation prevents an older peer from receiving them.
    pub fn check_minor_compatibility(self, next: Self) -> Result<(), BinaryCompatibilityError> {
        self.validate()
            .map_err(BinaryCompatibilityError::InvalidPreviousSchema)?;
        next.validate()
            .map_err(BinaryCompatibilityError::InvalidNextSchema)?;
        check_identity_and_version(self.id, self.version, next.id, next.version)?;
        if next.variants.len() < self.variants.len()
            || self
                .variants
                .iter()
                .zip(next.variants)
                .any(|(previous, candidate)| previous != candidate)
        {
            return Err(BinaryCompatibilityError::PositionalLayoutChanged);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinaryRecordSchema {
    id: &'static str,
    version: ProtocolVersion,
    fields: &'static [BinaryFieldSchema],
}

impl BinaryRecordSchema {
    pub const fn new(
        id: &'static str,
        version: ProtocolVersion,
        fields: &'static [BinaryFieldSchema],
    ) -> Self {
        Self {
            id,
            version,
            fields,
        }
    }

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub const fn version(self) -> ProtocolVersion {
        self.version
    }

    pub const fn fields(self) -> &'static [BinaryFieldSchema] {
        self.fields
    }

    pub fn validate(self) -> Result<(), BinarySchemaError> {
        validate_name("record", self.id)?;
        validate_fields(self.id, "record", self.fields)
    }

    /// Returns a deterministic fingerprint of the positional record layout.
    /// The semantic minor version is excluded.
    pub fn layout_fingerprint(self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        hash_bytes(&mut hash, self.id.as_bytes());
        hash_bytes(&mut hash, &self.version.major.to_be_bytes());
        hash_fields(&mut hash, self.fields);
        hash
    }

    /// Positional records cannot append fields compatibly because they have no
    /// enclosing length or unknown-field representation. Their layout must
    /// therefore remain exact throughout one major version.
    pub fn check_minor_compatibility(self, next: Self) -> Result<(), BinaryCompatibilityError> {
        self.validate()
            .map_err(BinaryCompatibilityError::InvalidPreviousSchema)?;
        next.validate()
            .map_err(BinaryCompatibilityError::InvalidNextSchema)?;
        check_identity_and_version(self.id, self.version, next.id, next.version)?;
        if self.fields != next.fields {
            return Err(BinaryCompatibilityError::PositionalLayoutChanged);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinaryExtensibleFieldSchema {
    tag: u16,
    field: BinaryFieldSchema,
}

impl BinaryExtensibleFieldSchema {
    pub const fn new(tag: u16, field: BinaryFieldSchema) -> Self {
        Self { tag, field }
    }

    pub const fn tag(self) -> u16 {
        self.tag
    }

    pub const fn field(self) -> BinaryFieldSchema {
        self.field
    }
}

/// Schema metadata for a length-delimited record with independently tagged
/// fields. The encoded size range is part of the stable wire contract and must
/// include a maximum so unknown fields cannot bypass resource bounds.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinaryExtensibleRecordSchema {
    id: &'static str,
    version: ProtocolVersion,
    encoded_size_range: BinarySizeRange,
    fields: &'static [BinaryExtensibleFieldSchema],
}

impl BinaryExtensibleRecordSchema {
    pub const fn new(
        id: &'static str,
        version: ProtocolVersion,
        encoded_size_range: BinarySizeRange,
        fields: &'static [BinaryExtensibleFieldSchema],
    ) -> Self {
        Self {
            id,
            version,
            encoded_size_range,
            fields,
        }
    }

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub const fn version(self) -> ProtocolVersion {
        self.version
    }

    pub const fn encoded_size_range(self) -> BinarySizeRange {
        self.encoded_size_range
    }

    pub const fn fields(self) -> &'static [BinaryExtensibleFieldSchema] {
        self.fields
    }

    pub fn validate(self) -> Result<(), BinarySchemaError> {
        validate_name("extensible record", self.id)?;
        if !valid_size_range(self.encoded_size_range) || self.encoded_size_range.maximum().is_none()
        {
            return Err(BinarySchemaError::InvalidExtensibleRecordSizeRange {
                schema: self.id,
                range: self.encoded_size_range,
            });
        }

        let mut previous_tag = None;
        for (index, tagged) in self.fields.iter().enumerate() {
            if tagged.tag == 0 || previous_tag.is_some_and(|previous| previous >= tagged.tag) {
                return Err(BinarySchemaError::NonCanonicalExtensibleFieldTag {
                    schema: self.id,
                    previous: previous_tag,
                    current: tagged.tag,
                });
            }
            previous_tag = Some(tagged.tag);

            let field = tagged.field;
            validate_name("field", field.name)?;
            validate_field_reference(self.id, "record", field)?;
            if self.fields[..index]
                .iter()
                .any(|previous| previous.field.name == field.name)
            {
                return Err(BinarySchemaError::DuplicateField {
                    variant: "record",
                    field: field.name,
                });
            }
            validate_extensible_external_count(self.id, &self.fields[..index], field)?;
        }
        Ok(())
    }

    pub fn layout_fingerprint(self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        hash_bytes(&mut hash, b"extensible_record");
        hash_bytes(&mut hash, self.id.as_bytes());
        hash_bytes(&mut hash, &self.version.major.to_be_bytes());
        hash_size_range(&mut hash, self.encoded_size_range);
        for tagged in self.fields {
            hash_bytes(&mut hash, &tagged.tag.to_be_bytes());
            hash_fields(&mut hash, &[tagged.field]);
        }
        hash
    }

    /// Existing tags and their field contracts remain exact. A minor release
    /// may add optional tags at any canonical position because older decoders
    /// skip unknown fields and newer decoders can default an absent option.
    pub fn check_minor_compatibility(self, next: Self) -> Result<(), BinaryCompatibilityError> {
        self.validate()
            .map_err(BinaryCompatibilityError::InvalidPreviousSchema)?;
        next.validate()
            .map_err(BinaryCompatibilityError::InvalidNextSchema)?;
        check_identity_and_version(self.id, self.version, next.id, next.version)?;
        if self.encoded_size_range != next.encoded_size_range {
            return Err(BinaryCompatibilityError::ExtensibleRecordSizeRangeChanged {
                previous: self.encoded_size_range,
                next: next.encoded_size_range,
            });
        }

        for previous in self.fields {
            let candidate = next
                .fields
                .binary_search_by_key(&previous.tag, |field| field.tag)
                .ok()
                .map(|index| next.fields[index])
                .ok_or(BinaryCompatibilityError::ExtensibleFieldRemoved { tag: previous.tag })?;
            if candidate != *previous {
                return Err(BinaryCompatibilityError::ExtensibleFieldChanged { tag: previous.tag });
            }
        }
        for candidate in next.fields {
            if self
                .fields
                .binary_search_by_key(&candidate.tag, |field| field.tag)
                .is_err()
                && candidate.field.cardinality != BinaryCardinality::Optional
            {
                return Err(BinaryCompatibilityError::NonOptionalExtensibleFieldAdded {
                    tag: candidate.tag,
                    cardinality: candidate.field.cardinality,
                });
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinaryEnumVariantSchema {
    tag: u8,
    name: &'static str,
}

impl BinaryEnumVariantSchema {
    pub const fn new(tag: u8, name: &'static str) -> Self {
        Self { tag, name }
    }

    pub const fn tag(self) -> u8 {
        self.tag
    }

    pub const fn name(self) -> &'static str {
        self.name
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinaryEnumSchema {
    id: &'static str,
    version: ProtocolVersion,
    variants: &'static [BinaryEnumVariantSchema],
}

impl BinaryEnumSchema {
    pub const fn new(
        id: &'static str,
        version: ProtocolVersion,
        variants: &'static [BinaryEnumVariantSchema],
    ) -> Self {
        Self {
            id,
            version,
            variants,
        }
    }

    pub const fn id(self) -> &'static str {
        self.id
    }

    pub const fn version(self) -> ProtocolVersion {
        self.version
    }

    pub const fn variants(self) -> &'static [BinaryEnumVariantSchema] {
        self.variants
    }

    pub fn validate(self) -> Result<(), BinarySchemaError> {
        validate_name("enum", self.id)?;
        if self.variants.is_empty() {
            return Err(BinarySchemaError::NoVariants);
        }
        let mut previous_tag = None;
        for (index, variant) in self.variants.iter().enumerate() {
            if let Some(previous) = previous_tag
                && previous >= variant.tag
            {
                return Err(BinarySchemaError::NonCanonicalVariantOrder {
                    previous,
                    current: variant.tag,
                });
            }
            previous_tag = Some(variant.tag);
            validate_name("enum variant", variant.name)?;
            if self.variants[..index]
                .iter()
                .any(|previous| previous.name == variant.name)
            {
                return Err(BinarySchemaError::DuplicateVariant {
                    schema: self.id,
                    variant: variant.name,
                });
            }
        }
        Ok(())
    }

    pub fn layout_fingerprint(self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        hash_bytes(&mut hash, self.id.as_bytes());
        hash_bytes(&mut hash, &self.version.major.to_be_bytes());
        for variant in self.variants {
            hash_bytes(&mut hash, &[variant.tag]);
            hash_bytes(&mut hash, variant.name.as_bytes());
        }
        hash
    }

    pub fn check_minor_compatibility(self, next: Self) -> Result<(), BinaryCompatibilityError> {
        self.validate()
            .map_err(BinaryCompatibilityError::InvalidPreviousSchema)?;
        next.validate()
            .map_err(BinaryCompatibilityError::InvalidNextSchema)?;
        check_identity_and_version(self.id, self.version, next.id, next.version)?;
        if next.variants.len() < self.variants.len() || !next.variants.starts_with(self.variants) {
            return Err(BinaryCompatibilityError::PositionalLayoutChanged);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BinaryTypeSchema {
    Enumeration(BinaryEnumSchema),
    ExtensibleRecord(BinaryExtensibleRecordSchema),
    Record(BinaryRecordSchema),
    TaggedUnion(BinarySchema),
}

impl BinaryTypeSchema {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Enumeration(schema) => schema.id(),
            Self::ExtensibleRecord(schema) => schema.id(),
            Self::Record(schema) => schema.id(),
            Self::TaggedUnion(schema) => schema.id(),
        }
    }

    pub const fn version(self) -> ProtocolVersion {
        match self {
            Self::Enumeration(schema) => schema.version(),
            Self::ExtensibleRecord(schema) => schema.version(),
            Self::Record(schema) => schema.version(),
            Self::TaggedUnion(schema) => schema.version(),
        }
    }

    pub fn layout_fingerprint(self) -> u64 {
        match self {
            Self::Enumeration(schema) => schema.layout_fingerprint(),
            Self::ExtensibleRecord(schema) => schema.layout_fingerprint(),
            Self::Record(schema) => schema.layout_fingerprint(),
            Self::TaggedUnion(schema) => schema.layout_fingerprint(),
        }
    }

    pub fn check_minor_compatibility(self, next: Self) -> Result<(), BinaryTypeCompatibilityError> {
        match (self, next) {
            (Self::Enumeration(previous), Self::Enumeration(candidate)) => previous
                .check_minor_compatibility(candidate)
                .map_err(BinaryTypeCompatibilityError::Schema),
            (Self::ExtensibleRecord(previous), Self::ExtensibleRecord(candidate)) => previous
                .check_minor_compatibility(candidate)
                .map_err(BinaryTypeCompatibilityError::Schema),
            (Self::Record(previous), Self::Record(candidate)) => previous
                .check_minor_compatibility(candidate)
                .map_err(BinaryTypeCompatibilityError::Schema),
            (Self::TaggedUnion(previous), Self::TaggedUnion(candidate)) => previous
                .check_minor_compatibility(candidate)
                .map_err(BinaryTypeCompatibilityError::Schema),
            (previous, candidate) => Err(BinaryTypeCompatibilityError::KindChanged {
                id: previous.id(),
                previous: previous.kind(),
                next: candidate.kind(),
            }),
        }
    }

    pub const fn kind(self) -> BinaryTypeKind {
        match self {
            Self::Enumeration(_) => BinaryTypeKind::Enumeration,
            Self::ExtensibleRecord(_) => BinaryTypeKind::ExtensibleRecord,
            Self::Record(_) => BinaryTypeKind::Record,
            Self::TaggedUnion(_) => BinaryTypeKind::TaggedUnion,
        }
    }

    fn validate(self) -> Result<(), BinarySchemaError> {
        match self {
            Self::Enumeration(schema) => schema.validate(),
            Self::ExtensibleRecord(schema) => schema.validate(),
            Self::Record(schema) => schema.validate(),
            Self::TaggedUnion(schema) => schema.validate(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BinaryTypeKind {
    Enumeration,
    ExtensibleRecord,
    Record,
    TaggedUnion,
}

impl Display for BinaryTypeKind {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Enumeration => "enumeration",
            Self::ExtensibleRecord => "extensible record",
            Self::Record => "record",
            Self::TaggedUnion => "tagged union",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct BinarySchemaCatalog {
    types: &'static [BinaryTypeSchema],
}

impl BinarySchemaCatalog {
    pub const fn new(types: &'static [BinaryTypeSchema]) -> Self {
        Self { types }
    }

    pub const fn types(self) -> &'static [BinaryTypeSchema] {
        self.types
    }

    pub fn validate(self) -> Result<(), BinaryCatalogError> {
        let mut previous = None;
        for schema in self.types {
            schema.validate().map_err(BinaryCatalogError::Schema)?;
            if let Some(previous) = previous
                && previous >= schema.id()
            {
                return Err(BinaryCatalogError::NonCanonicalTypeOrder {
                    previous,
                    current: schema.id(),
                });
            }
            previous = Some(schema.id());
        }
        for schema in self.types {
            match schema {
                BinaryTypeSchema::Enumeration(_) => {}
                BinaryTypeSchema::ExtensibleRecord(record) => {
                    for tagged in record.fields() {
                        self.validate_references(
                            record.id(),
                            std::slice::from_ref(&tagged.field()),
                        )?;
                    }
                }
                BinaryTypeSchema::Record(record) => {
                    self.validate_references(record.id(), record.fields())?;
                }
                BinaryTypeSchema::TaggedUnion(union) => {
                    for variant in union.variants() {
                        self.validate_references(union.id(), variant.fields())?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Returns a deterministic fingerprint over every type layout in canonical
    /// ID order. Type minor versions are deliberately excluded.
    pub fn layout_fingerprint(self) -> u64 {
        let mut hash = 0xcbf29ce484222325_u64;
        for schema in self.types {
            hash_bytes(&mut hash, schema.id().as_bytes());
            hash_bytes(&mut hash, &schema.layout_fingerprint().to_be_bytes());
        }
        hash
    }

    /// Checks all previously published types against a candidate catalog.
    /// New types may be added at any canonical position, but an existing type
    /// cannot disappear or change its schema kind or positional contract.
    pub fn check_minor_compatibility(
        self,
        next: Self,
    ) -> Result<(), BinaryCatalogCompatibilityError> {
        self.validate()
            .map_err(BinaryCatalogCompatibilityError::InvalidPreviousCatalog)?;
        next.validate()
            .map_err(BinaryCatalogCompatibilityError::InvalidNextCatalog)?;
        for previous in self.types {
            let candidate = next
                .types
                .binary_search_by_key(&previous.id(), |schema| schema.id())
                .ok()
                .map(|index| next.types[index])
                .ok_or(BinaryCatalogCompatibilityError::TypeRemoved { id: previous.id() })?;
            previous
                .check_minor_compatibility(candidate)
                .map_err(BinaryCatalogCompatibilityError::Type)?;
        }
        Ok(())
    }

    fn validate_references(
        self,
        schema: &'static str,
        fields: &[BinaryFieldSchema],
    ) -> Result<(), BinaryCatalogError> {
        for field in fields {
            if let Some(referenced) = field.message_type() {
                let referenced_schema = self
                    .types
                    .binary_search_by_key(&referenced, |candidate| candidate.id())
                    .ok()
                    .map(|index| self.types[index])
                    .ok_or(BinaryCatalogError::UnknownMessageType {
                        schema,
                        field: field.name(),
                        referenced,
                    })?;
                let valid_kind = match field.kind() {
                    BinaryFieldKind::Message => {
                        referenced_schema.kind() != BinaryTypeKind::Enumeration
                    }
                    BinaryFieldKind::Enumeration => {
                        referenced_schema.kind() == BinaryTypeKind::Enumeration
                    }
                    _ => true,
                };
                if !valid_kind {
                    return Err(BinaryCatalogError::WrongReferencedTypeKind {
                        schema,
                        field: field.name(),
                        referenced,
                        actual: referenced_schema.kind(),
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinaryCatalogError {
    Schema(BinarySchemaError),
    NonCanonicalTypeOrder {
        previous: &'static str,
        current: &'static str,
    },
    UnknownMessageType {
        schema: &'static str,
        field: &'static str,
        referenced: &'static str,
    },
    WrongReferencedTypeKind {
        schema: &'static str,
        field: &'static str,
        referenced: &'static str,
        actual: BinaryTypeKind,
    },
}

impl Display for BinaryCatalogError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Schema(error) => Display::fmt(error, formatter),
            Self::NonCanonicalTypeOrder { previous, current } => write!(
                formatter,
                "binary type `{current}` does not follow `{previous}`"
            ),
            Self::UnknownMessageType {
                schema,
                field,
                referenced,
            } => write!(
                formatter,
                "binary field `{schema}.{field}` references unknown type `{referenced}`"
            ),
            Self::WrongReferencedTypeKind {
                schema,
                field,
                referenced,
                actual,
            } => write!(
                formatter,
                "binary field `{schema}.{field}` references `{referenced}` with incompatible {actual} kind"
            ),
        }
    }
}

impl Error for BinaryCatalogError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Schema(error) => Some(error),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinaryTypeCompatibilityError {
    KindChanged {
        id: &'static str,
        previous: BinaryTypeKind,
        next: BinaryTypeKind,
    },
    Schema(BinaryCompatibilityError),
}

impl Display for BinaryTypeCompatibilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::KindChanged { id, previous, next } => write!(
                formatter,
                "binary type `{id}` changed from {previous} to {next}"
            ),
            Self::Schema(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for BinaryTypeCompatibilityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Schema(error) => Some(error),
            Self::KindChanged { .. } => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinaryCatalogCompatibilityError {
    InvalidPreviousCatalog(BinaryCatalogError),
    InvalidNextCatalog(BinaryCatalogError),
    TypeRemoved { id: &'static str },
    Type(BinaryTypeCompatibilityError),
}

impl Display for BinaryCatalogCompatibilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPreviousCatalog(error) => {
                write!(formatter, "previous binary catalog is invalid: {error}")
            }
            Self::InvalidNextCatalog(error) => {
                write!(formatter, "next binary catalog is invalid: {error}")
            }
            Self::TypeRemoved { id } => write!(formatter, "binary type `{id}` was removed"),
            Self::Type(error) => Display::fmt(error, formatter),
        }
    }
}

impl Error for BinaryCatalogCompatibilityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPreviousCatalog(error) | Self::InvalidNextCatalog(error) => Some(error),
            Self::Type(error) => Some(error),
            Self::TypeRemoved { .. } => None,
        }
    }
}

fn validate_fields(
    schema: &'static str,
    variant: &'static str,
    fields: &'static [BinaryFieldSchema],
) -> Result<(), BinarySchemaError> {
    for (index, field) in fields.iter().enumerate() {
        validate_name("field", field.name)?;
        validate_field_reference(schema, variant, *field)?;
        if fields[..index]
            .iter()
            .any(|previous| previous.name == field.name)
        {
            return Err(BinarySchemaError::DuplicateField {
                variant,
                field: field.name,
            });
        }
        if let Some(count_field) = field.external_count_field {
            validate_name("external count field", count_field)?;
            if field.cardinality != BinaryCardinality::Repeated {
                return Err(BinarySchemaError::ExternalCountOnNonRepeatedField {
                    schema,
                    variant,
                    field: field.name,
                });
            }
            let count = fields[..index]
                .iter()
                .find(|candidate| candidate.name == count_field)
                .ok_or(BinarySchemaError::ExternalCountFieldMissingOrNotEarlier {
                    schema,
                    variant,
                    field: field.name,
                    count_field,
                })?;
            if count.kind != BinaryFieldKind::U32
                || count.cardinality != BinaryCardinality::Required
            {
                return Err(BinarySchemaError::InvalidExternalCountField {
                    schema,
                    variant,
                    field: field.name,
                    count_field,
                });
            }
        }
    }
    Ok(())
}

fn validate_extensible_external_count(
    schema: &'static str,
    previous_fields: &[BinaryExtensibleFieldSchema],
    field: BinaryFieldSchema,
) -> Result<(), BinarySchemaError> {
    let Some(count_field) = field.external_count_field else {
        return Ok(());
    };
    validate_name("external count field", count_field)?;
    if field.cardinality != BinaryCardinality::Repeated {
        return Err(BinarySchemaError::ExternalCountOnNonRepeatedField {
            schema,
            variant: "record",
            field: field.name,
        });
    }
    let count = previous_fields
        .iter()
        .map(|tagged| tagged.field)
        .find(|candidate| candidate.name == count_field)
        .ok_or(BinarySchemaError::ExternalCountFieldMissingOrNotEarlier {
            schema,
            variant: "record",
            field: field.name,
            count_field,
        })?;
    if count.kind != BinaryFieldKind::U32 || count.cardinality != BinaryCardinality::Required {
        return Err(BinarySchemaError::InvalidExternalCountField {
            schema,
            variant: "record",
            field: field.name,
            count_field,
        });
    }
    Ok(())
}

fn validate_field_reference(
    schema: &'static str,
    variant: &'static str,
    field: BinaryFieldSchema,
) -> Result<(), BinarySchemaError> {
    match (field.kind, field.message_type) {
        (BinaryFieldKind::Message | BinaryFieldKind::Enumeration, Some(message_type)) => {
            validate_name("message type", message_type)?;
        }
        (BinaryFieldKind::Message | BinaryFieldKind::Enumeration, None) => {
            return Err(BinarySchemaError::MissingMessageType {
                schema,
                variant,
                field: field.name,
            });
        }
        (_, Some(message_type)) => {
            return Err(BinarySchemaError::UnexpectedMessageType {
                schema,
                variant,
                field: field.name,
                message_type,
            });
        }
        (_, None) => {}
    }
    if let Some(range) = field.numeric_range {
        let Some((kind_minimum, kind_maximum)) = binary_integer_bounds(field.kind) else {
            return Err(BinarySchemaError::UnexpectedNumericRange {
                schema,
                variant,
                field: field.name,
                kind: field.kind,
            });
        };
        let minimum = range.minimum();
        let maximum = range.maximum();
        let invalid = minimum.is_none() && maximum.is_none()
            || minimum
                .zip(maximum)
                .is_some_and(|(minimum, maximum)| minimum > maximum)
            || minimum.is_some_and(|minimum| minimum < kind_minimum || minimum > kind_maximum)
            || maximum.is_some_and(|maximum| maximum < kind_minimum || maximum > kind_maximum);
        if invalid {
            return Err(BinarySchemaError::InvalidNumericRange {
                schema,
                variant,
                field: field.name,
                kind: field.kind,
                range,
            });
        }
    }
    if let Some(range) = field.length_range {
        if !matches!(field.kind, BinaryFieldKind::Bytes | BinaryFieldKind::Utf8) {
            return Err(BinarySchemaError::UnexpectedLengthRange {
                schema,
                variant,
                field: field.name,
                kind: field.kind,
            });
        }
        if field.cardinality == BinaryCardinality::Repeated || !valid_size_range(range) {
            return Err(BinarySchemaError::InvalidLengthRange {
                schema,
                variant,
                field: field.name,
                range,
            });
        }
    }
    if let Some(range) = field.item_count_range {
        if field.cardinality != BinaryCardinality::Repeated {
            return Err(BinarySchemaError::UnexpectedItemCountRange {
                schema,
                variant,
                field: field.name,
            });
        }
        if !valid_size_range(range) {
            return Err(BinarySchemaError::InvalidItemCountRange {
                schema,
                variant,
                field: field.name,
                range,
            });
        }
    }
    Ok(())
}

fn valid_size_range(range: BinarySizeRange) -> bool {
    (range.minimum().is_some() || range.maximum().is_some())
        && !range
            .minimum()
            .zip(range.maximum())
            .is_some_and(|(minimum, maximum)| minimum > maximum)
}

const fn binary_integer_bounds(kind: BinaryFieldKind) -> Option<(i128, i128)> {
    match kind {
        BinaryFieldKind::U8 => Some((0, u8::MAX as i128)),
        BinaryFieldKind::U16 => Some((0, u16::MAX as i128)),
        BinaryFieldKind::U32 => Some((0, u32::MAX as i128)),
        BinaryFieldKind::U64 => Some((0, u64::MAX as i128)),
        BinaryFieldKind::I32 => Some((i32::MIN as i128, i32::MAX as i128)),
        BinaryFieldKind::I64 => Some((i64::MIN as i128, i64::MAX as i128)),
        BinaryFieldKind::Bytes
        | BinaryFieldKind::Bool
        | BinaryFieldKind::Enumeration
        | BinaryFieldKind::F64
        | BinaryFieldKind::Utf8
        | BinaryFieldKind::Message => None,
    }
}

fn validate_name(kind: &'static str, value: &'static str) -> Result<(), BinarySchemaError> {
    if value.is_empty() {
        return Err(BinarySchemaError::EmptyName { kind });
    }
    if let Some((index, byte)) = value
        .bytes()
        .enumerate()
        .find(|(_, byte)| !byte.is_ascii_graphic())
    {
        return Err(BinarySchemaError::InvalidNameByte {
            kind,
            value,
            index,
            byte,
        });
    }
    Ok(())
}

fn hash_bytes(hash: &mut u64, bytes: &[u8]) {
    for byte in bytes {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100000001b3);
    }
    *hash ^= 0xff;
    *hash = hash.wrapping_mul(0x100000001b3);
}

fn hash_fields(hash: &mut u64, fields: &[BinaryFieldSchema]) {
    for field in fields {
        hash_bytes(hash, field.name.as_bytes());
        hash_bytes(hash, &[field.kind as u8, field.cardinality as u8]);
        if let Some(message_type) = field.message_type {
            hash_bytes(hash, message_type.as_bytes());
        }
        if let Some(range) = field.numeric_range {
            hash_bytes(hash, b"numeric_range");
            if let Some(minimum) = range.minimum() {
                hash_bytes(hash, b"minimum");
                hash_bytes(hash, &minimum.to_be_bytes());
            }
            if let Some(maximum) = range.maximum() {
                hash_bytes(hash, b"maximum");
                hash_bytes(hash, &maximum.to_be_bytes());
            }
        }
        if let Some(range) = field.length_range {
            hash_bytes(hash, b"length_range");
            hash_size_range(hash, range);
        }
        if let Some(range) = field.item_count_range {
            hash_bytes(hash, b"item_count_range");
            hash_size_range(hash, range);
        }
        if let Some(count_field) = field.external_count_field {
            hash_bytes(hash, b"external_count_field");
            hash_bytes(hash, count_field.as_bytes());
        }
    }
}

fn hash_size_range(hash: &mut u64, range: BinarySizeRange) {
    if let Some(minimum) = range.minimum() {
        hash_bytes(hash, b"minimum");
        hash_bytes(hash, &minimum.to_be_bytes());
    }
    if let Some(maximum) = range.maximum() {
        hash_bytes(hash, b"maximum");
        hash_bytes(hash, &maximum.to_be_bytes());
    }
}

fn check_identity_and_version(
    previous_id: &'static str,
    previous_version: ProtocolVersion,
    next_id: &'static str,
    next_version: ProtocolVersion,
) -> Result<(), BinaryCompatibilityError> {
    if previous_id != next_id {
        return Err(BinaryCompatibilityError::SchemaIdChanged {
            previous: previous_id,
            next: next_id,
        });
    }
    if previous_version.major != next_version.major {
        return Err(BinaryCompatibilityError::MajorChanged {
            previous: previous_version.major,
            next: next_version.major,
        });
    }
    if next_version.minor < previous_version.minor {
        return Err(BinaryCompatibilityError::VersionRegressed {
            previous: previous_version,
            next: next_version,
        });
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinarySchemaError {
    EmptyName {
        kind: &'static str,
    },
    InvalidNameByte {
        kind: &'static str,
        value: &'static str,
        index: usize,
        byte: u8,
    },
    NoVariants,
    NonCanonicalVariantOrder {
        previous: u8,
        current: u8,
    },
    NonCanonicalExtensibleFieldTag {
        schema: &'static str,
        previous: Option<u16>,
        current: u16,
    },
    InvalidExtensibleRecordSizeRange {
        schema: &'static str,
        range: BinarySizeRange,
    },
    DuplicateField {
        variant: &'static str,
        field: &'static str,
    },
    DuplicateVariant {
        schema: &'static str,
        variant: &'static str,
    },
    MissingMessageType {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
    },
    UnexpectedMessageType {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
        message_type: &'static str,
    },
    UnexpectedNumericRange {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
        kind: BinaryFieldKind,
    },
    InvalidNumericRange {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
        kind: BinaryFieldKind,
        range: BinaryNumericRange,
    },
    UnexpectedLengthRange {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
        kind: BinaryFieldKind,
    },
    InvalidLengthRange {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
        range: BinarySizeRange,
    },
    UnexpectedItemCountRange {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
    },
    InvalidItemCountRange {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
        range: BinarySizeRange,
    },
    ExternalCountOnNonRepeatedField {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
    },
    ExternalCountFieldMissingOrNotEarlier {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
        count_field: &'static str,
    },
    InvalidExternalCountField {
        schema: &'static str,
        variant: &'static str,
        field: &'static str,
        count_field: &'static str,
    },
}

impl Display for BinarySchemaError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName { kind } => write!(formatter, "binary {kind} name is empty"),
            Self::InvalidNameByte {
                kind,
                value,
                index,
                byte,
            } => write!(
                formatter,
                "binary {kind} name `{value}` has invalid byte 0x{byte:02x} at {index}"
            ),
            Self::NoVariants => formatter.write_str("binary schema has no variants"),
            Self::NonCanonicalVariantOrder { previous, current } => write!(
                formatter,
                "binary variant tag {current} does not follow tag {previous}"
            ),
            Self::NonCanonicalExtensibleFieldTag {
                schema,
                previous,
                current,
            } => match previous {
                Some(previous) => write!(
                    formatter,
                    "extensible record `{schema}` field tag {current} does not follow tag {previous}"
                ),
                None => write!(
                    formatter,
                    "extensible record `{schema}` uses reserved field tag {current}"
                ),
            },
            Self::InvalidExtensibleRecordSizeRange { schema, range } => write!(
                formatter,
                "extensible record `{schema}` requires a valid bounded encoded-size range, got {:?}..={:?}",
                range.minimum(),
                range.maximum()
            ),
            Self::DuplicateField { variant, field } => write!(
                formatter,
                "binary variant `{variant}` repeats field `{field}`"
            ),
            Self::DuplicateVariant { schema, variant } => write!(
                formatter,
                "binary schema `{schema}` repeats variant `{variant}`"
            ),
            Self::MissingMessageType {
                schema,
                variant,
                field,
            } => write!(
                formatter,
                "binary message field `{schema}.{variant}.{field}` has no referenced type"
            ),
            Self::UnexpectedMessageType {
                schema,
                variant,
                field,
                message_type,
            } => write!(
                formatter,
                "non-message field `{schema}.{variant}.{field}` unexpectedly references `{message_type}`"
            ),
            Self::UnexpectedNumericRange {
                schema,
                variant,
                field,
                kind,
            } => write!(
                formatter,
                "non-integer field `{schema}.{variant}.{field}` of kind {kind:?} has a numeric range"
            ),
            Self::InvalidNumericRange {
                schema,
                variant,
                field,
                kind,
                range,
            } => write!(
                formatter,
                "integer field `{schema}.{variant}.{field}` has invalid {kind:?} range {:?}..={:?}",
                range.minimum(),
                range.maximum()
            ),
            Self::UnexpectedLengthRange {
                schema,
                variant,
                field,
                kind,
            } => write!(
                formatter,
                "non-string field `{schema}.{variant}.{field}` of kind {kind:?} has a length range"
            ),
            Self::InvalidLengthRange {
                schema,
                variant,
                field,
                range,
            } => write!(
                formatter,
                "binary field `{schema}.{variant}.{field}` has invalid length range {:?}..={:?}",
                range.minimum(),
                range.maximum()
            ),
            Self::UnexpectedItemCountRange {
                schema,
                variant,
                field,
            } => write!(
                formatter,
                "non-repeated field `{schema}.{variant}.{field}` has an item-count range"
            ),
            Self::InvalidItemCountRange {
                schema,
                variant,
                field,
                range,
            } => write!(
                formatter,
                "binary field `{schema}.{variant}.{field}` has invalid item-count range {:?}..={:?}",
                range.minimum(),
                range.maximum()
            ),
            Self::ExternalCountOnNonRepeatedField {
                schema,
                variant,
                field,
            } => write!(
                formatter,
                "non-repeated field `{schema}.{variant}.{field}` declares an external count"
            ),
            Self::ExternalCountFieldMissingOrNotEarlier {
                schema,
                variant,
                field,
                count_field,
            } => write!(
                formatter,
                "field `{schema}.{variant}.{field}` references missing or non-earlier count field `{count_field}`"
            ),
            Self::InvalidExternalCountField {
                schema,
                variant,
                field,
                count_field,
            } => write!(
                formatter,
                "field `{schema}.{variant}.{field}` requires `{count_field}` to be a required u32 count"
            ),
        }
    }
}

impl Error for BinarySchemaError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BinaryCompatibilityError {
    InvalidPreviousSchema(BinarySchemaError),
    InvalidNextSchema(BinarySchemaError),
    SchemaIdChanged {
        previous: &'static str,
        next: &'static str,
    },
    MajorChanged {
        previous: u16,
        next: u16,
    },
    VersionRegressed {
        previous: ProtocolVersion,
        next: ProtocolVersion,
    },
    PositionalLayoutChanged,
    ExtensibleRecordSizeRangeChanged {
        previous: BinarySizeRange,
        next: BinarySizeRange,
    },
    ExtensibleFieldRemoved {
        tag: u16,
    },
    ExtensibleFieldChanged {
        tag: u16,
    },
    NonOptionalExtensibleFieldAdded {
        tag: u16,
        cardinality: BinaryCardinality,
    },
}

impl Display for BinaryCompatibilityError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPreviousSchema(error) => {
                write!(formatter, "previous binary schema is invalid: {error}")
            }
            Self::InvalidNextSchema(error) => {
                write!(formatter, "next binary schema is invalid: {error}")
            }
            Self::SchemaIdChanged { previous, next } => {
                write!(
                    formatter,
                    "binary schema ID changed from `{previous}` to `{next}`"
                )
            }
            Self::MajorChanged { previous, next } => {
                write!(
                    formatter,
                    "binary schema major changed from {previous} to {next}"
                )
            }
            Self::VersionRegressed { previous, next } => {
                write!(
                    formatter,
                    "binary schema version regressed from {previous} to {next}"
                )
            }
            Self::PositionalLayoutChanged => formatter
                .write_str("binary positional layout changed within one compatible major version"),
            Self::ExtensibleRecordSizeRangeChanged { previous, next } => write!(
                formatter,
                "extensible record encoded-size range changed from {:?}..={:?} to {:?}..={:?}",
                previous.minimum(),
                previous.maximum(),
                next.minimum(),
                next.maximum()
            ),
            Self::ExtensibleFieldRemoved { tag } => {
                write!(formatter, "extensible record field tag {tag} was removed")
            }
            Self::ExtensibleFieldChanged { tag } => write!(
                formatter,
                "extensible record field tag {tag} changed its published contract"
            ),
            Self::NonOptionalExtensibleFieldAdded { tag, cardinality } => write!(
                formatter,
                "new extensible record field tag {tag} has non-optional cardinality {cardinality:?}"
            ),
        }
    }
}

impl Error for BinaryCompatibilityError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidPreviousSchema(error) | Self::InvalidNextSchema(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use pwright_core::ProtocolVersion;

    use super::{
        BinaryCardinality, BinaryCatalogCompatibilityError, BinaryCatalogError,
        BinaryCompatibilityError, BinaryDecodeError, BinaryEncode, BinaryEncodeSequenceRef,
        BinaryEncodeSlice, BinaryEnumSchema, BinaryEnumVariantSchema, BinaryExtensibleFieldSchema,
        BinaryExtensibleRecordSchema, BinaryFieldKind, BinaryFieldSchema, BinaryNumericRange,
        BinaryNumericRangeError, BinaryReader, BinaryRecordSchema, BinarySchema,
        BinarySchemaCatalog, BinarySchemaError, BinarySizeRange, BinarySizeRangeError,
        BinaryTypeCompatibilityError, BinaryTypeKind, BinaryTypeSchema, BinaryValueContext,
        BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits, BinaryValueResource,
        BinaryVariantSchema, BinaryWriter, check_numeric_range, check_size_range, decode_bytes,
        decode_extensible_record, decode_utf8, decode_utf8_constrained, encode_bytes,
        encode_extensible_record, encode_repeated_len, encode_utf8, encode_utf8_constrained,
    };

    const FIELDS: &[BinaryFieldSchema] = &[
        BinaryFieldSchema::new(
            "object_id",
            BinaryFieldKind::U64,
            BinaryCardinality::Required,
        ),
        BinaryFieldSchema::new("enabled", BinaryFieldKind::U8, BinaryCardinality::Required),
    ];
    const VARIANTS: &[BinaryVariantSchema] = &[BinaryVariantSchema::new(1, "set", FIELDS)];

    fn schema(minor: u16) -> BinarySchema {
        BinarySchema::new("test.control", ProtocolVersion::new(1, minor), VARIANTS)
    }

    #[test]
    fn writer_and_reader_round_trip_network_order_primitives() {
        // Arrange
        let mut writer = BinaryWriter::try_with_capacity(32).expect("writer should allocate");

        // Act
        writer.u8(1).expect("u8 should encode");
        writer.u16(0x0203).expect("u16 should encode");
        writer.u32(0x0405_0607).expect("u32 should encode");
        writer.i32(-1).expect("i32 should encode");
        writer
            .u64(0x0809_0a0b_0c0d_0e0f)
            .expect("u64 should encode");
        writer.i64(-2).expect("i64 should encode");
        writer.f64(-12.5).expect("f64 should encode");
        let bytes = writer.into_bytes();
        let mut reader = BinaryReader::new(&bytes);
        let decoded = (
            reader.u8(),
            reader.u16(),
            reader.u32(),
            reader.i32(),
            reader.u64(),
            reader.i64(),
            reader.f64().map(f64::to_bits),
            reader.finish(),
        );

        // Assert
        assert_eq!(
            decoded,
            (
                Ok(1),
                Ok(0x0203),
                Ok(0x0405_0607),
                Ok(-1),
                Ok(0x0809_0a0b_0c0d_0e0f),
                Ok(-2),
                Ok((-12.5_f64).to_bits()),
                Ok(())
            )
        );
    }

    #[test]
    fn extensible_record_round_trips_known_fields_and_skips_unknown_fields() {
        // Arrange
        let range = BinarySizeRange::new(Some(0), Some(64));
        let mut writer = BinaryWriter::new();
        encode_extensible_record(&mut writer, "test.options", range, |record| {
            record.field(1, "enabled", |writer| {
                writer.u8(1)?;
                Ok(())
            })?;
            record.field(9, "future", |_| Ok(()))?;
            record.field(12, "count", |writer| {
                writer.u16(0x0203)?;
                Ok(())
            })?;
            Ok(())
        })
        .expect("record should encode");
        let bytes = writer.into_bytes();
        let mut reader = BinaryReader::new(&bytes);

        // Act
        let mut record = decode_extensible_record(&mut reader, "test.options", range)
            .expect("record should open");
        let mut enabled = record
            .next_field()
            .expect("first field should parse")
            .expect("first field should exist");
        let enabled_value = enabled.reader().u8().expect("enabled should decode");
        enabled.finish().expect("enabled body should be exact");
        let future = record
            .next_field()
            .expect("future field should parse")
            .expect("future field should exist");
        let future_tag = future.tag();
        let mut count = record
            .next_field()
            .expect("count field should parse")
            .expect("count field should exist");
        let count_value = count.reader().u16().expect("count should decode");
        count.finish().expect("count body should be exact");
        let end = record.next_field();
        let record_end = record.finish();
        let payload_end = reader.finish();

        // Assert
        assert_eq!(
            bytes,
            [
                0, 0, 0, 21, // record length
                0, 1, 0, 0, 0, 1, 1, // tag 1, one-byte body
                0, 9, 0, 0, 0, 0, // tag 9, present empty body
                0, 12, 0, 0, 0, 2, 2, 3, // tag 12, two-byte body
            ]
        );
        assert_eq!(enabled_value, 1);
        assert_eq!(future_tag, 9);
        assert_eq!(count_value, 0x0203);
        assert_eq!(end, Ok(None));
        assert_eq!(record_end, Ok(()));
        assert_eq!(payload_end, Ok(()));
    }

    #[test]
    fn extensible_record_distinguishes_absent_from_present_empty() {
        // Arrange
        let range = BinarySizeRange::new(Some(0), Some(16));
        let mut absent = BinaryWriter::new();
        let mut present = BinaryWriter::new();

        // Act
        encode_extensible_record(&mut absent, "test.options", range, |_| Ok(()))
            .expect("empty record should encode");
        encode_extensible_record(&mut present, "test.options", range, |record| {
            record.field(1, "items", |_| Ok(()))
        })
        .expect("present empty field should encode");

        // Assert
        assert_eq!(absent.into_bytes(), [0, 0, 0, 0]);
        assert_eq!(present.into_bytes(), [0, 0, 0, 6, 0, 1, 0, 0, 0, 0]);
    }

    #[test]
    fn extensible_record_rejects_reserved_duplicate_and_descending_tags() {
        // Arrange
        let range = BinarySizeRange::new(Some(0), Some(32));
        let encode_with_tags = |tags: &[u16]| {
            let mut writer = BinaryWriter::new();
            encode_extensible_record(&mut writer, "test.options", range, |record| {
                for tag in tags {
                    record.field(*tag, "value", |_| Ok(()))?;
                }
                Ok(())
            })
        };
        let encoded_zero = [0, 0, 0, 6, 0, 0, 0, 0, 0, 0];
        let encoded_duplicate = [0, 0, 0, 12, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0];
        let encoded_descending = [0, 0, 0, 12, 0, 2, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0];
        let decode_tags = |bytes: &[u8]| {
            let mut reader = BinaryReader::new(bytes);
            let mut record = decode_extensible_record(&mut reader, "test.options", range)?;
            while record.next_field()?.is_some() {}
            Ok::<(), BinaryValueDecodeError>(())
        };

        // Act
        let encode_zero = encode_with_tags(&[0]);
        let encode_duplicate = encode_with_tags(&[1, 1]);
        let encode_descending = encode_with_tags(&[2, 1]);
        let decode_zero = decode_tags(&encoded_zero);
        let decode_duplicate = decode_tags(&encoded_duplicate);
        let decode_descending = decode_tags(&encoded_descending);

        // Assert
        assert!(matches!(
            encode_zero,
            Err(BinaryValueEncodeError::NonCanonicalExtensibleFieldTag {
                previous: None,
                current: 0,
                ..
            })
        ));
        for error in [encode_duplicate, encode_descending] {
            assert!(matches!(
                error,
                Err(BinaryValueEncodeError::NonCanonicalExtensibleFieldTag {
                    previous: Some(_),
                    ..
                })
            ));
        }
        assert!(matches!(
            decode_zero,
            Err(BinaryValueDecodeError::NonCanonicalExtensibleFieldTag {
                previous: None,
                current: 0,
                ..
            })
        ));
        for error in [decode_duplicate, decode_descending] {
            assert!(matches!(
                error,
                Err(BinaryValueDecodeError::NonCanonicalExtensibleFieldTag {
                    previous: Some(_),
                    ..
                })
            ));
        }
    }

    #[test]
    fn extensible_record_checks_bounds_before_body_and_exact_known_field_consumption() {
        // Arrange
        let range = BinarySizeRange::new(Some(0), Some(8));
        let oversized_payload = [0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let mut oversized_reader = BinaryReader::new(&oversized_payload);
        let trailing_known_field = [0, 0, 0, 8, 0, 1, 0, 0, 0, 2, 7, 8];
        let mut trailing_reader = BinaryReader::new(&trailing_known_field);

        // Act
        let oversized = decode_extensible_record(&mut oversized_reader, "test.options", range);
        let mut record = decode_extensible_record(&mut trailing_reader, "test.options", range)
            .expect("bounded record should open");
        let mut field = record
            .next_field()
            .expect("field should parse")
            .expect("field should exist");
        let value = field.reader().u8().expect("known prefix should decode");
        let trailing = field.finish();

        // Assert
        assert!(matches!(
            oversized,
            Err(BinaryValueDecodeError::SizeRange(BinarySizeRangeError {
                actual: 9,
                ..
            }))
        ));
        assert_eq!(oversized_reader.remaining(), 9, "body was never consumed");
        assert_eq!(value, 7);
        assert_eq!(
            trailing,
            Err(BinaryValueDecodeError::Primitive(
                BinaryDecodeError::TrailingBytes { remaining: 1 }
            ))
        );
    }

    #[test]
    fn reader_reports_exact_truncation_and_trailing_boundaries() {
        // Arrange
        let mut truncated = BinaryReader::new(&[1, 2, 3]);
        let trailing = BinaryReader::new(&[1]);

        // Act
        let truncation = truncated.u64();
        let remainder = trailing.finish();

        // Assert
        assert_eq!(
            truncation,
            Err(BinaryDecodeError::Truncated {
                offset: 0,
                needed: 8,
                remaining: 3,
            })
        );
        assert_eq!(
            remainder,
            Err(BinaryDecodeError::TrailingBytes { remaining: 1 })
        );
    }

    #[test]
    fn bounded_value_helpers_round_trip_utf8_and_binary() {
        // Arrange
        let limits = BinaryValueLimits::new(8, 4, 0, 0, 0);
        let context = BinaryValueContext::new(limits);
        let mut writer = BinaryWriter::new();

        // Act
        encode_utf8(&mut writer, &context, "name", "hello").expect("UTF-8 should encode");
        encode_bytes(&mut writer, &context, "body", &[1, 2, 3]).expect("bytes should encode");
        let bytes = writer.into_bytes();
        let mut reader = BinaryReader::new(&bytes);
        let name = decode_utf8(&mut reader, &context, "name").expect("UTF-8 should decode");
        let body = decode_bytes(&mut reader, &context, "body").expect("bytes should decode");

        // Assert
        assert_eq!(name, "hello");
        assert_eq!(body, [1, 2, 3]);
        assert_eq!(reader.finish(), Ok(()));
    }

    #[test]
    fn bounded_value_length_is_checked_before_declared_body() {
        // Arrange
        let encoded = 5_u32.to_be_bytes();
        let mut reader = BinaryReader::new(&encoded);
        let context = BinaryValueContext::new(BinaryValueLimits::new(0, 4, 0, 0, 0));

        // Act
        let error = decode_bytes(&mut reader, &context, "body")
            .expect_err("declared bytes above the limit must fail first");

        // Assert
        assert_eq!(
            error,
            BinaryValueDecodeError::Limit(super::BinaryValueLimitError {
                resource: BinaryValueResource::BinaryBytes,
                field: "body",
                actual: 5,
                limit: 4,
            })
        );
    }

    #[test]
    fn borrowed_sequence_encodes_a_slice_without_collecting_items() {
        // Arrange
        struct Byte(u8);
        impl BinaryEncode for Byte {
            fn encode_binary(
                &self,
                writer: &mut BinaryWriter,
                _context: &mut BinaryValueContext,
            ) -> Result<(), BinaryValueEncodeError> {
                writer.u8(self.0)?;
                Ok(())
            }
        }
        let values = [Byte(7), Byte(11), Byte(13)];
        let sequence = BinaryEncodeSlice::new(&values);
        let sequence = BinaryEncodeSequenceRef::new(&sequence);
        let mut writer = BinaryWriter::new();
        let mut context = BinaryValueContext::new(BinaryValueLimits::new(0, 0, 3, 0, 0));

        // Act
        encode_repeated_len(&mut writer, &context, "values", sequence.len())
            .expect("count should encode");
        sequence
            .encode_items(&mut writer, &mut context)
            .expect("borrowed values should encode");

        // Assert
        assert_eq!(writer.into_bytes(), [0, 0, 0, 3, 7, 11, 13]);
    }

    #[test]
    fn numeric_range_accepts_boundaries_and_rejects_values_outside_them() {
        // Arrange
        let range = BinaryNumericRange::new(Some(-2), Some(7));

        // Act
        let minimum = check_numeric_range("value", -2, range);
        let maximum = check_numeric_range("value", 7, range);
        let below = check_numeric_range("value", -3, range);
        let above = check_numeric_range("value", 8, range);

        // Assert
        assert_eq!(minimum, Ok(()));
        assert_eq!(maximum, Ok(()));
        assert_eq!(
            below,
            Err(BinaryNumericRangeError {
                field: "value",
                value: -3,
                range,
            })
        );
        assert_eq!(
            above,
            Err(BinaryNumericRangeError {
                field: "value",
                value: 8,
                range,
            })
        );
    }

    #[test]
    fn schema_rejects_numeric_ranges_on_non_integer_fields() {
        // Arrange
        const TEXT_FIELDS: &[BinaryFieldSchema] =
            &[
                BinaryFieldSchema::new("value", BinaryFieldKind::Utf8, BinaryCardinality::Required)
                    .with_numeric_range(BinaryNumericRange::new(None, Some(4))),
            ];
        let text = BinaryRecordSchema::new("test.text", ProtocolVersion::new(1, 0), TEXT_FIELDS);

        // Act
        let text_error = text.validate().expect_err("UTF-8 range must fail");

        // Assert
        assert!(matches!(
            text_error,
            BinarySchemaError::UnexpectedNumericRange {
                field: "value",
                kind: BinaryFieldKind::Utf8,
                ..
            }
        ));
    }

    #[test]
    fn size_range_rejects_values_outside_both_bounds() {
        // Arrange
        let range = BinarySizeRange::new(Some(2), Some(4));

        // Act
        let below = check_size_range("value", 1, range);
        let above = check_size_range("value", 5, range);

        // Assert
        assert_eq!(
            below,
            Err(BinarySizeRangeError {
                field: "value",
                actual: 1,
                range,
            })
        );
        assert_eq!(
            above,
            Err(BinarySizeRangeError {
                field: "value",
                actual: 5,
                range,
            })
        );
    }

    #[test]
    fn constrained_utf8_checks_declared_size_on_encode_and_before_decode_copy() {
        // Arrange
        let limits = BinaryValueLimits::new(16, 16, 16, 16, 16);
        let range = BinarySizeRange::new(Some(2), Some(4));
        let mut writer = BinaryWriter::new();
        let context = BinaryValueContext::new(limits);

        // Act
        let encode_error = encode_utf8_constrained(&mut writer, &context, "value", "x", range)
            .expect_err("short UTF-8 must fail");
        let mut reader = BinaryReader::new(&[0, 0, 0, 1, b'x']);
        let decode_error = decode_utf8_constrained(&mut reader, &context, "value", range)
            .expect_err("short encoded UTF-8 must fail");

        // Assert
        assert!(matches!(encode_error, BinaryValueEncodeError::SizeRange(_)));
        assert!(matches!(decode_error, BinaryValueDecodeError::SizeRange(_)));
        assert_eq!(reader.remaining(), 1, "payload was not copied or consumed");
    }

    #[test]
    fn field_size_ranges_participate_in_validation_and_layout_identity() {
        // Arrange
        const PREVIOUS_FIELDS: &[BinaryFieldSchema] = &[
            BinaryFieldSchema::new("name", BinaryFieldKind::Utf8, BinaryCardinality::Required)
                .with_length_range(BinarySizeRange::new(Some(1), Some(8))),
            BinaryFieldSchema::message("children", "test.node", BinaryCardinality::Repeated)
                .with_item_count_range(BinarySizeRange::new(None, Some(4))),
        ];
        const NEXT_FIELDS: &[BinaryFieldSchema] = &[
            BinaryFieldSchema::new("name", BinaryFieldKind::Utf8, BinaryCardinality::Required)
                .with_length_range(BinarySizeRange::new(Some(1), Some(9))),
            BinaryFieldSchema::message("children", "test.node", BinaryCardinality::Repeated)
                .with_item_count_range(BinarySizeRange::new(None, Some(4))),
        ];
        let previous =
            BinaryRecordSchema::new("test.node", ProtocolVersion::new(1, 0), PREVIOUS_FIELDS);
        let next = BinaryRecordSchema::new("test.node", ProtocolVersion::new(1, 1), NEXT_FIELDS);

        // Act
        let validation = previous.validate();
        let compatibility = previous.check_minor_compatibility(next);

        // Assert
        assert_eq!(validation, Ok(()));
        assert_eq!(
            compatibility,
            Err(BinaryCompatibilityError::PositionalLayoutChanged)
        );
        assert_ne!(previous.layout_fingerprint(), next.layout_fingerprint());
    }

    #[test]
    fn external_repeated_count_is_validated_and_participates_in_layout_identity() {
        // Arrange
        const EXTERNAL_FIELDS: &[BinaryFieldSchema] = &[
            BinaryFieldSchema::new(
                "item_count",
                BinaryFieldKind::U32,
                BinaryCardinality::Required,
            ),
            BinaryFieldSchema::new("items", BinaryFieldKind::U16, BinaryCardinality::Repeated)
                .with_external_count("item_count"),
        ];
        const INLINE_FIELDS: &[BinaryFieldSchema] = &[
            BinaryFieldSchema::new(
                "item_count",
                BinaryFieldKind::U32,
                BinaryCardinality::Required,
            ),
            BinaryFieldSchema::new("items", BinaryFieldKind::U16, BinaryCardinality::Repeated),
        ];
        const EXTERNAL_VARIANTS: &[BinaryVariantSchema] =
            &[BinaryVariantSchema::new(1, "items", EXTERNAL_FIELDS)];
        const INLINE_VARIANTS: &[BinaryVariantSchema] =
            &[BinaryVariantSchema::new(1, "items", INLINE_FIELDS)];
        let external = BinarySchema::new(
            "test.external_count",
            ProtocolVersion::new(1, 0),
            EXTERNAL_VARIANTS,
        );
        let inline = BinarySchema::new(
            "test.external_count",
            ProtocolVersion::new(1, 0),
            INLINE_VARIANTS,
        );

        // Act
        let validation = external.validate();
        let external_fingerprint = external.layout_fingerprint();
        let inline_fingerprint = inline.layout_fingerprint();

        // Assert
        assert_eq!(validation, Ok(()));
        assert_eq!(
            EXTERNAL_FIELDS[1].external_count_field(),
            Some("item_count")
        );
        assert_ne!(external_fingerprint, inline_fingerprint);
    }

    #[test]
    fn external_repeated_count_requires_an_earlier_required_u32_field() {
        // Arrange
        const NON_REPEATED_FIELDS: &[BinaryFieldSchema] = &[
            BinaryFieldSchema::new(
                "item_count",
                BinaryFieldKind::U32,
                BinaryCardinality::Required,
            ),
            BinaryFieldSchema::new("item", BinaryFieldKind::U16, BinaryCardinality::Required)
                .with_external_count("item_count"),
        ];
        const NOT_EARLIER_FIELDS: &[BinaryFieldSchema] = &[
            BinaryFieldSchema::new("items", BinaryFieldKind::U16, BinaryCardinality::Repeated)
                .with_external_count("item_count"),
            BinaryFieldSchema::new(
                "item_count",
                BinaryFieldKind::U32,
                BinaryCardinality::Required,
            ),
        ];
        const WRONG_TYPE_FIELDS: &[BinaryFieldSchema] = &[
            BinaryFieldSchema::new(
                "item_count",
                BinaryFieldKind::U16,
                BinaryCardinality::Required,
            ),
            BinaryFieldSchema::new("items", BinaryFieldKind::U16, BinaryCardinality::Repeated)
                .with_external_count("item_count"),
        ];
        let non_repeated = BinaryRecordSchema::new(
            "test.non_repeated",
            ProtocolVersion::new(1, 0),
            NON_REPEATED_FIELDS,
        );
        let not_earlier = BinaryRecordSchema::new(
            "test.not_earlier",
            ProtocolVersion::new(1, 0),
            NOT_EARLIER_FIELDS,
        );
        let wrong_type = BinaryRecordSchema::new(
            "test.wrong_type",
            ProtocolVersion::new(1, 0),
            WRONG_TYPE_FIELDS,
        );

        // Act
        let non_repeated_error = non_repeated.validate();
        let not_earlier_error = not_earlier.validate();
        let wrong_type_error = wrong_type.validate();

        // Assert
        assert!(matches!(
            non_repeated_error,
            Err(BinarySchemaError::ExternalCountOnNonRepeatedField { field: "item", .. })
        ));
        assert!(matches!(
            not_earlier_error,
            Err(BinarySchemaError::ExternalCountFieldMissingOrNotEarlier {
                field: "items",
                count_field: "item_count",
                ..
            })
        ));
        assert!(matches!(
            wrong_type_error,
            Err(BinarySchemaError::InvalidExternalCountField {
                field: "items",
                count_field: "item_count",
                ..
            })
        ));
    }

    #[test]
    fn schema_rejects_size_ranges_on_incompatible_field_shapes() {
        // Arrange
        const INTEGER_FIELDS: &[BinaryFieldSchema] =
            &[
                BinaryFieldSchema::new("value", BinaryFieldKind::U64, BinaryCardinality::Required)
                    .with_length_range(BinarySizeRange::new(None, Some(8))),
            ];
        const SCALAR_FIELDS: &[BinaryFieldSchema] =
            &[
                BinaryFieldSchema::new("value", BinaryFieldKind::Utf8, BinaryCardinality::Required)
                    .with_item_count_range(BinarySizeRange::new(None, Some(8))),
            ];
        let integer =
            BinaryRecordSchema::new("test.integer", ProtocolVersion::new(1, 0), INTEGER_FIELDS);
        let scalar =
            BinaryRecordSchema::new("test.scalar", ProtocolVersion::new(1, 0), SCALAR_FIELDS);

        // Act
        let length_error = integer
            .validate()
            .expect_err("integer length range must fail");
        let item_error = scalar.validate().expect_err("scalar item range must fail");

        // Assert
        assert!(matches!(
            length_error,
            BinarySchemaError::UnexpectedLengthRange {
                field: "value",
                kind: BinaryFieldKind::U64,
                ..
            }
        ));
        assert!(matches!(
            item_error,
            BinarySchemaError::UnexpectedItemCountRange { field: "value", .. }
        ));
    }

    #[test]
    fn schema_numeric_ranges_must_fit_the_wire_integer_kind() {
        // Arrange
        const BYTE_FIELDS: &[BinaryFieldSchema] =
            &[
                BinaryFieldSchema::new("value", BinaryFieldKind::U8, BinaryCardinality::Required)
                    .with_numeric_range(BinaryNumericRange::new(None, Some(256))),
            ];
        let byte = BinaryRecordSchema::new("test.byte", ProtocolVersion::new(1, 0), BYTE_FIELDS);

        // Act
        let error = byte.validate().expect_err("u8 range must fit u8");

        // Assert
        assert!(matches!(
            error,
            BinarySchemaError::InvalidNumericRange {
                field: "value",
                kind: BinaryFieldKind::U8,
                ..
            }
        ));
    }

    #[test]
    fn numeric_range_participates_in_fingerprint_and_minor_compatibility() {
        // Arrange
        const PREVIOUS_FIELDS: &[BinaryFieldSchema] =
            &[
                BinaryFieldSchema::new("value", BinaryFieldKind::U32, BinaryCardinality::Required)
                    .with_numeric_range(BinaryNumericRange::new(Some(0), Some(9))),
            ];
        const NEXT_FIELDS: &[BinaryFieldSchema] =
            &[
                BinaryFieldSchema::new("value", BinaryFieldKind::U32, BinaryCardinality::Required)
                    .with_numeric_range(BinaryNumericRange::new(Some(0), Some(10))),
            ];
        let previous = BinaryRecordSchema::new(
            "test.constrained",
            ProtocolVersion::new(1, 0),
            PREVIOUS_FIELDS,
        );
        let next =
            BinaryRecordSchema::new("test.constrained", ProtocolVersion::new(1, 1), NEXT_FIELDS);

        // Act
        let compatibility = previous.check_minor_compatibility(next);

        // Assert
        assert_eq!(
            compatibility,
            Err(BinaryCompatibilityError::PositionalLayoutChanged)
        );
        assert_ne!(previous.layout_fingerprint(), next.layout_fingerprint());
    }

    #[test]
    fn schema_validation_rejects_noncanonical_variant_order() {
        // Arrange
        const REVERSED: &[BinaryVariantSchema] = &[
            BinaryVariantSchema::new(2, "second", &[]),
            BinaryVariantSchema::new(1, "first", &[]),
        ];
        let schema = BinarySchema::new("test.order", ProtocolVersion::new(1, 0), REVERSED);

        // Act
        let error = schema.validate().expect_err("reversed tags must fail");

        // Assert
        assert_eq!(
            error,
            BinarySchemaError::NonCanonicalVariantOrder {
                previous: 2,
                current: 1,
            }
        );
    }

    #[test]
    fn unchanged_layout_is_compatible_across_minor_versions() {
        // Arrange
        let previous = schema(0);
        let next = schema(3);

        // Act
        let compatibility = previous.check_minor_compatibility(next);

        // Assert
        assert_eq!(compatibility, Ok(()));
        assert_eq!(previous.layout_fingerprint(), next.layout_fingerprint());
    }

    #[test]
    fn adding_a_positional_field_requires_a_new_major() {
        // Arrange
        const NEXT_FIELDS: &[BinaryFieldSchema] = &[
            BinaryFieldSchema::new(
                "object_id",
                BinaryFieldKind::U64,
                BinaryCardinality::Required,
            ),
            BinaryFieldSchema::new("enabled", BinaryFieldKind::U8, BinaryCardinality::Required),
            BinaryFieldSchema::new("reason", BinaryFieldKind::Utf8, BinaryCardinality::Optional),
        ];
        const NEXT_VARIANTS: &[BinaryVariantSchema] =
            &[BinaryVariantSchema::new(1, "set", NEXT_FIELDS)];
        let next = BinarySchema::new("test.control", ProtocolVersion::new(1, 1), NEXT_VARIANTS);

        // Act
        let error = schema(0)
            .check_minor_compatibility(next)
            .expect_err("positional extension must fail within a major");

        // Assert
        assert_eq!(error, BinaryCompatibilityError::PositionalLayoutChanged);
    }

    #[test]
    fn extensible_record_allows_a_new_optional_tag_within_a_minor() {
        // Arrange
        const RANGE: BinarySizeRange = BinarySizeRange::new(Some(0), Some(256));
        const PREVIOUS_FIELDS: &[BinaryExtensibleFieldSchema] = &[
            BinaryExtensibleFieldSchema::new(
                1,
                BinaryFieldSchema::new("force", BinaryFieldKind::Bool, BinaryCardinality::Optional),
            ),
            BinaryExtensibleFieldSchema::new(
                10,
                BinaryFieldSchema::new(
                    "timeout_ms",
                    BinaryFieldKind::U64,
                    BinaryCardinality::Optional,
                ),
            ),
        ];
        const NEXT_FIELDS: &[BinaryExtensibleFieldSchema] = &[
            PREVIOUS_FIELDS[0],
            BinaryExtensibleFieldSchema::new(
                5,
                BinaryFieldSchema::new("trial", BinaryFieldKind::Bool, BinaryCardinality::Optional),
            ),
            PREVIOUS_FIELDS[1],
        ];
        let previous = BinaryExtensibleRecordSchema::new(
            "test.click_options",
            ProtocolVersion::new(1, 0),
            RANGE,
            PREVIOUS_FIELDS,
        );
        let next = BinaryExtensibleRecordSchema::new(
            "test.click_options",
            ProtocolVersion::new(1, 1),
            RANGE,
            NEXT_FIELDS,
        );

        // Act
        let validation = previous.validate();
        let compatibility = previous.check_minor_compatibility(next);
        let kind = BinaryTypeSchema::ExtensibleRecord(previous).kind();

        // Assert
        assert_eq!(validation, Ok(()));
        assert_eq!(compatibility, Ok(()));
        assert_eq!(kind, BinaryTypeKind::ExtensibleRecord);
        assert_ne!(previous.layout_fingerprint(), next.layout_fingerprint());
    }

    #[test]
    fn extensible_record_rejects_unbounded_or_noncanonical_schema_tags() {
        // Arrange
        const ZERO_FIELDS: &[BinaryExtensibleFieldSchema] =
            &[BinaryExtensibleFieldSchema::new(0, FIELDS[0])];
        const REVERSED_FIELDS: &[BinaryExtensibleFieldSchema] = &[
            BinaryExtensibleFieldSchema::new(2, FIELDS[0]),
            BinaryExtensibleFieldSchema::new(1, FIELDS[1]),
        ];
        let unbounded = BinaryExtensibleRecordSchema::new(
            "test.options",
            ProtocolVersion::new(1, 0),
            BinarySizeRange::new(Some(0), None),
            &[],
        );
        let zero = BinaryExtensibleRecordSchema::new(
            "test.options",
            ProtocolVersion::new(1, 0),
            BinarySizeRange::new(Some(0), Some(64)),
            ZERO_FIELDS,
        );
        let reversed = BinaryExtensibleRecordSchema::new(
            "test.options",
            ProtocolVersion::new(1, 0),
            BinarySizeRange::new(Some(0), Some(64)),
            REVERSED_FIELDS,
        );

        // Act
        let unbounded_error = unbounded.validate();
        let zero_error = zero.validate();
        let reversed_error = reversed.validate();

        // Assert
        assert!(matches!(
            unbounded_error,
            Err(BinarySchemaError::InvalidExtensibleRecordSizeRange { .. })
        ));
        assert!(matches!(
            zero_error,
            Err(BinarySchemaError::NonCanonicalExtensibleFieldTag {
                previous: None,
                current: 0,
                ..
            })
        ));
        assert!(matches!(
            reversed_error,
            Err(BinarySchemaError::NonCanonicalExtensibleFieldTag {
                previous: Some(2),
                current: 1,
                ..
            })
        ));
    }

    #[test]
    fn extensible_record_rejects_destructive_minor_changes() {
        // Arrange
        const RANGE: BinarySizeRange = BinarySizeRange::new(Some(0), Some(256));
        const PREVIOUS_FIELDS: &[BinaryExtensibleFieldSchema] =
            &[BinaryExtensibleFieldSchema::new(
                1,
                BinaryFieldSchema::new("force", BinaryFieldKind::Bool, BinaryCardinality::Optional),
            )];
        const CHANGED_FIELDS: &[BinaryExtensibleFieldSchema] = &[BinaryExtensibleFieldSchema::new(
            1,
            BinaryFieldSchema::new("force", BinaryFieldKind::U8, BinaryCardinality::Optional),
        )];
        const REQUIRED_ADDITION: &[BinaryExtensibleFieldSchema] = &[
            PREVIOUS_FIELDS[0],
            BinaryExtensibleFieldSchema::new(
                2,
                BinaryFieldSchema::new(
                    "delay_ms",
                    BinaryFieldKind::U64,
                    BinaryCardinality::Required,
                ),
            ),
        ];
        let previous = BinaryExtensibleRecordSchema::new(
            "test.options",
            ProtocolVersion::new(1, 0),
            RANGE,
            PREVIOUS_FIELDS,
        );
        let schema = |range, fields| {
            BinaryExtensibleRecordSchema::new(
                "test.options",
                ProtocolVersion::new(1, 1),
                range,
                fields,
            )
        };

        // Act
        let removed = previous.check_minor_compatibility(schema(RANGE, &[]));
        let changed = previous.check_minor_compatibility(schema(RANGE, CHANGED_FIELDS));
        let required = previous.check_minor_compatibility(schema(RANGE, REQUIRED_ADDITION));
        let resized = previous.check_minor_compatibility(schema(
            BinarySizeRange::new(Some(0), Some(512)),
            PREVIOUS_FIELDS,
        ));

        // Assert
        assert_eq!(
            removed,
            Err(BinaryCompatibilityError::ExtensibleFieldRemoved { tag: 1 })
        );
        assert_eq!(
            changed,
            Err(BinaryCompatibilityError::ExtensibleFieldChanged { tag: 1 })
        );
        assert_eq!(
            required,
            Err(BinaryCompatibilityError::NonOptionalExtensibleFieldAdded {
                tag: 2,
                cardinality: BinaryCardinality::Required,
            })
        );
        assert_eq!(
            resized,
            Err(BinaryCompatibilityError::ExtensibleRecordSizeRangeChanged {
                previous: RANGE,
                next: BinarySizeRange::new(Some(0), Some(512)),
            })
        );
    }

    #[test]
    fn appending_a_higher_tagged_variant_is_minor_compatible() {
        // Arrange
        const NEXT_VARIANTS: &[BinaryVariantSchema] = &[
            BinaryVariantSchema::new(1, "set", FIELDS),
            BinaryVariantSchema::new(2, "get", &[]),
        ];
        let next = BinarySchema::new("test.control", ProtocolVersion::new(1, 1), NEXT_VARIANTS);

        // Act
        let compatibility = schema(0).check_minor_compatibility(next);

        // Assert
        assert_eq!(compatibility, Ok(()));
    }

    #[test]
    fn schema_fingerprint_is_deterministic_and_layout_sensitive() {
        // Arrange
        const OTHER_FIELDS: &[BinaryFieldSchema] = &[BinaryFieldSchema::new(
            "object_id",
            BinaryFieldKind::U32,
            BinaryCardinality::Required,
        )];
        const OTHER_VARIANTS: &[BinaryVariantSchema] =
            &[BinaryVariantSchema::new(1, "set", OTHER_FIELDS)];
        let other = BinarySchema::new("test.control", ProtocolVersion::new(1, 0), OTHER_VARIANTS);

        // Act
        let first = schema(0).layout_fingerprint();
        let repeated = schema(0).layout_fingerprint();
        let changed = other.layout_fingerprint();

        // Assert
        assert_eq!(first, repeated);
        assert_ne!(first, changed);
    }

    #[test]
    fn catalog_accepts_a_recursive_typed_message_reference() {
        // Arrange
        const NODE_FIELDS: &[BinaryFieldSchema] = &[BinaryFieldSchema::message(
            "next",
            "test.node",
            BinaryCardinality::Optional,
        )];
        const TYPES: &[BinaryTypeSchema] = &[BinaryTypeSchema::Record(BinaryRecordSchema::new(
            "test.node",
            ProtocolVersion::new(1, 0),
            NODE_FIELDS,
        ))];
        let catalog = BinarySchemaCatalog::new(TYPES);

        // Act
        let validation = catalog.validate();

        // Assert
        assert_eq!(validation, Ok(()));
    }

    #[test]
    fn catalog_rejects_an_unknown_nested_message_type() {
        // Arrange
        const ROOT_FIELDS: &[BinaryFieldSchema] = &[BinaryFieldSchema::message(
            "child",
            "test.missing",
            BinaryCardinality::Required,
        )];
        const TYPES: &[BinaryTypeSchema] = &[BinaryTypeSchema::Record(BinaryRecordSchema::new(
            "test.root",
            ProtocolVersion::new(1, 0),
            ROOT_FIELDS,
        ))];
        let catalog = BinarySchemaCatalog::new(TYPES);

        // Act
        let error = catalog
            .validate()
            .expect_err("unknown message reference must fail");

        // Assert
        assert_eq!(
            error,
            BinaryCatalogError::UnknownMessageType {
                schema: "test.root",
                field: "child",
                referenced: "test.missing",
            }
        );
    }

    #[test]
    fn catalog_validates_enumeration_references_and_kinds() {
        // Arrange
        const ENUM_VARIANTS: &[BinaryEnumVariantSchema] = &[
            BinaryEnumVariantSchema::new(0, "first"),
            BinaryEnumVariantSchema::new(1, "second"),
        ];
        const ROOT_FIELDS: &[BinaryFieldSchema] = &[BinaryFieldSchema::enumeration(
            "choice",
            "test.choice",
            BinaryCardinality::Required,
        )];
        const TYPES: &[BinaryTypeSchema] = &[
            BinaryTypeSchema::Enumeration(BinaryEnumSchema::new(
                "test.choice",
                ProtocolVersion::new(1, 0),
                ENUM_VARIANTS,
            )),
            BinaryTypeSchema::Record(BinaryRecordSchema::new(
                "test.root",
                ProtocolVersion::new(1, 0),
                ROOT_FIELDS,
            )),
        ];
        let catalog = BinarySchemaCatalog::new(TYPES);

        // Act
        let validation = catalog.validate();

        // Assert
        assert_eq!(validation, Ok(()));
    }

    #[test]
    fn catalog_rejects_enumeration_reference_to_a_record() {
        // Arrange
        const ROOT_FIELDS: &[BinaryFieldSchema] = &[BinaryFieldSchema::enumeration(
            "choice",
            "test.record",
            BinaryCardinality::Required,
        )];
        const TYPES: &[BinaryTypeSchema] = &[
            BinaryTypeSchema::Record(BinaryRecordSchema::new(
                "test.record",
                ProtocolVersion::new(1, 0),
                &[],
            )),
            BinaryTypeSchema::Record(BinaryRecordSchema::new(
                "test.root",
                ProtocolVersion::new(1, 0),
                ROOT_FIELDS,
            )),
        ];
        let catalog = BinarySchemaCatalog::new(TYPES);

        // Act
        let error = catalog
            .validate()
            .expect_err("enumeration reference to a record must fail");

        // Assert
        assert_eq!(
            error,
            BinaryCatalogError::WrongReferencedTypeKind {
                schema: "test.root",
                field: "choice",
                referenced: "test.record",
                actual: BinaryTypeKind::Record,
            }
        );
    }

    #[test]
    fn enumeration_allows_only_appended_tags_within_a_minor() {
        // Arrange
        const PREVIOUS: &[BinaryEnumVariantSchema] = &[BinaryEnumVariantSchema::new(0, "first")];
        const APPENDED: &[BinaryEnumVariantSchema] = &[
            BinaryEnumVariantSchema::new(0, "first"),
            BinaryEnumVariantSchema::new(1, "second"),
        ];
        const MUTATED: &[BinaryEnumVariantSchema] = &[BinaryEnumVariantSchema::new(0, "renamed")];
        let previous = BinaryEnumSchema::new("test.choice", ProtocolVersion::new(1, 0), PREVIOUS);
        let appended = BinaryEnumSchema::new("test.choice", ProtocolVersion::new(1, 1), APPENDED);
        let mutated = BinaryEnumSchema::new("test.choice", ProtocolVersion::new(1, 1), MUTATED);

        // Act
        let append_compatibility = previous.check_minor_compatibility(appended);
        let mutation = previous
            .check_minor_compatibility(mutated)
            .expect_err("existing enum tag meaning must not change");

        // Assert
        assert_eq!(append_compatibility, Ok(()));
        assert_eq!(mutation, BinaryCompatibilityError::PositionalLayoutChanged);
    }

    #[test]
    fn catalog_allows_new_types_and_appended_union_variants() {
        // Arrange
        const FIRST_VARIANTS: &[BinaryVariantSchema] = &[BinaryVariantSchema::new(1, "first", &[])];
        const NEXT_VARIANTS: &[BinaryVariantSchema] = &[
            BinaryVariantSchema::new(1, "first", &[]),
            BinaryVariantSchema::new(2, "second", &[]),
        ];
        const PREVIOUS_TYPES: &[BinaryTypeSchema] = &[BinaryTypeSchema::TaggedUnion(
            BinarySchema::new("test.message", ProtocolVersion::new(1, 0), FIRST_VARIANTS),
        )];
        const NEXT_TYPES: &[BinaryTypeSchema] = &[
            BinaryTypeSchema::TaggedUnion(BinarySchema::new(
                "test.message",
                ProtocolVersion::new(1, 1),
                NEXT_VARIANTS,
            )),
            BinaryTypeSchema::Record(BinaryRecordSchema::new(
                "test.new_record",
                ProtocolVersion::new(1, 1),
                &[],
            )),
        ];
        let previous = BinarySchemaCatalog::new(PREVIOUS_TYPES);
        let next = BinarySchemaCatalog::new(NEXT_TYPES);

        // Act
        let compatibility = previous.check_minor_compatibility(next);

        // Assert
        assert_eq!(compatibility, Ok(()));
        assert_ne!(previous.layout_fingerprint(), next.layout_fingerprint());
    }

    #[test]
    fn catalog_rejects_changed_positional_record_layout() {
        // Arrange
        const PREVIOUS_TYPES: &[BinaryTypeSchema] =
            &[BinaryTypeSchema::Record(BinaryRecordSchema::new(
                "test.record",
                ProtocolVersion::new(1, 0),
                &[BinaryFieldSchema::new(
                    "value",
                    BinaryFieldKind::U64,
                    BinaryCardinality::Required,
                )],
            ))];
        const NEXT_TYPES: &[BinaryTypeSchema] =
            &[BinaryTypeSchema::Record(BinaryRecordSchema::new(
                "test.record",
                ProtocolVersion::new(1, 1),
                &[BinaryFieldSchema::new(
                    "value",
                    BinaryFieldKind::U32,
                    BinaryCardinality::Required,
                )],
            ))];
        let previous = BinarySchemaCatalog::new(PREVIOUS_TYPES);
        let next = BinarySchemaCatalog::new(NEXT_TYPES);

        // Act
        let error = previous
            .check_minor_compatibility(next)
            .expect_err("record layout changes must fail within a major");

        // Assert
        assert_eq!(
            error,
            BinaryCatalogCompatibilityError::Type(BinaryTypeCompatibilityError::Schema(
                BinaryCompatibilityError::PositionalLayoutChanged
            ))
        );
    }

    #[test]
    fn catalog_rejects_removed_or_retyped_existing_types() {
        // Arrange
        const PREVIOUS_TYPES: &[BinaryTypeSchema] = &[BinaryTypeSchema::Record(
            BinaryRecordSchema::new("test.record", ProtocolVersion::new(1, 0), &[]),
        )];
        const RETYPED_TYPES: &[BinaryTypeSchema] =
            &[BinaryTypeSchema::TaggedUnion(BinarySchema::new(
                "test.record",
                ProtocolVersion::new(1, 1),
                &[BinaryVariantSchema::new(1, "value", &[])],
            ))];
        let previous = BinarySchemaCatalog::new(PREVIOUS_TYPES);
        let empty = BinarySchemaCatalog::new(&[]);
        let retyped = BinarySchemaCatalog::new(RETYPED_TYPES);

        // Act
        let removed = previous
            .check_minor_compatibility(empty)
            .expect_err("removing a published type must fail");
        let kind_changed = previous
            .check_minor_compatibility(retyped)
            .expect_err("changing a published type kind must fail");

        // Assert
        assert_eq!(
            removed,
            BinaryCatalogCompatibilityError::TypeRemoved { id: "test.record" }
        );
        assert_eq!(
            kind_changed,
            BinaryCatalogCompatibilityError::Type(BinaryTypeCompatibilityError::KindChanged {
                id: "test.record",
                previous: BinaryTypeKind::Record,
                next: BinaryTypeKind::TaggedUnion,
            })
        );
    }
}
