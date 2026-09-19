//! An acknowledged command is not a destruction receipt. The selecting native
//! coordinator retains started command futures through reply/transport failure.
use super::*;

pub type NativePageCloseCommandFuture =
    Pin<Box<dyn Future<Output = Result<AcknowledgedNativePageClose, Error>> + Send>>;

/// All command and receipt future storage is admitted and boxed before start.
/// Dropping this unstarted owner sends nothing and releases its cursor/slot.
pub struct PreparedNativePageCloseCommand {
    command: NativePageCloseCommandFuture,
}

impl PreparedNativePageCloseCommand {
    /// A started coordinator must retain this future until the command replies
    /// or its transport terminates, even if its Page caller/parent has finished.
    pub fn start(self) -> NativePageCloseCommandFuture {
        self.command
    }
}

/// Only native Target.closeTarget success creates this value. It proves command
/// acknowledgement, NOT target destruction. It retains the original exact-source
/// receiver, pending permit and byte lease until dropped or confirmed.
///
/// A coordinator may discard the remaining wait after independently validating
/// an exact native ancestor-destruction receipt. Merely dropping this value or
/// matching its target string is never sufficient evidence for public success.
pub struct AcknowledgedNativePageClose {
    source: NativePageCloseConfirmation,
    target: Box<str>,
    receipt: PageCloseFuture,
}

impl AcknowledgedNativePageClose {
    pub fn target_id(&self) -> &str {
        &self.target
    }

    /// Exact installed source identity, hence its bound concrete connection.
    /// Native Context ancestry and topology epoch are not represented here.
    pub fn belongs_to(&self, source: &NativePageCloseConfirmation) -> bool {
        Arc::ptr_eq(&self.source.0, &source.0)
    }

    /// Uses the prebuilt receiver future; no new allocation or command occurs.
    /// Success still requires an exact, unscoped Target.targetDestroyed event.
    pub fn wait_for_destruction(self) -> PageCloseFuture {
        self.receipt
    }
}

impl NativePageCloseConfirmation {
    /// Explicit split-phase alternative to the ordinary confirmation policy.
    /// Both future bodies, both bounded identity copies and interruption storage
    /// must fit the same declared per-call envelope before any native mutation.
    /// This does not install a native scheduler or increase pending/cursor limits.
    pub fn prepare_command(&self, target: &str) -> Result<PreparedNativePageCloseCommand, Error> {
        self.prepare_command_with_validation(target, || Ok(()))
    }

    /// Validate the selecting owner's current route immediately before native
    /// initiation, including after the discovery await. The validator's inline
    /// body is measured in the command envelope; its heap owners need separate
    /// admission. It must be synchronous and must not retain a mutation guard.
    pub fn prepare_command_with_validation<G>(
        &self,
        target: &str,
        validate: G,
    ) -> Result<PreparedNativePageCloseCommand, Error>
    where
        G: Fn() -> Result<(), Error> + Send + 'static,
    {
        let owner = self.prepare_owner(target)?;
        let interruption = owner.slot.0.interruption.interrupted()?;
        let retained = std::mem::size_of::<Prepared>()
            + 2 * target.len()
            + ERROR_BYTES
            + INTERRUPTION_FUTURE_BYTES;
        let receipt = async move {
            let mut owner = owner;
            wait_for_receipt(&owner.target, &mut owner.events, Some(interruption)).await
        };
        let receipt_bytes = std::mem::size_of_val(&receipt);
        if retained + receipt_bytes + std::mem::size_of::<AcknowledgedNativePageClose>()
            > CALL_BYTES as usize
        {
            return Err(failure(
                FailureKind::ResourceExhausted,
                "split native receipt exceeds admission",
            ));
        }
        // Prebuild before sending either discovery or close. The wrapper is
        // private to the command future until its close command actually replies.
        let acknowledged = AcknowledgedNativePageClose {
            source: self.clone(),
            target: target.into(),
            receipt: Box::pin(receipt),
        };
        let command = async move {
            let state = &acknowledged.source.0;
            if state.interruption.is_interrupted() {
                return Err(failure(
                    FailureKind::Cancelled,
                    "native close owner retired before execution",
                ));
            }
            if !state.discovery_enabled.load(Ordering::Acquire) {
                validate()?;
                state.connection.send_command("Target.setDiscoverTargets", serde_json::json!({
                    "discover": true, "filter": [{"type": "page", "exclude": false}, {"exclude": true}]
                })).await.map_err(|error| backend_error("Target.setDiscoverTargets", error))?;
                state.discovery_enabled.store(true, Ordering::Release);
            }
            if state.interruption.is_interrupted() {
                return Err(failure(
                    FailureKind::Cancelled,
                    "native close owner retired before target mutation",
                ));
            }
            // Do not race this reply against interruption/ancestor confirmation.
            // CDP already owns the written command's native timeout/termination.
            validate()?;
            CdpSession::browser(state.connection.clone())
                .target_close(&acknowledged.target)
                .await
                .map_err(|error| backend_error("Target.closeTarget", error))?;
            Ok(acknowledged)
        };
        if retained
            + receipt_bytes
            + std::mem::size_of_val(&command)
            + std::mem::size_of::<PreparedNativePageCloseCommand>()
            > CALL_BYTES as usize
        {
            return Err(failure(
                FailureKind::ResourceExhausted,
                "split native close futures exceed admission",
            ));
        }
        Ok(PreparedNativePageCloseCommand {
            command: Box::pin(command),
        })
    }
}

#[cfg(test)]
mod tests;
