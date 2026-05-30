//! 诊断输出模块

pub mod ansi;
pub mod json;
pub mod text;

pub use text::{TextEmitter, EmitterConfig};
pub use json::JsonEmitter;
