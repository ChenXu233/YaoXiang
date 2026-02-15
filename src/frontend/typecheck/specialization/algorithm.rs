#![allow(clippy::result_large_err)]

//! 特化算法
//!
//! 实现泛型特化的核心算法

use crate::util::diagnostic::Result;
use crate::frontend::core::type_system::{MonoType, PolyType, StructType, EnumType, TypeVar};
use std::collections::HashMap;

/// 特化算法
#[derive(Debug, Default)]
pub struct SpecializationAlgorithm {
    /// 替换映射：TypeVar -> MonoType
    substitution: HashMap<TypeVar, MonoType>,
}

impl SpecializationAlgorithm {
    /// 创建新的特化算法
    pub fn new() -> Self {
        Self {
            substitution: HashMap::new(),
        }
    }

    /// 执行特化
    ///
    /// 将多态类型中的泛型变量替换为具体类型
    pub fn specialize(
        &mut self,
        poly: &PolyType,
        args: &[MonoType],
    ) -> Result<MonoType> {
        // 构建替换映射
        self.substitution.clear();
        if poly.type_binders.len() != args.len() {
            return Err(
                crate::util::diagnostic::ErrorCodeDefinition::type_argument_count_mismatch(
                    poly.type_binders.len(),
                    args.len(),
                )
                .build(),
            );
        }

        for (var, arg) in poly.type_binders.iter().zip(args.iter()) {
            self.substitution.insert(*var, arg.clone());
        }

        // 递归替换类型
        let result = self.substitute_type(&poly.body);
        Ok(result)
    }

    /// 检查是否可以特化
    pub fn can_specialize(
        &self,
        poly: &PolyType,
        args: &[MonoType],
    ) -> bool {
        poly.type_binders.len() == args.len() && !args.is_empty()
    }

    /// 递归替换类型
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_type(
        &self,
        ty: &MonoType,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(v) => self
                .substitution
                .get(v)
                .cloned()
                .unwrap_or_else(|| ty.clone()),
            MonoType::Struct(s) => MonoType::Struct(StructType {
                name: s.name.clone(),
                fields: s
                    .fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.substitute_type(t)))
                    .collect(),
                methods: s.methods.clone(),
                field_mutability: s.field_mutability.clone(),
            }),
            MonoType::Enum(e) => MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
            MonoType::Tuple(ts) => {
                MonoType::Tuple(ts.iter().map(|t| self.substitute_type(t)).collect())
            }
            MonoType::List(t) => MonoType::List(Box::new(self.substitute_type(t))),
            MonoType::Dict(k, v) => MonoType::Dict(
                Box::new(self.substitute_type(k)),
                Box::new(self.substitute_type(v)),
            ),
            MonoType::Set(t) => MonoType::Set(Box::new(self.substitute_type(t))),
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params.iter().map(|t| self.substitute_type(t)).collect(),
                return_type: Box::new(self.substitute_type(return_type)),
                is_async: *is_async,
            },
            MonoType::Range { elem_type } => MonoType::Range {
                elem_type: Box::new(self.substitute_type(elem_type)),
            },
            MonoType::Union(types) => {
                MonoType::Union(types.iter().map(|t| self.substitute_type(t)).collect())
            }
            MonoType::Intersection(types) => {
                MonoType::Intersection(types.iter().map(|t| self.substitute_type(t)).collect())
            }
            MonoType::Arc(t) => MonoType::Arc(Box::new(self.substitute_type(t))),
            _ => ty.clone(),
        }
    }
}

/// 特化器
#[derive(Debug, Default)]
pub struct Specializer {
    algorithm: SpecializationAlgorithm,
}

impl Specializer {
    /// 创建新的特化器
    pub fn new() -> Self {
        Self {
            algorithm: SpecializationAlgorithm::new(),
        }
    }

    /// 特化多态类型
    pub fn specialize(
        &mut self,
        poly: &PolyType,
        args: &[MonoType],
    ) -> Result<MonoType> {
        self.algorithm.specialize(poly, args)
    }

    /// 检查是否可以特化
    pub fn can_specialize(
        &self,
        poly: &PolyType,
        args: &[MonoType],
    ) -> bool {
        self.algorithm.can_specialize(poly, args)
    }
}
