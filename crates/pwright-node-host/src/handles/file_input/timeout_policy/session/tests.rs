use super::*;

fn owners() -> Arc<Owners> {
    let requirements = crate::handles::pending::requirements();
    let mut limits = ResourceLimits::deny_all(requirements.scopes, requirements.leases);
    for class in ResourceClass::ALL {
        let bound = requirements.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let resources = ProductionResourceLedger::new(limits, requirements).unwrap();
    Owners::new(&resources).unwrap()
}

fn membership(context: u64, pages: &[u64]) -> Membership {
    let mut result = Membership::default();
    result.contexts[0] = Some(ObjectId::new(context));
    for (slot, page) in result.pages.iter_mut().zip(pages) {
        *slot = Some((ObjectId::new(*page), ObjectId::new(context)));
    }
    result
}

fn session(budget: &TimeoutPolicyBudget, owners: &Arc<Owners>) -> TimeoutPolicySession {
    budget
        .session_for_origin(Arc::downgrade(owners), false)
        .unwrap()
}

#[test]
fn repeated_session_factory_calls_share_canonical_cells() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    let members = membership(1, &[2]);
    let original = index.lookup(&members, ObjectId::new(2), true).unwrap();
    original.settings().set_default_timeout(Some(7.5)).unwrap();

    // Act
    let peer = session(&budget, &owners);
    let repeated = peer.lookup(&members, ObjectId::new(2), true).unwrap();

    // Assert
    assert!(Arc::ptr_eq(&index.0, &peer.0));
    assert!(Arc::ptr_eq(
        &original.settings.inner,
        &repeated.settings.inner
    ));
    assert_eq!(repeated.settings().local_default().unwrap(), Some(7.5));
    assert_eq!(budget.inner.indexes.metrics().unwrap().active_leases, 1);
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 5);
}

#[test]
fn same_origin_cannot_change_its_injected_debug_mode() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let _index = session(&budget, &owners);

    // Act
    let changed = budget.session_for_origin(Arc::downgrade(&owners), true);

    // Assert
    assert!(matches!(changed, Err(HostError::InvalidArgument(_))));
    assert_eq!(budget.inner.indexes.metrics().unwrap().active_leases, 1);
}

#[test]
fn same_portable_ids_do_not_authorize_another_session_source() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let original = owners();
    let foreign = owners();
    let index = session(&budget, &original);
    let page = index
        .lookup(&membership(1, &[2]), ObjectId::new(2), true)
        .unwrap();
    let source = HandleSourceIdentity::timeout_policy_fixture(&foreign, Some(ObjectId::new(2)));

    // Act
    let result = page.validate_source(&source);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
}

#[test]
fn index_lookup_rejects_a_different_exact_host_origin() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let original = owners();
    let foreign = owners();
    let index = session(&budget, &original);

    // Act
    let result = index.check_origin(&Arc::downgrade(&foreign));

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 1);
}

#[test]
fn another_page_in_the_same_context_cannot_replace_creation_policy() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    let page = index
        .lookup(&membership(1, &[2, 3]), ObjectId::new(3), true)
        .unwrap();
    let source = HandleSourceIdentity::timeout_policy_fixture(&owners, Some(ObjectId::new(2)));

    // Act
    let result = page.validate_source(&source);

    // Assert
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
}

#[test]
fn a_context_root_is_not_a_handle_page_policy() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    let context = index
        .lookup(&membership(1, &[2]), ObjectId::new(1), false)
        .unwrap();
    let source = HandleSourceIdentity::timeout_policy_fixture(&owners, Some(ObjectId::new(2)));

    // Act
    let result = context.validate_source(&source);

    // Assert
    assert!(!context.is_page());
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
}

#[test]
fn missing_frozen_page_proof_is_rejected_without_current_topology_fallback() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    let page = index
        .lookup(&membership(1, &[2]), ObjectId::new(2), true)
        .unwrap();
    let source = HandleSourceIdentity::timeout_policy_fixture(&owners, None);

    // Act
    let result = page.validate_source(&source);

    // Assert
    assert!(page.is_page());
    assert!(matches!(result, Err(HostError::InvalidArgument(_))));
}

#[test]
fn generic_source_provenance_survives_closed_and_dropped_native_owners() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let weak = Arc::downgrade(&owners);
    let index = session(&budget, &owners);
    let page = index
        .lookup(&membership(1, &[2]), ObjectId::new(2), true)
        .unwrap();
    let source = HandleSourceIdentity::timeout_policy_fixture(&owners, Some(ObjectId::new(2)));
    owners.close();
    index.close();
    drop(owners);

    // Act
    let result = page.validate_source(&source);

    // Assert
    assert!(result.is_ok());
    assert!(!source.is_element());
    assert!(weak.upgrade().is_none());
    assert_eq!(
        budget
            .inner
            .resources
            .metrics()
            .unwrap()
            .class(ResourceClass::RemoteHandle)
            .current,
        0
    );
}

#[test]
fn sibling_pages_observe_one_live_context_parent_without_sharing_local_defaults() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    let members = membership(1, &[2, 3]);
    let left = index.lookup(&members, ObjectId::new(2), true).unwrap();
    let right = index.lookup(&members, ObjectId::new(3), true).unwrap();
    let context = index.lookup(&members, ObjectId::new(1), false).unwrap();
    left.settings().set_default_timeout(Some(5.25)).unwrap();

    // Act
    context.settings().set_default_timeout(Some(15.75)).unwrap();

    // Assert
    assert_eq!(left.settings().local_default().unwrap(), Some(5.25));
    assert_eq!(right.settings().local_default().unwrap(), None);
    assert_eq!(
        left.settings().parent().unwrap().local_default().unwrap(),
        Some(15.75)
    );
    assert_eq!(
        right.settings().parent().unwrap().local_default().unwrap(),
        Some(15.75)
    );
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 7);
}

#[test]
fn lookup_prunes_retired_index_entries_without_revoking_returned_metadata() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    let retired = index
        .lookup(&membership(1, &[2]), ObjectId::new(2), true)
        .unwrap();
    retired.settings().set_default_timeout(Some(3.25)).unwrap();

    // Act
    let current = index
        .lookup(&membership(10, &[20]), ObjectId::new(20), true)
        .unwrap();

    // Assert
    assert_eq!(retired.settings().local_default().unwrap(), Some(3.25));
    assert_eq!(current.settings().local_default().unwrap(), None);
    let state = index.0.index.lock().unwrap();
    assert_eq!(state.contexts.iter().flatten().count(), 1);
    assert_eq!(state.pages.iter().flatten().count(), 1);
    drop(state);
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 7);
    drop(retired);
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 4);
}

#[test]
fn repeated_page_churn_reuses_fixed_index_slots_and_cell_admission() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);

    // Act
    for page in 2..66 {
        drop(
            index
                .lookup(&membership(1, &[page]), ObjectId::new(page), true)
                .unwrap(),
        );
    }

    // Assert
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 3);
    assert_eq!(
        index.0.index.lock().unwrap().pages.iter().flatten().count(),
        1
    );
}

#[test]
fn close_is_shared_and_does_not_invalidate_a_returned_page_cell() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    let peer = index.clone();
    let members = membership(1, &[2]);
    let page = index.lookup(&members, ObjectId::new(2), true).unwrap();
    page.settings().set_default_timeout(Some(8.5)).unwrap();

    // Act
    index.close();

    // Assert
    assert!(matches!(
        peer.lookup(&members, ObjectId::new(2), true),
        Err(HostError::TargetClosed)
    ));
    assert_eq!(page.settings().local_default().unwrap(), Some(8.5));
    assert_eq!(budget.inner.indexes.metrics().unwrap().active_leases, 1);
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 4);
    drop(page);
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 1);
}

#[test]
fn held_closed_index_keeps_its_admission_until_the_last_clone_drops() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let origins: Vec<_> = (0..MAX_SESSIONS + 1).map(|_| owners()).collect();
    let mut indexes: Vec<_> = origins[..MAX_SESSIONS]
        .iter()
        .map(|owners| session(&budget, owners))
        .collect();
    indexes[0].close();

    // Act
    let refused = budget.session_for_origin(Arc::downgrade(&origins[MAX_SESSIONS]), false);
    drop(indexes.pop());
    let replacement = budget.session_for_origin(Arc::downgrade(&origins[MAX_SESSIONS]), false);

    // Assert
    assert!(matches!(refused, Err(HostError::ResourceExhausted(_))));
    assert!(replacement.is_ok());
    assert_eq!(
        budget.inner.indexes.metrics().unwrap().active_leases,
        MAX_SESSIONS
    );
    drop(replacement);
    drop(indexes);
    assert_eq!(budget.inner.indexes.metrics().unwrap().active_leases, 0);
    assert!(
        budget
            .inner
            .sessions
            .lock()
            .unwrap()
            .iter()
            .all(Option::is_none)
    );
}

#[test]
fn closed_index_cannot_be_replaced_by_repeating_the_factory() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    index.close();

    // Act
    let duplicate = session(&budget, &owners);

    // Assert
    assert!(Arc::ptr_eq(&index.0, &duplicate.0));
    assert!(matches!(
        duplicate.lookup(&membership(1, &[2]), ObjectId::new(2), true),
        Err(HostError::TargetClosed)
    ));
}

#[test]
fn failed_page_cell_admission_does_not_leave_a_new_context_index_entry() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    let _unrelated: Vec<_> = (0..super::super::MAX_CELLS - 2)
        .map(|_| budget.root(false).unwrap())
        .collect();
    let before = budget.inner.resources.metrics().unwrap();

    // Act
    let result = index.lookup(&membership(1, &[2]), ObjectId::new(2), true);

    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    let state = index.0.index.lock().unwrap();
    assert!(state.contexts.iter().all(Option::is_none));
    assert!(state.pages.iter().all(Option::is_none));
    let after = budget.inner.resources.metrics().unwrap();
    assert_eq!(after.active_leases, before.active_leases);
    assert_eq!(
        after.class(ResourceClass::BackendStateBytes).current,
        before.class(ResourceClass::BackendStateBytes).current
    );
}

#[test]
fn returned_cell_does_not_keep_index_admission_or_native_owners_alive() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let weak = Arc::downgrade(&owners);
    let index = session(&budget, &owners);
    let page = index
        .lookup(&membership(1, &[2]), ObjectId::new(2), true)
        .unwrap();
    let resources = budget.inner.resources.clone();
    let indexes = budget.inner.indexes.clone();

    // Act
    drop(index);
    drop(owners);
    drop(budget);

    // Assert
    assert!(weak.upgrade().is_none());
    assert_eq!(indexes.metrics().unwrap().active_leases, 0);
    assert_eq!(resources.metrics().unwrap().active_leases, 4);
    assert_eq!(page.settings().local_default().unwrap(), None);
    drop(page);
    assert_eq!(resources.metrics().unwrap().active_leases, 0);
}

#[test]
fn absent_page_and_parent_never_create_implicit_settings() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);

    // Act
    let result = index.lookup(&Membership::default(), ObjectId::new(2), true);

    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 1);
}

#[test]
fn invalid_id_syntax_is_rejected_without_normalization_or_allocation() {
    // Arrange
    let invalid = [
        "",
        "+1",
        "-1",
        " 1",
        "1 ",
        "18446744073709551616",
        "123456789012345678901",
    ];

    // Act
    let results: Vec<_> = invalid.into_iter().map(parse_id).collect();

    // Assert
    assert!(
        results
            .iter()
            .all(|result| matches!(result, Err(HostError::InvalidArgument(_))))
    );
}

#[test]
fn repeated_canonical_page_lookups_have_bounded_publication_bindings() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    let members = membership(1, &[2]);
    let mut bindings: Vec<_> = (0..super::super::MAX_CELLS - 2)
        .map(|_| index.lookup(&members, ObjectId::new(2), true).unwrap())
        .collect();

    // Act
    let refused = index.lookup(&members, ObjectId::new(2), true);
    drop(bindings.pop());
    let replacement = index.lookup(&members, ObjectId::new(2), true);

    // Assert
    assert!(matches!(refused, Err(HostError::ResourceExhausted(_))));
    assert!(replacement.is_ok());
    assert_eq!(
        budget.inner.resources.metrics().unwrap().active_leases,
        super::super::MAX_CELLS + 1
    );
    drop(bindings);
    drop(replacement);
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 3);
    index.close();
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 1);
}

#[test]
fn cloning_a_scoped_binding_retains_one_admission_through_index_close() {
    // Arrange
    let budget = TimeoutPolicyBudget::new().unwrap();
    let owners = owners();
    let index = session(&budget, &owners);
    let page = index
        .lookup(&membership(1, &[2]), ObjectId::new(2), true)
        .unwrap();
    let duplicate = page.clone();
    index.close();
    drop(page);

    // Act
    let retained = budget.inner.resources.metrics().unwrap();

    // Assert
    assert_eq!(retained.active_leases, 4);
    assert_eq!(duplicate.settings().local_default().unwrap(), None);
    drop(duplicate);
    assert_eq!(budget.inner.resources.metrics().unwrap().active_leases, 1);
}
