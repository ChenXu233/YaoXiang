//! Lexer module

pub mod tokens;

use tokens::*;

pub use tokenizer::tokenize;

/// Lexer error
#[derive(Debug, thiserror::Error)]
pub enum LexError {
    #[error("Invalid token at {position}: {message}")]
    InvalidToken { position: String, message: String },
    #[error("Unterminated string starting at {position}")]
    UnterminatedString { position: String },
    #[error("Invalid escape sequence: {sequence}")]
    InvalidEscape { sequence: String },
    #[error("Invalid number literal: {0}")]
    InvalidNumber(String),
    #[error("Unexpected character: '{ch}'")]
    UnexpectedChar { ch: char },
}

/// Tokenize source code
mod tokenizer {
    use super::*;
    use crate::util::span::{Position, Span};
    use std::iter::Peekable;
    use std::str::Chars;

    pub fn tokenize(source: &str) -> Result<Vec<Token>, LexError> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();

        while let Some(token) = lexer.next_token() {
            tokens.push(token);
        }

        if let Some(err) = lexer.error {
            Err(err)
        } else {
            tokens.push(Token {
                kind: TokenKind::Eof,
                span: Span::new(
                    Position::with_offset(lexer.line, lexer.column, lexer.offset),
                    Position::with_offset(lexer.line, lexer.column + 1, lexer.offset + 1),
                ),
                literal: None,
            });
            Ok(tokens)
        }
    }

    struct Lexer<'a> {
        chars: Peekable<Chars<'a>>,
        offset: usize,
        line: usize,
        column: usize,
        start_offset: usize,
        start_line: usize,
        start_column: usize,
        error: Option<LexError>,
    }

    impl<'a> Lexer<'a> {
        fn new(source: &'a str) -> Self {
            Self {
                chars: source.chars().peekable(),
                offset: 0,
                line: 1,
                column: 1,
                start_offset: 0,
                start_line: 1,
                start_column: 1,
                error: None,
            }
        }

        fn position(&self) -> Position {
            Position::with_offset(self.line, self.column, self.offset)
        }

        fn start_position(&self) -> Position {
            Position::with_offset(self.start_line, self.start_column, self.start_offset)
        }

        fn span(&self) -> Span {
            Span::new(self.start_position(), self.position())
        }

        fn advance(&mut self) -> Option<char> {
            match self.chars.next() {
                Some('\n') => {
                    self.offset += 1;
                    self.line += 1;
                    self.column = 1;
                    Some('\n')
                }
                Some(c) => {
                    self.offset += c.len_utf8();
                    self.column += 1;
                    Some(c)
                }
                None => None,
            }
        }

        fn peek(&mut self) -> Option<&char> {
            self.chars.peek()
        }

        fn peek_next(&mut self) -> Option<char> {
            self.chars.clone().nth(1)
        }

        fn skip_whitespace_and_comments(&mut self) {
            while let Some(&c) = self.peek() {
                match c {
                    ' ' | '\t' | '\r' | '\n' => {
                        self.advance();
                    }
                    _ => break,
                }
            }
        }

        fn next_token(&mut self) -> Option<Token> {
            self.skip_whitespace_and_comments();

            // 检查是否到达文件末尾
            self.peek()?;

            self.start_offset = self.offset;
            self.start_line = self.line;
            self.start_column = self.column;

            let c = self.advance().unwrap();

            match c {
                '_' => {
                    // Check if next char is part of identifier (e.g., _foo)
                    // Only treat standalone _ as Underscore token
                    if self.peek().map(|&c| is_identifier_char(c)).unwrap_or(false) {
                        self.scan_identifier(c)
                    } else {
                        Some(self.make_token(TokenKind::Underscore))
                    }
                }
                c if is_identifier_start(c) => self.scan_identifier(c),
                c if is_digit(c) => self.scan_number(c),
                '"' => self.scan_string(),
                '\'' => self.scan_char(),
                '+' => Some(self.make_token(TokenKind::Plus)),
                '-' => {
                    if self.peek() == Some(&'>') {
                        self.advance();
                        Some(self.make_token(TokenKind::Arrow))
                    } else {
                        Some(self.make_token(TokenKind::Minus))
                    }
                }
                '*' => Some(self.make_token(TokenKind::Star)),
                '%' => Some(self.make_token(TokenKind::Percent)),
                ',' => Some(self.make_token(TokenKind::Comma)),
                ';' => Some(self.make_token(TokenKind::Semicolon)),
                '(' => Some(self.make_token(TokenKind::LParen)),
                ')' => Some(self.make_token(TokenKind::RParen)),
                '[' => Some(self.make_token(TokenKind::LBracket)),
                ']' => Some(self.make_token(TokenKind::RBracket)),
                '{' => Some(self.make_token(TokenKind::LBrace)),
                '}' => Some(self.make_token(TokenKind::RBrace)),
                '=' => {
                    if self.peek() == Some(&'>') {
                        self.advance();
                        Some(self.make_token(TokenKind::FatArrow))
                    } else if self.peek() == Some(&'=') {
                        self.advance();
                        Some(self.make_token(TokenKind::EqEq))
                    } else {
                        Some(self.make_token(TokenKind::Eq))
                    }
                }
                '!' => {
                    if self.peek() == Some(&'=') {
                        self.advance();
                        Some(self.make_token(TokenKind::Neq))
                    } else {
                        Some(self.make_token(TokenKind::Not))
                    }
                }
                '<' => {
                    if self.peek() == Some(&'=') {
                        self.advance();
                        Some(self.make_token(TokenKind::Le))
                    } else {
                        Some(self.make_token(TokenKind::Lt))
                    }
                }
                '>' => {
                    if self.peek() == Some(&'=') {
                        self.advance();
                        Some(self.make_token(TokenKind::Ge))
                    } else {
                        Some(self.make_token(TokenKind::Gt))
                    }
                }
                '&' => {
                    if self.peek() == Some(&'&') {
                        self.advance();
                        Some(self.make_token(TokenKind::And))
                    } else {
                        self.error = Some(LexError::UnexpectedChar { ch: '&' });
                        Some(
                            self.make_token(TokenKind::Error(
                                "Unexpected character: &".to_string(),
                            )),
                        )
                    }
                }
                '|' => {
                    if self.peek() == Some(&'|') {
                        self.advance();
                        Some(self.make_token(TokenKind::Or))
                    } else {
                        Some(self.make_token(TokenKind::Pipe))
                    }
                }
                ':' => {
                    if self.peek() == Some(&':') {
                        self.advance();
                        Some(self.make_token(TokenKind::ColonColon))
                    } else {
                        Some(self.make_token(TokenKind::Colon))
                    }
                }
                '.' => {
                    if self.peek() == Some(&'.') {
                        self.advance();
                        if self.peek() == Some(&'.') {
                            self.advance();
                            Some(self.make_token(TokenKind::DotDotDot))
                        } else {
                            Some(self.make_token(TokenKind::DotDot))
                        }
                    } else {
                        Some(self.make_token(TokenKind::Dot))
                    }
                }
                '/' => {
                    if self.peek() == Some(&'/') {
                        // Single line comment
                        self.advance();
                        self.advance();
                        while let Some(&c) = self.peek() {
                            if c == '\n' {
                                break;
                            }
                            self.advance();
                        }
                        // Continue to get next token
                        self.next_token()
                    } else if self.peek() == Some(&'*') {
                        // Multi-line comment
                        self.advance();
                        self.advance();
                        let mut depth = 1;
                        while depth > 0 {
                            if let Some(c) = self.advance() {
                                if c == '/' && self.peek() == Some(&'*') {
                                    self.advance();
                                    depth += 1;
                                } else if c == '*' && self.peek() == Some(&'/') {
                                    self.advance();
                                    depth -= 1;
                                }
                            } else {
                                break;
                            }
                        }
                        // Continue to get next token
                        self.next_token()
                    } else {
                        Some(self.make_token(TokenKind::Slash))
                    }
                }
                c => {
                    self.error = Some(LexError::UnexpectedChar { ch: c });
                    Some(self.make_token(TokenKind::Error(format!("Unexpected character: {}", c))))
                }
            }
        }

        fn scan_identifier(
            &mut self,
            first_char: char,
        ) -> Option<Token> {
            let mut value = String::new();
            value.push(first_char);

            while let Some(&c) = self.peek() {
                if is_identifier_char(c) {
                    value.push(c);
                    self.advance();
                } else {
                    break;
                }
            }

            if let Some(kind) = self.keyword_from_str(&value) {
                Some(Token {
                    kind,
                    span: self.span(),
                    literal: None,
                })
            } else {
                Some(Token {
                    kind: TokenKind::Identifier(value.clone()),
                    span: self.span(),
                    literal: None,
                })
            }
        }

        fn scan_number(
            &mut self,
            first_char: char,
        ) -> Option<Token> {
            let mut value = String::new();
            value.push(first_char);

            // 检测进制前缀
            let base = if first_char == '0' {
                if self.peek() == Some(&'x') || self.peek() == Some(&'X') {
                    self.advance();
                    value.push('x');
                    Some(16)
                } else if self.peek() == Some(&'o') || self.peek() == Some(&'O') {
                    self.advance();
                    value.push('o');
                    Some(8)
                } else if self.peek() == Some(&'b') || self.peek() == Some(&'B') {
                    self.advance();
                    value.push('b');
                    Some(2)
                } else {
                    None
                }
            } else {
                None
            };

            match base {
                Some(16) => self.scan_hex_number(value),
                Some(8) => self.scan_octal_number(value),
                Some(2) => self.scan_binary_number(value),
                None => self.scan_decimal_number(value),
                _ => unreachable!("Invalid base value"),
            }
        }

        fn scan_hex_number(
            &mut self,
            mut value: String,
        ) -> Option<Token> {
            let mut num_value: u128 = 0;
            let mut has_digits = false;

            while let Some(&c) = self.peek() {
                if is_hex_digit(c) {
                    num_value = num_value * 16 + hex_digit_value(c);
                    value.push(c);
                    self.advance();
                    has_digits = true;
                } else if c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }

            if !has_digits {
                self.error = Some(LexError::InvalidNumber("Expected hex digits".to_string()));
                return Some(self.make_token(TokenKind::Error("Invalid hex number".to_string())));
            }

            match num_value.try_into() {
                Ok(n) => Some(Token {
                    kind: TokenKind::IntLiteral(n),
                    span: self.span(),
                    literal: Some(Literal::Int(n)),
                }),
                Err(_) => {
                    self.error = Some(LexError::InvalidNumber(value));
                    Some(self.make_token(TokenKind::Error("Hex number too large".to_string())))
                }
            }
        }

        fn scan_octal_number(
            &mut self,
            mut value: String,
        ) -> Option<Token> {
            let mut num_value: u128 = 0;
            let mut has_digits = false;

            while let Some(&c) = self.peek() {
                if c >= '0' && c <= '7' {
                    num_value = num_value * 8 + (c as u128 - b'0' as u128);
                    value.push(c);
                    self.advance();
                    has_digits = true;
                } else if c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }

            if !has_digits {
                self.error = Some(LexError::InvalidNumber("Expected octal digits".to_string()));
                return Some(self.make_token(TokenKind::Error("Invalid octal number".to_string())));
            }

            match num_value.try_into() {
                Ok(n) => Some(Token {
                    kind: TokenKind::IntLiteral(n),
                    span: self.span(),
                    literal: Some(Literal::Int(n)),
                }),
                Err(_) => {
                    self.error = Some(LexError::InvalidNumber(value));
                    Some(self.make_token(TokenKind::Error("Octal number too large".to_string())))
                }
            }
        }

        fn scan_binary_number(
            &mut self,
            mut value: String,
        ) -> Option<Token> {
            let mut num_value: u128 = 0;
            let mut has_digits = false;

            while let Some(&c) = self.peek() {
                if c == '0' || c == '1' {
                    num_value = num_value * 2 + (c as u128 - b'0' as u128);
                    value.push(c);
                    self.advance();
                    has_digits = true;
                } else if c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }

            if !has_digits {
                self.error = Some(LexError::InvalidNumber(
                    "Expected binary digits".to_string(),
                ));
                return Some(
                    self.make_token(TokenKind::Error("Invalid binary number".to_string())),
                );
            }

            match num_value.try_into() {
                Ok(n) => Some(Token {
                    kind: TokenKind::IntLiteral(n),
                    span: self.span(),
                    literal: Some(Literal::Int(n)),
                }),
                Err(_) => {
                    self.error = Some(LexError::InvalidNumber(value));
                    Some(self.make_token(TokenKind::Error("Binary number too large".to_string())))
                }
            }
        }

        fn scan_decimal_number(
            &mut self,
            mut value: String,
        ) -> Option<Token> {
            while let Some(&c) = self.peek() {
                if is_digit(c) {
                    value.push(c);
                    self.advance();
                } else if c == '_' {
                    self.advance();
                } else {
                    break;
                }
            }

            // 检查是否有小数部分
            if self.peek() == Some(&'.') {
                let next = self.peek_next();
                if next.map(is_digit).unwrap_or(false) || next == Some('_') {
                    value.push(self.advance().unwrap());
                    while let Some(&c) = self.peek() {
                        if is_digit(c) {
                            value.push(c);
                            self.advance();
                        } else if c == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }

            // 检查是否有指数部分
            if self.peek() == Some(&'e') || self.peek() == Some(&'E') {
                value.push(self.advance().unwrap());
                if self.peek() == Some(&'+') || self.peek() == Some(&'-') {
                    value.push(self.advance().unwrap());
                }
                let mut has_digits = false;
                while let Some(&c) = self.peek() {
                    if is_digit(c) {
                        value.push(c);
                        self.advance();
                        has_digits = true;
                    } else if c == '_' {
                        self.advance();
                    } else {
                        break;
                    }
                }
                if !has_digits {
                    self.error = Some(LexError::InvalidNumber(
                        "Expected digits in exponent".to_string(),
                    ));
                }
            }

            let cleaned: String = value.chars().filter(|&c| c != '_').collect();
            let num_str = &cleaned;

            if num_str.contains('.') || num_str.contains('e') || num_str.contains('E') {
                match num_str.parse::<f64>() {
                    Ok(n) => Some(Token {
                        kind: TokenKind::FloatLiteral(n),
                        span: self.span(),
                        literal: Some(Literal::Float(n)),
                    }),
                    Err(_) => {
                        self.error = Some(LexError::InvalidNumber(value));
                        Some(self.make_token(TokenKind::Error("Invalid float".to_string())))
                    }
                }
            } else {
                match num_str.parse::<i128>() {
                    Ok(n) => Some(Token {
                        kind: TokenKind::IntLiteral(n),
                        span: self.span(),
                        literal: Some(Literal::Int(n)),
                    }),
                    Err(_) => {
                        self.error = Some(LexError::InvalidNumber(value));
                        Some(self.make_token(TokenKind::Error("Invalid integer".to_string())))
                    }
                }
            }
        }

        fn scan_string(&mut self) -> Option<Token> {
            let mut value = String::new();
            let start_pos = self.position();

            while let Some(&c) = self.peek() {
                match c {
                    '"' => {
                        self.advance();
                        return Some(Token {
                            kind: TokenKind::StringLiteral(value.clone()),
                            span: Span::new(
                                Position::with_offset(
                                    self.start_line,
                                    self.start_column,
                                    self.start_offset,
                                ),
                                self.position(),
                            ),
                            literal: Some(Literal::String(value.clone())),
                        });
                    }
                    '\\' => {
                        self.advance();
                        if let Some(escaped) = self.advance() {
                            match escaped {
                                'n' => value.push('\n'),
                                't' => value.push('\t'),
                                'r' => value.push('\r'),
                                '\\' => value.push('\\'),
                                '"' => value.push('"'),
                                '\'' => value.push('\''),
                                '0' => value.push('\0'),
                                'x' => {
                                    // 十六进制转义 \xFF
                                    let mut hex = String::new();
                                    for _ in 0..2 {
                                        if let Some(&hc) = self.peek() {
                                            if is_hex_digit(hc) {
                                                hex.push(hc);
                                                self.advance();
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                    if hex.len() == 2 {
                                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                                            value.push(byte as char);
                                        } else {
                                            self.error = Some(LexError::InvalidEscape {
                                                sequence: format!("\\x{}", hex),
                                            });
                                        }
                                    } else {
                                        self.error = Some(LexError::InvalidEscape {
                                            sequence: format!("\\x{}", hex),
                                        });
                                    }
                                }
                                'u' => {
                                    // Unicode 转义 \u{1F600}
                                    if self.peek() == Some(&'{') {
                                        self.advance();
                                        let mut hex = String::new();
                                        while let Some(&hc) = self.peek() {
                                            if is_hex_digit(hc) {
                                                hex.push(hc);
                                                self.advance();
                                            } else {
                                                break;
                                            }
                                        }
                                        if self.peek() == Some(&'}') && !hex.is_empty() {
                                            self.advance();
                                            if let Ok(codepoint) = u32::from_str_radix(&hex, 16) {
                                                if let Some(ch) = char::from_u32(codepoint) {
                                                    value.push(ch);
                                                } else {
                                                    self.error = Some(LexError::InvalidEscape {
                                                        sequence: format!("\\u{{{}}}", hex),
                                                    });
                                                }
                                            } else {
                                                self.error = Some(LexError::InvalidEscape {
                                                    sequence: format!("\\u{{{}}}", hex),
                                                });
                                            }
                                        } else {
                                            self.error = Some(LexError::InvalidEscape {
                                                sequence: "\\u{".to_string(),
                                            });
                                        }
                                    } else {
                                        self.error = Some(LexError::InvalidEscape {
                                            sequence: "\\u".to_string(),
                                        });
                                    }
                                }
                                c => {
                                    self.error = Some(LexError::InvalidEscape {
                                        sequence: c.to_string(),
                                    });
                                }
                            }
                        }
                    }
                    '\n' => {
                        self.error = Some(LexError::UnterminatedString {
                            position: format!("{}:{}", start_pos.line, start_pos.column),
                        });
                        return Some(Token {
                            kind: TokenKind::Error("Unterminated string".to_string()),
                            span: self.span(),
                            literal: None,
                        });
                    }
                    c => {
                        value.push(c);
                        self.advance();
                    }
                }
            }

            self.error = Some(LexError::UnterminatedString {
                position: format!("{}:{}", start_pos.line, start_pos.column),
            });
            Some(Token {
                kind: TokenKind::Error("Unterminated string".to_string()),
                span: self.span(),
                literal: None,
            })
        }

        fn scan_char(&mut self) -> Option<Token> {
            let start_pos = self.position();
            let mut value = String::new();

            while let Some(&c) = self.peek() {
                match c {
                    '\'' => {
                        self.advance();
                        let ch = match value.chars().next() {
                            Some(c) => c,
                            None => {
                                self.error = Some(LexError::InvalidToken {
                                    position: format!("{}:{}", start_pos.line, start_pos.column),
                                    message: "Empty character literal".to_string(),
                                });
                                return Some(Token {
                                    kind: TokenKind::Error("Empty character literal".to_string()),
                                    span: self.span(),
                                    literal: None,
                                });
                            }
                        };
                        return Some(Token {
                            kind: TokenKind::CharLiteral(ch),
                            span: Span::new(
                                Position::with_offset(
                                    self.start_line,
                                    self.start_column,
                                    self.start_offset,
                                ),
                                self.position(),
                            ),
                            literal: Some(Literal::Char(ch)),
                        });
                    }
                    '\\' => {
                        self.advance();
                        if let Some(escaped) = self.advance() {
                            match escaped {
                                'n' => value.push('\n'),
                                't' => value.push('\t'),
                                'r' => value.push('\r'),
                                '\\' => value.push('\\'),
                                '\'' => value.push('\''),
                                '"' => value.push('"'),
                                '0' => value.push('\0'),
                                'x' => {
                                    // 十六进制转义 \x41
                                    let mut hex = String::new();
                                    for _ in 0..2 {
                                        if let Some(&hc) = self.peek() {
                                            if is_hex_digit(hc) {
                                                hex.push(hc);
                                                self.advance();
                                            } else {
                                                break;
                                            }
                                        }
                                    }
                                    if hex.len() == 2 {
                                        if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                                            value.push(byte as char);
                                        } else {
                                            self.error = Some(LexError::InvalidEscape {
                                                sequence: format!("\\x{}", hex),
                                            });
                                        }
                                    } else {
                                        self.error = Some(LexError::InvalidEscape {
                                            sequence: format!("\\x{}", hex),
                                        });
                                    }
                                }
                                'u' => {
                                    // Unicode 转义 \u{1F600}
                                    if self.peek() == Some(&'{') {
                                        self.advance();
                                        let mut hex = String::new();
                                        while let Some(&hc) = self.peek() {
                                            if is_hex_digit(hc) {
                                                hex.push(hc);
                                                self.advance();
                                            } else {
                                                break;
                                            }
                                        }
                                        if self.peek() == Some(&'}') && !hex.is_empty() {
                                            self.advance();
                                            if let Ok(codepoint) = u32::from_str_radix(&hex, 16) {
                                                if let Some(ch) = char::from_u32(codepoint) {
                                                    value.push(ch);
                                                } else {
                                                    self.error = Some(LexError::InvalidEscape {
                                                        sequence: format!("\\u{{{}}}", hex),
                                                    });
                                                }
                                            } else {
                                                self.error = Some(LexError::InvalidEscape {
                                                    sequence: format!("\\u{{{}}}", hex),
                                                });
                                            }
                                        } else {
                                            self.error = Some(LexError::InvalidEscape {
                                                sequence: "\\u{".to_string(),
                                            });
                                        }
                                    } else {
                                        self.error = Some(LexError::InvalidEscape {
                                            sequence: "\\u".to_string(),
                                        });
                                    }
                                }
                                c => value.push(c),
                            }
                        }
                    }
                    '\n' => {
                        self.error = Some(LexError::InvalidToken {
                            position: format!("{}:{}", start_pos.line, start_pos.column),
                            message: "Unterminated character literal".to_string(),
                        });
                        return Some(Token {
                            kind: TokenKind::Error("Unterminated char".to_string()),
                            span: self.span(),
                            literal: None,
                        });
                    }
                    c => {
                        value.push(c);
                        self.advance();
                    }
                }
            }

            self.error = Some(LexError::InvalidToken {
                position: format!("{}:{}", start_pos.line, start_pos.column),
                message: "Unterminated character literal".to_string(),
            });
            Some(Token {
                kind: TokenKind::Error("Unterminated char".to_string()),
                span: self.span(),
                literal: None,
            })
        }

        fn make_token(
            &self,
            kind: TokenKind,
        ) -> Token {
            Token {
                kind,
                span: self.span(),
                literal: None,
            }
        }

        fn keyword_from_str(
            &self,
            s: &str,
        ) -> Option<TokenKind> {
            match s {
                "type" => Some(TokenKind::KwType),
                "pub" => Some(TokenKind::KwPub),
                "use" => Some(TokenKind::KwUse),
                "spawn" => Some(TokenKind::KwSpawn),
                "ref" => Some(TokenKind::KwRef),
                "mut" => Some(TokenKind::KwMut),
                "if" => Some(TokenKind::KwIf),
                "elif" => Some(TokenKind::KwElif),
                "else" => Some(TokenKind::KwElse),
                "match" => Some(TokenKind::KwMatch),
                "while" => Some(TokenKind::KwWhile),
                "for" => Some(TokenKind::KwFor),
                "in" => Some(TokenKind::KwIn),
                "return" => Some(TokenKind::KwReturn),
                "break" => Some(TokenKind::KwBreak),
                "continue" => Some(TokenKind::KwContinue),
                "as" => Some(TokenKind::KwAs),
                "true" => Some(TokenKind::BoolLiteral(true)),
                "false" => Some(TokenKind::BoolLiteral(false)),
                "void" => Some(TokenKind::VoidLiteral),
                _ => None,
            }
        }
    }

    fn is_identifier_start(c: char) -> bool {
        c.is_ascii_alphabetic() || c == '_'
    }
    fn is_identifier_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }
    fn is_digit(c: char) -> bool {
        c.is_ascii_digit()
    }
    fn is_hex_digit(c: char) -> bool {
        c.is_ascii_digit() || (c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')
    }
    fn hex_digit_value(c: char) -> u128 {
        if c.is_ascii_digit() {
            c as u128 - b'0' as u128
        } else if c >= 'a' && c <= 'f' {
            10 + c as u128 - b'a' as u128
        } else {
            10 + c as u128 - b'A' as u128
        }
    }
}

#[cfg(test)]
mod tests;
