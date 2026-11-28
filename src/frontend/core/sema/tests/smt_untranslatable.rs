//! F10（B.3/B.4 复审残留）：SMT 翻译失败 ≠ 反例（RFC-027 §9 / 提交信息）
//!
//! 背景：`translate_constraint` 遇到不可翻译项时返回**空命令表**。空表送进求解器
//! 等价于「没有任何断言」——check-sat 恒为 sat，于是被判成**反例**（E4018「谓词被
//! 违反」+ 伪造的反例赋值），而真实情况是「翻译不了、无法判断」（Unproven ⇒ E2031）。
//!
//! 本文件的判据：不可翻译的假设/约束不得产出伪造反例；可翻译的真反例照旧报 E4018。

use crate::frontend::compiler::Compiler;

/// 不可翻译的**分支守卫**（位运算不是 SMT 项）叠加无法证明的义务：不得伪造反例。
///
/// 守卫进 Γ（refined_walk_branch_with_guard 的假设栈）后，SMT 面既不能把它当
/// 背景断言、也不能假装它不存在——正确结论是「无法判断」（E2031）。
const UNTRANSLATABLE_ASSUMPTION: &str = r#"use std.assert

Pos: (x: Int) -> Type = { x > 0 }

f: (x: Pos(x)) -> Int = { 3 }

g: (y: Int) -> Int = {
    if y & 3 == 0 {
        f(y)
    } else {
        0
    }
}

main: () -> Void = {
    r: Int = g(4)
}
"#;

/// 可翻译的**真反例**：保持型负控——修正不得把 SMT 反例一并降级。
const TRANSLATABLE_COUNTEREXAMPLE: &str = r#"use std.assert

Pos: (x: Int) -> Type = { x > 0 }

f: (x: Pos(x)) -> Int = { 3 }

g: (y: Int) -> Int = f(y)

main: () -> Void = {
    r: Int = g(0)
}
"#;

/// 编译一段 .yx 源码，返回首个结构化错误的 (码, 消息)。
fn compile_error(source: &str) -> (String, String) {
    let mut compiler = Compiler::new();
    let error = compiler
        .compile("smt_untranslatable.yx", source)
        .expect_err("夹具应被编译期拒绝");
    let diagnostic = error
        .diagnostic()
        .expect("拒绝必须携带结构化诊断（不是自由文本）");
    (
        diagnostic.code.as_str().to_string(),
        diagnostic.message.clone(),
    )
}

/// 不可翻译的假设使 SMT 面无法判断 ⇒ Unproven（E2031），不得是「伪造反例」（E4018）。
#[test]
fn test_untranslatable_smt_assumption_reports_unproven_not_disproved() {
    // Arrange
    let source = UNTRANSLATABLE_ASSUMPTION;

    // Act
    let (code, message) = compile_error(source);

    // Assert
    assert_eq!(
        code, "E2031",
        "不可翻译的假设不得被判成反例（E4018 = 伪造反例）；实际: {message}"
    );
    assert!(
        !message.contains("Counterexample"),
        "E2031 不得携带反例段；实际: {message}"
    );
}

/// 保持型负控：可翻译且确实为假的条件仍必须给出真反例（E4018）。
#[test]
fn test_translatable_false_obligation_still_reports_counterexample() {
    // Arrange
    let source = TRANSLATABLE_COUNTEREXAMPLE;

    // Act
    let (code, message) = compile_error(source);

    // Assert
    assert_eq!(
        code, "E4018",
        "可翻译的真反例必须保持 E4018；实际: {message}"
    );
}
