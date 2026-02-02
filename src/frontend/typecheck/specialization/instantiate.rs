#![allow(clippy::result_large_err)]

//! 实例化算法
//!
//! 实现泛型实例化

use crate::frontend::shared::error::Result;
use crate::frontend::core::type_system::{MonoType, StructType, EnumType};

/// 实例化结果
#[derive(Debug, Clone)]
pub struct InstanceResult {
    pub instance: MonoType,
    pub generic: MonoType,
}

/// 实例化算法
#[derive(Debug, Default)]
pub struct Instantiator;

impl Instantiator {
    /// 创建新的实例化器
    pub fn new() -> Self {
        Self
    }

    /// 执行实例化
    pub fn instantiate(
        &self,
        generic: &MonoType,
        args: &[MonoType],
    ) -> Result<InstanceResult> {
        // 检查是否可以实例化
        if !self.can_instantiate(generic, args) {
            return Err(
                crate::frontend::shared::error::diagnostic::Diagnostic::error(
                    "E0701".to_string(),
                    "Cannot instantiate generic type with given arguments".to_string(),
                    None,
                ),
            );
        }

        // 执行实例化：用具体类型替换泛型参数
        let instance = self.substitute_generic_params(generic, args)?;

        Ok(InstanceResult {
            instance,
            generic: generic.clone(),
        })
    }

    /// 用具体类型替换泛型参数
    #[allow(clippy::only_used_in_recursion)]
    fn substitute_generic_params(
        &self,
        ty: &MonoType,
        args: &[MonoType],
    ) -> Result<MonoType> {
        match ty {
            // 对于类型变量，使用传入的参数替换
            MonoType::TypeVar(var) => {
                // 简化实现：假设类型变量按顺序对应参数
                let index = var.index();
                if index < args.len() {
                    Ok(args[index].clone())
                } else {
                    Ok(ty.clone())
                }
            }

            // 递归处理复合类型
            MonoType::List(inner) => {
                let new_inner = self.substitute_generic_params(inner, args)?;
                Ok(MonoType::List(Box::new(new_inner)))
            }

            MonoType::Tuple(types) => {
                let new_types = types
                    .iter()
                    .map(|t| self.substitute_generic_params(t, args))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(MonoType::Tuple(new_types))
            }

            MonoType::Dict(k, v) => {
                let new_k = self.substitute_generic_params(k, args)?;
                let new_v = self.substitute_generic_params(v, args)?;
                Ok(MonoType::Dict(Box::new(new_k), Box::new(new_v)))
            }

            MonoType::Set(t) => {
                let new_t = self.substitute_generic_params(t, args)?;
                Ok(MonoType::Set(Box::new(new_t)))
            }

            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => {
                let new_params = params
                    .iter()
                    .map(|p| self.substitute_generic_params(p, args))
                    .collect::<Result<Vec<_>, _>>()?;
                let new_return = self.substitute_generic_params(return_type, args)?;
                Ok(MonoType::Fn {
                    params: new_params,
                    return_type: Box::new(new_return),
                    is_async: *is_async,
                })
            }

            MonoType::Struct(struct_type) => {
                let new_fields = struct_type
                    .fields
                    .iter()
                    .map(|(name, field_ty)| {
                        Ok::<_, crate::frontend::shared::error::Diagnostic>((
                            name.clone(),
                            self.substitute_generic_params(field_ty, args)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(MonoType::Struct(StructType {
                    name: struct_type.name.clone(),
                    fields: new_fields,
                    methods: struct_type.methods.clone(),
                }))
            }

            MonoType::Enum(e) => Ok(MonoType::Enum(EnumType {
                name: e.name.clone(),
                variants: e.variants.clone(),
            })),

            MonoType::Range { elem_type } => {
                let new_elem = self.substitute_generic_params(elem_type, args)?;
                Ok(MonoType::Range {
                    elem_type: Box::new(new_elem),
                })
            }

            MonoType::Union(types) => {
                let new_types = types
                    .iter()
                    .map(|t| self.substitute_generic_params(t, args))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(MonoType::Union(new_types))
            }

            MonoType::Intersection(types) => {
                let new_types = types
                    .iter()
                    .map(|t| self.substitute_generic_params(t, args))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(MonoType::Intersection(new_types))
            }

            MonoType::Arc(t) => {
                let new_t = self.substitute_generic_params(t, args)?;
                Ok(MonoType::Arc(Box::new(new_t)))
            }

            _ => Ok(ty.clone()),
        }
    }

    /// 检查是否可以实例化
    pub fn can_instantiate(
        &self,
        generic: &MonoType,
        args: &[MonoType],
    ) -> bool {
        // 检查泛型类型是否包含类型变量
        self.contains_type_vars(generic) && !args.is_empty()
    }

    /// 检查类型是否包含类型变量
    #[allow(clippy::only_used_in_recursion)]
    fn contains_type_vars(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            MonoType::TypeVar(_) => true,
            MonoType::List(inner) => self.contains_type_vars(inner),
            MonoType::Tuple(types) => types.iter().any(|t| self.contains_type_vars(t)),
            MonoType::Dict(k, v) => self.contains_type_vars(k) || self.contains_type_vars(v),
            MonoType::Set(t) => self.contains_type_vars(t),
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                params.iter().any(|p| self.contains_type_vars(p))
                    || self.contains_type_vars(return_type)
            }
            MonoType::Struct(struct_type) => struct_type
                .fields
                .iter()
                .any(|(_, field_ty)| self.contains_type_vars(field_ty)),
            MonoType::Range { elem_type } => self.contains_type_vars(elem_type),
            MonoType::Union(types) | MonoType::Intersection(types) => {
                types.iter().any(|t| self.contains_type_vars(t))
            }
            MonoType::Arc(t) => self.contains_type_vars(t),
            _ => false,
        }
    }
}
