//! Declaration parsing tests — based on spec §5.2, §6.1, RFC-011 §4.1

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::parser::ast::{extract_generic_param_names, Expr, StmtKind, Type};

fn parse_stmt(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors, "解析不应有错误");
    assert_eq!(result.module.items.len(), 1);
    result.module.items.into_iter().next().unwrap().kind
}

// ============================================================================
// 变量声明 (Spec §5.2)
// ============================================================================

#[test]
fn test_var_simple() {
    let kind = parse_stmt("x = 42");
    assert!(
        matches!(&kind, StmtKind::Assign { target, .. } if matches!(target.as_ref(), crate::frontend::core::parser::ast::Expr::Var(name, _) if name == "x"))
    );
}

#[test]
fn test_var_typed() {
    // §6.2: name: type = value
    let kind = parse_stmt("x: Int = 42");
    if let StmtKind::Assign {
        target,
        type_annotation,
        ..
    } = &kind
    {
        let name = if let Expr::Var(n, _) = target.as_ref() {
            n.clone()
        } else {
            panic!("Expected Var target")
        };
        assert_eq!(name, "x");
        assert!(type_annotation.is_some(), "类型标注应被解析");
    } else {
        panic!("Expected StmtKind::Assign");
    }
}

#[test]
fn test_var_mut() {
    let kind = parse_stmt("mut x: Int = 0");
    if let StmtKind::Assign { target, is_mut, .. } = &kind {
        let name = if let Expr::Var(n, _) = target.as_ref() {
            n.clone()
        } else {
            panic!("Expected Var target")
        };
        assert_eq!(name, "x");
        assert!(is_mut, "mut 标记应被识别");
    } else {
        panic!("Expected StmtKind::Var");
    }
}

#[test]
fn test_var_no_annotation() {
    let kind = parse_stmt("x = 42");
    if let StmtKind::Assign {
        target,
        type_annotation,
        ..
    } = &kind
    {
        let name = if let Expr::Var(n, _) = target.as_ref() {
            n.clone()
        } else {
            panic!("Expected Var target")
        };
        assert_eq!(name, "x");
        assert!(
            type_annotation.is_none(),
            "无类型标注时不应有 type_annotation"
        );
    } else {
        panic!("Expected StmtKind::Assign");
    }
}

// RFC-010 uses StmtKind::Binding for block assignments
#[test]
fn test_var_block_initializer_is_binding() {
    let kind = parse_stmt("main = { x = 42 }");
    // main = { ... } 应该解析为 Binding（函数体）
    assert!(
        matches!(&kind, StmtKind::Assign { target, .. } if matches!(target.as_ref(), crate::frontend::core::parser::ast::Expr::Var(name, _) if name == "main"))
    );
}

// ============================================================================
// 函数定义 (RFC-010 / Spec §6.1)
// ============================================================================

#[test]
fn test_fn_def_simple() {
    let kind = parse_stmt("add: (a: Int, b: Int) -> Int = a + b");
    if let StmtKind::Assign { target, value, .. } = &kind {
        let name = if let Expr::Var(n, _) = target.as_ref() {
            n.clone()
        } else {
            panic!("Expected Var target")
        };
        let (params, _body) = if let Some(v) = value {
            if let Expr::Lambda { params, body, .. } = v.as_ref() {
                (params.clone(), body.stmts.clone())
            } else {
                (Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };
        assert_eq!(name, "add");
        assert_eq!(params.len(), 2);
    } else {
        panic!("Expected StmtKind::Binding, got {:?}", kind);
    }
}

#[test]
fn test_fn_def_no_params() {
    let kind = parse_stmt("main: () -> Void = {}");
    if let StmtKind::Assign { target, value, .. } = &kind {
        let name = if let Expr::Var(n, _) = target.as_ref() {
            n.clone()
        } else {
            panic!("Expected Var target")
        };
        let (params, _body) = if let Some(v) = value {
            if let Expr::Lambda { params, body, .. } = v.as_ref() {
                (params.clone(), body.stmts.clone())
            } else {
                (Vec::new(), Vec::new())
            }
        } else {
            (Vec::new(), Vec::new())
        };
        assert_eq!(name, "main");
        assert!(params.is_empty(), "无参数函数 params 应为空");
    } else {
        panic!("Expected StmtKind::Binding");
    }
}

#[test]
fn test_fn_def_block_body() {
    let kind = parse_stmt("add: (a: Int, b: Int) -> Int = { return a + b }");
    if let StmtKind::Assign { target, .. } = &kind {
        let name = if let Expr::Var(n, _) = target.as_ref() {
            n.clone()
        } else {
            panic!("Expected Var target")
        };
        assert_eq!(name, "add");
    } else {
        panic!("Expected StmtKind::Assign");
    }
}

// ============================================================================
// 类型定义 (RFC-010)
// ============================================================================

#[test]
fn test_type_def_struct() {
    let kind = parse_stmt("Point: Type = { x: Float, y: Float }");
    if let StmtKind::TypeDefinition {
        name, definition, ..
    } = &kind
    {
        assert_eq!(name, "Point", "类型名应为 Point");
        assert!(
            matches!(definition, Type::Struct { .. }),
            "definition 应为 Struct"
        );
    } else {
        panic!("Expected StmtKind::TypeDefinition for type def");
    }
}

#[test]
fn test_type_def_enum() {
    let kind = parse_stmt("Color: Type = { red | green | blue }");
    if let StmtKind::TypeDefinition { name, .. } = &kind {
        assert_eq!(name, "Color", "类型名应为 Color");
    } else {
        panic!("Expected StmtKind::TypeDefinition for enum def");
    }
}

#[test]
fn test_type_def_generic() {
    // RFC-011: Option: (T: Type) -> Type = { some(T) | none }
    // 验证 (T: Type) 被识别为泛型参数

    // Arrange
    let kind = parse_stmt("Option: (T: Type) -> Type = { some(T) | none }");

    // Act
    let StmtKind::TypeDefinition {
        name,
        signature_params,
        ..
    } = &kind
    else {
        panic!(
            "Expected StmtKind::TypeDefinition for generic type def, got {:?}",
            kind
        );
    };
    let generic_params = extract_generic_param_names(signature_params);

    // Assert
    assert_eq!(name, "Option", "类型名应为 Option");
    assert_eq!(generic_params.len(), 1, "应有 1 个泛型参数");
    assert_eq!(generic_params[0].name, "T", "泛型参数名应为 T");
}

// ============================================================================
// 方法定义 (RFC-010)
// ============================================================================

#[test]
fn test_method_def() {
    let kind = parse_stmt("Point.draw: (self: Point, s: Surface) -> Void = { }");
    if let StmtKind::Assign { target, .. } = &kind {
        if let Expr::FieldAccess { expr, field, .. } = target.as_ref() {
            if let Expr::Var(tn, _) = expr.as_ref() {
                assert_eq!(tn, "Point");
                assert_eq!(field, "draw");
            } else {
                panic!("Expected Var type_name");
            }
        } else {
            panic!("Expected FieldAccess target");
        }
    } else {
        panic!("Expected StmtKind::Assign for method def, got {:?}", kind);
    }
}

// ============================================================================
// 外部绑定 (RFC-004)
// ============================================================================

#[test]
fn test_external_binding() {
    let kind = parse_stmt("Point.distance = distance[0]");
    if let StmtKind::Assign { target, .. } = &kind {
        if let Expr::FieldAccess { expr, field, .. } = target.as_ref() {
            if let Expr::Var(tn, _) = expr.as_ref() {
                assert_eq!(tn, "Point");
                assert_eq!(field, "distance");
            } else {
                panic!("Expected Var type_name");
            }
        } else {
            panic!("Expected FieldAccess target");
        }
    } else {
        panic!("Expected StmtKind::Assign for external binding");
    }
}

// ============================================================================
// 表达式语句
// ============================================================================

#[test]
fn test_expr_stmt_literal() {
    let kind = parse_stmt("42");
    assert!(matches!(&kind, StmtKind::Expr(..)));
}

#[test]
fn test_expr_stmt_call() {
    let kind = parse_stmt("foo()");
    assert!(matches!(&kind, StmtKind::Expr(..)));
}

#[test]
fn test_signature_params_stored_verbatim() {
    // 覆盖: RFC-010 统一声明语法 — signature_params 原样存储签名第一组参数
    // 验证: 泛型与值参数混合签名 (T: Type, x: Int) 两项按序保留，名字与标注完整
    let kind = parse_stmt("foo: (T: Type, x: Int) -> Int = x");
    let StmtKind::Assign {
        signature_params, ..
    } = &kind
    else {
        panic!("Expected StmtKind::Binding, got {:?}", kind);
    };

    assert_eq!(
        signature_params.len(),
        2,
        "签名第一组两个参数应原样保留（泛型 T 与值参数 x）"
    );
    assert_eq!(signature_params[0].name, "T", "第一个参数应为 T");
    assert!(
        matches!(signature_params[0].ty, Some(Type::MetaType { .. })),
        "T 的标注应为 MetaType（Type 关键字）"
    );
    assert_eq!(signature_params[1].name, "x", "第二个参数应为 x");
    assert!(
        matches!(&signature_params[1].ty, Some(Type::Name { name, .. }) if name == "Int"),
        "x 的标注应为 Int"
    );
}

#[test]
fn test_value_params_no_merged_misalignment() {
    // 覆盖: RFC-007 参数匹配 — params 存值参数，标注来自签名对应值参数而非错位 zip
    // 验证: foo: (T: Type, x: Int) -> Int = (x) => x 的 params == [x: Int]
    //       （修复前 merged 产出 x: MetaType 错位——x 配上了 T 的标注）
    let kind = parse_stmt("foo: (T: Type, x: Int) -> Int = (x) => x");
    let StmtKind::Assign { value, .. } = &kind else {
        panic!("Expected StmtKind::Binding, got {:?}", kind);
    };
    let params = if let Some(v) = value {
        if let Expr::Lambda { params, .. } = v.as_ref() {
            params.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    assert_eq!(params.len(), 1, "params 应只含值参数（泛型 T 不在其中）");
    assert_eq!(params[0].name, "x", "值参数名应为 x");
    assert!(
        matches!(&params[0].ty, Some(Type::Name { name, .. }) if name == "Int"),
        "x 的标注应为 Int（来自签名值参数），而非 MetaType 错位"
    );
}

#[test]
fn test_generic_curried_fn_params_from_lambda() {
    // 覆盖: RFC-010 泛型函数 — 全泛型第一组时值参数在内层签名
    // 验证: map: (T: Type) -> ((x: Int) -> Int) = (x) => x 的 params == [x: 无标注]
    let kind = parse_stmt("map: (T: Type) -> ((x: Int) -> Int) = (x) => x");
    let StmtKind::Assign { value, .. } = &kind else {
        panic!("Expected StmtKind::Binding, got {:?}", kind);
    };
    let params = if let Some(v) = value {
        if let Expr::Lambda { params, .. } = v.as_ref() {
            params.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    assert_eq!(params.len(), 1, "params 应含 lambda 参数 x");
    assert_eq!(params[0].name, "x", "参数名应来自 lambda");
    assert!(
        params[0].ty.is_none(),
        "curried 泛型函数值参数标注 None（HM 推断），不应有 MetaType 错位标注"
    );
}

#[test]
fn test_const_generic_no_merged_misalignment() {
    // 覆盖: RFC-011 Const 泛型 — (N: Int) 被 extract_generic_params 判为 Const 泛型而非值参数
    // 泛型参数名改用法判定（不再大小写猜测）。
    // 验证: f: (N: Int, s: String) -> String = (s) => s 的 params == [s: String]
    //       (N: Int) 是 Const 泛型（Int 在 CONST_PARAM_TYPES 中），
    //       不参与 lambda name 匹配 → 仅 s 匹配上
    let kind = parse_stmt("f: (N: Int, s: String) -> String = (s) => s");
    let StmtKind::Assign { value, .. } = &kind else {
        panic!("Expected StmtKind::Binding, got {:?}", kind);
    };
    let params = if let Some(v) = value {
        if let Expr::Lambda { params, .. } = v.as_ref() {
            params.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    assert_eq!(
        params.len(),
        1,
        "params 应只含值参数 s（Const 泛型 N 不参与值参数对齐）"
    );
    assert_eq!(params[0].name, "s", "值参数名应为 s");
    assert!(
        matches!(&params[0].ty, Some(Type::Name { name, .. }) if name == "String"),
        "s 的标注应为 String（来自签名值参数），而非 N 的 Int 错位"
    );
}

#[test]
fn test_const_generic_filtered_without_lambda_head() {
    // 覆盖: RFC-011 §4.1 Const 泛型 — 无 lambda 头分支同样剔除 Const 泛型
    // 泛型参数按"用途"判定（不再大小写/名单猜测）：
    // N 被用在返回类型 Array(Int, N) 的类型位置 → 是 Const 泛型 → 从值参数剔除。
    // 验证: f: (N: Int, s: String) -> Array(Int, N) = s 的 params == [s: String]

    // Arrange
    let kind = parse_stmt("f: (N: Int, s: String) -> Array(Int, N) = s");
    let StmtKind::Assign { value, .. } = &kind else {
        panic!("Expected StmtKind::Binding, got {:?}", kind);
    };
    let params = if let Some(v) = value {
        if let Expr::Lambda { params, .. } = v.as_ref() {
            params.clone()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Assert

    assert_eq!(
        params.len(),
        1,
        "params 应只含值参数 s（Const 泛型 N 被当类型用，不是值参数）"
    );
    assert_eq!(params[0].name, "s", "值参数名应为 s");
    assert!(
        matches!(&params[0].ty, Some(Type::Name { name, .. }) if name == "String"),
        "s 的标注应为 String"
    );
}

// ============================================================================
// 精化约束类型定义 (RFC-011)
// ============================================================================

#[test]
fn test_type_def_constraint_expr_single() {
    // Arrange
    let kind = parse_stmt("IsPositive: Type = { x > 0 }");

    // Act & Assert
    let StmtKind::TypeDefinition {
        name, definition, ..
    } = &kind
    else {
        panic!(
            "Expected StmtKind::TypeDefinition for IsPositive, got {:?}",
            kind
        );
    };
    assert_eq!(name, "IsPositive", "类型名应为 IsPositive");
    let Type::Struct { body } = definition else {
        panic!("Expected Type::Struct, got {:?}", definition);
    };
    assert_eq!(body.len(), 1, "结构体应有 1 个体项");
    assert!(
        matches!(
            &body[0],
            crate::frontend::core::parser::ast::TypeBodyItem::Expr(Type::ConstExpr(_))
        ),
        "体项应为 Expr(ConstExpr), got {:?}",
        body[0]
    );
}

#[test]
fn test_type_def_constraint_expr_multi() {
    // Arrange
    let kind = parse_stmt("P: Type = { x > 0, y > 0 }");

    // Act & Assert
    let StmtKind::TypeDefinition {
        name, definition, ..
    } = &kind
    else {
        panic!("Expected StmtKind::TypeDefinition for P, got {:?}", kind);
    };
    assert_eq!(name, "P", "类型名应为 P");
    let Type::Struct { body } = definition else {
        panic!("Expected Type::Struct, got {:?}", definition);
    };
    assert_eq!(body.len(), 2, "结构体应有 2 个体项");
    assert!(
        matches!(
            &body[0],
            crate::frontend::core::parser::ast::TypeBodyItem::Expr(Type::ConstExpr(_))
        ),
        "第 1 项应为 Expr(ConstExpr), got {:?}",
        body[0]
    );
    assert!(
        matches!(
            &body[1],
            crate::frontend::core::parser::ast::TypeBodyItem::Expr(Type::ConstExpr(_))
        ),
        "第 2 项应为 Expr(ConstExpr), got {:?}",
        body[1]
    );
}

#[test]
fn test_type_def_constraint_expr_mixed() {
    // Arrange
    let kind = parse_stmt("PP: Type = { x: Float, x > 0 }");

    // Act & Assert
    let StmtKind::TypeDefinition {
        name, definition, ..
    } = &kind
    else {
        panic!("Expected StmtKind::TypeDefinition for PP, got {:?}", kind);
    };
    assert_eq!(name, "PP", "类型名应为 PP");
    let Type::Struct { body } = definition else {
        panic!("Expected Type::Struct, got {:?}", definition);
    };
    assert_eq!(body.len(), 2, "结构体应有 2 个体项");
    assert!(
        matches!(
            &body[0],
            crate::frontend::core::parser::ast::TypeBodyItem::Field(_)
        ),
        "第 1 项应为 Field, got {:?}",
        body[0]
    );
    assert!(
        matches!(
            &body[1],
            crate::frontend::core::parser::ast::TypeBodyItem::Expr(Type::ConstExpr(_))
        ),
        "第 2 项应为 Expr(ConstExpr), got {:?}",
        body[1]
    );
}

#[test]
fn test_type_def_constraint_expr_simplified_form() {
    // Arrange & Act
    let kind = parse_stmt("IsPositive: (x: Int) -> Type = x > 0");

    // Assert
    assert!(
        matches!(&kind, StmtKind::Assign { .. }),
        "简化形式应解析为 Assign, got {:?}",
        kind
    );
}

#[test]
fn test_type_def_constraint_expr_block_form() {
    // Arrange
    let kind = parse_stmt("IsPositive: (x: Int) -> Type = { x > 0 }");

    // Act & Assert
    let StmtKind::TypeDefinition {
        name, definition, ..
    } = &kind
    else {
        panic!(
            "Expected StmtKind::TypeDefinition for IsPositive (block form), got {:?}",
            kind
        );
    };
    assert_eq!(name, "IsPositive", "类型名应为 IsPositive");
    let Type::Struct { body } = definition else {
        panic!("Expected Type::Struct, got {:?}", definition);
    };
    assert_eq!(body.len(), 1, "结构体应有 1 个体项");
    assert!(
        matches!(
            &body[0],
            crate::frontend::core::parser::ast::TypeBodyItem::Expr(Type::ConstExpr(_))
        ),
        "体项应为 Expr(ConstExpr), got {:?}",
        body[0]
    );
}
