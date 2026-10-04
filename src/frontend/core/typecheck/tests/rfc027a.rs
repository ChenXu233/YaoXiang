//! RFC-027a 终止显式测度 — 类型解析测试
//!
//! 规范来源: RFC-027a `docs/src/design/rfc/review/027a-termination-explicit-measure.md`
//! 宿主语义: RFC-027 §6.9（`Terminates` 内置谓词，与 `Int`/`Never` 同属核心原语）
//!
//! 本文件覆盖 **T1**（`Terminates` 进类型解析器）、**T2**（测度提取）。
//! 义务生成（T3）、SMT 判定（T4）见后续任务。

use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::typecheck::types::TypeCheckResult;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::types::const_data::{BinOp, ConstExpr};
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
    //
    // 形参位带 `IsPositive(n)`：返回位精化现已按 RFC-027 §3 在 `return` 点验证
    //（后置条件），断言 `n > 0` 靠这条前置条件得证。若去掉它，`n = 0` 时返回
    // 类型为空，E4018 会（正确地）报出——本用例要钉的是 const 泛型误分类，
    // 不是后置条件，故源码必须良构。
    let src = r#"
IsPositive: (x: Int) -> Type = { x > 0 }
f: (n: IsPositive(n)) -> IsPositive(n) = {
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
/// 本用例只验证 `Terminates` 标注位本身可解析、测度可保留，用普通值绑定。
///
/// 循环形态（`acc: Terminates(n - i) = while i < n { … }`）的**值类型**已通：
/// `while` 现按 spec §2.9「所有 `{}` 块的值由尾表达式给出」取循环体块类型，
/// 不再恒为 `Void`。但循环的**回边测度义务**尚未生成，故循环形态仍非端到端。
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

// ==================== T2：测度提取 ====================
//
// 提取**从 AST 走**，不走已解析的 `MonoType`：`MonoType::from` 是有损转换，
// T1 调查中它正是把测度表达式丢掉的路径（`Terminates(b)` 的 `b` 被解析成
// `Int(64)`）。AST 是测度的无损来源。
//
// 测度实参在 AST 里有三种形态（探针实测）：
//   `Terminates(b)`     → `Type::Name { name: "b" }`
//   `Terminates(n - i)` → `Type::ConstExpr(Expr::BinOp { .. })`
//   `Terminates(f(n))`  → `Type::Generic { name: "f", args }`  ← 非测度表达式，暂不支持

/// 提取源码中所有 `Terminates(m)` 标注的测度（键 = 被标注的绑定/函数名）。
fn measures_of(source: &str) -> std::collections::HashMap<String, ConstExpr> {
    let tokens = tokenize(source).expect("词法分析应成功");
    let parsed = parse(&tokens);
    assert!(
        !parsed.has_errors,
        "解析应无错误，实际: {:?}",
        parsed.errors
    );
    TypeChecker::new("test").collect_termination_measures(&parsed.module)
}

/// RFC-027a §2.1 —— 测度写在**函数返回类型位**，须提取为参数表达式并挂在函数名下。
#[test]
fn test_measure_extracted_from_return_position() {
    // Arrange
    let source = "gcd: (a: Int, b: Int) -> Terminates(b) = { 1 }";

    // Act
    let measures = measures_of(source);

    // Assert
    assert_eq!(
        measures.get("gcd"),
        Some(&ConstExpr::NamedVar("b".to_string())),
        "返回类型位的 Terminates(b) 应提取出测度 NamedVar(b) 并挂到函数名 gcd 上；\
         实际提取结果: {measures:?}"
    );
    assert_eq!(
        measures.len(),
        1,
        "只应提取到一个测度（gcd），实际: {measures:?}"
    );
}

/// RFC-027a §2.1 —— 测度写在**变量绑定位**，须提取并挂在绑定名下。
#[test]
fn test_measure_extracted_from_binding_position() {
    // Arrange
    let source = "acc: Terminates(n) = 0";

    // Act
    let measures = measures_of(source);

    // Assert
    assert_eq!(
        measures.get("acc"),
        Some(&ConstExpr::NamedVar("n".to_string())),
        "绑定位的 Terminates(n) 应提取出测度 NamedVar(n) 并挂到绑定名 acc 上；\
         实际提取结果: {measures:?}"
    );
}

/// RFC-027a §2.3 —— 测度可以是**复合表达式**（`n - i`），不得退化成单变量。
///
/// 这是本任务的关键断言：早先设想把测度存成字符串或变量名会在此处把
/// `n - i` 压成 `n`，SMT 拿到的测度就错了。
#[test]
fn test_measure_extracted_compound_expression() {
    // Arrange
    let source = "acc: Terminates(n - i) = 0";

    // Act
    let measures = measures_of(source);

    // Assert
    let expected = ConstExpr::BinOp {
        op: BinOp::Sub,
        left: Box::new(ConstExpr::NamedVar("n".to_string())),
        right: Box::new(ConstExpr::NamedVar("i".to_string())),
    };
    assert_eq!(
        measures.get("acc"),
        Some(&expected),
        "复合测度 n - i 应被完整提取为 BinOp(Sub, n, i)，不得压成单变量；\
         实际提取结果: {measures:?}"
    );
}

/// RFC-027a §2.1 —— 测度写在**嵌套函数体内**的绑定位，同样须被提取。
#[test]
fn test_measure_found_in_nested_function_body() {
    // Arrange
    let source = "main: () -> Void = { acc: Terminates(n) = 0 }";

    // Act
    let measures = measures_of(source);

    // Assert
    assert_eq!(
        measures.get("acc"),
        Some(&ConstExpr::NamedVar("n".to_string())),
        "函数体内的 Terminates 绑定位应被递归提取；实际提取结果: {measures:?}"
    );
}

/// 回归 —— 普通类型标注不得产生测度（防止提取器误吞无关注解）。
#[test]
fn test_plain_annotation_yields_no_measure() {
    // Arrange
    let source = "f: (a: Int) -> Int = { a }\nx: Int = 1";

    // Act
    let measures = measures_of(source);

    // Assert
    assert!(
        measures.is_empty(),
        "无 Terminates 标注时不应提取出任何测度，实际: {measures:?}"
    );
}
