//! Parser module
//! Implements a Pratt Parser for the YaoXiang language with RFC-004/010/011 support.

pub mod ast;
pub mod parser_state;
pub mod pratt;
pub mod statements;
#[cfg(test)]
pub mod tests;

pub use parser_state::{ParserState, parse_msg};
pub use statements::StatementParser;
pub use pratt::*;
pub use ast::*;
pub use crate::frontend::core::lexer::tokens::*;
pub use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
pub use crate::util::span::Span;

#[derive(Debug, Clone)]
pub struct ParseResult {
    pub module: Module,
    pub errors: Vec<Diagnostic>,
    pub has_errors: bool,
}

fn find_first(state: &ParserState<'_>) -> Option<Diagnostic> {
    state.errors().first().cloned()
}

pub fn parse(tokens: &[Token]) -> ParseResult {
    let mut state = ParserState::new(tokens);
    let mut items = Vec::new();

    while !state.at_end() {
        if !state.can_start_stmt() {
            if state.at(&TokenKind::Semicolon) {
                state.bump();
                continue;
            }
            let esp = state.span();
            let found = state
                .current()
                .map(|t| t.kind.clone())
                .unwrap_or(TokenKind::Eof);
            state.error(
                ErrorCodeDefinition::unexpected_token(&format!("{:?}", found))
                    .at(esp)
                    .build(),
            );
            items.push(Stmt {
                kind: StmtKind::Error(esp),
                span: esp,
            });
            state.bump();
            continue;
        }
        if let Some(stmt) = state.parse_statement() {
            items.push(stmt);
        } else {
            state.bump();
        }
    }

    let has_errors = state.has_errors();
    let errors = state.take_errors();
    let span = if let (Some(f), Some(l)) = (items.first(), items.last()) {
        Span::new(f.span.start, l.span.end)
    } else {
        Span::dummy()
    };

    ParseResult {
        module: Module { items, span },
        errors,
        has_errors,
    }
}

pub fn parse_expression(tokens: &[Token]) -> Result<Expr, Diagnostic> {
    let mut state = ParserState::new(tokens);
    match state.parse_expression(BP_LOWEST) {
        Some(e) if !state.has_errors() => Ok(e),
        Some(_) | None => Err(find_first(&state).unwrap_or_else(|| {
            let found = state
                .current()
                .map(|t| t.kind.clone())
                .unwrap_or(TokenKind::Eof);
            ErrorCodeDefinition::unexpected_token(&format!("{:?}", found))
                .at(state.span())
                .build()
        })),
    }
}

/// 检查解析错误，如果有错误返回所有错误信息
pub fn check_parse_errors(parse_result: &ParseResult) -> anyhow::Result<()> {
    if parse_result.has_errors {
        let messages: Vec<String> = parse_result
            .errors
            .iter()
            .map(|e| format!("{}", e))
            .collect();
        return Err(anyhow::anyhow!("Parse errors:\n{}", messages.join("\n")));
    }
    Ok(())
}
