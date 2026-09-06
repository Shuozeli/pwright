//! Opt-in typed actions for `pwright-client` locator values.

use std::convert::Infallible;

use pwright_cap_locator::{
    AllInnerTexts, AllInnerTextsParams, AllTextContents, AllTextContentsParams, AriaRole, Blur,
    BlurParams, Click, ClickOptions, ClickParams, Count, CountParams, Fill, FillOptions,
    FillParams, Focus, FocusParams, GetAttribute, GetAttributeParams, Hover, HoverOptions,
    HoverParams, InnerHtml, InnerHtmlParams, InnerText, InnerTextParams, InputValue,
    InputValueParams, IsChecked, IsCheckedParams, IsEditable, IsEditableParams, IsEnabled,
    IsEnabledParams, IsVisible, IsVisibleParams, LocatorErrorDetails, Press, PressKey,
    PressOptions, PressParams, RegexMatcher, RoleOptions, RoleSelectorError,
    ScrollIntoViewIfNeeded, ScrollIntoViewIfNeededParams, SelectOption, SelectOptionOptions,
    SelectOptionParams, SelectOptionValues, Selector, SetChecked, SetCheckedOptions,
    SetCheckedParams, TestIdAttributes, TextContent, TextContentParams, UserFacingAttribute,
    WaitFor, WaitForOptions, WaitForParams,
};
use pwright_client::{CallIdSource, CommandFuture, Frame, Locator, LocatorScopeMismatch, Page};
use pwright_core::{CallMetadata, Command, CommandResult, FailureKind, OperationError, Service};

/// Locator 2.x client extensions selected by capability-major negotiation.
pub mod v2;

/// Largest current match count that can be represented by the selector AST's
/// signed `nth` index.
pub const MAX_LOCATOR_LIST_LENGTH: u32 = i32::MAX as u32;

pub trait LocatorClickExt {
    fn click(
        &self,
        options: ClickOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Playwright-shaped pointer hover with independent installability.
pub trait LocatorHoverExt {
    fn hover(
        &self,
        options: HoverOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Playwright-shaped strict focus using the element's native focus behavior.
pub trait LocatorFocusExt {
    fn focus(&self, metadata: CallMetadata) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Playwright-shaped strict scroll that waits only for element stability and
/// avoids repositioning an element that is already fully in the viewport.
pub trait LocatorScrollIntoViewIfNeededExt {
    fn scroll_into_view_if_needed(
        &self,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Playwright-shaped strict blur using the element's native blur behavior.
pub trait LocatorBlurExt {
    fn blur(&self, metadata: CallMetadata) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Playwright-shaped key press after strict focus of the current target.
pub trait LocatorPressExt {
    fn press(
        &self,
        key: PressKey,
        options: PressOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Primitive Playwright-shaped checked-state action.
pub trait LocatorSetCheckedExt {
    fn set_checked(
        &self,
        checked: bool,
        options: SetCheckedOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Playwright-shaped native `<select>` option selection.
pub trait LocatorSelectOptionExt {
    fn select_option(
        &self,
        values: impl Into<SelectOptionValues>,
        options: SelectOptionOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<Vec<String>, LocatorErrorDetails>;
}

/// Playwright-shaped check composed over one SetChecked command.
pub trait LocatorCheckExt {
    fn check(
        &self,
        options: SetCheckedOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Playwright-shaped uncheck composed over one SetChecked command.
pub trait LocatorUncheckExt {
    fn uncheck(
        &self,
        options: SetCheckedOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

pub trait LocatorFillExt {
    fn fill(
        &self,
        value: impl Into<String>,
        options: FillOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Playwright-shaped clear operation composed over the existing Fill command.
///
/// Installing this extension requires only `locator.fill`; it adds no command,
/// protocol route, backend implementation, or resource branch.
pub trait LocatorClearExt {
    fn clear(
        &self,
        options: FillOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

pub trait LocatorCountExt {
    fn count(&self, metadata: CallMetadata) -> CommandFuture<u32, LocatorErrorDetails>;
}

/// A count snapshot paired with one lazy base locator.
///
/// Playwright returns an eagerly allocated array of `nth(i)` locators from
/// `Locator.all()`. Pwright keeps the same observable index/count semantics but
/// stores the base selector once; locators are materialized only by `get` or
/// `iter`. They remain lazy and therefore resolve against the current document
/// when a later operation uses them.
pub struct LocatorList<Channel, Ids> {
    base: Locator<Channel, Ids, Selector>,
    len: u32,
}

impl<Channel, Ids> LocatorList<Channel, Ids> {
    const fn new(base: Locator<Channel, Ids, Selector>, len: u32) -> Self {
        Self { base, len }
    }

    pub const fn len(&self) -> u32 {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn get(&self, index: u32) -> Option<Locator<Channel, Ids, Selector>> {
        (index < self.len).then(|| self.locator_at(index))
    }

    pub fn iter(&self) -> impl Iterator<Item = Locator<Channel, Ids, Selector>> + '_ {
        (0..self.len).map(|index| self.locator_at(index))
    }

    fn locator_at(&self, index: u32) -> Locator<Channel, Ids, Selector> {
        let index = i32::try_from(index)
            .expect("LocatorList length validation keeps every nth index representable");
        self.base.clone().map_query(|selector| selector.nth(index))
    }
}

/// Playwright-shaped current-locator list composed over Count and local `nth`.
///
/// The receiver is consumed so the base selector moves into the returned
/// `LocatorList` rather than being cloned. Clone the Locator explicitly before
/// calling when both values are needed. Installing this extension requires only
/// `locator.count`.
pub trait LocatorAllExt<Channel, Ids>: Sized {
    fn all(
        self,
        metadata: CallMetadata,
    ) -> CommandFuture<LocatorList<Channel, Ids>, LocatorErrorDetails>;
}

/// Playwright-shaped non-strict snapshot of every current match's text content.
///
/// Installing this extension requires only `locator.all_text_contents`; it
/// does not pull in actions, waiting, or DOM-handle support.
pub trait LocatorAllTextContentsExt {
    fn all_text_contents(
        &self,
        metadata: CallMetadata,
    ) -> CommandFuture<Vec<String>, LocatorErrorDetails>;
}

/// Playwright-shaped non-strict snapshot of every current match's rendered text.
///
/// Installing this extension requires only `locator.all_inner_texts`; it does
/// not pull in actions, waiting, raw text-content, or DOM-handle support.
pub trait LocatorAllInnerTextsExt {
    fn all_inner_texts(
        &self,
        metadata: CallMetadata,
    ) -> CommandFuture<Vec<String>, LocatorErrorDetails>;
}

/// Playwright-shaped exact DOM attribute query.
///
/// Installing this extension requires only the `locator.get_attribute`
/// command service. It waits for one strict element and preserves a missing
/// attribute as `None`.
pub trait LocatorGetAttributeExt {
    fn get_attribute(
        &self,
        name: impl Into<String>,
        metadata: CallMetadata,
    ) -> CommandFuture<Option<String>, LocatorErrorDetails>;
}

/// Playwright-shaped exact DOM text-content query.
///
/// Installing this extension requires only the `locator.text_content` command
/// service. It waits for one strict target and preserves DOM `null` as `None`.
pub trait LocatorTextContentExt {
    fn text_content(
        &self,
        metadata: CallMetadata,
    ) -> CommandFuture<Option<String>, LocatorErrorDetails>;
}

/// Playwright-shaped serialized child-markup query.
///
/// Installing this extension requires only the `locator.inner_html` command
/// service. It waits for one strict element and returns its serialized markup.
pub trait LocatorInnerHtmlExt {
    fn inner_html(&self, metadata: CallMetadata) -> CommandFuture<String, LocatorErrorDetails>;
}

/// Playwright-shaped rendered HTML text query.
///
/// Installing this extension requires only the `locator.inner_text` command
/// service. It waits for one strict HTMLElement and returns its rendered text.
pub trait LocatorInnerTextExt {
    fn inner_text(&self, metadata: CallMetadata) -> CommandFuture<String, LocatorErrorDetails>;
}

/// Playwright-shaped current form-control value query.
///
/// Installing this extension requires only the `locator.input_value` command
/// service. It waits for one strict input, textarea, select, or associated
/// label and returns the current value without a DOM handle.
pub trait LocatorInputValueExt {
    fn input_value(&self, metadata: CallMetadata) -> CommandFuture<String, LocatorErrorDetails>;
}

/// Playwright-shaped immediate visibility queries.
///
/// `is_hidden` is composed locally by negating `is_visible`; installing this
/// extension therefore adds no duplicate protocol or backend command.
pub trait LocatorVisibilityExt {
    fn is_visible(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails>;

    fn is_hidden(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails>;
}

/// Playwright-shaped strict checked-state getter.
pub trait LocatorCheckedExt {
    fn is_checked(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails>;
}

/// Playwright-shaped strict editability getter.
pub trait LocatorEditableExt {
    fn is_editable(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails>;
}

/// Playwright-shaped strict enabled/disabled state getters.
///
/// `is_disabled` is the local inverse of `is_enabled` and therefore does not
/// require another protocol or backend command.
pub trait LocatorEnabledExt {
    fn is_enabled(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails>;

    fn is_disabled(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails>;
}

/// Playwright-shaped strict wait without exposing or retaining an element
/// handle. Installing this extension requires only the `locator.wait_for`
/// command service.
pub trait LocatorWaitExt {
    fn wait_for(
        &self,
        options: WaitForOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Playwright-shaped selector factories for a page or frame.
///
/// This trait lives in the optional locator crate so the base client does not
/// depend on any concrete selector engine.
pub trait LocatorFactoryExt<Channel, Ids> {
    fn locator_selector(&self, selector: Selector) -> Locator<Channel, Ids, Selector>;

    fn get_by_text(&self, text: impl Into<String>, exact: bool) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::text(text, exact))
    }

    fn get_by_text_regex(&self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::text_regex(matcher))
    }

    fn get_by_label(
        &self,
        text: impl Into<String>,
        exact: bool,
    ) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::label(text, exact))
    }

    fn get_by_label_regex(&self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::label_regex(matcher))
    }

    fn get_by_placeholder(
        &self,
        text: impl Into<String>,
        exact: bool,
    ) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::attribute_text(
            UserFacingAttribute::Placeholder,
            text,
            exact,
        ))
    }

    fn get_by_placeholder_regex(&self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::attribute_regex(
            UserFacingAttribute::Placeholder,
            matcher,
        ))
    }

    fn get_by_alt_text(
        &self,
        text: impl Into<String>,
        exact: bool,
    ) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::attribute_text(
            UserFacingAttribute::Alt,
            text,
            exact,
        ))
    }

    fn get_by_alt_text_regex(&self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::attribute_regex(UserFacingAttribute::Alt, matcher))
    }

    fn get_by_title(
        &self,
        text: impl Into<String>,
        exact: bool,
    ) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::attribute_text(
            UserFacingAttribute::Title,
            text,
            exact,
        ))
    }

    fn get_by_title_regex(&self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::attribute_regex(
            UserFacingAttribute::Title,
            matcher,
        ))
    }

    fn get_by_test_id(&self, value: impl Into<String>) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::test_id(value))
    }

    fn get_by_test_id_with_attributes(
        &self,
        attributes: TestIdAttributes,
        value: impl Into<String>,
    ) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::test_id_with_attributes(attributes, value))
    }

    fn get_by_test_id_regex(&self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::test_id_regex(matcher))
    }

    fn get_by_test_id_regex_with_attributes(
        &self,
        attributes: TestIdAttributes,
        matcher: RegexMatcher,
    ) -> Locator<Channel, Ids, Selector> {
        self.locator_selector(Selector::test_id_regex_with_attributes(attributes, matcher))
    }

    fn get_by_role(
        &self,
        role: AriaRole,
        options: RoleOptions,
    ) -> Result<Locator<Channel, Ids, Selector>, RoleSelectorError> {
        Selector::role(role, options).map(|selector| self.locator_selector(selector))
    }
}

impl<Channel, Ids> LocatorFactoryExt<Channel, Ids> for Page<Channel, Ids> {
    fn locator_selector(&self, selector: Selector) -> Locator<Channel, Ids, Selector> {
        self.locator(selector)
    }
}

impl<Channel, Ids> LocatorFactoryExt<Channel, Ids> for Frame<Channel, Ids> {
    fn locator_selector(&self, selector: Selector) -> Locator<Channel, Ids, Selector> {
        self.locator(selector)
    }
}

/// Purely local locator composition, including Playwright-style chained
/// descendant queries. No command is sent until an action such as `click`.
pub trait LocatorCompositionExt<Channel, Ids>: Sized {
    fn nth(self, index: i32) -> Locator<Channel, Ids, Selector>;

    fn filter_text(self, text: impl Into<String>, exact: bool) -> Locator<Channel, Ids, Selector>;

    fn filter_not_text(
        self,
        text: impl Into<String>,
        exact: bool,
    ) -> Locator<Channel, Ids, Selector>;

    fn filter_text_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector>;

    fn filter_not_text_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector>;

    fn filter_has<Other>(
        self,
        other: Locator<Channel, Ids, Other>,
    ) -> Result<Locator<Channel, Ids, Selector>, LocatorScopeMismatch>
    where
        Other: Into<Selector>;

    fn filter_has_not<Other>(
        self,
        other: Locator<Channel, Ids, Other>,
    ) -> Result<Locator<Channel, Ids, Selector>, LocatorScopeMismatch>
    where
        Other: Into<Selector>;

    fn filter_visible(self, visible: bool) -> Locator<Channel, Ids, Selector>;

    fn get_by_text(self, text: impl Into<String>, exact: bool) -> Locator<Channel, Ids, Selector>;

    fn get_by_text_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector>;

    fn get_by_label(self, text: impl Into<String>, exact: bool) -> Locator<Channel, Ids, Selector>;

    fn get_by_label_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector>;

    fn get_by_placeholder(
        self,
        text: impl Into<String>,
        exact: bool,
    ) -> Locator<Channel, Ids, Selector>;

    fn get_by_placeholder_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector>;

    fn get_by_alt_text(
        self,
        text: impl Into<String>,
        exact: bool,
    ) -> Locator<Channel, Ids, Selector>;

    fn get_by_alt_text_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector>;

    fn get_by_title(self, text: impl Into<String>, exact: bool) -> Locator<Channel, Ids, Selector>;

    fn get_by_title_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector>;

    fn get_by_test_id(self, value: impl Into<String>) -> Locator<Channel, Ids, Selector>;

    fn get_by_test_id_with_attributes(
        self,
        attributes: TestIdAttributes,
        value: impl Into<String>,
    ) -> Locator<Channel, Ids, Selector>;

    fn get_by_test_id_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector>;

    fn get_by_test_id_regex_with_attributes(
        self,
        attributes: TestIdAttributes,
        matcher: RegexMatcher,
    ) -> Locator<Channel, Ids, Selector>;

    fn get_by_role(
        self,
        role: AriaRole,
        options: RoleOptions,
    ) -> Result<Locator<Channel, Ids, Selector>, RoleSelectorError>;

    fn and<Other>(
        self,
        other: Locator<Channel, Ids, Other>,
    ) -> Result<Locator<Channel, Ids, Selector>, LocatorScopeMismatch>
    where
        Other: Into<Selector>;

    fn or<Other>(
        self,
        other: Locator<Channel, Ids, Other>,
    ) -> Result<Locator<Channel, Ids, Selector>, LocatorScopeMismatch>
    where
        Other: Into<Selector>;
}

impl<Channel, Ids, Query> LocatorCompositionExt<Channel, Ids> for Locator<Channel, Ids, Query>
where
    Query: Into<Selector>,
{
    fn nth(self, index: i32) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().nth(index))
    }

    fn filter_text(self, text: impl Into<String>, exact: bool) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().filter_text(text, exact))
    }

    fn filter_not_text(
        self,
        text: impl Into<String>,
        exact: bool,
    ) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().filter_not_text(text, exact))
    }

    fn filter_text_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().filter_text_regex(matcher))
    }

    fn filter_not_text_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().filter_not_text_regex(matcher))
    }

    fn filter_has<Other>(
        self,
        other: Locator<Channel, Ids, Other>,
    ) -> Result<Locator<Channel, Ids, Selector>, LocatorScopeMismatch>
    where
        Other: Into<Selector>,
    {
        self.combine(other, |base, descendant| {
            base.into().filter_has(descendant.into())
        })
    }

    fn filter_has_not<Other>(
        self,
        other: Locator<Channel, Ids, Other>,
    ) -> Result<Locator<Channel, Ids, Selector>, LocatorScopeMismatch>
    where
        Other: Into<Selector>,
    {
        self.combine(other, |base, descendant| {
            base.into().filter_has_not(descendant.into())
        })
    }

    fn filter_visible(self, visible: bool) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().filter_visible(visible))
    }

    fn get_by_text(self, text: impl Into<String>, exact: bool) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().within(Selector::text(text, exact)))
    }

    fn get_by_text_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().within(Selector::text_regex(matcher)))
    }

    fn get_by_label(self, text: impl Into<String>, exact: bool) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().within(Selector::label(text, exact)))
    }

    fn get_by_label_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().within(Selector::label_regex(matcher)))
    }

    fn get_by_placeholder(
        self,
        text: impl Into<String>,
        exact: bool,
    ) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| {
            selector.into().within(Selector::attribute_text(
                UserFacingAttribute::Placeholder,
                text,
                exact,
            ))
        })
    }

    fn get_by_placeholder_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| {
            selector.into().within(Selector::attribute_regex(
                UserFacingAttribute::Placeholder,
                matcher,
            ))
        })
    }

    fn get_by_alt_text(
        self,
        text: impl Into<String>,
        exact: bool,
    ) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| {
            selector.into().within(Selector::attribute_text(
                UserFacingAttribute::Alt,
                text,
                exact,
            ))
        })
    }

    fn get_by_alt_text_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| {
            selector
                .into()
                .within(Selector::attribute_regex(UserFacingAttribute::Alt, matcher))
        })
    }

    fn get_by_title(self, text: impl Into<String>, exact: bool) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| {
            selector.into().within(Selector::attribute_text(
                UserFacingAttribute::Title,
                text,
                exact,
            ))
        })
    }

    fn get_by_title_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| {
            selector.into().within(Selector::attribute_regex(
                UserFacingAttribute::Title,
                matcher,
            ))
        })
    }

    fn get_by_test_id(self, value: impl Into<String>) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().within(Selector::test_id(value)))
    }

    fn get_by_test_id_with_attributes(
        self,
        attributes: TestIdAttributes,
        value: impl Into<String>,
    ) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| {
            selector
                .into()
                .within(Selector::test_id_with_attributes(attributes, value))
        })
    }

    fn get_by_test_id_regex(self, matcher: RegexMatcher) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| selector.into().within(Selector::test_id_regex(matcher)))
    }

    fn get_by_test_id_regex_with_attributes(
        self,
        attributes: TestIdAttributes,
        matcher: RegexMatcher,
    ) -> Locator<Channel, Ids, Selector> {
        self.map_query(|selector| {
            selector
                .into()
                .within(Selector::test_id_regex_with_attributes(attributes, matcher))
        })
    }

    fn get_by_role(
        self,
        role: AriaRole,
        options: RoleOptions,
    ) -> Result<Locator<Channel, Ids, Selector>, RoleSelectorError> {
        let descendant = Selector::role(role, options)?;
        Ok(self.map_query(|selector| selector.into().within(descendant)))
    }

    fn and<Other>(
        self,
        other: Locator<Channel, Ids, Other>,
    ) -> Result<Locator<Channel, Ids, Selector>, LocatorScopeMismatch>
    where
        Other: Into<Selector>,
    {
        self.combine(other, |left, right| left.into().and(right.into()))
    }

    fn or<Other>(
        self,
        other: Locator<Channel, Ids, Other>,
    ) -> Result<Locator<Channel, Ids, Selector>, LocatorScopeMismatch>
    where
        Other: Into<Selector>,
    {
        self.combine(other, |left, right| left.into().or(right.into()))
    }
}

impl<Channel, Ids, Query> LocatorClickExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<Click>, Response = CommandResult<Click>, Error = Infallible> + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn click(
        &self,
        options: ClickOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<Click>(ClickParams::new(self.query().clone(), options), metadata)
    }
}

impl<Channel, Ids, Query> LocatorHoverExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<Hover>, Response = CommandResult<Hover>, Error = Infallible> + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn hover(
        &self,
        options: HoverOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<Hover>(
            HoverParams::new(self.query().clone().into(), options),
            metadata,
        )
    }
}

impl<Channel, Ids, Query> LocatorFocusExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<Focus>, Response = CommandResult<Focus>, Error = Infallible> + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn focus(&self, metadata: CallMetadata) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<Focus>(FocusParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorScrollIntoViewIfNeededExt for Locator<Channel, Ids, Query>
where
    Channel: Service<
            Command<ScrollIntoViewIfNeeded>,
            Response = CommandResult<ScrollIntoViewIfNeeded>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn scroll_into_view_if_needed(
        &self,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<ScrollIntoViewIfNeeded>(
            ScrollIntoViewIfNeededParams::new(self.query().clone()),
            metadata,
        )
    }
}

impl<Channel, Ids, Query> LocatorBlurExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<Blur>, Response = CommandResult<Blur>, Error = Infallible> + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn blur(&self, metadata: CallMetadata) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<Blur>(BlurParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorPressExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<Press>, Response = CommandResult<Press>, Error = Infallible> + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn press(
        &self,
        key: PressKey,
        options: PressOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<Press>(
            PressParams::new(self.query().clone(), key, options),
            metadata,
        )
    }
}

impl<Channel, Ids, Query> LocatorSetCheckedExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<SetChecked>, Response = CommandResult<SetChecked>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn set_checked(
        &self,
        checked: bool,
        options: SetCheckedOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<SetChecked>(
            SetCheckedParams::new(self.query().clone(), checked, options),
            metadata,
        )
    }
}

impl<Channel, Ids, Query> LocatorSelectOptionExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<SelectOption>, Response = CommandResult<SelectOption>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn select_option(
        &self,
        values: impl Into<SelectOptionValues>,
        options: SelectOptionOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<Vec<String>, LocatorErrorDetails> {
        self.dispatch::<SelectOption>(
            SelectOptionParams::new(self.query().clone(), values, options),
            metadata,
        )
    }
}

impl<Channel, Ids, Query> LocatorCheckExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<SetChecked>, Response = CommandResult<SetChecked>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn check(
        &self,
        options: SetCheckedOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        LocatorSetCheckedExt::set_checked(self, true, options, metadata)
    }
}

impl<Channel, Ids, Query> LocatorUncheckExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<SetChecked>, Response = CommandResult<SetChecked>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn uncheck(
        &self,
        options: SetCheckedOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        LocatorSetCheckedExt::set_checked(self, false, options, metadata)
    }
}

impl<Channel, Ids, Query> LocatorFillExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<Fill>, Response = CommandResult<Fill>, Error = Infallible> + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn fill(
        &self,
        value: impl Into<String>,
        options: FillOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<Fill>(
            FillParams::new(self.query().clone(), value, options),
            metadata,
        )
    }
}

impl<Channel, Ids, Query> LocatorClearExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<Fill>, Response = CommandResult<Fill>, Error = Infallible> + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn clear(
        &self,
        options: FillOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<Fill>(
            FillParams::new(self.query().clone(), String::new(), options),
            metadata,
        )
    }
}

impl<Channel, Ids, Query> LocatorCountExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<Count>, Response = CommandResult<Count>, Error = Infallible> + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn count(&self, metadata: CallMetadata) -> CommandFuture<u32, LocatorErrorDetails> {
        self.dispatch::<Count>(CountParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorAllExt<Channel, Ids> for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<Count>, Response = CommandResult<Count>, Error = Infallible>
        + Send
        + Sync
        + 'static,
    Ids: CallIdSource + Send + Sync + 'static,
    Query: Clone + Into<Selector>,
{
    fn all(
        self,
        metadata: CallMetadata,
    ) -> CommandFuture<LocatorList<Channel, Ids>, LocatorErrorDetails> {
        let count = self.dispatch::<Count>(CountParams::new(self.query().clone()), metadata);
        let base = self.map_query(Into::into);
        Box::pin(async move {
            let len = count.await?;
            if len > MAX_LOCATOR_LIST_LENGTH {
                let message = format!(
                    "locator match count {len} exceeds the maximum representable nth index {}",
                    MAX_LOCATOR_LIST_LENGTH
                );
                return Err(OperationError::new(
                    FailureKind::ResourceExhausted,
                    message.clone(),
                    Some(LocatorErrorDetails::Backend(message)),
                ));
            }
            Ok(LocatorList::new(base, len))
        })
    }
}

impl<Channel, Ids, Query> LocatorAllTextContentsExt for Locator<Channel, Ids, Query>
where
    Channel: Service<
            Command<AllTextContents>,
            Response = CommandResult<AllTextContents>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn all_text_contents(
        &self,
        metadata: CallMetadata,
    ) -> CommandFuture<Vec<String>, LocatorErrorDetails> {
        self.dispatch::<AllTextContents>(AllTextContentsParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorAllInnerTextsExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<AllInnerTexts>, Response = CommandResult<AllInnerTexts>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn all_inner_texts(
        &self,
        metadata: CallMetadata,
    ) -> CommandFuture<Vec<String>, LocatorErrorDetails> {
        self.dispatch::<AllInnerTexts>(AllInnerTextsParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorGetAttributeExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<GetAttribute>, Response = CommandResult<GetAttribute>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn get_attribute(
        &self,
        name: impl Into<String>,
        metadata: CallMetadata,
    ) -> CommandFuture<Option<String>, LocatorErrorDetails> {
        self.dispatch::<GetAttribute>(
            GetAttributeParams::new(self.query().clone(), name),
            metadata,
        )
    }
}

impl<Channel, Ids, Query> LocatorTextContentExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<TextContent>, Response = CommandResult<TextContent>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn text_content(
        &self,
        metadata: CallMetadata,
    ) -> CommandFuture<Option<String>, LocatorErrorDetails> {
        self.dispatch::<TextContent>(TextContentParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorInnerHtmlExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<InnerHtml>, Response = CommandResult<InnerHtml>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn inner_html(&self, metadata: CallMetadata) -> CommandFuture<String, LocatorErrorDetails> {
        self.dispatch::<InnerHtml>(InnerHtmlParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorInnerTextExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<InnerText>, Response = CommandResult<InnerText>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn inner_text(&self, metadata: CallMetadata) -> CommandFuture<String, LocatorErrorDetails> {
        self.dispatch::<InnerText>(InnerTextParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorInputValueExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<InputValue>, Response = CommandResult<InputValue>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn input_value(&self, metadata: CallMetadata) -> CommandFuture<String, LocatorErrorDetails> {
        self.dispatch::<InputValue>(InputValueParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorVisibilityExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<IsVisible>, Response = CommandResult<IsVisible>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn is_visible(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails> {
        self.dispatch::<IsVisible>(IsVisibleParams::new(self.query().clone()), metadata)
    }

    fn is_hidden(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails> {
        let visible = self.is_visible(metadata);
        Box::pin(async move { visible.await.map(|visible| !visible) })
    }
}

impl<Channel, Ids, Query> LocatorCheckedExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<IsChecked>, Response = CommandResult<IsChecked>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn is_checked(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails> {
        self.dispatch::<IsChecked>(IsCheckedParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorEditableExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<IsEditable>, Response = CommandResult<IsEditable>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn is_editable(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails> {
        self.dispatch::<IsEditable>(IsEditableParams::new(self.query().clone()), metadata)
    }
}

impl<Channel, Ids, Query> LocatorEnabledExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<IsEnabled>, Response = CommandResult<IsEnabled>, Error = Infallible>
        + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn is_enabled(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails> {
        self.dispatch::<IsEnabled>(IsEnabledParams::new(self.query().clone()), metadata)
    }

    fn is_disabled(&self, metadata: CallMetadata) -> CommandFuture<bool, LocatorErrorDetails> {
        let enabled = self.is_enabled(metadata);
        Box::pin(async move { enabled.await.map(|enabled| !enabled) })
    }
}

impl<Channel, Ids, Query> LocatorWaitExt for Locator<Channel, Ids, Query>
where
    Channel:
        Service<Command<WaitFor>, Response = CommandResult<WaitFor>, Error = Infallible> + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn wait_for(
        &self,
        options: WaitForOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<WaitFor>(WaitForParams::new(self.query().clone(), options), metadata)
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;
    use std::future::{Ready, ready};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use pwright_cap_locator::{
        AllInnerTexts, AllInnerTextsParams, AllTextContents, AllTextContentsParams, AriaRole, Blur,
        BlurParams, Click, ClickOptions, ClickParams, Count, CountParams, CssSelector, Fill,
        FillOptions, FillParams, Focus, FocusParams, GetAttribute, GetAttributeParams, Hover,
        HoverOptions, HoverParams, InnerHtml, InnerHtmlParams, InnerText, InnerTextParams,
        InputValue, InputValueParams, IsChecked, IsCheckedParams, IsEditable, IsEditableParams,
        IsEnabled, IsEnabledParams, IsVisible, IsVisibleParams, MouseButton, RegexMatcher,
        RoleOptions, ScrollIntoViewIfNeeded, ScrollIntoViewIfNeededParams, Selector,
        TestIdAttributes, TextContent, TextContentParams, UserFacingAttribute, WaitFor,
        WaitForOptions, WaitForParams, WaitForState,
    };
    use pwright_client::{
        AtomicCallIdSource, BrowserContextKind, BrowserKind, Client, FrameKind, Page, PageKind,
        RemoteInitializer, RemoteKind,
    };
    use pwright_core::{
        CallId, CallMetadata, CancellationId, Command, CommandResult, FailureKind, ObjectId,
        Service,
    };

    use super::{
        LocatorAllExt, LocatorAllInnerTextsExt, LocatorAllTextContentsExt, LocatorBlurExt,
        LocatorCheckedExt, LocatorClearExt, LocatorClickExt, LocatorCompositionExt,
        LocatorCountExt, LocatorEditableExt, LocatorEnabledExt, LocatorFactoryExt, LocatorFillExt,
        LocatorFocusExt, LocatorGetAttributeExt, LocatorHoverExt, LocatorInnerHtmlExt,
        LocatorInnerTextExt, LocatorInputValueExt, LocatorScrollIntoViewIfNeededExt,
        LocatorTextContentExt, LocatorVisibilityExt, LocatorWaitExt, MAX_LOCATOR_LIST_LENGTH,
    };

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedClick {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: ClickParams,
    }

    struct RecordingChannel {
        calls: Arc<Mutex<Vec<RecordedClick>>>,
        fills: Arc<Mutex<Vec<RecordedFill>>>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedCount {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: CountParams,
    }

    struct CountingChannel {
        calls: Arc<Mutex<Vec<RecordedCount>>>,
        result: u32,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedAllTextContents {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: AllTextContentsParams,
    }

    struct AllTextContentsChannel {
        calls: Arc<Mutex<Vec<RecordedAllTextContents>>>,
        result: Vec<String>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedAllInnerTexts {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: AllInnerTextsParams,
    }

    struct AllInnerTextsChannel {
        calls: Arc<Mutex<Vec<RecordedAllInnerTexts>>>,
        result: Vec<String>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedVisibility {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: IsVisibleParams,
    }

    struct VisibilityChannel {
        calls: Arc<Mutex<Vec<RecordedVisibility>>>,
        result: bool,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum RecordedState {
        Checked(IsCheckedParams),
        Editable(IsEditableParams),
        Enabled(IsEnabledParams),
    }

    struct StateChannel {
        calls: Arc<Mutex<Vec<RecordedState>>>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedWait {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: WaitForParams,
    }

    struct WaitingChannel {
        calls: Arc<Mutex<Vec<RecordedWait>>>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedTextContent {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: TextContentParams,
    }

    struct TextContentChannel {
        calls: Arc<Mutex<Vec<RecordedTextContent>>>,
        result: Option<String>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedGetAttribute {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: GetAttributeParams,
    }

    struct GetAttributeChannel {
        calls: Arc<Mutex<Vec<RecordedGetAttribute>>>,
        result: Option<String>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedInnerHtml {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: InnerHtmlParams,
    }

    struct InnerHtmlChannel {
        calls: Arc<Mutex<Vec<RecordedInnerHtml>>>,
        result: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedInnerText {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: InnerTextParams,
    }

    struct InnerTextChannel {
        calls: Arc<Mutex<Vec<RecordedInnerText>>>,
        result: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedInputValue {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: InputValueParams,
    }

    struct InputValueChannel {
        calls: Arc<Mutex<Vec<RecordedInputValue>>>,
        result: String,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedFill {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: FillParams,
    }

    #[derive(Clone, Debug, PartialEq)]
    struct RecordedHover {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: HoverParams,
    }

    struct HoverChannel {
        calls: Arc<Mutex<Vec<RecordedHover>>>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    enum RecordedFocusManagement {
        Focus {
            call_id: CallId,
            target: ObjectId,
            metadata: CallMetadata,
            params: FocusParams,
        },
        Blur {
            call_id: CallId,
            target: ObjectId,
            metadata: CallMetadata,
            params: BlurParams,
        },
    }

    struct FocusManagementChannel {
        calls: Arc<Mutex<Vec<RecordedFocusManagement>>>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RecordedScrollIntoViewIfNeeded {
        call_id: CallId,
        target: ObjectId,
        metadata: CallMetadata,
        params: ScrollIntoViewIfNeededParams,
    }

    struct ScrollIntoViewIfNeededChannel {
        calls: Arc<Mutex<Vec<RecordedScrollIntoViewIfNeeded>>>,
    }

    fn test_page<Channel>(
        client: &Client<Channel, AtomicCallIdSource>,
    ) -> Page<Channel, AtomicCallIdSource> {
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .expect("browser should initialize");
        let context = browser
            .context(RemoteInitializer::new(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BrowserContextKind::OBJECT_KIND,
            ))
            .expect("context should initialize");
        context
            .page(RemoteInitializer::new(
                ObjectId::new(7),
                Some(ObjectId::new(2)),
                PageKind::OBJECT_KIND,
            ))
            .expect("page should initialize")
    }

    impl Service<Command<Click>> for RecordingChannel {
        type Response = CommandResult<Click>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<Click>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("click calls should be available")
                .push(RecordedClick {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
        }
    }

    impl Service<Command<Fill>> for RecordingChannel {
        type Response = CommandResult<Fill>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<Fill>) -> Self::Future<'_> {
            self.fills
                .lock()
                .expect("fill calls should be available")
                .push(RecordedFill {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
        }
    }

    impl Service<Command<Hover>> for HoverChannel {
        type Response = CommandResult<Hover>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<Hover>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("hover calls should be available")
                .push(RecordedHover {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
        }
    }

    impl Service<Command<Focus>> for FocusManagementChannel {
        type Response = CommandResult<Focus>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<Focus>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("focus-management calls should be available")
                .push(RecordedFocusManagement::Focus {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
        }
    }

    impl Service<Command<Blur>> for FocusManagementChannel {
        type Response = CommandResult<Blur>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<Blur>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("focus-management calls should be available")
                .push(RecordedFocusManagement::Blur {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
        }
    }

    impl Service<Command<ScrollIntoViewIfNeeded>> for ScrollIntoViewIfNeededChannel {
        type Response = CommandResult<ScrollIntoViewIfNeeded>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<ScrollIntoViewIfNeeded>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("scroll calls should be available")
                .push(RecordedScrollIntoViewIfNeeded {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
        }
    }

    impl Service<Command<Count>> for CountingChannel {
        type Response = CommandResult<Count>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<Count>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("count calls should be available")
                .push(RecordedCount {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(command.call_id(), Ok(self.result))))
        }
    }

    impl Service<Command<AllTextContents>> for AllTextContentsChannel {
        type Response = CommandResult<AllTextContents>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<AllTextContents>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("all-text-contents calls should be available")
                .push(RecordedAllTextContents {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(self.result.clone()),
            )))
        }
    }

    impl Service<Command<AllInnerTexts>> for AllInnerTextsChannel {
        type Response = CommandResult<AllInnerTexts>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<AllInnerTexts>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("all-inner-texts calls should be available")
                .push(RecordedAllInnerTexts {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(self.result.clone()),
            )))
        }
    }

    impl Service<Command<IsVisible>> for VisibilityChannel {
        type Response = CommandResult<IsVisible>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<IsVisible>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("visibility calls should be available")
                .push(RecordedVisibility {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(command.call_id(), Ok(self.result))))
        }
    }

    impl Service<Command<IsChecked>> for StateChannel {
        type Response = CommandResult<IsChecked>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<IsChecked>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("state calls should be available")
                .push(RecordedState::Checked(command.params().clone()));
            ready(Ok(CommandResult::new(command.call_id(), Ok(true))))
        }
    }

    impl Service<Command<IsEditable>> for StateChannel {
        type Response = CommandResult<IsEditable>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<IsEditable>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("state calls should be available")
                .push(RecordedState::Editable(command.params().clone()));
            ready(Ok(CommandResult::new(command.call_id(), Ok(false))))
        }
    }

    impl Service<Command<IsEnabled>> for StateChannel {
        type Response = CommandResult<IsEnabled>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<IsEnabled>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("state calls should be available")
                .push(RecordedState::Enabled(command.params().clone()));
            ready(Ok(CommandResult::new(command.call_id(), Ok(false))))
        }
    }

    impl Service<Command<WaitFor>> for WaitingChannel {
        type Response = CommandResult<WaitFor>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<WaitFor>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("wait calls should be available")
                .push(RecordedWait {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(command.call_id(), Ok(()))))
        }
    }

    impl Service<Command<TextContent>> for TextContentChannel {
        type Response = CommandResult<TextContent>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<TextContent>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("text-content calls should be available")
                .push(RecordedTextContent {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(self.result.clone()),
            )))
        }
    }

    impl Service<Command<GetAttribute>> for GetAttributeChannel {
        type Response = CommandResult<GetAttribute>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<GetAttribute>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("get-attribute calls should be available")
                .push(RecordedGetAttribute {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(self.result.clone()),
            )))
        }
    }

    impl Service<Command<InnerText>> for InnerTextChannel {
        type Response = CommandResult<InnerText>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<InnerText>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("inner-text calls should be available")
                .push(RecordedInnerText {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(self.result.clone()),
            )))
        }
    }

    impl Service<Command<InputValue>> for InputValueChannel {
        type Response = CommandResult<InputValue>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<InputValue>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("input-value calls should be available")
                .push(RecordedInputValue {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(self.result.clone()),
            )))
        }
    }

    impl Service<Command<InnerHtml>> for InnerHtmlChannel {
        type Response = CommandResult<InnerHtml>;
        type Error = Infallible;
        type Future<'a> = Ready<Result<Self::Response, Self::Error>>;

        fn call(&self, command: Command<InnerHtml>) -> Self::Future<'_> {
            self.calls
                .lock()
                .expect("inner-html calls should be available")
                .push(RecordedInnerHtml {
                    call_id: command.call_id(),
                    target: command.object_id(),
                    metadata: command.metadata(),
                    params: command.params().clone(),
                });
            ready(Ok(CommandResult::new(
                command.call_id(),
                Ok(self.result.clone()),
            )))
        }
    }

    #[test]
    fn css_locator_click_dispatches_against_its_page_scope() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let fills = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls: Arc::clone(&calls),
                fills,
            },
            AtomicCallIdSource::new(7),
        );
        let browser = client
            .browser(RemoteInitializer::new(
                ObjectId::new(1),
                None,
                BrowserKind::OBJECT_KIND,
            ))
            .expect("browser should initialize");
        let context = browser
            .context(RemoteInitializer::new(
                ObjectId::new(2),
                Some(ObjectId::new(1)),
                BrowserContextKind::OBJECT_KIND,
            ))
            .expect("context should initialize");
        let page = context
            .page(RemoteInitializer::new(
                ObjectId::new(3),
                Some(ObjectId::new(2)),
                PageKind::OBJECT_KIND,
            ))
            .expect("page should initialize");
        let locator = page.locator(CssSelector::new("button.submit").expect("valid CSS"));
        let metadata =
            CallMetadata::new(Some(Duration::from_secs(5)), Some(CancellationId::new(9)));
        let options = ClickOptions::new().with_button(MouseButton::Right);

        // Act
        let result = futures_lite_block_on(locator.click(options, metadata));

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            calls
                .lock()
                .expect("click calls should be available")
                .as_slice(),
            &[RecordedClick {
                call_id: CallId::new(7),
                target: ObjectId::new(3),
                metadata,
                params: ClickParams::new(
                    CssSelector::new("button.submit").expect("valid CSS"),
                    options,
                ),
            }]
        );
    }

    #[test]
    fn locator_hover_dispatches_only_the_hover_command_service() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            HoverChannel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(19),
        );
        let locator =
            test_page(&client).locator(CssSelector::new("#target").expect("valid hover CSS"));
        let metadata = CallMetadata::new(Some(Duration::from_secs(2)), None);
        let options = HoverOptions::new().with_force(true);

        // Act
        let result = futures_lite_block_on(locator.hover(options, metadata));

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            calls
                .lock()
                .expect("hover calls should be available")
                .as_slice(),
            &[RecordedHover {
                call_id: CallId::new(19),
                target: ObjectId::new(7),
                metadata,
                params: HoverParams::new(
                    CssSelector::new("#target").expect("valid hover CSS").into(),
                    options,
                ),
            }]
        );
    }

    #[test]
    fn locator_focus_dispatches_selector_metadata_and_page_scope() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            FocusManagementChannel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(20),
        );
        let locator = test_page(&client).locator(Selector::test_id("focus-target"));
        let metadata =
            CallMetadata::new(Some(Duration::from_secs(4)), Some(CancellationId::new(45)));

        // Act
        let result = futures_lite_block_on(locator.focus(metadata));

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            calls
                .lock()
                .expect("focus-management calls should be available")
                .as_slice(),
            &[RecordedFocusManagement::Focus {
                call_id: CallId::new(20),
                target: ObjectId::new(7),
                metadata,
                params: FocusParams::new(Selector::test_id("focus-target")),
            }]
        );
    }

    #[test]
    fn locator_scroll_into_view_dispatches_selector_metadata_and_page_scope() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            ScrollIntoViewIfNeededChannel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(21),
        );
        let locator = test_page(&client).locator(Selector::test_id("below-fold"));
        let metadata =
            CallMetadata::new(Some(Duration::from_secs(3)), Some(CancellationId::new(46)));

        // Act
        let result = futures_lite_block_on(locator.scroll_into_view_if_needed(metadata));

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            calls
                .lock()
                .expect("scroll calls should be available")
                .as_slice(),
            &[RecordedScrollIntoViewIfNeeded {
                call_id: CallId::new(21),
                target: ObjectId::new(7),
                metadata,
                params: ScrollIntoViewIfNeededParams::new(Selector::test_id("below-fold")),
            }]
        );
    }

    #[test]
    fn frame_locator_focus_dispatches_against_the_frame_scope() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            FocusManagementChannel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(22),
        );
        let page = test_page(&client);
        let frame = page
            .frame(RemoteInitializer::new(
                ObjectId::new(8),
                Some(ObjectId::new(7)),
                FrameKind::OBJECT_KIND,
            ))
            .expect("child Frame should initialize");
        let locator = frame.locator(Selector::test_id("frame-focus-target"));
        let metadata = CallMetadata::default();

        // Act
        let result = futures_lite_block_on(locator.focus(metadata));

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            calls
                .lock()
                .expect("focus-management calls should be available")
                .as_slice(),
            &[RecordedFocusManagement::Focus {
                call_id: CallId::new(22),
                target: ObjectId::new(8),
                metadata,
                params: FocusParams::new(Selector::test_id("frame-focus-target")),
            }]
        );
    }

    #[test]
    fn locator_blur_dispatches_selector_metadata_and_page_scope() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            FocusManagementChannel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(21),
        );
        let locator = test_page(&client).locator(Selector::test_id("blur-target"));
        let metadata = CallMetadata::new(Some(Duration::from_secs(5)), None);

        // Act
        let result = futures_lite_block_on(locator.blur(metadata));

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            calls
                .lock()
                .expect("focus-management calls should be available")
                .as_slice(),
            &[RecordedFocusManagement::Blur {
                call_id: CallId::new(21),
                target: ObjectId::new(7),
                metadata,
                params: BlurParams::new(Selector::test_id("blur-target")),
            }]
        );
    }

    #[test]
    fn locator_fill_dispatches_value_options_and_metadata_against_page_scope() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let fills = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls,
                fills: Arc::clone(&fills),
            },
            AtomicCallIdSource::new(12),
        );
        let page = test_page(&client);
        let locator = page.get_by_label("Email", true);
        let metadata =
            CallMetadata::new(Some(Duration::from_secs(3)), Some(CancellationId::new(44)));
        let options = FillOptions::new().with_force(true);

        // Act
        let result = futures_lite_block_on(locator.fill("user@example.com", options, metadata));

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            fills
                .lock()
                .expect("fill calls should be available")
                .as_slice(),
            &[RecordedFill {
                call_id: CallId::new(12),
                target: ObjectId::new(7),
                metadata,
                params: FillParams::new(
                    Selector::label("Email", true),
                    "user@example.com",
                    options,
                ),
            }]
        );
    }

    #[test]
    fn locator_clear_is_composed_as_one_empty_fill_command() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let fills = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            RecordingChannel {
                calls,
                fills: Arc::clone(&fills),
            },
            AtomicCallIdSource::new(13),
        );
        let page = test_page(&client);
        let locator = page.get_by_label("Email", true);
        let metadata =
            CallMetadata::new(Some(Duration::from_secs(3)), Some(CancellationId::new(45)));
        let options = FillOptions::new().with_force(true);

        // Act
        let result = futures_lite_block_on(locator.clear(options, metadata));

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            fills
                .lock()
                .expect("fill calls should be available")
                .as_slice(),
            &[RecordedFill {
                call_id: CallId::new(13),
                target: ObjectId::new(7),
                metadata,
                params: FillParams::new(Selector::label("Email", true), "", options),
            }]
        );
    }

    #[test]
    fn locator_count_dispatches_a_non_strict_snapshot_query() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            CountingChannel {
                calls: Arc::clone(&calls),
                result: 3,
            },
            AtomicCallIdSource::new(21),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("row");
        let metadata = CallMetadata::new(None, Some(CancellationId::new(8)));

        // Act
        let result = futures_lite_block_on(locator.count(metadata));

        // Assert
        assert_eq!(result, Ok(3));
        assert_eq!(
            calls
                .lock()
                .expect("count calls should be available")
                .as_slice(),
            &[RecordedCount {
                call_id: CallId::new(21),
                target: ObjectId::new(7),
                metadata,
                params: CountParams::new(Selector::test_id("row")),
            }]
        );
    }

    #[test]
    fn locator_all_composes_one_count_snapshot_with_lazy_nth_locators() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            CountingChannel {
                calls: Arc::clone(&calls),
                result: 3,
            },
            AtomicCallIdSource::new(24),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("row");
        let metadata = CallMetadata::new(None, Some(CancellationId::new(11)));

        // Act
        let list = futures_lite_block_on(locator.all(metadata)).expect("list should build");
        let queries: Vec<_> = list.iter().map(|locator| locator.into_query()).collect();

        // Assert
        assert_eq!(list.len(), 3);
        assert!(!list.is_empty());
        assert_eq!(
            queries,
            [
                Selector::test_id("row").nth(0),
                Selector::test_id("row").nth(1),
                Selector::test_id("row").nth(2),
            ]
        );
        assert_eq!(
            list.get(1).map(|locator| locator.into_query()),
            Some(Selector::test_id("row").nth(1))
        );
        assert!(list.get(3).is_none());
        assert_eq!(
            calls
                .lock()
                .expect("count calls should be available")
                .as_slice(),
            &[RecordedCount {
                call_id: CallId::new(24),
                target: ObjectId::new(7),
                metadata,
                params: CountParams::new(Selector::test_id("row")),
            }]
        );
    }

    #[test]
    fn locator_all_rejects_a_count_that_cannot_be_expressed_as_nth() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            CountingChannel {
                calls: Arc::clone(&calls),
                result: MAX_LOCATOR_LIST_LENGTH + 1,
            },
            AtomicCallIdSource::new(25),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("row");

        // Act
        let result = futures_lite_block_on(locator.all(CallMetadata::default()));

        // Assert
        let Err(error) = result else {
            panic!("an unrepresentable nth count should fail before list construction");
        };
        assert_eq!(error.kind(), FailureKind::ResourceExhausted);
        assert_eq!(
            calls.lock().expect("count calls should be available").len(),
            1
        );
    }

    #[test]
    fn locator_all_text_contents_dispatches_only_its_snapshot_service() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            AllTextContentsChannel {
                calls: Arc::clone(&calls),
                result: vec!["first".to_string(), String::new(), "third".to_string()],
            },
            AtomicCallIdSource::new(22),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("row");
        let metadata = CallMetadata::new(None, Some(CancellationId::new(9)));

        // Act
        let result = futures_lite_block_on(locator.all_text_contents(metadata));

        // Assert
        assert_eq!(result, Ok(vec!["first".into(), "".into(), "third".into()]));
        assert_eq!(
            calls
                .lock()
                .expect("all-text-contents calls should be available")
                .as_slice(),
            &[RecordedAllTextContents {
                call_id: CallId::new(22),
                target: ObjectId::new(7),
                metadata,
                params: AllTextContentsParams::new(Selector::test_id("row")),
            }]
        );
    }

    #[test]
    fn locator_all_inner_texts_dispatches_only_its_rendered_snapshot_service() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            AllInnerTextsChannel {
                calls: Arc::clone(&calls),
                result: vec!["first".to_string(), String::new(), "third".to_string()],
            },
            AtomicCallIdSource::new(23),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("row");
        let metadata = CallMetadata::new(None, Some(CancellationId::new(10)));

        // Act
        let result = futures_lite_block_on(locator.all_inner_texts(metadata));

        // Assert
        assert_eq!(result, Ok(vec!["first".into(), "".into(), "third".into()]));
        assert_eq!(
            calls
                .lock()
                .expect("all-inner-texts calls should be available")
                .as_slice(),
            &[RecordedAllInnerTexts {
                call_id: CallId::new(23),
                target: ObjectId::new(7),
                metadata,
                params: AllInnerTextsParams::new(Selector::test_id("row")),
            }]
        );
    }

    #[test]
    fn locator_hidden_is_composed_from_the_visibility_command() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            VisibilityChannel {
                calls: Arc::clone(&calls),
                result: false,
            },
            AtomicCallIdSource::new(23),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("status");
        let visible_metadata = CallMetadata::default();
        let hidden_metadata = CallMetadata::new(None, Some(CancellationId::new(9)));

        // Act
        let visible = futures_lite_block_on(locator.is_visible(visible_metadata));
        let hidden = futures_lite_block_on(locator.is_hidden(hidden_metadata));

        // Assert
        assert_eq!(visible, Ok(false));
        assert_eq!(hidden, Ok(true));
        assert_eq!(
            calls
                .lock()
                .expect("visibility calls should be available")
                .as_slice(),
            &[
                RecordedVisibility {
                    call_id: CallId::new(23),
                    target: ObjectId::new(7),
                    metadata: visible_metadata,
                    params: IsVisibleParams::new(Selector::test_id("status")),
                },
                RecordedVisibility {
                    call_id: CallId::new(24),
                    target: ObjectId::new(7),
                    metadata: hidden_metadata,
                    params: IsVisibleParams::new(Selector::test_id("status")),
                },
            ]
        );
    }

    #[test]
    fn locator_state_getters_dispatch_independently_and_disabled_composes_over_enabled() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            StateChannel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(31),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("control");

        // Act
        let checked = futures_lite_block_on(locator.is_checked(CallMetadata::default()));
        let editable = futures_lite_block_on(locator.is_editable(CallMetadata::default()));
        let enabled = futures_lite_block_on(locator.is_enabled(CallMetadata::default()));
        let disabled = futures_lite_block_on(locator.is_disabled(CallMetadata::default()));

        // Assert
        assert_eq!(checked, Ok(true));
        assert_eq!(editable, Ok(false));
        assert_eq!(enabled, Ok(false));
        assert_eq!(disabled, Ok(true));
        assert_eq!(
            calls
                .lock()
                .expect("state calls should be available")
                .as_slice(),
            &[
                RecordedState::Checked(IsCheckedParams::new(Selector::test_id("control"))),
                RecordedState::Editable(IsEditableParams::new(Selector::test_id("control"))),
                RecordedState::Enabled(IsEnabledParams::new(Selector::test_id("control"))),
                RecordedState::Enabled(IsEnabledParams::new(Selector::test_id("control"))),
            ]
        );
    }

    #[test]
    fn locator_wait_dispatches_only_the_selected_wait_command() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            WaitingChannel {
                calls: Arc::clone(&calls),
            },
            AtomicCallIdSource::new(41),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("status");
        let metadata =
            CallMetadata::new(Some(Duration::from_secs(2)), Some(CancellationId::new(17)));
        let options = WaitForOptions::new().with_state(WaitForState::Hidden);

        // Act
        let result = futures_lite_block_on(locator.wait_for(options, metadata));

        // Assert
        assert_eq!(result, Ok(()));
        assert_eq!(
            calls
                .lock()
                .expect("wait calls should be available")
                .as_slice(),
            &[RecordedWait {
                call_id: CallId::new(41),
                target: ObjectId::new(7),
                metadata,
                params: WaitForParams::new(Selector::test_id("status"), options),
            }]
        );
    }

    #[test]
    fn locator_text_content_dispatches_only_the_selected_query() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            TextContentChannel {
                calls: Arc::clone(&calls),
                result: Some("  Ready\nnow  ".to_string()),
            },
            AtomicCallIdSource::new(51),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("status");
        let metadata = CallMetadata::new(Some(Duration::from_secs(2)), None);

        // Act
        let result = futures_lite_block_on(locator.text_content(metadata));

        // Assert
        assert_eq!(
            result
                .expect("text-content query should succeed")
                .as_deref(),
            Some("  Ready\nnow  ")
        );
        assert_eq!(
            calls
                .lock()
                .expect("text-content calls should be available")
                .as_slice(),
            &[RecordedTextContent {
                call_id: CallId::new(51),
                target: ObjectId::new(7),
                metadata,
                params: TextContentParams::new(Selector::test_id("status")),
            }]
        );
    }

    #[test]
    fn locator_inner_text_dispatches_only_the_selected_query() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            InnerTextChannel {
                calls: Arc::clone(&calls),
                result: "Ready now".to_string(),
            },
            AtomicCallIdSource::new(61),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("status");
        let metadata = CallMetadata::new(Some(Duration::from_secs(2)), None);

        // Act
        let result = futures_lite_block_on(locator.inner_text(metadata));

        // Assert
        assert_eq!(result.as_deref(), Ok("Ready now"));
        assert_eq!(
            calls
                .lock()
                .expect("inner-text calls should be available")
                .as_slice(),
            &[RecordedInnerText {
                call_id: CallId::new(61),
                target: ObjectId::new(7),
                metadata,
                params: InnerTextParams::new(Selector::test_id("status")),
            }]
        );
    }

    #[test]
    fn locator_input_value_dispatches_only_the_selected_query() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            InputValueChannel {
                calls: Arc::clone(&calls),
                result: "alice@example.test".to_string(),
            },
            AtomicCallIdSource::new(66),
        );
        let page = test_page(&client);
        let locator = page.get_by_label("Account", true);
        let metadata = CallMetadata::new(Some(Duration::from_secs(2)), None);

        // Act
        let result = futures_lite_block_on(locator.input_value(metadata));

        // Assert
        assert_eq!(result.as_deref(), Ok("alice@example.test"));
        assert_eq!(
            calls
                .lock()
                .expect("input-value calls should be available")
                .as_slice(),
            &[RecordedInputValue {
                call_id: CallId::new(66),
                target: ObjectId::new(7),
                metadata,
                params: InputValueParams::new(Selector::label("Account", true)),
            }]
        );
    }

    #[test]
    fn locator_inner_html_dispatches_only_the_selected_query() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            InnerHtmlChannel {
                calls: Arc::clone(&calls),
                result: "<span>Ready</span>".to_string(),
            },
            AtomicCallIdSource::new(71),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("container");
        let metadata = CallMetadata::new(Some(Duration::from_secs(2)), None);

        // Act
        let result = futures_lite_block_on(locator.inner_html(metadata));

        // Assert
        assert_eq!(result.as_deref(), Ok("<span>Ready</span>"));
        assert_eq!(
            calls
                .lock()
                .expect("inner-html calls should be available")
                .as_slice(),
            &[RecordedInnerHtml {
                call_id: CallId::new(71),
                target: ObjectId::new(7),
                metadata,
                params: InnerHtmlParams::new(Selector::test_id("container")),
            }]
        );
    }

    #[test]
    fn locator_get_attribute_dispatches_only_the_selected_query() {
        // Arrange
        let calls = Arc::new(Mutex::new(Vec::new()));
        let client = Client::new(
            GetAttributeChannel {
                calls: Arc::clone(&calls),
                result: Some("ready".to_string()),
            },
            AtomicCallIdSource::new(81),
        );
        let page = test_page(&client);
        let locator = page.get_by_test_id("container");
        let metadata = CallMetadata::new(Some(Duration::from_secs(2)), None);

        // Act
        let result = futures_lite_block_on(locator.get_attribute("data-state", metadata));

        // Assert
        assert_eq!(
            result.as_ref().map(|value| value.as_deref()),
            Ok(Some("ready"))
        );
        assert_eq!(
            calls
                .lock()
                .expect("get-attribute calls should be available")
                .as_slice(),
            &[RecordedGetAttribute {
                call_id: CallId::new(81),
                target: ObjectId::new(7),
                metadata,
                params: GetAttributeParams::new(Selector::test_id("container"), "data-state"),
            }]
        );
    }

    #[test]
    fn semantic_locator_factories_and_chaining_are_local_until_an_action() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let page = test_page(&client);
        let dialog = page
            .get_by_role(
                AriaRole::Dialog,
                RoleOptions::new().with_name("Settings", true),
            )
            .expect("dialog selector should be valid");

        // Act
        let save = dialog
            .get_by_role(AriaRole::Button, RoleOptions::new().with_name("Save", true))
            .expect("button selector should be valid")
            .filter_text("Save", false)
            .nth(0);

        // Assert
        assert_eq!(save.scope_id(), ObjectId::new(7));
        assert!(matches!(save.query(), Selector::Nth { index: 0, .. }));
        assert!(save.query().to_string().contains("within"));
    }

    #[test]
    fn semantic_and_or_composition_preserves_same_scope_validation() {
        // Arrange
        let first = Client::new((), AtomicCallIdSource::new(1));
        let second = Client::new((), AtomicCallIdSource::new(1));
        let first_page = test_page(&first);
        let text = first_page.get_by_text("Save", true);
        let label = first_page.get_by_label("Save", true);
        let foreign = test_page(&second).get_by_text("Save", true);

        // Act
        let local = text.or(label);
        let foreign = first_page.get_by_text("Save", true).and(foreign);

        // Assert
        assert!(matches!(
            local
                .expect("same connection and scope should compose")
                .query(),
            Selector::Or { .. }
        ));
        assert!(foreign.is_err());
    }

    #[test]
    fn filter_has_variants_validate_scope_before_building_a_selector() {
        // Arrange
        let first = Client::new((), AtomicCallIdSource::new(1));
        let second = Client::new((), AtomicCallIdSource::new(1));
        let first_page = test_page(&first);
        let cards = first_page.locator_selector(Selector::css(".card").expect("valid CSS"));
        let marker = first_page.locator_selector(Selector::css(".ready").expect("valid CSS"));
        let foreign =
            test_page(&second).locator_selector(Selector::css(".foreign").expect("valid CSS"));

        // Act
        let local = cards
            .filter_has(marker)
            .expect("same-frame has filter should compose")
            .filter_not_text("Blocked", false)
            .filter_visible(true);
        let mismatch = first_page
            .locator_selector(Selector::css(".card").expect("valid CSS"))
            .filter_has_not(foreign);

        // Assert
        assert!(matches!(
            local.query(),
            Selector::FilterVisible { visible: true, .. }
        ));
        assert!(mismatch.is_err());
    }

    #[test]
    fn regex_factories_and_filters_compose_without_dispatching() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let page = test_page(&client);
        let cards = page.locator_selector(Selector::css(".card").expect("valid CSS"));

        // Act
        let locator = cards
            .filter_text_regex(RegexMatcher::new("Release\\s+42", "i").expect("valid regex"))
            .filter_not_text_regex(RegexMatcher::new("Blocked", "").expect("valid regex"))
            .get_by_role(
                AriaRole::Button,
                RoleOptions::new().with_name_regex(
                    RegexMatcher::new("^Deploy\\s+build", "i").expect("valid regex"),
                ),
            )
            .expect("button regex should build")
            .or(page
                .get_by_label_regex(RegexMatcher::new("Release train", "i").expect("valid regex")))
            .expect("same-frame locators should compose")
            .or(page.get_by_text_regex(RegexMatcher::new("^Fallback$", "").expect("valid regex")))
            .expect("same-frame locators should compose");

        // Assert
        assert_eq!(locator.scope_id(), ObjectId::new(7));
        assert!(matches!(locator.query(), Selector::Or { .. }));
        let display = locator.query().to_string();
        assert!(display.contains("filter_text_regex"));
        assert!(display.contains("filter_not_text_regex"));
        assert!(display.contains("get_by_role"));
        assert!(display.contains("get_by_label_regex"));
        assert!(display.contains("get_by_text_regex"));
    }

    #[test]
    fn attribute_and_test_id_factories_build_lazy_selectors() {
        // Arrange
        let client = Client::new((), AtomicCallIdSource::new(1));
        let page = test_page(&client);
        let attributes =
            TestIdAttributes::new("data-pw,data-testid").expect("valid test-id attributes");

        // Act
        let placeholder = page.get_by_placeholder("Search", false);
        let alt = page
            .get_by_alt_text_regex(RegexMatcher::new("^Product \\d+$", "i").expect("valid regex"));
        let title = page.get_by_title("Release notes", true);
        let test_id = page.get_by_test_id_with_attributes(attributes.clone(), "save");
        let nested = page
            .locator_selector(Selector::css(".panel").expect("valid CSS"))
            .get_by_test_id_regex_with_attributes(
                attributes,
                RegexMatcher::new("^custom-", "").expect("valid regex"),
            );

        // Assert
        assert!(matches!(
            placeholder.query(),
            Selector::AttributeText {
                attribute: UserFacingAttribute::Placeholder,
                ..
            }
        ));
        assert!(matches!(
            alt.query(),
            Selector::AttributeRegex {
                attribute: UserFacingAttribute::Alt,
                ..
            }
        ));
        assert!(matches!(
            title.query(),
            Selector::AttributeText {
                attribute: UserFacingAttribute::Title,
                ..
            }
        ));
        assert!(matches!(test_id.query(), Selector::TestIdText { .. }));
        assert!(matches!(nested.query(), Selector::Within { .. }));
        assert_eq!(nested.scope_id(), ObjectId::new(7));
    }

    fn futures_lite_block_on<F: std::future::Future>(future: F) -> F::Output {
        use std::pin::pin;
        use std::task::{Context, Poll, Waker};

        let mut future = pin!(future);
        let mut context = Context::from_waker(Waker::noop());
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => output,
            Poll::Pending => panic!("recording channel should complete synchronously"),
        }
    }
}
