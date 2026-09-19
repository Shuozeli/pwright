use super::*;
use pwright_core::{CommandSpec, RegistryBuilder};
#[test]
fn six_independent_commands_target_page_only() {
    // Arrange
    let mut builder = RegistryBuilder::new();
    builder.register::<MouseCapability>().unwrap();
    // Act
    let registry = builder.build().unwrap();
    // Assert
    assert!(registry.contains(MOUSE_CAPABILITY_ID));
    assert_eq!(MOUSE_COMMAND_DESCRIPTORS.len(), 6);
    for targets in [
        MOUSE_MOVE_TARGET_OBJECT_KINDS,
        MOUSE_DOWN_TARGET_OBJECT_KINDS,
        MOUSE_UP_TARGET_OBJECT_KINDS,
        MOUSE_CLICK_TARGET_OBJECT_KINDS,
        MOUSE_DBLCLICK_TARGET_OBJECT_KINDS,
        MOUSE_WHEEL_TARGET_OBJECT_KINDS,
    ] {
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].as_str(), "pwright.browser.page");
    }
    assert_eq!(
        [
            MouseMove::METHOD.get(),
            MouseDown::METHOD.get(),
            MouseUp::METHOD.get(),
            MouseClick::METHOD.get(),
            MouseDblclick::METHOD.get(),
            MouseWheel::METHOD.get()
        ],
        [917505, 917506, 917507, 917508, 917509, 917510]
    );
}
#[test]
fn finite_numbers_preserve_fractional_negative_and_signed_zero_bits() {
    // Arrange
    let values = [-0.0, 0.0, -12.5, 14.25, f64::MAX];
    // Act
    let actual = values.map(|v| MouseNumber::try_from(v).unwrap().get().to_bits());
    // Assert
    assert_eq!(actual, values.map(f64::to_bits));
}
#[test]
fn nonfinite_numbers_reject_without_creating_parameters() {
    // Arrange
    let values = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];
    // Act
    let results = values.map(MouseNumber::try_from);
    // Assert
    assert_eq!(results, [Err(MouseArgumentError::NonFinite); 3]);
}
