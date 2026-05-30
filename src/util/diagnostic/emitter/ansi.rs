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
///
/// 处理 CSI 序列（`ESC [` ... 终止字节）和 OSC 序列（`ESC ]` ... `BEL` 或 `ESC \`）。
/// 对于本诊断系统生成的 SGR 颜色码（`ESC [ ... m`），此实现完全足够。
pub fn strip_ansi(s: &str) -> String {
    // 快速路径：无 ESC 字节则直接返回
    if !s.contains('\x1b') {
        return s.to_string();
    }

    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some(&'[') => {
                    // CSI 序列：ESC [ <params> <final byte>
                    chars.next();
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        // 终止字节：0x40-0x7E（@ 到 ~）
                        if (0x40..=0x7E).contains(&(next as u32)) {
                            break;
                        }
                    }
                }
                Some(&']') => {
                    // OSC 序列：ESC ] ... BEL(0x07) 或 ESC \
                    chars.next();
                    loop {
                        match chars.next() {
                            Some('\x07') => break, // BEL 终止
                            Some('\x1b') if chars.peek() == Some(&'\\') => {
                                chars.next();
                                break; // ESC \ 终止
                            }
                            None => break,
                            _ => {}
                        }
                    }
                }
                Some(_) => {
                    // 其他两字节序列（如 ESC D, ESC M）：跳过 ESC + 下一字节
                    chars.next();
                }
                None => {} // 孤立 ESC，跳过
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

    #[test]
    fn test_strip_ansi_multi_param() {
        // 多参数 CSI 序列
        let s = "\x1b[1;31;42mtext\x1b[0m";
        assert_eq!(strip_ansi(s), "text");
    }

    #[test]
    fn test_strip_ansi_osc_title() {
        // OSC 序列（终端标题）
        let s = "\x1b]0;my title\x07hello";
        assert_eq!(strip_ansi(s), "hello");
    }

    #[test]
    fn test_strip_ansi_osc_st() {
        // OSC 序列以 ESC \ 终止
        let s = "\x1b]0;title\x1b\\hello";
        assert_eq!(strip_ansi(s), "hello");
    }

    #[test]
    fn test_strip_ansi_cursor_move() {
        // 光标移动序列
        let s = "\x1b[2J\x1b[Hhello";
        assert_eq!(strip_ansi(s), "hello");
    }

    #[test]
    fn test_strip_ansi_no_esc() {
        // 无 ESC 字节的快速路径
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn test_strip_ansi_multiple_sequences() {
        let s = "\x1b[1m\x1b[31merror\x1b[0m\x1b[0m";
        assert_eq!(strip_ansi(s), "error");
    }
}
