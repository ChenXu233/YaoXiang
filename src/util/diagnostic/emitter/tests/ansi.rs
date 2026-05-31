//! ANSI 工具测试 — 基于 check-improvement 设计规范
//!
//! §4.2: ANSI 工具提取（colorize + strip_ansi）

use crate::util::diagnostic::emitter::ansi::{colorize, strip_ansi};

#[test]
fn test_ansi_colorize_error_style() {
    let result = colorize("error", "hello");
    assert!(
        result.contains("\x1b[31m"),
        "error style should use red (31m)"
    );
    assert!(result.contains("hello"));
    assert!(result.contains("\x1b[0m"), "should reset after color");
}

#[test]
fn test_ansi_colorize_unknown_style_passthrough() {
    let result = colorize("unknown", "hello");
    assert_eq!(
        result, "hello",
        "unknown style should pass through unchanged"
    );
}

#[test]
fn test_strip_ansi_plain_text_unchanged() {
    assert_eq!(strip_ansi("hello"), "hello");
}

#[test]
fn test_strip_ansi_removes_color_codes() {
    let colored = format!("\x1b[31m{}\x1b[0m", "error");
    assert_eq!(strip_ansi(&colored), "error");
}

#[test]
fn test_strip_ansi_removes_bold() {
    let bold = format!("\x1b[1m{}\x1b[0m", "bold text");
    assert_eq!(strip_ansi(&bold), "bold text");
}

#[test]
fn test_strip_ansi_empty_string() {
    assert_eq!(strip_ansi(""), "");
}

#[test]
fn test_strip_ansi_multi_param_csi() {
    let s = "\x1b[1;31;42mtext\x1b[0m";
    assert_eq!(strip_ansi(s), "text");
}

#[test]
fn test_strip_ansi_osc_with_bel_terminator() {
    let s = "\x1b]0;my title\x07hello";
    assert_eq!(strip_ansi(s), "hello");
}

#[test]
fn test_strip_ansi_osc_with_esc_backslash_terminator() {
    let s = "\x1b]0;title\x1b\\hello";
    assert_eq!(strip_ansi(s), "hello");
}

#[test]
fn test_strip_ansi_cursor_move_sequences() {
    let s = "\x1b[2J\x1b[Hhello";
    assert_eq!(strip_ansi(s), "hello");
}

#[test]
fn test_strip_ansi_fast_path_no_esc() {
    assert_eq!(strip_ansi("plain text"), "plain text");
}

#[test]
fn test_strip_ansi_multiple_consecutive_sequences() {
    let s = "\x1b[1m\x1b[31merror\x1b[0m\x1b[0m";
    assert_eq!(strip_ansi(s), "error");
}
