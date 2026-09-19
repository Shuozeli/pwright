use super::*;
use crate::CdpInputProductionProfile;
use pwright_backend_cdp_input::{CdpInputPolicy, CdpModifiers};

fn profile() -> ChromiumAttachProductionProfile {
    base_profile()
        .with_navigation(navigation_profile())
        .with_page_input(CdpInputProductionProfile::new(
            ResourceScope::extension(13, 4),
            CdpInputPolicy::Interleaved,
            8,
        ))
}
fn fixture(
    profile: &ChromiumAttachProductionProfile,
) -> (
    ProductionCdpTopology,
    CdpPageServices,
    ProductionResourceLedger,
    Arc<RecordingSessionFactory>,
) {
    let resources =
        ProductionResourceLedger::new(exact_limits(profile.requirements()), profile.requirements())
            .unwrap();
    let epoch = ConnectionEpoch::new(1);
    let mut topology =
        ProductionCdpTopology::new(ObjectId::new(1), epoch, profile.topology(), &resources)
            .unwrap();
    topology
        .register_context(
            epoch,
            "context-a",
            ObjectId::new(2),
            CdpContextOwnership::External,
        )
        .unwrap();
    topology
        .attach_page(
            epoch,
            CdpPageTarget::new("target-a", "session-a", "context-a"),
            CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
        )
        .unwrap();
    let factory = Arc::new(RecordingSessionFactory::default());
    let services =
        CdpPageServices::new_with_production_resources(factory.clone(), profile, &resources)
            .unwrap();
    services.synchronize(&topology).unwrap();
    (topology, services, resources, factory)
}

#[test]
fn page_input_can_be_selected_without_locator_or_keyboard_commands() {
    // Arrange
    let selected = profile();
    // Act
    let (_, services, _, _) = fixture(&selected);
    // Assert
    let input = services.page_input(ObjectId::new(3)).unwrap().unwrap();
    assert_eq!(input.policy(), CdpInputPolicy::Interleaved);
    assert!(services.locator.is_none());
    assert!(services.page_input(ObjectId::new(4)).unwrap().is_none());
    assert_eq!(services.page_input.as_ref().unwrap().active_states(), 1);
}

#[test]
fn narrow_input_binding_retains_the_original_client_and_state_after_detach() {
    // Arrange
    let (mut topology, services, _, factory) = fixture(&profile());
    let input = services.page_input(ObjectId::new(3)).unwrap().unwrap();
    let binding = services
        .page_input_binding(ObjectId::new(3))
        .unwrap()
        .unwrap();
    assert!(
        services
            .page_input_binding(ObjectId::new(4))
            .unwrap()
            .is_none()
    );
    // Act
    topology
        .detach_target(ConnectionEpoch::new(1), "session-a", Some("target-a"))
        .unwrap();
    services.synchronize(&topology).unwrap();
    // Assert
    assert!(
        services
            .page_input_binding(ObjectId::new(3))
            .unwrap()
            .is_none()
    );
    let (client, retained) = binding.into_parts();
    let expected: Arc<dyn CdpClient> = factory.client("target-a");
    assert!(Arc::ptr_eq(&client, &expected));
    assert!(retained.shares_state_with(&input));
    assert_eq!(services.page_input.as_ref().unwrap().active_states(), 1);
    drop(input);
    drop(retained);
    assert_eq!(services.page_input.as_ref().unwrap().active_states(), 0);
}

#[test]
fn explicit_page_policy_is_the_single_locator_policy_owner() {
    // Arrange
    let selected = profile().with_locator(locator_profile());
    // Act
    let (_, services, _, _) = fixture(&selected);
    // Assert
    let input = services.page_input(ObjectId::new(3)).unwrap().unwrap();
    let entry = services.route(ObjectId::new(3)).unwrap().unwrap();
    assert!(
        entry
            .locator
            .as_ref()
            .unwrap()
            .uses_input_coordinator(&input)
    );
    assert_eq!(
        selected.effective_locator_backend().unwrap().input_policy(),
        CdpInputPolicy::Interleaved
    );
}

#[tokio::test]
async fn main_frame_native_rebinding_preserves_pressed_keys() {
    // Arrange
    let (mut topology, services, _, factory) = fixture(&profile());
    let input = services.page_input(ObjectId::new(3)).unwrap().unwrap();
    let mut transaction = input
        .begin_transaction(factory.client("target-a"), None)
        .await
        .unwrap();
    transaction.key_down_named("Shift", false).await.unwrap();
    transaction.finish().await.unwrap();
    // Act
    topology
        .attach_frame(
            ConnectionEpoch::new(1),
            CdpFrameAttachment::main("target-a", "replacement"),
            ObjectId::new(5),
        )
        .unwrap();
    services.synchronize(&topology).unwrap();
    // Assert
    let replacement = services.page_input(ObjectId::new(3)).unwrap().unwrap();
    assert!(replacement.shares_state_with(&input));
    let mut transaction = replacement
        .begin_transaction(factory.client("target-a"), None)
        .await
        .unwrap();
    assert_eq!(transaction.modifiers(), CdpModifiers::SHIFT);
    transaction.key_up_named("Shift", false).await.unwrap();
    transaction.finish().await.unwrap();
    assert_eq!(services.page_input.as_ref().unwrap().active_states(), 1);
}

#[test]
fn removing_page_route_does_not_release_an_escaped_input_owner() {
    // Arrange
    let (mut topology, services, _, _) = fixture(&profile());
    let input = services.page_input(ObjectId::new(3)).unwrap().unwrap();
    let admission = services.page_input.as_ref().unwrap().clone();
    // Act
    topology
        .detach_target(ConnectionEpoch::new(1), "session-a", Some("target-a"))
        .unwrap();
    services.synchronize(&topology).unwrap();
    // Assert
    assert!(services.page_input(ObjectId::new(3)).unwrap().is_none());
    assert_eq!(admission.active_states(), 1);
    drop(input);
    assert_eq!(admission.active_states(), 0);
}

#[test]
fn disabled_page_input_preserves_legacy_no_input_profile() {
    // Arrange
    let selected = base_profile().with_navigation(navigation_profile());
    // Act
    let (_, services, _, _) = fixture(&selected);
    // Assert
    assert!(services.page_input(ObjectId::new(3)).unwrap().is_none());
    assert!(services.page_input.is_none());
}

#[test]
fn input_capacity_must_cover_declared_page_capacity() {
    // Arrange
    let selected = profile().with_page_input(CdpInputProductionProfile::new(
        ResourceScope::extension(13, 4),
        CdpInputPolicy::Interleaved,
        1,
    ));
    // Act
    let result = selected.validate();
    // Assert
    assert!(matches!(
        result,
        Err(ChromiumCompositionError::PageInputCapacityTooSmall)
    ));
}

#[test]
fn same_native_ids_in_a_new_connection_epoch_get_fresh_input_state() {
    // Arrange
    let selected = profile();
    let (old_topology, services, resources, _) = fixture(&selected);
    let old_input = services.page_input(ObjectId::new(3)).unwrap().unwrap();
    drop(old_topology);
    let epoch = ConnectionEpoch::new(2);
    let mut replacement =
        ProductionCdpTopology::new(ObjectId::new(1), epoch, selected.topology(), &resources)
            .unwrap();
    replacement
        .register_context(
            epoch,
            "context-a",
            ObjectId::new(2),
            CdpContextOwnership::External,
        )
        .unwrap();
    replacement
        .attach_page(
            epoch,
            CdpPageTarget::new("target-a", "session-a", "context-a"),
            CdpPageObjectIds::new(ObjectId::new(3), ObjectId::new(4)),
        )
        .unwrap();
    // Act
    services.synchronize(&replacement).unwrap();
    // Assert
    let new_input = services.page_input(ObjectId::new(3)).unwrap().unwrap();
    assert!(!new_input.shares_state_with(&old_input));
    assert_eq!(services.page_input.as_ref().unwrap().active_states(), 2);
    drop(old_input);
    assert_eq!(services.page_input.as_ref().unwrap().active_states(), 1);
}
