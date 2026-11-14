//! Deterministic FFI admission reproductions, not native-scheduler or browser
//! settlement tests. Holding execution owners models occupied foreign calls
//! without spawning tasks, polling native work, or allocating payload bodies.

use std::{
    future::Future,
    sync::atomic::{AtomicUsize, Ordering},
};

use pwright_node_host::{
    HostError,
    evaluation::FFI_ALLOWANCE,
    ffi_budget::{FfiBudget, FfiLane, FfiLimits, FfiMetrics},
};

const SMALL_ALLOWANCE: u64 = 32 * 1024;
const RESERVED_BYTES: u64 = 64 * 1024 * 1024;
const COMMAND_SLOTS: usize = 32;

fn python_default_profile_budget() -> FfiBudget {
    // Mirrors the Python addon's default EnvironmentOwner geometry. A larger
    // optional Watch slot limit does not change either occupied-budget case.
    FfiBudget::new(FfiLimits {
        connects: 16,
        commands: COMMAND_SLOTS,
        closes: 16,
        watches: 16,
        reserved_bytes: RESERVED_BYTES,
        future_bytes: [1024 * 1024, 128 * 1024, 16 * 1024, 16 * 1024],
    })
    .unwrap()
}

async fn run_fake_host_command(
    budget: &FfiBudget,
    allowance: u64,
    operation: impl Future<Output = Result<(), HostError>>,
) -> Result<(), HostError> {
    // Like async_python_command, admission precedes host-future polling. A
    // rejected operation is dropped without entering FfiExecution::run.
    let mut execution = budget.admit(FfiLane::Command, allowance)?;
    execution.reserve_future(std::mem::size_of_val(&operation))?;
    execution.run(operation).await.deliver(|outcome| outcome)?
}

#[tokio::test]
async fn occupied_evaluation_bytes_reject_result_and_error_before_host_polling() {
    // Arrange
    let budget = python_default_profile_budget();
    let watch = budget.admit::<()>(FfiLane::Watch, SMALL_ALLOWANCE).unwrap();
    let occupied_evaluations = (RESERVED_BYTES - SMALL_ALLOWANCE) / FFI_ALLOWANCE;
    let evaluations = (0..occupied_evaluations)
        .map(|_| budget.admit::<()>(FfiLane::Command, FFI_ALLOWANCE).unwrap())
        .collect::<Vec<_>>();
    let result_polls = AtomicUsize::new(0);
    let error_polls = AtomicUsize::new(0);
    let occupied = budget.metrics().unwrap();
    #[cfg(feature = "large-evaluation-inputs")]
    assert_eq!(FFI_ALLOWANCE, 12 * 1024 * 1024);
    #[cfg(not(feature = "large-evaluation-inputs"))]
    assert_eq!(FFI_ALLOWANCE, 8 * 1024 * 1024);
    assert_eq!(occupied.commands, occupied_evaluations as u32);
    assert_eq!(occupied.watches, 1);
    assert_eq!(occupied.pending, occupied_evaluations as u32 + 1);
    assert_eq!(occupied.running, occupied_evaluations as u32 + 1);
    assert_eq!(
        occupied.reserved_bytes,
        occupied_evaluations * FFI_ALLOWANCE + SMALL_ALLOWANCE
    );
    assert!(occupied.reserved_bytes + FFI_ALLOWANCE > RESERVED_BYTES);

    // Act: attempt result delivery, then the same-sized error fallback.
    let result = run_fake_host_command(&budget, FFI_ALLOWANCE, async {
        result_polls.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })
    .await;
    let error = run_fake_host_command(&budget, FFI_ALLOWANCE, async {
        error_polls.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })
    .await;

    // Assert: command slots remain available, but neither host future starts.
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert!(matches!(error, Err(HostError::ResourceExhausted(_))));
    assert_eq!(result_polls.load(Ordering::Relaxed), 0);
    assert_eq!(error_polls.load(Ordering::Relaxed), 0);
    assert_eq!(budget.metrics().unwrap(), occupied);
    drop(evaluations);
    drop(watch);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}

#[tokio::test]
async fn occupied_command_slots_reject_acknowledgement_before_host_polling() {
    // Arrange
    let budget = python_default_profile_budget();
    let commands = (0..COMMAND_SLOTS)
        .map(|_| {
            budget
                .admit::<()>(FfiLane::Command, SMALL_ALLOWANCE)
                .unwrap()
        })
        .collect::<Vec<_>>();
    let acknowledgement_polls = AtomicUsize::new(0);
    let occupied = budget.metrics().unwrap();
    assert_eq!(occupied.commands, 32);
    assert_eq!(occupied.pending, 32);
    assert_eq!(occupied.running, 32);
    assert_eq!(occupied.reserved_bytes, 32 * SMALL_ALLOWANCE);
    assert!(occupied.reserved_bytes + SMALL_ALLOWANCE < RESERVED_BYTES);

    // Act
    let acknowledged = run_fake_host_command(&budget, SMALL_ALLOWANCE, async {
        acknowledgement_polls.fetch_add(1, Ordering::Relaxed);
        Ok(())
    })
    .await;

    // Assert: bytes remain available, but the host acknowledgement never starts.
    assert!(matches!(
        acknowledged,
        Err(HostError::ResourceExhausted(message))
            if message == "FFI Command pending-result capacity exceeded (32)"
    ));
    assert_eq!(acknowledgement_polls.load(Ordering::Relaxed), 0);
    assert_eq!(budget.metrics().unwrap(), occupied);
    drop(commands);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}
