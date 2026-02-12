//! 泛型特化和多态处理
//!
//! 实现多态类型的实例化和特化
//!
//! 注意：核心实现已迁移到 `specialization/` 模块
//! 此文件保留向后兼容的便捷类型

#![allow(clippy::result_large_err)]

use crate::frontend::core::type_system::{
    MonoType, PolyType, StructType, EnumType, TypeConstraintSolver, TypeVar,
};
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition, I18nRegistry};
use std::collections::HashMap;

// 类型别名
type TypeResult = Result<MonoType, Diagnostic>;

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
    ) -> TypeResult {
        // 检查参数数量是否匹配
        if poly.type_binders.len() != args.len() {
            return Err(ErrorCodeDefinition::type_argument_count_mismatch(
                poly.type_binders.len(),
                args.len(),
            ).at(crate::util::span::Span::default()).build(I18nRegistry::en()));
        }

        // 构建替换映射
        let substitution: HashMap<_, _> = poly
            .type_binders
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
    ) -> TypeResult {
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
            MonoType::TypeVar(v) => substitution.get(v).cloned().unwrap_or_else(|| ty.clone()),
            MonoType::Struct(s) => MonoType::Struct(StructType {
                name: s.name.clone(),
                fields: s
                    .fields
                    .iter()
                    .map(|(n, t)| (n.clone(), self.substitute_type(t, substitution, _solver)))
                    .collect(),
                methods: s.methods.clone(),
                field_mutability: s.field_mutability.clone(),
            }),
            MonoType::Enum(e) => MonoType::Enum(EnumType {
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
            MonoType::Range { elem_type } => MonoType::Range {
                elem_type: Box::new(self.substitute_type(elem_type, substitution, _solver)),
            },
            MonoType::Union(types) => MonoType::Union(
                types
                    .iter()
                    .map(|t| self.substitute_type(t, substitution, _solver))
                    .collect(),
            ),
            MonoType::Intersection(types) => MonoType::Intersection(
                types
                    .iter()
                    .map(|t| self.substitute_type(t, substitution, _solver))
                    .collect(),
            ),
            MonoType::Arc(t) => {
                MonoType::Arc(Box::new(self.substitute_type(t, substitution, _solver)))
            }
            MonoType::Weak(t) => {
                MonoType::Weak(Box::new(self.substitute_type(t, substitution, _solver)))
            }
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
            .type_binders
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
    /// 检查是否包含泛型变量
    pub fn has_generics(&self) -> bool {
        !self.type_binders.is_empty()
    }

    /// 获取泛型变量的数量
    pub fn num_generics(&self) -> usize {
        self.type_binders.len()
    }

    /// 检查类型是否包含特定泛型变量
    pub fn contains_var(
        &self,
        var: &TypeVar,
    ) -> bool {
        self.type_binders.contains(var)
    }
}
