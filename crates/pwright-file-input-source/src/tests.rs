use super::*;
use pwright_core::ResourceLimits;
use pwright_file_input_model::UPLOAD_SIZE_LIMIT;
use std::io::Cursor;

fn ledger(bytes: u64, owners: u64) -> ResourceLedger {
    ResourceLedger::new(
        ResourceLimits::deny_all(4, 8)
            .with_class(ResourceClass::LiveObject, owners, owners)
            .with_class(ResourceClass::ArtifactBytes, bytes, bytes)
            .with_class(ResourceClass::BackendStateBytes, 16384, 16384),
    )
}
fn limits() -> FileInputLimits {
    FileInputLimits::new(4, 1024, 64, 64, 256).unwrap()
}
fn source(content: Content<'_>) -> PayloadSource<'_> {
    PayloadSource {
        name: "binary.txt",
        mime_type: "text/plain",
        last_modified_ms: Some(1234.5),
        content,
    }
}
fn used(ledger: &ResourceLedger, class: ResourceClass) -> u64 {
    ledger.metrics().unwrap().class(class).current
}
fn assert_released(ledger: &ResourceLedger) {
    for class in [
        ResourceClass::LiveObject,
        ResourceClass::ArtifactBytes,
        ResourceClass::BackendStateBytes,
    ] {
        assert_eq!(used(ledger, class), 0);
    }
}
fn prepare(
    ledger: &ResourceLedger,
    sources: &mut [PayloadSource<'_>],
) -> Result<PreparedPayloads, SourceError> {
    PreparedPayloads::materialize(ledger, ResourceScope::Connection, limits(), sources)
}

#[test]
fn payload_preserves_binary_metadata_and_holds_all_leases_until_drop() {
    // Arrange
    let resources = ledger(8, 1);
    let mut sources = [source(Content::Bytes(&[0, 255, 1]))];
    // Act
    let result = prepare(&resources, &mut sources).unwrap();
    // Assert
    let entry = &result.entries()[0];
    assert_eq!(entry.bytes(), &[0, 255, 1]);
    assert_eq!(entry.name(), "binary.txt");
    assert_eq!(entry.mime_type(), "text/plain");
    assert_eq!(entry.last_modified_ms(), Some(1234.5));
    assert_eq!(result.plan().source_bytes(), 3);
    assert_eq!(used(&resources, ResourceClass::ArtifactBytes), 3);
    assert_eq!(used(&resources, ResourceClass::LiveObject), 1);
    assert!(used(&resources, ResourceClass::BackendStateBytes) > 20);
    drop(result);
    assert_released(&resources);
}

#[test]
fn empty_clear_still_admits_an_owner_but_no_content_bytes() {
    // Arrange
    let resources = ledger(0, 1);
    // Act
    let result = prepare(&resources, &mut []).unwrap();
    // Assert
    assert!(result.entries().is_empty());
    assert_eq!(used(&resources, ResourceClass::LiveObject), 1);
    assert_eq!(used(&resources, ResourceClass::ArtifactBytes), 0);
    drop(result);
    assert_released(&resources);
}

#[test]
fn absent_modification_time_is_preserved_without_querying_a_clock() {
    // Arrange
    let resources = ledger(0, 1);
    let mut input = source(Content::Bytes(&[]));
    input.last_modified_ms = None;
    // Act
    let result = prepare(&resources, &mut [input]).unwrap();
    // Assert
    assert_eq!(result.entries()[0].last_modified_ms(), None);
}

#[test]
fn exact_reader_length_is_copied_without_changing_bytes() {
    // Arrange
    let resources = ledger(8, 1);
    let mut reader = Cursor::new([0, 255, 1]);
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut reader,
            byte_length: 3,
        })],
    )
    .unwrap();
    // Assert
    assert_eq!(result.entries()[0].bytes(), &[0, 255, 1]);
    assert_eq!(reader.position(), 3);
}

#[test]
fn growth_reads_only_one_probe_byte_and_releases_the_whole_batch() {
    // Arrange
    let resources = ledger(8, 1);
    let mut reader = Cursor::new([1, 2, 3, 4, 5, 6]);
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut reader,
            byte_length: 2,
        })],
    );
    // Assert
    assert!(matches!(result, Err(SourceError::SourceChanged)));
    assert_eq!(reader.position(), 3);
    assert_released(&resources);
}

#[test]
fn zero_length_file_growth_is_not_mistaken_for_an_empty_file() {
    // Arrange
    let resources = ledger(0, 1);
    let mut reader = Cursor::new([42, 43]);
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut reader,
            byte_length: 0,
        })],
    );
    // Assert
    assert!(matches!(result, Err(SourceError::SourceChanged)));
    assert_eq!(reader.position(), 1);
    assert_released(&resources);
}

#[test]
fn truncated_second_source_discards_already_materialized_first_source() {
    // Arrange
    let resources = ledger(8, 1);
    let mut reader = Cursor::new([1]);
    let mut input = [
        source(Content::Bytes(&[9, 8])),
        source(Content::Reader {
            reader: &mut reader,
            byte_length: 2,
        }),
    ];
    // Act
    let result = prepare(&resources, &mut input);
    // Assert
    assert!(matches!(result, Err(SourceError::SourceChanged)));
    assert_released(&resources);
}

struct FailingReader;
impl Read for FailingReader {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::PermissionDenied))
    }
}
#[test]
fn io_error_is_propagated_and_releases_admission() {
    // Arrange
    let resources = ledger(8, 1);
    let mut reader = FailingReader;
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut reader,
            byte_length: 1,
        })],
    );
    // Assert
    assert!(
        matches!(result, Err(SourceError::Io(error)) if error.kind() == io::ErrorKind::PermissionDenied)
    );
    assert_released(&resources);
}

#[test]
fn eof_probe_errors_are_not_swallowed() {
    // Arrange
    let resources = ledger(0, 1);
    let mut reader = FailingReader;
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut reader,
            byte_length: 0,
        })],
    );
    // Assert
    assert!(matches!(result, Err(SourceError::Io(_))));
    assert_released(&resources);
}

#[test]
fn byte_budget_failure_occurs_before_any_read() {
    // Arrange
    let resources = ledger(2, 1);
    let mut reader = Cursor::new([1, 2, 3]);
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut reader,
            byte_length: 3,
        })],
    );
    // Assert
    assert!(matches!(result, Err(SourceError::Budget(_))));
    assert_eq!(reader.position(), 0);
    assert_released(&resources);
}

#[test]
fn aggregate_byte_admission_is_atomic_before_first_read() {
    // Arrange
    let resources = ledger(3, 1);
    let mut reader = Cursor::new([1, 2]);
    let mut inputs = [
        source(Content::Reader {
            reader: &mut reader,
            byte_length: 2,
        }),
        source(Content::Bytes(&[3, 4])),
    ];
    // Act
    let result = prepare(&resources, &mut inputs);
    // Assert
    assert!(matches!(result, Err(SourceError::Budget(_))));
    assert_eq!(reader.position(), 0);
    assert_released(&resources);
}

#[test]
fn exclusive_semantic_limit_is_checked_without_read_or_allocation() {
    // Arrange
    let resources = ledger(UPLOAD_SIZE_LIMIT, 1);
    let mut reader = Cursor::new([]);
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut reader,
            byte_length: UPLOAD_SIZE_LIMIT,
        })],
    );
    // Assert
    assert!(matches!(
        result,
        Err(SourceError::Policy(FileInputError::PayloadSizeLimit))
    ));
    assert_eq!(reader.position(), 0);
    assert_released(&resources);
}

#[test]
fn metadata_limit_failure_happens_before_io() {
    // Arrange
    let resources = ledger(8, 1);
    let mut reader = Cursor::new([1]);
    let name = "x".repeat(65);
    let mut input = source(Content::Reader {
        reader: &mut reader,
        byte_length: 1,
    });
    input.name = &name;
    // Act
    let result = prepare(&resources, &mut [input]);
    // Assert
    assert!(matches!(
        result,
        Err(SourceError::Policy(FileInputError::NameByteLimit))
    ));
    assert_eq!(reader.position(), 0);
    assert_released(&resources);
}

#[test]
fn retained_owner_prevents_a_second_materialization() {
    // Arrange
    let resources = ledger(8, 1);
    let first = prepare(&resources, &mut [source(Content::Bytes(&[1]))]).unwrap();
    let mut reader = Cursor::new([2]);
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut reader,
            byte_length: 1,
        })],
    );
    // Assert
    assert!(matches!(result, Err(SourceError::Budget(_))));
    assert_eq!(reader.position(), 0);
    assert_eq!(used(&resources, ResourceClass::ArtifactBytes), 1);
    drop(first);
    assert_released(&resources);
}

#[test]
fn per_file_base64_padding_survives_materialization() {
    // Arrange
    let resources = ledger(8, 1);
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Bytes(&[1])), source(Content::Bytes(&[2]))],
    )
    .unwrap();
    // Assert
    assert_eq!(result.plan().base64_bytes(), 8);
    assert_eq!(result.entries()[1].bytes(), &[2]);
}

#[test]
fn real_file_reader_preserves_checked_in_fixture_bytes() {
    // Arrange
    let resources = ledger(128, 1);
    let mut file =
        std::fs::File::open(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixture.txt")).unwrap();
    let byte_length = file.metadata().unwrap().len();
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut file,
            byte_length,
        })],
    )
    .unwrap();
    // Assert
    assert_eq!(
        result.entries()[0].bytes(),
        include_bytes!("../tests/fixture.txt")
    );
    drop(result);
    assert_released(&resources);
}

#[test]
fn transferred_owner_retains_budget_until_receiving_worker_drops_it() {
    // Arrange
    let resources = ledger(8, 1);
    let prepared = prepare(&resources, &mut [source(Content::Bytes(&[1, 2]))]).unwrap();
    let (arrived_tx, arrived_rx) = std::sync::mpsc::sync_channel(0);
    let (release_tx, release_rx) = std::sync::mpsc::sync_channel(0);
    // Act
    let worker = std::thread::spawn(move || {
        arrived_tx.send(()).unwrap();
        release_rx.recv().unwrap();
        assert_eq!(prepared.entries()[0].bytes(), &[1, 2]);
        drop(prepared);
    });
    arrived_rx.recv().unwrap();
    // Assert
    assert_eq!(used(&resources, ResourceClass::ArtifactBytes), 2);
    assert_eq!(used(&resources, ResourceClass::LiveObject), 1);
    release_tx.send(()).unwrap();
    worker.join().unwrap();
    assert_released(&resources);
}

struct GatedReader {
    entered: Option<std::sync::mpsc::SyncSender<()>>,
    resume: std::sync::mpsc::Receiver<()>,
    bytes: Cursor<[u8; 2]>,
}
impl Read for GatedReader {
    fn read(&mut self, output: &mut [u8]) -> io::Result<usize> {
        if let Some(entered) = self.entered.take() {
            entered.send(()).unwrap();
            self.resume.recv().unwrap();
        }
        self.bytes.read(output)
    }
}

#[test]
fn abandoning_result_during_read_keeps_admission_until_worker_exits() {
    // Arrange
    let resources = ledger(8, 1);
    let worker_resources = resources.clone();
    let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(0);
    let (resume_tx, resume_rx) = std::sync::mpsc::sync_channel(0);
    let (result_tx, result_rx) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let mut reader = GatedReader {
            entered: Some(entered_tx),
            resume: resume_rx,
            bytes: Cursor::new([7, 8]),
        };
        let prepared = prepare(
            &worker_resources,
            &mut [source(Content::Reader {
                reader: &mut reader,
                byte_length: 2,
            })],
        )
        .unwrap();
        // A cancelled receiver cannot release the worker's buffer early.
        match result_tx.send(prepared) {
            Ok(()) => panic!("cancelled receiver unexpectedly accepted result"),
            Err(error) => drop(error.0),
        }
    });
    entered_rx.recv().unwrap();
    // Act
    drop(result_rx);
    // Assert
    assert_eq!(used(&resources, ResourceClass::ArtifactBytes), 2);
    assert_eq!(used(&resources, ResourceClass::LiveObject), 1);
    resume_tx.send(()).unwrap();
    worker.join().unwrap();
    assert_released(&resources);
}

struct InterruptedProbe(bool);
impl Read for InterruptedProbe {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        if std::mem::replace(&mut self.0, false) {
            Err(io::Error::from(io::ErrorKind::Interrupted))
        } else {
            Ok(0)
        }
    }
}

#[test]
fn interrupted_eof_probe_retries_without_reallocating() {
    // Arrange
    let resources = ledger(0, 1);
    let mut reader = InterruptedProbe(true);
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut reader,
            byte_length: 0,
        })],
    )
    .unwrap();
    // Assert
    assert!(result.entries()[0].bytes().is_empty());
    assert!(!reader.0);
    drop(result);
    assert_released(&resources);
}

#[test]
fn metadata_budget_failure_does_not_consume_source_bytes() {
    // Arrange
    let resources = ResourceLedger::new(
        ResourceLimits::deny_all(1, 1)
            .with_class(ResourceClass::LiveObject, 1, 1)
            .with_class(ResourceClass::ArtifactBytes, 8, 8),
    );
    let mut reader = Cursor::new([1]);
    // Act
    let result = prepare(
        &resources,
        &mut [source(Content::Reader {
            reader: &mut reader,
            byte_length: 1,
        })],
    );
    // Assert
    assert!(matches!(result, Err(SourceError::Budget(_))));
    assert_eq!(reader.position(), 0);
    assert_released(&resources);
}

#[test]
fn post_read_modification_observation_does_not_change_byte_admission() {
    // Arrange
    let resources = ledger(8, 1);
    let mut result = prepare(&resources, &mut [source(Content::Bytes(&[1, 2]))]).unwrap();
    // Act
    result.set_last_modified_ms(0, Some(-123.5)).unwrap();
    // Assert
    assert_eq!(result.entries()[0].last_modified_ms(), Some(-123.5));
    assert_eq!(result.entries()[0].bytes(), &[1, 2]);
    assert_eq!(used(&resources, ResourceClass::ArtifactBytes), 2);
    drop(result);
    assert_released(&resources);
}

#[test]
fn modification_observation_rejects_an_unknown_item() {
    // Arrange
    let resources = ledger(0, 1);
    let mut prepared = prepare(&resources, &mut []).unwrap();
    // Act
    let result = prepared.set_last_modified_ms(0, Some(123.5));
    // Assert
    assert!(matches!(result, Err(SourceError::ItemIndex)));
    assert!(prepared.entries().is_empty());
}
