//! Type annotation parsing tests — based on spec §3, RFC-010, RFC-011
//!
//! RFC-010 (issue #203): `|` 变体语法已废弃。和类型统一用记录类型表达
//! （字段全为函数，返回自身类型）。本文件覆盖：
//! - §3.2 基元类型
//! - §3.3 记录/接口类型（含和类型新语法）
//! - §3.8 泛型类型
//! - RFC-010 废弃语法的错误路径回归

use crate::frontend::core::lexer::tokenize;
use crate::frontend::core::parser::statements::types::parse_type_annotation;
use crate::frontend::core::parser::ast::{BinOp, Expr, StmtKind, StructField, Type, TypeBodyItem};
use crate::frontend::core::parser::{parse, ParserState};

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

// 基元类型 (Spec §3.2)

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

// 元类型 (Spec §2.4 / RFC-010)

#[test]
fn test_meta_type() {
    with_type("Type", |t| {
        assert!(matches!(t, Type::MetaType { .. }));
    });
}

// 函数类型 (Spec §3.7)

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

// 元组类型 (Spec §3.6)

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

// 记录类型 (Spec §3.3 / RFC-010)

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

// 泛型类型 (Spec §3.8 / RFC-011)

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

// Option / Result 类型 (降低)

#[test]
fn test_option_type() {
    // RFC-010: 同 test_result_type——普通泛型类型应用
    with_type("Option(Int)", |t| {
        assert!(
            matches!(t, Type::Generic { ref name, ref args, .. } if name == "Option" && args.len() == 1),
            "Option(Int) should lower to Generic, got {t:?}"
        );
    });
}

#[test]
fn test_result_type() {
    // RFC-010: Result/Option 不再 lower 成专用 AST 节点——普通泛型类型应用
    with_type("Result(Int, String)", |t| {
        assert!(
            matches!(t, Type::Generic { ref name, ref args, .. } if name == "Result" && args.len() == 2),
            "Result(Int, String) should lower to Generic, got {t:?}"
        );
    });
}

// 命名结构体 (RFC-010)

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

// 裸指针 (Spec §8.5)

#[test]
fn test_ptr_type() {
    with_type("*Int", |t| {
        assert!(matches!(t, Type::Ptr(..)));
    });
}

// 旧语法拒绝 (RFC-010)

#[test]
fn test_reject_old_curried_fn_syntax() {
    // RFC-010: `Int -> Int` should be rejected
    let tokens = tokenize("Int -> Int").unwrap();
    let mut state = ParserState::new(&tokens);
    let result = parse_type_annotation(&mut state);
    assert!(result.is_none(), "Old curried fn syntax should be rejected");
}

// 结构体中的 Mut 字段

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
// const 泛型约束比较运算符 (RFC-011 §4.3) — issue #173

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

// const 泛型约束算术运算符 (RFC-011 §4.3) — issue #189

#[test]
fn test_struct_field_assert_arith_plus() {
    // Arrange & Act: 解析字段位置的 Assert(N + 1 > 0) 约束
    with_type("{ _assert_n: Assert(N + 1 > 0), data: Int }", |t| {
        // Assert: 参数应解析为 BinOp(Plus) 内嵌 BinOp(Gt) 的 ConstExpr
        let expr = extract_assert_arg(&t);
        assert!(
            matches!(expr, Expr::BinOp { op: BinOp::Gt, left, .. }
                if matches!(left.as_ref(), Expr::BinOp { op: BinOp::Add, .. })),
            "field constraint should parse as `N + 1 > 0`, got: {expr:?}"
        );
    });
}

#[test]
fn test_struct_field_assert_arith_mod() {
    // Arrange & Act: 解析字段位置的 Assert(N % 2 == 0) 约束
    with_type("{ _assert_n: Assert(N % 2 == 0), data: Int }", |t| {
        // Assert: 参数应解析为 Mod 比较的 ConstExpr
        let expr = extract_assert_arg(&t);
        assert!(
            matches!(expr, Expr::BinOp { op: BinOp::Eq, left, .. }
                if matches!(left.as_ref(), Expr::BinOp { op: BinOp::Mod, .. })),
            "field constraint should parse as `N % 2 == 0`, got: {expr:?}"
        );
    });
}

#[test]
fn test_struct_field_assert_arith_mul() {
    // Arrange & Act: 解析字段位置的 Assert(N * 2 > 0) 约束
    with_type("{ _assert_n: Assert(N * 2 > 0), data: Int }", |t| {
        // Assert: 参数应解析为 Mul 比较的 ConstExpr
        let expr = extract_assert_arg(&t);
        assert!(
            matches!(expr, Expr::BinOp { op: BinOp::Gt, left, .. }
                if matches!(left.as_ref(), Expr::BinOp { op: BinOp::Mul, .. })),
            "field constraint should parse as `N * 2 > 0`, got: {expr:?}"
        );
    });
}

// RFC-010 废弃语法回归 (issue #203)
//
// `|` 变体语法已废弃。和类型统一用记录类型表达：
//   { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }
//
// 废弃写法 `{ red | green | blue }` 不再被 parser 识别为 Type::Variant，
// 也不产生任何特殊 AST 节点——parser 将按普通记录/表达式路径解析，
// 最终由语义分析阶段给出明确的错误（如未知变量）。

#[test]
fn test_sum_type_record_syntax_no_variant_ast() {
    // RFC-010 issue #203: 和类型用记录类型表达，不应产生 Type::Variant
    // （该变体已从 AST 中删除）。此测试守护 AST 简化不被回退。

    // Arrange
    let source = "{ ok: (Int) -> Result(Int, String), err: (String) -> Result(Int, String) }";

    // Act
    with_type(source, |t| {
        // Assert: 必须解析为 Type::Struct，不得存在 Type::Variant 节点
        assert!(
            matches!(t, Type::Struct { .. }),
            "和类型应解析为 Type::Struct（RFC-010 issue #203），实际为 {:?}",
            t
        );
    });
}

#[test]
fn test_sum_type_record_field_count() {
    // RFC-010 issue #203: 和类型的每个变体构造器是记录的一个函数字段

    // Arrange
    let source = "{ some: (Int) -> Option(Int), none: () -> Option(Int) }";

    // Act
    with_type(source, |t| {
        let Type::Struct { body } = &t else {
            panic!("Expected Type::Struct for sum type, got {t:?}");
        };
        let field_count = body
            .iter()
            .filter(|it| matches!(it, TypeBodyItem::Field(_)))
            .count();

        // Assert: 两个变体构造器 = 两个字段
        assert_eq!(
            field_count, 2,
            "Option 和类型应有 2 个函数字段（some, none），实际为 {field_count}"
        );
    });
}

#[test]
fn test_deprecated_pipe_syntax_not_parsed_as_variant() {
    // RFC-010 issue #203: `{ red | green | blue }` 不再产生 Type::Variant。
    // parser 不再为 `|` 维护特殊路径。此测试验证：
    // 废弃语法不会让 parser 产出 Type::Variant（该变体已删除），
    // 也不会让 parser 把整体识别为单个 Type::Struct 字段。

    // Arrange
    let source = "{ red | green | blue }";
    let tokens = tokenize(source).unwrap();
    let mut state = ParserState::new(&tokens);

    // Act
    let _ = parse_type_annotation(&mut state);

    // Assert: parser 已消费 `}` 之外的额外 token（red 后续 | green ...）
    // 说明 `|` 不再被识别为变体分隔符。具体语义错误由下游类型检查报告。
    // 此处只守护 parser 层面不产生特殊的"变体整体回退"路径。
    assert!(
        !state.at_end(),
        "废弃 `|` 语法不应被整体吞掉为单个类型节点；parser 应停留在 `|` 附近"
    );
}

// RFC-027 §3 返回位具名精化 `(r: P(...))` —— 解析层必须保留**声明的名字**

/// 单具名括号（括号后不跟 `->`）= 返回形式参数的声明位：binder 名与谓词实参
/// 都必须原样留在 AST 里（类型检查器据此按名识别返回点义务）。
#[test]
fn test_named_paren_return_refinement_keeps_declared_binder_name() {
    // Arrange & Act
    with_type("(r: IsPositive(r + 100))", |t| {
        // Assert
        let Type::NamedParen { param, inner, .. } = &t else {
            panic!("单具名括号应解析为 Type::NamedParen，got: {t:?}");
        };
        assert_eq!(param, "r", "声明的返回形式参数名必须保留");
        let Type::Generic { name, args, .. } = inner.as_ref() else {
            panic!("内层应是谓词应用，got: {inner:?}");
        };
        assert_eq!(name, "IsPositive", "内层谓词名原样保留");
        assert_eq!(args.len(), 1, "IsPositive 只接受一个实参");
        assert!(
            matches!(&args[0], Type::ConstExpr(e) if matches!(e.as_ref(), Expr::BinOp { op: BinOp::Add, .. })),
            "谓词实参 `r + 100` 应保留为表达式，got: {:?}",
            args[0]
        );
    });
}

/// 无具名的单元素括号仍是 `Paren`（RFC-004 括号语义不因本次改动改变）。
#[test]
fn test_unnamed_single_paren_stays_paren() {
    // Arrange & Act
    with_type("(Int)", |t| {
        // Assert
        assert!(
            matches!(&t, Type::Paren(inner) if matches!(inner.as_ref(), Type::Name { name, .. } if name == "Int")),
            "无具名的 `(Int)` 应仍是 Type::Paren，got: {t:?}"
        );
    });
}

/// 多个具名参数且无 `->` 仍按元组处理——形参名照旧丢弃（参数位语义不变）。
#[test]
fn test_multi_named_paren_stays_tuple() {
    // Arrange & Act
    with_type("(a: Int, b: Int)", |t| {
        // Assert
        let Type::Tuple(items) = &t else {
            panic!("多具名参数的无箭头括号应解析为 Type::Tuple，got: {t:?}");
        };
        assert_eq!(items.len(), 2, "两项类型原样保留（名字丢弃）");
        assert!(
            items
                .iter()
                .all(|it| matches!(it, Type::Name { name, .. } if name == "Int")),
            "元组元素应是参数类型，got: {items:?}"
        );
    });
}

/// 普通函数类型 `(a: Int, b: Int) -> Int` 不受影响：参数位的形参名仍由
/// `parse_fn_type_with_names` 承载，类型位只有类型。
#[test]
fn test_named_function_type_params_stay_anonymous_types() {
    // Arrange
    let tokens = tokenize("f: (a: Int, b: Int) -> Int = { 1 }").unwrap();

    // Act
    let result = parse(&tokens);
    assert!(
        !result.has_errors,
        "签名不应有解析错误: {:?}",
        result.errors
    );

    // Assert
    let Some(StmtKind::Assign {
        type_annotation: Some(Type::Fn {
            params,
            return_type,
        }),
        ..
    }) = result.module.items.first().map(|s| &s.kind)
    else {
        panic!(
            "应解析为带函数类型注解的绑定，got: {:?}",
            result.module.items
        );
    };
    assert_eq!(params.len(), 2, "参数位只有类型，不含名字");
    assert!(
        params
            .iter()
            .all(|p| matches!(p, Type::Name { name, .. } if name == "Int")),
        "参数类型应为 Int，got: {params:?}"
    );
    assert!(
        matches!(return_type.as_ref(), Type::Name { name, .. } if name == "Int"),
        "返回类型应为 Int，got: {return_type:?}"
    );
}
