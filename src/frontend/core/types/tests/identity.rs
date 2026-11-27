//! L3-0：类型身份视图（TypePath/Nominal）与今天 Debug/哈希的等价性。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::frontend::core::types::identity::{DefPath, Nominal, TypePath};
use crate::frontend::core::types::mono::StructFieldInfo;
use crate::frontend::core::types::{EnumType, MonoType, StructType};

/// 守卫：带点裸名、空名一律拒绝。
#[test]
fn test_type_path_guard_dotted_name() {
    // Arrange：三种非法形态——flat 含点、item 裸名含点、空名。
    // Act & Assert：构造器全部返回 None。
    assert!(
        TypePath::flat("a.T").is_none(),
        "flat 必须拒绝含点裸名，否则可绕过模块键"
    );
    assert!(
        TypePath::item("lib", "T.x").is_none(),
        "item 的裸名也必须拒绝含点"
    );
    assert!(TypePath::flat("").is_none(), "空裸名不是身份");
}

/// 守卫：方法路径（owner 非空）与带消歧序号的路径拒绝进入类型视图。
#[test]
fn test_type_path_guard_owner_and_disambiguator() {
    // Arrange：方法路径与手工构造的带序号路径。
    let method = DefPath::method("lib", "Point", "get_x");
    let mut numbered = DefPath::item("lib", "Point");
    numbered.disambiguator = 1;

    // Act & Assert：两者都不能构造 TypePath/Nominal。
    assert!(
        TypePath::from_def_path(&method).is_none(),
        "方法路径的 owner 非空，不是类型身份"
    );
    assert!(
        Nominal::from_def_path(&numbered).is_none(),
        "带消歧序号的路径暂不进入类型身份"
    );
}

/// Debug 金样：Nominal 与今天 String 的 Debug/Display 逐字一致；MonoType 文本不变。
#[test]
fn test_nominal_debug_matches_string_debug() {
    // Arrange：同一裸名分别以 Nominal 与 String 承载；另建四类今天的 MonoType 值。
    let nominal = Nominal::flat("Point").expect("合法裸名");
    let module_scoped = Nominal::item("a", "Point").expect("合法模块键 + 裸名");
    let struct_ty = MonoType::Struct(StructType {
        name: "P".to_string(),
        fields: vec![StructFieldInfo {
            name: "x".to_string(),
            ty: MonoType::Int(64),
            quantity: None,
            is_mut: false,
            has_default: false,
        }],
        methods: Default::default(),
        interfaces: vec![],
    });
    let enum_ty = MonoType::Enum(EnumType {
        name: "Color".to_string(),
        variants: vec!["Red".to_string(), "Green".to_string()],
    });
    let generic_ty = MonoType::Generic {
        name: "Option".to_string(),
        args: vec![MonoType::Int(64)],
    };
    let ref_ty = MonoType::TypeRef("Point".to_string());

    // Act：Nominal 的 Debug/Display，与四类 MonoType 的 Debug。
    let nominal_debug = format!("{:?}", nominal);
    let nominal_display = format!("{}", nominal);
    let scoped_debug = format!("{:?}", module_scoped);

    // Assert：Nominal 与 String 同形（含限定身份只打裸名）；MonoType 文本逐字固定。
    assert_eq!(
        nominal_debug,
        format!("{:?}", "Point"),
        "Nominal Debug 必须与同名 String 的 Debug 逐字相同"
    );
    assert_eq!(nominal_display, "Point", "Display 输出裸名");
    assert_eq!(
        scoped_debug, nominal_debug,
        "限定身份也只打印裸名（显示是纯函数）"
    );
    assert_eq!(
        format!("{:?}", ref_ty),
        "TypeRef(\"Point\")",
        "TypeRef 承载点位的文本与今天一致"
    );
    assert_eq!(
        format!("{:?}", generic_ty),
        "Generic { name: \"Option\", args: [Int(64)] }",
        "Generic 文本与今天一致"
    );
    assert_eq!(
        format!("{:?}", enum_ty),
        "Enum(EnumType { name: \"Color\", variants: [\"Red\", \"Green\"] })",
        "Enum 文本与今天一致"
    );
    assert_eq!(
        format!("{:?}", struct_ty),
        "Struct(StructType { name: \"P\", fields: [(\"x\", Int(64))], methods: {}, interfaces: [] })",
        "Struct 文本与今天一致"
    );
}

/// 哈希序等价：flat 的 hash 与同名 String 的 hash 在 DefaultHasher 下逐字节一致。
#[test]
fn test_type_path_hash_matches_string_hash() {
    // Arrange：同一裸名分别以 TypePath/Nominal 与 String 承载。
    let path = TypePath::flat("P").expect("合法裸名");
    let nominal = Nominal::flat("P").expect("合法裸名");
    let string = "P".to_string();

    // Act：各自写入新 hasher 并取 u64。
    let hash_of = |value: &dyn Fn(&mut DefaultHasher)| {
        let mut hasher = DefaultHasher::new();
        value(&mut hasher);
        hasher.finish()
    };
    let path_hash = hash_of(&|h| path.hash(h));
    let nominal_hash = hash_of(&|h| nominal.hash(h));
    let string_hash = hash_of(&|h| string.hash(h));

    // Assert：三者相等（R-H1 的 flat 路径保持 String 原序）。
    assert_eq!(
        path_hash, string_hash,
        "TypePath flat hash 必须等于 String hash"
    );
    assert_eq!(
        nominal_hash, string_hash,
        "Nominal flat hash 必须等于 String hash"
    );
}
