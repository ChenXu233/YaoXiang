//! RFC-011b: 运算符接口与实现登记表
//!
//! **Layer 0**（运算符 → 接口/方法的固定映射，语言级常量，用户不可改）与
//! **Layer 2**（七个运算符接口的编译器侧声明）在此成文。接口声明以规格
//! 形式注册，经 RFC-011a 既有实例化管线检查（E1095–E1100），通过后与
//! `ImplementationProof` 同步写入**接口实现登记表**（environment.rs，
//! 带类型实参维度——proof 无实参，无法区分同一接口的不同实例化）。
//!
//! **名字与登记分离**（RFC-011b §名字与登记）：运算符查询只看登记表，
//! 不经普通名字解析；本地同名绑定（如 RFC-011 §5.2 的类型级 `Add` 家族）
//! 不影响运算符可用性。
//!
//! 首批七个接口：`Add` `Subtract` `Multiply` `Divide` `Modulo` `Equal`
//! `Index`（算术五接口为三类型参数 `(Self, R, O)`——结果类型 `O` 显式化，
//! 使 `1 + 2.5` 与向量缩放可表达；`Try` 属阶段 2，不在本表）。

use std::collections::HashMap;

use crate::frontend::core::parser::ast::BinOp;
use crate::frontend::core::types::mono::StructType;
use crate::frontend::core::types::{MonoType, PolyType, TypeVar};

use super::environment::{GenericTypeDef, InterfaceImplEntry, TypeEnvironment};

/// 运算符接口规格（Layer 2 声明 + Layer 0 方法名映射）
pub struct OperatorInterfaceSpec {
    pub name: &'static str,
    /// 接口类型形参名，按序；首个恒为 `Self`
    pub params: &'static [&'static str],
    /// 运算符派发到的方法名（Layer 0 固定映射，语言级常量）
    pub method: &'static str,
}

/// 首批七个运算符接口（RFC-011b §Layer 0 映射表）
pub const SPECS: &[OperatorInterfaceSpec] = &[
    OperatorInterfaceSpec {
        name: "Add",
        params: &["Self", "R", "O"],
        method: "add",
    },
    OperatorInterfaceSpec {
        name: "Subtract",
        params: &["Self", "R", "O"],
        method: "subtract",
    },
    OperatorInterfaceSpec {
        name: "Multiply",
        params: &["Self", "R", "O"],
        method: "multiply",
    },
    OperatorInterfaceSpec {
        name: "Divide",
        params: &["Self", "R", "O"],
        method: "divide",
    },
    OperatorInterfaceSpec {
        name: "Modulo",
        params: &["Self", "R", "O"],
        method: "modulo",
    },
    OperatorInterfaceSpec {
        name: "Equal",
        params: &["Self", "R"],
        method: "equal",
    },
    OperatorInterfaceSpec {
        name: "Index",
        params: &["Self", "Key", "Value"],
        method: "index",
    },
];

/// 按名查规格
pub fn spec(name: &str) -> Option<&'static OperatorInterfaceSpec> {
    SPECS.iter().find(|s| s.name == name)
}

/// Layer 0：算术运算符 → 接口名（比较运算符保留原生指令，不在此表）
pub fn arithmetic_interface(op: &BinOp) -> Option<&'static str> {
    match op {
        BinOp::Add => Some("Add"),
        BinOp::Sub => Some("Subtract"),
        BinOp::Mul => Some("Multiply"),
        BinOp::Div => Some("Divide"),
        BinOp::Mod => Some("Modulo"),
        _ => None,
    }
}

/// Layer 0：`==` / `!=` → 接口名
pub const EQUAL_INTERFACE: &str = "Equal";

/// Layer 0：`[]` → 接口名
pub const INDEX_INTERFACE: &str = "Index";

/// 接口成员签名模板：以接口形参名表达（`Self`/`R`/`O`/`Key`/`Value`），
/// 实例化检查时经 `replace_type_params` 以实参替换。
///
/// - 算术五接口：`add: (self: &Self, other: &R) -> O`
/// - `Equal`：`equal: (self: &Self, other: &R) -> Bool`
/// - `Index`：`index: (self: &Self, key: &Key) -> Value`
pub fn member_signature(spec: &OperatorInterfaceSpec) -> MonoType {
    let borrow = |t: MonoType| MonoType::Ref {
        mutable: false,
        inner: Box::new(t),
    };
    let param = |n: &str| MonoType::TypeRef(n.to_string());
    let params = match spec.name {
        "Equal" => vec![borrow(param("Self")), borrow(param("R"))],
        "Index" => vec![borrow(param("Self")), borrow(param("Key"))],
        _ => vec![borrow(param("Self")), borrow(param("R"))],
    };
    let return_type = match spec.name {
        "Equal" => Box::new(MonoType::Bool),
        "Index" => Box::new(param("Value")),
        _ => Box::new(param("O")),
    };
    MonoType::Fn {
        params,
        return_type,
    }
}

/// 把七个接口注册为泛型类型构造器（接口形态，RFC-011a）。
///
/// 注册进 `generic_type_defs` 后，类型体内 `Add(Point, Point, Point)` 即被
/// `is_interface_application` 识别并进入实例化检查管线；成员展开与签名
/// 校验走本模块规格（`expand_interface_members` 的规格短路），不依赖
/// 用户源码里的接口声明体。
pub fn register_interface_defs(env: &mut TypeEnvironment) {
    for spec in SPECS {
        let body = MonoType::Struct(empty_struct(
            spec.name,
            vec![(spec.method.to_string(), member_signature(spec))],
        ));
        let type_binders = (0..spec.params.len()).map(TypeVar::new).collect();
        env.generic_type_defs.insert(
            spec.name.to_string(),
            GenericTypeDef {
                poly: PolyType::new(type_binders, body),
                type_param_names: spec.params.iter().map(|s| s.to_string()).collect(),
            },
        );
    }
}

/// 核心默认登记（native 条目）：基本类型的运算符接口登记，原生指令路径，
/// 无用户方法绑定（运算符代码生成走原生指令，不调用方法）。
///
/// `Add(Int, Float, Float)` 等混合条目是 RFC-011b 三参数 `O` 的直接落点：
/// `1 + 2.5` 经此登记为合法运算，结果 `Float`。
///
/// 注：`List`/`Vec` 等泛型容器的同型运算走 `infer_binary` 白名单快路径，
/// 带泛型实参的 native 登记条目待登记表支持 unify 匹配后再补。
pub fn register_native_entries(env: &mut TypeEnvironment) {
    let int = MonoType::Int(64);
    let float = MonoType::Float(64);
    let string = MonoType::make_string();
    let arithmetic_names = ["Add", "Subtract", "Multiply", "Divide", "Modulo"];
    for name in arithmetic_names {
        let combos: &[&[MonoType]] = if name == "Add" {
            &[
                &[int.clone(), int.clone(), int.clone()],
                &[int.clone(), float.clone(), float.clone()],
                &[float.clone(), int.clone(), float.clone()],
                &[float.clone(), float.clone(), float.clone()],
                &[string.clone(), string.clone(), string.clone()],
            ]
        } else {
            &[
                &[int.clone(), int.clone(), int.clone()],
                &[int.clone(), float.clone(), float.clone()],
                &[float.clone(), int.clone(), float.clone()],
                &[float.clone(), float.clone(), float.clone()],
            ]
        };
        for args in combos {
            env.add_interface_impl(
                name,
                InterfaceImplEntry {
                    impl_type: type_head_name(&args[0]).unwrap_or("Int").to_string(),
                    args: args.to_vec(),
                    methods: vec![spec(name).map(|s| s.method).unwrap_or_default().to_string()],
                    native: true,
                },
            );
        }
    }
    // Equal：基础类型同型比较（Bytes 沿用 trait_data 的排除口径，不登记）
    for lhs in [
        MonoType::Int(64),
        MonoType::Float(64),
        MonoType::Bool,
        MonoType::Char,
        MonoType::make_string(),
    ] {
        env.add_interface_impl(
            EQUAL_INTERFACE,
            InterfaceImplEntry {
                impl_type: type_head_name(&lhs).unwrap_or("Int").to_string(),
                args: vec![lhs.clone(), lhs],
                methods: vec!["equal".to_string()],
                native: true,
            },
        );
    }
}

/// RFC-011b: 运算符派发点（span 键控）。
///
/// typecheck 在 `==`/`!=`（显式 Equal 实例化命中）等位置产出，ir_gen 按
/// span 查表把原生指令替换为 `Call "{type}.{method}"`——与 RFC-011a
/// existential_coercions 同一「span 键表 → IR 注入」模式。
#[derive(Debug, Clone)]
pub struct OperatorDispatch {
    /// 运算符表达式在 AST 中的 span（ir_gen 查表键）
    pub span: crate::util::span::Span,
    /// 接收者类型名（如 "Vec3"）
    pub type_name: String,
    /// 派发方法名（如 "equal"）
    pub method: String,
    /// 结果取反（`!=` 派发到 equal 后需 Not）
    pub negate: bool,
}

/// 名义头提取：Struct/TypeRef/Generic 取名字；内建标量无名字返回 None。
fn type_head_name(ty: &MonoType) -> Option<&str> {
    match ty {
        MonoType::Struct(s) => Some(&s.name),
        MonoType::TypeRef(n) => Some(n),
        MonoType::Generic { name, .. } => Some(name),
        _ => None,
    }
}

/// 空结构体骨架（StructType 无 Default 派生，显式构造）
fn empty_struct(
    name: &str,
    fields: Vec<(String, MonoType)>,
) -> StructType {
    StructType {
        name: name.to_string(),
        fields,
        methods: HashMap::new(),
        field_mutability: Vec::new(),
        field_has_default: Vec::new(),
        interfaces: Vec::new(),
    }
}

/// 实参相容性判定（stage 1：条目实参均为具体类型，纯结构比较，不动 solver）。
///
/// 名义类型（Struct/TypeRef/Generic）比名字并递归比结构实参——登记侧
/// `TypeRef("Point")` 与查询侧 `Struct(Point)` 表示不同但名义相同视为命中；
/// 内建标量按变体比较（宽度不敏感，`Int(64)` 与 `Int(32)` 同为 Int）。
pub fn args_compatible(
    a: &MonoType,
    b: &MonoType,
) -> bool {
    use MonoType::*;
    match (a, b) {
        (Struct(x), Struct(y)) => x.name == y.name,
        (TypeRef(x), TypeRef(y)) => x == y,
        (Struct(x), TypeRef(y)) | (TypeRef(y), Struct(x)) => x.name == *y,
        (Generic { name: xn, args: xa }, Generic { name: yn, args: ya }) => {
            xn == yn
                && xa.len() == ya.len()
                && xa.iter().zip(ya.iter()).all(|(p, q)| args_compatible(p, q))
        }
        // 名义类型与裸名字（未实例化的查询侧形态）
        (Generic { name: xn, .. }, TypeRef(y)) | (TypeRef(y), Generic { name: xn, .. }) => xn == y,
        (Int(_), Int(_)) | (Float(_), Float(_)) | (Bool, Bool) | (Char, Char) => true,
        (Void, Void) | (Never, Never) => true,
        (
            Ref {
                mutable: m1,
                inner: i1,
            },
            Ref {
                mutable: m2,
                inner: i2,
            },
        ) => m1 == m2 && args_compatible(i1, i2),
        (
            Fn {
                params: p1,
                return_type: r1,
            },
            Fn {
                params: p2,
                return_type: r2,
            },
        ) => {
            p1.len() == p2.len()
                && p1.iter().zip(p2.iter()).all(|(x, y)| args_compatible(x, y))
                && args_compatible(r1, r2)
        }
        _ => a == b,
    }
}

/// 全实参查询：命中返回条目（如 `Equal` 的 `[Self, R]` 查询）。
pub fn query_exact<'e>(
    registry: &'e HashMap<String, Vec<InterfaceImplEntry>>,
    interface: &str,
    args: &[MonoType],
) -> Option<&'e InterfaceImplEntry> {
    registry.get(interface)?.iter().find(|e| {
        e.args.len() == args.len()
            && e.args
                .iter()
                .zip(args.iter())
                .all(|(x, y)| args_compatible(x, y))
    })
}

/// 前缀查询：以实参前缀匹配，返回（条目, 剩余实参）。
/// 算术接口 `Add(L, R, ?)` 以前缀 `[L, R]` 查询，余参即结果类型 `O`；
/// `Index(Self, Key, ?)` 同理，余参为 `Value`。
pub fn query_prefix<'e>(
    registry: &'e HashMap<String, Vec<InterfaceImplEntry>>,
    interface: &str,
    prefix: &[MonoType],
) -> Option<(&'e InterfaceImplEntry, Vec<MonoType>)> {
    let entry = registry.get(interface)?.iter().find(|e| {
        e.args.len() > prefix.len()
            && e.args
                .iter()
                .zip(prefix.iter())
                .all(|(x, y)| args_compatible(x, y))
    })?;
    let remaining = entry.args[prefix.len()..].to_vec();
    Some((entry, remaining))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_with_natives() -> TypeEnvironment {
        let mut env = TypeEnvironment::new();
        register_interface_defs(&mut env);
        register_native_entries(&mut env);
        env
    }

    #[test]
    fn native_arithmetic_matrix_queryable() {
        let env = registry_with_natives();
        let r = &env.interface_impl_registry;

        // 同型
        let (e, o) = query_prefix(r, "Add", &[MonoType::Int(64), MonoType::Int(64)]).unwrap();
        assert!(e.native);
        assert_eq!(o, vec![MonoType::Int(64)]);

        // 混合：结果恒 Float（O 显式）
        let (_, o) = query_prefix(r, "Add", &[MonoType::Int(64), MonoType::Float(64)]).unwrap();
        assert_eq!(o, vec![MonoType::Float(64)]);
        let (_, o) =
            query_prefix(r, "Multiply", &[MonoType::Float(64), MonoType::Int(64)]).unwrap();
        assert_eq!(o, vec![MonoType::Float(64)]);

        // 字符串拼接
        let (_, o) = query_prefix(
            r,
            "Add",
            &[MonoType::make_string(), MonoType::make_string()],
        )
        .unwrap();
        assert_eq!(o, vec![MonoType::make_string()]);

        // 未登记组合
        assert!(query_prefix(r, "Add", &[MonoType::make_string(), MonoType::Int(64)]).is_none());
        assert!(query_prefix(
            r,
            "Subtract",
            &[MonoType::make_string(), MonoType::make_string()]
        )
        .is_none());
    }

    #[test]
    fn nominal_entries_match_across_representations() {
        // 登记侧 TypeRef("Point") 与查询侧 Struct(Point) 名义相同应命中
        let point_struct = MonoType::Struct(empty_struct("Point", Vec::new()));
        assert!(args_compatible(
            &MonoType::TypeRef("Point".into()),
            &point_struct
        ));
        // Generic 递归：List(Int) vs List(Int)
        let li = |t: MonoType| MonoType::Generic {
            name: "List".into(),
            args: vec![t],
        };
        assert!(args_compatible(
            &li(MonoType::TypeRef("Point".into())),
            &li(point_struct),
        ));
        // 名字不同不命中
        assert!(!args_compatible(
            &MonoType::TypeRef("Point".into()),
            &MonoType::TypeRef("Vec".into()),
        ));
    }

    #[test]
    fn interface_defs_registered_for_routing() {
        let env = registry_with_natives();
        for name in [
            "Add", "Subtract", "Multiply", "Divide", "Modulo", "Equal", "Index",
        ] {
            assert!(
                env.generic_type_defs.contains_key(name),
                "{name} 应注册为接口构造器"
            );
        }
    }
}
