//! Embedded runtime for YaoXiang
//!
//! Provides immediate execution for WASM/game scripts/embedded scenarios.
//! Key characteristics:
//! - Immediate execution: no DAG, no scheduler
//! - Synchronous execution: all operations execute in order
//! - Spawn ignored: spawn markers treated as normal function calls
//!
//! See RFC-008 for runtime architecture details.

pub mod executor;
pub use executor::{EmbeddedRuntime, RuntimeError};
