//! RFC-004 Binding syntax parsing
//! Handles binding declarations: `Type.method = value`

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::frontend::core::parser::ParserState;
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
        kind: StmtKind::MethodBind {
            type_name,
            method_name,
            method_type,
            params,
            body: (stmts, expr),
        },
        span,
    })
}

/// RFC-004 Binding parser
pub struct BindingParser {
    /// Maximum binding positions allowed
    max_positions: usize,
}

impl BindingParser {
    pub fn new(max_positions: usize) -> Self {
        Self { max_positions }
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

// Helper functions (should be in declarations.rs, duplicated here for completeness)
fn parse_type_annotation(state: &mut ParserState<'_>) -> Option<Type> {
    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            Some(Type::Name(name))
        }
        Some(TokenKind::LParen) => parse_tuple_type(state),
        Some(TokenKind::LBrace) => parse_struct_type(state),
        _ => None,
    }
}

fn parse_tuple_type(state: &mut ParserState<'_>) -> Option<Type> {
    state.skip(&TokenKind::LParen);
    let mut types = Vec::new();
    if !state.at(&TokenKind::RParen) {
        while let Some(ty) = parse_type_annotation(state) {
            types.push(ty);
            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
    }
    state.skip(&TokenKind::RParen);
    Some(Type::Tuple(types))
}

fn parse_struct_type(state: &mut ParserState<'_>) -> Option<Type> {
    state.skip(&TokenKind::LBrace);
    let mut fields = Vec::new();
    if !state.at(&TokenKind::RBrace) {
        while let Some(TokenKind::Identifier(name)) = state.current().map(|t| &t.kind) {
            let name = name.clone();
            state.bump();
            state.skip(&TokenKind::Colon);
            let field_type = parse_type_annotation(state)?;
            fields.push((name, field_type));
            if !state.skip(&TokenKind::Comma) {
                break;
            }
        }
    }
    state.skip(&TokenKind::RBrace);
    Some(Type::Struct(fields))
}

fn parse_fn_params(state: &mut ParserState<'_>) -> Option<Vec<Param>> {
    let mut params = Vec::new();
    while !state.at(&TokenKind::RParen) && !state.at_end() {
        if !params.is_empty() && !state.expect(&TokenKind::Comma) {
            return None;
        }
        if state.at(&TokenKind::RParen) {
            break;
        }
        let param_span = state.span();
        let name = match state.current().map(|t| &t.kind) {
            Some(TokenKind::Identifier(n)) => n.clone(),
            _ => break,
        };
        state.bump();
        let ty = if state.skip(&TokenKind::Colon) {
            parse_type_annotation(state)
        } else {
            None
        };
        params.push(Param {
            name,
            ty,
            span: param_span,
        });
    }
    Some(params)
}

fn parse_fn_body(state: &mut ParserState<'_>) -> Option<(Vec<Stmt>, Option<Box<Expr>>)> {
    if state.at(&TokenKind::LBrace) {
        if !state.expect(&TokenKind::LBrace) {
            return None;
        }
        let body = parse_block_body(state)?;
        if !state.expect(&TokenKind::RBrace) {
            return None;
        }
        Some(body)
    } else {
        let expr = state.parse_expression(crate::frontend::core::parser::BP_LOWEST)?;
        Some((Vec::new(), Some(Box::new(expr))))
    }
}

fn parse_block_body(state: &mut ParserState<'_>) -> Option<(Vec<Stmt>, Option<Box<Expr>>)> {
    let mut stmts = Vec::new();
    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(stmt) = state.parse_statement() {
            stmts.push(stmt);
        } else {
            state.synchronize();
        }
    }
    let expr = if !state.at(&TokenKind::RBrace) {
        state.parse_expression(crate::frontend::core::parser::BP_LOWEST)
    } else {
        None
    };
    Some((stmts, expr.map(Box::new)))
}
