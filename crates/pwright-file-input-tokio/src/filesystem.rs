use super::JobState;
use pwright_file_input_fs::{FileObservation, FileSystem, FsError};
use std::{
    io::{self, Read},
    path::Path,
    sync::Arc,
};

pub struct CancellableReader<R> {
    reader: R,
    state: Arc<JobState>,
}
impl<R: Read> Read for CancellableReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.state.cancelled() {
            return Err(io::Error::other("file-input preparation cancelled"));
        }
        // A syscall itself is not preemptible. Bound the next request and check
        // cancellation again before every subsequent chunk or EOF probe.
        let limit = buffer.len().min(64 * 1024);
        self.reader.read(&mut buffer[..limit])
    }
}
pub(super) struct CancellableFileSystem<F> {
    pub filesystem: Arc<F>,
    pub state: Arc<JobState>,
}
impl<F: FileSystem> FileSystem for CancellableFileSystem<F> {
    type Reader = CancellableReader<F::Reader>;
    fn stat(&self, path: &Path) -> Result<FileObservation, FsError> {
        if self.state.cancelled() {
            return Err(io::Error::other("file-input preparation cancelled").into());
        }
        self.filesystem.stat(path)
    }
    fn open(&self, path: &Path) -> Result<Self::Reader, FsError> {
        if self.state.cancelled() {
            return Err(io::Error::other("file-input preparation cancelled").into());
        }
        self.filesystem.open(path).map(|reader| CancellableReader {
            reader,
            state: self.state.clone(),
        })
    }
}
