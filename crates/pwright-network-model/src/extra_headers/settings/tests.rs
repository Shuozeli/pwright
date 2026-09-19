use super::*;
use crate::extra_headers::HeaderProfile;
use pwright_core::ResourceLimits;

fn fixture(
    contexts: usize,
    pages: usize,
    owners: usize,
) -> (HeaderSettings, ProductionResourceLedger) {
    fixture_with_writers(contexts, pages, owners, 1)
}
fn fixture_with_writers(
    contexts: usize,
    pages: usize,
    owners: usize,
    writers: usize,
) -> (HeaderSettings, ProductionResourceLedger) {
    let values = HeaderProfile::new(ResourceScope::extension(130, 0), owners, 4, 128).unwrap();
    let settings = HeaderSettingsProfile::new(ResourceScope::extension(130, 1), contexts, pages)
        .unwrap()
        .with_concurrent_writers(writers)
        .unwrap();
    let req = values
        .requirements()
        .unwrap()
        .combine_concurrent(settings.requirements());
    let mut limits = ResourceLimits::deny_all(req.scopes, req.leases);
    for class in ResourceClass::ALL {
        let bound = req.class(class);
        limits = limits.with_class(class, bound.global, bound.per_scope);
    }
    let ledger = ProductionResourceLedger::new(limits, req).unwrap();
    let pool = HeaderPool::new(values, &ledger).unwrap();
    (
        HeaderSettings::new(settings, pool, &ledger).unwrap(),
        ledger,
    )
}
fn h(value: &str) -> [HeaderRef<'_>; 1] {
    [HeaderRef { name: "X-A", value }]
}
fn values(headers: &EffectiveHeaders) -> Vec<(&str, &str)> {
    headers
        .iter()
        .map(|header| (header.name, header.value))
        .collect()
}
const CONTEXT: ObjectId = ObjectId::new(1);
const PAGE: ObjectId = ObjectId::new(2);

#[test]
fn concurrent_writer_profile_rejects_zero_overflow_and_charges_pending_storage() {
    // Arrange
    let profile = HeaderSettingsProfile::new(ResourceScope::extension(130, 1), 1, 1).unwrap();
    // Act
    let two = profile.with_concurrent_writers(2).unwrap();
    // Assert
    assert_eq!(
        two.bytes - profile.bytes,
        std::mem::size_of::<Pending>() as u64
    );
    assert!(matches!(
        profile.with_concurrent_writers(0),
        Err(SettingsError::InvalidLimits)
    ));
    assert!(matches!(
        profile.with_concurrent_writers(usize::MAX),
        Err(SettingsError::InvalidLimits)
    ));
}

#[test]
fn overlapping_context_rollback_preserves_a_separately_committed_page_override() {
    // Arrange
    let (book, _) = fixture_with_writers(1, 1, 8, 2);
    book.begin_context(CONTEXT, &h("old-context"))
        .unwrap()
        .commit()
        .unwrap();
    let context = book.begin_context(CONTEXT, &h("new-context")).unwrap();
    let page = book
        .begin_page(
            CONTEXT,
            PAGE,
            &[HeaderRef {
                name: "X-B",
                value: "new-page",
            }],
        )
        .unwrap();
    context.restore_prior().unwrap();
    page.commit().unwrap();
    // Act
    context.rollback().unwrap();
    // Assert
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "old-context"), ("X-B", "new-page")]
    );
}

#[test]
fn older_failure_overwrites_newer_desired_value_and_later_success_does_not_republish() {
    // Arrange
    let (book, _) = fixture_with_writers(1, 1, 8, 2);
    book.begin_page(CONTEXT, PAGE, &h("old"))
        .unwrap()
        .commit()
        .unwrap();
    let first = book.begin_page(CONTEXT, PAGE, &h("first")).unwrap();
    let second = book.begin_page(CONTEXT, PAGE, &h("second")).unwrap();
    first.restore_prior().unwrap();
    // Act
    second.commit().unwrap();
    // Assert
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "old")]
    );
    first.rollback().unwrap();
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "old")]
    );
}

#[test]
fn late_compensation_resolution_never_reassigns_old_desired_state() {
    for fail_compensation in [false, true] {
        // Arrange
        let (book, _) = fixture_with_writers(1, 1, 8, 2);
        let first = book.begin_page(CONTEXT, PAGE, &h("first")).unwrap();
        first.restore_prior().unwrap();
        book.begin_page(CONTEXT, PAGE, &h("second"))
            .unwrap()
            .commit()
            .unwrap();
        // Act
        if fail_compensation {
            drop(first);
        } else {
            first.rollback().unwrap();
        }
        // Assert
        assert_eq!(
            values(&book.effective(CONTEXT, PAGE).unwrap()),
            [("X-A", "second")]
        );
    }
}

#[test]
fn overlapping_failures_restore_their_own_captured_values_not_a_global_baseline() {
    // Arrange
    let (book, _) = fixture_with_writers(1, 1, 8, 2);
    let first = book.begin_context(CONTEXT, &h("first")).unwrap();
    let second = book.begin_context(CONTEXT, &h("second")).unwrap();
    first.restore_prior().unwrap();
    // Act
    second.restore_prior().unwrap();
    // Assert
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "first")]
    );
    first.rollback().unwrap();
    second.rollback().unwrap();
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "first")]
    );
}

#[test]
fn concurrent_profile_admits_exact_writer_bound_and_retirement_reclaims_all_handles() {
    // Arrange
    let (book, ledger) = fixture_with_writers(1, 1, 8, 2);
    let first = book.begin_page(CONTEXT, PAGE, &h("first")).unwrap();
    let second = book.begin_page(CONTEXT, PAGE, &h("second")).unwrap();
    let before = ledger.metrics().unwrap();
    // Act
    let extra = book.begin_context(CONTEXT, &h("extra"));
    // Assert
    assert!(matches!(extra, Err(SettingsError::Busy)));
    assert_eq!(ledger.metrics().unwrap(), before);
    book.retire_context(CONTEXT).unwrap();
    assert!(matches!(first.commit(), Err(SettingsError::Superseded)));
    assert!(matches!(second.rollback(), Err(SettingsError::Superseded)));
    book.begin_context(CONTEXT, &h("fresh"))
        .unwrap()
        .commit()
        .unwrap();
    drop(book);
    for class in ResourceClass::ALL {
        assert_eq!(ledger.metrics().unwrap().class(class).current, 0);
    }
}

#[test]
fn reconciled_native_retirement_supersedes_pending_write_without_restoring_it() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    book.begin_context(CONTEXT, &h("context"))
        .unwrap()
        .commit()
        .unwrap();
    let pending = book.begin_page(CONTEXT, PAGE, &h("page")).unwrap();
    // Act
    book.retain_live(|_| true, |_, _| false).unwrap();
    // Assert
    assert!(matches!(pending.commit(), Err(SettingsError::Superseded)));
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "context")]
    );
    book.begin_page(CONTEXT, ObjectId::new(3), &h("next"))
        .unwrap()
        .commit()
        .unwrap();
}

#[test]
fn context_reconciliation_reclaims_absent_child_uncertainty_tombstones() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    drop(book.begin_page(CONTEXT, PAGE, &h("failed")).unwrap());
    let reconcile = book.begin_context(CONTEXT, &[]).unwrap();
    // Act
    reconcile.commit().unwrap();
    // Assert
    assert_eq!(
        book.update_status(CONTEXT, PAGE).unwrap(),
        HeaderUpdateStatus::Stable
    );
    let next = book.begin_page(CONTEXT, ObjectId::new(3), &h("next"));
    assert!(
        next.is_ok(),
        "resolved absent child must not consume sparse Page capacity"
    );
}

#[test]
fn confirmed_context_reapplication_reconciles_prior_child_uncertainty() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    book.begin_page(CONTEXT, PAGE, &h("old"))
        .unwrap()
        .commit()
        .unwrap();
    drop(book.begin_page(CONTEXT, PAGE, &h("failed")).unwrap());
    assert_eq!(
        book.update_status(CONTEXT, PAGE).unwrap(),
        HeaderUpdateStatus::Indeterminate
    );
    let update = book.begin_context(CONTEXT, &h("context")).unwrap();
    // Act
    update.commit().unwrap();
    // Assert
    assert_eq!(
        book.update_status(CONTEXT, PAGE).unwrap(),
        HeaderUpdateStatus::Stable
    );
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "old")]
    );
}

#[test]
fn restoring_metadata_is_visible_while_native_compensation_is_unconfirmed() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    book.begin_context(CONTEXT, &h("old"))
        .unwrap()
        .commit()
        .unwrap();
    let pending = book.begin_context(CONTEXT, &h("new")).unwrap();
    // Act
    pending.restore_prior().unwrap();
    // Assert
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "old")]
    );
    assert!(matches!(
        book.begin_context(CONTEXT, &h("third")),
        Err(SettingsError::Busy)
    ));
    drop(pending);
    assert_eq!(
        book.update_status(CONTEXT, PAGE).unwrap(),
        HeaderUpdateStatus::Indeterminate
    );
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "old")]
    );
}

#[test]
fn restoring_new_page_uses_absent_tombstone_without_allocating_snapshot() {
    // Arrange
    let (book, ledger) = fixture(1, 1, 8);
    book.begin_context(CONTEXT, &h("context"))
        .unwrap()
        .commit()
        .unwrap();
    let pending = book.begin_page(CONTEXT, PAGE, &h("page")).unwrap();
    let before = ledger
        .metrics()
        .unwrap()
        .class(ResourceClass::LiveObject)
        .current;
    // Act
    pending.restore_prior().unwrap();
    // Assert
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::LiveObject)
            .current,
        before - 1
    );
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "context")]
    );
    assert!(book.page_headers(CONTEXT, PAGE).unwrap().is_none());
    pending.rollback().unwrap();
    let next = book.begin_page(CONTEXT, ObjectId::new(3), &h("next"));
    assert!(next.is_ok());
}

#[test]
fn compensation_acknowledgement_releases_writer_without_poisoning_old_value() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    book.begin_context(CONTEXT, &h("old"))
        .unwrap()
        .commit()
        .unwrap();
    let pending = book.begin_context(CONTEXT, &h("new")).unwrap();
    pending.restore_prior().unwrap();
    // Act
    pending.rollback().unwrap();
    // Assert
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "old")]
    );
    let next = book.begin_context(CONTEXT, &h("next"));
    assert!(next.is_ok());
}

#[test]
fn restored_update_cannot_be_committed_as_successful_proposal() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    let pending = book.begin_context(CONTEXT, &h("new")).unwrap();
    pending.restore_prior().unwrap();
    // Act
    let result = pending.commit();
    // Assert
    assert!(matches!(result, Err(SettingsError::InvalidResolution)));
    assert_eq!(
        book.update_status(CONTEXT, PAGE).unwrap(),
        HeaderUpdateStatus::Indeterminate
    );
    assert!(values(&book.effective(CONTEXT, PAGE).unwrap()).is_empty());
}

#[test]
fn proposal_is_visible_before_commit_and_prior_plan_is_still_readable() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    book.begin_context(CONTEXT, &h("old"))
        .unwrap()
        .commit()
        .unwrap();
    // Act
    let pending = book.begin_context(CONTEXT, &h("new")).unwrap();
    // Assert
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "new")]
    );
    assert_eq!(
        values(&pending.prior_effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "old")]
    );
}

#[test]
fn explicit_rollback_restores_prior_without_losing_page_override() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    book.begin_context(CONTEXT, &h("context"))
        .unwrap()
        .commit()
        .unwrap();
    book.begin_page(CONTEXT, PAGE, &h("page"))
        .unwrap()
        .commit()
        .unwrap();
    let pending = book.begin_context(CONTEXT, &h("new")).unwrap();
    assert_eq!(
        values(&pending.prior_effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "page")]
    );
    // Act
    pending.rollback().unwrap();
    // Assert
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "page")]
    );
    assert_eq!(
        values(&book.effective(CONTEXT, ObjectId::new(3)).unwrap()),
        [("X-A", "context")]
    );
}

#[test]
fn committed_clear_reveals_context_and_reclaims_sparse_page_slot() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    book.begin_context(CONTEXT, &h("context"))
        .unwrap()
        .commit()
        .unwrap();
    book.begin_page(CONTEXT, PAGE, &h("page"))
        .unwrap()
        .commit()
        .unwrap();
    let clear = book.begin_page(CONTEXT, PAGE, &[]).unwrap();
    // Act
    clear.commit().unwrap();
    // Assert
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "context")]
    );
    assert!(book.page_headers(CONTEXT, PAGE).unwrap().is_none());
    let next = book.begin_page(CONTEXT, ObjectId::new(3), &h("other"));
    assert!(next.is_ok());
}

#[test]
fn abandoned_page_update_restores_desired_headers_and_only_marks_its_page_uncertain() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    book.begin_context(CONTEXT, &h("context"))
        .unwrap()
        .commit()
        .unwrap();
    let pending = book.begin_page(CONTEXT, PAGE, &h("new")).unwrap();
    // Act
    drop(pending);
    // Assert
    assert_eq!(
        book.update_status(CONTEXT, PAGE).unwrap(),
        HeaderUpdateStatus::Indeterminate
    );
    assert_eq!(
        values(&book.effective(CONTEXT, PAGE).unwrap()),
        [("X-A", "context")]
    );
    assert!(book.page_headers(CONTEXT, PAGE).unwrap().is_none());
    assert_eq!(
        book.update_status(CONTEXT, ObjectId::new(3)).unwrap(),
        HeaderUpdateStatus::Stable
    );
    assert_eq!(
        values(&book.effective(CONTEXT, ObjectId::new(3)).unwrap()),
        [("X-A", "context")]
    );
}

#[test]
fn abandoned_context_update_marks_its_pages_uncertain_but_not_other_contexts() {
    // Arrange
    let (book, _) = fixture(2, 1, 8);
    let other = ObjectId::new(3);
    book.begin_context(other, &h("other"))
        .unwrap()
        .commit()
        .unwrap();
    let pending = book.begin_context(CONTEXT, &h("new")).unwrap();
    // Act
    drop(pending);
    // Assert
    assert_eq!(
        book.update_status(CONTEXT, PAGE).unwrap(),
        HeaderUpdateStatus::Indeterminate
    );
    assert_eq!(
        book.update_status(other, ObjectId::new(4)).unwrap(),
        HeaderUpdateStatus::Stable
    );
    assert_eq!(
        values(&book.effective(other, ObjectId::new(4)).unwrap()),
        [("X-A", "other")]
    );
}

#[test]
fn overlapping_writer_rejects_before_charging_proposed_snapshot() {
    // Arrange
    let (book, ledger) = fixture(2, 1, 8);
    let _pending = book.begin_context(CONTEXT, &h("first")).unwrap();
    let before = ledger
        .metrics()
        .unwrap()
        .class(ResourceClass::LiveObject)
        .current;
    // Act
    let result = book.begin_context(ObjectId::new(3), &h("second"));
    // Assert
    assert!(matches!(result, Err(SettingsError::Busy)));
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::LiveObject)
            .current,
        before
    );
}

#[test]
fn parent_mismatch_never_exposes_another_contexts_page_settings() {
    // Arrange
    let (book, _) = fixture(2, 1, 8);
    book.begin_page(CONTEXT, PAGE, &h("secret"))
        .unwrap()
        .commit()
        .unwrap();
    // Act
    let result = book.effective(ObjectId::new(3), PAGE);
    // Assert
    assert!(matches!(result, Err(SettingsError::ParentMismatch)));
}

#[test]
fn full_registry_does_not_admit_an_extra_settings_snapshot() {
    // Arrange
    let (book, ledger) = fixture(1, 0, 8);
    book.begin_context(CONTEXT, &h("first"))
        .unwrap()
        .commit()
        .unwrap();
    let before = ledger
        .metrics()
        .unwrap()
        .class(ResourceClass::LiveObject)
        .current;
    // Act
    let result = book.begin_context(ObjectId::new(3), &h("second"));
    // Assert
    assert!(matches!(result, Err(SettingsError::Capacity)));
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::LiveObject)
            .current,
        before
    );
}

#[test]
fn failed_header_admission_preserves_old_metadata_and_writer_availability() {
    // Arrange
    let (book, _) = fixture(1, 0, 1);
    book.begin_context(CONTEXT, &h("old"))
        .unwrap()
        .commit()
        .unwrap();
    // Act
    let result = book.begin_context(CONTEXT, &h("new"));
    // Assert
    assert!(matches!(
        result,
        Err(SettingsError::Headers(HeaderError::OwnerLimit))
    ));
    // A second failure must be admission again, not a leaked Busy writer.
    assert!(matches!(
        book.begin_context(CONTEXT, &h("new")),
        Err(SettingsError::Headers(HeaderError::OwnerLimit))
    ));
}

#[test]
fn retirement_supersedes_old_handle_without_poisoning_reused_slot() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    let old = book.begin_page(CONTEXT, PAGE, &h("old")).unwrap();
    book.retire_page(CONTEXT, PAGE).unwrap();
    book.begin_page(CONTEXT, ObjectId::new(3), &h("new"))
        .unwrap()
        .commit()
        .unwrap();
    // Act
    let result = old.commit();
    // Assert
    assert!(matches!(result, Err(SettingsError::Superseded)));
    assert_eq!(
        values(&book.effective(CONTEXT, ObjectId::new(3)).unwrap()),
        [("X-A", "new")]
    );
}

#[test]
fn context_retirement_clears_children_and_supersedes_context_update() {
    // Arrange
    let (book, _) = fixture(1, 1, 8);
    book.begin_page(CONTEXT, PAGE, &h("page"))
        .unwrap()
        .commit()
        .unwrap();
    let pending = book.begin_context(CONTEXT, &h("context")).unwrap();
    // Act
    book.retire_context(CONTEXT).unwrap();
    // Assert
    assert!(matches!(pending.rollback(), Err(SettingsError::Superseded)));
    assert!(book.page_headers(CONTEXT, PAGE).unwrap().is_none());
    assert!(values(&book.effective(CONTEXT, PAGE).unwrap()).is_empty());
}

#[test]
fn original_page_referer_survives_close_until_retained_owner_drops() {
    // Arrange
    let (book, ledger) = fixture(1, 1, 8);
    book.begin_context(
        CONTEXT,
        &[HeaderRef {
            name: "Referer",
            value: "context",
        }],
    )
    .unwrap()
    .commit()
    .unwrap();
    book.begin_page(
        CONTEXT,
        PAGE,
        &[
            HeaderRef {
                name: "Referer",
                value: "first",
            },
            HeaderRef {
                name: "referer",
                value: "last",
            },
        ],
    )
    .unwrap()
    .commit()
    .unwrap();
    let retained = book.page_headers(CONTEXT, PAGE).unwrap().unwrap();
    // Act
    book.close().unwrap();
    drop(book);
    // Assert
    assert_eq!(retained.navigation_referrer(None).unwrap(), Some("first"));
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::LiveObject)
            .current,
        1
    );
    drop(retained);
    for class in ResourceClass::ALL {
        assert_eq!(ledger.metrics().unwrap().class(class).current, 0);
    }
}

#[test]
fn close_prevents_late_commit_and_leaves_no_charges_after_handle_drop() {
    // Arrange
    let (book, ledger) = fixture(1, 1, 8);
    let pending = book.begin_context(CONTEXT, &h("context")).unwrap();
    book.close().unwrap();
    // Act
    let result = pending.commit();
    drop(book);
    // Assert
    assert!(matches!(result, Err(SettingsError::Closed)));
    for class in ResourceClass::ALL {
        assert_eq!(ledger.metrics().unwrap().class(class).current, 0);
    }
}

#[test]
fn invalid_profiles_and_ids_reject_without_storage_growth() {
    // Arrange
    let (book, ledger) = fixture(1, 1, 8);
    let before = ledger
        .metrics()
        .unwrap()
        .class(ResourceClass::BackendStateBytes)
        .current;
    // Act
    let zero = book.begin_context(ObjectId::new(0), &h("value"));
    let overflow = HeaderSettingsProfile::new(ResourceScope::extension(130, 2), usize::MAX, 1);
    let empty = HeaderSettingsProfile::new(ResourceScope::extension(130, 2), 0, 0);
    // Assert
    assert!(matches!(zero, Err(SettingsError::InvalidIdentity)));
    assert!(matches!(overflow, Err(SettingsError::InvalidLimits)));
    assert!(matches!(empty, Err(SettingsError::InvalidLimits)));
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::BackendStateBytes)
            .current,
        before
    );
}
