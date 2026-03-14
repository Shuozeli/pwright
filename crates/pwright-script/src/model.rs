//! Script model types (internal representation).

use std::collections::HashMap;

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
    pub default_on_error: String,
}

impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 30000,
            default_on_error: "fail".into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParamDef {
    pub param_type: String,
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
    pub on_error: String,
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
            Self::Wait(_) => "wait",
        }
    }
}

#[derive(Debug, Clone)]
pub struct WaitStep {
    pub duration_ms: u64,
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
