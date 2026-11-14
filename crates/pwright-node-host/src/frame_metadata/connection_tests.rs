use super::*;
use crate::frame_trees::FrameTrees;
use pwright_plugin_chromium_frame_observer::ChromiumFrameOwnership;

fn fixture(page: bool) -> (Arc<FrameMetadata>, FrameTrees) {
    let book = Arc::new(
        FrameMetadata::new(FrameMetadataLimits {
            entries: 8,
            url_bytes: 64,
            total_bytes: 1024,
            publications: 4,
        })
        .unwrap(),
    );
    let trees = FrameTrees::new(book.clone()).unwrap();
    if page {
        trees
            .reconcile(
                std::iter::once(ChromiumFrameOwnership {
                    frame: ObjectId::new(2),
                    page: ObjectId::new(1),
                    parent: None,
                }),
                |_| true,
            )
            .unwrap();
    }
    (book, trees)
}

#[test]
fn native_teardown_does_not_publish_browser_disconnect() {
    // Arrange
    let (book, trees) = fixture(true);
    // Act
    trees.clear();
    let result = book.publications().publish_disconnect();
    // Assert
    assert!(matches!(result, Err(HostError::Backend(_))));
    assert!(book.publications().connection_is_published_open().unwrap());
}

#[test]
fn terminal_page_publication_is_not_the_browser_event_dispatch() {
    // Arrange
    let (book, trees) = fixture(true);
    let page = trees.get(ObjectId::new(1)).unwrap();
    trees.clear();
    // Act
    book.publications().fence().unwrap().publish().unwrap();
    // Assert
    assert!(page.is_closed().unwrap());
    assert!(book.publications().connection_is_published_open().unwrap());
}

#[test]
fn older_empty_fence_cannot_authorize_browser_disconnect() {
    // Arrange
    let (book, trees) = fixture(false);
    let old = book.publications().fence().unwrap();
    trees.clear();
    // Act
    old.publish().unwrap();
    let result = book.publications().publish_disconnect();
    // Assert
    assert!(matches!(result, Err(HostError::Backend(_))));
    assert!(book.publications().connection_is_published_open().unwrap());
}

#[test]
fn event_port_publishes_once_without_admission_and_old_fences_cannot_reopen() {
    // Arrange
    let (book, trees) = fixture(true);
    let old = book.publications().fence().unwrap();
    trees.clear();
    book.publications().fence().unwrap().publish().unwrap();
    let before = trees.metrics().unwrap();
    // Act
    let first = book.publications().publish_disconnect().unwrap();
    let repeated = book.publications().publish_disconnect().unwrap();
    old.publish().unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    // Assert
    assert!(first);
    assert!(!repeated);
    assert!(!book.publications().connection_is_published_open().unwrap());
    assert_eq!(trees.metrics().unwrap(), before);
}

#[test]
fn ordinary_page_closure_cannot_authorize_browser_disconnect() {
    // Arrange
    let (book, trees) = fixture(true);
    trees.reconcile(std::iter::empty(), |_| false).unwrap();
    book.publications().fence().unwrap().publish().unwrap();
    // Act
    let result = book.publications().publish_disconnect();
    // Assert
    assert!(matches!(result, Err(HostError::Backend(_))));
    assert!(book.publications().connection_is_published_open().unwrap());
}

#[test]
fn environment_discard_cannot_dispatch_a_ready_disconnect() {
    // Arrange
    let (book, trees) = fixture(false);
    trees.clear();
    book.publications().fence().unwrap().publish().unwrap();
    // Act
    book.publications().discard();
    let result = book.publications().publish_disconnect();
    // Assert
    assert!(matches!(result, Err(HostError::TargetClosed)));
    assert!(book.publications().connection_is_published_open().unwrap());
}

#[test]
fn discarded_terminal_fence_cannot_advance_readiness() {
    // Arrange
    let (book, trees) = fixture(true);
    trees.clear();
    let terminal = book.publications().fence().unwrap();
    book.publications().discard();
    // Act
    terminal.publish().unwrap();
    // Assert
    assert_eq!(
        book.state.lock().unwrap().connection,
        ConnectionPublication::Open
    );
}
