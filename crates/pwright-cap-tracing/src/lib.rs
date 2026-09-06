//! Portable, opt-in BrowserContext tracing lifecycle contracts.
//!
//! This capability controls bounded metadata chunks. It does not install a
//! sink, filesystem, archive writer, DOM/ARIA snapshotter, screenshot or
//! screencast producer, HAR recorder, source collector, runtime, or task.

#[doc(hidden)]
#[rustfmt::skip]
mod generated_capability;

pub use generated_capability::*;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TraceStartOptions {
    name: Option<String>,
}

impl TraceStartOptions {
    pub fn new(name: Option<String>) -> Self {
        Self { name }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn into_name(self) -> Option<String> {
        self.name
    }
}

pub type TraceStartResult = ();

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TraceStartChunkOptions {
    name: Option<String>,
    title: Option<String>,
}

impl TraceStartChunkOptions {
    pub fn new(name: Option<String>, title: Option<String>) -> Self {
        Self { name, title }
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn into_parts(self) -> (Option<String>, Option<String>) {
        (self.name, self.title)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceStartChunkResult {
    chunk_id: u64,
}

impl TraceStartChunkResult {
    pub const fn new(chunk_id: u64) -> Self {
        Self { chunk_id }
    }

    pub const fn chunk_id(self) -> u64 {
        self.chunk_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceLocation {
    file: String,
    line: Option<u32>,
    column: Option<u32>,
}

impl TraceLocation {
    pub fn new(file: impl Into<String>, line: Option<u32>, column: Option<u32>) -> Self {
        Self {
            file: file.into(),
            line,
            column,
        }
    }

    pub fn file(&self) -> &str {
        &self.file
    }

    pub const fn line(&self) -> Option<u32> {
        self.line
    }

    pub const fn column(&self) -> Option<u32> {
        self.column
    }

    pub fn into_parts(self) -> (String, Option<u32>, Option<u32>) {
        (self.file, self.line, self.column)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceGroupOptions {
    name: String,
    location: Option<TraceLocation>,
}

impl TraceGroupOptions {
    pub fn new(name: impl Into<String>, location: Option<TraceLocation>) -> Self {
        Self {
            name: name.into(),
            location,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub const fn location(&self) -> Option<&TraceLocation> {
        self.location.as_ref()
    }

    pub fn into_parts(self) -> (String, Option<TraceLocation>) {
        (self.name, self.location)
    }
}

pub type TraceGroupResult = ();
pub type TraceGroupEndOptions = ();
pub type TraceGroupEndResult = ();

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceStopChunkMode {
    Discard,
    Sink,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceStopChunkOptions {
    mode: TraceStopChunkMode,
}

impl TraceStopChunkOptions {
    pub const fn new(mode: TraceStopChunkMode) -> Self {
        Self { mode }
    }

    pub const fn mode(self) -> TraceStopChunkMode {
        self.mode
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceStopChunkResult {
    chunk_id: u64,
    entry_count: u64,
    payload_bytes: u64,
}

impl TraceStopChunkResult {
    pub const fn new(chunk_id: u64, entry_count: u64, payload_bytes: u64) -> Self {
        Self {
            chunk_id,
            entry_count,
            payload_bytes,
        }
    }

    pub const fn chunk_id(self) -> u64 {
        self.chunk_id
    }

    pub const fn entry_count(self) -> u64 {
        self.entry_count
    }

    pub const fn payload_bytes(self) -> u64 {
        self.payload_bytes
    }
}

pub type TraceStopOptions = ();
pub type TraceStopResult = ();

#[cfg(test)]
mod tests {
    use pwright_core::{CommandSpec, RegistryBuilder};

    use super::*;

    #[test]
    fn tracing_registers_six_browser_context_only_commands() {
        // Arrange
        let mut builder = RegistryBuilder::new();
        builder
            .register::<TracingCapability>()
            .expect("tracing capability should register");

        // Act
        let registry = builder.build().expect("registry should validate");

        // Assert
        assert!(registry.contains(TRACING_CAPABILITY_ID));
        assert_eq!(TRACING_COMMAND_DESCRIPTORS.len(), 6);
        for descriptor in TRACING_COMMAND_DESCRIPTORS {
            assert_eq!(
                registry.owner_of(descriptor.command()),
                Some(TRACING_CAPABILITY_ID)
            );
            assert_eq!(descriptor.targets().len(), 1);
            assert_eq!(
                descriptor.targets()[0].as_str(),
                "pwright.browser.browser_context"
            );
        }
        assert_eq!(TraceStart::METHOD, TRACE_START_METHOD);
        assert_eq!(TraceStartChunk::METHOD, TRACE_START_CHUNK_METHOD);
        assert_eq!(TraceGroup::METHOD, TRACE_GROUP_METHOD);
        assert_eq!(TraceGroupEnd::METHOD, TRACE_GROUP_END_METHOD);
        assert_eq!(TraceStopChunk::METHOD, TRACE_STOP_CHUNK_METHOD);
        assert_eq!(TraceStop::METHOD, TRACE_STOP_METHOD);
    }

    #[test]
    fn base_contract_exposes_sink_selection_without_artifact_or_snapshot_options() {
        // Arrange
        let start = TraceStartOptions::new(Some("session".to_owned()));
        let group = TraceGroupOptions::new(
            "checkout",
            Some(TraceLocation::new("flow.rs", Some(7), Some(9))),
        );

        // Act
        let stop = TraceStopChunkOptions::new(TraceStopChunkMode::Sink);
        let summary = TraceStopChunkResult::new(3, 4, 128);

        // Assert
        assert_eq!(start.name(), Some("session"));
        assert_eq!(group.location().map(TraceLocation::file), Some("flow.rs"));
        assert_eq!(stop.mode(), TraceStopChunkMode::Sink);
        assert_eq!(
            (
                summary.chunk_id(),
                summary.entry_count(),
                summary.payload_bytes()
            ),
            (3, 4, 128)
        );
    }
}
