//! 统一的类型替换模块
//!
//! 提供通用的类型替换算法，消除各模块间的重复代码。

use crate::frontend::core::types::{MonoType, TypeVar, StructType, EnumType};
use std::collections::HashMap;

/// 类型替换映射（使用类型变量索引）
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Substitution {
    bindings: HashMap<usize, MonoType>,
}

impl Substitution {
    /// 创建新的替换
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// 添加绑定（使用 TypeVar）
    pub fn bind(
        &mut self,
        tv: TypeVar,
        ty: MonoType,
    ) {
        self.bindings.insert(tv.index(), ty);
    }

    /// 添加绑定（使用索引）
    pub fn insert(
        &mut self,
        index: usize,
        ty: MonoType,
    ) {
        self.bindings.insert(index, ty);
    }

    /// 获取绑定
    pub fn get(
        &self,
        index: &usize,
    ) -> Option<&MonoType> {
        self.bindings.get(index)
    }

    /// 检查是否包含变量
    pub fn contains_var(
        &self,
        index: &usize,
    ) -> bool {
        self.bindings.contains_key(index)
    }

    /// 合并替换
    pub fn merge(
        &self,
        other: &Substitution,
    ) -> Substitution {
        let mut result = self.clone();
        for (k, v) in &other.bindings {
            result.bindings.insert(*k, v.clone());
        }
        result
    }

    /// 获取所有绑定的变量索引
    pub fn bound_vars(&self) -> Vec<usize> {
        self.bindings.keys().cloned().collect()
    }

    /// 获取绑定数量
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    /// 检查是否为空
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

/// 类型替换器
#[derive(Debug, Clone, Default)]
pub struct Substituter;

impl Substituter {
    /// 创建新的替换器
    pub fn new() -> Self {
        Self
    }

    /// 替换单个类型变量
    pub fn substitute_var(
        &self,
        ty: &MonoType,
        var: &TypeVar,
        replacement: &MonoType,
    ) -> MonoType {
        let mut lookup = |tv: &TypeVar| {
            if tv == var {
                Some(replacement.clone())
            } else {
                None
            }
        };
        self.substitute_internal(ty, &mut lookup)
    }

    /// 批量替换（使用 Substitution）
    pub fn substitute(
        &self,
        ty: &MonoType,
        sub: &Substitution,
    ) -> MonoType {
        let mut lookup = |tv: &TypeVar| sub.get(&tv.index()).cloned();
        self.substitute_internal(ty, &mut lookup)
    }

    /// 批量替换（使用 HashMap<usize, MonoType>）
    pub fn substitute_with_map(
        &self,
        ty: &MonoType,
        substitutions: &HashMap<usize, MonoType>,
    ) -> MonoType {
        let mut lookup = |tv: &TypeVar| substitutions.get(&tv.index()).cloned();
        self.substitute_internal(ty, &mut lookup)
    }

    /// 泛型参数替换（按索引顺序）
    pub fn substitute_generic_params(
        &self,
        ty: &MonoType,
        args: &[MonoType],
    ) -> MonoType {
        let mut lookup = |tv: &TypeVar| {
            let index = tv.index();
            if index < args.len() {
                Some(args[index].clone())
            } else {
                None
            }
        };
        self.substitute_internal(ty, &mut lookup)
    }

    /// 内部实现：使用闭包进行替换
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_internal<F>(
        &self,
        ty: &MonoType,
        lookup: &mut F,
    ) -> MonoType
    where
        F: FnMut(&TypeVar) -> Option<MonoType>,
    {
        match ty {
            MonoType::TypeVar(tv) => {
                if let Some(replacement) = lookup(tv) {
                    replacement
                } else {
                    ty.clone()
                }
            }
            MonoType::List(inner) => {
                MonoType::List(Box::new(self.substitute_internal(inner, lookup)))
            }
            MonoType::Tuple(types) => MonoType::Tuple(
                types
                    .iter()
                    .map(|t| self.substitute_internal(t, lookup))
                    .collect(),
            ),
            MonoType::Dict(k, v) => MonoType::Dict(
                Box::new(self.substitute_internal(k, lookup)),
                Box::new(self.substitute_internal(v, lookup)),
            ),
            MonoType::Set(t) => MonoType::Set(Box::new(self.substitute_internal(t, lookup))),
            MonoType::Fn {
                params,
                return_type,
            } => {
                let new_params = params
                    .iter()
                    .map(|p| self.substitute_internal(p, lookup))
                    .collect();
                let new_return_type = Box::new(self.substitute_internal(return_type, lookup));
                MonoType::Fn {
                    params: new_params,
                    return_type: new_return_type,
                }
            }
            MonoType::Struct(struct_type) => {
                let new_fields = struct_type
                    .fields
                    .iter()
                    .map(|(name, field_ty)| {
                        (name.clone(), self.substitute_internal(field_ty, lookup))
                    })
                    .collect();
                MonoType::Struct(StructType {
                    name: struct_type.name.clone(),
                    fields: new_fields,
                    methods: struct_type.methods.clone(),
                    field_mutability: struct_type.field_mutability.clone(),
                    field_has_default: struct_type.field_has_default.clone(),
                    interfaces: struct_type.interfaces.clone(),
                    constraints: struct_type.constraints.clone(),
                })
            }
            MonoType::Enum(e) => MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
            MonoType::Range { elem_type } => MonoType::Range {
                elem_type: Box::new(self.substitute_internal(elem_type, lookup)),
            },
            MonoType::Union(types) => MonoType::Union(
                types
                    .iter()
                    .map(|t| self.substitute_internal(t, lookup))
                    .collect(),
            ),
            MonoType::Intersection(types) => MonoType::Intersection(
                types
                    .iter()
                    .map(|t| self.substitute_internal(t, lookup))
                    .collect(),
            ),
            MonoType::Arc(t) => MonoType::Arc(Box::new(self.substitute_internal(t, lookup))),
            MonoType::Weak(t) => MonoType::Weak(Box::new(self.substitute_internal(t, lookup))),
            MonoType::AssocType {
                host_type,
                assoc_name,
                assoc_args,
            } => MonoType::AssocType {
                host_type: Box::new(self.substitute_internal(host_type, lookup)),
                assoc_name: assoc_name.clone(),
                assoc_args: assoc_args
                    .iter()
                    .map(|t| self.substitute_internal(t, lookup))
                    .collect(),
            },
            _ => ty.clone(),
        }
    }
}

/// 检查类型是否包含类型变量
#[allow(clippy::only_used_in_recursion)]
pub fn contains_type_vars(ty: &MonoType) -> bool {
    match ty {
        MonoType::TypeVar(_) => true,
        MonoType::List(inner) => contains_type_vars(inner),
        MonoType::Tuple(types) => types.iter().any(contains_type_vars),
        MonoType::Dict(k, v) => contains_type_vars(k) || contains_type_vars(v),
        MonoType::Set(t) => contains_type_vars(t),
        MonoType::Fn {
            params,
            return_type,
            ..
        } => params.iter().any(contains_type_vars) || contains_type_vars(return_type),
        MonoType::Struct(struct_type) => struct_type
            .fields
            .iter()
            .any(|(_, field_ty)| contains_type_vars(field_ty)),
        MonoType::Range { elem_type } => contains_type_vars(elem_type),
        MonoType::Union(types) | MonoType::Intersection(types) => {
            types.iter().any(contains_type_vars)
        }
        MonoType::Arc(t) => contains_type_vars(t),
        MonoType::Weak(t) => contains_type_vars(t),
        MonoType::AssocType {
            host_type,
            assoc_args,
            ..
        } => contains_type_vars(host_type) || assoc_args.iter().any(contains_type_vars),
        _ => false,
    }
}

// ========== RFC-010: Universe Level Tests ==========
