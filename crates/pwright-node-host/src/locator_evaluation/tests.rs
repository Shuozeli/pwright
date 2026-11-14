use super::*;
use pwright_core::ResourceLimits;
use std::task::{Context, Poll, Waker};

#[cfg(feature = "retained-file-input")]
#[test]
fn retained_uploads_add_eight_independent_world_snapshot_and_retirement_slots() {
    // Arrange: eight normal calls, one continuing upload, 64 Handle worlds,
    // and eight independently abandoned retained acquisitions.
    let capacity = 81
        + MARKER_WORLDS
        + if cfg!(feature = "drag-runtime") {
            1024
        } else {
            0
        };
    let expected = ChromiumFrameWorldAccessProfile::new(
        ResourceScope::extension(120, 14),
        NonZeroUsize::new(capacity).unwrap(),
    )
    .unwrap()
    .with_native_retirement()
    .unwrap()
    .requirements();
    // Act
    let actual = Profile::new().unwrap().access.requirements();
    // Assert
    assert_eq!(RETAINED_UPLOAD_WORLDS, 8);
    assert_eq!(actual, expected);
    assert_eq!(actual.leases, capacity + 1);
}

#[cfg(feature = "retained-target-marking")]
#[test]
fn marker_worlds_add_seventy_two_independent_persistent_snapshot_slots() {
    // Arrange
    let baseline = 81
        + if cfg!(feature = "drag-runtime") {
            1024
        } else {
            0
        };
    // Act
    let capacity = WORLD_SNAPSHOTS;
    // Assert
    assert_eq!(MARKER_WORLDS, 72);
    assert_eq!(capacity, baseline + 72);
    assert_eq!(RETAINED_UPLOAD_WORLDS, 8);
}

fn poll<F: Future>(future: Pin<&mut F>) -> Poll<F::Output> {
    future.poll(&mut Context::from_waker(Waker::noop()))
}

#[test]
fn expired_resolution_deadline_prevents_queued_work_from_starting() {
    // Arrange
    let started = AtomicBool::new(false);
    let operation = async {
        started.store(true, Ordering::Relaxed);
        Ok(1)
    };
    let mut run = Box::pin(phase::run(
        operation,
        Arc::new(AtomicBool::new(true)),
        ready(()),
        pending(),
        23,
    ));
    // Act
    let outcome = poll(run.as_mut());
    // Assert
    assert!(matches!(outcome, Poll::Ready(Err(HostError::Timeout(23)))));
    assert!(!started.load(Ordering::Relaxed));
}

#[test]
fn resolution_deadline_does_not_expire_delayed_conversion_or_evaluation() {
    // Arrange
    let preparing = Arc::new(AtomicBool::new(true));
    let entered = preparing.clone();
    let (expire, deadline) = tokio::sync::oneshot::channel();
    let (complete, value) = tokio::sync::oneshot::channel();
    let mut run = Box::pin(phase::run(
        async move {
            entered.store(false, Ordering::Release);
            Ok(value.await.unwrap())
        },
        preparing,
        async {
            deadline.await.unwrap();
        },
        pending(),
        23,
    ));
    assert!(poll(run.as_mut()).is_pending());
    // Act: expire only the former preparation budget, while conversion waits.
    expire.send(()).unwrap();
    let during = poll(run.as_mut());
    complete.send(7).unwrap();
    let outcome = poll(run.as_mut());
    // Assert
    assert!(during.is_pending());
    assert!(matches!(outcome, Poll::Ready(Ok(7))));
}

#[test]
fn environment_shutdown_still_abandons_work_after_preparation() {
    // Arrange
    let dropped = Arc::new(AtomicBool::new(false));
    struct Guard(Arc<AtomicBool>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.store(true, Ordering::Release);
        }
    }
    let guard = Guard(dropped.clone());
    let (close, stop) = tokio::sync::oneshot::channel();
    let mut run = Box::pin(phase::run(
        async move {
            let _guard = guard;
            pending::<Result<(), HostError>>().await
        },
        Arc::new(AtomicBool::new(false)),
        ready(()),
        async {
            stop.await.unwrap();
        },
        23,
    ));
    assert!(poll(run.as_mut()).is_pending());
    // Act
    close.send(()).unwrap();
    let outcome = poll(run.as_mut());
    drop(run);
    // Assert
    assert!(matches!(outcome, Poll::Ready(Err(HostError::TargetClosed))));
    assert!(dropped.load(Ordering::Acquire));
}

#[test]
fn primary_error_is_not_replaced_by_an_unexpired_resolution_deadline() {
    // Arrange
    let mut run = Box::pin(phase::run(
        ready(Err::<(), _>(HostError::InvalidArgument(
            "getter failed".into(),
        ))),
        Arc::new(AtomicBool::new(true)),
        pending(),
        pending(),
        23,
    ));
    // Act
    let outcome = poll(run.as_mut());
    // Assert
    assert!(
        matches!(outcome, Poll::Ready(Err(HostError::InvalidArgument(message))) if message == "getter failed")
    );
}

fn budget() -> (ProductionResourceLedger, ResourceLedger, Arc<Slots>) {
    let requirements = Profile::state_requirements();
    let bytes = requirements.class(ResourceClass::BackendStateBytes);
    let limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases).with_class(
        ResourceClass::BackendStateBytes,
        bytes.global,
        bytes.per_scope,
    );
    let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
    let ledger = resources.claim(requirements).unwrap();
    let lease = ledger
        .reserve(STATE_SCOPE, ResourceClass::BackendStateBytes, STATE_BYTES)
        .unwrap();
    (
        resources,
        ledger,
        Arc::new(Slots {
            active: AtomicUsize::new(0),
            _state: lease,
        }),
    )
}

#[test]
fn call_admission_is_bounded_across_clones_and_reclaimed_after_drop() {
    // Arrange
    let (resources, ledger, slots) = budget();
    let mut calls: Vec<_> = (0..PENDING)
        .map(|_| CallLease::admit(slots.clone(), &ledger).unwrap())
        .collect();
    // Act
    let full = CallLease::admit(slots.clone(), &ledger);
    calls.pop();
    let next = CallLease::admit(slots.clone(), &ledger);
    // Assert
    assert!(matches!(full, Err(HostError::ResourceExhausted(_))));
    assert!(next.is_ok());
    assert_eq!(slots.active.load(Ordering::Acquire), PENDING);
    drop(next);
    drop(calls);
    assert_eq!(slots.active.load(Ordering::Acquire), 0);
    drop(slots);
    drop(ledger);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn failed_resource_reservation_rolls_back_the_call_counter() {
    // Arrange
    let (_resources, ledger, slots) = budget();
    let retained = ledger
        .reserve(
            STATE_SCOPE,
            ResourceClass::BackendStateBytes,
            CALL_BYTES * PENDING as u64,
        )
        .unwrap();
    // Act
    let rejected = CallLease::admit(slots.clone(), &ledger);
    // Assert
    assert!(rejected.is_err());
    assert_eq!(slots.active.load(Ordering::Acquire), 0);
    drop(retained);
    assert!(CallLease::admit(slots, &ledger).is_ok());
}

#[test]
fn declared_call_storage_covers_bounded_conversion_and_value_trees() {
    // Arrange
    let value_bytes = evaluation::MAX_NODES * std::mem::size_of::<EvaluationValue>()
        + evaluation::MAX_ENTRIES * std::mem::size_of::<evaluation::EvaluationProperty>()
        + evaluation::STRING_BYTES
        + evaluation::BINARY_BYTES;
    // Act
    let peak =
        2 * value_bytes + COLLECTION_SOURCE_BYTES + CONVERSION_STATE_BYTES + COMMAND_FUTURE_BYTES;
    // Assert
    assert!(peak < CALL_BYTES as usize, "{peak} exceeds {CALL_BYTES}");
    assert!(
        std::mem::size_of::<Installation>() + std::mem::size_of::<Slots>() < STATE_BYTES as usize
    );
}

#[test]
fn collection_evaluation_wraps_one_compiled_selector_and_preserves_argument() {
    // Arrange
    let selector = CollectionSelector::Standard(Selector::css("div.item".to_owned()).unwrap());
    let argument = EvaluationValue::String("!".into());
    let params = EvaluateParams::function(
        "(elements, suffix) => elements.map(element => element.textContent + suffix)",
        argument.clone(),
    );

    // Act
    let wrapped = collection_params(&selector, CollectionMode::All, params).unwrap();

    // Assert
    assert_eq!(wrapped.mode(), EvaluationMode::Function);
    assert_eq!(wrapped.argument(), Some(&argument));
    assert!(wrapped.source().contains(SELECTOR_RUNTIME_SYMBOL));
    assert!(wrapped.source().contains("div.item"));
    assert!(
        wrapped
            .source()
            .contains("__pwFunction(__pwMatches, __pwArgument)")
    );
    assert!(wrapped.source().len() <= COLLECTION_SOURCE_BYTES);
}

#[cfg(feature = "react-selectors")]
#[test]
fn react_count_checks_deprecation_only_after_final_composed_matches() {
    // Arrange
    let budget = crate::locator_query::QueryBudget::new(crate::locator_query::QueryLimits {
        recipes: 4,
        bytes: 64 * 1024,
    })
    .unwrap();
    let source = "_react=BookItem >> text=Gatsby";
    let query = budget
        .prepare_leaf(source.len())
        .unwrap()
        .css(source.to_owned())
        .unwrap();
    let (crate::locator_query::CommandPlan::Browser(program), _lease) =
        query.command_plan().unwrap()
    else {
        panic!("React selector must use the browser plan");
    };
    let params = EvaluateParams::function_without_argument("elements => elements.length");

    // Act
    let wrapped = collection_params(
        &CollectionSelector::Browser(program),
        CollectionMode::All,
        params,
    )
    .unwrap();
    let script = wrapped.source();

    // Assert
    let query_at = script.find("__pw.within").unwrap();
    let check_at = script.find("if (__pwMatches.length)").unwrap();
    assert!(query_at < check_at);
    assert!(script.contains("\"_react\" selector is not supported"));
    assert!(script.contains("return await __pwFunction(__pwMatches"));
}

#[test]
fn standard_single_evaluation_uses_the_first_match_without_locator_strictness() {
    // Arrange
    let selector = CollectionSelector::Standard(Selector::text("ready".to_owned(), true));
    let params = EvaluateParams::function_without_argument("element => element.nodeName");

    // Act
    let wrapped = collection_params(&selector, CollectionMode::One, params).unwrap();

    // Assert
    assert!(wrapped.source().contains("__pwMatches[0]"));
    assert!(!wrapped.source().contains("strict mode violation"));
}

#[test]
fn collection_evaluation_rejects_expression_mode_before_browser_dispatch() {
    // Arrange
    let selector = CollectionSelector::Standard(Selector::css("div".to_owned()).unwrap());
    let params = EvaluateParams::expression("document.body");

    // Act
    let result = collection_params(&selector, CollectionMode::All, params);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(message))
        if message == "locator.evaluateAll requires a function"));
}

#[cfg(feature = "selector-registration")]
#[test]
fn registered_single_evaluation_uses_the_first_match_without_locator_strictness() {
    // Arrange
    let registry = crate::selector_registry::SelectorRegistry::default();
    registry
        .register("tag", "({ queryAll() { return [1, 2]; } })", false)
        .unwrap();
    let selector = CollectionSelector::Registered(
        registry
            .parse_program("tag=DIV")
            .unwrap()
            .expect("registered selector should produce a program"),
    );
    let params = EvaluateParams::function_without_argument("element => element.nodeName");

    // Act
    let wrapped = collection_params(&selector, CollectionMode::One, params).unwrap();

    // Assert
    assert!(wrapped.source().contains("__pwMatches[0]"));
    assert!(!wrapped.source().contains("strict mode violation"));
}

#[test]
fn collection_evaluation_has_an_independent_bounded_backend_lane() {
    // Arrange
    let profile = Profile::new().unwrap();

    // Act
    let collection = profile.collection;

    // Assert
    assert_eq!(collection.max_concurrent_operations(), EXECUTING);
    assert_eq!(collection.limits(), collection_limits());
    assert_eq!(collection.requirements().leases, EXECUTING);
    assert!(collection.uses_user_gesture());
    assert!(profile.adoption.uses_user_gesture());
}

#[test]
fn structured_javascript_errors_survive_element_evaluation_mapping() {
    // Arrange
    let error = pwright_cap_handle_evaluation::HandleEvaluationErrorDetails::Javascript {
        name: "TypeError".into(),
        message: "original".into(),
        stack: Some("stack".into()),
    };
    // Act
    let error = errors::evaluation(
        pwright_composition_chromium_locator_evaluation::ElementEvaluationError::Backend(
            OperationError::new(FailureKind::Backend, "native failed", Some(error)),
        ),
    );
    // Assert
    assert!(
        matches!(error, HostError::Javascript { name, message, stack: Some(stack) }
        if name == "TypeError" && message == "original" && stack == "stack")
    );
}

#[test]
fn conversion_source_is_validated_before_native_cloning() {
    // Arrange
    let params = EvaluateParams::expression("x".repeat(evaluation::SOURCE_BYTES + 1));
    // Act
    let error = validate_input(params);
    // Assert
    assert!(matches!(error, Err(HostError::InvalidArgument(_))));
}
