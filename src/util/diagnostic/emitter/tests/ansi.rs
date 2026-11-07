//! ANSI 工具测试 — 基于 check-improvement 设计规范
//!
//! §4.2: ANSI 工具提取（colorize + strip_ansi）

use crate::util::diagnostic::emitter::ansi::{colorize, strip_ansi};

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
