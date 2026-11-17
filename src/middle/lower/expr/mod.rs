//! 表达式族入口（`AstToIrGenerator` 的表达式 lowering）
//!
//! `generate_expr_ir` 是入口（存在类型包装点查表），`generate_expr_ir_inner` 是总派发器；
//! 各类表达式按方法簇分在同目录子模块。闭包自由变量收集器（`collect_free_vars_*`）
//! 是 lambda 体与匿名绑定共用的纯 AST 遍历。方法体逐字搬自原 `middle/ir/ir_gen.rs`
//! （WBS 7.0.3），语义未改。

mod access;
mod call;
mod closure;
mod container;
mod control;
mod lit;
mod op;
mod reference;
mod spawn;

use crate::ast::Expr;
use crate::ast;
use crate::middle::ir::ir::{ConstValue, Instruction, Operand};
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use super::AstToIrGenerator;

/// 收集 lambda 体引用的自由变量（名字），按出现顺序去重。
///
/// 用途：闭包捕获（RFC-009a / #254）。`bound` 是 lambda 自身参数（不算自由），
/// `seen` 跨调用累积去重，`out` 为结果。
///
/// 仅做保守的语法级遍历：多收集一个变量只会多一条 env 槽位（不影响正确性），
/// 漏收集才会导致闭包体内 `LoadUpvalue` 越界，故宁多勿少。
fn collect_free_vars_stmt(
    stmt: &crate::ast::Stmt,
    bound: &std::collections::HashSet<&str>,
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<String>,
) {
    use crate::ast::StmtKind;
    // 小工具：走一个块（避免闭包借用冲突）
    fn walk_block(
        b: &crate::ast::Block,
        bound: &std::collections::HashSet<&str>,
        seen: &mut std::collections::HashSet<String>,
        out: &mut Vec<String>,
    ) {
        for s in &b.stmts {
            collect_free_vars_stmt(s, bound, seen, out);
        }
    }
    match &stmt.kind {
        StmtKind::Expr(e) => collect_free_vars_expr(e, bound, seen, out),
        StmtKind::Return(Some(e)) => collect_free_vars_expr(e, bound, seen, out),
        StmtKind::Assign { target, value, .. } => {
            collect_free_vars_expr(target, bound, seen, out);
            if let Some(v) = value {
                collect_free_vars_expr(v, bound, seen, out);
            }
        }
        StmtKind::DestructureAssign { rhs, .. } => collect_free_vars_expr(rhs, bound, seen, out),
        StmtKind::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => {
            collect_free_vars_expr(condition, bound, seen, out);
            walk_block(then_branch, bound, seen, out);
            for (c, b) in else_if_branches {
                collect_free_vars_expr(c, bound, seen, out);
                walk_block(b, bound, seen, out);
            }
            if let Some(b) = else_branch {
                walk_block(b, bound, seen, out);
            }
        }
        StmtKind::For { iterable, body, .. } => {
            collect_free_vars_expr(iterable, bound, seen, out);
            walk_block(body, bound, seen, out);
        }
        _ => {}
    }
}

/// 表达式层的自由变量收集（递归）。
pub(in crate::middle) fn collect_free_vars_expr(
    expr: &crate::ast::Expr,
    bound: &std::collections::HashSet<&str>,
    seen: &mut std::collections::HashSet<String>,
    out: &mut Vec<String>,
) {
    use crate::ast::Expr;
    match expr {
        Expr::Var(name, _) => {
            if !bound.contains(name.as_str()) && seen.insert(name.clone()) {
                out.push(name.clone());
            }
        }
        Expr::BinOp { left, right, .. } => {
            collect_free_vars_expr(left, bound, seen, out);
            collect_free_vars_expr(right, bound, seen, out);
        }
        Expr::UnOp { expr, .. } | Expr::Try { expr, .. } | Expr::Cast { expr, .. } => {
            collect_free_vars_expr(expr, bound, seen, out);
        }
        Expr::Call { func, args, .. } => {
            collect_free_vars_expr(func, bound, seen, out);
            for a in args {
                collect_free_vars_expr(a, bound, seen, out);
            }
        }
        Expr::FieldAccess { expr, .. } | Expr::Borrow { expr, .. } => {
            collect_free_vars_expr(expr, bound, seen, out);
        }
        Expr::Index { expr, index, .. } => {
            collect_free_vars_expr(expr, bound, seen, out);
            collect_free_vars_expr(index, bound, seen, out);
        }
        Expr::Tuple(items, _) | Expr::List(items, _) => {
            for i in items {
                collect_free_vars_expr(i, bound, seen, out);
            }
        }
        Expr::Dict(entries, _) => {
            for (k, v) in entries {
                collect_free_vars_expr(k, bound, seen, out);
                collect_free_vars_expr(v, bound, seen, out);
            }
        }
        Expr::If {
            condition,
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => {
            collect_free_vars_expr(condition, bound, seen, out);
            for s in &then_branch.stmts {
                collect_free_vars_stmt(s, bound, seen, out);
            }
            for (c, b) in else_if_branches {
                collect_free_vars_expr(c, bound, seen, out);
                for s in &b.stmts {
                    collect_free_vars_stmt(s, bound, seen, out);
                }
            }
            if let Some(b) = else_branch {
                for s in &b.stmts {
                    collect_free_vars_stmt(s, bound, seen, out);
                }
            }
        }
        Expr::Block(b) => {
            for s in &b.stmts {
                collect_free_vars_stmt(s, bound, seen, out);
            }
        }
        Expr::While {
            condition, body, ..
        } => {
            collect_free_vars_expr(condition, bound, seen, out);
            for s in &body.stmts {
                collect_free_vars_stmt(s, bound, seen, out);
            }
        }
        Expr::For { iterable, body, .. } => {
            collect_free_vars_expr(iterable, bound, seen, out);
            for s in &body.stmts {
                collect_free_vars_stmt(s, bound, seen, out);
            }
        }
        _ => {}
    }
}

impl AstToIrGenerator {
    /// 生成表达式 IR（入口分发器）
    ///
    /// RFC-011a §6: 查存在类型包装点表——命中则先生成裸值到临时寄存器，
    /// 再包 CreateVariant。包装点由 typecheck 强制点收集产出（let 注解/实参/
    /// return/字面量元素），span 键与 AST 节点一一对应。
    pub(in crate::middle) fn generate_expr_ir(
        &mut self,
        expr: &ast::Expr,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        if let Some(iface) = self.lookup_existential_wrap(expr) {
            let group = format!("{}$Group", iface);
            let concrete = self.get_expr_struct_type_name(expr).ok_or_else(|| {
                ErrorCodeDefinition::ir_internal_error(&format!(
                    "存在类型 {} 的包装点无法确定实参具体类型（span {:?}）",
                    iface,
                    Self::expr_span(expr)
                ))
                .build()
            })?;
            let variant = self
                .interface_variants
                .get(&iface)
                .and_then(|v| v.iter().position(|t| *t == concrete))
                .ok_or_else(|| {
                    ErrorCodeDefinition::ir_internal_error(&format!(
                        "类型 {} 不在接口 {} 的实现收集表中",
                        concrete, iface
                    ))
                    .build()
                })? as u32;
            let payload_reg = self.next_temp_reg();
            self.generate_expr_ir_inner(expr, payload_reg, instructions, constants)?;
            instructions.push(Instruction::CreateVariant {
                dst: Operand::Local(result_reg),
                group,
                variant,
                payload: Operand::Local(payload_reg),
                span: Self::expr_span(expr),
            });
            return Ok(());
        }
        self.generate_expr_ir_inner(expr, result_reg, instructions, constants)
    }

    fn generate_expr_ir_inner(
        &mut self,
        expr: &ast::Expr,
        result_reg: usize,
        instructions: &mut Vec<Instruction>,
        constants: &mut Vec<ConstValue>,
    ) -> Result<(), Diagnostic> {
        match expr {
            Expr::Lit(literal, _) => {
                self.generate_lit_expr_ir(literal, result_reg, instructions, constants)?;
            }
            Expr::Var(var_name, var_span) => {
                self.generate_var_expr_ir(var_name, var_span, result_reg, instructions)?;
            }
            Expr::BinOp {
                op,
                left,
                right,
                span,
                ..
            } => {
                self.generate_bin_op_expr_ir(
                    op,
                    left,
                    right,
                    span,
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            Expr::Call {
                func,
                args,
                named_args,
                span,
                plan_id,
            } => {
                self.generate_call_expr_ir(
                    func,
                    args,
                    named_args,
                    span,
                    *plan_id,
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            Expr::FieldAccess {
                expr, field, span, ..
            } => {
                self.generate_field_access_expr_ir(
                    expr,
                    field,
                    span,
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            Expr::ListComp {
                element,
                generators,
                span,
                ..
            } => {
                self.generate_list_comp_expr_ir(
                    element,
                    generators,
                    span,
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            Expr::List(elements, span) => {
                self.generate_list_expr_ir(elements, span, result_reg, instructions, constants)?;
            }
            Expr::Dict(pairs, _) => {
                self.generate_dict_expr_ir(pairs, result_reg, instructions, constants)?;
            }
            Expr::Index {
                expr, index, span, ..
            } => {
                self.generate_index_expr_ir(
                    expr,
                    index,
                    span,
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            // #299 §3: membership 谓词 `elem in container`
            // Range 字面量脱糖成比较链（left >= start && left < end），无需 Range 运行时值；
            // 其余容器发 Contains 指令
            Expr::In {
                elem,
                container,
                span,
            } => {
                self.generate_in_ir(elem, container, result_reg, *span, instructions, constants)?;
            }
            Expr::Return(expr, _) => {
                self.generate_return_expr_ir(expr, result_reg, instructions, constants)?;
            }
            Expr::Break(span) => {
                self.generate_break_expr_ir(span, instructions)?;
            }
            Expr::Continue(span) => {
                self.generate_continue_expr_ir(span, instructions)?;
            }
            Expr::Try { expr, span, .. } => {
                self.generate_try_expr_ir(expr, span, result_reg, instructions, constants)?;
            }
            Expr::If {
                condition,
                then_branch,
                else_if_branches,
                else_branch,
                span: _,
            } => {
                // 重新实现 if 表达式，使用更简单的方法
                self.generate_if_expr_ir(
                    condition,
                    then_branch,
                    else_if_branches,
                    else_branch.as_deref(),
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            Expr::While {
                condition,
                body,
                span: _,
            } => {
                self.generate_while_expr_ir(condition, body, result_reg, instructions, constants)?;
            }
            Expr::For {
                var,
                var_mut,
                iterable,
                body,
                span: for_span,
                ..
            } => {
                self.generate_for_expr_ir(
                    var,
                    var_mut,
                    iterable,
                    body,
                    for_span,
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            Expr::Ref { expr, .. } => {
                self.generate_ref_expr_ir(expr, result_reg, instructions, constants)?;
            }
            Expr::Unsafe { body, .. } => {
                self.generate_unsafe_expr_ir(body, result_reg, instructions, constants)?;
            }
            // spawn for 数据并行循环（RFC-024 §2.4）
            Expr::SpawnFor {
                var,
                var_mut,
                iterable,
                body,
                span,
                ..
            } => {
                self.generate_spawn_for_expr_ir(
                    var,
                    var_mut,
                    iterable,
                    body,
                    span,
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            Expr::Spawn { body, span, .. } => {
                self.generate_spawn_expr_ir(body, span, expr, result_reg, instructions, constants)?;
            }
            Expr::UnOp { op, expr, .. } => {
                self.generate_un_op_expr_ir(op, expr, result_reg, instructions, constants)?;
            }
            Expr::Lambda { params, body, .. } => {
                self.generate_lambda_expr_ir(params, body, result_reg, instructions, constants)?;
            }
            Expr::Borrow { expr, .. } => {
                self.generate_borrow_expr_ir(expr, result_reg, instructions, constants)?;
            }
            Expr::Match {
                expr: match_expr,
                arms,
                span,
            } => {
                self.generate_match_expr_ir(
                    match_expr,
                    arms,
                    *span,
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            // RFC-012: F-string 代码生成
            Expr::FString { segments, span, .. } => {
                self.generate_f_string_expr_ir(
                    segments,
                    span,
                    expr,
                    result_reg,
                    instructions,
                    constants,
                )?;
            }
            Expr::Tuple(items, _) => {
                self.generate_tuple_expr_ir(items, result_reg, instructions, constants)?;
            }
            Expr::Block(block) => {
                // 语句位置块表达式（SPEC §12.5 good_seq）：逐语句生成，
                // 最后表达式值写入 result_reg（复用块 IR 生成）。
                self.generate_block_ir(block, Some(result_reg), instructions, constants)?;
            }
            other => {
                // 未实现的表达式变体：硬错误，禁止静默归零（#251：&&/|| 曾被同类兜底吞掉）
                return Err(ErrorCodeDefinition::ir_internal_error(&format!(
                    "unhandled expression in IR generation: {:?}",
                    std::mem::discriminant(other)
                ))
                .at(Self::get_expr_span(other))
                .build());
            }
        }
        Ok(())
    }
}
