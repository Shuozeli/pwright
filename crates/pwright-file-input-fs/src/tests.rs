use super::*;
use pwright_core::ResourceLimits;
use std::{cell::Cell, io::Cursor};

struct FakeFs {
    stats: Cell<usize>,
    opens: Cell<usize>,
    length: u64,
}
impl FakeFs {
    fn new() -> Self {
        Self {
            stats: Cell::new(0),
            opens: Cell::new(0),
            length: 2,
        }
    }
}
impl FileSystem for FakeFs {
    type Reader = Cursor<&'static [u8]>;
    fn stat(&self, path: &Path) -> Result<FileObservation, FsError> {
        self.stats.set(self.stats.get() + 1);
        if path.ends_with("missing") {
            return Err(std::io::Error::from(std::io::ErrorKind::NotFound).into());
        }
        let kind = if path.ends_with("dir") || path.ends_with("dir2") {
            PathKind::Directory
        } else {
            PathKind::File {
                byte_length: self.length,
            }
        };
        Ok(FileObservation {
            kind,
            modified_ms: if self.opens.get() > 0 { 99.5 } else { 10.0 },
        })
    }
    fn open(&self, _: &Path) -> Result<Self::Reader, FsError> {
        self.opens.set(self.opens.get() + 1);
        Ok(Cursor::new(&[0, 255]))
    }
}
fn resources(objects: u64) -> ResourceLedger {
    ResourceLedger::new(
        ResourceLimits::deny_all(4, 8)
            .with_class(ResourceClass::LiveObject, objects, objects)
            .with_class(ResourceClass::BackendStateBytes, 100000, 100000)
            .with_class(ResourceClass::ArtifactBytes, 100, 100),
    )
}
fn limits() -> FileInputLimits {
    FileInputLimits::new(4, 512, 128, 128, 2048).unwrap()
}
fn prepare(
    fs: &FakeFs,
    ledger: &ResourceLedger,
    paths: &[&str],
    location: BrowserLocation,
) -> Result<PreparedPaths<Cursor<&'static [u8]>>, FsError> {
    PreparedPaths::prepare(
        ledger,
        ResourceScope::Connection,
        limits(),
        paths,
        location,
        PathServices {
            base: Path::new("/base"),
            filesystem: fs,
            mime: &PinnedMime,
        },
    )
}
fn released(ledger: &ResourceLedger) {
    for class in [
        ResourceClass::LiveObject,
        ResourceClass::BackendStateBytes,
        ResourceClass::ArtifactBytes,
    ] {
        assert_eq!(ledger.metrics().unwrap().class(class).current, 0);
    }
}

#[test]
fn relative_path_is_retained_in_lexically_resolved_form() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(8);
    // Act
    let paths = prepare(&fs, &ledger, &["x/../input.txt"], BrowserLocation::Separate).unwrap();
    // Assert
    assert_eq!(paths.paths().next(), Some(Path::new("/base/input.txt")));
    assert_eq!(paths.plan().route(), UploadRoute::ReadFiles);
    assert_eq!(fs.stats.get(), 2);
    assert_eq!(fs.opens.get(), 0);
    drop(paths);
    released(&ledger);
}

#[test]
fn parent_traversal_cannot_pop_absolute_root() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(8);
    // Act
    let paths = prepare(
        &fs,
        &ledger,
        &["/../../input.txt"],
        BrowserLocation::Colocated,
    )
    .unwrap();
    // Assert
    assert_eq!(paths.paths().next(), Some(Path::new("/input.txt")));
}

#[test]
fn file_materialization_preserves_basename_mime_and_post_read_mtime() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(8);
    let paths = prepare(&fs, &ledger, &["input.TXT"], BrowserLocation::Separate).unwrap();
    // Act
    let PreparedInput::Payloads(result) = paths.materialize(&fs).unwrap() else {
        panic!("expected payloads")
    };
    // Assert
    let entry = &result.entries()[0];
    assert_eq!(entry.name(), "input.TXT");
    assert_eq!(entry.mime_type(), "text/plain");
    assert_eq!(entry.bytes(), &[0, 255]);
    assert_eq!(entry.last_modified_ms(), Some(99.5));
    assert_eq!(fs.stats.get(), 3);
    assert_eq!(fs.opens.get(), 1);
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::LiveObject)
            .current,
        1
    );
    drop(result);
    released(&ledger);
}

#[test]
fn unknown_extension_uses_octet_stream() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(8);
    let paths = prepare(
        &fs,
        &ledger,
        &["a.unknown-pwright"],
        BrowserLocation::Separate,
    )
    .unwrap();
    // Act
    let PreparedInput::Payloads(result) = paths.materialize(&fs).unwrap() else {
        panic!("expected payloads")
    };
    // Assert
    assert_eq!(result.entries()[0].mime_type(), "application/octet-stream");
}

#[test]
fn separate_browser_directory_retains_native_path_without_open() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(8);
    let paths = prepare(&fs, &ledger, &["dir"], BrowserLocation::Separate).unwrap();
    // Act
    let PreparedInput::Native(result) = paths.materialize(&fs).unwrap() else {
        panic!("expected directory")
    };
    // Assert
    assert_eq!(result.plan().route(), UploadRoute::NativeDirectory);
    assert_eq!(fs.opens.get(), 0);
    assert_eq!(
        ledger
            .metrics()
            .unwrap()
            .class(ResourceClass::ArtifactBytes)
            .current,
        0
    );
    drop(result);
    released(&ledger);
}

#[test]
fn colocated_large_file_is_not_read_or_subject_to_transfer_limit() {
    // Arrange
    let mut fs = FakeFs::new();
    fs.length = u64::MAX;
    let ledger = resources(8);
    let paths = prepare(&fs, &ledger, &["huge.txt"], BrowserLocation::Colocated).unwrap();
    // Act
    let PreparedInput::Native(result) = paths.materialize(&fs).unwrap() else {
        panic!("expected native files")
    };
    // Assert
    assert_eq!(result.plan().route(), UploadRoute::NativePaths);
    assert_eq!(fs.opens.get(), 0);
}

#[test]
fn remote_size_error_precedes_open_and_releases_paths() {
    // Arrange
    let mut fs = FakeFs::new();
    fs.length = pwright_file_input_model::UPLOAD_SIZE_LIMIT;
    let ledger = resources(8);
    // Act
    let result = prepare(&fs, &ledger, &["huge.txt"], BrowserLocation::Separate);
    // Assert
    assert!(matches!(
        result,
        Err(FsError::Source(SourceError::Policy(
            FileInputError::RemoteFileSizeLimit
        )))
    ));
    assert_eq!(fs.opens.get(), 0);
    released(&ledger);
}

#[test]
fn mixed_directory_and_file_reject_before_open() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(8);
    // Act
    let result = prepare(
        &fs,
        &ledger,
        &["dir", "input.txt"],
        BrowserLocation::Separate,
    );
    // Assert
    assert!(matches!(
        result,
        Err(FsError::Source(SourceError::Policy(
            FileInputError::MixedFilesAndDirectory
        )))
    ));
    assert_eq!(fs.opens.get(), 0);
    released(&ledger);
}

#[test]
fn second_directory_rejects_without_opening_either() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(8);
    // Act
    let result = prepare(&fs, &ledger, &["dir", "dir2"], BrowserLocation::Separate);
    // Assert
    assert!(matches!(
        result,
        Err(FsError::Source(SourceError::Policy(
            FileInputError::MultipleDirectories
        )))
    ));
    assert_eq!(fs.opens.get(), 0);
    released(&ledger);
}

#[test]
fn missing_path_fails_before_materialization() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(8);
    // Act
    let result = prepare(&fs, &ledger, &["missing"], BrowserLocation::Separate);
    // Assert
    assert!(
        matches!(result, Err(FsError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound)
    );
    released(&ledger);
}

#[test]
fn descriptor_and_path_admission_precedes_stat() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(1);
    // Act
    let result = prepare(&fs, &ledger, &["input.txt"], BrowserLocation::Separate);
    // Assert
    assert!(matches!(
        result,
        Err(FsError::Source(SourceError::Budget(_)))
    ));
    assert_eq!(fs.stats.get(), 0);
    assert_eq!(fs.opens.get(), 0);
    released(&ledger);
}

#[test]
fn empty_input_clears_without_filesystem_access() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(8);
    let paths = prepare(&fs, &ledger, &[], BrowserLocation::Separate).unwrap();
    // Act
    let PreparedInput::Payloads(result) = paths.materialize(&fs).unwrap() else {
        panic!("expected empty payloads")
    };
    // Assert
    assert!(result.entries().is_empty());
    assert_eq!(fs.stats.get(), 0);
    assert_eq!(fs.opens.get(), 0);
    drop(result);
    released(&ledger);
}

#[test]
fn real_file_passes_through_production_provider() {
    // Arrange
    let ledger = resources(8);
    let fs = SystemFileSystem;
    let paths = PreparedPaths::prepare(
        &ledger,
        ResourceScope::Connection,
        limits(),
        &["../pwright-file-input-source/tests/fixture.txt"],
        BrowserLocation::Separate,
        PathServices {
            base: Path::new(env!("CARGO_MANIFEST_DIR")),
            filesystem: &fs,
            mime: &PinnedMime,
        },
    )
    .unwrap();
    // Act
    let PreparedInput::Payloads(result) = paths.materialize(&fs).unwrap() else {
        panic!("expected file bytes")
    };
    // Assert
    assert_eq!(
        result.entries()[0].bytes(),
        include_bytes!("../../pwright-file-input-source/tests/fixture.txt")
    );
    assert_eq!(result.entries()[0].name(), "fixture.txt");
    assert_eq!(result.entries()[0].mime_type(), "text/plain");
    assert!(result.entries()[0].last_modified_ms().unwrap().is_finite());
    drop(result);
    released(&ledger);
}

#[test]
fn pinned_mime_preserves_bare_extension_dotfile_and_path_rules() {
    // Arrange
    let cases = [
        ("txt", Some("text/plain")),
        (".txt", Some("text/plain")),
        ("a/.txt", None),
        ("a/txt", None),
        ("x\\a.TXT", Some("text/plain")),
        ("a.unknown-pwright", None),
        ("a.js", Some("application/javascript")),
    ];
    // Act
    let actual: Vec<_> = cases
        .iter()
        .map(|(name, _)| PinnedMime.lookup(name))
        .collect();
    // Assert
    assert_eq!(
        actual,
        cases.iter().map(|(_, value)| *value).collect::<Vec<_>>()
    );
}

#[test]
fn second_directory_error_precedes_a_later_missing_path() {
    // Arrange
    let fs = FakeFs::new();
    let ledger = resources(8);
    // Act
    let result = prepare(
        &fs,
        &ledger,
        &["dir", "dir2", "missing"],
        BrowserLocation::Separate,
    );
    // Assert
    assert!(matches!(
        result,
        Err(FsError::Source(SourceError::Policy(
            FileInputError::MultipleDirectories
        )))
    ));
    assert_eq!(fs.stats.get(), 2);
    released(&ledger);
}

#[test]
fn real_missing_component_is_not_erased_by_lexical_parent_normalization() {
    // Arrange
    let ledger = resources(8);
    // Act
    let result = PreparedPaths::prepare(
        &ledger,
        ResourceScope::Connection,
        limits(),
        &["missing-node143/../Cargo.toml"],
        BrowserLocation::Separate,
        PathServices {
            base: Path::new(env!("CARGO_MANIFEST_DIR")),
            filesystem: &SystemFileSystem,
            mime: &PinnedMime,
        },
    );
    // Assert
    assert!(
        matches!(result, Err(FsError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound)
    );
    released(&ledger);
}

#[test]
fn production_provider_rejects_a_device_without_reading_it() {
    // Arrange
    let fs = SystemFileSystem;
    // Act
    let result = fs.open(Path::new("/dev/null"));
    // Assert
    assert!(matches!(result, Err(FsError::UnsupportedFile)));
}

#[test]
fn growth_after_size_observation_does_not_expand_the_buffer() {
    // Arrange
    let mut fs = FakeFs::new();
    fs.length = 1;
    let ledger = resources(8);
    let paths = prepare(&fs, &ledger, &["input.txt"], BrowserLocation::Separate).unwrap();
    // Act
    let result = paths.materialize(&fs);
    // Assert
    assert!(matches!(
        result,
        Err(FsError::Source(SourceError::SourceChanged))
    ));
    released(&ledger);
}

struct TrackedReader(std::sync::Arc<std::sync::atomic::AtomicUsize>);
impl Read for TrackedReader {
    fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}
impl Drop for TrackedReader {
    fn drop(&mut self) {
        self.0.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
    }
}
struct FailingOpenFs {
    open: Cell<usize>,
    live: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}
impl FileSystem for FailingOpenFs {
    type Reader = TrackedReader;
    fn stat(&self, _: &Path) -> Result<FileObservation, FsError> {
        Ok(FileObservation {
            kind: PathKind::File { byte_length: 0 },
            modified_ms: 1.0,
        })
    }
    fn open(&self, _: &Path) -> Result<Self::Reader, FsError> {
        let index = self.open.get();
        self.open.set(index + 1);
        if index == 1 {
            return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into());
        }
        self.live.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(TrackedReader(self.live.clone()))
    }
}

#[test]
fn second_open_failure_closes_first_reader_and_releases_all_path_owners() {
    // Arrange
    let fs = FailingOpenFs {
        open: Cell::new(0),
        live: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
    };
    let ledger = resources(8);
    let paths = PreparedPaths::prepare(
        &ledger,
        ResourceScope::Connection,
        limits(),
        &["first.txt", "second.txt"],
        BrowserLocation::Separate,
        PathServices {
            base: Path::new("/base"),
            filesystem: &fs,
            mime: &PinnedMime,
        },
    )
    .unwrap();
    // Act
    let result = paths.materialize(&fs);
    // Assert
    assert!(
        matches!(result, Err(FsError::Io(error)) if error.kind() == std::io::ErrorKind::PermissionDenied)
    );
    assert_eq!(fs.live.load(std::sync::atomic::Ordering::SeqCst), 0);
    released(&ledger);
}
