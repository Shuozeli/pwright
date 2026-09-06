//! Generated selector wire values shared by independently installable codecs.
//!
//! This crate has no command root, capability, transport, runtime, browser
//! backend, YAML parser, or build script.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::num::NonZeroU32;
use std::str::FromStr;

use pwright_selector_model::{
    AriaRole, AriaTriState, CssSelector, RegexMatcher, RoleOptions, RolePatternSelector,
    RoleSelector, Selector, TestIdAttributes, TextMatcher, UserFacingAttribute,
};

#[doc(hidden)]
#[rustfmt::skip]
mod generated_schema;

pub use generated_schema::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectorModelDecodeError(String);

impl SelectorModelDecodeError {
    fn new(error: impl Display) -> Self {
        Self(error.to_string())
    }
}

impl Display for SelectorModelDecodeError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Error for SelectorModelDecodeError {}

/// Borrows one validated semantic selector into the generated reusable wire
/// model without copying its strings.
pub fn selector_to_wire(selector: &Selector) -> PwrightLocatorSelectorRef<'_> {
    match selector {
        Selector::Css(value) => PwrightLocatorSelectorRef::Css {
            value: value.as_str(),
        },
        Selector::Nth { base, index } => PwrightLocatorSelectorRef::Nth {
            base: Box::new(selector_to_wire(base)),
            index: *index,
        },
        Selector::FilterText { base, text, exact } => PwrightLocatorSelectorRef::FilterText {
            base: Box::new(selector_to_wire(base)),
            text,
            exact: *exact,
        },
        Selector::And { left, right } => PwrightLocatorSelectorRef::And {
            left: Box::new(selector_to_wire(left)),
            right: Box::new(selector_to_wire(right)),
        },
        Selector::Or { left, right } => PwrightLocatorSelectorRef::Or {
            left: Box::new(selector_to_wire(left)),
            right: Box::new(selector_to_wire(right)),
        },
        Selector::Text(matcher) => PwrightLocatorSelectorRef::Text {
            matcher: text_matcher_to_wire(matcher),
        },
        Selector::Label(matcher) => PwrightLocatorSelectorRef::Label {
            matcher: text_matcher_to_wire(matcher),
        },
        Selector::Role(selector) => PwrightLocatorSelectorRef::Role {
            selector: role_selector_to_wire(selector),
        },
        Selector::Within { base, descendant } => PwrightLocatorSelectorRef::Within {
            base: Box::new(selector_to_wire(base)),
            descendant: Box::new(selector_to_wire(descendant)),
        },
        Selector::FilterNotText { base, text, exact } => PwrightLocatorSelectorRef::FilterNotText {
            base: Box::new(selector_to_wire(base)),
            text,
            exact: *exact,
        },
        Selector::FilterHas { base, descendant } => PwrightLocatorSelectorRef::FilterHas {
            base: Box::new(selector_to_wire(base)),
            descendant: Box::new(selector_to_wire(descendant)),
        },
        Selector::FilterHasNot { base, descendant } => PwrightLocatorSelectorRef::FilterHasNot {
            base: Box::new(selector_to_wire(base)),
            descendant: Box::new(selector_to_wire(descendant)),
        },
        Selector::FilterVisible { base, visible } => PwrightLocatorSelectorRef::FilterVisible {
            base: Box::new(selector_to_wire(base)),
            visible: *visible,
        },
        Selector::TextRegex(matcher) => PwrightLocatorSelectorRef::TextRegex {
            matcher: regex_matcher_to_wire(matcher),
        },
        Selector::LabelRegex(matcher) => PwrightLocatorSelectorRef::LabelRegex {
            matcher: regex_matcher_to_wire(matcher),
        },
        Selector::FilterTextRegex { base, matcher } => PwrightLocatorSelectorRef::FilterTextRegex {
            base: Box::new(selector_to_wire(base)),
            matcher: regex_matcher_to_wire(matcher),
        },
        Selector::FilterNotTextRegex { base, matcher } => {
            PwrightLocatorSelectorRef::FilterNotTextRegex {
                base: Box::new(selector_to_wire(base)),
                matcher: regex_matcher_to_wire(matcher),
            }
        }
        Selector::RolePattern(selector) => PwrightLocatorSelectorRef::RolePattern {
            selector: role_pattern_selector_to_wire(selector),
        },
        Selector::AttributeText { attribute, matcher } => {
            PwrightLocatorSelectorRef::AttributeText {
                attribute: user_facing_attribute_to_wire(*attribute),
                matcher: text_matcher_to_wire(matcher),
            }
        }
        Selector::AttributeRegex { attribute, matcher } => {
            PwrightLocatorSelectorRef::AttributeRegex {
                attribute: user_facing_attribute_to_wire(*attribute),
                matcher: regex_matcher_to_wire(matcher),
            }
        }
        Selector::TestIdText { attributes, value } => PwrightLocatorSelectorRef::TestIdText {
            attributes: attributes.as_csv(),
            value,
        },
        Selector::TestIdRegex {
            attributes,
            matcher,
        } => PwrightLocatorSelectorRef::TestIdRegex {
            attributes: attributes.as_csv(),
            matcher: regex_matcher_to_wire(matcher),
        },
    }
}

/// Validates and reconstructs the portable selector model after bounded wire
/// decoding.
pub fn selector_from_wire(
    selector: PwrightLocatorSelector,
) -> Result<Selector, SelectorModelDecodeError> {
    match selector {
        PwrightLocatorSelector::Css { value } => CssSelector::new(value)
            .map(Selector::Css)
            .map_err(SelectorModelDecodeError::new),
        PwrightLocatorSelector::Nth { base, index } => Ok(Selector::Nth {
            base: Box::new(selector_from_wire(*base)?),
            index,
        }),
        PwrightLocatorSelector::FilterText { base, text, exact } => Ok(Selector::FilterText {
            base: Box::new(selector_from_wire(*base)?),
            text,
            exact,
        }),
        PwrightLocatorSelector::And { left, right } => Ok(Selector::And {
            left: Box::new(selector_from_wire(*left)?),
            right: Box::new(selector_from_wire(*right)?),
        }),
        PwrightLocatorSelector::Or { left, right } => Ok(Selector::Or {
            left: Box::new(selector_from_wire(*left)?),
            right: Box::new(selector_from_wire(*right)?),
        }),
        PwrightLocatorSelector::Text { matcher } => {
            Ok(Selector::Text(text_matcher_from_wire(matcher)))
        }
        PwrightLocatorSelector::Label { matcher } => {
            Ok(Selector::Label(text_matcher_from_wire(matcher)))
        }
        PwrightLocatorSelector::Role { selector } => {
            role_selector_from_wire(selector).map(Selector::Role)
        }
        PwrightLocatorSelector::Within { base, descendant } => Ok(Selector::Within {
            base: Box::new(selector_from_wire(*base)?),
            descendant: Box::new(selector_from_wire(*descendant)?),
        }),
        PwrightLocatorSelector::FilterNotText { base, text, exact } => {
            Ok(Selector::FilterNotText {
                base: Box::new(selector_from_wire(*base)?),
                text,
                exact,
            })
        }
        PwrightLocatorSelector::FilterHas { base, descendant } => Ok(Selector::FilterHas {
            base: Box::new(selector_from_wire(*base)?),
            descendant: Box::new(selector_from_wire(*descendant)?),
        }),
        PwrightLocatorSelector::FilterHasNot { base, descendant } => Ok(Selector::FilterHasNot {
            base: Box::new(selector_from_wire(*base)?),
            descendant: Box::new(selector_from_wire(*descendant)?),
        }),
        PwrightLocatorSelector::FilterVisible { base, visible } => Ok(Selector::FilterVisible {
            base: Box::new(selector_from_wire(*base)?),
            visible,
        }),
        PwrightLocatorSelector::TextRegex { matcher } => {
            regex_matcher_from_wire(matcher).map(Selector::TextRegex)
        }
        PwrightLocatorSelector::LabelRegex { matcher } => {
            regex_matcher_from_wire(matcher).map(Selector::LabelRegex)
        }
        PwrightLocatorSelector::FilterTextRegex { base, matcher } => {
            Ok(Selector::FilterTextRegex {
                base: Box::new(selector_from_wire(*base)?),
                matcher: regex_matcher_from_wire(matcher)?,
            })
        }
        PwrightLocatorSelector::FilterNotTextRegex { base, matcher } => {
            Ok(Selector::FilterNotTextRegex {
                base: Box::new(selector_from_wire(*base)?),
                matcher: regex_matcher_from_wire(matcher)?,
            })
        }
        PwrightLocatorSelector::RolePattern { selector } => {
            role_pattern_selector_from_wire(selector).map(Selector::RolePattern)
        }
        PwrightLocatorSelector::AttributeText { attribute, matcher } => {
            Ok(Selector::AttributeText {
                attribute: user_facing_attribute_from_wire(attribute),
                matcher: text_matcher_from_wire(matcher),
            })
        }
        PwrightLocatorSelector::AttributeRegex { attribute, matcher } => {
            Ok(Selector::AttributeRegex {
                attribute: user_facing_attribute_from_wire(attribute),
                matcher: regex_matcher_from_wire(matcher)?,
            })
        }
        PwrightLocatorSelector::TestIdText { attributes, value } => Ok(Selector::TestIdText {
            attributes: TestIdAttributes::new(attributes).map_err(SelectorModelDecodeError::new)?,
            value,
        }),
        PwrightLocatorSelector::TestIdRegex {
            attributes,
            matcher,
        } => Ok(Selector::TestIdRegex {
            attributes: TestIdAttributes::new(attributes).map_err(SelectorModelDecodeError::new)?,
            matcher: regex_matcher_from_wire(matcher)?,
        }),
    }
}

fn text_matcher_to_wire(matcher: &TextMatcher) -> PwrightLocatorTextMatcherRef<'_> {
    PwrightLocatorTextMatcherRef {
        value: matcher.value(),
        exact: matcher.exact(),
    }
}

fn text_matcher_from_wire(matcher: PwrightLocatorTextMatcher) -> TextMatcher {
    TextMatcher::new(matcher.value, matcher.exact)
}

fn regex_matcher_to_wire(matcher: &RegexMatcher) -> PwrightLocatorRegexMatcherRef<'_> {
    PwrightLocatorRegexMatcherRef {
        source: matcher.source(),
        flags: matcher.flags(),
    }
}

fn regex_matcher_from_wire(
    matcher: PwrightLocatorRegexMatcher,
) -> Result<RegexMatcher, SelectorModelDecodeError> {
    RegexMatcher::new(matcher.source, matcher.flags).map_err(SelectorModelDecodeError::new)
}

fn role_selector_to_wire(selector: &RoleSelector) -> PwrightLocatorRoleSelectorRef<'_> {
    let options = selector.options();
    PwrightLocatorRoleSelectorRef {
        role: selector.role().as_str(),
        name: options.name.as_ref().map(text_matcher_to_wire),
        description: options.description.as_ref().map(text_matcher_to_wire),
        checked: optional_tri_state_to_wire(options.checked),
        disabled: optional_bool_to_wire(options.disabled),
        expanded: optional_bool_to_wire(options.expanded),
        include_hidden: options.include_hidden,
        level: options.level.map(NonZeroU32::get),
        pressed: optional_tri_state_to_wire(options.pressed),
        selected: optional_bool_to_wire(options.selected),
    }
}

fn role_pattern_selector_to_wire(
    selector: &RolePatternSelector,
) -> PwrightLocatorRolePatternSelectorRef<'_> {
    let options = selector.options();
    PwrightLocatorRolePatternSelectorRef {
        role: selector.role().as_str(),
        name: text_pattern_to_wire(options.name.as_ref(), options.name_regex.as_ref()),
        description: text_pattern_to_wire(
            options.description.as_ref(),
            options.description_regex.as_ref(),
        ),
        checked: optional_tri_state_to_wire(options.checked),
        disabled: optional_bool_to_wire(options.disabled),
        expanded: optional_bool_to_wire(options.expanded),
        include_hidden: options.include_hidden,
        level: options.level.map(NonZeroU32::get),
        pressed: optional_tri_state_to_wire(options.pressed),
        selected: optional_bool_to_wire(options.selected),
    }
}

fn text_pattern_to_wire<'a>(
    text: Option<&'a TextMatcher>,
    regex: Option<&'a RegexMatcher>,
) -> Option<PwrightLocatorTextPatternRef<'a>> {
    match (text, regex) {
        (Some(matcher), None) => Some(PwrightLocatorTextPatternRef::String {
            matcher: text_matcher_to_wire(matcher),
        }),
        (None, Some(matcher)) => Some(PwrightLocatorTextPatternRef::Regex {
            matcher: regex_matcher_to_wire(matcher),
        }),
        (None, None) => None,
        (Some(_), Some(_)) => unreachable!("validated role options choose one matcher kind"),
    }
}

fn role_selector_from_wire(
    selector: PwrightLocatorRoleSelector,
) -> Result<RoleSelector, SelectorModelDecodeError> {
    let role = AriaRole::from_str(&selector.role).map_err(SelectorModelDecodeError::new)?;
    let options = RoleOptions {
        name: selector.name.map(text_matcher_from_wire),
        description: selector.description.map(text_matcher_from_wire),
        name_regex: None,
        description_regex: None,
        checked: optional_tri_state_from_wire(selector.checked),
        disabled: optional_bool_from_wire(selector.disabled),
        expanded: optional_bool_from_wire(selector.expanded),
        include_hidden: selector.include_hidden,
        level: optional_level_from_wire(selector.level)?,
        pressed: optional_tri_state_from_wire(selector.pressed),
        selected: optional_bool_from_wire(selector.selected),
    };
    RoleSelector::new(role, options).map_err(SelectorModelDecodeError::new)
}

fn role_pattern_selector_from_wire(
    selector: PwrightLocatorRolePatternSelector,
) -> Result<RolePatternSelector, SelectorModelDecodeError> {
    let role = AriaRole::from_str(&selector.role).map_err(SelectorModelDecodeError::new)?;
    let (name, name_regex) = text_pattern_from_wire(selector.name)?;
    let (description, description_regex) = text_pattern_from_wire(selector.description)?;
    let options = RoleOptions {
        name,
        description,
        name_regex,
        description_regex,
        checked: optional_tri_state_from_wire(selector.checked),
        disabled: optional_bool_from_wire(selector.disabled),
        expanded: optional_bool_from_wire(selector.expanded),
        include_hidden: selector.include_hidden,
        level: optional_level_from_wire(selector.level)?,
        pressed: optional_tri_state_from_wire(selector.pressed),
        selected: optional_bool_from_wire(selector.selected),
    };
    RolePatternSelector::new(role, options).map_err(SelectorModelDecodeError::new)
}

fn text_pattern_from_wire(
    pattern: Option<PwrightLocatorTextPattern>,
) -> Result<(Option<TextMatcher>, Option<RegexMatcher>), SelectorModelDecodeError> {
    match pattern {
        None => Ok((None, None)),
        Some(PwrightLocatorTextPattern::String { matcher }) => {
            Ok((Some(text_matcher_from_wire(matcher)), None))
        }
        Some(PwrightLocatorTextPattern::Regex { matcher }) => {
            Ok((None, Some(regex_matcher_from_wire(matcher)?)))
        }
    }
}

fn optional_level_from_wire(
    level: Option<u32>,
) -> Result<Option<NonZeroU32>, SelectorModelDecodeError> {
    level
        .map(|value| {
            NonZeroU32::new(value)
                .ok_or_else(|| SelectorModelDecodeError("ARIA level must be non-zero".to_string()))
        })
        .transpose()
}

const fn optional_bool_to_wire(value: Option<bool>) -> PwrightLocatorOptionalBool {
    match value {
        None => PwrightLocatorOptionalBool::Absent,
        Some(false) => PwrightLocatorOptionalBool::False,
        Some(true) => PwrightLocatorOptionalBool::True,
    }
}

const fn optional_bool_from_wire(value: PwrightLocatorOptionalBool) -> Option<bool> {
    match value {
        PwrightLocatorOptionalBool::Absent => None,
        PwrightLocatorOptionalBool::False => Some(false),
        PwrightLocatorOptionalBool::True => Some(true),
    }
}

const fn optional_tri_state_to_wire(
    value: Option<AriaTriState>,
) -> PwrightLocatorOptionalAriaTriState {
    match value {
        None => PwrightLocatorOptionalAriaTriState::Absent,
        Some(AriaTriState::False) => PwrightLocatorOptionalAriaTriState::False,
        Some(AriaTriState::True) => PwrightLocatorOptionalAriaTriState::True,
        Some(AriaTriState::Mixed) => PwrightLocatorOptionalAriaTriState::Mixed,
    }
}

const fn optional_tri_state_from_wire(
    value: PwrightLocatorOptionalAriaTriState,
) -> Option<AriaTriState> {
    match value {
        PwrightLocatorOptionalAriaTriState::Absent => None,
        PwrightLocatorOptionalAriaTriState::False => Some(AriaTriState::False),
        PwrightLocatorOptionalAriaTriState::True => Some(AriaTriState::True),
        PwrightLocatorOptionalAriaTriState::Mixed => Some(AriaTriState::Mixed),
    }
}

const fn user_facing_attribute_to_wire(
    value: UserFacingAttribute,
) -> PwrightLocatorUserFacingAttribute {
    match value {
        UserFacingAttribute::Placeholder => PwrightLocatorUserFacingAttribute::Placeholder,
        UserFacingAttribute::Alt => PwrightLocatorUserFacingAttribute::Alt,
        UserFacingAttribute::Title => PwrightLocatorUserFacingAttribute::Title,
    }
}

const fn user_facing_attribute_from_wire(
    value: PwrightLocatorUserFacingAttribute,
) -> UserFacingAttribute {
    match value {
        PwrightLocatorUserFacingAttribute::Placeholder => UserFacingAttribute::Placeholder,
        PwrightLocatorUserFacingAttribute::Alt => UserFacingAttribute::Alt,
        PwrightLocatorUserFacingAttribute::Title => UserFacingAttribute::Title,
    }
}

#[cfg(test)]
mod tests {
    use pwright_codec_binary::{
        BinaryDecode, BinaryEncode, BinaryReader, BinaryValueContext, BinaryValueLimits,
        BinaryWriter,
    };
    use pwright_codec_locator::{LOCATOR_SCHEMA_CATALOG, LOCATOR_V2_SCHEMA_CATALOG};
    use pwright_selector_model::{AriaRole, RegexMatcher, RoleOptions, Selector};

    use super::{CATALOG, PwrightLocatorSelector, selector_from_wire, selector_to_wire};

    #[test]
    fn selector_library_is_valid_and_matches_published_locator_catalog_identities() {
        // Arrange
        let v1 = LOCATOR_SCHEMA_CATALOG.types();
        let v2 = LOCATOR_V2_SCHEMA_CATALOG.types();

        // Act
        // Assert
        assert_eq!(CATALOG.validate(), Ok(()));
        assert_eq!(CATALOG.types().len(), 9);
        for selector_type in CATALOG.types() {
            let v1_type = v1
                .iter()
                .find(|candidate| candidate.id() == selector_type.id())
                .unwrap_or_else(|| panic!("locator 1.x is missing {}", selector_type.id()));
            assert_eq!(v1_type.kind(), selector_type.kind());
            assert_eq!(
                v1_type.layout_fingerprint(),
                selector_type.layout_fingerprint()
            );

            let v2_type = v2
                .iter()
                .find(|candidate| candidate.id() == selector_type.id())
                .unwrap_or_else(|| panic!("locator 2.x is missing {}", selector_type.id()));
            assert_eq!(v2_type.kind(), selector_type.kind());
        }
    }

    #[test]
    fn semantic_selector_round_trips_through_the_generated_library_codec() {
        // Arrange
        let selector = Selector::role(
            AriaRole::Button,
            RoleOptions::new().with_name_regex(
                RegexMatcher::new("save|submit", "i").expect("regex should validate"),
            ),
        )
        .expect("role selector should validate")
        .filter_visible(true)
        .nth(2);
        let limits = BinaryValueLimits::new(1_024, 0, 0, 64, 128);
        let mut writer = BinaryWriter::new();
        let mut encode_context = BinaryValueContext::new(limits);

        // Act
        selector_to_wire(&selector)
            .encode_binary(&mut writer, &mut encode_context)
            .expect("selector should encode");
        let bytes = writer.into_bytes();
        let mut reader = BinaryReader::new(&bytes);
        let mut decode_context = BinaryValueContext::new(limits);
        let wire = PwrightLocatorSelector::decode_binary(&mut reader, &mut decode_context)
            .expect("selector should decode");
        let decoded = selector_from_wire(wire).expect("semantic selector should validate");

        // Assert
        reader.finish().expect("selector bytes should be exhausted");
        assert_eq!(decoded, selector);
    }
}
