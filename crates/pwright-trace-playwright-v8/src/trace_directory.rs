//! Explicit caller-owned trace directory preparation for opt-in browser launch.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceDirectoryLimits {
    pub max_path_bytes: usize,
    pub max_components: usize,
}

impl TraceDirectoryLimits {
    pub const fn new(max_path_bytes: usize, max_components: usize) -> Self {
        Self {
            max_path_bytes,
            max_components,
        }
    }

    fn validate(self) -> Result<(), TraceDirectoryError> {
        if self.max_path_bytes == 0 {
            return Err(TraceDirectoryError::ZeroLimit("path bytes"));
        }
        if self.max_components == 0 {
            return Err(TraceDirectoryError::ZeroLimit("path components"));
        }
        Ok(())
    }

    fn check_path(self, path: &Path) -> Result<(), TraceDirectoryError> {
        let bytes = path.as_os_str().as_encoded_bytes().len();
        if bytes > self.max_path_bytes {
            return Err(TraceDirectoryError::PathBytesExceeded {
                actual: bytes,
                limit: self.max_path_bytes,
            });
        }
        let components = path.components().count();
        if components > self.max_components {
            return Err(TraceDirectoryError::PathComponentsExceeded {
                actual: components,
                limit: self.max_components,
            });
        }
        Ok(())
    }
}

/// A verified absolute path, still owned by its caller.
///
/// Dropping this value never deletes files or the directory. The launch layer
/// may retain it across failed process startup and must decide its own cleanup
/// policy for browser-owned temporary profiles separately.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedTraceDirectory {
    path: PathBuf,
}

impl PreparedTraceDirectory {
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn into_path(self) -> PathBuf {
        self.path
    }
}

/// Creates or reuses the requested trace directory without taking ownership.
///
/// `base_directory` is supplied by the caller and must be absolute. An
/// absolute `requested` path ignores it; a relative path is resolved against
/// it, never against ambient process cwd. The returned path is canonical and
/// absolute. Existing files in the directory are never removed or rewritten.
/// A failed preparation also never rolls back any directories already made.
pub fn prepare_trace_directory(
    base_directory: &Path,
    requested: &Path,
    limits: TraceDirectoryLimits,
) -> Result<PreparedTraceDirectory, TraceDirectoryError> {
    limits.validate()?;
    if !base_directory.is_absolute() {
        return Err(TraceDirectoryError::BaseMustBeAbsolute);
    }
    if requested.as_os_str().is_empty() {
        return Err(TraceDirectoryError::EmptyRequestedPath);
    }
    let path = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        base_directory.join(requested)
    };
    limits.check_path(&path)?;
    fs::create_dir_all(&path).map_err(|source| TraceDirectoryError::Create {
        path: path.clone(),
        source,
    })?;
    let canonical =
        fs::canonicalize(&path).map_err(|source| TraceDirectoryError::Canonicalize {
            path: path.clone(),
            source,
        })?;
    limits.check_path(&canonical)?;
    Ok(PreparedTraceDirectory { path: canonical })
}

#[derive(Debug)]
pub enum TraceDirectoryError {
    ZeroLimit(&'static str),
    BaseMustBeAbsolute,
    EmptyRequestedPath,
    PathBytesExceeded { actual: usize, limit: usize },
    PathComponentsExceeded { actual: usize, limit: usize },
    Create { path: PathBuf, source: io::Error },
    Canonicalize { path: PathBuf, source: io::Error },
}

impl Display for TraceDirectoryError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit(field) => {
                write!(formatter, "trace directory {field} limit must be non-zero")
            }
            Self::BaseMustBeAbsolute => {
                formatter.write_str("trace directory base path must be absolute")
            }
            Self::EmptyRequestedPath => formatter.write_str("trace directory path is empty"),
            Self::PathBytesExceeded { actual, limit } => write!(
                formatter,
                "trace directory path uses {actual} bytes, exceeding limit {limit}"
            ),
            Self::PathComponentsExceeded { actual, limit } => write!(
                formatter,
                "trace directory path has {actual} components, exceeding limit {limit}"
            ),
            Self::Create { path, source } => {
                write!(
                    formatter,
                    "cannot create trace directory {}: {source}",
                    path.display()
                )
            }
            Self::Canonicalize { path, source } => write!(
                formatter,
                "cannot resolve trace directory {}: {source}",
                path.display()
            ),
        }
    }
}

impl Error for TraceDirectoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Create { source, .. } | Self::Canonicalize { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::{TraceDirectoryError, TraceDirectoryLimits, prepare_trace_directory};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(1);

    fn fresh_directory() -> PathBuf {
        let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "pwright-v8-trace-directory-{}-{sequence}",
            std::process::id()
        ))
    }

    fn limits() -> TraceDirectoryLimits {
        TraceDirectoryLimits::new(1024, 32)
    }

    #[test]
    fn relative_request_uses_explicit_base_and_preserves_existing_content() {
        // Arrange
        let base = fresh_directory();
        fs::create_dir(&base).unwrap();
        let requested = Path::new("nested/traces");
        let expected = base.join(requested);
        fs::create_dir_all(&expected).unwrap();
        let marker = expected.join("keep.txt");
        fs::write(&marker, b"caller data").unwrap();

        // Act
        let prepared = prepare_trace_directory(&base, requested, limits()).unwrap();
        let prepared_again = prepare_trace_directory(&base, requested, limits()).unwrap();
        let path = prepared.path().to_path_buf();
        drop(prepared);

        // Assert
        assert_eq!(path, expected.canonicalize().unwrap());
        assert_eq!(prepared_again.path(), path);
        assert!(
            path.exists(),
            "dropping the handle must not remove caller data"
        );
        assert_eq!(fs::read(marker).unwrap(), b"caller data");
        fs::remove_dir_all(base).unwrap();
    }

    #[test]
    fn absolute_request_ignores_base_and_recursively_creates_directories() {
        // Arrange
        let root = fresh_directory();
        fs::create_dir(&root).unwrap();
        let requested = root.join("outside").join("nested");
        let other_base = root.join("unused-base");

        // Act
        let prepared = prepare_trace_directory(&other_base, &requested, limits()).unwrap();

        // Assert
        assert_eq!(prepared.path(), requested.canonicalize().unwrap());
        assert!(!other_base.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_inputs_fail_without_creating_a_directory() {
        // Arrange
        let base = fresh_directory();
        let target = base.join("traces");

        // Act
        let relative_base = prepare_trace_directory(Path::new("relative"), &target, limits());
        let empty = prepare_trace_directory(&base, Path::new(""), limits());
        let bytes = prepare_trace_directory(&base, &target, TraceDirectoryLimits::new(1, 32));
        let components =
            prepare_trace_directory(&base, &target, TraceDirectoryLimits::new(1024, 1));

        // Assert
        assert!(matches!(
            relative_base,
            Err(TraceDirectoryError::BaseMustBeAbsolute)
        ));
        assert!(matches!(
            empty,
            Err(TraceDirectoryError::EmptyRequestedPath)
        ));
        assert!(matches!(
            bytes,
            Err(TraceDirectoryError::PathBytesExceeded { .. })
        ));
        assert!(matches!(
            components,
            Err(TraceDirectoryError::PathComponentsExceeded { .. })
        ));
        assert!(!target.exists());
    }

    #[test]
    fn existing_file_is_reported_and_preserved() {
        // Arrange
        let root = fresh_directory();
        fs::create_dir(&root).unwrap();
        let requested = root.join("not-a-directory");
        fs::write(&requested, b"untouched").unwrap();

        // Act
        let error = prepare_trace_directory(&root, &requested, limits()).unwrap_err();

        // Assert
        assert!(matches!(error, TraceDirectoryError::Create { .. }));
        assert_eq!(fs::read(requested).unwrap(), b"untouched");
        fs::remove_dir_all(root).unwrap();
    }
}
