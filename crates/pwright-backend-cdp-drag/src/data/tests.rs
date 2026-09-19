use super::*;
use pwright_core::ResourceLimits;
use serde_json::json;

fn limits() -> DragDataLimits {
    DragDataLimits {
        items: 4,
        files: 3,
        text_bytes: 256,
    }
}
fn setup(limits: DragDataLimits, records: usize) -> (DragDataStore, ProductionResourceLedger) {
    let profile = DragDataProfile::new(
        ResourceScope::extension(14, 61),
        limits,
        NonZeroUsize::new(records).unwrap(),
    )
    .unwrap();
    let req = profile.requirements();
    let bytes = req.class(ResourceClass::BackendStateBytes);
    let resources = ProductionResourceLedger::new(
        ResourceLimits::deny_all(req.scopes, req.leases).with_class(
            ResourceClass::BackendStateBytes,
            bytes.global,
            bytes.per_scope,
        ),
        req,
    )
    .unwrap();
    (DragDataStore::new(profile, &resources).unwrap(), resources)
}
fn event(data: Value) -> CdpEvent {
    CdpEvent::new(
        "Input.dragIntercepted".into(),
        json!({"data":data}),
        Some("session-1".into()),
    )
}
fn sample() -> Value {
    json!({"items":[{"mimeType":"text/plain","data":"hello"}], "dragOperationsMask":17})
}

#[test]
fn admitted_payload_preserves_all_fields_without_retaining_source_event() {
    // Arrange
    let (store, resources) = setup(limits(), 1);
    let source = event(
        json!({"items":[{"mimeType":"text/html","data":"<b>你好</b>","title":"title","baseURL":"https://example.test/"}],
        "files":["/opaque/path"],"dragOperationsMask":i64::MAX}),
    );
    // Act
    let admitted = store.admit(&source, "session-1").unwrap().unwrap();
    drop(source);
    // Assert
    assert_eq!(admitted.data().items[0].data, "<b>你好</b>");
    assert_eq!(admitted.data().items[0].title.as_deref(), Some("title"));
    assert_eq!(
        admitted.data().items[0].base_url.as_deref(),
        Some("https://example.test/")
    );
    assert_eq!(admitted.data().files.as_ref().unwrap(), &["/opaque/path"]);
    assert_eq!(admitted.data().drag_operations_mask, i64::MAX);
    assert_eq!(admitted.session_id(), "session-1");
    assert_eq!(store.active_records(), 1);
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
}

#[test]
fn clones_share_admitted_storage_until_the_last_escape_drops() {
    // Arrange
    let (store, resources) = setup(limits(), 1);
    let original = store.admit(&event(sample()), "session-1").unwrap().unwrap();
    let escaped = original.clone();
    assert!(original.shares_payload_with(&escaped));
    // Act
    drop(original);
    drop(store);
    // Assert
    assert_eq!(resources.metrics().unwrap().active_leases, 2);
    assert_eq!(escaped.data().items[0].data, "hello");
    drop(escaped);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn retained_payload_blocks_capacity_until_its_last_clone_drops() {
    // Arrange
    let (store, _) = setup(limits(), 1);
    let source = event(sample());
    let first = store.admit(&source, "session-1").unwrap().unwrap();
    let clone = first.clone();
    drop(first);
    // Act
    let rejected = store.admit(&source, "session-1").err().unwrap();
    // Assert
    assert_eq!(rejected, DragDataAdmissionError::Capacity);
    assert_eq!(store.active_records(), 1);
    drop(clone);
    assert_eq!(store.active_records(), 0);
    assert!(store.admit(&source, "session-1").unwrap().is_some());
}

#[test]
fn wrong_session_or_method_never_validates_or_retains_unrelated_payload() {
    // Arrange
    let (store, resources) = setup(limits(), 1);
    let wrong_method = CdpEvent::new(
        "Runtime.consoleAPICalled".into(),
        Value::Null,
        Some("session-1".into()),
    );
    let wrong_session = CdpEvent::new(
        "Input.dragIntercepted".into(),
        Value::Null,
        Some("other".into()),
    );
    let no_session = CdpEvent::new("Input.dragIntercepted".into(), Value::Null, None);
    // Act
    let results =
        [wrong_method, wrong_session, no_session].map(|event| store.admit(&event, "session-1"));
    // Assert
    for result in results {
        assert!(result.unwrap().is_none());
    }
    assert_eq!(store.active_records(), 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 1);
}

#[test]
fn empty_expected_session_is_rejected_even_for_unrelated_events() {
    // Arrange
    let (store, _) = setup(limits(), 1);
    // Act
    let result = store.admit(&event(sample()), "").err().unwrap();
    // Assert
    assert_eq!(result, DragDataAdmissionError::Session);
    assert_eq!(store.active_records(), 0);
}

#[test]
fn malformed_fields_are_rejected_without_a_retained_record() {
    for (data, expected) in [
        (Value::Null, DragDataAdmissionError::Object),
        (
            json!({"items":null,"dragOperationsMask":1}),
            DragDataAdmissionError::Items,
        ),
        (
            json!({"items":[],"dragOperationsMask":1.5}),
            DragDataAdmissionError::Mask,
        ),
        (
            json!({"items":[],"dragOperationsMask":u64::MAX}),
            DragDataAdmissionError::Mask,
        ),
        (
            json!({"items":[false],"dragOperationsMask":1}),
            DragDataAdmissionError::Object,
        ),
        (
            json!({"items":[{"mimeType":null,"data":"x"}],"dragOperationsMask":1}),
            DragDataAdmissionError::String,
        ),
        (
            json!({"items":[{"mimeType":"x","data":1}],"dragOperationsMask":1}),
            DragDataAdmissionError::String,
        ),
        (
            json!({"items":[{"mimeType":"x","data":"y","title":false}],"dragOperationsMask":1}),
            DragDataAdmissionError::String,
        ),
        (
            json!({"items":[],"files":false,"dragOperationsMask":1}),
            DragDataAdmissionError::Files,
        ),
        (
            json!({"items":[],"files":[null],"dragOperationsMask":1}),
            DragDataAdmissionError::String,
        ),
    ] {
        // Arrange
        let (store, resources) = setup(limits(), 1);
        // Act
        let error = store.admit(&event(data), "session-1").err().unwrap();
        // Assert
        assert_eq!(error, expected);
        assert_eq!(store.active_records(), 0);
        assert_eq!(resources.metrics().unwrap().active_leases, 1);
    }
}

#[test]
fn unknown_fields_do_not_get_silently_dropped_during_typed_copy() {
    for params in [
        json!({"data":sample(),"future":true}),
        json!({"data":{"items":[],"dragOperationsMask":1,"future":true}}),
        json!({"data":{"items":[{"mimeType":"x","data":"y","future":true}],"dragOperationsMask":1}}),
    ] {
        // Arrange
        let (store, _) = setup(limits(), 1);
        let source = CdpEvent::new(
            "Input.dragIntercepted".into(),
            params,
            Some("session-1".into()),
        );
        // Act
        let result = store.admit(&source, "session-1").err().unwrap();
        // Assert
        assert_eq!(result, DragDataAdmissionError::UnknownField);
        assert_eq!(store.active_records(), 0);
    }
}

#[test]
fn utf8_limit_is_aggregate_across_every_retained_string() {
    // Arrange
    let data = json!({"items":[{"mimeType":"x","data":"你","title":"t","baseURL":"u"}],"files":["f"],"dragOperationsMask":1});
    let (exact, _) = setup(
        DragDataLimits {
            text_bytes: 7,
            ..limits()
        },
        1,
    );
    let (short, _) = setup(
        DragDataLimits {
            text_bytes: 6,
            ..limits()
        },
        1,
    );
    // Act
    let accepted = exact
        .admit(&event(data.clone()), "session-1")
        .unwrap()
        .unwrap();
    let rejected = short.admit(&event(data), "session-1").err().unwrap();
    // Assert
    assert_eq!(accepted.data().items[0].data, "你");
    assert_eq!(rejected, DragDataAdmissionError::TextBytes);
    assert_eq!(short.active_records(), 0);
}

#[test]
fn count_limits_cover_empty_strings_not_just_total_bytes() {
    // Arrange
    let (store, _) = setup(
        DragDataLimits {
            items: 1,
            files: 1,
            text_bytes: 0,
        },
        1,
    );
    // Act
    let items = store.admit(&event(json!({"items":[{"mimeType":"","data":""},{"mimeType":"","data":""}],"dragOperationsMask":0})), "session-1").err().unwrap();
    let files = store
        .admit(
            &event(json!({"items":[],"files":["",""],"dragOperationsMask":0})),
            "session-1",
        )
        .err()
        .unwrap();
    // Assert
    assert_eq!(items, DragDataAdmissionError::ItemCount);
    assert_eq!(files, DragDataAdmissionError::FileCount);
    assert_eq!(store.active_records(), 0);
}

#[test]
fn absent_and_null_optional_fields_preserve_generated_type_semantics() {
    // Arrange
    let (store, _) = setup(limits(), 1);
    let source = event(
        json!({"items":[{"mimeType":"","data":"","title":null,"baseURL":null}],"files":null,"dragOperationsMask":i64::MIN}),
    );
    // Act
    let admitted = store.admit(&source, "session-1").unwrap().unwrap();
    // Assert
    assert!(admitted.data().files.is_none());
    assert!(admitted.data().items[0].title.is_none());
    assert!(admitted.data().items[0].base_url.is_none());
    assert_eq!(admitted.data().drag_operations_mask, i64::MIN);
}

#[test]
fn empty_file_list_remains_distinct_from_absent_file_list() {
    // Arrange
    let (store, _) = setup(limits(), 1);
    let source = event(json!({"items":[],"files":[],"dragOperationsMask":0}));
    // Act
    let admitted = store.admit(&source, "session-1").unwrap().unwrap();
    // Assert
    assert_eq!(admitted.data().files, Some(vec![]));
}

#[test]
fn storage_rejection_rolls_back_count_before_any_payload_copy() {
    // Arrange
    let (store, resources) = setup(limits(), 1);
    let blocker = resources
        .claim(store.0.profile.requirements())
        .unwrap()
        .reserve(store.0.profile.scope, ResourceClass::BackendStateBytes, 1)
        .unwrap();
    // Act
    let result = store.admit(&event(sample()), "session-1").err().unwrap();
    // Assert
    assert_eq!(result, DragDataAdmissionError::Storage);
    assert_eq!(store.active_records(), 0);
    drop(blocker);
    assert!(
        store
            .admit(&event(sample()), "session-1")
            .unwrap()
            .is_some()
    );
}

#[test]
fn profile_arithmetic_fails_closed_on_size_overflow() {
    // Arrange
    let scope = ResourceScope::extension(14, 61);
    // Act
    let result = DragDataProfile::new(
        scope,
        DragDataLimits {
            items: usize::MAX,
            files: 0,
            text_bytes: 0,
        },
        NonZeroUsize::new(1).unwrap(),
    );
    // Assert
    assert_eq!(result.unwrap_err(), DragDataAdmissionError::ProfileOverflow);
}

#[test]
fn requested_record_payload_fits_its_declared_lease() {
    // Arrange
    let (store, _) = setup(limits(), 1);
    let admitted = store.admit(&event(sample()), "session-1").unwrap().unwrap();
    let data = admitted.data();
    // Act
    let requested = std::mem::size_of::<Record>()
        + admitted.0.session.capacity()
        + 2 * std::mem::size_of::<usize>()
        + data.items.capacity() * std::mem::size_of::<DragDataItem>()
        + data
            .items
            .iter()
            .map(|item| {
                item.mime_type.capacity()
                    + item.data.capacity()
                    + item.title.as_ref().map_or(0, String::capacity)
                    + item.base_url.as_ref().map_or(0, String::capacity)
            })
            .sum::<usize>();
    let requested = requested
        + data.files.as_ref().map_or(0, |files| {
            files.capacity() * std::mem::size_of::<String>()
                + files.iter().map(String::capacity).sum::<usize>()
        });
    // Assert
    assert!(requested as u64 <= store.0.profile.record_bytes());
}
