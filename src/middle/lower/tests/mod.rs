//! `middle/lower` 的单元测试 — 对应 `src/middle/lower/` 各单文件模块
//!
//! 按测试编写规范规则 1.1（单文件模块 → 父级 `tests/`）落位：
//! `query.rs`（纯查询族）的测试在 [`query`]。
//!
//! 文件 ↔ 被测源文件对应：
//! - `query.rs` — `super::super::query`（#403 类型域落点归一的 L3 第四门）

mod query;
