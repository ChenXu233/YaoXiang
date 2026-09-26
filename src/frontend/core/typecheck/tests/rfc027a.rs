//! RFC-027a 终止显式测度 — 类型解析测试
//!
//! 规范来源: RFC-027a `docs/src/design/rfc/review/027a-termination-explicit-measure.md`
//! 宿主语义: RFC-027 §6.9（`Terminates` 内置谓词，与 `Int`/`Never` 同属核心原语）
//!
//! 本文件只覆盖 **T1**：`Terminates` 进类型解析器（一元形态）。
//! 测度提取（T2）、义务生成（T3）、SMT 判定（T4）见后续任务。

use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::types::TypeCheckResult;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::util::diagnostic::Diagnostic;

/// 解析源代码并类型检查
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

/// 断言整份源码**零诊断**。
///
/// 为何用「零诊断」而非「无某码」：早先版本只断言「不报 E1003」，
/// 于是 `Terminates(b)` 在更下游炸出 E1092 时测试仍然绿——弱断言掩盖了
/// 真实失败。`Terminates` 标注不应产生**任何**诊断。
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

/// RFC-027a §锚点 —— 函数返回类型位的 `Terminates(b)` 必须零诊断通过
#[test]
fn test_terminates_in_return_position_is_clean() {
    // Arrange
    let src = r#"
gcd: (a: Int, b: Int) -> Terminates(b) = {
    if b == 0 { return a }
    return gcd(b, a % b)
}
"#;

    // Act & Assert
    assert_clean(
        src,
        "Terminates(b) 在返回类型位：测度 b 是形参表达式，须原样保留为编译期测度",
    );
}

/// RFC-027a §锚点 —— 测度形参不得被分类为 const 泛型参数
///
/// 根因回归（T1 调查实证）：`classify_generic_params` 把「类型名 ∈
/// CONST_PARAM_TYPES」的形参（`b: Int` 里的 Int）当作 const 泛型**候选**；
/// 再用 `collect_used_in_type` 扫返回位，一旦测度里出现该形参名（`Terminates(b)`
/// 的 `b`）即判定「被当作类型引用」→ 建立 const binder → **把 `b` 替换成 `Int(64)`**。
/// 测度表达式因此在到达终止检查层之前就丢了，报 E1092。
///
/// 本用例用**非 `Terminates`** 的精化谓词复现同一根因，证明这不是 `Terminates`
/// 的特例，而是签名位「谓词实参引用形参」的通用缺陷。
#[test]
fn test_predicate_arg_referencing_param_is_clean() {
    // Arrange — `n` 是形参名，同时也是精化谓词的实参（求值到 n，非常量）
    let src = r#"
IsPositive: (x: Int) -> Type = { x > 0 }
f: (n: Int) -> IsPositive(n) = {
    return n
}
"#;

    // Act & Assert — 测度/精化实参是形参引用，应作为符号表达式保留
    assert_clean(
        src,
        "IsPositive(n)：精化实参 n 是形参引用，不得被当作 const 泛型参数替换为 Int",
    );
}

/// RFC-027a §锚点 —— 绑定类型位的 `Terminates(m)` 解析可用
///
/// 循环形态的书写位。注意：循环的**端到端**未落地——`while` 恒为 `Void`
/// （计划 §一 D5），故本用例改用普通值绑定，只验证 `Terminates` 标注位本身
/// 可解析、测度可保留。
#[test]
fn test_terminates_in_binding_position_is_clean() {
    // Arrange
    let src = r#"
main: () -> Void = {
    mut n: Int = 10
    acc: Terminates(n) = 0
}
"#;

    // Act & Assert
    assert_clean(src, "Terminates(n) 在绑定类型位：测度 n 须原样保留");
}

/// RFC-027a §锚点 —— `Terminates` 不得放宽未知类型检查
///
/// 回归性质：#371/#372 的 E1003 校验必须照常工作，`Terminates` 特判不得
/// 顺带把任意未知名也放行。
#[test]
fn test_other_unknown_type_still_reports_e1003() {
    // Arrange
    let src = r#"
f: (a: Int) -> BogusType = {
    return a
}
"#;

    // Act
    let result = check_source(src);

    // Assert
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1003"),
        "未知名 BogusType 应照常报 E1003。实际: {:#?}",
        result.diagnostics
    );
}

/// RFC-027a —— `Terminates` 元数错误须报错，不得静默忽略
///
/// 二元形态 `Terminates(fn_ty, m)` 尚为 park（D1），多余实参不得被丢弃。
#[test]
fn test_terminates_arity_mismatch_is_rejected() {
    // Arrange
    let src = r#"
f: (a: Int) -> Terminates(1, 2) = {
    return a
}
"#;

    // Act
    let result = check_source(src);

    // Assert — 必须报错（E1093 精化实参个数不匹配）
    assert!(
        !result.diagnostics.is_empty(),
        "Terminates(1, 2) 元数错误应报错，实际零诊断"
    );
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1093"),
        "Terminates 元数错误应走 E1093（精化实参个数不匹配）。实际: {:#?}",
        result.diagnostics
    );
}
