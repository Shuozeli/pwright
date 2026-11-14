#[path = "../src/locator/click_steps.rs"]
mod click_steps;

use click_steps::{CLICK_STEPS_ERROR, bounded_click_steps};

#[test]
fn accepts_only_bounded_positive_integer_click_steps() {
    // Arrange
    let accepted = [1.0, 5.0, 10_000.0];

    // Act and assert
    for value in accepted {
        assert_eq!(
            bounded_click_steps(value).expect("valid click steps").get(),
            value as u32
        );
    }
}

#[test]
fn rejects_fractional_nonfinite_and_out_of_range_click_steps() {
    // Arrange
    let rejected = [
        0.0,
        -0.0,
        -1.0,
        0.5,
        1.5,
        10_001.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];

    // Act and assert
    for value in rejected {
        assert_eq!(bounded_click_steps(value), Err(CLICK_STEPS_ERROR));
    }
}
