use super::*;

#[test]
fn hidden_ready_descriptor_binds_without_publishing_membership() {
    // Arrange
    let fixture = Fixture::new();
    fixture.popup();
    let factory = fixture.first.lookup_resources();
    let before = factory.metrics().unwrap();
    let drain = fixture
        .book
        .publications()
        .fence()
        .unwrap()
        .try_begin()
        .unwrap();
    // Act
    let binding = drain
        .with_next_notice(|value| {
            value
                .revision
                .pages()
                .iter()
                .find(|page| page.id == SECOND)
                .unwrap()
                .bind_main_frame()
        })
        .unwrap()
        .unwrap();
    // Assert
    assert_eq!(binding.node().id(), ObjectId::new(4));
    assert_eq!(
        fixture
            .book
            .publications()
            .published_collections()
            .unwrap()
            .unwrap()
            .pages()
            .len(),
        1
    );
    assert_eq!(factory.metrics().unwrap().states, before.states + 1);
    drop(binding);
    assert_eq!(factory.metrics().unwrap(), before);
}

#[test]
fn failed_binding_admission_preserves_the_uncommitted_ready_notice() {
    // Arrange
    let fixture = Fixture::new();
    fixture.popup();
    let mut held = Vec::new();
    while let Ok(binding) = fixture.first.bind_tree() {
        held.push(binding);
    }
    assert!(!held.is_empty());
    let drain = fixture
        .book
        .publications()
        .fence()
        .unwrap()
        .try_begin()
        .unwrap();
    // Act
    let result = drain
        .with_next_notice(|value| value.revision.pages()[1].bind_main_frame())
        .unwrap();
    // Assert
    assert!(result.is_err());
    assert_eq!(
        drain.with_next_notice(|value| value.notice).unwrap(),
        LifecycleNotice::ContextPage {
            context: CONTEXT,
            page: SECOND
        }
    );
    assert_eq!(
        fixture
            .book
            .publications()
            .published_collections()
            .unwrap()
            .unwrap()
            .pages()
            .len(),
        1
    );
}

#[test]
fn retained_ready_descriptor_binds_after_native_teardown() {
    // Arrange
    let fixture = Fixture::new();
    let planned = fixture.popup();
    fixture._trees.clear();
    fixture.book.clear();
    let page = planned
        .revision()
        .pages()
        .iter()
        .find(|page| page.id == SECOND)
        .unwrap();
    // Act
    let binding = page.bind_main_frame().unwrap();
    // Assert
    assert_eq!(binding.node().id(), page.main_frame.id());
    assert!(binding.node().same_identity(&page.main_frame));
    assert_eq!(fixture.book.metrics().unwrap().active_frames, 0);
    assert!(fixture._trees.get(SECOND).is_err());
}

#[test]
fn abandoned_ready_conversion_releases_both_tree_and_root_bindings() {
    // Arrange
    let fixture = Fixture::new();
    fixture.popup();
    let factory = fixture.first.lookup_resources();
    let before = factory.metrics().unwrap();
    let drain = fixture
        .book
        .publications()
        .fence()
        .unwrap()
        .try_begin()
        .unwrap();
    let bindings = drain
        .with_next_notice(|value| {
            let page = &value.revision.pages()[1];
            (
                page.bind_main_frame().unwrap(),
                page.tree.bind_tree().unwrap(),
            )
        })
        .unwrap();
    assert_eq!(factory.metrics().unwrap().states, before.states + 2);
    // Act
    drop(bindings);
    // Assert
    assert_eq!(factory.metrics().unwrap(), before);
    assert_eq!(
        drain.with_next_notice(|value| value.notice).unwrap(),
        LifecycleNotice::ContextPage {
            context: CONTEXT,
            page: SECOND
        }
    );
}
