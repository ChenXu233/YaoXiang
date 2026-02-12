//! 类型推断模块
//!
//! 实现 Hindley-Milner 类型推断算法

pub mod expressions;
pub mod generics;
pub mod patterns;
pub mod statements;

// 类型推断器trait
pub trait TypeInferrer {
    fn infer_expr(
        &mut self,
        expr: &crate::frontend::core::parser::ast::Expr,
    ) -> Result<MonoType, Diagnostic>;
    fn infer_stmt(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) -> Result<(), Diagnostic>;
    fn infer_pattern(
        &mut self,
        pattern: &crate::frontend::core::parser::ast::Pattern,
    ) -> Result<MonoType, Diagnostic>;
}

// 重新导出
pub use expressions::ExprInferrer;
pub use statements::StmtInferrer;
pub use patterns::PatternInferrer;
pub use generics::GenericInferrer;

pub use crate::frontend::core::type_system::{MonoType, PolyType, TypeConstraintSolver};
pub use crate::util::diagnostic::{Diagnostic, Result};
