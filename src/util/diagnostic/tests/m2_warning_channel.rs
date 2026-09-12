//! #321 M2 警告通道测试
//!
//! 验证：W 前缀码缺省 Warning severity、ErrorCollector 按严重级别分流、
//! 未使用导入（W1003）经 typecheck use elaboration 检出后走 warnings 通道
//! （Warning severity、不混入错误 diagnostics）。

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::util::diagnostic::{ErrorCollector, ErrorCodeDefinition, Severity};

#[test]
fn test_w_prefix_codes_default_to_warning_severity() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // W 前缀码：builder 未显式指定 severity 时缺省 Warning（#321 M2）
    let diag = ErrorCodeDefinition::find("W1001")
        .expect("W1001 registered")
        .builder()
        .param("name", "foo")
        .build();
    assert_eq!(diag.severity, Severity::Warning);

    // E 前缀码：缺省仍为 Error
    let diag = ErrorCodeDefinition::find("E1001")
        .expect("E1001 registered")
        .builder()
        .param("name", "x")
        .build();
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn test_explicit_severity_overrides_w_prefix_default() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    // 显式指定的 severity 优先于 W 前缀缺省推导
    let diag = ErrorCodeDefinition::find("W1001")
        .expect("W1001 registered")
        .builder()
        .param("name", "foo")
        .severity(Severity::Error)
        .build();
    assert_eq!(diag.severity, Severity::Error);
}

#[test]
fn test_error_collector_splits_errors_and_warnings() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut collector = ErrorCollector::new();

    let warning = ErrorCodeDefinition::find("W1001")
        .expect("W1001 registered")
        .builder()
        .param("name", "foo")
        .build();
    collector.add_error(warning);
    // 仅警告：不构成错误，has_warnings 为真
    assert!(!collector.has_errors());
    assert!(collector.has_warnings());
    assert_eq!(collector.error_count(), 0);

    let error = ErrorCodeDefinition::find("E1001")
        .expect("E1001 registered")
        .builder()
        .param("name", "x")
        .build();
    collector.add_error(error);
    // 错误出现后 has_errors 为真，error_count 只计 Error
    assert!(collector.has_errors());
    assert!(collector.has_warnings());
    assert_eq!(collector.error_count(), 1);
}

/// 辅助：源码 → 类型检查结果（语法必须正确，否则测试本身有错）
fn check_source(source: &str) -> crate::frontend::core::typecheck::types::TypeCheckResult {
    let tokens = tokenize(source).expect("测试源码词法不应失败");
    let parse_result = parse(&tokens);
    assert!(
        !parse_result.has_errors,
        "测试源码语法必须正确（否则无法区分 bug 与测试写错）: {:?}",
        parse_result.errors
    );

    let mut checker = TypeChecker::new("test");
    checker.check_module(&parse_result.module)
}

#[test]
fn test_unused_named_import_reports_w1003_via_warnings_channel() {
    // Arrange: 具名导入（use std.io.{print}）且 print 未被引用 → W1003
    let source = r#"
use std.io.{print}
main = {
    x = 1
    x
}
"#;

    // Act
    let result = check_source(source);

    // Assert: 警告在 warnings 通道（Warning severity），错误通道保持干净
    let w1003 = result
        .warnings
        .iter()
        .find(|d| d.code == "W1003")
        .expect("未使用的具名导入应经 warnings 通道报 W1003");
    assert_eq!(w1003.severity, Severity::Warning);
    assert!(
        w1003.message.contains("print"),
        "警告应包含导入名 'print'，实际: {}",
        w1003.message
    );
    assert!(
        !result.diagnostics.iter().any(|d| d.code.starts_with('W')),
        "警告不得混入错误 diagnostics 通道（会按错误计数阻断编译），实际: {:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.is_empty(),
        "该源码不应有任何类型错误，实际: {:?}",
        result.diagnostics
    );
}

#[test]
fn test_unused_module_import_reports_w1003() {
    // Arrange: 整模块导入（use std.io）且模块别名未被引用 → W1003。
    // 此前 analyzer 侧因缺 resolver 级信息显式跳过（#321 M2 遗留），
    // 移交 typecheck use elaboration 后恢复检测。
    let source = r#"
use std.io
main = {
    x = 1
    x
}
"#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        result.warnings.iter().any(|d| d.code == "W1003"),
        "未使用的整模块导入应报 W1003，实际: {:?}",
        result.warnings
    );
}

#[test]
fn test_used_import_no_warning() {
    // Arrange: 具名导入被调用 → 不报 W1003
    let source = r#"
use std.io.{print}
main = {
    print(42)
}
"#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        !result.warnings.iter().any(|d| d.code == "W1003"),
        "已使用的导入不应报 W1003，实际: {:?}",
        result.warnings
    );
}

#[test]
fn test_module_alias_member_call_marks_import_used() {
    // Arrange: 整模块导入后经别名限定调用（io.print）→ 导入已使用
    let source = r#"
use std.io
main = {
    io.print(42)
}
"#;

    // Act
    let result = check_source(source);

    // Assert
    assert!(
        !result.warnings.iter().any(|d| d.code == "W1003"),
        "别名限定调用应使整模块导入视为已使用，实际: {:?}",
        result.warnings
    );
}
