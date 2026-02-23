//! Parser module
//!
//! Implements a Pratt Parser for the YaoXiang language with RFC-004/010/011 support.
//! This module provides the main entry points for parsing tokens into AST.

pub mod ast;
pub mod parser_state;
pub mod pratt;
pub mod statements;
#[cfg(test)]
pub mod tests;

// Re-export commonly used items
pub use parser_state::{ParserState, ParseError};
pub use statements::StatementParser;
pub use pratt::*;
pub use ast::*;

// Re-export lexer tokens
pub use crate::frontend::core::lexer::tokens::*;
pub use crate::util::span::Span;

/// 解析结果（含错误恢复信息）
///
/// 用于 LSP 等场景：即使存在语法错误，也返回尽可能完整的 AST。
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// 解析得到的模块（可能包含 Error 占位节点）
    pub module: Module,
    /// 解析过程中收集的所有错误
    pub errors: Vec<ParseError>,
    /// 是否存在错误
    pub has_errors: bool,
}

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
        // Skip empty statements (like stray semicolons)
        if !state.can_start_stmt() {
            if state.at(&TokenKind::Semicolon) {
                state.bump();
                continue;
            }

            state.error(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.span(),
            });
            state.bump();
            continue;
        }

        if let Some(stmt) = state.parse_statement() {
            items.push(stmt);
        } else {
            state.bump(); // Skip problematic tokens
        }
    }

    if state.has_errors() {
        if let Some(error) = state.first_error().cloned() {
            Err(error)
        } else {
            Err(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: state.span(),
            })
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

/// 带错误恢复的解析
///
/// 与 `parse` 不同，此函数总是返回一个 `ParseResult`，
/// 即使存在语法错误也会返回尽可能完整的 AST。
/// 解析失败的位置会插入 `Expr::Error` 或 `StmtKind::Error` 占位节点。
///
/// # 用途
///
/// 主要用于 LSP 等 IDE 集成场景，需要在代码不完整时仍然提供诊断信息。
///
/// # 参数
/// * `tokens` - 词法分析器生成的 token 流
///
/// # 返回
/// `ParseResult` 包含解析后的模块和所有错误
pub fn parse_with_recovery(tokens: &[Token]) -> ParseResult {
    let mut state = ParserState::new(tokens);
    let mut items = Vec::new();

    while !state.at_end() {
        // Skip empty statements (like stray semicolons)
        if !state.can_start_stmt() {
            if state.at(&TokenKind::Semicolon) {
                state.bump();
                continue;
            }

            let error_span = state.span();
            state.error(ParseError::UnexpectedToken {
                found: state
                    .current()
                    .map(|t| t.kind.clone())
                    .unwrap_or(TokenKind::Eof),
                span: error_span,
            });

            // 插入错误占位语句
            items.push(Stmt {
                kind: StmtKind::Error(error_span),
                span: error_span,
            });
            state.bump();
            continue;
        }

        if let Some(stmt) = state.parse_statement() {
            items.push(stmt);
        } else {
            // 解析失败时插入错误占位语句
            let error_span = state.span();
            items.push(Stmt {
                kind: StmtKind::Error(error_span),
                span: error_span,
            });
            state.bump(); // Skip problematic tokens
        }
    }

    let has_errors = state.has_errors();
    let errors = state.take_errors();

    let span = if let Some(first) = items.first() {
        if let Some(last) = items.last() {
            Span::new(first.span.start, last.span.end)
        } else {
            Span::dummy()
        }
    } else {
        Span::dummy()
    };

    ParseResult {
        module: Module { items, span },
        errors,
        has_errors,
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
                Err(ParseError::UnexpectedToken {
                    found: state
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof),
                    span: state.span(),
                })
            }
        }
    }
}
