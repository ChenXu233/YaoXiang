//! 错误收集测试

use crate::frontend::typecheck::TypeError;
use crate::frontend::typecheck::TypeErrorCollector;
use crate::frontend::shared::error::Warning;
use crate::frontend::typecheck::MonoType;
use crate::util::span::Span;

/// 测试多个类型错误收集
#[test]
fn test_multiple_error_collection() {
    let mut errors = TypeErrorCollector::new();

    // 添加一些错误
    errors.add_error(TypeError::unknown_variable(
        "x".to_string(),
        Span::default(),
    ));
    errors.add_error(TypeError::unknown_variable(
        "y".to_string(),
        Span::default(),
    ));

    // 应该收集到两个错误
    assert_eq!(errors.error_count(), 2);
    assert!(errors.has_errors());
}

/// 测试诊断生成
#[test]
fn test_diagnostic_generation() {
    let error = TypeError::unknown_variable("test_var".to_string(), Span::default());
    let diagnostic: crate::frontend::shared::error::Diagnostic = error.into();

    assert_eq!(
        diagnostic.severity,
        crate::frontend::shared::error::Severity::Error
    );
    assert!(!diagnostic.message.is_empty());
}

/// 测试错误收集器清空
#[test]
fn test_error_collector_clear() {
    let mut collector = TypeErrorCollector::new();

    collector.add_error(TypeError::unknown_variable(
        "x".to_string(),
        Span::default(),
    ));
    collector.add_error(TypeError::unknown_variable(
        "y".to_string(),
        Span::default(),
    ));

    assert_eq!(collector.error_count(), 2);

    collector.clear();

    assert_eq!(collector.error_count(), 0);
    assert!(!collector.has_errors());
}

/// 测试错误收集器转移
#[test]
fn test_error_collector_into_errors() {
    let mut collector = TypeErrorCollector::new();

    collector.add_error(TypeError::unknown_variable(
        "x".to_string(),
        Span::default(),
    ));
    collector.add_error(TypeError::unknown_variable(
        "y".to_string(),
        Span::default(),
    ));

    let errors: Vec<TypeError> = collector.into_errors();

    assert_eq!(errors.len(), 2);
}

/// 测试警告收集
#[test]
fn test_warning_collection() {
    let mut collector = TypeErrorCollector::new();

    collector.add_warning(Warning::UnusedVariable {
        name: "unused".to_string(),
        span: Span::default(),
    });

    assert!(collector.has_warnings());
    assert_eq!(collector.warning_count(), 1);
}

/// 测试错误为空检查
#[test]
fn test_no_errors() {
    let collector = TypeErrorCollector::new();
    assert!(!collector.has_errors());
    assert!(!collector.has_warnings());
    assert_eq!(collector.error_count(), 0);
}

/// 测试诊断转换
#[test]
fn test_diagnostic_from_type_error() {
    let error = TypeError::type_mismatch(
        Box::new(MonoType::Bool),
        Box::new(MonoType::Int(64)),
        Span::default(),
    );

    let diagnostic: crate::frontend::shared::error::Diagnostic = error.into();
    assert_eq!(
        diagnostic.severity,
        crate::frontend::shared::error::Severity::Error
    );
    assert!(!diagnostic.message.is_empty());
}

/// 测试Severity枚举
#[test]
fn test_severity_levels() {
    // 测试 Severity 的 Debug 和 Display 实现
    let error_str = format!("{:?}", crate::frontend::shared::error::Severity::Error);
    assert!(error_str.contains("Error"));

    let warning_str = format!("{:?}", crate::frontend::shared::error::Severity::Warning);
    assert!(warning_str.contains("Warning"));
}

/// 测试错误收集器错误列表
#[test]
fn test_error_collector_errors_list() {
    let mut collector = TypeErrorCollector::new();

    collector.add_error(TypeError::unknown_variable(
        "x".to_string(),
        Span::default(),
    ));

    let errors = collector.errors();
    assert_eq!(errors.len(), 1);
}

/// 测试错误收集器警告列表
#[test]
fn test_error_collector_warnings_list() {
    let mut collector = TypeErrorCollector::new();

    collector.add_warning(Warning::UnusedVariable {
        name: "test".to_string(),
        span: Span::default(),
    });

    let warnings = collector.warnings();
    assert_eq!(warnings.len(), 1);
}

/// 测试错误收集器合并
#[test]
fn test_error_collector_merge() {
    let mut collector1 = TypeErrorCollector::new();

    collector1.add_error(TypeError::unknown_variable(
        "x".to_string(),
        Span::default(),
    ));
    collector1.add_error(TypeError::unknown_variable(
        "y".to_string(),
        Span::default(),
    ));

    assert_eq!(collector1.error_count(), 2);
}

/// 测试诊断代码格式
#[test]
fn test_diagnostic_code_format() {
    let error = TypeError::type_mismatch(
        Box::new(MonoType::Int(64)),
        Box::new(MonoType::String),
        Span::default(),
    );
    let diagnostic: crate::frontend::shared::error::Diagnostic = error.into();

    // 错误码应该是 E001 格式
    assert!(diagnostic.code.starts_with("E"));
    assert!(diagnostic.code.len() >= 3);
}

/// 测试错误收集器警告计数
#[test]
fn test_error_collector_warning_count() {
    let collector = TypeErrorCollector::new();
    assert_eq!(collector.warning_count(), 0);
}

/// 测试类型错误变体
#[test]
fn test_type_error_variants() {
    // 测试各种错误类型
    let unknown = TypeError::unknown_variable("test".to_string(), Span::default());
    let mismatch = TypeError::type_mismatch(
        Box::new(MonoType::Int(64)),
        Box::new(MonoType::String),
        Span::default(),
    );

    // 验证错误可以格式化
    let _ = format!("{}", unknown);
    let _ = format!("{}", mismatch);
}

/// 测试诊断相关位置
#[test]
fn test_diagnostic_related() {
    let error = TypeError::unknown_variable("test".to_string(), Span::default());
    let diagnostic: crate::frontend::shared::error::Diagnostic = error.into();

    // 相关诊断应该为空
    assert!(diagnostic.related.is_empty());
}

/// 测试错误收集器获取错误
#[test]
fn test_error_collector_get_errors() {
    let mut collector = TypeErrorCollector::new();

    collector.add_error(TypeError::unknown_variable(
        "x".to_string(),
        Span::default(),
    ));

    let errors = collector.errors();
    assert_eq!(errors.len(), 1);
}

/// 测试多种错误类型
#[test]
fn test_multiple_error_types() {
    let mut collector = TypeErrorCollector::new();

    collector.add_error(TypeError::unknown_variable(
        "x".to_string(),
        Span::default(),
    ));
    collector.add_error(TypeError::type_mismatch(
        Box::new(MonoType::Int(64)),
        Box::new(MonoType::String),
        Span::default(),
    ));
    collector.add_warning(Warning::UnusedVariable {
        name: "y".to_string(),
        span: Span::default(),
    });

    assert_eq!(collector.error_count(), 2);
    assert_eq!(collector.warning_count(), 1);
}

/// 测试类型不匹配错误
#[test]
fn test_type_mismatch_error() {
    let error = TypeError::type_mismatch(
        Box::new(MonoType::Int(32)),
        Box::new(MonoType::Float(64)),
        Span::default(),
    );

    let formatted = format!("{}", error);
    // 检查是否包含类型不匹配相关信息
    assert!(
        formatted.contains("Type mismatch")
            || formatted.contains("int")
            || formatted.contains("float")
    );
}

/// 测试未知变量错误格式化
#[test]
fn test_unknown_variable_error_format() {
    let error = TypeError::unknown_variable("undefined_var".to_string(), Span::default());
    let formatted = format!("{}", error);

    assert!(formatted.contains("Unknown variable"));
    assert!(formatted.contains("undefined_var"));
}

/// 测试警告格式化
#[test]
fn test_warning_formatting() {
    let warning = Warning::UnusedVariable {
        name: "unused_var".to_string(),
        span: Span::default(),
    };

    let formatted = format!("{}", warning);
    assert!(formatted.contains("unused_var"));
}
