//! Script error types.

#[derive(Debug, thiserror::Error)]
pub enum ScriptError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("CDP error: {0}")]
    Cdp(#[from] pwright_cdp::connection::CdpError),

    #[error("execution error: {0}")]
    Execution(String),

    #[error("output error: {0}")]
    Output(#[from] std::io::Error),
}
