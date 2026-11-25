//! 语法面收口后的 formatter 输出（SPEC syntax.md §2.8 / §3.1 / §3.2.1）。
//!
//! 守的性质：formatter 输出必须落在新面上——
//! - match 分支箭头输出 `->`（§2.8）；
//! - 解构赋值输出括号式 `(a, b) = e`（§3.2.1）；
//! - 同一行的多条语句以 `; ` 分隔（§3.1 的「同行多语句用分号」形态）。

use crate::formatter::{format_source, FormatOptions};

/// 格式化一例输入；失败时给出输入全文。
fn format_or_panic(source: &str) -> String {
    format_source(source, &FormatOptions::default())
        .unwrap_or_else(|e| panic!("用例输入应可格式化，实际失败：{e}\n--- 输入 ---\n{source}"))
}

#[test]
fn test_formatter_emits_dash_arrow_for_match_arms() {
    // Arrange：新面 match 源。
    let source = "m = match 1 {\n    0 -> 0\n    _ -> 1\n}\n";

    // Act：格式化。
    let out = format_or_panic(source);

    // Assert：分支箭头是 `->`，且没有残留 `=>`。
    assert!(out.contains(" -> "), "match 分支必须输出 `->`：\n{out}");
    assert!(!out.contains("=>"), "match 分支不得残留 `=>`：\n{out}");
}

#[test]
fn test_formatter_emits_paren_destructure() {
    // Arrange：括号式解构源（AST 里 names 已无括号，输出面必须补回）。
    let source = "(a, b) = (1, 2)\n";

    // Act：格式化。
    let out = format_or_panic(source);

    // Assert：输出保持 `(a, b) = ...`。
    assert!(
        out.contains("(a, b) = (1, 2)"),
        "解构赋值必须输出括号式：\n{out}"
    );
}

#[test]
fn test_formatter_emits_semicolon_for_same_line_statements() {
    // Arrange：块内同一行的多条语句（§3.1 要求 `;` 分隔）。
    let source = "main: () -> Void = { x = 1; y = 2 }\n";

    // Act：格式化。
    let out = format_or_panic(source);

    // Assert：保持单行且以 `; ` 分隔。
    assert!(
        out.contains("x = 1; y = 2"),
        "同行多语句必须以 `; ` 分隔：\n{out}"
    );
}
