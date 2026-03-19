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
    fn emit(&mut self, result: StepResult);
}

/// JSONL output: one JSON line per step to a writer.
pub struct JsonlSink<W: Write> {
    writer: W,
}

impl<W: Write> JsonlSink<W> {
    pub fn new(writer: W) -> Self {
        Self { writer }
    }

    pub fn write_summary(&mut self, name: &str, result: &ExecutionResult) {
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
        match serde_json::to_string(&summary) {
            Ok(json) => {
                if let Err(e) = writeln!(self.writer, "{json}") {
                    tracing::warn!("failed to write JSONL summary: {e}");
                }
            }
            Err(e) => tracing::warn!("failed to serialize JSONL summary: {e}"),
        }
    }
}

impl<W: Write> OutputSink for JsonlSink<W> {
    fn emit(&mut self, result: StepResult) {
        match serde_json::to_string(&result) {
            Ok(json) => {
                if let Err(e) = writeln!(self.writer, "{json}") {
                    tracing::warn!("failed to write JSONL step: {e}");
                }
            }
            Err(e) => tracing::warn!("failed to serialize JSONL step: {e}"),
        }
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
    fn emit(&mut self, result: StepResult) {
        self.results.push(result);
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
        });

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
        });
        assert_eq!(sink.results.len(), 1);
        assert_eq!(sink.results[0].step_type, "click");
    }
}
