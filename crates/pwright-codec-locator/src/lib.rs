//! Canonical bounded binary payloads for the optional locator capability.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::{NonZeroU8, NonZeroU32};
use std::str::{FromStr, Utf8Error};
use std::time::Duration;

use pwright_cap_locator::{
    AllInnerTexts, AllInnerTextsParams, AllInnerTextsResult, AllTextContents,
    AllTextContentsParams, AllTextContentsResult, AriaRole, AriaTriState, Blur, BlurParams, Click,
    ClickOptions, ClickOptionsV2, ClickParams, ClickParamsV2, Count, CountParams, CountResult,
    CssSelector, Fill, FillOptions, FillParams, Focus, FocusParams, GetAttribute,
    GetAttributeParams, GetAttributeResult, Hover, HoverOptions, HoverParams, InnerHtml,
    InnerHtmlParams, InnerHtmlResult, InnerText, InnerTextParams, InnerTextResult, InputValue,
    InputValueParams, InputValueResult, IsChecked, IsCheckedParams, IsCheckedResult, IsEditable,
    IsEditableParams, IsEditableResult, IsEnabled, IsEnabledParams, IsEnabledResult, IsVisible,
    IsVisibleParams, IsVisibleResult, KeyboardModifier, KeyboardModifiers, LocatorErrorDetails,
    MouseButton, PRESS_KEY_MAX_BYTES, Position, Press, PressKey, PressOptions, PressParams,
    RegexMatcher, RegexMatcherError, RoleOptions, RolePatternSelector, RoleSelector,
    ScrollIntoViewIfNeeded, ScrollIntoViewIfNeededParams, ScrollMode, SelectOption,
    SelectOptionOptions, SelectOptionParams, SelectOptionResult, SelectOptionValue, Selector,
    SetChecked, SetCheckedOptions, SetCheckedParams, TestIdAttributes, TestIdAttributesError,
    TextContent, TextContentParams, TextContentResult, TextMatcher, UserFacingAttribute, WaitFor,
    WaitForOptions, WaitForParams, WaitForState,
};
use pwright_codec_binary::{
    BinaryDecode, BinaryDecodeError, BinaryEncode, BinaryEncodeError, BinaryReader,
    BinaryValueContext, BinaryValueDecodeError, BinaryValueEncodeError, BinaryValueLimits,
    BinaryWriter,
};
use pwright_core::{
    CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
    MethodTag, ObjectId, OperationError, ProtocolVersion,
};
use pwright_transport::{EncodedParts, FrameRoute, WireCodec, WireFrame};

const CLICK_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_CLICK_SCHEMA.command_tag();
const CLICK_RESULT: u8 = generated_schema::COMMAND_LOCATOR_CLICK_SCHEMA.result_tag();
const COUNT_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_COUNT_SCHEMA.command_tag();
const COUNT_RESULT: u8 = generated_schema::COMMAND_LOCATOR_COUNT_SCHEMA.result_tag();
const ALL_INNER_TEXTS_COMMAND: u8 =
    generated_schema::COMMAND_LOCATOR_ALL_INNER_TEXTS_SCHEMA.command_tag();
const ALL_INNER_TEXTS_RESULT: u8 =
    generated_schema::COMMAND_LOCATOR_ALL_INNER_TEXTS_SCHEMA.result_tag();
const ALL_TEXT_CONTENTS_COMMAND: u8 =
    generated_schema::COMMAND_LOCATOR_ALL_TEXT_CONTENTS_SCHEMA.command_tag();
const ALL_TEXT_CONTENTS_RESULT: u8 =
    generated_schema::COMMAND_LOCATOR_ALL_TEXT_CONTENTS_SCHEMA.result_tag();
const BLUR_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_BLUR_SCHEMA.command_tag();
const BLUR_RESULT: u8 = generated_schema::COMMAND_LOCATOR_BLUR_SCHEMA.result_tag();
const FILL_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_FILL_SCHEMA.command_tag();
const FILL_RESULT: u8 = generated_schema::COMMAND_LOCATOR_FILL_SCHEMA.result_tag();
const FOCUS_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_FOCUS_SCHEMA.command_tag();
const FOCUS_RESULT: u8 = generated_schema::COMMAND_LOCATOR_FOCUS_SCHEMA.result_tag();
const GET_ATTRIBUTE_COMMAND: u8 =
    generated_schema::COMMAND_LOCATOR_GET_ATTRIBUTE_SCHEMA.command_tag();
const GET_ATTRIBUTE_RESULT: u8 =
    generated_schema::COMMAND_LOCATOR_GET_ATTRIBUTE_SCHEMA.result_tag();
const HOVER_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_HOVER_SCHEMA.command_tag();
const HOVER_RESULT: u8 = generated_schema::COMMAND_LOCATOR_HOVER_SCHEMA.result_tag();
const INNER_HTML_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_INNER_HTML_SCHEMA.command_tag();
const INNER_HTML_RESULT: u8 = generated_schema::COMMAND_LOCATOR_INNER_HTML_SCHEMA.result_tag();
const INNER_TEXT_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_INNER_TEXT_SCHEMA.command_tag();
const INNER_TEXT_RESULT: u8 = generated_schema::COMMAND_LOCATOR_INNER_TEXT_SCHEMA.result_tag();
const INPUT_VALUE_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_INPUT_VALUE_SCHEMA.command_tag();
const INPUT_VALUE_RESULT: u8 = generated_schema::COMMAND_LOCATOR_INPUT_VALUE_SCHEMA.result_tag();
const IS_CHECKED_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_IS_CHECKED_SCHEMA.command_tag();
const IS_CHECKED_RESULT: u8 = generated_schema::COMMAND_LOCATOR_IS_CHECKED_SCHEMA.result_tag();
const IS_EDITABLE_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_IS_EDITABLE_SCHEMA.command_tag();
const IS_EDITABLE_RESULT: u8 = generated_schema::COMMAND_LOCATOR_IS_EDITABLE_SCHEMA.result_tag();
const IS_ENABLED_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_IS_ENABLED_SCHEMA.command_tag();
const IS_ENABLED_RESULT: u8 = generated_schema::COMMAND_LOCATOR_IS_ENABLED_SCHEMA.result_tag();
const IS_VISIBLE_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_IS_VISIBLE_SCHEMA.command_tag();
const IS_VISIBLE_RESULT: u8 = generated_schema::COMMAND_LOCATOR_IS_VISIBLE_SCHEMA.result_tag();
const PRESS_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_PRESS_SCHEMA.command_tag();
const PRESS_RESULT: u8 = generated_schema::COMMAND_LOCATOR_PRESS_SCHEMA.result_tag();
const SCROLL_INTO_VIEW_IF_NEEDED_COMMAND: u8 =
    generated_schema::COMMAND_LOCATOR_SCROLL_INTO_VIEW_IF_NEEDED_SCHEMA.command_tag();
const SCROLL_INTO_VIEW_IF_NEEDED_RESULT: u8 =
    generated_schema::COMMAND_LOCATOR_SCROLL_INTO_VIEW_IF_NEEDED_SCHEMA.result_tag();
const SELECT_OPTION_COMMAND: u8 =
    generated_schema::COMMAND_LOCATOR_SELECT_OPTION_SCHEMA.command_tag();
const SELECT_OPTION_RESULT: u8 =
    generated_schema::COMMAND_LOCATOR_SELECT_OPTION_SCHEMA.result_tag();
const SET_CHECKED_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_SET_CHECKED_SCHEMA.command_tag();
const SET_CHECKED_RESULT: u8 = generated_schema::COMMAND_LOCATOR_SET_CHECKED_SCHEMA.result_tag();
const TEXT_CONTENT_COMMAND: u8 =
    generated_schema::COMMAND_LOCATOR_TEXT_CONTENT_SCHEMA.command_tag();
const TEXT_CONTENT_RESULT: u8 = generated_schema::COMMAND_LOCATOR_TEXT_CONTENT_SCHEMA.result_tag();
const WAIT_FOR_COMMAND: u8 = generated_schema::COMMAND_LOCATOR_WAIT_FOR_SCHEMA.command_tag();
const WAIT_FOR_RESULT: u8 = generated_schema::COMMAND_LOCATOR_WAIT_FOR_SCHEMA.result_tag();
const MAX_ATTRIBUTE_NAME_BYTES: usize = 64 * 1024;
const MAX_ATTRIBUTE_RESULT_BYTES: usize = 1024 * 1024;
const MAX_INPUT_VALUE_RESULT_BYTES: usize = 1024 * 1024;
pub const MAX_ALL_TEXT_CONTENTS_ITEMS: usize = 4096;
pub const MAX_ALL_TEXT_CONTENTS_ITEM_BYTES: usize = 1024 * 1024;
pub const MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_ALL_INNER_TEXTS_ITEMS: usize = MAX_ALL_TEXT_CONTENTS_ITEMS;
pub const MAX_ALL_INNER_TEXTS_ITEM_BYTES: usize = MAX_ALL_TEXT_CONTENTS_ITEM_BYTES;
pub const MAX_ALL_INNER_TEXTS_TOTAL_BYTES: usize = MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES;
pub const MAX_SELECT_OPTION_ITEMS: usize = 1024;
pub const MAX_SELECT_OPTION_ITEM_BYTES: usize = 64 * 1024;
pub const MAX_SELECT_OPTION_TOTAL_BYTES: usize = 1024 * 1024;

#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema;

/// Generated locator 2.x wire model kept separate from the published 1.x
/// codec while the connection-selected adapter is implemented.
#[doc(hidden)]
#[rustfmt::skip]
pub mod generated_schema_v2;

mod v2;

pub use v2::LocatorV2Codec;

pub use generated_schema::{
    CATALOG as LOCATOR_SCHEMA_CATALOG, COMMANDS as LOCATOR_COMMAND_SCHEMAS,
    ROOT_SCHEMA as LOCATOR_SCHEMA,
};

pub use generated_schema_v2::{
    CATALOG as LOCATOR_V2_SCHEMA_CATALOG, COMMANDS as LOCATOR_V2_COMMAND_SCHEMAS,
    ROOT_SCHEMA as LOCATOR_V2_SCHEMA,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocatorCodecLimits {
    pub string_bytes: usize,
    pub selector_depth: usize,
    pub selector_nodes: usize,
}

impl LocatorCodecLimits {
    pub const fn new(string_bytes: usize) -> Self {
        Self {
            string_bytes,
            selector_depth: 32,
            selector_nodes: 256,
        }
    }

    pub const fn with_selector_complexity(mut self, depth: usize, nodes: usize) -> Self {
        self.selector_depth = depth;
        self.selector_nodes = nodes;
        self
    }
}

#[derive(Clone, Copy, Debug)]
pub struct LocatorCodec {
    limits: LocatorCodecLimits,
    capability_version: ProtocolVersion,
}

impl LocatorCodec {
    pub const fn new(limits: LocatorCodecLimits) -> Self {
        Self {
            limits,
            capability_version: ProtocolVersion::new(1, 22),
        }
    }

    pub const fn limits(self) -> LocatorCodecLimits {
        self.limits
    }

    pub const fn with_capability_version(mut self, capability_version: ProtocolVersion) -> Self {
        self.capability_version = capability_version;
        self
    }

    pub const fn capability_version(self) -> ProtocolVersion {
        self.capability_version
    }

    fn value_limits(self) -> BinaryValueLimits {
        BinaryValueLimits::new(
            self.limits.string_bytes,
            0,
            5,
            self.limits.selector_depth,
            self.limits.selector_nodes,
        )
    }

    fn encode_string(
        &self,
        payload: &mut BinaryWriter,
        value: &str,
    ) -> Result<(), LocatorCodecError> {
        if value.len() > self.limits.string_bytes {
            return Err(LocatorCodecError::StringTooLong {
                actual: value.len(),
                limit: self.limits.string_bytes,
            });
        }
        let length =
            u32::try_from(value.len()).map_err(|_| LocatorCodecError::IntegerOutOfRange {
                field: "string length",
                value: u64::try_from(value.len()).unwrap_or(u64::MAX),
            })?;
        payload.u32(length)?;
        payload.raw(value.as_bytes())?;
        Ok(())
    }

    fn encode_bounded_string(
        &self,
        payload: &mut BinaryWriter,
        value: &str,
        schema_limit: usize,
    ) -> Result<(), LocatorCodecError> {
        if value.len() > schema_limit {
            return Err(LocatorCodecError::StringTooLong {
                actual: value.len(),
                limit: schema_limit,
            });
        }
        self.encode_string(payload, value)
    }

    fn decode_string(&self, reader: &mut BinaryReader<'_>) -> Result<String, LocatorCodecError> {
        let length = usize::try_from(u64::from(reader.u32()?)).map_err(|_| {
            LocatorCodecError::IntegerOutOfRange {
                field: "string length",
                value: u64::MAX,
            }
        })?;
        if length > self.limits.string_bytes {
            return Err(LocatorCodecError::StringTooLong {
                actual: length,
                limit: self.limits.string_bytes,
            });
        }
        std::str::from_utf8(reader.take(length)?)
            .map(str::to_owned)
            .map_err(LocatorCodecError::InvalidUtf8)
    }

    fn decode_bounded_string(
        &self,
        reader: &mut BinaryReader<'_>,
        schema_limit: usize,
    ) -> Result<String, LocatorCodecError> {
        let length = usize::try_from(u64::from(reader.u32()?)).map_err(|_| {
            LocatorCodecError::IntegerOutOfRange {
                field: "string length",
                value: u64::MAX,
            }
        })?;
        let limit = self.limits.string_bytes.min(schema_limit);
        if length > limit {
            return Err(LocatorCodecError::StringTooLong {
                actual: length,
                limit,
            });
        }
        std::str::from_utf8(reader.take(length)?)
            .map(str::to_owned)
            .map_err(LocatorCodecError::InvalidUtf8)
    }

    fn encode_error(
        &self,
        payload: &mut BinaryWriter,
        error: &OperationError<LocatorErrorDetails>,
    ) -> Result<(), LocatorCodecError> {
        payload.u8(encode_failure_kind(error.kind()))?;
        self.encode_string(payload, error.message())?;
        match error.details() {
            None => payload.u8(0)?,
            Some(details) => {
                payload.u8(1)?;
                match details {
                    LocatorErrorDetails::InvalidSelector(message) => {
                        payload.u8(0)?;
                        self.encode_string(payload, message)?;
                    }
                    LocatorErrorDetails::StrictViolation { selector, matches } => {
                        payload.u8(1)?;
                        self.encode_string(payload, selector)?;
                        payload.u32(*matches)?;
                    }
                    LocatorErrorDetails::InvalidTarget(message) => {
                        payload.u8(4)?;
                        self.encode_string(payload, message)?;
                    }
                    LocatorErrorDetails::InvalidValue(message) => {
                        payload.u8(5)?;
                        self.encode_string(payload, message)?;
                    }
                    LocatorErrorDetails::InvalidBackendResponse(message) => {
                        payload.u8(2)?;
                        self.encode_string(payload, message)?;
                    }
                    LocatorErrorDetails::Backend(message) => {
                        payload.u8(3)?;
                        self.encode_string(payload, message)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn decode_error(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<OperationError<LocatorErrorDetails>, LocatorCodecError> {
        let kind = decode_failure_kind(reader.u8()?)?;
        let message = self.decode_string(reader)?;
        let details = match reader.u8()? {
            0 => None,
            1 => Some(match reader.u8()? {
                0 => LocatorErrorDetails::InvalidSelector(self.decode_string(reader)?),
                1 => LocatorErrorDetails::StrictViolation {
                    selector: self.decode_string(reader)?,
                    matches: reader.u32()?,
                },
                2 => LocatorErrorDetails::InvalidBackendResponse(self.decode_string(reader)?),
                3 => LocatorErrorDetails::Backend(self.decode_string(reader)?),
                4 => LocatorErrorDetails::InvalidTarget(self.decode_string(reader)?),
                5 => LocatorErrorDetails::InvalidValue(self.decode_string(reader)?),
                value => return Err(LocatorCodecError::InvalidErrorDetails(value)),
            }),
            value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
        };
        Ok(OperationError::new(kind, message, details))
    }

    fn encode_selector(
        &self,
        payload: &mut BinaryWriter,
        selector: &Selector,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<(), LocatorCodecError> {
        self.claim_selector_node(depth, nodes)?;
        match selector {
            Selector::Css(css) => {
                payload.u8(0)?;
                self.encode_string(payload, css.as_str())?;
            }
            Selector::Nth { base, index } => {
                payload.u8(1)?;
                self.encode_selector(payload, base, depth + 1, nodes)?;
                payload.i32(*index)?;
            }
            Selector::FilterText { base, text, exact } => {
                payload.u8(2)?;
                self.encode_selector(payload, base, depth + 1, nodes)?;
                self.encode_string(payload, text)?;
                payload.u8(u8::from(*exact))?;
            }
            Selector::And { left, right } => {
                payload.u8(3)?;
                self.encode_selector(payload, left, depth + 1, nodes)?;
                self.encode_selector(payload, right, depth + 1, nodes)?;
            }
            Selector::Or { left, right } => {
                payload.u8(4)?;
                self.encode_selector(payload, left, depth + 1, nodes)?;
                self.encode_selector(payload, right, depth + 1, nodes)?;
            }
            Selector::Text(matcher) => {
                payload.u8(5)?;
                self.encode_text_matcher(payload, matcher)?;
            }
            Selector::Label(matcher) => {
                payload.u8(6)?;
                self.encode_text_matcher(payload, matcher)?;
            }
            Selector::Role(role) => {
                payload.u8(7)?;
                self.encode_role_selector(payload, role)?;
            }
            Selector::Within { base, descendant } => {
                payload.u8(8)?;
                self.encode_selector(payload, base, depth + 1, nodes)?;
                self.encode_selector(payload, descendant, depth + 1, nodes)?;
            }
            Selector::FilterNotText { base, text, exact } => {
                payload.u8(9)?;
                self.encode_selector(payload, base, depth + 1, nodes)?;
                self.encode_string(payload, text)?;
                payload.u8(u8::from(*exact))?;
            }
            Selector::FilterHas { base, descendant } => {
                payload.u8(10)?;
                self.encode_selector(payload, base, depth + 1, nodes)?;
                self.encode_selector(payload, descendant, depth + 1, nodes)?;
            }
            Selector::FilterHasNot { base, descendant } => {
                payload.u8(11)?;
                self.encode_selector(payload, base, depth + 1, nodes)?;
                self.encode_selector(payload, descendant, depth + 1, nodes)?;
            }
            Selector::FilterVisible { base, visible } => {
                payload.u8(12)?;
                self.encode_selector(payload, base, depth + 1, nodes)?;
                payload.u8(u8::from(*visible))?;
            }
            Selector::TextRegex(matcher) => {
                payload.u8(13)?;
                self.encode_regex_matcher(payload, matcher)?;
            }
            Selector::LabelRegex(matcher) => {
                payload.u8(14)?;
                self.encode_regex_matcher(payload, matcher)?;
            }
            Selector::FilterTextRegex { base, matcher } => {
                payload.u8(15)?;
                self.encode_selector(payload, base, depth + 1, nodes)?;
                self.encode_regex_matcher(payload, matcher)?;
            }
            Selector::FilterNotTextRegex { base, matcher } => {
                payload.u8(16)?;
                self.encode_selector(payload, base, depth + 1, nodes)?;
                self.encode_regex_matcher(payload, matcher)?;
            }
            Selector::RolePattern(selector) => {
                payload.u8(17)?;
                self.encode_role_pattern_selector(payload, selector)?;
            }
            Selector::AttributeText { attribute, matcher } => {
                payload.u8(18)?;
                payload.u8(encode_user_facing_attribute(*attribute))?;
                self.encode_text_matcher(payload, matcher)?;
            }
            Selector::AttributeRegex { attribute, matcher } => {
                payload.u8(19)?;
                payload.u8(encode_user_facing_attribute(*attribute))?;
                self.encode_regex_matcher(payload, matcher)?;
            }
            Selector::TestIdText { attributes, value } => {
                payload.u8(20)?;
                self.encode_string(payload, attributes.as_csv())?;
                self.encode_string(payload, value)?;
            }
            Selector::TestIdRegex {
                attributes,
                matcher,
            } => {
                payload.u8(21)?;
                self.encode_string(payload, attributes.as_csv())?;
                self.encode_regex_matcher(payload, matcher)?;
            }
        }
        Ok(())
    }

    fn decode_selector(
        &self,
        reader: &mut BinaryReader<'_>,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<Selector, LocatorCodecError> {
        self.claim_selector_node(depth, nodes)?;
        match reader.u8()? {
            0 => CssSelector::new(self.decode_string(reader)?)
                .map(Selector::Css)
                .map_err(|_| LocatorCodecError::EmptySelector),
            1 => Ok(Selector::Nth {
                base: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                index: reader.i32()?,
            }),
            2 => Ok(Selector::FilterText {
                base: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                text: self.decode_string(reader)?,
                exact: decode_bool(reader.u8()?)?,
            }),
            3 => Ok(Selector::And {
                left: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                right: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
            }),
            4 => Ok(Selector::Or {
                left: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                right: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
            }),
            5 => Ok(Selector::Text(self.decode_text_matcher(reader)?)),
            6 => Ok(Selector::Label(self.decode_text_matcher(reader)?)),
            7 => Ok(Selector::Role(self.decode_role_selector(reader)?)),
            8 => Ok(Selector::Within {
                base: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                descendant: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
            }),
            9 => Ok(Selector::FilterNotText {
                base: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                text: self.decode_string(reader)?,
                exact: decode_bool(reader.u8()?)?,
            }),
            10 => Ok(Selector::FilterHas {
                base: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                descendant: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
            }),
            11 => Ok(Selector::FilterHasNot {
                base: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                descendant: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
            }),
            12 => Ok(Selector::FilterVisible {
                base: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                visible: decode_bool(reader.u8()?)?,
            }),
            13 => self.decode_regex_matcher(reader).map(Selector::TextRegex),
            14 => self.decode_regex_matcher(reader).map(Selector::LabelRegex),
            15 => Ok(Selector::FilterTextRegex {
                base: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                matcher: self.decode_regex_matcher(reader)?,
            }),
            16 => Ok(Selector::FilterNotTextRegex {
                base: Box::new(self.decode_selector(reader, depth + 1, nodes)?),
                matcher: self.decode_regex_matcher(reader)?,
            }),
            17 => self
                .decode_role_pattern_selector(reader)
                .map(Selector::RolePattern),
            18 => Ok(Selector::AttributeText {
                attribute: decode_user_facing_attribute(reader.u8()?)?,
                matcher: self.decode_text_matcher(reader)?,
            }),
            19 => Ok(Selector::AttributeRegex {
                attribute: decode_user_facing_attribute(reader.u8()?)?,
                matcher: self.decode_regex_matcher(reader)?,
            }),
            20 => Ok(Selector::TestIdText {
                attributes: self.decode_test_id_attributes(reader)?,
                value: self.decode_string(reader)?,
            }),
            21 => Ok(Selector::TestIdRegex {
                attributes: self.decode_test_id_attributes(reader)?,
                matcher: self.decode_regex_matcher(reader)?,
            }),
            value => Err(LocatorCodecError::InvalidSelectorKind(value)),
        }
    }

    fn decode_test_id_attributes(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<TestIdAttributes, LocatorCodecError> {
        TestIdAttributes::new(self.decode_string(reader)?)
            .map_err(LocatorCodecError::InvalidTestIdAttributes)
    }

    fn encode_regex_matcher(
        &self,
        payload: &mut BinaryWriter,
        matcher: &RegexMatcher,
    ) -> Result<(), LocatorCodecError> {
        self.encode_string(payload, matcher.source())?;
        self.encode_string(payload, matcher.flags())?;
        Ok(())
    }

    fn decode_regex_matcher(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<RegexMatcher, LocatorCodecError> {
        RegexMatcher::new(self.decode_string(reader)?, self.decode_string(reader)?)
            .map_err(LocatorCodecError::InvalidRegexFlags)
    }

    fn encode_text_matcher(
        &self,
        payload: &mut BinaryWriter,
        matcher: &TextMatcher,
    ) -> Result<(), LocatorCodecError> {
        self.encode_string(payload, matcher.value())?;
        payload.u8(u8::from(matcher.exact()))?;
        Ok(())
    }

    fn decode_text_matcher(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<TextMatcher, LocatorCodecError> {
        Ok(TextMatcher::new(
            self.decode_string(reader)?,
            decode_bool(reader.u8()?)?,
        ))
    }

    fn encode_role_selector(
        &self,
        payload: &mut BinaryWriter,
        selector: &RoleSelector,
    ) -> Result<(), LocatorCodecError> {
        self.encode_string(payload, selector.role().as_str())?;
        self.encode_optional_text_matcher(payload, selector.options().name.as_ref())?;
        self.encode_optional_text_matcher(payload, selector.options().description.as_ref())?;
        encode_optional_tri_state(payload, selector.options().checked)?;
        encode_optional_bool(payload, selector.options().disabled)?;
        encode_optional_bool(payload, selector.options().expanded)?;
        payload.u8(u8::from(selector.options().include_hidden))?;
        match selector.options().level {
            None => payload.u8(0)?,
            Some(level) => {
                payload.u8(1)?;
                payload.u32(level.get())?;
            }
        }
        encode_optional_tri_state(payload, selector.options().pressed)?;
        encode_optional_bool(payload, selector.options().selected)?;
        Ok(())
    }

    fn decode_role_selector(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<RoleSelector, LocatorCodecError> {
        let role_text = self.decode_string(reader)?;
        let role = AriaRole::from_str(&role_text)
            .map_err(|_| LocatorCodecError::InvalidAriaRole(role_text))?;
        let options = RoleOptions {
            name: self.decode_optional_text_matcher(reader)?,
            description: self.decode_optional_text_matcher(reader)?,
            name_regex: None,
            description_regex: None,
            checked: decode_optional_tri_state(reader)?,
            disabled: decode_optional_bool(reader)?,
            expanded: decode_optional_bool(reader)?,
            include_hidden: decode_bool(reader.u8()?)?,
            level: match reader.u8()? {
                0 => None,
                1 => Some(NonZeroU32::new(reader.u32()?).ok_or(LocatorCodecError::ZeroAriaLevel)?),
                value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
            },
            pressed: decode_optional_tri_state(reader)?,
            selected: decode_optional_bool(reader)?,
        };
        RoleSelector::new(role, options).map_err(|error| LocatorCodecError::InvalidRoleOption {
            role,
            option: error.option,
        })
    }

    fn encode_role_pattern_selector(
        &self,
        payload: &mut BinaryWriter,
        selector: &RolePatternSelector,
    ) -> Result<(), LocatorCodecError> {
        self.encode_string(payload, selector.role().as_str())?;
        self.encode_optional_text_pattern(
            payload,
            selector.options().name.as_ref(),
            selector.options().name_regex.as_ref(),
        )?;
        self.encode_optional_text_pattern(
            payload,
            selector.options().description.as_ref(),
            selector.options().description_regex.as_ref(),
        )?;
        encode_optional_tri_state(payload, selector.options().checked)?;
        encode_optional_bool(payload, selector.options().disabled)?;
        encode_optional_bool(payload, selector.options().expanded)?;
        payload.u8(u8::from(selector.options().include_hidden))?;
        match selector.options().level {
            None => payload.u8(0)?,
            Some(level) => {
                payload.u8(1)?;
                payload.u32(level.get())?;
            }
        }
        encode_optional_tri_state(payload, selector.options().pressed)?;
        encode_optional_bool(payload, selector.options().selected)?;
        Ok(())
    }

    fn decode_role_pattern_selector(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<RolePatternSelector, LocatorCodecError> {
        let role_text = self.decode_string(reader)?;
        let role = AriaRole::from_str(&role_text)
            .map_err(|_| LocatorCodecError::InvalidAriaRole(role_text))?;
        let (name, name_regex) = self.decode_optional_text_pattern(reader)?;
        let (description, description_regex) = self.decode_optional_text_pattern(reader)?;
        let options = RoleOptions {
            name,
            description,
            name_regex,
            description_regex,
            checked: decode_optional_tri_state(reader)?,
            disabled: decode_optional_bool(reader)?,
            expanded: decode_optional_bool(reader)?,
            include_hidden: decode_bool(reader.u8()?)?,
            level: match reader.u8()? {
                0 => None,
                1 => Some(NonZeroU32::new(reader.u32()?).ok_or(LocatorCodecError::ZeroAriaLevel)?),
                value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
            },
            pressed: decode_optional_tri_state(reader)?,
            selected: decode_optional_bool(reader)?,
        };
        RolePatternSelector::new(role, options).map_err(|error| {
            LocatorCodecError::InvalidRoleOption {
                role,
                option: error.option,
            }
        })
    }

    fn encode_optional_text_pattern(
        &self,
        payload: &mut BinaryWriter,
        string: Option<&TextMatcher>,
        regex: Option<&RegexMatcher>,
    ) -> Result<(), LocatorCodecError> {
        match (string, regex) {
            (None, None) => payload.u8(0)?,
            (Some(matcher), None) => {
                payload.u8(1)?;
                payload.u8(0)?;
                self.encode_text_matcher(payload, matcher)?;
            }
            (None, Some(matcher)) => {
                payload.u8(1)?;
                payload.u8(1)?;
                self.encode_regex_matcher(payload, matcher)?;
            }
            (Some(_), Some(_)) => {
                return Err(LocatorCodecError::InvalidTextPattern(2));
            }
        }
        Ok(())
    }

    fn decode_optional_text_pattern(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<(Option<TextMatcher>, Option<RegexMatcher>), LocatorCodecError> {
        match reader.u8()? {
            0 => Ok((None, None)),
            1 => match reader.u8()? {
                0 => self
                    .decode_text_matcher(reader)
                    .map(|matcher| (Some(matcher), None)),
                1 => self
                    .decode_regex_matcher(reader)
                    .map(|matcher| (None, Some(matcher))),
                value => Err(LocatorCodecError::InvalidTextPattern(value)),
            },
            value => Err(LocatorCodecError::InvalidOptionFlag(value)),
        }
    }

    fn encode_optional_text_matcher(
        &self,
        payload: &mut BinaryWriter,
        matcher: Option<&TextMatcher>,
    ) -> Result<(), LocatorCodecError> {
        match matcher {
            None => payload.u8(0)?,
            Some(matcher) => {
                payload.u8(1)?;
                self.encode_text_matcher(payload, matcher)?;
            }
        }
        Ok(())
    }

    fn decode_optional_text_matcher(
        &self,
        reader: &mut BinaryReader<'_>,
    ) -> Result<Option<TextMatcher>, LocatorCodecError> {
        match reader.u8()? {
            0 => Ok(None),
            1 => self.decode_text_matcher(reader).map(Some),
            value => Err(LocatorCodecError::InvalidOptionFlag(value)),
        }
    }

    fn claim_selector_node(
        &self,
        depth: usize,
        nodes: &mut usize,
    ) -> Result<(), LocatorCodecError> {
        if depth >= self.limits.selector_depth {
            return Err(LocatorCodecError::SelectorTooDeep {
                actual: depth + 1,
                limit: self.limits.selector_depth,
            });
        }
        *nodes = nodes.saturating_add(1);
        if *nodes > self.limits.selector_nodes {
            return Err(LocatorCodecError::TooManySelectorNodes {
                actual: *nodes,
                limit: self.limits.selector_nodes,
            });
        }
        Ok(())
    }
}

impl WireCodec<Command<Click>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<Click>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(CLICK_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        payload.u8(encode_button(command.params().options.button))?;
        payload.u8(command.params().options.click_count.get())?;
        payload.u8(u8::from(command.params().options.force))?;
        payload.u8(u8::from(command.params().options.trial))?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: Click::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<Click>, Self::Error> {
        command_route(&frame, Click::METHOD)?;
        let mut reader = payload_reader(&frame, CLICK_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        let options = ClickOptions::new()
            .with_button(decode_button(reader.u8()?)?)
            .with_click_count(
                NonZeroU8::new(reader.u8()?).ok_or(LocatorCodecError::ZeroClickCount)?,
            )
            .with_force(decode_bool(reader.u8()?)?)
            .with_trial(decode_bool(reader.u8()?)?);
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            ClickParams::new(selector, options),
        ))
    }
}

impl WireCodec<Command<Hover>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<Hover>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(HOVER_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        encode_hover_options(&mut payload, command.params().options)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: Hover::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<Hover>, Self::Error> {
        command_route(&frame, Hover::METHOD)?;
        let mut reader = payload_reader(&frame, HOVER_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        let options = decode_hover_options(&mut reader)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            HoverParams::new(selector, options),
        ))
    }
}

macro_rules! impl_selector_unit_codec {
    ($command:ident, $params:ident, $command_tag:ident, $result_tag:ident, $require:ident) => {
        impl WireCodec<Command<$command>> for LocatorCodec {
            type Error = LocatorCodecError;

            fn encode_parts(
                &self,
                command: &Command<$command>,
            ) -> Result<EncodedParts, Self::Error> {
                $require(self.capability_version)?;
                let mut payload = BinaryWriter::new();
                payload.u8($command_tag)?;
                payload.u64(command.call_id().get())?;
                payload.u64(command.object_id().get())?;
                encode_metadata(&mut payload, command.metadata())?;
                let mut selector_nodes = 0;
                self.encode_selector(
                    &mut payload,
                    &command.params().selector,
                    0,
                    &mut selector_nodes,
                )?;
                Ok(EncodedParts::new(
                    FrameRoute::Command {
                        call_id: command.call_id(),
                        target: command.object_id(),
                        method: $command::METHOD,
                    },
                    payload.into_bytes(),
                    Vec::new(),
                ))
            }

            fn decode_frame(&self, frame: WireFrame) -> Result<Command<$command>, Self::Error> {
                $require(self.capability_version)?;
                command_route(&frame, $command::METHOD)?;
                let mut reader = payload_reader(&frame, $command_tag)?;
                let call_id = CallId::new(reader.u64()?);
                let object_id = ObjectId::new(reader.u64()?);
                let metadata = decode_metadata(&mut reader)?;
                let mut selector_nodes = 0;
                let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
                reader.finish()?;
                Ok(Command::new(
                    call_id,
                    object_id,
                    metadata,
                    $params::new(selector),
                ))
            }
        }

        impl WireCodec<CommandResult<$command>> for LocatorCodec {
            type Error = LocatorCodecError;

            fn encode_parts(
                &self,
                result: &CommandResult<$command>,
            ) -> Result<EncodedParts, Self::Error> {
                $require(self.capability_version)?;
                let mut payload = BinaryWriter::new();
                payload.u8($result_tag)?;
                payload.u64(result.call_id().get())?;
                match result.outcome() {
                    Ok(()) => payload.u8(0)?,
                    Err(error) => {
                        payload.u8(1)?;
                        self.encode_error(&mut payload, error)?;
                    }
                }
                Ok(EncodedParts::new(
                    FrameRoute::Result {
                        call_id: result.call_id(),
                    },
                    payload.into_bytes(),
                    Vec::new(),
                ))
            }

            fn decode_frame(
                &self,
                frame: WireFrame,
            ) -> Result<CommandResult<$command>, Self::Error> {
                $require(self.capability_version)?;
                result_route(&frame)?;
                let mut reader = payload_reader(&frame, $result_tag)?;
                let call_id = CallId::new(reader.u64()?);
                let outcome = match reader.u8()? {
                    0 => Ok(()),
                    1 => Err(self.decode_error(&mut reader)?),
                    value => return Err(LocatorCodecError::InvalidOutcome(value)),
                };
                reader.finish()?;
                Ok(CommandResult::new(call_id, outcome))
            }
        }
    };
}

impl_selector_unit_codec!(
    Blur,
    BlurParams,
    BLUR_COMMAND,
    BLUR_RESULT,
    require_focus_management
);
impl_selector_unit_codec!(
    Focus,
    FocusParams,
    FOCUS_COMMAND,
    FOCUS_RESULT,
    require_focus_management
);
impl_selector_unit_codec!(
    ScrollIntoViewIfNeeded,
    ScrollIntoViewIfNeededParams,
    SCROLL_INTO_VIEW_IF_NEEDED_COMMAND,
    SCROLL_INTO_VIEW_IF_NEEDED_RESULT,
    require_scroll_into_view_if_needed
);

impl WireCodec<Command<Press>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<Press>) -> Result<EncodedParts, Self::Error> {
        require_press(self.capability_version)?;
        let mut payload = BinaryWriter::new();
        payload.u8(PRESS_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        self.encode_bounded_string(
            &mut payload,
            command.params().key.as_str(),
            PRESS_KEY_MAX_BYTES,
        )?;
        encode_press_options(&mut payload, command.params().options)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: Press::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<Press>, Self::Error> {
        require_press(self.capability_version)?;
        command_route(&frame, Press::METHOD)?;
        let mut reader = payload_reader(&frame, PRESS_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        let key = PressKey::new(self.decode_bounded_string(&mut reader, PRESS_KEY_MAX_BYTES)?)
            .map_err(|error| LocatorCodecError::InvalidPressKey(error.to_string()))?;
        let options = decode_press_options(&mut reader)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            PressParams::new(selector, key, options),
        ))
    }
}

impl WireCodec<CommandResult<Press>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, result: &CommandResult<Press>) -> Result<EncodedParts, Self::Error> {
        require_press(self.capability_version)?;
        let mut payload = BinaryWriter::new();
        payload.u8(PRESS_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(()) => payload.u8(0)?,
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<Press>, Self::Error> {
        require_press(self.capability_version)?;
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, PRESS_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome = match reader.u8()? {
            0 => Ok(()),
            1 => Err(self.decode_error(&mut reader)?),
            value => return Err(LocatorCodecError::InvalidOutcome(value)),
        };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<Command<SelectOption>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<SelectOption>) -> Result<EncodedParts, Self::Error> {
        require_select_option(self.capability_version)?;
        validate_select_option_descriptors(&command.params().values, self.limits.string_bytes)?;
        let mut payload = BinaryWriter::new();
        payload.u8(SELECT_OPTION_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        payload.u32(u32::try_from(command.params().values.len()).map_err(|_| {
            LocatorCodecError::IntegerOutOfRange {
                field: "select_option descriptor count",
                value: u64::try_from(command.params().values.len()).unwrap_or(u64::MAX),
            }
        })?)?;
        let mut context = BinaryValueContext::new(self.value_limits());
        for value in &command.params().values {
            select_option_value_to_wire(value).encode_binary(&mut payload, &mut context)?;
        }
        select_option_options_to_wire(command.params().options)
            .encode_binary(&mut payload, &mut context)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: SelectOption::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<SelectOption>, Self::Error> {
        require_select_option(self.capability_version)?;
        command_route(&frame, SelectOption::METHOD)?;
        let mut reader = payload_reader(&frame, SELECT_OPTION_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        let length = usize::try_from(u64::from(reader.u32()?)).map_err(|_| {
            LocatorCodecError::IntegerOutOfRange {
                field: "select_option descriptor count",
                value: u64::MAX,
            }
        })?;
        if length > MAX_SELECT_OPTION_ITEMS {
            return Err(LocatorCodecError::TooManyItems {
                field: "select_option descriptors",
                actual: length,
                limit: MAX_SELECT_OPTION_ITEMS,
            });
        }
        let mut context = BinaryValueContext::new(self.value_limits());
        let mut values = Vec::with_capacity(length);
        for _ in 0..length {
            let value = generated_schema::PwrightLocatorSelectOption::decode_binary(
                &mut reader,
                &mut context,
            )?;
            values.push(select_option_value_from_wire(value));
        }
        validate_select_option_descriptors(&values, self.limits.string_bytes)?;
        let options = generated_schema::PwrightLocatorSelectOptionOptions::decode_binary(
            &mut reader,
            &mut context,
        )?;
        let options = select_option_options_from_wire(options);
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            SelectOptionParams::new(selector, values, options),
        ))
    }
}

impl WireCodec<CommandResult<SelectOption>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<SelectOption>,
    ) -> Result<EncodedParts, Self::Error> {
        require_select_option(self.capability_version)?;
        let mut payload = BinaryWriter::new();
        payload.u8(SELECT_OPTION_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(values) => {
                validate_select_option_results(values, self.limits.string_bytes)?;
                payload.u8(0)?;
                payload.u32(u32::try_from(values.len()).map_err(|_| {
                    LocatorCodecError::IntegerOutOfRange {
                        field: "select_option result count",
                        value: u64::try_from(values.len()).unwrap_or(u64::MAX),
                    }
                })?)?;
                for value in values {
                    self.encode_bounded_string(&mut payload, value, MAX_SELECT_OPTION_ITEM_BYTES)?;
                }
            }
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<SelectOption>, Self::Error> {
        require_select_option(self.capability_version)?;
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, SELECT_OPTION_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome: Result<SelectOptionResult, OperationError<LocatorErrorDetails>> =
            match reader.u8()? {
                0 => {
                    let length = usize::try_from(u64::from(reader.u32()?)).map_err(|_| {
                        LocatorCodecError::IntegerOutOfRange {
                            field: "select_option result count",
                            value: u64::MAX,
                        }
                    })?;
                    if length > MAX_SELECT_OPTION_ITEMS {
                        return Err(LocatorCodecError::TooManyItems {
                            field: "select_option results",
                            actual: length,
                            limit: MAX_SELECT_OPTION_ITEMS,
                        });
                    }
                    let mut values = Vec::with_capacity(length);
                    for _ in 0..length {
                        values.push(
                            self.decode_bounded_string(&mut reader, MAX_SELECT_OPTION_ITEM_BYTES)?,
                        );
                    }
                    validate_select_option_results(&values, self.limits.string_bytes)?;
                    Ok(values)
                }
                1 => Err(self.decode_error(&mut reader)?),
                value => return Err(LocatorCodecError::InvalidOutcome(value)),
            };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<Command<SetChecked>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<SetChecked>) -> Result<EncodedParts, Self::Error> {
        require_set_checked(self.capability_version)?;
        let mut payload = BinaryWriter::new();
        payload.u8(SET_CHECKED_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        payload.u8(u8::from(command.params().checked))?;
        let options = set_checked_options_to_wire(command.params().options);
        let mut context = BinaryValueContext::new(self.value_limits());
        options.encode_binary(&mut payload, &mut context)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: SetChecked::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<SetChecked>, Self::Error> {
        require_set_checked(self.capability_version)?;
        command_route(&frame, SetChecked::METHOD)?;
        let mut reader = payload_reader(&frame, SET_CHECKED_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        let checked = match reader.u8()? {
            0 => false,
            1 => true,
            value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
        };
        let mut context = BinaryValueContext::new(self.value_limits());
        let options = generated_schema::PwrightLocatorSetCheckedOptions::decode_binary(
            &mut reader,
            &mut context,
        )?;
        let options = set_checked_options_from_wire(options)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            SetCheckedParams::new(selector, checked, options),
        ))
    }
}

impl WireCodec<CommandResult<SetChecked>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<SetChecked>,
    ) -> Result<EncodedParts, Self::Error> {
        require_set_checked(self.capability_version)?;
        let mut payload = BinaryWriter::new();
        payload.u8(SET_CHECKED_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(()) => payload.u8(0)?,
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<SetChecked>, Self::Error> {
        require_set_checked(self.capability_version)?;
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, SET_CHECKED_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome = match reader.u8()? {
            0 => Ok(()),
            1 => Err(self.decode_error(&mut reader)?),
            value => return Err(LocatorCodecError::InvalidOutcome(value)),
        };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<Command<AllInnerTexts>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<AllInnerTexts>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(ALL_INNER_TEXTS_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: AllInnerTexts::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<AllInnerTexts>, Self::Error> {
        command_route(&frame, AllInnerTexts::METHOD)?;
        let mut reader = payload_reader(&frame, ALL_INNER_TEXTS_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            AllInnerTextsParams::new(selector),
        ))
    }
}

impl WireCodec<Command<AllTextContents>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(
        &self,
        command: &Command<AllTextContents>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(ALL_TEXT_CONTENTS_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: AllTextContents::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<AllTextContents>, Self::Error> {
        command_route(&frame, AllTextContents::METHOD)?;
        let mut reader = payload_reader(&frame, ALL_TEXT_CONTENTS_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            AllTextContentsParams::new(selector),
        ))
    }
}

impl WireCodec<Command<Count>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<Count>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(COUNT_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: Count::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<Count>, Self::Error> {
        command_route(&frame, Count::METHOD)?;
        let mut reader = payload_reader(&frame, COUNT_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            CountParams::new(selector),
        ))
    }
}

impl WireCodec<Command<Fill>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<Fill>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(FILL_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        self.encode_string(&mut payload, &command.params().value)?;
        payload.u8(u8::from(command.params().options.force))?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: Fill::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<Fill>, Self::Error> {
        command_route(&frame, Fill::METHOD)?;
        let mut reader = payload_reader(&frame, FILL_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        let value = self.decode_string(&mut reader)?;
        let options = FillOptions::new().with_force(decode_bool(reader.u8()?)?);
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            FillParams::new(selector, value, options),
        ))
    }
}

macro_rules! impl_bool_query_command_codec {
    ($command:ty, $params:ty, $payload_kind:ident) => {
        impl WireCodec<Command<$command>> for LocatorCodec {
            type Error = LocatorCodecError;

            fn encode_parts(
                &self,
                command: &Command<$command>,
            ) -> Result<EncodedParts, Self::Error> {
                let mut payload = BinaryWriter::new();
                payload.u8($payload_kind)?;
                payload.u64(command.call_id().get())?;
                payload.u64(command.object_id().get())?;
                encode_metadata(&mut payload, command.metadata())?;
                let mut selector_nodes = 0;
                self.encode_selector(
                    &mut payload,
                    &command.params().selector,
                    0,
                    &mut selector_nodes,
                )?;
                Ok(EncodedParts::new(
                    FrameRoute::Command {
                        call_id: command.call_id(),
                        target: command.object_id(),
                        method: <$command>::METHOD,
                    },
                    payload.into_bytes(),
                    Vec::new(),
                ))
            }

            fn decode_frame(&self, frame: WireFrame) -> Result<Command<$command>, Self::Error> {
                command_route(&frame, <$command>::METHOD)?;
                let mut reader = payload_reader(&frame, $payload_kind)?;
                let call_id = CallId::new(reader.u64()?);
                let object_id = ObjectId::new(reader.u64()?);
                let metadata = decode_metadata(&mut reader)?;
                let mut selector_nodes = 0;
                let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
                reader.finish()?;
                Ok(Command::new(
                    call_id,
                    object_id,
                    metadata,
                    <$params>::new(selector),
                ))
            }
        }
    };
}

impl_bool_query_command_codec!(IsChecked, IsCheckedParams, IS_CHECKED_COMMAND);
impl_bool_query_command_codec!(IsEditable, IsEditableParams, IS_EDITABLE_COMMAND);
impl_bool_query_command_codec!(IsEnabled, IsEnabledParams, IS_ENABLED_COMMAND);

impl WireCodec<Command<IsVisible>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<IsVisible>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(IS_VISIBLE_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: IsVisible::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<IsVisible>, Self::Error> {
        command_route(&frame, IsVisible::METHOD)?;
        let mut reader = payload_reader(&frame, IS_VISIBLE_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            IsVisibleParams::new(selector),
        ))
    }
}

impl WireCodec<Command<TextContent>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<TextContent>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(TEXT_CONTENT_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: TextContent::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<TextContent>, Self::Error> {
        command_route(&frame, TextContent::METHOD)?;
        let mut reader = payload_reader(&frame, TEXT_CONTENT_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            TextContentParams::new(selector),
        ))
    }
}

impl WireCodec<Command<GetAttribute>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<GetAttribute>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(GET_ATTRIBUTE_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        self.encode_bounded_string(
            &mut payload,
            &command.params().name,
            MAX_ATTRIBUTE_NAME_BYTES,
        )?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: GetAttribute::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<GetAttribute>, Self::Error> {
        command_route(&frame, GetAttribute::METHOD)?;
        let mut reader = payload_reader(&frame, GET_ATTRIBUTE_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        let name = self.decode_bounded_string(&mut reader, MAX_ATTRIBUTE_NAME_BYTES)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            GetAttributeParams::new(selector, name),
        ))
    }
}

impl WireCodec<Command<InnerHtml>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<InnerHtml>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(INNER_HTML_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: InnerHtml::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<InnerHtml>, Self::Error> {
        command_route(&frame, InnerHtml::METHOD)?;
        let mut reader = payload_reader(&frame, INNER_HTML_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            InnerHtmlParams::new(selector),
        ))
    }
}

impl WireCodec<Command<InnerText>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<InnerText>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(INNER_TEXT_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: InnerText::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<InnerText>, Self::Error> {
        command_route(&frame, InnerText::METHOD)?;
        let mut reader = payload_reader(&frame, INNER_TEXT_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            InnerTextParams::new(selector),
        ))
    }
}

impl WireCodec<Command<InputValue>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<InputValue>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(INPUT_VALUE_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: InputValue::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<InputValue>, Self::Error> {
        command_route(&frame, InputValue::METHOD)?;
        let mut reader = payload_reader(&frame, INPUT_VALUE_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            InputValueParams::new(selector),
        ))
    }
}

impl WireCodec<Command<WaitFor>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, command: &Command<WaitFor>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(WAIT_FOR_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        payload.u8(encode_wait_for_state(command.params().options.state))?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: WaitFor::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<WaitFor>, Self::Error> {
        command_route(&frame, WaitFor::METHOD)?;
        let mut reader = payload_reader(&frame, WAIT_FOR_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self.decode_selector(&mut reader, 0, &mut selector_nodes)?;
        let options = WaitForOptions::new().with_state(decode_wait_for_state(reader.u8()?)?);
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            WaitForParams::new(selector, options),
        ))
    }
}

impl WireCodec<CommandResult<Click>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, result: &CommandResult<Click>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(CLICK_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(()) => payload.u8(0)?,
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<Click>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, CLICK_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome = match reader.u8()? {
            0 => Ok(()),
            1 => Err(self.decode_error(&mut reader)?),
            value => return Err(LocatorCodecError::InvalidOutcome(value)),
        };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<Hover>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, result: &CommandResult<Hover>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(HOVER_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(()) => payload.u8(0)?,
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<Hover>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, HOVER_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome = match reader.u8()? {
            0 => Ok(()),
            1 => Err(self.decode_error(&mut reader)?),
            value => return Err(LocatorCodecError::InvalidOutcome(value)),
        };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<AllInnerTexts>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<AllInnerTexts>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(ALL_INNER_TEXTS_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(values) => {
                validate_all_inner_texts(values, self.limits.string_bytes)?;
                payload.u8(0)?;
                payload.u32(u32::try_from(values.len()).map_err(|_| {
                    LocatorCodecError::IntegerOutOfRange {
                        field: "all_inner_texts item count",
                        value: u64::try_from(values.len()).unwrap_or(u64::MAX),
                    }
                })?)?;
                for value in values {
                    self.encode_bounded_string(
                        &mut payload,
                        value,
                        MAX_ALL_INNER_TEXTS_ITEM_BYTES,
                    )?;
                }
            }
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<AllInnerTexts>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, ALL_INNER_TEXTS_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome: Result<AllInnerTextsResult, OperationError<LocatorErrorDetails>> =
            match reader.u8()? {
                0 => {
                    let length = usize::try_from(u64::from(reader.u32()?)).map_err(|_| {
                        LocatorCodecError::IntegerOutOfRange {
                            field: "all_inner_texts item count",
                            value: u64::MAX,
                        }
                    })?;
                    if length > MAX_ALL_INNER_TEXTS_ITEMS {
                        return Err(LocatorCodecError::TooManyItems {
                            field: "all_inner_texts values",
                            actual: length,
                            limit: MAX_ALL_INNER_TEXTS_ITEMS,
                        });
                    }
                    let mut values = Vec::with_capacity(length);
                    let mut aggregate_bytes = 0usize;
                    for _ in 0..length {
                        let value = self
                            .decode_bounded_string(&mut reader, MAX_ALL_INNER_TEXTS_ITEM_BYTES)?;
                        aggregate_bytes = aggregate_bytes.saturating_add(value.len());
                        if aggregate_bytes > MAX_ALL_INNER_TEXTS_TOTAL_BYTES {
                            return Err(LocatorCodecError::AggregateStringBytesTooLarge {
                                field: "all_inner_texts values",
                                actual: aggregate_bytes,
                                limit: MAX_ALL_INNER_TEXTS_TOTAL_BYTES,
                            });
                        }
                        values.push(value);
                    }
                    Ok(values)
                }
                1 => Err(self.decode_error(&mut reader)?),
                value => return Err(LocatorCodecError::InvalidOutcome(value)),
            };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<AllTextContents>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<AllTextContents>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(ALL_TEXT_CONTENTS_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(values) => {
                validate_all_text_contents(values, self.limits.string_bytes)?;
                payload.u8(0)?;
                payload.u32(u32::try_from(values.len()).map_err(|_| {
                    LocatorCodecError::IntegerOutOfRange {
                        field: "all_text_contents item count",
                        value: u64::try_from(values.len()).unwrap_or(u64::MAX),
                    }
                })?)?;
                for value in values {
                    self.encode_bounded_string(
                        &mut payload,
                        value,
                        MAX_ALL_TEXT_CONTENTS_ITEM_BYTES,
                    )?;
                }
            }
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(
        &self,
        frame: WireFrame,
    ) -> Result<CommandResult<AllTextContents>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, ALL_TEXT_CONTENTS_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome: Result<AllTextContentsResult, OperationError<LocatorErrorDetails>> =
            match reader.u8()? {
                0 => {
                    let length = usize::try_from(u64::from(reader.u32()?)).map_err(|_| {
                        LocatorCodecError::IntegerOutOfRange {
                            field: "all_text_contents item count",
                            value: u64::MAX,
                        }
                    })?;
                    if length > MAX_ALL_TEXT_CONTENTS_ITEMS {
                        return Err(LocatorCodecError::TooManyItems {
                            field: "all_text_contents values",
                            actual: length,
                            limit: MAX_ALL_TEXT_CONTENTS_ITEMS,
                        });
                    }
                    let mut values = Vec::with_capacity(length);
                    let mut aggregate_bytes = 0usize;
                    for _ in 0..length {
                        let value = self
                            .decode_bounded_string(&mut reader, MAX_ALL_TEXT_CONTENTS_ITEM_BYTES)?;
                        aggregate_bytes = aggregate_bytes.saturating_add(value.len());
                        if aggregate_bytes > MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES {
                            return Err(LocatorCodecError::AggregateStringBytesTooLarge {
                                field: "all_text_contents values",
                                actual: aggregate_bytes,
                                limit: MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES,
                            });
                        }
                        values.push(value);
                    }
                    Ok(values)
                }
                1 => Err(self.decode_error(&mut reader)?),
                value => return Err(LocatorCodecError::InvalidOutcome(value)),
            };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<Count>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, result: &CommandResult<Count>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(COUNT_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(count) => {
                payload.u8(0)?;
                payload.u32(*count)?;
            }
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<Count>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, COUNT_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome: Result<CountResult, OperationError<LocatorErrorDetails>> = match reader.u8()? {
            0 => Ok(reader.u32()?),
            1 => Err(self.decode_error(&mut reader)?),
            value => return Err(LocatorCodecError::InvalidOutcome(value)),
        };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<Fill>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, result: &CommandResult<Fill>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(FILL_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(()) => payload.u8(0)?,
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<Fill>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, FILL_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome = match reader.u8()? {
            0 => Ok(()),
            1 => Err(self.decode_error(&mut reader)?),
            value => return Err(LocatorCodecError::InvalidOutcome(value)),
        };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

macro_rules! impl_bool_query_result_codec {
    ($command:ty, $result:ty, $payload_kind:ident) => {
        impl WireCodec<CommandResult<$command>> for LocatorCodec {
            type Error = LocatorCodecError;

            fn encode_parts(
                &self,
                result: &CommandResult<$command>,
            ) -> Result<EncodedParts, Self::Error> {
                let mut payload = BinaryWriter::new();
                payload.u8($payload_kind)?;
                payload.u64(result.call_id().get())?;
                match result.outcome() {
                    Ok(value) => {
                        payload.u8(0)?;
                        payload.u8(u8::from(*value))?;
                    }
                    Err(error) => {
                        payload.u8(1)?;
                        self.encode_error(&mut payload, error)?;
                    }
                }
                Ok(EncodedParts::new(
                    FrameRoute::Result {
                        call_id: result.call_id(),
                    },
                    payload.into_bytes(),
                    Vec::new(),
                ))
            }

            fn decode_frame(
                &self,
                frame: WireFrame,
            ) -> Result<CommandResult<$command>, Self::Error> {
                result_route(&frame)?;
                let mut reader = payload_reader(&frame, $payload_kind)?;
                let call_id = CallId::new(reader.u64()?);
                let outcome: Result<$result, OperationError<LocatorErrorDetails>> =
                    match reader.u8()? {
                        0 => Ok(decode_bool(reader.u8()?)?),
                        1 => Err(self.decode_error(&mut reader)?),
                        value => return Err(LocatorCodecError::InvalidOutcome(value)),
                    };
                reader.finish()?;
                Ok(CommandResult::new(call_id, outcome))
            }
        }
    };
}

impl_bool_query_result_codec!(IsChecked, IsCheckedResult, IS_CHECKED_RESULT);
impl_bool_query_result_codec!(IsEditable, IsEditableResult, IS_EDITABLE_RESULT);
impl_bool_query_result_codec!(IsEnabled, IsEnabledResult, IS_ENABLED_RESULT);

impl WireCodec<CommandResult<IsVisible>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, result: &CommandResult<IsVisible>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(IS_VISIBLE_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(visible) => {
                payload.u8(0)?;
                payload.u8(u8::from(*visible))?;
            }
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<IsVisible>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, IS_VISIBLE_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome: Result<IsVisibleResult, OperationError<LocatorErrorDetails>> =
            match reader.u8()? {
                0 => Ok(decode_bool(reader.u8()?)?),
                1 => Err(self.decode_error(&mut reader)?),
                value => return Err(LocatorCodecError::InvalidOutcome(value)),
            };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<TextContent>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<TextContent>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(TEXT_CONTENT_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(value) => {
                payload.u8(0)?;
                match value {
                    None => payload.u8(0)?,
                    Some(value) => {
                        payload.u8(1)?;
                        self.encode_string(&mut payload, value)?;
                    }
                }
            }
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<TextContent>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, TEXT_CONTENT_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome: Result<TextContentResult, OperationError<LocatorErrorDetails>> =
            match reader.u8()? {
                0 => Ok(match reader.u8()? {
                    0 => None,
                    1 => Some(self.decode_string(&mut reader)?),
                    value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
                }),
                1 => Err(self.decode_error(&mut reader)?),
                value => return Err(LocatorCodecError::InvalidOutcome(value)),
            };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<GetAttribute>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<GetAttribute>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(GET_ATTRIBUTE_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(value) => {
                payload.u8(0)?;
                match value {
                    None => payload.u8(0)?,
                    Some(value) => {
                        payload.u8(1)?;
                        self.encode_bounded_string(
                            &mut payload,
                            value,
                            MAX_ATTRIBUTE_RESULT_BYTES,
                        )?;
                    }
                }
            }
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<GetAttribute>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, GET_ATTRIBUTE_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome: Result<GetAttributeResult, OperationError<LocatorErrorDetails>> =
            match reader.u8()? {
                0 => Ok(match reader.u8()? {
                    0 => None,
                    1 => Some(self.decode_bounded_string(&mut reader, MAX_ATTRIBUTE_RESULT_BYTES)?),
                    value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
                }),
                1 => Err(self.decode_error(&mut reader)?),
                value => return Err(LocatorCodecError::InvalidOutcome(value)),
            };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<InnerHtml>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, result: &CommandResult<InnerHtml>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(INNER_HTML_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(value) => {
                payload.u8(0)?;
                self.encode_string(&mut payload, value)?;
            }
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<InnerHtml>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, INNER_HTML_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome: Result<InnerHtmlResult, OperationError<LocatorErrorDetails>> =
            match reader.u8()? {
                0 => Ok(self.decode_string(&mut reader)?),
                1 => Err(self.decode_error(&mut reader)?),
                value => return Err(LocatorCodecError::InvalidOutcome(value)),
            };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<InnerText>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, result: &CommandResult<InnerText>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(INNER_TEXT_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(value) => {
                payload.u8(0)?;
                self.encode_string(&mut payload, value)?;
            }
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<InnerText>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, INNER_TEXT_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome: Result<InnerTextResult, OperationError<LocatorErrorDetails>> =
            match reader.u8()? {
                0 => Ok(self.decode_string(&mut reader)?),
                1 => Err(self.decode_error(&mut reader)?),
                value => return Err(LocatorCodecError::InvalidOutcome(value)),
            };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<InputValue>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(
        &self,
        result: &CommandResult<InputValue>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(INPUT_VALUE_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(value) => {
                payload.u8(0)?;
                self.encode_bounded_string(&mut payload, value, MAX_INPUT_VALUE_RESULT_BYTES)?;
            }
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<InputValue>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, INPUT_VALUE_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome: Result<InputValueResult, OperationError<LocatorErrorDetails>> =
            match reader.u8()? {
                0 => Ok(self.decode_bounded_string(&mut reader, MAX_INPUT_VALUE_RESULT_BYTES)?),
                1 => Err(self.decode_error(&mut reader)?),
                value => return Err(LocatorCodecError::InvalidOutcome(value)),
            };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

impl WireCodec<CommandResult<WaitFor>> for LocatorCodec {
    type Error = LocatorCodecError;

    fn encode_parts(&self, result: &CommandResult<WaitFor>) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(WAIT_FOR_RESULT)?;
        payload.u64(result.call_id().get())?;
        match result.outcome() {
            Ok(()) => payload.u8(0)?,
            Err(error) => {
                payload.u8(1)?;
                self.encode_error(&mut payload, error)?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Result {
                call_id: result.call_id(),
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<WaitFor>, Self::Error> {
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, WAIT_FOR_RESULT)?;
        let call_id = CallId::new(reader.u64()?);
        let outcome = match reader.u8()? {
            0 => Ok(()),
            1 => Err(self.decode_error(&mut reader)?),
            value => return Err(LocatorCodecError::InvalidOutcome(value)),
        };
        reader.finish()?;
        Ok(CommandResult::new(call_id, outcome))
    }
}

fn validate_all_inner_texts(
    values: &[String],
    codec_string_limit: usize,
) -> Result<(), LocatorCodecError> {
    if values.len() > MAX_ALL_INNER_TEXTS_ITEMS {
        return Err(LocatorCodecError::TooManyItems {
            field: "all_inner_texts values",
            actual: values.len(),
            limit: MAX_ALL_INNER_TEXTS_ITEMS,
        });
    }
    let item_limit = codec_string_limit.min(MAX_ALL_INNER_TEXTS_ITEM_BYTES);
    let mut aggregate_bytes = 0usize;
    for value in values {
        if value.len() > item_limit {
            return Err(LocatorCodecError::StringTooLong {
                actual: value.len(),
                limit: item_limit,
            });
        }
        aggregate_bytes = aggregate_bytes.saturating_add(value.len());
        if aggregate_bytes > MAX_ALL_INNER_TEXTS_TOTAL_BYTES {
            return Err(LocatorCodecError::AggregateStringBytesTooLarge {
                field: "all_inner_texts values",
                actual: aggregate_bytes,
                limit: MAX_ALL_INNER_TEXTS_TOTAL_BYTES,
            });
        }
    }
    Ok(())
}

fn validate_all_text_contents(
    values: &[String],
    codec_string_limit: usize,
) -> Result<(), LocatorCodecError> {
    if values.len() > MAX_ALL_TEXT_CONTENTS_ITEMS {
        return Err(LocatorCodecError::TooManyItems {
            field: "all_text_contents values",
            actual: values.len(),
            limit: MAX_ALL_TEXT_CONTENTS_ITEMS,
        });
    }
    let item_limit = codec_string_limit.min(MAX_ALL_TEXT_CONTENTS_ITEM_BYTES);
    let mut aggregate_bytes = 0usize;
    for value in values {
        if value.len() > item_limit {
            return Err(LocatorCodecError::StringTooLong {
                actual: value.len(),
                limit: item_limit,
            });
        }
        aggregate_bytes = aggregate_bytes.saturating_add(value.len());
        if aggregate_bytes > MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES {
            return Err(LocatorCodecError::AggregateStringBytesTooLarge {
                field: "all_text_contents values",
                actual: aggregate_bytes,
                limit: MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES,
            });
        }
    }
    Ok(())
}

fn require_focus_management(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(1, 18)) {
        Ok(())
    } else {
        Err(LocatorCodecError::FocusManagementUnsupported { version })
    }
}

fn require_press(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(1, 19)) {
        Ok(())
    } else {
        Err(LocatorCodecError::PressUnsupported { version })
    }
}

fn require_select_option(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(1, 21)) {
        Ok(())
    } else {
        Err(LocatorCodecError::SelectOptionUnsupported { version })
    }
}

fn require_scroll_into_view_if_needed(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(1, 22)) {
        Ok(())
    } else {
        Err(LocatorCodecError::ScrollIntoViewIfNeededUnsupported { version })
    }
}

fn select_option_value_to_wire(
    value: &SelectOptionValue,
) -> generated_schema::PwrightLocatorSelectOption {
    generated_schema::PwrightLocatorSelectOption {
        value_or_label: value.value_or_label.clone(),
        value: value.value.clone(),
        label: value.label.clone(),
        index: value.index,
    }
}

fn select_option_value_from_wire(
    value: generated_schema::PwrightLocatorSelectOption,
) -> SelectOptionValue {
    SelectOptionValue {
        value_or_label: value.value_or_label,
        value: value.value,
        label: value.label,
        index: value.index,
    }
}

fn select_option_options_to_wire(
    options: SelectOptionOptions,
) -> generated_schema::PwrightLocatorSelectOptionOptions {
    generated_schema::PwrightLocatorSelectOptionOptions {
        force: options.force,
    }
}

fn select_option_options_from_wire(
    options: generated_schema::PwrightLocatorSelectOptionOptions,
) -> SelectOptionOptions {
    SelectOptionOptions {
        force: options.force,
    }
}

fn validate_select_option_descriptors(
    values: &[SelectOptionValue],
    codec_string_limit: usize,
) -> Result<(), LocatorCodecError> {
    if values.len() > MAX_SELECT_OPTION_ITEMS {
        return Err(LocatorCodecError::TooManyItems {
            field: "select_option descriptors",
            actual: values.len(),
            limit: MAX_SELECT_OPTION_ITEMS,
        });
    }
    let item_limit = MAX_SELECT_OPTION_ITEM_BYTES.min(codec_string_limit);
    let mut aggregate_bytes = 0usize;
    for value in values {
        for text in [
            value.value_or_label.as_deref(),
            value.value.as_deref(),
            value.label.as_deref(),
        ]
        .into_iter()
        .flatten()
        {
            if text.len() > item_limit {
                return Err(LocatorCodecError::StringTooLong {
                    actual: text.len(),
                    limit: item_limit,
                });
            }
            aggregate_bytes = aggregate_bytes.saturating_add(text.len());
            if aggregate_bytes > MAX_SELECT_OPTION_TOTAL_BYTES {
                return Err(LocatorCodecError::AggregateStringBytesTooLarge {
                    field: "select_option descriptors",
                    actual: aggregate_bytes,
                    limit: MAX_SELECT_OPTION_TOTAL_BYTES,
                });
            }
        }
    }
    Ok(())
}

fn validate_select_option_results(
    values: &[String],
    codec_string_limit: usize,
) -> Result<(), LocatorCodecError> {
    if values.len() > MAX_SELECT_OPTION_ITEMS {
        return Err(LocatorCodecError::TooManyItems {
            field: "select_option results",
            actual: values.len(),
            limit: MAX_SELECT_OPTION_ITEMS,
        });
    }
    let item_limit = MAX_SELECT_OPTION_ITEM_BYTES.min(codec_string_limit);
    let mut aggregate_bytes = 0usize;
    for value in values {
        if value.len() > item_limit {
            return Err(LocatorCodecError::StringTooLong {
                actual: value.len(),
                limit: item_limit,
            });
        }
        aggregate_bytes = aggregate_bytes.saturating_add(value.len());
        if aggregate_bytes > MAX_SELECT_OPTION_TOTAL_BYTES {
            return Err(LocatorCodecError::AggregateStringBytesTooLarge {
                field: "select_option results",
                actual: aggregate_bytes,
                limit: MAX_SELECT_OPTION_TOTAL_BYTES,
            });
        }
    }
    Ok(())
}

fn require_set_checked(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(1, 20)) {
        Ok(())
    } else {
        Err(LocatorCodecError::SetCheckedUnsupported { version })
    }
}

fn set_checked_options_to_wire(
    options: SetCheckedOptions,
) -> generated_schema::PwrightLocatorSetCheckedOptions {
    generated_schema::PwrightLocatorSetCheckedOptions {
        force: options.force,
        position: options
            .position
            .map(|position| generated_schema::PwrightLocatorPoint {
                x: position.x(),
                y: position.y(),
            }),
        scroll: options.scroll.map(|scroll| match scroll {
            ScrollMode::Auto => generated_schema::PwrightLocatorScrollMode::Auto,
            ScrollMode::None => generated_schema::PwrightLocatorScrollMode::None,
        }),
        trial: options.trial,
    }
}

fn set_checked_options_from_wire(
    options: generated_schema::PwrightLocatorSetCheckedOptions,
) -> Result<SetCheckedOptions, LocatorCodecError> {
    let position = options
        .position
        .map(|position| {
            Position::new(position.x, position.y)
                .map_err(|_| LocatorCodecError::InvalidPointerPosition)
        })
        .transpose()?;
    let scroll = options.scroll.map(|scroll| match scroll {
        generated_schema::PwrightLocatorScrollMode::Auto => ScrollMode::Auto,
        generated_schema::PwrightLocatorScrollMode::None => ScrollMode::None,
    });
    Ok(SetCheckedOptions {
        force: options.force,
        position,
        scroll,
        trial: options.trial,
    })
}

fn encode_press_options(
    payload: &mut BinaryWriter,
    options: PressOptions,
) -> Result<(), LocatorCodecError> {
    match options.delay {
        Some(delay) => {
            payload.u8(1)?;
            payload.u64(delay.as_secs())?;
            payload.u32(delay.subsec_nanos())?;
        }
        None => payload.u8(0)?,
    }
    payload.u8(u8::from(options.no_wait_after))?;
    Ok(())
}

fn decode_press_options(reader: &mut BinaryReader<'_>) -> Result<PressOptions, LocatorCodecError> {
    let delay = match reader.u8()? {
        0 => None,
        1 => {
            let seconds = reader.u64()?;
            let nanoseconds = reader.u32()?;
            if nanoseconds >= 1_000_000_000 {
                return Err(LocatorCodecError::InvalidDurationNanos(nanoseconds));
            }
            Some(Duration::new(seconds, nanoseconds))
        }
        value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
    };
    let no_wait_after = match reader.u8()? {
        0 => false,
        1 => true,
        value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
    };
    Ok(PressOptions {
        delay,
        no_wait_after,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocatorCodecError {
    FocusManagementUnsupported {
        version: ProtocolVersion,
    },
    PressUnsupported {
        version: ProtocolVersion,
    },
    SetCheckedUnsupported {
        version: ProtocolVersion,
    },
    SelectOptionUnsupported {
        version: ProtocolVersion,
    },
    ScrollIntoViewIfNeededUnsupported {
        version: ProtocolVersion,
    },
    InvalidPressKey(String),
    WrongRoute(FrameRoute),
    WrongMethod {
        expected: MethodTag,
        actual: MethodTag,
    },
    AttachmentsUnsupported {
        actual: usize,
    },
    WrongPayloadKind {
        expected: u8,
        actual: u8,
    },
    StringTooLong {
        actual: usize,
        limit: usize,
    },
    TooManyItems {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    AggregateStringBytesTooLarge {
        field: &'static str,
        actual: usize,
        limit: usize,
    },
    InvalidUtf8(Utf8Error),
    EmptySelector,
    InvalidSelectorKind(u8),
    InvalidUserFacingAttribute(u8),
    InvalidTestIdAttributes(TestIdAttributesError),
    InvalidTextPattern(u8),
    InvalidRegexFlags(RegexMatcherError),
    InvalidAriaRole(String),
    InvalidRoleOption {
        role: AriaRole,
        option: &'static str,
    },
    InvalidTriState(u8),
    ZeroAriaLevel,
    SelectorTooDeep {
        actual: usize,
        limit: usize,
    },
    TooManySelectorNodes {
        actual: usize,
        limit: usize,
    },
    InvalidBoolean(u8),
    InvalidOptionFlag(u8),
    InvalidWaitForState(u8),
    InvalidButton(u8),
    InvalidKeyboardModifier(u8),
    InvalidScrollMode(u8),
    TooManyKeyboardModifiers {
        actual: usize,
        limit: usize,
    },
    NonCanonicalKeyboardModifiers,
    InvalidPointerPosition,
    ZeroClickCount,
    ZeroSteps,
    InvalidFailureKind(u8),
    InvalidErrorDetails(u8),
    InvalidOutcome(u8),
    InvalidDurationNanos(u32),
    IntegerOutOfRange {
        field: &'static str,
        value: u64,
    },
    Encode(BinaryEncodeError),
    ValueEncode(BinaryValueEncodeError),
    ValueDecode(BinaryValueDecodeError),
    Truncated {
        offset: usize,
        needed: usize,
        remaining: usize,
    },
    TrailingBytes {
        remaining: usize,
    },
}

impl Display for LocatorCodecError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::FocusManagementUnsupported { version } => write!(
                f,
                "locator focus and blur require capability version 1.18 or 2.1, but {version} was selected"
            ),
            Self::PressUnsupported { version } => write!(
                f,
                "locator press requires capability version 1.19 or 2.2, but {version} was selected"
            ),
            Self::SetCheckedUnsupported { version } => write!(
                f,
                "locator set_checked requires capability version 1.20 or 2.3, but {version} was selected"
            ),
            Self::SelectOptionUnsupported { version } => write!(
                f,
                "locator select_option requires capability version 1.21 or 2.4, but {version} was selected"
            ),
            Self::ScrollIntoViewIfNeededUnsupported { version } => write!(
                f,
                "locator scroll_into_view_if_needed requires capability version 1.22 or 2.5, but {version} was selected"
            ),
            Self::InvalidPressKey(message) => write!(f, "invalid locator press key: {message}"),
            Self::WrongRoute(route) => write!(f, "locator payload arrived on {route:?}"),
            Self::WrongMethod { expected, actual } => {
                write!(f, "expected locator method {expected}, received {actual}")
            }
            Self::AttachmentsUnsupported { actual } => write!(
                f,
                "locator payload has {actual} attachment(s), but none are allowed"
            ),
            Self::WrongPayloadKind { expected, actual } => {
                write!(f, "expected payload kind {expected}, received {actual}")
            }
            Self::StringTooLong { actual, limit } => {
                write!(f, "string has {actual} bytes, exceeding limit {limit}")
            }
            Self::TooManyItems {
                field,
                actual,
                limit,
            } => write!(f, "{field} has {actual} items, exceeding limit {limit}"),
            Self::AggregateStringBytesTooLarge {
                field,
                actual,
                limit,
            } => write!(
                f,
                "{field} contains {actual} string bytes, exceeding limit {limit}"
            ),
            Self::InvalidUtf8(error) => write!(f, "string is not UTF-8: {error}"),
            Self::EmptySelector => f.write_str("CSS selector must not be empty"),
            Self::InvalidSelectorKind(value) => write!(f, "invalid selector kind {value}"),
            Self::InvalidUserFacingAttribute(value) => {
                write!(f, "invalid user-facing attribute kind {value}")
            }
            Self::InvalidTestIdAttributes(error) => Display::fmt(error, f),
            Self::InvalidTextPattern(value) => write!(f, "invalid text-pattern kind {value}"),
            Self::InvalidRegexFlags(error) => Display::fmt(error, f),
            Self::InvalidAriaRole(role) => write!(f, "invalid ARIA role `{role}`"),
            Self::InvalidRoleOption { role, option } => {
                write!(f, "role `{role}` does not support `{option}` filtering")
            }
            Self::InvalidTriState(value) => write!(f, "invalid ARIA tri-state {value}"),
            Self::ZeroAriaLevel => f.write_str("ARIA level must be nonzero"),
            Self::SelectorTooDeep { actual, limit } => {
                write!(f, "selector depth {actual} exceeds limit {limit}")
            }
            Self::TooManySelectorNodes { actual, limit } => {
                write!(f, "selector node count {actual} exceeds limit {limit}")
            }
            Self::InvalidBoolean(value) => write!(f, "invalid boolean value {value}"),
            Self::InvalidOptionFlag(value) => write!(f, "invalid option flag {value}"),
            Self::InvalidWaitForState(value) => write!(f, "invalid wait-for state {value}"),
            Self::InvalidButton(value) => write!(f, "invalid mouse button {value}"),
            Self::InvalidKeyboardModifier(value) => {
                write!(f, "invalid keyboard modifier {value}")
            }
            Self::InvalidScrollMode(value) => write!(f, "invalid scroll mode {value}"),
            Self::TooManyKeyboardModifiers { actual, limit } => write!(
                f,
                "pointer action has {actual} keyboard modifiers, exceeding limit {limit}"
            ),
            Self::NonCanonicalKeyboardModifiers => {
                f.write_str("pointer-action modifiers are not in canonical unique order")
            }
            Self::InvalidPointerPosition => {
                f.write_str("pointer position coordinates must be finite")
            }
            Self::ZeroClickCount => f.write_str("click count must be nonzero"),
            Self::ZeroSteps => f.write_str("mouse-move steps must be nonzero"),
            Self::InvalidFailureKind(value) => write!(f, "invalid failure kind {value}"),
            Self::InvalidErrorDetails(value) => {
                write!(f, "invalid locator error-details kind {value}")
            }
            Self::InvalidOutcome(value) => write!(f, "invalid result outcome {value}"),
            Self::InvalidDurationNanos(value) => {
                write!(f, "duration nanoseconds {value} are not canonical")
            }
            Self::IntegerOutOfRange { field, value } => {
                write!(f, "wire value {value} does not fit {field}")
            }
            Self::Encode(error) => Display::fmt(error, f),
            Self::ValueEncode(error) => Display::fmt(error, f),
            Self::ValueDecode(error) => Display::fmt(error, f),
            Self::Truncated {
                offset,
                needed,
                remaining,
            } => write!(
                f,
                "locator payload at byte {offset} needs {needed} bytes but only {remaining} remain"
            ),
            Self::TrailingBytes { remaining } => {
                write!(f, "locator payload has {remaining} trailing bytes")
            }
        }
    }
}

impl Error for LocatorCodecError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidUtf8(error) => Some(error),
            Self::InvalidRegexFlags(error) => Some(error),
            Self::InvalidTestIdAttributes(error) => Some(error),
            Self::Encode(error) => Some(error),
            Self::ValueEncode(error) => Some(error),
            Self::ValueDecode(error) => Some(error),
            _ => None,
        }
    }
}

impl From<BinaryEncodeError> for LocatorCodecError {
    fn from(error: BinaryEncodeError) -> Self {
        Self::Encode(error)
    }
}

impl From<BinaryValueEncodeError> for LocatorCodecError {
    fn from(error: BinaryValueEncodeError) -> Self {
        Self::ValueEncode(error)
    }
}

impl From<BinaryValueDecodeError> for LocatorCodecError {
    fn from(error: BinaryValueDecodeError) -> Self {
        Self::ValueDecode(error)
    }
}

impl From<BinaryDecodeError> for LocatorCodecError {
    fn from(error: BinaryDecodeError) -> Self {
        match error {
            BinaryDecodeError::IntegerOutOfRange { field, value } => {
                Self::IntegerOutOfRange { field, value }
            }
            BinaryDecodeError::Truncated {
                offset,
                needed,
                remaining,
            } => Self::Truncated {
                offset,
                needed,
                remaining,
            },
            BinaryDecodeError::TrailingBytes { remaining } => Self::TrailingBytes { remaining },
        }
    }
}

fn command_route(frame: &WireFrame, expected: MethodTag) -> Result<(), LocatorCodecError> {
    let FrameRoute::Command { method, .. } = frame.route() else {
        return Err(LocatorCodecError::WrongRoute(frame.route()));
    };
    if method != expected {
        return Err(LocatorCodecError::WrongMethod {
            expected,
            actual: method,
        });
    }
    Ok(())
}

fn result_route(frame: &WireFrame) -> Result<(), LocatorCodecError> {
    if !matches!(frame.route(), FrameRoute::Result { .. }) {
        return Err(LocatorCodecError::WrongRoute(frame.route()));
    }
    Ok(())
}

fn payload_reader<'a>(
    frame: &'a WireFrame,
    expected: u8,
) -> Result<BinaryReader<'a>, LocatorCodecError> {
    if !frame.attachments().is_empty() {
        return Err(LocatorCodecError::AttachmentsUnsupported {
            actual: frame.attachments().len(),
        });
    }
    let mut reader = BinaryReader::new(frame.payload());
    let actual = reader.u8()?;
    if actual != expected {
        return Err(LocatorCodecError::WrongPayloadKind { expected, actual });
    }
    Ok(reader)
}

fn encode_metadata(
    payload: &mut BinaryWriter,
    metadata: CallMetadata,
) -> Result<(), LocatorCodecError> {
    match metadata.timeout {
        Some(timeout) => {
            payload.u8(1)?;
            payload.u64(timeout.as_secs())?;
            payload.u32(timeout.subsec_nanos())?;
        }
        None => payload.u8(0)?,
    }
    match metadata.cancellation {
        Some(id) => {
            payload.u8(1)?;
            payload.u64(id.get())?;
        }
        None => payload.u8(0)?,
    }
    Ok(())
}

fn decode_metadata(reader: &mut BinaryReader<'_>) -> Result<CallMetadata, LocatorCodecError> {
    let timeout = match reader.u8()? {
        0 => None,
        1 => {
            let seconds = reader.u64()?;
            let nanos = reader.u32()?;
            if nanos >= 1_000_000_000 {
                return Err(LocatorCodecError::InvalidDurationNanos(nanos));
            }
            Some(Duration::new(seconds, nanos))
        }
        value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
    };
    let cancellation = match reader.u8()? {
        0 => None,
        1 => Some(CancellationId::new(reader.u64()?)),
        value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
    };
    Ok(CallMetadata::new(timeout, cancellation))
}

fn encode_hover_options(
    payload: &mut BinaryWriter,
    options: HoverOptions,
) -> Result<(), LocatorCodecError> {
    payload.u8(u8::from(options.force))?;
    payload.u8(match options.scroll {
        ScrollMode::Auto => 0,
        ScrollMode::None => 1,
    })?;
    payload.u8(u8::from(options.modifiers.is_some()))?;
    if let Some(modifiers) = options.modifiers {
        payload
            .u32(u32::try_from(modifiers.len()).expect("modifier set has at most five members"))?;
        for modifier in modifiers.iter() {
            payload.u8(encode_keyboard_modifier(modifier))?;
        }
    }
    payload.u8(u8::from(options.position.is_some()))?;
    if let Some(position) = options.position {
        payload.f64(position.x())?;
        payload.f64(position.y())?;
    }
    payload.u8(u8::from(options.trial))?;
    Ok(())
}

fn decode_hover_options(reader: &mut BinaryReader<'_>) -> Result<HoverOptions, LocatorCodecError> {
    let force = decode_bool(reader.u8()?)?;
    let scroll = match reader.u8()? {
        0 => ScrollMode::Auto,
        1 => ScrollMode::None,
        value => return Err(LocatorCodecError::InvalidScrollMode(value)),
    };
    let modifiers = match reader.u8()? {
        0 => None,
        1 => {
            let count = usize::try_from(reader.u32()?).map_err(|_| {
                LocatorCodecError::TooManyKeyboardModifiers {
                    actual: usize::MAX,
                    limit: 5,
                }
            })?;
            if count > 5 {
                return Err(LocatorCodecError::TooManyKeyboardModifiers {
                    actual: count,
                    limit: 5,
                });
            }
            reader.ensure(count)?;
            let mut modifiers = KeyboardModifiers::new();
            let mut previous = None;
            for _ in 0..count {
                let tag = reader.u8()?;
                if previous.is_some_and(|previous| previous >= tag) {
                    return Err(LocatorCodecError::NonCanonicalKeyboardModifiers);
                }
                modifiers = modifiers.with(decode_keyboard_modifier(tag)?);
                previous = Some(tag);
            }
            Some(modifiers)
        }
        value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
    };
    let position = match reader.u8()? {
        0 => None,
        1 => Some(
            Position::new(reader.f64()?, reader.f64()?)
                .map_err(|_| LocatorCodecError::InvalidPointerPosition)?,
        ),
        value => return Err(LocatorCodecError::InvalidOptionFlag(value)),
    };
    let trial = decode_bool(reader.u8()?)?;
    Ok(HoverOptions {
        force,
        scroll,
        modifiers,
        position,
        trial,
    })
}

fn encode_keyboard_modifier(modifier: KeyboardModifier) -> u8 {
    match modifier {
        KeyboardModifier::Alt => 0,
        KeyboardModifier::Control => 1,
        KeyboardModifier::ControlOrMeta => 2,
        KeyboardModifier::Meta => 3,
        KeyboardModifier::Shift => 4,
    }
}

fn decode_keyboard_modifier(value: u8) -> Result<KeyboardModifier, LocatorCodecError> {
    match value {
        0 => Ok(KeyboardModifier::Alt),
        1 => Ok(KeyboardModifier::Control),
        2 => Ok(KeyboardModifier::ControlOrMeta),
        3 => Ok(KeyboardModifier::Meta),
        4 => Ok(KeyboardModifier::Shift),
        value => Err(LocatorCodecError::InvalidKeyboardModifier(value)),
    }
}

fn encode_button(button: MouseButton) -> u8 {
    match button {
        MouseButton::Left => 0,
        MouseButton::Right => 1,
        MouseButton::Middle => 2,
    }
}
fn encode_wait_for_state(state: WaitForState) -> u8 {
    match state {
        WaitForState::Attached => 0,
        WaitForState::Detached => 1,
        WaitForState::Visible => 2,
        WaitForState::Hidden => 3,
    }
}
fn decode_wait_for_state(value: u8) -> Result<WaitForState, LocatorCodecError> {
    match value {
        0 => Ok(WaitForState::Attached),
        1 => Ok(WaitForState::Detached),
        2 => Ok(WaitForState::Visible),
        3 => Ok(WaitForState::Hidden),
        value => Err(LocatorCodecError::InvalidWaitForState(value)),
    }
}
fn encode_user_facing_attribute(attribute: UserFacingAttribute) -> u8 {
    match attribute {
        UserFacingAttribute::Placeholder => 0,
        UserFacingAttribute::Alt => 1,
        UserFacingAttribute::Title => 2,
    }
}
fn decode_user_facing_attribute(value: u8) -> Result<UserFacingAttribute, LocatorCodecError> {
    match value {
        0 => Ok(UserFacingAttribute::Placeholder),
        1 => Ok(UserFacingAttribute::Alt),
        2 => Ok(UserFacingAttribute::Title),
        value => Err(LocatorCodecError::InvalidUserFacingAttribute(value)),
    }
}
fn decode_button(value: u8) -> Result<MouseButton, LocatorCodecError> {
    match value {
        0 => Ok(MouseButton::Left),
        1 => Ok(MouseButton::Right),
        2 => Ok(MouseButton::Middle),
        value => Err(LocatorCodecError::InvalidButton(value)),
    }
}
fn decode_bool(value: u8) -> Result<bool, LocatorCodecError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        value => Err(LocatorCodecError::InvalidBoolean(value)),
    }
}
fn encode_optional_bool(
    payload: &mut BinaryWriter,
    value: Option<bool>,
) -> Result<(), LocatorCodecError> {
    payload.u8(match value {
        None => 0,
        Some(false) => 1,
        Some(true) => 2,
    })?;
    Ok(())
}
fn decode_optional_bool(reader: &mut BinaryReader<'_>) -> Result<Option<bool>, LocatorCodecError> {
    match reader.u8()? {
        0 => Ok(None),
        1 => Ok(Some(false)),
        2 => Ok(Some(true)),
        value => Err(LocatorCodecError::InvalidOptionFlag(value)),
    }
}
fn encode_optional_tri_state(
    payload: &mut BinaryWriter,
    value: Option<AriaTriState>,
) -> Result<(), LocatorCodecError> {
    payload.u8(match value {
        None => 0,
        Some(AriaTriState::False) => 1,
        Some(AriaTriState::True) => 2,
        Some(AriaTriState::Mixed) => 3,
    })?;
    Ok(())
}
fn decode_optional_tri_state(
    reader: &mut BinaryReader<'_>,
) -> Result<Option<AriaTriState>, LocatorCodecError> {
    match reader.u8()? {
        0 => Ok(None),
        1 => Ok(Some(AriaTriState::False)),
        2 => Ok(Some(AriaTriState::True)),
        3 => Ok(Some(AriaTriState::Mixed)),
        value => Err(LocatorCodecError::InvalidTriState(value)),
    }
}
fn encode_failure_kind(value: FailureKind) -> u8 {
    match value {
        FailureKind::Timeout => 0,
        FailureKind::Cancelled => 1,
        FailureKind::TargetClosed => 2,
        FailureKind::BrowserCrashed => 3,
        FailureKind::UnsupportedCapability => 4,
        FailureKind::ResourceExhausted => 5,
        FailureKind::TransportClosed => 6,
        FailureKind::ProtocolViolation => 7,
        FailureKind::Backend => 8,
        FailureKind::Cleanup => 9,
    }
}
fn decode_failure_kind(value: u8) -> Result<FailureKind, LocatorCodecError> {
    match value {
        0 => Ok(FailureKind::Timeout),
        1 => Ok(FailureKind::Cancelled),
        2 => Ok(FailureKind::TargetClosed),
        3 => Ok(FailureKind::BrowserCrashed),
        4 => Ok(FailureKind::UnsupportedCapability),
        5 => Ok(FailureKind::ResourceExhausted),
        6 => Ok(FailureKind::TransportClosed),
        7 => Ok(FailureKind::ProtocolViolation),
        8 => Ok(FailureKind::Backend),
        9 => Ok(FailureKind::Cleanup),
        value => Err(LocatorCodecError::InvalidFailureKind(value)),
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;
    use std::time::Duration;

    use pwright_cap_locator::{
        ALL_INNER_TEXTS_COMMAND_ID, ALL_INNER_TEXTS_METHOD, ALL_TEXT_CONTENTS_COMMAND_ID,
        ALL_TEXT_CONTENTS_METHOD, AllInnerTexts, AllInnerTextsParams, AllTextContents,
        AllTextContentsParams, AriaRole, AriaTriState, BLUR_COMMAND_ID, BLUR_METHOD, Blur,
        BlurParams, CLICK_COMMAND_ID, CLICK_METHOD, COUNT_COMMAND_ID, COUNT_METHOD, Click,
        ClickOptions, ClickParams, Count, CountParams, CssSelector, FILL_COMMAND_ID, FILL_METHOD,
        FOCUS_COMMAND_ID, FOCUS_METHOD, Fill, FillOptions, FillParams, Focus, FocusParams,
        GET_ATTRIBUTE_COMMAND_ID, GET_ATTRIBUTE_METHOD, GetAttribute, GetAttributeParams,
        HOVER_COMMAND_ID, HOVER_METHOD, Hover, HoverOptions, HoverParams, INNER_HTML_COMMAND_ID,
        INNER_HTML_METHOD, INNER_TEXT_COMMAND_ID, INNER_TEXT_METHOD, INPUT_VALUE_COMMAND_ID,
        INPUT_VALUE_METHOD, IS_CHECKED_COMMAND_ID, IS_CHECKED_METHOD, IS_EDITABLE_COMMAND_ID,
        IS_EDITABLE_METHOD, IS_ENABLED_COMMAND_ID, IS_ENABLED_METHOD, IS_VISIBLE_COMMAND_ID,
        IS_VISIBLE_METHOD, InnerHtml, InnerHtmlParams, InnerText, InnerTextParams, InputValue,
        InputValueParams, IsChecked, IsCheckedParams, IsEditable, IsEditableParams, IsEnabled,
        IsEnabledParams, IsVisible, IsVisibleParams, KeyboardModifier, KeyboardModifiers,
        LocatorErrorDetails, MouseButton, PRESS_COMMAND_ID, PRESS_METHOD, Position, Press,
        PressKey, PressOptions, PressParams, RegexMatcher, RegexMatcherError, RoleOptions,
        SCROLL_INTO_VIEW_IF_NEEDED_COMMAND_ID, SCROLL_INTO_VIEW_IF_NEEDED_METHOD,
        SELECT_OPTION_COMMAND_ID, SELECT_OPTION_METHOD, SET_CHECKED_COMMAND_ID, SET_CHECKED_METHOD,
        ScrollMode, SelectOption, SelectOptionOptions, SelectOptionParams, SelectOptionValue,
        Selector, SetChecked, SetCheckedOptions, SetCheckedParams, TEXT_CONTENT_COMMAND_ID,
        TEXT_CONTENT_METHOD, TestIdAttributes, TestIdAttributesError, TextContent,
        TextContentParams, UserFacingAttribute, WAIT_FOR_COMMAND_ID, WAIT_FOR_METHOD, WaitFor,
        WaitForOptions, WaitForParams, WaitForState,
    };
    use pwright_codec_binary::{BinaryReader, BinaryWriter};
    use pwright_core::{
        CallId, CallMetadata, CancellationId, Command, CommandResult, CommandSpec, FailureKind,
        MethodTag, ObjectId, OperationError, ProtocolVersion,
    };
    use pwright_transport::{
        FrameLimits, FrameRoute, VersionPolicy, WireCodec, WireFrame, WireProtocol,
    };

    use super::{
        ALL_INNER_TEXTS_COMMAND, ALL_INNER_TEXTS_RESULT, ALL_TEXT_CONTENTS_COMMAND,
        ALL_TEXT_CONTENTS_RESULT, BLUR_COMMAND, BLUR_RESULT, CLICK_COMMAND, CLICK_RESULT,
        COUNT_COMMAND, COUNT_RESULT, FILL_COMMAND, FILL_RESULT, FOCUS_COMMAND, FOCUS_RESULT,
        GET_ATTRIBUTE_COMMAND, GET_ATTRIBUTE_RESULT, HOVER_COMMAND, HOVER_RESULT,
        INNER_HTML_COMMAND, INNER_HTML_RESULT, INNER_TEXT_COMMAND, INNER_TEXT_RESULT,
        INPUT_VALUE_COMMAND, INPUT_VALUE_RESULT, IS_CHECKED_COMMAND, IS_CHECKED_RESULT,
        IS_EDITABLE_COMMAND, IS_EDITABLE_RESULT, IS_ENABLED_COMMAND, IS_ENABLED_RESULT,
        IS_VISIBLE_COMMAND, IS_VISIBLE_RESULT, LOCATOR_COMMAND_SCHEMAS, LOCATOR_SCHEMA,
        LOCATOR_SCHEMA_CATALOG, LocatorCodec, LocatorCodecError, LocatorCodecLimits,
        MAX_ALL_INNER_TEXTS_ITEM_BYTES, MAX_ALL_INNER_TEXTS_ITEMS, MAX_ALL_INNER_TEXTS_TOTAL_BYTES,
        MAX_ALL_TEXT_CONTENTS_ITEM_BYTES, MAX_ALL_TEXT_CONTENTS_ITEMS,
        MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES, MAX_ATTRIBUTE_NAME_BYTES, MAX_SELECT_OPTION_ITEM_BYTES,
        PRESS_COMMAND, PRESS_RESULT, SCROLL_INTO_VIEW_IF_NEEDED_COMMAND,
        SCROLL_INTO_VIEW_IF_NEEDED_RESULT, SELECT_OPTION_COMMAND, SELECT_OPTION_RESULT,
        SET_CHECKED_COMMAND, SET_CHECKED_RESULT, TEXT_CONTENT_COMMAND, TEXT_CONTENT_RESULT,
        WAIT_FOR_COMMAND, WAIT_FOR_RESULT, generated_schema, set_checked_options_from_wire,
    };

    fn codec() -> LocatorCodec {
        LocatorCodec::new(LocatorCodecLimits::new(128))
    }

    fn protocol() -> WireProtocol<LocatorCodec> {
        WireProtocol::new(
            codec(),
            VersionPolicy::new(ProtocolVersion::new(1, 0), ProtocolVersion::new(1, 0))
                .expect("version policy should build"),
            ProtocolVersion::new(1, 0),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .expect("protocol should build")
    }

    fn command() -> Command<Click> {
        Command::new(
            CallId::new(7),
            ObjectId::new(3),
            CallMetadata::new(Some(Duration::new(5, 9)), Some(CancellationId::new(11))),
            ClickParams::new(
                Selector::role(
                    AriaRole::Dialog,
                    RoleOptions::new().with_name("Settings", true),
                )
                .expect("valid dialog role")
                .within(
                    Selector::role(
                        AriaRole::Button,
                        RoleOptions::new()
                            .with_name_regex(
                                RegexMatcher::new("^Save\\s+changes$", "i").expect("valid regex"),
                            )
                            .with_pressed(AriaTriState::False),
                    )
                    .expect("valid button role"),
                )
                .and(Selector::label_regex(
                    RegexMatcher::new("Primary\\s+action", "i").expect("valid regex"),
                ))
                .filter_text("Save", false)
                .filter_not_text("Discard", false)
                .filter_text_regex(RegexMatcher::new("Save", "i").expect("valid regex"))
                .filter_not_text_regex(RegexMatcher::new("Discard", "i").expect("valid regex"))
                .filter_has(Selector::css(".icon").expect("valid CSS"))
                .filter_has_not(Selector::css("[hidden]").expect("valid CSS"))
                .filter_visible(true)
                .or(Selector::text_regex(
                    RegexMatcher::new("^Fallback$", "").expect("valid regex"),
                ))
                .nth(-1),
                ClickOptions::new()
                    .with_button(MouseButton::Right)
                    .with_click_count(NonZeroU8::new(2).expect("two is nonzero"))
                    .with_force(true)
                    .with_trial(true),
            ),
        )
    }

    fn fill_command() -> Command<Fill> {
        Command::new(
            CallId::new(19),
            ObjectId::new(3),
            CallMetadata::new(Some(Duration::from_secs(2)), Some(CancellationId::new(20))),
            FillParams::new(
                Selector::label("Email address", true),
                "user@example.com",
                FillOptions::new().with_force(true),
            ),
        )
    }

    fn rebuild(frame: &WireFrame, payload: Vec<u8>, attachments: Vec<Vec<u8>>) -> WireFrame {
        WireFrame::new(
            frame.version(),
            frame.route(),
            payload,
            attachments,
            FrameLimits::new(1024, 1, 16, 1024),
        )
        .expect("mutated frame should fit")
    }

    fn decode_command(frame: WireFrame) -> Result<Command<Click>, LocatorCodecError> {
        <LocatorCodec as WireCodec<Command<Click>>>::decode_frame(&codec(), frame)
    }

    #[test]
    fn command_round_trips_route_metadata_selector_and_options() {
        // Arrange
        let protocol = protocol();

        // Act
        let decoded = protocol
            .decode::<Command<Click>>(protocol.encode(&command()).expect("command should encode"))
            .expect("command should decode");

        // Assert
        assert_eq!(decoded.call_id(), CallId::new(7));
        assert_eq!(decoded.object_id(), ObjectId::new(3));
        assert_eq!(decoded.metadata(), command().metadata());
        assert_eq!(decoded.params(), command().params());
    }

    #[test]
    fn hover_command_and_result_round_trip_fractional_position_and_modifiers() {
        // Arrange
        let protocol = protocol();
        let options = HoverOptions::new()
            .with_force(true)
            .with_scroll(ScrollMode::None)
            .with_modifier(KeyboardModifier::ControlOrMeta)
            .with_modifier(KeyboardModifier::Shift)
            .with_position(Position::new(12.5, 3.25).expect("finite position should build"))
            .with_trial(true);
        let command = Command::<Hover>::new(
            CallId::new(8),
            ObjectId::new(4),
            CallMetadata::new(Some(Duration::from_secs(3)), Some(CancellationId::new(12))),
            HoverParams::new(
                Selector::css("button").expect("selector should build"),
                options,
            ),
        );
        let result = CommandResult::<Hover>::new(CallId::new(8), Ok(()));

        // Act
        let decoded_command = protocol
            .decode::<Command<Hover>>(protocol.encode(&command).expect("command should encode"))
            .expect("command should decode");
        let decoded_result = protocol
            .decode::<CommandResult<Hover>>(protocol.encode(&result).expect("result should encode"))
            .expect("result should decode");

        // Assert
        assert_eq!(decoded_command.call_id(), CallId::new(8));
        assert_eq!(decoded_command.object_id(), ObjectId::new(4));
        assert_eq!(decoded_command.metadata(), command.metadata());
        assert_eq!(decoded_command.params(), command.params());
        assert_eq!(decoded_result.call_id(), result.call_id());
        assert_eq!(decoded_result.outcome(), &Ok(()));
    }

    #[test]
    fn focus_and_blur_commands_and_results_round_trip_independently() {
        // Arrange
        let protocol = WireProtocol::new_exact_directional(
            codec().with_capability_version(ProtocolVersion::new(1, 18)),
            ProtocolVersion::new(1, 18),
            FrameLimits::new(1024, 0, 0, 1024),
            FrameLimits::new(1024, 0, 0, 1024),
        );
        let metadata =
            CallMetadata::new(Some(Duration::from_secs(3)), Some(CancellationId::new(42)));
        let focus = Command::<Focus>::new(
            CallId::new(21),
            ObjectId::new(22),
            metadata,
            FocusParams::new(Selector::test_id("focus-target")),
        );
        let blur = Command::<Blur>::new(
            CallId::new(23),
            ObjectId::new(24),
            metadata,
            BlurParams::new(Selector::css("#blur-target").expect("valid CSS")),
        );
        let focus_result = CommandResult::<Focus>::new(CallId::new(21), Ok(()));
        let blur_result = CommandResult::<Blur>::new(CallId::new(23), Ok(()));

        // Act
        let decoded_focus = protocol
            .decode::<Command<Focus>>(protocol.encode(&focus).expect("focus should encode"))
            .expect("focus should decode");
        let decoded_blur = protocol
            .decode::<Command<Blur>>(protocol.encode(&blur).expect("blur should encode"))
            .expect("blur should decode");
        let decoded_focus_result = protocol
            .decode::<CommandResult<Focus>>(
                protocol
                    .encode(&focus_result)
                    .expect("focus result should encode"),
            )
            .expect("focus result should decode");
        let decoded_blur_result = protocol
            .decode::<CommandResult<Blur>>(
                protocol
                    .encode(&blur_result)
                    .expect("blur result should encode"),
            )
            .expect("blur result should decode");

        // Assert
        assert_eq!(decoded_focus.call_id(), focus.call_id());
        assert_eq!(decoded_focus.object_id(), focus.object_id());
        assert_eq!(decoded_focus.metadata(), focus.metadata());
        assert_eq!(decoded_focus.params(), focus.params());
        assert_eq!(decoded_blur.call_id(), blur.call_id());
        assert_eq!(decoded_blur.object_id(), blur.object_id());
        assert_eq!(decoded_blur.metadata(), blur.metadata());
        assert_eq!(decoded_blur.params(), blur.params());
        assert_eq!(decoded_focus_result.call_id(), focus_result.call_id());
        assert_eq!(decoded_focus_result.outcome(), focus_result.outcome());
        assert_eq!(decoded_blur_result.call_id(), blur_result.call_id());
        assert_eq!(decoded_blur_result.outcome(), blur_result.outcome());
        assert_eq!(FOCUS_COMMAND, 33);
        assert_eq!(FOCUS_RESULT, 34);
        assert_eq!(BLUR_COMMAND, 35);
        assert_eq!(BLUR_RESULT, 36);
        assert_eq!(PRESS_COMMAND, 37);
        assert_eq!(PRESS_RESULT, 38);
        assert_eq!(Focus::METHOD, FOCUS_METHOD);
        assert_eq!(Blur::METHOD, BLUR_METHOD);
        assert_eq!(FOCUS_COMMAND_ID.as_str(), "locator.focus");
        assert_eq!(BLUR_COMMAND_ID.as_str(), "locator.blur");
    }

    #[test]
    fn locator_one_seventeen_rejects_focus_management_before_encoding() {
        // Arrange
        let old_codec = codec().with_capability_version(ProtocolVersion::new(1, 17));
        let focus = Command::<Focus>::new(
            CallId::new(31),
            ObjectId::new(32),
            CallMetadata::default(),
            FocusParams::new(Selector::test_id("focus-target")),
        );
        let blur = Command::<Blur>::new(
            CallId::new(33),
            ObjectId::new(34),
            CallMetadata::default(),
            BlurParams::new(Selector::test_id("blur-target")),
        );

        // Act
        let focus_error =
            <LocatorCodec as WireCodec<Command<Focus>>>::encode_parts(&old_codec, &focus)
                .expect_err("1.17 must reject focus");
        let blur_error =
            <LocatorCodec as WireCodec<Command<Blur>>>::encode_parts(&old_codec, &blur)
                .expect_err("1.17 must reject blur");

        // Assert
        assert_eq!(
            focus_error,
            LocatorCodecError::FocusManagementUnsupported {
                version: ProtocolVersion::new(1, 17),
            }
        );
        assert_eq!(blur_error, focus_error);
    }

    #[test]
    fn press_command_round_trips_and_one_eighteen_rejects_it() {
        // Arrange
        let current = WireProtocol::new_exact_directional(
            codec().with_capability_version(ProtocolVersion::new(1, 19)),
            ProtocolVersion::new(1, 19),
            FrameLimits::new(1024, 0, 0, 1024),
            FrameLimits::new(1024, 0, 0, 1024),
        );
        let command = Command::<Press>::new(
            CallId::new(35),
            ObjectId::new(36),
            CallMetadata::new(Some(Duration::from_secs(2)), Some(CancellationId::new(37))),
            PressParams::new(
                Selector::test_id("editor"),
                PressKey::new("ControlOrMeta+Shift++").expect("key should build"),
                PressOptions::new()
                    .with_delay(Duration::from_millis(25))
                    .with_no_wait_after(true),
            ),
        );
        let result = CommandResult::<Press>::new(CallId::new(35), Ok(()));
        let old = codec().with_capability_version(ProtocolVersion::new(1, 18));

        // Act
        let decoded = current
            .decode::<Command<Press>>(current.encode(&command).expect("press should encode"))
            .expect("press should decode");
        let decoded_result = current
            .decode::<CommandResult<Press>>(
                current.encode(&result).expect("press result should encode"),
            )
            .expect("press result should decode");
        let old_error = <LocatorCodec as WireCodec<Command<Press>>>::encode_parts(&old, &command)
            .expect_err("1.18 must reject press");

        // Assert
        assert_eq!(decoded.call_id(), command.call_id());
        assert_eq!(decoded.object_id(), command.object_id());
        assert_eq!(decoded.metadata(), command.metadata());
        assert_eq!(decoded.params(), command.params());
        assert_eq!(decoded_result.outcome(), &Ok(()));
        assert_eq!(
            old_error,
            LocatorCodecError::PressUnsupported {
                version: ProtocolVersion::new(1, 18),
            }
        );
    }

    #[test]
    fn set_checked_command_round_trips_and_one_nineteen_rejects_it() {
        // Arrange
        let current = WireProtocol::new_exact_directional(
            codec().with_capability_version(ProtocolVersion::new(1, 20)),
            ProtocolVersion::new(1, 20),
            FrameLimits::new(1024, 0, 0, 1024),
            FrameLimits::new(1024, 0, 0, 1024),
        );
        let command = Command::<SetChecked>::new(
            CallId::new(39),
            ObjectId::new(40),
            CallMetadata::new(Some(Duration::from_secs(2)), Some(CancellationId::new(41))),
            SetCheckedParams::new(
                Selector::label("Remember me", true),
                true,
                SetCheckedOptions::new()
                    .with_force(false)
                    .with_position(Position::new(2.5, 4.0).expect("finite point"))
                    .with_scroll(ScrollMode::None)
                    .with_trial(false),
            ),
        );
        let result = CommandResult::<SetChecked>::new(CallId::new(39), Ok(()));
        let old = codec().with_capability_version(ProtocolVersion::new(1, 19));

        // Act
        let decoded = current
            .decode::<Command<SetChecked>>(
                current.encode(&command).expect("SetChecked should encode"),
            )
            .expect("SetChecked should decode");
        let decoded_result = current
            .decode::<CommandResult<SetChecked>>(
                current
                    .encode(&result)
                    .expect("SetChecked result should encode"),
            )
            .expect("SetChecked result should decode");
        let old_error =
            <LocatorCodec as WireCodec<Command<SetChecked>>>::encode_parts(&old, &command)
                .expect_err("1.19 must reject SetChecked");

        // Assert
        assert_eq!(decoded.call_id(), command.call_id());
        assert_eq!(decoded.object_id(), command.object_id());
        assert_eq!(decoded.metadata(), command.metadata());
        assert_eq!(decoded.params(), command.params());
        assert_eq!(decoded_result.outcome(), &Ok(()));
        assert_eq!(
            old_error,
            LocatorCodecError::SetCheckedUnsupported {
                version: ProtocolVersion::new(1, 19),
            }
        );
    }

    #[test]
    fn set_checked_rejects_nonfinite_wire_positions() {
        // Arrange
        let options = generated_schema::PwrightLocatorSetCheckedOptions {
            force: None,
            position: Some(generated_schema::PwrightLocatorPoint {
                x: f64::NAN,
                y: 1.0,
            }),
            scroll: None,
            trial: None,
        };

        // Act
        let result = set_checked_options_from_wire(options);

        // Assert
        assert_eq!(result, Err(LocatorCodecError::InvalidPointerPosition));
    }

    #[test]
    fn select_option_round_trips_descriptors_results_and_rejects_one_twenty() {
        // Arrange
        let current = WireProtocol::new_exact_directional(
            codec().with_capability_version(ProtocolVersion::new(1, 21)),
            ProtocolVersion::new(1, 21),
            FrameLimits::new(4096, 0, 0, 4096),
            FrameLimits::new(4096, 0, 0, 4096),
        );
        let command = Command::<SelectOption>::new(
            CallId::new(41),
            ObjectId::new(42),
            CallMetadata::new(Some(Duration::from_secs(2)), Some(CancellationId::new(43))),
            SelectOptionParams::new(
                Selector::label("Color", true),
                vec![
                    SelectOptionValue::value_or_label("Blue"),
                    SelectOptionValue::new()
                        .with_value("green")
                        .with_label(" Green ")
                        .with_index(-1),
                ],
                SelectOptionOptions::new().with_force(false),
            ),
        );
        let result = CommandResult::<SelectOption>::new(
            CallId::new(41),
            Ok(vec!["blue".to_owned(), "green".to_owned()]),
        );
        let old = codec().with_capability_version(ProtocolVersion::new(1, 20));

        // Act
        let decoded = current
            .decode::<Command<SelectOption>>(
                current
                    .encode(&command)
                    .expect("SelectOption should encode"),
            )
            .expect("SelectOption should decode");
        let decoded_result = current
            .decode::<CommandResult<SelectOption>>(
                current
                    .encode(&result)
                    .expect("SelectOption result should encode"),
            )
            .expect("SelectOption result should decode");
        let old_error =
            <LocatorCodec as WireCodec<Command<SelectOption>>>::encode_parts(&old, &command)
                .expect_err("1.20 must reject SelectOption");

        // Assert
        assert_eq!(decoded.call_id(), command.call_id());
        assert_eq!(decoded.object_id(), command.object_id());
        assert_eq!(decoded.metadata(), command.metadata());
        assert_eq!(decoded.params(), command.params());
        assert_eq!(decoded_result.outcome(), result.outcome());
        assert_eq!(
            old_error,
            LocatorCodecError::SelectOptionUnsupported {
                version: ProtocolVersion::new(1, 20),
            }
        );
    }

    #[test]
    fn select_option_enforces_descriptor_and_result_string_bounds() {
        // Arrange
        let oversized = "x".repeat(MAX_SELECT_OPTION_ITEM_BYTES + 1);
        let command = Command::<SelectOption>::new(
            CallId::new(44),
            ObjectId::new(45),
            CallMetadata::default(),
            SelectOptionParams::new(
                Selector::test_id("color"),
                SelectOptionValue::value_or_label(oversized.clone()),
                SelectOptionOptions::new(),
            ),
        );
        let result = CommandResult::<SelectOption>::new(CallId::new(44), Ok(vec![oversized]));
        let large_codec = LocatorCodec::new(LocatorCodecLimits::new(2 * 1024 * 1024));

        // Act
        let command_error = <LocatorCodec as WireCodec<Command<SelectOption>>>::encode_parts(
            &large_codec,
            &command,
        )
        .expect_err("oversized descriptor must fail");
        let result_error = <LocatorCodec as WireCodec<CommandResult<SelectOption>>>::encode_parts(
            &large_codec,
            &result,
        )
        .expect_err("oversized result must fail");

        // Assert
        assert_eq!(
            command_error,
            LocatorCodecError::StringTooLong {
                actual: MAX_SELECT_OPTION_ITEM_BYTES + 1,
                limit: MAX_SELECT_OPTION_ITEM_BYTES,
            }
        );
        assert_eq!(result_error, command_error);
    }

    #[test]
    fn hover_codec_preserves_absent_and_explicitly_empty_modifiers() {
        // Arrange
        let protocol = protocol();
        let command = |call_id, options| {
            Command::<Hover>::new(
                CallId::new(call_id),
                ObjectId::new(4),
                CallMetadata::default(),
                HoverParams::new(
                    Selector::css("button").expect("selector should build"),
                    options,
                ),
            )
        };
        let absent = command(9, HoverOptions::new());
        let empty = command(
            10,
            HoverOptions::new().with_modifiers(KeyboardModifiers::new()),
        );

        // Act
        let decoded_absent = protocol
            .decode::<Command<Hover>>(protocol.encode(&absent).expect("absent should encode"))
            .expect("absent should decode");
        let decoded_empty = protocol
            .decode::<Command<Hover>>(protocol.encode(&empty).expect("empty should encode"))
            .expect("empty should decode");

        // Assert
        assert_eq!(decoded_absent.params().options.modifiers, None);
        assert_eq!(
            decoded_empty
                .params()
                .options
                .modifiers
                .map(KeyboardModifiers::len),
            Some(0)
        );
    }

    #[test]
    fn hover_decoder_rejects_noncanonical_modifiers_and_nonfinite_positions() {
        // Arrange
        let command = Command::<Hover>::new(
            CallId::new(8),
            ObjectId::new(4),
            CallMetadata::default(),
            HoverParams::new(
                Selector::css("x").expect("selector should build"),
                HoverOptions::new()
                    .with_modifier(KeyboardModifier::Alt)
                    .with_modifier(KeyboardModifier::Shift)
                    .with_position(
                        Position::new(12.5, 3.25).expect("finite position should build"),
                    ),
            ),
        );
        let encoded = protocol().encode(&command).expect("command should encode");
        let mut duplicate_payload = encoded.payload().to_vec();
        let modifier_window = duplicate_payload
            .windows(6)
            .position(|window| window == [0, 0, 0, 2, 0, 4])
            .expect("modifier encoding should be present");
        duplicate_payload[modifier_window + 5] = 0;
        let mut excessive_payload = encoded.payload().to_vec();
        excessive_payload[modifier_window + 3] = 6;
        let mut nonfinite_payload = encoded.payload().to_vec();
        let x_window = nonfinite_payload
            .windows(8)
            .position(|window| window == 12.5_f64.to_be_bytes())
            .expect("x coordinate encoding should be present");
        nonfinite_payload[x_window..x_window + 8].copy_from_slice(&f64::NAN.to_be_bytes());

        // Act
        let duplicate = <LocatorCodec as WireCodec<Command<Hover>>>::decode_frame(
            &codec(),
            rebuild(&encoded, duplicate_payload, Vec::new()),
        );
        let excessive = <LocatorCodec as WireCodec<Command<Hover>>>::decode_frame(
            &codec(),
            rebuild(&encoded, excessive_payload, Vec::new()),
        );
        let nonfinite = <LocatorCodec as WireCodec<Command<Hover>>>::decode_frame(
            &codec(),
            rebuild(&encoded, nonfinite_payload, Vec::new()),
        );

        // Assert
        assert_eq!(
            duplicate.err(),
            Some(LocatorCodecError::NonCanonicalKeyboardModifiers)
        );
        assert_eq!(
            excessive.err(),
            Some(LocatorCodecError::TooManyKeyboardModifiers {
                actual: 6,
                limit: 5,
            })
        );
        assert_eq!(
            nonfinite.err(),
            Some(LocatorCodecError::InvalidPointerPosition)
        );
    }

    #[test]
    fn decoded_regex_flags_must_be_canonical_ecmascript_flags() {
        // Arrange
        let encoded = protocol()
            .encode(&Command::<Click>::new(
                CallId::new(1),
                ObjectId::new(2),
                CallMetadata::default(),
                ClickParams::new(
                    Selector::text_regex(RegexMatcher::new("needle", "i").expect("valid regex")),
                    ClickOptions::new(),
                ),
            ))
            .expect("regex command should encode");
        let mut payload = encoded.payload().to_vec();
        let flag = payload
            .iter()
            .position(|byte| *byte == b'i')
            .expect("encoded flag should be present");
        payload[flag] = b'x';

        // Act
        let decoded = decode_command(rebuild(&encoded, payload, Vec::new()));

        // Assert
        assert_eq!(
            decoded.err(),
            Some(LocatorCodecError::InvalidRegexFlags(
                RegexMatcherError::UnsupportedFlag('x')
            ))
        );
    }

    #[test]
    fn attribute_and_test_id_selector_variants_round_trip() {
        // Arrange
        let attributes =
            TestIdAttributes::new("data-pw,data-testid").expect("valid test-id attributes");
        let selectors = vec![
            Selector::attribute_text(UserFacingAttribute::Placeholder, "Search", false),
            Selector::attribute_regex(
                UserFacingAttribute::Alt,
                RegexMatcher::new("^Product \\d+$", "i").expect("valid regex"),
            ),
            Selector::attribute_text(UserFacingAttribute::Title, "Release notes", true),
            Selector::test_id_with_attributes(attributes.clone(), "save"),
            Selector::test_id_regex_with_attributes(
                attributes,
                RegexMatcher::new("^custom-", "").expect("valid regex"),
            ),
        ];

        // Act
        let decoded = selectors
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, selector)| {
                let command = Command::<Click>::new(
                    CallId::new(index as u64 + 1),
                    ObjectId::new(2),
                    CallMetadata::default(),
                    ClickParams::new(selector, ClickOptions::new()),
                );
                protocol()
                    .decode::<Command<Click>>(
                        protocol().encode(&command).expect("selector should encode"),
                    )
                    .expect("selector should decode")
                    .params()
                    .selector
                    .clone()
            })
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(decoded, selectors);
    }

    #[test]
    fn unknown_user_facing_attribute_kind_is_rejected() {
        // Arrange
        let encoded = protocol()
            .encode(&Command::<Click>::new(
                CallId::new(1),
                ObjectId::new(2),
                CallMetadata::default(),
                ClickParams::new(
                    Selector::attribute_text(UserFacingAttribute::Placeholder, "Search", false),
                    ClickOptions::new(),
                ),
            ))
            .expect("attribute command should encode");
        let mut payload = encoded.payload().to_vec();
        payload[20] = 99;

        // Act
        let decoded = decode_command(rebuild(&encoded, payload, Vec::new()));

        // Assert
        assert_eq!(
            decoded.err(),
            Some(LocatorCodecError::InvalidUserFacingAttribute(99))
        );
    }

    #[test]
    fn malformed_test_id_attribute_names_are_rejected_on_decode() {
        // Arrange
        let encoded = protocol()
            .encode(&Command::<Click>::new(
                CallId::new(1),
                ObjectId::new(2),
                CallMetadata::default(),
                ClickParams::new(Selector::test_id("save"), ClickOptions::new()),
            ))
            .expect("test-id command should encode");
        let mut payload = encoded.payload().to_vec();
        let start = payload
            .windows(b"data-testid".len())
            .position(|window| window == b"data-testid")
            .expect("encoded attribute name should be present");
        payload[start + 4] = b' ';

        // Act
        let decoded = decode_command(rebuild(&encoded, payload, Vec::new()));

        // Assert
        assert_eq!(
            decoded.err(),
            Some(LocatorCodecError::InvalidTestIdAttributes(
                TestIdAttributesError::InvalidCharacter {
                    index: 0,
                    character: ' ',
                }
            ))
        );
    }

    #[test]
    fn locator_schema_is_valid_and_simple_fill_bytes_are_stable() {
        // Arrange
        let command = Command::<Fill>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::default(),
            FillParams::new(
                Selector::css("x").expect("selector should build"),
                "y",
                FillOptions::new(),
            ),
        );

        // Act
        let encoded = codec().encode_parts(&command).expect("fill should encode");

        // Assert
        assert_eq!(LOCATOR_SCHEMA.validate(), Ok(()));
        assert_eq!(LOCATOR_SCHEMA_CATALOG.validate(), Ok(()));
        assert_eq!(
            encoded.payload(),
            &[
                FILL_COMMAND,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                2,
                0,
                0,
                0,
                0,
                0,
                0,
                1,
                b'x',
                0,
                0,
                0,
                1,
                b'y',
                0,
            ]
        );
    }

    #[test]
    fn generated_command_metadata_matches_the_typed_locator_contract() {
        // Arrange
        let all_inner_texts = LOCATOR_COMMAND_SCHEMAS[0];
        let all_text_contents = LOCATOR_COMMAND_SCHEMAS[1];
        let blur = LOCATOR_COMMAND_SCHEMAS[2];
        let click = LOCATOR_COMMAND_SCHEMAS[3];
        let count = LOCATOR_COMMAND_SCHEMAS[4];
        let fill = LOCATOR_COMMAND_SCHEMAS[5];
        let focus = LOCATOR_COMMAND_SCHEMAS[6];
        let get_attribute = LOCATOR_COMMAND_SCHEMAS[7];
        let hover = LOCATOR_COMMAND_SCHEMAS[8];
        let inner_html = LOCATOR_COMMAND_SCHEMAS[9];
        let inner_text = LOCATOR_COMMAND_SCHEMAS[10];
        let input_value = LOCATOR_COMMAND_SCHEMAS[11];
        let is_checked = LOCATOR_COMMAND_SCHEMAS[12];
        let is_editable = LOCATOR_COMMAND_SCHEMAS[13];
        let is_enabled = LOCATOR_COMMAND_SCHEMAS[14];
        let is_visible = LOCATOR_COMMAND_SCHEMAS[15];
        let press = LOCATOR_COMMAND_SCHEMAS[16];
        let scroll_into_view_if_needed = LOCATOR_COMMAND_SCHEMAS[17];
        let select_option = LOCATOR_COMMAND_SCHEMAS[18];
        let set_checked = LOCATOR_COMMAND_SCHEMAS[19];
        let text_content = LOCATOR_COMMAND_SCHEMAS[20];
        let wait_for = LOCATOR_COMMAND_SCHEMAS[21];

        // Act
        let wire_mappings = [
            (
                all_inner_texts.id(),
                all_inner_texts.method(),
                all_inner_texts.command_tag(),
                all_inner_texts.result_tag(),
            ),
            (
                all_text_contents.id(),
                all_text_contents.method(),
                all_text_contents.command_tag(),
                all_text_contents.result_tag(),
            ),
            (
                blur.id(),
                blur.method(),
                blur.command_tag(),
                blur.result_tag(),
            ),
            (
                click.id(),
                click.method(),
                click.command_tag(),
                click.result_tag(),
            ),
            (
                count.id(),
                count.method(),
                count.command_tag(),
                count.result_tag(),
            ),
            (
                fill.id(),
                fill.method(),
                fill.command_tag(),
                fill.result_tag(),
            ),
            (
                focus.id(),
                focus.method(),
                focus.command_tag(),
                focus.result_tag(),
            ),
            (
                get_attribute.id(),
                get_attribute.method(),
                get_attribute.command_tag(),
                get_attribute.result_tag(),
            ),
            (
                hover.id(),
                hover.method(),
                hover.command_tag(),
                hover.result_tag(),
            ),
            (
                inner_html.id(),
                inner_html.method(),
                inner_html.command_tag(),
                inner_html.result_tag(),
            ),
            (
                inner_text.id(),
                inner_text.method(),
                inner_text.command_tag(),
                inner_text.result_tag(),
            ),
            (
                input_value.id(),
                input_value.method(),
                input_value.command_tag(),
                input_value.result_tag(),
            ),
            (
                is_checked.id(),
                is_checked.method(),
                is_checked.command_tag(),
                is_checked.result_tag(),
            ),
            (
                is_editable.id(),
                is_editable.method(),
                is_editable.command_tag(),
                is_editable.result_tag(),
            ),
            (
                is_enabled.id(),
                is_enabled.method(),
                is_enabled.command_tag(),
                is_enabled.result_tag(),
            ),
            (
                is_visible.id(),
                is_visible.method(),
                is_visible.command_tag(),
                is_visible.result_tag(),
            ),
            (
                press.id(),
                press.method(),
                press.command_tag(),
                press.result_tag(),
            ),
            (
                scroll_into_view_if_needed.id(),
                scroll_into_view_if_needed.method(),
                scroll_into_view_if_needed.command_tag(),
                scroll_into_view_if_needed.result_tag(),
            ),
            (
                select_option.id(),
                select_option.method(),
                select_option.command_tag(),
                select_option.result_tag(),
            ),
            (
                set_checked.id(),
                set_checked.method(),
                set_checked.command_tag(),
                set_checked.result_tag(),
            ),
            (
                text_content.id(),
                text_content.method(),
                text_content.command_tag(),
                text_content.result_tag(),
            ),
            (
                wait_for.id(),
                wait_for.method(),
                wait_for.command_tag(),
                wait_for.result_tag(),
            ),
        ];

        // Assert
        assert_eq!(
            wire_mappings,
            [
                (
                    ALL_INNER_TEXTS_COMMAND_ID,
                    ALL_INNER_TEXTS_METHOD,
                    ALL_INNER_TEXTS_COMMAND,
                    ALL_INNER_TEXTS_RESULT,
                ),
                (
                    ALL_TEXT_CONTENTS_COMMAND_ID,
                    ALL_TEXT_CONTENTS_METHOD,
                    ALL_TEXT_CONTENTS_COMMAND,
                    ALL_TEXT_CONTENTS_RESULT,
                ),
                (BLUR_COMMAND_ID, BLUR_METHOD, BLUR_COMMAND, BLUR_RESULT),
                (CLICK_COMMAND_ID, CLICK_METHOD, CLICK_COMMAND, CLICK_RESULT),
                (COUNT_COMMAND_ID, COUNT_METHOD, COUNT_COMMAND, COUNT_RESULT),
                (FILL_COMMAND_ID, FILL_METHOD, FILL_COMMAND, FILL_RESULT),
                (FOCUS_COMMAND_ID, FOCUS_METHOD, FOCUS_COMMAND, FOCUS_RESULT),
                (
                    GET_ATTRIBUTE_COMMAND_ID,
                    GET_ATTRIBUTE_METHOD,
                    GET_ATTRIBUTE_COMMAND,
                    GET_ATTRIBUTE_RESULT,
                ),
                (HOVER_COMMAND_ID, HOVER_METHOD, HOVER_COMMAND, HOVER_RESULT),
                (
                    INNER_HTML_COMMAND_ID,
                    INNER_HTML_METHOD,
                    INNER_HTML_COMMAND,
                    INNER_HTML_RESULT,
                ),
                (
                    INNER_TEXT_COMMAND_ID,
                    INNER_TEXT_METHOD,
                    INNER_TEXT_COMMAND,
                    INNER_TEXT_RESULT,
                ),
                (
                    INPUT_VALUE_COMMAND_ID,
                    INPUT_VALUE_METHOD,
                    INPUT_VALUE_COMMAND,
                    INPUT_VALUE_RESULT,
                ),
                (
                    IS_CHECKED_COMMAND_ID,
                    IS_CHECKED_METHOD,
                    IS_CHECKED_COMMAND,
                    IS_CHECKED_RESULT,
                ),
                (
                    IS_EDITABLE_COMMAND_ID,
                    IS_EDITABLE_METHOD,
                    IS_EDITABLE_COMMAND,
                    IS_EDITABLE_RESULT,
                ),
                (
                    IS_ENABLED_COMMAND_ID,
                    IS_ENABLED_METHOD,
                    IS_ENABLED_COMMAND,
                    IS_ENABLED_RESULT,
                ),
                (
                    IS_VISIBLE_COMMAND_ID,
                    IS_VISIBLE_METHOD,
                    IS_VISIBLE_COMMAND,
                    IS_VISIBLE_RESULT,
                ),
                (PRESS_COMMAND_ID, PRESS_METHOD, PRESS_COMMAND, PRESS_RESULT),
                (
                    SCROLL_INTO_VIEW_IF_NEEDED_COMMAND_ID,
                    SCROLL_INTO_VIEW_IF_NEEDED_METHOD,
                    SCROLL_INTO_VIEW_IF_NEEDED_COMMAND,
                    SCROLL_INTO_VIEW_IF_NEEDED_RESULT,
                ),
                (
                    SELECT_OPTION_COMMAND_ID,
                    SELECT_OPTION_METHOD,
                    SELECT_OPTION_COMMAND,
                    SELECT_OPTION_RESULT,
                ),
                (
                    SET_CHECKED_COMMAND_ID,
                    SET_CHECKED_METHOD,
                    SET_CHECKED_COMMAND,
                    SET_CHECKED_RESULT,
                ),
                (
                    TEXT_CONTENT_COMMAND_ID,
                    TEXT_CONTENT_METHOD,
                    TEXT_CONTENT_COMMAND,
                    TEXT_CONTENT_RESULT,
                ),
                (
                    WAIT_FOR_COMMAND_ID,
                    WAIT_FOR_METHOD,
                    WAIT_FOR_COMMAND,
                    WAIT_FOR_RESULT,
                ),
            ]
        );
    }

    #[test]
    fn text_content_command_and_nullable_result_round_trip() {
        // Arrange
        let protocol = protocol();
        let command = Command::<TextContent>::new(
            CallId::new(61),
            ObjectId::new(3),
            CallMetadata::new(Some(Duration::from_secs(2)), None),
            TextContentParams::new(Selector::test_id("status")),
        );
        let present = CommandResult::<TextContent>::new(
            CallId::new(61),
            Ok(Some("  Ready\nnow  ".to_string())),
        );
        let absent = CommandResult::<TextContent>::new(CallId::new(62), Ok(None));

        // Act
        let decoded_command = protocol
            .decode::<Command<TextContent>>(
                protocol.encode(&command).expect("command should encode"),
            )
            .expect("command should decode");
        let decoded_present = protocol
            .decode::<CommandResult<TextContent>>(
                protocol
                    .encode(&present)
                    .expect("present result should encode"),
            )
            .expect("present result should decode");
        let decoded_absent = protocol
            .decode::<CommandResult<TextContent>>(
                protocol
                    .encode(&absent)
                    .expect("absent result should encode"),
            )
            .expect("absent result should decode");

        // Assert
        assert_eq!(decoded_command.call_id(), command.call_id());
        assert_eq!(decoded_command.object_id(), command.object_id());
        assert_eq!(decoded_command.metadata(), command.metadata());
        assert_eq!(decoded_command.params(), command.params());
        assert_eq!(decoded_present.call_id(), present.call_id());
        assert_eq!(decoded_present.outcome(), present.outcome());
        assert_eq!(decoded_absent.call_id(), absent.call_id());
        assert_eq!(decoded_absent.outcome(), absent.outcome());
    }

    #[test]
    fn text_content_result_enforces_string_limit_and_nullable_tag() {
        // Arrange
        let tiny = LocatorCodec::new(LocatorCodecLimits::new(4));
        let oversized =
            CommandResult::<TextContent>::new(CallId::new(63), Ok(Some("12345".to_string())));
        let absent = CommandResult::<TextContent>::new(CallId::new(64), Ok(None));
        let encoded_absent = protocol()
            .encode(&absent)
            .expect("absent result should encode");
        let mut malformed_payload = encoded_absent.payload().to_vec();
        malformed_payload[10] = 2;
        let malformed = rebuild(&encoded_absent, malformed_payload, Vec::new());

        // Act
        let oversized = <LocatorCodec as WireCodec<CommandResult<TextContent>>>::encode_parts(
            &tiny, &oversized,
        );
        let malformed = <LocatorCodec as WireCodec<CommandResult<TextContent>>>::decode_frame(
            &codec(),
            malformed,
        );

        // Assert
        assert!(matches!(
            oversized,
            Err(LocatorCodecError::StringTooLong { limit: 4, .. })
        ));
        assert_eq!(
            malformed.err(),
            Some(LocatorCodecError::InvalidOptionFlag(2))
        );
    }

    #[test]
    fn get_attribute_command_and_nullable_result_round_trip_with_schema_bounds() {
        // Arrange
        let protocol = protocol();
        let command = Command::<GetAttribute>::new(
            CallId::new(65),
            ObjectId::new(3),
            CallMetadata::new(Some(Duration::from_secs(2)), None),
            GetAttributeParams::new(Selector::test_id("status"), "data-state"),
        );
        let present =
            CommandResult::<GetAttribute>::new(CallId::new(65), Ok(Some("ready".to_string())));
        let absent = CommandResult::<GetAttribute>::new(CallId::new(66), Ok(None));
        let wide = LocatorCodec::new(LocatorCodecLimits::new(2 * 1024 * 1024));
        let oversized_name = Command::<GetAttribute>::new(
            CallId::new(67),
            ObjectId::new(3),
            CallMetadata::default(),
            GetAttributeParams::new(Selector::test_id("status"), "n".repeat(64 * 1024 + 1)),
        );
        let oversized_result = CommandResult::<GetAttribute>::new(
            CallId::new(68),
            Ok(Some("v".repeat(1024 * 1024 + 1))),
        );
        let mut oversized_declared_name = BinaryWriter::new();
        oversized_declared_name
            .u32(64 * 1024 + 1)
            .expect("declared name length should encode");
        let oversized_declared_name = oversized_declared_name.into_bytes();

        // Act
        let decoded_command = protocol
            .decode::<Command<GetAttribute>>(
                protocol.encode(&command).expect("command should encode"),
            )
            .expect("command should decode");
        let decoded_present = protocol
            .decode::<CommandResult<GetAttribute>>(
                protocol.encode(&present).expect("result should encode"),
            )
            .expect("result should decode");
        let decoded_absent = protocol
            .decode::<CommandResult<GetAttribute>>(
                protocol.encode(&absent).expect("null result should encode"),
            )
            .expect("null result should decode");
        let oversized_name = <LocatorCodec as WireCodec<Command<GetAttribute>>>::encode_parts(
            &wide,
            &oversized_name,
        );
        let oversized_result =
            <LocatorCodec as WireCodec<CommandResult<GetAttribute>>>::encode_parts(
                &wide,
                &oversized_result,
            );
        let oversized_declared_name = wide.decode_bounded_string(
            &mut BinaryReader::new(&oversized_declared_name),
            MAX_ATTRIBUTE_NAME_BYTES,
        );

        // Assert
        assert_eq!(decoded_command.params(), command.params());
        assert_eq!(decoded_present.outcome(), present.outcome());
        assert_eq!(decoded_absent.outcome(), absent.outcome());
        assert!(matches!(
            oversized_name,
            Err(LocatorCodecError::StringTooLong { limit: 65_536, .. })
        ));
        assert!(matches!(
            oversized_result,
            Err(LocatorCodecError::StringTooLong {
                limit: 1_048_576,
                ..
            })
        ));
        assert!(matches!(
            oversized_declared_name,
            Err(LocatorCodecError::StringTooLong { limit: 65_536, .. })
        ));
    }

    #[test]
    fn inner_html_command_and_bounded_result_round_trip() {
        // Arrange
        let protocol = protocol();
        let command = Command::<InnerHtml>::new(
            CallId::new(66),
            ObjectId::new(3),
            CallMetadata::new(Some(Duration::from_secs(2)), None),
            InnerHtmlParams::new(Selector::test_id("container")),
        );
        let result =
            CommandResult::<InnerHtml>::new(CallId::new(66), Ok("<span>Ready</span>".to_string()));
        let tiny = LocatorCodec::new(LocatorCodecLimits::new(4));

        // Act
        let decoded_command = protocol
            .decode::<Command<InnerHtml>>(protocol.encode(&command).expect("command should encode"))
            .expect("command should decode");
        let decoded_result = protocol
            .decode::<CommandResult<InnerHtml>>(
                protocol.encode(&result).expect("result should encode"),
            )
            .expect("result should decode");
        let oversized =
            <LocatorCodec as WireCodec<CommandResult<InnerHtml>>>::encode_parts(&tiny, &result);

        // Assert
        assert_eq!(decoded_command.call_id(), command.call_id());
        assert_eq!(decoded_command.object_id(), command.object_id());
        assert_eq!(decoded_command.metadata(), command.metadata());
        assert_eq!(decoded_command.params(), command.params());
        assert_eq!(decoded_result.call_id(), result.call_id());
        assert_eq!(decoded_result.outcome(), result.outcome());
        assert!(matches!(
            oversized,
            Err(LocatorCodecError::StringTooLong { limit: 4, .. })
        ));
    }

    #[test]
    fn inner_text_command_and_bounded_result_round_trip() {
        // Arrange
        let protocol = protocol();
        let command = Command::<InnerText>::new(
            CallId::new(65),
            ObjectId::new(3),
            CallMetadata::new(Some(Duration::from_secs(2)), None),
            InnerTextParams::new(Selector::test_id("status")),
        );
        let result = CommandResult::<InnerText>::new(CallId::new(65), Ok("Ready\nnow".to_string()));
        let tiny = LocatorCodec::new(LocatorCodecLimits::new(4));

        // Act
        let decoded_command = protocol
            .decode::<Command<InnerText>>(protocol.encode(&command).expect("command should encode"))
            .expect("command should decode");
        let decoded_result = protocol
            .decode::<CommandResult<InnerText>>(
                protocol.encode(&result).expect("result should encode"),
            )
            .expect("result should decode");
        let oversized =
            <LocatorCodec as WireCodec<CommandResult<InnerText>>>::encode_parts(&tiny, &result);

        // Assert
        assert_eq!(decoded_command.call_id(), command.call_id());
        assert_eq!(decoded_command.object_id(), command.object_id());
        assert_eq!(decoded_command.metadata(), command.metadata());
        assert_eq!(decoded_command.params(), command.params());
        assert_eq!(decoded_result.call_id(), result.call_id());
        assert_eq!(decoded_result.outcome(), result.outcome());
        assert!(matches!(
            oversized,
            Err(LocatorCodecError::StringTooLong { limit: 4, .. })
        ));
    }

    #[test]
    fn input_value_command_and_bounded_result_round_trip() {
        // Arrange
        let protocol = protocol();
        let command = Command::<InputValue>::new(
            CallId::new(67),
            ObjectId::new(3),
            CallMetadata::new(Some(Duration::from_secs(2)), None),
            InputValueParams::new(Selector::test_id("account")),
        );
        let result = CommandResult::<InputValue>::new(CallId::new(67), Ok("alice".to_string()));
        let tiny = LocatorCodec::new(LocatorCodecLimits::new(4));

        // Act
        let decoded_command = protocol
            .decode::<Command<InputValue>>(
                protocol.encode(&command).expect("command should encode"),
            )
            .expect("command should decode");
        let decoded_result = protocol
            .decode::<CommandResult<InputValue>>(
                protocol.encode(&result).expect("result should encode"),
            )
            .expect("result should decode");
        let oversized =
            <LocatorCodec as WireCodec<CommandResult<InputValue>>>::encode_parts(&tiny, &result);

        // Assert
        assert_eq!(decoded_command.call_id(), command.call_id());
        assert_eq!(decoded_command.object_id(), command.object_id());
        assert_eq!(decoded_command.metadata(), command.metadata());
        assert_eq!(decoded_command.params(), command.params());
        assert_eq!(decoded_result.call_id(), result.call_id());
        assert_eq!(decoded_result.outcome(), result.outcome());
        assert!(matches!(
            oversized,
            Err(LocatorCodecError::StringTooLong { limit: 4, .. })
        ));
    }

    #[test]
    fn wait_for_command_states_and_unit_result_round_trip() {
        // Arrange
        let protocol = protocol();
        let states = [
            WaitForState::Attached,
            WaitForState::Detached,
            WaitForState::Visible,
            WaitForState::Hidden,
        ];

        // Act
        let decoded = states
            .into_iter()
            .enumerate()
            .map(|(index, state)| {
                let call_id = CallId::new(index as u64 + 41);
                let command = Command::<WaitFor>::new(
                    call_id,
                    ObjectId::new(3),
                    CallMetadata::default(),
                    WaitForParams::new(
                        Selector::test_id("status"),
                        WaitForOptions::new().with_state(state),
                    ),
                );
                let command = protocol
                    .decode::<Command<WaitFor>>(
                        protocol.encode(&command).expect("wait should encode"),
                    )
                    .expect("wait should decode");
                let result = CommandResult::<WaitFor>::new(call_id, Ok(()));
                let result = protocol
                    .decode::<CommandResult<WaitFor>>(
                        protocol.encode(&result).expect("result should encode"),
                    )
                    .expect("result should decode");
                (command.params().options.state, result.into_outcome())
            })
            .collect::<Vec<_>>();

        // Assert
        assert_eq!(
            decoded,
            vec![
                (WaitForState::Attached, Ok(())),
                (WaitForState::Detached, Ok(())),
                (WaitForState::Visible, Ok(())),
                (WaitForState::Hidden, Ok(())),
            ]
        );
    }

    #[test]
    fn unknown_wait_for_state_is_rejected_at_decode() {
        // Arrange
        let command = Command::<WaitFor>::new(
            CallId::new(45),
            ObjectId::new(3),
            CallMetadata::default(),
            WaitForParams::new(Selector::test_id("status"), WaitForOptions::new()),
        );
        let encoded = protocol()
            .encode(&command)
            .expect("wait command should encode");
        let mut payload = encoded.payload().to_vec();
        *payload.last_mut().expect("state byte should exist") = 9;

        // Act
        let decoded = <LocatorCodec as WireCodec<Command<WaitFor>>>::decode_frame(
            &codec(),
            rebuild(&encoded, payload, Vec::new()),
        );

        // Assert
        assert_eq!(
            decoded.err(),
            Some(LocatorCodecError::InvalidWaitForState(9))
        );
    }

    #[test]
    fn count_command_and_success_or_error_results_round_trip() {
        // Arrange
        let protocol = protocol();
        let command = Command::<Count>::new(
            CallId::new(31),
            ObjectId::new(3),
            CallMetadata::default(),
            CountParams::new(Selector::test_id("row")),
        );
        let success = CommandResult::<Count>::new(CallId::new(31), Ok(4));
        let failure = CommandResult::<Count>::new(
            CallId::new(32),
            Err(OperationError::new(
                FailureKind::Backend,
                "invalid selector",
                Some(LocatorErrorDetails::InvalidSelector("[".to_string())),
            )),
        );

        // Act
        let command = protocol
            .decode::<Command<Count>>(protocol.encode(&command).expect("count should encode"))
            .expect("count should decode");
        let success = protocol
            .decode::<CommandResult<Count>>(
                protocol.encode(&success).expect("success should encode"),
            )
            .expect("success should decode");
        let failure = protocol
            .decode::<CommandResult<Count>>(
                protocol.encode(&failure).expect("failure should encode"),
            )
            .expect("failure should decode");

        // Assert
        assert_eq!(
            command.params(),
            &CountParams::new(Selector::test_id("row"))
        );
        assert_eq!(success.into_outcome(), Ok(4));
        assert_eq!(
            failure
                .into_outcome()
                .expect_err("count error should remain typed")
                .details(),
            Some(&LocatorErrorDetails::InvalidSelector("[".to_string()))
        );
    }

    #[test]
    fn all_text_contents_command_and_ordered_collection_round_trip() {
        // Arrange
        let protocol = protocol();
        let command = Command::<AllTextContents>::new(
            CallId::new(33),
            ObjectId::new(3),
            CallMetadata::default(),
            AllTextContentsParams::new(Selector::test_id("row")),
        );
        let success = CommandResult::<AllTextContents>::new(
            CallId::new(33),
            Ok(vec!["first".into(), String::new(), "third".into()]),
        );

        // Act
        let command = protocol
            .decode::<Command<AllTextContents>>(
                protocol.encode(&command).expect("command should encode"),
            )
            .expect("command should decode");
        let success = protocol
            .decode::<CommandResult<AllTextContents>>(
                protocol.encode(&success).expect("result should encode"),
            )
            .expect("result should decode");

        // Assert
        assert_eq!(
            command.params(),
            &AllTextContentsParams::new(Selector::test_id("row"))
        );
        assert_eq!(
            success.into_outcome(),
            Ok(vec!["first".into(), "".into(), "third".into()])
        );
    }

    #[test]
    fn all_inner_texts_command_and_ordered_rendered_collection_round_trip() {
        // Arrange
        let protocol = protocol();
        let command = Command::<AllInnerTexts>::new(
            CallId::new(40),
            ObjectId::new(3),
            CallMetadata::default(),
            AllInnerTextsParams::new(Selector::test_id("row")),
        );
        let success = CommandResult::<AllInnerTexts>::new(
            CallId::new(40),
            Ok(vec!["first".into(), String::new(), "third".into()]),
        );

        // Act
        let command = protocol
            .decode::<Command<AllInnerTexts>>(
                protocol.encode(&command).expect("command should encode"),
            )
            .expect("command should decode");
        let success = protocol
            .decode::<CommandResult<AllInnerTexts>>(
                protocol.encode(&success).expect("result should encode"),
            )
            .expect("result should decode");

        // Assert
        assert_eq!(
            command.params(),
            &AllInnerTextsParams::new(Selector::test_id("row"))
        );
        assert_eq!(
            success.into_outcome(),
            Ok(vec!["first".into(), "".into(), "third".into()])
        );
    }

    #[test]
    fn all_inner_texts_encode_enforces_collection_bounds() {
        // Arrange
        let wide = LocatorCodec::new(LocatorCodecLimits::new(8 * 1024 * 1024));
        let too_many = CommandResult::<AllInnerTexts>::new(
            CallId::new(41),
            Ok(vec![String::new(); MAX_ALL_INNER_TEXTS_ITEMS + 1]),
        );
        let large_item = CommandResult::<AllInnerTexts>::new(
            CallId::new(42),
            Ok(vec!["x".repeat(MAX_ALL_INNER_TEXTS_ITEM_BYTES + 1)]),
        );
        let large_aggregate =
            CommandResult::<AllInnerTexts>::new(CallId::new(43), Ok(vec!["x".repeat(900_000); 5]));

        // Act
        let too_many = wide.encode_parts(&too_many);
        let large_item = wide.encode_parts(&large_item);
        let large_aggregate = wide.encode_parts(&large_aggregate);

        // Assert
        assert!(matches!(
            too_many,
            Err(LocatorCodecError::TooManyItems {
                limit: MAX_ALL_INNER_TEXTS_ITEMS,
                ..
            })
        ));
        assert!(matches!(
            large_item,
            Err(LocatorCodecError::StringTooLong {
                limit: MAX_ALL_INNER_TEXTS_ITEM_BYTES,
                ..
            })
        ));
        assert!(matches!(
            large_aggregate,
            Err(LocatorCodecError::AggregateStringBytesTooLarge {
                limit: MAX_ALL_INNER_TEXTS_TOTAL_BYTES,
                ..
            })
        ));
    }

    #[test]
    fn all_inner_texts_decode_rejects_item_count_before_collection_allocation() {
        // Arrange
        let mut payload = BinaryWriter::new();
        payload
            .u8(ALL_INNER_TEXTS_RESULT)
            .expect("kind should encode");
        payload.u64(44).expect("call ID should encode");
        payload.u8(0).expect("success should encode");
        payload
            .u32(u32::try_from(MAX_ALL_INNER_TEXTS_ITEMS + 1).expect("limit fits u32"))
            .expect("count should encode");
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Result {
                call_id: CallId::new(44),
            },
            payload.into_bytes(),
            Vec::new(),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .expect("small malicious frame should fit transport limits");

        // Act
        let decoded = <LocatorCodec as WireCodec<CommandResult<AllInnerTexts>>>::decode_frame(
            &LocatorCodec::new(LocatorCodecLimits::new(8 * 1024 * 1024)),
            frame,
        );

        // Assert
        assert!(matches!(
            decoded,
            Err(LocatorCodecError::TooManyItems {
                actual,
                limit: MAX_ALL_INNER_TEXTS_ITEMS,
                ..
            }) if actual == MAX_ALL_INNER_TEXTS_ITEMS + 1
        ));
    }

    #[test]
    fn all_text_contents_encode_enforces_item_and_aggregate_bounds() {
        // Arrange
        let wide = LocatorCodec::new(LocatorCodecLimits::new(8 * 1024 * 1024));
        let too_many = CommandResult::<AllTextContents>::new(
            CallId::new(34),
            Ok(vec![String::new(); MAX_ALL_TEXT_CONTENTS_ITEMS + 1]),
        );
        let large_item = CommandResult::<AllTextContents>::new(
            CallId::new(35),
            Ok(vec!["x".repeat(MAX_ALL_TEXT_CONTENTS_ITEM_BYTES + 1)]),
        );
        let large_aggregate = CommandResult::<AllTextContents>::new(
            CallId::new(36),
            Ok(vec!["x".repeat(900_000); 5]),
        );

        // Act
        let too_many = wide.encode_parts(&too_many);
        let large_item = wide.encode_parts(&large_item);
        let large_aggregate = wide.encode_parts(&large_aggregate);

        // Assert
        assert!(matches!(
            too_many,
            Err(LocatorCodecError::TooManyItems {
                limit: MAX_ALL_TEXT_CONTENTS_ITEMS,
                ..
            })
        ));
        assert!(matches!(
            large_item,
            Err(LocatorCodecError::StringTooLong {
                limit: MAX_ALL_TEXT_CONTENTS_ITEM_BYTES,
                ..
            })
        ));
        assert!(matches!(
            large_aggregate,
            Err(LocatorCodecError::AggregateStringBytesTooLarge {
                limit: MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES,
                ..
            })
        ));
    }

    #[test]
    fn all_text_contents_decode_rejects_item_count_before_collection_allocation() {
        // Arrange
        let mut payload = BinaryWriter::new();
        payload
            .u8(ALL_TEXT_CONTENTS_RESULT)
            .expect("kind should encode");
        payload.u64(37).expect("call ID should encode");
        payload.u8(0).expect("success should encode");
        payload
            .u32(u32::try_from(MAX_ALL_TEXT_CONTENTS_ITEMS + 1).expect("limit fits u32"))
            .expect("count should encode");
        let frame = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Result {
                call_id: CallId::new(37),
            },
            payload.into_bytes(),
            Vec::new(),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .expect("small malicious frame should fit transport limits");

        // Act
        let decoded = <LocatorCodec as WireCodec<CommandResult<AllTextContents>>>::decode_frame(
            &LocatorCodec::new(LocatorCodecLimits::new(8 * 1024 * 1024)),
            frame,
        );

        // Assert
        assert!(matches!(
            decoded,
            Err(LocatorCodecError::TooManyItems {
                actual,
                limit: MAX_ALL_TEXT_CONTENTS_ITEMS,
                ..
            }) if actual == MAX_ALL_TEXT_CONTENTS_ITEMS + 1
        ));
    }

    #[test]
    fn all_text_contents_decode_enforces_item_and_aggregate_string_bounds() {
        // Arrange
        let mut oversized_item = BinaryWriter::new();
        oversized_item
            .u8(ALL_TEXT_CONTENTS_RESULT)
            .expect("kind should encode");
        oversized_item.u64(38).expect("call ID should encode");
        oversized_item.u8(0).expect("success should encode");
        oversized_item.u32(1).expect("count should encode");
        oversized_item
            .u32(u32::try_from(MAX_ALL_TEXT_CONTENTS_ITEM_BYTES + 1).expect("item limit fits u32"))
            .expect("declared string length should encode");
        let oversized_item = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Result {
                call_id: CallId::new(38),
            },
            oversized_item.into_bytes(),
            Vec::new(),
            FrameLimits::new(1024, 0, 0, 1024),
        )
        .expect("small malicious frame should fit transport limits");

        let mut aggregate = BinaryWriter::new();
        aggregate
            .u8(ALL_TEXT_CONTENTS_RESULT)
            .expect("kind should encode");
        aggregate.u64(39).expect("call ID should encode");
        aggregate.u8(0).expect("success should encode");
        aggregate.u32(5).expect("count should encode");
        for _ in 0..5 {
            let value = "x".repeat(900_000);
            aggregate.u32(900_000).expect("length should encode");
            aggregate
                .raw(value.as_bytes())
                .expect("string should encode");
        }
        let aggregate = WireFrame::new(
            ProtocolVersion::new(1, 0),
            FrameRoute::Result {
                call_id: CallId::new(39),
            },
            aggregate.into_bytes(),
            Vec::new(),
            FrameLimits::new(5 * 1024 * 1024, 0, 0, 5 * 1024 * 1024),
        )
        .expect("bounded aggregate frame should fit transport limits");
        let wide = LocatorCodec::new(LocatorCodecLimits::new(8 * 1024 * 1024));

        // Act
        let oversized_item =
            <LocatorCodec as WireCodec<CommandResult<AllTextContents>>>::decode_frame(
                &wide,
                oversized_item,
            );
        let aggregate = <LocatorCodec as WireCodec<CommandResult<AllTextContents>>>::decode_frame(
            &wide, aggregate,
        );

        // Assert
        assert!(matches!(
            oversized_item,
            Err(LocatorCodecError::StringTooLong {
                limit: MAX_ALL_TEXT_CONTENTS_ITEM_BYTES,
                ..
            })
        ));
        assert!(matches!(
            aggregate,
            Err(LocatorCodecError::AggregateStringBytesTooLarge {
                limit: MAX_ALL_TEXT_CONTENTS_TOTAL_BYTES,
                ..
            })
        ));
    }

    #[test]
    fn visibility_command_and_boolean_results_round_trip() {
        // Arrange
        let protocol = protocol();
        let command = Command::<IsVisible>::new(
            CallId::new(33),
            ObjectId::new(3),
            CallMetadata::default(),
            IsVisibleParams::new(Selector::test_id("status")),
        );
        let visible = CommandResult::<IsVisible>::new(CallId::new(33), Ok(true));
        let hidden = CommandResult::<IsVisible>::new(CallId::new(34), Ok(false));

        // Act
        let command = protocol
            .decode::<Command<IsVisible>>(
                protocol.encode(&command).expect("visibility should encode"),
            )
            .expect("visibility should decode");
        let visible = protocol
            .decode::<CommandResult<IsVisible>>(
                protocol.encode(&visible).expect("true should encode"),
            )
            .expect("true should decode");
        let hidden = protocol
            .decode::<CommandResult<IsVisible>>(
                protocol.encode(&hidden).expect("false should encode"),
            )
            .expect("false should decode");

        // Assert
        assert_eq!(
            command.params(),
            &IsVisibleParams::new(Selector::test_id("status"))
        );
        assert_eq!(visible.into_outcome(), Ok(true));
        assert_eq!(hidden.into_outcome(), Ok(false));
    }

    #[test]
    fn element_state_commands_and_boolean_results_round_trip_independently() {
        // Arrange
        let protocol = protocol();
        let checked = Command::<IsChecked>::new(
            CallId::new(35),
            ObjectId::new(3),
            CallMetadata::default(),
            IsCheckedParams::new(Selector::test_id("check")),
        );
        let editable = Command::<IsEditable>::new(
            CallId::new(36),
            ObjectId::new(3),
            CallMetadata::default(),
            IsEditableParams::new(Selector::test_id("edit")),
        );
        let enabled = Command::<IsEnabled>::new(
            CallId::new(37),
            ObjectId::new(3),
            CallMetadata::default(),
            IsEnabledParams::new(Selector::test_id("enable")),
        );
        let checked_result = CommandResult::<IsChecked>::new(CallId::new(35), Ok(true));
        let editable_result = CommandResult::<IsEditable>::new(CallId::new(36), Ok(false));
        let enabled_result = CommandResult::<IsEnabled>::new(CallId::new(37), Ok(true));

        // Act
        let checked = protocol
            .decode::<Command<IsChecked>>(protocol.encode(&checked).expect("checked should encode"))
            .expect("checked should decode");
        let editable = protocol
            .decode::<Command<IsEditable>>(
                protocol.encode(&editable).expect("editable should encode"),
            )
            .expect("editable should decode");
        let enabled = protocol
            .decode::<Command<IsEnabled>>(protocol.encode(&enabled).expect("enabled should encode"))
            .expect("enabled should decode");
        let checked_result = protocol
            .decode::<CommandResult<IsChecked>>(
                protocol
                    .encode(&checked_result)
                    .expect("checked result should encode"),
            )
            .expect("checked result should decode");
        let editable_result = protocol
            .decode::<CommandResult<IsEditable>>(
                protocol
                    .encode(&editable_result)
                    .expect("editable result should encode"),
            )
            .expect("editable result should decode");
        let enabled_result = protocol
            .decode::<CommandResult<IsEnabled>>(
                protocol
                    .encode(&enabled_result)
                    .expect("enabled result should encode"),
            )
            .expect("enabled result should decode");

        // Assert
        assert_eq!(
            checked.params(),
            &IsCheckedParams::new(Selector::test_id("check"))
        );
        assert_eq!(
            editable.params(),
            &IsEditableParams::new(Selector::test_id("edit"))
        );
        assert_eq!(
            enabled.params(),
            &IsEnabledParams::new(Selector::test_id("enable"))
        );
        assert_eq!(checked_result.into_outcome(), Ok(true));
        assert_eq!(editable_result.into_outcome(), Ok(false));
        assert_eq!(enabled_result.into_outcome(), Ok(true));
    }

    #[test]
    fn fill_command_round_trips_route_metadata_selector_value_and_options() {
        // Arrange
        let protocol = protocol();
        let command = fill_command();

        // Act
        let decoded = protocol
            .decode::<Command<Fill>>(protocol.encode(&command).expect("fill should encode"))
            .expect("fill should decode");

        // Assert
        assert_eq!(decoded.call_id(), CallId::new(19));
        assert_eq!(decoded.object_id(), ObjectId::new(3));
        assert_eq!(decoded.metadata(), command.metadata());
        assert_eq!(decoded.params(), command.params());
        assert_eq!(Fill::METHOD, decoded.method());
    }

    #[test]
    fn success_and_every_portable_error_variant_round_trip() {
        // Arrange
        let protocol = protocol();
        let success = CommandResult::<Click>::new(CallId::new(1), Ok(()));
        let details = [
            LocatorErrorDetails::InvalidSelector("invalid".to_string()),
            LocatorErrorDetails::StrictViolation {
                selector: "button".to_string(),
                matches: 3,
            },
            LocatorErrorDetails::InvalidTarget("button cannot be filled".to_string()),
            LocatorErrorDetails::InvalidValue("not a number".to_string()),
            LocatorErrorDetails::InvalidBackendResponse("invalid response".to_string()),
            LocatorErrorDetails::Backend("backend".to_string()),
        ];
        let kinds = [
            FailureKind::Timeout,
            FailureKind::Cancelled,
            FailureKind::TargetClosed,
            FailureKind::BrowserCrashed,
            FailureKind::UnsupportedCapability,
            FailureKind::ResourceExhausted,
            FailureKind::TransportClosed,
            FailureKind::ProtocolViolation,
            FailureKind::Backend,
            FailureKind::Cleanup,
        ];

        // Act / Assert
        assert_eq!(
            protocol
                .decode::<CommandResult<Click>>(protocol.encode(&success).expect("encode success"))
                .expect("decode success")
                .into_outcome(),
            Ok(())
        );
        for (index, kind) in kinds.into_iter().enumerate() {
            let expected = details[index % details.len()].clone();
            let result = CommandResult::<Click>::new(
                CallId::new(index as u64 + 2),
                Err(OperationError::new(kind, "failed", Some(expected.clone()))),
            );
            let error = protocol
                .decode::<CommandResult<Click>>(protocol.encode(&result).expect("encode error"))
                .expect("decode error")
                .into_outcome()
                .expect_err("outcome should fail");
            assert_eq!(error.kind(), kind);
            assert_eq!(error.details(), Some(&expected));
        }
    }

    #[test]
    fn fill_result_and_new_typed_errors_round_trip() {
        // Arrange
        let protocol = protocol();
        let success = CommandResult::<Fill>::new(CallId::new(19), Ok(()));
        let failure = CommandResult::<Fill>::new(
            CallId::new(20),
            Err(OperationError::new(
                FailureKind::Backend,
                "invalid fill value",
                Some(LocatorErrorDetails::InvalidValue(
                    "not a number".to_string(),
                )),
            )),
        );

        // Act
        let success = protocol
            .decode::<CommandResult<Fill>>(protocol.encode(&success).expect("encode success"))
            .expect("decode success");
        let failure = protocol
            .decode::<CommandResult<Fill>>(protocol.encode(&failure).expect("encode failure"))
            .expect("decode failure");

        // Assert
        assert_eq!(success.into_outcome(), Ok(()));
        assert_eq!(
            failure
                .into_outcome()
                .expect_err("fill should fail")
                .details(),
            Some(&LocatorErrorDetails::InvalidValue(
                "not a number".to_string()
            ))
        );
    }

    #[test]
    fn fill_value_limit_and_force_boolean_are_checked() {
        // Arrange
        let tiny = LocatorCodec::new(LocatorCodecLimits::new(4));
        let oversized_value = Command::<Fill>::new(
            CallId::new(1),
            ObjectId::new(3),
            CallMetadata::default(),
            FillParams::new(
                CssSelector::new("x").expect("valid CSS"),
                "12345",
                FillOptions::new(),
            ),
        );
        let encoded = protocol()
            .encode(&fill_command())
            .expect("fill should encode");
        let mut invalid_force = encoded.payload().to_vec();
        *invalid_force.last_mut().expect("force byte should exist") = 2;
        let invalid_force = rebuild(&encoded, invalid_force, Vec::new());

        // Act
        let oversized =
            <LocatorCodec as WireCodec<Command<Fill>>>::encode_parts(&tiny, &oversized_value);
        let invalid_force =
            <LocatorCodec as WireCodec<Command<Fill>>>::decode_frame(&codec(), invalid_force);

        // Assert
        assert!(matches!(
            oversized,
            Err(LocatorCodecError::StringTooLong { limit: 4, .. })
        ));
        assert_eq!(
            invalid_force.err(),
            Some(LocatorCodecError::InvalidBoolean(2))
        );
    }

    #[test]
    fn size_and_selector_validation_happen_before_unbounded_allocation() {
        // Arrange
        let tiny = LocatorCodec::new(LocatorCodecLimits::new(3));
        let simple = protocol()
            .encode(&Command::<Click>::new(
                CallId::new(1),
                ObjectId::new(2),
                CallMetadata::default(),
                ClickParams::new(
                    CssSelector::new("x").expect("valid CSS"),
                    ClickOptions::new(),
                ),
            ))
            .expect("simple command should encode");
        let mut declared = simple.payload()[..24].to_vec();
        declared[20..24].copy_from_slice(&129_u32.to_be_bytes());
        let declared = rebuild(&simple, declared, Vec::new());
        let mut empty_payload = simple.payload().to_vec();
        empty_payload[24] = b' ';

        // Act
        let encode_error = tiny.encode_parts(&command());
        let declared_error = decode_command(declared);
        let empty_error = decode_command(rebuild(&simple, empty_payload, Vec::new()));

        // Assert
        assert!(matches!(
            encode_error,
            Err(LocatorCodecError::StringTooLong { limit: 3, .. })
        ));
        assert_eq!(
            declared_error.err(),
            Some(LocatorCodecError::StringTooLong {
                actual: 129,
                limit: 128
            })
        );
        assert_eq!(empty_error.err(), Some(LocatorCodecError::EmptySelector));
    }

    #[test]
    fn malformed_option_values_duration_and_trailing_data_are_rejected() {
        // Arrange
        let simple = protocol()
            .encode(&Command::<Click>::new(
                CallId::new(1),
                ObjectId::new(2),
                CallMetadata::default(),
                ClickParams::new(
                    CssSelector::new("x").expect("valid CSS"),
                    ClickOptions::new(),
                ),
            ))
            .expect("simple command should encode");
        let mutate = |index, value| {
            let mut payload = simple.payload().to_vec();
            payload[index] = value;
            rebuild(&simple, payload, Vec::new())
        };
        let timed = protocol()
            .encode(&command())
            .expect("command should encode");
        let mut nanos_payload = timed.payload().to_vec();
        nanos_payload[26..30].copy_from_slice(&1_000_000_000_u32.to_be_bytes());
        let mut trailing_payload = simple.payload().to_vec();
        trailing_payload.push(0);

        // Act / Assert
        assert_eq!(
            decode_command(mutate(25, 3)).err(),
            Some(LocatorCodecError::InvalidButton(3))
        );
        assert_eq!(
            decode_command(mutate(26, 0)).err(),
            Some(LocatorCodecError::ZeroClickCount)
        );
        assert_eq!(
            decode_command(mutate(27, 2)).err(),
            Some(LocatorCodecError::InvalidBoolean(2))
        );
        assert_eq!(
            decode_command(rebuild(&timed, nanos_payload, Vec::new())).err(),
            Some(LocatorCodecError::InvalidDurationNanos(1_000_000_000))
        );
        assert_eq!(
            decode_command(rebuild(&simple, trailing_payload, Vec::new())).err(),
            Some(LocatorCodecError::TrailingBytes { remaining: 1 })
        );
        assert!(matches!(
            decode_command(rebuild(&simple, Vec::new(), Vec::new())),
            Err(LocatorCodecError::Truncated { .. })
        ));
    }

    #[test]
    fn wrong_route_method_kind_and_attachments_are_rejected() {
        // Arrange
        let encoded = protocol()
            .encode(&command())
            .expect("command should encode");
        let limits = FrameLimits::new(1024, 1, 16, 1024);
        let frame = |route, payload, attachments| {
            WireFrame::new(
                ProtocolVersion::new(1, 0),
                route,
                payload,
                attachments,
                limits,
            )
            .expect("frame should build")
        };

        // Act
        let route = decode_command(frame(FrameRoute::Control, vec![CLICK_COMMAND], Vec::new()));
        let method = decode_command(frame(
            FrameRoute::Command {
                call_id: CallId::new(1),
                target: ObjectId::new(2),
                method: MethodTag::new(99),
            },
            vec![CLICK_COMMAND],
            Vec::new(),
        ));
        let kind = decode_command(frame(encoded.route(), vec![99], Vec::new()));
        let attachment =
            decode_command(rebuild(&encoded, encoded.payload().to_vec(), vec![vec![]]));

        // Assert
        assert!(matches!(route, Err(LocatorCodecError::WrongRoute(_))));
        assert!(matches!(method, Err(LocatorCodecError::WrongMethod { .. })));
        assert!(matches!(
            kind,
            Err(LocatorCodecError::WrongPayloadKind {
                expected: CLICK_COMMAND,
                actual: 99
            })
        ));
        assert_eq!(
            attachment.err(),
            Some(LocatorCodecError::AttachmentsUnsupported { actual: 1 })
        );
    }

    #[test]
    fn malformed_result_outcome_failure_and_detail_tags_are_rejected() {
        // Arrange
        let result = CommandResult::<Click>::new(
            CallId::new(1),
            Err(OperationError::new(
                FailureKind::Backend,
                "m",
                Some(LocatorErrorDetails::Backend("d".to_string())),
            )),
        );
        let encoded = protocol().encode(&result).expect("result should encode");
        let mutate = |index, value| {
            let mut payload = encoded.payload().to_vec();
            payload[index] = value;
            rebuild(&encoded, payload, Vec::new())
        };
        let decode = |frame| {
            <LocatorCodec as WireCodec<CommandResult<Click>>>::decode_frame(&codec(), frame)
        };

        // Act / Assert
        assert_eq!(
            decode(mutate(9, 2)).err(),
            Some(LocatorCodecError::InvalidOutcome(2))
        );
        assert_eq!(
            decode(mutate(10, 10)).err(),
            Some(LocatorCodecError::InvalidFailureKind(10))
        );
        assert_eq!(
            decode(mutate(16, 2)).err(),
            Some(LocatorCodecError::InvalidOptionFlag(2))
        );
        assert_eq!(
            decode(mutate(17, 6)).err(),
            Some(LocatorCodecError::InvalidErrorDetails(6))
        );
    }

    #[test]
    fn selector_depth_and_node_limits_apply_on_encode_and_decode() {
        // Arrange
        let depth_limited =
            LocatorCodec::new(LocatorCodecLimits::new(128).with_selector_complexity(2, 256));
        let node_limited =
            LocatorCodec::new(LocatorCodecLimits::new(128).with_selector_complexity(32, 2));
        let deep = Command::<Click>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::default(),
            ClickParams::new(
                Selector::css("button").expect("valid CSS").nth(0).nth(0),
                ClickOptions::new(),
            ),
        );
        let wide = Command::<Click>::new(
            CallId::new(2),
            ObjectId::new(2),
            CallMetadata::default(),
            ClickParams::new(
                Selector::css("button")
                    .expect("valid CSS")
                    .and(Selector::css(":enabled").expect("valid CSS")),
                ClickOptions::new(),
            ),
        );
        let encoded_wide = protocol()
            .encode(&wide)
            .expect("wide selector should encode");

        // Act
        let depth_error = depth_limited.encode_parts(&deep);
        let node_error =
            <LocatorCodec as WireCodec<Command<Click>>>::decode_frame(&node_limited, encoded_wide);

        // Assert
        assert!(matches!(
            depth_error,
            Err(LocatorCodecError::SelectorTooDeep {
                actual: 3,
                limit: 2
            })
        ));
        assert!(matches!(
            node_error,
            Err(LocatorCodecError::TooManySelectorNodes {
                actual: 3,
                limit: 2
            })
        ));
    }

    #[test]
    fn malformed_role_values_and_state_combinations_are_rejected() {
        // Arrange
        let role = protocol()
            .encode(&Command::<Click>::new(
                CallId::new(1),
                ObjectId::new(2),
                CallMetadata::default(),
                ClickParams::new(
                    Selector::role(AriaRole::Button, RoleOptions::new())
                        .expect("button selector should be valid"),
                    ClickOptions::new(),
                ),
            ))
            .expect("role command should encode");
        let mut unknown_role = role.payload().to_vec();
        unknown_role[24..30].copy_from_slice(b"bogus!");
        let mut invalid_checked = role.payload().to_vec();
        invalid_checked[32] = 2;

        let checkbox = protocol()
            .encode(&Command::<Click>::new(
                CallId::new(2),
                ObjectId::new(2),
                CallMetadata::default(),
                ClickParams::new(
                    Selector::role(
                        AriaRole::Checkbox,
                        RoleOptions::new().with_checked(AriaTriState::True),
                    )
                    .expect("checkbox selector should be valid"),
                    ClickOptions::new(),
                ),
            ))
            .expect("checkbox command should encode");
        let mut invalid_tri_state = checkbox.payload().to_vec();
        invalid_tri_state[34] = 4;

        // Act
        let unknown_role = decode_command(rebuild(&role, unknown_role, Vec::new()));
        let invalid_checked = decode_command(rebuild(&role, invalid_checked, Vec::new()));
        let invalid_tri_state = decode_command(rebuild(&checkbox, invalid_tri_state, Vec::new()));

        // Assert
        assert_eq!(
            unknown_role.err(),
            Some(LocatorCodecError::InvalidAriaRole("bogus!".to_string()))
        );
        assert_eq!(
            invalid_checked.err(),
            Some(LocatorCodecError::InvalidRoleOption {
                role: AriaRole::Button,
                option: "checked",
            })
        );
        assert_eq!(
            invalid_tri_state.err(),
            Some(LocatorCodecError::InvalidTriState(4))
        );
    }
}
