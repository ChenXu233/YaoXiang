//! Trait 求解器测试 — 基于语言规范 §3.5.2 & §3.9 & RFC-011 §2
//!
//! §3.5.2: 标准库接口（Clone, Equal, Debug, Iterator）
//! §3.9: 类型约束
//! RFC-011 §2: 类型约束系统
//!
//! 测试范围：
//! - 内置特质求解（Clone, Debug）
//! - 用户定义特质求解（通过 TraitTable）
//! - 批量求解与缓存
//! - TraitSolverError 的 Display 输出

use crate::frontend::core::typecheck::traits::solver::{TraitConstraint, TraitSolver};
use crate::frontend::core::types::base::MonoType;
use crate::frontend::core::types::base::trait_data::{TraitDefinition, TraitImplementation, TraitTable};
use std::collections::HashMap;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_trait_solver_creation() {
    // Arrange & Act
    let solver = TraitSolver::new();

    // Assert - 新建求解器应无已缓存约束
    assert!(
        solver.unsatisfied_constraints().is_empty(),
        "新建求解器的未满足约束应为空"
    );
}

#[test]
fn test_default_trait_solver() {
    // Arrange & Act
    let solver = TraitSolver::default();

    // Assert - Default 应创建可用的求解器
    assert!(
        solver.unsatisfied_constraints().is_empty(),
        "通过 Default 创建的求解器应可用"
    );
}

#[test]
fn test_solve_clone_for_int() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Int(32)],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "Int 类型应满足 Clone 约束");
}

#[test]
fn test_solve_clone_for_float() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Float(64)],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "Float 类型应满足 Clone 约束");
}

#[test]
fn test_solve_clone_for_bool() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Bool],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "Bool 类型应满足 Clone 约束");
}

#[test]
fn test_solve_clone_for_char() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Char],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "Char 类型应满足 Clone 约束");
}

#[test]
fn test_solve_clone_for_string() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::String],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "String 类型应满足 Clone 约束");
}

#[test]
fn test_solve_debug_for_int() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Debug".to_string(),
        args: vec![MonoType::Int(32)],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "Int 类型应满足 Debug 约束");
}

#[test]
fn test_solve_debug_for_void() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Debug".to_string(),
        args: vec![MonoType::Void],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(
        result.is_ok(),
        "Void 类型也应满足 Debug 约束（Debug 接受所有类型）"
    );
}

#[test]
fn test_solve_dup_for_int() {
    let mut solver = TraitSolver::new();
    let table = TraitTable::new();
    solver.set_trait_table(table);
    assert!(
        !solver.check_trait(&MonoType::Int(64), "Dup"),
        "Int is a primitive value type, NOT Dup"
    );
}

#[test]
fn test_solve_dup_for_float() {
    let mut solver = TraitSolver::new();
    let table = TraitTable::new();
    solver.set_trait_table(table);
    assert!(
        !solver.check_trait(&MonoType::Float(64), "Dup"),
        "Float is a primitive value type, NOT Dup"
    );
}

#[test]
fn test_solve_dup_for_bool() {
    let mut solver = TraitSolver::new();
    let table = TraitTable::new();
    solver.set_trait_table(table);
    assert!(
        !solver.check_trait(&MonoType::Bool, "Dup"),
        "Bool is a primitive value type, NOT Dup"
    );
}

#[test]
fn test_solve_dup_for_char() {
    let mut solver = TraitSolver::new();
    let table = TraitTable::new();
    solver.set_trait_table(table);
    assert!(
        !solver.check_trait(&MonoType::Char, "Dup"),
        "Char is a primitive value type, NOT Dup"
    );
}

#[test]
fn test_solve_dup_for_string() {
    let mut solver = TraitSolver::new();
    let table = TraitTable::new();
    solver.set_trait_table(table);
    assert!(
        solver.check_trait(&MonoType::String, "Dup"),
        "String should satisfy Dup"
    );
}

#[test]
fn test_solve_dup_for_bytes() {
    let mut solver = TraitSolver::new();
    let table = TraitTable::new();
    solver.set_trait_table(table);
    assert!(
        solver.check_trait(&MonoType::Bytes, "Dup"),
        "Bytes should satisfy Dup"
    );
}

#[test]
fn test_solve_unknown_builtin_trait_fails() {
    // Arrange - 规范 §3.5.2: 未知的 trait 名应报错，不应默认通过
    // 当前代码 _ => true 是 fallback 行为，测试按规范期望报错
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Display".to_string(),
        args: vec![MonoType::Int(32)],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert - 规范行为：未知 trait 应报错
    assert!(
        result.is_err(),
        "未知的 trait 名 'Display' 应返回错误，不应默认满足约束"
    );
}

#[test]
fn test_solve_all_multiple_constraints() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraints = vec![
        TraitConstraint {
            name: "Clone".to_string(),
            args: vec![MonoType::Int(32)],
        },
        TraitConstraint {
            name: "Debug".to_string(),
            args: vec![MonoType::Bool],
        },
        TraitConstraint {
            name: "Dup".to_string(),
            args: vec![MonoType::String],
        },
    ];

    // Act
    let result = solver.solve_all(&constraints);

    // Assert
    assert!(result.is_ok(), "批量求解多个合法约束应全部成功");
}

#[test]
fn test_solve_all_empty_constraints() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraints: Vec<TraitConstraint> = vec![];

    // Act
    let result = solver.solve_all(&constraints);

    // Assert
    assert!(result.is_ok(), "批量求解空约束列表应成功");
}

#[test]
fn test_solve_caches_result() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Int(32)],
    };

    // Act - 第一次求解
    let first = solver.solve(&constraint);
    // 第二次求解相同约束（应命中缓存）
    let second = solver.solve(&constraint);

    // Assert
    assert!(first.is_ok(), "首次求解应成功");
    assert!(second.is_ok(), "再次求解相同约束应命中缓存并成功");
}

#[test]
fn test_set_trait_table() {
    // Arrange
    let mut solver = TraitSolver::new();
    let table = TraitTable::new();

    // Act & Assert - 设置 trait 表不应 panic
    solver.set_trait_table(table);
}

#[test]
fn test_solve_user_defined_trait_with_impl() {
    // Arrange
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "Printable".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
        is_marker: false,
    });
    table.add_impl(TraitImplementation {
        trait_name: "Printable".to_string(),
        for_type_name: "int32".to_string(),
        methods: HashMap::new(),
    });

    let mut solver = TraitSolver::new();
    solver.set_trait_table(table);

    let constraint = TraitConstraint {
        name: "Printable".to_string(),
        args: vec![MonoType::Int(32)],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(
        result.is_ok(),
        "Int 类型实现了 Printable trait 时应求解成功"
    );
}

#[test]
fn test_check_trait_builtin_clone() {
    // Arrange
    let mut solver = TraitSolver::new();

    // Act & Assert
    assert!(
        solver.check_trait(&MonoType::Int(32), "Clone"),
        "Int 应满足 Clone"
    );
    assert!(
        solver.check_trait(&MonoType::Bool, "Clone"),
        "Bool 应满足 Clone"
    );
    assert!(
        solver.check_trait(&MonoType::String, "Clone"),
        "String 应满足 Clone"
    );
}

#[test]
fn test_check_trait_builtin_debug() {
    // Arrange
    let mut solver = TraitSolver::new();

    // Act & Assert
    assert!(
        solver.check_trait(&MonoType::Void, "Debug"),
        "Debug 应接受所有类型"
    );
}

#[test]
fn test_check_trait_user_defined() {
    // Arrange
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "Serializable".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
        is_marker: false,
    });
    table.add_impl(TraitImplementation {
        trait_name: "Serializable".to_string(),
        for_type_name: "string".to_string(),
        methods: HashMap::new(),
    });

    let mut solver = TraitSolver::new();
    solver.set_trait_table(table);

    // Act & Assert
    assert!(
        solver.check_trait(&MonoType::String, "Serializable"),
        "String 实现了 Serializable 时应返回 true"
    );
    assert!(
        !solver.check_trait(&MonoType::Int(32), "Serializable"),
        "Int 未实现 Serializable 时应返回 false"
    );
}

#[test]
fn test_propagate_constraints_returns_empty() {
    // Arrange
    let solver = TraitSolver::new();

    // Act
    let result = solver.propagate_constraints_to_type_args(&MonoType::Int(32), "Clone");

    // Assert
    assert!(result.is_empty(), "当前传播实现应返回空约束列表");
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_solve_clone_for_void_succeeds() {
    // Arrange - Void 是基本类型，应满足 Clone
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Void],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "Void 类型应满足 Clone 约束");
}

#[test]
fn test_solve_clone_for_bytes_succeeds() {
    // Arrange - Bytes 是基本类型，应满足 Clone
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Bytes],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "Bytes 类型应满足 Clone 约束");
}

#[test]
fn test_solve_user_trait_without_impl_fails() {
    // Arrange - 定义了 trait 但没有对应实现
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "Printable".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
        is_marker: false,
    });
    // 故意不添加 impl

    let mut solver = TraitSolver::new();
    solver.set_trait_table(table);

    let constraint = TraitConstraint {
        name: "Printable".to_string(),
        args: vec![MonoType::Int(32)],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_err(), "类型未实现用户定义 trait 时应返回错误");
    let diag = result.unwrap_err();
    assert!(
        diag.message.contains("Printable"),
        "错误消息应提及 trait 名称，实际: {}",
        diag.message
    );
}

#[test]
fn test_solve_empty_args_fails() {
    // Arrange - args 为空时 can_satisfy_constraint 返回 false
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_err(), "约束参数为空时应返回错误");
}

#[test]
fn test_solve_all_first_failure_stops() {
    // Arrange - List 不满足 Clone，应作为首个失败约束
    let mut solver = TraitSolver::new();
    let constraints = vec![
        TraitConstraint {
            name: "Clone".to_string(),
            args: vec![MonoType::List(Box::new(MonoType::Int(32)))], // 失败：List 不满足 Clone
        },
        TraitConstraint {
            name: "Clone".to_string(),
            args: vec![MonoType::Int(32)], // 不会执行
        },
    ];

    // Act
    let result = solver.solve_all(&constraints);

    // Assert
    assert!(result.is_err(), "批量求解中首个约束失败时应立即返回错误");
}

#[test]
fn test_check_trait_unknown_trait_returns_false() {
    // Arrange - 规范 §3.5.2: 未知 trait 应返回 false，不应默认通过
    // 当前代码 _ => true 是 fallback 行为，测试按规范期望返回 false
    let mut solver = TraitSolver::new();

    // Act
    let result = solver.check_trait(&MonoType::Int(32), "NonExistentTrait");

    // Assert - 规范行为：未知 trait 应返回 false
    assert!(
        !result,
        "未知的 trait 名 'NonExistentTrait' 应返回 false，不应默认满足"
    );
}

#[test]
fn test_unsatisfied_constraints_empty_after_creation() {
    // Arrange
    let solver = TraitSolver::new();

    // Act
    let unsatisfied = solver.unsatisfied_constraints();

    // Assert
    assert!(unsatisfied.is_empty(), "新建求解器不应有未满足的约束");
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_solve_clone_for_struct_succeeds() {
    // Arrange - 结构体所有字段都是 Clone 类型时应满足 Clone
    let mut solver = TraitSolver::new();
    let struct_ty = MonoType::Struct(crate::frontend::core::types::base::StructType {
        name: "Point".to_string(),
        fields: vec![("x".to_string(), MonoType::Int(32))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![struct_ty],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(
        result.is_ok(),
        "Struct（所有字段可 Clone）应满足 Clone 约束"
    );
}

#[test]
fn test_solve_clone_for_list_fails() {
    // Arrange
    let mut solver = TraitSolver::new();
    let list_ty = MonoType::List(Box::new(MonoType::Int(32)));
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![list_ty],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_err(), "List 类型不应满足 Clone 约束");
}

#[test]
fn test_solve_clone_for_tuple_succeeds() {
    // Arrange - 元组所有元素都是 Clone 类型时应满足 Clone
    let mut solver = TraitSolver::new();
    let tuple_ty = MonoType::Tuple(vec![MonoType::Int(32), MonoType::Bool]);
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![tuple_ty],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "Tuple（所有元素可 Clone）应满足 Clone 约束");
}

#[test]
fn test_solve_idempotent() {
    // Arrange
    let mut solver = TraitSolver::new();
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Bool],
    };

    // Act - 多次求解同一约束
    let r1 = solver.solve(&constraint);
    let r2 = solver.solve(&constraint);
    let r3 = solver.solve(&constraint);

    // Assert
    assert!(r1.is_ok(), "首次求解应成功");
    assert!(r2.is_ok(), "第二次求解应成功（缓存命中）");
    assert!(r3.is_ok(), "第三次求解应成功（缓存命中）");
}

#[test]
fn test_solve_different_types_same_trait() {
    // Arrange - 使用独立 solver 避免 simple_constraints 缓存干扰
    let mut solver_int = TraitSolver::new();
    let mut solver_float = TraitSolver::new();
    let mut solver_list = TraitSolver::new();

    // Act
    let r_int = solver_int.solve(&TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Int(32)],
    });
    let r_float = solver_float.solve(&TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Float(64)],
    });
    let r_list = solver_list.solve(&TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::List(Box::new(MonoType::Int(32)))],
    });

    // Assert
    assert!(r_int.is_ok(), "Int 应满足 Clone");
    assert!(r_float.is_ok(), "Float 应满足 Clone");
    assert!(r_list.is_err(), "List 不应满足 Clone");
}

#[test]
fn test_solve_same_type_different_traits() {
    // Arrange - 同一类型接受不同 trait 约束
    let mut solver = TraitSolver::new();
    let ty = MonoType::String;

    // Act
    let r_clone = solver.solve(&TraitConstraint {
        name: "Clone".to_string(),
        args: vec![ty.clone()],
    });
    let r_debug = solver.solve(&TraitConstraint {
        name: "Debug".to_string(),
        args: vec![ty.clone()],
    });
    let r_dup = solver.solve(&TraitConstraint {
        name: "Dup".to_string(),
        args: vec![ty.clone()],
    });
    let r_equal = solver.solve(&TraitConstraint {
        name: "Equal".to_string(),
        args: vec![ty],
    });

    // Assert
    assert!(r_clone.is_ok(), "String 应满足 Clone");
    assert!(r_debug.is_ok(), "String 应满足 Debug");
    assert!(r_dup.is_ok(), "String 应满足 Dup");
    assert!(r_equal.is_ok(), "String 应满足 Equal");
}

#[test]
fn test_solve_user_trait_with_wrong_type_name() {
    // Arrange - trait 存在、impl 存在，但类型不匹配
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "Printable".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
        is_marker: false,
    });
    table.add_impl(TraitImplementation {
        trait_name: "Printable".to_string(),
        for_type_name: "string".to_string(), // 只给 String 实现了
        methods: HashMap::new(),
    });

    let mut solver = TraitSolver::new();
    solver.set_trait_table(table);

    // Act - 用 Int(32) 去求解
    let result = solver.solve(&TraitConstraint {
        name: "Printable".to_string(),
        args: vec![MonoType::Int(32)],
    });

    // Assert
    assert!(result.is_err(), "类型未实现用户定义 trait 时应返回错误");
}

#[test]
fn test_solve_user_trait_not_in_table_falls_back_to_builtin() {
    // Arrange - TraitTable 中没有该 trait，应 fallback 到内置求解
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "SomeTrait".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
        is_marker: false,
    });

    let mut solver = TraitSolver::new();
    solver.set_trait_table(table);

    // Act - Clone 不在用户 trait 表中，应走内置路径
    let result = solver.solve(&TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Int(32)],
    });

    // Assert
    assert!(
        result.is_ok(),
        "当 trait 不在用户表中时应 fallback 到内置求解器"
    );
}

#[test]
fn test_trait_solver_error_display_undefined_trait() {
    // Arrange
    let err = crate::frontend::core::typecheck::traits::solver::TraitSolverError::UndefinedTrait {
        trait_name: "Foo".to_string(),
    };

    // Act
    let display = format!("{}", err);

    // Assert
    assert!(
        display.contains("Foo"),
        "UndefinedTrait 错误消息应包含 trait 名称，实际: {}",
        display
    );
}

#[test]
fn test_trait_solver_error_display_missing_impl() {
    // Arrange
    let err = crate::frontend::core::typecheck::traits::solver::TraitSolverError::MissingImpl {
        trait_name: "Bar".to_string(),
        type_name: "Baz".to_string(),
    };

    // Act
    let display = format!("{}", err);

    // Assert
    assert!(
        display.contains("Baz") && display.contains("Bar"),
        "MissingImpl 错误消息应包含类型和 trait 名称，实际: {}",
        display
    );
}

#[test]
fn test_trait_solver_error_display_unsatisfied_constraint() {
    // Arrange
    let err =
        crate::frontend::core::typecheck::traits::solver::TraitSolverError::UnsatisfiedConstraint {
            trait_name: "Clone".to_string(),
            type_name: "void".to_string(),
        };

    // Act
    let display = format!("{}", err);

    // Assert
    assert!(
        display.contains("void") && display.contains("Clone"),
        "UnsatisfiedConstraint 错误消息应包含类型和约束名，实际: {}",
        display
    );
}

#[test]
fn test_trait_solver_error_display_cyclic_inheritance() {
    // Arrange
    let err =
        crate::frontend::core::typecheck::traits::solver::TraitSolverError::CyclicInheritance {
            trait_name: "Recursive".to_string(),
        };

    // Act
    let display = format!("{}", err);

    // Assert
    assert!(
        display.contains("Recursive"),
        "CyclicInheritance 错误消息应包含 trait 名称，实际: {}",
        display
    );
}

#[test]
fn test_trait_solver_error_display_method_not_found() {
    // Arrange
    let err = crate::frontend::core::typecheck::traits::solver::TraitSolverError::MethodNotFound {
        trait_name: "Drawable".to_string(),
        method_name: "draw".to_string(),
    };

    // Act
    let display = format!("{}", err);

    // Assert
    assert!(
        display.contains("draw") && display.contains("Drawable"),
        "MethodNotFound 错误消息应包含方法和 trait 名称，实际: {}",
        display
    );
}

#[test]
fn test_trait_solver_error_is_clone() {
    // Arrange
    let err = crate::frontend::core::typecheck::traits::solver::TraitSolverError::UndefinedTrait {
        trait_name: "Test".to_string(),
    };

    // Act
    let cloned = err.clone();

    // Assert
    let original_display = format!("{}", err);
    let cloned_display = format!("{}", cloned);
    assert_eq!(
        original_display, cloned_display,
        "TraitSolverError 的 Clone 应产生相同内容"
    );
}

#[test]
fn test_trait_constraint_debug_format() {
    // Arrange
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Int(32)],
    };

    // Act
    let debug_str = format!("{:?}", constraint);

    // Assert
    assert!(
        debug_str.contains("Clone"),
        "TraitConstraint 的 Debug 输出应包含 trait 名称"
    );
}

#[test]
fn test_solve_clone_for_option_fails() {
    // Arrange
    let mut solver = TraitSolver::new();
    let option_ty = MonoType::Option(Box::new(MonoType::Int(32)));
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![option_ty],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(
        result.is_err(),
        "Option 类型不应满足 Clone 约束（不在内置检查列表中）"
    );
}

#[test]
fn test_solve_clone_for_result_fails() {
    // Arrange
    let mut solver = TraitSolver::new();
    let result_ty = MonoType::Result(Box::new(MonoType::Int(32)), Box::new(MonoType::String));
    let constraint = TraitConstraint {
        name: "Clone".to_string(),
        args: vec![result_ty],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(
        result.is_err(),
        "Result 类型不应满足 Clone 约束（不在内置检查列表中）"
    );
}

#[test]
fn test_solve_dup_for_struct_succeeds() {
    let mut solver = TraitSolver::new();
    // 所有字段都是 Dup 类型（String + &T 令牌），结构体应满足 Dup
    let struct_ty = MonoType::Struct(crate::frontend::core::types::base::StructType {
        name: "View".to_string(),
        fields: vec![
            ("name".to_string(), MonoType::String),
            (
                "ref_field".to_string(),
                MonoType::Ref {
                    mutable: false,
                    inner: Box::new(MonoType::Int(64)),
                },
            ),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    });
    assert!(
        solver.check_trait(&struct_ty, "Dup"),
        "Struct with all-Dup fields (String + &T) should satisfy Dup"
    );
}

#[test]
fn test_solve_dup_for_struct_fails() {
    let mut solver = TraitSolver::new();
    let struct_ty = MonoType::Struct(crate::frontend::core::types::base::StructType {
        name: "Container".to_string(),
        fields: vec![(
            "data".to_string(),
            MonoType::List(Box::new(MonoType::Int(32))),
        )],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    assert!(
        !solver.check_trait(&struct_ty, "Dup"),
        "Struct with non-Dup field (List) should NOT satisfy Dup"
    );
}

#[test]
fn test_solve_dup_for_tuple_succeeds() {
    let mut solver = TraitSolver::new();
    // 所有元素都是 Dup 类型（String + Bytes + &T）
    let tuple_ty = MonoType::Tuple(vec![
        MonoType::String,
        MonoType::Bytes,
        MonoType::Ref {
            mutable: false,
            inner: Box::new(MonoType::Bool),
        },
    ]);
    assert!(
        solver.check_trait(&tuple_ty, "Dup"),
        "Tuple with all Dup elements should satisfy Dup"
    );
}

#[test]
fn test_solve_dup_for_tuple_fails() {
    let mut solver = TraitSolver::new();
    let tuple_ty = MonoType::Tuple(vec![
        MonoType::Int(32),
        MonoType::List(Box::new(MonoType::Bool)),
    ]);
    assert!(
        !solver.check_trait(&tuple_ty, "Dup"),
        "Tuple with non-Dup element (List) should NOT satisfy Dup"
    );
}

#[test]
fn test_solve_dup_for_list_fails() {
    let mut solver = TraitSolver::new();
    let list_ty = MonoType::List(Box::new(MonoType::Int(32)));
    assert!(
        !solver.check_trait(&list_ty, "Dup"),
        "List type should NOT satisfy Dup (conservative)"
    );
}

#[test]
fn test_solve_dup_for_void() {
    let mut solver = TraitSolver::new();
    let table = TraitTable::new();
    solver.set_trait_table(table);
    let ty = MonoType::Void;
    assert!(solver.check_trait(&ty, "Dup"), "Void should satisfy Dup");
}

#[test]
fn test_solve_dup_for_arc() {
    let mut solver = TraitSolver::new();
    let table = TraitTable::new();
    solver.set_trait_table(table);
    let ty = MonoType::Arc(Box::new(MonoType::Int(64)));
    assert!(solver.check_trait(&ty, "Dup"), "Arc should satisfy Dup");
}

#[test]
fn test_solve_debug_for_struct() {
    // Arrange
    let mut solver = TraitSolver::new();
    let struct_ty = MonoType::Struct(crate::frontend::core::types::base::StructType {
        name: "Foo".to_string(),
        fields: vec![],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    });
    let constraint = TraitConstraint {
        name: "Debug".to_string(),
        args: vec![struct_ty],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "Debug 应接受所有类型（包括 Struct）");
}

#[test]
fn test_solve_debug_for_list() {
    // Arrange
    let mut solver = TraitSolver::new();
    let list_ty = MonoType::List(Box::new(MonoType::Int(32)));
    let constraint = TraitConstraint {
        name: "Debug".to_string(),
        args: vec![list_ty],
    };

    // Act
    let result = solver.solve(&constraint);

    // Assert
    assert!(result.is_ok(), "Debug 应接受所有类型（包括 List）");
}

#[test]
fn test_user_trait_same_name_as_builtin_prefers_user() {
    // Arrange - 用户定义了与内置同名的 trait（如 "Clone"）
    let mut table = TraitTable::new();
    table.add_trait(TraitDefinition {
        name: "Clone".to_string(),
        methods: HashMap::new(),
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
        is_marker: false,
    });
    table.add_impl(TraitImplementation {
        trait_name: "Clone".to_string(),
        for_type_name: "void".to_string(),
        methods: HashMap::new(),
    });

    let mut solver = TraitSolver::new();
    solver.set_trait_table(table);

    // Act - 用户定义的 Clone 在 TraitTable 中有 void 的 impl
    let result = solver.solve(&TraitConstraint {
        name: "Clone".to_string(),
        args: vec![MonoType::Void],
    });

    // Assert
    assert!(
        result.is_ok(),
        "用户定义的 Clone trait 应优先于内置检查（表中有 void 的实现）"
    );
}
