//! Browser-independent lifecycle for one opt-in Playwright v8 Context trace.
//!
//! This crate chooses no browser, executor, task, filesystem, ZIP writer, or
//! network recorder. A caller injects a clock, identifier source, output port,
//! and connection-local resource ledger. The output port owns its own disk and
//! archive quotas; this coordinator bounds its names, groups, chunks, JSONL
//! records, and in-memory scratch/labels. Snapshot, screenshot, and source
//! capture are deliberately rejected until independent producers are installed.
//! Context close, disconnect, and cancellation owners must call [`ContextTraceSession::abort`]
//! and handle its error. Dropping the session releases its memory lease, but
//! cannot report a failed output cleanup; production output ports must also
//! use RAII for any unfinished temporary file.

use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::Write;
use std::path::Path;

use pwright_core::{ResourceClass, ResourceError, ResourceLease, ResourceLedger, ResourceScope};
use pwright_trace_playwright_v8::{
    AfterAction, ApiTraceEvent, BeforeAction, ContextOptions, PlaywrightV8TraceWriter, SdkLanguage,
    StackFrame, TraceOrigin, TraceWriterError, TraceWriterLimits,
};
use serde_json::{Map, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextTraceLimits {
    pub writer: TraceWriterLimits,
    pub max_name_bytes: usize,
    pub max_export_path_bytes: usize,
    pub max_group_depth: usize,
    pub max_chunks_per_session: usize,
}

impl ContextTraceLimits {
    pub const fn new(
        writer: TraceWriterLimits,
        max_name_bytes: usize,
        max_export_path_bytes: usize,
        max_group_depth: usize,
        max_chunks_per_session: usize,
    ) -> Self {
        Self {
            writer,
            max_name_bytes,
            max_export_path_bytes,
            max_group_depth,
            max_chunks_per_session,
        }
    }

    /// Minimum `TraceBytes` lease per active Context; the composition root
    /// multiplies this by its bounded active-Context count for the connection.
    pub fn required_trace_bytes(self) -> Result<u64, TraceLimitError> {
        for (name, value) in [
            ("record bytes", self.writer.max_record_bytes),
            ("events", self.writer.max_events),
            ("name bytes", self.max_name_bytes),
            ("export path bytes", self.max_export_path_bytes),
            ("group depth", self.max_group_depth),
            ("chunks per session", self.max_chunks_per_session),
        ] {
            if value == 0 {
                return Err(TraceLimitError::ZeroLimit(name));
            }
        }
        if self.writer.max_total_bytes == 0 {
            return Err(TraceLimitError::ZeroLimit("total bytes"));
        }
        let labels = self
            .max_group_depth
            .checked_add(2)
            .and_then(|count| count.checked_mul(self.max_name_bytes))
            .ok_or(TraceLimitError::Overflow)?;
        let groups = self
            .max_group_depth
            .checked_mul(std::mem::size_of::<String>())
            .ok_or(TraceLimitError::Overflow)?;
        let bytes = labels
            .checked_add(groups)
            .and_then(|bytes| bytes.checked_add(self.writer.max_record_bytes))
            .ok_or(TraceLimitError::Overflow)?;
        u64::try_from(bytes).map_err(|_| TraceLimitError::Overflow)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceLimitError {
    ZeroLimit(&'static str),
    Overflow,
}

impl Display for TraceLimitError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit(name) => write!(formatter, "trace {name} limit must be non-zero"),
            Self::Overflow => formatter.write_str("trace resource limit overflowed"),
        }
    }
}

impl Error for TraceLimitError {}

#[derive(Clone, Copy, Debug)]
pub struct TraceTime {
    pub wall_time_ms: f64,
    pub monotonic_time_ms: f64,
}

pub trait TraceClock {
    fn now(&self) -> TraceTime;
}

/// Supplied by the composition root; it must not call hidden global state.
pub trait TraceIdSource {
    fn next_id(&mut self) -> String;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceChunkKey<'a> {
    pub session_name: &'a str,
    pub trace_name: &'a str,
    pub ordinal: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChunkDisposition<'a> {
    Discard,
    Export(&'a Path),
}

/// One Context-scoped output policy. It must namespace names from other
/// contexts, confine requested export paths according to the host's policy,
/// and budget its own writer buffers, disk, and archive work. The returned
/// writer is the sole live writer for a chunk; `finish_chunk` must consume it
/// without retaining it. `abort_session` must clean any partial operation and
/// be idempotent so cleanup can be retried after an error.
pub trait TraceOutput {
    type Writer: Write;
    type Error: Error + 'static;

    fn open_chunk(&mut self, key: TraceChunkKey<'_>) -> Result<Self::Writer, Self::Error>;
    fn finish_chunk(
        &mut self,
        key: TraceChunkKey<'_>,
        writer: Self::Writer,
        disposition: ChunkDisposition<'_>,
    ) -> Result<(), Self::Error>;
    fn finish_session(&mut self) -> Result<(), Self::Error>;
    fn abort_session(&mut self) -> Result<(), Self::Error>;
}

#[derive(Clone, Copy, Debug)]
pub struct TraceContextMetadata<'a> {
    pub origin: TraceOrigin,
    pub browser_name: &'a str,
    pub channel: Option<&'a str>,
    pub platform: &'a str,
    pub options: &'a Value,
    pub sdk_language: Option<SdkLanguage>,
    pub test_id_attribute_name: Option<&'a str>,
    pub context_id: Option<&'a str>,
    pub playwright_version: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StartOptions<'a> {
    pub name: Option<&'a str>,
    pub title: Option<&'a str>,
    pub snapshots: bool,
    pub screenshots: bool,
    pub sources: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct StartChunkOptions<'a> {
    pub name: Option<&'a str>,
    pub title: Option<&'a str>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GroupLocation<'a> {
    pub file: &'a str,
    pub line: Option<u32>,
    pub column: Option<u32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContextTraceState {
    Idle,
    BetweenChunks,
    Recording,
    Poisoned,
}

struct Recording<W: Write> {
    writer: PlaywrightV8TraceWriter<W>,
    ordinal: usize,
    groups: Vec<String>,
}

struct ActiveSession<W: Write> {
    session_name: String,
    trace_name: String,
    next_ordinal: usize,
    chunks_started: usize,
    recording: Option<Recording<W>>,
    _lease: ResourceLease,
}

enum State<W: Write> {
    Idle,
    Active(ActiveSession<W>),
    Poisoned,
}

pub struct ContextTraceSession<Output, Clock, Ids>
where
    Output: TraceOutput,
    Clock: TraceClock,
    Ids: TraceIdSource,
{
    output: Output,
    clock: Clock,
    ids: Ids,
    ledger: ResourceLedger,
    scope: ResourceScope,
    limits: ContextTraceLimits,
    trace_bytes: u64,
    state: State<Output::Writer>,
}

impl<Output, Clock, Ids> ContextTraceSession<Output, Clock, Ids>
where
    Output: TraceOutput,
    Clock: TraceClock,
    Ids: TraceIdSource,
{
    pub fn new(
        output: Output,
        clock: Clock,
        ids: Ids,
        ledger: ResourceLedger,
        scope: ResourceScope,
        limits: ContextTraceLimits,
    ) -> Result<Self, ContextTraceError<Output::Error>> {
        let trace_bytes = limits.required_trace_bytes().map_err(|error| match error {
            TraceLimitError::ZeroLimit(name) => ContextTraceError::ZeroLimit(name),
            TraceLimitError::Overflow => ContextTraceError::LimitOverflow,
        })?;
        Ok(Self {
            output,
            clock,
            ids,
            ledger,
            scope,
            limits,
            trace_bytes,
            state: State::Idle,
        })
    }

    pub fn state(&self) -> ContextTraceState {
        match &self.state {
            State::Idle => ContextTraceState::Idle,
            State::Active(session) if session.recording.is_some() => ContextTraceState::Recording,
            State::Active(_) => ContextTraceState::BetweenChunks,
            State::Poisoned => ContextTraceState::Poisoned,
        }
    }

    /// A public Playwright `start` includes its first chunk.
    pub fn start(
        &mut self,
        metadata: &TraceContextMetadata<'_>,
        options: StartOptions<'_>,
    ) -> Result<(), ContextTraceError<Output::Error>> {
        match self.state {
            State::Idle => {}
            State::Active(_) => return Err(ContextTraceError::AlreadyStarted),
            State::Poisoned => return Err(ContextTraceError::Poisoned),
        }
        if options.snapshots {
            return Err(ContextTraceError::UnsupportedProducer("snapshots"));
        }
        if options.screenshots {
            return Err(ContextTraceError::UnsupportedProducer("screenshots"));
        }
        if options.sources {
            return Err(ContextTraceError::UnsupportedProducer("sources"));
        }
        if let Some(title) = options.title {
            self.check_label(title)?;
        }
        let name = self.selected_name(options.name)?;
        let lease = self
            .ledger
            .reserve(self.scope, ResourceClass::TraceBytes, self.trace_bytes)?;
        let recording = self.open_recording(metadata, &name, &name, 0, options.title);
        let recording = match recording {
            Ok(recording) => recording,
            Err(primary) => {
                self.state = State::Poisoned;
                return Err(self.abort_output_after(primary));
            }
        };
        self.state = State::Active(ActiveSession {
            session_name: name.clone(),
            trace_name: name,
            next_ordinal: 1,
            chunks_started: 1,
            recording: Some(recording),
            _lease: lease,
        });
        Ok(())
    }

    pub fn start_chunk(
        &mut self,
        metadata: &TraceContextMetadata<'_>,
        options: StartChunkOptions<'_>,
    ) -> Result<(), ContextTraceError<Output::Error>> {
        match &self.state {
            State::Idle => return Err(ContextTraceError::NotStartedChunk),
            State::Poisoned => return Err(ContextTraceError::Poisoned),
            State::Active(session)
                if session.chunks_started == self.limits.max_chunks_per_session =>
            {
                return Err(ContextTraceError::ChunkCapacityExceeded {
                    limit: self.limits.max_chunks_per_session,
                });
            }
            State::Active(_) => {}
        }
        let requested_name = options.name.filter(|name| !name.is_empty());
        if let Some(name) = requested_name {
            self.check_name(name)?;
        }
        if let Some(title) = options.title {
            self.check_label(title)?;
        }
        self.stop_chunk(ChunkDisposition::Discard)?;
        let State::Active(mut session) = std::mem::replace(&mut self.state, State::Poisoned) else {
            unreachable!("active state was checked before stopping the previous chunk")
        };
        let trace_name = requested_name.unwrap_or(&session.trace_name).to_owned();
        let ordinal = if trace_name == session.trace_name {
            session.next_ordinal
        } else {
            0
        };
        let Some(next_ordinal) = ordinal.checked_add(1) else {
            return Err(self.abort_output_after(ContextTraceError::LimitOverflow));
        };
        let recording = self.open_recording(
            metadata,
            &session.session_name,
            &trace_name,
            ordinal,
            options.title,
        );
        match recording {
            Ok(recording) => {
                session.trace_name = trace_name;
                session.next_ordinal = next_ordinal;
                session.chunks_started += 1;
                session.recording = Some(recording);
                self.state = State::Active(session);
                Ok(())
            }
            Err(primary) => Err(self.abort_output_after(primary)),
        }
    }

    pub fn record_api(
        &mut self,
        event: ApiTraceEvent<'_>,
    ) -> Result<bool, ContextTraceError<Output::Error>> {
        let result = match &mut self.state {
            State::Idle
            | State::Active(ActiveSession {
                recording: None, ..
            }) => return Ok(false),
            State::Poisoned => return Err(ContextTraceError::Poisoned),
            State::Active(session) => {
                let recording = session
                    .recording
                    .as_mut()
                    .expect("recording state was checked");
                let event = match event {
                    ApiTraceEvent::Before(before) if before.parent_id.is_none() => {
                        ApiTraceEvent::Before(BeforeAction {
                            parent_id: recording.groups.last().map(String::as_str),
                            ..before
                        })
                    }
                    event => event,
                };
                recording.writer.write_api_event(event)
            }
        };
        match result {
            Ok(()) => Ok(true),
            Err(error) => Err(self.poison_active(ContextTraceError::Writer(error))),
        }
    }

    /// Groups outside an active chunk are ignored by this bounded recorder.
    pub fn group(
        &mut self,
        name: &str,
        location: Option<GroupLocation<'_>>,
    ) -> Result<(), ContextTraceError<Output::Error>> {
        let depth = match &self.state {
            State::Idle
            | State::Active(ActiveSession {
                recording: None, ..
            }) => return Ok(()),
            State::Poisoned => return Err(ContextTraceError::Poisoned),
            State::Active(session) => session.recording.as_ref().unwrap().groups.len(),
        };
        if depth == self.limits.max_group_depth {
            return Err(ContextTraceError::GroupDepthExceeded {
                limit: self.limits.max_group_depth,
            });
        }
        self.check_label(name)?;
        if let Some(location) = location
            && !location.file.is_empty()
        {
            self.check_label(location.file)?;
        }
        let call_id = self.ids.next_id();
        self.check_name(&call_id)?;
        let timestamp = self.clock.now();
        let params = Value::Object(Map::new());
        let stack_frame = location
            .filter(|location| !location.file.is_empty())
            .map(|location| StackFrame {
                file: location.file,
                line: location.line.unwrap_or_default(),
                column: location.column.unwrap_or_default(),
                function: None,
            });
        let stack = stack_frame.as_slice();
        let result = {
            let State::Active(session) = &mut self.state else {
                unreachable!("recording state was checked")
            };
            let recording = session.recording.as_mut().unwrap();
            let parent_id = recording.groups.last().map(String::as_str);
            recording
                .writer
                .write_api_event(ApiTraceEvent::Before(BeforeAction {
                    call_id: &call_id,
                    start_time_ms: timestamp.monotonic_time_ms,
                    title: Some(name),
                    class_name: "Tracing",
                    method: "tracingGroup",
                    params: &params,
                    step_id: None,
                    before_snapshot: None,
                    stack: Some(stack),
                    page_id: None,
                    parent_id,
                }))
        };
        match result {
            Ok(()) => {
                let State::Active(session) = &mut self.state else {
                    unreachable!("recording state was checked")
                };
                session.recording.as_mut().unwrap().groups.push(call_id);
                Ok(())
            }
            Err(error) => Err(self.poison_active(ContextTraceError::Writer(error))),
        }
    }

    pub fn group_end(&mut self) -> Result<(), ContextTraceError<Output::Error>> {
        match &self.state {
            State::Idle
            | State::Active(ActiveSession {
                recording: None, ..
            }) => return Ok(()),
            State::Poisoned => return Err(ContextTraceError::Poisoned),
            State::Active(session) if session.recording.as_ref().unwrap().groups.is_empty() => {
                return Ok(());
            }
            State::Active(_) => {}
        }
        let time = self.clock.now().monotonic_time_ms;
        let result = {
            let State::Active(session) = &mut self.state else {
                unreachable!("recording state was checked")
            };
            let recording = session.recording.as_mut().unwrap();
            let call_id = recording.groups.last().unwrap();
            recording
                .writer
                .write_api_event(ApiTraceEvent::After(AfterAction {
                    call_id,
                    end_time_ms: time,
                    after_snapshot: None,
                    error: None,
                    result: None,
                    point: None,
                }))
        };
        match result {
            Ok(()) => {
                let State::Active(session) = &mut self.state else {
                    unreachable!("recording state was checked")
                };
                session.recording.as_mut().unwrap().groups.pop();
                Ok(())
            }
            Err(error) => Err(self.poison_active(ContextTraceError::Writer(error))),
        }
    }

    pub fn stop_chunk(
        &mut self,
        disposition: ChunkDisposition<'_>,
    ) -> Result<(), ContextTraceError<Output::Error>> {
        self.check_disposition(disposition)?;
        match &self.state {
            State::Idle
            | State::Active(ActiveSession {
                recording: None, ..
            }) => {
                return match disposition {
                    ChunkDisposition::Discard => Ok(()),
                    ChunkDisposition::Export(_) => Err(ContextTraceError::NotStarted),
                };
            }
            State::Poisoned => return Err(ContextTraceError::Poisoned),
            State::Active(_) => {}
        }
        let State::Active(mut session) = std::mem::replace(&mut self.state, State::Poisoned) else {
            unreachable!("recording state was checked")
        };
        let Recording {
            mut writer,
            ordinal,
            mut groups,
        } = session.recording.take().unwrap();
        let key = TraceChunkKey {
            session_name: &session.session_name,
            trace_name: &session.trace_name,
            ordinal,
        };
        while let Some(call_id) = groups.last() {
            let time = self.clock.now().monotonic_time_ms;
            if let Err(error) = writer.write_api_event(ApiTraceEvent::After(AfterAction {
                call_id,
                end_time_ms: time,
                after_snapshot: None,
                error: None,
                result: None,
                point: None,
            })) {
                drop(writer);
                return Err(self.abort_output_after(ContextTraceError::Writer(error)));
            }
            groups.pop();
        }
        let writer = match writer.finish() {
            Ok(writer) => writer,
            Err(error) => return Err(self.abort_output_after(ContextTraceError::Writer(error))),
        };
        if let Err(error) = self.output.finish_chunk(key, writer, disposition) {
            return Err(self.abort_output_after(ContextTraceError::Output(error)));
        }
        self.state = State::Active(session);
        Ok(())
    }

    pub fn stop(
        &mut self,
        disposition: ChunkDisposition<'_>,
    ) -> Result<(), ContextTraceError<Output::Error>> {
        self.stop_chunk(disposition)?;
        if !matches!(self.state, State::Active(_)) {
            return Ok(());
        }
        let state = std::mem::replace(&mut self.state, State::Poisoned);
        drop(state);
        match self.output.finish_session() {
            Ok(()) => {
                self.state = State::Idle;
                Ok(())
            }
            Err(error) => Err(self.abort_output_after(ContextTraceError::Output(error))),
        }
    }

    /// Mandatory explicit Context close/disconnect/cancellation cleanup.
    /// Releases the lease even if output cleanup fails, but keeps a poisoned
    /// state until cleanup succeeds. Merely dropping the session cannot report
    /// output cleanup failure to the caller.
    pub fn abort(&mut self) -> Result<(), ContextTraceError<Output::Error>> {
        if matches!(self.state, State::Idle) {
            return Ok(());
        }
        let old = std::mem::replace(&mut self.state, State::Poisoned);
        drop(old);
        match self.output.abort_session() {
            Ok(()) => {
                self.state = State::Idle;
                Ok(())
            }
            Err(error) => Err(ContextTraceError::Output(error)),
        }
    }

    fn selected_name(
        &mut self,
        requested: Option<&str>,
    ) -> Result<String, ContextTraceError<Output::Error>> {
        if let Some(name) = requested.filter(|name| !name.is_empty()) {
            self.check_name(name)?;
            return Ok(name.to_owned());
        }
        let generated = self.ids.next_id();
        self.check_name(&generated)?;
        Ok(generated)
    }

    fn check_name(&self, name: &str) -> Result<(), ContextTraceError<Output::Error>> {
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains(['/', '\\', '\0'])
            || name.len() > self.limits.max_name_bytes
        {
            return Err(ContextTraceError::InvalidName);
        }
        Ok(())
    }

    fn check_label(&self, label: &str) -> Result<(), ContextTraceError<Output::Error>> {
        if label.is_empty() || label.len() > self.limits.max_name_bytes {
            return Err(ContextTraceError::InvalidLabel);
        }
        Ok(())
    }

    fn check_disposition(
        &self,
        disposition: ChunkDisposition<'_>,
    ) -> Result<(), ContextTraceError<Output::Error>> {
        if let ChunkDisposition::Export(path) = disposition {
            let bytes = path.as_os_str().as_encoded_bytes().len();
            if bytes == 0 || bytes > self.limits.max_export_path_bytes {
                return Err(ContextTraceError::InvalidExportPath);
            }
        }
        Ok(())
    }

    fn open_recording(
        &mut self,
        metadata: &TraceContextMetadata<'_>,
        session_name: &str,
        trace_name: &str,
        ordinal: usize,
        title: Option<&str>,
    ) -> Result<Recording<Output::Writer>, ContextTraceError<Output::Error>> {
        if let Some(title) = title {
            self.check_label(title)?;
        }
        let key = TraceChunkKey {
            session_name,
            trace_name,
            ordinal,
        };
        let writer = self
            .output
            .open_chunk(key)
            .map_err(ContextTraceError::Output)?;
        let mut writer = PlaywrightV8TraceWriter::new(writer, self.limits.writer)
            .map_err(ContextTraceError::Writer)?;
        let now = self.clock.now();
        writer
            .write_context(&ContextOptions {
                origin: metadata.origin,
                browser_name: metadata.browser_name,
                channel: metadata.channel,
                platform: metadata.platform,
                wall_time_ms: now.wall_time_ms,
                monotonic_time_ms: now.monotonic_time_ms,
                title,
                options: metadata.options,
                sdk_language: metadata.sdk_language,
                test_id_attribute_name: metadata.test_id_attribute_name,
                context_id: metadata.context_id,
                playwright_version: metadata.playwright_version,
            })
            .map_err(ContextTraceError::Writer)?;
        let mut groups = Vec::new();
        groups
            .try_reserve_exact(self.limits.max_group_depth)
            .map_err(|_| ContextTraceError::AllocationFailed("group stack"))?;
        Ok(Recording {
            writer,
            ordinal,
            groups,
        })
    }

    fn poison_active(
        &mut self,
        primary: ContextTraceError<Output::Error>,
    ) -> ContextTraceError<Output::Error> {
        let old = std::mem::replace(&mut self.state, State::Poisoned);
        drop(old);
        self.abort_output_after(primary)
    }

    fn abort_output_after(
        &mut self,
        primary: ContextTraceError<Output::Error>,
    ) -> ContextTraceError<Output::Error> {
        match self.output.abort_session() {
            Ok(()) => primary,
            Err(cleanup) => ContextTraceError::CleanupAfterFailure {
                primary: Box::new(primary),
                cleanup,
            },
        }
    }
}

#[derive(Debug)]
pub enum ContextTraceError<E: Error> {
    ZeroLimit(&'static str),
    LimitOverflow,
    InvalidName,
    InvalidLabel,
    InvalidExportPath,
    UnsupportedProducer(&'static str),
    NotStarted,
    NotStartedChunk,
    AlreadyStarted,
    Poisoned,
    ChunkCapacityExceeded { limit: usize },
    GroupDepthExceeded { limit: usize },
    AllocationFailed(&'static str),
    Resource(ResourceError),
    Writer(TraceWriterError),
    Output(E),
    CleanupAfterFailure { primary: Box<Self>, cleanup: E },
}

impl<E: Error> Display for ContextTraceError<E> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroLimit(name) => write!(formatter, "trace {name} limit must be non-zero"),
            Self::LimitOverflow => formatter.write_str("trace resource limit overflowed"),
            Self::InvalidName => formatter.write_str("invalid trace name"),
            Self::InvalidLabel => formatter.write_str("invalid trace label"),
            Self::InvalidExportPath => formatter.write_str("invalid trace export path"),
            Self::UnsupportedProducer(name) => {
                write!(formatter, "{name} producer is not installed")
            }
            Self::NotStarted => formatter.write_str("Must start tracing before stopping"),
            Self::NotStartedChunk => {
                formatter.write_str("Must start tracing before starting a new chunk")
            }
            Self::AlreadyStarted => formatter.write_str("Tracing has been already started"),
            Self::Poisoned => formatter.write_str("trace session is poisoned"),
            Self::ChunkCapacityExceeded { limit } => {
                write!(formatter, "trace session exceeded {limit} chunks")
            }
            Self::GroupDepthExceeded { limit } => {
                write!(formatter, "trace group depth exceeded {limit}")
            }
            Self::AllocationFailed(name) => write!(formatter, "cannot allocate bounded {name}"),
            Self::Resource(error) => Display::fmt(error, formatter),
            Self::Writer(error) => Display::fmt(error, formatter),
            Self::Output(error) => Display::fmt(error, formatter),
            Self::CleanupAfterFailure { primary, cleanup } => {
                write!(formatter, "{primary}; trace cleanup failed: {cleanup}")
            }
        }
    }
}

impl<E: Error + 'static> Error for ContextTraceError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(error) => Some(error),
            Self::Writer(error) => Some(error),
            Self::Output(error) => Some(error),
            Self::CleanupAfterFailure { primary, .. } => Some(primary),
            _ => None,
        }
    }
}

impl<E: Error> From<ResourceError> for ContextTraceError<E> {
    fn from(error: ResourceError) -> Self {
        Self::Resource(error)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::io::{self, Write};
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    use pwright_core::{ObjectId, ResourceClass, ResourceLedger, ResourceLimits, ResourceScope};
    use pwright_trace_playwright_v8::{
        ApiTraceEvent, BeforeAction, SdkLanguage, TraceOrigin, TraceWriterLimits,
    };
    use serde_json::{Value, json};

    use super::{
        ChunkDisposition, ContextTraceError, ContextTraceLimits, ContextTraceSession,
        ContextTraceState, GroupLocation, StartChunkOptions, StartOptions, TraceChunkKey,
        TraceClock, TraceContextMetadata, TraceIdSource, TraceOutput, TraceTime,
    };

    #[derive(Default)]
    struct OutputState {
        opened: Vec<(String, usize)>,
        chunks: Vec<RecordedChunk>,
        finished_sessions: usize,
        aborted_sessions: usize,
        fail_open: bool,
        fail_finish_chunk: bool,
        fail_abort_once: bool,
        fail_after_bytes: Option<usize>,
    }

    struct RecordedChunk {
        name: String,
        ordinal: usize,
        export: Option<PathBuf>,
        bytes: Vec<u8>,
    }

    struct FakeOutput(Rc<RefCell<OutputState>>);

    struct FakeWriter {
        bytes: Vec<u8>,
        fail_after_bytes: Option<usize>,
    }

    impl Write for FakeWriter {
        fn write(&mut self, input: &[u8]) -> io::Result<usize> {
            if let Some(limit) = self.fail_after_bytes {
                if self.bytes.len() == limit {
                    return Err(io::Error::other("injected partial write"));
                }
                let allowed = limit.saturating_sub(self.bytes.len()).min(input.len());
                self.bytes.extend_from_slice(&input[..allowed]);
                return Ok(allowed);
            }
            self.bytes.extend_from_slice(input);
            Ok(input.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    impl TraceOutput for FakeOutput {
        type Writer = FakeWriter;
        type Error = io::Error;

        fn open_chunk(&mut self, key: TraceChunkKey<'_>) -> Result<Self::Writer, Self::Error> {
            let mut state = self.0.borrow_mut();
            if state.fail_open {
                return Err(io::Error::other("injected open failure"));
            }
            state.opened.push((key.trace_name.to_owned(), key.ordinal));
            Ok(FakeWriter {
                bytes: Vec::new(),
                fail_after_bytes: state.fail_after_bytes,
            })
        }

        fn finish_chunk(
            &mut self,
            key: TraceChunkKey<'_>,
            writer: Self::Writer,
            disposition: ChunkDisposition<'_>,
        ) -> Result<(), Self::Error> {
            let mut state = self.0.borrow_mut();
            if state.fail_finish_chunk {
                return Err(io::Error::other("injected export failure"));
            }
            state.chunks.push(RecordedChunk {
                name: key.trace_name.to_owned(),
                ordinal: key.ordinal,
                export: match disposition {
                    ChunkDisposition::Discard => None,
                    ChunkDisposition::Export(path) => Some(path.to_path_buf()),
                },
                bytes: writer.bytes,
            });
            Ok(())
        }

        fn finish_session(&mut self) -> Result<(), Self::Error> {
            self.0.borrow_mut().finished_sessions += 1;
            Ok(())
        }

        fn abort_session(&mut self) -> Result<(), Self::Error> {
            let mut state = self.0.borrow_mut();
            state.aborted_sessions += 1;
            if state.fail_abort_once {
                state.fail_abort_once = false;
                return Err(io::Error::other("injected cleanup failure"));
            }
            Ok(())
        }
    }

    struct FakeClock(Cell<u64>);

    impl TraceClock for FakeClock {
        fn now(&self) -> TraceTime {
            let tick = self.0.get();
            self.0.set(tick + 1);
            TraceTime {
                wall_time_ms: 1000.0 + tick as f64,
                monotonic_time_ms: 10.0 + tick as f64,
            }
        }
    }

    struct InvalidClock;

    impl TraceClock for InvalidClock {
        fn now(&self) -> TraceTime {
            TraceTime {
                wall_time_ms: f64::NAN,
                monotonic_time_ms: 10.0,
            }
        }
    }

    struct FakeIds(u64);

    impl TraceIdSource for FakeIds {
        fn next_id(&mut self) -> String {
            let id = self.0;
            self.0 += 1;
            format!("id-{id}")
        }
    }

    fn limits() -> ContextTraceLimits {
        ContextTraceLimits::new(TraceWriterLimits::new(2048, 16 * 1024, 16), 64, 256, 4, 4)
    }

    #[test]
    fn requirement_reports_exact_per_context_trace_lease() {
        // Arrange
        let configured = limits();
        let expected = 2048 + 6 * 64 + 4 * std::mem::size_of::<String>();

        // Act
        let required = configured.required_trace_bytes().unwrap();

        // Assert
        assert_eq!(required, expected as u64);
    }

    fn setup(
        state: Rc<RefCell<OutputState>>,
        limits: ContextTraceLimits,
    ) -> (
        ContextTraceSession<FakeOutput, FakeClock, FakeIds>,
        ResourceLedger,
    ) {
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::TraceBytes,
            100_000,
            100_000,
        ));
        let session = ContextTraceSession::new(
            FakeOutput(state),
            FakeClock(Cell::new(0)),
            FakeIds(1),
            ledger.clone(),
            ResourceScope::Object(ObjectId::new(7)),
            limits,
        )
        .unwrap();
        (session, ledger)
    }

    fn metadata(options: &Value) -> TraceContextMetadata<'_> {
        TraceContextMetadata {
            origin: TraceOrigin::Library,
            browser_name: "chromium",
            channel: None,
            platform: "darwin",
            options,
            sdk_language: Some(SdkLanguage::JavaScript),
            test_id_attribute_name: Some("data-testid"),
            context_id: Some("context@7"),
            playwright_version: Some("1.58.0"),
        }
    }

    fn parsed_lines(bytes: &[u8]) -> Vec<Value> {
        bytes
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| serde_json::from_slice(line).unwrap())
            .collect()
    }

    #[test]
    fn inactive_stop_without_export_is_noop_but_export_requires_start() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let (mut session, ledger) = setup(state.clone(), limits());

        // Act
        session.stop(ChunkDisposition::Discard).unwrap();
        let error = session
            .stop(ChunkDisposition::Export(Path::new("trace.zip")))
            .unwrap_err();

        // Assert
        assert!(matches!(error, ContextTraceError::NotStarted));
        assert_eq!(session.state(), ContextTraceState::Idle);
        assert!(state.borrow().opened.is_empty());
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    }

    #[test]
    fn inactive_start_chunk_uses_playwright_error_and_opens_nothing() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let (mut session, ledger) = setup(state.clone(), limits());
        let options = json!({});

        // Act
        let error = session
            .start_chunk(&metadata(&options), StartChunkOptions::default())
            .unwrap_err();

        // Assert
        assert!(matches!(error, ContextTraceError::NotStartedChunk));
        assert_eq!(
            error.to_string(),
            "Must start tracing before starting a new chunk"
        );
        assert_eq!(session.state(), ContextTraceState::Idle);
        assert!(state.borrow().opened.is_empty());
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    }

    #[test]
    fn start_writes_first_header_and_stop_releases_lease_for_restart() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let (mut session, ledger) = setup(state.clone(), limits());
        let options = json!({"viewport": {"width": 1280, "height": 720}});

        // Act
        session
            .start(&metadata(&options), StartOptions::default())
            .unwrap();
        let active_leases = ledger.metrics().unwrap().active_leases;
        session.stop(ChunkDisposition::Discard).unwrap();
        session
            .start(&metadata(&options), StartOptions::default())
            .unwrap();
        session.stop(ChunkDisposition::Discard).unwrap();

        // Assert
        assert_eq!(active_leases, 1);
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
        assert_eq!(session.state(), ContextTraceState::Idle);
        let state = state.borrow();
        assert_eq!(
            state.opened,
            [("id-1".to_owned(), 0), ("id-2".to_owned(), 0)]
        );
        assert_eq!(state.finished_sessions, 2);
        assert_eq!(state.chunks.len(), 2);
        let lines = parsed_lines(&state.chunks[0].bytes);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0]["version"], 8);
        assert_eq!(lines[0]["type"], "context-options");
        assert_eq!(lines[0]["options"], options);
        assert_eq!(lines[0]["wallTime"], 1000.0);
    }

    #[test]
    fn duplicate_start_and_unsupported_producers_do_not_change_recording() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let (mut session, _) = setup(state.clone(), limits());
        let options = json!({});
        let snapshots = StartOptions {
            snapshots: true,
            ..StartOptions::default()
        };

        // Act
        let unsupported = session.start(&metadata(&options), snapshots).unwrap_err();
        session
            .start(&metadata(&options), StartOptions::default())
            .unwrap();
        let duplicate = session
            .start(&metadata(&options), StartOptions::default())
            .unwrap_err();

        // Assert
        assert!(matches!(
            unsupported,
            ContextTraceError::UnsupportedProducer("snapshots")
        ));
        assert!(matches!(duplicate, ContextTraceError::AlreadyStarted));
        assert_eq!(session.state(), ContextTraceState::Recording);
        assert_eq!(state.borrow().opened.len(), 1);
        session.abort().unwrap();
    }

    #[test]
    fn replacing_chunk_discards_prior_and_auto_closes_nested_groups() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let (mut session, _) = setup(state.clone(), limits());
        let options = json!({});

        // Act
        session
            .start(
                &metadata(&options),
                StartOptions {
                    name: Some("name1"),
                    ..StartOptions::default()
                },
            )
            .unwrap();
        session
            .group(
                "outer",
                Some(GroupLocation {
                    file: "test.ts",
                    line: Some(2),
                    column: Some(3),
                }),
            )
            .unwrap();
        session.group("inner", None).unwrap();
        session
            .start_chunk(
                &metadata(&options),
                StartChunkOptions {
                    name: Some("name2"),
                    title: Some("part 2"),
                },
            )
            .unwrap();
        session
            .stop(ChunkDisposition::Export(Path::new("trace.zip")))
            .unwrap();

        // Assert
        assert_eq!(session.state(), ContextTraceState::Idle);
        let state = state.borrow();
        assert_eq!(state.chunks.len(), 2);
        assert_eq!(
            (&*state.chunks[0].name, state.chunks[0].ordinal),
            ("name1", 0)
        );
        assert_eq!(
            (&*state.chunks[1].name, state.chunks[1].ordinal),
            ("name2", 0)
        );
        assert_eq!(state.chunks[0].export, None);
        assert_eq!(
            state.chunks[1].export.as_deref(),
            Some(Path::new("trace.zip"))
        );
        let first = parsed_lines(&state.chunks[0].bytes);
        assert_eq!(
            first
                .iter()
                .map(|line| line["type"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ["context-options", "before", "before", "after", "after"]
        );
        assert_eq!(first[2]["parentId"], first[1]["callId"]);
        assert_eq!(first[3]["callId"], first[2]["callId"]);
        assert_eq!(first[4]["callId"], first[1]["callId"]);
        assert_eq!(first[1]["stack"][0]["file"], "test.ts");
        assert_eq!(first[2]["stack"], json!([]));
        let second = parsed_lines(&state.chunks[1].bytes);
        assert_eq!(second[0]["title"], "part 2");
    }

    #[test]
    fn same_name_chunks_increment_ordinal_and_inactive_group_is_ignored() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let (mut session, _) = setup(state.clone(), limits());
        let options = json!({});

        // Act
        session.group("ignored", None).unwrap();
        session
            .start(
                &metadata(&options),
                StartOptions {
                    name: Some("named"),
                    ..StartOptions::default()
                },
            )
            .unwrap();
        session.stop_chunk(ChunkDisposition::Discard).unwrap();
        session.group("ignored-between", None).unwrap();
        session
            .start_chunk(&metadata(&options), StartChunkOptions::default())
            .unwrap();
        session.group_end().unwrap();
        session.stop(ChunkDisposition::Discard).unwrap();

        // Assert
        assert_eq!(
            state.borrow().opened,
            [("named".to_owned(), 0), ("named".to_owned(), 1)]
        );
        assert_eq!(parsed_lines(&state.borrow().chunks[1].bytes).len(), 1);
    }

    #[test]
    fn api_before_inherits_current_group_as_parent() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let (mut session, _) = setup(state.clone(), limits());
        let options = json!({});
        let params = json!({});
        session
            .start(&metadata(&options), StartOptions::default())
            .unwrap();
        session.group("step", None).unwrap();

        // Act
        session
            .record_api(ApiTraceEvent::Before(BeforeAction {
                call_id: "api-call",
                start_time_ms: 11.0,
                title: None,
                class_name: "Page",
                method: "goto",
                params: &params,
                step_id: None,
                before_snapshot: None,
                stack: None,
                page_id: None,
                parent_id: None,
            }))
            .unwrap();
        session.stop(ChunkDisposition::Discard).unwrap();

        // Assert
        let state = state.borrow();
        let lines = parsed_lines(&state.chunks[0].bytes);
        assert_eq!(lines[1]["type"], "before");
        assert_eq!(lines[2]["parentId"], lines[1]["callId"]);
    }

    #[test]
    fn partial_header_write_poisoned_state_releases_lease_and_can_abort() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState {
            fail_after_bytes: Some(3),
            ..OutputState::default()
        }));
        let (mut session, ledger) = setup(state.clone(), limits());
        let options = json!({});

        // Act
        let error = session
            .start(&metadata(&options), StartOptions::default())
            .unwrap_err();
        let poisoned = session
            .start(&metadata(&options), StartOptions::default())
            .unwrap_err();
        session.abort().unwrap();

        // Assert
        assert!(matches!(error, ContextTraceError::Writer(_)));
        assert!(matches!(poisoned, ContextTraceError::Poisoned));
        assert_eq!(session.state(), ContextTraceState::Idle);
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
        assert_eq!(state.borrow().aborted_sessions, 2);
        assert!(state.borrow().chunks.is_empty());
    }

    #[test]
    fn open_failure_and_cleanup_failure_are_visible_without_retaining_lease() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState {
            fail_open: true,
            fail_abort_once: true,
            ..OutputState::default()
        }));
        let (mut session, ledger) = setup(state.clone(), limits());
        let options = json!({});

        // Act
        let error = session
            .start(&metadata(&options), StartOptions::default())
            .unwrap_err();
        let abort = session.abort();

        // Assert
        assert!(matches!(
            error,
            ContextTraceError::CleanupAfterFailure { .. }
        ));
        assert!(abort.is_ok());
        assert_eq!(session.state(), ContextTraceState::Idle);
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
        assert_eq!(state.borrow().aborted_sessions, 2);
    }

    #[test]
    fn writer_event_limit_poisoned_state_releases_lease() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let mut limited = limits();
        limited.writer.max_events = 2;
        let (mut session, ledger) = setup(state.clone(), limited);
        let options = json!({});
        let params = json!({});
        let event = || {
            ApiTraceEvent::Before(BeforeAction {
                call_id: "call-1",
                start_time_ms: 11.0,
                title: None,
                class_name: "Page",
                method: "goto",
                params: &params,
                step_id: None,
                before_snapshot: None,
                stack: None,
                page_id: None,
                parent_id: None,
            })
        };
        session
            .start(&metadata(&options), StartOptions::default())
            .unwrap();

        // Act
        session.record_api(event()).unwrap();
        let error = session.record_api(event()).unwrap_err();

        // Assert
        assert!(matches!(error, ContextTraceError::Writer(_)));
        assert_eq!(session.state(), ContextTraceState::Poisoned);
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
        assert_eq!(state.borrow().aborted_sessions, 1);
    }

    #[test]
    fn group_depth_and_invalid_next_name_reject_without_discarding_active_chunk() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let mut limited = limits();
        limited.max_group_depth = 1;
        let (mut session, _) = setup(state.clone(), limited);
        let options = json!({});
        session
            .start(&metadata(&options), StartOptions::default())
            .unwrap();
        session.group("outer", None).unwrap();

        // Act
        let depth = session.group("inner", None).unwrap_err();
        let invalid = session
            .start_chunk(
                &metadata(&options),
                StartChunkOptions {
                    name: Some("../escape"),
                    title: None,
                },
            )
            .unwrap_err();

        // Assert
        assert!(matches!(
            depth,
            ContextTraceError::GroupDepthExceeded { limit: 1 }
        ));
        assert!(matches!(invalid, ContextTraceError::InvalidName));
        assert_eq!(session.state(), ContextTraceState::Recording);
        assert!(state.borrow().chunks.is_empty());
        session.abort().unwrap();
    }

    #[test]
    fn failed_export_poisoned_state_aborts_output_and_releases_lease() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState {
            fail_finish_chunk: true,
            ..OutputState::default()
        }));
        let (mut session, ledger) = setup(state.clone(), limits());
        let options = json!({});
        session
            .start(&metadata(&options), StartOptions::default())
            .unwrap();

        // Act
        let error = session
            .stop(ChunkDisposition::Export(Path::new("trace.zip")))
            .unwrap_err();

        // Assert
        assert!(matches!(error, ContextTraceError::Output(_)));
        assert_eq!(session.state(), ContextTraceState::Poisoned);
        assert_eq!(state.borrow().aborted_sessions, 1);
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    }

    #[test]
    fn next_chunk_open_failure_poisoned_state_releases_lease_after_discard() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let (mut session, ledger) = setup(state.clone(), limits());
        let options = json!({});
        session
            .start(&metadata(&options), StartOptions::default())
            .unwrap();
        state.borrow_mut().fail_open = true;

        // Act
        let error = session
            .start_chunk(&metadata(&options), StartChunkOptions::default())
            .unwrap_err();

        // Assert
        assert!(matches!(error, ContextTraceError::Output(_)));
        assert_eq!(session.state(), ContextTraceState::Poisoned);
        assert_eq!(state.borrow().chunks.len(), 1);
        assert_eq!(state.borrow().aborted_sessions, 1);
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    }

    #[test]
    fn insufficient_trace_lease_rejects_before_open_without_poisoning() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::TraceBytes,
            1,
            1,
        ));
        let mut session = ContextTraceSession::new(
            FakeOutput(state.clone()),
            FakeClock(Cell::new(0)),
            FakeIds(1),
            ledger.clone(),
            ResourceScope::Object(ObjectId::new(7)),
            limits(),
        )
        .unwrap();
        let options = json!({});

        // Act
        let error = session
            .start(&metadata(&options), StartOptions::default())
            .unwrap_err();

        // Assert
        assert!(matches!(error, ContextTraceError::Resource(_)));
        assert_eq!(session.state(), ContextTraceState::Idle);
        assert!(state.borrow().opened.is_empty());
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    }

    #[test]
    fn oversized_requested_start_name_rejects_before_open_or_lease() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let (mut session, ledger) = setup(state.clone(), limits());
        let options = json!({});
        let oversized = "x".repeat(65);

        // Act
        let error = session
            .start(
                &metadata(&options),
                StartOptions {
                    name: Some(&oversized),
                    ..StartOptions::default()
                },
            )
            .unwrap_err();

        // Assert
        assert!(matches!(error, ContextTraceError::InvalidName));
        assert_eq!(session.state(), ContextTraceState::Idle);
        assert!(state.borrow().opened.is_empty());
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    }

    #[test]
    fn invalid_limits_fail_before_any_output_or_lease() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1));
        let mut invalid = limits();
        invalid.writer.max_total_bytes = 0;

        // Act
        let result = ContextTraceSession::new(
            FakeOutput(state.clone()),
            FakeClock(Cell::new(0)),
            FakeIds(1),
            ledger.clone(),
            ResourceScope::Object(ObjectId::new(7)),
            invalid,
        );

        // Assert
        assert!(matches!(
            result,
            Err(ContextTraceError::ZeroLimit("total bytes"))
        ));
        assert!(state.borrow().opened.is_empty());
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    }

    #[test]
    fn chunk_limit_rejects_before_replacing_current_recording() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let mut limited = limits();
        limited.max_chunks_per_session = 1;
        let (mut session, ledger) = setup(state.clone(), limited);
        let options = json!({});
        session
            .start(&metadata(&options), StartOptions::default())
            .unwrap();

        // Act
        let error = session
            .start_chunk(&metadata(&options), StartChunkOptions::default())
            .unwrap_err();

        // Assert
        assert!(matches!(
            error,
            ContextTraceError::ChunkCapacityExceeded { limit: 1 }
        ));
        assert_eq!(session.state(), ContextTraceState::Recording);
        assert!(state.borrow().chunks.is_empty());
        assert_eq!(ledger.metrics().unwrap().active_leases, 1);
        session.abort().unwrap();
    }

    #[test]
    fn nonfinite_clock_poisoned_state_aborts_partial_output_and_releases_lease() {
        // Arrange
        let state = Rc::new(RefCell::new(OutputState::default()));
        let ledger = ResourceLedger::new(ResourceLimits::deny_all(1, 1).with_class(
            ResourceClass::TraceBytes,
            100_000,
            100_000,
        ));
        let mut session = ContextTraceSession::new(
            FakeOutput(state.clone()),
            InvalidClock,
            FakeIds(1),
            ledger.clone(),
            ResourceScope::Object(ObjectId::new(7)),
            limits(),
        )
        .unwrap();
        let options = json!({});

        // Act
        let error = session
            .start(&metadata(&options), StartOptions::default())
            .unwrap_err();

        // Assert
        assert!(matches!(error, ContextTraceError::Writer(_)));
        assert_eq!(session.state(), ContextTraceState::Poisoned);
        assert_eq!(state.borrow().aborted_sessions, 1);
        assert!(state.borrow().chunks.is_empty());
        assert_eq!(ledger.metrics().unwrap().active_leases, 0);
    }
}
