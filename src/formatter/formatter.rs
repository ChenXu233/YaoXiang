//! Formatter 主结构
//!
//! 顶层格式化入口，协调 SourceMap、FormatContext 和各 Handler。

use crate::frontend::core::parser::ast::Module;

use super::context::FormatContext;
use super::options::FormatOptions;
use super::rules::sort_imports::sort_imports;
use super::source_map::SourceMap;

/// 格式化器
///
/// 主要入口结构体，管理格式化状态和选项。
#[derive(Debug)]
pub struct Formatter {
    /// 格式化上下文
    pub ctx: FormatContext,
    /// 源映射
    pub source_map: SourceMap,
}

impl Formatter {
    /// 创建新的格式化器
    pub fn new(
        options: FormatOptions,
        source_map: SourceMap,
    ) -> Self {
        Self {
            ctx: FormatContext::new(options),
            source_map,
        }
    }

    /// 格式化模块
    pub fn format_module(
        &self,
        module: &Module,
    ) -> String {
        let mut sorted_module = module.clone();
        let mut source_map = self.source_map.clone();
        if self.ctx.options.sort_imports {
            sort_imports(&mut sorted_module.items, &mut source_map);
        }
        super::handlers::module::format_module(&sorted_module, &self.ctx, &source_map)
    }

    /// 格式化单个表达式（用于测试）
    pub fn format_expr(
        &self,
        expr: &crate::frontend::core::parser::ast::Expr,
    ) -> String {
        super::handlers::expr::format_expr(expr, &self.ctx, &self.source_map)
    }

    /// 格式化单个语句（用于测试）
    pub fn format_stmt(
        &self,
        stmt: &crate::frontend::core::parser::ast::StmtKind,
    ) -> String {
        super::handlers::stmt::format_stmt(stmt, &self.ctx, &self.source_map)
    }
}
