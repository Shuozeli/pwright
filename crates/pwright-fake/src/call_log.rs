//! Shared, thread-safe CDP call recording for test clients.

use std::sync::Mutex;

use serde_json::Value;

/// One recorded native method invocation.
pub type RecordedCdpCall = (String, Vec<Value>);

/// Reusable recorder shared by canned and stateful CDP test clients.
#[derive(Default)]
pub struct CdpCallLog {
    calls: Mutex<Vec<RecordedCdpCall>>,
}

impl CdpCallLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record(&self, method: &str, args: Vec<Value>) {
        self.calls
            .lock()
            .expect("CDP call log should not be poisoned")
            .push((method.to_owned(), args));
    }

    pub fn calls(&self) -> Vec<RecordedCdpCall> {
        self.calls
            .lock()
            .expect("CDP call log should not be poisoned")
            .clone()
    }

    pub fn calls_for(&self, method: &str) -> Vec<RecordedCdpCall> {
        self.calls
            .lock()
            .expect("CDP call log should not be poisoned")
            .iter()
            .filter(|(recorded_method, _)| recorded_method == method)
            .cloned()
            .collect()
    }

    pub fn method_names(&self) -> Vec<String> {
        self.calls
            .lock()
            .expect("CDP call log should not be poisoned")
            .iter()
            .map(|(method, _)| method.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::CdpCallLog;

    #[test]
    fn records_and_filters_calls_without_reordering_them() {
        // Arrange
        let log = CdpCallLog::new();

        // Act
        log.record("Page.enable", Vec::new());
        log.record("Runtime.evaluate", vec![json!({"expression": "1 + 1"})]);
        log.record("Page.enable", vec![json!(true)]);

        // Assert
        assert_eq!(
            log.method_names(),
            ["Page.enable", "Runtime.evaluate", "Page.enable"]
        );
        assert_eq!(log.calls_for("Page.enable").len(), 2);
        assert_eq!(log.calls()[1].1[0]["expression"], "1 + 1");
    }
}
