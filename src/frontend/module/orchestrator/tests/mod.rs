//! orchestrator 测试 — RFC-014 §模块解析顺序（vendor priority 2）
//!
//! 覆盖 vendor 依赖多版本目录的解析选择语义。

mod vendor_version_resolution;

pub(crate) use super::{compare_version, resolve_in_vendor};
