use super::*;
use crate::CdpEventAdmissionPolicy;
use std::task::{Context, Waker};

fn policy() -> CdpEventAdmissionPolicy {
    CdpEventAdmissionPolicy::new("test-selection", |method, _, _| method != "Test.omit")
}
fn profile(filtered: bool) -> CdpConnectionProductionProfile {
    let profile = CdpConnectionProductionProfile::new(
        ResourceScope::Connection,
        ConnectionConfig {
            event_channel_capacity: 2,
            max_event_bytes: 512,
            ..ConnectionConfig::default()
        },
    )
    .with_reclaiming_event_fanout(1)
    .unwrap();
    if filtered {
        profile.with_event_admission_policy(policy()).unwrap()
    } else {
        profile
    }
}
fn message(value: Value) -> std::result::Result<Message, std::convert::Infallible> {
    Ok(Message::Text(value.to_string().into()))
}
async fn read(
    connection: &CdpConnection,
    input: impl Iterator<Item = std::result::Result<Message, std::convert::Infallible>> + Unpin,
) {
    let config = connection.resources.as_ref().unwrap().profile.config();
    CdpConnection::reader_loop(
        futures_util::stream::iter(input),
        connection.pending.clone(),
        connection.event_delivery.clone(),
        config.max_response_bytes,
        config.max_event_bytes,
        config.max_protocol_error_bytes,
        connection.resources.clone(),
    )
    .await;
}

#[test]
fn optional_policy_changes_neither_default_behavior_nor_resource_declaration() {
    // Arrange
    let plain = profile(false);
    // Act
    let selected = plain.clone().with_event_admission_policy(policy()).unwrap();
    // Assert
    assert!(plain.event_admission_policy().is_none());
    assert_eq!(
        selected.event_admission_policy().unwrap().name(),
        "test-selection"
    );
    assert_eq!(selected.requirements(), plain.requirements());
    assert_eq!(selected.config(), plain.config());
    assert!(selected.with_event_admission_policy(policy()).is_err());
}

#[tokio::test]
async fn thousands_of_omitted_events_do_not_need_a_slot_or_block_the_following_response() {
    // Arrange: even the full event budget is held by the caller. The actual
    // reader must skip irrelevant events BEFORE acquiring an item/byte lease.
    let profile = profile(true);
    let resources = production_resources(&profile);
    let (connection, writer) = test_production_connection_with_writer(&profile, &resources);
    let mut receiver = connection.subscribe_composable_events().unwrap();
    let held = resources
        .claim(profile.requirements())
        .unwrap()
        .reserve(ResourceScope::Connection, ResourceClass::EventItem, 3)
        .unwrap();
    let mut call = Box::pin(connection.send_command("Test.command", Value::Null));
    assert!(
        call.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let messages = (0..4096)
        .map(|_| {
            message(
                serde_json::json!({"method":"Test.omit","sessionId":"exact","params":{"value":42}}),
            )
        })
        .chain(std::iter::once(message(
            serde_json::json!({"id":1,"result":{"answer":7}}),
        )));
    // Act
    read(&connection, messages).await;
    let result = call.await;
    // Assert
    assert_eq!(result.unwrap()["answer"], 7);
    assert_eq!(
        receiver.try_recv().unwrap_err(),
        crate::CdpEventTryRecvError::Empty
    );
    assert!(connection.pending.is_empty());
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::EventItem)
            .current,
        3
    );
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::EventBytes)
            .current,
        0
    );
    drop(held);
    drop(connection);
    drop(receiver);
    // This fake writer has not sent its queued command. Discarding that
    // separate wire owner is necessary before asserting transport drainage.
    drop(writer);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[tokio::test]
async fn selected_events_retain_their_original_payload_session_and_lease() {
    // Arrange
    let profile = profile(true);
    let resources = production_resources(&profile);
    let connection = test_production_connection(&profile, &resources);
    let mut receiver = connection.subscribe_composable_events().unwrap();
    let input = serde_json::json!({"method":"Test.keep","sessionId":"exact","params":{"nested":{"answer":42}}});
    // Act
    read(&connection, std::iter::once(message(input.clone()))).await;
    let event = receiver.try_recv().unwrap();
    // Assert
    assert_eq!(event.method, "Test.keep");
    assert_eq!(event.params, input["params"]);
    assert_eq!(event.session_id.as_deref(), Some("exact"));
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::EventItem)
            .current,
        1
    );
    drop(event);
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::EventItem)
            .current,
        0
    );
}

#[tokio::test]
async fn unselected_default_profile_still_delivers_every_event_method() {
    // Arrange
    let profile = profile(false);
    let resources = production_resources(&profile);
    let connection = test_production_connection(&profile, &resources);
    let mut receiver = connection.subscribe_composable_events().unwrap();
    // Act
    read(
        &connection,
        std::iter::once(message(
            serde_json::json!({"method":"Test.omit","params":{}}),
        )),
    )
    .await;
    let event = receiver.try_recv().unwrap();
    // Assert
    assert_eq!(event.method, "Test.omit");
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::EventItem)
            .current,
        1
    );
}

#[tokio::test]
async fn oversized_omitted_event_still_terminates_the_reader_before_the_following_response() {
    // Arrange
    let profile = profile(true);
    let resources = production_resources(&profile);
    let (connection, _writer) = test_production_connection_with_writer(&profile, &resources);
    let mut call = Box::pin(connection.send_command("Test.command", Value::Null));
    assert!(
        call.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    let messages = [
        message(serde_json::json!({"method":"Test.omit","params":{"large":"x".repeat(1024)}})),
        message(serde_json::json!({"id":1,"result":{}})),
    ];
    // Act
    read(&connection, messages.into_iter()).await;
    // Assert: standalone reader has no owner failsafe. Its pending registration
    // proves it did not consume the response; dropping the caller releases it.
    assert!(
        call.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert_eq!(connection.pending.len(), 1);
    drop(call);
    assert!(connection.pending.is_empty());
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::EventItem)
            .current,
        0
    );
}

#[tokio::test]
async fn selected_event_exhaustion_is_not_hidden_by_a_policy() {
    // Arrange
    let profile = profile(true);
    let resources = production_resources(&profile);
    let (connection, _writer) = test_production_connection_with_writer(&profile, &resources);
    let mut receiver = connection.subscribe_composable_events().unwrap();
    let held = resources
        .claim(profile.requirements())
        .unwrap()
        .reserve(ResourceScope::Connection, ResourceClass::EventItem, 3)
        .unwrap();
    let mut call = Box::pin(connection.send_command("Test.command", Value::Null));
    assert!(
        call.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    // Act
    read(
        &connection,
        [
            message(serde_json::json!({"method":"Test.keep","params":{}})),
            message(serde_json::json!({"id":1,"result":{}})),
        ]
        .into_iter(),
    )
    .await;
    // Assert
    assert!(
        call.as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
            .is_pending()
    );
    assert_eq!(connection.pending.len(), 1);
    drop(call);
    assert_eq!(
        receiver.try_recv().unwrap_err(),
        crate::CdpEventTryRecvError::Empty
    );
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::EventItem)
            .current,
        3
    );
    drop(held);
}
