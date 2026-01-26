//! Const求值模块（RFC-011 Phase 4）
//!
//! 这个模块包含了YaoXiang语言的Const泛型与编译期计算功能。

pub mod const_evaluator;
pub mod const_fn;
pub mod static_assert;

#[cfg(test)]
mod tests;

// 导出主要类型
pub use const_evaluator::{
    ConstEvalEnv, ConstEvaluator,
};

pub use const_fn::{
    ConstFunction, ConstFnCallEnv, ConstFnEvaluator,
};

pub use static_assert::{
    StaticAssert, StaticAssertChecker,
};

// 导出错误类型
pub use crate::frontend::typecheck::types::ConstEvalError;
