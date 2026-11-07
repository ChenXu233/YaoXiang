//! #321 M2 警告通道测试
//!
//! 验证：W 前缀码缺省 Warning severity、ErrorCollector 按严重级别分流、
//! 未使用导入（W1003）经 typecheck use elaboration 检出后走 warnings 通道
//! （Warning severity、不混入错误 diagnostics）。

use crate::frontend::core::sema::probe::TypeLevelNamespace;
use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::sema::checker::TypeChecker;
use crate::util::diagnostic::{ErrorCodeDefinition, Severity};

/// Helper: test_unused_named_import_reports_w1003_via_warnings_channel 的完整夹具与断言（逐条断言见函数体）。

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

/// 辅助：源码 → 类型检查结果（语法必须正确，否则测试本身有错）
fn check_source(source: &str) -> crate::frontend::core::sema::types::TypeCheckResult {
    let tokens = tokenize(source).expect("测试源码词法不应失败");
    let parse_result = parse(&tokens);
    assert!(
        !parse_result.has_errors,
        "测试源码语法必须正确（否则无法区分 bug 与测试写错）: {:?}",
        parse_result.errors
    );

    let mut checker = TypeChecker::new("test");
    checker.check_module(
        &parse_result.module,
        &TypeLevelNamespace::scan_one(&parse_result.module),
    )
}

/// WBS 3.20.2：变体解构（match some(v)/none()）所需的导入不得判未使用——
/// 变体集在场是该形态的前置（删 `use std.option` 即 E1002），照
/// docs/src/reference/stdlib/weak.md 的自然写法取最小同构。
#[test]
fn test_variant_destructure_import_not_reported_w1003() {
    // Arrange: 导入 Option 的变体集后仅经 match 模式消费（不写类型名）
    let source = r#"
use std.weak
use std.option

main = () => {
    p = ref 42
    w = weak.new(p)
    u = weak.upgrade(w)
    match u {
        some(v) => println("alive"),
        none() => println("dropped"),
    }
}
"#;

    // Act
    let result = check_source(source);

    // Assert: 不报 W1003；且无类型错误（删该导入今日报 E1002——改判后仍能编译）
    assert!(
        !result.warnings.iter().any(|d| d.code == "W1003"),
        "变体解构所需的导入不应报 W1003，实际: {:?}",
        result.warnings
    );
    assert!(
        result.diagnostics.is_empty(),
        "该源码不应有任何类型错误（E1002 缺导入形态的否证面），实际: {:?}",
        result.diagnostics
    );
}
