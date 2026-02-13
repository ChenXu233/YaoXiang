//! Parser state tests - ParserState 单元测试

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::parser_state::{ParserState, ParseError};
use crate::util::span::{Position, Span};

fn create_token(kind: TokenKind) -> Token {
    Token {
        kind,
        span: Span::new(
            Position::with_offset(1, 1, 0),
            Position::with_offset(1, 2, 1),
        ),
        literal: None,
    }
}

fn create_dummy_span() -> Span {
    Span::dummy()
}

#[cfg(test)]
mod parser_state_tests {
    use super::*;

    // =========================================================================
    // ParserState 初始化测试
    // =========================================================================

    #[test]
    fn test_parser_state_new() {
        let tokens = vec![];
        let state = ParserState::new(&tokens);

        assert!(state.at_end());
        assert!(!state.has_errors());
    }

    #[test]
    fn test_parser_state_new_with_tokens() {
        let tokens = vec![
            create_token(TokenKind::IntLiteral(42)),
            create_token(TokenKind::Plus),
            create_token(TokenKind::IntLiteral(10)),
        ];
        let state = ParserState::new(&tokens);

        assert!(!state.at_end());
        assert!(state.current().is_some());
    }

    // =========================================================================
    // 结束判断测试
    // =========================================================================

    #[test]
    fn test_at_end_true() {
        let tokens = vec![];
        let state = ParserState::new(&tokens);
        assert!(state.at_end());
    }

    #[test]
    fn test_at_end_false_with_tokens() {
        let tokens = vec![create_token(TokenKind::IntLiteral(42))];
        let state = ParserState::new(&tokens);
        assert!(!state.at_end());
    }

    // =========================================================================
    // 当前 token 测试
    // =========================================================================

    #[test]
    fn test_current() {
        let tokens = vec![
            create_token(TokenKind::IntLiteral(42)),
            create_token(TokenKind::Plus),
        ];
        let state = ParserState::new(&tokens);

        assert!(state.current().is_some());
        if let Some(token) = state.current() {
            assert!(matches!(token.kind, TokenKind::IntLiteral(42)));
        }
    }

    #[test]
    fn test_current_none_at_end() {
        let tokens = vec![];
        let state = ParserState::new(&tokens);
        assert!(state.current().is_none());
    }

    // =========================================================================
    // Token 类型判断测试
    // =========================================================================

    #[test]
    fn test_at() {
        let tokens = vec![create_token(TokenKind::IntLiteral(42))];
        let state = ParserState::new(&tokens);

        assert!(state.at(&TokenKind::IntLiteral(42)));
        assert!(!state.at(&TokenKind::Plus));
    }

    #[test]
    fn test_at_multiple_tokens() {
        let tokens = vec![
            create_token(TokenKind::IntLiteral(42)),
            create_token(TokenKind::Plus),
        ];
        let mut state = ParserState::new(&tokens);

        assert!(state.at(&TokenKind::IntLiteral(42)));
        assert!(!state.at(&TokenKind::Plus));

        state.bump();
        assert!(!state.at(&TokenKind::IntLiteral(42)));
        assert!(state.at(&TokenKind::Plus));
    }

    // =========================================================================
    // Peek 测试
    // =========================================================================

    #[test]
    fn test_peek() {
        let tokens = vec![
            create_token(TokenKind::IntLiteral(42)),
            create_token(TokenKind::Plus),
        ];
        let state = ParserState::new(&tokens);

        let peeked = state.peek();
        assert!(peeked.is_some());
        if let Some(token) = peeked {
            assert!(matches!(token.kind, TokenKind::Plus));
        }
    }

    #[test]
    fn test_peek_none_at_end() {
        let tokens = vec![create_token(TokenKind::Eof)];
        let state = ParserState::new(&tokens);

        let peeked = state.peek();
        assert!(peeked.is_none());
    }

    #[test]
    fn test_peek_nth() {
        let tokens = vec![
            create_token(TokenKind::IntLiteral(42)),
            create_token(TokenKind::Plus),
            create_token(TokenKind::IntLiteral(10)),
        ];
        let state = ParserState::new(&tokens);

        assert!(state.peek_nth(0).is_some());
        assert!(state.peek_nth(1).is_some());
        assert!(state.peek_nth(2).is_some());
        assert!(state.peek_nth(3).is_none());

        if let Some(token) = state.peek_nth(1) {
            assert!(matches!(token.kind, TokenKind::Plus));
        }
    }

    // =========================================================================
    // Bump 测试
    // =========================================================================

    #[test]
    fn test_bump() {
        let tokens = vec![
            create_token(TokenKind::IntLiteral(42)),
            create_token(TokenKind::Plus),
        ];
        let mut state = ParserState::new(&tokens);

        assert!(state.at(&TokenKind::IntLiteral(42)));
        state.bump();
        assert!(state.at(&TokenKind::Plus));
    }

    #[test]
    fn test_bump_past_end() {
        let tokens = vec![create_token(TokenKind::IntLiteral(42))];
        let mut state = ParserState::new(&tokens);

        state.bump();
        // Should not panic, just stay at end
        assert!(state.at_end());
    }

    // =========================================================================
    // Skip 测试
    // =========================================================================

    #[test]
    fn test_skip_true() {
        let tokens = vec![
            create_token(TokenKind::Plus),
            create_token(TokenKind::IntLiteral(42)),
        ];
        let mut state = ParserState::new(&tokens);

        assert!(state.skip(&TokenKind::Plus));
        assert!(state.at(&TokenKind::IntLiteral(42)));
    }

    #[test]
    fn test_skip_false() {
        let tokens = vec![create_token(TokenKind::IntLiteral(42))];
        let mut state = ParserState::new(&tokens);

        assert!(!state.skip(&TokenKind::Plus));
        assert!(state.at(&TokenKind::IntLiteral(42)));
    }

    #[test]
    fn test_skip_chain() {
        let tokens = vec![
            create_token(TokenKind::Plus),
            create_token(TokenKind::Plus),
            create_token(TokenKind::IntLiteral(42)),
        ];
        let mut state = ParserState::new(&tokens);

        assert!(state.skip(&TokenKind::Plus));
        assert!(state.skip(&TokenKind::Plus));
        assert!(state.at(&TokenKind::IntLiteral(42)));
    }

    // =========================================================================
    // Expect 测试
    // =========================================================================

    #[test]
    fn test_expect_true() {
        let tokens = vec![create_token(TokenKind::IntLiteral(42))];
        let mut state = ParserState::new(&tokens);

        assert!(state.expect(&TokenKind::IntLiteral(42)));
        assert!(state.at_end());
    }

    #[test]
    fn test_expect_false_wrong_token() {
        let tokens = vec![create_token(TokenKind::IntLiteral(42))];
        let mut state = ParserState::new(&tokens);

        assert!(!state.expect(&TokenKind::Plus));
        assert!(state.has_errors());
    }

    #[test]
    fn test_expect_false_eof() {
        let tokens = vec![];
        let mut state = ParserState::new(&tokens);

        assert!(!state.expect(&TokenKind::IntLiteral(42)));
        assert!(state.has_errors());
    }

    // =========================================================================
    // 错误处理测试
    // =========================================================================

    #[test]
    fn test_error() {
        let tokens = vec![create_token(TokenKind::IntLiteral(42))];
        let mut state = ParserState::new(&tokens);

        state.error(ParseError::ExpectedToken {
            expected: TokenKind::Plus,
            found: TokenKind::IntLiteral(42),
            span: Span::dummy(),
        });

        assert!(state.has_errors());
        assert!(state.first_error().is_some());
    }

    #[test]
    fn test_first_error() {
        let tokens = vec![create_token(TokenKind::IntLiteral(42))];
        let mut state = ParserState::new(&tokens);

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

        let first = state.first_error();
        assert!(first.is_some());
    }

    // =========================================================================
    // 同步点测试
    // =========================================================================

    #[test]
    fn test_synchronize() {
        let tokens = vec![
            create_token(TokenKind::IntLiteral(42)),
            create_token(TokenKind::Plus),
            create_token(TokenKind::IntLiteral(10)),
            create_token(TokenKind::Eof),
        ];

        // Set pos to middle
        let tokens_ref: &[Token] = &tokens;
        let mut state = ParserState::new(tokens_ref);
        state.bump(); // At Plus
        state.error(ParseError::ExpectedToken {
            expected: TokenKind::IntLiteral(1),
            found: TokenKind::Plus,
            span: Span::dummy(),
        });

        state.synchronize();

        // Should stop at EOF (a sync point)
        assert!(state.at_end());
    }

    // =========================================================================
    // 语句开始判断测试
    // =========================================================================

    #[test]
    fn test_can_start_stmt_int() {
        let tokens = vec![create_token(TokenKind::IntLiteral(42))];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_string() {
        let tokens = vec![create_token(TokenKind::StringLiteral("hello".to_string()))];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_identifier() {
        let tokens = vec![create_token(TokenKind::Identifier("x".to_string()))];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_kw_mut() {
        let tokens = vec![create_token(TokenKind::KwMut)];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_identifier_type() {
        // RFC-010: 'type' is no longer a keyword, it's just a regular identifier
        // Type definitions use `Name: Type = { ... }` syntax (starts with Identifier)
        let tokens = vec![create_token(TokenKind::Identifier("Type".to_string()))];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_kw_if() {
        let tokens = vec![create_token(TokenKind::KwIf)];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_kw_while() {
        let tokens = vec![create_token(TokenKind::KwWhile)];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_kw_for() {
        let tokens = vec![create_token(TokenKind::KwFor)];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_kw_match() {
        let tokens = vec![create_token(TokenKind::KwMatch)];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_kw_return() {
        let tokens = vec![create_token(TokenKind::KwReturn)];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_kw_break() {
        let tokens = vec![create_token(TokenKind::KwBreak)];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_kw_continue() {
        let tokens = vec![create_token(TokenKind::KwContinue)];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_lbrace() {
        let tokens = vec![create_token(TokenKind::LBrace)];
        let state = ParserState::new(&tokens);

        assert!(state.can_start_stmt());
    }

    #[test]
    fn test_can_start_stmt_not_at_end() {
        let tokens = vec![create_token(TokenKind::Eof)];
        let state = ParserState::new(&tokens);

        assert!(!state.can_start_stmt());
    }

    // =========================================================================
    // Span 测试
    // =========================================================================

    #[test]
    fn test_span() {
        let tokens = vec![create_token(TokenKind::IntLiteral(42))];
        let state = ParserState::new(&tokens);

        let span = state.span();
        assert!(!span.is_dummy());
    }

    #[test]
    fn test_span_after_bump() {
        let tokens = vec![
            create_token(TokenKind::IntLiteral(42)),
            create_token(TokenKind::Plus),
        ];
        let mut state = ParserState::new(&tokens);

        // Initial span should be valid
        let span1 = state.span();
        assert!(!span1.is_dummy());

        // After bump, span should still be valid
        state.bump();
        let span2 = state.span();
        assert!(!span2.is_dummy());
    }

    #[test]
    fn test_at_end_with_eof() {
        let tokens = vec![
            create_token(TokenKind::IntLiteral(42)),
            create_token(TokenKind::Eof),
        ];
        let mut state = ParserState::new(&tokens);
        // Initial state: not at end (pos=0, token is IntLiteral)
        assert!(!state.at_end());
        // After bumping: at end (current token is Eof)
        state.bump();
        assert!(state.at_end());
    }
}
