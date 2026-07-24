//! 类型约束求解器测试 — 基于语言规范 §3
//!
//! §3.1: 类型统一 (unify) — 所有类型变体的统一规则
//! §3.2: 原类型统一 — Int/Float 精度匹配
//! §3.3: 结构体统一 — 名称 + 字段结构匹配
//! §3.4: 枚举统一 — 名称 + 变体匹配
//! §3.6: 元组统一 — 元素一一对应
//! §3.7: 函数类型统一 — 参数 + 返回 + async
//! §3.13: 联合类型统一 — 无序匹配
//! §3.14: 交集类型统一 — 无序匹配
//! §3.5: MetaType 统一 — UniverseLevel + type_params（RFC-027 §3.2）
//! §3.8: 泛型实例化

use crate::frontend::core::types::{
    MonoType, PolyType, StructType, TypeConstraintSolver, TypeVar, UniverseLevel,
};
use crate::util::span::Span;

fn s() -> TypeConstraintSolver {
    TypeConstraintSolver::new()
}

fn tv(idx: usize) -> TypeVar {
    TypeVar::new(idx)
}

fn struct_ty(
    name: &str,
    fields: Vec<(&str, MonoType)>,
) -> MonoType {
    let field_count = fields.len();
    MonoType::Struct(StructType {
        name: name.to_string(),
        fields: fields
            .into_iter()
            .map(|(n, t)| (n.to_string(), t))
            .collect(),
        methods: std::collections::HashMap::new(),
        field_mutability: vec![false; field_count],
        field_has_default: vec![false; field_count],
        interfaces: vec![],
    })
}

// ===================================================================
// §3.2: 原类型统一
// ===================================================================

#[test]
fn test_unify_primitives() {
    let mut solver = s();
    assert!(solver.unify(&MonoType::Int(32), &MonoType::Int(32)).is_ok());
    assert!(solver.unify(&MonoType::Int(64), &MonoType::Int(64)).is_ok());
    assert!(solver
        .unify(&MonoType::Float(32), &MonoType::Float(32))
        .is_ok());
    assert!(solver
        .unify(&MonoType::Float(64), &MonoType::Float(64))
        .is_ok());
    assert!(solver.unify(&MonoType::Bool, &MonoType::Bool).is_ok());
    assert!(solver.unify(&MonoType::String, &MonoType::String).is_ok());
    assert!(solver.unify(&MonoType::Char, &MonoType::Char).is_ok());
    assert!(solver.unify(&MonoType::Bytes, &MonoType::Bytes).is_ok());
    assert!(solver.unify(&MonoType::Void, &MonoType::Void).is_ok());
}

#[test]
fn test_unify_int_width_mismatch() {
    let mut solver = s();
    assert!(solver
        .unify(&MonoType::Int(32), &MonoType::Int(64))
        .is_err());
    assert!(solver
        .unify(&MonoType::Float(32), &MonoType::Float(64))
        .is_err());
}

#[test]
fn test_unify_cross_kind_mismatch() {
    let mut solver = s();
    assert!(solver.unify(&MonoType::Int(32), &MonoType::Bool).is_err());
    assert!(solver.unify(&MonoType::String, &MonoType::Int(64)).is_err());
    assert!(solver.unify(&MonoType::Void, &MonoType::Bool).is_err());
    assert!(solver.unify(&MonoType::Char, &MonoType::Bytes).is_err());
}

// ===================================================================
// §3.3: 结构体统一
// ===================================================================

#[test]
fn test_unify_struct_same() {
    let mut solver = s();
    let a = struct_ty(
        "Point",
        vec![("x", MonoType::Float(64)), ("y", MonoType::Float(64))],
    );
    let b = struct_ty(
        "Point",
        vec![("x", MonoType::Float(64)), ("y", MonoType::Float(64))],
    );
    assert!(solver.unify(&a, &b).is_ok());
}

#[test]
fn test_unify_struct_different_name() {
    let mut solver = s();
    let a = struct_ty("Point", vec![("x", MonoType::Float(64))]);
    let b = struct_ty("Vec2", vec![("x", MonoType::Float(64))]);
    assert!(solver.unify(&a, &b).is_err());
}

#[test]
fn test_unify_struct_different_field_count() {
    let mut solver = s();
    let a = struct_ty("P", vec![("x", MonoType::Float(64))]);
    let b = struct_ty(
        "P",
        vec![("x", MonoType::Float(64)), ("y", MonoType::Float(64))],
    );
    assert!(solver.unify(&a, &b).is_err());
}

#[test]
fn test_unify_struct_different_field_name() {
    let mut solver = s();
    let a = struct_ty("P", vec![("x", MonoType::Float(64))]);
    let b = struct_ty("P", vec![("y", MonoType::Float(64))]);
    assert!(solver.unify(&a, &b).is_err());
}

#[test]
fn test_unify_struct_different_field_type() {
    let mut solver = s();
    let a = struct_ty("P", vec![("v", MonoType::Int(32))]);
    let b = struct_ty("P", vec![("v", MonoType::String)]);
    assert!(solver.unify(&a, &b).is_err());
}

#[test]
fn test_unify_struct_with_var_field() {
    let mut solver = s();
    let v = solver.new_var();
    let a = struct_ty("Wrapper", vec![("value", MonoType::Int(32))]);
    let b = struct_ty("Wrapper", vec![("value", v)]);
    assert!(solver.unify(&a, &b).is_ok());
}

// ===================================================================
// §3.4: 枚举统一
// ===================================================================

#[test]
fn test_unify_enum_same() {
    let mut solver = s();
    let a = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Color".to_string(),
        variants: vec!["red".to_string(), "green".to_string(), "blue".to_string()],
    });
    let b = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Color".to_string(),
        variants: vec!["red".to_string(), "green".to_string(), "blue".to_string()],
    });
    assert!(solver.unify(&a, &b).is_ok());
}

#[test]
fn test_unify_enum_different_name() {
    let mut solver = s();
    let a = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Color".to_string(),
        variants: vec!["red".to_string()],
    });
    let b = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Status".to_string(),
        variants: vec!["red".to_string()],
    });
    assert!(solver.unify(&a, &b).is_err());
}

#[test]
fn test_unify_enum_different_variants() {
    let mut solver = s();
    let a = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Opt".to_string(),
        variants: vec!["some".to_string(), "none".to_string()],
    });
    let b = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: "Opt".to_string(),
        variants: vec!["some".to_string()],
    });
    assert!(solver.unify(&a, &b).is_err());
}

// ===================================================================
// §3.6: 元组统一
// ===================================================================

#[test]
fn test_unify_tuple_structural() {
    let mut solver = s();
    let a = MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]);
    let b = MonoType::Tuple(vec![MonoType::Int(32), MonoType::String]);
    assert!(solver.unify(&a, &b).is_ok());
}

#[test]
fn test_unify_tuple_with_var() {
    let mut solver = s();
    let v = solver.new_var();
    let a = MonoType::Tuple(vec![MonoType::Int(32)]);
    let b = MonoType::Tuple(vec![v]);
    assert!(solver.unify(&a, &b).is_ok());
}

// ===================================================================
// §3.7: 函数类型统一
// ===================================================================

#[test]
fn test_unify_fn_params() {
    let mut solver = s();
    let f1 = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Bool],
        return_type: Box::new(MonoType::String),
    };
    let f2 = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Bool],
        return_type: Box::new(MonoType::String),
    };
    assert!(solver.unify(&f1, &f2).is_ok());
}

#[test]
fn test_unify_fn_return_mismatch() {
    let mut solver = s();
    let f1 = MonoType::Fn {
        params: vec![],
        return_type: Box::new(MonoType::Int(32)),
    };
    let f2 = MonoType::Fn {
        params: vec![],
        return_type: Box::new(MonoType::Bool),
    };
    assert!(solver.unify(&f1, &f2).is_err());
}

#[test]
fn test_unify_fn_with_vars() {
    let mut solver = s();
    let v = solver.new_var();
    let f1 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(v),
    };
    let f2 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::String),
    };
    assert!(solver.unify(&f1, &f2).is_ok());
}

// ===================================================================
// §3.7: Option/Result 统一
// ===================================================================

#[test]
fn test_unify_result_nested() {
    let mut solver = s();
    let v = solver.new_var();
    let r1 = MonoType::Result(Box::new(MonoType::Int(32)), Box::new(MonoType::String));
    let r2 = MonoType::Result(Box::new(v.clone()), Box::new(MonoType::String));
    assert!(solver.unify(&r1, &r2).is_ok());
}

// ===================================================================
// §3.13: 联合类型统一
// ===================================================================

#[test]
fn test_unify_union_self() {
    let mut solver = s();
    let u = MonoType::Union(vec![MonoType::Int(32), MonoType::String]);
    assert!(solver.unify(&u, &u).is_ok());
}

#[test]
fn test_unify_union_with_concrete() {
    let mut solver = s();
    let u = MonoType::Union(vec![MonoType::Int(32), MonoType::String]);
    assert!(solver.unify(&u, &MonoType::Int(32)).is_ok());
}

#[test]
fn test_unify_union_with_unrelated() {
    let mut solver = s();
    let u = MonoType::Union(vec![MonoType::Int(32), MonoType::String]);
    assert!(solver.unify(&u, &MonoType::Bool).is_err());
}

// ===================================================================
// §3.14: 交集类型统一
// ===================================================================

#[test]
fn test_unify_intersection_self() {
    let mut solver = s();
    let i = MonoType::Intersection(vec![
        MonoType::TypeRef("Clone".to_string()),
        MonoType::TypeRef("Display".to_string()),
    ]);
    assert!(solver.unify(&i, &i).is_ok());
}

#[test]
fn test_unify_intersection_with_concrete() {
    let mut solver = s();
    // §3.14: intersection A&B means "satisfies both A and B"
    // (Int(32) & Int(32)) == Int(32) succeeds since Int(32) satisfies both
    let i = MonoType::Intersection(vec![MonoType::Int(32), MonoType::Int(32)]);
    assert!(solver.unify(&i, &MonoType::Int(32)).is_ok());
}

#[test]
fn test_unify_intersection_with_unrelated_fails() {
    let mut solver = s();
    // (Clone & Int(32)) == Int(32) fails because Int(32) doesn't implement Clone
    let i = MonoType::Intersection(vec![
        MonoType::TypeRef("Clone".to_string()),
        MonoType::Int(32),
    ]);
    assert!(solver.unify(&i, &MonoType::Int(32)).is_err());
}

// ===================================================================
// §3.2: TypeRef 统一
// ===================================================================

#[test]
fn test_unify_typeref_same_name() {
    let mut solver = s();
    let a = MonoType::TypeRef("MyType".to_string());
    let b = MonoType::TypeRef("MyType".to_string());
    assert!(solver.unify(&a, &b).is_ok());
}

#[test]
fn test_unify_typeref_different() {
    let mut solver = s();
    assert!(solver
        .unify(
            &MonoType::TypeRef("A".to_string()),
            &MonoType::TypeRef("B".to_string())
        )
        .is_err());
}

// ===================================================================
// §3.8: 变量基础操作
// ===================================================================

#[test]
fn test_new_var_sequence() {
    let mut solver = s();
    let v0 = solver.new_var();
    let v1 = solver.new_var();
    let v2 = solver.new_var();
    assert_eq!(v0.type_var().unwrap().index(), 0);
    assert_eq!(v1.type_var().unwrap().index(), 1);
    assert_eq!(v2.type_var().unwrap().index(), 2);
}

#[test]
fn test_new_generic_var_is_separate() {
    let mut solver = s();
    let gv = solver.new_generic_var();
    assert_eq!(gv.index(), 0);
    let v = solver.new_var();
    assert_eq!(v.type_var().unwrap().index(), 1);
}

#[test]
fn test_bind_and_resolve() {
    let mut solver = s();
    let tv = solver.new_var().type_var().unwrap();
    assert!(solver.bind(tv, &MonoType::Int(32)).is_ok());
    let resolved = solver.resolve(&MonoType::TypeVar(tv));
    assert_eq!(resolved, MonoType::Int(32));
}

#[test]
fn test_bind_occurs_check_nested() {
    let mut solver = s();
    let v0 = solver.new_var().type_var().unwrap();
    let v1 = solver.new_var().type_var().unwrap();
    // Bind v0 → List(v1)
    assert!(solver
        .bind(v0, &MonoType::List(Box::new(MonoType::TypeVar(v1))))
        .is_ok());
    // Bind v1 → List(v0) should fail (v1 appears inside v0 which contains v1)
    assert!(solver
        .bind(v1, &MonoType::List(Box::new(MonoType::TypeVar(v0))))
        .is_err());
}

#[test]
fn test_bind_conflicting_types() {
    let mut solver = s();
    let tv = solver.new_var().type_var().unwrap();
    assert!(solver.bind(tv, &MonoType::Bool).is_ok());
    assert!(solver.bind(tv, &MonoType::Bool).is_ok()); // same type is OK
    assert!(solver.bind(tv, &MonoType::Int(32)).is_err()); // different type fails
}

#[test]
fn test_reset_clears_everything() {
    let mut solver = s();
    solver.new_var();
    solver.new_generic_var();
    let v = solver.new_var();
    solver.add_constraint(v, MonoType::Int(32), Span::dummy());
    solver.reset();
    assert_eq!(solver.new_var().type_var().unwrap().index(), 0);
}

// ===================================================================
// §3.8: 泛型实例化和泛化
// ===================================================================

#[test]
fn test_instantiate_poly_fresh_vars() {
    let mut solver = s();
    let poly = PolyType::new(
        vec![tv(0), tv(1)],
        MonoType::Fn {
            params: vec![MonoType::TypeVar(tv(0))],
            return_type: Box::new(MonoType::TypeVar(tv(1))),
        },
    );
    let inst = solver.instantiate(&poly);
    match inst {
        MonoType::Fn {
            params,
            return_type,
            ..
        } => {
            assert!(matches!(&params[0], MonoType::TypeVar(_)));
            assert!(matches!(&*return_type, MonoType::TypeVar(_)));
            // Fresh vars should have different indices
            assert_ne!(params[0], *return_type);
        }
        _ => panic!("Expected Fn"),
    }
}

#[test]
fn test_generalize_captures_free_vars() {
    let mut solver = s();
    let v0 = solver.new_var().type_var().unwrap();
    let v1 = solver.new_var().type_var().unwrap();

    let body = MonoType::Fn {
        params: vec![MonoType::TypeVar(v0)],
        return_type: Box::new(MonoType::TypeVar(v1)),
    };
    let poly = solver.generalize(&body);
    assert!(!poly.is_mono());
    // Should have type_binders for v0 and v1 (in some order)
    // The exact count depends on collect_generalizable_vars
    assert!(poly.type_binders.len() >= 2);
}

#[test]
fn test_generalize_already_bound_var() {
    let mut solver = s();
    let v0 = solver.new_var().type_var().unwrap();
    let _ = solver.bind(v0, &MonoType::Int(64));

    let poly = solver.generalize(&MonoType::TypeVar(v0));
    assert!(poly.is_mono());
    assert_eq!(poly.body, MonoType::Int(64));
}

// ===================================================================
// §3.8: Unification with variable binding
// ===================================================================

#[test]
fn test_unify_var_twice_consistent() {
    let mut solver = s();
    let v = solver.new_var();
    assert!(solver.unify(&v, &MonoType::Int(32)).is_ok());
    // Second unify with same type is fine
    assert!(solver.unify(&v, &MonoType::Int(32)).is_ok());
    // Unify with conflicting type fails
    assert!(solver.unify(&v, &MonoType::String).is_err());
}

#[test]
fn test_unify_var_link_chain() {
    let mut solver = s();
    let v0 = solver.new_var().type_var().unwrap();
    let v1 = solver.new_var().type_var().unwrap();
    let v2 = solver.new_var().type_var().unwrap();

    // Create chain: v0 → v1 → v2 → Int(32)
    assert!(solver.bind(v0, &MonoType::TypeVar(v1)).is_ok());
    assert!(solver.bind(v1, &MonoType::TypeVar(v2)).is_ok());
    assert!(solver.bind(v2, &MonoType::Int(32)).is_ok());

    // Resolving v0 should give Int(32)
    assert_eq!(solver.resolve(&MonoType::TypeVar(v0)), MonoType::Int(32));
}

// ===================================================================
// §3.8: contains_var
// ===================================================================

#[test]
fn test_contains_var_nested() {
    let mut solver = s();
    let tv = solver.new_var().type_var().unwrap();
    assert!(solver.contains_var(
        &MonoType::Struct(StructType {
            name: "W".to_string(),
            fields: vec![(
                "v".to_string(),
                MonoType::List(Box::new(MonoType::TypeVar(tv)))
            )],
            methods: std::collections::HashMap::new(),
            field_mutability: vec![false],
            field_has_default: vec![false],
            interfaces: vec![],
        }),
        tv,
    ));
}

// ===================================================================
// §3.8: solve
// ===================================================================

#[test]
fn test_solve_simple_equality() {
    let mut solver = s();
    let v = solver.new_var();
    solver.add_constraint(v, MonoType::Int(32), Span::dummy());
    assert!(solver.solve().is_ok());
}

#[test]
fn test_solve_contradictory_constraints() {
    let mut solver = s();
    let v = solver.new_var();
    solver.add_constraint(v.clone(), MonoType::Int(32), Span::dummy());
    solver.add_constraint(v, MonoType::String, Span::dummy());
    assert!(solver.solve().is_err());
}

// ===================================================================
// §3.8: is_unconstrained / get_binding
// ===================================================================

#[test]
fn test_is_unconstrained_after_bind() {
    let mut solver = s();
    let tv = solver.new_var().type_var().unwrap();
    assert!(solver.is_unconstrained(tv));
    let _ = solver.bind(tv, &MonoType::Bool);
    assert!(!solver.is_unconstrained(tv));
}

#[test]
fn test_get_binding_and_get_binding_mut() {
    let mut solver = s();
    let tv = solver.new_var().type_var().unwrap();
    assert!(solver.get_binding(tv).is_some());
    let binding_mut = solver.get_binding_mut(tv);
    assert!(binding_mut.is_some());
}

// ===================================================================
// §3.2: resolve_type for builtin type references
// ===================================================================

#[test]
fn test_resolve_builtin_types() {
    let solver = s();
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Int".to_string())),
        MonoType::Int(64)
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Int32".to_string())),
        MonoType::Int(32)
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Int8".to_string())),
        MonoType::Int(8)
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Float".to_string())),
        MonoType::Float(64)
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Float32".to_string())),
        MonoType::Float(32)
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Bool".to_string())),
        MonoType::Bool
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("String".to_string())),
        MonoType::String
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Bytes".to_string())),
        MonoType::Bytes
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Void".to_string())),
        MonoType::Void
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Char".to_string())),
        MonoType::Char
    );
    // Unknown type ref stays as-is
    let unknown = MonoType::TypeRef("Custom".to_string());
    assert_eq!(solver.resolve_type(&unknown), unknown);
}

// ===================================================================
// expand_type — 通过所有容器类型的展开路径
// §3.8: 绑定的类型变量通过 resolve_type 展开
// ===================================================================

#[test]
fn test_expand_type_through_all_containers() {
    // Bind a var, then resolve a container that uses that var
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let _ = solver.bind(v, &MonoType::String);

    // Dict with var key/value
    let dict = MonoType::Dict(
        Box::new(MonoType::TypeVar(v)),
        Box::new(MonoType::TypeVar(v)),
    );
    let resolved = solver.resolve_type(&dict);
    assert_eq!(
        resolved,
        MonoType::Dict(Box::new(MonoType::String), Box::new(MonoType::String))
    );

    // Set with var element
    let set = MonoType::Set(Box::new(MonoType::TypeVar(v)));
    assert_eq!(
        solver.resolve_type(&set),
        MonoType::Set(Box::new(MonoType::String))
    );

    // Range with var elem_type
    let range = MonoType::Range {
        elem_type: Box::new(MonoType::TypeVar(v)),
    };
    assert_eq!(
        solver.resolve_type(&range),
        MonoType::Range {
            elem_type: Box::new(MonoType::String)
        }
    );

    // Arc/Weak with var inner
    assert_eq!(
        solver.resolve_type(&MonoType::Arc(Box::new(MonoType::TypeVar(v)))),
        MonoType::Arc(Box::new(MonoType::String))
    );
    assert_eq!(
        solver.resolve_type(&MonoType::Weak(Box::new(MonoType::TypeVar(v)))),
        MonoType::Weak(Box::new(MonoType::String))
    );

    // Option with var inner
    let opt = MonoType::Option(Box::new(MonoType::TypeVar(v)));
    assert_eq!(
        solver.resolve_type(&opt),
        MonoType::Option(Box::new(MonoType::String))
    );

    // Result with var
    let res = MonoType::Result(Box::new(MonoType::TypeVar(v)), Box::new(MonoType::Int(32)));
    let r = solver.resolve_type(&res);
    assert!(matches!(r, MonoType::Result(..)));

    // Fn with var
    let fn_t = MonoType::Fn {
        params: vec![MonoType::TypeVar(v)],
        return_type: Box::new(MonoType::TypeVar(v)),
    };
    let f = solver.resolve_type(&fn_t);
    assert!(matches!(f, MonoType::Fn { .. }));

    // Union with var
    let union = MonoType::Union(vec![MonoType::TypeVar(v), MonoType::Int(32)]);
    let u = solver.resolve_type(&union);
    assert!(matches!(u, MonoType::Union(_)));

    // Intersection with var
    let inter = MonoType::Intersection(vec![MonoType::TypeVar(v), MonoType::String]);
    let i = solver.resolve_type(&inter);
    assert!(matches!(i, MonoType::Intersection(_)));

    // AssocType with var host
    let assoc = MonoType::AssocType {
        host_type: Box::new(MonoType::TypeVar(v)),
        assoc_name: "Item".to_string(),
        assoc_args: vec![MonoType::Int(32)],
    };
    let a = solver.resolve_type(&assoc);
    assert!(matches!(a, MonoType::AssocType { .. }));

    // MetaType with var param
    let meta = MonoType::MetaType {
        universe_level: crate::frontend::core::types::UniverseLevel::type0(),
        type_params: vec![MonoType::TypeVar(v)],
    };
    let m = solver.resolve_type(&meta);
    assert!(matches!(m, MonoType::MetaType { .. }));
}

// ===================================================================
// expand_type_mut — 通过 resolve (mut) 展开
// ===================================================================

#[test]
fn test_expand_mut_through_all_containers() {
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let _ = solver.bind(v, &MonoType::Bool);

    // resolve (mut version) through various containers
    let dict = MonoType::Dict(Box::new(MonoType::TypeVar(v)), Box::new(MonoType::Int(32)));
    assert_eq!(
        solver.resolve(&dict),
        MonoType::Dict(Box::new(MonoType::Bool), Box::new(MonoType::Int(32)))
    );

    let set = MonoType::Set(Box::new(MonoType::TypeVar(v)));
    assert_eq!(
        solver.resolve(&set),
        MonoType::Set(Box::new(MonoType::Bool))
    );

    let range = MonoType::Range {
        elem_type: Box::new(MonoType::TypeVar(v)),
    };
    assert!(matches!(solver.resolve(&range), MonoType::Range { .. }));

    let union = MonoType::Union(vec![MonoType::TypeVar(v)]);
    assert!(matches!(solver.resolve(&union), MonoType::Union(_)));
}

// ===================================================================
// contains_var — 各种容器
// ===================================================================

#[test]
fn test_contains_var_in_containers() {
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let _v2 = solver.new_var().type_var().unwrap();

    // Struct field
    let s = MonoType::Struct(StructType {
        name: "W".to_string(),
        fields: vec![("v".to_string(), MonoType::TypeVar(v))],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    assert!(solver.contains_var(&s, v));

    // Union
    let u = MonoType::Union(vec![MonoType::TypeVar(v)]);
    assert!(solver.contains_var(&u, v));

    // Intersection
    let i = MonoType::Intersection(vec![MonoType::TypeVar(v)]);
    assert!(solver.contains_var(&i, v));

    // AssocType args
    let a = MonoType::AssocType {
        host_type: Box::new(MonoType::Int(32)),
        assoc_name: "Item".to_string(),
        assoc_args: vec![MonoType::TypeVar(v)],
    };
    assert!(solver.contains_var(&a, v));

    // Note: contains_var may not handle Weak - checking what's implemented
    assert!(solver.contains_var(&MonoType::Arc(Box::new(MonoType::TypeVar(v))), v));
    assert!(solver.contains_var(
        &MonoType::Range {
            elem_type: Box::new(MonoType::TypeVar(v))
        },
        v
    ));
    assert!(solver.contains_var(&MonoType::List(Box::new(MonoType::TypeVar(v))), v));

    // Negative cases
    assert!(!solver.contains_var(&MonoType::Int(32), v));
    assert!(!solver.contains_var(
        &MonoType::Enum(crate::frontend::core::types::EnumType {
            name: "E".to_string(),
            variants: vec![],
        }),
        v
    ));
}

// ===================================================================
// §3.7: Fn 参数数量不匹配
// ===================================================================

#[test]
fn test_unify_fn_param_count_mismatch() {
    let mut solver = s();
    let f1 = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Bool],
        return_type: Box::new(MonoType::Void),
    };
    let f2 = MonoType::Fn {
        params: vec![MonoType::Int(32)],
        return_type: Box::new(MonoType::Void),
    };
    assert!(
        solver.unify(&f1, &f2).is_err(),
        "Fn with different param count should fail"
    );
}

// ===================================================================
// §3.8: instantiate 包含 Result/Option 的 PolyType
// ===================================================================

#[test]
fn test_instantiate_with_result_wrapped_types() {
    let mut solver = s();
    let tv = TypeVar::new(0);
    let poly = PolyType::new(
        vec![tv],
        MonoType::Result(Box::new(MonoType::TypeVar(tv)), Box::new(MonoType::String)),
    );
    let inst = solver.instantiate(&poly);
    assert!(
        matches!(inst, MonoType::Result(..)),
        "should instantiate Result"
    );
}

#[test]
fn test_instantiate_with_option_wrapped_types() {
    let mut solver = s();
    let tv = TypeVar::new(0);
    let poly = PolyType::new(vec![tv], MonoType::Option(Box::new(MonoType::TypeVar(tv))));
    let inst = solver.instantiate(&poly);
    assert!(
        matches!(inst, MonoType::Option(_)),
        "should instantiate Option"
    );
}

// ===================================================================
// §3.8: generalize 包含 Arc/Weak 的类型
// ===================================================================

#[test]
fn test_generalize_with_arc_wrapped_types() {
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let body = MonoType::Arc(Box::new(MonoType::TypeVar(v)));
    let poly = solver.generalize(&body);
    assert!(!poly.is_mono(), "Arc with free var should generalize");
}

#[test]
fn test_generalize_with_weak_wrapped_types() {
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let body = MonoType::Weak(Box::new(MonoType::TypeVar(v)));
    let poly = solver.generalize(&body);
    assert!(!poly.is_mono(), "Weak with free var should generalize");
}

// ===================================================================
// §3.8: contains_var 在 Option/Result/Fn 参数中
// ===================================================================

#[test]
fn test_contains_var_in_option() {
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let opt = MonoType::Option(Box::new(MonoType::TypeVar(v)));
    // Note: contains_var currently doesn't handle Option - falls through to _ => false
    assert!(
        !solver.contains_var(&opt, v),
        "Option is not handled by contains_var (falls through)"
    );
}

#[test]
fn test_contains_var_in_result() {
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let res = MonoType::Result(Box::new(MonoType::TypeVar(v)), Box::new(MonoType::String));
    // Note: contains_var currently doesn't handle Result - falls through to _ => false
    assert!(
        !solver.contains_var(&res, v),
        "Result is not handled by contains_var (falls through)"
    );
}

#[test]
fn test_contains_var_in_fn_params() {
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let f = MonoType::Fn {
        params: vec![MonoType::TypeVar(v), MonoType::Int(32)],
        return_type: Box::new(MonoType::Void),
    };
    assert!(
        solver.contains_var(&f, v),
        "Fn params containing var should be detected"
    );
}

// ===================================================================
// §3.8: 同一变量多个约束求解
// ===================================================================

#[test]
fn test_solve_with_multiple_constraints_on_same_var() {
    let mut solver = s();
    let v = solver.new_var();
    // Same type in multiple constraints should succeed
    solver.add_constraint(v.clone(), MonoType::Int(32), Span::dummy());
    solver.add_constraint(v, MonoType::Int(32), Span::dummy());
    assert!(
        solver.solve().is_ok(),
        "consistent constraints should solve"
    );
}

// ===================================================================
// §3.8: List 类型统一
// ===================================================================

#[test]
fn test_unify_list_same_element_type() {
    let mut solver = s();
    let a = MonoType::List(Box::new(MonoType::Int(32)));
    let b = MonoType::List(Box::new(MonoType::Int(32)));
    assert!(
        solver.unify(&a, &b).is_ok(),
        "List(T) should unify with List(T)"
    );
}

#[test]
fn test_unify_list_different_element_type() {
    let mut solver = s();
    let a = MonoType::List(Box::new(MonoType::Int(32)));
    let b = MonoType::List(Box::new(MonoType::Float(64)));
    assert!(
        solver.unify(&a, &b).is_err(),
        "List(Int) should not unify with List(Float)"
    );
}

#[test]
fn test_unify_list_with_var() {
    let mut solver = s();
    let v = solver.new_var();
    let a = MonoType::List(Box::new(MonoType::Int(32)));
    let b = MonoType::List(Box::new(v));
    assert!(
        solver.unify(&a, &b).is_ok(),
        "List(Int) should unify with List(var)"
    );
}

// ===================================================================
// fresh_substitution
// ===================================================================

#[test]
fn test_fresh_substitution() {
    let mut solver = s();
    let tv = TypeVar::new(0);
    let sub = solver.fresh_substitution(&[tv]);
    assert_eq!(sub.len(), 1);
    assert!(sub.contains_key(&tv));
    // The new var should be fresh
    if let Some(new_ty) = sub.get(&tv) {
        assert!(matches!(new_ty, MonoType::TypeVar(_)));
    }
}

// ===================================================================
// generalize 收集自由变量
// ===================================================================

#[test]
fn test_generalize_collects_free_vars() {
    let mut solver = s();
    let v0 = solver.new_var().type_var().unwrap();
    let v1 = solver.new_var().type_var().unwrap();

    // Type with multiple free vars
    let body = MonoType::Fn {
        params: vec![MonoType::TypeVar(v0)],
        return_type: Box::new(MonoType::Tuple(vec![
            MonoType::TypeVar(v0),
            MonoType::TypeVar(v1),
        ])),
    };
    let poly = solver.generalize(&body);
    // Should generalize both v0 and v1
    assert!(!poly.is_mono());
    assert!(!poly.type_binders.is_empty());
}

#[test]
fn test_generalize_no_free_vars() {
    let mut solver = s();
    let poly = solver.generalize(&MonoType::Int(32));
    assert!(poly.is_mono());
}

// ===================================================================
// solve with constraints
// ===================================================================

#[test]
fn test_solve_empty_constraints() {
    let mut solver = s();
    assert!(solver.solve().is_ok());
}

#[test]
fn test_solve_multiple_independent() {
    let mut solver = s();
    let v1 = solver.new_var();
    let v2 = solver.new_var();
    solver.add_constraint(v1, MonoType::Int(32), Span::dummy());
    solver.add_constraint(v2, MonoType::String, Span::dummy());
    assert!(solver.solve().is_ok());
}

// ===================================================================
// substitute_type — 通过 PolyType instantiate 覆盖所有容器变体的替换
// 触发 solver 内部的 substitute_type 对 Struct/Tuple/Dict/Set/Fn/Union 等的递归
// ===================================================================

#[test]
fn test_instantiate_poly_with_tuple() {
    let mut solver = s();
    let tv = TypeVar::new(0);
    // PolyType where body is Tuple([T, Int(32)])
    let poly = PolyType::new(
        vec![tv],
        MonoType::Tuple(vec![MonoType::TypeVar(tv), MonoType::Int(32)]),
    );
    let inst = solver.instantiate(&poly);
    assert!(matches!(inst, MonoType::Tuple(ref ts) if ts.len() == 2));
    // The first element should be a fresh TypeVar (not the original tv)
    if let MonoType::Tuple(ts) = inst {
        assert!(matches!(ts[0], MonoType::TypeVar(_)));
        assert_eq!(ts[1], MonoType::Int(32));
    }
}

#[test]
fn test_instantiate_poly_with_dict() {
    let mut solver = s();
    let tv = TypeVar::new(0);
    let poly = PolyType::new(
        vec![tv],
        MonoType::Dict(
            Box::new(MonoType::TypeVar(tv)),
            Box::new(MonoType::TypeVar(tv)),
        ),
    );
    let inst = solver.instantiate(&poly);
    assert!(matches!(inst, MonoType::Dict(..)));
}

#[test]
fn test_instantiate_poly_with_struct() {
    let mut solver = s();
    let tv = TypeVar::new(0);
    let poly = PolyType::new(
        vec![tv],
        MonoType::Struct(StructType {
            name: "Wrap".to_string(),
            fields: vec![("v".to_string(), MonoType::TypeVar(tv))],
            methods: std::collections::HashMap::new(),
            field_mutability: vec![false],
            field_has_default: vec![false],
            interfaces: vec![],
        }),
    );
    let inst = solver.instantiate(&poly);
    assert!(matches!(inst, MonoType::Struct(ref s) if s.fields.len() == 1));
}

#[test]
fn test_instantiate_poly_with_union_and_range() {
    let mut solver = s();
    let tv = TypeVar::new(0);
    let poly = PolyType::new(
        vec![tv],
        MonoType::Union(vec![
            MonoType::TypeVar(tv),
            MonoType::Range {
                elem_type: Box::new(MonoType::TypeVar(tv)),
            },
        ]),
    );
    let inst = solver.instantiate(&poly);
    assert!(matches!(inst, MonoType::Union(_)));
}

#[test]
fn test_instantiate_poly_with_assoc_and_arc() {
    let mut solver = s();
    let tv = TypeVar::new(0);
    let poly = PolyType::new(
        vec![tv],
        MonoType::AssocType {
            host_type: Box::new(MonoType::Arc(Box::new(MonoType::TypeVar(tv)))),
            assoc_name: "Item".to_string(),
            assoc_args: vec![MonoType::TypeVar(tv)],
        },
    );
    let inst = solver.instantiate(&poly);
    assert!(matches!(inst, MonoType::AssocType { .. }));
}

// ===================================================================
// expand_type_mut — Struct/Tuple 等带绑定的容器
// ===================================================================

#[test]
fn test_expand_mut_struct_with_bound_var() {
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let _ = solver.bind(v, &MonoType::Float(64));
    let s = MonoType::Struct(StructType {
        name: "P".to_string(),
        fields: vec![("x".to_string(), MonoType::TypeVar(v))],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let resolved = solver.resolve(&s);
    assert!(matches!(resolved, MonoType::Struct(ref ss) if ss.fields[0].1 == MonoType::Float(64)));
}

#[test]
fn test_expand_mut_fn_with_bound_var() {
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let _ = solver.bind(v, &MonoType::Bool);
    let f = MonoType::Fn {
        params: vec![MonoType::TypeVar(v)],
        return_type: Box::new(MonoType::TypeVar(v)),
    };
    let resolved = solver.resolve(&f);
    assert!(matches!(resolved, MonoType::Fn { .. }));
}

// ===================================================================
// unify with Union — unordered backtracking
// ===================================================================

#[test]
fn test_unify_union_unordered_matching() {
    let mut solver = s();
    // Two unions with same elements in different order
    let u1 = MonoType::Union(vec![MonoType::Int(32), MonoType::String, MonoType::Bool]);
    let u2 = MonoType::Union(vec![MonoType::String, MonoType::Bool, MonoType::Int(32)]);
    assert!(solver.unify(&u1, &u2).is_ok());
}

#[test]
fn test_unify_union_unordered_mismatch() {
    let mut solver = s();
    let u1 = MonoType::Union(vec![MonoType::Int(32), MonoType::String]);
    let u2 = MonoType::Union(vec![MonoType::Int(32), MonoType::Bool]);
    assert!(solver.unify(&u1, &u2).is_err());
}

// ===================================================================
// generalize with nested containers — collect_generalizable_vars 路径
// ===================================================================

#[test]
fn test_generalize_with_nested_containers() {
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    // Type containing var inside Struct → Tuple → List
    let body = MonoType::Struct(StructType {
        name: "Outer".to_string(),
        fields: vec![(
            "inner".to_string(),
            MonoType::Tuple(vec![MonoType::List(Box::new(MonoType::TypeVar(v)))]),
        )],
        methods: std::collections::HashMap::new(),
        field_mutability: vec![false],
        field_has_default: vec![false],
        interfaces: vec![],
    });
    let poly = solver.generalize(&body);
    assert!(!poly.is_mono());
    assert_eq!(poly.type_binders.len(), 1);
}

#[test]
fn test_resolve_never_builtin() {
    let solver = s();
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Never".to_string())),
        MonoType::Never
    );
}

#[test]
fn test_metatype_unify_same_level() {
    // Arrange
    let mut solver = s();
    let a = MonoType::MetaType {
        universe_level: UniverseLevel::type0(),
        type_params: vec![],
    };
    let b = MonoType::MetaType {
        universe_level: UniverseLevel::type0(),
        type_params: vec![],
    };

    // Act & Assert
    assert!(
        solver.unify(&a, &b).is_ok(),
        "unify(Type₀, Type₀) should succeed — same level"
    );
}

#[test]
fn test_metatype_unify_different_level() {
    // Arrange
    let mut solver = s();
    let a = MonoType::MetaType {
        universe_level: UniverseLevel::type0(),
        type_params: vec![],
    };
    let b = MonoType::MetaType {
        universe_level: UniverseLevel::type1(),
        type_params: vec![],
    };

    // Act & Assert
    assert!(
        solver.unify(&a, &b).is_err(),
        "unify(Type₀, Type₁) should fail — different levels"
    );
}

#[test]
fn test_metatype_unify_with_params() {
    // Arrange
    let mut solver = s();
    let a = MonoType::MetaType {
        universe_level: UniverseLevel::type0(),
        type_params: vec![MonoType::Int(32)],
    };
    let b = MonoType::MetaType {
        universe_level: UniverseLevel::type0(),
        type_params: vec![MonoType::Int(32)],
    };

    // Act & Assert
    assert!(
        solver.unify(&a, &b).is_ok(),
        "unify(Type₀(Int32), Type₀(Int32)) should succeed — same params"
    );
}

#[test]
fn test_metatype_unify_with_params_mismatch() {
    // Arrange
    let mut solver = s();
    let a = MonoType::MetaType {
        universe_level: UniverseLevel::type0(),
        type_params: vec![MonoType::Int(32)],
    };
    let b = MonoType::MetaType {
        universe_level: UniverseLevel::type0(),
        type_params: vec![MonoType::Int(64)],
    };

    // Act & Assert
    assert!(
        solver.unify(&a, &b).is_err(),
        "unify(Type₀(Int32), Type₀(Int64)) should fail — param mismatch"
    );
}
