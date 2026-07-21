//! TypeChecker 测试 — 基于语言规范 §3 & RFC-010/011
//!
//! §3.1-§3.17: 类型系统检查
//! §6: 函数定义检查
//! RFC-010: 统一类型语法
//! RFC-011: 泛型系统设计

use crate::frontend::core::typecheck::checker::TypeChecker;
use crate::frontend::core::types::{MonoType, PolyType};
use crate::frontend::core::parser::ast::{Module, Stmt, Expr, Type as AstType};
use crate::util::span::Span;

// ===================================================================
// Happy path 测试
// ===================================================================

#[test]
fn test_type_checker_new_creates_with_module_name() {
    // Arrange & Act
    let checker = TypeChecker::new("test_module");

    // Assert
    assert_eq!(checker.module_name(), "test_module");
}

#[test]
fn test_type_checker_has_builtin_types() {
    // Arrange
    let mut checker = TypeChecker::new("test");

    // Act
    let env = checker.env();

    // Assert - 检查内置类型是否存在
    assert!(env.types.contains_key("int"), "should have int type");
    assert!(env.types.contains_key("float"), "should have float type");
    assert!(env.types.contains_key("bool"), "should have bool type");
    assert!(env.types.contains_key("string"), "should have string type");
    assert!(env.types.contains_key("void"), "should have void type");
}

#[test]
fn test_type_checker_can_add_var() {
    // Arrange
    let mut checker = TypeChecker::new("test");

    // Act
    checker
        .env()
        .add_var("x".to_string(), PolyType::mono(MonoType::Int(32)));

    // Assert
    let env = checker.env();
    assert!(env.vars.contains_key("x"), "should have variable x");
}

#[test]
fn test_type_checker_has_no_errors_initially() {
    // Arrange & Act
    let checker = TypeChecker::new("test");

    // Assert
    assert!(!checker.has_errors(), "should have no errors initially");
}

#[test]
fn test_type_checker_check_empty_module() {
    // Arrange
    let mut checker = TypeChecker::new("test");
    let module = Module {
        items: vec![],
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);

    // Assert
    assert!(
        result.diagnostics.is_empty(),
        "empty module should pass type check"
    );
}

// ===================================================================
// Error path 测试
// ===================================================================

#[test]
fn test_type_checker_reports_type_mismatch() {
    // Arrange
    let mut checker = TypeChecker::new("test");

    // 构造一个类型不匹配的 AST：将 Int 赋值给 String 变量
    let module = Module {
        items: vec![Stmt {
            kind: crate::frontend::core::parser::ast::StmtKind::Var {
                name: "x".to_string(),
                name_span: Span::dummy(),
                type_annotation: Some(AstType::String),
                initializer: Some(Box::new(Expr::Lit(
                    crate::frontend::core::lexer::tokens::Literal::Int(42),
                    Span::dummy(),
                ))),
                is_mut: false,
            },
            span: Span::dummy(),
        }],
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);

    // Assert - 应该报告类型错误
    assert!(
        !result.diagnostics.is_empty() || checker.has_errors(),
        "x: String = 42 should report type mismatch error"
    );
    let has_type_error = !result.diagnostics.is_empty()
        || checker.errors().iter().any(|e| {
            let msg = format!("{:?}", e);
            msg.contains("mismatch") || msg.contains("type")
        });
    assert!(has_type_error, "error should be related to type mismatch");
}

#[test]
fn test_type_checker_reports_undefined_variable() {
    // Arrange
    let mut checker = TypeChecker::new("test");
    let module = Module {
        items: vec![Stmt {
            kind: crate::frontend::core::parser::ast::StmtKind::Expr(Box::new(Expr::Var(
                "undefined_var".to_string(),
                Span::dummy(),
            ))),
            span: Span::dummy(),
        }],
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);

    // Assert - 使用未定义变量应该报错
    assert!(
        !result.diagnostics.is_empty() || checker.has_errors(),
        "using undefined variable should produce an error"
    );
}

#[test]
fn test_type_checker_reports_fn_param_type_mismatch() {
    // Arrange: 定义 add: (x: Int) -> Int = x，然后调用 add("hello")
    let mut checker = TypeChecker::new("test");
    let module = Module {
        items: vec![
            // fn add(x: Int) -> Int { x }
            Stmt {
                kind: crate::frontend::core::parser::ast::StmtKind::Binding {
                    name: "add".to_string(),
                    type_name: None,
                    method_type: None,
                    signature_params: vec![],
                    type_annotation: Some(AstType::Fn {
                        params: vec![AstType::Int(32)],
                        return_type: Box::new(AstType::Int(32)),
                    }),

                    params: vec![crate::frontend::core::parser::ast::Param {
                        name: "x".to_string(),
                        ty: Some(AstType::Int(32)),
                        is_mut: false,
                        span: Span::dummy(),
                    }],
                    body: vec![Stmt {
                        kind: crate::frontend::core::parser::ast::StmtKind::Expr(Box::new(
                            Expr::Var("x".to_string(), Span::dummy()),
                        )),
                        span: Span::dummy(),
                    }],
                    is_pub: false,
                },
                span: Span::dummy(),
            },
            // add("hello") — 传入 String 但参数期望 Int
            Stmt {
                kind: crate::frontend::core::parser::ast::StmtKind::Expr(Box::new(Expr::Call {
                    func: Box::new(Expr::Var("add".to_string(), Span::dummy())),
                    args: vec![Expr::Lit(
                        crate::frontend::core::lexer::tokens::Literal::String("hello".to_string()),
                        Span::dummy(),
                    )],
                    named_args: vec![],
                    span: Span::dummy(),
                })),
                span: Span::dummy(),
            },
        ],
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);

    // Assert - 规范 §6.3: 函数参数类型必须匹配，传入 String 但参数期望 Int 应报错
    assert!(
        !result.diagnostics.is_empty() || checker.has_errors(),
        "add(\"hello\") with param type Int should report type mismatch"
    );
}

// ===================================================================
// Boundary 测试
// ===================================================================

#[test]
fn test_type_checker_with_large_module() {
    // Arrange
    let mut checker = TypeChecker::new("test");
    let mut items = vec![];
    for i in 0..100 {
        // 添加大量语句
        items.push(Stmt {
            kind: crate::frontend::core::parser::ast::StmtKind::Expr(Box::new(Expr::Var(
                format!("var_{}", i),
                Span::dummy(),
            ))),
            span: Span::dummy(),
        });
    }
    let module = Module {
        items,
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);

    // Assert - 100 个未定义变量的语句应报错（规范 §6.3: 使用未定义变量是编译错误）
    assert!(
        !result.diagnostics.is_empty() || checker.has_errors(),
        "100 undefined variables should produce errors"
    );
}

#[test]
fn test_type_checker_with_multiple_function_definitions() {
    // Arrange: 定义三个函数
    let mut checker = TypeChecker::new("test");
    let make_fn_binding = |name: &str| -> Stmt {
        Stmt {
            kind: crate::frontend::core::parser::ast::StmtKind::Binding {
                name: name.to_string(),
                type_name: None,
                method_type: None,
                signature_params: vec![],
                type_annotation: Some(AstType::Fn {
                    params: vec![AstType::Int(32)],
                    return_type: Box::new(AstType::Int(32)),
                }),

                params: vec![crate::frontend::core::parser::ast::Param {
                    name: "x".to_string(),
                    ty: Some(AstType::Int(32)),
                    is_mut: false,
                    span: Span::dummy(),
                }],
                body: vec![Stmt {
                    kind: crate::frontend::core::parser::ast::StmtKind::Expr(Box::new(Expr::Var(
                        "x".to_string(),
                        Span::dummy(),
                    ))),
                    span: Span::dummy(),
                }],
                is_pub: false,
            },
            span: Span::dummy(),
        }
    };
    let module = Module {
        items: vec![
            make_fn_binding("add"),
            make_fn_binding("sub"),
            make_fn_binding("mul"),
        ],
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);

    // Assert - 多个函数定义应该正常处理
    assert!(
        result.diagnostics.is_empty(),
        "multiple function definitions should be accepted"
    );
}

#[test]
fn test_type_checker_with_nested_function_definition() {
    // Arrange: 外层函数内部定义一个 FnDef 表达式
    let mut checker = TypeChecker::new("test");
    let inner_fn = Expr::FnDef {
        name: "inner".to_string(),
        params: vec![crate::frontend::core::parser::ast::Param {
            name: "y".to_string(),
            ty: Some(AstType::Int(32)),
            is_mut: false,
            span: Span::dummy(),
        }],
        return_type: Some(AstType::Int(32)),
        body: Box::new(crate::frontend::core::parser::ast::Block {
            stmts: vec![Stmt {
                kind: crate::frontend::core::parser::ast::StmtKind::Expr(Box::new(Expr::Var(
                    "y".to_string(),
                    Span::dummy(),
                ))),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        }),
        span: Span::dummy(),
    };
    let module = Module {
        items: vec![Stmt {
            kind: crate::frontend::core::parser::ast::StmtKind::Binding {
                name: "outer".to_string(),
                type_name: None,
                method_type: None,
                signature_params: vec![],
                type_annotation: Some(AstType::Fn {
                    params: vec![AstType::Int(32)],
                    return_type: Box::new(AstType::Int(32)),
                }),

                params: vec![crate::frontend::core::parser::ast::Param {
                    name: "x".to_string(),
                    ty: Some(AstType::Int(32)),
                    is_mut: false,
                    span: Span::dummy(),
                }],
                body: vec![
                    Stmt {
                        kind: crate::frontend::core::parser::ast::StmtKind::Expr(Box::new(
                            inner_fn,
                        )),
                        span: Span::dummy(),
                    },
                    Stmt {
                        kind: crate::frontend::core::parser::ast::StmtKind::Expr(Box::new(
                            Expr::Var("x".to_string(), Span::dummy()),
                        )),
                        span: Span::dummy(),
                    },
                ],
                is_pub: false,
            },
            span: Span::dummy(),
        }],
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);

    // Assert - 规范 §6.3: 嵌套函数定义应通过类型检查（所有变量已定义，类型一致）
    assert!(
        result.diagnostics.is_empty(),
        "nested function definition with correct types should pass type check"
    );
}

#[test]
fn test_type_checker_with_generic_type_binding() {
    // Arrange: 定义泛型类型 Wrapper[T] = { value: T }，然后使用 Wrapper<Int>
    let mut checker = TypeChecker::new("test");
    let module = Module {
        items: vec![
            // Wrapper: Type = { value: T }  (泛型类型定义)
            Stmt {
                kind: crate::frontend::core::parser::ast::StmtKind::Binding {
                    name: "Wrapper".to_string(),
                    type_name: None,
                    method_type: None,
                    signature_params: vec![crate::frontend::core::parser::ast::Param {
                        name: "T".to_string(),
                        ty: Some(crate::frontend::core::parser::ast::Type::MetaType {
                            name_span: crate::util::span::Span::dummy(),
                            args: vec![],
                        }),
                        is_mut: false,
                        span: crate::util::span::Span::dummy(),
                    }],
                    type_annotation: Some(AstType::Struct {
                        body: vec![crate::frontend::core::parser::ast::TypeBodyItem::Field(
                            crate::frontend::core::parser::ast::StructField {
                                name: "value".to_string(),
                                is_mut: false,
                                ty: AstType::Name {
                                    name: "T".to_string(),
                                    span: Span::dummy(),
                                },
                                default: None,
                            },
                        )],
                    }),

                    params: vec![],
                    body: vec![],
                    is_pub: false,
                },
                span: Span::dummy(),
            },
            // let w: Wrapper<Int> = ...  (使用泛型类型)
            Stmt {
                kind: crate::frontend::core::parser::ast::StmtKind::Var {
                    name: "w".to_string(),
                    name_span: Span::dummy(),
                    type_annotation: Some(AstType::Generic {
                        name: "Wrapper".to_string(),
                        name_span: Span::dummy(),
                        args: vec![AstType::Int(32)],
                    }),
                    initializer: None,
                    is_mut: false,
                },
                span: Span::dummy(),
            },
        ],
        span: Span::dummy(),
    };

    // Act
    let result = checker.check_module(&module);

    // Assert - 规范 §3.8: 泛型类型定义和使用应通过类型检查（所有类型参数已提供）
    assert!(
        result.diagnostics.is_empty(),
        "generic type definition and usage with all type params provided should pass"
    );
}
