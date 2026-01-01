//! Parser module
//!
//! This module implements a Pratt Parser for the YaoXiang language.
//! The parser transforms tokens into an Abstract Syntax Tree (AST).

pub mod ast;
mod state;
mod nud;
mod led;
mod stmt;
mod expr;
mod type_parser;

pub use state::{ParserState, BP_LOWEST, BP_HIGHEST};

use crate::frontend::lexer::tokens::*;
use crate::util::span::Span;
use ast::*;

/// Parse tokens into an AST module
///
/// # Arguments
/// * `tokens` - Token stream from the lexer
///
/// # Returns
/// Parsed module or first parse error
///
/// # Example
/// ```yaoxiang
/// fn main() {
///     print("Hello");
/// }
/// ```
pub fn parse(tokens: &[Token]) -> Result<Module, ParseError> {
    let mut state = ParserState::new(tokens);
    let mut items = Vec::new();

    while !state.at_end() {
        match state.parse_stmt() {
            Some(stmt) => items.push(stmt),
            None => {
                // Skip to next statement or EOF
                state.synchronize();
            }
        }
    }

    if state.has_errors() {
        // Return the first error
        if let Some(error) = state.first_error().cloned() {
            Err(error)
        } else {
            // Should not happen, but return a generic error
            Err(ParseError::UnexpectedToken(
                state.current().map(|t| t.kind.clone()).unwrap_or(TokenKind::Eof),
            ))
        }
    } else {
        let span = if let Some(first) = items.first() {
            if let Some(last) = items.last() {
                Span::new(first.span.start, last.span.end)
            } else {
                Span::dummy()
            }
        } else {
            Span::dummy()
        };

        Ok(Module { items, span })
    }
}

/// Parse a single expression
///
/// # Arguments
/// * `tokens` - Token stream
///
/// # Returns
/// Parsed expression or error
pub fn parse_expression(tokens: &[Token]) -> Result<Expr, ParseError> {
    let mut state = ParserState::new(tokens);
    let expr = state.parse_expression(BP_LOWEST);

    match expr {
        Some(e) => {
            if state.has_errors() {
                if let Some(error) = state.first_error().cloned() {
                    Err(error)
                } else {
                    Ok(e)
                }
            } else {
                Ok(e)
            }
        }
        None => {
            if let Some(error) = state.first_error().cloned() {
                Err(error)
            } else {
                Err(ParseError::InvalidExpression)
            }
        }
    }
}

/// Parse error types
#[derive(Debug, thiserror::Error, Clone)]
pub enum ParseError {
    #[error("Unexpected token: {0:?}")]
    UnexpectedToken(TokenKind),

    #[error("Expected token: {0:?}, found: {1:?}")]
    ExpectedToken(TokenKind, TokenKind),

    #[error("Unterminated block")]
    UnterminatedBlock,

    #[error("Invalid expression")]
    InvalidExpression,

    #[error("Invalid pattern")]
    InvalidPattern,

    #[error("Invalid type annotation")]
    InvalidType,

    #[error("Missing semicolon after statement")]
    MissingSemicolon,

    #[error("Unexpected end of input")]
    UnexpectedEof,

    #[error("Parser error: {0}")]
    Custom(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontend::lexer::tokenize;

    #[test]
    fn test_parse_empty_module() {
        let tokens = tokenize("").unwrap();
        let result = parse(&tokens);
        assert!(result.is_ok());
        let module = result.unwrap();
        assert!(module.items.is_empty());
    }

    #[test]
    fn test_parse_simple_expression() {
        let tokens = tokenize("1 + 2").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_var_statement() {
        let tokens = tokenize("x: int = 42;").unwrap();
        let result = parse(&tokens);
        assert!(result.is_ok());
        let module = result.unwrap();
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn test_parse_fn_definition() {
        let tokens = tokenize("fn add(a: Int, b: Int) -> Int { a + b }").unwrap();
        let result = parse(&tokens);
        assert!(result.is_ok());
        let module = result.unwrap();
        assert_eq!(module.items.len(), 1);
    }

    #[test]
    fn test_parse_if_simple() {
        let tokens = tokenize("if true { 1 }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_if_with_comparison() {
        let tokens = tokenize("if x > 0 { 1 }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_if_expression() {
        let tokens = tokenize("if x > 0 { 1 } else { 0 }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_match_expression() {
        let tokens = tokenize("match x { 1 => \"one\", 2 => \"two\" }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_lambda() {
        let tokens = tokenize("|x| => x + 1").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;
    use crate::frontend::lexer::tokenize;

    // === Nested Control Flow ===

    #[test]
    fn test_parse_nested_if() {
        let tokens = tokenize("if true { if false { 1 } else { 2 } }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_nested_while() {
        let tokens = tokenize("while true { while false { break } }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_if_else_if() {
        // YaoXiang uses "else { if ... }" instead of "else if ..."
        let tokens = tokenize("if x > 0 { 1 } else { if x < 0 { -1 } else { 0 } }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_match_with_multiple_arms() {
        let tokens = tokenize("match x { 1 => \"one\", 2 => \"two\", 3 => \"three\", _ => \"other\" }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    // === Operator Precedence ===

    #[test]
    fn test_parse_operator_precedence_add_mul() {
        let tokens = tokenize("1 + 2 * 3").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_operator_precedence_complex() {
        let tokens = tokenize("1 + 2 * 3 - 4 / 2").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_comparison_chaining() {
        let tokens = tokenize("1 < 2 && 3 > 4").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_unary_minus() {
        let tokens = tokenize("-1 + 2").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_double_negation() {
        // Note: --5 is parsed as -(-5), but the lexer might not support it
        let tokens = tokenize("-( -5)").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    // === Lambda and Function Calls ===

    #[test]
    fn test_parse_lambda_with_multiple_params() {
        let tokens = tokenize("|a, b, c| => a + b + c").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_lambda_with_type_annot() {
        let tokens = tokenize("|x: Int| => x * 2").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_var_declaration() {
        // Simple variable declaration
        let tokens = tokenize("x = 1").unwrap();
        let result = parse(&tokens);
        assert!(result.is_ok(), "Simple variable declaration should parse successfully");
    }

    #[test]
    fn test_parse_nested_lambda() {
        let tokens = tokenize("|x| => |y| => x + y").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_call_with_args() {
        let tokens = tokenize("foo(1, 2, 3)").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_call_chaining() {
        let tokens = tokenize("foo(bar(baz(1)))").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_method_call() {
        let tokens = tokenize("obj.method(1, 2)").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    // === Literals and Types ===

    #[test]
    fn test_parse_string_with_escapes() {
        let tokens = tokenize("\"hello\\nworld\"").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_char_literal() {
        let tokens = tokenize("'a'").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_bool_literals() {
        let tokens = tokenize("true && false").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    // === Empty and Simple Blocks ===

    #[test]
    fn test_parse_empty_block() {
        let tokens = tokenize("{}").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_block_with_single_expr() {
        let tokens = tokenize("{ 42 }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_block_with_multiple_stmts() {
        let tokens = tokenize("{ let x = 1; let y = 2; x + y }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    // === For Loop ===

    #[test]
    fn test_parse_for_loop() {
        let tokens = tokenize("for item in items { print(item) }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    // Note: Range syntax (0..10) is not yet supported by the lexer

    // === Field Access and Indexing ===

    #[test]
    fn test_parse_field_access() {
        let tokens = tokenize("point.x").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_chained_field_access() {
        let tokens = tokenize("point.x.y.z").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_indexing() {
        let tokens = tokenize("arr[0]").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_field_and_index() {
        let tokens = tokenize("matrix[0].x").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    // === Match Patterns ===

    #[test]
    fn test_parse_match_with_identifier_pattern() {
        let tokens = tokenize("match x { Foo => 1, Bar => 2 }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_match_with_tuple_pattern() {
        let tokens = tokenize("match p { (x, y) => x + y }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_match_with_wildcard() {
        let tokens = tokenize("match x { _ => 0 }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_match_with_string_pattern() {
        let tokens = tokenize("match s { \"hello\" => 1, _ => 0 }").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    // === Type Cast ===

    #[test]
    fn test_parse_type_cast() {
        let tokens = tokenize("num as Float").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    // === Assignment ===

    #[test]
    fn test_parse_assignment() {
        let tokens = tokenize("x = 5").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_assignment_in_expression() {
        let tokens = tokenize("(x = 5) + 10").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    // === Tuple Expressions ===

    #[test]
    fn test_parse_empty_tuple() {
        let tokens = tokenize("()").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_single_element_tuple() {
        let tokens = tokenize("(42,)").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_multi_element_tuple() {
        let tokens = tokenize("(1, 2, 3)").unwrap();
        let result = parse_expression(&tokens);
        assert!(result.is_ok());
    }
}

#[cfg(test)]
mod fuzz_tests {
    use proptest::prelude::*;
    use super::*;
    use crate::frontend::lexer::tokenize;

    // === Property-based fuzz tests ===

    proptest! {
        #[test]
        fn fuzz_parse_simple_expression(s in "[0-9]+") {
            let tokens = tokenize(&s).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse simple integer: {}", s);
        }

        #[test]
        fn fuzz_parse_identifier(s in "[a-zA-Z][a-zA-Z0-9_]*") {
            // Skip reserved keywords
            let reserved = ["as", "if", "for", "while", "match", "fn", "let", "type", "pub", "use", "return", "break", "continue", "in", "spawn", "ref", "mut", "elif", "else"];
            if reserved.iter().any(|&k| s == k) {
                return Ok(());
            }
            let tokens = tokenize(&s).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse identifier: {}", s);
        }

        #[test]
        fn fuzz_parse_binary_expr(
            left in "[0-9]+",
            op in "\\+|\\-|\\*|/|%",
            right in "[0-9]+"
        ) {
            let code = format!("{} {} {}", left, op, right);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_parenthesized_expr(s in "[0-9]+") {
            let code = format!("({})", s);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_unary_minus(s in "[0-9]+") {
            let code = format!("-{}", s);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_call_with_args(
            func in "[a-z][a-z0-9]*",
            arg1 in "[0-9]+",
            arg2 in "[0-9]+"
        ) {
            // Skip reserved keywords
            let reserved = ["as", "if", "for", "while", "match", "fn", "let", "type", "pub", "use", "return", "break", "continue", "in", "spawn", "ref", "mut", "elif", "else"];
            if reserved.iter().any(|&k| func == k) {
                return Ok(());
            }
            let code = format!("{}({}, {})", func, arg1, arg2);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_field_access(
            obj in "[a-z][a-z0-9]*",
            field in "[a-z][a-z0-9]*"
        ) {
            // Skip reserved keywords
            let reserved = ["as", "if", "for", "while", "match", "fn", "let", "type", "pub", "use", "return", "break", "continue", "in", "spawn", "ref", "mut", "elif", "else"];
            if reserved.iter().any(|&k| obj == k || field == k) {
                return Ok(());
            }
            let code = format!("{}.{}", obj, field);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_indexing(
            arr in "[a-z][a-z0-9]*",
            idx in "[0-9]+"
        ) {
            // Skip reserved keywords
            let reserved = ["as", "if", "for", "while", "match", "fn", "let", "type", "pub", "use", "return", "break", "continue", "in", "spawn", "ref", "mut", "elif", "else"];
            if reserved.iter().any(|&k| arr == k) {
                return Ok(());
            }
            let code = format!("{}[{}]", arr, idx);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_logical_expr(
            a in "true|false",
            b in "true|false",
            op in "&&|\\|\\|"
        ) {
            let code = format!("{} {} {}", a, op, b);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_comparison(
            a in "[0-9]+",
            b in "[0-9]+",
            op in "!=|>|<|<=|>="
        ) {
            let code = format!("{} {} {}", a, op, b);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_nested_parens(
            depth in 1..4usize,
            s in "[0-9]+"
        ) {
            let code: String = (0..depth).map(|_| "(").chain(std::iter::once(s.as_str())).chain((0..depth).map(|_| ")")).collect();
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_call_chain(
            func1 in "[a-z]+",
            func2 in "[a-z]+",
            arg in "[0-9]+"
        ) {
            // Skip reserved keywords
            let reserved = ["as", "if", "for", "while", "match", "fn", "let", "type", "pub", "use", "return", "break", "continue", "in", "spawn", "ref", "mut", "elif", "else"];
            if reserved.iter().any(|&k| func1 == k || func2 == k) {
                return Ok(());
            }
            let code = format!("{}({}({}))", func1, func2, arg);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_lambda_simple(
            param in "[a-z]+",
            arg in "[0-9]+"
        ) {
            // Skip reserved keywords
            let reserved = ["as", "if", "for", "while", "match", "fn", "let", "type", "pub", "use", "return", "break", "continue", "in", "spawn", "ref", "mut", "elif", "else"];
            if reserved.iter().any(|&k| param == k) {
                return Ok(());
            }
            let code = format!("|{}| => {}", param, arg);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_if_simple(
            cond in "true|false",
            then_val in "[0-9]+",
            else_val in "[0-9]+"
        ) {
            let code = format!("if {} {{ {} }} else {{ {} }}", cond, then_val, else_val);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_while_simple(
            cond in "true|false",
            body in "[0-9]+"
        ) {
            let code = format!("while {} {{ {} }}", cond, body);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_for_simple(
            var in "[a-z]+",
            iterable in "[a-z]+",
            body in "[0-9]+"
        ) {
            // Skip reserved keywords
            let reserved = ["as", "if", "for", "while", "match", "fn", "let", "type", "pub", "use", "return", "break", "continue", "in", "spawn", "ref", "mut", "elif", "else"];
            if reserved.iter().any(|&k| var == k || iterable == k) {
                return Ok(());
            }
            let code = format!("for {} in {} {{ {} }}", var, iterable, body);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }

        #[test]
        fn fuzz_parse_match_simple(
            var in "[a-z]+",
            pat1 in "[0-9]+",
            val1 in "[a-z]+",
            pat2 in "[0-9]+",
            val2 in "[a-z]+"
        ) {
            // Skip reserved keywords
            let reserved = ["as", "if", "for", "while", "match", "fn", "let", "type", "pub", "use", "return", "break", "continue", "in", "spawn", "ref", "mut", "elif", "else"];
            if reserved.iter().any(|&k| var == k || val1 == k || val2 == k) {
                return Ok(());
            }
            let code = format!("match {} {{ {} => {}, {} => {} }}", var, pat1, val1, pat2, val2);
            let tokens = tokenize(&code).unwrap();
            let result = parse_expression(&tokens);
            prop_assert!(result.is_ok(), "Failed to parse: {}", code);
        }
    }
}
