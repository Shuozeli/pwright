pub mod error;
pub mod executor;
pub mod model;
pub mod output;
pub mod parser;
pub mod validator;

/// Generated protobuf types.
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/pwright.script.v1.rs"));
}
