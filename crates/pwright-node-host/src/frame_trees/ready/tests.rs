use super::*;
use crate::frame_trees::close_events::tests::Fixture;
use crate::{LifecycleDelivery, LifecycleNotice, PublicationProgress};

const CONTEXT: ObjectId = ObjectId::new(1000);
const FIRST: ObjectId = ObjectId::new(1);
const SECOND: ObjectId = ObjectId::new(3);
fn ready(page: ObjectId) -> CdpTopologyEffect {
    CdpTopologyEffect::PageReady {
        page,
        context: CONTEXT,
    }
}
fn link() -> CdpTopologyEffect {
    CdpTopologyEffect::PopupLinked {
        opener: FIRST,
        popup: SECOND,
    }
}

#[test]
fn ready_cause_uses_admitted_storage_and_defers_membership_until_notice_commit() {
    // Arrange
    let fixture = Fixture::ready();
    let before = fixture.owner.factory.metrics().unwrap();
    // Act
    fixture.owner.publish_ready_causes(&[ready(FIRST)]).unwrap();
    // Assert
    assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
    assert_eq!(
        fixture
            .book
            .published_collections()
            .unwrap()
            .unwrap()
            .pages()
            .len(),
        0
    );
    assert_eq!(
        fixture.drain(),
        vec![LifecycleNotice::ContextPage {
            context: CONTEXT,
            page: FIRST
        }]
    );
    assert_eq!(
        fixture
            .book
            .published_collections()
            .unwrap()
            .unwrap()
            .pages()
            .len(),
        1
    );
}

#[test]
fn internal_page_ready_keeps_native_owner_without_public_page_or_event() {
    // Arrange
    let fixture = Fixture::ready();
    fixture
        .owner
        .state
        .lock()
        .unwrap()
        .pages
        .iter_mut()
        .find(|entry| entry.id == FIRST)
        .unwrap()
        .internal = true;

    // Act
    fixture.owner.publish_ready_causes(&[ready(FIRST)]).unwrap();

    // Assert
    assert!(fixture.drain().is_empty());
    let published = fixture.book.published_collections().unwrap().unwrap();
    assert_eq!(published.pages().len(), 0);
    assert!(
        published
            .revision()
            .pages()
            .iter()
            .any(|page| page.id == FIRST)
    );
    assert!(fixture.owner.is_internal_page(FIRST).unwrap());
    assert!(!fixture.owner.is_internal_page(SECOND).unwrap());
}

#[test]
fn hidden_default_context_ready_and_popup_have_no_public_delivery() {
    // Arrange: model the internal Context by publishing an empty projection
    // while its Page remains in native frame ownership.
    let fixture = Fixture::ready();
    let reserved = fixture
        .book
        .reserve_collection(&fixture.owner.factory)
        .unwrap();
    let planned = fixture
        .book
        .begin_collection(reserved)
        .unwrap()
        .commit_view(|revision| Ok(CollectionView::hidden(revision.clone())))
        .unwrap();
    {
        let mut state = fixture.owner.state.lock().unwrap();
        let collection = state.collections.as_mut().unwrap();
        collection.native = Some(planned.revision().clone());
        collection.planned = Some(planned);
    }
    fixture.book.fence().unwrap().publish().unwrap();

    // Act
    fixture
        .owner
        .publish_ready_causes(&[ready(FIRST), link()])
        .unwrap();

    // Assert
    assert!(fixture.drain().is_empty());
    let published = fixture.book.published_collections().unwrap().unwrap();
    assert!(published.contexts().is_empty());
    assert!(published.pages().is_empty());
}

#[test]
fn ready_and_link_keep_dependent_notification_order() {
    // Arrange
    let fixture = Fixture::ready();
    fixture.owner.publish_ready_causes(&[ready(FIRST)]).unwrap();
    fixture.drain();
    // Act
    fixture
        .owner
        .publish_ready_causes(&[ready(SECOND), link()])
        .unwrap();
    // Assert
    assert_eq!(
        fixture.drain(),
        vec![
            LifecycleNotice::ContextPage {
                context: CONTEXT,
                page: SECOND
            },
            LifecycleNotice::Popup {
                opener: FIRST,
                popup: SECOND
            },
        ]
    );
}

#[test]
fn deferred_link_uses_earlier_creation_acknowledgement_without_repeating_ready() {
    // Arrange: popup ready before opener, as allowed by the native topology.
    let fixture = Fixture::ready();
    fixture
        .owner
        .publish_ready_causes(&[ready(SECOND)])
        .unwrap();
    fixture.drain();
    // Act
    fixture
        .owner
        .publish_ready_causes(&[ready(FIRST), link()])
        .unwrap();
    // Assert
    assert_eq!(
        fixture.drain(),
        vec![
            LifecycleNotice::ContextPage {
                context: CONTEXT,
                page: FIRST
            },
            LifecycleNotice::Popup {
                opener: FIRST,
                popup: SECOND
            },
        ]
    );
}

#[test]
fn throwing_ready_listener_suppresses_a_later_link_but_not_the_opener_ready() {
    // Arrange
    let fixture = Fixture::ready();
    fixture
        .owner
        .publish_ready_causes(&[ready(SECOND)])
        .unwrap();
    let mut drain = fixture.book.fence().unwrap().try_begin().unwrap();
    drain.commit_notice().unwrap();
    drain
        .acknowledge_notice(LifecycleDelivery::ListenerThrew)
        .unwrap();
    drop(drain);
    // Act
    fixture
        .owner
        .publish_ready_causes(&[ready(FIRST), link()])
        .unwrap();
    // Assert
    assert_eq!(
        fixture.drain(),
        vec![LifecycleNotice::ContextPage {
            context: CONTEXT,
            page: FIRST
        }]
    );
}

#[test]
fn interrupted_ready_dispatch_suppresses_a_later_dependent_popup() {
    // Arrange
    let fixture = Fixture::ready();
    fixture.owner.publish_ready_causes(&[ready(FIRST)]).unwrap();
    fixture.drain();
    fixture
        .owner
        .publish_ready_causes(&[ready(SECOND)])
        .unwrap();
    let mut drain = fixture.book.fence().unwrap().try_begin().unwrap();
    drain.commit_notice().unwrap();
    drop(drain);
    // Act
    fixture.owner.publish_ready_causes(&[link()]).unwrap();
    // Assert
    assert!(fixture.drain().is_empty());
}

#[test]
fn duplicate_ready_cause_does_not_repeat_notification_or_spend_storage() {
    // Arrange
    let fixture = Fixture::ready();
    fixture.owner.publish_ready_causes(&[ready(FIRST)]).unwrap();
    let before = fixture.owner.factory.metrics().unwrap();
    // Act
    fixture.owner.publish_ready_causes(&[ready(FIRST)]).unwrap();
    // Assert
    assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
    assert_eq!(fixture.drain().len(), 1);
}

#[test]
fn abandoned_creation_restores_ready_pool_capacity() {
    // Arrange
    let fixture = Fixture::ready();
    fixture.owner.publish_ready_causes(&[ready(FIRST)]).unwrap();
    let before = fixture.owner.factory.metrics().unwrap();
    let creation = fixture.owner.reserve_creation_collection().unwrap();
    // Act
    drop(creation);
    // Assert
    assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
}

#[test]
fn already_admitted_ready_succeeds_when_fresh_group_capacity_is_exhausted() {
    // Arrange
    let fixture = Fixture::ready();
    let mut held = Vec::new();
    while let Ok(group) = fixture.book.reserve_lifecycle(&fixture.owner.factory) {
        held.push(group);
    }
    let before = fixture.owner.factory.metrics().unwrap();
    // Act
    fixture.owner.publish_ready_causes(&[ready(FIRST)]).unwrap();
    // Assert
    assert_eq!(fixture.owner.factory.metrics().unwrap(), before);
    assert_eq!(fixture.drain().len(), 1);
}

#[test]
fn terminal_handoff_preserves_ready_then_close_order_and_hides_unready_pages() {
    // Arrange
    let fixture = Fixture::ready();
    fixture.owner.publish_ready_causes(&[ready(FIRST)]).unwrap();
    // Act
    fixture.owner.clear();
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
fn throwing_ready_skips_following_popup_and_keeps_the_committed_page_visible() {
    // Arrange
    let fixture = Fixture::ready();
    fixture.owner.publish_ready_causes(&[ready(FIRST)]).unwrap();
    fixture.drain();
    fixture
        .owner
        .publish_ready_causes(&[ready(SECOND), link()])
        .unwrap();
    let mut drain = fixture.book.fence().unwrap().try_begin().unwrap();
    drain.commit_notice().unwrap();
    // Act
    drain
        .acknowledge_notice(LifecycleDelivery::ListenerThrew)
        .unwrap();
    // Assert
    assert_eq!(
        drain.publish_next().unwrap(),
        PublicationProgress::Published
    );
    assert_eq!(drain.publish_next().unwrap(), PublicationProgress::Complete);
    assert_eq!(
        fixture
            .book
            .published_collections()
            .unwrap()
            .unwrap()
            .pages()
            .len(),
        2
    );
}
