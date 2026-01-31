//! Literal scanning implementations
//! Handles number, string, and character literals
//! Supports RFC-010/011 generic type literal syntax

use crate::util::span::{Position, Span};
use crate::frontend::core::lexer::tokens::*;

/// Scan a number literal (decimal, hex, octal, binary)
pub fn scan_number(
    lexer: &mut super::tokenizer::Lexer<'_>,
    first_char: char,
) -> Option<Token> {
    let mut value = String::new();
    value.push(first_char);

    // Detect base prefix
    let base = if first_char == '0' {
        if lexer.peek() == Some(&'x') || lexer.peek() == Some(&'X') {
            lexer.advance();
            value.push('x');
            Some(16)
        } else if lexer.peek() == Some(&'o') || lexer.peek() == Some(&'O') {
            lexer.advance();
            value.push('o');
            Some(8)
        } else if lexer.peek() == Some(&'b') || lexer.peek() == Some(&'B') {
            lexer.advance();
            value.push('b');
            Some(2)
        } else {
            None
        }
    } else {
        None
    };

    match base {
        Some(16) => scan_hex_number(lexer, value),
        Some(8) => scan_octal_number(lexer, value),
        Some(2) => scan_binary_number(lexer, value),
        None => scan_decimal_number(lexer, value),
        _ => unreachable!("Invalid base value"),
    }
}

/// Scan hexadecimal number
fn scan_hex_number(
    lexer: &mut super::tokenizer::Lexer<'_>,
    mut value: String,
) -> Option<Token> {
    let mut num_value: u128 = 0;
    let mut has_digits = false;
    let mut overflow = false;

    while let Some(&c) = lexer.peek() {
        if is_hex_digit(c) {
            let digit = hex_digit_value(c);
            if overflow {
                // Already overflowed, just continue to consume input
                value.push(c);
                lexer.advance();
            } else {
                match num_value.checked_mul(16).and_then(|v| v.checked_add(digit)) {
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

    if !has_digits {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
            "Expected hex digits".to_string(),
        ));
        return Some(lexer.make_token(TokenKind::Error("Invalid hex number".to_string())));
    }

    if overflow {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(value));
        return Some(lexer.make_token(TokenKind::Error("Hex number too large".to_string())));
    }

    match num_value.try_into() {
        Ok(n) => Some(Token {
            kind: TokenKind::IntLiteral(n),
            span: lexer.span(),
            literal: Some(Literal::Int(n)),
        }),
        Err(_) => {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(value));
            Some(lexer.make_token(TokenKind::Error("Hex number too large".to_string())))
        }
    }
}

/// Scan octal number
fn scan_octal_number(
    lexer: &mut super::tokenizer::Lexer<'_>,
    mut value: String,
) -> Option<Token> {
    let mut num_value: u128 = 0;
    let mut has_digits = false;
    let mut overflow = false;

    while let Some(&c) = lexer.peek() {
        if ('0'..='7').contains(&c) {
            let digit = c as u128 - b'0' as u128;
            if overflow {
                value.push(c);
                lexer.advance();
            } else {
                match num_value.checked_mul(8).and_then(|v| v.checked_add(digit)) {
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

    if !has_digits {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
            "Expected octal digits".to_string(),
        ));
        return Some(lexer.make_token(TokenKind::Error("Invalid octal number".to_string())));
    }

    if overflow {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(value));
        return Some(lexer.make_token(TokenKind::Error("Octal number too large".to_string())));
    }

    match num_value.try_into() {
        Ok(n) => Some(Token {
            kind: TokenKind::IntLiteral(n),
            span: lexer.span(),
            literal: Some(Literal::Int(n)),
        }),
        Err(_) => {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(value));
            Some(lexer.make_token(TokenKind::Error("Octal number too large".to_string())))
        }
    }
}

/// Scan binary number
fn scan_binary_number(
    lexer: &mut super::tokenizer::Lexer<'_>,
    mut value: String,
) -> Option<Token> {
    let mut num_value: u128 = 0;
    let mut has_digits = false;
    let mut overflow = false;

    while let Some(&c) = lexer.peek() {
        if c == '0' || c == '1' {
            let digit = c as u128 - b'0' as u128;
            if overflow {
                value.push(c);
                lexer.advance();
            } else {
                match num_value.checked_mul(2).and_then(|v| v.checked_add(digit)) {
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

    if !has_digits {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(
            "Expected binary digits".to_string(),
        ));
        return Some(lexer.make_token(TokenKind::Error("Invalid binary number".to_string())));
    }

    if overflow {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(value));
        return Some(lexer.make_token(TokenKind::Error("Binary number too large".to_string())));
    }

    match num_value.try_into() {
        Ok(n) => Some(Token {
            kind: TokenKind::IntLiteral(n),
            span: lexer.span(),
            literal: Some(Literal::Int(n)),
        }),
        Err(_) => {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(value));
            Some(lexer.make_token(TokenKind::Error("Binary number too large".to_string())))
        }
    }
}

/// Scan decimal number
fn scan_decimal_number(
    lexer: &mut super::tokenizer::Lexer<'_>,
    mut value: String,
) -> Option<Token> {
    let mut num_value: u128 = 0;
    let mut overflow = false;

    while let Some(&c) = lexer.peek() {
        if is_digit(c) {
            let digit = c as u128 - b'0' as u128;
            if overflow {
                value.push(c);
                lexer.advance();
            } else {
                match num_value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                    Some(new_val) => {
                        num_value = new_val;
                        value.push(c);
                        lexer.advance();
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

    if overflow {
        lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(value));
        return Some(lexer.make_token(TokenKind::Error("Integer too large".to_string())));
    }

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
            ));
        }
    }

    let cleaned: String = value.chars().filter(|&c| c != '_').collect();
    let num_str = &cleaned;

    if num_str.contains('.') || num_str.contains('e') || num_str.contains('E') {
        match num_str.parse::<f64>() {
            Ok(n) => Some(Token {
                kind: TokenKind::FloatLiteral(n),
                span: lexer.span(),
                literal: Some(Literal::Float(n)),
            }),
            Err(_) => {
                lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(value));
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
                lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(value));
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
            ));
        }
    }

    let cleaned: String = value.chars().filter(|&c| c != '_').collect();
    match cleaned.parse::<f64>() {
        Ok(n) => Some(Token {
            kind: TokenKind::FloatLiteral(n),
            span: Span::new(start_pos, lexer.position()),
            literal: Some(Literal::Float(n)),
        }),
        Err(_) => {
            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidNumber(value));
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
        return scan_multi_line_string(lexer);
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
                if let Some(escaped) = lexer.advance() {
                    match escaped {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        'r' => value.push('\r'),
                        '\\' => value.push('\\'),
                        '"' => value.push('"'),
                        '\'' => value.push('\''),
                        '0' => value.push('\0'),
                        'x' => {
                            // Hexadecimal escape \xFF
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
                            if hex.len() == 2 {
                                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                                    value.push(byte as char);
                                } else {
                                    lexer.error = Some(
                                        crate::frontend::core::lexer::LexError::InvalidEscape {
                                            sequence: format!("\\x{}", hex),
                                        },
                                    );
                                }
                            } else {
                                lexer.error =
                                    Some(crate::frontend::core::lexer::LexError::InvalidEscape {
                                        sequence: format!("\\x{}", hex),
                                    });
                            }
                        }
                        'u' => {
                            // Unicode escape \u{1F600}
                            if lexer.peek() == Some(&'{') {
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
                                if lexer.peek() == Some(&'}') && !hex.is_empty() {
                                    lexer.advance();
                                    if let Ok(codepoint) = u32::from_str_radix(&hex, 16) {
                                        if let Some(ch) = char::from_u32(codepoint) {
                                            value.push(ch);
                                        } else {
                                            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidEscape {
                                                sequence: format!("\\u{{{}}}", hex),
                                            });
                                        }
                                    } else {
                                        lexer.error = Some(
                                            crate::frontend::core::lexer::LexError::InvalidEscape {
                                                sequence: format!("\\u{{{}}}", hex),
                                            },
                                        );
                                    }
                                } else {
                                    lexer.error = Some(
                                        crate::frontend::core::lexer::LexError::InvalidEscape {
                                            sequence: "\\u{".to_string(),
                                        },
                                    );
                                }
                            } else {
                                lexer.error =
                                    Some(crate::frontend::core::lexer::LexError::InvalidEscape {
                                        sequence: "\\u".to_string(),
                                    });
                            }
                        }
                        c => {
                            lexer.error =
                                Some(crate::frontend::core::lexer::LexError::InvalidEscape {
                                    sequence: c.to_string(),
                                });
                        }
                    }
                }
            }
            '\n' => {
                lexer.error = Some(crate::frontend::core::lexer::LexError::UnterminatedString {
                    position: format!("{}:{}", start_pos.line, start_pos.column),
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
    });
    Some(Token {
        kind: TokenKind::Error("Unterminated string".to_string()),
        span: lexer.span(),
        literal: None,
    })
}

/// Scan multi-line string
fn scan_multi_line_string(lexer: &mut super::tokenizer::Lexer<'_>) -> Option<Token> {
    let start_pos = lexer.position();
    let mut value = String::new();

    // The opening """ has already been consumed by scan_string
    // Now we just need to read the content and find the closing """

    while let Some(c) = lexer.advance() {
        // Check for closing """
        if c == '"' {
            // Check if we have two more quotes
            let actual_peek1 = lexer.peek_public().copied();
            let mut temp_clone = lexer.chars_clone();
            let actual_peek2 = temp_clone.nth(1);

            if actual_peek1 == Some('"') && actual_peek2 == Some('"') {
                // Found closing """
                // Clear any previous error and consume the remaining two quotes
                lexer.error = None;
                lexer.advance(); // consume second quote
                lexer.advance(); // consume third quote
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
            } else {
                // This is just a single quote inside the string
                value.push(c);
            }
        } else if c == '\\' {
            // Handle escape sequences
            if let Some(escaped) = lexer.advance() {
                match escaped {
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    '\'' => value.push('\''),
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    _ => {
                        // For unrecognized escape sequences, keep both the backslash and the character
                        value.push('\\');
                        value.push(escaped);
                    }
                }
            }
        } else {
            value.push(c);
        }
    }

    lexer.error = Some(crate::frontend::core::lexer::LexError::UnterminatedString {
        position: format!("{}:{}", start_pos.line, start_pos.column),
    });
    Some(Token {
        kind: TokenKind::Error("Unterminated multi-line string".to_string()),
        span: lexer.span(),
        literal: None,
    })
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
                if let Some(escaped) = lexer.advance() {
                    match escaped {
                        'n' => value.push('\n'),
                        't' => value.push('\t'),
                        'r' => value.push('\r'),
                        '\\' => value.push('\\'),
                        '\'' => value.push('\''),
                        '"' => value.push('"'),
                        '0' => value.push('\0'),
                        'x' => {
                            // Hexadecimal escape \x41
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
                            if hex.len() == 2 {
                                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                                    value.push(byte as char);
                                } else {
                                    lexer.error = Some(
                                        crate::frontend::core::lexer::LexError::InvalidEscape {
                                            sequence: format!("\\x{}", hex),
                                        },
                                    );
                                }
                            } else {
                                lexer.error =
                                    Some(crate::frontend::core::lexer::LexError::InvalidEscape {
                                        sequence: format!("\\x{}", hex),
                                    });
                            }
                        }
                        'u' => {
                            // Unicode escape \u{1F600}
                            if lexer.peek() == Some(&'{') {
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
                                if lexer.peek() == Some(&'}') && !hex.is_empty() {
                                    lexer.advance();
                                    if let Ok(codepoint) = u32::from_str_radix(&hex, 16) {
                                        if let Some(ch) = char::from_u32(codepoint) {
                                            value.push(ch);
                                        } else {
                                            lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidEscape {
                                                sequence: format!("\\u{{{}}}", hex),
                                            });
                                        }
                                    } else {
                                        lexer.error = Some(
                                            crate::frontend::core::lexer::LexError::InvalidEscape {
                                                sequence: format!("\\u{{{}}}", hex),
                                            },
                                        );
                                    }
                                } else {
                                    lexer.error = Some(
                                        crate::frontend::core::lexer::LexError::InvalidEscape {
                                            sequence: "\\u{".to_string(),
                                        },
                                    );
                                }
                            } else {
                                lexer.error =
                                    Some(crate::frontend::core::lexer::LexError::InvalidEscape {
                                        sequence: "\\u".to_string(),
                                    });
                            }
                        }
                        c => value.push(c),
                    }
                }
            }
            '\n' => {
                lexer.error = Some(crate::frontend::core::lexer::LexError::InvalidToken {
                    position: format!("{}:{}", start_pos.line, start_pos.column),
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
