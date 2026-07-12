//! 单态化器测试 — 基于 RFC-011: 泛型系统设计
//!
//! RFC-011 §3: 零成本抽象与单态化
//! RFC-011 §4: 泛型函数特化
//! RFC-031: 优化级别与 Pass 管理器

mod instance;
mod monomorphizer;
mod type_mono;
