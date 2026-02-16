//! RFC-011 类型计算引擎
//!
//! 整合归约、范式化和统一，提供完整的类型级计算能力。
//!
//! 核心功能：
//! - 类型表达式求值
//! - 条件类型求解
//! - 类型级函数应用

use crate::frontend::core::type_system::{MonoType, TypeVar};
use crate::frontend::type_level::evaluation::{TypeNormalizer, TypeReducer, ReductionConfig};
use std::collections::HashMap;

/// 类型计算结果
#[derive(Debug, Clone, PartialEq)]
pub enum ComputeResult {
    /// 计算成功
    Done(MonoType),

    /// 需要更多信息
    Pending(Vec<MonoType>),

    /// 计算失败
    Error(String),
}

/// 类型计算引擎
///
/// 整合所有类型级计算能力
#[derive(Debug, Clone)]
pub struct TypeComputer {
    /// 范式化器
    normalizer: TypeNormalizer,

    /// 归约器
    reducer: TypeReducer,

    /// 计算配置
    config: ComputeConfig,

    /// 上下文数据
    context: ComputeContext,
}

/// 计算配置
#[derive(Debug, Clone, Default)]
pub struct ComputeConfig {
    /// 归约配置
    pub reduction: ReductionConfig,

    /// 最大迭代次数
    pub max_iterations: usize,

    /// 是否启用缓存
    pub enable_cache: bool,
}

/// 类型函数
///
/// 定义类型级函数
#[derive(Debug, Clone)]
pub struct TypeFunction {
    /// 函数名称
    pub name: String,

    /// 类型参数
    pub params: Vec<String>,

    /// 函数体（类型表达式）
    pub body: MonoType,
}

impl TypeFunction {
    /// 创建新的类型函数
    pub fn new(
        name: String,
        params: Vec<String>,
        body: MonoType,
    ) -> Self {
        Self { name, params, body }
    }
}

/// 计算上下文
#[derive(Debug, Clone, Default)]
pub struct ComputeContext {
    /// 类型别名
    type_aliases: HashMap<String, MonoType>,

    /// 类型函数
    type_functions: HashMap<String, TypeFunction>,

    /// 计算缓存
    cache: HashMap<MonoType, ComputeResult>,
}

impl ComputeContext {
    /// 创建新的计算上下文
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册类型别名
    pub fn register_alias(
        &mut self,
        name: String,
        ty: MonoType,
    ) {
        self.type_aliases.insert(name, ty);
    }

    /// 注册类型函数
    pub fn register_function(
        &mut self,
        name: String,
        func: TypeFunction,
    ) {
        self.type_functions.insert(name, func);
    }

    /// 获取类型别名
    pub fn get_alias(
        &self,
        name: &str,
    ) -> Option<&MonoType> {
        self.type_aliases.get(name)
    }

    /// 获取类型函数
    pub fn get_function(
        &self,
        name: &str,
    ) -> Option<&TypeFunction> {
        self.type_functions.get(name)
    }
}

impl Default for TypeComputer {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeComputer {
    /// 创建新的类型计算引擎
    pub fn new() -> Self {
        Self {
            normalizer: TypeNormalizer::new(),
            reducer: TypeReducer::new(),
            config: ComputeConfig::default(),
            context: ComputeContext::new(),
        }
    }

    /// 创建带配置的引擎
    pub fn with_config(config: ComputeConfig) -> Self {
        let reduction = config.reduction.clone();
        Self {
            normalizer: TypeNormalizer::with_config(reduction.clone()),
            reducer: TypeReducer::with_config(reduction.clone()),
            config,
            context: ComputeContext::new(),
        }
    }

    /// 计算类型
    pub fn compute(
        &mut self,
        ty: &MonoType,
    ) -> ComputeResult {
        // 检查缓存
        if self.config.enable_cache {
            if let Some(result) = self.context.cache.get(ty).cloned() {
                return result;
            }
        }

        let result = self.compute_internal(ty);

        // 缓存结果
        if self.config.enable_cache {
            self.context.cache.insert(ty.clone(), result.clone());
        }

        result
    }

    /// 内部计算逻辑
    fn compute_internal(
        &mut self,
        ty: &MonoType,
    ) -> ComputeResult {
        // 先尝试归约
        let reduced = self.reducer.reduce(ty);

        match reduced {
            super::ReductionResult::Reduced(new_ty, _) => {
                // 递归计算
                self.compute(&new_ty)
            }
            super::ReductionResult::Stuck => {
                // 检查是否是条件类型
                self.compute_conditional(ty)
            }
            super::ReductionResult::Failed(msg) => ComputeResult::Error(msg),
        }
    }

    /// 计算条件类型
    ///
    /// 使用 TypeEvaluator 计算 If、Match 等条件类型的值
    fn compute_conditional(
        &mut self,
        ty: &MonoType,
    ) -> ComputeResult {
        // 使用 normalizer 中的 evaluator 计算条件类型
        let evaluator = self.normalizer.evaluator();

        // 计算类型
        let eval_result = evaluator.eval(ty);

        match eval_result {
            crate::frontend::typecheck::type_eval::EvalResult::Value(result_ty) => {
                // 进一步归一化结果
                let normalized = self.normalizer.normalize(&result_ty);
                if matches!(normalized, super::NormalForm::Normalized) {
                    ComputeResult::Done(result_ty)
                } else {
                    // 如果还需要归约，继续处理
                    ComputeResult::Pending(vec![result_ty])
                }
            }
            crate::frontend::typecheck::type_eval::EvalResult::Pending => {
                ComputeResult::Pending(vec![ty.clone()])
            }
            crate::frontend::typecheck::type_eval::EvalResult::Error(msg) => {
                ComputeResult::Error(msg)
            }
        }
    }

    /// 设置上下文
    pub fn set_context(
        &mut self,
        context: ComputeContext,
    ) {
        self.context = context;
    }

    /// 获取上下文
    pub fn context(&self) -> &ComputeContext {
        &self.context
    }

    /// 注册类型别名
    pub fn register_alias(
        &mut self,
        name: String,
        ty: MonoType,
    ) {
        self.context.register_alias(name.clone(), ty.clone());
        self.reducer.register_alias(name, ty);
    }

    /// 注册类型函数
    pub fn register_function(
        &mut self,
        name: String,
        func: TypeFunction,
    ) {
        self.context.register_function(name, func);
    }
}

/// 预定义的类型级函数
pub mod functions {
    use super::*;

    /// Identity 类型函数
    pub fn identity() -> TypeFunction {
        TypeFunction::new(
            "identity".to_string(),
            vec!["T".to_string()],
            MonoType::TypeVar(TypeVar::new(0)),
        )
    }

    /// Const 类型函数
    pub fn const_fn() -> TypeFunction {
        TypeFunction::new(
            "const".to_string(),
            vec!["T".to_string(), "U".to_string()],
            MonoType::TypeVar(TypeVar::new(0)),
        )
    }

    /// Swap 类型函数
    pub fn swap() -> TypeFunction {
        TypeFunction::new(
            "swap".to_string(),
            vec!["T".to_string(), "U".to_string()],
            MonoType::Tuple(vec![
                MonoType::TypeVar(TypeVar::new(1)),
                MonoType::TypeVar(TypeVar::new(0)),
            ]),
        )
    }

    /// 柯里化类型函数
    pub fn curry() -> TypeFunction {
        TypeFunction::new(
            "curry".to_string(),
            vec!["A".to_string(), "B".to_string(), "C".to_string()],
            MonoType::Fn {
                params: vec![MonoType::TypeVar(TypeVar::new(0))],
                return_type: Box::new(MonoType::Fn {
                    params: vec![MonoType::TypeVar(TypeVar::new(1))],
                    return_type: Box::new(MonoType::TypeVar(TypeVar::new(2))),
                    is_async: false,
                }),
                is_async: false,
            },
        )
    }
}
