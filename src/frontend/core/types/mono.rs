//! 单态类型定义
//!
//! 实现具体类型：
//! - MonoType: 单态类型（具体类型）
//! - StructType: 结构体类型
//! - EnumType: 枚举类型
//! - UniverseLevel: RFC-010 类型宇宙层级

use crate::frontend::core::parser::ast;
use crate::frontend::core::types::var::TypeVar;
use crate::frontend::core::types::const_data::{ConstExpr, ConstVarDef, ConstValue};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

/// RFC-010: 类型宇宙层级
///
/// 使用字符串存储层级，以表示层级可以无穷无尽。
/// 编译器内部维护宇宙层级：
/// - "0" = Type0: 日常类型（Int, Float, Point 等）
/// - "1" = Type1: 类型构造器（List, Maybe 等）
/// - "2"+ = Type2+: 高阶构造器
///
/// 用户永远不会看到这些数字，只看见 `: Type`。
/// 编译器自动处理 Type0、Type1、Type2... 的区分，对用户透明。
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UniverseLevel {
    /// 层级值，使用字符串存储以支持理论上无穷的层级
    pub level: String,
}

impl UniverseLevel {
    /// 创建 Type0 层级（日常类型）
    pub fn type0() -> Self {
        Self {
            level: "0".to_string(),
        }
    }

    /// 创建 Type1 层级（类型构造器）
    pub fn type1() -> Self {
        Self {
            level: "1".to_string(),
        }
    }

    /// 创建指定层级
    pub fn new(level: impl Into<String>) -> Self {
        Self {
            level: level.into(),
        }
    }

    /// 获取下一个层级（level + 1）
    pub fn succ(&self) -> Self {
        // 将字符串解析为大整数并加一
        // 使用字符串算术以支持任意大层级
        let level_str = &self.level;
        let next = increment_level_string(level_str);
        Self { level: next }
    }

    /// 检查是否是 Type0
    pub fn is_type0(&self) -> bool {
        self.level == "0"
    }

    /// 检查是否是 Type1
    pub fn is_type1(&self) -> bool {
        self.level == "1"
    }

    /// 比较两个层级的大小
    pub fn cmp_level(
        &self,
        other: &Self,
    ) -> std::cmp::Ordering {
        let a = &self.level;
        let b = &other.level;
        // 先比较长度（位数多的更大），再字典序比较
        match a.len().cmp(&b.len()) {
            std::cmp::Ordering::Equal => a.cmp(b),
            ord => ord,
        }
    }
}

impl fmt::Display for UniverseLevel {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "Type{}", self.level)
    }
}

/// 字符串形式的大整数加一
fn increment_level_string(s: &str) -> String {
    let mut digits: Vec<u8> = s.bytes().map(|b| b - b'0').collect();
    let mut carry = 1u8;
    for d in digits.iter_mut().rev() {
        let sum = *d + carry;
        *d = sum % 10;
        carry = sum / 10;
        if carry == 0 {
            break;
        }
    }
    if carry > 0 {
        digits.insert(0, carry);
    }
    digits.into_iter().map(|d| (d + b'0') as char).collect()
}

/// 类型绑定的状态（union-find 结构）
///
/// 使用 union-find 模式实现类型变量的绑定和查找
#[derive(Debug, Clone)]
pub enum TypeBinding {
    /// 未绑定，可接受任何类型
    Unbound,
    /// 已绑定到具体类型
    Bound(MonoType),
    /// 链接到另一个类型变量（用于路径压缩）
    Link(TypeVar),
}

/// 依赖函数参数：名字 + 类型标注
///
/// 类型标注可引用前面已声明的参数名。例：
/// `(a: Int, b: Refined { base: Int, constraint: BinOp { op: Gt, left: NamedVar("b"), right: Lit(Int(0)) } })`
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DepParam {
    pub name: String,
    pub ty: MonoType,
}

/// 单态类型（具体类型）
///
/// 不包含类型变量的具体类型，用于类型检查的最终结果
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MonoType {
    /// 空类型
    Void,
    /// 布尔类型
    Bool,
    /// 整数类型（宽度）
    Int(usize),
    /// 浮点类型（宽度）
    Float(usize),
    /// 字符类型
    Char,
    /// 字符串类型
    String,
    /// 字节数组
    Bytes,
    /// 结构体类型
    Struct(StructType),
    /// 枚举类型
    Enum(EnumType),
    /// 元组类型
    Tuple(Vec<MonoType>),
    /// 列表类型
    List(Box<MonoType>),
    /// 字典类型
    Dict(Box<MonoType>, Box<MonoType>),
    /// 集合类型
    Set(Box<MonoType>),
    /// 函数类型
    Fn {
        /// 参数类型列表
        params: Vec<MonoType>,
        /// 返回类型
        return_type: Box<MonoType>,
    },
    /// `Option[T]`（RFC-001）
    Option(Box<MonoType>),
    /// Result[T, E]（RFC-001）
    Result(Box<MonoType>, Box<MonoType>),
    /// 范围类型 (start..end)
    Range {
        /// 元素类型
        elem_type: Box<MonoType>,
    },
    /// 类型变量（推断中）
    TypeVar(TypeVar),
    /// 类型引用（如自定义类型名）
    TypeRef(String),
    /// 泛型实例化类型，如 Option(Int), List(String)
    /// 与 TypeRef 的区别：TypeRef 是未解析的名称，Generic 携带结构化的类型参数
    Generic {
        /// 泛型类型名（如 "Option", "List"）
        name: String,
        /// 类型参数（如 [Int(64)] 对应 Option(Int)）
        args: Vec<MonoType>,
    },
    /// 联合类型 `T1 | T2`
    Union(Vec<MonoType>),
    /// 交集类型 `T1 & T2`
    Intersection(Vec<MonoType>),
    /// Arc 类型（原子引用计数）
    Arc(Box<MonoType>),
    /// Weak 类型（不增加引用计数）
    Weak(Box<MonoType>),
    /// 借用引用类型：`&T`（不可变）或 `&mut T`（可变）
    /// 编译期零大小类型 — 无运行时表示
    Ref {
        /// 是否可变引用
        mutable: bool,
        /// 被引用的内部类型
        inner: Box<MonoType>,
    },
    /// 关联类型访问（如 T::Item）
    AssocType {
        /// 宿主类型
        host_type: Box<MonoType>,
        /// 关联类型名称
        assoc_name: String,
        /// 关联类型参数（如果有关联类型也是泛型的）
        assoc_args: Vec<MonoType>,
    },
    /// RFC-010: 元类型 `Type`
    /// 表示一个值是类型本身
    /// universe_level 表示类型宇宙层级（用户不可见，编译器自动管理）
    /// 支持无限层级：`Type[Type[T]]` → Type2, `Type[Type[Type[T]]]` → Type3, etc.
    MetaType {
        /// 类型宇宙层级
        universe_level: UniverseLevel,
        /// 泛型参数（`Type[T]` 中的 T，可以是嵌套的 MetaType）
        /// e.g., `Type[Type[T]]` 的 type_params = `MetaType { type_params: T }`
        type_params: Vec<MonoType>,
    },
    /// 字面量类型（编译期常量值作为类型）
    /// 用于 Const 泛型，如 "5" 表示值 5 的字面量类型
    Literal {
        /// 字面量名称（标识符）
        name: String,
        /// 基础类型（如 Int）
        base_type: Box<MonoType>,
        /// 对应的 ConstValue
        value: ConstValue,
    },

    /// 精化类型：基类型 + 编译期谓词约束
    ///
    /// 例：Positive(b) 正格化后 → Refined { base: Int, constraint: b > 0 }
    /// 运行时擦除为 base，编译期由证明管道验证 constraint
    Refined {
        base: Box<MonoType>,
        constraint: ConstExpr,
    },

    /// 依赖函数类型：参数的类型标注可引用之前声明的参数名
    ///
    /// 与 Fn 的区别：DepFn 的参数类型可形成依赖链（后面的参数类型可引用前面的参数名）
    DepFn {
        params: Vec<DepParam>,
        return_type: Box<MonoType>,
    },
}

impl MonoType {
    /// 检查是否是数值类型
    pub fn is_numeric(&self) -> bool {
        match self {
            MonoType::Int(_) | MonoType::Float(_) => true,
            MonoType::Refined { base, .. } => base.is_numeric(),
            _ => false,
        }
    }

    /// 检查是否是可索引类型
    pub fn is_indexable(&self) -> bool {
        match self {
            MonoType::List(_) | MonoType::Dict(_, _) | MonoType::String | MonoType::Tuple(_) => {
                true
            }
            MonoType::Refined { base, .. } => base.is_indexable(),
            _ => false,
        }
    }

    /// 判断是否是约束类型（所有字段都是函数类型）
    ///
    /// 约束类型 = 接口，定义为所有字段都是函数类型的记录类型
    pub fn is_constraint(&self) -> bool {
        match self {
            // 结构体类型：检查所有字段是否都是函数类型
            MonoType::Struct(s) => s
                .fields
                .iter()
                .all(|(_, ty)| matches!(ty, MonoType::Fn { .. })),
            // TypeRef 指向的可能是约束类型（需要结合类型环境判断）
            // 这里返回 false，具体判断在类型检查时结合环境确定
            MonoType::TypeRef(_) => false,
            // Generic 携带类型参数，不是约束类型
            MonoType::Generic { .. } => false,
            _ => false,
        }
    }

    /// 获取约束的所有要求字段
    /// 返回字段名和类型的列表
    pub fn constraint_fields(&self) -> Vec<(String, &MonoType)> {
        match self {
            MonoType::Struct(s) => {
                // 只返回函数字段
                s.fields
                    .iter()
                    .filter(|(_, ty)| matches!(ty, MonoType::Fn { .. }))
                    .map(|(name, ty)| (name.clone(), ty))
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    /// 获取类型的字符串描述
    pub fn type_name(&self) -> String {
        match self {
            MonoType::Void => "void".to_string(),
            MonoType::Bool => "bool".to_string(),
            MonoType::Int(n) => format!("int{}", n),
            MonoType::Float(n) => format!("float{}", n),
            MonoType::Char => "char".to_string(),
            MonoType::String => "string".to_string(),
            MonoType::Bytes => "bytes".to_string(),
            MonoType::Struct(s) => s.name.clone(),
            MonoType::Enum(e) => e.name.clone(),
            MonoType::Tuple(types) => {
                format!(
                    "({})",
                    types
                        .iter()
                        .map(|t| t.type_name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            MonoType::List(t) => format!("List({})", t.type_name()),
            MonoType::Dict(k, v) => format!("Dict({}, {})", k.type_name(), v.type_name()),
            MonoType::Set(t) => format!("Set({})", t.type_name()),
            MonoType::Fn {
                params,
                return_type,
            } => {
                let params_str = params
                    .iter()
                    .map(|t| t.type_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fn({}) -> {}", params_str, return_type.type_name())
            }
            MonoType::Option(inner) => format!("{}?", inner.type_name()),
            MonoType::Result(ok, err) => {
                format!("Result({}, {})", ok.type_name(), err.type_name())
            }
            MonoType::TypeVar(v) => format!("{}", v),
            MonoType::TypeRef(name) => name.clone(),
            MonoType::Generic { name, args } => {
                format!(
                    "{}({})",
                    name,
                    args.iter()
                        .map(|t| t.type_name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            MonoType::Range { elem_type } => format!("Range({})", elem_type.type_name()),
            MonoType::Union(types) => {
                format!(
                    "({})",
                    types
                        .iter()
                        .map(|t| t.type_name())
                        .collect::<Vec<_>>()
                        .join(" | ")
                )
            }
            MonoType::Intersection(types) => {
                format!(
                    "({})",
                    types
                        .iter()
                        .map(|t| t.type_name())
                        .collect::<Vec<_>>()
                        .join(" & ")
                )
            }
            MonoType::Arc(t) => format!("Arc({})", t.type_name()),
            MonoType::Weak(t) => format!("Weak({})", t.type_name()),
            MonoType::Ref { mutable, inner } => {
                if *mutable {
                    format!("&mut {}", inner.type_name())
                } else {
                    format!("&{}", inner.type_name())
                }
            }
            MonoType::AssocType {
                host_type,
                assoc_name,
                assoc_args,
            } => {
                let args_str = if assoc_args.is_empty() {
                    String::new()
                } else {
                    format!(
                        "({})",
                        assoc_args
                            .iter()
                            .map(|t| t.type_name())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                format!("{}::{}{}", host_type.type_name(), assoc_name, args_str)
            }
            MonoType::Literal {
                name: _,
                base_type,
                value,
            } => {
                format!("{}::{}", base_type.type_name(), value)
            }
            MonoType::MetaType {
                universe_level: _,
                type_params: _,
            } => {
                // RFC-010: Type is always just "Type", no [] syntax
                "Type".to_string()
            }
            MonoType::Refined { base, constraint } => {
                format!("{} {{{}}}", base.type_name(), constraint)
            }
            MonoType::DepFn {
                params,
                return_type,
            } => {
                let params_str = params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, p.ty.type_name()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({}) -> {}", params_str, return_type.type_name())
            }
        }
    }

    /// 如果是 TypeVar 变体，返回内部 TypeVar
    pub fn type_var(&self) -> Option<TypeVar> {
        match self {
            MonoType::TypeVar(v) => Some(*v),
            _ => None,
        }
    }
}

impl fmt::Display for MonoType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.type_name())
    }
}

impl From<ast::Type> for MonoType {
    fn from(ast_type: ast::Type) -> Self {
        match ast_type {
            ast::Type::Name { name, .. } => MonoType::TypeRef(name),
            ast::Type::Int(n) => MonoType::Int(n),
            ast::Type::Float(n) => MonoType::Float(n),
            ast::Type::Char => MonoType::Char,
            ast::Type::String => MonoType::String,
            ast::Type::Bytes => MonoType::Bytes,
            ast::Type::Bool => MonoType::Bool,
            ast::Type::Void => MonoType::Void,
            ast::Type::Struct {
                fields, interfaces, ..
            } => {
                let (field_names, field_types, field_mutability, field_has_default) = fields
                    .into_iter()
                    .map(|f| (f.name, MonoType::from(f.ty), f.is_mut, f.default.is_some()))
                    .fold(
                        (Vec::new(), Vec::new(), Vec::new(), Vec::new()),
                        |(mut names, mut types, mut mutability, mut defaults), (n, t, m, d)| {
                            names.push(n);
                            types.push(t);
                            mutability.push(m);
                            defaults.push(d);
                            (names, types, mutability, defaults)
                        },
                    );
                MonoType::Struct(StructType {
                    name: String::new(),
                    fields: field_names.into_iter().zip(field_types).collect(),
                    methods: HashMap::new(),
                    field_mutability,
                    field_has_default,
                    interfaces,
                })
            }
            ast::Type::Union(variants) => MonoType::Enum(EnumType {
                name: String::new(),
                variants: variants.into_iter().map(|(n, _)| n).collect(),
            }),
            ast::Type::Enum(variants) => MonoType::Enum(EnumType {
                name: String::new(),
                variants,
            }),
            // New variant type: `type Color = red | green | blue`
            ast::Type::Variant(variants) => MonoType::Enum(EnumType {
                name: String::new(),
                variants: variants.into_iter().map(|v| v.name).collect(),
            }),
            ast::Type::Tuple(types) => {
                MonoType::Tuple(types.into_iter().map(MonoType::from).collect())
            }
            ast::Type::Fn {
                params,
                return_type,
            } => MonoType::Fn {
                params: params.into_iter().map(MonoType::from).collect(),
                return_type: Box::new(MonoType::from(*return_type)),
            },
            ast::Type::Option(t) => MonoType::Option(Box::new(MonoType::from(*t))),
            ast::Type::Result(ok, err) => MonoType::Result(
                Box::new(MonoType::from(*ok)),
                Box::new(MonoType::from(*err)),
            ),
            ast::Type::Generic { name, args, .. } => {
                // RFC-001: lower well-known generics.
                if name == "Option" && args.len() == 1 {
                    return MonoType::Option(Box::new(MonoType::from(args[0].clone())));
                }
                if name == "Result" && args.len() == 2 {
                    return MonoType::Result(
                        Box::new(MonoType::from(args[0].clone())),
                        Box::new(MonoType::from(args[1].clone())),
                    );
                }
                if name == "List" && args.len() == 1 {
                    return MonoType::List(Box::new(MonoType::from(args[0].clone())));
                }
                if name == "Dict" && args.len() == 2 {
                    return MonoType::Dict(
                        Box::new(MonoType::from(args[0].clone())),
                        Box::new(MonoType::from(args[1].clone())),
                    );
                }
                if name == "Set" && args.len() == 1 {
                    return MonoType::Set(Box::new(MonoType::from(args[0].clone())));
                }
                // 泛型类型，如 Option(T), List(Int)
                MonoType::Generic {
                    name,
                    args: args.into_iter().map(MonoType::from).collect(),
                }
            }
            ast::Type::AssocType {
                host_type,
                assoc_name,
                assoc_args,
                ..
            } => MonoType::AssocType {
                host_type: Box::new(MonoType::from(*host_type)),
                assoc_name,
                assoc_args: assoc_args.into_iter().map(MonoType::from).collect(),
            },
            // NamedStruct and Sum types (placeholder implementations)
            ast::Type::NamedStruct { name, fields, .. } => {
                let (field_names, field_types, field_mutability) = fields
                    .into_iter()
                    .map(|f| (f.name, MonoType::from(f.ty), f.is_mut))
                    .fold(
                        (Vec::new(), Vec::new(), Vec::new()),
                        |(mut names, mut types, mut mutability), (n, t, m)| {
                            names.push(n);
                            types.push(t);
                            mutability.push(m);
                            (names, types, mutability)
                        },
                    );
                MonoType::Struct(StructType {
                    name,
                    fields: field_names.into_iter().zip(field_types).collect(),
                    methods: HashMap::new(),
                    field_mutability,
                    field_has_default: Vec::new(),
                    interfaces: vec![],
                })
            }
            ast::Type::Sum(types) => {
                // Sum type - treat as union for now
                MonoType::TypeRef(format!(
                    "({})",
                    types
                        .iter()
                        .map(|t| MonoType::from(t.clone()).type_name())
                        .collect::<Vec<_>>()
                        .join(" | ")
                ))
            }
            ast::Type::Literal {
                name: _, base_type, ..
            } => {
                // Literal type - for now, convert to the base type
                // The actual literal value is handled during const evaluation
                MonoType::from(*base_type)
            }
            ast::Type::Ptr(inner) => {
                // Raw pointer type: *T
                MonoType::TypeRef(format!("*{}", MonoType::from(*inner).type_name()))
            }
            ast::Type::Ref { mutable, inner, .. } => MonoType::Ref {
                mutable,
                inner: Box::new(MonoType::from(*inner)),
            },
            ast::Type::MetaType { args, .. } => {
                // RFC-010: Meta-type `Type` or `Type[T]` or `Type[Type[T]]`
                // Determine universe level recursively:
                // - plain Type = Type0
                // - Type[T] = Type1 (if T is Type0)
                // - Type[Type[T]] = Type2 (if T is Type1)
                // - etc.
                let universe_level = calculate_meta_type_level(&args);
                MonoType::MetaType {
                    universe_level,
                    type_params: args.into_iter().map(MonoType::from).collect(),
                }
            }
        }
    }
}

/// 将 AST 类型注解转换为 PolyType
///
/// 复用 `From<ast::Type> for MonoType` 实现，包装为 PolyType。
/// 用于跨文件检查时注册模块导出的类型信息。
pub fn ast_type_to_poly_type(
    ast_type: &crate::frontend::core::parser::ast::Type
) -> super::PolyType {
    super::PolyType::mono(MonoType::from(ast_type.clone()))
}

/// Calculate the universe level for a meta-type
/// Returns max(arg_level) + 1, or Type0 if args is empty
pub fn calculate_meta_type_level(args: &[ast::Type]) -> UniverseLevel {
    if args.is_empty() {
        return UniverseLevel::type0();
    }

    // Find the maximum universe level among all arguments
    let max_arg_level = args.iter().fold(0, |max, arg| {
        let arg_level = get_ast_type_universe_level(arg);
        max.max(arg_level)
    });

    // Result level is max_arg_level + 1
    UniverseLevel::new((max_arg_level + 1).to_string())
}

/// Get the universe level of an AST type (before conversion to MonoType)
pub fn get_ast_type_universe_level(ast_type: &ast::Type) -> usize {
    match ast_type {
        ast::Type::MetaType { args, .. } => {
            // Recursively calculate level
            if args.is_empty() {
                0 // Type itself is Type0
            } else {
                let max_arg_level = args
                    .iter()
                    .fold(0, |max, arg| max.max(get_ast_type_universe_level(arg)));
                max_arg_level + 1
            }
        }
        _ => 0, // Non-meta types are at Type0
    }
}

/// 结构体类型
#[derive(Debug, Clone)]
pub struct StructType {
    pub name: String,
    pub fields: Vec<(String, MonoType)>,
    /// 方法表：方法名 -> 方法类型
    pub methods: HashMap<String, PolyType>,
    /// 字段可变性标记：与 fields 索引对应
    pub field_mutability: Vec<bool>,
    /// 字段默认值标记：与 fields 索引对应，标记哪些字段有默认值
    pub field_has_default: Vec<bool>,
    /// RFC-010: 接口约束列表
    pub interfaces: Vec<String>,
}

impl StructType {
    /// 检查指定字段是否可变
    pub fn field_is_mut(
        &self,
        field_name: &str,
    ) -> Option<bool> {
        self.fields
            .iter()
            .position(|(name, _)| name == field_name)
            .map(|idx| self.field_mutability.get(idx).copied().unwrap_or(false))
    }
}

// 为 StructType 实现自定义的 Hash 和 Eq
impl PartialEq for StructType {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.name == other.name && self.fields == other.fields
    }
}

impl Eq for StructType {}

impl Hash for StructType {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        self.fields.hash(state);
        self.field_mutability.hash(state);
    }
}

/// 枚举类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumType {
    pub name: String,
    pub variants: Vec<String>,
}

/// 多态类型（带泛型参数和Const泛型参数）
///
/// 包含泛型变量列表、Const泛型变量列表和类型体，用于表示泛型函数或泛型类型的签名
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyType {
    /// 类型泛型变量列表（按顺序）
    pub type_binders: Vec<TypeVar>,
    /// Const泛型变量列表（按顺序）
    pub const_binders: Vec<ConstVarDef>,
    /// 类型体
    pub body: MonoType,
}

impl PolyType {
    /// 创建新的多态类型（仅类型泛型）
    pub fn new(
        type_binders: Vec<TypeVar>,
        body: MonoType,
    ) -> Self {
        PolyType {
            type_binders,
            const_binders: Vec::new(),
            body,
        }
    }

    /// 创建新的多态类型（包含Const泛型）
    pub fn new_with_const(
        type_binders: Vec<TypeVar>,
        const_binders: Vec<ConstVarDef>,
        body: MonoType,
    ) -> Self {
        PolyType {
            type_binders,
            const_binders,
            body,
        }
    }

    /// 创建单态类型（无泛型）
    pub fn mono(body: MonoType) -> Self {
        PolyType {
            type_binders: Vec::new(),
            const_binders: Vec::new(),
            body,
        }
    }

    /// 获取类型的字符串表示
    pub fn type_name(&self) -> String {
        self.body.type_name()
    }

    /// 检查是否为空泛型（无类型参数）
    pub fn is_mono(&self) -> bool {
        self.type_binders.is_empty() && self.const_binders.is_empty()
    }
}

impl fmt::Display for PolyType {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.type_name())
    }
}
