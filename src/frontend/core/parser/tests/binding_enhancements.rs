//! RFC-004 / RFC-010 后续可选增强测试
//!
//! 测试 6 项后续可选增强功能：
//! 1. 命名参数构造：Point(x=1, y=2)
//! 2. 负数索引绑定：func[-1]
//! 3. 默认绑定：Type.method = function（无 [positions]）
//! 4. 外部绑定语句：Type.method = function[pos]
//! 5. 接口约束：Type: Type = { fields, Interface1, Interface2 }
//! 6. 匿名函数 IR 生成（集成测试级别）

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::ast::{self, BindingKind, StmtKind};
use crate::frontend::core::parser::{parse, parse_expression};

// ======== Feature 1: 命名参数构造 ========

#[test]
fn test_named_args_parse_basic() {
    // Point(x=1, y=2, z=3)
    let expr = "foo(x=1, y=2, z=3)";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(
        result.is_ok(),
        "Failed to parse named args: {:?}",
        result.err()
    );
    let expr = result.unwrap();
    match expr {
        ast::Expr::Call {
            named_args, args, ..
        } => {
            assert!(
                args.is_empty(),
                "Positional args should be empty, got {}",
                args.len()
            );
            assert_eq!(named_args.len(), 3, "Expected 3 named args");
            assert_eq!(named_args[0].0, "x");
            assert_eq!(named_args[1].0, "y");
            assert_eq!(named_args[2].0, "z");
        }
        _ => panic!("Expected Call expression, got {:?}", expr),
    }
}

#[test]
fn test_named_args_mixed_with_positional() {
    // foo(1, y=2) — positional before named
    let expr = "foo(1, y=2)";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(
        result.is_ok(),
        "Failed to parse mixed args: {:?}",
        result.err()
    );
    let expr = result.unwrap();
    match expr {
        ast::Expr::Call {
            named_args, args, ..
        } => {
            assert_eq!(args.len(), 1, "Expected 1 positional arg");
            assert_eq!(named_args.len(), 1, "Expected 1 named arg");
            assert_eq!(named_args[0].0, "y");
        }
        _ => panic!("Expected Call expression"),
    }
}

#[test]
fn test_named_args_empty_call() {
    let expr = "foo()";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    let expr = result.unwrap();
    match expr {
        ast::Expr::Call {
            named_args, args, ..
        } => {
            assert!(args.is_empty());
            assert!(named_args.is_empty());
        }
        _ => panic!("Expected Call expression"),
    }
}

#[test]
fn test_named_args_single() {
    let expr = "create(name=42)";
    let tokens = tokenize(expr).unwrap();
    let result = parse_expression(&tokens);
    assert!(result.is_ok());
    let expr = result.unwrap();
    match expr {
        ast::Expr::Call {
            named_args, args, ..
        } => {
            assert!(args.is_empty());
            assert_eq!(named_args.len(), 1);
            assert_eq!(named_args[0].0, "name");
        }
        _ => panic!("Expected Call expression"),
    }
}

// ======== Feature 2: 负数索引绑定 ========

#[test]
fn test_negative_index_binding_in_struct() {
    // 结构体内的负索引绑定
    let source = r#"Point: Type = { x: Float, test = func[-1] }"#;
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        StmtKind::TypeDef {
            name, definition, ..
        } => {
            assert_eq!(name, "Point");
            match definition {
                ast::Type::Struct { bindings, .. } => {
                    assert_eq!(bindings.len(), 1);
                    assert_eq!(bindings[0].name, "test");
                    match &bindings[0].kind {
                        BindingKind::External {
                            function,
                            positions,
                        } => {
                            assert_eq!(function, "func");
                            assert_eq!(positions, &vec![-1i64]);
                        }
                        _ => panic!("Expected External binding, got {:?}", bindings[0].kind),
                    }
                }
                _ => panic!("Expected Struct type"),
            }
        }
        _ => panic!("Expected TypeDef"),
    }
}

#[test]
fn test_negative_index_binding_multiple() {
    // 多位置绑定含负索引
    let source = r#"Point: Type = { x: Float, calc = calculate[0, -1] }"#;
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::TypeDef { definition, .. } => match definition {
            ast::Type::Struct { bindings, .. } => {
                assert_eq!(bindings.len(), 1);
                match &bindings[0].kind {
                    BindingKind::External { positions, .. } => {
                        assert_eq!(positions, &vec![0i64, -1i64]);
                    }
                    _ => panic!("Expected External binding"),
                }
            }
            _ => panic!("Expected Struct type"),
        },
        _ => panic!("Expected TypeDef"),
    }
}

// ======== Feature 3: 默认绑定（无位置） ========

#[test]
fn test_default_binding_in_struct() {
    // 默认绑定：无 [positions]
    let source = r#"Point: Type = { x: Float, distance = calc_distance }"#;
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::TypeDef { definition, .. } => match definition {
            ast::Type::Struct { bindings, .. } => {
                assert_eq!(bindings.len(), 1);
                assert_eq!(bindings[0].name, "distance");
                match &bindings[0].kind {
                    BindingKind::DefaultExternal { function } => {
                        assert_eq!(function, "calc_distance");
                    }
                    _ => panic!(
                        "Expected DefaultExternal binding, got {:?}",
                        bindings[0].kind
                    ),
                }
            }
            _ => panic!("Expected Struct type"),
        },
        _ => panic!("Expected TypeDef"),
    }
}

#[test]
fn test_default_binding_vs_external_binding() {
    // 同时存在默认绑定和位置绑定
    let source = r#"Point: Type = { x: Float, auto_method = func, explicit_method = func2[0] }"#;
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::TypeDef { definition, .. } => match definition {
            ast::Type::Struct { bindings, .. } => {
                assert_eq!(bindings.len(), 2);
                assert!(matches!(
                    &bindings[0].kind,
                    BindingKind::DefaultExternal { .. }
                ));
                assert!(matches!(&bindings[1].kind, BindingKind::External { .. }));
            }
            _ => panic!("Expected Struct type"),
        },
        _ => panic!("Expected TypeDef"),
    }
}

// ======== Feature 4: 外部绑定语句 ========

#[test]
fn test_external_binding_stmt_basic() {
    // Point.distance = distance[0]
    let source = "Point.distance = distance[0]";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    assert_eq!(module.items.len(), 1);
    match &module.items[0].kind {
        StmtKind::ExternalBindingStmt {
            type_name,
            method_name,
            binding,
        } => {
            assert_eq!(type_name, "Point");
            assert_eq!(method_name, "distance");
            match binding {
                BindingKind::External {
                    function,
                    positions,
                } => {
                    assert_eq!(function, "distance");
                    assert_eq!(positions, &vec![0i64]);
                }
                _ => panic!("Expected External binding kind"),
            }
        }
        _ => panic!(
            "Expected ExternalBindingStmt, got {:?}",
            module.items[0].kind
        ),
    }
}

#[test]
fn test_external_binding_stmt_multiple_positions() {
    // Point.transform = transform[1, 2]
    let source = "Point.transform = transform[1, 2]";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::ExternalBindingStmt {
            type_name,
            method_name,
            binding,
        } => {
            assert_eq!(type_name, "Point");
            assert_eq!(method_name, "transform");
            match binding {
                BindingKind::External {
                    function,
                    positions,
                } => {
                    assert_eq!(function, "transform");
                    assert_eq!(positions, &vec![1i64, 2i64]);
                }
                _ => panic!("Expected External binding kind"),
            }
        }
        _ => panic!("Expected ExternalBindingStmt"),
    }
}

#[test]
fn test_external_binding_stmt_default() {
    // Point.distance = distance（默认绑定，无位置）
    let source = "Point.distance = distance";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::ExternalBindingStmt {
            type_name,
            method_name,
            binding,
        } => {
            assert_eq!(type_name, "Point");
            assert_eq!(method_name, "distance");
            match binding {
                BindingKind::DefaultExternal { function } => {
                    assert_eq!(function, "distance");
                }
                _ => panic!("Expected DefaultExternal binding kind, got {:?}", binding),
            }
        }
        _ => panic!("Expected ExternalBindingStmt"),
    }
}

#[test]
fn test_external_binding_stmt_negative_index() {
    // Point.test = func[-1]
    let source = "Point.test = func[-1]";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::ExternalBindingStmt {
            type_name,
            method_name,
            binding,
        } => {
            assert_eq!(type_name, "Point");
            assert_eq!(method_name, "test");
            match binding {
                BindingKind::External {
                    function,
                    positions,
                } => {
                    assert_eq!(function, "func");
                    assert_eq!(positions, &vec![-1i64]);
                }
                _ => panic!("Expected External binding kind"),
            }
        }
        _ => panic!("Expected ExternalBindingStmt"),
    }
}

// ======== Feature 5: 接口约束 ========

#[test]
fn test_interface_constraint_parsed() {
    // 接口约束被解析到 interfaces 字段
    let source = "Shape: Type = { x: Float, y: Float, Drawable, Serializable }";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::TypeDef { definition, .. } => match definition {
            ast::Type::Struct {
                fields, interfaces, ..
            } => {
                assert_eq!(fields.len(), 2);
                assert_eq!(interfaces.len(), 2);
                assert_eq!(interfaces[0], "Drawable");
                assert_eq!(interfaces[1], "Serializable");
            }
            _ => panic!("Expected Struct type"),
        },
        _ => panic!("Expected TypeDef"),
    }
}

#[test]
fn test_interface_constraint_only() {
    // 只有接口，没有字段
    let source = "EmptyType: Type = { Drawable, Serializable }";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::TypeDef { definition, .. } => match definition {
            ast::Type::Struct {
                fields, interfaces, ..
            } => {
                assert!(fields.is_empty());
                assert_eq!(interfaces.len(), 2);
                assert_eq!(interfaces[0], "Drawable");
                assert_eq!(interfaces[1], "Serializable");
            }
            _ => panic!("Expected Struct type"),
        },
        _ => panic!("Expected TypeDef"),
    }
}

#[test]
fn test_interface_constraint_with_binding() {
    // 接口约束 + 绑定
    let source = "Point: Type = { x: Float, distance = calc_distance[0], Drawable }";
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::TypeDef { definition, .. } => match definition {
            ast::Type::Struct {
                fields,
                bindings,
                interfaces,
            } => {
                assert_eq!(fields.len(), 1);
                assert_eq!(fields[0].name, "x");
                assert_eq!(bindings.len(), 1);
                assert_eq!(bindings[0].name, "distance");
                assert_eq!(interfaces.len(), 1);
                assert_eq!(interfaces[0], "Drawable");
            }
            _ => panic!("Expected Struct type"),
        },
        _ => panic!("Expected TypeDef"),
    }
}

// ======== Feature 6: 匿名函数 IR 生成 ========
// IR 生成测试通过端到端方式验证

#[test]
fn test_anonymous_binding_parses() {
    // 匿名函数绑定在结构体内部解析
    let source = r#"Point: Type = { x: Float, norm: ((p: Point) -> Float)[0] = ((p) => 0.0) }"#;
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::TypeDef { definition, .. } => {
            match definition {
                ast::Type::Struct {
                    fields, bindings, ..
                } => {
                    assert_eq!(fields.len(), 1);
                    assert_eq!(bindings.len(), 1);
                    assert_eq!(bindings[0].name, "norm");
                    match &bindings[0].kind {
                        BindingKind::Anonymous {
                            params, positions, ..
                        } => {
                            assert_eq!(params.len(), 1);
                            // 参数名从类型注解推断，Type::Fn 不保留参数名，所以生成 arg0
                            assert_eq!(params[0].name, "arg0");
                            assert_eq!(positions, &vec![0i64]);
                        }
                        _ => panic!("Expected Anonymous binding, got {:?}", bindings[0].kind),
                    }
                }
                _ => panic!("Expected Struct type"),
            }
        }
        _ => panic!("Expected TypeDef"),
    }
}

// ======== 综合测试 ========

#[test]
fn test_struct_with_all_binding_types() {
    // 综合测试：字段 + 外部绑定 + 默认绑定 + 接口
    let source = r#"Point: Type = {
        x: Float,
        y: Float,
        distance = calc_distance[0],
        auto_method = some_func,
        Drawable
    }"#;
    let tokens = tokenize(source).unwrap();
    let module = parse(&tokens).unwrap();

    match &module.items[0].kind {
        StmtKind::TypeDef {
            name, definition, ..
        } => {
            assert_eq!(name, "Point");
            match definition {
                ast::Type::Struct {
                    fields,
                    bindings,
                    interfaces,
                } => {
                    assert_eq!(fields.len(), 2);
                    assert_eq!(bindings.len(), 2);
                    assert_eq!(interfaces.len(), 1);

                    // distance = calc_distance[0] → External
                    assert_eq!(bindings[0].name, "distance");
                    assert!(matches!(&bindings[0].kind, BindingKind::External { .. }));

                    // auto_method = some_func → DefaultExternal
                    assert_eq!(bindings[1].name, "auto_method");
                    assert!(matches!(
                        &bindings[1].kind,
                        BindingKind::DefaultExternal { .. }
                    ));

                    // Drawable → interface
                    assert_eq!(interfaces[0], "Drawable");
                }
                _ => panic!("Expected Struct type"),
            }
        }
        _ => panic!("Expected TypeDef"),
    }
}
