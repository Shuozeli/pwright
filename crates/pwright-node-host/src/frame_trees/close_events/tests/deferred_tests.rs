use super::*;
use pwright_composition_chromium::ChromiumCloseTarget;

fn all_notices() -> Vec<LifecycleNotice> {
    vec![
        LifecycleNotice::PageClose {
            context: CONTEXT,
            page: FIRST,
        },
        LifecycleNotice::PageClose {
            context: CONTEXT,
            page: SECOND,
        },
        LifecycleNotice::ContextClose { context: CONTEXT },
    ]
}

#[test]
fn deferred_context_covers_new_revision_with_later_descendants() {
    // Arrange
    let fixture = Fixture::new();
    let target = ChromiumCloseTarget::BrowserContext(CONTEXT);
    let capacity = fixture.observer.reserve_capacity(target).unwrap();
    let pages = (0..16)
        .map(|index| ObjectId::new(index * 2 + 1))
        .collect::<Vec<_>>();
    fixture
        .owner
        .reconcile(
            pages.iter().map(|page| ChromiumFrameOwnership {
                frame: ObjectId::new(page.get() + 1),
                page: *page,
                parent: None,
            }),
            |_| true,
        )
        .unwrap();
    let previous = fixture.book.published_collections().unwrap().unwrap();
    let settings = previous.revision().contexts()[0].settings.clone();
    let mut reserved = fixture
        .book
        .reserve_collection(&fixture.owner.factory)
        .unwrap();
    reserved.add_context(CONTEXT, settings).unwrap();
    for page in &pages {
        reserved
            .add_page(*page, CONTEXT, fixture.owner.get(*page).unwrap())
            .unwrap();
    }
    let latest = fixture
        .book
        .begin_collection(reserved)
        .unwrap()
        .commit_view(|revision| Ok(CollectionView::full(revision.clone())))
        .unwrap();
    {
        let mut state = fixture.owner.state.lock().unwrap();
        let collection = state.collections.as_mut().unwrap();
        collection.native = Some(latest.revision().clone());
        collection.planned = Some(latest);
    }
    fixture.book.fence().unwrap().publish().unwrap();
    let native = effects(&[CONTEXT]);
    // Act
    capacity
        .prepare(ChromiumClosePlan::new(target, &native))
        .unwrap()
        .committed(&native)
        .unwrap();
    // Assert
    let mut expected = pages
        .iter()
        .map(|page| LifecycleNotice::PageClose {
            context: CONTEXT,
            page: *page,
        })
        .collect::<Vec<_>>();
    expected.push(LifecycleNotice::ContextClose { context: CONTEXT });
    assert_eq!(fixture.drain(), expected);
    assert!(!fixture.owner.state.lock().unwrap().closed);
}

#[test]
fn deferred_context_includes_page_that_becomes_visible_after_admission() {
    // Arrange
    let fixture = Fixture::ready();
    let target = ChromiumCloseTarget::BrowserContext(CONTEXT);
    let capacity = fixture.observer.reserve_capacity(target).unwrap();
    fixture
        .owner
        .publish_ready_causes(&[CdpTopologyEffect::PageReady {
            page: FIRST,
            context: CONTEXT,
        }])
        .unwrap();
    let native = effects(&[CONTEXT]);
    // Act
    capacity
        .prepare(ChromiumClosePlan::new(target, &native))
        .unwrap()
        .committed(&native)
        .unwrap();
    // Assert
    assert_eq!(
        fixture.drain(),
        vec![
            LifecycleNotice::ContextPage {
                context: CONTEXT,
                page: FIRST
            },
            LifecycleNotice::PageClose {
                context: CONTEXT,
                page: FIRST
            },
            LifecycleNotice::ContextClose { context: CONTEXT },
        ]
    );
}

#[test]
fn deferred_capacity_accounts_exact_maximum_groups() {
    for (target, maximum) in [
        (ChromiumCloseTarget::Page(FIRST), 1),
        (ChromiumCloseTarget::BrowserContext(CONTEXT), 17),
    ] {
        // Arrange
        let fixture = Fixture::new();
        let before = fixture.owner.factory.metrics().unwrap();
        // Act
        let capacity = deferred::Capacity::new(fixture.owner.clone(), target).unwrap();
        // Assert: one envelope plus one factory storage reservation per group.
        let admitted = fixture.owner.factory.metrics().unwrap();
        assert_eq!(admitted.revisions - before.revisions, maximum + 1);
        assert_eq!(admitted.states, before.states);
        assert!(admitted.bytes > before.bytes);
        drop(capacity);
        assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
    }
}

#[test]
fn deferred_prepared_plan_drop_returns_unused_and_retained_groups() {
    // Arrange
    let fixture = Fixture::new();
    let before = fixture.owner.factory.metrics().unwrap();
    let target = ChromiumCloseTarget::BrowserContext(CONTEXT);
    let native = effects(&[CONTEXT]);
    let prepared = fixture
        .observer
        .reserve_capacity(target)
        .unwrap()
        .prepare(ChromiumClosePlan::new(target, &native))
        .unwrap();
    // Act
    drop(prepared);
    // Assert
    assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
    assert!(fixture.drain().is_empty());
}

#[test]
fn deferred_context_uses_latest_view_after_page_commit() {
    // Arrange
    let fixture = Fixture::new();
    let target = ChromiumCloseTarget::BrowserContext(CONTEXT);
    let capacity = fixture.observer.reserve_capacity(target).unwrap();
    fixture.explicit_close(&[FIRST]);
    let native = effects(&[CONTEXT]);
    // Act
    capacity
        .prepare(ChromiumClosePlan::new(target, &native))
        .unwrap()
        .committed(&native)
        .unwrap();
    // Assert
    assert_eq!(fixture.drain(), all_notices());
    assert!(!fixture.owner.state.lock().unwrap().closed);
}

#[test]
fn deferred_page_after_context_commit_does_not_duplicate_notices() {
    // Arrange
    let fixture = Fixture::new();
    let page = ChromiumCloseTarget::Page(FIRST);
    let context = ChromiumCloseTarget::BrowserContext(CONTEXT);
    let page_capacity = fixture.observer.reserve_capacity(page).unwrap();
    let context_capacity = fixture.observer.reserve_capacity(context).unwrap();
    let native_context = effects(&[CONTEXT]);
    context_capacity
        .prepare(ChromiumClosePlan::new(context, &native_context))
        .unwrap()
        .committed(&native_context)
        .unwrap();
    let native_page = effects(&[FIRST]);
    // Act
    page_capacity
        .prepare(ChromiumClosePlan::new(page, &native_page))
        .unwrap()
        .committed(&native_page)
        .unwrap();
    // Assert
    assert_eq!(fixture.drain(), all_notices());
    assert!(!fixture.owner.state.lock().unwrap().closed);
}

#[test]
fn deferred_prepared_plan_still_rejects_interleaved_commit() {
    // Arrange
    let fixture = Fixture::new();
    let target = ChromiumCloseTarget::BrowserContext(CONTEXT);
    let native = effects(&[CONTEXT]);
    let prepared = fixture
        .observer
        .reserve_capacity(target)
        .unwrap()
        .prepare(ChromiumClosePlan::new(target, &native))
        .unwrap();
    fixture.explicit_close(&[FIRST]);
    // Act
    let result = prepared.committed(&native);
    // Assert
    assert!(result.is_err());
    assert!(fixture.owner.state.lock().unwrap().closed);
    fixture.owner.clear();
    assert_eq!(fixture.drain(), all_notices());
}

#[test]
fn native_completion_after_local_page_retirement_accepts_empty_delta_once() {
    // Arrange
    let fixture = Fixture::new();
    let page = ChromiumCloseTarget::Page(FIRST);
    let capacity = fixture.observer.reserve_capacity(page).unwrap();
    fixture.explicit_close(&[FIRST]);
    // Act
    capacity
        .prepare(ChromiumClosePlan::new(page, &[]))
        .unwrap()
        .committed(&[])
        .unwrap();
    // Assert
    assert_eq!(
        fixture.drain(),
        vec![LifecycleNotice::PageClose {
            context: CONTEXT,
            page: FIRST
        }]
    );
    assert!(!fixture.owner.state.lock().unwrap().closed);
    assert!(!fixture.owner.get(SECOND).unwrap().is_closed().unwrap());
}

#[test]
fn native_page_completion_after_ancestor_retirement_accepts_empty_delta() {
    // Arrange
    let fixture = Fixture::new();
    let page = ChromiumCloseTarget::Page(FIRST);
    let capacity = fixture.observer.reserve_capacity(page).unwrap();
    fixture.explicit_close(&[CONTEXT]);
    // Act
    capacity
        .prepare(ChromiumClosePlan::new(page, &[]))
        .unwrap()
        .committed(&[])
        .unwrap();
    // Assert
    assert_eq!(fixture.drain(), all_notices());
    assert!(!fixture.owner.state.lock().unwrap().closed);
}

#[test]
fn native_context_completion_after_local_retirement_accepts_empty_delta() {
    // Arrange
    let fixture = Fixture::new();
    let context = ChromiumCloseTarget::BrowserContext(CONTEXT);
    let capacity = fixture.observer.reserve_capacity(context).unwrap();
    fixture.explicit_close(&[CONTEXT]);
    // Act
    capacity
        .prepare(ChromiumClosePlan::new(context, &[]))
        .unwrap()
        .committed(&[])
        .unwrap();
    // Assert
    assert_eq!(fixture.drain(), all_notices());
    assert!(!fixture.owner.state.lock().unwrap().closed);
}

#[test]
fn retired_target_does_not_authorize_unrelated_disposal() {
    // Arrange
    let fixture = Fixture::new();
    let page = ChromiumCloseTarget::Page(FIRST);
    let capacity = fixture.observer.reserve_capacity(page).unwrap();
    fixture.explicit_close(&[FIRST]);
    let unrelated = effects(&[SECOND]);
    // Act
    let result = capacity.prepare(ChromiumClosePlan::new(page, &unrelated));
    // Assert
    assert!(result.is_err());
    assert_eq!(
        fixture.drain(),
        vec![LifecycleNotice::PageClose {
            context: CONTEXT,
            page: FIRST
        }]
    );
    assert!(!fixture.owner.get(SECOND).unwrap().is_closed().unwrap());
}

#[test]
fn abandoned_deferred_capacity_releases_all_storage_and_groups() {
    // Arrange
    let fixture = Fixture::new();
    let before = fixture.owner.factory.metrics().unwrap();
    let capacity = fixture
        .observer
        .reserve_capacity(ChromiumCloseTarget::BrowserContext(CONTEXT))
        .unwrap();
    // Act
    drop(capacity);
    // Assert
    assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
    assert!(fixture.drain().is_empty());
    assert!(!fixture.owner.get(FIRST).unwrap().is_closed().unwrap());
}

#[test]
fn invalid_deferred_plans_release_admission_without_publication() {
    for (target, plan_target, ids) in [
        (
            ChromiumCloseTarget::Page(FIRST),
            ChromiumCloseTarget::Page(SECOND),
            vec![SECOND],
        ),
        (
            ChromiumCloseTarget::Page(FIRST),
            ChromiumCloseTarget::Page(FIRST),
            vec![],
        ),
        (
            ChromiumCloseTarget::Page(FIRST),
            ChromiumCloseTarget::Page(FIRST),
            vec![FIRST, SECOND],
        ),
        (
            ChromiumCloseTarget::BrowserContext(CONTEXT),
            ChromiumCloseTarget::BrowserContext(CONTEXT),
            vec![CONTEXT, ObjectId::new(999)],
        ),
    ] {
        // Arrange
        let fixture = Fixture::new();
        let before = fixture.owner.factory.metrics().unwrap();
        let capacity = fixture.observer.reserve_capacity(target).unwrap();
        let native = effects(&ids);
        // Act
        let result = capacity.prepare(ChromiumClosePlan::new(plan_target, &native));
        // Assert
        assert!(result.is_err());
        assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
        assert!(fixture.drain().is_empty());
        assert!(!fixture.owner.state.lock().unwrap().closed);
    }
}

#[test]
fn terminal_before_deferred_prepare_reports_closed_without_new_notices() {
    // Arrange
    let fixture = Fixture::new();
    let target = ChromiumCloseTarget::BrowserContext(CONTEXT);
    let capacity = fixture.observer.reserve_capacity(target).unwrap();
    fixture.owner.clear();
    let native = effects(&[CONTEXT]);
    // Act
    let result = capacity.prepare(ChromiumClosePlan::new(target, &native));
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::TargetClosed));
    assert_eq!(fixture.drain(), all_notices());
}

#[test]
fn partial_deferred_admission_failure_releases_earlier_groups() {
    // Arrange
    let fixture = Fixture::new();
    let mut held = Vec::new();
    while let Ok(reserved) = fixture.book.reserve_lifecycle(&fixture.owner.factory) {
        held.push(reserved);
    }
    assert!(held.len() > 3);
    held.pop();
    held.pop();
    let before = fixture.owner.factory.metrics().unwrap();
    // Act
    let result = fixture
        .observer
        .reserve_capacity(ChromiumCloseTarget::BrowserContext(CONTEXT));
    // Assert
    assert!(matches!(result, Err(error) if error.kind() == FailureKind::ResourceExhausted));
    assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
    assert_eq!(
        fixture.owner.urls.metrics().unwrap().pending_publications,
        0
    );
    assert!(!fixture.owner.get(FIRST).unwrap().is_closed().unwrap());
}

#[test]
fn deferred_prepare_succeeds_with_no_remaining_group_capacity() {
    // Arrange
    let fixture = Fixture::new();
    let target = ChromiumCloseTarget::Page(FIRST);
    let capacity = fixture.observer.reserve_capacity(target).unwrap();
    let mut held = Vec::new();
    while let Ok(reserved) = fixture.book.reserve_lifecycle(&fixture.owner.factory) {
        held.push(reserved);
    }
    let native = effects(&[FIRST]);
    // Act
    capacity
        .prepare(ChromiumClosePlan::new(target, &native))
        .unwrap()
        .committed(&native)
        .unwrap();
    // Assert
    assert_eq!(
        fixture.drain(),
        vec![LifecycleNotice::PageClose {
            context: CONTEXT,
            page: FIRST
        }]
    );
    assert!(!fixture.owner.state.lock().unwrap().closed);
}

#[test]
fn context_accepts_exact_descendant_disposal_effects() {
    // Arrange
    let fixture = Fixture::new();
    let target = ChromiumCloseTarget::BrowserContext(CONTEXT);
    let capacity = fixture.observer.reserve_capacity(target).unwrap();
    let native = effects(&[FIRST, SECOND, CONTEXT]);
    // Act
    capacity
        .prepare(ChromiumClosePlan::new(target, &native))
        .unwrap()
        .committed(&native)
        .unwrap();
    // Assert
    assert_eq!(fixture.drain(), all_notices());
}
