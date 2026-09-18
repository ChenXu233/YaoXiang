#![allow(clippy::module_inception)]

//! Executor module for YaoXiang bytecode interpreter
//!
//! This module provides the main interpreter implementation split into:
//! - `executor.rs`: Interpreter struct and core functionality
//! - `execute.rs`: Executor trait implementation with bytecode execution
//! - `debug.rs`: 步进引擎与指令分派（各族实现见 `ops/`）
//! - `ops/`: 按指令族拆分的执行逻辑

mod debug;
mod execute;
mod executor;
mod ops;

#[cfg(test)]
mod tests;

pub use executor::Interpreter;
