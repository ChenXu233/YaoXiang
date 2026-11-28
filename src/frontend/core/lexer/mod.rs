//! Lexer module - refactored for RFC support
//! Split into specialized modules for better maintainability and RFC feature support

pub mod literals;
pub mod state;
pub mod symbols;
pub mod tokenizer;
pub mod tokens;

// Re-export types
pub use tokenizer::Lexer;
pub use tokens::{LexError, Literal, Token, TokenKind};

/// Tokenize source code with RFC support
/// Supports:
/// - RFC-004: Binding syntax (e.g., function[0, 1, 2])
/// - RFC-010: Generic syntax (e.g., List(T), Vec(T: Clone))
/// - RFC-011: Advanced type system features
pub fn tokenize(source: &str) -> Result<Vec<Token>, crate::frontend::core::lexer::LexError> {
    #[cfg(test)]
    record_tokenize_call();

    tokenize_with_logging(source, crate::util::span::Position::with_offset(1, 1, 0))
}

/// 公共词法的带日志实现：f-string 插值的一次性批量词法复用它，保证插值 token
/// 流与逐插值调用公共入口时逐 token 同形（含 tracing 日志）。
///
/// base 是子源文本起点在**源文件**中的绝对位置：整文件词法传文件起点，
/// f-string 插值子词法传插值段携带的 base——子 token 的 Span 以它为原点，
/// 直接落回源文件绝对行列与偏移（#574 / WBS 8.2.3）。
fn tokenize_with_logging(
    source: &str,
    base: crate::util::span::Position,
) -> Result<Vec<Token>, crate::frontend::core::lexer::LexError> {
    use crate::util::i18n::{t_cur, MSG};

    let source_len = source.len();
    tracing::debug!("{}", t_cur(MSG::LexStart, Some(&[&source_len])));

    let mut lexer = Lexer::with_base(source, base);
    let mut tokens = Vec::new();

    while let Some(token) = lexer.next_token() {
        // Log each token (except EOF which is added later)
        if !matches!(token.kind, TokenKind::Eof) {
            log_token(&token);
        }
        tokens.push(token);
    }

    if let Some(err) = lexer.error {
        Err(err)
    } else {
        tokens.push(Token {
            kind: TokenKind::Eof,
            span: crate::util::span::Span::new(
                lexer.position(),
                lexer.position(), // Use current position for EOF
            ),
            literal: None,
        });
        let token_count = tokens.len();
        tracing::debug!(
            "{}",
            t_cur(MSG::LexCompleteWithTokens, Some(&[&token_count]))
        );
        Ok(tokens)
    }
}

/// 一次性词法全部 f-string 插值表达式（05 §「f-string 嵌套编译必须显式处理」）。
///
/// parser 在首次需要某个 f-string 节点时用一次调用取回它的全部插值 token 流，
/// 结果顺序与 sources 一一对应（每项与 tokenize 同形：含 Eof，词法错误为 Err）。
/// 单点实现：插值词法只发生在这里，parser 不再逐插值重入公共词法入口。
///
/// 每项携带该插值 expr 起始的**源文件绝对位置**（`FStringTokenSegment` 的
/// base，扫描期锚点记录，#574）：子词法以它为词法起点，插值内 token 的
/// Span 直接是源文件绝对行列——不再从插值文本 0/1/1 起扫。嵌套 f-string
/// 天然可组合：内层插值的 base 取自外层子词法已含 base 的 position。
pub fn lex_interpolations(
    sources: &[(&str, crate::util::span::Position)]
) -> Vec<Result<Vec<Token>, crate::frontend::core::lexer::LexError>> {
    #[cfg(test)]
    record_interpolation_batch_call();

    sources
        .iter()
        .map(|(source, base)| tokenize_with_logging(source, *base))
        .collect()
}

// 测试计数器用 thread_local：libtest 每个 #[test] 跑在独立线程上，其它用例的
// tokenize 调用不会窜进本测试的 reset 窗口（进程级 static 在 --test-threads>1
// 时会间歇假红）。断言与 reset 语义不变。
#[cfg(test)]
std::thread_local! {
    static TOKENIZE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static INTERPOLATION_BATCH_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_tokenize_call() {
    TOKENIZE_CALLS.with(|calls| calls.set(calls.get() + 1));
}

#[cfg(test)]
fn record_interpolation_batch_call() {
    INTERPOLATION_BATCH_CALLS.with(|calls| calls.set(calls.get() + 1));
}

/// 测试用：公共词法入口被调用次数（entry-call 特征化：检查 parser 不再逐插值
/// 重入公共入口；完整词法分析次数仍为 N+1，单遍目标见 WBS 8.2.4）。
#[cfg(test)]
pub(crate) fn tokenize_call_count() -> usize {
    TOKENIZE_CALLS.with(std::cell::Cell::get)
}

/// 测试用：一次性插值批量词法（lex_interpolations）被调用次数（entry-call
/// 特征化：每个 f-string 只允许一次批量入口调用；内部仍逐插值跑 Lexer，
/// 完整词法分析仍 N+1，见 WBS 8.2.4）。
#[cfg(test)]
pub(crate) fn interpolation_batch_call_count() -> usize {
    INTERPOLATION_BATCH_CALLS.with(std::cell::Cell::get)
}

/// 测试用：清零本线程的两个词法调用计数（thread_local，天然不影响其它用例）。
#[cfg(test)]
pub(crate) fn reset_lexing_call_counts() {
    TOKENIZE_CALLS.with(|calls| calls.set(0));
    INTERPOLATION_BATCH_CALLS.with(|calls| calls.set(0));
}

/// Log a token for debugging
fn log_token(token: &Token) {
    use crate::util::i18n::{t_cur, MSG};

    let (msg, arg) = match &token.kind {
        TokenKind::Identifier(name) => (MSG::LexTokenIdentifier, name.clone()),
        TokenKind::KwUse
        | TokenKind::KwSpawn
        | TokenKind::KwRef
        | TokenKind::KwMut
        | TokenKind::KwIf
        | TokenKind::KwElse
        | TokenKind::KwMatch
        | TokenKind::KwWhile
        | TokenKind::KwFor
        | TokenKind::KwIn
        | TokenKind::KwReturn
        | TokenKind::KwBreak
        | TokenKind::KwContinue
        | TokenKind::KwAs => (MSG::LexTokenKeyword, format!("{:?}", token.kind)),
        TokenKind::IntLiteral(n) => (MSG::LexTokenNumber, n.to_string()),
        TokenKind::FloatLiteral(f) => (MSG::LexTokenNumber, f.to_string()),
        TokenKind::StringLiteral(s) => (MSG::LexTokenString, s.clone()),
        TokenKind::FStringLiteral(s) => (MSG::LexTokenString, format!("f\"{}\"", s.raw)),
        TokenKind::CharLiteral(c) => (MSG::LexTokenChar, c.to_string()),
        TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash
        | TokenKind::Percent
        | TokenKind::Arrow
        | TokenKind::FatArrow
        | TokenKind::EqEq
        | TokenKind::Neq
        | TokenKind::Le
        | TokenKind::Lt
        | TokenKind::Ge
        | TokenKind::Gt
        | TokenKind::And
        | TokenKind::Or
        | TokenKind::Not
        | TokenKind::ColonColon => (MSG::LexTokenOperator, format!("{:?}", token.kind)),
        _ => (MSG::LexTokenPunctuation, format!("{:?}", token.kind)),
    };
    tracing::debug!("{}", t_cur(msg, Some(&[&arg])));
}

#[cfg(test)]
mod tests;
