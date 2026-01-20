//! Runtime system
//!
//! This module contains memory management and concurrency scheduling.

pub mod dag;
pub mod extfunc;
pub mod interrupt;
pub mod memory;
pub mod scheduler;
pub mod value;

#[cfg(test)]
mod tests;
