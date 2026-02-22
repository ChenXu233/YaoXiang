//! 类型推断模块
//!
//! 实现 Hindley-Milner 类型推断算法
//! 统一的类型推断入口，合并了原 checking/ 和 inference/ 模块

// ✅ 核心模块
pub mod expressions;
pub mod scope;
pub mod statements;
pub mod types;

// ✅ 从 checking/ 移入的模块
pub mod assignment;
pub mod bounds;
pub mod compatibility;
pub mod subtyping;

// ✅ 保留的独立模块
pub mod generics;
pub mod patterns;

// 重新导出核心类型
pub use scope::ScopeManager;
pub use types::TypeSystem;
pub use statements::StatementChecker;
pub use expressions::ExpressionInferrer;
pub use assignment::AssignmentChecker;
pub use assignment::ConstraintAssignmentInfo;
pub use subtyping::SubtypeChecker;
pub use compatibility::CompatibilityChecker;
pub use bounds::BoundsChecker;
pub use patterns::PatternInferrer;
pub use generics::GenericInferrer;

// 向后兼容别名
pub use expressions::ExprInferrer;

pub use crate::frontend::core::type_system::{MonoType, PolyType, TypeConstraintSolver};
pub use crate::util::diagnostic::{Diagnostic, Result};
