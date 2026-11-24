//! 8.3 最小 LALRPOP go/no-go 探针——RFC-038 与 RFC-039 D24/D26。
//!
//! 只经测试模块引入生成 parser；生产 parse()、Pratt 与 lexer 均不改。

use crate::ast::{
    BinOp, Block, Expr, FStringSegment, MatchArm, Param, Pattern, PlanId, Stmt, StmtKind,
};
use crate::frontend::core::lexer::tokens::{FStringToken, FStringTokenSegment, Token, TokenKind};
use crate::frontend::core::lexer::{lex_interpolations, tokenize};
use crate::frontend::core::parser::parse;
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use crate::util::span::{Position, Span};
use lalrpop_util::{ErrorRecovery, ParseError};

// reason: LALRPOP 生成的 parser 将 errors 参数固定为 &mut Vec，生成物不可手改。
lalrpop_util::lalrpop_mod!(
    #[allow(clippy::ptr_arg)]
    pub probe,
    "/frontend/core/parser/grammar/probe.rs"
);

#[derive(Clone, Debug)]
pub enum ProbeToken {
    Ident(String),
    Int(i128),
    Text(String),
    KwMain,
    KwIf,
    Eq,
    Plus,
    LParen,
    NewlineLParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Newline,
    Dot,
    NewlineDot,
    FatArrow,
    Pipe,
    FStringBegin,
    FStringEnd,
    InterpolationBegin,
    InterpolationEnd,
}

type Lexeme = Result<(Position, ProbeToken, Position), String>;
type Recovery = ErrorRecovery<Position, ProbeToken, String>;

enum PostfixSuffix {
    Call { args: Vec<Expr>, span: Span },
    Field { name: String, span: Span },
}

fn apply_suffix(
    base: Expr,
    suffixes: Vec<PostfixSuffix>,
) -> Expr {
    suffixes
        .into_iter()
        .fold(base, |expr, suffix| match suffix {
            PostfixSuffix::Call { args, span } => {
                let full_span = Span::new(expr.span().start, span.end);
                Expr::call(expr, args, vec![], full_span, PlanId::UNPLANNED)
            }
            PostfixSuffix::Field { name, span } => Expr::FieldAccess {
                expr: Box::new(expr),
                field: name,
                span,
            },
        })
}

fn parenthesized(
    name: String,
    param_span: Span,
    tail: Option<(Span, Expr)>,
) -> Expr {
    let Some((arrow_span, body)) = tail else {
        return Expr::Var(name, param_span);
    };
    let body_span = body.span();
    let param = Param {
        name,
        ty: None,
        is_mut: false,
        quantity: None,
        span: param_span,
    };
    let return_stmt = Stmt {
        kind: StmtKind::Return(Some(Box::new(body))),
        span: body_span,
        plan_id: PlanId::UNPLANNED,
    };
    Expr::Lambda {
        params: vec![param],
        body: Box::new(Block {
            stmts: vec![return_stmt],
            span: body_span,
        }),
        span: arrow_span,
    }
}

fn adapt_fstring(
    fstring: &FStringToken,
    span: Span,
    out: &mut Vec<Lexeme>,
) {
    out.push(Ok((span.start, ProbeToken::FStringBegin, span.start)));
    let sources: Vec<&str> = fstring
        .segments
        .iter()
        .filter_map(|segment| match segment {
            FStringTokenSegment::Text(_) => None,
            FStringTokenSegment::Interpolation { expr, .. } => Some(expr.as_str()),
        })
        .collect();
    let mut interpolated = lex_interpolations(&sources).into_iter();
    for segment in &fstring.segments {
        match segment {
            FStringTokenSegment::Text(text) => {
                out.push(Ok((span.start, ProbeToken::Text(text.clone()), span.end)));
            }
            FStringTokenSegment::Interpolation { .. } => {
                out.push(Ok((span.start, ProbeToken::InterpolationBegin, span.start)));
                let tokens = interpolated
                    .next()
                    .expect("插值数量必须与词法段一致")
                    .unwrap_or_else(|e| panic!("插值词法失败: {e:?}"));
                out.extend(adapt_tokens(&tokens));
                out.push(Ok((span.end, ProbeToken::InterpolationEnd, span.end)));
            }
        }
    }
    out.push(Ok((span.end, ProbeToken::FStringEnd, span.end)));
}

fn adapt_tokens(tokens: &[Token]) -> Vec<Lexeme> {
    let mut out = Vec::new();
    let mut previous_end_line = None;
    for token in tokens {
        if matches!(token.kind, TokenKind::Eof) {
            continue;
        }
        let span = token.span;
        let newline = previous_end_line.is_some_and(|line| line < span.start.line);
        if newline && !matches!(token.kind, TokenKind::Dot) {
            out.push(Ok((span.start, ProbeToken::Newline, span.start)));
        }
        if let TokenKind::FStringLiteral(fstring) = &token.kind {
            adapt_fstring(fstring, span, &mut out);
        } else {
            let kind = match &token.kind {
                TokenKind::Identifier(name) if name == "main" => ProbeToken::KwMain,
                TokenKind::Identifier(name) => ProbeToken::Ident(name.clone()),
                TokenKind::IntLiteral(value) => ProbeToken::Int(*value),
                TokenKind::KwIf => ProbeToken::KwIf,
                TokenKind::Eq => ProbeToken::Eq,
                TokenKind::Plus => ProbeToken::Plus,
                TokenKind::LParen if newline => ProbeToken::NewlineLParen,
                TokenKind::LParen => ProbeToken::LParen,
                TokenKind::RParen => ProbeToken::RParen,
                TokenKind::LBrace => ProbeToken::LBrace,
                TokenKind::RBrace => ProbeToken::RBrace,
                TokenKind::Comma => ProbeToken::Comma,
                TokenKind::Semicolon => ProbeToken::Semicolon,
                TokenKind::Dot if newline => ProbeToken::NewlineDot,
                TokenKind::Dot => ProbeToken::Dot,
                TokenKind::FatArrow => ProbeToken::FatArrow,
                TokenKind::Pipe => ProbeToken::Pipe,
                other => panic!("探针尚未覆盖 token: {other:?}"),
            };
            out.push(Ok((span.start, kind, span.end)));
        }
        previous_end_line = Some(span.end.line);
    }
    out
}

fn diagnostic_from_error(error: &ParseError<Position, ProbeToken, String>) -> Diagnostic {
    match error {
        ParseError::UnrecognizedToken {
            token: (start, ProbeToken::Semicolon, end),
            ..
        } => ErrorCodeDefinition::invalid_syntax("Expected expression after '='")
            .at(Span::new(*start, *end))
            .build(),
        ParseError::UnrecognizedToken {
            token: (start, token, end),
            ..
        }
        | ParseError::ExtraToken {
            token: (start, token, end),
        } => ErrorCodeDefinition::unexpected_token(&format!("{token:?}"))
            .at(Span::new(*start, *end))
            .build(),
        ParseError::InvalidToken { location } | ParseError::UnrecognizedEof { location, .. } => {
            ErrorCodeDefinition::unexpected_token("Eof")
                .at(Span::new(*location, *location))
                .build()
        }
        ParseError::User { error } => ErrorCodeDefinition::invalid_syntax(error)
            .at(Span::dummy())
            .build(),
    }
}

fn diagnostics(recoveries: &[Recovery]) -> Vec<Diagnostic> {
    recoveries
        .iter()
        .map(|recovery| diagnostic_from_error(&recovery.error))
        .collect()
}

fn new_file(source: &str) -> (Vec<StmtKind>, Vec<Diagnostic>) {
    let tokens = tokenize(source).expect("A 探针必须词法通过");
    let input = adapt_tokens(&tokens);
    let mut recoveries = Vec::new();
    let statements = probe::FileParser::new()
        .parse(&mut recoveries, input)
        .expect("LALRPOP File 必须解析成功");
    (statements, diagnostics(&recoveries))
}

fn new_block(source: &str) -> (Vec<StmtKind>, Vec<Diagnostic>) {
    let tokens = tokenize(source).expect("B 探针必须词法通过");
    let input = adapt_tokens(&tokens);
    let mut recoveries = Vec::new();
    let statements = probe::BlockParser::new()
        .parse(&mut recoveries, input)
        .expect("LALRPOP Block 必须完成错误恢复");
    (statements, diagnostics(&recoveries))
}

fn new_expr(source: &str) -> Expr {
    let tokens = tokenize(source).expect("表达式探针必须词法通过");
    let input = adapt_tokens(&tokens);
    let mut recoveries = Vec::new();
    let expr = probe::ExprParser::new()
        .parse(&mut recoveries, input)
        .expect("LALRPOP Expr 必须解析成功");
    assert!(recoveries.is_empty(), "合法表达式不得恢复: {recoveries:?}");
    expr
}

fn new_match_arm(source: &str) -> MatchArm {
    let tokens = tokenize(source).expect("模式探针必须词法通过");
    let input = adapt_tokens(&tokens);
    let mut recoveries = Vec::new();
    let arm = probe::MatchArmParser::new()
        .parse(&mut recoveries, input)
        .expect("LALRPOP MatchArm 必须解析成功");
    assert!(recoveries.is_empty(), "合法模式不得恢复: {recoveries:?}");
    arm
}

fn new_fstring(source: &str) -> Expr {
    let tokens = tokenize(source).expect("f-string 探针必须词法通过");
    let input = adapt_tokens(&tokens);
    let mut recoveries = Vec::new();
    let expr = probe::FStringExprParser::new()
        .parse(&mut recoveries, input)
        .expect("LALRPOP FStringExpr 必须解析成功");
    assert!(
        recoveries.is_empty(),
        "合法 f-string 不得恢复: {recoveries:?}"
    );
    expr
}

fn current_stmt_kinds(source: &str) -> Vec<StmtKind> {
    let tokens = tokenize(source).expect("现树对照必须词法通过");
    parse(&tokens)
        .module
        .items
        .into_iter()
        .map(|stmt| stmt.kind)
        .collect()
}

fn diagnostic_keys(
    errors: &[Diagnostic]
) -> Vec<(String, &'static str, usize, usize, usize, usize)> {
    errors
        .iter()
        .map(|error| {
            let span = error.span.expect("对照诊断必须有 span");
            (
                error.code.clone(),
                "grammar_probe.yx",
                span.start.line,
                span.start.column,
                span.end.line,
                span.end.column,
            )
        })
        .collect()
}

#[test]
fn test_grammar_probe_newline_dot_matches_current() {
    // Arrange：RFC-038 例外 3——行首 . 允许链式续行。
    let source = "a
.b";
    let current = current_stmt_kinds(source);

    // Act
    let (generated, errors) = new_file(source);

    // Assert：AST 变体与字段 token span 都逐项相同。
    assert!(errors.is_empty(), "续行不得产生恢复诊断: {errors:?}");
    assert_eq!(generated.len(), current.len(), "语句数必须与现树一致");
    for (actual, expected) in generated.iter().zip(current.iter()) {
        assert_eq!(
            format!("{actual:#?}"),
            format!("{expected:#?}"),
            "a\n.b 的 AST 与 span 必须与现树一致"
        );
    }
    match &generated[0] {
        StmtKind::Expr(expr) => match expr.as_ref() {
            Expr::FieldAccess { field, span, .. } => {
                assert_eq!(field, "b", "字段必须来自第二行 token");
                assert_eq!(
                    (
                        span.start.line,
                        span.start.column,
                        span.end.line,
                        span.end.column
                    ),
                    (2, 2, 2, 3),
                    "FieldAccess span 必须精确覆盖第二行 b"
                );
            }
            other => panic!("续行必须产 FieldAccess，实际 {other:?}"),
        },
        other => panic!("续行必须是表达式语句，实际 {other:?}"),
    }
}

#[test]
fn test_grammar_probe_newline_lparen_starts_new_statement() {
    // Arrange：RFC-038 永不合并——行首 ( 恒为新语句。
    let source = "a
(b)";
    let current = current_stmt_kinds(source);

    // Act
    let (generated, errors) = new_file(source);

    // Assert
    assert!(errors.is_empty(), "断句不得产生恢复诊断: {errors:?}");
    assert_eq!(generated.len(), 2, "a\n(b) 必须产两条语句");
    assert_eq!(generated.len(), current.len(), "语句数必须与现树一致");
    for (actual, expected) in generated.iter().zip(current.iter()) {
        assert_eq!(
            format!("{actual:#?}"),
            format!("{expected:#?}"),
            "a\n(b) 的两条 AST 与 span 必须与现树一致"
        );
    }
}

#[test]
fn test_grammar_probe_same_line_lparen_remains_call() {
    // Arrange：同行 ( 仍是调用后缀，作为断句对照。
    let source = "a(b)";
    let current = current_stmt_kinds(source);

    // Act
    let (generated, errors) = new_file(source);

    // Assert
    assert!(errors.is_empty(), "同行调用不得产生恢复诊断: {errors:?}");
    assert_eq!(generated.len(), 1, "a(b) 必须只产一条调用语句");
    for statement in generated.iter().chain(current.iter()) {
        assert!(
            matches!(
                statement,
                StmtKind::Expr(expr) if matches!(
                    expr.as_ref(),
                    Expr::Call { func, args, .. }
                        if matches!(func.as_ref(), Expr::Var(name, _) if name == "a")
                            && matches!(args.as_slice(), [Expr::Var(name, _)] if name == "b")
                )
            ),
            "a(b) 必须保留 Call(a, [b]) 形状: {statement:#?}"
        );
    }
}

#[test]
fn test_grammar_probe_error_recovery_code_span_matches_current() {
    // Arrange：现树仅报 E0012 @1:18-19，并恢复 b = 2。
    let source = "main = { a = 1 + ; b = 2 }";
    let tokens = tokenize(source).expect("恢复对照必须词法通过");
    let current = parse(&tokens);

    // Act
    let (generated, errors) = new_block(source);

    // Assert：code + file/line/col（含 end）逐条一致，后续 b 不丢。
    assert_eq!(
        diagnostic_keys(&errors),
        diagnostic_keys(&current.errors),
        "LALRPOP 与 synchronize 的诊断 code + span 必须逐条一致"
    );
    assert_eq!(errors.len(), 1, "最小反例必须恰一条诊断");
    assert!(
        generated.iter().any(|stmt| matches!(
            stmt,
            StmtKind::Assign { target, .. } if matches!(target.as_ref(), Expr::Var(name, _) if name == "b")
        )),
        "错误恢复后必须保留 b = 2: {generated:#?}"
    );
}

#[test]
fn test_grammar_probe_lambda_context_needs_no_runtime_switch() {
    // Arrange / Act：(x) => x + 1 直接走 LambdaExpr 产生式。
    let expr = new_expr("(x) => x + 1");

    // Assert
    let Expr::Lambda { params, body, .. } = expr else {
        panic!("必须产 Lambda，实际 {expr:?}");
    };
    assert_eq!(params.len(), 1, "必须恰一个参数");
    assert_eq!(params[0].name, "x", "参数名必须为 x");
    assert_eq!(body.stmts.len(), 1, "表达式体必须合成一条 return");
    assert!(
        matches!(
            &body.stmts[0].kind,
            StmtKind::Return(Some(value)) if matches!(
                value.as_ref(),
                Expr::BinOp { op: BinOp::Add, .. }
            )
        ),
        "lambda 体必须是 x + 1: {body:#?}"
    );
}

#[test]
fn test_grammar_probe_variant_pattern_context_needs_no_runtime_switch() {
    // Arrange / Act：ok(v) 直接走 PatternAtom 的 Union 产生式。
    let arm = new_match_arm("ok(v) => 1");

    // Assert
    match arm.pattern {
        Pattern::Union {
            name,
            variant,
            pattern,
        } => {
            assert_eq!(name, "ok", "构造器名必须为 ok");
            assert_eq!(variant, "ok", "变体名必须为 ok");
            match pattern.as_deref() {
                Some(Pattern::Identifier(name)) => {
                    assert_eq!(name, "v", "载荷必须是绑定 v");
                }
                other => panic!("载荷必须是绑定 v，实际 {other:?}"),
            }
        }
        other => panic!("ok(v) 必须产 Union 模式，实际 {other:?}"),
    }
}

#[test]
fn test_grammar_probe_or_pattern_context_needs_no_runtime_switch() {
    // Arrange / Act：1 | 2 直接走 Pattern 的 Or 产生式。
    let arm = new_match_arm("1 | 2 => 0");

    // Assert
    match arm.pattern {
        Pattern::Or(patterns) => {
            assert_eq!(patterns.len(), 2, "或模式必须恰好两项");
            assert!(
                matches!(patterns[0], Pattern::Literal(crate::ast::Literal::Int(1))),
                "首项必须是字面量 1"
            );
            assert!(
                matches!(patterns[1], Pattern::Literal(crate::ast::Literal::Int(2))),
                "次项必须是字面量 2"
            );
        }
        other => panic!("1 | 2 必须产 Or 模式，实际 {other:?}"),
    }
}

#[test]
fn test_grammar_probe_fstring_interpolation_context_needs_no_runtime_switch() {
    // Arrange / Act：插值边界内部复用 Expr，lambda 不需模式开关。
    let expr = new_fstring("f\"lambda {(x) => x + 1}\"");

    // Assert
    let Expr::FString { segments, .. } = expr else {
        panic!("必须产 FString，实际 {expr:?}");
    };
    assert_eq!(segments.len(), 2, "必须保留文本段与插值段");
    assert!(
        matches!(&segments[1], FStringSegment::Interpolation { expr, .. } if matches!(expr.as_ref(), Expr::Lambda { .. })),
        "插值内部必须是 Lambda: {segments:#?}"
    );
}
