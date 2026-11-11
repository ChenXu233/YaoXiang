//! Tests for ParserState: helpers, error tracking, save/restore.

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::{NullProbe, ParserState};
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
    let mut state = ParserState::new(&tokens, &NullProbe);
    f(&mut state);
}

#[test]
fn test_new_parser_state() {
    with_state("", |state| {
        assert!(state.at_end());
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
fn test_at_matches() {
    with_state("+", |state| {
        assert!(state.at(&TokenKind::Plus));
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

// ── 释放计划身份分配（RFC-039 D20 / WBS 4.5.1） ──────────────────

/// 合成语句（不经解析器）恒为未分配身份——它不属任何解析单元，也不构成释放计划义务。
#[test]
fn test_unplanned_statement_has_no_plan_id() {
    // Arrange：手工构造的合成语句（ir_gen 内建 / 直驱夹具同款）
    use crate::ast::{Expr, Stmt, StmtKind};
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
