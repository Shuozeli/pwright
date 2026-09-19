use super::*;
use crate::{
    context_metadata::{ContextMetadata, ContextMetadataLimits},
    frame_metadata::{FrameMetadataLimits, PublicationProgress},
    frame_tree::FrameTreeLimits,
    frame_trees::FrameTrees,
};
use pwright_composition_chromium::ChromiumFrameUrlObserver;

struct Fixture {
    book: Arc<FrameMetadata>,
    _index: FrameTrees,
    factory: FrameTreeFactory,
    terminal: Option<Box<TerminalLifecycle>>,
    view: CollectionView,
    trees: Vec<FrameTree>,
    permits: Vec<Option<FrameClosePermit>>,
}

impl Fixture {
    fn new(contexts: usize, pages: usize) -> Self {
        let book = Arc::new(
            FrameMetadata::new(FrameMetadataLimits {
                entries: 128,
                url_bytes: 64,
                total_bytes: 8192,
                publications: 96,
            })
            .unwrap(),
        );
        let index = FrameTrees::new(book.clone()).unwrap();
        let factory = FrameTreeFactory::new(FrameTreeLimits {
            states: 256,
            frames_per_page: 128,
            revisions: 256,
            bytes: 1024 * 1024,
        })
        .unwrap();
        let terminal = TerminalLifecycle::reserve(&book.publications(), &factory).unwrap();
        let metadata = ContextMetadata::new(ContextMetadataLimits {
            active_contexts: 16,
            states: 256,
            conversions: 128,
            base_url_bytes: 64,
            bytes: 8192,
        })
        .unwrap();
        let mut collection = book.publications().reserve_collection(&factory).unwrap();
        for index in 0..contexts {
            let id = ObjectId::new(1000 + index as u64);
            let created = metadata.reserve_creation(None).unwrap().commit(id).unwrap();
            collection.add_context(id, created.snapshot).unwrap();
        }
        let mut trees = Vec::new();
        let mut permits = Vec::new();
        for index in 0..pages {
            let permit = book.publications().reserve_close().unwrap();
            let frame = ObjectId::new(100 + index as u64);
            let root = factory
                .frame(frame, None, book.ensure_attached(frame).unwrap())
                .unwrap();
            let tree = factory.page(root).unwrap();
            collection
                .add_page(
                    ObjectId::new(10 + index as u64),
                    ObjectId::new(1000 + (index % contexts) as u64),
                    tree.clone(),
                )
                .unwrap();
            trees.push(tree);
            permits.push(Some(permit));
        }
        let revision = book
            .publications()
            .begin_collection(collection)
            .unwrap()
            .commit()
            .unwrap();
        book.publications().fence().unwrap().publish().unwrap();
        Self {
            book,
            _index: index,
            factory,
            terminal: Some(terminal),
            view: CollectionView::full(revision),
            trees,
            permits,
        }
    }

    fn close(&mut self) {
        (*self.terminal.take().unwrap()).stage(
            &self.book.publications(),
            self.view.clone(),
            self.trees.iter().zip(self.permits.iter_mut()),
        );
    }

    fn drain(&self, first_throws: bool) -> Vec<LifecycleNotice> {
        let publications = self.book.publications();
        let mut owner = publications.fence().unwrap().try_begin().unwrap();
        let mut notices = Vec::new();
        loop {
            match owner.publish_next() {
                Ok(PublicationProgress::Published) => {}
                Ok(PublicationProgress::Complete) => break,
                Err(PublicationDrainError::NoticePending) => {
                    let inspected = owner
                        .with_next_notice(|view| {
                            assert!(self.book.state.try_lock().is_ok());
                            view.notice
                        })
                        .unwrap();
                    let committed = owner.commit_notice().unwrap();
                    assert_eq!(inspected, committed);
                    let view = publications.published_collections().unwrap().unwrap();
                    match committed {
                        LifecycleNotice::PageClose { context, page } => {
                            assert!(!view.pages().iter().any(|entry| entry.id == page));
                            assert!(view.contexts().iter().any(|entry| entry.id == context));
                            let count = notices
                                .iter()
                                .filter(|notice| {
                                    matches!(notice, LifecycleNotice::PageClose { .. })
                                })
                                .count()
                                + 1;
                            assert_eq!(
                                self.trees
                                    .iter()
                                    .filter(|tree| tree.is_closed().unwrap())
                                    .count(),
                                count
                            );
                        }
                        LifecycleNotice::ContextClose { context } => {
                            assert!(!view.contexts().iter().any(|entry| entry.id == context));
                            assert!(!view.pages().iter().any(|entry| entry.context == context));
                        }
                        LifecycleNotice::ContextPage { .. } | LifecycleNotice::Popup { .. } => {
                            panic!("terminal input cannot create a Page")
                        }
                    }
                    let delivery = if first_throws && notices.is_empty() {
                        LifecycleDelivery::ListenerThrew
                    } else {
                        LifecycleDelivery::Delivered
                    };
                    notices.push(committed);
                    owner.acknowledge_notice(delivery).unwrap();
                }
                Err(error) => panic!("unexpected publication error: {error}"),
            }
        }
        notices
    }
}

#[test]
fn maximum_terminal_members_publish_incrementally_before_disconnect_readiness() {
    // Arrange
    let mut fixture = Fixture::new(16, 16);
    let before = fixture.view.clone();
    // Act
    fixture.close();
    // Assert
    assert!(fixture.trees.iter().all(|tree| !tree.is_closed().unwrap()));
    assert!(fixture.book.publications().publish_disconnect().is_err());
    let notices = fixture.drain(false);
    assert_eq!(notices.len(), 32);
    assert_eq!(before.pages().len(), 16);
    assert_eq!(before.contexts().len(), 16);
    assert!(
        fixture
            .book
            .publications()
            .connection_is_published_open()
            .unwrap()
    );
    assert!(fixture.book.publications().publish_disconnect().unwrap());
    assert!(!fixture.book.publications().publish_disconnect().unwrap());
}

#[test]
fn a_throwing_page_listener_does_not_cancel_sibling_or_context_groups() {
    // Arrange
    let mut fixture = Fixture::new(1, 2);
    // Act
    fixture.close();
    let notices = fixture.drain(true);
    // Assert
    assert_eq!(
        notices,
        [
            LifecycleNotice::PageClose {
                context: ObjectId::new(1000),
                page: ObjectId::new(10)
            },
            LifecycleNotice::PageClose {
                context: ObjectId::new(1000),
                page: ObjectId::new(11)
            },
            LifecycleNotice::ContextClose {
                context: ObjectId::new(1000)
            },
        ]
    );
    assert!(fixture.book.publications().publish_disconnect().unwrap());
}

#[test]
fn saturated_ordinary_queue_does_not_allocate_or_block_terminal_groups() {
    // Arrange
    let mut fixture = Fixture::new(16, 16);
    // Alternate URL values because identical updates need no publication.
    for index in 0..97 {
        let url = if index % 2 == 0 {
            "queued-a"
        } else {
            "queued-b"
        };
        if fixture.book.update(ObjectId::new(100), url, None).is_err() {
            break;
        }
    }
    let capacity = fixture.book.state.lock().unwrap().pending.capacity();
    let factory = fixture.factory.metrics().unwrap();
    assert!(fixture.book.publications().reserve_close().is_err());
    // Act
    fixture.close();
    // Assert
    assert_eq!(
        fixture.book.state.lock().unwrap().pending.capacity(),
        capacity
    );
    assert!(fixture.factory.metrics().unwrap().bytes <= factory.bytes);
    assert_eq!(fixture.drain(false).len(), 32);
}

#[test]
fn old_fence_cannot_publish_terminal_callbacks_or_closed_flags() {
    // Arrange
    let mut fixture = Fixture::new(1, 2);
    let old = fixture.book.publications().fence().unwrap();
    fixture.close();
    // Act
    old.publish().unwrap();
    // Assert
    assert!(fixture.trees.iter().all(|tree| !tree.is_closed().unwrap()));
    assert_eq!(fixture.book.metrics().unwrap().pending_publications, 3);
    assert_eq!(fixture.drain(false).len(), 3);
}

#[test]
fn discard_before_terminal_staging_emits_nothing_and_releases_all_groups() {
    // Arrange
    let mut fixture = Fixture::new(1, 2);
    let baseline = fixture.factory.metrics().unwrap();
    fixture.book.publications().discard();
    // Act
    fixture.close();
    // Assert
    assert!(fixture.book.publications().is_drained().unwrap());
    assert!(fixture.factory.metrics().unwrap().bytes < baseline.bytes);
    assert!(fixture.permits.iter().all(Option::is_none));
    assert!(fixture.trees.iter().all(|tree| !tree.is_closed().unwrap()));
}

#[test]
fn partial_terminal_reservation_failure_reclaims_all_admitted_storage() {
    // Arrange
    let fixture = Fixture::new(1, 2);
    let mut held = Vec::new();
    while let Ok(permit) = fixture.book.publications().reserve_close() {
        held.push(permit);
    }
    held.pop();
    held.pop();
    let baseline = fixture.factory.metrics().unwrap();
    // Act
    let result = TerminalLifecycle::reserve(&fixture.book.publications(), &fixture.factory);
    // Assert
    assert!(matches!(result, Err(HostError::ResourceExhausted(_))));
    assert_eq!(fixture.factory.metrics().unwrap(), baseline);
    assert!(fixture.book.publications().reserve_close().is_ok());
}

#[test]
fn an_empty_terminal_view_still_reaches_disconnect_without_a_notice() {
    // Arrange
    let mut fixture = Fixture::new(0, 0);
    // Act
    fixture.close();
    // Assert
    assert!(fixture.drain(false).is_empty());
    assert!(fixture.book.publications().publish_disconnect().unwrap());
}

#[test]
fn abandoned_collection_closes_transfer_to_terminal_groups_without_early_flags() {
    // Arrange: native reconciliation retired two Pages, but its collection
    // projection failed before commit. The batch gate hid both raw close flags.
    let mut fixture = Fixture::new(1, 2);
    let old = fixture.book.publications().fence().unwrap();
    let reserved = fixture
        .book
        .publications()
        .reserve_collection(&fixture.factory)
        .unwrap();
    let batch = fixture
        .book
        .publications()
        .begin_collection(reserved)
        .unwrap();
    for (tree, permit) in fixture.trees.iter().zip(&mut fixture.permits) {
        fixture
            .book
            .publications()
            .stage_close(tree, permit)
            .unwrap();
    }
    drop(batch);
    // Act
    fixture.close();
    old.publish().unwrap();
    // Assert
    assert_eq!(fixture.book.metrics().unwrap().pending_publications, 3);
    assert!(fixture.trees.iter().all(|tree| !tree.is_closed().unwrap()));
    assert_eq!(fixture.drain(false).len(), 3);
    assert!(fixture.book.publications().publish_disconnect().unwrap());
}
