//! 诊断输出模块

pub mod ansi;
pub mod json;
pub mod text;

pub use json::JsonEmitter;
pub use text::{EmitterConfig, TextEmitter};

#[cfg(test)]
mod tests;
