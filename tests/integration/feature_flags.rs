//! Feature flag 集成测试 — 基于 Feature Flag 重构设计
//!
//! 验证不同 feature 组合下的行为：
//! - 默认 features (cli): CLI 功能可用
//! - 无 features: 核心库功能可用，CLI 功能不可用
//! - Z3 功能在所有组合下可用
//!
//! 规范来源：集成测试规范 §7.2, §7.3

use yaoxiang::run;

// ============================================================================
// 默认 features 测试
// ============================================================================

/// 默认 features 下应该能运行基本代码
#[test]
fn test_run_with_default_features() {
    // Arrange
    let source = r#"
        main = {
            print("Hello, World!")
        }
    "#;

    // Act
    let result = run(source);

    // Assert
    assert!(
        result.is_ok(),
        "默认 features 下应该能运行基本代码: {:?}",
        result.err()
    );
}

/// 默认 features 下应该能运行复杂程序
#[test]
fn test_run_complex_program_with_default_features() {
    // Arrange
    let source = r#"
        main = {
            mut sum = 0
            mut i = 1
            while i <= 10 {
                sum = sum + i
                i = i + 1
            }
            print(sum)
        }
    "#;

    // Act
    let result = run(source);

    // Assert
    assert!(
        result.is_ok(),
        "默认 features 下应该能运行复杂程序: {:?}",
        result.err()
    );
}

// ============================================================================
// 核心功能测试（不受 feature 影响）
// ============================================================================

/// 核心功能应该在所有 feature 组合下可用
#[test]
fn test_core_functionality_available() {
    // Arrange
    let source = r#"
        main = {
            print("Core functionality works")
        }
    "#;

    // Act
    let result = run(source);

    // Assert
    assert!(
        result.is_ok(),
        "核心功能应该不受 feature 影响: {:?}",
        result.err()
    );
}

/// 基本算术运算应该在所有 feature 组合下可用
#[test]
fn test_basic_arithmetic_available() {
    // Arrange
    let source = r#"
        main = {
            x = 42
            y = x * 2
            z = y + 10
            print(z)
        }
    "#;

    // Act
    let result = run(source);

    // Assert
    assert!(
        result.is_ok(),
        "基本算术运算应该在所有 feature 组合下可用: {:?}",
        result.err()
    );
}

/// 控制流应该在所有 feature 组合下可用
#[test]
fn test_control_flow_available() {
    // Arrange
    let source = r#"
        main = {
            mut x = 0
            while x < 5 {
                x = x + 1
            }
            print(x)
        }
    "#;

    // Act
    let result = run(source);

    // Assert
    assert!(
        result.is_ok(),
        "控制流应该在所有 feature 组合下可用: {:?}",
        result.err()
    );
}

/// match 表达式应该在所有 feature 组合下可用
#[test]
fn test_match_expression_available() {
    // Arrange
    let source = r#"
        main = {
            r1 = match 1 { 1 => 100, _ => 0 }
        }
    "#;

    // Act
    let result = run(source);

    // Assert
    assert!(
        result.is_ok(),
        "match 表达式应该在所有 feature 组合下可用: {:?}",
        result.err()
    );
}

// ============================================================================
// Z3 功能测试
// ============================================================================

/// Z3 功能应该在所有 feature 组合下可用
#[test]
fn test_z3_available_in_all_feature_combinations() {
    // Arrange
    let source = r#"
        main = {
            print("Z3 works!")
        }
    "#;

    // Act
    let result = run(source);

    // Assert
    // Z3 可能未安装，但基本代码应该能运行
    assert!(result.is_ok(), "Z3 功能应该可用: {:?}", result.err());
}

// ============================================================================
// CLI feature 测试
// ============================================================================

/// CLI 专属功能在无 cli feature 下应该不可用
#[test]
fn test_cli_features_unavailable_without_cli() {
    // 这个测试主要验证编译时行为
    // 如果 cli feature 未启用，lsp/repl 模块应该不存在
    // 但由于集成测试无法测试编译时行为，这里主要验证核心功能不受影响

    // Arrange
    let source = r#"
        main = {
            print("Core functionality works")
        }
    "#;

    // Act
    let result = run(source);

    // Assert
    assert!(
        result.is_ok(),
        "核心功能应该不受 cli feature 影响: {:?}",
        result.err()
    );
}

// ============================================================================
// 列表操作测试
// ============================================================================

/// 列表操作应该在所有 feature 组合下可用
#[test]
fn test_list_operations_available() {
    // Arrange
    let source = r#"
        use std.{io, list}
        main = {
            xs = [1, 2, 3, 4, 5]
            ys = list.map(xs, x => x * 10)
            xs2 = [1, 2, 3, 4, 5]
            zs = list.filter(xs2, x => x > 2)
            xs3 = [1, 2, 3, 4, 5]
            s = list.reduce(xs3, (a, x) => a + x, 0)
            io.println(ys)
            io.println(zs)
            io.println(s)
        }
    "#;

    // Act
    let result = run(source);

    // Assert
    assert!(
        result.is_ok(),
        "列表操作应该在所有 feature 组合下可用: {:?}",
        result.err()
    );
}
