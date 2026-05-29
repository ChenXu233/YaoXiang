//! 实现检查测试 — 基于语言规范 §3.5 & RFC-010 / RFC-011 §2
//!
//! §3.5: 接口类型
//! RFC-010: 接口约束系统
//! RFC-011 §2: 类型约束系统

use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::parser::ast::{Expr, Param, StmtKind, Type};
use crate::frontend::core::typecheck::traits::impl_check::{TraitImplChecker, TraitImplError};
use crate::frontend::core::types::base::trait_data::{
    TraitDefinition, TraitMethodSignature, TraitTable,
};
use crate::frontend::core::types::base::MonoType;
use crate::util::span::{Position, Span};

// ===================================================================
// 辅助函数
// ===================================================================

/// 创建一个 dummy Span 用于测试
fn dummy_span() -> Span {
    Span {
        start: Position::dummy(),
        end: Position::dummy(),
    }
}

/// 创建 TraitDefinition
fn make_trait_def(
    name: &str,
    methods: Vec<(&str, bool)>,
) -> TraitDefinition {
    let methods_map = methods
        .into_iter()
        .map(|(n, is_static)| {
            (
                n.to_string(),
                TraitMethodSignature {
                    name: n.to_string(),
                    params: vec![],
                    return_type: MonoType::Void,
                    is_static,
                },
            )
        })
        .collect();
    TraitDefinition {
        name: name.to_string(),
        methods: methods_map,
        parent_traits: vec![],
        generic_params: vec![],
        span: None,
        is_marker: false,
    }
}

/// 创建 StmtKind::Binding 方法绑定（type_name = Some）
fn make_method_bind_stmt(
    name: &str,
    type_name: &str,
    params: Vec<Param>,
) -> StmtKind {
    StmtKind::Binding {
        name: name.to_string(),
        type_name: Some(type_name.to_string()),
        method_type: None,
        generic_params: vec![],
        type_annotation: None,
        eval: None,
        params,
        body: (vec![], None),
        is_pub: false,
    }
}

/// 创建一个简单的 StmtKind::Expr 语句（非 Binding 变体）
fn make_expr_stmt() -> StmtKind {
    StmtKind::Expr(Box::new(Expr::Lit(Literal::Int(42), dummy_span())))
}

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_check_method_bind_valid_binding() {
    // Arrange: trait "clone" 已定义且包含 "clone" 方法
    let mut trait_table = TraitTable::default();
    trait_table.add_trait(make_trait_def("clone", vec![("clone", false)]));
    let checker = TraitImplChecker::new(&trait_table);
    let stmt = make_method_bind_stmt("clone", "Point", vec![]);

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(
        result.is_ok(),
        "有效的方法绑定应通过检查，实际错误: {:?}",
        result.err()
    );
}

#[test]
fn test_check_method_bind_with_params() {
    // Arrange: trait "draw" 带有 "draw" 方法，绑定包含参数
    let mut trait_table = TraitTable::default();
    trait_table.add_trait(make_trait_def("draw", vec![("draw", false)]));
    let checker = TraitImplChecker::new(&trait_table);
    let params = vec![Param {
        name: "self".to_string(),
        ty: Some(Type::Name {
            name: "Canvas".to_string(),
            span: dummy_span(),
        }),
        is_mut: false,
        span: dummy_span(),
    }];
    let stmt = make_method_bind_stmt("draw", "Shape", params);

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(
        result.is_ok(),
        "带参数的方法绑定应通过检查，实际错误: {:?}",
        result.err()
    );
}

#[test]
fn test_check_method_bind_static_method() {
    // Arrange: trait 定义了静态方法，绑定实现该静态方法
    let mut trait_table = TraitTable::default();
    trait_table.add_trait(make_trait_def("from_str", vec![("from_str", true)]));
    let checker = TraitImplChecker::new(&trait_table);
    let stmt = make_method_bind_stmt("from_str", "Point", vec![]);

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(
        result.is_ok(),
        "静态方法绑定应通过检查，实际错误: {:?}",
        result.err()
    );
}

#[test]
fn test_check_method_bind_trait_with_multiple_methods() {
    // Arrange: trait 定义了多个方法，绑定实现其中一个
    let mut trait_table = TraitTable::default();
    trait_table.add_trait(make_trait_def(
        "display",
        vec![("display", false), ("fmt", false)],
    ));
    let checker = TraitImplChecker::new(&trait_table);
    let stmt = make_method_bind_stmt("display", "Point", vec![]);

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(
        result.is_ok(),
        "多方法 trait 中实现其中一个应通过检查，实际错误: {:?}",
        result.err()
    );
}

#[test]
fn test_check_method_bind_multiple_impls_same_trait() {
    // Arrange: 对同一 trait ("eq") 的绑定多次检查
    // 注意：check_method_bind 将 binding name 用作 trait table 的查找 key，
    // 因此多次绑定同一名称应产生一致结果
    let mut trait_table = TraitTable::default();
    trait_table.add_trait(make_trait_def("eq", vec![("eq", false)]));
    let checker = TraitImplChecker::new(&trait_table);
    let stmt1 = make_method_bind_stmt("eq", "Point", vec![]);
    let stmt2 = make_method_bind_stmt("eq", "Line", vec![]);

    // Act
    let result1 = checker.check_method_bind(&stmt1);
    let result2 = checker.check_method_bind(&stmt2);

    // Assert
    assert!(
        result1.is_ok(),
        "第一次实现 eq 方法应通过检查，实际错误: {:?}",
        result1.err()
    );
    assert!(
        result2.is_ok(),
        "第二次实现 eq 方法应通过检查，实际错误: {:?}",
        result2.err()
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_check_method_bind_non_binding_stmt_returns_error() {
    // Arrange: 传入非 Binding 的 StmtKind
    let trait_table = TraitTable::default();
    let checker = TraitImplChecker::new(&trait_table);
    let stmt = make_expr_stmt();

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(result.is_err(), "非 Binding 语句应返回错误");
    assert!(
        matches!(result.unwrap_err(), TraitImplError::InvalidMethodBind),
        "非 Binding 语句应返回 InvalidMethodBind 错误"
    );
}

#[test]
fn test_check_method_bind_no_type_name_returns_error() {
    // Arrange: type_name 为 None 的 Binding 不匹配方法绑定模式
    let trait_table = TraitTable::default();
    let checker = TraitImplChecker::new(&trait_table);
    let stmt = StmtKind::Binding {
        name: "clone".to_string(),
        type_name: None, // 没有类型名称
        method_type: None,
        generic_params: vec![],
        type_annotation: None,
        eval: None,
        params: vec![],
        body: (vec![], None),
        is_pub: false,
    };

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(result.is_err(), "type_name 为 None 的 Binding 应返回错误");
    assert!(
        matches!(result.unwrap_err(), TraitImplError::InvalidMethodBind),
        "type_name 为 None 时应返回 InvalidMethodBind 错误"
    );
}

#[test]
fn test_check_method_bind_trait_not_found() {
    // Arrange: trait 表为空，方法绑定引用不存在的 trait
    let trait_table = TraitTable::default();
    let checker = TraitImplChecker::new(&trait_table);
    let stmt = make_method_bind_stmt("clone", "Point", vec![]);

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(result.is_err(), "引用不存在的 trait 应返回错误");
    match result.unwrap_err() {
        TraitImplError::TraitNotFound { trait_name } => {
            assert_eq!(
                trait_name, "clone",
                "TraitNotFound 错误中的 trait 名称应为 'clone'"
            );
        }
        other => panic!("应返回 TraitNotFound 错误，实际: {:?}", other),
    }
}

#[test]
fn test_check_method_bind_method_not_required_by_trait() {
    // Arrange: trait "display" 存在但不包含 "to_string" 方法
    let mut trait_table = TraitTable::default();
    trait_table.add_trait(make_trait_def("display", vec![("fmt", false)]));
    let checker = TraitImplChecker::new(&trait_table);
    let stmt = make_method_bind_stmt("to_string", "Point", vec![]);

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(result.is_err(), "trait 中不包含的方法应返回错误");
    match result.unwrap_err() {
        TraitImplError::TraitNotFound { trait_name } => {
            assert_eq!(
                trait_name, "to_string",
                "TraitNotFound 错误中应报告正确的 trait 名称"
            );
        }
        other => panic!("应返回 TraitNotFound 错误，实际: {:?}", other),
    }
}

#[test]
fn test_check_method_bind_method_exists_but_not_in_trait_methods() {
    // Arrange: trait "clone" 存在，但其 methods 中不含 "clone" 键
    // （仅含 "deep_clone"），绑定尝试实现 "clone"
    // 注意：get_trait 查找的是 trait table 的 key，而 check_required_methods_simple
    // 检查的是 trait_def.methods 中是否包含 method_name
    // 此处 trait table key = "deep_clone"，但 Binding name = "clone"
    // 因此 get_trait("clone") 返回 None，触发 TraitNotFound
    // 要触发 NotRequiredMethod，需要 trait table key == method_name，但 methods 中不含该 key
    let mut trait_table = TraitTable::default();
    let trait_def = make_trait_def("clone", vec![("deep_clone", false)]);
    // trait_def.name = "clone"，methods = {"deep_clone": ...}
    // add_trait 将以 "clone" 为 key 存入 trait table
    trait_table.add_trait(trait_def);
    let checker = TraitImplChecker::new(&trait_table);
    // Binding name = "clone" -> get_trait("clone") 找到定义
    // -> check_required_methods_simple: methods.contains_key("clone") = false
    let stmt = make_method_bind_stmt("clone", "Point", vec![]);

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(result.is_err(), "方法不在 trait 的 methods 中应返回错误");
    match result.unwrap_err() {
        TraitImplError::NotRequiredMethod {
            trait_name,
            method_name,
        } => {
            assert_eq!(trait_name, "clone", "错误中 trait 名称应为 'clone'");
            assert_eq!(method_name, "clone", "错误中方法名称应为 'clone'");
        }
        other => panic!("应返回 NotRequiredMethod 错误，实际: {:?}", other),
    }
}

#[test]
fn test_check_method_bind_empty_trait_table() {
    // Arrange: 空 trait 表 + 多个绑定均应失败
    let trait_table = TraitTable::default();
    let checker = TraitImplChecker::new(&trait_table);

    let methods = ["clone", "display", "eq", "hash"];
    for method in &methods {
        let stmt = make_method_bind_stmt(method, "MyType", vec![]);

        // Act
        let result = checker.check_method_bind(&stmt);

        // Assert
        assert!(
            result.is_err(),
            "空 trait 表下绑定方法 '{}' 应返回错误",
            method
        );
    }
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_check_method_bind_empty_method_map_trait() {
    // Arrange: trait 存在但 methods 为空
    let mut trait_table = TraitTable::default();
    trait_table.add_trait(make_trait_def("empty_trait", vec![]));
    let checker = TraitImplChecker::new(&trait_table);
    let stmt = make_method_bind_stmt("empty_trait", "Point", vec![]);

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(
        result.is_err(),
        "methods 为空的 trait 绑定应返回 NotRequiredMethod 错误"
    );
    assert!(
        matches!(
            result.unwrap_err(),
            TraitImplError::NotRequiredMethod { .. }
        ),
        "应返回 NotRequiredMethod 错误变体"
    );
}

#[test]
fn test_check_method_bind_with_multiple_params() {
    // Arrange: 绑定包含多个参数
    let mut trait_table = TraitTable::default();
    trait_table.add_trait(make_trait_def("add", vec![("add", false)]));
    let checker = TraitImplChecker::new(&trait_table);
    let params = vec![
        Param {
            name: "self".to_string(),
            ty: Some(Type::Int(64)),
            is_mut: false,
            span: dummy_span(),
        },
        Param {
            name: "other".to_string(),
            ty: Some(Type::Int(64)),
            is_mut: false,
            span: dummy_span(),
        },
    ];
    let stmt = make_method_bind_stmt("add", "Int", params);

    // Act
    let result = checker.check_method_bind(&stmt);

    // Assert
    assert!(
        result.is_ok(),
        "多参数方法绑定应通过检查，实际错误: {:?}",
        result.err()
    );
}

#[test]
fn test_impl_error_display_trait_not_found() {
    // Arrange
    let err = TraitImplError::TraitNotFound {
        trait_name: "Clone".to_string(),
    };

    // Act
    let display = format!("{}", err);

    // Assert
    assert!(
        display.contains("Clone"),
        "TraitNotFound 的 Display 应包含 trait 名称，实际: {}",
        display
    );
    assert!(
        display.contains("not found"),
        "TraitNotFound 的 Display 应包含 'not found'，实际: {}",
        display
    );
}

#[test]
fn test_impl_error_display_missing_method() {
    // Arrange
    let err = TraitImplError::MissingMethod {
        trait_name: "Clone".to_string(),
        methods: vec!["clone".to_string(), "deep_copy".to_string()],
    };

    // Act
    let display = format!("{}", err);

    // Assert
    assert!(
        display.contains("clone") && display.contains("deep_copy"),
        "MissingMethod 的 Display 应包含缺失方法名，实际: {}",
        display
    );
    assert!(
        display.contains("Clone"),
        "MissingMethod 的 Display 应包含 trait 名称，实际: {}",
        display
    );
}

#[test]
fn test_impl_error_display_signature_mismatch() {
    // Arrange
    let err = TraitImplError::SignatureMismatch {
        method_name: "clone".to_string(),
        message: "return type mismatch".to_string(),
    };

    // Act
    let display = format!("{}", err);

    // Assert
    assert!(
        display.contains("clone"),
        "SignatureMismatch 的 Display 应包含方法名，实际: {}",
        display
    );
    assert!(
        display.contains("return type mismatch"),
        "SignatureMismatch 的 Display 应包含详细消息，实际: {}",
        display
    );
}

#[test]
fn test_impl_error_display_not_required_method() {
    // Arrange
    let err = TraitImplError::NotRequiredMethod {
        trait_name: "Display".to_string(),
        method_name: "nonexistent".to_string(),
    };

    // Act
    let display = format!("{}", err);

    // Assert
    assert!(
        display.contains("nonexistent"),
        "NotRequiredMethod 的 Display 应包含方法名，实际: {}",
        display
    );
    assert!(
        display.contains("Display"),
        "NotRequiredMethod 的 Display 应包含 trait 名称，实际: {}",
        display
    );
}

#[test]
fn test_impl_error_display_invalid_method_bind() {
    // Arrange
    let err = TraitImplError::InvalidMethodBind;

    // Act
    let display = format!("{}", err);

    // Assert
    assert!(
        display.contains("Invalid"),
        "InvalidMethodBind 的 Display 应包含 'Invalid'，实际: {}",
        display
    );
}

#[test]
fn test_impl_error_is_clone() {
    // Arrange
    let err = TraitImplError::TraitNotFound {
        trait_name: "Clone".to_string(),
    };

    // Act
    let cloned = err.clone();

    // Assert
    match cloned {
        TraitImplError::TraitNotFound { trait_name } => {
            assert_eq!(trait_name, "Clone", "克隆后的 TraitImplError 内容应一致");
        }
        other => panic!("克隆后类型应保持不变，实际: {:?}", other),
    }
}

#[test]
fn test_impl_error_debug_format() {
    // Arrange
    let err = TraitImplError::InvalidMethodBind;

    // Act
    let debug_str = format!("{:?}", err);

    // Assert
    assert!(
        debug_str.contains("InvalidMethodBind"),
        "TraitImplError 的 Debug 输出应包含变体名，实际: {}",
        debug_str
    );
}

#[test]
fn test_impl_checker_new_does_not_panic() {
    // Arrange & Act: 用各种大小的 trait 表创建 checker
    let empty_table = TraitTable::default();
    let _checker1 = TraitImplChecker::new(&empty_table);

    let mut populated_table = TraitTable::default();
    for i in 0..10 {
        populated_table.add_trait(make_trait_def(&format!("trait_{}", i), vec![]));
    }
    let _checker2 = TraitImplChecker::new(&populated_table);

    // Assert - 不 panic 即通过
}
