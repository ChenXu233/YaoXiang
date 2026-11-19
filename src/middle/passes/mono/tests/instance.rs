//! 实例化数据类型测试 — 对应 src/middle/passes/mono/instance.rs
//!
//! RFC-011 §3: 零成本抽象与单态化
//! RFC-011 §4: 实例化请求与缓存键定义

use crate::frontend::core::sema::MonoType;
use crate::middle::passes::mono::instance::{
    GenericFunctionId, InstantiationRequest, SpecializationKey,
};
use crate::util::span::Span;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

#[test]
fn test_specialization_key_deduplication() {
    // Arrange
    let key_a = SpecializationKey::new("identity".to_string(), vec![MonoType::Int(64)]);
    let key_b = SpecializationKey::new("identity".to_string(), vec![MonoType::Int(64)]);
    let key_c = SpecializationKey::new("identity".to_string(), vec![MonoType::make_string()]);

    // Assert: 相同类型参数的 key 应相等
    assert_eq!(key_a, key_b, "相同类型参数的 SpecializationKey 应相等");

    // Assert: 不同类型参数的 key 应不等
    assert_ne!(key_a, key_c, "不同类型参数的 SpecializationKey 应不等");
}

#[test]
fn test_generic_function_id_name_accessor() {
    // Arrange
    let id = GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]);

    // Assert
    assert_eq!(id.name(), "identity");
    assert_eq!(id.type_params(), &["T".to_string()]);
}

#[test]
fn test_instantiation_request_specialization_key_generation() {
    // Arrange
    let req = InstantiationRequest::new(
        GenericFunctionId::new("identity".to_string(), vec!["T".to_string()]),
        vec![crate::frontend::core::types::mono::GenericArg::Type(
            MonoType::Int(64),
        )],
        Span::default(),
    );

    // Act
    let key = req.specialization_key();

    // Assert
    assert_eq!(key.name, "identity");
    assert_eq!(key.type_args, vec![MonoType::Int(64)]);
}

/// 6.9.2-C1 ①：`GenericArg` 的四个投影（`type_name` / `as_type` / `as_term` / `substitute`）
/// 都在同一 impl 块的穷尽 match 里——本条钉住两条变体的语义差：
/// 类型位有类型无项；编译期项位闭项有项、类型位由 **`as_type` 单向派生**。
#[test]
fn test_generic_arg_projections_distinguish_type_and_const() {
    // Arrange
    use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
    use crate::frontend::core::types::mono::GenericArg;
    let type_arg = GenericArg::Type(MonoType::Int(64));
    let const_arg = GenericArg::Const(ConstExpr::Lit(ConstValue::Int(42)));

    // Act + Assert — 类型位：有类型、无项
    assert_eq!(
        type_arg.as_type(),
        Some(MonoType::Int(64)),
        "类型位实参投影回自身"
    );
    assert!(type_arg.as_term().is_none(), "类型位实参不是编译期项");
    assert_eq!(
        type_arg.type_name(),
        "int64",
        "类型位名走 MonoType::type_name"
    );

    // Assert — 项位：有项，类型位由归一化后的值单向派生（Literal）
    assert!(const_arg.as_term().is_some(), "项位实参必须带项");
    match const_arg.as_type() {
        Some(MonoType::Literal { name, value, .. }) => {
            assert_eq!(
                name, "42",
                "Literal 名口径 = 值的 Display（与既有特化名一致）"
            );
            assert_eq!(value, ConstValue::Int(42), "Literal 值 = 项归一化结果");
        }
        other => panic!("闭项应投影出 Literal，实际: {other:?}"),
    }
    assert_eq!(
        const_arg.type_name(),
        const_arg.as_type().expect("闭项必有类型位").type_name(),
        "项位的名与类型位投影同一口径"
    );
}

// ==================== issue #474 / WBS S9.2：type_name 与 type_name_hash 输出 pin ====================
//
// 病史门（coding-rules 第七部分 7.2）：`MonoType::type_name`（frontend/core/types/mono.rs）
// 与 `type_name_hash`（本目录 instance.rs）各自 match 全部 MonoType 变体——同一个
// 「类型子节点枚举」的两份手写副本；issue #474 指出副本间的臂内子节点面可能漂移且
// 不报错。单源化重构（`MonoType::type_children`）的硬判据是**输出逐字节不变**
// （等价性判据 C1 zero-diff）——本组把代表性类型集（全变体 + Generic guard 分支 +
// 稀见变体 + 嵌套复合）的渲染串与哈希值冻结下来：任一面输出漂移（即使语义等价）
// 立即变红。换一个会改变渲染文本或哈希写入序的正确替代实现它同样红——这正是
// 判据要求它红的地方（输出冻结是本批的特征化判据，不是性质测试）。
//
// 期望值抓取自重构前实现（dev@a60a93cb 工作树，2026-11-20，抓取用例与本文件同表）。

/// 面向 pin 的确定性哈希器：全部 `write_*` 归一到同一条字节流（FNV-1a 累加）。
///
/// 不用 `std::collections::hash_map::DefaultHasher`——其跨 rustc 版本不保证稳定，
/// 会让 pin 在无关的工具链升级上假红。pin 值的稳定性由本类型自己定义：
/// 改动本类型即改动全部期望值口径，须重新抓取 pin 表（抓取方式 = 本组注释口径）。
#[derive(Default)]
struct PinHasher(u64);

impl PinHasher {
    fn absorb(
        &mut self,
        bytes: &[u8],
    ) {
        for b in bytes {
            self.0 = (self.0 ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3);
        }
    }
}

impl Hasher for PinHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(
        &mut self,
        bytes: &[u8],
    ) {
        self.absorb(bytes);
    }
    fn write_u8(
        &mut self,
        n: u8,
    ) {
        self.absorb(&[n]);
    }
    fn write_u16(
        &mut self,
        n: u16,
    ) {
        self.absorb(&n.to_le_bytes());
    }
    fn write_u32(
        &mut self,
        n: u32,
    ) {
        self.absorb(&n.to_le_bytes());
    }
    fn write_u64(
        &mut self,
        n: u64,
    ) {
        self.absorb(&n.to_le_bytes());
    }
    fn write_u128(
        &mut self,
        n: u128,
    ) {
        self.absorb(&n.to_le_bytes());
    }
    fn write_usize(
        &mut self,
        n: usize,
    ) {
        self.absorb(&(n as u64).to_le_bytes());
    }
    fn write_i8(
        &mut self,
        n: i8,
    ) {
        self.absorb(&(n as u8).to_le_bytes());
    }
    fn write_i16(
        &mut self,
        n: i16,
    ) {
        self.absorb(&n.to_le_bytes());
    }
    fn write_i32(
        &mut self,
        n: i32,
    ) {
        self.absorb(&n.to_le_bytes());
    }
    fn write_i64(
        &mut self,
        n: i64,
    ) {
        self.absorb(&n.to_le_bytes());
    }
    fn write_i128(
        &mut self,
        n: i128,
    ) {
        self.absorb(&n.to_le_bytes());
    }
    fn write_isize(
        &mut self,
        n: isize,
    ) {
        self.absorb(&(n as i64).to_le_bytes());
    }
    // 不 override `write_str`（unstable `hasher_prefixfree_extras`）：
    // 默认实现 = write(bytes) + write_u8(0xff)，上面两个方法已覆盖，归一结果一致。
}

/// pin 表：标签 → 类型值。标签按变体与场景命名，断言消息据此定位漂移点。
///
/// 覆盖面：全部 22 个 `MonoType` 变体；`Generic` 的每个 guard 分支（含空实参、
/// 多余实参——渲染只消费前缀而哈希消费全集的分歧点）；`Struct` / `Enum` 的
/// 复合载荷（其字段类型**不是**类型子节点，误枚举即红）；嵌套复合。
fn pinned_cases() -> Vec<(&'static str, MonoType)> {
    use crate::frontend::core::types::const_data::{ConstExpr, ConstValue};
    use crate::frontend::core::types::mono::{DepParam, StructFieldInfo};
    use crate::frontend::core::types::{EnumType, StructType, TypeVar, UniverseLevel};

    vec![
        ("never", MonoType::Never),
        ("void", MonoType::Void),
        ("bool", MonoType::Bool),
        ("int64", MonoType::Int(64)),
        ("int32", MonoType::Int(32)),
        ("float64", MonoType::Float(64)),
        ("float32", MonoType::Float(32)),
        ("char", MonoType::Char),
        ("var0", MonoType::TypeVar(TypeVar::new(0))),
        ("typeref", MonoType::TypeRef("Point".into())),
        (
            "struct",
            MonoType::Struct(StructType {
                name: "Point".into(),
                fields: vec![StructFieldInfo {
                    name: "x".into(),
                    ty: MonoType::Int(64),
                    quantity: None,
                    is_mut: false,
                    has_default: false,
                }],
                methods: HashMap::new(),
                interfaces: vec![],
            }),
        ),
        (
            "enum",
            MonoType::Enum(EnumType {
                name: "Color".into(),
                variants: vec!["Red".into(), "Green".into()],
            }),
        ),
        (
            "fn",
            MonoType::Fn {
                params: vec![MonoType::Int(64), MonoType::Bool],
                return_type: Box::new(MonoType::Float(64)),
            },
        ),
        ("generic-string-empty", MonoType::make_string()),
        (
            "generic-string-extra-arg",
            MonoType::Generic {
                name: "String".into(),
                args: vec![MonoType::Int(64)],
            },
        ),
        (
            "generic-bytes",
            MonoType::Generic {
                name: "Bytes".into(),
                args: vec![],
            },
        ),
        (
            "generic-tuple",
            MonoType::Generic {
                name: "Tuple".into(),
                args: vec![MonoType::Int(64), MonoType::Bool],
            },
        ),
        (
            "generic-tuple-empty",
            MonoType::Generic {
                name: "Tuple".into(),
                args: vec![],
            },
        ),
        (
            "generic-option",
            MonoType::Generic {
                name: "Option".into(),
                args: vec![MonoType::Int(64)],
            },
        ),
        (
            "generic-option-extra-arg",
            MonoType::Generic {
                name: "Option".into(),
                args: vec![MonoType::Int(64), MonoType::make_string()],
            },
        ),
        (
            "generic-result",
            MonoType::Generic {
                name: "Result".into(),
                args: vec![MonoType::Int(64), MonoType::Bool],
            },
        ),
        (
            "generic-result-arity3",
            MonoType::Generic {
                name: "Result".into(),
                args: vec![MonoType::Int(64), MonoType::Bool, MonoType::Char],
            },
        ),
        (
            "generic-range",
            MonoType::Generic {
                name: "Range".into(),
                args: vec![MonoType::Int(64)],
            },
        ),
        (
            "generic-arc",
            MonoType::Generic {
                name: "Arc".into(),
                args: vec![MonoType::Int(64)],
            },
        ),
        (
            "generic-weak",
            MonoType::Generic {
                name: "Weak".into(),
                args: vec![MonoType::Int(64)],
            },
        ),
        (
            "generic-custom",
            MonoType::Generic {
                name: "MyBox".into(),
                args: vec![MonoType::Int(64)],
            },
        ),
        (
            "generic-custom-empty",
            MonoType::Generic {
                name: "MyBox".into(),
                args: vec![],
            },
        ),
        (
            "generic-vec",
            MonoType::Generic {
                name: "Vec".into(),
                args: vec![MonoType::Int(64)],
            },
        ),
        (
            "union",
            MonoType::Union(vec![MonoType::Int(64), MonoType::Bool, MonoType::Char]),
        ),
        (
            "intersection",
            MonoType::Intersection(vec![MonoType::Int(64), MonoType::Float(64)]),
        ),
        (
            "ref-immutable",
            MonoType::Ref {
                mutable: false,
                inner: Box::new(MonoType::Int(64)),
            },
        ),
        (
            "ref-mutable",
            MonoType::Ref {
                mutable: true,
                inner: Box::new(MonoType::Int(64)),
            },
        ),
        (
            "assoc",
            MonoType::AssocType {
                host_type: Box::new(MonoType::TypeRef("T".into())),
                assoc_name: "Item".into(),
                assoc_args: vec![MonoType::Int(64)],
            },
        ),
        (
            "assoc-noargs",
            MonoType::AssocType {
                host_type: Box::new(MonoType::TypeRef("T".into())),
                assoc_name: "Item".into(),
                assoc_args: vec![],
            },
        ),
        (
            "literal-int",
            MonoType::Literal {
                name: "5".into(),
                base_type: Box::new(MonoType::Int(64)),
                value: ConstValue::Int(5),
            },
        ),
        (
            "literal-bool",
            MonoType::Literal {
                name: "true".into(),
                base_type: Box::new(MonoType::Bool),
                value: ConstValue::Bool(true),
            },
        ),
        (
            "literal-float",
            MonoType::Literal {
                name: "1.5".into(),
                base_type: Box::new(MonoType::Float(64)),
                value: ConstValue::Float(1.5),
            },
        ),
        (
            "meta-empty",
            MonoType::MetaType {
                universe_level: UniverseLevel::type0(),
                type_params: vec![],
            },
        ),
        (
            "meta-params",
            MonoType::MetaType {
                universe_level: UniverseLevel::new("1"),
                type_params: vec![MonoType::Int(64)],
            },
        ),
        (
            "refined",
            MonoType::Refined {
                base: Box::new(MonoType::Int(64)),
                constraint: ConstExpr::Lit(ConstValue::Int(0)),
            },
        ),
        (
            "depfn",
            MonoType::DepFn {
                params: vec![
                    DepParam {
                        name: "n".into(),
                        ty: MonoType::Int(64),
                    },
                    DepParam {
                        name: "m".into(),
                        ty: MonoType::Bool,
                    },
                ],
                return_type: Box::new(MonoType::Float(64)),
            },
        ),
        (
            "libraryref",
            MonoType::LibraryRef {
                mechanism: "dylib".into(),
                lib: "foo.so".into(),
            },
        ),
        (
            "externref",
            MonoType::ExternRef {
                mechanism: "dylib".into(),
                lib: "foo.so".into(),
                symbol: "bar".into(),
            },
        ),
        (
            "nested-ref-vec-union",
            MonoType::Ref {
                mutable: false,
                inner: Box::new(MonoType::Generic {
                    name: "Vec".into(),
                    args: vec![MonoType::Union(vec![
                        MonoType::Int(64),
                        MonoType::Ref {
                            mutable: true,
                            inner: Box::new(MonoType::Char),
                        },
                    ])],
                }),
            },
        ),
        (
            "nested-depfn",
            MonoType::DepFn {
                params: vec![DepParam {
                    name: "a".into(),
                    ty: MonoType::Generic {
                        name: "Vec".into(),
                        args: vec![MonoType::Int(64)],
                    },
                }],
                return_type: Box::new(MonoType::Ref {
                    mutable: false,
                    inner: Box::new(MonoType::Generic {
                        name: "Option".into(),
                        args: vec![MonoType::Int(64)],
                    }),
                }),
            },
        ),
    ]
}

/// 经公共 API（`SpecializationKey::hash`）驱动 `type_name_hash`：
/// 哈希写入序 = name → param_types（空跳）→ type_args 逐个 type_name_hash。
/// 固定 name，pin 值的差异完全来自 type_name_hash 的写入序列。
fn pinned_hash(ty: &MonoType) -> u64 {
    let key = SpecializationKey::new("f".to_string(), vec![ty.clone()]);
    let mut hasher = PinHasher::default();
    Hash::hash(&key, &mut hasher);
    hasher.finish()
}

#[test]
fn test_type_name_pin_renders_frozen_strings_for_all_variants() {
    // Arrange — 重构前抓取的渲染串（见文件头注释的抓取口径）。
    let cases: Vec<(&str, &str)> = vec![
        ("never", "never"),
        ("void", "void"),
        ("bool", "bool"),
        ("int64", "int64"),
        ("int32", "int32"),
        ("float64", "float64"),
        ("float32", "float32"),
        ("char", "char"),
        ("var0", "t0"),
        ("typeref", "Point"),
        ("struct", "Point"),
        ("enum", "Color"),
        ("fn", "fn(int64, bool) -> float64"),
        ("generic-string-empty", "string"),
        ("generic-string-extra-arg", "string"),
        ("generic-bytes", "bytes"),
        ("generic-tuple", "(int64, bool)"),
        ("generic-tuple-empty", "()"),
        ("generic-option", "int64?"),
        ("generic-option-extra-arg", "int64?"),
        ("generic-result", "Result(int64, bool)"),
        ("generic-result-arity3", "Result(int64, bool, char)"),
        ("generic-range", "Range(int64)"),
        ("generic-arc", "Arc(int64)"),
        ("generic-weak", "Weak(int64)"),
        ("generic-custom", "MyBox(int64)"),
        ("generic-custom-empty", "MyBox()"),
        ("generic-vec", "Vec(int64)"),
        ("union", "(int64 | bool | char)"),
        ("intersection", "(int64 & float64)"),
        ("ref-immutable", "&int64"),
        ("ref-mutable", "&mut int64"),
        ("assoc", "T::Item(int64)"),
        ("assoc-noargs", "T::Item"),
        ("literal-int", "int64::5"),
        ("literal-bool", "bool::true"),
        ("literal-float", "float64::1.5"),
        ("meta-empty", "Type"),
        ("meta-params", "Type"),
        ("refined", "int64 {0}"),
        ("depfn", "(n: int64, m: bool) -> float64"),
        ("libraryref", "LibraryRef(dylib, \"foo.so\")"),
        ("externref", "ExternRef(dylib, \"foo.so\", \"bar\")"),
        ("nested-ref-vec-union", "&Vec((int64 | &mut char))"),
        ("nested-depfn", "(a: Vec(int64)) -> &int64?"),
    ];

    // Act + Assert — 渲染串逐条对照 pin 表；漂移（含语义等价的文本变化）即红。
    for ((label, ty), (_, expected)) in pinned_cases().into_iter().zip(cases) {
        assert_eq!(ty.type_name(), expected, "type_name 渲染漂移（{label}）");
    }
}

#[test]
fn test_type_name_hash_pin_freezes_hash_values_for_all_variants() {
    // Arrange — 重构前抓取的哈希值（SpecializationKey::hash 口径，name = "f"）。
    let cases: Vec<(&str, u64)> = vec![
        ("never", 14585495681828089700),
        ("void", 576767476500602130),
        ("bool", 4120533627171511924),
        ("int64", 18381090431172827775),
        ("int32", 16537575568523375538),
        ("float64", 12510918264063514890),
        ("float32", 9487370138938285579),
        ("char", 16985654078077995856),
        ("var0", 15978885427281006545),
        ("typeref", 14628773616550543088),
        ("struct", 14628773616550543088),
        ("enum", 14561254204538495867),
        ("fn", 8583637562698499640),
        ("generic-string-empty", 16678978251470804437),
        ("generic-string-extra-arg", 16056942215327905457),
        ("generic-bytes", 1050639064701347409),
        ("generic-tuple", 7842717353256705995),
        ("generic-tuple-empty", 13182477735624710106),
        ("generic-option", 16383855948270799557),
        ("generic-option-extra-arg", 8866572861422040495),
        ("generic-result", 8446992642504694262),
        ("generic-result-arity3", 8671291801064578079),
        ("generic-range", 254315604409402767),
        ("generic-arc", 14405259546554717352),
        ("generic-weak", 11989949198775585858),
        ("generic-custom", 11183862763235902245),
        ("generic-custom-empty", 6619688748465570145),
        ("generic-vec", 11118011570043882886),
        ("union", 1446792346255800785),
        ("intersection", 190554762417462298),
        ("ref-immutable", 11800691148340858391),
        ("ref-mutable", 8258003741072258328),
        ("assoc", 15084669842161827575),
        ("assoc-noargs", 11349960789508987423),
        ("literal-int", 4395549033729786138),
        ("literal-bool", 13374110321896696214),
        ("literal-float", 2676443580956486531),
        ("meta-empty", 13939842812207588309),
        ("meta-params", 13634796419152832687),
        ("refined", 427711343430109435),
        ("depfn", 2994895120515759410),
        ("libraryref", 6506290583170699591),
        ("externref", 17138971209954200310),
        ("nested-ref-vec-union", 15414025407310445804),
        ("nested-depfn", 15276605436862312786),
    ];

    // Act + Assert — 哈希值逐条对照 pin 表；写入序或内容的任何漂移即红。
    for ((label, ty), (_, expected)) in pinned_cases().into_iter().zip(cases) {
        assert_eq!(
            pinned_hash(&ty),
            expected,
            "type_name_hash 哈希漂移（{label}）"
        );
    }
}
