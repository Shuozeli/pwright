//! Command-free, backend-neutral input bounds for Pwright.
//!
//! This crate owns no command, protocol, transport, runtime, browser backend,
//! selector, or browser process API.

/// Maximum physical keys retained by one bounded input chord transaction.
pub const INPUT_CHORD_MAX_KEYS: usize = 16;
