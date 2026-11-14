#![cfg(feature = "cdp-session")]

#[path = "../src/cdp_sessions/lease.rs"]
mod lease;
#[path = "../src/cdp_sessions/validation.rs"]
mod validation;

use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use lease::ClaimLease;
use pwright_node_host::HostError;
use serde_json::json;

#[test]
fn dropping_an_unclaimed_attach_result_cleans_up_once() {
    // Arrange
    let cleanups = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&cleanups);
    let lease = ClaimLease::new(42, move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });

    // Act
    drop(lease);

    // Assert
    assert_eq!(cleanups.load(Ordering::SeqCst), 1);
}

#[test]
fn aborting_an_unclaimed_attach_result_cleans_up_before_drop() {
    // Arrange
    let cleanups = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&cleanups);
    let mut lease = ClaimLease::new(42, move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });

    // Act
    lease.abort();

    // Assert
    assert_eq!(cleanups.load(Ordering::SeqCst), 1);
    lease.abort();
    drop(lease);
    assert_eq!(cleanups.load(Ordering::SeqCst), 1);
}

#[test]
fn claiming_an_attach_result_transfers_cleanup_to_the_facade() {
    // Arrange
    let cleanups = Arc::new(AtomicUsize::new(0));
    let observed = Arc::clone(&cleanups);
    let mut lease = ClaimLease::new(42, move || {
        observed.fetch_add(1, Ordering::SeqCst);
    });
    let advertised = lease.token();

    // Act
    let token = lease.claim().expect("first claim");
    lease.abort();
    drop(lease);

    // Assert
    assert_eq!(advertised, 42);
    assert_eq!(token, 42);
    assert_eq!(cleanups.load(Ordering::SeqCst), 0);
}

#[test]
fn attach_result_cannot_be_claimed_twice() {
    // Arrange
    let mut lease = ClaimLease::new(42, || {});
    lease.claim().expect("first claim");

    // Act
    let error = lease.claim().expect_err("second claim must fail");

    // Assert
    assert!(matches!(error, HostError::InvalidArgument(_)));
}

#[test]
fn cdp_params_require_a_bounded_json_object() {
    // Arrange
    let valid = r#"{"expression":"1 + 1"}"#;
    let scalar = "null";
    let malformed = "{";
    let oversized = format!(
        "{{\"value\":\"{}\"}}",
        "x".repeat(validation::MAX_CDP_PARAMS_JSON_BYTES)
    );

    // Act
    let accepted = validation::parse_params_json(valid);
    let scalar_error = validation::parse_params_json(scalar);
    let malformed_error = validation::parse_params_json(malformed);
    let oversized_error = validation::parse_params_json(&oversized);

    // Assert
    assert_eq!(accepted.expect("object"), json!({"expression": "1 + 1"}));
    assert!(matches!(scalar_error, Err(HostError::InvalidArgument(_))));
    assert!(matches!(
        malformed_error,
        Err(HostError::InvalidArgument(_))
    ));
    assert!(matches!(
        oversized_error,
        Err(HostError::InvalidArgument(_))
    ));
}

#[test]
fn cdp_result_has_a_measured_object_conversion_cap() {
    // Arrange
    let result = json!({"result": 7});
    let scalar = json!(7);
    let oversized = json!({"value": "x".repeat(validation::MAX_CDP_RESULT_JSON_BYTES)});

    // Act
    let accepted = validation::measured_result(result);
    let scalar_error = validation::measured_result(scalar);
    let oversized_error = validation::measured_result(oversized);

    // Assert
    assert_eq!(accepted.expect("small object").1, 12);
    assert!(matches!(scalar_error, Err(HostError::Backend(_))));
    assert!(matches!(
        oversized_error,
        Err(HostError::ResourceExhausted(_))
    ));
}
