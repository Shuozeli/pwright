use super::*;
use crate::frame_metadata::{FrameMetadata, FrameMetadataLimits, FrameNameLimits};
use crate::frame_tree::{FrameTreeFactory, FrameTreeLimits};
use pwright_composition_chromium::ChromiumFrameUrlObserver;
use pwright_core::ObjectId;
use std::sync::Arc;

struct Fixture {
    book: FrameMetadata,
    resources: FrameTreeFactory,
    tree: FrameTree,
    root: FrameNode,
    child: FrameNode,
}

#[test]
fn selected_path_visits_only_ancestors_and_stops_at_cached_parent() {
    // Arrange
    let fixture = Fixture::new(8);
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    let before = fixture.resources.metrics().unwrap();
    let mut visited = Vec::new();
    // Act
    lookup
        .visit_path(fixture.child.id(), |node| {
            visited.push(node.id().get());
            Ok::<_, ()>(node.id() != fixture.root.id())
        })
        .unwrap();
    // Assert
    assert_eq!(visited, [3, 1]);
    assert_eq!(fixture.resources.metrics().unwrap(), before);
}

#[test]
fn pinned_selection_can_bind_after_reentrant_tree_publication_retires_it() {
    // Arrange
    let fixture = Fixture::new(8);
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    fixture
        .tree
        .stage(std::slice::from_ref(&fixture.root))
        .unwrap()
        .publish()
        .unwrap();
    let before = fixture.resources.metrics().unwrap();
    // Act
    let binding = lookup.bind(fixture.child.id()).unwrap();
    // Assert
    assert!(binding.node().same_identity(&fixture.child));
    assert!(binding.node().is_detached());
    assert_eq!(
        fixture.resources.metrics().unwrap().states,
        before.states + 1
    );
    drop(binding);
    assert_eq!(fixture.resources.metrics().unwrap(), before);
}

#[test]
fn selected_path_preserves_visitor_failure_without_visiting_parent() {
    // Arrange
    let fixture = Fixture::new(8);
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    let mut calls = 0;
    // Act
    let result = lookup.visit_path(fixture.child.id(), |_| {
        calls += 1;
        Err::<bool, _>(73)
    });
    // Assert
    assert!(matches!(result, Err(LookupError::Matcher(73))));
    assert_eq!(calls, 1);
}

impl Fixture {
    fn new(revisions: usize) -> Self {
        let book = FrameMetadata::new(FrameMetadataLimits {
            entries: 4,
            url_bytes: 64,
            total_bytes: 1024,
            publications: 0,
        })
        .unwrap()
        .with_names(FrameNameLimits {
            name_bytes: 16,
            total_bytes: 1024,
        })
        .unwrap();
        let resources = FrameTreeFactory::new(FrameTreeLimits {
            states: 8,
            frames_per_page: 4,
            revisions,
            bytes: 32 * 1024,
        })
        .unwrap();
        let make = |id, parent: Option<u64>, name, url| {
            book.update_navigation(ObjectId::new(id), url, None, name)
                .unwrap();
            resources
                .frame(
                    ObjectId::new(id),
                    parent.map(ObjectId::new),
                    book.get(ObjectId::new(id)).unwrap(),
                )
                .unwrap()
        };
        let root = make(1, None::<u64>, "root", "about:blank");
        let child = make(3, Some(1), "duplicate", "https://child.test/");
        let last = make(2, Some(1), "duplicate", "https://last.test/");
        let tree = resources.page(root.clone()).unwrap();
        tree.stage(&[root.clone(), child.clone(), last])
            .unwrap()
            .publish()
            .unwrap();
        Self {
            book,
            resources,
            tree,
            root,
            child,
        }
    }
}

#[test]
fn exact_name_returns_the_first_published_identity_without_sorting_or_binding() {
    // Arrange
    let fixture = Fixture::new(8);
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    let before = fixture.resources.metrics().unwrap();
    // Act
    let result = lookup.by_name("duplicate").unwrap().unwrap();
    // Assert
    assert!(result.same_identity(&fixture.child));
    assert_eq!(fixture.resources.metrics().unwrap(), before);
}

#[test]
fn missing_name_returns_none_without_allocating_a_binding() {
    // Arrange
    let fixture = Fixture::new(8);
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    let before = fixture.resources.metrics().unwrap();
    // Act
    let result = lookup.by_name("absent").unwrap();
    // Assert
    assert!(result.is_none());
    assert_eq!(fixture.resources.metrics().unwrap(), before);
}

#[test]
fn url_matcher_state_is_preserved_and_stops_at_the_first_match() {
    // Arrange
    let fixture = Fixture::new(8);
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    let mut calls = Vec::new();
    // Act
    let result = lookup
        .by_url::<()>(|url| {
            calls.push(url.to_owned());
            Ok(calls.len() == 2)
        })
        .unwrap()
        .unwrap();
    // Assert
    assert!(result.same_identity(&fixture.child));
    assert_eq!(calls, ["about:blank", "https://child.test/"]);
}

#[test]
fn matcher_error_identity_is_preserved_and_no_later_frame_is_visited() {
    // Arrange
    let fixture = Fixture::new(8);
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    let failure = Arc::new("original foreign exception");
    let mut calls = 0;
    // Act
    let result = lookup.by_url::<Arc<&str>>(|_| {
        calls += 1;
        Err(failure.clone())
    });
    // Assert
    assert!(matches!(result, Err(LookupError::Matcher(error)) if Arc::ptr_eq(&error, &failure)));
    assert_eq!(calls, 1);
}

#[test]
fn callbacks_can_publish_without_mutating_pinned_membership_or_deadlocking() {
    // Arrange
    let fixture = Fixture::new(8);
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    // Act
    let result = lookup
        .by_url::<HostError>(|url| {
            if url == "about:blank" {
                fixture
                    .tree
                    .stage(std::slice::from_ref(&fixture.root))?
                    .publish()?;
                Ok(false)
            } else {
                Ok(true)
            }
        })
        .unwrap()
        .unwrap();
    // Assert
    assert!(result.same_identity(&fixture.child));
    assert!(result.is_detached());
    assert_eq!(fixture.tree.frames().unwrap().len(), 1);
}

#[test]
fn url_conversion_can_publish_a_new_value_while_borrowing_the_old_revision() {
    // Arrange
    let fixture = Fixture::new(8);
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    // Act
    let result = lookup
        .by_url::<()>(|url| {
            fixture
                .book
                .update(ObjectId::new(1), "changed", None)
                .unwrap();
            Ok(url == "about:blank")
        })
        .unwrap()
        .unwrap();
    // Assert
    assert!(result.same_identity(&fixture.root));
    assert_eq!(fixture.root.url().unwrap(), "changed");
}

#[test]
fn closed_tree_lookup_requires_no_live_browser() {
    // Arrange
    let fixture = Fixture::new(8);
    fixture.tree.close_for_teardown();
    // Act
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    // Assert
    assert!(
        lookup
            .by_name("duplicate")
            .unwrap()
            .unwrap()
            .same_identity(&fixture.child)
    );
}

#[test]
fn read_admission_blocks_nested_lookup_and_releases_on_drop() {
    // Arrange
    let fixture = Fixture::new(2);
    let before = fixture.resources.metrics().unwrap();
    let lookup = FrameLookup::new(&fixture.tree).unwrap();
    let admitted = fixture.resources.metrics().unwrap();
    // Act
    let nested = FrameLookup::new(&fixture.tree);
    // Assert
    assert!(matches!(nested, Err(HostError::ResourceExhausted(_))));
    assert_eq!(fixture.resources.metrics().unwrap(), admitted);
    drop(lookup);
    assert_eq!(fixture.resources.metrics().unwrap(), before);
}

#[test]
fn lookup_outlives_the_metadata_book_and_tree_without_retaining_their_owners() {
    // Arrange
    let Fixture {
        book,
        resources,
        tree,
        root,
        child,
    } = Fixture::new(8);
    let lookup = FrameLookup::new(&tree).unwrap();
    // Act
    drop((book, tree, root, child));
    // Assert
    assert_eq!(
        lookup.by_name("duplicate").unwrap().unwrap().id(),
        ObjectId::new(3)
    );
    assert_eq!(resources.metrics().unwrap().states, 3);
    drop(lookup);
    assert_eq!(resources.metrics().unwrap().states, 0);
    assert_eq!(resources.metrics().unwrap().revisions, 0);
    assert_eq!(resources.metrics().unwrap().bytes, 0);
}
