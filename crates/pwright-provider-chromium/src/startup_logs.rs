//! Bounded, optional output capture for an owned Chromium process.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncRead, AsyncReadExt};

// Keep the formatted launch error below HostError::backend's 4096-byte limit.
// Pending lines and read chunks have independent, fixed-size bounds.
const MAX_LOG_BYTES: usize = 3 * 1024;
const MAX_LINE_BYTES: usize = 1024;
const MAX_LINES: usize = 150;
const READ_CHUNK_BYTES: usize = 4096;

#[derive(Clone, Copy)]
pub(super) enum StreamKind {
    Out,
    Err,
}

impl StreamKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Out => "out",
            Self::Err => "err",
        }
    }
}

#[derive(Default)]
struct PendingLine {
    bytes: Vec<u8>,
    truncated: bool,
}

impl PendingLine {
    fn push(&mut self, byte: u8) {
        if self.bytes.len() < MAX_LINE_BYTES {
            self.bytes.push(byte);
        } else {
            self.truncated = true;
        }
    }

    fn finish(&mut self) -> (Vec<u8>, bool) {
        (
            std::mem::take(&mut self.bytes),
            std::mem::take(&mut self.truncated),
        )
    }

    fn is_empty(&self) -> bool {
        self.bytes.is_empty() && !self.truncated
    }
}

struct StartupLogState {
    process_id: Option<u32>,
    lines: VecDeque<String>,
    line_bytes: usize,
    stdout: PendingLine,
    stderr: PendingLine,
}

impl StartupLogState {
    fn new(process_id: Option<u32>) -> Self {
        Self {
            process_id,
            lines: VecDeque::new(),
            line_bytes: 0,
            stdout: PendingLine::default(),
            stderr: PendingLine::default(),
        }
    }

    fn pending(&mut self, stream: StreamKind) -> &mut PendingLine {
        match stream {
            StreamKind::Out => &mut self.stdout,
            StreamKind::Err => &mut self.stderr,
        }
    }

    fn record_bytes(&mut self, stream: StreamKind, chunk: &[u8]) {
        for &byte in chunk {
            if byte == b'\n' {
                let (bytes, truncated) = self.pending(stream).finish();
                self.record_stream_line(stream, &bytes, truncated);
            } else {
                self.pending(stream).push(byte);
            }
        }
    }

    fn finish_stream(&mut self, stream: StreamKind) {
        if self.pending(stream).is_empty() {
            return;
        }
        let (bytes, truncated) = self.pending(stream).finish();
        self.record_stream_line(stream, &bytes, truncated);
    }

    fn record_stream_line(&mut self, stream: StreamKind, bytes: &[u8], mut truncated: bool) {
        let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
        let process_id = self
            .process_id
            .map_or_else(|| "N/A".to_owned(), |id| id.to_string());
        let mut payload = String::from_utf8_lossy(bytes).into_owned();
        if payload.len() > MAX_LINE_BYTES {
            let mut boundary = MAX_LINE_BYTES;
            while !payload.is_char_boundary(boundary) {
                boundary -= 1;
            }
            payload.truncate(boundary);
            truncated = true;
        }
        let mut line = format!("[pid={process_id}][{}] {}", stream.label(), payload);
        if truncated {
            line.push_str(" [line truncated]");
        }
        self.record_line(line);
    }

    fn record_line(&mut self, mut line: String) {
        if line.len() > MAX_LOG_BYTES {
            let mut boundary = MAX_LOG_BYTES;
            while !line.is_char_boundary(boundary) {
                boundary -= 1;
            }
            line.truncate(boundary);
        }
        while self.lines.len() >= MAX_LINES || self.line_bytes + line.len() > MAX_LOG_BYTES {
            let removed = self
                .lines
                .pop_front()
                .expect("over-budget log has an entry");
            self.line_bytes -= removed.len();
        }
        self.line_bytes += line.len();
        self.lines.push_back(line);
    }

    fn snapshot(&self) -> String {
        let mut result = String::with_capacity(self.line_bytes + self.lines.len());
        for (index, line) in self.lines.iter().enumerate() {
            if index != 0 {
                result.push('\n');
            }
            result.push_str(line);
        }
        result
    }
}

#[derive(Clone)]
pub(super) struct StartupLogSink(Arc<Mutex<StartupLogState>>);

impl StartupLogSink {
    pub(super) fn new(process_id: Option<u32>) -> Self {
        Self(Arc::new(Mutex::new(StartupLogState::new(process_id))))
    }

    pub(super) fn record_event(&self, message: &str) {
        self.with_state(|state| state.record_line(message.to_owned()));
    }

    pub(super) fn snapshot(&self) -> String {
        self.with_state(|state| state.snapshot())
    }

    pub(super) fn finish_all(&self) {
        self.with_state(|state| {
            state.finish_stream(StreamKind::Out);
            state.finish_stream(StreamKind::Err);
        });
    }

    pub(super) async fn drain<R: AsyncRead + Unpin>(self, mut reader: R, stream: StreamKind) {
        let mut chunk = [0u8; READ_CHUNK_BYTES];
        loop {
            match reader.read(&mut chunk).await {
                Ok(0) => break,
                Ok(count) => {
                    self.with_state(|state| state.record_bytes(stream, &chunk[..count]));
                    // A chatty child must not monopolize a single-threaded
                    // runtime or postpone cancellation indefinitely.
                    tokio::task::yield_now().await;
                }
                Err(error) => {
                    self.with_state(|state| {
                        state.record_line(format!(
                            "[{}] output read failed: {error}",
                            stream.label()
                        ));
                    });
                    break;
                }
            }
        }
        self.with_state(|state| state.finish_stream(stream));
    }

    fn with_state<T>(&self, work: impl FnOnce(&mut StartupLogState) -> T) -> T {
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        work(&mut state)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_LOG_BYTES, StartupLogSink, StreamKind};

    #[test]
    fn unterminated_and_invalid_output_remains_bounded() {
        // Arrange
        let logs = StartupLogSink::new(Some(42));
        let hostile = vec![0xff; 1024 * 1024];

        // Act
        logs.with_state(|state| state.record_bytes(StreamKind::Err, &hostile));
        logs.finish_all();
        let snapshot = logs.snapshot();

        // Assert
        assert!(snapshot.len() <= MAX_LOG_BYTES);
        assert!(snapshot.contains("[line truncated]"));
        assert!(snapshot.contains("[pid=42][err]"));
    }

    #[test]
    fn only_recent_complete_lines_survive_total_byte_budget() {
        // Arrange
        let logs = StartupLogSink::new(None);

        // Act
        for number in 0..200 {
            logs.record_event(&format!("event-{number:03}-{}", "x".repeat(40)));
        }
        let snapshot = logs.snapshot();

        // Assert
        assert!(snapshot.len() <= MAX_LOG_BYTES + 149);
        assert!(!snapshot.contains("event-000"));
        assert!(snapshot.contains("event-199"));
    }
}
