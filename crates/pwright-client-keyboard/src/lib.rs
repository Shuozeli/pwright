//! Independently bounded raw Keyboard extensions for a Page handle.
use pwright_cap_keyboard::*;
use pwright_client::{CallIdSource, CommandFuture, Page};
use pwright_core::{CallMetadata, Command, CommandResult, Service};
use std::convert::Infallible;

macro_rules! keyboard_extension {
    ($extension:ident, $method:ident, $command:ty, $params:ty) => {
        pub trait $extension {
            fn $method(&self, params: $params, metadata: CallMetadata) -> CommandFuture<(), KeyboardErrorDetails>;
        }
        impl<Channel, Ids> $extension for Page<Channel, Ids>
        where Channel: Service<Command<$command>, Response = CommandResult<$command>, Error = Infallible> + 'static,
            Ids: CallIdSource,
        {
            fn $method(&self, params: $params, metadata: CallMetadata) -> CommandFuture<(), KeyboardErrorDetails> {
                self.as_remote().dispatch::<$command>(params, metadata)
            }
        }
    };
}

keyboard_extension!(KeyboardDownExt, keyboard_down, KeyboardDown, DownParams);
keyboard_extension!(KeyboardUpExt, keyboard_up, KeyboardUp, UpParams);
keyboard_extension!(KeyboardPressExt, keyboard_press, KeyboardPress, PressParams);
keyboard_extension!(
    KeyboardInsertTextExt,
    keyboard_insert_text,
    KeyboardInsertText,
    InsertTextParams
);
keyboard_extension!(KeyboardTypeExt, keyboard_type, KeyboardType, TypeParams);

#[cfg(test)]
mod tests;
