//! 类型替换测试 — 补充内联测试未覆盖的容器类型替换路径
//!
//! Substitution: bind, insert, get, contains_var, merge, len, is_empty, bound_vars
//! Substituter: substitute 经过所有容器类型 (Struct, Tuple, Dict, Set, Enum, Range, Arc, Weak, Union, Intersection, AssocType, Fn, Option, Result)

use crate::frontend::core::types::identity::Nominal;
use crate::frontend::core::types::mono::StructFieldInfo;
use crate::frontend::core::types::substitute::contains_type_vars;
use crate::frontend::core::types::substitute::rebind_nominal_refs;
use crate::frontend::core::types::{MonoType, StructType, Substituter, Substitution, TypeVar};
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// 借用位的「含类型变量」判据（WBS 6.8.3 病灶 M2 的**承重判据**，单元级）
// ─────────────────────────────────────────────────────────────────────────────
//
// 与 `TypeConstraintSolver::contains_var` 同族：缺 `Ref` 臂时「含类型变量」对借用位
// 恒为假，凡依赖它做判断的地方（occurs check 一类）在借用位整段失灵。端到端用例
// 区分不了这一臂（验证轮实测：去掉 Ref 臂后 2950 条全过），故判据落在函数本身。

#[test]
fn test_contains_type_vars_sees_through_borrow() {
    // Arrange — `&T`（借用包着一个类型变量），以及不含量变量的借用对照
    let tv = MonoType::TypeVar(TypeVar::new(7));
    let borrow_tv = MonoType::Ref {
        mutable: false,
        inner: Box::new(tv),
    };
    let borrow_int = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Int(64)),
    };

    // Act & Assert ①：`&T` 含类型变量——穿透借用位（删掉 Ref 臂即红）
    assert!(
        contains_type_vars(&borrow_tv),
        "contains_type_vars(&T) 必须为真：Ref 臂缺失时借用位恒判「不含变量」"
    );

    // Assert ②：负对照——`&Int` 不含类型变量（不是「有借用就恒真」）
    assert!(
        !contains_type_vars(&borrow_int),
        "contains_type_vars(&Int) 必须为假：判据只穿借用、不虚构变量"
    );
}

#[test]
fn test_substitution_bind_insert_get() {
    let mut sub = Substitution::new();
    sub.bind(TypeVar::new(0), MonoType::Int(32));
    sub.insert(1, MonoType::make_string());
    assert_eq!(sub.get(&0), Some(&MonoType::Int(32)));
    assert_eq!(sub.get(&1), Some(&MonoType::make_string()));
    assert_eq!(sub.get(&99), None);
}

#[test]
fn test_substitution_contains_var() {
    let mut sub = Substitution::new();
    sub.insert(5, MonoType::Bool);
    assert!(sub.contains_var(&5));
    assert!(!sub.contains_var(&0));
}

#[test]
fn test_substitution_bound_vars_len_is_empty() {
    let mut sub = Substitution::new();
    assert!(sub.is_empty());
    assert_eq!(sub.len(), 0);
    sub.insert(0, MonoType::Bool);
    sub.insert(2, MonoType::Int(32));
    assert!(!sub.is_empty());
    assert_eq!(sub.len(), 2);
    let vars = sub.bound_vars();
    assert!(vars.contains(&0));
    assert!(vars.contains(&2));
}

#[test]
fn test_substituter_substitute_var() {
    let subber = Substituter::new();
    let tv = TypeVar::new(0);
    let result = subber.substitute_var(&MonoType::TypeVar(tv), &tv, &MonoType::Int(32));
    assert_eq!(result, MonoType::Int(32));
    // Non-matching var stays as-is
    let result2 = subber.substitute_var(&MonoType::TypeVar(tv), &TypeVar::new(1), &MonoType::Bool);
    assert_eq!(result2, MonoType::TypeVar(tv));
}

#[test]
fn test_substitute_no_match_returns_original() {
    let subber = Substituter::new();
    let sub = Substitution::new();
    let ty = MonoType::Int(32);
    // No variables in the type → returns clone
    assert_eq!(subber.substitute(&ty, &sub), MonoType::Int(32));
}

#[test]
fn test_substitute_generic_params_too_few() {
    let subber = Substituter::new();
    let tv = TypeVar::new(5);
    let ty = MonoType::TypeVar(tv);
    // args is empty, index 5 > length → lookup returns None → original returned
    let result = subber.substitute_generic_params(&ty, &[]);
    assert_eq!(result, MonoType::TypeVar(tv));
}

#[test]
fn test_substitute_with_map_no_match() {
    let subber = Substituter::new();
    let map = HashMap::new();
    let ty = MonoType::Int(32);
    let result = subber.substitute_with_map(&ty, &map);
    assert_eq!(
        result,
        MonoType::Int(32),
        "no substitution should return original"
    );
}

#[test]
fn test_substitute_generic_params_matching() {
    let subber = Substituter::new();
    let ty = MonoType::make_tuple(vec![
        MonoType::TypeVar(TypeVar::new(0)),
        MonoType::TypeVar(TypeVar::new(1)),
    ]);
    let args = vec![MonoType::Int(32), MonoType::make_string()];
    let result = subber.substitute_generic_params(&ty, &args);
    assert_eq!(
        result,
        MonoType::make_tuple(vec![MonoType::Int(32), MonoType::make_string()])
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 名义引用改写（WBS 3.24.3 / #504 病史锚的承重判据，单元级）
// ─────────────────────────────────────────────────────────────────────────────
//
// `rebind_nominal_refs` 是导入面签名改绑的唯一实现：判据 = 改写 `TypeRef`
// 引用体与 `Struct` 接口实现名（名义名位，t3 F3）；`Generic` 头部 /
// `Struct` 记录名不动——它们是构造器与记录的身份，改写会改变诊断渲染与
// 记录查表键。端到端用例（tests/integration/multifile.rs 的 #504 锚组）
// 区分不了「改对了名字」与「把头部也改了」，故判据落在函数本身。

#[test]
fn test_rebind_nominal_refs_renames_fn_params_and_return() {
    // Arrange — 导入函数签名（参数位 + 返回位）携带定义单元裸名
    let mut rebinds = HashMap::new();
    rebinds.insert("Temp".to_string(), "TempB".to_string());
    let signature = MonoType::Fn {
        params: vec![MonoType::Ref {
            mutable: false,
            inner: Box::new(MonoType::TypeRef(
                Nominal::flat(AsRef::<str>::as_ref(&("Temp")))
                    .expect("type name must be bare and non-empty"),
            )),
        }],
        return_type: Box::new(MonoType::TypeRef(
            Nominal::flat(AsRef::<str>::as_ref(&("Temp")))
                .expect("type name must be bare and non-empty"),
        )),
    };

    // Act
    let rebound = rebind_nominal_refs(&signature, &rebinds);

    // Assert — 参数位的借用内层与返回位都改绑到本地名
    let MonoType::Fn {
        params,
        return_type,
    } = &rebound
    else {
        panic!("rebind must preserve Fn shape, got {rebound:?}");
    };
    assert!(
        matches!(return_type.as_ref(), MonoType::TypeRef(n) if n == "TempB"),
        "return-side nominal ref should be rebound to the local name"
    );
    let MonoType::Ref { inner, .. } = &params[0] else {
        panic!("rebind must preserve Ref shape, got {rebound:?}");
    };
    assert!(
        matches!(inner.as_ref(), MonoType::TypeRef(n) if n == "TempB"),
        "borrow-inner nominal ref should be rebound to the local name"
    );
}

#[test]
fn test_rebind_nominal_refs_keeps_generic_head_and_struct_name() {
    // Arrange — 头部/记录名是身份面（不随导入改名），实参位/字段位是引用面
    let mut rebinds = HashMap::new();
    rebinds.insert("Temp".to_string(), "TempB".to_string());
    let holder = MonoType::Struct(StructType {
        name: Nominal::flat(AsRef::<str>::as_ref(&("Holder")))
            .expect("type name must be bare and non-empty"),
        fields: vec![StructFieldInfo {
            name: "item".to_string(),
            ty: MonoType::Generic {
                name: Nominal::flat(AsRef::<str>::as_ref(&("List")))
                    .expect("type name must be bare and non-empty"),
                args: vec![MonoType::TypeRef(
                    Nominal::flat(AsRef::<str>::as_ref(&("Temp")))
                        .expect("type name must be bare and non-empty"),
                )],
            },
            quantity: None,
            is_mut: false,
            has_default: false,
        }],
        methods: HashMap::new(),
        interfaces: vec![],
    });

    // Act
    let rebound = rebind_nominal_refs(&holder, &rebinds);

    // Assert
    let MonoType::Struct(s) = &rebound else {
        panic!("rebind must preserve Struct shape, got {rebound:?}");
    };
    assert_eq!(
        s.name, "Holder",
        "record name is the record identity and must not be rebound"
    );
    let MonoType::Generic { name, args } = &s.fields[0].ty else {
        panic!("rebind must preserve Generic shape, got {rebound:?}");
    };
    assert_eq!(
        name, "List",
        "generic head is the constructor identity and must not be rebound"
    );
    assert!(
        matches!(args[0], MonoType::TypeRef(ref n) if n == "TempB"),
        "generic args are nominal-ref positions and must be rebound"
    );
}

#[test]
fn test_rebind_nominal_refs_empty_map_is_identity() {
    // Arrange — 无类型导入的 use（整体导入/native）不改绑
    let ty = MonoType::TypeRef(
        Nominal::flat(AsRef::<str>::as_ref(&("Temp")))
            .expect("type name must be bare and non-empty"),
    );

    // Act
    let rebound = rebind_nominal_refs(&ty, &HashMap::new());

    // Assert
    assert_eq!(
        rebound, ty,
        "empty rebind table must be a structural identity"
    );
}

#[test]
fn test_rebind_nominal_refs_unknown_names_pass_through() {
    // Arrange — 表外的裸名（未导入的类型/类型参数名）不改写
    let mut rebinds = HashMap::new();
    rebinds.insert("Temp".to_string(), "TempB".to_string());
    let ty = MonoType::TypeRef(
        Nominal::flat(AsRef::<str>::as_ref(&("A"))).expect("type name must be bare and non-empty"),
    );

    // Act
    let rebound = rebind_nominal_refs(&ty, &rebinds);

    // Assert
    assert!(
        matches!(&rebound, MonoType::TypeRef(n) if n == "A"),
        "names outside the rebind table must pass through unchanged"
    );
}

#[test]
fn test_rebind_nominal_refs_keeps_generic_head_in_rebind_table() {
    // 3.24.3 t3 F2 补牙：头部名在改绑表**内**的形态。此前守卫钉的 Generic
    // 头名 List 在表外，「按表中介改写头部」的变异对它不可见（M2x 实测表内
    // 名形态能红）。头部是构造器身份——即使名字命中改绑表也不得改写，
    // 改绑只触 TypeRef 引用体（实参位）。
    // Arrange — 头名 List 与引用体 Temp 同表
    let mut rebinds = HashMap::new();
    rebinds.insert("Temp".to_string(), "TempB".to_string());
    rebinds.insert("List".to_string(), "ListB".to_string());
    let holder = MonoType::Generic {
        name: Nominal::flat(AsRef::<str>::as_ref(&("List")))
            .expect("type name must be bare and non-empty"),
        args: vec![MonoType::TypeRef(
            Nominal::flat(AsRef::<str>::as_ref(&("Temp")))
                .expect("type name must be bare and non-empty"),
        )],
    };

    // Act
    let rebound = rebind_nominal_refs(&holder, &rebinds);

    // Assert
    let MonoType::Generic { name, args } = &rebound else {
        panic!("rebind must preserve Generic shape, got {rebound:?}");
    };
    assert_eq!(
        name, "List",
        "generic head must not be rebound even when the head name is in the rebind table"
    );
    assert!(
        matches!(args[0], MonoType::TypeRef(ref n) if n == "TempB"),
        "generic args are nominal-ref positions and must be rebound"
    );
}

#[test]
fn test_rebind_nominal_refs_leaves_solver_and_primitive_variants_untouched() {
    // 3.24.3 t3 F1 的语义面钉：无名义引用位的变体在非空改绑表下原样克隆。
    // 枚举名与 TypeVar 编号是身份面（判等键），即便名字/形态撞上表键也
    // 不得改写——改写会静默改掉判等身份。
    // Arrange — 表键 Temp；枚举名取同一名字制造最大诱导
    let mut rebinds = HashMap::new();
    rebinds.insert("Temp".to_string(), "TempB".to_string());
    let enum_ty = MonoType::Enum(crate::frontend::core::types::EnumType {
        name: Nominal::flat(AsRef::<str>::as_ref(&("Temp")))
            .expect("type name must be bare and non-empty"),
        variants: vec!["A".to_string(), "Temp".to_string()],
    });
    let tv = MonoType::TypeVar(TypeVar::new(3));
    let int_ty = MonoType::Int(64);

    // Act
    let rebound_enum = rebind_nominal_refs(&enum_ty, &rebinds);
    let rebound_tv = rebind_nominal_refs(&tv, &rebinds);
    let rebound_int = rebind_nominal_refs(&int_ty, &rebinds);

    // Assert
    assert_eq!(
        rebound_enum, enum_ty,
        "enum name/variant strings are identity faces and must not be rebound"
    );
    assert_eq!(
        rebound_tv, tv,
        "type vars are solver-managed identities and must not be touched"
    );
    assert_eq!(
        rebound_int, int_ty,
        "primitives carry no nominal refs and must clone through"
    );
}

#[test]
fn test_rebind_nominal_refs_rebinds_struct_interfaces() {
    // 3.24.3 t3 F3：接口实现名是名义名位（语义与 TypeRef 引用体同）——
    // 别名导入接口后，实现类型的接口名按本地名同源。不改绑则
    // Struct-vs-TypeRef 判等臂拿定义单元裸名与本地名判异 ⇒ E1002 误拒。
    // 记录名是身份面，不动。
    // Arrange — 实现类型 { v: Float, Temp }，接口 Temp 别名为 TempB
    let mut rebinds = HashMap::new();
    rebinds.insert("Temp".to_string(), "TempB".to_string());
    let widget = MonoType::Struct(StructType {
        name: Nominal::flat(AsRef::<str>::as_ref(&("Widget")))
            .expect("type name must be bare and non-empty"),
        fields: vec![StructFieldInfo {
            name: "v".to_string(),
            ty: MonoType::Float(64),
            quantity: None,
            is_mut: false,
            has_default: false,
        }],
        methods: HashMap::new(),
        interfaces: vec![Nominal::flat(AsRef::<str>::as_ref(&("Temp")))
            .expect("type name must be bare and non-empty")],
    });

    // Act
    let rebound = rebind_nominal_refs(&widget, &rebinds);

    // Assert
    let MonoType::Struct(s) = &rebound else {
        panic!("rebind must preserve Struct shape, got {rebound:?}");
    };
    assert_eq!(
        s.name, "Widget",
        "record name is the record identity and must not be rebound"
    );
    assert_eq!(
        s.interfaces,
        vec!["TempB".to_string()],
        "interface impl names are nominal-ref positions and must be rebound"
    );
}
