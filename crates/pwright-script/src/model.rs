//! Script model types (internal representation).

use std::collections::HashMap;

use serde::Deserialize;

/// Error handling behavior for a step.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnError {
    #[default]
    Fail,
    Continue,
    Retry,
}

/// Type of a script parameter.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamType {
    #[default]
    String,
    Integer,
    Boolean,
}

#[derive(Debug, Clone)]
pub struct Script {
    pub name: String,
    pub description: String,
    pub version: i32,
    pub params: HashMap<String, ParamDef>,
    pub config: ScriptConfig,
    pub scripts: HashMap<String, JsFunction>,
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone)]
pub struct ScriptConfig {
    pub default_timeout_ms: u64,
    pub default_on_error: OnError,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 30000,
            default_on_error: OnError::Fail,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParamDef {
    pub param_type: ParamType,
    pub required: bool,
    pub default_value: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct JsFunction {
    pub body: String,
    pub description: String,
    pub is_async: bool,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub kind: StepKind,
    pub on_error: OnError,
}

#[derive(Debug, Clone)]
pub enum StepKind {
    Goto(GotoStep),
    Click(ClickStep),
    Fill(FillStep),
    Press(PressStep),
    Extract(ExtractStep),
    Eval(EvalStep),
    Output(OutputStep),
    Wait(WaitStep),
}

impl StepKind {
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Goto(_) => "goto",
            Self::Click(_) => "click",
            Self::Fill(_) => "fill",
            Self::Press(_) => "press",
            Self::Extract(_) => "extract",
            Self::Eval(_) => "eval",
            Self::Output(_) => "output",
            Self::Wait(w) => match &w.kind {
                WaitKind::Duration(_) => "wait",
                WaitKind::Text { .. } => "wait_for_text",
                WaitKind::Selector { .. } => "wait_for",
                WaitKind::Expression { .. } => "wait_until",
            },
        }
    }
}

/// What kind of wait to perform.
#[derive(Debug, Clone)]
pub enum WaitKind {
    /// Sleep for a fixed duration.
    Duration(u64),
    /// Wait until page body contains text.
    Text { text: String, timeout_ms: u64 },
    /// Wait until a CSS selector is attached to the DOM.
    Selector { selector: String, timeout_ms: u64 },
    /// Wait until a JS expression returns truthy.
    Expression { js: String, timeout_ms: u64 },
}

#[derive(Debug, Clone)]
pub struct WaitStep {
    pub kind: WaitKind,
}

#[derive(Debug, Clone)]
pub struct GotoStep {
    pub url: String,
    pub wait_for: Option<String>,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ClickStep {
    pub selector: String,
    pub wait_for: Option<String>,
}

#[derive(Debug, Clone)]
pub struct FillStep {
    pub selector: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct PressStep {
    pub key: String,
}

#[derive(Debug, Clone)]
pub struct ExtractStep {
    pub selector: String,
    pub field: String,
    pub save_as: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EvalStep {
    pub expression: Option<String>,
    pub js_ref: Option<String>,
    pub save_as: Option<String>,
    pub args: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct OutputStep {
    pub fields: HashMap<String, String>,
}
