use super::*;
use pwright_core::ResourceLimits;

#[test]
fn initialization_failures_latch_first_error_without_adding_duplicate_owners() {
    // Arrange
    let mut failures = InitializationFailures::new();
    let page = ObjectId::new(1);
    failures
        .record(page, HeaderSyntaxError::InvalidValue)
        .unwrap();
    // Act
    failures
        .record(page, HeaderSyntaxError::InvalidName)
        .unwrap();
    // Assert
    assert_eq!(failures.get(page), Some(HeaderSyntaxError::InvalidValue));
    assert_eq!(failures.0.len(), 1);
}

#[test]
fn initialization_failure_capacity_reclaims_only_retired_pages() {
    // Arrange
    let mut failures = InitializationFailures::new();
    for id in 1..=MAX_FAILED_PAGES {
        failures
            .record(ObjectId::new(id as u64), HeaderSyntaxError::InvalidName)
            .unwrap();
    }
    let extra = ObjectId::new(100);
    // Act
    let rejected = failures.record(extra, HeaderSyntaxError::NonAsciiName);
    // Assert
    assert!(matches!(rejected, Err(HostError::ResourceExhausted(_))));
    assert_eq!(failures.0.len(), MAX_FAILED_PAGES);
    failures.retain(|id| id != ObjectId::new(1));
    failures
        .record(extra, HeaderSyntaxError::NonAsciiName)
        .unwrap();
    assert_eq!(failures.get(ObjectId::new(1)), None);
    assert_eq!(
        failures.get(ObjectId::new(2)),
        Some(HeaderSyntaxError::InvalidName)
    );
    assert_eq!(failures.get(extra), Some(HeaderSyntaxError::NonAsciiName));
    assert_eq!(failures.0.len(), MAX_FAILED_PAGES);
}

#[test]
fn header_syntax_errors_preserve_native_messages() {
    // Arrange
    let cases = [
        (HeaderSyntaxError::NonAsciiName, "Invalid parameters"),
        (HeaderSyntaxError::InvalidName, "Invalid header name"),
        (HeaderSyntaxError::InvalidValue, "Invalid header value"),
    ];
    for (error, expected) in cases {
        // Act
        let result = syntax_error(error);
        // Assert
        assert!(matches!(result, HostError::Backend(message)
            if message == format!("Protocol error (Network.setExtraHTTPHeaders): {expected}")));
    }
}

#[test]
fn header_capacity_errors_remain_resource_failures() {
    // Arrange
    let cases = [
        HeaderError::CountLimit,
        HeaderError::ByteLimit,
        HeaderError::OwnerLimit,
    ];
    for error in cases {
        // Act
        let result = input_error(error);
        // Assert
        assert!(matches!(result, HostError::ResourceExhausted(_)));
    }
}

pub(super) fn ledger() -> ProductionResourceLedger {
    let req = requirements().unwrap();
    let mut limits = ResourceLimits::deny_all(req.scopes, req.leases);
    for class in ResourceClass::ALL {
        let bound = req.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    ProductionResourceLedger::new(limits, req).unwrap()
}

#[test]
fn native_parameters_preserve_page_winning_case_and_clear_to_empty_object() {
    // Arrange
    let resources = ledger();
    let pool = HeaderPool::new(value_profile().unwrap(), &resources).unwrap();
    let context = pool
        .context(&[HeaderRef {
            name: "X-A",
            value: "context",
        }])
        .unwrap();
    let page = pool
        .page(&[HeaderRef {
            name: "x-a",
            value: "page",
        }])
        .unwrap();
    let merged = pool.merge(Some(&context), Some(&page)).unwrap();
    let empty = pool.merge(None, None).unwrap();
    // Act
    let values = [native::encode(&merged), native::encode(&empty)];
    // Assert
    assert_eq!(
        values,
        [
            serde_json::json!({"headers":{"x-a":"page"}}),
            serde_json::json!({"headers":{}})
        ]
    );
}

#[test]
fn slot_admission_rejects_excess_without_incrementing_counter() {
    // Arrange
    let requests = AtomicUsize::new(MAX_REQUESTS);
    // Act
    let result = admit(&requests, MAX_REQUESTS);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(requests.load(Ordering::Acquire), MAX_REQUESTS);
}

#[test]
fn selected_allowances_cover_route_input_and_native_json_storage() {
    // Arrange
    let req = body_requirements();
    let routes = MAX_ROUTES * std::mem::size_of::<Route>();
    // Both plans share the same per-route watch, including its two waker slots.
    let watches = MAX_ROUTES * session_retirement::route_bytes();
    let inputs = MAX_HEADERS * std::mem::size_of::<HeaderRef<'static>>();
    let plans = 2
        * MAX_FAILED_PAGES
        * std::mem::size_of::<(
            ObjectId,
            Option<pwright_network_model::extra_headers::EffectiveHeaders>,
        )>();
    // Act
    let native_copy = HEADER_BYTES + MAX_HEADERS * 256 + 32 * 1024;
    // Assert
    assert!(
        2 * routes
            + watches
            + plans
            + inputs
            + native::plan_scratch_bytes()
            + std::mem::size_of::<Request>()
            + 16 * 1024
            <= REQUEST_PLAN_BYTES
    );
    assert!(inputs + std::mem::size_of::<CreationHeaders>() + 16 * 1024 <= REQUEST_BYTES);
    assert!(native_copy <= NATIVE_BYTES);
    assert!(
        req.class(ResourceClass::BackendStateBytes).global
            >= (MAX_REQUESTS * REQUEST_BYTES + MAX_ROUTES * NATIVE_BYTES) as u64
    );
}

#[test]
fn selected_pool_covers_all_frozen_plans_and_overlapping_readers() {
    // Arrange: deliberately overcount shared snapshots to bound unique owners.
    let resources = ledger();
    let pool = HeaderPool::new(value_profile().unwrap(), &resources).unwrap();
    let required = 32 + MAX_REQUESTS * (18 + 16) + MAX_ROUTES + 1;
    // Act
    let owners: Vec<_> = (0..required).map(|_| pool.page(&[]).unwrap()).collect();
    let overflow = pool.page(&[]);
    // Assert
    assert_eq!(required, MAX_VALUE_OWNERS);
    assert!(matches!(overflow, Err(HeaderError::OwnerLimit)));
    drop(owners);
    drop(pool);
    for class in ResourceClass::ALL {
        assert_eq!(resources.metrics().unwrap().class(class).current, 0);
    }
}

#[test]
fn selected_pool_and_settings_release_every_charge_after_last_owner() {
    // Arrange
    let resources = ledger();
    let pool = HeaderPool::new(value_profile().unwrap(), &resources).unwrap();
    let settings =
        HeaderSettings::new(settings_profile().unwrap(), pool.clone(), &resources).unwrap();
    settings
        .begin_context(
            ObjectId::new(2),
            &[HeaderRef {
                name: "X-A",
                value: "one",
            }],
        )
        .unwrap()
        .commit()
        .unwrap();
    let escaped = settings
        .effective(ObjectId::new(2), ObjectId::new(3))
        .unwrap();
    drop(settings);
    drop(pool);
    // Act
    drop(escaped);
    // Assert
    for class in ResourceClass::ALL {
        assert_eq!(resources.metrics().unwrap().class(class).current, 0);
    }
}

#[tokio::test]
async fn production_rejection_precedes_held_compensation_and_drains_primary_siblings() {
    for compensation_fails in [false, true] {
        // Arrange
        let resources = ledger();
        {
            let pool = HeaderPool::new(value_profile().unwrap(), &resources).unwrap();
            let settings = Arc::new(Mutex::new(
                HeaderSettings::new(settings_profile().unwrap(), pool, &resources).unwrap(),
            ));
            let context = ObjectId::new(1);
            let page = ObjectId::new(2);
            let value = |value| {
                [HeaderRef {
                    name: "X-Layer",
                    value,
                }]
            };
            settings
                .lock()
                .unwrap()
                .begin_context(context, &value("old"))
                .unwrap()
                .commit()
                .unwrap();
            let update = settings
                .lock()
                .unwrap()
                .begin_context(context, &value("new"))
                .unwrap();
            let mut update = PreparedUpdate::new(settings.clone(), update);
            let plan = DispatchPlan::capture(
                &settings.lock().unwrap(),
                vec![Route {
                    context,
                    page,
                    watch: session_retirement::Watch::new("session", "session", "target"),
                }],
            )
            .unwrap();
            let (primary_ack, primary_done) = oneshot::channel();
            let mut primary = fanout::Fanout::new();
            primary
                .push(async { Err(HostError::Backend("primary failure".into())) })
                .unwrap();
            primary
                .push(async {
                    primary_done.await.unwrap();
                    Ok(())
                })
                .unwrap();
            let error = primary.first_result().await.unwrap_err();
            let compensation_plan = update.restore_plan(&plan).unwrap();
            let (compensation_ack, compensation_done) = oneshot::channel();
            let (reply, mut received) = oneshot::channel();
            let mut compensation = fanout::Fanout::new();
            compensation
                .push(async {
                    assert_eq!(
                        compensation_plan.pages[0]
                            .1
                            .as_ref()
                            .unwrap()
                            .iter()
                            .next()
                            .unwrap()
                            .value,
                        "old"
                    );
                    compensation_done.await.unwrap();
                    if compensation_fails {
                        Err(HostError::Backend("secondary compensation failure".into()))
                    } else {
                        Ok(())
                    }
                })
                .unwrap();
            let mut work = std::pin::pin!(reject_and_drain(
                &mut update,
                &mut primary,
                Some(&mut compensation),
                reply,
                error
            ));
            // Act: poll the actual production rejection/drain coordinator once.
            std::future::poll_fn(|cx| {
                assert!(work.as_mut().poll(cx).is_pending());
                std::task::Poll::Ready(())
            })
            .await;
            // Assert: primary is visible while both native gates are still held.
            assert!(
                matches!(received.try_recv().unwrap(), Err(HostError::Backend(message)) if message == "primary failure")
            );
            assert_eq!(
                settings
                    .lock()
                    .unwrap()
                    .effective(context, page)
                    .unwrap()
                    .iter()
                    .next()
                    .unwrap()
                    .value,
                "old"
            );
            settings
                .lock()
                .unwrap()
                .begin_context(context, &value("later"))
                .unwrap()
                .commit()
                .unwrap();
            compensation_ack.send(()).unwrap();
            std::future::poll_fn(|cx| {
                assert!(work.as_mut().poll(cx).is_pending());
                std::task::Poll::Ready(())
            })
            .await;
            primary_ack.send(()).unwrap();
            work.await;
            assert_eq!(
                settings
                    .lock()
                    .unwrap()
                    .effective(context, page)
                    .unwrap()
                    .iter()
                    .next()
                    .unwrap()
                    .value,
                "later"
            );
        }
        for class in ResourceClass::ALL {
            assert_eq!(resources.metrics().unwrap().class(class).current, 0);
        }
    }
}
