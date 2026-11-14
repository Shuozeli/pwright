use pwright_cap_locator::v2 as locator_v2;

use super::*;
use crate::generated_schema_v2::{
    PwrightCommonDuration, PwrightLocatorClickOptions, PwrightLocatorKeyboardModifier,
    PwrightLocatorKeyboardModifiers, PwrightLocatorMouseButton, PwrightLocatorPoint,
    PwrightLocatorScrollMode,
};

/// Locator protocol 2.x codec.
///
/// Commands whose layout did not change delegate to the proven 1.x mappings.
/// Click uses the generated bounded extensible option bag. A connection must
/// select this codec only after negotiating `pwright.locator` major 2.
#[derive(Clone, Copy, Debug)]
pub struct LocatorV2Codec {
    inner: LocatorCodec,
    capability_version: ProtocolVersion,
}

impl LocatorV2Codec {
    pub const fn new(limits: LocatorCodecLimits) -> Self {
        Self {
            inner: LocatorCodec::new(limits).with_selector_version(ProtocolVersion::new(2, 7)),
            capability_version: ProtocolVersion::new(2, 7),
        }
    }

    pub const fn limits(self) -> LocatorCodecLimits {
        self.inner.limits()
    }

    pub const fn with_capability_version(mut self, capability_version: ProtocolVersion) -> Self {
        self.capability_version = capability_version;
        self.inner = self.inner.with_selector_version(capability_version);
        self
    }

    pub const fn capability_version(self) -> ProtocolVersion {
        self.capability_version
    }

    fn value_limits(self) -> BinaryValueLimits {
        BinaryValueLimits::new(
            self.limits().string_bytes,
            0,
            5,
            self.limits().selector_depth,
            self.limits().selector_nodes,
        )
    }
}

impl WireCodec<Command<locator_v2::Click>> for LocatorV2Codec {
    type Error = LocatorCodecError;

    fn encode_parts(
        &self,
        command: &Command<locator_v2::Click>,
    ) -> Result<EncodedParts, Self::Error> {
        let mut payload = BinaryWriter::new();
        payload.u8(CLICK_COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        let mut selector_nodes = 0;
        self.inner.encode_selector(
            &mut payload,
            &command.params().selector,
            0,
            &mut selector_nodes,
        )?;
        let options = options_to_wire(&command.params().options);
        let mut context = BinaryValueContext::new(self.value_limits());
        options.encode_binary(&mut payload, &mut context)?;
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: locator_v2::Click::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<locator_v2::Click>, Self::Error> {
        command_route(&frame, locator_v2::Click::METHOD)?;
        let mut reader = payload_reader(&frame, CLICK_COMMAND)?;
        let call_id = CallId::new(reader.u64()?);
        let object_id = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let mut selector_nodes = 0;
        let selector = self
            .inner
            .decode_selector(&mut reader, 0, &mut selector_nodes)?;
        let mut context = BinaryValueContext::new(self.value_limits());
        let options = PwrightLocatorClickOptions::decode_binary(&mut reader, &mut context)?;
        let options = options_from_wire(options)?;
        reader.finish()?;
        Ok(Command::new(
            call_id,
            object_id,
            metadata,
            ClickParamsV2::new(selector, options),
        ))
    }
}

macro_rules! delegate_v2_command_codec {
    ($v2:ty, $v1:ty) => {
        impl WireCodec<Command<$v2>> for LocatorV2Codec {
            type Error = LocatorCodecError;

            fn encode_parts(&self, command: &Command<$v2>) -> Result<EncodedParts, Self::Error> {
                let command = Command::<$v1>::new(
                    command.call_id(),
                    command.object_id(),
                    command.metadata(),
                    command.params().clone(),
                );
                <LocatorCodec as WireCodec<Command<$v1>>>::encode_parts(&self.inner, &command)
            }

            fn decode_frame(&self, frame: WireFrame) -> Result<Command<$v2>, Self::Error> {
                let command =
                    <LocatorCodec as WireCodec<Command<$v1>>>::decode_frame(&self.inner, frame)?;
                let (call_id, object_id, metadata, params) = command.into_parts();
                Ok(Command::new(call_id, object_id, metadata, params))
            }
        }
    };
}

macro_rules! delegate_v2_result_codec {
    ($v2:ty, $v1:ty) => {
        impl WireCodec<CommandResult<$v2>> for LocatorV2Codec {
            type Error = LocatorCodecError;

            fn encode_parts(
                &self,
                result: &CommandResult<$v2>,
            ) -> Result<EncodedParts, Self::Error> {
                let result = CommandResult::<$v1>::new(result.call_id(), result.outcome().clone());
                <LocatorCodec as WireCodec<CommandResult<$v1>>>::encode_parts(&self.inner, &result)
            }

            fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<$v2>, Self::Error> {
                let result = <LocatorCodec as WireCodec<CommandResult<$v1>>>::decode_frame(
                    &self.inner,
                    frame,
                )?;
                Ok(CommandResult::new(result.call_id(), result.into_outcome()))
            }
        }
    };
}

delegate_v2_result_codec!(locator_v2::Click, Click);

delegate_v2_command_codec!(locator_v2::AllInnerTexts, AllInnerTexts);
delegate_v2_result_codec!(locator_v2::AllInnerTexts, AllInnerTexts);
delegate_v2_command_codec!(locator_v2::AllTextContents, AllTextContents);
delegate_v2_result_codec!(locator_v2::AllTextContents, AllTextContents);
delegate_v2_command_codec!(locator_v2::Count, Count);
delegate_v2_result_codec!(locator_v2::Count, Count);
delegate_v2_command_codec!(locator_v2::Fill, Fill);
delegate_v2_result_codec!(locator_v2::Fill, Fill);
delegate_v2_command_codec!(locator_v2::GetAttribute, GetAttribute);
delegate_v2_result_codec!(locator_v2::GetAttribute, GetAttribute);
delegate_v2_command_codec!(locator_v2::Hover, Hover);
delegate_v2_result_codec!(locator_v2::Hover, Hover);
delegate_v2_command_codec!(locator_v2::InnerHtml, InnerHtml);
delegate_v2_result_codec!(locator_v2::InnerHtml, InnerHtml);
delegate_v2_command_codec!(locator_v2::InnerText, InnerText);
delegate_v2_result_codec!(locator_v2::InnerText, InnerText);
delegate_v2_command_codec!(locator_v2::InputValue, InputValue);
delegate_v2_result_codec!(locator_v2::InputValue, InputValue);
delegate_v2_command_codec!(locator_v2::IsChecked, IsChecked);
delegate_v2_result_codec!(locator_v2::IsChecked, IsChecked);
delegate_v2_command_codec!(locator_v2::IsEditable, IsEditable);
delegate_v2_result_codec!(locator_v2::IsEditable, IsEditable);
delegate_v2_command_codec!(locator_v2::IsEnabled, IsEnabled);
delegate_v2_result_codec!(locator_v2::IsEnabled, IsEnabled);
delegate_v2_command_codec!(locator_v2::IsVisible, IsVisible);
delegate_v2_result_codec!(locator_v2::IsVisible, IsVisible);
delegate_v2_command_codec!(locator_v2::TextContent, TextContent);
delegate_v2_result_codec!(locator_v2::TextContent, TextContent);
delegate_v2_command_codec!(locator_v2::WaitFor, WaitFor);
delegate_v2_result_codec!(locator_v2::WaitFor, WaitFor);

macro_rules! delegate_v2_versioned_codec {
    ($v2:ty, $v1:ty, $require:ident $(, $validate:ident)?) => {
        impl WireCodec<Command<$v2>> for LocatorV2Codec {
            type Error = LocatorCodecError;

            fn encode_parts(&self, command: &Command<$v2>) -> Result<EncodedParts, Self::Error> {
                $require(self.capability_version)?;
                $($validate(self, command.params())?;)?
                let command = Command::<$v1>::new(
                    command.call_id(),
                    command.object_id(),
                    command.metadata(),
                    command.params().clone(),
                );
                <LocatorCodec as WireCodec<Command<$v1>>>::encode_parts(&self.inner, &command)
            }

            fn decode_frame(&self, frame: WireFrame) -> Result<Command<$v2>, Self::Error> {
                $require(self.capability_version)?;
                let command =
                    <LocatorCodec as WireCodec<Command<$v1>>>::decode_frame(&self.inner, frame)?;
                let (call_id, object_id, metadata, params) = command.into_parts();
                Ok(Command::new(call_id, object_id, metadata, params))
            }
        }

        impl WireCodec<CommandResult<$v2>> for LocatorV2Codec {
            type Error = LocatorCodecError;

            fn encode_parts(
                &self,
                result: &CommandResult<$v2>,
            ) -> Result<EncodedParts, Self::Error> {
                $require(self.capability_version)?;
                let result = CommandResult::<$v1>::new(result.call_id(), result.outcome().clone());
                <LocatorCodec as WireCodec<CommandResult<$v1>>>::encode_parts(&self.inner, &result)
            }

            fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<$v2>, Self::Error> {
                $require(self.capability_version)?;
                let result = <LocatorCodec as WireCodec<CommandResult<$v1>>>::decode_frame(
                    &self.inner,
                    frame,
                )?;
                Ok(CommandResult::new(result.call_id(), result.into_outcome()))
            }
        }
    };
}

fn require_v2_focus_management(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(2, 1)) {
        Ok(())
    } else {
        Err(LocatorCodecError::FocusManagementUnsupported { version })
    }
}

fn require_v2_press(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(2, 2)) {
        Ok(())
    } else {
        Err(LocatorCodecError::PressUnsupported { version })
    }
}

fn require_v2_press_sequentially(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(2, 6)) {
        Ok(())
    } else {
        Err(LocatorCodecError::PressSequentiallyUnsupported { version })
    }
}

fn validate_sequential_text_capacity(
    codec: &LocatorV2Codec,
    params: &pwright_cap_locator::PressSequentiallyParams,
) -> Result<(), LocatorCodecError> {
    let limit = pwright_cap_locator::INPUT_TEXT_MAX_BYTES.min(codec.limits().string_bytes);
    if params.text.capacity() > limit {
        return Err(LocatorCodecError::StringTooLong {
            actual: params.text.capacity(),
            limit,
        });
    }
    Ok(())
}

delegate_v2_versioned_codec!(
    locator_v2::PressSequentially,
    pwright_cap_locator::PressSequentially,
    require_v2_press_sequentially,
    validate_sequential_text_capacity
);

fn require_v2_set_checked(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(2, 3)) {
        Ok(())
    } else {
        Err(LocatorCodecError::SetCheckedUnsupported { version })
    }
}

fn require_v2_select_option(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(2, 4)) {
        Ok(())
    } else {
        Err(LocatorCodecError::SelectOptionUnsupported { version })
    }
}

fn require_v2_scroll_into_view_if_needed(
    version: ProtocolVersion,
) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(2, 5)) {
        Ok(())
    } else {
        Err(LocatorCodecError::ScrollIntoViewIfNeededUnsupported { version })
    }
}

delegate_v2_versioned_codec!(locator_v2::Blur, Blur, require_v2_focus_management);
delegate_v2_versioned_codec!(locator_v2::Focus, Focus, require_v2_focus_management);
delegate_v2_versioned_codec!(locator_v2::Press, Press, require_v2_press);
delegate_v2_versioned_codec!(
    locator_v2::ScrollIntoViewIfNeeded,
    ScrollIntoViewIfNeeded,
    require_v2_scroll_into_view_if_needed
);
delegate_v2_versioned_codec!(
    locator_v2::SelectOption,
    SelectOption,
    require_v2_select_option
);
delegate_v2_versioned_codec!(locator_v2::SetChecked, SetChecked, require_v2_set_checked);

fn options_to_wire(options: &ClickOptionsV2) -> PwrightLocatorClickOptions {
    PwrightLocatorClickOptions {
        button: options.button.map(|button| match button {
            MouseButton::Left => PwrightLocatorMouseButton::Left,
            MouseButton::Right => PwrightLocatorMouseButton::Right,
            MouseButton::Middle => PwrightLocatorMouseButton::Middle,
        }),
        click_count: options.click_count.map(NonZeroU8::get),
        delay: options.delay.map(|delay| PwrightCommonDuration {
            seconds: delay.as_secs(),
            nanoseconds: delay.subsec_nanos(),
        }),
        force: options.force,
        modifiers: options
            .modifiers
            .map(|modifiers| PwrightLocatorKeyboardModifiers {
                values: modifiers
                    .iter()
                    .map(|modifier| match modifier {
                        KeyboardModifier::Alt => PwrightLocatorKeyboardModifier::Alt,
                        KeyboardModifier::Control => PwrightLocatorKeyboardModifier::Control,
                        KeyboardModifier::ControlOrMeta => {
                            PwrightLocatorKeyboardModifier::ControlOrMeta
                        }
                        KeyboardModifier::Meta => PwrightLocatorKeyboardModifier::Meta,
                        KeyboardModifier::Shift => PwrightLocatorKeyboardModifier::Shift,
                    })
                    .collect(),
            }),
        no_wait_after: options.no_wait_after,
        position: options.position.map(|position| PwrightLocatorPoint {
            x: position.x(),
            y: position.y(),
        }),
        scroll: options.scroll.map(|scroll| match scroll {
            ScrollMode::Auto => PwrightLocatorScrollMode::Auto,
            ScrollMode::None => PwrightLocatorScrollMode::None,
        }),
        steps: options.steps.map(NonZeroU32::get),
        trial: options.trial,
    }
}

fn options_from_wire(
    options: PwrightLocatorClickOptions,
) -> Result<ClickOptionsV2, LocatorCodecError> {
    let button = options.button.map(|button| match button {
        PwrightLocatorMouseButton::Left => MouseButton::Left,
        PwrightLocatorMouseButton::Right => MouseButton::Right,
        PwrightLocatorMouseButton::Middle => MouseButton::Middle,
    });
    let click_count = options
        .click_count
        .map(|value| NonZeroU8::new(value).ok_or(LocatorCodecError::ZeroClickCount))
        .transpose()?;
    let delay = options
        .delay
        .map(|delay| {
            (delay.nanoseconds < 1_000_000_000)
                .then(|| Duration::new(delay.seconds, delay.nanoseconds))
                .ok_or(LocatorCodecError::InvalidDurationNanos(delay.nanoseconds))
        })
        .transpose()?;
    let modifiers = options
        .modifiers
        .map(decode_generated_modifiers)
        .transpose()?;
    let position = options
        .position
        .map(|position| {
            Position::new(position.x, position.y)
                .map_err(|_| LocatorCodecError::InvalidPointerPosition)
        })
        .transpose()?;
    let scroll = options.scroll.map(|scroll| match scroll {
        PwrightLocatorScrollMode::Auto => ScrollMode::Auto,
        PwrightLocatorScrollMode::None => ScrollMode::None,
    });
    let steps = options
        .steps
        .map(|value| NonZeroU32::new(value).ok_or(LocatorCodecError::ZeroSteps))
        .transpose()?;
    Ok(ClickOptionsV2 {
        button,
        click_count,
        delay,
        force: options.force,
        modifiers,
        no_wait_after: options.no_wait_after,
        position,
        scroll,
        steps,
        trial: options.trial,
    })
}

fn decode_generated_modifiers(
    modifiers: PwrightLocatorKeyboardModifiers,
) -> Result<KeyboardModifiers, LocatorCodecError> {
    let mut decoded = KeyboardModifiers::new();
    let mut previous = None;
    for modifier in modifiers.values {
        let (tag, modifier) = match modifier {
            PwrightLocatorKeyboardModifier::Alt => (0, KeyboardModifier::Alt),
            PwrightLocatorKeyboardModifier::Control => (1, KeyboardModifier::Control),
            PwrightLocatorKeyboardModifier::ControlOrMeta => (2, KeyboardModifier::ControlOrMeta),
            PwrightLocatorKeyboardModifier::Meta => (3, KeyboardModifier::Meta),
            PwrightLocatorKeyboardModifier::Shift => (4, KeyboardModifier::Shift),
        };
        if previous.is_some_and(|previous| previous >= tag) {
            return Err(LocatorCodecError::NonCanonicalKeyboardModifiers);
        }
        previous = Some(tag);
        decoded = decoded.with(modifier);
    }
    Ok(decoded)
}
