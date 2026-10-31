//! 单态化函数子模块测试 — 对应 src/middle/passes/mono/function.rs
//!
//! RFC-011 §3: 零成本抽象与单态化
//! RFC-011 §4: 泛型函数特化
//! RFC-039 6.3: `ast::Type` 由 26 变体收敛为 13 后，反向桥 B（`mono_to_ast_type`）
//!   改用 `MonoType::canonical_name` + 结构回写；口径见
//!   `docs/src/dev/architecture/03-type-unification.md` §6.3「规范名往返口径」
//!
//! 本文件覆盖 **leg B（MCC 全链往返）**：`MonoType` → `ast::Type`（反向桥 B）→
//! `MonoType`（`From<ast::Type>`）。leg A（15 个内置名逐名往返）由
//! `src/frontend/core/types/tests/mono.rs` 覆盖；leg B 此前只有一次性探针覆盖、
//! 未入库（`docs/src/dev/architecture/09a-wbs-history.md` §P6 6.3 残留风险②），
//! 本文件把这条等价性固化成门禁——反向桥 B 的产物再 lower 回来必须与原类型的关系
//! 逐组锁定：规范名与结构类型**全等**，其余变体**有损**。

use std::collections::HashMap;

use crate::frontend::core::parser::ast::Type as AstType;
use crate::frontend::core::typecheck::{ConstValue, EnumType, MonoType, StructType, TypeVar};
use crate::middle::passes::mono::function::FunctionMonomorphizer;
use crate::middle::passes::mono::Monomorphizer;
use crate::util::span::Span;

/// 替换点的占位类型名：`substitute_type_in_ast` 命中 `Type::Name{"T"}` 时，
/// 用 `name_map["T"]` 构造反向桥 B 的输入实参。
const PLACEHOLDER: &str = "T";

/// 15 个内置类型名（`from_builtin_name` 的接受面）→ 反向桥应回写的规范名。
///
/// `Int` / `Float` / `()` 是**入参别名**：三者与 `Int64` / `Float64` / `Void`
/// 共用同一 `MonoType`，回写恒取宽度显式形式，故 15 个接受名折叠成 12 个规范拼写
/// （否则产物不唯一，见 03 §6.3）。
const BUILTIN_NAME_IMAGE: &[(&str, &str)] = &[
    ("Never", "Never"),
    ("Void", "Void"),
    ("()", "Void"),
    ("Bool", "Bool"),
    ("Int", "Int64"),
    ("Int8", "Int8"),
    ("Int16", "Int16"),
    ("Int32", "Int32"),
    ("Int64", "Int64"),
    ("Float", "Float64"),
    ("Float32", "Float32"),
    ("Float64", "Float64"),
    ("Char", "Char"),
    ("String", "String"),
    ("Bytes", "Bytes"),
];

/// MCC 前半：`MonoType` → `ast::Type`（反向桥 B 的产物）。
///
/// `mono_to_ast_type` 是 `substitute_type_in_ast` 内的**嵌套私有 fn**，仓内无法
/// 从外部调用，也不为测试把它放成 `pub`（那会扩大 API 面）。唯一可达路径是喂一处
/// `Type::Name{"T"}` 并让 `name_map` 把 `"T"` 指向待测类型——该替换点即桥的唯一出口。
fn bridge_to_ast(mono: &MonoType) -> AstType {
    let mut name_map = HashMap::new();
    name_map.insert(PLACEHOLDER.to_string(), mono.clone());
    Monomorphizer::new().substitute_type_in_ast(
        &AstType::Name {
            name: PLACEHOLDER.to_string(),
            span: Span::default(),
        },
        &name_map,
    )
}

/// MCC 全链：`MonoType` → `ast::Type`（反向桥 B）→ `MonoType`（`From<ast::Type>`）。
fn mcc_round_trip(mono: &MonoType) -> MonoType {
    MonoType::from(bridge_to_ast(mono))
}

// ==================== ① 规范名往返 ====================

#[test]
fn test_bridge_writes_int64_as_canonical_name() {
    // Arrange: 宽度显式写法的 Int64
    let mono = MonoType::Int(64);

    // Act: 取出反向桥的中间产物，再走完 MCC 后半
    let image = bridge_to_ast(&mono);
    let re_lowered = MonoType::from(image.clone());

    // Assert: 中间态必须是规范名 Int64——落成诊断名 int64 会在下游被当成用户类型
    assert!(
        matches!(&image, AstType::Name { name, .. } if name == "Int64"),
        "Int(64) 的回写必须是规范名 Name(\"Int64\")，实际为 {image:?}"
    );
    assert_eq!(
        re_lowered, mono,
        "MCC 必须把 Int(64) 经 Name(Int64) 还原成同一 Int(64)"
    );
}

// ==================== ④ 标量 15 名 vs 容器 2 名 ====================

#[test]
fn test_all_builtin_names_round_trip_through_bridge() {
    // Arrange + Act + Assert: 15 个内置名逐名走全链（名 → MonoType → 桥 → MonoType）
    for &(written, expected_image) in BUILTIN_NAME_IMAGE {
        // Arrange
        let mono = MonoType::from_builtin_name(written)
            .unwrap_or_else(|| panic!("内置名 {written} 应被 from_builtin_name 接受"));

        // Act
        let image = bridge_to_ast(&mono);

        // Assert: 回写恒为规范名（Int / Float / () 折叠成宽度显式形式与 Void）
        assert!(
            matches!(&image, AstType::Name { name, .. } if name.as_str() == expected_image),
            "内置名 {written} 应回写成规范名 Name(\"{expected_image}\")，实际为 {image:?}"
        );
        assert_eq!(
            MonoType::from(image),
            mono,
            "内置名 {written} 的 MCC 必须还原同一 MonoType"
        );
    }
}

#[test]
fn test_string_and_bytes_are_container_shaped_names() {
    // Arrange: String / Bytes 在 MonoType 侧是**空参 Generic**（容器形态），不是标量变体
    for name in ["String", "Bytes"] {
        let mono = MonoType::from_builtin_name(name)
            .unwrap_or_else(|| panic!("内置名 {name} 应被 from_builtin_name 接受"));

        // Act
        let image = bridge_to_ast(&mono);

        // Assert: 两者是 15 名里唯一的容器形态，且由 canonical_name 的特例臂截住
        // ⇒ 产物是 Name 而非 Generic（结构回写路径根本到不了这里）
        assert!(
            matches!(&mono, MonoType::Generic { name: n, args } if n.as_str() == name && args.is_empty()),
            "{name} 应是空参 Generic（容器形态），实际为 {mono:?}"
        );
        assert_eq!(
            mono.canonical_name(),
            Some(name),
            "{name} 必须由 canonical_name 的特例臂认领（空参 generic 的规范名）"
        );
        assert!(
            matches!(&image, AstType::Name { name: n, .. } if n.as_str() == name),
            "{name} 应回写成 Name(\"{name}\") 而不是 Generic，实际为 {image:?}"
        );
        assert_eq!(
            MonoType::from(image),
            mono,
            "{name} 的 MCC 必须还原同一空参 Generic"
        );
    }
}

// ==================== ② 结构类型逐字全等 ====================

#[test]
fn test_generic_types_round_trip_verbatim_through_bridge() {
    // Arrange: 容器 / 元组 / 任意泛型实例（含嵌套与结构参数）
    let cases: Vec<(&str, MonoType)> = vec![
        ("Vec", MonoType::make_list(MonoType::Int(64))),
        (
            "Dict",
            MonoType::make_dict(MonoType::make_string(), MonoType::Int(32)),
        ),
        (
            "Option",
            MonoType::make_option(MonoType::make_result(
                MonoType::Int(64),
                MonoType::make_string(),
            )),
        ),
        (
            "Tuple",
            MonoType::make_tuple(vec![MonoType::Bool, MonoType::make_string()]),
        ),
        (
            "Box",
            MonoType::Generic {
                name: "Box".to_string(),
                args: vec![MonoType::Ref {
                    mutable: true,
                    inner: Box::new(MonoType::Char),
                }],
            },
        ),
    ];

    for (expected_name, mono) in cases {
        // Act
        let image = bridge_to_ast(&mono);

        // Assert: 产物是同名 Generic（元组也走 Generic，与 From<Tuple> 的结果逐字相同）
        assert!(
            matches!(&image, AstType::Generic { name, args, .. } if name.as_str() == expected_name && !args.is_empty()),
            "Generic({expected_name}) 应回写成同名且带参的 Type::Generic，实际为 {image:?}"
        );
        assert_eq!(
            MonoType::from(image),
            mono,
            "Generic({expected_name}) 的 MCC 必须逐字全等"
        );
    }
}

#[test]
fn test_empty_tuple_round_trip_does_not_collapse_to_void() {
    // Arrange: 空元组在 MonoType 侧是 Generic{Tuple, []}
    let mono = MonoType::make_tuple(vec![]);

    // Act
    let image = bridge_to_ast(&mono);

    // Assert: 桥必须产 Type::Generic{"Tuple"}——若产 Type::Tuple([])，再 lower 会塌成 Void
    assert!(
        matches!(&image, AstType::Generic { name, args, .. } if name == "Tuple" && args.is_empty()),
        "空元组应回写成空参 Type::Generic(\"Tuple\")，实际为 {image:?}"
    );
    assert_eq!(
        MonoType::from(image),
        MonoType::make_tuple(vec![]),
        "空元组的 MCC 必须还原成 Generic{{Tuple, []}}，不得塌成 Void"
    );
}

#[test]
fn test_ref_types_round_trip_verbatim_through_bridge() {
    // Arrange: 可变 / 不可变 / 嵌套引用 / 容器内层
    let cases: Vec<(bool, MonoType)> = vec![
        (
            false,
            MonoType::Ref {
                mutable: false,
                inner: Box::new(MonoType::Int(64)),
            },
        ),
        (
            true,
            MonoType::Ref {
                mutable: true,
                inner: Box::new(MonoType::make_string()),
            },
        ),
        (
            false,
            MonoType::Ref {
                mutable: false,
                inner: Box::new(MonoType::Ref {
                    mutable: true,
                    inner: Box::new(MonoType::make_list(MonoType::Char)),
                }),
            },
        ),
    ];

    for (expected_mutable, mono) in cases {
        // Act
        let image = bridge_to_ast(&mono);

        // Assert: 可变性逐字保留（借用的 span 不入 MonoType，故全链仍全等）
        assert!(
            matches!(&image, AstType::Ref { mutable, .. } if *mutable == expected_mutable),
            "Ref(mutable = {expected_mutable}) 应回写成同可变性的 Type::Ref，实际为 {image:?}"
        );
        assert_eq!(
            MonoType::from(image),
            mono,
            "Ref(mutable = {expected_mutable}) 的 MCC 必须逐字全等"
        );
    }
}

#[test]
fn test_fn_types_round_trip_verbatim_through_bridge() {
    // Arrange: 零参 / 混合参数 / 高阶（参数与返回都嵌函数类型）
    let cases: Vec<(usize, MonoType)> = vec![
        (
            0,
            MonoType::Fn {
                params: vec![],
                return_type: Box::new(MonoType::Void),
            },
        ),
        (
            2,
            MonoType::Fn {
                params: vec![MonoType::Int(64), MonoType::make_string()],
                return_type: Box::new(MonoType::Bool),
            },
        ),
        (
            1,
            MonoType::Fn {
                params: vec![MonoType::Fn {
                    params: vec![MonoType::Char],
                    return_type: Box::new(MonoType::Void),
                }],
                return_type: Box::new(MonoType::Ref {
                    mutable: false,
                    inner: Box::new(MonoType::make_option(MonoType::Int(16))),
                }),
            },
        ),
    ];

    for (expected_arity, mono) in cases {
        // Act
        let image = bridge_to_ast(&mono);

        // Assert: 形参个数与形参表逐字全等。
        //
        // **有损点（供后续改动核对）**：`From<ast::Type> for MonoType` 的 `Type::Fn`
        // 臂用 `filter_map(|p| p.ty)`——`ty = None` 的形参（parser 的 `(Int64)`
        // 参数位裸标识符形态）会被**静默丢弃**。本往返之所以全等，是因为反向桥 B 恒用
        // `Param::from_type` 合成**带类型**的无名形参（`ty = Some(..)`）；桥若改产
        // `ty = None`，MCC 会静默少形参且不报错。
        assert!(
            matches!(&image, AstType::Fn { params, .. } if params.len() == expected_arity),
            "Fn 应回写成 {expected_arity} 个形参的 Type::Fn，实际为 {image:?}"
        );
        assert_eq!(
            MonoType::from(image),
            mono,
            "Fn 的 MCC 必须逐字全等——形参个数不得被 filter_map 吃掉"
        );
    }
}

// ==================== ③ 有损兜底边界 ====================

#[test]
fn test_lossy_fallback_degrades_structural_types_to_type_ref() {
    // Arrange: 5 个没有结构回写形态的变体，一律落进 `other => Name(type_name())`
    let cases: Vec<(&str, MonoType)> = vec![
        (
            "Struct",
            MonoType::Struct(StructType {
                name: "Point".to_string(),
                fields: vec![("x".to_string(), MonoType::Int(64))],
                methods: HashMap::new(),
                field_mutability: vec![true],
                field_has_default: vec![false],
                interfaces: vec![],
            }),
        ),
        (
            "Enum",
            MonoType::Enum(EnumType {
                name: "Color".to_string(),
                variants: vec!["Red".to_string()],
            }),
        ),
        ("TypeVar", MonoType::TypeVar(TypeVar::new(0))),
        (
            "Literal",
            MonoType::Literal {
                name: "5".to_string(),
                base_type: Box::new(MonoType::Int(64)),
                value: ConstValue::Int(5),
            },
        ),
        (
            "Union",
            MonoType::Union(vec![MonoType::Int(64), MonoType::make_string()]),
        ),
    ];

    for (label, mono) in cases {
        // Act
        let image = bridge_to_ast(&mono);

        // Assert: 只保留 type_name() 文本 ⇒ 是**有损**降级，不是往返相等
        assert!(
            matches!(&image, AstType::Name { name, .. } if *name == mono.type_name()),
            "{label} 应走有损兜底回写成 Name(\"{}\")，实际为 {image:?}",
            mono.type_name()
        );
        let re_lowered = MonoType::from(image);
        assert_ne!(
            re_lowered, mono,
            "{label} 走有损兜底，MCC 不可能相等（结构与身份已丢）"
        );
        assert_eq!(
            re_lowered,
            MonoType::TypeRef(mono.type_name()),
            "{label} 再 lower 后只剩 TypeRef(type_name()) 这一层文本壳"
        );
    }
}

#[test]
fn test_unsupported_scalar_width_degrades_to_type_ref() {
    // Arrange: Int(128) 没有内置名（canonical_name 只覆盖 8 / 16 / 32 / 64）
    let mono = MonoType::Int(128);

    // Act
    let image = bridge_to_ast(&mono);

    // Assert: 无规范名 ⇒ 落进兜底臂，回写成诊断名 int128
    assert!(
        matches!(&image, AstType::Name { name, .. } if name == "int128"),
        "Int(128) 无规范名，应走兜底回写成 Name(\"int128\")，实际为 {image:?}"
    );
    assert_eq!(
        MonoType::from(image),
        MonoType::TypeRef("int128".to_string()),
        "无规范名的宽度再 lower 后是 TypeRef(int128)，不是 Int(128)"
    );
}

#[test]
fn test_type_ref_round_trip_is_verbatim_unless_name_collides_with_builtin() {
    // Arrange: 用户类型名 vs 与内置规范名撞名的 TypeRef
    let user = MonoType::TypeRef("Widget".to_string());
    let collided = MonoType::TypeRef("Int64".to_string());

    // Act
    let user_round_trip = mcc_round_trip(&user);
    let collided_round_trip = mcc_round_trip(&collided);

    // Assert: 兜底臂在 TypeRef 上无损（名字就是它的全部载荷）——这是 ③ 名单里没有
    // TypeRef 的原因；但撞内置名时身份被改写成内置标量。
    assert_eq!(
        user_round_trip, user,
        "用户类型名的 TypeRef 应逐字往返（兜底臂只保留名字，而名字就是全部信息）"
    );
    assert_eq!(
        collided_round_trip,
        MonoType::Int(64),
        "TypeRef 与内置规范名撞名时会被 from_builtin_name 认领成内置标量——故 TypeRef 不得用内置名"
    );
}
