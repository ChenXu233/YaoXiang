//! Tests for ParserState: helpers, error tracking, save/restore.

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::ParserState;
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
        state.error(parse_msg("test error"));
        assert!(state.has_errors());
        assert_eq!(state.error_count(), 1);
    });
}

#[test]
fn test_take_errors() {
    with_state("x", |state| {
        state.error(parse_msg("err1"));
        state.error(parse_msg("err2"));
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
