/// ANSI 颜色样式
pub fn colorize(
    style: &str,
    text: &str,
) -> String {
    match style {
        "error" => format!("\x1b[31m{}\x1b[0m", text),
        "warning" => format!("\x1b[33m{}\x1b[0m", text),
        "info" => format!("\x1b[34m{}\x1b[0m", text),
        "hint" => format!("\x1b[36m{}\x1b[0m", text),
        "bold" => format!("\x1b[1m{}\x1b[0m", text),
        "muted" => format!("\x1b[90m{}\x1b[0m", text),
        _ => text.to_string(),
    }
}

/// 移除所有 ANSI 转义序列
pub fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_alphabetic() || next == 'm' {
                        chars.next();
                        break;
                    }
                    chars.next();
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colorize_error() {
        let result = colorize("error", "hello");
        assert!(result.contains("\x1b[31m"));
        assert!(result.contains("hello"));
        assert!(result.contains("\x1b[0m"));
    }

    #[test]
    fn test_colorize_unknown_style() {
        let result = colorize("unknown", "hello");
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_strip_ansi_plain() {
        assert_eq!(strip_ansi("hello"), "hello");
    }

    #[test]
    fn test_strip_ansi_with_color() {
        let colored = format!("\x1b[31m{}\x1b[0m", "error");
        assert_eq!(strip_ansi(&colored), "error");
    }

    #[test]
    fn test_strip_ansi_bold() {
        let bold = format!("\x1b[1m{}\x1b[0m", "bold text");
        assert_eq!(strip_ansi(&bold), "bold text");
    }

    #[test]
    fn test_strip_ansi_empty() {
        assert_eq!(strip_ansi(""), "");
    }
}
