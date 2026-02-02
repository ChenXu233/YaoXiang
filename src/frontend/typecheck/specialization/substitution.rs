#![allow(clippy::result_large_err)]

//! 替换逻辑
//!
//! 实现类型替换算法

use crate::frontend::shared::error::Result;
use crate::frontend::core::type_system::{MonoType, TypeVar, StructType, EnumType};

/// 替换结果
pub struct SubstitutionResult {
    pub substituted: MonoType,
    pub success: bool,
}

/// 替换器
#[derive(Debug, Default)]
pub struct Substituter;

impl Substituter {
    /// 创建新的替换器
    pub fn new() -> Self {
        Self
    }

    /// 执行类型替换
    pub fn substitute(
        &self,
        ty: &MonoType,
        var: &TypeVar,
        replacement: &MonoType,
    ) -> Result<SubstitutionResult> {
        let substituted = self.substitute_internal(ty, var, replacement);
        Ok(SubstitutionResult {
            substituted,
            success: true,
        })
    }

    /// 内部替换实现
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_internal(
        &self,
        ty: &MonoType,
        var: &TypeVar,
        replacement: &MonoType,
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(v) if v == var => replacement.clone(),
            MonoType::List(inner) => {
                MonoType::List(Box::new(self.substitute_internal(inner, var, replacement)))
            }
            MonoType::Tuple(types) => MonoType::Tuple(
                types
                    .iter()
                    .map(|t| self.substitute_internal(t, var, replacement))
                    .collect(),
            ),
            MonoType::Dict(k, v) => MonoType::Dict(
                Box::new(self.substitute_internal(k, var, replacement)),
                Box::new(self.substitute_internal(v, var, replacement)),
            ),
            MonoType::Set(t) => {
                MonoType::Set(Box::new(self.substitute_internal(t, var, replacement)))
            }
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => {
                let new_params = params
                    .iter()
                    .map(|p| self.substitute_internal(p, var, replacement))
                    .collect();
                let new_return_type =
                    Box::new(self.substitute_internal(return_type, var, replacement));
                MonoType::Fn {
                    params: new_params,
                    return_type: new_return_type,
                    is_async: *is_async,
                }
            }
            MonoType::Struct(struct_type) => {
                let new_fields = struct_type
                    .fields
                    .iter()
                    .map(|(name, ty)| {
                        (name.clone(), self.substitute_internal(ty, var, replacement))
                    })
                    .collect();
                MonoType::Struct(StructType {
                    name: struct_type.name.clone(),
                    fields: new_fields,
                    methods: struct_type.methods.clone(),
                })
            }
            MonoType::Enum(e) => MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            }),
            MonoType::Range { elem_type } => MonoType::Range {
                elem_type: Box::new(self.substitute_internal(elem_type, var, replacement)),
            },
            MonoType::Union(types) => MonoType::Union(
                types
                    .iter()
                    .map(|t| self.substitute_internal(t, var, replacement))
                    .collect(),
            ),
            MonoType::Intersection(types) => MonoType::Intersection(
                types
                    .iter()
                    .map(|t| self.substitute_internal(t, var, replacement))
                    .collect(),
            ),
            MonoType::Arc(t) => {
                MonoType::Arc(Box::new(self.substitute_internal(t, var, replacement)))
            }
            _ => ty.clone(),
        }
    }

    /// 批量替换
    pub fn substitute_batch(
        &self,
        ty: &MonoType,
        substitutions: &[(TypeVar, MonoType)],
    ) -> Result<MonoType> {
        let mut result = ty.clone();

        // 依次应用所有替换
        for (var, replacement) in substitutions {
            result = self.substitute_internal(&result, var, replacement);
        }

        Ok(result)
    }
}
