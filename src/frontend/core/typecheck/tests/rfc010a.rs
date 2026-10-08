//! RFC-010a 边界测试 — `return` 的函数边界判定（Script 顶层拒绝）
//!
//! RFC-010a: docs/src/rfc/accepted/010a-tail-expression-and-return.md
//!
//! 测试点：
//! - 规则② 边界条件：`return` 退出**最近的函数边界**；Script 顶层是模块初始化层，
//!   没有函数边界可退——编译期拒绝（E1109），不再静默终止初始化、跳过后续顶层语句
//! - 块值不是函数边界：顶层绑定值块内的 `return` 同样拒绝
//! - 合法位置回归：命名函数体 / lambda 体 / spawn 体内的 `return` 不受影响

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;

/// 辅助函数：解析源代码并类型检查（与 rfc010.rs 同一 harness 形态）
fn check_source(source: &str) -> crate::frontend::core::typecheck::types::TypeCheckResult {
    use crate::util::diagnostic::Diagnostic;
    let tokens = match tokenize(source) {
        Ok(t) => t,
        Err(e) => {
            return crate::frontend::core::typecheck::types::TypeCheckResult {
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
            let diag = crate::util::diagnostic::ErrorCodeDefinition::invalid_syntax(&format!(
                "解析错误: {:?}",
                result.errors
            ))
            .build();
            return crate::frontend::core::typecheck::types::TypeCheckResult {
                diagnostics: vec![diag],
                ..Default::default()
            };
        }
        result => result.module,
    };
    let mut checker = TypeChecker::new("test");
    checker.check_module(&module)
}

/// 规范：Script 顶层 `return`（模块初始化层，无函数边界）应报 E1109
///
/// 预期行为：
/// - 编译期拒绝，而不是运行期静默终止模块初始化、跳过后续顶层语句
#[test]
fn test_top_level_return_reports_e1109() {
    // Arrange：Script 顶层直接 return，后随一条断言（修复前该断言被静默跳过）
    let source = r#"
return 5
assert_value: Int = 1
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：必须出现 E1109（return 出现在函数外）
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1109"),
        "Script 顶层 return 应报 E1109，实际诊断: {:?}",
        result.diagnostics
    );
}

/// 规范：顶层绑定值块内的 `return` 同样报 E1109（块值不是函数边界）
#[test]
fn test_return_in_top_level_block_value_reports_e1109() {
    // Arrange：块值出现在顶层绑定右侧——求值发生在模块初始化层
    let source = r#"
x: Int = { return 5 }
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：必须出现 E1109
    assert!(
        result.diagnostics.iter().any(|d| d.code == "E1109"),
        "顶层块值内的 return 应报 E1109，实际诊断: {:?}",
        result.diagnostics
    );
}

/// 规范：命名函数体内的 `return` 合法（E1109 不得误伤）
#[test]
fn test_return_inside_named_function_body_allowed() {
    // Arrange：函数体内 return 是规则② 的正用形态
    let source = r#"
f: (n: Int) -> Int = {
    if n > 0 { return n }
    0
}
main: () -> Void = {}
main()
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：不得出现 E1109
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E1109"),
        "函数体内 return 不应报 E1109，实际诊断: {:?}",
        result.diagnostics
    );
}

/// 规范：lambda 体内的 `return` 合法（lambda 是函数边界）
#[test]
fn test_return_inside_lambda_body_allowed() {
    // Arrange：箭头 lambda 体（解析为含单条 return 语句的块）
    let source = r#"
f = (n: Int) => { return n + 1 }
a: Int = f(1)
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：不得出现 E1109
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E1109"),
        "lambda 体内 return 不应报 E1109，实际诊断: {:?}",
        result.diagnostics
    );
}

/// 规范：spawn 体是函数边界，体内 `return` 合法（既有已验证行为不得回归）
#[test]
fn test_return_inside_spawn_body_allowed() {
    // Arrange：spawn 引入新函数边界（RFC-010 §spawn：`r = spawn { return 3 + 4 }` 得 7）
    let source = r#"
f: () -> Int = {
    r = spawn { return 3 + 4 }
    r
}
main: () -> Void = {}
main()
"#;

    // Act：类型检查
    let result = check_source(source);

    // Assert：不得出现 E1109
    assert!(
        !result.diagnostics.iter().any(|d| d.code == "E1109"),
        "spawn 体内 return 不应报 E1109，实际诊断: {:?}",
        result.diagnostics
    );
}
