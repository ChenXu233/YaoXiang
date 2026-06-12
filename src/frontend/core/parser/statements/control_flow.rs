//! Control flow statement parsing
//! Handles if/else, loops, match, and return statements

use crate::frontend::core::lexer::tokens::*;
use crate::frontend::core::parser::ast::*;
use crate::util::span::Span;
use crate::util::diagnostic::ErrorCodeDefinition;

/// Parse return statement: `return [expr];`
pub fn parse_return_stmt(
    state: &mut crate::frontend::core::parser::ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    state.bump(); // consume 'return'

    let value = if state.at(&TokenKind::Semicolon) || state.at(&TokenKind::RBrace) || state.at_end()
    {
        None
    } else {
        Some(Box::new(state.parse_expression(
            crate::frontend::core::parser::BP_LOWEST,
        )?))
    };

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Return(value, span))),
        span,
    })
}

/// Parse break statement: `break;` or `break label;`
pub fn parse_break_stmt(
    state: &mut crate::frontend::core::parser::ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    state.bump(); // consume 'break'

    let label = if state.at(&TokenKind::ColonColon) {
        Some(parse_loop_label(state)?)
    } else {
        None
    };

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Break(label, span))),
        span,
    })
}

/// Parse continue statement: `continue;` or `continue label;`
pub fn parse_continue_stmt(
    state: &mut crate::frontend::core::parser::ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    state.bump(); // consume 'continue'

    let label = if state.at(&TokenKind::ColonColon) {
        Some(parse_loop_label(state)?)
    } else {
        None
    };

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Continue(label, span))),
        span,
    })
}

/// Parse loop label (for break/continue)
fn parse_loop_label(state: &mut crate::frontend::core::parser::ParserState<'_>) -> Option<String> {
    state.bump(); // consume '::'

    match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(name)) => {
            let name = name.clone();
            state.bump();
            Some(name)
        }
        _ => None,
    }
}

/// Parse for loop statement: `for [mut] item in iterable { body }`
pub fn parse_for_stmt(
    state: &mut crate::frontend::core::parser::ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    state.bump(); // consume 'for'

    // Check for 'mut' keyword after 'for'
    let var_mut = state.skip(&TokenKind::KwMut);

    // Parse loop variable and record its span
    let var_span = state.span();
    let var = match state.current().map(|t| &t.kind) {
        Some(TokenKind::Identifier(n)) => n.clone(),
        _ => {
            state.error(
                ErrorCodeDefinition::unexpected_token(&format!(
                    "{:?}",
                    state
                        .current()
                        .map(|t| t.kind.clone())
                        .unwrap_or(TokenKind::Eof)
                ))
                .at(state.span())
                .build(),
            );
            return None;
        }
    };
    state.bump();

    // Expect 'in' keyword
    if !state.expect(&TokenKind::KwIn) {
        return None;
    }

    // Parse iterable expression
    let iterable = Box::new(state.parse_expression(crate::frontend::core::parser::BP_LOWEST)?);

    // Parse body
    let body = if state.at(&TokenKind::LBrace) {
        parse_block_expression(state)?
    } else {
        let expr = state.parse_expression(crate::frontend::core::parser::BP_LOWEST)?;
        let span = state.span();
        Block {
            stmts: vec![Stmt {
                kind: StmtKind::Expr(Box::new(expr)),
                span,
            }],
            span,
        }
    };

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::For {
            var,
            var_span,
            var_mut,
            iterable,
            body: Box::new(body),
            label: None,
        },
        span,
    })
}

/// Parse if statement: `if condition { then_branch } elif branches else_branch`
pub fn parse_if_stmt(
    state: &mut crate::frontend::core::parser::ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    state.bump(); // consume 'if'

    let condition = state.parse_expression(crate::frontend::core::parser::BP_LOWEST)?;

    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    let then_branch = parse_block_after_lbrace(state, span)?;

    // Parse elif branches
    let mut elif_branches = Vec::new();
    while state.skip(&TokenKind::KwElif) {
        let elif_condition = state.parse_expression(crate::frontend::core::parser::BP_LOWEST)?;
        if !state.expect(&TokenKind::LBrace) {
            return None;
        }
        let elif_body = parse_block_after_lbrace(state, state.span())?;
        elif_branches.push((Box::new(elif_condition), Box::new(elif_body)));
    }

    // Parse else branch
    let else_branch = if state.skip(&TokenKind::KwElse) {
        if state.at(&TokenKind::LBrace) {
            state.bump(); // consume '{'
            Some(Box::new(parse_block_after_lbrace(state, state.span())?))
        } else {
            let expr = state.parse_expression(crate::frontend::core::parser::BP_LOWEST)?;
            let span = state.span();
            Some(Box::new(Block {
                stmts: vec![Stmt {
                    kind: StmtKind::Expr(Box::new(expr)),
                    span,
                }],
                span,
            }))
        }
    } else {
        None
    };

    Some(Stmt {
        kind: StmtKind::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            elif_branches,
            else_branch,
            span,
        },
        span,
    })
}
/// Parse while loop statement: `while condition { body }`
pub fn parse_while_stmt(
    state: &mut crate::frontend::core::parser::ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    state.bump(); // consume 'while'

    // Parse condition expression
    let condition = Box::new(state.parse_expression(crate::frontend::core::parser::BP_LOWEST)?);

    // Parse body block
    let body = if state.at(&TokenKind::LBrace) {
        parse_block_expression(state)?
    } else {
        let expr = state.parse_expression(crate::frontend::core::parser::BP_LOWEST)?;
        let span = state.span();
        Block {
            stmts: vec![Stmt {
                kind: StmtKind::Expr(Box::new(expr)),
                span,
            }],
            span,
        }
    };

    state.skip(&TokenKind::Semicolon);

    Some(Stmt {
        kind: StmtKind::Expr(Box::new(Expr::While {
            condition,
            body: Box::new(body),
            label: None,
            span,
        })),
        span,
    })
}
/// Parse block statement: `{ ... }`
pub fn parse_block_stmt(
    state: &mut crate::frontend::core::parser::ParserState<'_>,
    span: Span,
) -> Option<Stmt> {
    let block = parse_block_expression(state)?;
    Some(Stmt {
        kind: StmtKind::Expr(Box::new(Expr::Block(block))),
        span,
    })
}

/// Parse block expression (returns Block)
pub fn parse_block_expression(
    state: &mut crate::frontend::core::parser::ParserState<'_>
) -> Option<Block> {
    let block_start = state.span();
    if !state.expect(&TokenKind::LBrace) {
        return None;
    }

    let stmts = parse_block_body(state)?;

    if !state.expect(&TokenKind::RBrace) {
        return None;
    }

    Some(Block {
        stmts,
        span: block_start,
    })
}

/// Parse block body (statements only)
pub fn parse_block_body(
    state: &mut crate::frontend::core::parser::ParserState<'_>
) -> Option<Vec<Stmt>> {
    let mut stmts = Vec::new();

    while !state.at(&TokenKind::RBrace) && !state.at_end() {
        if let Some(stmt) = state.parse_statement() {
            stmts.push(stmt);
        } else {
            state.synchronize();
        }
    }

    Some(stmts)
}

/// Parse block after LBrace has been consumed (expects RBrace at end)
fn parse_block_after_lbrace(
    state: &mut crate::frontend::core::parser::ParserState<'_>,
    block_start: Span,
) -> Option<Block> {
    let stmts = parse_block_body(state)?;

    if !state.expect(&TokenKind::RBrace) {
        return None;
    }

    Some(Block {
        stmts,
        span: block_start,
    })
}
