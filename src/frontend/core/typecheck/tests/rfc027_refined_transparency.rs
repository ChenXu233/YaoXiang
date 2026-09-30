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

use crate::frontend::core::types::const_data::ConstValue;
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

/// 调用点实参对形参精化的义务：`check_module` 生成证明调用（RFC-027 §3.4）。
///
/// `check_module` 不执行证明函数（由 `pipeline` 在其后跑），故此处断言
/// **义务已生成**且带着实参值。执行后谓词返回 false 即 E4018，端到端见
/// `tests/yaoxiang/06-compile-errors/refined_param_arg_violates_err.yx`。
#[test]
fn test_call_arg_refinement_generates_proof_obligation() {
    // Arrange — f(-5) 违反 b >= 0
    let source = {
        let f = "f: (b: NonNegative(b)) -> Int = { 0 }";
        let m = "main: () -> Void = {\n    z: Int = f(-5)\n}";
        format!("{NON_NEGATIVE}{f}\n{m}")
    };

    // Act
    let result = check_source(&source);

    // Assert
    assert!(
        result
            .proof_calls
            .iter()
            .any(|c| c.func_name == "NonNegative"
                && c.args.iter().any(|v| matches!(v, ConstValue::Int(-5)))),
        "调用点应生成形参精化的证明义务并携带实参值 Int(-5)；实际: {:?}",
        result.proof_calls
    );
}

/// **Tripwire** —— 实参**无静态值**时不生成义务（RFC-027 §3.4 的强半场未启用）。
///
/// §3.4 要求「无静态证据即拒绝」：`divide(x, y)`（实参无界、无精化标注）也应
/// 报错。本版只做证伪（实参可折叠时才判），故此类调用放过——它会连带拒掉
/// 大量合法代码，需单独决策后另行落地。本用例把「放过」钉住：谁启用强半场，
/// 它会变红，提醒把断言改成「报错」。
#[test]
fn test_call_arg_without_static_value_generates_no_obligation() {
    // Arrange — 实参是形参（无静态值）
    let source = {
        let g = "g: (x: Int) -> Int = { x }";
        let f = "f: (b: NonNegative(b)) -> Int = { 0 }";
        let m = "main: () -> Void = {\n    z: Int = f(g(1))\n}";
        format!("{NON_NEGATIVE}{g}\n{f}\n{m}")
    };

    // Act
    let result = check_source(&source);

    // Assert — 当前无义务；强半场启用后应改为断言报错
    assert!(
        !result
            .proof_calls
            .iter()
            .any(|c| c.func_name == "NonNegative"),
        "实参非编译期常量时本版不生成义务；若此处变红说明已落地静态证据要求，\
         请把断言改为「报错」。实际: {:?}",
        result.proof_calls
    );
}
