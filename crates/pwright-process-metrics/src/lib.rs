//! Optional bounded process-memory attribution for Pwright.
//!
//! This crate samples an operating-system process table and separates one
//! Pwright runtime process from one browser process tree. It neither connects
//! to nor owns the browser and has no browser, async-runtime, protocol, or Node
//! dependency. RSS is reported as an OS observation; it is not a resource
//! limit and may double-count shared pages across processes. The optional
//! `ledger` feature adds bounded caller-driven workload and repeated-series
//! recorders; series ranges remain diagnostics rather than implicit budgets.

use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::io::Read;
use std::process::{Command, Stdio};

#[cfg(feature = "ledger")]
mod workload;

#[cfg(feature = "ledger")]
pub use workload::*;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProcessId(u32);

impl ProcessId {
    pub fn new(value: u32) -> Result<Self, ProcessMetricsError> {
        if value == 0 {
            Err(ProcessMetricsError::InvalidProcessId)
        } else {
            Ok(Self(value))
        }
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

impl TryFrom<u32> for ProcessId {
    type Error = ProcessMetricsError;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessMetricsLimits {
    max_snapshot_bytes: usize,
    max_processes: usize,
    max_command_bytes: usize,
}

impl ProcessMetricsLimits {
    pub fn new(
        max_snapshot_bytes: usize,
        max_processes: usize,
        max_command_bytes: usize,
    ) -> Result<Self, ProcessMetricsError> {
        if max_snapshot_bytes == 0 {
            return Err(ProcessMetricsError::InvalidLimit("snapshot bytes"));
        }
        if max_processes == 0 {
            return Err(ProcessMetricsError::InvalidLimit("processes"));
        }
        if max_command_bytes == 0 {
            return Err(ProcessMetricsError::InvalidLimit("command bytes"));
        }
        Ok(Self {
            max_snapshot_bytes,
            max_processes,
            max_command_bytes,
        })
    }

    pub const fn max_snapshot_bytes(self) -> usize {
        self.max_snapshot_bytes
    }

    pub const fn max_processes(self) -> usize {
        self.max_processes
    }

    pub const fn max_command_bytes(self) -> usize {
        self.max_command_bytes
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProcessRoleMetrics {
    pub processes: usize,
    pub resident_bytes: u64,
}

impl ProcessRoleMetrics {
    fn add(&mut self, resident_bytes: u64) -> Result<(), ProcessMetricsError> {
        self.processes = self
            .processes
            .checked_add(1)
            .ok_or(ProcessMetricsError::AggregateOverflow)?;
        self.resident_bytes = self
            .resident_bytes
            .checked_add(resident_bytes)
            .ok_or(ProcessMetricsError::AggregateOverflow)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RuntimeProcessMetrics {
    pub process_id: ProcessId,
    pub resident_bytes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BrowserProcessMetrics {
    pub process_id: ProcessId,
    pub browser_parent: ProcessRoleMetrics,
    pub renderers: ProcessRoleMetrics,
    pub gpu: ProcessRoleMetrics,
    pub utility: ProcessRoleMetrics,
    pub other_children: ProcessRoleMetrics,
    pub total: ProcessRoleMetrics,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessAttributionMetrics {
    pub runtime: RuntimeProcessMetrics,
    pub browser: BrowserProcessMetrics,
}

/// One runtime sample paired with proof that an independently owned process
/// was absent from the same bounded process-table snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProcessExitMetrics {
    runtime: RuntimeProcessMetrics,
    exited_process_id: ProcessId,
}

impl ProcessExitMetrics {
    pub const fn runtime(self) -> RuntimeProcessMetrics {
        self.runtime
    }

    pub const fn exited_process_id(self) -> ProcessId {
        self.exited_process_id
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BrowserRole {
    Renderer,
    Gpu,
    Utility,
    OtherChild,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProcessRecord {
    process_id: ProcessId,
    parent_process_id: u32,
    resident_bytes: u64,
    role_hint: BrowserRole,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessSnapshot {
    records: Vec<ProcessRecord>,
}

impl ProcessSnapshot {
    pub fn parse(input: &[u8], limits: ProcessMetricsLimits) -> Result<Self, ProcessMetricsError> {
        if input.len() > limits.max_snapshot_bytes {
            return Err(ProcessMetricsError::SnapshotTooLarge {
                actual: input.len(),
                limit: limits.max_snapshot_bytes,
            });
        }
        let input = std::str::from_utf8(input).map_err(|_| ProcessMetricsError::InvalidUtf8)?;
        let mut records = Vec::new();
        for (line_index, line) in input.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if records.len() >= limits.max_processes {
                return Err(ProcessMetricsError::TooManyProcesses {
                    actual: records.len().saturating_add(1),
                    limit: limits.max_processes,
                });
            }
            records.push(parse_record(line, line_index.saturating_add(1), limits)?);
        }
        records.sort_unstable_by_key(|record| record.process_id);
        if let Some(duplicate) = records
            .windows(2)
            .find(|pair| pair[0].process_id == pair[1].process_id)
        {
            return Err(ProcessMetricsError::DuplicateProcessId(
                duplicate[0].process_id,
            ));
        }
        records.shrink_to_fit();
        Ok(Self { records })
    }

    pub fn attribute(
        &self,
        runtime_process_id: ProcessId,
        browser_process_id: ProcessId,
    ) -> Result<ProcessAttributionMetrics, ProcessMetricsError> {
        if runtime_process_id == browser_process_id {
            return Err(ProcessMetricsError::OverlappingRoots(runtime_process_id));
        }
        let by_pid: BTreeMap<_, _> = self
            .records
            .iter()
            .map(|record| (record.process_id, record))
            .collect();
        let runtime = by_pid
            .get(&runtime_process_id)
            .ok_or(ProcessMetricsError::MissingProcess(runtime_process_id))?;
        let browser_parent = by_pid
            .get(&browser_process_id)
            .ok_or(ProcessMetricsError::MissingProcess(browser_process_id))?;
        if is_descendant(runtime, browser_process_id, &by_pid, self.records.len())? {
            return Err(ProcessMetricsError::RuntimeInsideBrowserTree(
                runtime_process_id,
            ));
        }
        let mut browser = BrowserProcessMetrics {
            process_id: browser_process_id,
            browser_parent: ProcessRoleMetrics::default(),
            renderers: ProcessRoleMetrics::default(),
            gpu: ProcessRoleMetrics::default(),
            utility: ProcessRoleMetrics::default(),
            other_children: ProcessRoleMetrics::default(),
            total: ProcessRoleMetrics::default(),
        };
        browser.browser_parent.add(browser_parent.resident_bytes)?;
        browser.total.add(browser_parent.resident_bytes)?;
        for record in &self.records {
            if record.process_id == browser_process_id {
                continue;
            }
            if !is_descendant(record, browser_process_id, &by_pid, self.records.len())? {
                continue;
            }
            let role = match record.role_hint {
                BrowserRole::Renderer => &mut browser.renderers,
                BrowserRole::Gpu => &mut browser.gpu,
                BrowserRole::Utility => &mut browser.utility,
                BrowserRole::OtherChild => &mut browser.other_children,
            };
            role.add(record.resident_bytes)?;
            browser.total.add(record.resident_bytes)?;
        }
        Ok(ProcessAttributionMetrics {
            runtime: RuntimeProcessMetrics {
                process_id: runtime_process_id,
                resident_bytes: runtime.resident_bytes,
            },
            browser,
        })
    }

    /// Samples the runtime while proving another process is absent from this
    /// exact snapshot. This is the postcondition needed by an owned-browser
    /// teardown; ordinary external-browser observations continue to require
    /// both roots to exist.
    pub fn attribute_runtime_after_exit(
        &self,
        runtime_process_id: ProcessId,
        exited_process_id: ProcessId,
    ) -> Result<ProcessExitMetrics, ProcessMetricsError> {
        if runtime_process_id == exited_process_id {
            return Err(ProcessMetricsError::OverlappingRoots(runtime_process_id));
        }
        let runtime = self
            .records
            .binary_search_by_key(&runtime_process_id, |record| record.process_id)
            .ok()
            .map(|index| self.records[index])
            .ok_or(ProcessMetricsError::MissingProcess(runtime_process_id))?;
        if self
            .records
            .binary_search_by_key(&exited_process_id, |record| record.process_id)
            .is_ok()
        {
            return Err(ProcessMetricsError::ProcessStillRunning(exited_process_id));
        }
        Ok(ProcessExitMetrics {
            runtime: RuntimeProcessMetrics {
                process_id: runtime_process_id,
                resident_bytes: runtime.resident_bytes,
            },
            exited_process_id,
        })
    }
}

pub trait ProcessTableSource: Send + Sync {
    fn snapshot(&self, max_bytes: usize) -> Result<Vec<u8>, ProcessMetricsError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PsProcessTable;

impl ProcessTableSource for PsProcessTable {
    fn snapshot(&self, max_bytes: usize) -> Result<Vec<u8>, ProcessMetricsError> {
        if max_bytes == 0 {
            return Err(ProcessMetricsError::InvalidLimit("snapshot bytes"));
        }
        #[cfg(unix)]
        {
            let mut child = Command::new("ps")
                .args(["-axo", "pid=,ppid=,rss=,command="])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .map_err(|error| ProcessMetricsError::SourceIo(error.kind()))?;
            let mut output = Vec::with_capacity(max_bytes.min(64 * 1024).saturating_add(1));
            let read_limit = u64::try_from(max_bytes)
                .unwrap_or(u64::MAX)
                .saturating_add(1);
            let read = child
                .stdout
                .take()
                .expect("piped ps stdout should exist")
                .take(read_limit)
                .read_to_end(&mut output);
            if let Err(error) = read {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProcessMetricsError::SourceIo(error.kind()));
            }
            if output.len() > max_bytes {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProcessMetricsError::SnapshotTooLarge {
                    actual: output.len(),
                    limit: max_bytes,
                });
            }
            let status = child
                .wait()
                .map_err(|error| ProcessMetricsError::SourceIo(error.kind()))?;
            if !status.success() {
                return Err(ProcessMetricsError::SourceExit(status.code()));
            }
            output.shrink_to_fit();
            Ok(output)
        }
        #[cfg(not(unix))]
        {
            let _ = max_bytes;
            Err(ProcessMetricsError::UnsupportedPlatform)
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProcessMetricsProbe<Source = PsProcessTable> {
    source: Source,
    limits: ProcessMetricsLimits,
}

impl ProcessMetricsProbe<PsProcessTable> {
    pub const fn system(limits: ProcessMetricsLimits) -> Self {
        Self {
            source: PsProcessTable,
            limits,
        }
    }
}

impl<Source> ProcessMetricsProbe<Source>
where
    Source: ProcessTableSource,
{
    pub const fn new(source: Source, limits: ProcessMetricsLimits) -> Self {
        Self { source, limits }
    }

    pub fn sample(
        &self,
        runtime_process_id: ProcessId,
        browser_process_id: ProcessId,
    ) -> Result<ProcessAttributionMetrics, ProcessMetricsError> {
        let bytes = self.source.snapshot(self.limits.max_snapshot_bytes)?;
        ProcessSnapshot::parse(&bytes, self.limits)?
            .attribute(runtime_process_id, browser_process_id)
    }

    pub fn sample_current_runtime(
        &self,
        browser_process_id: ProcessId,
    ) -> Result<ProcessAttributionMetrics, ProcessMetricsError> {
        self.sample(ProcessId::new(std::process::id())?, browser_process_id)
    }

    pub fn sample_current_runtime_after_exit(
        &self,
        exited_process_id: ProcessId,
    ) -> Result<ProcessExitMetrics, ProcessMetricsError> {
        let bytes = self.source.snapshot(self.limits.max_snapshot_bytes)?;
        ProcessSnapshot::parse(&bytes, self.limits)?
            .attribute_runtime_after_exit(ProcessId::new(std::process::id())?, exited_process_id)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProcessMetricsError {
    InvalidLimit(&'static str),
    InvalidProcessId,
    SnapshotTooLarge {
        actual: usize,
        limit: usize,
    },
    InvalidUtf8,
    TooManyProcesses {
        actual: usize,
        limit: usize,
    },
    CommandTooLong {
        line: usize,
        actual: usize,
        limit: usize,
    },
    MalformedLine {
        line: usize,
        field: &'static str,
    },
    RssOverflow {
        line: usize,
    },
    DuplicateProcessId(ProcessId),
    MissingProcess(ProcessId),
    ProcessStillRunning(ProcessId),
    OverlappingRoots(ProcessId),
    RuntimeInsideBrowserTree(ProcessId),
    ParentCycle(ProcessId),
    AggregateOverflow,
    SourceIo(std::io::ErrorKind),
    SourceExit(Option<i32>),
    UnsupportedPlatform,
}

impl Display for ProcessMetricsError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidLimit(name) => write!(formatter, "{name} limit must be non-zero"),
            Self::InvalidProcessId => formatter.write_str("process ID must be non-zero"),
            Self::SnapshotTooLarge { actual, limit } => write!(
                formatter,
                "process snapshot has {actual} bytes, exceeding limit {limit}"
            ),
            Self::InvalidUtf8 => formatter.write_str("process snapshot is not UTF-8"),
            Self::TooManyProcesses { actual, limit } => write!(
                formatter,
                "process snapshot has {actual} records, exceeding limit {limit}"
            ),
            Self::CommandTooLong {
                line,
                actual,
                limit,
            } => write!(
                formatter,
                "process command on line {line} has {actual} bytes, exceeding limit {limit}"
            ),
            Self::MalformedLine { line, field } => {
                write!(
                    formatter,
                    "process snapshot line {line} has invalid {field}"
                )
            }
            Self::RssOverflow { line } => {
                write!(formatter, "process RSS on line {line} overflows bytes")
            }
            Self::DuplicateProcessId(process_id) => {
                write!(
                    formatter,
                    "process snapshot repeats PID {}",
                    process_id.get()
                )
            }
            Self::MissingProcess(process_id) => {
                write!(
                    formatter,
                    "process snapshot has no PID {}",
                    process_id.get()
                )
            }
            Self::ProcessStillRunning(process_id) => write!(
                formatter,
                "process snapshot still contains PID {}",
                process_id.get()
            ),
            Self::OverlappingRoots(process_id) => write!(
                formatter,
                "runtime and browser roots both name PID {}",
                process_id.get()
            ),
            Self::RuntimeInsideBrowserTree(process_id) => write!(
                formatter,
                "runtime PID {} is inside the browser process tree",
                process_id.get()
            ),
            Self::ParentCycle(process_id) => write!(
                formatter,
                "process parent graph contains a cycle at PID {}",
                process_id.get()
            ),
            Self::AggregateOverflow => formatter.write_str("process metric aggregate overflowed"),
            Self::SourceIo(kind) => write!(formatter, "process table source failed: {kind:?}"),
            Self::SourceExit(code) => {
                write!(formatter, "process table source exited with {code:?}")
            }
            Self::UnsupportedPlatform => {
                formatter.write_str("the system process-table source is unsupported")
            }
        }
    }
}

impl std::error::Error for ProcessMetricsError {}

fn parse_record(
    line: &str,
    line_number: usize,
    limits: ProcessMetricsLimits,
) -> Result<ProcessRecord, ProcessMetricsError> {
    let mut cursor = line;
    let pid = parse_field::<u32>(take_token(&mut cursor), line_number, "PID")?;
    let parent_pid = parse_field::<u32>(take_token(&mut cursor), line_number, "parent PID")?;
    let rss_kib = parse_field::<u64>(take_token(&mut cursor), line_number, "RSS")?;
    let command = cursor.trim_start();
    if command.len() > limits.max_command_bytes {
        return Err(ProcessMetricsError::CommandTooLong {
            line: line_number,
            actual: command.len(),
            limit: limits.max_command_bytes,
        });
    }
    let resident_bytes = rss_kib
        .checked_mul(1024)
        .ok_or(ProcessMetricsError::RssOverflow { line: line_number })?;
    Ok(ProcessRecord {
        process_id: ProcessId::new(pid).map_err(|_| ProcessMetricsError::MalformedLine {
            line: line_number,
            field: "PID",
        })?,
        parent_process_id: parent_pid,
        resident_bytes,
        role_hint: classify_browser_role(command),
    })
}

fn take_token<'a>(cursor: &mut &'a str) -> Option<&'a str> {
    *cursor = cursor.trim_start();
    if cursor.is_empty() {
        return None;
    }
    let end = cursor.find(char::is_whitespace).unwrap_or(cursor.len());
    let token = &cursor[..end];
    *cursor = &cursor[end..];
    Some(token)
}

fn parse_field<T>(
    value: Option<&str>,
    line: usize,
    field: &'static str,
) -> Result<T, ProcessMetricsError>
where
    T: std::str::FromStr,
{
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|value| value.parse().ok())
        .ok_or(ProcessMetricsError::MalformedLine { line, field })
}

fn classify_browser_role(command: &str) -> BrowserRole {
    if command.contains("--type=renderer") {
        BrowserRole::Renderer
    } else if command.contains("--type=gpu-process") {
        BrowserRole::Gpu
    } else if command.contains("--type=utility") {
        BrowserRole::Utility
    } else {
        BrowserRole::OtherChild
    }
}

fn is_descendant(
    record: &ProcessRecord,
    root: ProcessId,
    by_pid: &BTreeMap<ProcessId, &ProcessRecord>,
    maximum_hops: usize,
) -> Result<bool, ProcessMetricsError> {
    let mut parent = record.parent_process_id;
    for _ in 0..maximum_hops {
        if parent == root.get() {
            return Ok(true);
        }
        if parent == 0 {
            return Ok(false);
        }
        let parent_id = ProcessId::new(parent)?;
        if parent_id == record.process_id {
            return Err(ProcessMetricsError::ParentCycle(record.process_id));
        }
        let Some(parent_record) = by_pid.get(&parent_id) else {
            return Ok(false);
        };
        parent = parent_record.parent_process_id;
    }
    Err(ProcessMetricsError::ParentCycle(record.process_id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limits() -> ProcessMetricsLimits {
        ProcessMetricsLimits::new(4096, 16, 256).expect("test limits should validate")
    }

    #[test]
    fn attributes_runtime_and_each_browser_process_role_without_unrelated_processes() {
        // Arrange
        let snapshot = ProcessSnapshot::parse(
            b"10 1 100 pwright-server\n20 1 200 Google Chrome --remote-debugging-port=9222\n21 20 30 Google Chrome Helper --type=renderer\n22 20 40 Google Chrome Helper --type=gpu-process\n23 20 50 Google Chrome Helper --type=utility\n24 21 60 crashpad helper\n99 1 999 unrelated\n",
            limits(),
        )
        .expect("snapshot should parse");

        // Act
        let metrics = snapshot
            .attribute(ProcessId::new(10).unwrap(), ProcessId::new(20).unwrap())
            .expect("roots should attribute");

        // Assert
        assert_eq!(metrics.runtime.resident_bytes, 100 * 1024);
        assert_eq!(metrics.browser.browser_parent.resident_bytes, 200 * 1024);
        assert_eq!(metrics.browser.renderers.resident_bytes, 30 * 1024);
        assert_eq!(metrics.browser.gpu.resident_bytes, 40 * 1024);
        assert_eq!(metrics.browser.utility.resident_bytes, 50 * 1024);
        assert_eq!(metrics.browser.other_children.resident_bytes, 60 * 1024);
        assert_eq!(metrics.browser.total.processes, 5);
        assert_eq!(metrics.browser.total.resident_bytes, 380 * 1024);
    }

    #[test]
    fn process_count_limit_fails_before_retaining_an_extra_record() {
        // Arrange
        let limits = ProcessMetricsLimits::new(1024, 1, 64).unwrap();

        // Act
        let error = ProcessSnapshot::parse(b"1 0 1 one\n2 0 1 two\n", limits)
            .expect_err("second record should exceed the count limit");

        // Assert
        assert_eq!(
            error,
            ProcessMetricsError::TooManyProcesses {
                actual: 2,
                limit: 1
            }
        );
    }

    #[test]
    fn snapshot_byte_limit_fails_before_parsing() {
        // Arrange
        let snapshot_limits = ProcessMetricsLimits::new(4, 1, 4).unwrap();

        // Act
        let snapshot_error = ProcessSnapshot::parse(b"1 0 1 x", snapshot_limits)
            .expect_err("snapshot should exceed its byte limit");

        // Assert
        assert!(matches!(
            snapshot_error,
            ProcessMetricsError::SnapshotTooLarge { .. }
        ));
    }

    #[test]
    fn command_byte_limit_fails_without_retaining_the_command() {
        // Arrange
        let limits = ProcessMetricsLimits::new(64, 1, 3).unwrap();

        // Act
        let error = ProcessSnapshot::parse(b"1 0 1 long", limits)
            .expect_err("command should exceed its byte limit");

        // Assert
        assert!(matches!(error, ProcessMetricsError::CommandTooLong { .. }));
    }

    #[test]
    fn duplicate_process_ids_are_rejected() {
        // Arrange
        let duplicate = b"10 1 1 one\n10 1 2 two\n";

        // Act
        let duplicate_error =
            ProcessSnapshot::parse(duplicate, limits()).expect_err("duplicate PID should fail");

        // Assert
        assert!(matches!(
            duplicate_error,
            ProcessMetricsError::DuplicateProcessId(_)
        ));
    }

    #[test]
    fn cyclic_process_graph_is_rejected() {
        // Arrange
        let snapshot = ProcessSnapshot::parse(
            b"10 1 1 runtime\n20 1 2 browser\n21 22 3 child\n22 21 4 child\n",
            limits(),
        )
        .unwrap();

        // Act
        let cycle_error = snapshot
            .attribute(ProcessId::new(10).unwrap(), ProcessId::new(20).unwrap())
            .expect_err("parent cycle should fail");

        // Assert
        assert!(matches!(cycle_error, ProcessMetricsError::ParentCycle(_)));
    }

    #[test]
    fn runtime_and_browser_roots_must_be_distinct() {
        // Arrange
        let snapshot = ProcessSnapshot::parse(b"10 1 1 runtime\n20 1 2 browser\n", limits())
            .expect("snapshot should parse");

        // Act
        let overlap = snapshot
            .attribute(ProcessId::new(10).unwrap(), ProcessId::new(10).unwrap())
            .expect_err("overlapping roots should fail");

        // Assert
        assert!(matches!(overlap, ProcessMetricsError::OverlappingRoots(_)));
    }

    #[test]
    fn both_attribution_roots_must_exist_in_the_snapshot() {
        // Arrange
        let snapshot = ProcessSnapshot::parse(b"10 1 1 runtime\n20 1 2 browser\n", limits())
            .expect("snapshot should parse");

        // Act
        let missing = snapshot
            .attribute(ProcessId::new(10).unwrap(), ProcessId::new(30).unwrap())
            .expect_err("missing browser should fail");

        // Assert
        assert!(matches!(missing, ProcessMetricsError::MissingProcess(_)));
    }

    #[test]
    fn owned_process_exit_is_proven_by_the_same_runtime_snapshot() {
        // Arrange
        let snapshot = ProcessSnapshot::parse(b"10 1 7 runtime\n30 1 2 other\n", limits())
            .expect("snapshot should parse");

        // Act
        let proof = snapshot
            .attribute_runtime_after_exit(ProcessId::new(10).unwrap(), ProcessId::new(20).unwrap())
            .expect("missing owned process should produce exit proof");

        // Assert
        assert_eq!(proof.runtime().process_id, ProcessId::new(10).unwrap());
        assert_eq!(proof.runtime().resident_bytes, 7 * 1024);
        assert_eq!(proof.exited_process_id(), ProcessId::new(20).unwrap());
    }

    #[test]
    fn owned_process_exit_rejects_a_still_live_pid() {
        // Arrange
        let snapshot = ProcessSnapshot::parse(b"10 1 7 runtime\n20 1 9 browser\n", limits())
            .expect("snapshot should parse");

        // Act
        let error = snapshot
            .attribute_runtime_after_exit(ProcessId::new(10).unwrap(), ProcessId::new(20).unwrap())
            .expect_err("live browser PID must not produce exit proof");

        // Assert
        assert_eq!(
            error,
            ProcessMetricsError::ProcessStillRunning(ProcessId::new(20).unwrap())
        );
    }

    #[test]
    fn runtime_process_must_not_be_a_browser_descendant() {
        // Arrange
        let snapshot = ProcessSnapshot::parse(b"10 20 1 runtime\n20 1 2 browser\n", limits())
            .expect("snapshot should parse");

        // Act
        let error = snapshot
            .attribute(ProcessId::new(10).unwrap(), ProcessId::new(20).unwrap())
            .expect_err("runtime inside the browser tree should fail");

        // Assert
        assert!(matches!(
            error,
            ProcessMetricsError::RuntimeInsideBrowserTree(_)
        ));
    }
}
