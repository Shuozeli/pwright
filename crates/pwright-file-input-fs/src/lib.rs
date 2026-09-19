//! Optional filesystem source preparation. Synchronous; no workers are spawned.
//! Environment/base path, filesystem and MIME policy are explicit dependencies.

use pwright_core::{ResourceClass, ResourceLease, ResourceLedger, ResourceRequest, ResourceScope};
use pwright_file_input_model::{
    BrowserLocation, FileInputError, FileInputLimits, PathKind, ResolvedPathMetadata, UploadPlan,
    UploadRoute,
};
use pwright_file_input_source::{Content, PayloadSource, PreparedPayloads, SourceError};
use std::{
    fmt,
    fs::File,
    io::Read,
    path::{Component, Path, PathBuf},
};

mod mime;
pub use mime::PinnedMime;

pub trait MimeLookup {
    fn lookup(&self, name: &str) -> Option<&str>;
}
#[derive(Clone, Copy, Debug)]
pub struct FileObservation {
    pub kind: PathKind,
    pub modified_ms: f64,
}
pub trait FileSystem {
    type Reader: Read;
    fn stat(&self, path: &Path) -> Result<FileObservation, FsError>;
    fn open(&self, path: &Path) -> Result<Self::Reader, FsError>;
}

#[derive(Debug)]
pub enum FsError {
    Source(SourceError),
    Io(std::io::Error),
    InvalidBase,
    PathEncoding,
    UnsupportedFile,
    UnsupportedPlatform,
}
impl From<SourceError> for FsError {
    fn from(value: SourceError) -> Self {
        Self::Source(value)
    }
}
impl From<std::io::Error> for FsError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Source(error) => error.fmt(f),
            Self::Io(error) => error.fmt(f),
            Self::InvalidBase => f.write_str("file-input base must be an absolute UTF-8 path"),
            Self::PathEncoding => f.write_str("file-input path is not UTF-8"),
            Self::UnsupportedFile => {
                f.write_str("file-input source must be a regular file or directory")
            }
            Self::UnsupportedPlatform => {
                f.write_str("file-input filesystem provider requires Unix nonblocking open")
            }
        }
    }
}
impl std::error::Error for FsError {}

/// Opening nonblocking then checking the descriptor avoids hanging on a FIFO
/// substituted between stat and open. Symlinks are followed, as by upstream stat.
pub struct SystemFileSystem;
impl FileSystem for SystemFileSystem {
    type Reader = File;
    fn stat(&self, path: &Path) -> Result<FileObservation, FsError> {
        observation(std::fs::metadata(path)?)
    }
    fn open(&self, path: &Path) -> Result<File, FsError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            let file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(path)?;
            if !file.metadata()?.is_file() {
                return Err(FsError::UnsupportedFile);
            }
            Ok(file)
        }
        #[cfg(not(unix))]
        {
            let _ = path;
            Err(FsError::UnsupportedPlatform)
        }
    }
}
fn observation(metadata: std::fs::Metadata) -> Result<FileObservation, FsError> {
    let kind = if metadata.is_file() {
        PathKind::File {
            byte_length: metadata.len(),
        }
    } else if metadata.is_dir() {
        PathKind::Directory
    } else {
        return Err(FsError::UnsupportedFile);
    };
    let modified = metadata.modified()?;
    let modified_ms = match modified.duration_since(std::time::UNIX_EPOCH) {
        Ok(duration) => duration.as_secs_f64() * 1000.0,
        Err(error) => -error.duration().as_secs_f64() * 1000.0,
    };
    Ok(FileObservation { kind, modified_ms })
}

struct Resolved {
    path: PathBuf,
    observation: FileObservation,
    mime: String,
}
impl Resolved {
    fn metadata(&self) -> ResolvedPathMetadata<'_> {
        ResolvedPathMetadata {
            path: self.path.to_str().expect("resolved UTF-8 path"),
            kind: self.observation.kind,
        }
    }
    fn name(&self) -> &str {
        self.path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("")
    }
}

pub struct PathServices<'a, F, M> {
    pub base: &'a Path,
    pub filesystem: &'a F,
    pub mime: &'a M,
}

pub struct PreparedPaths<R> {
    entries: Vec<Resolved>,
    plan: UploadPlan,
    ledger: ResourceLedger,
    scope: ResourceScope,
    limits: FileInputLimits,
    reader: std::marker::PhantomData<fn() -> R>,
    _lease: ResourceLease,
}
pub enum PreparedInput<R> {
    Payloads(PreparedPayloads),
    Native(PreparedPaths<R>),
}
impl<R: Read> PreparedPaths<R> {
    pub const fn plan(&self) -> UploadPlan {
        self.plan
    }
    pub fn paths(&self) -> impl ExactSizeIterator<Item = &Path> {
        self.entries.iter().map(|entry| entry.path.as_path())
    }
    pub fn prepare(
        ledger: &ResourceLedger,
        scope: ResourceScope,
        limits: FileInputLimits,
        paths: &[&str],
        location: BrowserLocation,
        services: PathServices<'_, impl FileSystem<Reader = R>, impl MimeLookup>,
    ) -> Result<Self, FsError> {
        let PathServices {
            base,
            filesystem,
            mime,
        } = services;
        if paths.len() > limits.max_items() {
            return Err(SourceError::Policy(FileInputError::CountLimit).into());
        }
        let base_text = base
            .to_str()
            .filter(|_| base.is_absolute())
            .ok_or(FsError::InvalidBase)?;
        // Check raw input bounds before path construction. Allowance covers both
        // normalized path and temporary joined path, metadata/source view arrays,
        // MIME strings and opened File values. Kernel descriptor memory is external.
        if base_text.len() > limits.max_path_bytes()
            || paths
                .iter()
                .any(|path| path.len() > limits.max_path_bytes())
        {
            return Err(SourceError::Policy(FileInputError::PathByteLimit).into());
        }
        let entry_bytes = limits
            .max_path_bytes()
            .checked_mul(5)
            .and_then(|n| n.checked_add(2))
            .and_then(|n| n.checked_add(limits.max_mime_bytes()))
            .and_then(|n| {
                n.checked_add(
                    std::mem::size_of::<Resolved>()
                        + std::mem::size_of::<PayloadSource<'static>>()
                        + std::mem::size_of::<R>(),
                )
            });
        let bytes = entry_bytes
            .and_then(|n| n.checked_mul(paths.len()))
            .and_then(|n| n.checked_add(std::mem::size_of::<Self>() + 512))
            .and_then(|n| u64::try_from(n).ok())
            .ok_or(SourceError::SizeOverflow)?;
        let handles = u64::try_from(paths.len())
            .ok()
            .and_then(|n| n.checked_add(1))
            .ok_or(SourceError::SizeOverflow)?;
        let lease = ledger
            .reserve_many(
                scope,
                &[
                    ResourceRequest::new(ResourceClass::LiveObject, handles),
                    ResourceRequest::new(ResourceClass::BackendStateBytes, bytes),
                ],
            )
            .map_err(SourceError::Budget)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(paths.len())
            .map_err(SourceError::Allocation)?;
        let mut directory = false;
        for path in paths {
            // Upstream stats the original path BEFORE lexical path.resolve.
            // Preserve symlink/.. and missing-component behavior at this stage.
            let observation = filesystem.stat(&join(base, path)?)?;
            if observation.kind == PathKind::Directory {
                if directory {
                    return Err(SourceError::Policy(FileInputError::MultipleDirectories).into());
                }
                directory = true;
            }
            let path = resolve(base, path, limits.max_path_bytes())?;
            entries.push(Resolved {
                path,
                observation,
                mime: String::new(),
            });
        }
        let shape = limits
            .paths_iter(
                entries.iter().map(Resolved::metadata),
                BrowserLocation::Colocated,
            )
            .map_err(SourceError::Policy)?;
        if location == BrowserLocation::Separate && shape.route() == UploadRoute::NativePaths {
            // Server-side transfer sizes are a second stat of resolved paths.
            for entry in &mut entries {
                let observed = filesystem.stat(&entry.path)?;
                if !matches!(observed.kind, PathKind::File { .. }) {
                    return Err(SourceError::SourceChanged.into());
                }
                entry.observation = observed;
            }
        }
        let plan = limits
            .paths_iter(entries.iter().map(Resolved::metadata), location)
            .map_err(SourceError::Policy)?;
        if plan.route() == UploadRoute::ReadFiles {
            for entry in &mut entries {
                let name = entry.name();
                if name.len() > limits.max_name_bytes() {
                    return Err(SourceError::Policy(FileInputError::NameByteLimit).into());
                }
                let value = mime.lookup(name).unwrap_or("application/octet-stream");
                if value.len() > limits.max_mime_bytes() {
                    return Err(SourceError::Policy(FileInputError::MimeByteLimit).into());
                }
                entry
                    .mime
                    .try_reserve_exact(value.len())
                    .map_err(SourceError::Allocation)?;
                entry.mime.push_str(value);
            }
        }
        Ok(Self {
            entries,
            plan,
            ledger: ledger.clone(),
            scope,
            limits,
            reader: std::marker::PhantomData,
            _lease: lease,
        })
    }

    /// Consumes path state after transfer; native routes retain it through the
    /// later browser command. All opened readers drop before the path lease.
    pub fn materialize(
        self,
        filesystem: &impl FileSystem<Reader = R>,
    ) -> Result<PreparedInput<R>, FsError> {
        match self.plan.route() {
            UploadRoute::NativePaths | UploadRoute::NativeDirectory => {
                return Ok(PreparedInput::Native(self));
            }
            UploadRoute::Payloads | UploadRoute::ReadFiles => (),
        }
        let mut readers = Vec::new();
        readers
            .try_reserve_exact(self.entries.len())
            .map_err(SourceError::Allocation)?;
        for entry in &self.entries {
            readers.push(filesystem.open(&entry.path)?);
        }
        let mut sources = Vec::new();
        sources
            .try_reserve_exact(self.entries.len())
            .map_err(SourceError::Allocation)?;
        for (entry, reader) in self.entries.iter().zip(&mut readers) {
            let PathKind::File { byte_length } = entry.observation.kind else {
                unreachable!("read route contains only files")
            };
            sources.push(PayloadSource {
                name: entry.name(),
                mime_type: &entry.mime,
                last_modified_ms: None,
                content: Content::Reader {
                    reader,
                    byte_length,
                },
            });
        }
        let mut payloads =
            PreparedPayloads::materialize(&self.ledger, self.scope, self.limits, &mut sources)?;
        // Upstream observes path mtime after reading, not the initial size stat.
        for (index, entry) in self.entries.iter().enumerate() {
            payloads
                .set_last_modified_ms(index, Some(filesystem.stat(&entry.path)?.modified_ms))?;
        }
        Ok(PreparedInput::Payloads(payloads))
    }
}

fn resolve(base: &Path, input: &str, limit: usize) -> Result<PathBuf, FsError> {
    let joined = join(base, input)?;
    let mut result = PathBuf::new();
    result
        .try_reserve_exact(joined.as_os_str().len())
        .map_err(SourceError::Allocation)?;
    for component in joined.components() {
        match component {
            Component::CurDir => (),
            Component::ParentDir => {
                result.pop();
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                result.push(component.as_os_str())
            }
        }
    }
    let text = result.to_str().ok_or(FsError::PathEncoding)?;
    if text.len() > limit {
        return Err(SourceError::Policy(FileInputError::PathByteLimit).into());
    }
    Ok(result)
}

fn join(base: &Path, input: &str) -> Result<PathBuf, FsError> {
    let mut joined = PathBuf::new();
    joined
        .try_reserve_exact(base.as_os_str().len() + input.len() + 1)
        .map_err(SourceError::Allocation)?;
    if !Path::new(input).is_absolute() {
        joined.push(base);
    }
    joined.push(input);
    Ok(joined)
}

#[cfg(test)]
mod tests;
