//! Typed leaf construction for Node user-facing locators; no browser access.

use super::{LocatorQuery, Plan, QueryDraft, QueryIssue};
use crate::HostError;
use pwright_cap_locator::{
    AriaRole, AriaTriState, RegexMatcher, RoleOptions, Selector, UserFacingAttribute,
};
use std::{num::NonZeroU32, str::FromStr};

#[derive(Clone, Copy, Debug)]
pub enum SemanticSelector {
    Text,
    Label,
    Placeholder,
    AltText,
    Title,
    TestId,
}

impl FromStr for SemanticSelector {
    type Err = HostError;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(match value {
            "text" => Self::Text,
            "label" => Self::Label,
            "placeholder" => Self::Placeholder,
            "altText" => Self::AltText,
            "title" => Self::Title,
            "testId" => Self::TestId,
            _ => {
                return Err(HostError::InvalidArgument(
                    "unknown semantic Locator kind".into(),
                ));
            }
        })
    }
}

impl SemanticSelector {
    pub const fn implicit_string_bytes(self) -> usize {
        if matches!(self, Self::TestId) {
            "data-testid".len()
        } else {
            0
        }
    }
}

#[derive(Default)]
pub struct RoleQueryOptions {
    pub name: Option<String>,
    pub name_flags: Option<String>,
    pub exact: bool,
    pub checked: Option<bool>,
    pub disabled: Option<bool>,
    pub expanded: Option<bool>,
    pub include_hidden: bool,
    pub level: Option<u32>,
    pub pressed: Option<bool>,
    pub selected: Option<bool>,
}

fn regex(source: String, flags: String) -> Result<RegexMatcher, HostError> {
    RegexMatcher::new(source, flags).map_err(|error| HostError::InvalidArgument(error.to_string()))
}

impl QueryDraft {
    pub fn semantic(
        self,
        kind: SemanticSelector,
        source: String,
        flags: Option<String>,
        exact: bool,
    ) -> Result<LocatorQuery, HostError> {
        let selector = match flags {
            Some(flags) => {
                let matcher = regex(source, flags)?;
                match kind {
                    SemanticSelector::Text => Selector::text_regex(matcher),
                    SemanticSelector::Label => Selector::label_regex(matcher),
                    SemanticSelector::Placeholder => {
                        Selector::attribute_regex(UserFacingAttribute::Placeholder, matcher)
                    }
                    SemanticSelector::AltText => {
                        Selector::attribute_regex(UserFacingAttribute::Alt, matcher)
                    }
                    SemanticSelector::Title => {
                        Selector::attribute_regex(UserFacingAttribute::Title, matcher)
                    }
                    SemanticSelector::TestId => Selector::test_id_regex(matcher),
                }
            }
            None => match kind {
                SemanticSelector::Text => Selector::text(source, exact),
                SemanticSelector::Label => Selector::label(source, exact),
                SemanticSelector::Placeholder => {
                    Selector::attribute_text(UserFacingAttribute::Placeholder, source, exact)
                }
                SemanticSelector::AltText => {
                    Selector::attribute_text(UserFacingAttribute::Alt, source, exact)
                }
                SemanticSelector::Title => {
                    Selector::attribute_text(UserFacingAttribute::Title, source, exact)
                }
                SemanticSelector::TestId => Selector::test_id(source),
            },
        };
        self.finish(Plan::Selector(selector))
    }

    pub fn role(self, role: &str, input: RoleQueryOptions) -> Result<LocatorQuery, HostError> {
        let Ok(role) = AriaRole::from_str(role) else {
            return self.finish(Plan::Issue(QueryIssue::InvalidRole));
        };
        let tristate = |value| {
            if value {
                AriaTriState::True
            } else {
                AriaTriState::False
            }
        };
        let mut options = RoleOptions {
            checked: input.checked.map(tristate),
            disabled: input.disabled,
            expanded: input.expanded,
            include_hidden: input.include_hidden,
            level: input.level.and_then(NonZeroU32::new),
            pressed: input.pressed.map(tristate),
            selected: input.selected,
            ..Default::default()
        };
        if input.level == Some(0) {
            return self.finish(Plan::Issue(QueryIssue::InvalidRoleOptions));
        }
        if let Some(name) = input.name {
            options = match input.name_flags {
                Some(flags) => options.with_name_regex(regex(name, flags)?),
                None => options.with_name(name, input.exact),
            };
        }
        let plan = match Selector::role(role, options) {
            Ok(selector) => Plan::Selector(selector),
            Err(_) => Plan::Issue(QueryIssue::InvalidRoleOptions),
        };
        self.finish(plan)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locator_query::{QueryBudget, QueryLimits, QueryMetrics};

    fn budget() -> QueryBudget {
        QueryBudget::new(QueryLimits {
            recipes: 8,
            bytes: 32 * 1024,
        })
        .unwrap()
    }

    #[test]
    fn user_facing_leaves_keep_their_typed_variants_and_release_their_storage() {
        // Arrange
        let budget = budget();
        let kinds = [
            SemanticSelector::Text,
            SemanticSelector::Label,
            SemanticSelector::Placeholder,
            SemanticSelector::AltText,
            SemanticSelector::Title,
            SemanticSelector::TestId,
        ];
        // Act
        let queries = kinds.map(|kind| {
            budget
                .prepare_leaf(5 + kind.implicit_string_bytes())
                .unwrap()
                .semantic(kind, "Hello".to_owned(), None, true)
                .unwrap()
        });
        // Assert
        assert_eq!(budget.metrics().unwrap().recipes, 6);
        let (text, lease) = queries[0].command_selector().unwrap();
        assert!(
            matches!(text, Selector::Text(matcher) if matcher.exact() && matcher.value() == "Hello")
        );
        drop(lease);
        let (id, lease) = queries[5].command_selector().unwrap();
        assert!(
            matches!(id, Selector::TestIdText { attributes, value } if attributes.as_csv() == "data-testid" && value == "Hello")
        );
        drop(lease);
        drop(queries);
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn regex_source_and_flags_are_owned_without_compiling_another_regex_engine() {
        // Arrange
        let budget = budget();
        let draft = budget.prepare_leaf(6).unwrap();
        // Act
        let query = draft
            .semantic(
                SemanticSelector::Label,
                "^name".into(),
                Some("i".into()),
                false,
            )
            .unwrap();
        // Assert
        let (selector, _lease) = query.command_selector().unwrap();
        assert!(
            matches!(selector, Selector::LabelRegex(matcher) if matcher.source() == "^name" && matcher.flags() == "i")
        );
    }

    #[test]
    fn invalid_regex_flags_return_reserved_query_capacity() {
        // Arrange
        let budget = budget();
        let draft = budget.prepare_leaf(3).unwrap();
        // Act
        let result = draft.semantic(SemanticSelector::Text, "x".into(), Some("ii".into()), false);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn test_id_attribute_storage_must_be_declared_as_well_as_the_user_value() {
        // Arrange
        let budget = budget();
        let draft = budget.prepare_leaf(1).unwrap();
        // Act
        let result = draft.semantic(SemanticSelector::TestId, "x".into(), None, true);
        // Assert
        assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
        assert_eq!(budget.metrics().unwrap(), QueryMetrics::default());
    }

    #[test]
    fn role_name_regex_and_boolean_filters_use_existing_portable_role_values() {
        // Arrange
        let budget = budget();
        let draft = budget.prepare_leaf(5).unwrap();
        // Act
        let query = draft
            .role(
                "button",
                RoleQueryOptions {
                    name: Some("save".into()),
                    name_flags: Some("i".into()),
                    pressed: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        // Assert
        let (selector, _lease) = query.command_selector().unwrap();
        assert!(
            matches!(selector, Selector::RolePattern(role) if role.role() == AriaRole::Button
            && role.options().pressed == Some(AriaTriState::True)
            && role.options().name_regex.as_ref().unwrap().flags() == "i")
        );
    }

    #[test]
    fn invalid_role_filters_remain_lazy_and_reject_when_a_command_is_prepared() {
        // Arrange
        let budget = budget();
        let draft = budget.prepare_leaf(0).unwrap();
        let query = draft
            .role(
                "link",
                RoleQueryOptions {
                    pressed: Some(true),
                    ..Default::default()
                },
            )
            .unwrap();
        // Act
        let result = query.command_selector();
        // Assert
        assert!(matches!(result, Err(HostError::Backend(_))));
        assert_eq!(query.deferred_issue(), Some(QueryIssue::InvalidRoleOptions));
        assert_eq!(budget.metrics().unwrap().recipes, 1);
    }
}
