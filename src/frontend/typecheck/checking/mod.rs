//! 类型检查模块
//!
//! 负责检查模块、函数和语句的类型正确性

pub mod assignment;
pub mod bounds;
pub mod compatibility;
pub mod subtyping;

// 类型检查器trait
pub trait TypeChecker {
    fn check_module(
        &mut self,
        module: &crate::frontend::core::parser::ast::Module,
    ) -> Result<(), TypeError>;
    fn check_function(
        &mut self,
        name: &str,
        params: &[crate::frontend::core::parser::ast::Param],
    ) -> Result<(), TypeError>;
    fn check_statement(
        &mut self,
        stmt: &crate::frontend::core::parser::ast::Stmt,
    ) -> Result<(), TypeError>;
}

// 重新导出
pub use subtyping::SubtypeChecker;
pub use assignment::AssignmentChecker;
pub use compatibility::CompatibilityChecker;
pub use bounds::BoundsChecker;

pub use crate::frontend::typecheck::TypeError;
pub use crate::frontend::shared::error::Result;
