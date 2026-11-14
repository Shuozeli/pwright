//! Bounded same-connection recovery for the host's caller-driven topology cursor.

use std::future::Future;

use pwright_backend_cdp_topology::ConnectionEpoch;
use pwright_composition_chromium::{
    ChromiumTopologyEventDriver, ChromiumTopologyEventDriverError,
    ChromiumTopologyEventDriverOutcome,
};

use crate::{CommandRuntime, HostError};

pub(super) async fn initialize_snapshot(
    driver: &mut ChromiumTopologyEventDriver,
    runtime: &CommandRuntime,
) -> Result<ChromiumTopologyEventDriverOutcome, HostError> {
    let snapshot_event_limit = driver.profile().event_channel_capacity();
    let result = driver
        .initialize_snapshot(runtime, snapshot_event_limit)
        .await;
    finish_or_resynchronize(driver, runtime, result).await
}

pub(super) async fn receive_once_or_signal(
    driver: &mut ChromiumTopologyEventDriver,
    runtime: &CommandRuntime,
    signal: impl Future<Output = ()>,
) -> Result<ChromiumTopologyEventDriverOutcome, HostError> {
    let result = driver.receive_once_or_signal(runtime, signal).await;
    finish_or_resynchronize(driver, runtime, result).await
}

async fn finish_or_resynchronize(
    driver: &mut ChromiumTopologyEventDriver,
    runtime: &CommandRuntime,
    result: Result<ChromiumTopologyEventDriverOutcome, ChromiumTopologyEventDriverError>,
) -> Result<ChromiumTopologyEventDriverOutcome, HostError> {
    match result {
        Ok(outcome) => Ok(outcome),
        Err(initial) => {
            let Some(skipped) = driver.resync_required() else {
                return Err(HostError::backend(initial));
            };
            let next_epoch = next_recovery_epoch(driver.epoch(), skipped)?;
            driver
                .resynchronize_after_event_loss(runtime, next_epoch)
                .await
                .map_err(|recovery| {
                    HostError::backend(format_args!(
                        "topology reconstruction after `{initial}` failed: {recovery}"
                    ))
                })
        }
    }
}

fn next_recovery_epoch(
    current: ConnectionEpoch,
    skipped: u64,
) -> Result<ConnectionEpoch, HostError> {
    current.checked_next().ok_or_else(|| {
        HostError::backend(format_args!(
            "cannot reconstruct topology after {skipped} skipped CDP events because connection epoch {} is exhausted",
            current.get()
        ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_epoch_advances_exactly_once() {
        // Arrange
        let current = ConnectionEpoch::new(41);

        // Act
        let next = next_recovery_epoch(current, 7).expect("epoch should advance");

        // Assert
        assert_eq!(next, ConnectionEpoch::new(42));
    }

    #[test]
    fn recovery_epoch_exhaustion_is_explicit() {
        // Arrange
        let current = ConnectionEpoch::new(u64::MAX);

        // Act
        let error = next_recovery_epoch(current, 7).expect_err("epoch must be bounded");

        // Assert
        assert_eq!(error.code(), "PWRIGHT_BACKEND");
        assert_eq!(
            error.to_string(),
            format!(
                "cannot reconstruct topology after 7 skipped CDP events because connection epoch {} is exhausted",
                u64::MAX
            )
        );
    }
}
