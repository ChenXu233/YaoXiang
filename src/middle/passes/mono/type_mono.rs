//! 类型单态化子模块
//!
//! 提供类型单态化相关的辅助函数和trait

use crate::frontend::parser::ast::Type as AstType;
use crate::frontend::typecheck::{EnumType, MonoType, StructType};
use crate::middle::core::ir::ModuleIR;
use crate::middle::passes::mono::instance::{GenericTypeId, SpecializationKey, TypeId, TypeInstance};

/// 类型单态化相关trait
pub trait TypeMonomorphizer {
    /// 收集所有泛型类型定义
    fn collect_generic_types(
        &mut self,
        module: &ModuleIR,
    );

    /// 将AST Type转换为MonoType
    fn type_to_mono_type(
        &self,
        ty: &AstType,
    ) -> MonoType;

    /// 获取类型的名称
    fn get_type_name(ty: &AstType) -> String;

    /// 检查类型是否包含类型变量（AST Type版本）
    fn contains_type_var_type(
        &self,
        ty: &AstType,
    ) -> bool;

    /// 从类型中提取类型参数
    fn extract_type_params_from_type(
        &self,
        ty: &AstType,
    ) -> Vec<String>;

    /// 递归收集类型变量
    fn collect_type_vars_from_type(
        &self,
        ty: &AstType,
        type_params: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
    );

    /// 单态化泛型类型
    fn monomorphize_type(
        &mut self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
    ) -> Option<MonoType>;

    /// 实例化具体类型
    fn instantiate_type(
        &self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
        generic_type: &MonoType,
    ) -> Option<MonoType>;

    /// 递归替换类型中的泛型参数
    fn substitute_type_args(
        &self,
        ty: &MonoType,
        type_args: &[MonoType],
        type_params: &[String],
    ) -> MonoType;

    /// 生成类型ID
    fn generate_type_id(
        &self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
    ) -> TypeId;

    /// 生成单态化类型名称
    fn generate_type_name(
        &self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
    ) -> String;

    /// 注册单态化后的类型
    fn register_monomorphized_type(
        &mut self,
        mono_type: MonoType,
    ) -> TypeId;

    /// 从MonoType提取类型参数
    fn extract_type_params_from_mono_type(
        &self,
        ty: &MonoType,
    ) -> Vec<String>;

    /// 递归收集MonoType中的类型变量
    fn collect_type_vars_from_mono_type(
        &self,
        ty: &MonoType,
        type_params: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
    );

    /// 获取已实例化的类型数量
    fn type_instance_count(&self) -> usize;

    /// 获取泛型类型数量
    fn generic_type_count(&self) -> usize;
}

/// 类型单态化器的默认实现
#[allow(clippy::only_used_in_recursion)]
impl TypeMonomorphizer for super::Monomorphizer {
    fn collect_generic_types(
        &mut self,
        module: &ModuleIR,
    ) {
        for ty in &module.types {
            if self.contains_type_var_type(ty) {
                let type_params = self.extract_type_params_from_type(ty);
                let type_name = Self::get_type_name(ty);
                let generic_id = GenericTypeId::new(type_name, type_params);
                let mono_type = self.type_to_mono_type(ty);
                self.generic_types.insert(generic_id, mono_type);
            }
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn type_to_mono_type(
        &self,
        ty: &AstType,
    ) -> MonoType {
        match ty {
            AstType::Name(name) => MonoType::TypeRef(name.clone()),
            AstType::Int(n) => MonoType::Int(*n),
            AstType::Float(n) => MonoType::Float(*n),
            AstType::Char => MonoType::Char,
            AstType::String => MonoType::String,
            AstType::Bytes => MonoType::Bytes,
            AstType::Bool => MonoType::Bool,
            AstType::Void => MonoType::Void,
            AstType::Struct(fields) => MonoType::Struct(StructType {
                name: fields
                    .first()
                    .map(|(n, _)| n.clone())
                    .unwrap_or_else(|| "Struct".to_string()),
                fields: fields
                    .iter()
                    .map(|(n, ty)| (n.clone(), self.type_to_mono_type(ty)))
                    .collect(),
            }),
            AstType::NamedStruct { name, fields } => MonoType::Struct(StructType {
                name: name.clone(),
                fields: fields
                    .iter()
                    .map(|(n, ty)| (n.clone(), self.type_to_mono_type(ty)))
                    .collect(),
            }),
            AstType::Union(variants) => MonoType::Union(
                variants
                    .iter()
                    .filter_map(|(_, ty)| ty.as_ref().map(|t| self.type_to_mono_type(t)))
                    .collect(),
            ),
            AstType::Enum(variants) => MonoType::Enum(EnumType {
                name: variants
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Enum".to_string()),
                variants: variants.clone(),
            }),
            AstType::Variant(_) => MonoType::TypeRef("Variant".to_string()),
            AstType::Tuple(types) => {
                MonoType::Tuple(types.iter().map(|t| self.type_to_mono_type(t)).collect())
            }
            AstType::List(elem) => MonoType::List(Box::new(self.type_to_mono_type(elem))),
            AstType::Dict(key, value) => MonoType::Dict(
                Box::new(self.type_to_mono_type(key)),
                Box::new(self.type_to_mono_type(value)),
            ),
            AstType::Set(elem) => MonoType::Set(Box::new(self.type_to_mono_type(elem))),
            AstType::Fn {
                params,
                return_type,
                ..
            } => MonoType::Fn {
                params: params.iter().map(|t| self.type_to_mono_type(t)).collect(),
                return_type: Box::new(self.type_to_mono_type(return_type)),
                is_async: false,
            },
            AstType::Option(inner) => MonoType::Union(vec![self.type_to_mono_type(inner)]),
            AstType::Result(_, _) => MonoType::TypeRef("Result".to_string()),
            AstType::Generic { name, args } => {
                let args_str = args
                    .iter()
                    .map(|t| self.type_to_mono_type(t).type_name())
                    .collect::<Vec<_>>()
                    .join(", ");
                MonoType::TypeRef(format!("{}<{}>", name, args_str))
            }
            AstType::Sum(types) => {
                MonoType::Union(types.iter().map(|t| self.type_to_mono_type(t)).collect())
            }
        }
    }

    fn get_type_name(ty: &AstType) -> String {
        match ty {
            AstType::Name(name) => name.clone(),
            AstType::Int(n) => format!("int{}", n),
            AstType::Float(n) => format!("float{}", n),
            AstType::Char => "char".to_string(),
            AstType::String => "string".to_string(),
            AstType::Bytes => "bytes".to_string(),
            AstType::Bool => "bool".to_string(),
            AstType::Void => "void".to_string(),
            AstType::Struct(fields) => fields
                .first()
                .map(|(n, _)| n.clone())
                .unwrap_or_else(|| "Struct".to_string()),
            AstType::NamedStruct { name, .. } => name.clone(),
            AstType::Union(variants) => variants
                .first()
                .map(|(n, _)| n.clone())
                .unwrap_or_else(|| "Union".to_string()),
            AstType::Enum(variants) => variants
                .first()
                .cloned()
                .unwrap_or_else(|| "Enum".to_string()),
            AstType::Variant(variants) => variants
                .first()
                .map(|v| v.name.clone())
                .unwrap_or_else(|| "Variant".to_string()),
            AstType::Tuple(types) => format!("tuple{}", types.len()),
            AstType::List(_) => "List".to_string(),
            AstType::Dict(_, _) => "Dict".to_string(),
            AstType::Set(_) => "Set".to_string(),
            AstType::Fn { .. } => "Fn".to_string(),
            AstType::Option(_) => "Option".to_string(),
            AstType::Result(_, _) => "Result".to_string(),
            AstType::Generic { name, .. } => name.clone(),
            AstType::Sum(_) => "Sum".to_string(),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn contains_type_var_type(
        &self,
        ty: &AstType,
    ) -> bool {
        match ty {
            AstType::Name(_) => false,
            AstType::Int(_)
            | AstType::Float(_)
            | AstType::Char
            | AstType::String
            | AstType::Bytes
            | AstType::Bool
            | AstType::Void => false,
            AstType::Struct(fields) | AstType::NamedStruct { fields, .. } => fields
                .iter()
                .any(|(_, fty)| self.contains_type_var_type(fty)),
            AstType::Union(variants) => variants
                .iter()
                .any(|(_, ty)| ty.as_ref().is_some_and(|t| self.contains_type_var_type(t))),
            AstType::Enum(_) => false,
            AstType::Variant(_) => false,
            AstType::Tuple(types) => types.iter().any(|t| self.contains_type_var_type(t)),
            AstType::List(elem) => self.contains_type_var_type(elem),
            AstType::Dict(key, value) => {
                self.contains_type_var_type(key) || self.contains_type_var_type(value)
            }
            AstType::Set(elem) => self.contains_type_var_type(elem),
            AstType::Fn {
                params,
                return_type,
                ..
            } => {
                params.iter().any(|t| self.contains_type_var_type(t))
                    || self.contains_type_var_type(return_type)
            }
            AstType::Option(inner) => self.contains_type_var_type(inner),
            AstType::Result(ok, err) => {
                self.contains_type_var_type(ok) || self.contains_type_var_type(err)
            }
            AstType::Generic { args, .. } => args.iter().any(|t| self.contains_type_var_type(t)),
            AstType::Sum(types) => types.iter().any(|t| self.contains_type_var_type(t)),
        }
    }

    fn extract_type_params_from_type(
        &self,
        ty: &AstType,
    ) -> Vec<String> {
        let mut type_params = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.collect_type_vars_from_type(ty, &mut type_params, &mut seen);
        type_params
    }

    #[allow(clippy::only_used_in_recursion)]
    fn collect_type_vars_from_type(
        &self,
        ty: &AstType,
        type_params: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        match ty {
            AstType::Name(name) => {
                if name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
                    && seen.insert(name.clone())
                {
                    type_params.push(name.clone());
                }
            }
            AstType::Struct(fields) | AstType::NamedStruct { fields, .. } => {
                fields
                    .iter()
                    .for_each(|(_, fty)| self.collect_type_vars_from_type(fty, type_params, seen));
            }
            AstType::Union(variants) => {
                variants.iter().for_each(|(_, ty)| {
                    if let Some(t) = ty {
                        self.collect_type_vars_from_type(t, type_params, seen);
                    }
                });
            }
            AstType::Enum(_) | AstType::Variant(_) => {}
            AstType::Tuple(types) => types
                .iter()
                .for_each(|t| self.collect_type_vars_from_type(t, type_params, seen)),
            AstType::List(elem) => self.collect_type_vars_from_type(elem, type_params, seen),
            AstType::Dict(key, value) => {
                self.collect_type_vars_from_type(key, type_params, seen);
                self.collect_type_vars_from_type(value, type_params, seen);
            }
            AstType::Set(elem) => self.collect_type_vars_from_type(elem, type_params, seen),
            AstType::Fn {
                params,
                return_type,
                ..
            } => {
                params
                    .iter()
                    .for_each(|p| self.collect_type_vars_from_type(p, type_params, seen));
                self.collect_type_vars_from_type(return_type, type_params, seen);
            }
            AstType::Option(inner) => self.collect_type_vars_from_type(inner, type_params, seen),
            AstType::Result(ok, err) => {
                self.collect_type_vars_from_type(ok, type_params, seen);
                self.collect_type_vars_from_type(err, type_params, seen);
            }
            AstType::Generic { args, .. } => args
                .iter()
                .for_each(|t| self.collect_type_vars_from_type(t, type_params, seen)),
            AstType::Sum(types) => types
                .iter()
                .for_each(|t| self.collect_type_vars_from_type(t, type_params, seen)),
            AstType::Int(_)
            | AstType::Float(_)
            | AstType::Char
            | AstType::String
            | AstType::Bytes
            | AstType::Bool
            | AstType::Void => {}
        }
    }

    fn monomorphize_type(
        &mut self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
    ) -> Option<MonoType> {
        let cache_key = SpecializationKey::new(generic_id.name().to_string(), type_args.to_vec());
        if let Some(cached_id) = self.type_specialization_cache.get(&cache_key) {
            if let Some(instance) = self.type_instances.get(cached_id) {
                return instance.get_mono_type().cloned();
            }
        }

        let generic_type = self.generic_types.get(generic_id)?;
        let mono_type = self.instantiate_type(generic_id, type_args, generic_type)?;

        let type_id = self.generate_type_id(generic_id, type_args);
        let mut instance =
            TypeInstance::new(type_id.clone(), generic_id.clone(), type_args.to_vec());
        instance.set_mono_type(mono_type.clone());

        self.type_specialization_cache
            .insert(cache_key, type_id.clone());
        self.type_instances.insert(type_id, instance);

        Some(mono_type)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn instantiate_type(
        &self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
        generic_type: &MonoType,
    ) -> Option<MonoType> {
        let type_params = generic_id.type_params().to_vec();

        match generic_type {
            MonoType::Struct(struct_type) => {
                let mono_fields: Vec<(String, MonoType)> = struct_type
                    .fields
                    .iter()
                    .map(|(name, ty)| {
                        (
                            name.clone(),
                            self.substitute_type_args(ty, type_args, &type_params),
                        )
                    })
                    .collect();
                Some(MonoType::Struct(StructType {
                    name: self.generate_type_name(generic_id, type_args),
                    fields: mono_fields,
                }))
            }
            MonoType::Enum(enum_type) => Some(MonoType::Enum(EnumType {
                name: self.generate_type_name(generic_id, type_args),
                variants: enum_type.variants.clone(),
            })),
            MonoType::List(elem) => Some(MonoType::List(Box::new(self.substitute_type_args(
                elem,
                type_args,
                &type_params,
            )))),
            MonoType::Dict(key, value) => Some(MonoType::Dict(
                Box::new(self.substitute_type_args(key, type_args, &type_params)),
                Box::new(self.substitute_type_args(value, type_args, &type_params)),
            )),
            MonoType::Set(elem) => Some(MonoType::Set(Box::new(self.substitute_type_args(
                elem,
                type_args,
                &type_params,
            )))),
            MonoType::Tuple(types) => Some(MonoType::Tuple(
                types
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, &type_params))
                    .collect(),
            )),
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => Some(MonoType::Fn {
                params: params
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, &type_params))
                    .collect(),
                return_type: Box::new(self.substitute_type_args(
                    return_type,
                    type_args,
                    &type_params,
                )),
                is_async: *is_async,
            }),
            MonoType::Arc(inner) => Some(MonoType::Arc(Box::new(self.substitute_type_args(
                inner,
                type_args,
                &type_params,
            )))),
            MonoType::Range { elem_type } => Some(MonoType::Range {
                elem_type: Box::new(self.substitute_type_args(elem_type, type_args, &type_params)),
            }),
            MonoType::Union(types) | MonoType::Intersection(types) => {
                let substituted: Vec<MonoType> = types
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, &type_params))
                    .collect();
                Some(if matches!(generic_type, MonoType::Union(_)) {
                    MonoType::Union(substituted)
                } else {
                    MonoType::Intersection(substituted)
                })
            }
            _ => Some(generic_type.clone()),
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn substitute_type_args(
        &self,
        ty: &MonoType,
        type_args: &[MonoType],
        type_params: &[String],
    ) -> MonoType {
        match ty {
            MonoType::TypeVar(tv) => {
                let idx = tv.index();
                if idx < type_args.len() {
                    type_args[idx].clone()
                } else {
                    ty.clone()
                }
            }
            MonoType::Struct(struct_type) => MonoType::Struct(StructType {
                name: struct_type.name.clone(),
                fields: struct_type
                    .fields
                    .iter()
                    .map(|(name, field_ty)| {
                        (
                            name.clone(),
                            self.substitute_type_args(field_ty, type_args, type_params),
                        )
                    })
                    .collect(),
            }),
            MonoType::List(elem) => MonoType::List(Box::new(self.substitute_type_args(
                elem,
                type_args,
                type_params,
            ))),
            MonoType::Dict(key, value) => MonoType::Dict(
                Box::new(self.substitute_type_args(key, type_args, type_params)),
                Box::new(self.substitute_type_args(value, type_args, type_params)),
            ),
            MonoType::Set(elem) => MonoType::Set(Box::new(self.substitute_type_args(
                elem,
                type_args,
                type_params,
            ))),
            MonoType::Tuple(types) => MonoType::Tuple(
                types
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, type_params))
                    .collect(),
            ),
            MonoType::Fn {
                params,
                return_type,
                is_async,
            } => MonoType::Fn {
                params: params
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, type_params))
                    .collect(),
                return_type: Box::new(self.substitute_type_args(
                    return_type,
                    type_args,
                    type_params,
                )),
                is_async: *is_async,
            },
            MonoType::Arc(inner) => MonoType::Arc(Box::new(self.substitute_type_args(
                inner,
                type_args,
                type_params,
            ))),
            MonoType::Range { elem_type } => MonoType::Range {
                elem_type: Box::new(self.substitute_type_args(elem_type, type_args, type_params)),
            },
            MonoType::Union(types) | MonoType::Intersection(types) => {
                let substituted: Vec<MonoType> = types
                    .iter()
                    .map(|ty| self.substitute_type_args(ty, type_args, type_params))
                    .collect();
                if matches!(ty, MonoType::Union(_)) {
                    MonoType::Union(substituted)
                } else {
                    MonoType::Intersection(substituted)
                }
            }
            _ => ty.clone(),
        }
    }

    fn generate_type_id(
        &self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
    ) -> TypeId {
        TypeId::new(
            self.generate_type_name(generic_id, type_args),
            type_args.to_vec(),
        )
    }

    fn generate_type_name(
        &self,
        generic_id: &GenericTypeId,
        type_args: &[MonoType],
    ) -> String {
        if type_args.is_empty() {
            generic_id.name().to_string()
        } else {
            let args_str = type_args
                .iter()
                .map(|t| t.type_name())
                .collect::<Vec<_>>()
                .join("_");
            format!("{}_{}", generic_id.name(), args_str)
        }
    }

    fn register_monomorphized_type(
        &mut self,
        mono_type: MonoType,
    ) -> TypeId {
        let type_params = self.extract_type_params_from_mono_type(&mono_type);
        let type_id = TypeId::new(mono_type.type_name(), vec![]);
        let generic_id = GenericTypeId::new(mono_type.type_name(), type_params);
        let mut instance = TypeInstance::new(type_id.clone(), generic_id, vec![]);
        instance.set_mono_type(mono_type.clone());
        self.type_instances.insert(type_id.clone(), instance);
        type_id
    }

    fn extract_type_params_from_mono_type(
        &self,
        ty: &MonoType,
    ) -> Vec<String> {
        let mut type_params = Vec::new();
        let mut seen = std::collections::HashSet::new();
        self.collect_type_vars_from_mono_type(ty, &mut type_params, &mut seen);
        type_params
    }

    #[allow(clippy::only_used_in_recursion)]
    fn collect_type_vars_from_mono_type(
        &self,
        ty: &MonoType,
        type_params: &mut Vec<String>,
        seen: &mut std::collections::HashSet<String>,
    ) {
        match ty {
            MonoType::TypeVar(tv) => {
                let name = format!("T{}", tv.index());
                if seen.insert(name.clone()) {
                    type_params.push(name);
                }
            }
            MonoType::Struct(struct_type) => {
                struct_type.fields.iter().for_each(|(_, field_ty)| {
                    self.collect_type_vars_from_mono_type(field_ty, type_params, seen);
                });
            }
            MonoType::Enum(_) => {}
            MonoType::Tuple(types) => types
                .iter()
                .for_each(|t| self.collect_type_vars_from_mono_type(t, type_params, seen)),
            MonoType::List(elem) => self.collect_type_vars_from_mono_type(elem, type_params, seen),
            MonoType::Dict(key, value) => {
                self.collect_type_vars_from_mono_type(key, type_params, seen);
                self.collect_type_vars_from_mono_type(value, type_params, seen);
            }
            MonoType::Set(elem) => self.collect_type_vars_from_mono_type(elem, type_params, seen),
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                params
                    .iter()
                    .for_each(|p| self.collect_type_vars_from_mono_type(p, type_params, seen));
                self.collect_type_vars_from_mono_type(return_type, type_params, seen);
            }
            MonoType::Range { elem_type } => {
                self.collect_type_vars_from_mono_type(elem_type, type_params, seen)
            }
            MonoType::TypeRef(_)
            | MonoType::Void
            | MonoType::Bool
            | MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes => {}
            MonoType::Union(types) | MonoType::Intersection(types) => {
                types
                    .iter()
                    .for_each(|t| self.collect_type_vars_from_mono_type(t, type_params, seen));
            }
            MonoType::Arc(inner) => self.collect_type_vars_from_mono_type(inner, type_params, seen),
        }
    }

    fn type_instance_count(&self) -> usize {
        self.type_instances.len()
    }

    fn generic_type_count(&self) -> usize {
        self.generic_types.len()
    }
}
