#![allow(clippy::module_inception)]

//! Executor module for YaoXiang bytecode interpreter
//!
//! This module provides the main interpreter implementation split into:
//! - `executor.rs`: Interpreter struct and core functionality
//! - `execute.rs`: Executor trait implementation with bytecode execution
//! - `debug.rs`: DebuggableExecutor trait and tests

mod debug;
mod execute;
mod executor;

pub use executor::Interpreter;
