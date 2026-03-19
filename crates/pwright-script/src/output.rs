//! Output sink for streaming JSONL results.

use std::collections::HashMap;
use std::io::Write;

use serde::Serialize;

use crate::executor::{ExecutionResult, ExecutionStatus};

/// A result emitted for each completed step.
#[derive(Debug, Clone, Serialize)]
pub struct StepResult {
    pub step_index: u32,
    pub step_type: String,
    pub status: ExecutionStatus,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub details: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Final script execution summary.
#[derive(Debug, Clone, Serialize)]
pub struct ScriptSummary {
    pub summary: bool,
    pub name: String,
    pub status: ExecutionStatus,
    pub total_steps: u32,
    pub succeeded: u32,
    pub failed: u32,
    pub skipped: u32,
    pub duration_ms: u64,
    pub outputs: Vec<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Trait for receiving step results during execution.
pub trait OutputSink {
    /// Emit a step result. Returns an error if the result cannot be written.
    fn emit(&mut self, result: StepResult) -> std::io::Result<()>;
}

/// JSONL output: one JSON line per step to a writer.
pub struct JsonlSink<W: Write> {
    writer: W,
}

impl<W: Write> JsonlSink<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn write_summary(&mut self, name: &str, result: &ExecutionResult) -> std::io::Result<()> {
        let summary = ScriptSummary {
            summary: true,
            name: name.to_string(),
            status: result.status,
            total_steps: result.total_steps,
            succeeded: result.succeeded,
            failed: result.failed,
            skipped: result.skipped,
            duration_ms: result.duration_ms,
            outputs: result.outputs.clone(),
            error: result.error.clone(),
        };
        let json = serde_json::to_string(&summary)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(self.writer, "{json}")
    }
}

impl<W: Write> OutputSink for JsonlSink<W> {
    fn emit(&mut self, result: StepResult) -> std::io::Result<()> {
        let json = serde_json::to_string(&result)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        writeln!(self.writer, "{json}")
    }
}

/// In-memory sink for testing.
pub struct VecSink {
    pub results: Vec<StepResult>,
}

impl VecSink {
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }
}

impl Default for VecSink {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputSink for VecSink {
    fn emit(&mut self, result: StepResult) -> std::io::Result<()> {
        self.results.push(result);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonl_sink_writes_lines() {
        let mut buf = Vec::new();
        let mut sink = JsonlSink::new(&mut buf);

        sink.emit(StepResult {
            step_index: 0,
            step_type: "goto".into(),
            status: ExecutionStatus::Ok,
            duration_ms: 100,
            details: HashMap::from([("url".into(), "https://example.com".into())]),
            error: None,
        })
        .unwrap();

        let output = String::from_utf8(buf).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
        assert_eq!(parsed["step_type"], "goto");
        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["details"]["url"], "https://example.com");
    }

    #[test]
    fn vec_sink_collects() {
        let mut sink = VecSink::new();
        sink.emit(StepResult {
            step_index: 0,
            step_type: "click".into(),
            status: ExecutionStatus::Ok,
            duration_ms: 50,
            details: HashMap::new(),
            error: None,
        })
        .unwrap();
        assert_eq!(sink.results.len(), 1);
        assert_eq!(sink.results[0].step_type, "click");
    }
}
