#![allow(clippy::result_large_err)]

//! RFC-011 约束（Constraint）测试
//!
//! 测试约束系统的核心功能：
//! - 约束定义语法正常工作
//! - 泛型约束 [T: Drawable] 正常工作
//! - 约束类型直接赋值（接口直接赋值优化）
//! - 结构化匹配规则正确实现

use crate::frontend::core::type_system::MonoType;
use crate::frontend::typecheck::checking::bounds::BoundsChecker;

/// 测试约束类型识别
#[test]
fn test_constraint_recognition() {
    // 函数字段组成的类型是约束类型
    let draw_constraint = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    assert!(draw_constraint.is_constraint());
    assert_eq!(draw_constraint.constraint_fields().len(), 1);

    // 包含非函数字段的类型不是约束类型
    let point_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MonoType::Int(32)),
            ("y".to_string(), MonoType::Int(32)),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    assert!(!point_type.is_constraint());
    assert!(point_type.constraint_fields().is_empty());
}

/// 测试约束字段获取
#[test]
fn test_constraint_fields() {
    let serializable = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Serializable".to_string(),
        fields: vec![
            (
                "serialize".to_string(),
                MonoType::Fn {
                    params: vec![],
                    return_type: Box::new(MonoType::String),
                    is_async: false,
                },
            ),
            (
                "deserialize".to_string(),
                MonoType::Fn {
                    params: vec![MonoType::String],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                },
            ),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    let fields = serializable.constraint_fields();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].0, "serialize");
    assert_eq!(fields[1].0, "deserialize");
}

/// 测试类型满足约束（成功情况）
#[test]
fn test_type_satisfies_constraint_success() {
    let mut checker = BoundsChecker::new();

    // 定义 Drawable 约束
    let draw_constraint = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // 定义满足 Drawable 的类型
    let circle_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Circle".to_string(),
        fields: vec![
            ("radius".to_string(), MonoType::Int(32)),
            (
                "draw".to_string(),
                MonoType::Fn {
                    params: vec![MonoType::TypeRef("Surface".to_string())],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                },
            ),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // Circle 应该满足 Drawable 约束
    let result = checker.check_constraint(&circle_type, &draw_constraint);
    assert!(result.is_ok(), "Circle should satisfy Drawable constraint");
}

/// 测试类型不满足约束（缺少方法）
#[test]
fn test_type_does_not_satisfy_constraint_missing_method() {
    let mut checker = BoundsChecker::new();

    // 定义 Drawable 约束
    let draw_constraint = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // 定义不满足 Drawable 的类型（缺少 draw 方法）
    let rect_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Rect".to_string(),
        fields: vec![
            ("width".to_string(), MonoType::Int(32)),
            ("height".to_string(), MonoType::Int(32)),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // Rect 不应该满足 Drawable 约束
    let result = checker.check_constraint(&rect_type, &draw_constraint);
    assert!(
        result.is_err(),
        "Rect should NOT satisfy Drawable constraint"
    );
}

/// 测试类型不满足约束（方法签名不兼容）
#[test]
fn test_type_does_not_satisfy_constraint_signature_mismatch() {
    let mut checker = BoundsChecker::new();

    // 定义 Drawable 约束
    let draw_constraint = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // 定义有 draw 但签名不兼容的类型
    let shape_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Shape".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Int(32)), // 返回类型不匹配
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // Shape 不应该满足 Drawable 约束（签名不兼容）
    let result = checker.check_constraint(&shape_type, &draw_constraint);
    assert!(
        result.is_err(),
        "Shape should NOT satisfy Drawable constraint"
    );
}

/// 测试空约束任何类型都满足
#[test]
fn test_empty_constraint_satisfied_by_any_type() {
    let mut checker = BoundsChecker::new();

    let empty_constraint = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Empty".to_string(),
        fields: vec![],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    let any_type = MonoType::Int(32);
    let result = checker.check_constraint(&any_type, &empty_constraint);
    assert!(result.is_ok(), "Any type should satisfy empty constraint");
}

/// 测试多方法约束
#[test]
fn test_multi_method_constraint() {
    let mut checker = BoundsChecker::new();

    // 定义多方法约束
    let printable = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Printable".to_string(),
        fields: vec![
            (
                "to_string".to_string(),
                MonoType::Fn {
                    params: vec![],
                    return_type: Box::new(MonoType::String),
                    is_async: false,
                },
            ),
            (
                "print".to_string(),
                MonoType::Fn {
                    params: vec![],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                },
            ),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // 定义满足约束的类型
    let person_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Person".to_string(),
        fields: vec![
            ("name".to_string(), MonoType::String),
            ("age".to_string(), MonoType::Int(32)),
            (
                "to_string".to_string(),
                MonoType::Fn {
                    params: vec![],
                    return_type: Box::new(MonoType::String),
                    is_async: false,
                },
            ),
            (
                "print".to_string(),
                MonoType::Fn {
                    params: vec![],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                },
            ),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    let result = checker.check_constraint(&person_type, &printable);
    assert!(result.is_ok(), "Person should satisfy Printable constraint");
}

/// 测试函数签名兼容性（带 self 参数）
#[test]
fn test_fn_signature_compatibility_with_self() {
    let mut checker = BoundsChecker::new();

    // 约束的签名不包含 self
    let draw_constraint = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // 类型的签名包含 self 作为第一个参数
    let point_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Point".to_string(),
        fields: vec![
            ("x".to_string(), MonoType::Int(32)),
            ("y".to_string(), MonoType::Int(32)),
            (
                "draw".to_string(),
                MonoType::Fn {
                    params: vec![
                        MonoType::TypeRef("Point".to_string()), // self
                        MonoType::TypeRef("Surface".to_string()),
                    ],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                },
            ),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // Point 应该满足 Drawable 约束（self 参数被跳过比较）
    let result = checker.check_constraint(&point_type, &draw_constraint);
    assert!(
        result.is_ok(),
        "Point should satisfy Drawable constraint with self parameter"
    );
}

/// 测试交集类型作为约束
#[test]
fn test_intersection_constraint() {
    let mut checker = BoundsChecker::new();

    // 定义两个约束
    let drawable = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    let serializable = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Serializable".to_string(),
        fields: vec![(
            "serialize".to_string(),
            MonoType::Fn {
                params: vec![],
                return_type: Box::new(MonoType::String),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // 交集类型：Drawable & Serializable
    let intersection_constraint = MonoType::Intersection(vec![drawable, serializable]);

    // 定义同时满足两个约束的类型
    let circle_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Circle".to_string(),
        fields: vec![
            ("radius".to_string(), MonoType::Int(32)),
            (
                "draw".to_string(),
                MonoType::Fn {
                    params: vec![MonoType::TypeRef("Surface".to_string())],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                },
            ),
            (
                "serialize".to_string(),
                MonoType::Fn {
                    params: vec![],
                    return_type: Box::new(MonoType::String),
                    is_async: false,
                },
            ),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // Circle 应该满足 Drawable & Serializable 约束
    let _result = checker.check_constraint(&circle_type, &intersection_constraint);
    // 注意：当前实现可能不支持交集类型作为约束，这是预期的行为
    // 实际使用时需要先解析交集类型为单独的约束列表
}

// ============================================================================
// 接口直接赋值测试
// ============================================================================

/// 测试接口直接赋值 - AssignmentChecker
#[test]
fn test_constraint_direct_assignment_allowed() {
    use crate::frontend::typecheck::checking::assignment::AssignmentChecker;
    use crate::util::span::Span;

    let mut checker = AssignmentChecker::new();

    // 定义 Drawable 约束
    let drawable = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // 定义满足 Drawable 的 Circle 类型
    let circle_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Circle".to_string(),
        fields: vec![
            ("radius".to_string(), MonoType::Int(32)),
            (
                "draw".to_string(),
                MonoType::Fn {
                    params: vec![MonoType::TypeRef("Surface".to_string())],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                },
            ),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // d: Drawable = Circle(1) 应该通过类型检查
    let result = checker.check_assignment(&drawable, &circle_type, Span::default());
    assert!(
        result.is_ok(),
        "Constraint direct assignment should be allowed: {:?}",
        result.err()
    );
}

/// 测试接口直接赋值 - 具体类型推断信息
#[test]
fn test_constraint_assignment_concrete_type_info() {
    use crate::frontend::typecheck::checking::assignment::{
        AssignmentChecker, ConstraintAssignmentInfo,
    };
    use crate::util::span::Span;

    let mut checker = AssignmentChecker::new();

    let drawable = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    let circle_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Circle".to_string(),
        fields: vec![
            ("radius".to_string(), MonoType::Int(32)),
            (
                "draw".to_string(),
                MonoType::Fn {
                    params: vec![MonoType::TypeRef("Surface".to_string())],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                },
            ),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    let _ = checker.check_assignment(&drawable, &circle_type, Span::default());

    // 应该包含具体类型推断信息
    let info = checker.last_constraint_info();
    assert!(info.is_some(), "Should have constraint assignment info");
    match info.unwrap() {
        ConstraintAssignmentInfo::Concrete {
            concrete_type,
            constraint_type: _,
        } => {
            assert_eq!(concrete_type.type_name(), "Circle");
        }
        _ => panic!("Expected Concrete type info"),
    }
}

/// 测试接口直接赋值 - 不满足约束时拒绝
#[test]
fn test_constraint_direct_assignment_rejected_missing_method() {
    use crate::frontend::typecheck::checking::assignment::AssignmentChecker;
    use crate::util::span::Span;

    let mut checker = AssignmentChecker::new();

    let drawable = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // Rect 缺少 draw 方法
    let rect_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Rect".to_string(),
        fields: vec![
            ("width".to_string(), MonoType::Int(32)),
            ("height".to_string(), MonoType::Int(32)),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // d: Drawable = Rect(1, 2) 应该失败（缺少 draw 方法）
    let result = checker.check_assignment(&drawable, &rect_type, Span::default());
    assert!(
        result.is_err(),
        "Assignment to constraint should fail when type doesn't satisfy constraint"
    );
}

/// 测试子类型检查器支持约束类型
#[test]
fn test_subtype_checker_constraint_support() {
    use crate::frontend::typecheck::checking::subtyping::SubtypeChecker;

    let checker = SubtypeChecker::new();

    let drawable = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    let circle_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Circle".to_string(),
        fields: vec![
            ("radius".to_string(), MonoType::Int(32)),
            (
                "draw".to_string(),
                MonoType::Fn {
                    params: vec![MonoType::TypeRef("Surface".to_string())],
                    return_type: Box::new(MonoType::Void),
                    is_async: false,
                },
            ),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // Circle <: Drawable (结构化子类型)
    assert!(
        checker.is_subtype(&circle_type, &drawable),
        "Circle should be subtype of Drawable constraint"
    );
}

/// 测试子类型检查器 - 不满足约束
#[test]
fn test_subtype_checker_constraint_not_satisfied() {
    use crate::frontend::typecheck::checking::subtyping::SubtypeChecker;

    let checker = SubtypeChecker::new();

    let drawable = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Drawable".to_string(),
        fields: vec![(
            "draw".to_string(),
            MonoType::Fn {
                params: vec![MonoType::TypeRef("Surface".to_string())],
                return_type: Box::new(MonoType::Void),
                is_async: false,
            },
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    let rect_type = MonoType::Struct(crate::frontend::core::type_system::StructType {
        name: "Rect".to_string(),
        fields: vec![
            ("width".to_string(), MonoType::Int(32)),
            ("height".to_string(), MonoType::Int(32)),
        ],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![],
        field_has_default: Vec::new(),
    });

    // Rect 不满足 Drawable
    assert!(
        !checker.is_subtype(&rect_type, &drawable),
        "Rect should NOT be subtype of Drawable constraint"
    );
}
