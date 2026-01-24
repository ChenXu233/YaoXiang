//! Coverage tests - Tests for parser branches that need coverage but aren't in basic/boundary
//!
//! 这些测试针对需要覆盖但 basic.rs 和 boundary.rs 没有覆盖到的代码路径

use super::*;
use crate::frontend::lexer::tokenize;
use crate::frontend::parser::{parse, parse_expression, ParserState};
use crate::util::span::Span;

/// =========================================================================
/// nud.rs 覆盖率补充测试 - 前缀操作符和字面量
/// =========================================================================

#[test]
fn test_parse_negative_float() {
    let tokens = tokenize("-3.14").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_not_of_comparison() {
    let tokens = tokenize("!(1 < 2)").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_unary_plus_on_expression() {
    let tokens = tokenize("+(a + b)").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
/// led.rs 覆盖率补充测试 - 中缀操作符和函数调用
/// =========================================================================

#[test]
fn test_parse_range_inclusive() {
    let tokens = tokenize("[1..10]").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_range_assignment() {
    let tokens = tokenize("r = 1..10").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_nested_index() {
    let tokens = tokenize("matrix[0][1]").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_call_with_no_args() {
    let tokens = tokenize("foo()").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_call_with_multiple_args() {
    let tokens = tokenize("foo(1, 2, 3, 4, 5)").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_field_then_call() {
    let tokens = tokenize("obj.method()").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_cast_to_type() {
    let tokens = tokenize("x as Int").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
/// stmt.rs 覆盖率补充测试 - 语句解析边界
/// =========================================================================

#[test]
fn test_parse_use_with_alias() {
    let tokens = tokenize("use std.io as io;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_use_with_items() {
    let tokens = tokenize("use std.io.{Reader, Writer};").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_multiple_statements_same_line() {
    let tokens = tokenize("x = 1; y = 2; z = 3;").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
    let module = result.unwrap();
    assert_eq!(module.items.len(), 3);
}

#[test]
fn test_parse_block_empty() {
    let tokens = tokenize("x = {}").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_block_with_single_expr() {
    let tokens = tokenize("x = { 42 }").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_block_with_multiple_stmts() {
    let tokens = tokenize("x = { a = 1; b = 2; a + b }").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_while_with_break() {
    let tokens = tokenize("while true { break }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_while_with_continue() {
    let tokens = tokenize("i = 0; while i < 10 { i = i + 1; continue }").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_labeled_break() {
    let tokens = tokenize("break ::outer").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_for_with_range() {
    let tokens = tokenize("for i in 0..10 { print(i) }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_for_with_variable() {
    let tokens = tokenize("for item in items { print(item) }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_if_without_else() {
    let tokens = tokenize("if x > 0 { 1 }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_if_elif_chain() {
    let tokens = tokenize("if a { 1 } elif b { 2 } elif c { 3 } else { 4 }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_with_guard() {
    let tokens = tokenize("match x { n if n > 0 => 1, _ => 0 }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_match_with_multiple_patterns() {
    let tokens =
        tokenize("match x { 1 => \"one\", 2 => \"two\", 3 => \"three\", _ => \"other\" }").unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
}

#[test]
fn test_parse_return_in_block() {
    let tokens = tokenize("add: (Int, Int) -> Int = (a, b) => { return a + b }").unwrap();
    let result = parse(&tokens);
    assert!(result.is_ok());
}

/// =========================================================================
/// type_parser.rs 覆盖率补充测试 - 类型注解解析
/// =========================================================================

fn parse_type_anno(
    tokens: &[crate::frontend::lexer::tokens::Token]
) -> Option<crate::frontend::parser::ast::Type> {
    let mut state = ParserState::new(tokens);
    state.parse_type_anno()
}

#[test]
fn test_parse_type_void() {
    let tokens = tokenize("Void").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_bytes() {
    let tokens = tokenize("bytes").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_named() {
    let tokens = tokenize("MyType").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_qualified_name() {
    let tokens = tokenize("std.io.Reader").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_generic_single_arg() {
    let tokens = tokenize("List[int]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_generic_multiple_args() {
    let tokens = tokenize("Dict[string, int]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_nested_generic() {
    let tokens = tokenize("List[List[int]]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_list() {
    let tokens = tokenize("[int]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_empty_tuple() {
    let tokens = tokenize("()").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_single_element_tuple() {
    let tokens = tokenize("(int,)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_two_element_tuple() {
    let tokens = tokenize("(int, string)").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_fn_no_params() {
    let tokens = tokenize("() -> int").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_fn_with_params() {
    let tokens = tokenize("(int, string) -> bool").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_fn_returns_fn() {
    let tokens = tokenize("(int) -> (string) -> bool").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_struct_empty() {
    let tokens = tokenize("{}").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_struct_single_field() {
    let tokens = tokenize("{ x: int }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_struct_multiple_fields() {
    let tokens = tokenize("{ x: int, y: string, z: bool }").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_option() {
    let tokens = tokenize("Option[int]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_result() {
    let tokens = tokenize("Result[int, string]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

#[test]
fn test_parse_type_set() {
    let tokens = tokenize("Set[int]").unwrap();
    let result = parse_type_anno(&tokens);
    assert!(result.is_some());
}

/// =========================================================================
/// state.rs 覆盖率补充测试 - ParserState 方法
/// =========================================================================

fn create_token(
    kind: crate::frontend::lexer::tokens::TokenKind
) -> crate::frontend::lexer::tokens::Token {
    use crate::util::span::{Position, Span};
    crate::frontend::lexer::tokens::Token {
        kind,
        span: Span::new(
            Position::with_offset(1, 1, 0),
            Position::with_offset(1, 2, 1),
        ),
        literal: None,
    }
}

#[test]
fn test_state_peek_nth_zero() {
    let tokens = tokenize("42 + 10").unwrap();
    let state = ParserState::new(&tokens);
    let peeked = state.peek_nth(0);
    assert!(peeked.is_some());
}

#[test]
fn test_state_peek_nth_one() {
    let tokens = tokenize("42 + 10").unwrap();
    let state = ParserState::new(&tokens);
    let peeked = state.peek_nth(1);
    assert!(peeked.is_some());
}

#[test]
fn test_state_peek_nth_two() {
    let tokens = tokenize("42 + 10").unwrap();
    let state = ParserState::new(&tokens);
    let peeked = state.peek_nth(2);
    assert!(peeked.is_some());
}

#[test]
fn test_state_peek_nth_out_of_bounds() {
    let tokens = tokenize("42").unwrap();
    let state = ParserState::new(&tokens);
    let peeked = state.peek_nth(100);
    assert!(peeked.is_none());
}

#[test]
fn test_state_at_with_different_int_values() {
    let tokens = tokenize("42 100").unwrap();
    let state = ParserState::new(&tokens);
    use crate::frontend::lexer::tokens::TokenKind;
    assert!(state.at(&TokenKind::IntLiteral(42)));
    assert!(!state.at(&TokenKind::IntLiteral(100)));
}

#[test]
fn test_state_skip_matching_token() {
    let tokens = tokenize("+ 42").unwrap();
    let mut state = ParserState::new(&tokens);
    use crate::frontend::lexer::tokens::TokenKind;
    assert!(state.skip(&TokenKind::Plus));
    assert!(state.at(&TokenKind::IntLiteral(42)));
}

#[test]
fn test_state_skip_non_matching_token() {
    let tokens = tokenize("+ 42").unwrap();
    let mut state = ParserState::new(&tokens);
    use crate::frontend::lexer::tokens::TokenKind;
    assert!(!state.skip(&TokenKind::IntLiteral(42)));
    assert!(state.at(&TokenKind::Plus));
}

#[test]
fn test_state_expect_wrong_token() {
    let tokens = tokenize("42").unwrap();
    let mut state = ParserState::new(&tokens);
    use crate::frontend::lexer::tokens::TokenKind;
    assert!(!state.expect(&TokenKind::Plus));
    assert!(state.has_errors());
}

#[test]
fn test_state_expect_correct_token() {
    let tokens = tokenize("42").unwrap();
    let mut state = ParserState::new(&tokens);
    use crate::frontend::lexer::tokens::TokenKind;
    assert!(state.expect(&TokenKind::IntLiteral(42)));
    assert!(state.at_end());
}

#[test]
fn test_state_at_end_with_eof() {
    let tokens = tokenize("").unwrap();
    let state = ParserState::new(&tokens);
    assert!(state.at_end());
}

#[test]
fn test_state_at_end_after_consume() {
    let tokens = tokenize("42").unwrap();
    let mut state = ParserState::new(&tokens);
    state.bump();
    assert!(state.at_end());
}

#[test]
fn test_state_has_errors_initially() {
    let tokens = tokenize("").unwrap();
    let state = ParserState::new(&tokens);
    assert!(!state.has_errors());
}

#[test]
fn test_state_first_error() {
    let tokens = tokenize("42").unwrap();
    let mut state = ParserState::new(&tokens);
    use crate::frontend::parser::ParseError;
    use crate::frontend::lexer::tokens::TokenKind;
    state.error(ParseError::ExpectedToken {
        expected: TokenKind::Plus,
        found: TokenKind::IntLiteral(42),
        span: Span::dummy(),
    });
    let first = state.first_error();
    assert!(first.is_some());
}

#[test]
fn test_state_into_errors() {
    let tokens = tokenize("42").unwrap();
    let mut state = ParserState::new(&tokens);
    use crate::frontend::parser::ParseError;
    use crate::frontend::lexer::tokens::TokenKind;
    state.error(ParseError::ExpectedToken {
        expected: TokenKind::Plus,
        found: TokenKind::IntLiteral(42),
        span: Span::dummy(),
    });
    state.error(ParseError::ExpectedToken {
        expected: TokenKind::Minus,
        found: TokenKind::IntLiteral(42),
        span: Span::dummy(),
    });
    let errors = state.into_errors();
    assert_eq!(errors.len(), 2);
}

#[test]
fn test_state_span() {
    let tokens = tokenize("42").unwrap();
    let state = ParserState::new(&tokens);
    let span = state.span();
    assert!(!span.is_dummy());
}

#[test]
fn test_state_start_span() {
    let tokens = tokenize("42 + 10").unwrap();
    let mut state = ParserState::new(&tokens);
    state.start_span();
    let span = state.span();
    assert!(!span.is_dummy());
}

#[test]
fn test_state_span_from() {
    let tokens = tokenize("42").unwrap();
    let mut state = ParserState::new(&tokens);
    let start = state.span();
    state.bump();
    let span = state.span_from(start);
    assert!(!span.is_dummy());
}

#[test]
fn test_state_can_start_stmt_with_keywords() {
    use crate::frontend::lexer::tokens::TokenKind;
    let cases = vec![
        TokenKind::KwMut,
        TokenKind::KwType,
        TokenKind::KwUse,
        TokenKind::KwReturn,
        TokenKind::KwBreak,
        TokenKind::KwContinue,
        TokenKind::KwIf,
        TokenKind::KwMatch,
        TokenKind::KwWhile,
        TokenKind::KwFor,
    ];
    for kind in cases {
        let tokens = vec![create_token(kind.clone())];
        let state = ParserState::new(&tokens);
        assert!(
            state.can_start_stmt(),
            " {:?} should start a statement",
            kind
        );
    }
}

#[test]
fn test_state_can_start_expr_with_punctuation() {
    use crate::frontend::lexer::tokens::TokenKind;
    let cases = vec![
        TokenKind::Minus,
        TokenKind::Plus,
        TokenKind::Not,
        TokenKind::LParen,
        TokenKind::LBrace,
        TokenKind::LBracket,
        TokenKind::Pipe,
    ];
    for kind in cases {
        let tokens = vec![create_token(kind.clone())];
        let state = ParserState::new(&tokens);
        assert!(
            state.can_start_expr(),
            " {:?} should start an expression",
            kind
        );
    }
}

#[test]
fn test_state_can_start_expr_with_literals() {
    use crate::frontend::lexer::tokens::TokenKind;
    let cases = vec![
        TokenKind::IntLiteral(0),
        TokenKind::FloatLiteral(0.0),
        TokenKind::StringLiteral(String::new()),
        TokenKind::CharLiteral('a'),
        TokenKind::BoolLiteral(true),
    ];
    for kind in cases {
        let tokens = vec![create_token(kind.clone())];
        let state = ParserState::new(&tokens);
        assert!(
            state.can_start_expr(),
            " {:?} should start an expression",
            kind
        );
    }
}

#[test]
fn test_state_synchronize_skips_to_sync_point() {
    use crate::frontend::lexer::tokens::*;
    let tokens = vec![
        create_token(TokenKind::IntLiteral(42)),
        create_token(TokenKind::Error("error".to_string())),
        create_token(TokenKind::IntLiteral(10)),
        create_token(TokenKind::KwMut), // sync point
        create_token(TokenKind::IntLiteral(20)),
        create_token(TokenKind::Eof),
    ];
    let mut state = ParserState::new(&tokens);
    state.synchronize();
    assert!(state.at(&TokenKind::KwMut));
}

#[test]
fn test_state_skip_to_sync() {
    use crate::frontend::lexer::tokens::*;
    let tokens = vec![
        create_token(TokenKind::IntLiteral(42)),
        create_token(TokenKind::Error("error".to_string())),
        create_token(TokenKind::KwType), // sync point
        create_token(TokenKind::Eof),
    ];
    let mut state = ParserState::new(&tokens);
    state.skip_to_sync();
    assert!(state.at(&TokenKind::KwType));
}
