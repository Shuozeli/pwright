use pwright_playwright_server_client_core::Ingress;
use serde_json::{Value, json};

use super::{
    CloseNotice, MAX_PROJECTED_OBJECTS, MAX_QUEUED_NOTICES, MAX_URL_BYTES, Notice,
    PageEventProjection, ProjectionError,
};

fn create(parent: &str, guid: &str, object_type: &str, initializer: Value) -> Ingress {
    Ingress::Created {
        parent: parent.to_owned(),
        guid: guid.to_owned(),
        object_type: object_type.to_owned(),
        initializer,
    }
}

fn event(guid: &str, method: &str, params: Value) -> Ingress {
    Ingress::Event {
        guid: guid.to_owned(),
        method: method.to_owned(),
        params,
    }
}

fn base() -> PageEventProjection {
    let mut projection = PageEventProjection::new();
    projection
        .accept(&create("", "chromium@1", "BrowserType", json!({})))
        .unwrap();
    projection
        .accept(&create("chromium@1", "browser@1", "Browser", json!({})))
        .unwrap();
    projection
        .accept(&create(
            "browser@1",
            "context@1",
            "BrowserContext",
            json!({}),
        ))
        .unwrap();
    projection
        .accept(&create("context@1", "page@1", "Page", json!({})))
        .unwrap();
    projection
}

fn base_close_only() -> PageEventProjection {
    let mut projection = PageEventProjection::new_close_only();
    for ingress in [
        create("", "chromium@1", "BrowserType", json!({})),
        create("chromium@1", "browser@1", "Browser", json!({})),
        create("browser@1", "context@1", "BrowserContext", json!({})),
        create("context@1", "page@1", "Page", json!({})),
    ] {
        projection.accept(&ingress).unwrap();
    }
    projection
}

#[test]
fn close_only_page_event_publishes_one_typed_close_before_disposal() {
    // Arrange
    let mut projection = base_close_only();

    // Act
    projection
        .accept(&event("page@1", "close", json!({})))
        .unwrap();
    projection
        .accept(&Ingress::Disposed {
            guids: vec!["page@1".into()],
            reason: None,
        })
        .unwrap();

    // Assert
    assert!(matches!(projection.next_close_notice().unwrap(),
        Some(CloseNotice::PageClosed(id)) if id.guid == "page@1" && id.generation == 4));
    assert_eq!(projection.next_close_notice(), Ok(None));
}

#[test]
fn close_only_context_subtree_precedes_terminal_and_deduplicates_disposal() {
    // Arrange
    let mut projection = base_close_only();
    projection
        .accept(&create("context@1", "page@2", "Page", json!({})))
        .unwrap();

    // Act
    projection
        .accept(&event("context@1", "close", json!({})))
        .unwrap();
    projection
        .accept(&Ingress::Disposed {
            guids: vec!["page@1".into(), "page@2".into(), "context@1".into()],
            reason: None,
        })
        .unwrap();
    projection.finish().unwrap();

    // Assert
    assert!(matches!(projection.next_close_notice().unwrap(),
        Some(CloseNotice::PageClosed(id)) if id.guid == "page@1"));
    assert!(matches!(projection.next_close_notice().unwrap(),
        Some(CloseNotice::PageClosed(id)) if id.guid == "page@2"));
    assert!(matches!(projection.next_close_notice().unwrap(),
        Some(CloseNotice::ContextClosed(id)) if id.guid == "context@1"));
    assert!(matches!(projection.next_close_notice().unwrap(),
        Some(CloseNotice::BrowserClosed(id)) if id.guid == "browser@1"));
    assert_eq!(
        projection.next_close_notice(),
        Ok(Some(CloseNotice::Disconnected))
    );
    assert_eq!(projection.next_close_notice(), Ok(None));
}

#[test]
fn close_only_reused_guid_has_distinct_generation() {
    // Arrange
    let mut projection = base_close_only();

    // Act
    projection
        .accept(&Ingress::Disposed {
            guids: vec!["page@1".into()],
            reason: None,
        })
        .unwrap();
    projection
        .accept(&create("context@1", "page@1", "Page", json!({})))
        .unwrap();
    projection
        .accept(&event("page@1", "close", json!({})))
        .unwrap();

    // Assert
    let Some(CloseNotice::PageClosed(first)) = projection.next_close_notice().unwrap() else {
        panic!("expected first Page close");
    };
    let Some(CloseNotice::PageClosed(second)) = projection.next_close_notice().unwrap() else {
        panic!("expected second Page close");
    };
    assert_eq!(first.guid, second.guid);
    assert!(first.generation < second.generation);
    assert_eq!(projection.next_close_notice(), Ok(None));
}

#[test]
fn close_only_ignores_over_four_thousand_unrelated_objects_and_keeps_generation() {
    // Arrange
    let mut projection = base_close_only();
    let mut unrelated = Vec::new();
    for index in 0..=MAX_PROJECTED_OBJECTS {
        let guid = format!("request@{index}");
        projection
            .accept(&create(
                "page@1",
                &guid,
                "Request",
                json!({"url": "not parsed"}),
            ))
            .unwrap();
        unrelated.push(guid);
    }
    for (parent, guid, object_type) in [
        ("request@0", "response@0", "Response"),
        ("context@1", "artifact@0", "Artifact"),
    ] {
        projection
            .accept(&create(parent, guid, object_type, json!({"invalid": true})))
            .unwrap();
        unrelated.push(guid.to_owned());
    }

    // Act
    projection
        .accept(&Ingress::Adopted {
            parent: "context@1".into(),
            guid: "request@0".into(),
        })
        .unwrap();
    projection
        .accept(&event("request@0", "close", json!({})))
        .unwrap();
    projection
        .accept(&Ingress::Disposed {
            guids: unrelated.clone(),
            reason: None,
        })
        .unwrap();
    projection
        .accept(&event("page@1", "close", json!({})))
        .unwrap();
    projection
        .accept(&Ingress::Disposed {
            guids: vec!["page@1".into()],
            reason: None,
        })
        .unwrap();
    projection
        .accept(&create("context@1", "page@1", "Page", json!({})))
        .unwrap();
    projection
        .accept(&event("page@1", "close", json!({})))
        .unwrap();

    // Assert
    let Some(CloseNotice::PageClosed(first)) = projection.next_close_notice().unwrap() else {
        panic!("expected first Page close");
    };
    let Some(CloseNotice::PageClosed(second)) = projection.next_close_notice().unwrap() else {
        panic!("expected second Page close");
    };
    assert_eq!(first.guid, "page@1");
    assert_eq!(first.generation, 4);
    assert_eq!(second.guid, first.guid);
    assert_eq!(
        second.generation,
        first.generation + unrelated.len() as u64 + 1
    );
    assert_eq!(projection.objects.len(), 5);
    assert!(projection.retained_object_bytes() < 4 * 1024);
    assert_eq!(projection.next_close_notice(), Ok(None));
}

#[test]
fn close_only_ignores_network_download_crash_payloads_and_unsupported_burst() {
    // Arrange
    let mut projection = base_close_only();
    let oversized = "x".repeat(MAX_URL_BYTES + 1);

    // Act
    projection
        .accept(&create(
            "page@1",
            "request@1",
            "Request",
            json!({"url": oversized}),
        ))
        .unwrap();
    projection
        .accept(&create(
            "request@1",
            "response@1",
            "Response",
            json!({"status": "invalid"}),
        ))
        .unwrap();
    for method in ["request", "response", "download", "crash"] {
        projection
            .accept(&event("page@1", method, Value::Null))
            .unwrap();
    }
    for _ in 0..(MAX_QUEUED_NOTICES + 1) {
        projection
            .accept(&event("page@1", "other", Value::Null))
            .unwrap();
    }

    // Assert
    assert_eq!(projection.queued_notice_count(), 0);
    assert!(projection.retained_object_bytes() < 4 * 1024);
    assert_eq!(projection.next_close_notice(), Ok(None));
}

#[test]
fn close_only_notice_queue_overflow_fails_closed_with_readable_prefix() {
    // Arrange
    let mut projection = base_close_only();
    for index in 0..MAX_QUEUED_NOTICES {
        let guid = format!("page@{index}");
        if guid != "page@1" {
            projection
                .accept(&create("context@1", &guid, "Page", json!({})))
                .unwrap();
        }
        projection
            .accept(&event(&guid, "close", json!({})))
            .unwrap();
    }
    projection
        .accept(&create("context@1", "page@overflow", "Page", json!({})))
        .unwrap();

    // Act
    let result = projection.accept(&event("page@overflow", "close", json!({})));

    // Assert
    assert_eq!(result, Err(ProjectionError::NoticeLimit));
    assert_eq!(projection.queued_notice_count(), MAX_QUEUED_NOTICES);
    assert!(matches!(
        projection.next_close_notice(),
        Ok(Some(CloseNotice::PageClosed(_)))
    ));
    assert_eq!(projection.finish(), Err(ProjectionError::Failed));
}

fn add_request(projection: &mut PageEventProjection, parent: &str, guid: &str) {
    projection
        .accept(&create(
            parent,
            guid,
            "Request",
            json!({
                "url": "https://example.test/report",
                "method": "POST",
                "resourceType": "fetch",
                "headers": [{"name": "Content-Type", "value": "application/json"}],
                "isNavigationRequest": false,
                "postData": "ignored-by-this-leaf"
            }),
        ))
        .unwrap();
}

fn add_response(projection: &mut PageEventProjection, request_guid: &str, guid: &str) {
    projection
        .accept(&create(
            request_guid,
            guid,
            "Response",
            json!({
                "request": {"guid": request_guid},
                "url": "https://example.test/report",
                "status": 201,
                "statusText": "Created",
                "headers": [{"name": "Content-Type", "value": "application/json"}],
                "fromServiceWorker": false,
                "timing": {}
            }),
        ))
        .unwrap();
}

#[test]
fn matching_request_and_response_notices_survive_wire_disposal() {
    // Arrange
    let mut projection = base();
    add_request(&mut projection, "page@1", "request@1");
    add_response(&mut projection, "request@1", "response@1");

    // Act
    projection
        .accept(&event(
            "context@1",
            "request",
            json!({"request": {"guid": "request@1"}, "page": {"guid": "page@1"}}),
        ))
        .unwrap();
    projection
        .accept(&event(
            "context@1",
            "response",
            json!({"response": {"guid": "response@1"}, "page": {"guid": "page@1"}}),
        ))
        .unwrap();
    projection
        .accept(&Ingress::Disposed {
            guids: vec!["request@1".into(), "response@1".into()],
            reason: None,
        })
        .unwrap();

    // Assert
    let Some(Notice::Request {
        page,
        request,
        snapshot,
    }) = projection.next_notice()
    else {
        panic!("expected Request notice");
    };
    assert_eq!(page.guid, "page@1");
    assert_eq!(request.guid, "request@1");
    assert_eq!(snapshot.url, "https://example.test/report");
    assert_eq!(snapshot.method, "POST");
    let Some(Notice::Response {
        page,
        response,
        snapshot,
        request_snapshot,
    }) = projection.next_notice()
    else {
        panic!("expected Response notice");
    };
    assert_eq!(page.guid, "page@1");
    assert_eq!(response.guid, "response@1");
    assert_eq!(snapshot.status, 201);
    assert_eq!(snapshot.request.guid, "request@1");
    assert_eq!(request_snapshot.url, "https://example.test/report");
    assert!(projection.next_notice().is_none());
}

#[test]
fn download_notice_uses_context_scoped_artifact_without_server_path() {
    // Arrange
    let mut projection = base();
    projection
        .accept(&create(
            "context@1",
            "artifact@1",
            "Artifact",
            json!({"absolutePath": "/private/server/path"}),
        ))
        .unwrap();

    // Act
    projection
        .accept(&event(
            "page@1",
            "download",
            json!({
                "url": "https://example.test/report.csv",
                "suggestedFilename": "report.csv",
                "artifact": {"guid": "artifact@1"}
            }),
        ))
        .unwrap();
    projection
        .accept(&Ingress::Disposed {
            guids: vec!["page@1".into()],
            reason: None,
        })
        .unwrap();

    // Assert
    let Some(Notice::Download {
        page,
        artifact,
        url,
        suggested_filename,
    }) = projection.next_notice()
    else {
        panic!("expected Download notice");
    };
    assert_eq!(page.guid, "page@1");
    assert_eq!(artifact.guid, "artifact@1");
    assert_eq!(url, "https://example.test/report.csv");
    assert_eq!(suggested_filename, "report.csv");
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::PageClosed(_))
    ));
    assert!(projection.next_notice().is_none());
}

#[test]
fn context_close_retires_page_first_and_disposal_does_not_repeat_it() {
    // Arrange
    let mut projection = base();

    // Act
    projection
        .accept(&event("context@1", "close", json!({})))
        .unwrap();
    projection
        .accept(&Ingress::Disposed {
            guids: vec!["context@1".into(), "page@1".into()],
            reason: Some("closed".into()),
        })
        .unwrap();
    projection.finish().unwrap();

    // Assert
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::PageClosed(_))
    ));
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::ContextClosed(_))
    ));
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::BrowserClosed(_))
    ));
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::Disconnected)
    ));
    assert!(projection.next_notice().is_none());
}

#[test]
fn closed_context_rejects_late_page_before_it_can_break_close_order() {
    // Arrange
    let mut projection = base();
    projection
        .accept(&event("context@1", "close", json!({})))
        .unwrap();

    // Act
    let result = projection.accept(&create("context@1", "page@late", "Page", json!({})));

    // Assert
    assert_eq!(result, Err(ProjectionError::Invalid("closed parent")));
    assert!(!projection.objects.contains_key("page@late"));
    assert_eq!(projection.queued_notice_count(), 2);
    assert_eq!(projection.finish(), Err(ProjectionError::Failed));
}

#[test]
fn closed_context_rejects_page_adoption_before_it_can_break_close_order() {
    // Arrange
    let mut projection = base();
    projection
        .accept(&create(
            "browser@1",
            "context@2",
            "BrowserContext",
            json!({}),
        ))
        .unwrap();
    projection
        .accept(&create("context@2", "page@2", "Page", json!({})))
        .unwrap();
    projection
        .accept(&event("context@1", "close", json!({})))
        .unwrap();

    // Act
    let result = projection.accept(&Ingress::Adopted {
        parent: "context@1".into(),
        guid: "page@2".into(),
    });

    // Assert
    assert_eq!(result, Err(ProjectionError::Invalid("closed parent")));
    assert_eq!(
        projection.objects.get("page@2").unwrap().parent.as_deref(),
        Some("context@2")
    );
    assert_eq!(projection.finish(), Err(ProjectionError::Failed));
}

#[test]
fn browser_close_event_retires_descendants_before_disconnected() {
    // Arrange
    let mut projection = base();

    // Act
    projection
        .accept(&event("browser@1", "close", json!({})))
        .unwrap();
    projection.finish().unwrap();

    // Assert
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::PageClosed(_))
    ));
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::ContextClosed(_))
    ));
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::BrowserClosed(_))
    ));
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::Disconnected)
    ));
    assert!(projection.next_notice().is_none());
}

#[test]
fn page_crash_and_page_close_are_distinct_wire_notices() {
    // Arrange
    let mut projection = base();

    // Act
    projection
        .accept(&event("page@1", "crash", json!({})))
        .unwrap();
    projection
        .accept(&event("page@1", "close", json!({})))
        .unwrap();

    // Assert
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::PageCrashed(_))
    ));
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::PageClosed(_))
    ));
}

#[test]
fn frame_adoption_preserves_exact_page_attribution() {
    // Arrange
    let mut projection = base();
    projection
        .accept(&create("context@1", "frame@1", "Frame", json!({})))
        .unwrap();
    projection
        .accept(&Ingress::Adopted {
            parent: "page@1".into(),
            guid: "frame@1".into(),
        })
        .unwrap();
    add_request(&mut projection, "frame@1", "request@1");

    // Act
    projection
        .accept(&event(
            "context@1",
            "request",
            json!({"request": {"guid": "request@1"}, "page": {"guid": "page@1"}}),
        ))
        .unwrap();

    // Assert
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::Request { .. })
    ));
}

#[test]
fn context_only_network_event_and_unsupported_burst_do_not_queue_notices() {
    // Arrange
    let mut projection = base();
    add_request(&mut projection, "context@1", "request@1");

    // Act
    projection
        .accept(&event(
            "context@1",
            "request",
            json!({"request": {"guid": "request@1"}}),
        ))
        .unwrap();
    for index in 0..160 {
        projection
            .accept(&event(
                "browser@1",
                "syntheticNoise",
                json!({"index": index}),
            ))
            .unwrap();
    }

    // Assert
    assert_eq!(projection.queued_notice_count(), 0);
    assert!(projection.next_notice().is_none());
}

#[test]
fn malformed_page_reference_fails_closed_instead_of_attributing_elsewhere() {
    // Arrange
    let mut projection = base();
    add_request(&mut projection, "page@1", "request@1");
    projection
        .accept(&create("context@1", "page@2", "Page", json!({})))
        .unwrap();

    // Act
    let result = projection.accept(&event(
        "context@1",
        "request",
        json!({"request": {"guid": "request@1"}, "page": {"guid": "page@2"}}),
    ));

    // Assert
    assert_eq!(
        result,
        Err(ProjectionError::Invalid("network Request Page"))
    );
    assert_eq!(
        projection.accept(&event("browser@1", "noise", json!({}))),
        Err(ProjectionError::Failed)
    );
}

#[test]
fn wrong_download_artifact_parent_and_invalid_guid_fail_closed() {
    // Arrange
    let mut projection = base();
    projection
        .accept(&create("page@1", "artifact@1", "Artifact", json!({})))
        .unwrap();

    // Act
    let result = projection.accept(&event(
        "page@1",
        "download",
        json!({"url": "https://example.test/a", "suggestedFilename": "a", "artifact": {"guid": "artifact@1"}}),
    ));

    // Assert
    assert_eq!(
        result,
        Err(ProjectionError::Invalid("download Artifact context"))
    );
    let mut other = base();
    assert_eq!(
        other.accept(&create("context@1", "bad\npage", "Page", json!({}))),
        Err(ProjectionError::Invalid("GUID syntax"))
    );
}

#[test]
fn notice_capacity_is_bounded_and_terminal_after_overflow() {
    // Arrange
    let mut projection = base();
    add_request(&mut projection, "page@1", "request@1");
    let request = event(
        "context@1",
        "request",
        json!({"request": {"guid": "request@1"}, "page": {"guid": "page@1"}}),
    );

    // Act
    for _ in 0..MAX_QUEUED_NOTICES {
        projection.accept(&request).unwrap();
    }
    let overflow = projection.accept(&request);

    // Assert
    assert_eq!(overflow, Err(ProjectionError::NoticeLimit));
    assert_eq!(projection.queued_notice_count(), MAX_QUEUED_NOTICES);
    assert!(matches!(
        projection.next_notice(),
        Some(Notice::Request { .. })
    ));
    assert_eq!(projection.queued_notice_count(), MAX_QUEUED_NOTICES - 1);
    assert_eq!(projection.finish(), Err(ProjectionError::Failed));
}

#[test]
fn oversized_request_url_is_rejected_before_retention() {
    // Arrange
    let mut projection = base();
    let too_large = "x".repeat(MAX_URL_BYTES + 1);

    // Act
    let result = projection.accept(&create(
        "page@1",
        "request@1",
        "Request",
        json!({
            "url": too_large,
            "method": "GET",
            "resourceType": "fetch",
            "headers": [],
            "isNavigationRequest": false
        }),
    ));

    // Assert
    assert_eq!(result, Err(ProjectionError::MetadataLimit));
    assert!(projection.retained_object_bytes() < MAX_URL_BYTES);
}

#[test]
fn relevant_object_parent_type_mismatch_fails_closed() {
    // Arrange
    let mut projection = base();

    // Act
    let result = projection.accept(&create("page@1", "browser@2", "Browser", json!({})));

    // Assert
    assert_eq!(result, Err(ProjectionError::Invalid("object parent type")));
    assert_eq!(
        projection.accept(&event("page@1", "close", json!({}))),
        Err(ProjectionError::Failed)
    );
}

#[test]
fn reused_request_guid_gets_a_distinct_generation() {
    // Arrange
    let mut projection = base();
    add_request(&mut projection, "page@1", "request@1");
    let request_event = event(
        "context@1",
        "request",
        json!({"request": {"guid": "request@1"}, "page": {"guid": "page@1"}}),
    );

    // Act
    projection.accept(&request_event).unwrap();
    projection
        .accept(&Ingress::Disposed {
            guids: vec!["request@1".into()],
            reason: None,
        })
        .unwrap();
    add_request(&mut projection, "page@1", "request@1");
    projection.accept(&request_event).unwrap();

    // Assert
    let Some(Notice::Request { request: first, .. }) = projection.next_notice() else {
        panic!("expected first Request notice");
    };
    let Some(Notice::Request {
        request: second, ..
    }) = projection.next_notice()
    else {
        panic!("expected second Request notice");
    };
    assert_eq!(first.guid, second.guid);
    assert!(first.generation < second.generation);
}

#[test]
fn object_count_limit_is_terminal_without_partial_insertion() {
    // Arrange
    let mut projection = base();
    let already_created = 5;
    for index in already_created..MAX_PROJECTED_OBJECTS {
        projection
            .accept(&create(
                "browser@1",
                &format!("other@{index}"),
                "Other",
                json!({}),
            ))
            .unwrap();
    }

    // Act
    let result = projection.accept(&create("browser@1", "overflow@1", "Other", json!({})));

    // Assert
    assert_eq!(result, Err(ProjectionError::ObjectLimit));
    assert_eq!(projection.queued_notice_count(), 0);
    assert_eq!(projection.finish(), Err(ProjectionError::Failed));
}
