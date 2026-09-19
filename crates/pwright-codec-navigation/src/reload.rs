use super::*;
use generated_schema::PwrightNavigationReloadParams;
use pwright_cap_navigation::Reload;

#[cfg(test)]
mod tests;

impl WireCodec<Command<Reload>> for NavigationCodec {
    type Error = NavigationCodecError;

    fn encode_parts(&self, command: &Command<Reload>) -> Result<EncodedParts, Self::Error> {
        let wire = PwrightNavigationRef::ReloadCommand {
            call_id: command.call_id().get(),
            object_id: command.object_id().get(),
            metadata: metadata_to_wire(command.metadata()),
            params: PwrightNavigationReloadParams {
                wait_until: command.params().wait_until.into(),
            },
        };
        let payload = encode_value(&wire, self.value_limits()).map_err(map_generated_encode)?;
        Ok(command_parts(command, payload))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<Command<Reload>, Self::Error> {
        command_route::<Reload>(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightNavigation>(frame.payload(), self.value_limits())
            .map_err(|error| map_generated_decode(error, RELOAD_COMMAND))?
        {
            PwrightNavigation::ReloadCommand {
                call_id,
                object_id,
                metadata,
                params,
            } => Ok(Command::new(
                CallId::new(call_id),
                ObjectId::new(object_id),
                metadata_from_wire(metadata),
                params.into(),
            )),
            other => Err(NavigationCodecError::WrongPayloadKind {
                expected: RELOAD_COMMAND,
                actual: navigation_wire_kind(&other),
            }),
        }
    }
}

impl WireCodec<CommandResult<Reload>> for NavigationCodec {
    type Error = NavigationCodecError;

    fn encode_parts(&self, result: &CommandResult<Reload>) -> Result<EncodedParts, Self::Error> {
        let canonical_headers = result
            .outcome()
            .as_ref()
            .ok()
            .and_then(|value| value.response.as_ref())
            .map(|response| CanonicalHeaders::new(&response.headers));
        let outcome = match result.outcome() {
            Ok(value) => PwrightNavigationGotoOutcomeRef::Success {
                result: self.goto_result_to_wire_ref(value, canonical_headers.as_ref())?,
            },
            Err(error) => PwrightNavigationGotoOutcomeRef::Error {
                error: operation_error_to_wire_ref(error),
            },
        };
        let wire = PwrightNavigationRef::ReloadResult {
            call_id: result.call_id().get(),
            outcome,
        };
        let payload = encode_value(&wire, self.value_limits()).map_err(map_generated_encode)?;
        Ok(result_parts(result.call_id(), payload))
    }

    fn decode_frame(&self, frame: WireFrame) -> Result<CommandResult<Reload>, Self::Error> {
        result_route(&frame)?;
        ensure_no_attachments(&frame)?;
        match decode_value::<PwrightNavigation>(frame.payload(), self.value_limits())
            .map_err(|error| map_generated_decode(error, RELOAD_RESULT))?
        {
            PwrightNavigation::ReloadResult { call_id, outcome } => {
                let outcome = match outcome {
                    PwrightNavigationGotoOutcome::Success { result } => {
                        Ok(self.goto_result_from_wire(result)?)
                    }
                    PwrightNavigationGotoOutcome::Error { error } => {
                        Err(operation_error_from_wire(error))
                    }
                };
                Ok(CommandResult::new(CallId::new(call_id), outcome))
            }
            other => Err(NavigationCodecError::WrongPayloadKind {
                expected: RELOAD_RESULT,
                actual: navigation_wire_kind(&other),
            }),
        }
    }
}
