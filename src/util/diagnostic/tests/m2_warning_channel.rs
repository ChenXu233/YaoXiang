//! #321 M2 警告通道测试
//!
//! 验证：W 前缀码缺省 Warning severity、ErrorCollector 按严重级别分流、
//! dead_code 发射点的警告诊断经 builder 推导后带 Warning severity。

use crate::frontend::core::parser::ast::{Module, SpannedIdent, Stmt, StmtKind};
use crate::frontend::core::typecheck::passes::dead_code::DeadCodeAnalyzer;
use crate::frontend::core::typecheck::semantic_db::SemanticDB;
use crate::util::diagnostic::{ErrorCollector, ErrorCodeDefinition, Severity};
use crate::util::span::Span;

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

#[test]
fn test_dead_code_warning_carries_warning_severity() {
    // 显式成员导入（use std.io.{print}）且 print 未被引用 → W1003
    let import = Stmt {
        kind: StmtKind::Use {
            path: "std.io".to_string(),
            path_span: Span::dummy(),
            path_parts: vec![SpannedIdent {
                name: "io".to_string(),
                span: Span::dummy(),
            }],
            items: Some(vec!["print".to_string()]),
            item_aliases: None,
            alias: None,
        },
        span: Span::dummy(),
    };
    let ast = Module {
        items: vec![import],
        span: Span::dummy(),
    };

    let mut analyzer = DeadCodeAnalyzer::new();
    let warnings = analyzer.analyze(&ast, &SemanticDB::new());
    assert!(
        warnings.iter().any(|w| w.code == "W1003"),
        "显式成员导入未使用应报 W1003，实际: {:?}",
        warnings
    );

    let diagnostics = analyzer.to_diagnostics(&warnings);
    let w1003 = diagnostics
        .iter()
        .find(|d| d.code == "W1003")
        .expect("W1003 diagnostic");
    assert_eq!(w1003.severity, Severity::Warning);
}
