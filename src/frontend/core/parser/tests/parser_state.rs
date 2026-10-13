//! Tests for ParserState: helpers, error tracking, save/restore.

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::ParserState;
use crate::util::plan_id::PlanId;
use crate::frontend::core::lexer::tokens::TokenKind;
use crate::frontend::core::parser::parse_msg;

fn with_state<F>(
    source: &str,
    mut f: F,
) where
    F: FnMut(&mut ParserState<'_>),
{
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens);
    f(&mut state);
}

#[test]
fn test_new_parser_state() {
    with_state("", |state| {
        assert!(state.at_end());
    });
}

#[test]
fn test_not_at_end_with_tokens() {
    with_state("42", |state| {
        assert!(!state.at_end());
    });
}

#[test]
fn test_current_returns_first_token() {
    with_state("42", |state| {
        let tok = state.current();
        assert!(tok.is_some());
        assert_eq!(&tok.unwrap().kind, &TokenKind::IntLiteral(42));
    });
}

#[test]
fn test_peek_returns_second_token() {
    with_state("1 + 2", |state| {
        let peeked = state.peek();
        assert!(peeked.is_some());
        assert_eq!(&peeked.unwrap().kind, &TokenKind::Plus);
    });
}

#[test]
fn test_bump_consumes_tokens() {
    with_state("a b", |state| {
        assert_eq!(
            &state.current().unwrap().kind,
            &TokenKind::Identifier("a".into())
        );
        state.bump();
        assert_eq!(
            &state.current().unwrap().kind,
            &TokenKind::Identifier("b".into())
        );
    });
}

#[test]
fn test_bump_to_end() {
    with_state("x", |state| {
        state.bump();
        assert!(state.at_end());
    });
}

#[test]
fn test_at_matches() {
    with_state("+", |state| {
        assert!(state.at(&TokenKind::Plus));
    });
}

#[test]
fn test_at_not_matches() {
    with_state("+", |state| {
        assert!(!state.at(&TokenKind::Minus));
    });
}

#[test]
fn test_skip_success() {
    with_state("; x", |state| {
        assert!(state.skip(&TokenKind::Semicolon));
        assert_eq!(
            &state.current().unwrap().kind,
            &TokenKind::Identifier("x".into())
        );
    });
}

#[test]
fn test_skip_failure() {
    with_state("x", |state| {
        assert!(!state.skip(&TokenKind::Semicolon));
    });
}

#[test]
fn test_error_tracking() {
    with_state("x", |state| {
        state.error(parse_msg(state, "test error"));
        assert!(state.has_errors());
        assert_eq!(state.error_count(), 1);
    });
}

#[test]
fn test_take_errors() {
    with_state("x", |state| {
        state.error(parse_msg(state, "err1"));
        state.error(parse_msg(state, "err2"));
        let errors = state.take_errors();
        assert_eq!(errors.len(), 2);
        assert!(!state.has_errors());
    });
}

#[test]
fn test_save_restore_position() {
    with_state("a b c", |state| {
        state.bump(); // consume 'a'
        let saved = state.save_position();
        state.bump(); // consume 'b'
        state.restore_position(saved);
        assert_eq!(
            &state.current().unwrap().kind,
            &TokenKind::Identifier("b".into())
        );
    });
}

#[test]
fn test_nested_save_restore() {
    with_state("1 2 3", |state| {
        let s1 = state.save_position();
        state.bump();
        let s2 = state.save_position();
        state.bump();
        state.restore_position(s2);
        assert_eq!(&state.current().unwrap().kind, &TokenKind::IntLiteral(2));
        state.restore_position(s1);
        assert_eq!(&state.current().unwrap().kind, &TokenKind::IntLiteral(1));
    });
}

#[test]
fn test_can_start_stmt_semicolon() {
    with_state(";", |state| {
        assert!(!state.can_start_stmt());
    });
}

#[test]
fn test_can_start_stmt_keyword() {
    with_state("if", |state| {
        assert!(state.can_start_stmt());
    });
}

#[test]
fn test_can_start_stmt_identifier() {
    with_state("x", |state| {
        assert!(state.can_start_stmt());
    });
}
// ── 释放计划身份分配（RFC-039 D20 / WBS 4.5.1） ──────────────────

/// 收集模块里全部语句的释放计划身份（语句自身 → 其嵌套块）。
///
/// 夹具级 AST 走查：只覆盖本用例用到的语句/表达式形态。
fn collect_stmt_plan_ids(module: &crate::frontend::core::parser::ast::Module) -> Vec<PlanId> {
    let mut ids = Vec::new();
    for stmt in &module.items {
        collect_one_stmt(stmt, &mut ids);
    }
    ids
}

fn collect_one_stmt(
    stmt: &crate::frontend::core::parser::ast::Stmt,
    ids: &mut Vec<PlanId>,
) {
    use crate::frontend::core::parser::ast::{Block, StmtKind};
    ids.push(stmt.plan_id);
    let mut block = |b: &Block, ids: &mut Vec<PlanId>| {
        for inner in &b.stmts {
            collect_one_stmt(inner, ids);
        }
    };
    match &stmt.kind {
        StmtKind::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => {
            block(then_branch, ids);
            for (_, body) in else_if_branches {
                block(body, ids);
            }
            if let Some(body) = else_branch {
                block(body, ids);
            }
        }
        StmtKind::For { body, .. } => block(body, ids),
        StmtKind::Assign {
            value: Some(value), ..
        } => collect_expr_plan_ids(value, ids, &mut block),
        StmtKind::Expr(expr) => collect_expr_plan_ids(expr, ids, &mut block),
        _ => {}
    }
}

fn collect_expr_plan_ids(
    expr: &crate::frontend::core::parser::ast::Expr,
    ids: &mut Vec<PlanId>,
    block: &mut impl FnMut(&crate::frontend::core::parser::ast::Block, &mut Vec<PlanId>),
) {
    use crate::frontend::core::parser::ast::Expr;
    match expr {
        Expr::Block(b) => block(b, ids),
        Expr::Lambda { body, .. } => block(body, ids),
        Expr::If {
            then_branch,
            else_if_branches,
            else_branch,
            ..
        } => {
            block(then_branch, ids);
            for (_, body) in else_if_branches {
                block(body, ids);
            }
            if let Some(body) = else_branch {
                block(body, ids);
            }
        }
        Expr::While { body, .. } | Expr::For { body, .. } => block(body, ids),
        Expr::Spawn { body, .. } | Expr::Unsafe { body, .. } => block(body, ids),
        _ => {}
    }
}

/// 解析期分配：每条最终 AST 语句恰有一个身份，**编译单元内唯一**、分配确定
/// （同一源码解析两次给出同一批身份）。
///
/// 不要求「密集」：解析器有投机解析（先试一条路，restore 后重来），被丢弃那
/// 一趟里构造的语句已经领号——那些号在最终 AST 上不可见，也无害：身份只作键，
/// 同一条语句在 ownership 与 ir_gen 两侧读的是同一个值。
#[test]
fn test_parse_statement_plan_ids_are_unique_and_deterministic() {
    // Arrange：含嵌套（函数体 / if 分支 / 尾语句）的源码
    let source = concat!(
        "f: (a: Int) -> Int = {\n",
        "    x = a + 1\n",
        "    if x > 0 {\n",
        "        y = x\n",
        "    }\n",
        "    x\n",
        "}\n",
        "main: () -> Void = {\n",
        "    z = f(1)\n",
        "}\n",
        "main()\n",
    );

    // Act
    let parsed = crate::frontend::core::parser::parse(&tokenize(source).unwrap());
    assert!(
        !parsed.has_errors,
        "夹具必须无语法错误: {:?}",
        parsed.errors
    );
    let ids = collect_stmt_plan_ids(&parsed.module);

    // Assert：全部已分配 + 唯一 + 再解析一次结果相同
    assert!(
        ids.iter().all(|id| id.is_planned()),
        "解析器产出的语句不得带未分配身份: {ids:?}"
    );
    assert!(
        ids.len() >= 8,
        "夹具含 ≥8 条语句，实际 {}: {ids:?}",
        ids.len()
    );
    let unique: std::collections::HashSet<PlanId> = ids.iter().copied().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "语句身份必须编译单元内唯一: {ids:?}"
    );
    let max_ordinal = ids.iter().map(|id| id.ordinal()).max().unwrap();
    assert!(
        max_ordinal as usize + 1 >= ids.len(),
        "身份序号不得少于语句数（重号或复用计数器都会踩到这里）: {ids:?}"
    );
    let reparsed = crate::frontend::core::parser::parse(&tokenize(source).unwrap());
    assert_eq!(
        collect_stmt_plan_ids(&reparsed.module),
        ids,
        "同一源码两次解析必须给出同一批语句身份（否则跨进程/跨轮次不可复现）"
    );
}

/// 合成语句（不经解析器）恒为未分配身份——它不属任何解析单元，也不构成释放计划义务。
#[test]
fn test_unplanned_statement_has_no_plan_id() {
    // Arrange：手工构造的合成语句（ir_gen 内建 / 直驱夹具同款）
    use crate::frontend::core::parser::ast::{Expr, Stmt, StmtKind};
    let stmt = Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Lit(
            crate::frontend::core::lexer::tokens::Literal::Int(1),
            crate::util::span::Span::dummy(),
        ))),
        span: crate::util::span::Span::dummy(),
        plan_id: PlanId::UNPLANNED,
    };

    // Act
    let planned = stmt.plan_id.is_planned();

    // Assert
    assert!(!planned, "合成语句不得持有已分配身份: {:?}", stmt.plan_id);
    assert_eq!(
        PlanId::UNPLANNED.to_string(),
        "plan#unplanned",
        "未分配身份的显示形态是诊断可读性的一部分"
    );
}
