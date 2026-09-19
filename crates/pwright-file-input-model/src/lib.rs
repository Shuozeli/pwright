//! Opt-in, allocation-free file-input preparation policy.
//!
//! Metadata comes from the embedding boundary, not filesystem or global state.
//! This plan does not own/certify file contents or admit their allocations. The
//! embedding caller must validate actual Buffer lengths, inject filesystem/MIME
//! services, and admit source, conversion, transport and browser-operation owners.
//! Neither Node, CDP, a browser process nor the ordinary Locator crate is selected.

use std::fmt;

/// Pinned Playwright 1.58.2 rejects totals greater than OR EQUAL to this limit.
pub const UPLOAD_SIZE_LIMIT: u64 = 50 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputItemKind {
    Path,
    Payload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputShape {
    Paths,
    Payloads,
}

/// Explicit browser/server relationship; a loopback endpoint is not proof.
/// Upstream connectOverCDP defaults to Separate even when Chrome runs locally.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserLocation {
    Colocated,
    Separate,
}

#[derive(Clone, Copy)]
pub struct PayloadMetadata<'a> {
    pub name: &'a str,
    pub mime_type: &'a str,
    pub byte_length: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathKind {
    File { byte_length: u64 },
    Directory,
}

#[derive(Clone, Copy)]
pub struct ResolvedPathMetadata<'a> {
    /// Caller-resolved absolute path. Path syntax is platform-specific and is
    /// validated by the filesystem boundary, not this portable planner.
    pub path: &'a str,
    pub kind: PathKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UploadRoute {
    /// Includes clearing with an empty public input array.
    Payloads,
    /// Read caller-local files and materialize browser-side File payloads.
    ReadFiles,
    /// Native file paths require the explicitly selected collocated policy.
    NativePaths,
    /// Upstream preserves a directory as a native path even with Separate.
    /// This is a routing fact, not proof of a shared browser filesystem.
    NativeDirectory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UploadPlan {
    route: UploadRoute,
    items: usize,
    source_bytes: u64,
    base64_bytes: u64,
    metadata_bytes: usize,
}

impl UploadPlan {
    pub const fn route(self) -> UploadRoute {
        self.route
    }
    pub const fn items(self) -> usize {
        self.items
    }
    /// Zero for native paths/directories: their file contents are not buffered.
    pub const fn source_bytes(self) -> u64 {
        self.source_bytes
    }
    /// Sum of each file's individually padded base64, not base64(total bytes).
    pub const fn base64_bytes(self) -> u64 {
        self.base64_bytes
    }
    /// Borrowed input metadata only, not a whole-operation peak allowance.
    pub const fn metadata_bytes(self) -> usize {
        self.metadata_bytes
    }

    /// Called after the backend has followed labels and checked connectivity.
    /// Deliberately does not invent a universal input[type=file] validation:
    /// upstream native-path and injected-payload branches differ there.
    pub fn validate_target(self, target: InputTarget) -> Result<(), FileInputError> {
        if !target.is_input {
            return Err(FileInputError::NotInput);
        }
        if self.items > 1 && !target.multiple && !target.directory {
            return Err(FileInputError::MultipleFiles);
        }
        let directory = self.route == UploadRoute::NativeDirectory;
        if directory && !target.directory {
            return Err(FileInputError::DirectoryNotSupported);
        }
        if !directory && target.directory {
            return Err(FileInputError::DirectoryRequired);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InputTarget {
    pub is_input: bool,
    pub multiple: bool,
    pub directory: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileInputError {
    InvalidLimits,
    CountLimit,
    PathByteLimit,
    NameByteLimit,
    MimeByteLimit,
    MetadataByteLimit,
    MixedPathsAndPayloads,
    MultipleDirectories,
    MixedFilesAndDirectory,
    PayloadSizeLimit,
    RemoteFileSizeLimit,
    NotInput,
    MultipleFiles,
    DirectoryNotSupported,
    DirectoryRequired,
}

impl fmt::Display for FileInputError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::InvalidLimits => "file-input limits must be positive",
            Self::CountLimit => "file-input item count exceeds profile",
            Self::PathByteLimit => "file-input path bytes exceed profile",
            Self::NameByteLimit => "file-input name bytes exceed profile",
            Self::MimeByteLimit => "file-input MIME bytes exceed profile",
            Self::MetadataByteLimit => "file-input metadata bytes exceed profile",
            Self::MixedPathsAndPayloads => "File paths cannot be mixed with buffers",
            Self::MultipleDirectories => "Multiple directories are not supported",
            Self::MixedFilesAndDirectory => "File paths must be all files or a single directory",
            Self::PayloadSizeLimit => "Cannot set buffer larger than 50Mb, please write it to a file and pass its path instead.",
            Self::RemoteFileSizeLimit => "Cannot transfer files larger than 50Mb to a browser not co-located with the server",
            Self::NotInput => "Node is not an HTMLInputElement",
            Self::MultipleFiles => "Non-multiple file input can only accept single file",
            Self::DirectoryNotSupported => "File input does not support directories, pass individual files instead",
            Self::DirectoryRequired => "[webkitdirectory] input requires passing a path to a directory",
        })
    }
}
impl std::error::Error for FileInputError {}

#[derive(Clone, Copy, Debug)]
pub struct FileInputLimits {
    items: usize,
    path_bytes: usize,
    name_bytes: usize,
    mime_bytes: usize,
    metadata_bytes: usize,
}

impl FileInputLimits {
    pub const fn max_items(self) -> usize {
        self.items
    }
    pub const fn max_path_bytes(self) -> usize {
        self.path_bytes
    }
    pub const fn max_name_bytes(self) -> usize {
        self.name_bytes
    }
    pub const fn max_mime_bytes(self) -> usize {
        self.mime_bytes
    }
    pub fn new(
        items: usize,
        path_bytes: usize,
        name_bytes: usize,
        mime_bytes: usize,
        metadata_bytes: usize,
    ) -> Result<Self, FileInputError> {
        if [items, path_bytes, name_bytes, mime_bytes, metadata_bytes].contains(&0) {
            return Err(FileInputError::InvalidLimits);
        }
        Ok(Self {
            items,
            path_bytes,
            name_bytes,
            mime_bytes,
            metadata_bytes,
        })
    }
    fn count(self, count: usize) -> Result<(), FileInputError> {
        if count > self.items {
            Err(FileInputError::CountLimit)
        } else {
            Ok(())
        }
    }
    fn metadata(self, total: &mut usize, bytes: usize) -> Result<(), FileInputError> {
        *total = total
            .checked_add(bytes)
            .filter(|sum| *sum <= self.metadata_bytes)
            .ok_or(FileInputError::MetadataByteLimit)?;
        Ok(())
    }
    pub fn classify(self, items: &[InputItemKind]) -> Result<InputShape, FileInputError> {
        self.count(items.len())?;
        let has_paths = items.contains(&InputItemKind::Path);
        if has_paths && items.contains(&InputItemKind::Payload) {
            return Err(FileInputError::MixedPathsAndPayloads);
        }
        Ok(if has_paths {
            InputShape::Paths
        } else {
            InputShape::Payloads
        })
    }
    pub fn payloads(self, items: &[PayloadMetadata<'_>]) -> Result<UploadPlan, FileInputError> {
        self.payloads_iter(items.iter().copied())
    }
    /// Validate borrowed sources without first allocating a metadata array.
    pub fn payloads_iter<'a>(
        self,
        items: impl ExactSizeIterator<Item = PayloadMetadata<'a>>,
    ) -> Result<UploadPlan, FileInputError> {
        self.count(items.len())?;
        let mut plan = UploadPlan {
            route: UploadRoute::Payloads,
            items: items.len(),
            source_bytes: 0,
            base64_bytes: 0,
            metadata_bytes: 0,
        };
        for item in items {
            if item.name.len() > self.name_bytes {
                return Err(FileInputError::NameByteLimit);
            }
            if item.mime_type.len() > self.mime_bytes {
                return Err(FileInputError::MimeByteLimit);
            }
            self.metadata(&mut plan.metadata_bytes, item.name.len())?;
            self.metadata(&mut plan.metadata_bytes, item.mime_type.len())?;
            add_payload_bytes(
                &mut plan,
                item.byte_length,
                FileInputError::PayloadSizeLimit,
            )?;
        }
        Ok(plan)
    }
    pub fn paths(
        self,
        items: &[ResolvedPathMetadata<'_>],
        location: BrowserLocation,
    ) -> Result<UploadPlan, FileInputError> {
        self.paths_iter(items.iter().copied(), location)
    }
    pub fn paths_iter<'a>(
        self,
        items: impl ExactSizeIterator<Item = ResolvedPathMetadata<'a>> + Clone,
        location: BrowserLocation,
    ) -> Result<UploadPlan, FileInputError> {
        self.count(items.len())?;
        let mut plan = UploadPlan {
            route: UploadRoute::NativePaths,
            items: items.len(),
            source_bytes: 0,
            base64_bytes: 0,
            metadata_bytes: 0,
        };
        let mut directory = false;
        let mut files = false;
        for item in items.clone() {
            if item.path.len() > self.path_bytes {
                return Err(FileInputError::PathByteLimit);
            }
            self.metadata(&mut plan.metadata_bytes, item.path.len())?;
            match item.kind {
                PathKind::Directory if directory => {
                    return Err(FileInputError::MultipleDirectories);
                }
                PathKind::Directory => directory = true,
                PathKind::File { .. } => files = true,
            }
        }
        if directory && files {
            return Err(FileInputError::MixedFilesAndDirectory);
        }
        plan.route = if directory {
            UploadRoute::NativeDirectory
        } else if items.len() == 0 {
            UploadRoute::Payloads
        } else if location == BrowserLocation::Separate {
            UploadRoute::ReadFiles
        } else {
            UploadRoute::NativePaths
        };
        if plan.route == UploadRoute::ReadFiles {
            for item in items {
                if let PathKind::File { byte_length } = item.kind {
                    add_payload_bytes(&mut plan, byte_length, FileInputError::RemoteFileSizeLimit)?;
                }
            }
        }
        Ok(plan)
    }
}

fn add_payload_bytes(
    plan: &mut UploadPlan,
    bytes: u64,
    error: FileInputError,
) -> Result<(), FileInputError> {
    plan.source_bytes = plan
        .source_bytes
        .checked_add(bytes)
        .filter(|sum| *sum < UPLOAD_SIZE_LIMIT)
        .ok_or(error)?;
    // Safe after the exclusive 50 MiB check, including for zero-byte files.
    plan.base64_bytes += bytes.div_ceil(3) * 4;
    Ok(())
}

#[cfg(test)]
mod tests;
