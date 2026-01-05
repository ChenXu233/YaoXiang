//! 泛型特化和多态处理
//!
//! 实现多态类型的实例化和特化

#![allow(clippy::result_large_err)]

use super::errors::TypeResult;
use super::types::{MonoType, PolyType, TypeConstraintSolver, TypeVar};
use std::collections::HashMap;
use std::fmt;
use std::hash::Hash;

/// 泛型特化器
///
/// 负责将多态类型特化为具体类型
#[derive(Debug)]
pub struct GenericSpecializer {
    /// 特化缓存：签名 -> 特化后的多态类型
    cache: HashMap<String, HashMap<String, PolyType>>,
    /// 下一个特化 ID
    next_id: usize,
}

impl GenericSpecializer {
    /// 创建新的泛型特化器
    pub fn new() -> Self {
        GenericSpecializer {
            cache: HashMap::new(),
            next_id: 0,
        }
    }

    /// 重置特化器
    pub fn reset(&mut self) {
        self.cache.clear();
        self.next_id = 0;
    }

    /// 特化多态类型
    ///
    /// 将多态类型中的泛型变量替换为具体类型
    pub fn specialize(
        &mut self,
        poly: &PolyType,
        args: &[MonoType],
        solver: &mut TypeConstraintSolver,
    ) -> TypeResult<MonoType> {
        // 检查参数数量是否匹配
        if poly.binders.len() != args.len() {
            return Err(super::errors::TypeError::ArityMismatch {
                expected: poly.binders.len(),
                found: args.len(),
                span: crate::util::span::Span::default(),
            });
        }

        // 构建替换映射
        let substitution: HashMap<_, _> = poly
            .binders
            .iter()
            .zip(args.iter())
            .map(|(var, arg)| (*var, arg.clone()))
            .collect();

        // 替换类型
        let result = self.substitute_type(&poly.body, &substitution, solver);

        Ok(result)
    }

    /// 特化并缓存多态类型
    pub fn specialize_with_cache(
        &mut self,
        poly: &PolyType,
        args: &[MonoType],
        solver: &mut TypeConstraintSolver,
    ) -> TypeResult<MonoType> {
        // 生成签名
        let signature = self.signature(poly, args);

        // 检查缓存
        let cache_key = self.generate_cache_key(poly, args);
        if let Some(cached) = self.cache.get(&signature).and_then(|m| m.get(&cache_key)) {
            return Ok(solver.instantiate(cached));
        }

        // 执行特化
        let result = self.specialize(poly, args, solver)?;

        // 缓存结果
        self.cache
            .entry(signature)
            .or_default()
            .insert(cache_key, PolyType::mono(result.clone()));

        Ok(result)
    }

    /// 替换类型中的变量
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_type(
        &self,
        ty: &MonoType,
        substitution: &HashMap<TypeVar, MonoType>,
        _solver: &mut TypeConstraintSolver,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(v) => {
                if let Some(ty) = substitution.get(v) {
                    ty.clone()
                } else {
                    ty.clone()
                }
            }
            MonoType::Struct(s) => MonoType::Struct(super::types::StructType {
                name: s.name.clone(),
                fields: s
                    .fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.substitute_type(t, substitution, _solver)))
                    .collect(),
            }),
            MonoType::Enum(e) => MonoType::Enum(super::types::EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
            MonoType::Tuple(ts) => MonoType::Tuple(
                ts.iter()
                    .map(|t| self.substitute_type(t, substitution, _solver))
                    .collect(),
            ),
            MonoType::List(t) => {
                MonoType::List(Box::new(self.substitute_type(t, substitution, _solver)))
            }
            MonoType::Dict(k, v) => MonoType::Dict(
                Box::new(self.substitute_type(k, substitution, _solver)),
                Box::new(self.substitute_type(v, substitution, _solver)),
            ),
            MonoType::Set(t) => {
                MonoType::Set(Box::new(self.substitute_type(t, substitution, _solver)))
            }
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params
                    .iter()
                    .map(|t| self.substitute_type(t, substitution, _solver))
                    .collect(),
                return_type: Box::new(self.substitute_type(return_type, substitution, _solver)),
                is_async: *is_async,
            },
            _ => ty.clone(),
        }
    }

    /// 生成泛型签名
    fn signature(
        &self,
        poly: &PolyType,
        args: &[MonoType],
    ) -> String {
        let binders_str = poly
            .binders
            .iter()
            .map(|v| format!("{}", v))
            .collect::<Vec<_>>()
            .join(",");

        let args_str = args
            .iter()
            .map(|t| t.type_name())
            .collect::<Vec<_>>()
            .join(",");

        format!("fn({}) -> ({})", binders_str, args_str)
    }

    /// 生成缓存键
    fn generate_cache_key(
        &self,
        _poly: &PolyType,
        args: &[MonoType],
    ) -> String {
        let args_str = args
            .iter()
            .map(|t| t.type_name())
            .collect::<Vec<_>>()
            .join(",");

        format!("[{}]", args_str)
    }
}

impl Default for GenericSpecializer {
    fn default() -> Self {
        GenericSpecializer::new()
    }
}

// =========================================================================
// 多态类型扩展
// =========================================================================

/// 扩展多态类型操作
impl PolyType {
    /// 检车是否包含泛型变量
    pub fn has_generics(&self) -> bool {
        !self.binders.is_empty()
    }

    /// 获取泛型变量的数量
    pub fn num_generics(&self) -> usize {
        self.binders.len()
    }

    /// 检查类型是否包含特定泛型变量
    pub fn contains_var(
        &self,
        var: &TypeVar,
    ) -> bool {
        self.binders.contains(var)
    }

    /// 替换类型体中的泛型变量
    pub fn substitute(
        &self,
        var: TypeVar,
        ty: MonoType,
    ) -> PolyType {
        let substitution: HashMap<_, _> = vec![(var, ty)].into_iter().collect();
        let body = substitute_mono_type(&self.body, &substitution);
        PolyType::new(self.binders.clone(), body)
    }
}

/// 替换单态类型中的变量
fn substitute_mono_type(
    ty: &MonoType,
    substitution: &HashMap<TypeVar, MonoType>,
) -> MonoType {
    match ty {
        MonoType::TypeVar(v) => {
            if let Some(ty) = substitution.get(v) {
                ty.clone()
            } else {
                ty.clone()
            }
        }
        MonoType::Struct(s) => MonoType::Struct(super::types::StructType {
            name: s.name.clone(),
            fields: s
                .fields
                .iter()
                .map(|(n, t)| (n.clone(), substitute_mono_type(t, substitution)))
                .collect(),
        }),
        MonoType::Enum(e) => MonoType::Enum(super::types::EnumType {
            name: e.name.clone(),
            variants: e.variants.clone(),
        }),
        MonoType::Tuple(ts) => MonoType::Tuple(
            ts.iter()
                .map(|t| substitute_mono_type(t, substitution))
                .collect(),
        ),
        MonoType::List(t) => MonoType::List(Box::new(substitute_mono_type(t, substitution))),
        MonoType::Dict(k, v) => MonoType::Dict(
            Box::new(substitute_mono_type(k, substitution)),
            Box::new(substitute_mono_type(v, substitution)),
        ),
        MonoType::Set(t) => MonoType::Set(Box::new(substitute_mono_type(t, substitution))),
        MonoType::Fn {
            params,
            return_type,
            is_async,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|t| substitute_mono_type(t, substitution))
                .collect(),
            return_type: Box::new(substitute_mono_type(return_type, substitution)),
            is_async: *is_async,
        },
        _ => ty.clone(),
    }
}

// =========================================================================
// 泛型约束
// =========================================================================

/// 泛型约束
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericConstraint {
    /// 类型必须实现特定 trait
    Trait(String),
    /// 类型必须是特定类型的子类型
    SubType(MonoType),
    /// 类型必须是枚举的变体
    EnumVariant {
        enum_name: String,
        variant_name: String,
    },
}

/// 泛型约束求解器
#[derive(Debug, Default)]
pub struct GenericConstraintSolver {
    /// 约束列表
    constraints: Vec<GenericConstraint>,
}

impl GenericConstraintSolver {
    /// 创建新的约束求解器
    pub fn new() -> Self {
        GenericConstraintSolver {
            constraints: Vec::new(),
        }
    }

    /// 添加约束
    pub fn add_constraint(
        &mut self,
        constraint: GenericConstraint,
    ) {
        self.constraints.push(constraint);
    }

    /// 检查约束是否满足
    pub fn check(
        &self,
        _ty: &MonoType,
    ) -> bool {
        // 简化实现：始终返回 true
        // 完整实现需要检查 trait 实现等
        true
    }

    /// 获取所有约束
    pub fn constraints(&self) -> &[GenericConstraint] {
        &self.constraints
    }
}

// =========================================================================
// 特化缓存键
// =========================================================================

/// 特化缓存键生成器
#[derive(Debug)]
pub struct SpecializationKey {
    /// 函数/类型名称
    name: String,
    /// 类型参数
    type_args: Vec<MonoType>,
}

impl SpecializationKey {
    /// 创建新的缓存键
    pub fn new(
        name: String,
        type_args: Vec<MonoType>,
    ) -> Self {
        SpecializationKey { name, type_args }
    }

    /// 生成字符串键
    pub fn as_string(&self) -> String {
        let args_str = self
            .type_args
            .iter()
            .map(|t| t.type_name())
            .collect::<Vec<_>>()
            .join(",");
        format!("{}<{}>", self.name, args_str)
    }
}

impl fmt::Display for SpecializationKey {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

impl PartialEq for SpecializationKey {
    fn eq(
        &self,
        other: &Self,
    ) -> bool {
        self.name == other.name && self.type_args == other.type_args
    }
}

impl Eq for SpecializationKey {}

impl std::hash::Hash for SpecializationKey {
    fn hash<H: std::hash::Hasher>(
        &self,
        state: &mut H,
    ) {
        self.name.hash(state);
        for ty in &self.type_args {
            self.type_name_hash(ty, state);
        }
    }
}

impl SpecializationKey {
    #[allow(clippy::only_used_in_recursion)]
    fn type_name_hash<H: std::hash::Hasher>(
        &self,
        ty: &MonoType,
        state: &mut H,
    ) {
        match ty {
            MonoType::Void => "void".hash(state),
            MonoType::Bool => "bool".hash(state),
            MonoType::Int(n) => format!("int{}", n).hash(state),
            MonoType::Float(n) => format!("float{}", n).hash(state),
            MonoType::Char => "char".hash(state),
            MonoType::String => "string".hash(state),
            MonoType::Bytes => "bytes".hash(state),
            MonoType::Struct(s) => s.name.hash(state),
            MonoType::Enum(e) => e.name.hash(state),
            MonoType::Tuple(ts) => {
                "tuple".hash(state);
                for t in ts {
                    self.type_name_hash(t, state);
                }
            }
            MonoType::List(t) => {
                "list".hash(state);
                self.type_name_hash(t, state);
            }
            MonoType::Dict(k, v) => {
                "dict".hash(state);
                self.type_name_hash(k, state);
                self.type_name_hash(v, state);
            }
            MonoType::Set(t) => {
                "set".hash(state);
                self.type_name_hash(t, state);
            }
            MonoType::Fn { .. } => "fn".hash(state),
            MonoType::TypeVar(v) => format!("var{}", v.index()).hash(state),
            MonoType::TypeRef(n) => n.hash(state),
            MonoType::Range { elem_type } => {
                "range".hash(state);
                self.type_name_hash(elem_type, state);
            }
        }
    }
}
