//! 类型系统
//!
//! 类型系统的核心定义和操作，分为两层：
//! - base: 基础类型（MonoType, PolyType, TypeVar, 约束求解）
//! - computation: 类型级计算（条件类型、Const泛型、类型族）

pub mod base;
pub mod computation;
