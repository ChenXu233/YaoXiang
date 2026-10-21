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

/// 应用头部的构造器名：`TypeRef("Vec")` / `Generic{"Vec", []}` 都表示构造器 `Vec`。
///
/// 用于「构造器变量只能出现在应用头部」这条规则的绑定与替换（RFC-010 元类型
/// 层级：Type1/Type2+ 的构造器在类型实参位以**名字**传递）。
pub fn ctor_head_name(ty: &MonoType) -> Option<String> {
    match ty {
        MonoType::TypeRef(name) => Some(name.clone()),
        MonoType::Generic { name, args } if args.is_empty() => Some(name.clone()),
        _ => None,
    }
}

/// 构造器变量在**应用头部**的替换：把 `F(A)` 的头部名按 `head_map`
///（构造器参数名 → 实参构造器名）改写为真实构造器。
///
/// 共享实现在此（复审轮 1 F2 校正：此前 `typecheck/environment.rs` 与
/// `typecheck/inference/expressions.rs` 各有一份内联改名，与当时「只有一处
/// 实现」的表述不符；历史提交信息不改写，此处按事实更正）。两处调用点现均为
/// 「先做各自的 TypeRef/变量遍历，再调用本函数」。
///
/// 普通替换（按 TypeVar 或 TypeRef）覆盖不到 `MonoType::Generic` 的头部字符串，
/// 而 `F(A)` 的 F 正是声明里绑定的构造器变量——只按名在头部替换，不触碰实参位。
pub fn substitute_ctor_heads(
    ty: &MonoType,
    head_map: &HashMap<String, String>,
) -> MonoType {
    if head_map.is_empty() {
        return ty.clone();
    }
    match ty {
        MonoType::Generic { name, args } => MonoType::Generic {
            name: head_map.get(name).cloned().unwrap_or_else(|| name.clone()),
            args: args
                .iter()
                .map(|a| substitute_ctor_heads(a, head_map))
                .collect(),
        },
        MonoType::Fn {
            params,
            return_type,
        } => MonoType::Fn {
            params: params
                .iter()
                .map(|p| substitute_ctor_heads(p, head_map))
                .collect(),
            return_type: Box::new(substitute_ctor_heads(return_type, head_map)),
        },
        MonoType::Struct(s) => MonoType::Struct(StructType {
            fields: s
                .fields
                .iter()
                .map(|(n, t)| (n.clone(), substitute_ctor_heads(t, head_map)))
                .collect(),
            ..s.clone()
        }),
        MonoType::Ref { mutable, inner } => MonoType::Ref {
            mutable: *mutable,
            inner: Box::new(substitute_ctor_heads(inner, head_map)),
        },
        MonoType::Union(ts) => MonoType::Union(
            ts.iter()
                .map(|t| substitute_ctor_heads(t, head_map))
                .collect(),
        ),
        MonoType::Intersection(ts) => MonoType::Intersection(
            ts.iter()
                .map(|t| substitute_ctor_heads(t, head_map))
                .collect(),
        ),
        MonoType::MetaType {
            universe_level,
            type_params,
        } => MonoType::MetaType {
            universe_level: universe_level.clone(),
            type_params: type_params
                .iter()
                .map(|t| substitute_ctor_heads(t, head_map))
                .collect(),
        },
        MonoType::AssocType {
            host_type,
            assoc_name,
            assoc_args,
        } => MonoType::AssocType {
            host_type: Box::new(substitute_ctor_heads(host_type, head_map)),
            assoc_name: assoc_name.clone(),
            assoc_args: assoc_args
                .iter()
                .map(|t| substitute_ctor_heads(t, head_map))
                .collect(),
        },
        MonoType::Literal {
            name,
            base_type,
            value,
        } => MonoType::Literal {
            name: name.clone(),
            base_type: Box::new(substitute_ctor_heads(base_type, head_map)),
            value: value.clone(),
        },
        _ => ty.clone(),
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
            MonoType::Generic { name, args } => MonoType::Generic {
                name: name.clone(),
                args: args
                    .iter()
                    .map(|t| self.substitute_internal(t, lookup))
                    .collect(),
            },
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
                })
            }
            MonoType::Enum(e) => MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
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
            MonoType::Ref { mutable, inner } => MonoType::Ref {
                mutable: *mutable,
                inner: Box::new(self.substitute_internal(inner, lookup)),
            },
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
        MonoType::Generic { name: _, args } => args.iter().any(contains_type_vars),
        MonoType::Fn {
            params,
            return_type,
            ..
        } => params.iter().any(contains_type_vars) || contains_type_vars(return_type),
        MonoType::Struct(struct_type) => struct_type
            .fields
            .iter()
            .any(|(_, field_ty)| contains_type_vars(field_ty)),
        MonoType::Union(types) | MonoType::Intersection(types) => {
            types.iter().any(contains_type_vars)
        }
        MonoType::AssocType {
            host_type,
            assoc_args,
            ..
        } => contains_type_vars(host_type) || assoc_args.iter().any(contains_type_vars),
        _ => false,
    }
}

// ========== RFC-010: Universe Level Tests ==========
