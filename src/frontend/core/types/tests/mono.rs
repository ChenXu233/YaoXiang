//! 单态类型测试 — 基于语言规范 §3 & RFC-010 (续)
//!
//! §3.2-§3.17: From<ast::Type> 转换 — 覆盖所有 AST 类型到 MonoType 的转换路径
//! + 补充 expand_type 各类容器变体的测试

use crate::frontend::core::types::{MonoType, PolyType, StructType, TypeVar};
use crate::ast;
use crate::frontend::core::lexer::tokens::Literal;
use crate::frontend::core::types::EnumType;
use crate::util::span::Span;
use std::collections::HashMap;

// ─────────────────────────────────────────────────────────────────────────────
// 借用形 ↔ 值形的同形态判据（WBS 6.8.3 病灶 B 的**承重判据**，单元级）
// ─────────────────────────────────────────────────────────────────────────────
//
// 端到端判据区分不了这条判据：6.8.3 把体内绑定改成取声明标注原文后，出口 unify 直接
// 成功，剥借用那一步在两万级用例里都不再被走到（验证轮实测：删掉对齐后 2950 条全过）。
// 故判据落在**判定函数本身**：删掉「剥借用」或退回「无条件剥借用」，本组必红。

#[test]
fn test_borrow_value_forms_match_aligns_only_generic_positions() {
    // Arrange — 三格真值表的输入：泛型位借用/值形、具体类型借用/值形、跨类型。
    let tv = MonoType::TypeVar(TypeVar::new(3));
    let borrow_tv = MonoType::Ref {
        mutable: false,
        inner: Box::new(tv.clone()),
    };
    let borrow_int = MonoType::Ref {
        mutable: false,
        inner: Box::new(MonoType::Int(64)),
    };

    // Act & Assert ①：`&T` 与 `T`（同一类型变量）**同形态**——借用是编译期零大小
    // 令牌，两个方向都必须相容（`= (x) => x` 的返回值形态 ± 声明返回位形态）。
    assert!(
        MonoType::borrow_value_forms_match(&borrow_tv, &tv),
        "&T 与 T（同一类型变量）必须判相等（删掉剥借用这一步即红）"
    );
    assert!(
        MonoType::borrow_value_forms_match(&tv, &borrow_tv),
        "T 与 &T 必须判相等（对称方向；只认一个方向同样是漏判）"
    );

    // Assert ②：`&Int` 与 `String` **不同形态**——跨类型不得因借用位而放行
    assert!(
        !MonoType::borrow_value_forms_match(&borrow_int, &MonoType::make_string()),
        "&Int 与 String 必须判不等：形态对齐只剥一层借用、不换类型"
    );

    // Assert ③：`&Int` 与 `Int` **不同形态**——具体类型上的借用不是令牌的另一种写法。
    // 这一格是**防回归**、不是新语义：6.8.3 之前的树上 `f: (x: &Int) -> Int = (x) => x`
    // 本就报 E1012（`expected 'int64', found '&int64'`）；无条件剥借用是「声明来源」
    // 改造过程中的中间态，会把这条已拒绝的形态放行成静默错答。
    assert!(
        !MonoType::borrow_value_forms_match(&borrow_int, &MonoType::Int(64)),
        "&Int 与 Int 必须判不等（退回无条件剥借用即红：那会放行上一版已报 E1012 的形态）"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 容器注解的**长度槽 / 元素槽**分离（WBS 6.8.7-3 的承重判据，单元级）
// ─────────────────────────────────────────────────────────────────────────────
//
// 端到端只看得到「`v: Vec(3) = [1,2,3]` 是否通过」，区分不了「字面量实参按长度读」
// 与「元素期望退回 Literal{3}」这两条判据；故判据落在两个切槽函数本身——把元素槽
// 退回 `args[0]` 原样，第一组必红。

/// 单值类型 `int64::n`（`Vec(3)` 的 `3`、`Array(T, 3)` 的 `3` 的载体）。
fn int_literal_type(n: i128) -> MonoType {
    MonoType::Literal {
        name: n.to_string(),
        base_type: Box::new(MonoType::Int(64)),
        value: crate::frontend::core::types::const_data::ConstValue::Int(n),
    }
}

#[test]
fn test_container_element_slot_replaces_length_literal_with_base_type() {
    // Arrange — `Vec(3)`（字面量实参 = 长度槽）与 `Vec(Int)`（既有元素槽形态）。
    let vec_of_three = MonoType::Generic {
        name: "Vec".to_string(),
        args: vec![int_literal_type(3)],
    };
    let vec_of_int = MonoType::make_vec(MonoType::Int(64));

    // Act
    let elem_of_literal = vec_of_three.container_element_slot();
    let elem_of_int = vec_of_int.container_element_slot();

    // Assert — `Vec(3)` 的元素取字面量的**基类型**（长度只校验个数）；`Vec(Int)`
    // 的元素仍是 Int（既有形态零回归）。
    assert_eq!(
        elem_of_literal,
        Some(&MonoType::Int(64)),
        "Vec(3) 的元素槽必须是字面量的基类型 Int(64)：退回 Literal{{3}} 会把每个元素按 int64::3 拒绝"
    );
    assert_eq!(
        elem_of_int,
        Some(&MonoType::Int(64)),
        "Vec(Int) 的元素槽必须仍是 Int(64)（既有 Vec(Int) 形态零回归）"
    );
}

#[test]
fn test_container_length_slot_reads_only_literal_arguments() {
    // Arrange — 三格：`Vec(3)` 的字面量是长度；`Array(Int, 3)` 的第二位是长度；
    // `Array(Int, N)` 的 N 是符号常量（RFC-011 const 参数形态），长度推迟。
    let vec_of_three = MonoType::Generic {
        name: "Vec".to_string(),
        args: vec![int_literal_type(3)],
    };
    let array_of_three = MonoType::Generic {
        name: "Array".to_string(),
        args: vec![MonoType::Int(64), int_literal_type(3)],
    };
    let array_of_symbolic_n = MonoType::Generic {
        name: "Array".to_string(),
        args: vec![MonoType::Int(64), MonoType::TypeRef("N".to_string())],
    };
    let vec_of_int = MonoType::make_vec(MonoType::Int(64));

    // Act & Assert
    assert!(
        matches!(
            vec_of_three.container_length_slot(),
            Some(MonoType::Literal {
                value: crate::frontend::core::types::const_data::ConstValue::Int(3),
                ..
            })
        ),
        "Vec(3) 的长度槽必须是字面量 3（元素个数只由它校验）"
    );
    assert!(
        matches!(
            array_of_three.container_length_slot(),
            Some(MonoType::Literal {
                value: crate::frontend::core::types::const_data::ConstValue::Int(3),
                ..
            })
        ),
        "Array(Int, 3) 的长度槽必须是第二位（既有定长数组长度校验零回归）"
    );
    assert!(
        array_of_symbolic_n.container_length_slot().is_none(),
        "Array(Int, N) 的 N 是符号常量：长度槽必须返回 None（校验推迟，不得按 0 处理）"
    );
    assert!(
        vec_of_int.container_length_slot().is_none(),
        "Vec(Int) 没有长度槽：必须返回 None（既有形态不做长度校验，零回归）"
    );
}

// §3.2-§3.17: From<ast::Type> 转换

/// 内建类型在 AST 里的唯一写法：Name（WBS 6.3 删除了 Int/Float/… 专用变体）。
fn name_ty(name: &str) -> ast::Type {
    ast::Type::Name {
        name: name.to_string(),
        span: Span::dummy(),
    }
}

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
    assert_eq!(MonoType::from(name_ty("Int32")), MonoType::Int(32));
    assert_eq!(MonoType::from(name_ty("Int64")), MonoType::Int(64));
    assert_eq!(
        MonoType::from(name_ty("Int")),
        MonoType::Int(64),
        "Int 是 Int64 的别名"
    );
    assert_eq!(MonoType::from(name_ty("Float64")), MonoType::Float(64));
    assert_eq!(MonoType::from(name_ty("Char")), MonoType::Char);
    assert_eq!(MonoType::from(name_ty("String")), MonoType::make_string());
    assert_eq!(
        MonoType::from(name_ty("Bytes")),
        MonoType::Generic {
            name: "Bytes".into(),
            args: vec![]
        },
        "Bytes 落 Generic，不再有专用变体"
    );
    assert_eq!(MonoType::from(name_ty("Bool")), MonoType::Bool);
    assert_eq!(MonoType::from(name_ty("Void")), MonoType::Void);
    assert_eq!(
        MonoType::from(name_ty("()")),
        MonoType::Void,
        "空括号是 Void 的类型写法"
    );
}

#[test]
fn test_from_ast_type_struct() {
    let ast_ty = ast::Type::Struct {
        body: vec![
            ast::TypeBodyItem::Field(ast::StructField::new(
                "x".to_string(),
                false,
                name_ty("Float64"),
            )),
            ast::TypeBodyItem::Field(ast::StructField::new(
                "y".to_string(),
                false,
                name_ty("Float64"),
            )),
        ],
    };
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Struct(_)));
    if let MonoType::Struct(s) = mono {
        assert_eq!(s.fields.len(), 2);
    }
}

#[test]
fn test_from_ast_type_struct_with_defaults() {
    let ast_ty = ast::Type::Struct {
        body: vec![ast::TypeBodyItem::Field(ast::StructField {
            name: "x".to_string(),
            is_mut: false,
            ty: name_ty("Float64"),
            default: Some(Box::new(ast::Expr::Lit(Literal::Float(0.0), Span::dummy()))),
        })],
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
            name_ty("Float64"),
        )],
    };
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Struct(s) if s.name == "Point"));
}

#[test]
fn test_from_ast_type_name_void_spellings() {
    // Void 的两个写法（Void 与空括号）都落 MonoType::Void；6.3 删掉专用变体后
    // 这条映射由 Name 路径承担（也是 types.rs 非 Fn 签名兜底的语义前提）。
    assert_eq!(MonoType::from(name_ty("Void")), MonoType::Void);
    assert_eq!(MonoType::from(name_ty("()")), MonoType::Void);
}

#[test]
fn test_from_ast_type_tuple() {
    let ast_ty = ast::Type::Tuple(vec![name_ty("Int32"), name_ty("String")]);
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Generic { name, args } if name == "Tuple" && args.len() == 2));
}

#[test]
fn test_from_ast_type_fn() {
    let ast_ty = ast::Type::Fn {
        params: vec![ast::Param::from_type(name_ty("Int32"))],
        return_type: Box::new(name_ty("Bool")),
    };
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::Fn { .. }));
}

#[test]
fn test_from_ast_type_fn_drops_untyped_param_as_loss() {
    // Arrange: 一个带类型形参 + 一个**裸标识符形参**（`ty = None`——6.5 / 8.9.1 的
    // `(Int64)` 形参位：标识符占位、无类型标注）。后者在 AST 里是合法且常见的形态。
    let ast_ty = ast::Type::Fn {
        params: vec![
            ast::Param::from_type(name_ty("Int32")),
            ast::Param::anonymous(None, Span::dummy()),
        ],
        return_type: Box::new(name_ty("Bool")),
    };
    let ast_param_count = match &ast_ty {
        ast::Type::Fn { params, .. } => params.len(),
        _ => unreachable!("夹具就是 Type::Fn"),
    };

    // Act
    let mono: MonoType = ast_ty.into();

    // Assert: **有损**——`From<ast::Type> for MonoType` 的 Fn 臂用
    // `.filter_map(|p| p.ty)`，`ty = None` 的形参被直接丢掉，故 2 个只剩 1 个。
    //
    // 为什么不在本轮改行为（WBS 6.6.15）：给无名形参合成占位类型会改变
    // `MonoType::Fn` 的形参数，进而改变函数类型相等性（E1002 的误报面）、重载解析
    // 与 IR 快照 / 语料基线——属 D 类可观察变更，须先经裁决。本测试只把这条损失
    // **钉成可失败的断言**：实现若改成「保留全部形参」，下面的差值为 0、断言红。
    match mono {
        MonoType::Fn { params, .. } => {
            assert_eq!(
                ast_param_count - params.len(),
                1,
                "Fn 降级必须丢掉 ty=None 的形参（有损事实：{ast_param_count} → 1）；保留全部形参时本断言红——那时须同时裁决形参数与相等性口径"
            );
            assert_eq!(
                params,
                vec![MonoType::Int(32)],
                "留下来的必须是带类型标注的那个形参，且相对顺序不变"
            );
        }
        other => panic!("Fn 注解必须降级成 MonoType::Fn，实际 {other:?}"),
    }
}

#[test]
fn test_from_ast_type_empty_tuple_is_unit() {
    // 零元元组 = 单位类型 = Void；parser 对空括号的产出就是 Type::Tuple(vec![])。
    assert_eq!(MonoType::from(ast::Type::Tuple(vec![])), MonoType::Void);
}

#[test]
fn test_from_ast_type_result() {
    // Result 的真实 AST 形态是 Generic（parser 不 lower 成专用节点）。
    let ast_ty = ast::Type::Generic {
        name: "Result".to_string(),
        name_span: Span::dummy(),
        args: vec![name_ty("Int32"), name_ty("String")],
    };
    let mono: MonoType = ast_ty.into();
    assert!(
        matches!(
            &mono,
            MonoType::Generic { name, args }
                if name == "Result" && args == &vec![MonoType::Int(32), MonoType::make_string()]
        ),
        "Result 应逐字保留名字与实参，实际 {mono:?}"
    );
}

#[test]
fn test_from_ast_type_generic() {
    let ast_ty = ast::Type::Generic {
        name: "Option".to_string(),
        name_span: Span::dummy(),
        args: vec![name_ty("Int32")],
    };
    let mono: MonoType = ast_ty.into();
    // #299：泛型不再 lower 成原生变体，保持 Generic
    assert!(matches!(mono, MonoType::Generic { name, .. } if name == "Option"));
}

#[test]
fn test_from_ast_type_generic_custom() {
    let ast_ty = ast::Type::Generic {
        name: "List".to_string(),
        name_span: Span::dummy(),
        args: vec![name_ty("Int32")],
    };
    let mono: MonoType = ast_ty.into();
    // #299：泛型不再 lower 成原生变体，保持 Generic
    assert!(
        matches!(mono, MonoType::Generic { name, args } if name == "List" && args[0] == MonoType::Int(32))
    );
}

#[test]
fn test_from_ast_type_generic_tuple() {
    // Tuple 的 AST 常态是 Generic（回写桥也用它），与 Tuple 变体经 make_tuple
    // 的结果逐字相同。
    let ast_ty = ast::Type::Generic {
        name: "Tuple".to_string(),
        name_span: Span::dummy(),
        args: vec![name_ty("Int32"), name_ty("Bool")],
    };
    let mono: MonoType = ast_ty.into();
    assert!(
        matches!(
            &mono,
            MonoType::Generic { name, args } if name == "Tuple" && args.len() == 2
        ),
        "Generic(Tuple) 应原样保留，实际 {mono:?}"
    );
}

#[test]
fn test_from_ast_type_meta_type() {
    let ast_ty = ast::Type::MetaType {
        name_span: Span::dummy(),
        args: vec![name_ty("Int32")],
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

// §3.4: EnumType

#[test]
fn test_from_ast_type_name_never() {
    // Never 同属 15 个规范名之一，走 Name 路径。
    assert_eq!(MonoType::from(name_ty("Never")), MonoType::Never);
}

#[test]
fn test_from_ast_type_ptr() {
    let ast_ty = ast::Type::Ptr(Box::new(name_ty("Int32")));
    let mono: MonoType = ast_ty.into();
    assert!(matches!(mono, MonoType::TypeRef(n) if n.starts_with("*")));
}

#[test]
fn test_from_ast_type_literal() {
    let ast_ty = ast::Type::Literal {
        name: "5".to_string(),
        name_span: Span::dummy(),
        base_type: Box::new(name_ty("Int64")),
    };
    let mono: MonoType = ast_ty.into();
    // Literal type preserves value as MonoType::Literal
    assert_eq!(
        mono,
        MonoType::Literal {
            name: "5".to_string(),
            base_type: Box::new(MonoType::Int(64)),
            value: crate::frontend::core::types::const_data::ConstValue::Int(5),
        }
    );
}

// §3.3: StructType PartialEq / Eq 完整测试

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

// §3.17: get_ast_type_universe_level 覆盖嵌套场景

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

// §3.13-3.14: 类型联合/交集 type_name 测试

#[test]
fn test_type_name_dict_set_range() {
    let d = MonoType::make_dict(MonoType::make_string(), MonoType::Int(32));
    assert_eq!(d.type_name(), "Dict(string, int32)");
    let s = MonoType::Generic {
        name: "Set".into(),
        args: vec![MonoType::Bool],
    };
    assert_eq!(s.type_name(), "Set(bool)");
    let r = MonoType::Generic {
        name: "Range".into(),
        args: vec![MonoType::Int(64)],
    };
    assert_eq!(r.type_name(), "Range(int64)");
}

#[test]
fn test_type_name_arc_weak_assoc() {
    let a = MonoType::Generic {
        name: "Arc".into(),
        args: vec![MonoType::Int(32)],
    };
    assert_eq!(a.type_name(), "Arc(int32)");
    let w = MonoType::Generic {
        name: "Weak".into(),
        args: vec![MonoType::make_string()],
    };
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
    let u = MonoType::Union(vec![
        MonoType::Int(32),
        MonoType::make_string(),
        MonoType::Bool,
    ]);
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
fn test_universe_level_le() {
    use crate::frontend::core::types::UniverseLevel;
    // 同层级
    assert!(UniverseLevel::type0().le(&UniverseLevel::type0()));
    assert!(UniverseLevel::type1().le(&UniverseLevel::type1()));
    // 低 <= 高
    assert!(UniverseLevel::type0().le(&UniverseLevel::type1()));
    assert!(UniverseLevel::new("2").le(&UniverseLevel::new("5")));
    // 高 <= 低：不成立
    assert!(!UniverseLevel::type1().le(&UniverseLevel::type0()));
    assert!(!UniverseLevel::new("9").le(&UniverseLevel::new("3")));
    // 大层级
    assert!(UniverseLevel::new("999").le(&UniverseLevel::new("1000")));
}

// §3.18: canonical_name ↔ from_builtin_name 逐名互逆（WBS 6.3）

/// 已定稿的 15 个内置类型名（from_builtin_name 的接受面）。
const CANONICAL_NAMES: &[&str] = &[
    "Never", "Void", "()", "Bool", "Int", "Int8", "Int16", "Int32", "Int64", "Float", "Float32",
    "Float64", "Char", "String", "Bytes",
];

#[test]
fn test_canonical_name_round_trips_all_builtin_names() {
    // Arrange + Act + Assert：逐名往返（名 -> MonoType -> 规范名 -> MonoType）
    for name in CANONICAL_NAMES {
        // Arrange
        let mono = MonoType::from_builtin_name(name)
            .unwrap_or_else(|| panic!("内置名 {name} 应被 from_builtin_name 接受"));
        // Act
        let canonical = mono
            .canonical_name()
            .unwrap_or_else(|| panic!("{name} 的 MonoType 应有规范名：{mono:?}"));
        // Assert
        let round_tripped = MonoType::from_builtin_name(canonical)
            .unwrap_or_else(|| panic!("规范名 {canonical} 应被 from_builtin_name 接受"));
        assert_eq!(
            round_tripped, mono,
            "规范名 {canonical} 必须还原同一 MonoType"
        );
    }
}

#[test]
fn test_canonical_name_uses_width_explicit_spellings() {
    // 别名（Int/Float/空括号）只入不出：回写恒取唯一写法。
    assert_eq!(MonoType::Int(64).canonical_name(), Some("Int64"));
    assert_eq!(MonoType::Float(64).canonical_name(), Some("Float64"));
    assert_eq!(MonoType::Void.canonical_name(), Some("Void"));
    // DateTime 是 std 层单向别名：只允许 from_builtin_name 接受，永不写出。
    assert_eq!(
        MonoType::from_builtin_name("DateTime"),
        Some(MonoType::Int(64)),
        "DateTime 仍是接受面内的临时别名"
    );
    assert_ne!(
        MonoType::Int(64).canonical_name(),
        Some("DateTime"),
        "DateTime 不得作为回写名（只入不出）"
    );
}

#[test]
fn test_canonical_name_rejects_structural_types() {
    // 结构类型没有规范名：回写必须走结构（Generic/Tuple/Ref/Fn）或既有有损兜底。
    assert_eq!(
        MonoType::Generic {
            name: "List".into(),
            args: vec![MonoType::Int(64)]
        }
        .canonical_name(),
        None
    );
    assert_eq!(
        MonoType::make_tuple(vec![MonoType::Int(64)]).canonical_name(),
        None
    );
    assert_eq!(MonoType::TypeRef("T".into()).canonical_name(), None);
    assert_eq!(
        MonoType::Ref {
            mutable: false,
            inner: Box::new(MonoType::Int(64))
        }
        .canonical_name(),
        None
    );
    assert_eq!(
        MonoType::Int(128).canonical_name(),
        None,
        "无 Int128 内置名（宽度超出 8/16/32/64 即无规范名）"
    );
}

#[test]
fn test_universe_level_max() {
    use crate::frontend::core::types::UniverseLevel;
    // 同层级
    assert_eq!(
        UniverseLevel::max(&UniverseLevel::type0(), &UniverseLevel::type0()),
        &UniverseLevel::type0()
    );
    // 不同层级
    assert_eq!(
        UniverseLevel::max(&UniverseLevel::type0(), &UniverseLevel::type1()),
        &UniverseLevel::type1()
    );
    assert_eq!(
        UniverseLevel::max(&UniverseLevel::type1(), &UniverseLevel::type0()),
        &UniverseLevel::type1()
    );
    // 大层级
    assert_eq!(
        UniverseLevel::max(&UniverseLevel::new("5"), &UniverseLevel::new("500")),
        &UniverseLevel::new("500")
    );
}

#[test]
fn test_poly_type_display_and_name() {
    let poly = PolyType::mono(MonoType::make_string());
    assert_eq!(poly.type_name(), "string");
    assert_eq!(format!("{}", poly), "string");
    assert!(PolyType::mono(MonoType::Int(32)).is_mono());
    assert!(!PolyType::new(vec![TypeVar::new(0)], MonoType::Int(32)).is_mono());
}

#[test]
fn test_type_name_result() {
    let r = MonoType::make_result(MonoType::Int(32), MonoType::make_string());
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

// §3: MonoType 方法补充测试

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
        MonoType::make_string().type_var(),
        None,
        "String should return None"
    );
    assert_eq!(MonoType::Bool.type_var(), None, "Bool should return None");
}

#[test]
fn test_mono_type_list_variant() {
    let list = MonoType::make_list(MonoType::Int(32));
    // D8-B 名字分层：内置层是 `Vec(T)`（`List` 是标准库类型，见 std/list.yx）。
    assert!(
        list.type_name().contains("Vec"),
        "builtin buffer type_name should contain 'Vec'"
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
        MonoType::make_list(MonoType::Int(32)).is_indexable(),
        "List should be indexable"
    );
    assert!(
        MonoType::make_string().is_indexable(),
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
    let opt = MonoType::make_option(MonoType::Int(32));
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
    let res = MonoType::make_result(MonoType::Int(32), MonoType::make_string());
    assert!(
        res.type_name().contains("Result"),
        "Result type_name should contain 'Result'"
    );
}

// ─── WBS 6.1.2 / SPEC §2.1：内置类型名表（15 名 + 临时兼容别名 DateTime）───
//
// 下面三个用例共同钉住 MonoType::from_builtin_name 的接受面：少一个规范名、
// 或多回任何一个已删除的拼写，都会变红。

#[test]
fn test_from_builtin_name_accepts_the_fifteen_canonical_names() {
    // Arrange：SPEC §2.1 的 15 个内置类型名（"()" 是 Void 的类型写法）
    let expectations: Vec<(&str, MonoType)> = vec![
        ("Never", MonoType::Never),
        ("Void", MonoType::Void),
        ("()", MonoType::Void),
        ("Bool", MonoType::Bool),
        ("Int", MonoType::Int(64)),
        ("Int8", MonoType::Int(8)),
        ("Int16", MonoType::Int(16)),
        ("Int32", MonoType::Int(32)),
        ("Int64", MonoType::Int(64)),
        ("Float", MonoType::Float(64)),
        ("Float32", MonoType::Float(32)),
        ("Float64", MonoType::Float(64)),
        ("Char", MonoType::Char),
        ("String", MonoType::make_string()),
        (
            "Bytes",
            MonoType::Generic {
                name: "Bytes".into(),
                args: vec![],
            },
        ),
    ];

    // Act + Assert：逐个名字解析到期望的 MonoType
    for (name, expected) in &expectations {
        assert_eq!(
            MonoType::from_builtin_name(name),
            Some(expected.clone()),
            "内置名 {name} 应解析为 {expected:?}"
        );
    }
}

#[test]
fn test_from_builtin_name_rejects_the_twenty_two_removed_spellings() {
    // Arrange：2026-10-27 定稿删除的 22 个实现内拼写
    let removed = [
        "int", "int64", "i64", "datetime", "int32", "i32", "int16", "i16", "int8", "i8", "float",
        "float64", "f64", "float32", "f32", "bool", "char", "string", "str", "bytes", "void",
        "never",
    ];

    // Act + Assert：任何一个被加回解析表，本用例立即变红
    for name in removed {
        assert_eq!(
            MonoType::from_builtin_name(name),
            None,
            "拼写 {name} 已随 WBS 6.1.2 删除，不得再被当作内置类型名"
        );
    }
}

#[test]
fn test_from_builtin_name_keeps_datetime_as_compat_alias() {
    // Arrange：DateTime 是 std 层语义别名（src/std/time.rs 的签名串仍在用），
    // 迁出内置名表属 P10 S11（issue #513）
    // Act
    let upper = MonoType::from_builtin_name("DateTime");
    let lower = MonoType::from_builtin_name("datetime");

    // Assert：只保留大写形态，且归一为时间戳的运行时表示 Int(64)
    assert_eq!(
        upper,
        Some(MonoType::Int(64)),
        "DateTime 应归一为 Int(64)（#338 连带修复）"
    );
    assert_eq!(lower, None, "小写 datetime 已删除，不是兼容别名的形态");
}
