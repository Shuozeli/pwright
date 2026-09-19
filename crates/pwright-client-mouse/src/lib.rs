//! Independent per-command Page Mouse extensions; no main Frame is required.
use pwright_cap_mouse::*;
use pwright_client::{CallIdSource, CommandFuture, Page};
use pwright_core::{CallMetadata, Command, CommandResult, Service};
use std::convert::Infallible;
macro_rules! mouse_extension {
    ($extension:ident, $method:ident, $command:ty, $params:ty) => {
        pub trait $extension {
            fn $method(&self, params: $params, metadata: CallMetadata) -> CommandFuture<(), MouseErrorDetails>;
        }
        impl<Channel, Ids> $extension for Page<Channel, Ids>
        where Channel: Service<Command<$command>, Response = CommandResult<$command>, Error = Infallible> + 'static,
            Ids: CallIdSource,
        {
            fn $method(&self, params: $params, metadata: CallMetadata) -> CommandFuture<(), MouseErrorDetails> {
                self.as_remote().dispatch::<$command>(params, metadata)
            }
        }
    };
}
mouse_extension!(MouseMoveExt, mouse_move, MouseMove, MoveParams);
mouse_extension!(MouseDownExt, mouse_down, MouseDown, DownParams);
mouse_extension!(MouseUpExt, mouse_up, MouseUp, UpParams);
mouse_extension!(MouseClickExt, mouse_click, MouseClick, ClickParams);
mouse_extension!(
    MouseDblclickExt,
    mouse_dblclick,
    MouseDblclick,
    DblclickParams
);
mouse_extension!(MouseWheelExt, mouse_wheel, MouseWheel, WheelParams);
#[cfg(test)]
mod tests;
