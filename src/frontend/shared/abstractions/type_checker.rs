//! TypeChecker 抽象接口
//!
//! 定义类型检查器的抽象接口

use crate::frontend::core::parser::ast;
use crate::frontend::core::type_system::MonoType;

/// TypeChecker 特质
pub trait TypeCheckerTrait {
    /// 检查模块
    fn check_module(
        &mut self,
        module: &ast::Module,
    ) -> Result<(), Vec<String>>;

    /// 检查表达式
    fn check_expr(
        &mut self,
        expr: &ast::Expr,
    ) -> Result<MonoType, String>;

    /// 推断表达式类型
    fn infer_expr(
        &mut self,
        expr: &ast::Expr,
    ) -> Result<MonoType, String>;
}
