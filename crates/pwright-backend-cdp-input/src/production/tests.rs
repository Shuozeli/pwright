use super::*;
use pwright_core::ResourceLimits;
use pwright_fake::FakeCdpClient;

fn factory(
    policy: CdpInputPolicy,
    count: usize,
) -> (CdpInputServiceFactory, ProductionResourceLedger) {
    let profile = CdpInputProductionProfile::new(ResourceScope::extension(13, 4), policy, count);
    let requirements = profile.requirements();
    let bytes = requirements.class(ResourceClass::BackendStateBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        ),
        requirements,
    )
    .unwrap();
    (
        CdpInputServiceFactory::new_with_production_resources(profile, &resources).unwrap(),
        resources,
    )
}

#[test]
fn last_coordinator_clone_releases_state_admission() {
    // Arrange
    let (factory, resources) = factory(CdpInputPolicy::Interleaved, 1);
    let input = factory.create().unwrap();
    let retained = input.clone();
    // Act
    drop(input);
    // Assert
    assert_eq!(factory.active_states(), 1);
    assert!(matches!(
        factory.create(),
        Err(CdpInputAdmissionError::Capacity { limit: 1 })
    ));
    drop(retained);
    assert_eq!(factory.active_states(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn transactions_keep_state_admission_after_all_coordinators_drop() {
    for policy in [CdpInputPolicy::Exclusive, CdpInputPolicy::Interleaved] {
        // Arrange
        let (factory, resources) = factory(policy, 1);
        let input = factory.create().unwrap();
        let transaction = input
            .begin_transaction(Arc::new(FakeCdpClient::from_html("")), None)
            .await
            .unwrap();
        // Act
        drop(input);
        // Assert
        assert_eq!(factory.active_states(), 1);
        assert_eq!(resources.metrics().unwrap().active_leases, 1);
        transaction.finish().await.unwrap();
        assert_eq!(factory.active_states(), 0);
        assert_eq!(resources.metrics().unwrap().active_leases, 0);
    }
}

#[test]
fn ledger_failure_rolls_back_state_count() {
    // Arrange
    let (factory, resources) = factory(CdpInputPolicy::Exclusive, 1);
    let held = resources
        .claim(factory.profile().requirements())
        .unwrap()
        .reserve(
            factory.profile().scope(),
            ResourceClass::BackendStateBytes,
            1,
        )
        .unwrap();
    // Act
    let result = factory.create();
    // Assert
    assert!(matches!(result, Err(CdpInputAdmissionError::Resource(_))));
    assert_eq!(factory.active_states(), 0);
    drop(held);
    assert!(factory.create().is_ok());
}

#[test]
fn zero_state_profile_is_explicitly_disabled() {
    // Arrange
    let (factory, resources) = factory(CdpInputPolicy::Interleaved, 0);
    // Act
    let result = factory.create();
    // Assert
    assert!(matches!(
        result,
        Err(CdpInputAdmissionError::Capacity { limit: 0 })
    ));
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn click_owner_retains_input_admission_after_transaction_and_coordinator_drop() {
    // Arrange
    let (factory, resources) = factory(CdpInputPolicy::Interleaved, 1);
    let input = factory.create().unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    let mut transaction = input.begin_transaction(client.clone(), None).await.unwrap();
    let mut click = transaction
        .raw_mouse_click(crate::MouseClickOptions::new(10.0, 20.0).unwrap(), |_| {
            std::future::ready(Ok(()))
        })
        .unwrap();
    // Act
    drop(transaction);
    drop(input);
    (&mut click).await.unwrap();
    click.retire().await.unwrap();
    // Assert
    assert_eq!(client.calls().len(), 3);
    assert_eq!(factory.active_states(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    drop(click);
    assert_eq!(factory.active_states(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn escaped_mouse_reader_retains_page_state_admission_until_last_reader_drops() {
    use crate::{
        CdpMouseDispatcher, DirectMouseDispatch, MouseDispatchOperation, MouseDispatchRequest,
        MouseStateReader,
    };
    use pwright_cdp::CdpClient;
    use std::sync::Mutex;

    struct ReaderCapture(Arc<Mutex<Option<MouseStateReader>>>);
    impl CdpMouseDispatcher for ReaderCapture {
        fn start(
            &self,
            client: Arc<dyn CdpClient>,
            request: MouseDispatchRequest,
        ) -> Box<dyn MouseDispatchOperation> {
            *self.0.lock().unwrap() = Some(request.live_state);
            Box::new(DirectMouseDispatch::new(client, request.event))
        }
    }

    // Arrange
    let (factory, resources) = factory(CdpInputPolicy::Interleaved, 1);
    let input = factory.create().unwrap();
    let client = Arc::new(FakeCdpClient::from_html(""));
    let mut transaction = input.begin_transaction(client, None).await.unwrap();
    let reader = Arc::new(Mutex::new(None));
    let mut click = transaction
        .raw_mouse_click_with_dispatcher(
            crate::MouseClickOptions::new(10.0, 20.0).unwrap(),
            |_| std::future::ready(Ok(())),
            Arc::new(ReaderCapture(reader.clone())),
        )
        .unwrap();
    (&mut click).await.unwrap();
    click.retire().await.unwrap();
    // Act
    drop(click);
    drop(transaction);
    drop(input);
    // Assert
    assert_eq!(factory.active_states(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
    assert!(matches!(
        factory.create(),
        Err(CdpInputAdmissionError::Capacity { limit: 1 })
    ));
    let escaped = reader.lock().unwrap().take().unwrap();
    assert_eq!(escaped.snapshot().buttons, 0);
    let last = escaped.clone();
    drop(escaped);
    assert_eq!(factory.active_states(), 1);
    drop(last);
    assert_eq!(factory.active_states(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}
