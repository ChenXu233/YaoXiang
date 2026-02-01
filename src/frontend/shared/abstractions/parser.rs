//! Parser 抽象接口
//!
//! 定义解析器的抽象接口

use crate::frontend::core::parser::ast;

/// Parser 特质
pub trait ParserTrait {
    /// 解析源代码
    fn parse(
        &mut self,
        source: &str,
    ) -> Result<ast::Module, String>;

    /// 解析表达式
    fn parse_expr(
        &mut self,
        source: &str,
    ) -> Result<ast::Expr, String>;
}
