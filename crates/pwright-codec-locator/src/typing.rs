use super::*;
use pwright_cap_locator::{INPUT_TEXT_MAX_BYTES, PressSequentially, PressSequentiallyParams};

const COMMAND: u8 = generated_schema::COMMAND_LOCATOR_PRESS_SEQUENTIALLY_SCHEMA.command_tag();
const RESULT: u8 = generated_schema::COMMAND_LOCATOR_PRESS_SEQUENTIALLY_SCHEMA.result_tag();

fn require(version: ProtocolVersion) -> Result<(), LocatorCodecError> {
    if version.satisfies(ProtocolVersion::new(1, 23)) {
        Ok(())
    } else {
        Err(LocatorCodecError::PressSequentiallyUnsupported { version })
    }
}

impl WireCodec<Command<PressSequentially>> for LocatorCodec {
    type Error = LocatorCodecError;
    fn encode_parts(
        &self,
        command: &Command<PressSequentially>,
    ) -> Result<EncodedParts, Self::Error> {
        require(self.capability_version)?;
        let params = command.params();
        let limit = INPUT_TEXT_MAX_BYTES.min(self.limits.string_bytes);
        if params.text.capacity() > limit {
            return Err(LocatorCodecError::StringTooLong {
                actual: params.text.capacity(),
                limit,
            });
        }
        let mut payload = BinaryWriter::new();
        payload.u8(COMMAND)?;
        payload.u64(command.call_id().get())?;
        payload.u64(command.object_id().get())?;
        encode_metadata(&mut payload, command.metadata())?;
        self.encode_selector(&mut payload, &params.selector, 0, &mut 0)?;
        self.encode_bounded_string(&mut payload, &params.text, INPUT_TEXT_MAX_BYTES)?;
        match params.delay {
            None => payload.u8(0)?,
            Some(delay) => {
                payload.u8(1)?;
                payload.u64(delay.as_secs())?;
                payload.u32(delay.subsec_nanos())?;
            }
        }
        Ok(EncodedParts::new(
            FrameRoute::Command {
                call_id: command.call_id(),
                target: command.object_id(),
                method: PressSequentially::METHOD,
            },
            payload.into_bytes(),
            Vec::new(),
        ))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<PressSequentially>, Self::Error> {
        require(self.capability_version)?;
        command_route(&frame, PressSequentially::METHOD)?;
        let mut reader = payload_reader(&frame, COMMAND)?;
        let call = CallId::new(reader.u64()?);
        let object = ObjectId::new(reader.u64()?);
        let metadata = decode_metadata(&mut reader)?;
        let selector = self.decode_selector(&mut reader, 0, &mut 0)?;
        let text = self.decode_bounded_string(&mut reader, INPUT_TEXT_MAX_BYTES)?;
        let delay = match reader.u8()? {
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
        reader.finish()?;
        Ok(Command::new(
            call,
            object,
            metadata,
            PressSequentiallyParams::new(selector, text, delay),
        ))
    }
}

impl WireCodec<CommandResult<PressSequentially>> for LocatorCodec {
    type Error = LocatorCodecError;
    fn encode_parts(
        &self,
        result: &CommandResult<PressSequentially>,
    ) -> Result<EncodedParts, Self::Error> {
        require(self.capability_version)?;
        let mut payload = BinaryWriter::new();
        payload.u8(RESULT)?;
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
    ) -> Result<CommandResult<PressSequentially>, Self::Error> {
        require(self.capability_version)?;
        result_route(&frame)?;
        let mut reader = payload_reader(&frame, RESULT)?;
        let call = CallId::new(reader.u64()?);
        let outcome = match reader.u8()? {
            0 => Ok(()),
            1 => Err(self.decode_error(&mut reader)?),
            value => return Err(LocatorCodecError::InvalidOutcome(value)),
        };
        reader.finish()?;
        Ok(CommandResult::new(call, outcome))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pwright_cap_locator::v2;
    use pwright_transport::{FrameLimits, WireProtocol};

    fn params() -> PressSequentiallyParams {
        PressSequentiallyParams::new(
            Selector::test_id("editor"),
            "a中+\n",
            Some(Duration::from_nanos(1_500_001)),
        )
    }

    #[test]
    fn both_tracks_preserve_metadata_text_delay_and_identical_payloads() {
        // Arrange
        let limits = FrameLimits::new(4096, 0, 0, 4096);
        let one = LocatorCodec::new(LocatorCodecLimits::new(2048));
        let two = crate::LocatorV2Codec::new(LocatorCodecLimits::new(2048));
        let first = Command::<PressSequentially>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::new(Some(Duration::from_secs(2)), Some(CancellationId::new(3))),
            params(),
        );
        let second = Command::<v2::PressSequentially>::new(
            first.call_id(),
            first.object_id(),
            first.metadata(),
            params(),
        );
        let wire1 =
            WireProtocol::new_exact_directional(one, ProtocolVersion::new(1, 24), limits, limits);
        let wire2 =
            WireProtocol::new_exact_directional(two, ProtocolVersion::new(2, 7), limits, limits);
        // Act
        let decoded1 = wire1
            .decode::<Command<PressSequentially>>(wire1.encode(&first).unwrap())
            .unwrap();
        let decoded2 = wire2
            .decode::<Command<v2::PressSequentially>>(wire2.encode(&second).unwrap())
            .unwrap();
        // Assert
        assert_eq!(decoded1.params(), first.params());
        assert_eq!(decoded2.params(), first.params());
        assert_eq!(decoded2.metadata(), first.metadata());
        assert_eq!(decoded1.object_id(), first.object_id());
        assert_eq!(
            one.encode_parts(&first).unwrap().payload(),
            two.encode_parts(&second).unwrap().payload()
        );
        assert_eq!(one.encode_parts(&first).unwrap().payload()[0], 45);
        let result1 = CommandResult::<PressSequentially>::new(CallId::new(1), Ok(()));
        let result2 = CommandResult::<v2::PressSequentially>::new(CallId::new(1), Ok(()));
        assert_eq!(
            one.encode_parts(&result1).unwrap().payload(),
            two.encode_parts(&result2).unwrap().payload()
        );
        assert_eq!(
            wire2
                .decode::<CommandResult<v2::PressSequentially>>(wire2.encode(&result2).unwrap())
                .unwrap()
                .outcome(),
            &Ok(())
        );
    }

    #[test]
    fn old_minors_reject_before_text_validation_and_encoding() {
        // Arrange
        let text = String::with_capacity(INPUT_TEXT_MAX_BYTES + 1);
        let first = Command::<PressSequentially>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::default(),
            PressSequentiallyParams::new(Selector::test_id("editor"), text, None),
        );
        let second = Command::<v2::PressSequentially>::new(
            first.call_id(),
            first.object_id(),
            first.metadata(),
            first.params().clone(),
        );
        let one = LocatorCodec::new(LocatorCodecLimits::new(1))
            .with_capability_version(ProtocolVersion::new(1, 22));
        let two = crate::LocatorV2Codec::new(LocatorCodecLimits::new(1))
            .with_capability_version(ProtocolVersion::new(2, 5));
        // Act
        let rejected1 = one.encode_parts(&first);
        let rejected2 = two.encode_parts(&second);
        // Assert
        assert!(matches!(
            rejected1,
            Err(LocatorCodecError::PressSequentiallyUnsupported { .. })
        ));
        assert!(matches!(
            rejected2,
            Err(LocatorCodecError::PressSequentiallyUnsupported { .. })
        ));
    }

    #[test]
    fn v2_rejects_excess_text_capacity_before_compatibility_clone() {
        // Arrange
        let codec = crate::LocatorV2Codec::new(LocatorCodecLimits::new(INPUT_TEXT_MAX_BYTES));
        let command = Command::<v2::PressSequentially>::new(
            CallId::new(1),
            ObjectId::new(2),
            CallMetadata::default(),
            PressSequentiallyParams::new(
                Selector::test_id("editor"),
                String::with_capacity(INPUT_TEXT_MAX_BYTES + 1),
                None,
            ),
        );
        // Act
        let result = codec.encode_parts(&command);
        // Assert
        assert!(matches!(
            result,
            Err(LocatorCodecError::StringTooLong { .. })
        ));
    }
}
