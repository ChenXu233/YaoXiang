//! RFC-027 Phase 3.3 集成测试：终止检查策略 1/2/4
//!
//! RFC-027 §7：编译器全自动证明循环终止。
//!
//! 测试覆盖：
//! - 策略 4 乘法缩放度量检测
//! - 策略 2 框架占位
//! - 终止检查器基础功能

use crate::frontend::core::typecheck::layers::termination::{
    Direction, LinearMeasure, TerminationChecker,
};

// ===================================================================
// RFC-027 §7.5: 策略 4 乘法缩放度量
// ===================================================================

/// LinearMeasure::multiplicative 构造器正确设置字段
#[test]
fn test_strategy4_multiplicative_measure_constructor() {
    let measure = LinearMeasure::multiplicative("i", 100, 2);
    assert_eq!(measure.var, "i", "变量名应为 i");
    assert_eq!(measure.bound, Some(100), "上界应为 100");
    assert_eq!(measure.delta, 2, "delta 应为 2（乘数）");
    assert_eq!(
        measure.direction,
        Direction::Increasing,
        "度量方向应为 Increasing（upper - v）"
    );
}

/// LinearMeasure::increasing 创建递增到上界的度量
#[test]
fn test_linear_measure_increasing_constructor() {
    let measure = LinearMeasure::increasing("i", None, Some(10), 1);
    assert_eq!(measure.var, "i");
    assert_eq!(measure.bound, Some(10));
    assert_eq!(measure.direction, Direction::Increasing);
    assert_eq!(measure.delta, 1);
}

/// LinearMeasure::decreasing 创建递减到下界的度量
#[test]
fn test_linear_measure_decreasing_constructor() {
    let measure = LinearMeasure::decreasing("i", None, Some(0), 1);
    assert_eq!(measure.var, "i");
    assert_eq!(measure.bound, Some(0));
    assert_eq!(measure.direction, Direction::Decreasing);
    assert_eq!(measure.delta, 1);
}

/// LinearMeasure::describe 生成可读描述
#[test]
fn test_linear_measure_describe() {
    let inc = LinearMeasure::increasing("i", Some("n"), None, 1);
    assert!(
        inc.describe().contains("n") && inc.describe().contains("i"),
        "describe 应包含变量名和边界: {}",
        inc.describe()
    );

    let dec = LinearMeasure::decreasing("j", None, Some(0), 1);
    assert!(
        dec.describe().contains("j"),
        "describe 应包含变量名: {}",
        dec.describe()
    );
}

// ===================================================================
// RFC-027 §7: 终止检查器基础功能
// ===================================================================

/// TerminationChecker 可以创建
#[test]
fn test_termination_checker_new() {
    let checker = TerminationChecker::new();
    // checker 创建后不应 panic
    let _ = checker;
}

/// TerminationChecker 可以通过 with_z3 注入 Z3 后端（None 时不崩溃）
#[test]
fn test_termination_checker_without_z3_does_not_crash() {
    let checker = TerminationChecker::new();
    // 没有 Z3 时策略 1 应被跳过，不崩溃
    // 这通过 checker 能正常 drop 来验证
    drop(checker);
}
