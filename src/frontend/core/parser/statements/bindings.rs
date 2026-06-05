//! RFC-004 Binding syntax parsing
//! Handles binding declarations: `Type.method = value`

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::ParserState;
use crate::frontend::core::parser::statements::types::parse_type_annotation;
use crate::frontend::core::parser::statements::functions::{parse_fn_params, parse_fn_body};
use crate::util::span::Span;

/// Parse method binding: `Type.method: (Type, ...) -> ReturnType = (params) => body`
pub fn parse_method_bind(
    state: &mut ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    // Parse type name
    let type_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    if !state.expect(&TokenKind::Dot) {
        return None;
    }

    let method_name = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => return None,
    };
    state.bump();

    if !state.expect(&TokenKind::Colon) {
        return None;
    }

    let method_type = parse_type_annotation(state)?;

    if !state.expect(&TokenKind::Eq) {
        return None;
    }

    if !state.expect(&TokenKind::LParen) {
        return None;
    }
    let params = parse_fn_params(state)?;
    if !state.expect(&TokenKind::RParen) {
        return None;
    }

    if !state.expect(&TokenKind::FatArrow) {
        return None;
    }

    let (stmts, expr) = parse_fn_body(state)?;

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::Binding {
            name: method_name,
            type_name: Some(type_name),
            method_type: Some(method_type),
            generic_params: Vec::new(),
            type_annotation: None,
            params,
            body: (stmts, expr),
            is_pub: false,
        },
        span,
    })
}

/// RFC-004 Binding parser
pub struct BindingParser {}

impl Default for BindingParser {
    fn default() -> Self {
        Self::new()
    }
}

impl BindingParser {
    pub fn new() -> Self {
        Self {}
    }

    /// Parse binding declaration: `Type.method = value`
    pub fn parse_binding(
        &self,
        tokens: &[Token],
        _start_pos: usize,
    ) -> Result<Stmt, crate::frontend::core::parser::ParseError> {
        // RFC-004 Binding syntax parser:
        // Format: Type.method = value
        let mut state = ParserState::new(tokens);

        // Parse type name
        let _type_name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                return Err(crate::frontend::core::parser::ParseError::UnexpectedToken {
                    found: state
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                    span: state.span(),
                });
            }
        };
        state.bump();

        // Expect dot
        if !state.skip(&TokenKind::Dot) {
            return Err(crate::frontend::core::parser::ParseError::ExpectedToken {
                expected: TokenKind::Dot,
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.span(),
            });
        }

        // Parse method name
        let _method_name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => {
                return Err(crate::frontend::core::parser::ParseError::UnexpectedToken {
                    found: state
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                    span: state.span(),
                });
            }
        };
        state.bump();

        // Expect equals
        if !state.skip(&TokenKind::Eq) {
            return Err(crate::frontend::core::parser::ParseError::ExpectedToken {
                expected: TokenKind::Eq,
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.span(),
            });
        }

        // Parse value expression
        let value = state.parse_expression(crate::frontend::core::parser::BP_LOWEST);

        let value_span = state.span();
        Ok(Stmt {
            kind: StmtKind::Expr(Box::new(
                value.unwrap_or(Expr::Lit(Literal::Bool(false), value_span)),
            )),
            span: value_span,
        })
    }

    pub fn validate_binding_syntax(
        &self,
        binding: &str,
    ) -> Result<(), String> {
        if !binding.contains('=') {
            return Err("Invalid binding syntax: missing '='".to_string());
        }
        Ok(())
    }
}

/// Binding position validator
pub struct BindingPositionValidator {
    max_positions: usize,
}

impl BindingPositionValidator {
    pub fn new(max_positions: usize) -> Self {
        Self { max_positions }
    }

    pub fn validate_positions(
        &self,
        positions: &[i32],
    ) -> Result<(), String> {
        for &pos in positions {
            if pos < 0 {
                return Err(format!("Negative position index: {}", pos));
            }
            if pos as usize >= self.max_positions {
                return Err(format!(
                    "Position index {} exceeds maximum allowed positions {}",
                    pos, self.max_positions
                ));
            }
        }
        Ok(())
    }

    pub fn validate_binding_syntax(
        &self,
        binding: &str,
    ) -> Result<(), String> {
        if !binding.contains('[') || !binding.contains(']') {
            return Err("Invalid binding syntax: missing brackets".to_string());
        }
        Ok(())
    }
}
