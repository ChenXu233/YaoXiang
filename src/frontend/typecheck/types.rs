//! 类型变量、类型绑定和 unify 算法
//!
//! 实现 Hindley-Milner 类型系统的核心数据结构：
//! - TypeVar: 类型变量（用于类型推断）
//! - TypeBinding: 类型绑定（union-find 结构）
//! - MonoType: 单态类型（具体类型）
//! - PolyType: 多态类型（带泛型参数）

#![allow(clippy::result_large_err)]

use super::super::parser::ast;
use crate::util::span::Span;
use std::collections::HashMap;
use std::fmt;

/// 类型变量（用于类型推断）
///
/// 每个类型变量有一个唯一的索引，用于在类型环境中追踪
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TypeVar(usize);

impl TypeVar {
    /// 创建新类型变量
    pub fn new(index: usize) -> Self {
        TypeVar(index)
    }

    /// 获取变量的索引
    pub fn index(&self) -> usize {
        self.0
    }
}

impl fmt::Display for TypeVar {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
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

impl MonoType {
    /// 如果是 TypeVar 变体，返回内部 TypeVar
    pub fn type_var(&self) -> Option<TypeVar> {
        match self {
            MonoType::TypeVar(v) => Some(*v),
            _ => None,
        }
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
            // NamedStruct and Sum types (placeholder implementations)
            ast::Type::NamedStruct { name, fields } => MonoType::Struct(StructType {
                name,
                fields: fields
                    .into_iter()
                    .map(|(n, t)| (n, MonoType::from(t)))
                    .collect(),
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StructType {
    pub name: String,
    pub fields: Vec<(String, MonoType)>,
}

/// 枚举类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumType {
    pub name: String,
    pub variants: Vec<String>,
}

/// 多态类型（带泛型参数）
///
/// 包含泛型变量列表和类型体，用于表示泛型函数或泛型类型的签名
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyType {
    /// 泛型变量列表（按顺序）
    pub binders: Vec<TypeVar>,
    /// 类型体
    pub body: MonoType,
}

impl PolyType {
    /// 创建新的多态类型
    pub fn new(
        binders: Vec<TypeVar>,
        body: MonoType,
    ) -> Self {
        PolyType { binders, body }
    }

    /// 创建无泛型的多态类型
    pub fn mono(body: MonoType) -> Self {
        PolyType {
            binders: Vec::new(),
            body,
        }
    }
}

/// 类型约束
///
/// 在类型推断过程中收集的约束条件
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    /// 约束的左边
    pub left: MonoType,
    /// 约束的右边
    pub right: MonoType,
    /// 约束的来源位置
    pub span: Span,
}

impl TypeConstraint {
    /// 创建新的类型约束
    pub fn new(
        left: MonoType,
        right: MonoType,
        span: Span,
    ) -> Self {
        TypeConstraint { left, right, span }
    }
}

// =========================================================================
// Send/Sync 约束
// =========================================================================

/// Send/Sync 约束
///
/// 用于标记类型变量必须满足的 Send/Sync 约束：
/// - Send: 类型可以安全地跨线程传输
/// - Sync: 类型可以安全地跨线程共享
///
/// 根据 RFC-009：
/// - 值类型默认 Send + Sync
/// - ref T (Arc) 默认 Send + Sync
/// - *T (裸指针) 既不是 Send 也不是 Sync
/// - Rc[T] 既不是 Send 也不是 Sync
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct SendSyncConstraint {
    /// 是否必须 Send
    pub require_send: bool,
    /// 是否必须 Sync
    pub require_sync: bool,
}

impl SendSyncConstraint {
    /// 创建新的约束
    pub fn new(
        require_send: bool,
        require_sync: bool,
    ) -> Self {
        Self {
            require_send,
            require_sync,
        }
    }

    /// 只有 Send 约束
    pub fn send_only() -> Self {
        Self {
            require_send: true,
            require_sync: false,
        }
    }

    /// Send + Sync 约束
    pub fn send_sync() -> Self {
        Self {
            require_send: true,
            require_sync: true,
        }
    }

    /// 只有 Sync 约束
    pub fn sync_only() -> Self {
        Self {
            require_send: false,
            require_sync: true,
        }
    }

    /// 无约束
    pub fn none() -> Self {
        Self::default()
    }

    /// 合并两个约束（取并集）
    pub fn merge(
        &self,
        other: &Self,
    ) -> Self {
        Self {
            require_send: self.require_send || other.require_send,
            require_sync: self.require_sync || other.require_sync,
        }
    }

    /// 检查是否满足约束
    pub fn is_satisfied(
        &self,
        is_send: bool,
        is_sync: bool,
    ) -> bool {
        (!self.require_send || is_send) && (!self.require_sync || is_sync)
    }
}

/// Send/Sync 约束求解器
///
/// 负责管理类型变量的 Send/Sync 约束收集和求解
#[derive(Debug, Default)]
pub struct SendSyncConstraintSolver {
    /// 类型变量的 Send/Sync 约束
    constraints: HashMap<TypeVar, SendSyncConstraint>,
}

impl SendSyncConstraintSolver {
    /// 创建新的求解器
    pub fn new() -> Self {
        Self {
            constraints: HashMap::new(),
        }
    }

    /// 重置求解器
    pub fn reset(&mut self) {
        self.constraints.clear();
    }

    /// 添加 Send 约束
    ///
    /// 如果类型是泛型类型，约束会传播到其类型参数
    pub fn add_send_constraint(
        &mut self,
        ty: &MonoType,
    ) {
        self.add_constraint(ty, true, false);
    }

    /// 添加 Sync 约束
    ///
    /// 如果类型是泛型类型，约束会传播到其类型参数
    pub fn add_sync_constraint(
        &mut self,
        ty: &MonoType,
    ) {
        self.add_constraint(ty, false, true);
    }

    /// 添加 Send + Sync 约束
    pub fn add_send_sync_constraint(
        &mut self,
        ty: &MonoType,
    ) {
        self.add_constraint(ty, true, true);
    }

    /// 添加约束
    fn add_constraint(
        &mut self,
        ty: &MonoType,
        require_send: bool,
        require_sync: bool,
    ) {
        match ty {
            MonoType::TypeVar(v) => {
                let existing = self.constraints.entry(*v).or_default();
                if require_send {
                    existing.require_send = true;
                }
                if require_sync {
                    existing.require_sync = true;
                }
            }
            MonoType::Struct(s) => {
                // 结构体：约束传播到所有字段类型
                for (_, field_ty) in &s.fields {
                    self.add_constraint(field_ty, require_send, require_sync);
                }
            }
            MonoType::Tuple(types) => {
                // 元组：约束传播到所有元素类型
                for elem_ty in types {
                    self.add_constraint(elem_ty, require_send, require_sync);
                }
            }
            MonoType::List(elem) | MonoType::Set(elem) | MonoType::Range { elem_type: elem } => {
                // 列表/集合/Range：约束传播到元素类型
                self.add_constraint(elem, require_send, require_sync);
            }
            MonoType::Dict(key, value) => {
                // 字典：约束传播到键和值类型
                self.add_constraint(key, require_send, require_sync);
                self.add_constraint(value, require_send, require_sync);
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                // 函数类型：约束传播到参数和返回类型
                for param_ty in params {
                    self.add_constraint(param_ty, require_send, require_sync);
                }
                self.add_constraint(return_type, require_send, require_sync);
            }
            MonoType::Union(types) | MonoType::Intersection(types) => {
                // 联合/交集：约束传播到所有成员类型
                for member_ty in types {
                    self.add_constraint(member_ty, require_send, require_sync);
                }
            }
            MonoType::Arc(inner) => {
                // Arc 内部类型需要满足约束（因为 Arc 可以跨线程共享）
                self.add_constraint(inner, require_send, require_sync);
            }
            // 基本类型、类型引用等不需要额外处理
            // 它们默认满足 Send/Sync
            _ => {}
        }
    }

    /// 获取类型的 Send/Sync 约束
    pub fn get_constraint(
        &self,
        ty: &MonoType,
    ) -> SendSyncConstraint {
        match ty {
            MonoType::TypeVar(v) => self.constraints.get(v).cloned().unwrap_or_default(),
            _ => SendSyncConstraint::none(),
        }
    }

    /// 检查类型是否满足 Send 约束
    pub fn is_send(
        &self,
        ty: &MonoType,
    ) -> bool {
        let constraint = self.get_constraint(ty);
        constraint.require_send || self.is_type_inherently_send(ty)
    }

    /// 检查类型是否满足 Sync 约束
    pub fn is_sync(
        &self,
        ty: &MonoType,
    ) -> bool {
        let constraint = self.get_constraint(ty);
        constraint.require_sync || self.is_type_inherently_sync(ty)
    }

    /// 检查类型是否固有地满足 Send（RFC-009）
    ///
    /// 根据 RFC-009：
    /// - 值类型（Int, Float, Bool, Char, String, Bytes）默认 Send
    /// - ref T (Arc) 默认 Send
    /// - Arc[T] 默认 Send
    /// - Rc[T] 不是 Send
    /// - *T 不是 Send
    fn is_type_inherently_send(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            // 基本类型默认 Send
            MonoType::Void
            | MonoType::Bool
            | MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes => true,
            // Arc 默认 Send
            MonoType::Arc(_) => true,
            // 枚举默认 Send（只是标签）
            MonoType::Enum(_) => true,
            // 联合/交集类型需要所有成员 Send
            MonoType::Union(types) | MonoType::Intersection(types) => {
                types.iter().all(|t| self.is_send(t))
            }
            // 结构体需要所有字段 Send
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.is_send(f)),
            // 元组需要所有元素 Send
            MonoType::Tuple(types) => types.iter().all(|t| self.is_send(t)),
            // 列表/集合/Range 需要元素 Send
            MonoType::List(elem) | MonoType::Set(elem) | MonoType::Range { elem_type: elem } => {
                self.is_send(elem)
            }
            // 字典需要键和值 Send
            MonoType::Dict(k, v) => self.is_send(k) && self.is_send(v),
            // 函数类型需要参数和返回类型 Send
            MonoType::Fn {
                params,
                return_type,
                ..
            } => params.iter().all(|p| self.is_send(p)) && self.is_send(return_type),
            // Rc 不是 Send（无法跨线程安全共享引用计数）
            MonoType::TypeRef(name) if name.starts_with("Rc") => false,
            // 其他类型引用保守假设为 Send
            MonoType::TypeRef(_) => true,
            // 类型变量需要根据约束判断
            MonoType::TypeVar(_) => true, // 保守假设
        }
    }

    /// 检查类型是否固有地满足 Sync（RFC-009）
    fn is_type_inherently_sync(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            // 基本类型默认 Sync
            MonoType::Void
            | MonoType::Bool
            | MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes => true,
            // Arc 默认 Sync（安全共享）
            MonoType::Arc(_) => true,
            // 枚举默认 Sync
            MonoType::Enum(_) => true,
            // 联合/交集需要所有成员 Sync
            MonoType::Union(types) | MonoType::Intersection(types) => {
                types.iter().all(|t| self.is_sync(t))
            }
            // 结构体需要所有字段 Sync
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.is_sync(f)),
            // 元组需要所有元素 Sync
            MonoType::Tuple(types) => types.iter().all(|t| self.is_sync(t)),
            // 列表/集合/Range 需要元素 Sync
            MonoType::List(elem) | MonoType::Set(elem) | MonoType::Range { elem_type: elem } => {
                self.is_sync(elem)
            }
            // 字典需要键和值 Sync
            MonoType::Dict(k, v) => self.is_sync(k) && self.is_sync(v),
            // 函数类型通常不用于共享
            MonoType::Fn { .. } => false,
            // Rc 不是 Sync
            MonoType::TypeRef(name) if name.starts_with("Rc") => false,
            // 其他类型引用保守假设为 Sync
            MonoType::TypeRef(_) => true,
            // 类型变量保守假设为 Sync
            MonoType::TypeVar(_) => true,
        }
    }

    /// 获取所有约束
    pub fn constraints(&self) -> &HashMap<TypeVar, SendSyncConstraint> {
        &self.constraints
    }
}

/// 类型约束求解器（union-find 实现）
///
/// 负责管理类型变量的绑定和约束求解
#[derive(Debug, Default)]
pub struct TypeConstraintSolver {
    /// 类型变量的绑定状态
    bindings: Vec<TypeBinding>,
    /// 收集的约束
    constraints: Vec<TypeConstraint>,
    /// 下一个类型变量的索引
    next_var: usize,
    /// 泛型变量集合（不应被实例化）
    generic_vars: HashMap<TypeVar, usize>,
}

impl TypeConstraintSolver {
    /// 创建新的求解器
    pub fn new() -> Self {
        TypeConstraintSolver {
            bindings: Vec::new(),
            constraints: Vec::new(),
            next_var: 0,
            generic_vars: HashMap::new(),
        }
    }

    /// 重置求解器
    pub fn reset(&mut self) {
        self.bindings.clear();
        self.constraints.clear();
        self.next_var = 0;
        self.generic_vars.clear();
    }

    /// 创建新的类型变量
    pub fn new_var(&mut self) -> MonoType {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        self.bindings.push(TypeBinding::Unbound);
        MonoType::TypeVar(var)
    }

    /// 创建新的泛型变量
    pub fn new_generic_var(&mut self) -> TypeVar {
        let var = TypeVar(self.next_var);
        self.next_var += 1;
        self.bindings.push(TypeBinding::Unbound);
        self.generic_vars.insert(var, self.generic_vars.len());
        var
    }

    /// 查找类型变量的最终绑定（路径压缩）
    pub fn find(
        &mut self,
        var: TypeVar,
    ) -> TypeVar {
        match self.bindings.get(var.0) {
            Some(TypeBinding::Link(next)) => {
                let root = self.find(*next);
                // 路径压缩
                if let Some(binding) = self.bindings.get_mut(var.0) {
                    *binding = TypeBinding::Link(root);
                }
                root
            }
            Some(TypeBinding::Bound(_)) => var,
            Some(TypeBinding::Unbound) => var,
            None => var,
        }
    }

    /// 获取类型变量的当前绑定
    pub fn get_binding(
        &self,
        var: TypeVar,
    ) -> Option<&TypeBinding> {
        self.bindings.get(var.0)
    }

    /// 获取类型变量的当前绑定（可变）
    pub fn get_binding_mut(
        &mut self,
        var: TypeVar,
    ) -> Option<&mut TypeBinding> {
        self.bindings.get_mut(var.0)
    }

    /// 绑定类型变量到类型
    ///
    /// 如果类型变量已经绑定，会尝试合并
    pub fn bind(
        &mut self,
        var: TypeVar,
        ty: &MonoType,
    ) -> Result<(), TypeMismatch> {
        let resolved_var = self.find(var);

        // 检查是否产生无限类型
        if let MonoType::TypeVar(tv) = ty {
            if self.find(*tv) == resolved_var {
                return Err(TypeMismatch {
                    left: MonoType::TypeVar(resolved_var),
                    right: ty.clone(),
                    span: Span::default(),
                });
            }
        }

        // 展开类型变量链
        let ty = self.expand_type(ty);

        // 绑定
        if let Some(binding) = self.bindings.get_mut(resolved_var.0) {
            match binding {
                TypeBinding::Unbound => {
                    *binding = TypeBinding::Bound(ty);
                    Ok(())
                }
                TypeBinding::Bound(existing) => {
                    // 已绑定，检查是否一致
                    if existing == &ty {
                        Ok(())
                    } else {
                        Err(TypeMismatch {
                            left: (*existing).clone(),
                            right: ty,
                            span: Span::default(),
                        })
                    }
                }
                TypeBinding::Link(_) => {
                    // 不应该到这里
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }

    /// 展开类型变量，获取具体类型
    fn expand_type(
        &self,
        ty: &MonoType,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(v) => {
                if let Some(TypeBinding::Bound(bound_ty)) = self.bindings.get(v.0) {
                    self.expand_type(bound_ty)
                } else {
                    ty.clone()
                }
            }
            MonoType::Struct(s) => MonoType::Struct(StructType {
                name: s.name.clone(),
                fields: s
                    .fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.expand_type(t)))
                    .collect(),
            }),
            MonoType::Enum(e) => MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
            MonoType::Tuple(ts) => {
                MonoType::Tuple(ts.iter().map(|t| self.expand_type(t)).collect())
            }
            MonoType::List(t) => MonoType::List(Box::new(self.expand_type(t))),
            MonoType::Dict(k, v) => {
                MonoType::Dict(Box::new(self.expand_type(k)), Box::new(self.expand_type(v)))
            }
            MonoType::Set(t) => MonoType::Set(Box::new(self.expand_type(t))),
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params.iter().map(|t| self.expand_type(t)).collect(),
                return_type: Box::new(self.expand_type(return_type)),
                is_async: *is_async,
            },
            // 联合类型展开
            MonoType::Union(types) => {
                MonoType::Union(types.iter().map(|t| self.expand_type(t)).collect())
            }
            // 交集类型展开
            MonoType::Intersection(types) => {
                MonoType::Intersection(types.iter().map(|t| self.expand_type(t)).collect())
            }
            _ => ty.clone(),
        }
    }

    /// 添加类型约束
    pub fn add_constraint(
        &mut self,
        left: MonoType,
        right: MonoType,
        span: Span,
    ) {
        self.constraints
            .push(TypeConstraint::new(left, right, span));
    }

    /// 求解所有约束
    ///
    /// 返回求解后的类型环境状态
    pub fn solve(&mut self) -> Result<(), Vec<TypeConstraintError>> {
        let mut errors = Vec::new();

        // 逐一求解约束
        for constraint in std::mem::take(&mut self.constraints) {
            if let Err(e) = self.unify(&constraint.left, &constraint.right) {
                errors.push(TypeConstraintError {
                    error: e,
                    span: constraint.span,
                });
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// 解析类型，展开所有类型变量
    pub fn resolve_type(
        &self,
        ty: &MonoType,
    ) -> MonoType {
        self.expand_type(ty)
    }

    /// Unify 两个类型
    ///
    /// 尝试将两个类型统一，返回约束或错误
    pub fn unify(
        &mut self,
        t1: &MonoType,
        t2: &MonoType,
    ) -> Result<(), TypeMismatch> {
        let t1 = self.expand_type(t1);
        let t2 = self.expand_type(t2);

        match (&t1, &t2) {
            // 类型变量 unify
            (MonoType::TypeVar(v1), MonoType::TypeVar(v2)) => {
                let v1 = self.find(*v1);
                let v2 = self.find(*v2);
                if v1 == v2 {
                    Ok(())
                } else {
                    // 建立链接
                    self.bind(v1, &MonoType::TypeVar(v2))
                }
            }
            (MonoType::TypeVar(v), _) => self.bind(*v, &t2),
            (_, MonoType::TypeVar(v)) => self.bind(*v, &t1),

            // 具体类型 unify
            (MonoType::Void, MonoType::Void) => Ok(()),
            (MonoType::Bool, MonoType::Bool) => Ok(()),
            (MonoType::Int(n1), MonoType::Int(n2)) if n1 == n2 => Ok(()),
            (MonoType::Float(n1), MonoType::Float(n2)) if n1 == n2 => Ok(()),
            (MonoType::Char, MonoType::Char) => Ok(()),
            (MonoType::String, MonoType::String) => Ok(()),
            (MonoType::Bytes, MonoType::Bytes) => Ok(()),

            // 函数类型 unify
            (
                MonoType::Fn {
                    params: p1,
                    return_type: r1,
                    is_async: a1,
                },
                MonoType::Fn {
                    params: p2,
                    return_type: r2,
                    is_async: a2,
                },
            ) => {
                if p1.len() != p2.len() || a1 != a2 {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                // unify 参数和返回类型
                for (p1, p2) in p1.iter().zip(p2.iter()) {
                    self.unify(p1, p2)?;
                }
                self.unify(r1, r2)?;
                Ok(())
            }

            // 结构体类型 unify
            (MonoType::Struct(s1), MonoType::Struct(s2)) => {
                if s1.fields.len() != s2.fields.len() {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                for ((_, f1), (_, f2)) in s1.fields.iter().zip(s2.fields.iter()) {
                    self.unify(f1, f2)?;
                }
                Ok(())
            }

            // 枚举类型 unify
            (MonoType::Enum(e1), MonoType::Enum(e2)) => {
                if e1.variants.len() != e2.variants.len() {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                Ok(())
            }

            // 元组类型 unify
            (MonoType::Tuple(ts1), MonoType::Tuple(ts2)) => {
                if ts1.len() != ts2.len() {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                for (t1, t2) in ts1.iter().zip(ts2.iter()) {
                    self.unify(t1, t2)?;
                }
                Ok(())
            }

            // 列表类型 unify
            (MonoType::List(t1), MonoType::List(t2)) => self.unify(t1, t2),

            // 字典类型 unify
            (MonoType::Dict(k1, v1), MonoType::Dict(k2, v2)) => {
                self.unify(k1, k2)?;
                self.unify(v1, v2)?;
                Ok(())
            }

            // 集合类型 unify
            (MonoType::Set(t1), MonoType::Set(t2)) => self.unify(t1, t2),

            // 类型引用 unify（仅比较名称）
            (MonoType::TypeRef(n1), MonoType::TypeRef(n2)) if n1 == n2 => Ok(()),

            // 联合类型 unify：T1 | T2 == T3 分解为 (T1 == T3) | (T2 == T3)
            // 即：检查 T3 是否是联合类型的超类型，或者 T3 是否兼容联合的每个成员
            (MonoType::Union(types1), MonoType::Union(types2)) => {
                // 联合类型 == 联合类型：检查是否集合相等
                // 简化处理：元素数量相同且一一兼容
                if types1.len() != types2.len() {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                for (t1, t2) in types1.iter().zip(types2.iter()) {
                    self.unify(t1, t2)?;
                }
                Ok(())
            }
            (MonoType::Union(types), other) | (other, MonoType::Union(types)) => {
                // 联合类型 == 具体类型：检查具体类型是否是联合的成员之一
                // 或者尝试将具体类型与每个成员统一
                let mut unified = false;
                for member in types {
                    if self.unify(member, other).is_ok() {
                        unified = true;
                        break;
                    }
                }
                if !unified {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                Ok(())
            }

            // 交集类型 unify：T1 & T2 == T3 分解为 (T1 == T3) & (T2 == T3)
            // 即：检查 T3 是否同时满足 T1 和 T2 的约束
            (MonoType::Intersection(types1), MonoType::Intersection(types2)) => {
                // 交集类型 == 交集类型：需要两个类型的成员都兼容
                // 简化处理：元素数量相同且一一兼容
                if types1.len() != types2.len() {
                    return Err(TypeMismatch {
                        left: t1,
                        right: t2,
                        span: Span::default(),
                    });
                }
                for (t1, t2) in types1.iter().zip(types2.iter()) {
                    self.unify(t1, t2)?;
                }
                Ok(())
            }
            (MonoType::Intersection(types), other) | (other, MonoType::Intersection(types)) => {
                // 交集类型 == 具体类型：检查具体类型是否与所有成员兼容
                for member in types {
                    self.unify(member, other)?;
                }
                Ok(())
            }

            // 不兼容类型
            _ => Err(TypeMismatch {
                left: t1,
                right: t2,
                span: Span::default(),
            }),
        }
    }

    /// 实例化多态类型
    ///
    /// 将多态类型中的泛型变量替换为新类型变量
    pub fn instantiate(
        &mut self,
        poly: &PolyType,
    ) -> MonoType {
        let substitution: HashMap<_, _> = poly
            .binders
            .iter()
            .map(|var| (*var, self.new_var()))
            .collect();

        self.substitute_type(&poly.body, &substitution)
    }

    /// 替换类型中的变量
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_type(
        &self,
        ty: &MonoType,
        substitution: &HashMap<TypeVar, MonoType>,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(v) => {
                if let Some(new_ty) = substitution.get(v) {
                    new_ty.clone()
                } else {
                    ty.clone()
                }
            }
            MonoType::Struct(s) => MonoType::Struct(StructType {
                name: s.name.clone(),
                fields: s
                    .fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.substitute_type(t, substitution)))
                    .collect(),
            }),
            MonoType::Enum(e) => MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
            MonoType::Tuple(ts) => MonoType::Tuple(
                ts.iter()
                    .map(|t| self.substitute_type(t, substitution))
                    .collect(),
            ),
            MonoType::List(t) => MonoType::List(Box::new(self.substitute_type(t, substitution))),
            MonoType::Dict(k, v) => MonoType::Dict(
                Box::new(self.substitute_type(k, substitution)),
                Box::new(self.substitute_type(v, substitution)),
            ),
            MonoType::Set(t) => MonoType::Set(Box::new(self.substitute_type(t, substitution))),
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params
                    .iter()
                    .map(|t| self.substitute_type(t, substitution))
                    .collect(),
                return_type: Box::new(self.substitute_type(return_type, substitution)),
                is_async: *is_async,
            },
            // 联合类型替换
            MonoType::Union(types) => MonoType::Union(
                types
                    .iter()
                    .map(|t| self.substitute_type(t, substitution))
                    .collect(),
            ),
            // 交集类型替换
            MonoType::Intersection(types) => MonoType::Intersection(
                types
                    .iter()
                    .map(|t| self.substitute_type(t, substitution))
                    .collect(),
            ),
            _ => ty.clone(),
        }
    }

    /// 泛化类型
    ///
    /// 将单态类型中的自由变量提取为泛型变量
    pub fn generalize(
        &self,
        ty: &MonoType,
    ) -> PolyType {
        let free_vars = self.free_variables(ty);
        PolyType::new(free_vars, ty.clone())
    }

    /// 获取类型中的自由变量
    fn free_variables(
        &self,
        ty: &MonoType,
    ) -> Vec<TypeVar> {
        let mut free = Vec::new();
        self.collect_free_vars(ty, &mut free);
        // 去重
        free.sort_by_key(|v| v.index());
        free.dedup_by_key(|v| v.index());
        free
    }

    fn collect_free_vars(
        &self,
        ty: &MonoType,
        free: &mut Vec<TypeVar>,
    ) {
        match ty {
            MonoType::TypeVar(v) => {
                if !self.generic_vars.contains_key(v) {
                    free.push(*v);
                }
            }
            MonoType::Struct(s) => {
                for (_, t) in &s.fields {
                    self.collect_free_vars(t, free);
                }
            }
            MonoType::Tuple(ts) => {
                for t in ts {
                    self.collect_free_vars(t, free);
                }
            }
            MonoType::List(t) => self.collect_free_vars(t, free),
            MonoType::Dict(k, v) => {
                self.collect_free_vars(k, free);
                self.collect_free_vars(v, free);
            }
            MonoType::Set(t) => self.collect_free_vars(t, free),
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                for p in params {
                    self.collect_free_vars(p, free);
                }
                self.collect_free_vars(return_type, free);
            }
            // 联合类型收集自由变量
            MonoType::Union(types) | MonoType::Intersection(types) => {
                for t in types {
                    self.collect_free_vars(t, free);
                }
            }
            _ => {}
        }
    }

    /// 获取求解器状态（用于调试）
    pub fn state(&self) -> String {
        let mut state = String::new();
        for (i, binding) in self.bindings.iter().enumerate() {
            match binding {
                TypeBinding::Unbound => {}
                TypeBinding::Bound(ty) => {
                    state.push_str(&format!("t{} = {}\n", i, ty.type_name()));
                }
                TypeBinding::Link(v) => {
                    state.push_str(&format!("t{} -> t{}\n", i, v.index()));
                }
            }
        }
        state
    }

    /// 检查类型变量是否未约束
    ///
    /// 如果类型变量仍然是 Unbound 状态，返回 true
    pub fn is_unconstrained(
        &self,
        var: TypeVar,
    ) -> bool {
        if let Some(binding) = self.bindings.get(var.0) {
            matches!(binding, TypeBinding::Unbound)
        } else {
            false
        }
    }

    /// 检查类型变量是否出现在任何未求解的约束中
    ///
    /// 如果类型变量出现在约束中，说明它被使用了
    pub fn appears_in_constraints(
        &self,
        var: TypeVar,
    ) -> bool {
        self.constraints.iter().any(|c| {
            Self::type_contains_var(&c.left, var) || Self::type_contains_var(&c.right, var)
        })
    }

    /// 检查类型是否包含指定的类型变量
    fn type_contains_var(
        ty: &MonoType,
        var: TypeVar,
    ) -> bool {
        match ty {
            MonoType::TypeVar(v) => *v == var,
            MonoType::Tuple(types) => types.iter().any(|t| Self::type_contains_var(t, var)),
            MonoType::List(t) => Self::type_contains_var(t, var),
            MonoType::Dict(k, v) => {
                Self::type_contains_var(k, var) || Self::type_contains_var(v, var)
            }
            MonoType::Set(t) => Self::type_contains_var(t, var),
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                params.iter().any(|p| Self::type_contains_var(p, var))
                    || Self::type_contains_var(return_type, var)
            }
            // 联合/交集类型检查
            MonoType::Union(types) | MonoType::Intersection(types) => {
                types.iter().any(|t| Self::type_contains_var(t, var))
            }
            _ => false,
        }
    }
}

/// 类型不匹配错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeMismatch {
    pub left: MonoType,
    pub right: MonoType,
    pub span: Span,
}

impl fmt::Display for TypeMismatch {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(
            f,
            "expected {}, found {}",
            self.left.type_name(),
            self.right.type_name()
        )
    }
}

/// 约束求解错误
#[derive(Debug, Clone)]
pub struct TypeConstraintError {
    pub error: TypeMismatch,
    pub span: Span,
}

impl fmt::Display for TypeConstraintError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{} at {:?}", self.error, self.span)
    }
}
