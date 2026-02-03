//! 单态类型定义
//!
//! 实现具体类型：
//! - MonoType: 单态类型（具体类型）
//! - StructType: 结构体类型
//! - EnumType: 枚举类型

use crate::frontend::core::parser::ast;
use super::var::TypeVar;
use super::const_data::{ConstVarDef, ConstValue};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

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
        /// 是否异步
        is_async: bool,
    },
    /// 范围类型 (start..end)
    Range {
        /// 元素类型
        elem_type: Box<MonoType>,
    },
    /// 类型变量（推断中）
    TypeVar(TypeVar),
    /// 类型引用（如自定义类型名）
    TypeRef(String),
    /// 联合类型 `T1 | T2`
    Union(Vec<MonoType>),
    /// 交集类型 `T1 & T2`
    Intersection(Vec<MonoType>),
    /// Arc 类型（原子引用计数）
    Arc(Box<MonoType>),
    /// 关联类型访问（如 T::Item）
    AssocType {
        /// 宿主类型
        host_type: Box<MonoType>,
        /// 关联类型名称
        assoc_name: String,
        /// 关联类型参数（如果有关联类型也是泛型的）
        assoc_args: Vec<MonoType>,
    },
}

impl MonoType {
    /// 检查是否是数值类型
    pub fn is_numeric(&self) -> bool {
        matches!(self, MonoType::Int(_) | MonoType::Float(_))
    }

    /// 检查是否是可索引类型
    pub fn is_indexable(&self) -> bool {
        matches!(
            self,
            MonoType::List(_) | MonoType::Dict(_, _) | MonoType::String | MonoType::Tuple(_)
        )
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
            MonoType::List(t) => format!("List<{}>", t.type_name()),
            MonoType::Dict(k, v) => format!("Dict<{}, {}>", k.type_name(), v.type_name()),
            MonoType::Set(t) => format!("Set<{}>", t.type_name()),
            MonoType::Fn {
                params,
                return_type,
                is_async: _,
            } => {
                let params_str = params
                    .iter()
                    .map(|t| t.type_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fn({}) -> {}", params_str, return_type.type_name())
            }
            MonoType::TypeVar(v) => format!("{}", v),
            MonoType::TypeRef(name) => name.clone(),
            MonoType::Range { elem_type } => format!("Range<{}>", elem_type.type_name()),
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
            MonoType::Arc(t) => format!("Arc<{}>", t.type_name()),
            MonoType::AssocType {
                host_type,
                assoc_name,
                assoc_args,
            } => {
                let args_str = if assoc_args.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<{}>",
                        assoc_args
                            .iter()
                            .map(|t| t.type_name())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                format!("{}::{}{}", host_type.type_name(), assoc_name, args_str)
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
            ast::Type::Name(name) => MonoType::TypeRef(name),
            ast::Type::Int(n) => MonoType::Int(n),
            ast::Type::Float(n) => MonoType::Float(n),
            ast::Type::Char => MonoType::Char,
            ast::Type::String => MonoType::String,
            ast::Type::Bytes => MonoType::Bytes,
            ast::Type::Bool => MonoType::Bool,
            ast::Type::Void => MonoType::Void,
            ast::Type::Struct(fields) => MonoType::Struct(StructType {
                name: String::new(),
                fields: fields
                    .into_iter()
                    .map(|(n, t)| (n, MonoType::from(t)))
                    .collect(),
                methods: HashMap::new(),
            }),
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
            ast::Type::List(t) => MonoType::List(Box::new(MonoType::from(*t))),
            ast::Type::Dict(k, v) => {
                MonoType::Dict(Box::new(MonoType::from(*k)), Box::new(MonoType::from(*v)))
            }
            ast::Type::Set(t) => MonoType::Set(Box::new(MonoType::from(*t))),
            ast::Type::Fn {
                params,
                return_type,
            } => MonoType::Fn {
                params: params.into_iter().map(MonoType::from).collect(),
                return_type: Box::new(MonoType::from(*return_type)),
                is_async: false,
            },
            ast::Type::Option(_t) => MonoType::Enum(EnumType {
                name: "Option".to_string(),
                variants: vec!["Some".to_string(), "None".to_string()],
            }),
            ast::Type::Result(_ok, _err) => MonoType::Enum(EnumType {
                name: "Result".to_string(),
                variants: vec!["Ok".to_string(), "Err".to_string()],
            }),
            ast::Type::Generic { name, args } => {
                // 泛型类型，如 List<T>
                MonoType::TypeRef(format!(
                    "{}<{}>",
                    name,
                    args.iter()
                        .map(|t| MonoType::from(t.clone()).type_name())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            }
            ast::Type::AssocType {
                host_type,
                assoc_name,
                assoc_args,
            } => MonoType::AssocType {
                host_type: Box::new(MonoType::from(*host_type)),
                assoc_name,
                assoc_args: assoc_args.into_iter().map(MonoType::from).collect(),
            },
            // NamedStruct and Sum types (placeholder implementations)
            ast::Type::NamedStruct { name, fields } => MonoType::Struct(StructType {
                name,
                fields: fields
                    .into_iter()
                    .map(|(n, t)| (n, MonoType::from(t)))
                    .collect(),
                methods: HashMap::new(),
            }),
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
        }
    }
}

/// 结构体类型
#[derive(Debug, Clone)]
pub struct StructType {
    pub name: String,
    pub fields: Vec<(String, MonoType)>,
    /// 方法表：方法名 -> 方法类型
    pub methods: HashMap<String, PolyType>,
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

    /// 实例化泛型类型
    pub fn instantiate(
        &self,
        type_args: Vec<MonoType>,
        const_args: Vec<ConstValue>,
    ) -> Result<MonoType, String> {
        if type_args.len() != self.type_binders.len() {
            return Err(format!(
                "Expected {} type arguments, got {}",
                self.type_binders.len(),
                type_args.len()
            ));
        }

        if const_args.len() != self.const_binders.len() {
            return Err(format!(
                "Expected {} const arguments, got {}",
                self.const_binders.len(),
                const_args.len()
            ));
        }

        // TODO: 实现类型替换逻辑
        // 这里应该用 type_args 和 const_args 替换 body 中的对应变量
        Ok(self.body.clone())
    }

    /// 泛型化具体类型
    pub fn generalize(mono_type: &MonoType) -> Self {
        // TODO: 实现类型泛化逻辑
        // 找出 mono_type 中不在环境中的类型变量，添加到泛型变量列表中
        PolyType::mono(mono_type.clone())
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
