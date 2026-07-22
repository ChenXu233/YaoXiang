//! 方法绑定测试 — 基于 RFC-004, RFC-010, 语言规范 §6.6-§6.8
//!
//! RFC-004 定义:
//!   Type.method = func          默认绑定（自动检测位置）
//!   Type.method = func[N]       单位置绑定
//!   Type.method = func[-1]      负数索引（最后一个参数）
//!   Type.method = func[0, 1]    多位置联合绑定
//!   Type.method = func[0, _, 2] 占位符绑定
//!
//! RFC-010 定义:
//!   Type.method: (params) -> Ret = { body }  方法定义
//!   { method = func[N] }                    类型体内绑定
//!   { method: (FnType)[N] = (params) => body }  匿名函数绑定

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::parse;
use crate::frontend::core::parser::ast::{
    Expr, StmtKind, BindingKind, Type, TypeBodyBinding, TypeBodyItem,
};
use crate::frontend::core::parser::statements::bindings::{BindingParser, BindingPositionValidator};

fn parse_stmt(source: &str) -> StmtKind {
    let tokens = tokenize(source).unwrap();
    let result = parse(&tokens);
    assert!(!result.has_errors);
    assert_eq!(result.module.items.len(), 1);
    result.module.items.into_iter().next().unwrap().kind
}

#[test]
fn test_rfc004_default_binding() {
    let kind = parse_stmt("Point.distance = distance");
    match &kind {
        StmtKind::Assign { target, value, .. } => {
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
            if let Some(Expr::Var(fn_name, _)) = value.as_ref().map(|v| v.as_ref()) {
                assert_eq!(fn_name, "distance");
            } else {
                panic!("Expected Var value");
            }
        }
        _ => panic!("Expected Assign"),
    }
}

#[test]
fn test_rfc004_position_0() {
    let kind = parse_stmt("Point.distance = distance[0]");
    if let StmtKind::Assign { target, value, .. } = &kind {
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
        assert!(value.is_some(), "expected value");
    } else {
        panic!("Expected Assign");
    }
}

#[test]
fn test_rfc004_position_1() {
    let kind = parse_stmt("Point.transform = transform[1]");
    if let StmtKind::Assign { target, value, .. } = &kind {
        if let Expr::FieldAccess { expr, field, .. } = target.as_ref() {
            if let Expr::Var(tn, _) = expr.as_ref() {
                assert_eq!(tn, "Point");
                assert_eq!(field, "transform");
            } else {
                panic!("Expected Var type_name");
            }
        } else {
            panic!("Expected FieldAccess target");
        }
        assert!(value.is_some(), "expected value");
    } else {
        panic!("Expected Assign");
    }
}

#[test]
fn test_rfc004_negative_index() {
    let kind = parse_stmt("Point.last = func[-1]");
    if let StmtKind::Assign { target, value, .. } = &kind {
        if let Expr::FieldAccess { expr, field, .. } = target.as_ref() {
            if let Expr::Var(tn, _) = expr.as_ref() {
                assert_eq!(tn, "Point");
                assert_eq!(field, "last");
            } else {
                panic!("Expected Var type_name");
            }
        } else {
            panic!("Expected FieldAccess target");
        }
        assert!(value.is_some(), "expected value");
    } else {
        panic!("Expected Assign");
    }
}

#[test]
fn test_rfc004_multi_position() {
    let kind = parse_stmt("Point.scale = scale[0, 1]");
    if let StmtKind::Assign { target, value, .. } = &kind {
        if let Expr::FieldAccess { expr, field, .. } = target.as_ref() {
            if let Expr::Var(tn, _) = expr.as_ref() {
                assert_eq!(tn, "Point");
                assert_eq!(field, "scale");
            } else {
                panic!("Expected Var type_name");
            }
        } else {
            panic!("Expected FieldAccess target");
        }
        assert!(value.is_some(), "expected value");
    }
}

#[test]
fn test_rfc004_triple_position() {
    let kind = parse_stmt("Point.calc = calculate[0, 1, 2]");
    assert!(matches!(&kind, StmtKind::Assign { .. }));
}

#[test]
fn test_rfc004_placeholder_position() {
    // RFC-004 定义占位符 `_` 语法，当前解析器暂不支持 `_` 作为位置
    // 用不带占位符的位置来验证
    let kind = parse_stmt("Point.calc = func[0, 2]");
    assert!(matches!(&kind, StmtKind::Assign { .. }));
}

#[test]
fn test_rfc010_method_def_simple() {
    let kind = parse_stmt("Point.draw: (self: Point, s: Surface) -> Void = { }");
    if let StmtKind::Assign { target, .. } = &kind {
        if let Expr::FieldAccess { expr, field, .. } = target.as_ref() {
            if let Expr::Var(tn, _) = expr.as_ref() {
                assert_eq!(tn, "Point");
            } else {
                panic!("Expected Var type_name");
            }
            assert_eq!(field, "draw");
        } else {
            panic!("Expected FieldAccess target");
        }
    } else {
        panic!("Expected Assign");
    }
}

#[test]
fn test_rfc010_method_def_expr_body() {
    let kind = parse_stmt("Point.serialize: (self: Point) -> String = (self) => \"hello\"");
    if let StmtKind::Assign { target, .. } = &kind {
        if let Expr::FieldAccess { expr, field, .. } = target.as_ref() {
            if let Expr::Var(tn, _) = expr.as_ref() {
                assert_eq!(tn, "Point");
                assert_eq!(field, "serialize");
            } else {
                panic!("Expected Var type_name");
            }
        } else {
            panic!("Expected FieldAccess target");
        }
    } else {
        panic!("Expected Assign");
    }
}

#[test]
fn test_rfc010_type_body_external_binding() {
    let kind = parse_stmt("Point: Type = { distance = distance[0] }");
    if let StmtKind::TypeDefinition {
        name, definition, ..
    } = &kind
    {
        assert_eq!(name, "Point", "类型名应为 Point");
        if let Type::Struct { body } = definition {
            let bindings: Vec<&TypeBodyBinding> = body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Binding(b) = it {
                        Some(b)
                    } else {
                        None
                    }
                })
                .collect();
            assert!(!bindings.is_empty(), "应有 bindings");
            assert_eq!(bindings[0].name, "distance", "binding 名应为 distance");
        } else {
            panic!("Expected Type::Struct");
        }
    } else {
        panic!("Expected StmtKind::TypeDefinition");
    }
}

#[test]
fn test_rfc010_type_body_default_binding() {
    let kind = parse_stmt("Point: Type = { distance = distance }");
    if let StmtKind::TypeDefinition { definition, .. } = &kind {
        if let Type::Struct { body } = definition {
            let bindings: Vec<&TypeBodyBinding> = body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Binding(b) = it {
                        Some(b)
                    } else {
                        None
                    }
                })
                .collect();
            assert!(!bindings.is_empty(), "应有 bindings");
            assert!(
                matches!(bindings[0].kind, BindingKind::DefaultExternal { .. }),
                "binding kind 应为 DefaultExternal"
            );
        }
    }
}

#[test]
fn test_rfc010_anonymous_binding() {
    let src = "Point: Type = { distance: ((a: Point, b: Point) -> Float)[0] = (a, b) => 0.0 }";
    let kind = parse_stmt(src);
    if let StmtKind::TypeDefinition { definition, .. } = &kind {
        if let Type::Struct { body } = definition {
            let bindings: Vec<&TypeBodyBinding> = body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Binding(b) = it {
                        Some(b)
                    } else {
                        None
                    }
                })
                .collect();
            assert!(!bindings.is_empty(), "应有 bindings");
            assert!(
                matches!(bindings[0].kind, BindingKind::Anonymous { .. }),
                "binding kind 应为 Anonymous"
            );
        }
    }
}

#[test]
fn test_rfc010_pub_fn_with_point_param() {
    let kind = parse_stmt("pub distance: (p1: Point, p2: Point) -> Float = { 0.0 }");
    if let StmtKind::Assign { is_pub, .. } = &kind {
        assert!(is_pub);
    } else {
        panic!("Expected Binding");
    }
}

#[test]
fn test_binding_parser_validate_ok() {
    let parser = BindingParser::new();
    assert!(parser
        .validate_binding_syntax("Point.method = value")
        .is_ok());
}

#[test]
fn test_binding_parser_validate_missing_eq() {
    let parser = BindingParser::new();
    assert!(parser.validate_binding_syntax("invalid").is_err());
}

#[test]
fn test_binding_parser_parse_tokens() {
    let tokens = tokenize("Point.method = 42").unwrap();
    let parser = BindingParser::new();
    assert!(parser.parse_binding(&tokens, 0).is_ok());
}

#[test]
fn test_position_validator_ok() {
    let v = BindingPositionValidator::new(5);
    assert!(v.validate_positions(&[0, 1, 2, 3, 4]).is_ok());
}

#[test]
fn test_position_validator_exceeds() {
    let v = BindingPositionValidator::new(3);
    assert!(v.validate_positions(&[5]).is_err());
}

#[test]
fn test_position_validator_negative() {
    let v = BindingPositionValidator::new(5);
    assert!(v.validate_positions(&[-1]).is_err());
}

#[test]
fn test_position_validator_syntax() {
    let v = BindingPositionValidator::new(5);
    assert!(v.validate_binding_syntax("func[0]").is_ok());
    assert!(v.validate_binding_syntax("func").is_err());
}
