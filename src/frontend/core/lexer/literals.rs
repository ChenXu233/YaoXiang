//! Literal scanning implementations

//! Handles number, string, and character literals

//! Supports RFC-010/011 generic type literal syntax

use crate::util::span::{Position, Span};

use crate::frontend::core::lexer::tokens::*;

/// 错误点 span（当前位置，#324）
fn point(lexer: &super::tokenizer::Lexer<'_>) -> Span {
    let p = lexer.position();

    Span::new(p, p)
}

/// 自 start 起到当前位置的 span（#324）
fn since(
    lexer: &super::tokenizer::Lexer<'_>,
    start: Position,
) -> Span {
    Span::new(start, lexer.position())
}

/// Scan a number literal (decimal, hex, octal, binary)
pub fn scan_number(
    lexer: &mut super::tokenizer::Lexer<'_>,
    first_char: char,
) -> Option<Token> {
    let mut value = String::new();
    value.push(first_char);

    // Detect base prefix
    let radix = if first_char == '0' {
        match lexer.peek() {
            Some(&'x') | Some(&'X') => {
                lexer.advance();
                value.push('x');
                Radix::Hex
            }
            Some(&'o') | Some(&'O') => {
                lexer.advance();
                value.push('o');
                Radix::Octal
            }
            Some(&'b') | Some(&'B') => {
                lexer.advance();
                value.push('b');
                Radix::Binary
            }
            _ => Radix::Decimal,
        }
    } else {
        Radix::Decimal
    };

    scan_radix_number(lexer, radix, value)
}

/// §1.6.1 基数字面量收尾校验：字面量后不得紧跟字母 / 数字 / 下划线。
///
/// 非法邻接（如 `0b102`、`0x1FG`、`123abc`）此前被静默拆成两个字面量——
/// `0b102` 甚至能编译通过并得到错误值 2（拆成 `0b10` + `2` 两个整数 token），
/// 或留给下游报误导性的「未知变量」（E1001）。统一在词法层拒绝：
/// 返回 Some(错误 token) 表示违规，None 表示无邻接。
/// （RFC-039 P1 1.3.1：同一行为不复制 5 处，实现集中于此，调用点各一行。）
fn reject_trailing_alnum(
    lexer: &mut super::tokenizer::Lexer<'_>,
    radix_label: &str,
    value: &str,
) -> Option<Token> {
    if let Some(&c) = lexer.peek() {
        if c.is_ascii_alphanumeric() || c == '_' {
            // E0002 的 `literal` 参数是 i18n 模板 `'{literal}'` 的唯一填充位，
            // 只能承载字面量本身的文本——把「哪个进制 / 哪个字符非法」的说明
            // 混进该参数会让消息出现嵌套引号、并把源码注释漏进全部语言（实测
            // 连 -L en 也是中文）。参数取「已扫描的 value + 触发拒绝的那个字符」：
            // 单字符后缀即用户写下的完整非法字面量（0b12 / 0x1FG / 0o18），
            // 多字符后缀时给到首个非法字符为止（123abc → 123a，E0002 只有一个
            // 参数位，不为此新增参数与六语言模板）。
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                format!("{value}{c}"),
                point(lexer),
            ));
            return Some(
                lexer.make_token(TokenKind::Error(format!("Invalid {radix_label} number"))),
            );
        }
    }
    None
}

/// 一个数制：数字字母表、进制基数与诊断文案的差异都收在这里；
/// 扫描骨架只有一份（scan_radix_number）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Radix {
    Decimal,
    Hex,
    Octal,
    Binary,
}

impl Radix {
    /// 进制基数
    fn base(self) -> u128 {
        match self {
            Radix::Decimal => 10,
            Radix::Hex => 16,
            Radix::Octal => 8,
            Radix::Binary => 2,
        }
    }

    /// 该基数下一个字符的数值；None 表示不是该基数的数字
    fn digit_value(
        self,
        c: char,
    ) -> Option<u128> {
        match self {
            Radix::Decimal => {
                if !is_digit(c) {
                    return None;
                }
                Some(c as u128 - b'0' as u128)
            }
            Radix::Hex => {
                if !is_hex_digit(c) {
                    return None;
                }
                Some(hex_digit_value(c))
            }
            Radix::Octal => {
                if !('0'..='7').contains(&c) {
                    return None;
                }
                Some(c as u128 - b'0' as u128)
            }
            Radix::Binary => {
                if c != '0' && c != '1' {
                    return None;
                }
                Some(c as u128 - b'0' as u128)
            }
        }
    }

    /// 诊断文案里的进制名（小写）
    fn name(self) -> &'static str {
        match self {
            Radix::Decimal => "decimal",
            Radix::Hex => "hex",
            Radix::Octal => "octal",
            Radix::Binary => "binary",
        }
    }

    /// 诊断文案里的进制名（首字母大写）
    fn capitalized(self) -> &'static str {
        match self {
            Radix::Decimal => "Decimal",
            Radix::Hex => "Hex",
            Radix::Octal => "Octal",
            Radix::Binary => "Binary",
        }
    }
}

/// 基数数字扫描的结果：字面量原文本（下划线不进 value）与累积值。
///
/// 溢出后仍继续消费输入以保持 token 流位置，由收尾函数决定怎么报错。
struct RadixScan {
    value: String,
    num_value: u128,
    has_digits: bool,
    overflow: bool,
}

/// 唯一一份基数数字扫描骨架：消费本基数的数字，下划线只作分隔符跳过。
///
/// 累积从调用点之后的第一个字符开始（调用点已消费首字符或进制前缀），
/// 因此十进制首字符只进 value、不进 num_value——与改造前的溢出判定逐位一致。
fn scan_radix_digits(
    lexer: &mut super::tokenizer::Lexer<'_>,
    radix: Radix,
    mut value: String,
) -> RadixScan {
    let mut num_value: u128 = 0;
    let mut has_digits = false;
    let mut overflow = false;

    while let Some(&c) = lexer.peek() {
        if let Some(digit) = radix.digit_value(c) {
            if overflow {
                value.push(c);
                lexer.advance();
            } else {
                match num_value
                    .checked_mul(radix.base())
                    .and_then(|v| v.checked_add(digit))
                {
                    Some(new_val) => {
                        num_value = new_val;
                        value.push(c);
                        lexer.advance();
                        has_digits = true;
                    }
                    None => {
                        overflow = true;
                        value.push(c);
                        lexer.advance();
                    }
                }
            }
        } else if c == '_' {
            lexer.advance();
        } else {
            break;
        }
    }

    RadixScan {
        value,
        num_value,
        has_digits,
        overflow,
    }
}

/// 四基数扫描器的唯一实现：共用数字骨架，按数制收尾。
fn scan_radix_number(
    lexer: &mut super::tokenizer::Lexer<'_>,
    radix: Radix,
    value: String,
) -> Option<Token> {
    let scan = scan_radix_digits(lexer, radix, value);
    if radix == Radix::Decimal {
        scan_decimal_literal(lexer, scan)
    } else {
        scan_integer_literal(lexer, radix, scan)
    }
}

/// 非十进制整数的收尾：邻接拒绝、无数字、溢出、i128 转换。
fn scan_integer_literal(
    lexer: &mut super::tokenizer::Lexer<'_>,
    radix: Radix,
    scan: RadixScan,
) -> Option<Token> {
    if let Some(err) = reject_trailing_alnum(lexer, radix.name(), &scan.value) {
        return Some(err);
    }

    if !scan.has_digits {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
            format!("Expected {} digits", radix.name()),
            point(lexer),
        ));
        return Some(
            lexer.make_token(TokenKind::Error(format!("Invalid {} number", radix.name()))),
        );
    }

    if scan.overflow {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
            scan.value,
            point(lexer),
        ));
        return Some(lexer.make_token(TokenKind::Error(format!(
            "{} number too large",
            radix.capitalized()
        ))));
    }

    match scan.num_value.try_into() {
        Ok(n) => Some(Token {
            kind: TokenKind::IntLiteral(n),
            span: lexer.span(),
            literal: Some(Literal::Int(n)),
        }),
        Err(_) => {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                scan.value,
                point(lexer),
            ));
            Some(lexer.make_token(TokenKind::Error(format!(
                "{} number too large",
                radix.capitalized()
            ))))
        }
    }
}

/// 十进制收尾：整数溢出、小数/指数尾部、邻接拒绝、数值解析。
fn scan_decimal_literal(
    lexer: &mut super::tokenizer::Lexer<'_>,
    scan: RadixScan,
) -> Option<Token> {
    if scan.overflow {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
            scan.value,
            point(lexer),
        ));
        return Some(lexer.make_token(TokenKind::Error("Integer too large".to_string())));
    }

    let mut value = scan.value;

    // Check for decimal point
    if lexer.peek() == Some(&'.') {
        let next = lexer.peek_next();

        // Only proceed if decimal point is followed by digit or underscore+digit
        if next.map(is_digit).unwrap_or(false) || next == Some('_') {
            value.push(lexer.advance().unwrap());

            while let Some(&c) = lexer.peek() {
                if is_digit(c) {
                    value.push(c);
                    lexer.advance();
                } else if c == '_' {
                    // Underscore can only be separator between digits
                    if lexer.peek_next().map(is_digit).unwrap_or(false) {
                        lexer.advance();
                    } else {
                        // Underscore followed by non-digit is an error
                        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                            "Invalid number format: underscore must be between digits".to_string(),
                            point(lexer),
                        ));
                        return Some(
                            lexer.make_token(TokenKind::Error("Invalid number".to_string())),
                        );
                    }
                } else {
                    break;
                }
            }
        }
    }

    // Check for exponent part
    if lexer.peek() == Some(&'e') || lexer.peek() == Some(&'E') {
        value.push(lexer.advance().unwrap());

        if lexer.peek() == Some(&'+') || lexer.peek() == Some(&'-') {
            value.push(lexer.advance().unwrap());
        }

        let mut has_digits = false;

        while let Some(&c) = lexer.peek() {
            if is_digit(c) {
                value.push(c);
                lexer.advance();
                has_digits = true;
            } else if c == '_' {
                // Underscore must be followed by a digit
                if lexer.peek_next().map(is_digit).unwrap_or(false) {
                    lexer.advance();
                } else {
                    // Underscore followed by non-digit is an error
                    lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                        "Invalid number format: underscore must be between digits".to_string(),
                        point(lexer),
                    ));
                    return Some(lexer.make_token(TokenKind::Error("Invalid number".to_string())));
                }
            } else {
                break;
            }
        }

        if !has_digits {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                "Expected digits in exponent".to_string(),
                point(lexer),
            ));
        }
    }

    if let Some(err) = reject_trailing_alnum(lexer, "decimal", &value) {
        return Some(err);
    }

    let num_str: String = value.chars().filter(|&c| c != '_').collect();

    if num_str.contains('.') || num_str.contains('e') || num_str.contains('E') {
        match num_str.parse::<f64>() {
            Ok(n) => Some(Token {
                kind: TokenKind::FloatLiteral(n),
                span: lexer.span(),
                literal: Some(Literal::Float(n)),
            }),
            Err(_) => {
                lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                    value,
                    point(lexer),
                ));
                Some(lexer.make_token(TokenKind::Error("Invalid float".to_string())))
            }
        }
    } else {
        match num_str.parse::<i128>() {
            Ok(n) => Some(Token {
                kind: TokenKind::IntLiteral(n),
                span: lexer.span(),
                literal: Some(Literal::Int(n)),
            }),
            Err(_) => {
                lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                    value,
                    point(lexer),
                ));
                Some(lexer.make_token(TokenKind::Error("Invalid integer".to_string())))
            }
        }
    }
}

/// Scan a float literal that starts with a leading decimal point (e.g., .5)
pub fn scan_leading_dot(lexer: &mut super::tokenizer::Lexer<'_>) -> Option<Token> {
    let start_pos = lexer.position(); // Save start position after '.'

    let mut value = String::from("0.");

    // Collect digits after the decimal point

    while let Some(&c) = lexer.peek() {
        if is_digit(c) {
            value.push(c);

            lexer.advance();
        } else if c == '_' {
            // Underscore must be followed by a digit

            if lexer.peek_next().map(is_digit).unwrap_or(false) {
                lexer.advance();
            } else {
                // Underscore followed by non-digit is an error

                lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                    "Invalid number format: underscore must be between digits".to_string(),
                    point(lexer),
                ));

                return Some(Token {
                    kind: TokenKind::Error("Invalid number".to_string()),

                    span: Span::new(start_pos, lexer.position()),

                    literal: None,
                });
            }
        } else {
            break;
        }
    }

    // Check for exponent part

    if lexer.peek() == Some(&'e') || lexer.peek() == Some(&'E') {
        value.push(lexer.advance().unwrap());

        if lexer.peek() == Some(&'+') || lexer.peek() == Some(&'-') {
            value.push(lexer.advance().unwrap());
        }

        let mut has_digits = false;

        while let Some(&c) = lexer.peek() {
            if is_digit(c) {
                value.push(c);

                lexer.advance();

                has_digits = true;
            } else if c == '_' {
                // Underscore must be followed by a digit

                if lexer.peek_next().map(is_digit).unwrap_or(false) {
                    lexer.advance();
                } else {
                    // Underscore followed by non-digit is an error

                    lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                        "Invalid number format: underscore must be between digits".to_string(),
                        point(lexer),
                    ));

                    return Some(Token {
                        kind: TokenKind::Error("Invalid number".to_string()),

                        span: Span::new(start_pos, lexer.position()),

                        literal: None,
                    });
                }
            } else {
                break;
            }
        }

        if !has_digits {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                "Expected digits in exponent".to_string(),
                point(lexer),
            ));
        }
    }

    if let Some(err) = reject_trailing_alnum(lexer, "float", &value) {
        return Some(err);
    }

    let cleaned: String = value.chars().filter(|&c| c != '_').collect();

    match cleaned.parse::<f64>() {
        Ok(n) => Some(Token {
            kind: TokenKind::FloatLiteral(n),

            span: Span::new(start_pos, lexer.position()),

            literal: Some(Literal::Float(n)),
        }),

        Err(_) => {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
                value,
                point(lexer),
            ));

            Some(Token {
                kind: TokenKind::Error("Invalid float".to_string()),

                span: Span::new(start_pos, lexer.position()),

                literal: None,
            })
        }
    }
}

/// Scan string literal
pub fn scan_string(lexer: &mut super::tokenizer::Lexer<'_>) -> Option<Token> {
    let start_pos = lexer.position();

    // Check for multi-line string (""")

    // NOTE: The first '"' has already been consumed by next_token before calling this

    // So we need to check if the next TWO characters are also '"'

    let c0 = lexer.peek_public().copied();

    let chars_copy = lexer.chars_clone();

    let chars_vec: Vec<char> = chars_copy.collect();

    // Check if we have two more quotes (for a total of three)

    // c0 is the second quote (first was consumed by next_token)

    let has_three_quotes = c0 == Some('"') && chars_vec.get(1) == Some(&'"');

    if has_three_quotes {
        // Skip the remaining two characters of the opening """

        // (The first '"' was already consumed by next_token)

        let _ = lexer.advance(); // Skip second quote

        let _ = lexer.advance(); // Skip third quote

        return scan_triple_quoted(lexer, TripleQuoted::String);
    }

    let mut value = String::new();

    while let Some(&c) = lexer.peek() {
        match c {
            '"' => {
                lexer.advance();

                return Some(Token {
                    kind: TokenKind::StringLiteral(value.clone()),

                    span: Span::new(
                        Position::with_offset(
                            lexer.start_line(),
                            lexer.start_column(),
                            lexer.start_offset(),
                        ),
                        lexer.position(),
                    ),

                    literal: Some(Literal::String(value.clone())),
                });
            }

            '\\' => {
                lexer.advance();
                decode_escape(lexer, &mut value, EscapeMode::Strict);
            }

            '\n' => {
                lexer.error = Some(crate::frontend::core::lexer::LexError::UnterminatedString {
                    position: format!("{}:{}", start_pos.line, start_pos.column),

                    span: since(lexer, start_pos),
                });

                return Some(Token {
                    kind: TokenKind::Error("Unterminated string".to_string()),

                    span: lexer.span(),

                    literal: None,
                });
            }

            c => {
                value.push(c);

                lexer.advance();
            }
        }
    }

    lexer.error = Some(crate::frontend::core::lexer::LexError::UnterminatedString {
        position: format!("{}:{}", start_pos.line, start_pos.column),

        span: since(lexer, start_pos),
    });

    Some(Token {
        kind: TokenKind::Error("Unterminated string".to_string()),

        span: lexer.span(),

        literal: None,
    })
}

/// 三引号串的两种形态：普通多行字符串与多行 f-string。
/// 扫描骨架只有一份（scan_triple_quoted），差异只有转义口径、
/// 成功 token 形态与未终结文案。
#[derive(Clone, Copy)]
enum TripleQuoted {
    String,
    FString,
}

impl TripleQuoted {
    fn escape_mode(self) -> EscapeMode {
        match self {
            TripleQuoted::String => EscapeMode::Lenient,
            TripleQuoted::FString => EscapeMode::Strict,
        }
    }

    fn token(
        self,
        value: &str,
        anchors: &[FStringAnchor],
    ) -> (TokenKind, Option<Literal>) {
        match self {
            TripleQuoted::String => (
                TokenKind::StringLiteral(value.to_string()),
                Some(Literal::String(value.to_string())),
            ),
            TripleQuoted::FString => (
                TokenKind::FStringLiteral(make_fstring_token(value.to_string(), anchors)),
                None,
            ),
        }
    }

    fn unterminated_kind(self) -> &'static str {
        match self {
            TripleQuoted::String => "Unterminated multi-line string",
            TripleQuoted::FString => "Unterminated multi-line f-string",
        }
    }
}

/// 普通多行串与多行 f-string 的唯一扫描骨架。开头的三引号已由调用方消费；
/// 内容原样收集（换行属于内容），遇到未转义的三引号结束。
///
/// f-string 形态下，每个「`{` 且下一字符不是 `{`」的插值开括号记录一条锚点
/// （[`FStringAnchor`]，#574）：`advance` 已消费 `{`，`lexer.position()` 恰是
/// 紧随其后的源文件绝对位置。多行 f-string 的插值内容同样原样收集（花括号
/// 序列保留、无括号深度跟踪），切段端按「`{` 且下一字符不是 `{`」查锚点。
/// **已知边界**：本扫描器对任意 `\` 解码（`decode_escape` 无花括号深度闸），
/// 插值内含转义序列时解码使 expr 缓冲与源文本错位，锚点后的 base 推进在该
/// 子集上不精确——单行形态（`scan_fstring`）无此问题（转义只在文本段解码）；
/// 登记形态（8.2.3 四类）均不含此情形。
fn scan_triple_quoted(
    lexer: &mut super::tokenizer::Lexer<'_>,
    kind: TripleQuoted,
) -> Option<Token> {
    let start_pos = lexer.position();
    let mut value = String::new();
    let mut anchors: Vec<FStringAnchor> = Vec::new();

    while let Some(c) = lexer.advance() {
        if c == '"' {
            let actual_peek1 = lexer.peek_public().copied();
            let actual_peek2 = lexer.chars_clone().nth(1);

            if actual_peek1 == Some('"') && actual_peek2 == Some('"') {
                // Found closing triple quote: clear any previous error and consume it
                lexer.error = None;
                lexer.advance(); // consume second quote
                lexer.advance(); // consume third quote

                let (token_kind, literal) = kind.token(&value, &anchors);
                return Some(Token {
                    kind: token_kind,
                    span: Span::new(
                        Position::with_offset(
                            lexer.start_line(),
                            lexer.start_column(),
                            lexer.start_offset(),
                        ),
                        lexer.position(),
                    ),
                    literal,
                });
            }
            // This is just a single quote inside the string
            value.push(c);
        } else if c == '\\' {
            decode_escape(lexer, &mut value, kind.escape_mode());
        } else {
            if matches!(kind, TripleQuoted::FString)
                && c == '{'
                && lexer.peek_public().copied() != Some('{')
            {
                anchors.push((value.len(), lexer.position()));
            }
            value.push(c);
        }
    }

    lexer.error = Some(crate::frontend::core::lexer::LexError::UnterminatedString {
        position: format!("{}:{}", start_pos.line, start_pos.column),
        span: since(lexer, start_pos),
    });
    Some(lexer.make_token(TokenKind::Error(kind.unterminated_kind().to_string())))
}

/// Scan character literal
pub fn scan_char(lexer: &mut super::tokenizer::Lexer<'_>) -> Option<Token> {
    let start_pos = lexer.position();

    let mut value = String::new();

    while let Some(&c) = lexer.peek() {
        match c {
            '\'' => {
                lexer.advance();

                let ch = match value.chars().next() {
                    Some(c) => c,

                    None => {
                        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidToken {
                            position: format!("{}:{}", start_pos.line, start_pos.column),

                            span: since(lexer, start_pos),

                            message: "Empty character literal".to_string(),
                        });

                        return Some(Token {
                            kind: TokenKind::Error("Empty character literal".to_string()),

                            span: lexer.span(),

                            literal: None,
                        });
                    }
                };

                return Some(Token {
                    kind: TokenKind::CharLiteral(ch),

                    span: Span::new(
                        Position::with_offset(
                            lexer.start_line(),
                            lexer.start_column(),
                            lexer.start_offset(),
                        ),
                        lexer.position(),
                    ),

                    literal: Some(Literal::Char(ch)),
                });
            }

            '\\' => {
                lexer.advance();
                decode_escape(lexer, &mut value, EscapeMode::Char);
            }

            '\n' => {
                lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidToken {
                    position: format!("{}:{}", start_pos.line, start_pos.column),

                    span: since(lexer, start_pos),

                    message: "Unterminated character literal".to_string(),
                });

                return Some(Token {
                    kind: TokenKind::Error("Unterminated char".to_string()),

                    span: lexer.span(),

                    literal: None,
                });
            }

            c => {
                value.push(c);

                lexer.advance();
            }
        }
    }

    lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidToken {
        position: format!("{}:{}", start_pos.line, start_pos.column),

        span: since(lexer, start_pos),

        message: "Unterminated character literal".to_string(),
    });

    Some(Token {
        kind: TokenKind::Error("Unterminated char".to_string()),

        span: lexer.span(),

        literal: None,
    })
}

/// Check if character is valid identifier start
pub fn is_identifier_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

/// Check if character is valid identifier continuation
pub fn is_identifier_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Check if character is a digit
pub fn is_digit(c: char) -> bool {
    c.is_ascii_digit()
}

/// Check if character is a hexadecimal digit
pub fn is_hex_digit(c: char) -> bool {
    c.is_ascii_digit() || ('a'..='f').contains(&c) || ('A'..='F').contains(&c)
}

/// Get hexadecimal digit value
pub fn hex_digit_value(c: char) -> u128 {
    if c.is_ascii_digit() {
        c as u128 - b'0' as u128
    } else if ('a'..='f').contains(&c) {
        10 + c as u128 - b'a' as u128
    } else {
        10 + c as u128 - b'A' as u128
    }
}

/// RFC-012: 把插值内容切成表达式与格式说明符。
///
/// "pi:.2f" → ("pi", Some(".2f"))
/// "x + y" → ("x + y", None)
/// "name" → ("name", None)
///
/// 8.2.2 起口径只此一份（原在 parser），词法期为唯一使用者。
fn split_format_spec(s: &str) -> (String, Option<String>) {
    // Find the last ':' that is not inside balanced brackets/parens
    let mut depth_paren = 0i32;
    let mut depth_bracket = 0i32;
    let mut last_colon_pos = None;

    for (i, c) in s.char_indices() {
        match c {
            '(' => depth_paren += 1,
            ')' => depth_paren -= 1,
            '[' => depth_bracket += 1,
            ']' => depth_bracket -= 1,
            ':' if depth_paren == 0 && depth_bracket == 0 => {
                last_colon_pos = Some(i);
            }
            _ => {}
        }
    }

    if let Some(pos) = last_colon_pos {
        let expr_part = s[..pos].to_string();
        let spec_part = s[pos + 1..].to_string();
        // Only treat as format spec if the part after : looks like a format spec
        // (doesn't contain spaces or look like another expression)
        if !spec_part.is_empty() && !spec_part.contains(' ') {
            return (expr_part, Some(spec_part));
        }
    }

    (s.to_string(), None)
}

/// 插值开括号的扫描期锚点：raw 内 `{` 的字节偏移 → 紧随其后的源文件绝对位置。
///
/// raw 是转义解码后的内容，raw 偏移与源偏移会错位（如文本段的 `\n` 两字符解码
/// 成一字符）；只有扫描器在消费 `{` 的当下才拿得到源文件绝对位置，故锚点必须
/// 在扫描期记录、切段期查用（#574：插值子词法的 base 不允许事后从 raw 反推）。
/// 「插值开括号」两侧判据一致：扫描器与 [`split_fstring_segments`] 都以
/// 「`{` 且下一字符不是 `{`」界定，因此切段端遇到的每个开括号必有锚点
/// （扫描器还会为插值内部的嵌套 `{` 多记几条，查用时按偏移命中即可）。
type FStringAnchor = (usize, Position);

/// 从「紧随插值开括号」的绝对位置走到 trim 后表达式首字符。
///
/// 前导空白推进的前提是 expr 缓冲的前缀与源文本逐字对应：**单行** f-string
/// 的转义只在文本段解码（插值收集路径原样拷贝，见 `scan_fstring`），该前提
/// 成立；**三引号** f-string 的扫描器对任意 `\` 解码、无花括号深度闸（见
/// [`scan_triple_quoted`] 文注），插值内含转义序列时 expr 缓冲与源文本错位，
/// 本函数的推进在该病态子集上不精确——登记形态（8.2.3 四类）均不含此情形。
/// 前导空白本身逐字符推进：换行使 line 前进、column 归一，其余空白 column
/// 前进。
fn position_after_leading_whitespace(
    start: Position,
    leading: &str,
) -> Position {
    let mut pos = start;
    for c in leading.chars() {
        if c == '\n' {
            pos.line += 1;
            pos.column = 1;
        } else {
            pos.column += 1;
        }
        pos.offset += c.len_utf8();
    }
    pos
}

/// 按开括号的 raw 偏移取扫描期锚点。锚点按扫描顺序（即 raw 偏移升序）记录，
/// 二分命中；查不到只可能是两侧「插值开括号」判据失配（内部不变量被破坏），
/// 回退哑位置并把症状留在诊断位置上，不 panic。
fn anchor_after_open(
    anchors: &[FStringAnchor],
    open_offset: usize,
) -> Position {
    match anchors.binary_search_by_key(&open_offset, |a| a.0) {
        Ok(i) => anchors[i].1,
        Err(_) => {
            debug_assert!(
                false,
                "interpolation open at raw offset {open_offset} has no scanner anchor"
            );
            Position::dummy()
        }
    }
}

/// RFC-012 / #402: 词法期切分 f-string 内容（唯一事实源）。
///
/// 输入是已解码转义、保留花括号序列的 raw 内容；输出 Text / Interpolation 段，
/// 口径与 8.2.2 之前 parser 的切分逐字一致：
/// - {{ → 文本 {，}} 与孤立 } → 文本 }；
/// - 插值按花括号配对截取，顶层 ':' 后的非空无空格部分作为格式说明符；
/// - 表达式 trim 后为空则不产段（parser 旧实现同样跳过）。
///
/// anchors 是扫描期记录的插值开括号锚点（[`FStringAnchor`]）：每个插值段的
/// base 由「开括号后的绝对位置 + expr 前导空白推进」得出，插值子词法据此
/// 落回源文件绝对行列（#574 / WBS 8.2.3）。
fn split_fstring_segments(
    raw: &str,
    anchors: &[FStringAnchor],
) -> Vec<FStringTokenSegment> {
    let mut segments = Vec::new();
    let mut chars = raw.char_indices().peekable();
    let mut text_buf = String::new();

    while let Some(&(open_offset, c)) = chars.peek() {
        if c == '{' {
            chars.next();

            // Escaped {{ → literal {
            if chars.peek().map(|&(_, next)| next) == Some('{') {
                chars.next();
                text_buf.push('{');
                continue;
            }

            // Flush text buffer
            if !text_buf.is_empty() {
                segments.push(FStringTokenSegment::Text(std::mem::take(&mut text_buf)));
            }

            // Collect expression content (until matching })
            let mut expr_buf = String::new();
            let mut depth = 1;
            while let Some(&(_, inner)) = chars.peek() {
                if inner == '{' {
                    depth += 1;
                    expr_buf.push(inner);
                    chars.next();
                } else if inner == '}' {
                    depth -= 1;
                    if depth == 0 {
                        chars.next();
                        break;
                    }
                    expr_buf.push(inner);
                    chars.next();
                } else {
                    expr_buf.push(inner);
                    chars.next();
                }
            }

            // Split on ':' for format spec (top level only, not inside nested exprs)
            let (expr_str, format_spec) = split_format_spec(&expr_buf);
            let expr = expr_str.trim().to_string();
            if expr.is_empty() {
                // Empty interpolation: f"{}" → 不产段（与旧 parser 一致）
                continue;
            }
            // base = 开括号后的绝对位置 + expr 前导空白推进（trim 的前导部分
            // 与源文本逐字对应）；尾随空白不参与——子词法只从 expr 首字符起扫。
            let leading_ws_len = expr_str.len() - expr_str.trim_start().len();
            let base = position_after_leading_whitespace(
                anchor_after_open(anchors, open_offset),
                &expr_str[..leading_ws_len],
            );
            segments.push(FStringTokenSegment::Interpolation {
                expr,
                format_spec,
                raw: expr_buf,
                base,
            });
        } else if c == '}' {
            chars.next();

            // Escaped }} → literal }; a lone } is kept literal too
            if chars.peek().map(|&(_, next)| next) == Some('}') {
                chars.next();
            }
            text_buf.push('}');
        } else {
            text_buf.push(c);
            chars.next();
        }
    }

    // Flush remaining text
    if !text_buf.is_empty() {
        segments.push(FStringTokenSegment::Text(text_buf));
    }

    // If no segments found, return a single empty text
    if segments.is_empty() {
        segments.push(FStringTokenSegment::Text(String::new()));
    }

    segments
}

/// 构造 f-string token 载荷：raw 逐字保留（Display 与既有消费者不变），
/// segments 是词法期切好的唯一事实源，锚点随切段算出各插值段的 base。
fn make_fstring_token(
    raw: String,
    anchors: &[FStringAnchor],
) -> FStringToken {
    let segments = split_fstring_segments(&raw, anchors);
    FStringToken { raw, segments }
}

/// RFC-012: Scan f-string literal
///
/// Called after consuming `f` and `"`. Scans the content of f"..." including
/// interpolation expressions delimited by `{` and `}`.
/// The raw content is stored as a single string with `{` and `}` markers preserved.
/// 段（Text / Interpolation）在词法期切好放进 token 载荷，parser 直接消费。
///
/// Raw protocol (#402): brace sequences are preserved **verbatim** — `{{` and
/// `}}` stay as two characters so the lexer can distinguish escaped braces
/// from interpolation markers when splitting segments. Backslash escape
/// sequences (\n, \t, \u{...}, ...) are processed here, same as regular strings.
///
/// `f"""` opens a multi-line f-string (RFC-012), terminated by `"""`.
pub fn scan_fstring(lexer: &mut super::tokenizer::Lexer<'_>) -> Option<Token> {
    let start_pos = lexer.position();

    // Check for multi-line f-string (f""")

    // The first '"' was already consumed by scan_identifier; check for two more

    let c0 = lexer.peek_public().copied();

    let chars_copy = lexer.chars_clone();

    let chars_vec: Vec<char> = chars_copy.collect();

    let has_three_quotes = c0 == Some('"') && chars_vec.get(1) == Some(&'"');

    if has_three_quotes {
        let _ = lexer.advance(); // Skip second quote

        let _ = lexer.advance(); // Skip third quote

        return scan_triple_quoted(lexer, TripleQuoted::FString);
    }

    let mut value = String::new();

    let mut brace_depth = 0;

    // 插值开括号的扫描期锚点（#574）：`{{` 转义不是开括号、不记录；
    // 其余进入插值收集的 `{` 在 advance 消费后记下紧随其后的源文件绝对位置。
    let mut anchors: Vec<FStringAnchor> = Vec::new();

    while let Some(&c) = lexer.peek() {
        match c {
            '"' if brace_depth == 0 => {
                lexer.advance();

                return Some(Token {
                    kind: TokenKind::FStringLiteral(make_fstring_token(value.clone(), &anchors)),

                    span: Span::new(
                        Position::with_offset(
                            lexer.start_line(),
                            lexer.start_column(),
                            lexer.start_offset(),
                        ),
                        lexer.position(),
                    ),

                    literal: None,
                });
            }

            '{' => {
                lexer.advance();

                // `{{` at text level is a brace escape (RFC-012): preserved
                // raw and NOT counted as interpolation open — otherwise the
                // closing `"` would be swallowed by a phantom depth (#402)

                if brace_depth == 0 && lexer.peek() == Some(&'{') {
                    lexer.advance();

                    value.push('{');

                    value.push('{');

                    continue;
                }

                brace_depth += 1;

                anchors.push((value.len(), lexer.position()));

                value.push('{');
            }

            '}' => {
                lexer.advance();

                if brace_depth > 0 {
                    brace_depth -= 1;
                }

                value.push('}');
            }

            '\\' if brace_depth == 0 => {
                lexer.advance();
                decode_escape(lexer, &mut value, EscapeMode::Strict);
            }

            '\n' if brace_depth == 0 => {
                lexer.error = Some(crate::frontend::core::lexer::LexError::UnterminatedString {
                    position: format!("{}:{}", start_pos.line, start_pos.column),

                    span: since(lexer, start_pos),
                });

                return Some(Token {
                    kind: TokenKind::Error("Unterminated f-string".to_string()),

                    span: lexer.span(),

                    literal: None,
                });
            }

            c => {
                value.push(c);

                lexer.advance();
            }
        }
    }

    // Unterminated f-string

    if brace_depth > 0 {
        lexer.error = Some(
            crate::frontend::core::lexer::LexError::UnterminatedFStringInterpolation {
                position: format!("{}:{}", start_pos.line, start_pos.column),

                span: since(lexer, start_pos),
            },
        );
    } else {
        lexer.error = Some(crate::frontend::core::lexer::LexError::UnterminatedString {
            position: format!("{}:{}", start_pos.line, start_pos.column),

            span: since(lexer, start_pos),
        });
    }

    Some(Token {
        kind: TokenKind::Error("Unterminated f-string".to_string()),

        span: lexer.span(),

        literal: None,
    })
}

/// 转义序列的解码口径。三处调用点的历史行为不同，差异只在未知转义
/// 与零字节 / 十六进制 / Unicode 转义是否解码；解码骨架唯一（decode_escape）。
#[derive(Clone, Copy, PartialEq, Eq)]
enum EscapeMode {
    /// 单行字符串与 f-string：解码零字节、十六进制与 Unicode 转义，
    /// 未知转义报 InvalidEscape。
    Strict,
    /// 字符字面量：解码零字节、十六进制与 Unicode 转义，未知转义原样取该字符。
    Char,
    /// 多行字符串：只解码 n / t / r / 反斜杠 / 双引号 / 单引号，
    /// 其余序列（含 0 / x / u）原样保留反斜杠与字符，不报错。
    Lenient,
}

/// 唯一的转义解码实现。调用点已消费反斜杠；本函数消费紧随其后的转义字符。
fn decode_escape(
    lexer: &mut super::tokenizer::Lexer<'_>,
    value: &mut String,
    mode: EscapeMode,
) {
    let Some(escaped) = lexer.advance() else {
        return;
    };

    if mode == EscapeMode::Lenient {
        match escaped {
            'n' => value.push('\n'),
            't' => value.push('\t'),
            'r' => value.push('\r'),
            '\\' => value.push('\\'),
            '"' => value.push('"'),
            '\'' => value.push('\''),
            other => {
                // 多行字符串：未识别的转义保留反斜杠与字符
                value.push('\\');
                value.push(other);
            }
        }
        return;
    }

    match escaped {
        'n' => value.push('\n'),
        't' => value.push('\t'),
        'r' => value.push('\r'),
        '\\' => value.push('\\'),
        '"' => value.push('"'),
        '\'' => value.push('\''),
        '0' => value.push('\0'),
        'x' => decode_hex_byte(lexer, value),
        'u' => decode_unicode(lexer, value),
        other if mode == EscapeMode::Char => value.push(other),
        other => {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidEscape {
                sequence: other.to_string(),
                span: point(lexer),
            });
        }
    }
}

/// 解码十六进制字节转义：两位十六进制数字。不足两位或非法报 InvalidEscape。
fn decode_hex_byte(
    lexer: &mut super::tokenizer::Lexer<'_>,
    value: &mut String,
) {
    let mut hex = String::new();
    for _ in 0..2 {
        if let Some(&hc) = lexer.peek() {
            if is_hex_digit(hc) {
                hex.push(hc);
                lexer.advance();
            } else {
                break;
            }
        }
    }

    let byte = if hex.len() == 2 {
        u8::from_str_radix(&hex, 16).ok()
    } else {
        None
    };
    match byte {
        Some(byte) => value.push(byte as char),
        None => {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidEscape {
                sequence: format!("\\x{hex}"),
                span: point(lexer),
            });
        }
    }
}

/// 解码 Unicode 转义：u{...} 码点。缺花括号、空码点或越界码点报 InvalidEscape。
fn decode_unicode(
    lexer: &mut super::tokenizer::Lexer<'_>,
    value: &mut String,
) {
    if lexer.peek() != Some(&'{') {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidEscape {
            sequence: "\\u".to_string(),
            span: point(lexer),
        });
        return;
    }
    lexer.advance();

    let mut hex = String::new();
    while let Some(&hc) = lexer.peek() {
        if is_hex_digit(hc) {
            hex.push(hc);
            lexer.advance();
        } else {
            break;
        }
    }

    if lexer.peek() != Some(&'}') || hex.is_empty() {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidEscape {
            sequence: "\\u{".to_string(),
            span: point(lexer),
        });
        return;
    }
    lexer.advance();

    let decoded = u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32);
    match decoded {
        Some(ch) => value.push(ch),
        None => {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidEscape {
                sequence: format!("\\u{{{hex}}}"),
                span: point(lexer),
            });
        }
    }
}
