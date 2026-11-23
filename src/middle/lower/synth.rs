//! L3/L4 生产代码唯一的 AST 合成边界（RFC-039 D22 / WBS 7c.1）。
//!
//! AST 节点不携带 lowering 环境；SynthEnv 因而不伪造为 AST 字段，而是与合成
//! 节点一起显式交给 generate_expr_ir_with_env。调用方不再预写生成器的共享字段。

use crate::ast;
use crate::middle::ir::ir::Operand;
use crate::util::span::Span;

/// 一次合成闭包 lowering 的显式环境。
#[derive(Debug, Clone, Default)]
pub(super) struct SynthEnv {
    /// 按运行时 env 槽顺序排列的值。
    pub(super) env_vars: Vec<Operand>,
    /// 与命名捕获槽对应的源码名；spawn-for 的匿名迭代值不占此表。
    pub(super) env_names: Vec<String>,
}

impl SynthEnv {
    pub(super) fn new(
        env_vars: Vec<Operand>,
        env_names: Vec<String>,
    ) -> Self {
        Self {
            env_vars,
            env_names,
        }
    }
}

/// 合成一个闭包 AST；环境由调用方随该节点显式传给 lowering。
pub(super) fn synth_closure(
    params: Vec<ast::Param>,
    body: ast::Block,
    span: Span,
) -> ast::Expr {
    ast::Expr::Lambda {
        params,
        body: Box::new(body),
        span,
    }
}

/// 把单个表达式包装成合成闭包的函数体。
pub(super) fn synth_expr_block(
    expr: ast::Expr,
    span: Span,
) -> ast::Block {
    ast::Block {
        stmts: vec![ast::Stmt {
            kind: ast::StmtKind::Expr(Box::new(expr)),
            span,
            plan_id: ast::PlanId::UNPLANNED,
        }],
        span,
    }
}
