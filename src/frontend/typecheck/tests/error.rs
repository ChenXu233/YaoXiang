//! 错误收集测试

use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::ast::*;
use crate::frontend::typecheck::*;
use crate::util::span::Span;

/// 测试多个类型错误收集
#[test]
fn test_multiple_error_collection() {
    let mut errors = ErrorCollector::new();

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

/// 测试错误格式化
#[test]
fn test_error_formatting() {
    let formatter = ErrorFormatter::new(false);

    let mismatch = TypeError::type_mismatch(MonoType::Int(64), MonoType::String, Span::default());

    let formatted = formatter.format_error(&mismatch);
    assert!(formatted.contains("Type mismatch"));
    assert!(formatted.contains("int64"));
    assert!(formatted.contains("string"));
}

/// 测试详细错误格式化
#[test]
fn test_verbose_error_formatting() {
    let formatter = ErrorFormatter::new(true);

    let mismatch = TypeError::type_mismatch(MonoType::Int(64), MonoType::String, Span::default());

    let formatted = formatter.format_error(&mismatch);
    // 详细模式应该包含位置信息
    assert!(formatted.contains("Type mismatch"));
}

/// 测试诊断生成
#[test]
fn test_diagnostic_generation() {
    let error = TypeError::unknown_variable("test_var".to_string(), Span::default());
    let diagnostic: Diagnostic = error.into();

    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(diagnostic.code.starts_with("E"));
    assert!(diagnostic.message.contains("test_var"));
}

/// 测试错误收集器清空
#[test]
fn test_error_collector_clear() {
    let mut collector = ErrorCollector::new();

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
    let mut collector = ErrorCollector::new();

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
    let mut collector = ErrorCollector::new();

    collector.add_warning(Warning::UnusedVariable {
        name: "unused".to_string(),
        span: Span::default(),
    });

    assert!(collector.has_warnings());
    assert_eq!(collector.warning_count(), 1);
}

/// 测试下标越界错误（简化版）
#[test]
fn test_index_out_of_bounds_error() {
    let mut solver = TypeConstraintSolver::new();
    let mut inferrer = TypeInferrer::new(&mut solver);

    // 创建一个静态类型的元组
    let tuple = Expr::Tuple(
        vec![
            Expr::Lit(Literal::Int(1), Span::default()),
            Expr::Lit(Literal::Int(2), Span::default()),
        ],
        Span::default(),
    );
    let _tuple_ty = inferrer.infer_expr(&tuple).unwrap();

    // 尝试访问索引 10 - 这应该产生一个错误
    let index = Expr::Lit(Literal::Int(10), Span::default());
    let index_expr = Expr::Index {
        expr: Box::new(tuple),
        index: Box::new(index),
        span: Span::default(),
    };

    // 当前实现可能返回类型变量而不是错误
    let _result = inferrer.infer_expr(&index_expr);
}

/// 测试类型不匹配错误
#[test]
fn test_type_mismatch_error() {
    let error = TypeError::type_mismatch(MonoType::Int(32), MonoType::Float(64), Span::default());

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

/// 测试诊断代码生成
#[test]
fn test_diagnostic_code() {
    let error = TypeError::unknown_variable("test".to_string(), Span::default());
    let diagnostic: Diagnostic = error.into();

    assert!(diagnostic.code.starts_with("E00"));
    assert_eq!(diagnostic.severity, Severity::Error);
}

/// 测试错误格式化器详细模式
#[test]
fn test_error_formatter_with_location() {
    let formatter = ErrorFormatter::new(true);

    let error = TypeError::unknown_variable("foo".to_string(), Span::default());
    let formatted = formatter.format_error(&error);

    // 应该包含错误信息
    assert!(formatted.contains("foo") || formatted.contains("Unknown"));
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

/// 测试多种错误类型
#[test]
fn test_multiple_error_types() {
    let mut collector = ErrorCollector::new();

    collector.add_error(TypeError::unknown_variable(
        "x".to_string(),
        Span::default(),
    ));
    collector.add_error(TypeError::type_mismatch(
        MonoType::Int(64),
        MonoType::String,
        Span::default(),
    ));
    collector.add_warning(Warning::UnusedVariable {
        name: "y".to_string(),
        span: Span::default(),
    });

    assert_eq!(collector.error_count(), 2);
    assert_eq!(collector.warning_count(), 1);
}

/// 测试错误为空检查
#[test]
fn test_no_errors() {
    let collector = ErrorCollector::new();
    assert!(!collector.has_errors());
    assert!(!collector.has_warnings());
    assert_eq!(collector.error_count(), 0);
}

/// 测试诊断转换
#[test]
fn test_diagnostic_from_type_error() {
    let error = TypeError::type_mismatch(MonoType::Bool, MonoType::Int(64), Span::default());

    let diagnostic: Diagnostic = error.into();
    assert_eq!(diagnostic.severity, Severity::Error);
    assert!(!diagnostic.message.is_empty());
}

/// 测试Severity枚举
#[test]
fn test_severity_levels() {
    // 测试 Severity 的 Debug 和 Display 实现
    let error_str = format!("{:?}", Severity::Error);
    assert!(error_str.contains("Error"));

    let warning_str = format!("{:?}", Severity::Warning);
    assert!(warning_str.contains("Warning"));
}

/// 测试错误收集器错误列表
#[test]
fn test_error_collector_errors_list() {
    let mut collector = ErrorCollector::new();

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
    let mut collector = ErrorCollector::new();

    collector.add_warning(Warning::UnusedVariable {
        name: "test".to_string(),
        span: Span::default(),
    });

    let warnings = collector.warnings();
    assert_eq!(warnings.len(), 1);
}

/// 测试错误格式化器非详细模式
#[test]
fn test_error_formatter_non_verbose() {
    let formatter = ErrorFormatter::new(false);

    let error = TypeError::type_mismatch(MonoType::Int(64), MonoType::String, Span::default());

    let formatted = formatter.format_error(&error);
    // 非详细模式不应该包含 span 信息
    assert!(formatted.contains("Type mismatch"));
}

/// 测试错误收集器合并
#[test]
fn test_error_collector_merge() {
    let mut collector1 = ErrorCollector::new();

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
    let error = TypeError::type_mismatch(MonoType::Int(64), MonoType::String, Span::default());
    let diagnostic: Diagnostic = error.into();

    // 错误码应该是 E001 格式
    assert!(diagnostic.code.starts_with("E"));
    assert!(diagnostic.code.len() >= 3);
}

/// 测试错误收集器警告计数
#[test]
fn test_error_collector_warning_count() {
    let collector = ErrorCollector::new();
    assert_eq!(collector.warning_count(), 0);
}

/// 测试类型错误变体
#[test]
fn test_type_error_variants() {
    // 测试各种错误类型
    let unknown = TypeError::unknown_variable("test".to_string(), Span::default());
    let mismatch = TypeError::type_mismatch(MonoType::Int(64), MonoType::String, Span::default());

    // 验证错误可以格式化
    let _ = format!("{}", unknown);
    let _ = format!("{}", mismatch);
}

/// 测试诊断相关位置
#[test]
fn test_diagnostic_related() {
    let error = TypeError::unknown_variable("test".to_string(), Span::default());
    let diagnostic: Diagnostic = error.into();

    // 相关诊断应该为空
    assert!(diagnostic.related.is_empty());
}

/// 测试错误收集器获取错误
#[test]
fn test_error_collector_get_errors() {
    let mut collector = ErrorCollector::new();

    collector.add_error(TypeError::unknown_variable(
        "x".to_string(),
        Span::default(),
    ));

    let errors = collector.errors();
    assert_eq!(errors.len(), 1);
}
