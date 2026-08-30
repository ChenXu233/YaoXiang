//! RFC-011a §6 存在类型强制点收集
//!
//! 接口构造器名（如 `Animal: (Self: Type) -> Type` 的 `Animal`）未实例化地出现在
//! 期望类型位置时，它是存在类型 `∃S. Animal(S)`：具体类型的值进入该位置必须包装成
//! 变体值（阶段3 的 CreateVariant）。
//!
//! 本模块在 typecheck 已完成"具体 vs 存在"兼容判定的位置（带注解 let、调用实参、
//! return、存在类型变量的重赋值），把每个需要包装的表达式 span 与目标接口记录下来；
//! ir_gen 按 span 查表注入包装指令（span-keyed 侧信道，与
//! `InstantiationRequest.source_location` 同一模式）。列表字面量按元素下钻——
//! List 是持久化结构（std.list 全部变体操作返回新 List），字面量填充与原生 item
//! 实参是具体值进入容器的仅有的两条通道，全部可数。

use std::collections::HashMap;

use crate::frontend::core::parser::ast::Expr;
use crate::frontend::core::types::{MonoType, PolyType, TypeConstraintSolver};
use crate::frontend::core::typecheck::environment::GenericTypeDef;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};

use super::scope::ScopeManager;

/// 一个具体→存在类型的包装点（ir_gen 按 span 注入 CreateVariant）
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExistentialCoercion {
    /// 被包装表达式在 AST 中的 span（ir_gen generate_expr_ir 入口查表键）
    pub span: crate::util::span::Span,
    /// 目标接口构造器名（合成变体类型 `Animal$Group` 的来源）
    pub interface: String,
}

/// RFC-011a 接口判定：构造器名命中泛型类型定义表（与 Phase 2 的
/// `is_interface_application` 同一判据）。遗留 trait 约束（`Drawable: Type = {..}`，
/// 无泛型参数）不进 generic_type_defs，天然被排除，回归面隔离。
fn is_interface_ctor(
    generic_type_defs: &HashMap<String, GenericTypeDef>,
    name: &str,
) -> bool {
    generic_type_defs.contains_key(name)
}

/// 从表达式提取源码 span（与 ownership.rs 的 expr_span 同构；该版本私有且
/// 在不同分层，这里独立维护一份，缺省落 dummy——dummy span 永远查不中强制表，
/// 安全方向失败）
fn expr_span(expr: &Expr) -> crate::util::span::Span {
    match expr {
        Expr::Lit(_, s)
        | Expr::Var(_, s)
        | Expr::Return(_, s)
        | Expr::Break(s)
        | Expr::Continue(s) => *s,
        Expr::BinOp { span, .. }
        | Expr::UnOp { span, .. }
        | Expr::Call { span, .. }
        | Expr::FnDef { span, .. }
        | Expr::If { span, .. }
        | Expr::Match { span, .. }
        | Expr::While { span, .. }
        | Expr::For { span, .. }
        | Expr::SpawnFor { span, .. }
        | Expr::Borrow { span, .. }
        | Expr::FieldAccess { span, .. }
        | Expr::Index { span, .. }
        | Expr::Tuple(_, span)
        | Expr::List(_, span)
        | Expr::Cast { span, .. }
        | Expr::Try { span, .. } => *span,
        Expr::Block(block) => block.span,
        _ => crate::util::span::Span::dummy(),
    }
}

/// 叶子表达式的具体类型（只读推导，不重跑推断——重跑会重复触发
/// instantiation_requests 等副作用）：
/// - 构造器调用 `Dog(...)` / 普通函数调用 `make_dog()`：scope 中该名字的
///   PolyType（构造器注册为 Struct，函数为 Fn → 取返回类型）
/// - 变量引用：scope 中该变量的类型
fn leaf_concrete_type(
    solver: &TypeConstraintSolver,
    scope: &ScopeManager,
    expr: &Expr,
) -> Option<MonoType> {
    match expr {
        Expr::Call { func, .. } => match func.as_ref() {
            Expr::Var(name, _) => {
                let poly: &PolyType = scope.get_var(name)?;
                match &poly.body {
                    MonoType::Fn { return_type, .. } => Some(solver.resolve_type(return_type)),
                    other => Some(solver.resolve_type(other)),
                }
            }
            _ => None,
        },
        Expr::Var(name, _) => {
            let poly = scope.get_var(name)?;
            Some(solver.resolve_type(&poly.body))
        }
        _ => None,
    }
}

/// 期望类型下钻 + 强制点收集。
///
/// - 期望为 `List/Array(elem)` 且表达式是列表字面量 → 对每个元素以 elem 递归
///   （元素成员检查在 typecheck 主判定里只作用于首个元素，这里补齐全量）
/// - 期望为接口构造器名 → 叶子求具体类型：实现了接口 → 记包装点；
///   未实现 → E1101（字面量后位元素的成员检查缺口由这里兜住）
fn walk(
    solver: &TypeConstraintSolver,
    scope: &ScopeManager,
    generic_type_defs: &HashMap<String, GenericTypeDef>,
    expr: &Expr,
    expected: &MonoType,
    out: &mut Vec<ExistentialCoercion>,
    errors: &mut Vec<Diagnostic>,
) {
    let expected = solver.resolve_type(expected);
    match &expected {
        MonoType::Generic { name, args }
            if (name == "List" || name == "Array") && args.len() == 1 =>
        {
            if let Expr::List(elems, _) = expr {
                for e in elems {
                    walk(solver, scope, generic_type_defs, e, &args[0], out, errors);
                }
            }
        }
        MonoType::TypeRef(iface) if is_interface_ctor(generic_type_defs, iface) => {
            let span = expr_span(expr);
            if span == crate::util::span::Span::dummy() {
                return; // 无法定位的表达式（If/Match 等）留给运行时守卫兜底
            }
            if let Some(MonoType::Struct(s)) = leaf_concrete_type(solver, scope, expr) {
                if s.interfaces.iter().any(|i| i == iface) {
                    out.push(ExistentialCoercion {
                        span,
                        interface: iface.clone(),
                    });
                } else {
                    errors.push(
                        ErrorCodeDefinition::type_does_not_implement_interface(&s.name, iface)
                            .at(span)
                            .build(),
                    );
                }
            }
            // else: TypeVar/Any/未知形态——编译期无法定向，运行时守卫兜底
        }
        _ => {}
    }
}

/// 决策点统一入口：在 typecheck 接受"具体→存在"赋值的位置调用。
/// 返回收集到的包装点；成员违规直接以 E1101 诊断返回。
pub fn collect_existential_coercions(
    solver: &TypeConstraintSolver,
    scope: &ScopeManager,
    type_defs: &HashMap<String, MonoType>,
    generic_type_defs: &HashMap<String, GenericTypeDef>,
    expr: &Expr,
    expected: &MonoType,
) -> (Vec<ExistentialCoercion>, Vec<Diagnostic>) {
    let _ = type_defs; // 预留：别名链解析（TypeRef→TypeRef）时启用
    let mut out = Vec::new();
    let mut errors = Vec::new();
    walk(
        solver,
        scope,
        generic_type_defs,
        expr,
        expected,
        &mut out,
        &mut errors,
    );
    (out, errors)
}
