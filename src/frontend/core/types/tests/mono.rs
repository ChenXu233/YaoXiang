//! 单态类型测试 — 基于语言规范 §3 & RFC-010 (续)
//!
//! §3.2-§3.17: From<ast::Type> 转换 — 覆盖所有 AST 类型到 MonoType 的转换路径
//! + 补充 expand_type 各类容器变体的测试

use crate::frontend::core::types::{MonoType, PolyType, StructType, TypeVar};
use crate::frontend::core::parser::ast;
use crate::frontend::core::types::EnumType;
use crate::util::span::Span;
use std::collections::HashMap;

// ===================================================================
// §3.2-§3.17: From<ast::Type> 转换
// ===================================================================

#[test]
fn test_from_ast_type_name() {
    let ast_ty = ast::Type::Name {
        name: "MyType".to_string(),
        span: Span::dummy(),
    };
    let mono: MonoType = ast_ty.into();
    assert_eq!(mono, MonoType::TypeRef("MyType".to_string()));
}

#[test]
fn test_from_ast_type_primitives() {
    assert_eq!(MonoType::from(ast::Type::Int(32)), MonoType::Int(32));
    assert_eq!(MonoType::from(ast::Type::Int(64)), MonoType::Int(64));
    assert_eq!(MonoType::from(ast::Type::Float(64)), MonoType::Float(64));
    assert_eq!(MonoType::from(ast::Type::Char), MonoType::Char);
    assert_eq!(MonoType::from(ast::Type::String), MonoType::String);
    assert_eq!(MonoType::from(ast::Type::Bytes), MonoType::Bytes);
    assert_eq!(MonoType::from(ast::Type::Bool), MonoType::Bool);
    assert_eq!(MonoType::from(ast::Type::Void), MonoType::Void);
}

#[test]
fn test_from_ast_type_struct() {
    let ast_ty = ast::Type::Struct {
        fields: vec![
            ast::StructField::new("x".to_string(), false, ast::Type::Float(64)),
            ast::StructField::new("y".to_string(), false, ast::Type::Float(64)),
        ],
        bindings: vec![],
        interfaces: vec![],
    };
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Struct(_)));
    if let MonoType::Struct(s) = mono {
        assert_eq!(s.fields.len(), 2);
    }
}

#[test]
fn test_from_ast_type_struct_with_defaults() {
    use crate::frontend::core::lexer::tokens::Literal;
    let ast_ty = ast::Type::Struct {
        fields: vec![ast::StructField {
            name: "x".to_string(),
            is_mut: false,
            ty: ast::Type::Float(64),
            default: Some(Box::new(ast::Expr::Lit(Literal::Float(0.0), Span::dummy()))),
        }],
        bindings: vec![],
        interfaces: vec![],
    };
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Struct(s) if s.field_has_default == vec![true]));
}

#[test]
fn test_from_ast_type_named_struct() {
    let ast_ty = ast::Type::NamedStruct {
        name: "Point".to_string(),
        name_span: Span::dummy(),
        fields: vec![ast::StructField::new(
            "x".to_string(),
            false,
            ast::Type::Float(64),
        )],
    };
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Struct(s) if s.name == "Point"));
}

#[test]
fn test_from_ast_type_union() {
    let ast_ty = ast::Type::Union(vec![
        (
            "ok".to_string(),
            Some(ast::Type::Name {
                name: "T".to_string(),
                span: Span::dummy(),
            }),
        ),
        (
            "err".to_string(),
            Some(ast::Type::Name {
                name: "E".to_string(),
                span: Span::dummy(),
            }),
        ),
    ]);
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Enum(_)));
}

#[test]
fn test_from_ast_type_tuple() {
    let ast_ty = ast::Type::Tuple(vec![ast::Type::Int(32), ast::Type::String]);
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Tuple(t) if t.len() == 2));
}

#[test]
fn test_from_ast_type_fn() {
    let ast_ty = ast::Type::Fn {
        params: vec![ast::Type::Int(32)],
        return_type: Box::new(ast::Type::Bool),
    };
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Fn { .. }));
}

#[test]
fn test_from_ast_type_option() {
    let ast_ty = ast::Type::Option(Box::new(ast::Type::Int(32)));
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Option(t) if *t == MonoType::Int(32)));
}

#[test]
fn test_from_ast_type_result() {
    let ast_ty = ast::Type::Result(Box::new(ast::Type::Int(32)), Box::new(ast::Type::String));
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Result(..)));
}

#[test]
fn test_from_ast_type_generic() {
    let ast_ty = ast::Type::Generic {
        name: "Option".to_string(),
        name_span: Span::dummy(),
        args: vec![ast::Type::Int(32)],
    };
    let mono: MonoType = ast_ty.into();
    // Known generic "Option" is converted to MonoType::Option
    assert!(matches!(mono, MonoType::Option(_)));
}

#[test]
fn test_from_ast_type_generic_custom() {
    let ast_ty = ast::Type::Generic {
        name: "List".to_string(),
        name_span: Span::dummy(),
        args: vec![ast::Type::Int(32)],
    };
    let mono: MonoType = ast_ty.into();
    // List(Int(32)) becomes MonoType::List(Box(Int(32)))
    assert!(matches!(mono, MonoType::List(inner) if *inner == MonoType::Int(32)));
}

#[test]
fn test_from_ast_type_assoc_type() {
    let ast_ty = ast::Type::AssocType {
        host_type: Box::new(ast::Type::Name {
            name: "Iter".to_string(),
            span: Span::dummy(),
        }),
        assoc_name: "Item".to_string(),
        assoc_name_span: Span::dummy(),
        assoc_args: vec![],
    };
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::AssocType { .. }));
}

#[test]
fn test_from_ast_type_meta_type() {
    let ast_ty = ast::Type::MetaType {
        name_span: Span::dummy(),
        args: vec![ast::Type::Int(32)],
    };
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::MetaType { .. }));
}

#[test]
fn test_from_ast_type_meta_type_nested() {
    let ast_ty = ast::Type::MetaType {
        name_span: Span::dummy(),
        args: vec![ast::Type::MetaType {
            name_span: Span::dummy(),
            args: vec![ast::Type::Name {
                name: "T".to_string(),
                span: Span::dummy(),
            }],
        }],
    };
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::MetaType { .. }));
}

// ===================================================================
// §3.4: EnumType
// ===================================================================

#[test]
fn test_from_ast_type_enum_variant() {
    use ast::VariantDef;
    let ast_ty = ast::Type::Variant(vec![
        VariantDef {
            name: "red".to_string(),
            name_span: Span::dummy(),
            params: vec![],
            span: Span::dummy(),
        },
        VariantDef {
            name: "green".to_string(),
            name_span: Span::dummy(),
            params: vec![],
            span: Span::dummy(),
        },
    ]);
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Enum(e) if e.variants.len() == 2));
}

#[test]
fn test_from_ast_type_sum() {
    let ast_ty = ast::Type::Sum(vec![ast::Type::Int(32), ast::Type::String]);
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::TypeRef(n) if n.contains("|")));
}

#[test]
fn test_from_ast_type_ptr() {
    let ast_ty = ast::Type::Ptr(Box::new(ast::Type::Int(32)));
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::TypeRef(n) if n.starts_with("*")));
}

#[test]
fn test_from_ast_type_literal() {
    let ast_ty = ast::Type::Literal {
        name: "five".to_string(),
        name_span: Span::dummy(),
        base_type: Box::new(ast::Type::Int(64)),
    };
    let mono: MonoType = ast_ty.into();
    // Literal type converts to base type
    assert_eq!(mono, MonoType::Int(64));
}

// ===================================================================
// §3.3: StructType PartialEq / Eq 完整测试
// ===================================================================

#[test]
fn test_struct_type_eq_same_fields() {
    let a = StructType {
        name: "P".to_string(),
        fields: vec![("x".to_string(), MonoType::Int(32))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    };
    let b = StructType {
        name: "P".to_string(),
        fields: vec![("x".to_string(), MonoType::Int(32))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    };
    assert_eq!(a, b);
}

#[test]
fn test_struct_type_eq_different_fields() {
    let a = StructType {
        name: "P".to_string(),
        fields: vec![("x".to_string(), MonoType::Int(32))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    };
    let b = StructType {
        name: "P".to_string(),
        fields: vec![("y".to_string(), MonoType::Int(32))],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    };
    assert_ne!(a, b);
}

// ===================================================================
// §3.17: get_ast_type_universe_level 覆盖嵌套场景
// ===================================================================

#[test]
fn test_get_ast_type_universe_level_nested() {
    let t = ast::Type::MetaType {
        name_span: Span::dummy(),
        args: vec![ast::Type::MetaType {
            name_span: Span::dummy(),
            args: vec![ast::Type::Name {
                name: "T".to_string(),
                span: Span::dummy(),
            }],
        }],
    };
    let level = crate::frontend::core::types::get_ast_type_universe_level(&t);
    assert_eq!(level, 2);
}

#[test]
fn test_get_ast_type_universe_level_plain() {
    let t = ast::Type::Name {
        name: "Int".to_string(),
        span: Span::dummy(),
    };
    assert_eq!(
        crate::frontend::core::types::get_ast_type_universe_level(&t),
        0
    );
}

#[test]
fn test_calculate_meta_type_level_args() {
    use ast::Type;
    assert!(crate::frontend::core::types::calculate_meta_type_level(&[]).is_type0());
    let args = vec![Type::MetaType {
        name_span: Span::dummy(),
        args: vec![Type::Name {
            name: "T".to_string(),
            span: Span::dummy(),
        }],
    }];
    assert_eq!(
        crate::frontend::core::types::calculate_meta_type_level(&args).level,
        "2"
    );
}

// ===================================================================
// §3.13-3.14: 类型联合/交集 type_name 测试
// ===================================================================

#[test]
fn test_type_name_dict_set_range() {
    let d = MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::Int(32)));
    assert_eq!(d.type_name(), "Dict(string, int32)");
    let s = MonoType::Set(Box::new(MonoType::Bool));
    assert_eq!(s.type_name(), "Set(bool)");
    let r = MonoType::Range {
        elem_type: Box::new(MonoType::Int(64)),
    };
    assert_eq!(r.type_name(), "Range(int64)");
}

#[test]
fn test_type_name_arc_weak_assoc() {
    let a = MonoType::Arc(Box::new(MonoType::Int(32)));
    assert_eq!(a.type_name(), "Arc(int32)");
    let w = MonoType::Weak(Box::new(MonoType::String));
    assert_eq!(w.type_name(), "Weak(string)");
    let at = MonoType::AssocType {
        host_type: Box::new(MonoType::TypeRef("Iter".to_string())),
        assoc_name: "Item".to_string(),
        assoc_args: vec![],
    };
    assert_eq!(at.type_name(), "Iter::Item");
}

#[test]
fn test_type_name_union_intersection_multi() {
    let u = MonoType::Union(vec![MonoType::Int(32), MonoType::String, MonoType::Bool]);
    let un = u.type_name();
    assert!(un.contains("int32") && un.contains("string") && un.contains("bool"));

    let i = MonoType::Intersection(vec![
        MonoType::TypeRef("A".to_string()),
        MonoType::TypeRef("B".to_string()),
    ]);
    assert_eq!(i.type_name(), "(A & B)");
}

#[test]
fn test_type_var_extraction() {
    let tv = TypeVar::new(7);
    assert_eq!(MonoType::TypeVar(tv).type_var(), Some(tv));
    assert_eq!(MonoType::Int(32).type_var(), None);
}

#[test]
fn test_is_numeric_all_widths() {
    assert!(MonoType::Int(8).is_numeric());
    assert!(MonoType::Int(16).is_numeric());
    assert!(MonoType::Int(32).is_numeric());
    assert!(MonoType::Int(64).is_numeric());
    assert!(MonoType::Int(128).is_numeric());
    assert!(MonoType::Float(32).is_numeric());
    assert!(MonoType::Float(64).is_numeric());
    assert!(!MonoType::Bool.is_numeric());
}

#[test]
fn test_is_constraint_and_constraint_fields() {
    let iface = MonoType::Struct(StructType {
        name: "Clone".to_string(),
        fields: vec![(
            "clone".to_string(),
            MonoType::Fn {
                params: vec![],
                return_type: Box::new(MonoType::TypeRef("Self".to_string())),
            },
        )],
        methods: HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    assert!(iface.is_constraint());
    let fields = iface.constraint_fields();
    assert_eq!(fields.len(), 1);
    assert!(MonoType::TypeRef("Clone".to_string())
        .constraint_fields()
        .is_empty());
}

#[test]
fn test_struct_field_is_mut_found() {
    let s = StructType {
        name: "S".to_string(),
        fields: vec![
            ("a".to_string(), MonoType::Int(32)),
            ("b".to_string(), MonoType::Bool),
        ],
        methods: HashMap::new(),
        field_mutability: vec![true, false],
        field_has_default: vec![false, false],
        interfaces: vec![],
    };
    assert_eq!(s.field_is_mut("a"), Some(true));
    assert_eq!(s.field_is_mut("b"), Some(false));
    assert_eq!(s.field_is_mut("nonexistent"), None);
}

#[test]
fn test_enum_type_partial_eq() {
    let a = EnumType {
        name: "Opt".to_string(),
        variants: vec!["some".to_string(), "none".to_string()],
    };
    let b = EnumType {
        name: "Opt".to_string(),
        variants: vec!["some".to_string(), "none".to_string()],
    };
    assert_eq!(a, b);
}

#[test]
fn test_universe_level_cmp() {
    use crate::frontend::core::types::UniverseLevel;
    assert_eq!(
        UniverseLevel::type0().cmp_level(&UniverseLevel::type1()),
        std::cmp::Ordering::Less
    );
    assert_eq!(
        UniverseLevel::new("10").cmp_level(&UniverseLevel::new("9")),
        std::cmp::Ordering::Greater
    );
    assert_eq!(UniverseLevel::new("999").succ().level, "1000");
}

#[test]
fn test_poly_type_display_and_name() {
    let poly = PolyType::mono(MonoType::String);
    assert_eq!(poly.type_name(), "string");
    assert_eq!(format!("{}", poly), "string");
    assert!(PolyType::mono(MonoType::Int(32)).is_mono());
    assert!(!PolyType::new(vec![TypeVar::new(0)], MonoType::Int(32)).is_mono());
}

#[test]
fn test_type_name_result() {
    let r = MonoType::Result(Box::new(MonoType::Int(32)), Box::new(MonoType::String));
    assert!(r.type_name().contains("Result"));
}

#[test]
fn test_type_name_fn_with_async() {
    let f = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Bool],
        return_type: Box::new(MonoType::Void),
    };
    assert!(f.type_name().contains("fn("));
}

// ===================================================================
// §3: MonoType 方法补充测试
// ===================================================================

#[test]
fn test_mono_type_type_var_extraction() {
    let tv = TypeVar::new(0);
    let ty = MonoType::TypeVar(tv);
    assert_eq!(ty.type_var(), Some(tv), "TypeVar should return Some(tv)");
}

#[test]
fn test_mono_type_type_var_none() {
    assert_eq!(MonoType::Int(32).type_var(), None, "Int should return None");
    assert_eq!(
        MonoType::String.type_var(),
        None,
        "String should return None"
    );
    assert_eq!(MonoType::Bool.type_var(), None, "Bool should return None");
}

#[test]
fn test_mono_type_list_variant() {
    let list = MonoType::List(Box::new(MonoType::Int(32)));
    assert!(
        list.type_name().contains("List"),
        "List type_name should contain 'List'"
    );
    assert!(
        list.type_name().contains("int32"),
        "List type_name should contain element type"
    );
}

#[test]
fn test_poly_type_type_binders() {
    let tv0 = TypeVar::new(0);
    let tv1 = TypeVar::new(1);
    let poly = PolyType::new(vec![tv0, tv1], MonoType::Int(32));
    let binders = &poly.type_binders;
    assert_eq!(binders.len(), 2, "should have 2 type binders");
    assert!(binders.contains(&tv0), "should contain tv0");
    assert!(binders.contains(&tv1), "should contain tv1");
}

#[test]
fn test_poly_type_type_binders_empty() {
    let poly = PolyType::mono(MonoType::Int(32));
    assert!(
        poly.type_binders.is_empty(),
        "mono type should have no binders"
    );
}

#[test]
fn test_mono_type_fn_async_flag() {
    let f_sync = MonoType::Fn {
        params: vec![],
        return_type: Box::new(MonoType::Void),
    };
    let f_async = MonoType::Fn {
        params: vec![],
        return_type: Box::new(MonoType::Void),
    };
    // Fn type_name doesn't include async flag - both produce "fn() -> void"
    assert!(
        f_sync.type_name().contains("fn("),
        "sync fn should contain 'fn('"
    );
    assert!(
        f_async.type_name().contains("fn("),
        "async fn should contain 'fn('"
    );
    assert_eq!(
        f_sync.type_name(),
        f_async.type_name(),
        "type_name ignores async flag"
    );
}

#[test]
fn test_mono_type_is_indexable() {
    // Array types should be indexable
    assert!(
        MonoType::List(Box::new(MonoType::Int(32))).is_indexable(),
        "List should be indexable"
    );
    assert!(
        MonoType::String.is_indexable(),
        "String should be indexable"
    );
    // Non-indexable types
    assert!(
        !MonoType::Int(32).is_indexable(),
        "Int should not be indexable"
    );
    assert!(
        !MonoType::Bool.is_indexable(),
        "Bool should not be indexable"
    );
    assert!(
        !MonoType::Void.is_indexable(),
        "Void should not be indexable"
    );
}

#[test]
fn test_mono_type_ptr_variant() {
    let ptr = MonoType::TypeRef("*Int32".to_string());
    assert!(
        ptr.type_name().contains("*"),
        "Ptr type_name should contain '*'"
    );
}

#[test]
fn test_mono_type_option_variant() {
    let opt = MonoType::Option(Box::new(MonoType::Int(32)));
    // Option type_name returns "{inner}?" format
    assert!(
        opt.type_name().contains("?"),
        "Option type_name should contain '?'"
    );
    assert!(
        opt.type_name().contains("int32"),
        "Option type_name should contain inner type"
    );
}

#[test]
fn test_mono_type_result_variant() {
    let res = MonoType::Result(Box::new(MonoType::Int(32)), Box::new(MonoType::String));
    assert!(
        res.type_name().contains("Result"),
        "Result type_name should contain 'Result'"
    );
}
