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

/// RFC-039 D53 / SPEC type-system §3.5：未命名带型参数（裸类型项）必须原样输出——
/// 名字槽是空串，写成 `: Int` 会产出无法再解析的源码（idempotency 会当场红）。
#[test]
fn test_formatter_keeps_bare_type_params() {
    // Arrange：声明签名里两种形参项并存（未命名裸类型 / 具名）。
    let source = "\
        Point: Type = { x: Int }\n\
        Surface: Type = { w: Int }\n\
        add: (Int, Int) -> Int = (a, b) => a + b\n\
        mk: (p: Point, Surface) -> Int = (p, s) => p.x\n";

    // Act
    let out = format_or_panic(source);

    // Assert：裸类型项只写类型；具名项保持 `名: 类型`。
    assert!(
        out.contains("add: (Int, Int) -> Int"),
        "未命名带型参数必须原样输出为 `(Int, Int)`：\n{out}"
    );
    assert!(
        out.contains("mk: (p: Point, Surface) -> Int"),
        "混合形参必须按位保留（具名 `p: Point` + 裸类型 `Surface`）：\n{out}"
    );
    assert!(
        !out.contains("(: "),
        "空名字槽不得渲染成 `: 类型`（会产生无法再解析的源码）：\n{out}"
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
