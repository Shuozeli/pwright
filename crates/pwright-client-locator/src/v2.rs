//! Playwright-aligned locator 2.x client actions.

use std::convert::Infallible;

use pwright_cap_locator::{PressSequentiallyParams, v2::PressSequentially};

/// Strict focus followed by literal sequential text under one call deadline.
pub trait LocatorPressSequentiallyExt {
    fn press_sequentially(
        &self,
        text: impl Into<String>,
        delay: Option<std::time::Duration>,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

impl<Channel, Ids, Query> LocatorPressSequentiallyExt for Locator<Channel, Ids, Query>
where
    Channel: Service<
            Command<PressSequentially>,
            Response = CommandResult<PressSequentially>,
            Error = Infallible,
        > + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn press_sequentially(
        &self,
        text: impl Into<String>,
        delay: Option<std::time::Duration>,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<PressSequentially>(
            PressSequentiallyParams::new(self.query().clone(), text, delay),
            metadata,
        )
    }
}

use pwright_cap_locator::{
    BlurParams, ClickOptionsV2, ClickParamsV2, FocusParams, LocatorErrorDetails, PressKey,
    PressOptions, PressParams, ScrollIntoViewIfNeededParams, SelectOptionOptions,
    SelectOptionParams, SelectOptionValues, Selector, SetCheckedOptions, SetCheckedParams,
    v2::{Blur, Click, Focus, Press, ScrollIntoViewIfNeeded, SelectOption, SetChecked},
};
use pwright_client::{CallIdSource, CommandFuture, Locator};
use pwright_core::{CallMetadata, Command, CommandResult, Service};

/// Locator 2.x Click using the extensible option bag.
///
/// The method name intentionally remains `click`: protocol-major negotiation,
/// rather than a permanent `click_v2` RPC, chooses its wire contract.
pub trait LocatorClickExt {
    fn click(
        &self,
        options: ClickOptionsV2,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Locator 2.1 strict focus using the same public semantics as locator 1.18.
pub trait LocatorFocusExt {
    fn focus(&self, metadata: CallMetadata) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Locator 2.5 strict scroll with the same public semantics as locator 1.22.
pub trait LocatorScrollIntoViewIfNeededExt {
    fn scroll_into_view_if_needed(
        &self,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Locator 2.1 strict blur using the same public semantics as locator 1.18.
pub trait LocatorBlurExt {
    fn blur(&self, metadata: CallMetadata) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Locator 2.2 strict key press using the same public semantics as locator 1.19.
pub trait LocatorPressExt {
    fn press(
        &self,
        key: PressKey,
        options: PressOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Locator 2.3 primitive checked-state action.
pub trait LocatorSetCheckedExt {
    fn set_checked(
        &self,
        checked: bool,
        options: SetCheckedOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Locator 2.4 native `<select>` option selection.
pub trait LocatorSelectOptionExt {
    fn select_option(
        &self,
        values: impl Into<SelectOptionValues>,
        options: SelectOptionOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<Vec<String>, LocatorErrorDetails>;
}

/// Locator 2.3 check composed over one SetChecked command.
pub trait LocatorCheckExt {
    fn check(
        &self,
        options: SetCheckedOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

/// Locator 2.3 uncheck composed over one SetChecked command.
pub trait LocatorUncheckExt {
    fn uncheck(
        &self,
        options: SetCheckedOptions,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails>;
}

impl<Channel, Ids, Query> LocatorClickExt for Locator<Channel, Ids, Query>
where
    Channel: Service<Command<Click>, Response = CommandResult<Click>, Error = Infallible> + 'static,
    Ids: CallIdSource,
    Query: Clone + Into<Selector>,
{
    fn click(
        &self,
        options: ClickOptionsV2,
        metadata: CallMetadata,
    ) -> CommandFuture<(), LocatorErrorDetails> {
        self.dispatch::<Click>(
            ClickParamsV2::new(self.query().clone().into(), options),
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
        self.dispatch::<Focus>(FocusParams::new(self.query().clone().into()), metadata)
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
            ScrollIntoViewIfNeededParams::new(self.query().clone().into()),
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
        self.dispatch::<Blur>(BlurParams::new(self.query().clone().into()), metadata)
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
            PressParams::new(self.query().clone().into(), key, options),
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
