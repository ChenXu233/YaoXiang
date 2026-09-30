//! RFC-027 §6.1 精化类型的**透明性**测试
//!
//! 精化类型 `{x: T | p(x)}` 是**编译期约束**，不是独立的运行时类型：一个
//! `NonNegative(b)` 的值**就是**一个 `Int` 的值，凡接受 `Int` 处皆应接受它。
//! 本节钉住这条基础规则——「精化类型可当它的基类型用」。
//!
//! 背景：绑定位的同类规则已落地（`c280ea86`，语料
//! `tests/yaoxiang/02-type-system/refined_variable_uses_base_type.yx`），但
//! **形参位不成立**：`b: NonNegative(b)` 在体内是名义类型，于是
//! 1) 体内 `b` 当值用报 E1012/E1002，2) 任何调用方传入基类型报 E1002
//!    （连合法的 `f(5)` 都被拒——不是谓词校验，是个坏类型）。
//!
//! 本文件只断言 `check_module` 能看到的东西。证明调用（E4018）由 `pipeline.rs`
//! 在 `check_module` **之后**执行，故精化**违反**的用例在 `.yx` 层，见
//! `tests/yaoxiang/06-compile-errors/refined_annotation_literal_violates_err.yx`。

use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::types::TypeCheckResult;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::util::diagnostic::Diagnostic;

/// 下界谓词：`x >= 0`。源码内定义（语言里没有内建的非负类型）。
const NON_NEGATIVE: &str = "NonNegative: (x: Int) -> Type = { x >= 0 }\n";

/// 解析并类型检查
fn check_source(source: &str) -> TypeCheckResult {
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(e) => {
            return TypeCheckResult {
                diagnostics: vec![Diagnostic::error(
                    "E0001".to_string(),
                    format!("词法错误: {}", e),
                    String::new(),
                    None,
                )],
                ..Default::default()
            };
        }
    };
    let module = match parse(&tokens) {
        result if result.has_errors => {
            return TypeCheckResult {
                diagnostics: vec![Diagnostic::error(
                    "E0001".to_string(),
                    format!("解析错误: {:?}", result.errors),
                    String::new(),
                    None,
                )],
                ..Default::default()
            };
        }
        result => result.module,
    };
    let mut checker = TypeChecker::new("test");
    checker.check_module(&module)
}

/// 断言零诊断。
///
/// 断言「零诊断」而非「无某码」：E1002 / E1012 是同一个病的两个出口，
/// 只堵一个会让测试在另一处漏绿。
fn assert_clean(
    source: &str,
    why: &str,
) -> TypeCheckResult {
    let result = check_source(source);
    assert!(
        result.diagnostics.is_empty(),
        "{why}；期望零诊断，实际: {:#?}",
        result.diagnostics
    );
    result
}

/// 返回位：精化形参当作基类型返回。
#[test]
fn test_param_refinement_transparent_in_return_position() {
    // Arrange
    let source = format!("{NON_NEGATIVE}f: (b: NonNegative(b)) -> Int = {{ b }}");

    // Act / Assert
    assert_clean(
        &source,
        "精化形参 b: NonNegative(b) 的值就是一个 Int，直接返回它不应报 E1012",
    );
}

/// 操作数位：精化形参参与算术。
#[test]
fn test_param_refinement_transparent_in_operator() {
    // Arrange
    let source = format!("{NON_NEGATIVE}f: (b: NonNegative(b)) -> Int = {{ b + 1 }}");

    // Act / Assert
    assert_clean(&source, "精化形参参与算术不应报 E1002（期望基类型 Int）");
}

/// 相等比较：精化形参与基类型比较。
#[test]
fn test_param_refinement_transparent_in_equality() {
    // Arrange
    let source = {
        let f = "f: (b: NonNegative(b)) -> Int = { if b == 0 { 1 } else { 0 } }";
        format!("{NON_NEGATIVE}{f}")
    };

    // Act / Assert
    assert_clean(
        &source,
        "精化形参与基类型比较不应报 E1101（refined 未实现 Equal）",
    );
}

/// 调用实参位：精化形参传给接受基类型的函数。
#[test]
fn test_param_refinement_transparent_as_call_argument() {
    // Arrange
    let source = {
        let f = "g: (x: Int) -> Int = { x }";
        let h = "h: (b: NonNegative(b)) -> Int = { g(b) }";
        format!("{NON_NEGATIVE}{f}\n{h}")
    };

    // Act / Assert
    assert_clean(&source, "精化形参传入接受 Int 的函数不应报类型不符");
}

/// 调用方一侧：以基类型实参调用精化形参的函数。
///
/// 这是修复前最刺眼的一处——**合法的** `f(5)` 被拒。
#[test]
fn test_refined_fn_accepts_base_type_argument() {
    // Arrange
    let source = {
        let f = "f: (b: NonNegative(b)) -> Int = { b }";
        let m = "main: () -> Void = {\n    z: Int = f(5)\n}";
        format!("{NON_NEGATIVE}{f}\n{m}")
    };

    // Act / Assert
    assert_clean(
        &source,
        "精化形参的函数应接受基类型实参；修复前连 f(5) 都报 E1002",
    );
}

/// **Tripwire** —— 形参精化在**调用点**尚未被语义校验（RFC-027 待办）。
///
/// 透明化之前，`b: NonNegative(b)` 在调用点按**名义**匹配，报
/// 「期望 `NonNegative(b)`，实得 `int64`」——那不是谓词校验，而是拒绝
/// **所有**调用方（连合法的 `f(5)` 都被拒）。故修前「全拒」、修后「全收」，
/// 两态都不是校验。
///
/// 本用例把「全收」钉住，使缺口可见而非静默：谁实现了调用点谓词校验，
/// 本用例就会变红，提醒他把断言改成「诊断含 E4018」而非直接删掉。
#[test]
fn test_param_refinement_not_enforced_at_call_site_currently() {
    // Arrange — f(-5) 违反 b >= 0
    let source = {
        let f = "f: (b: NonNegative(b)) -> Int = { 0 }";
        let m = "main: () -> Void = {\n    z: Int = f(-5)\n}";
        format!("{NON_NEGATIVE}{f}\n{m}")
    };

    // Act
    let result = check_source(&source);

    // Assert — 当前全收（无诊断）；实现调用点校验后应改为断言 E4018
    assert!(
        result.diagnostics.is_empty(),
        "形参精化调用点校验尚未实现，本用例预期「无诊断」；若此处变红说明校验已落地，\
         请把断言改为「诊断含 E4018」。实际: {:#?}",
        result.diagnostics
    );
}
