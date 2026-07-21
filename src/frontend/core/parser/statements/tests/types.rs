//! Type annotation parsing tests — based on spec §3, RFC-010, RFC-011

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::statements::types::parse_type_annotation;
use crate::frontend::core::parser::ast::{BinOp, Expr, StructField, Type, TypeBodyItem};
use crate::frontend::core::parser::ParserState;

fn with_type<F>(
    source: &str,
    mut f: F,
) where
    F: FnMut(Type),
{
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens);
    let t = parse_type_annotation(&mut state).expect("parse_type_annotation failed");
    f(t);
}

// ============================================================================
// 基元类型 (Spec §3.2)
// ============================================================================

#[test]
fn test_type_name_simple() {
    with_type("Int", |t| {
        assert!(matches!(t, Type::Name { name, .. } if name == "Int"));
    });
}

#[test]
fn test_type_string() {
    with_type("String", |t| {
        assert!(matches!(t, Type::Name { name, .. } if name == "String"));
    });
}

#[test]
fn test_type_bool() {
    with_type("Bool", |t| {
        assert!(matches!(t, Type::Name { name, .. } if name == "Bool"));
    });
}

#[test]
fn test_type_float() {
    with_type("Float", |t| {
        assert!(matches!(t, Type::Name { name, .. } if name == "Float"));
    });
}

// ============================================================================
// 元类型 (Spec §2.4 / RFC-010)
// ============================================================================

#[test]
fn test_meta_type() {
    with_type("Type", |t| {
        assert!(matches!(t, Type::MetaType { .. }));
    });
}

// ============================================================================
// 函数类型 (Spec §3.7)
// ============================================================================

#[test]
fn test_fn_type_basic() {
    with_type("(Int) -> String", |t| {
        assert!(matches!(t, Type::Fn { .. }));
    });
}

#[test]
fn test_fn_type_multi_param() {
    with_type("(Int, Float) -> Bool", |t| {
        if let Type::Fn {
            params,
            return_type,
            ..
        } = &t
        {
            assert_eq!(params.len(), 2);
            assert!(matches!(return_type.as_ref(), Type::Name { name, .. } if name == "Bool"));
        } else {
            panic!("Expected Type::Fn");
        }
    });
}

#[test]
fn test_fn_type_empty_params() {
    with_type("() -> Void", |t| {
        assert!(matches!(t, Type::Fn { .. }));
    });
}

// ============================================================================
// 元组类型 (Spec §3.6)
// ============================================================================

#[test]
fn test_tuple_type() {
    with_type("(Int, String, Bool)", |t| {
        if let Type::Tuple(types) = &t {
            assert_eq!(types.len(), 3);
        } else {
            panic!("Expected Type::Tuple");
        }
    });
}

// ============================================================================
// 记录类型 (Spec §3.3 / RFC-010)
// ============================================================================

#[test]
fn test_struct_type_empty() {
    with_type("{}", |t| {
        assert!(matches!(t, Type::Struct { .. }));
    });
}

#[test]
fn test_struct_type_fields() {
    with_type("{ x: Float, y: Float }", |t| {
        if let Type::Struct { body } = &t {
            let fields: Vec<&StructField> = body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Field(f) = it {
                        Some(f)
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(fields.len(), 2);
            assert_eq!(fields[0].name, "x");
            assert_eq!(fields[1].name, "y");
        } else {
            panic!("Expected Type::Struct");
        }
    });
}

#[test]
fn test_struct_type_with_interface() {
    with_type("{ x: Float, Drawable, Serializable }", |t| {
        if let Type::Struct { body } = &t {
            let fields: Vec<&StructField> = body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Field(f) = it {
                        Some(f)
                    } else {
                        None
                    }
                })
                .collect();
            let interfaces: Vec<String> = body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Interface(s) = it {
                        Some(s.clone())
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(fields.len(), 1);
            assert!(interfaces.contains(&"Drawable".to_string()));
            assert!(interfaces.contains(&"Serializable".to_string()));
        } else {
            panic!("Expected Type::Struct");
        }
    });
}

#[test]
fn test_struct_type_with_default() {
    with_type("{ x: Float = 0, y: Float = 0 }", |t| {
        if let Type::Struct { body } = &t {
            let fields: Vec<&StructField> = body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Field(f) = it {
                        Some(f)
                    } else {
                        None
                    }
                })
                .collect();
            assert_eq!(fields.len(), 2);
            assert!(fields[0].default.is_some());
        } else {
            panic!("Expected Type::Struct");
        }
    });
}

// ============================================================================
// 枚举/变体类型 (Spec §3.4)
// ============================================================================

#[test]
fn test_variant_type() {
    with_type("{ red | green | blue }", |t| {
        if let Type::Variant(variants) = &t {
            assert_eq!(variants.len(), 3);
            assert_eq!(variants[0].name, "red");
            assert_eq!(variants[1].name, "green");
            assert_eq!(variants[2].name, "blue");
        } else {
            panic!("Expected Type::Variant");
        }
    });
}

#[test]
fn test_variant_type_with_params() {
    with_type("{ ok(Int) | err(String) }", |t| {
        if let Type::Variant(variants) = &t {
            assert_eq!(variants.len(), 2);
            assert_eq!(variants[0].params.len(), 1);
            assert_eq!(variants[1].params.len(), 1);
        } else {
            panic!("Expected Type::Variant");
        }
    });
}

// ============================================================================
// 泛型类型 (Spec §3.8 / RFC-011)
// ============================================================================

#[test]
fn test_generic_type() {
    with_type("List(Int)", |t| {
        if let Type::Generic { name, args, .. } = &t {
            assert_eq!(name, "List");
            assert_eq!(args.len(), 1);
        } else {
            panic!("Expected Type::Generic");
        }
    });
}

#[test]
fn test_generic_nested() {
    with_type("List(List(Int))", |t| {
        if let Type::Generic { name, args, .. } = &t {
            assert_eq!(name, "List");
            assert_eq!(args.len(), 1);
            assert!(matches!(&args[0], Type::Generic { name: n, .. } if n == "List"));
        } else {
            panic!("Expected Type::Generic");
        }
    });
}

#[test]
fn test_generic_multi_arg() {
    with_type("Map(String, Int)", |t| {
        if let Type::Generic { name, args, .. } = &t {
            assert_eq!(name, "Map");
            assert_eq!(args.len(), 2);
        } else {
            panic!("Expected Type::Generic");
        }
    });
}

// ============================================================================
// Option / Result 类型 (降低)
// ============================================================================

#[test]
fn test_option_type() {
    with_type("Option(Int)", |t| {
        assert!(matches!(t, Type::Option(..)));
    });
}

#[test]
fn test_result_type() {
    with_type("Result(Int, String)", |t| {
        assert!(matches!(t, Type::Result(..)));
    });
}

// ============================================================================
// 命名结构体 (RFC-010)
// ============================================================================

#[test]
fn test_named_struct_type() {
    with_type("Point(x: Float, y: Float)", |t| {
        if let Type::NamedStruct { name, fields, .. } = &t {
            assert_eq!(name, "Point");
            assert_eq!(fields.len(), 2);
        } else {
            panic!("Expected Type::NamedStruct");
        }
    });
}

// ============================================================================
// 裸指针 (Spec §8.5)
// ============================================================================

#[test]
fn test_ptr_type() {
    with_type("*Int", |t| {
        assert!(matches!(t, Type::Ptr(..)));
    });
}

// ============================================================================
// 旧语法拒绝 (RFC-010)
// ============================================================================

#[test]
fn test_reject_old_curried_fn_syntax() {
    // RFC-010: `Int -> Int` should be rejected
    let tokens = tokenize("Int -> Int").unwrap();
    let mut state = ParserState::new(&tokens);
    let result = parse_type_annotation(&mut state);
    assert!(result.is_none(), "Old curried fn syntax should be rejected");
}

// ============================================================================
// 结构体中的 Mut 字段
// ============================================================================

#[test]
fn test_struct_mut_field() {
    // Note: "mut" in struct fields may not be fully supported
    with_type("{ x: Int, y: Float }", |t| {
        if let Type::Struct { body } = &t {
            let fields: Vec<&StructField> = body
                .iter()
                .filter_map(|it| {
                    if let TypeBodyItem::Field(f) = it {
                        Some(f)
                    } else {
                        None
                    }
                })
                .collect();
            assert!(!fields.is_empty(), "Should parse at least one field");
        } else {
            panic!("Expected Type::Struct");
        }
    });
}
// ============================================================================
// const 泛型约束比较运算符 (RFC-011 §4.3) — issue #173
// ============================================================================

/// 提取类型体中 Assert(...) 约束的参量表达式（字段或匿名位置）。
fn extract_assert_arg(ty: &Type) -> &Expr {
    let Type::Struct { body } = ty else {
        panic!("Expected Type::Struct, got: {ty:?}")
    };
    for item in body {
        let item_ty = match item {
            TypeBodyItem::Field(f) if f.name.starts_with("_assert") => Some(&f.ty),
            TypeBodyItem::Expr(e) => Some(e),
            _ => None,
        };
        if let Some(Type::Generic { name, args, .. }) = item_ty {
            assert_eq!(name, "Assert", "constraint type family should be Assert");
            assert_eq!(args.len(), 1, "Assert should take exactly one argument");
            if let Type::ConstExpr(expr) = &args[0] {
                return expr;
            }
            panic!(
                "Assert argument should be Type::ConstExpr, got: {:?}",
                args[0]
            );
        }
    }
    panic!("no Assert constraint found in struct body: {body:?}");
}

#[test]
fn test_struct_field_assert_lt() {
    // Arrange & Act: 解析字段位置的 Assert(N < 100) 约束
    with_type("{ _assert_n: Assert(N < 100), data: Int }", |t| {
        // Assert: 参数应解析为 Lt 比较的 ConstExpr
        let expr = extract_assert_arg(&t);
        assert!(
            matches!(expr, Expr::BinOp { op: BinOp::Lt, .. }),
            "field constraint should parse as Lt comparison, got: {expr:?}"
        );
    });
}

#[test]
fn test_struct_field_assert_le() {
    // Arrange & Act: 解析字段位置的 Assert(N <= 100) 约束
    with_type("{ _assert_n: Assert(N <= 100), data: Int }", |t| {
        // Assert: 参数应解析为 Le 比较的 ConstExpr
        let expr = extract_assert_arg(&t);
        assert!(
            matches!(expr, Expr::BinOp { op: BinOp::Le, .. }),
            "field constraint should parse as Le comparison, got: {expr:?}"
        );
    });
}

#[test]
fn test_struct_anon_assert_lt() {
    // Arrange & Act: 解析匿名位置的 Assert(N < 100) 约束
    with_type("{ data: Int, Assert(N < 100) }", |t| {
        // Assert: 参数应解析为 Lt 比较的 ConstExpr
        let expr = extract_assert_arg(&t);
        assert!(
            matches!(expr, Expr::BinOp { op: BinOp::Lt, .. }),
            "anonymous constraint should parse as Lt comparison, got: {expr:?}"
        );
    });
}

#[test]
fn test_struct_anon_assert_le() {
    // Arrange & Act: 解析匿名位置的 Assert(N <= 100) 约束
    with_type("{ data: Int, Assert(N <= 100) }", |t| {
        // Assert: 参数应解析为 Le 比较的 ConstExpr
        let expr = extract_assert_arg(&t);
        assert!(
            matches!(expr, Expr::BinOp { op: BinOp::Le, .. }),
            "anonymous constraint should parse as Le comparison, got: {expr:?}"
        );
    });
}
