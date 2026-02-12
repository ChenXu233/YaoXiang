//! 诊断输出模块

pub mod json;
pub mod rich;
pub mod text;

pub use text::{TextEmitter, EmitterConfig};
pub use json::JsonEmitter;
pub use rich::{RichEmitter, RichConfig};
