//! 赋值检查测试 — 基于语言规范 §5.2 & RFC-010
//!
//! §5.2: 变量声明
//! RFC-010: 统一类型语法

use crate::frontend::core::typecheck::inference::assignment::{
    AssignmentChecker, ConstraintAssignmentInfo,
};
use crate::frontend::core::types::mono::StructType;
use crate::frontend::core::types::MonoType;
use crate::frontend::core::typecheck::environment::TypeEnvironment;
use crate::util::span::Span;
use std::collections::HashMap;

// ===================================================================
// 辅助函数
// ===================================================================

/// 创建测试用的 Span
fn dummy_span() -> Span {
    Span::dummy()
}

/// 创建空的 StructType（无字段、无方法）
fn empty_struct(name: &str) -> MonoType {
    MonoType::Struct(StructType {
        name: name.to_string(),
        fields: vec![],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    })
}

/// 创建接口类型（所有字段均为 Fn 类型）
///
/// Drawable 接口 = { draw: (Surface) -> Void }
fn make_drawable_interface() -> MonoType {
    MonoType::Struct(StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
            },
        )],
        methods: HashMap::new(),
        field_mutability: vec![],
        field_has_default: vec![],
        interfaces: vec![],
    })
}

/// 创建实现了 Drawable 的具体类型 Circle
fn make_circle_implementing_drawable() -> MonoType {
    MonoType::Struct(StructType {
        name: "Circle".to_string(),
        fields: vec![
            ("radius".to_string(), MonoType::Float(64)),
            (
                "draw".to_string(),
                MonoType::Fn {
                    params: vec![MonoType::TypeRef("Surface".to_string())],
                    return_type: Box::new(MonoType::Void),
                },
            ),
        ],
        methods: HashMap::new(),
        field_mutability: vec![false, false],
        field_has_default: vec![false, false],
        interfaces: vec!["Drawable".to_string()],
    })
}

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_assignment_same_type() {
    // Arrange
    let mut checker = AssignmentChecker::new();
    let lhs = MonoType::Int(32);
    let rhs = MonoType::Int(32);
    let span = dummy_span();

    // Act
    let result = checker.check_assignment(&lhs, &rhs, span, None);

    // Assert
    assert!(result.is_ok(), "相同类型 Int(32) 赋值给 Int(32) 应成功");
    assert!(
        checker.last_constraint_info().is_none(),
        "非约束赋值不应产生 ConstraintAssignmentInfo"
    );
}

#[test]
fn test_assignment_int_to_float_widening() {
    // Arrange — 规范 §3.2.1：禁止隐式拓宽，Int 不是 Float 的子类型
    // Int → Float 必须使用 Float(x) 显式转换
    let mut checker = AssignmentChecker::new();
    let lhs = MonoType::Float(64);
    let rhs = MonoType::Int(32);
    let span = dummy_span();

    // Act
    let result = checker.check_assignment(&lhs, &rhs, span, None);

    // Assert — 规范 §3.2.1：Int(32) 赋值给 Float(64) 应失败（禁止隐式拓宽）
    assert!(
        result.is_err(),
        "Int(32) 赋值给 Float(64) 应失败（规范 §3.2.1 禁止隐式拓宽，必须使用 Float(x) 显式转换）"
    );
}

#[test]
fn test_assignment_constraint_concrete() {
    // Arrange — 接口赋值，右值为编译期可确定的具体 Struct
    let mut checker = AssignmentChecker::new();
    let constraint = make_drawable_interface();
    let circle = make_circle_implementing_drawable();
    let span = dummy_span();

    // Act
    let result = checker.check_assignment(&constraint, &circle, span, None);

    // Assert
    assert!(
        result.is_ok(),
        "Circle（实现了 Drawable）赋值给 Drawable 接口应成功"
    );
    let info = checker
        .last_constraint_info()
        .expect("约束赋值应产生 ConstraintAssignmentInfo");
    match info {
        ConstraintAssignmentInfo::Concrete {
            concrete_type,
            constraint_type,
        } => {
            assert_eq!(*concrete_type, circle, "Concrete.concrete_type 应为 Circle");
            assert_eq!(
                *constraint_type, constraint,
                "Concrete.constraint_type 应为 Drawable"
            );
        }
        other => panic!("期望 Concrete 变体，实际得到 {:?}", other),
    }
}

#[test]
fn test_assignment_constraint_dynamic() {
    // Arrange — 接口赋值，右值为 TypeRef（编译期无法确定具体类型）
    // 通过 TypeEnvironment 提供方法绑定，使 TypeRef 满足接口约束
    let mut checker = AssignmentChecker::new();
    let constraint = make_drawable_interface();
    let rhs = MonoType::TypeRef("SomeShape".to_string());
    let span = dummy_span();

    // 构造环境：SomeShape.draw 绑定为 (Surface) -> Void
    let mut env = TypeEnvironment::default();
    env.method_bindings.insert(
        "SomeShape.draw".to_string(),
        MonoType::Fn {
            params: vec![MonoType::TypeRef("Surface".to_string())],
            return_type: Box::new(MonoType::Void),
        },
    );

    // Act
    let result = checker.check_assignment(&constraint, &rhs, span, Some(&env));

    // Assert
    assert!(
        result.is_ok(),
        "TypeRef（有方法绑定）赋值给 Drawable 接口应成功"
    );
    let info = checker
        .last_constraint_info()
        .expect("约束赋值应产生 ConstraintAssignmentInfo");
    match info {
        ConstraintAssignmentInfo::Dynamic { constraint_type } => {
            assert_eq!(
                *constraint_type, constraint,
                "Dynamic.constraint_type 应为 Drawable"
            );
        }
        other => panic!("期望 Dynamic 变体，实际得到 {:?}", other),
    }
}

#[test]
fn test_assignment_constraint_dynamic_via_empty_constraint() {
    // Arrange — 空约束（无方法要求）赋值，右值为 TypeRef
    // 空约束下任何类型都满足，且 TypeRef 非具体 Struct → Dynamic
    let mut checker = AssignmentChecker::new();
    let empty_constraint = empty_struct("Serializable");
    let rhs = MonoType::TypeRef("AnyType".to_string());
    let span = dummy_span();

    // Act
    let result = checker.check_assignment(&empty_constraint, &rhs, span, None);

    // Assert
    assert!(result.is_ok(), "TypeRef 赋值给空约束（Serializable）应成功");
    let info = checker
        .last_constraint_info()
        .expect("约束赋值应产生 ConstraintAssignmentInfo");
    match info {
        ConstraintAssignmentInfo::Dynamic { constraint_type } => {
            assert_eq!(
                *constraint_type, empty_constraint,
                "Dynamic.constraint_type 应为 Serializable"
            );
        }
        other => panic!("期望 Dynamic 变体，实际得到 {:?}", other),
    }
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_assignment_incompatible_types() {
    // Arrange — Int(32) 和 String 无子类型关系
    let mut checker = AssignmentChecker::new();
    let lhs = MonoType::Int(32);
    let rhs = MonoType::String;
    let span = dummy_span();

    // Act
    let result = checker.check_assignment(&lhs, &rhs, span, None);

    // Assert
    assert!(
        result.is_err(),
        "String 赋值给 Int(32) 应失败（类型不兼容）"
    );
}

#[test]
fn test_assignment_float_to_int_narrowing() {
    // Arrange — 不允许收窄：Float 不是 Int 的子类型
    let mut checker = AssignmentChecker::new();
    let lhs = MonoType::Int(32);
    let rhs = MonoType::Float(64);
    let span = dummy_span();

    // Act
    let result = checker.check_assignment(&lhs, &rhs, span, None);

    // Assert
    assert!(
        result.is_err(),
        "Float(64) 赋值给 Int(32) 应失败（不允许收窄）"
    );
}

#[test]
fn test_assignment_constraint_missing_method() {
    // Arrange — 右值缺少接口要求的方法
    let mut checker = AssignmentChecker::new();
    let constraint = make_drawable_interface();
    // Square 没有 draw 方法
    let square = MonoType::Struct(StructType {
        name: "Square".to_string(),
        fields: vec![("side".to_string(), MonoType::Float(64))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let span = dummy_span();

    // Act
    let result = checker.check_assignment(&constraint, &square, span, None);

    // Assert
    assert!(
        result.is_err(),
        "Square（无 draw 方法）赋值给 Drawable 应失败"
    );
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_assignment_fn_type_compatibility() {
    // Arrange — 相同签名的函数类型赋值
    let mut checker = AssignmentChecker::new();
    let fn_type = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Bool],
        return_type: Box::new(MonoType::String),
    };
    let lhs = fn_type.clone();
    let rhs = fn_type.clone();
    let span = dummy_span();

    // Act
    let result = checker.check_assignment(&lhs, &rhs, span, None);

    // Assert
    assert!(result.is_ok(), "相同签名的函数类型赋值应成功");
}

#[test]
fn test_assignment_destructuring() {
    // Arrange
    let checker = AssignmentChecker::new();
    let patterns = vec!["x".to_string(), "y".to_string()];
    let rhs = MonoType::Tuple(vec![MonoType::Int(32), MonoType::Bool]);
    let span = dummy_span();

    // Act
    let result = checker.check_destructuring(&patterns, &rhs, span);

    // Assert
    assert!(result.is_ok(), "解构赋值检查应成功（简化实现始终返回 Ok）");
}

#[test]
fn test_assignment_new_checker_has_no_constraint_info() {
    // Arrange & Act
    let checker = AssignmentChecker::new();

    // Assert
    assert!(
        checker.last_constraint_info().is_none(),
        "新建的 AssignmentChecker 不应有 ConstraintAssignmentInfo"
    );
}

#[test]
fn test_assignment_constraint_info_reset_on_non_constraint() {
    // Arrange — 先执行一次约束赋值，再执行普通赋值
    let mut checker = AssignmentChecker::new();
    let constraint = make_drawable_interface();
    let circle = make_circle_implementing_drawable();
    let span = dummy_span();

    // 第一次赋值：约束赋值
    checker
        .check_assignment(&constraint, &circle, span, None)
        .expect("约束赋值应成功");
    assert!(
        checker.last_constraint_info().is_some(),
        "约束赋值后应有 ConstraintAssignmentInfo"
    );

    // Act — 第二次赋值：普通赋值（非约束）
    let result = checker.check_assignment(&MonoType::Int(32), &MonoType::Int(32), span, None);

    // Assert
    assert!(result.is_ok(), "普通赋值应成功");
    assert!(
        checker.last_constraint_info().is_none(),
        "非约束赋值后应清除 ConstraintAssignmentInfo"
    );
}
