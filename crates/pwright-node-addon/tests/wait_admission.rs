#[path = "../src/wait_admission.rs"]
mod wait_admission;

use pwright_node_host::HostError;
use wait_admission::{COMMAND_CAPACITY, MAX_LONG_WAITS, RESERVED_COMMANDS, WaitAdmissions};

#[test]
fn long_waits_leave_eight_command_slots_for_unblockers() {
    // Arrange
    let waits = WaitAdmissions::new();

    // Act
    let held: Vec<_> = (0..MAX_LONG_WAITS)
        .map(|_| waits.admit().expect("long wait slot"))
        .collect();

    // Assert
    assert_eq!(held.len(), 24);
    assert_eq!(COMMAND_CAPACITY - held.len(), RESERVED_COMMANDS);
    assert_eq!(RESERVED_COMMANDS, 8);
}

#[test]
fn an_excess_long_wait_is_rejected_immediately_across_session_clones() {
    // Arrange
    let first_session = WaitAdmissions::new();
    let second_session = first_session.clone();
    let _held: Vec<_> = (0..MAX_LONG_WAITS)
        .map(|_| first_session.admit().expect("long wait slot"))
        .collect();

    // Act
    let rejected = match second_session.admit() {
        Ok(_) => panic!("excess long wait must be rejected"),
        Err(error) => error,
    };

    // Assert
    assert!(matches!(rejected, HostError::ResourceExhausted(_)));
    assert_eq!(
        rejected.to_string(),
        "FFI long wait capacity exceeded (24); 8 Command slots reserved for unblockers"
    );
}

#[test]
fn dropping_a_completed_or_cancelled_wait_returns_its_slot() {
    // Arrange
    let waits = WaitAdmissions::new();
    let mut held: Vec<_> = (0..MAX_LONG_WAITS)
        .map(|_| waits.admit().expect("long wait slot"))
        .collect();
    drop(held.pop());

    // Act
    let reclaimed = waits.admit();

    // Assert
    assert!(reclaimed.is_ok());
}
