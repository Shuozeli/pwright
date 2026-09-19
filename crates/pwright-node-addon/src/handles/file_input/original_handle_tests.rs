//! Actual InputRecord registry/layout admission tests. These do not construct
//! NativeHandle/Node roots and do not prove browser lifetime or Weak upgrade.

use super::*;
use pwright_node_host::ffi_budget::{FfiLimits, FfiMetrics};

fn budget() -> FfiBudget {
    FfiBudget::new(FfiLimits {
        connects: 0,
        commands: 33,
        closes: 1,
        watches: 0,
        reserved_bytes: 34 * registry::CALL_BYTES,
        future_bytes: [1024; 4],
    })
    .unwrap()
}

#[test]
fn enlarged_input_record_layout_keeps_exactly_thirty_two_initial_slots() {
    // Arrange: this constructor checks State<InputRecord> against the actual
    // fixed administrative allowance, including the new original-handle field.
    let preparations = FileInputPreparations::new().unwrap();
    let scope = preparations.scope().unwrap();
    let budget = budget();
    let tickets: [_; 32] = std::array::from_fn(|_| scope.admit(&budget).unwrap());
    let before = budget.metrics().unwrap();

    // Act
    let refused = scope.admit(&budget);

    // Assert: the registry, not this fixture's larger FFI lane, refuses entry33.
    assert!(matches!(refused, Err(HostError::ResourceExhausted(_))));
    assert_eq!(budget.metrics().unwrap(), before);
    assert_eq!(before.running, 0);
    assert_eq!(before.pending, 32);
    assert_eq!(before.reserved_bytes, 32 * registry::CALL_BYTES);
    assert_eq!(
        preparations.metrics().unwrap(),
        FileInputPreparationMetrics {
            pending: 32,
            root_containers: 0,
            rooted_values: 0,
        }
    );
    drop(tickets);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert_eq!(
        preparations.metrics().unwrap(),
        FileInputPreparationMetrics::default()
    );
}

#[test]
fn actual_input_record_registry_close_reclaims_admission_while_receipts_survive() {
    // Arrange: no NAPI roots or native source are needed for the layout and
    // admission boundary. Surviving weak receipt backing is not a heap metric.
    let preparations = FileInputPreparations::new().unwrap();
    let scope = preparations.scope().unwrap();
    let budget = budget();
    let tickets: [_; 32] = std::array::from_fn(|_| scope.admit(&budget).unwrap());

    // Act
    scope.close();

    // Assert
    assert!(tickets.iter().all(|ticket| !ticket.is_pending()));
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
    assert_eq!(
        preparations.metrics().unwrap(),
        FileInputPreparationMetrics::default()
    );
    drop(tickets);
    assert_eq!(budget.metrics().unwrap(), FfiMetrics::default());
}
