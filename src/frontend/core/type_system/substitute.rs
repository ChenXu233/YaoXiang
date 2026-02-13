//! 统一的类型替换模块
//!
//! 提供通用的类型替换算法，消除各模块间的重复代码。

use crate::frontend::core::type_system::{MonoType, TypeVar, StructType, EnumType};
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
                is_async,
            } => {
                let new_params = params
                    .iter()
                    .map(|p| self.substitute_internal(p, lookup))
                    .collect();
                let new_return_type = Box::new(self.substitute_internal(return_type, lookup));
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
                    .map(|(name, field_ty)| {
                        (name.clone(), self.substitute_internal(field_ty, lookup))
                    })
                    .collect();
                MonoType::Struct(StructType {
                    name: struct_type.name.clone(),
                    fields: new_fields,
                    methods: struct_type.methods.clone(),
                    field_mutability: struct_type.field_mutability.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_substitute_var() {
        let substituter = Substituter::new();
        let tv = TypeVar::new(0);
        let int_type = MonoType::Int(32);

        // T -> int
        let result = substituter.substitute_var(
            &MonoType::List(Box::new(MonoType::TypeVar(tv))),
            &tv,
            &int_type,
        );

        assert_eq!(result, MonoType::List(Box::new(MonoType::Int(32))));
    }

    #[test]
    fn test_substitute_generic_params() {
        let substituter = Substituter::new();
        let tv1 = TypeVar::new(0);
        let tv2 = TypeVar::new(1);

        // fn(T, U) -> T
        let fn_type = MonoType::Fn {
            params: vec![MonoType::TypeVar(tv1), MonoType::TypeVar(tv2)],
            return_type: Box::new(MonoType::TypeVar(tv1)),
            is_async: false,
        };

        let args = vec![MonoType::Int(32), MonoType::String];
        let result = substituter.substitute_generic_params(&fn_type, &args);

        match result {
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                assert_eq!(params, vec![MonoType::Int(32), MonoType::String]);
                assert_eq!(*return_type, MonoType::Int(32));
            }
            _ => panic!("Expected Fn type"),
        }
    }

    #[test]
    fn test_contains_type_vars() {
        let tv = TypeVar::new(0);
        assert!(contains_type_vars(&MonoType::TypeVar(tv)));

        let list_with_tv = MonoType::List(Box::new(MonoType::TypeVar(tv)));
        assert!(contains_type_vars(&list_with_tv));

        assert!(!contains_type_vars(&MonoType::Int(32)));
        assert!(!contains_type_vars(&MonoType::String));
    }
}

// ========== RFC-010: Universe Level Tests ==========

#[cfg(test)]
mod universe_level_tests {
    use super::*;
    use crate::frontend::core::type_system::{
        UniverseLevel, calculate_meta_type_level, get_ast_type_universe_level,
    };

    /// Test UniverseLevel basic operations
    #[test]
    fn test_universe_level_basics() {
        let level0 = UniverseLevel::type0();
        let level1 = UniverseLevel::type1();

        assert_eq!(level0.level, "0");
        assert_eq!(level1.level, "1");
    }

    #[test]
    fn test_universe_level_successor() {
        let level0 = UniverseLevel::type0();
        let level1 = level0.succ();
        let level2 = level1.succ();
        let level10 = UniverseLevel::new("9".to_string()).succ();

        assert_eq!(level1.level, "1");
        assert_eq!(level2.level, "2");
        assert_eq!(level10.level, "10");
    }

    #[test]
    fn test_universe_level_carry() {
        // Test "9" -> "10" carry
        let level9 = UniverseLevel::new("9".to_string());
        let level10 = level9.succ();
        assert_eq!(level10.level, "10");

        // Test "99" -> "100" carry
        let level99 = UniverseLevel::new("99".to_string());
        let level100 = level99.succ();
        assert_eq!(level100.level, "100");
    }

    #[test]
    fn test_universe_level_display() {
        let level0 = UniverseLevel::type0();
        let level1 = UniverseLevel::type1();
        let level2 = UniverseLevel::new("2".to_string());

        assert_eq!(format!("{}", level0), "Type0");
        assert_eq!(format!("{}", level1), "Type1");
        assert_eq!(format!("{}", level2), "Type2");
    }

    /// Test get_ast_type_universe_level function
    #[test]
    fn test_get_ast_type_universe_level() {
        use crate::frontend::core::parser::ast::Type;

        // Plain Type should be Type0
        let plain_type = Type::Name("Int".to_string());
        assert_eq!(get_ast_type_universe_level(&plain_type), 0);

        // MetaType without args should be Type0
        let meta_empty = Type::MetaType { args: Vec::new() };
        assert_eq!(get_ast_type_universe_level(&meta_empty), 0);

        // MetaType with simple arg (Type0) should be Type1
        let meta_with_t = Type::MetaType {
            args: vec![Type::Name("T".to_string())],
        };
        assert_eq!(get_ast_type_universe_level(&meta_with_t), 1);

        // MetaType with MetaType arg (Type1) should be Type2
        let meta_nested = Type::MetaType {
            args: vec![Type::MetaType {
                args: vec![Type::Name("T".to_string())],
            }],
        };
        assert_eq!(get_ast_type_universe_level(&meta_nested), 2);

        // MetaType with deeply nested (Type2) should be Type3
        let meta_deep = Type::MetaType {
            args: vec![Type::MetaType {
                args: vec![Type::MetaType {
                    args: vec![Type::Name("T".to_string())],
                }],
            }],
        };
        assert_eq!(get_ast_type_universe_level(&meta_deep), 3);
    }

    /// Test calculate_meta_type_level function
    #[test]
    fn test_calculate_meta_type_level() {
        use crate::frontend::core::parser::ast::Type;

        // Empty args -> Type0
        let empty_args: Vec<Type> = Vec::new();
        let level = calculate_meta_type_level(&empty_args);
        assert!(level.is_type0());

        // Single Type0 arg -> Type1
        let single_arg = vec![Type::Name("T".to_string())];
        let level = calculate_meta_type_level(&single_arg);
        assert!(level.is_type1());

        // Multiple Type0 args -> Type1 (max is 0, +1 = 1)
        let multi_args = vec![Type::Name("T".to_string()), Type::Name("U".to_string())];
        let level = calculate_meta_type_level(&multi_args);
        assert!(level.is_type1());

        // Nested MetaType -> Type2
        let nested = vec![Type::MetaType {
            args: vec![Type::Name("T".to_string())],
        }];
        let level = calculate_meta_type_level(&nested);
        assert_eq!(level.level, "2");
    }

    /// Test MonoType MetaType display
    #[test]
    fn test_mono_type_meta_type_display() {
        // Plain Type
        let plain = MonoType::MetaType {
            universe_level: UniverseLevel::type0(),
            type_params: Vec::new(),
        };
        assert_eq!(plain.type_name(), "Type");

        // Type[T] - RFC-010: users only see "Type", not "Type1"
        let with_t = MonoType::MetaType {
            universe_level: UniverseLevel::type1(),
            type_params: vec![MonoType::TypeVar(TypeVar::new(0))],
        };
        let name = with_t.type_name();
        assert_eq!(name, "Type[t0]");

        // Type[Type[T]]
        let nested = MonoType::MetaType {
            universe_level: UniverseLevel::new("2".to_string()),
            type_params: vec![MonoType::MetaType {
                universe_level: UniverseLevel::type1(),
                type_params: vec![MonoType::TypeVar(TypeVar::new(0))],
            }],
        };
        let name = nested.type_name();
        assert_eq!(name, "Type[Type[t0]]");
    }
}
