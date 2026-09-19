use super::*;
use crate::{
    evaluation::{EvaluationProperty, EvaluationValue},
    handles::{HandleArgumentSlot, OwnedHandleArgument},
};
use pwright_cap_handle_evaluation::HandleEvaluationValue;
use std::num::NonZeroU32;

#[tokio::test]
async fn nested_slots_preserve_graph_references_and_repeated_handle_identity() {
    // Arrange
    let (owner, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let (initializer, ticket, admission) = ticket(
        2,
        owner.admit().unwrap(),
        &cleanup,
        Arc::new(AtomicUsize::new(0)),
    );
    let pending = Arc::new(owner.retain(initializer, ticket, admission).unwrap());
    let root = NonZeroU32::new(1).unwrap();
    let value = EvaluationValue::Object {
        id: Some(root),
        properties: vec![EvaluationProperty::new(
            "list",
            EvaluationValue::Array {
                id: None,
                items: vec![
                    EvaluationValue::Undefined,
                    EvaluationValue::Ref(root),
                    EvaluationValue::Undefined,
                ],
            },
        )],
    };
    let slots = vec![
        HandleArgumentSlot::new(2, pending.clone()),
        HandleArgumentSlot::new(4, pending.clone()),
    ];

    // Act
    let argument = OwnedHandleArgument::from_slots(value, slots).unwrap();

    // Assert
    assert!(argument.has_handles());
    let (value, held) = argument.into_parts();
    let HandleEvaluationValue::Object { properties, .. } = value else {
        panic!("root object lost");
    };
    let HandleEvaluationValue::Array { items, .. } = &properties[0].value else {
        panic!("nested array lost");
    };
    assert_eq!(
        items,
        &[
            HandleEvaluationValue::Handle(ObjectId::new(2)),
            HandleEvaluationValue::Ref(root),
            HandleEvaluationValue::Handle(ObjectId::new(2))
        ]
    );
    assert_eq!(held.len(), 2);
    assert!(Arc::ptr_eq(&held[0], &held[1]));
    pending.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.completed, 1);
}

#[tokio::test]
async fn slots_cannot_overwrite_values_or_reference_missing_positions() {
    // Arrange
    let (owner, _) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let (initializer, ticket, admission) = ticket(
        2,
        owner.admit().unwrap(),
        &cleanup,
        Arc::new(AtomicUsize::new(0)),
    );
    let pending = Arc::new(owner.retain(initializer, ticket, admission).unwrap());
    for (value, indices) in [
        (EvaluationValue::Bool(true), vec![0]),
        (EvaluationValue::Undefined, vec![1]),
        (EvaluationValue::Undefined, vec![0, 0]),
        (
            EvaluationValue::Array {
                id: None,
                items: vec![EvaluationValue::Undefined, EvaluationValue::Undefined],
            },
            vec![2, 1],
        ),
    ] {
        let slots = indices
            .into_iter()
            .map(|index| HandleArgumentSlot::new(index, pending.clone()))
            .collect();
        // Act
        let result = OwnedHandleArgument::from_slots(value, slots);
        // Assert
        assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    }
    pending.dispose().await.unwrap();
    assert_eq!(cleanup.shutdown().await.metrics.completed, 1);
}

#[tokio::test]
async fn owned_arguments_retain_descriptor_admission_until_the_argument_is_dropped() {
    // Arrange
    let (owner, resources) = owners();
    let cleanup = TokioCleanupExecutor::new(1);
    let (initializer, ticket, admission) = ticket(
        2,
        owner.admit().unwrap(),
        &cleanup,
        Arc::new(AtomicUsize::new(0)),
    );
    let pending = Arc::new(owner.retain(initializer, ticket, admission).unwrap());
    let argument = OwnedHandleArgument::from_slots(
        EvaluationValue::Undefined,
        vec![HandleArgumentSlot::new(0, pending.clone())],
    )
    .unwrap();
    let released = pending.request_dispose();
    drop(pending);
    released.wait().await.unwrap();
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES + OWNER_BYTES
    );

    // Act
    drop(argument);

    // Assert
    assert_eq!(
        resources
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        STATE_BYTES
    );
    assert_eq!(cleanup.shutdown().await.metrics.completed, 1);
}
