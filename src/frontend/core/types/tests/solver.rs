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

use crate::frontend::core::types::{MonoType, StructType, TypeConstraintSolver, UniverseLevel};
use crate::util::span::Span;

fn s() -> TypeConstraintSolver {
    TypeConstraintSolver::new()
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

// §3.2: 原类型统一

/// Helper: test_contains_var_in_containers 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: 求解器 + 两个互不相同的类型变量（`v` 与 `_v2`）。
fn solver_with_two_type_vars() -> (
    TypeConstraintSolver,
    crate::frontend::core::types::TypeVar,
    crate::frontend::core::types::TypeVar,
) {
    let mut solver = s();
    let v = solver.new_var().type_var().expect("新建类型变量应成功");
    let _v2 = solver.new_var().type_var().expect("新建类型变量应成功");

    (solver, v, _v2)
}

/// Assert: 含 `v` 的容器（Struct/Union/Intersection/AssocType/泛型/Lst）均被识别。
fn assert_contains_var_positive(
    solver: &TypeConstraintSolver,
    v: crate::frontend::core::types::TypeVar,
) {
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
    assert!(solver.contains_var(
        &MonoType::Generic {
            name: "Arc".into(),
            args: vec![MonoType::TypeVar(v)]
        },
        v
    ));
    assert!(solver.contains_var(
        &MonoType::Generic {
            name: "Range".into(),
            args: vec![MonoType::TypeVar(v)]
        },
        v
    ));
    assert!(solver.contains_var(&MonoType::make_list(MonoType::TypeVar(v)), v));
}

/// Assert: 不含 `v` 的类型（Int32、空枚举）返回 false。
fn assert_contains_var_negative(
    solver: &TypeConstraintSolver,
    v: crate::frontend::core::types::TypeVar,
) {
    assert!(!solver.contains_var(&MonoType::Int(32), v));
    assert!(!solver.contains_var(
        &MonoType::Enum(crate::frontend::core::types::EnumType {
            name: "E".to_string(),
            variants: vec![],
        }),
        v
    ));
}

/// Helper: test_expand_type_through_all_containers 的完整夹具与断言（逐条断言见函数体）。
/// Fixture: 求解器 + 一个已绑定为 `String` 的类型变量。
fn solver_with_bound_string_var() -> (TypeConstraintSolver, crate::frontend::core::types::TypeVar) {
    // Bind a var, then resolve a container that uses that var
    let mut solver = s();
    let v = solver.new_var().type_var().expect("新建类型变量应成功");
    let _ = solver.bind(v, &MonoType::make_string());

    (solver, v)
}

/// Assert: 容器（Dict/Set/Range/Arc/Weak/Option）内变量展开为 String。
fn assert_var_containers_resolved(
    solver: &TypeConstraintSolver,
    v: crate::frontend::core::types::TypeVar,
) {
    // Dict with var key/value
    let dict = MonoType::make_dict(MonoType::TypeVar(v), MonoType::TypeVar(v));
    let resolved = solver.resolve_type(&dict);
    assert_eq!(
        resolved,
        MonoType::make_dict(MonoType::make_string(), MonoType::make_string())
    );
    // Set with var element
    let set = MonoType::Generic {
        name: "Set".into(),
        args: vec![MonoType::TypeVar(v)],
    };
    assert_eq!(
        solver.resolve_type(&set),
        MonoType::Generic {
            name: "Set".into(),
            args: vec![MonoType::make_string()]
        }
    );
    // Range with var elem_type
    let range = MonoType::Generic {
        name: "Range".into(),
        args: vec![MonoType::TypeVar(v)],
    };
    assert_eq!(
        solver.resolve_type(&range),
        MonoType::Generic {
            name: "Range".into(),
            args: vec![MonoType::make_string()]
        }
    );
    // Arc/Weak with var inner
    assert_eq!(
        solver.resolve_type(&MonoType::Generic {
            name: "Arc".into(),
            args: vec![MonoType::TypeVar(v)]
        }),
        MonoType::Generic {
            name: "Arc".into(),
            args: vec![MonoType::make_string()]
        }
    );
    assert_eq!(
        solver.resolve_type(&MonoType::Generic {
            name: "Weak".into(),
            args: vec![MonoType::TypeVar(v)]
        }),
        MonoType::Generic {
            name: "Weak".into(),
            args: vec![MonoType::make_string()]
        }
    );
    // Option with var inner
    let opt = MonoType::make_option(MonoType::TypeVar(v));
    assert_eq!(
        solver.resolve_type(&opt),
        MonoType::make_option(MonoType::make_string())
    );
}

/// Assert: 复合类型（Result/Fn/Union/Intersection/AssocType/MetaType）保持各自形态。
fn assert_composite_types_preserved(
    solver: &TypeConstraintSolver,
    v: crate::frontend::core::types::TypeVar,
) {
    // Result with var
    let res = MonoType::make_result(MonoType::TypeVar(v), MonoType::Int(32));
    let r = solver.resolve_type(&res);
    assert!(matches!(r, MonoType::Generic { name, .. } if name == "Result"));
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
    let inter = MonoType::Intersection(vec![MonoType::TypeVar(v), MonoType::make_string()]);
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

/// Helper: test_resolve_builtin_types 的完整夹具与断言（逐条断言见函数体）。
/// Assert: 内建类型名（Int/Int32/Int8/Float/Float32/Bool/String/Bytes/Void/Char）的解析结果。
fn assert_builtin_resolutions(solver: &TypeConstraintSolver) {
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
        MonoType::make_string()
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Bytes".to_string())),
        MonoType::Generic {
            name: "Bytes".into(),
            args: vec![]
        }
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Void".to_string())),
        MonoType::Void
    );
    assert_eq!(
        solver.resolve_type(&MonoType::TypeRef("Char".to_string())),
        MonoType::Char
    );
}

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
    assert!(solver
        .unify(&MonoType::make_string(), &MonoType::make_string())
        .is_ok());
    assert!(solver.unify(&MonoType::Char, &MonoType::Char).is_ok());
    let bytes = MonoType::Generic {
        name: "Bytes".into(),
        args: vec![],
    };
    assert!(solver.unify(&bytes, &bytes).is_ok());
    assert!(solver.unify(&MonoType::Void, &MonoType::Void).is_ok());
}

#[test]
fn test_unify_int_width_mismatch() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut solver = s();
    assert!(solver
        .unify(&MonoType::Int(32), &MonoType::Int(64))
        .is_err());
    assert!(solver
        .unify(&MonoType::Float(32), &MonoType::Float(64))
        .is_err());
}

#[test]
fn test_unify_struct_different_field_type() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut solver = s();
    let a = struct_ty("P", vec![("v", MonoType::Int(32))]);
    let b = struct_ty("P", vec![("v", MonoType::make_string())]);
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

// §3.6: 元组统一

#[test]
fn test_unify_tuple_structural() {
    let mut solver = s();
    let a = MonoType::make_tuple(vec![MonoType::Int(32), MonoType::make_string()]);
    let b = MonoType::make_tuple(vec![MonoType::Int(32), MonoType::make_string()]);
    assert!(solver.unify(&a, &b).is_ok());
}

// §3.7: 函数类型统一

#[test]
fn test_unify_fn_params() {
    let mut solver = s();
    let f1 = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Bool],
        return_type: Box::new(MonoType::make_string()),
    };
    let f2 = MonoType::Fn {
        params: vec![MonoType::Int(32), MonoType::Bool],
        return_type: Box::new(MonoType::make_string()),
    };
    assert!(solver.unify(&f1, &f2).is_ok());
}

// §3.8: 变量基础操作

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
fn test_bind_occurs_check_nested() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut solver = s();
    let v0 = solver.new_var().type_var().unwrap();
    let v1 = solver.new_var().type_var().unwrap();
    // Bind v0 → List(v1)
    assert!(solver
        .bind(v0, &MonoType::make_list(MonoType::TypeVar(v1)))
        .is_ok());
    // Bind v1 → List(v0) should fail (v1 appears inside v0 which contains v1)
    assert!(solver
        .bind(v1, &MonoType::make_list(MonoType::TypeVar(v0)))
        .is_err());
}

#[test]
fn test_bind_occurs_check_through_ref() {
    // Arrange — 借用位是 occurs check 必须**穿透**的一层：`&T` 与 `T` 是同一个值世界
    // 的类型（借用编译期零大小）。缺这一臂时 `bind(t, &t)` 被判「不含 t」而建立
    // 自指绑定，此后任何 resolve_type(t) 都在 expand_type 里无限自递归（实测栈溢出）。
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut solver = s();
    let v = solver.new_var().type_var().unwrap();
    let self_ref = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::TypeVar(v)),
    };

    // Act & Assert — 自指借用必须被拒
    assert!(
        solver.bind(v, &self_ref).is_err(),
        "occurs check 必须穿透借用位（`bind(t, &t)` 是无限类型）"
    );
    // 借用内层是**别的**变量时照常可绑——过滤不是「见 Ref 一律拒」
    let w = solver.new_var().type_var().unwrap();
    let borrowed_other = MonoType::Ref {
        mutable: true,
        inner: Box::new(MonoType::TypeVar(w)),
    };
    assert!(
        solver.bind(v, &borrowed_other).is_ok(),
        "借用内层是另一个类型变量时应正常绑定"
    );
}

#[test]
fn test_bind_conflicting_types() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut solver = s();
    let tv = solver.new_var().type_var().unwrap();
    assert!(solver.bind(tv, &MonoType::Bool).is_ok());
    assert!(solver.bind(tv, &MonoType::Bool).is_ok()); // same type is OK
    assert!(solver.bind(tv, &MonoType::Int(32)).is_err()); // different type fails
}

// §3.8: 泛型实例化和泛化

// §3.8: Unification with variable binding

#[test]
fn test_unify_var_twice_consistent() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut solver = s();
    let v = solver.new_var();
    assert!(solver.unify(&v, &MonoType::Int(32)).is_ok());
    // Second unify with same type is fine
    assert!(solver.unify(&v, &MonoType::Int(32)).is_ok());
    // Unify with conflicting type fails
    assert!(solver.unify(&v, &MonoType::make_string()).is_err());
}

#[test]
fn test_solve_contradictory_constraints() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut solver = s();
    let v = solver.new_var();
    solver.add_constraint(v.clone(), MonoType::Int(32), Span::dummy());
    solver.add_constraint(v, MonoType::make_string(), Span::dummy());
    assert!(solver.solve().is_err());
}

// §3.8: is_unconstrained / get_binding

#[test]
fn test_get_binding_and_get_binding_mut() {
    let mut solver = s();
    let tv = solver.new_var().type_var().unwrap();
    assert!(solver.get_binding(tv).is_some());
    let binding_mut = solver.get_binding_mut(tv);
    assert!(binding_mut.is_some());
}

// §3.2: resolve_type for builtin type references

#[test]
fn test_resolve_builtin_types() {
    // Arrange
    let solver = s();

    // Act & Assert — 内建类型名解析为目标类型
    assert_builtin_resolutions(&solver);

    // Unknown type ref stays as-is
    let unknown = MonoType::TypeRef("Custom".to_string());
    assert_eq!(solver.resolve_type(&unknown), unknown);
}

// expand_type — 通过所有容器类型的展开路径
// §3.8: 绑定的类型变量通过 resolve_type 展开

#[test]
fn test_expand_type_through_all_containers() {
    // Arrange — 绑定一个 String 类型变量，再解析使用该变量的容器
    let (solver, v) = solver_with_bound_string_var();

    // Act & Assert — 容器内变量展开为 String
    assert_var_containers_resolved(&solver, v);

    // Act & Assert — 复合类型保持各自形态（不被错误展开）
    assert_composite_types_preserved(&solver, v);
}

// contains_var — 各种容器

#[test]
fn test_contains_var_in_containers() {
    // Arrange — 两个互不相同的类型变量
    let (solver, v, _v2) = solver_with_two_type_vars();

    // Act & Assert — 含 v 的容器应被识别为「包含该变量」
    assert_contains_var_positive(&solver, v);

    // Act & Assert — 不含 v 的类型应返回 false
    assert_contains_var_negative(&solver, v);
}

// §3.8: List 类型统一

#[test]
fn test_unify_list_same_element_type() {
    let mut solver = s();
    let a = MonoType::make_list(MonoType::Int(32));
    let b = MonoType::make_list(MonoType::Int(32));
    assert!(
        solver.unify(&a, &b).is_ok(),
        "List(T) should unify with List(T)"
    );
}

#[test]
fn test_unify_list_different_element_type() {
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
    let mut solver = s();
    let a = MonoType::make_list(MonoType::Int(32));
    let b = MonoType::make_list(MonoType::Float(64));
    assert!(
        solver.unify(&a, &b).is_err(),
        "List(Int) should not unify with List(Float)"
    );
}

// generalize 收集自由变量

// solve with constraints

#[test]
fn test_solve_empty_constraints() {
    let mut solver = s();
    assert!(solver.solve().is_ok());
}

// substitute_type — 通过 PolyType instantiate 覆盖所有容器变体的替换
// 触发 solver 内部的 substitute_type 对 Struct/Tuple/Dict/Set/Fn/Union 等的递归

// expand_type_mut — Struct/Tuple 等带绑定的容器

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
    // #324：这些 API 生产上运行于类型检查 walk 内（guard 覆盖），单测直调需模拟 walk 上下文
    let _walk_guard = crate::util::diagnostic::push_current_span(crate::util::span::Span::dummy());
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
