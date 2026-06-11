//! RFC-011 类型归约与计算引擎
//!
//! 提供类型级归约、计算和统一能力：
//! - reduce: 类型归约（Beta, Eta, Delta, Iota）
//! - compute: 类型计算引擎
//! - unify: 类型级统一

use crate::frontend::core::types::{MonoType, TypeVar, Substitution, Substituter};
use super::normalizer::{NormalForm, ReductionConfig, TypeNormalizer};
use std::collections::HashMap;

/// 归约步
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReductionStep {
    /// Beta 归约
    Beta,

    /// Eta 归约
    Eta,

    /// Delta 归约（类型展开）
    Delta,

    /// Iota 归约（模式匹配）
    Iota,

    /// 自定义归约
    Custom(String),
}

// ====================================================================
// TypeComputer
// ====================================================================
/// RFC-011 类型计算引擎
///
/// 整合归约、范式化和统一，提供完整的类型级计算能力。
///
/// 核心功能：
/// - 类型表达式求值
/// - 条件类型求解
/// - 类型级函数应用

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
            ReductionResult::Reduced(new_ty, _) => {
                // 递归计算
                self.compute(&new_ty)
            }
            ReductionResult::Stuck => {
                // 检查是否是条件类型
                self.compute_conditional(ty)
            }
            ReductionResult::Failed(msg) => ComputeResult::Error(msg),
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
            super::evaluator::EvalResult::Value(result_ty) => {
                // 进一步归一化结果
                let normalized = self.normalizer.normalize(&result_ty);
                if matches!(normalized, NormalForm::Normalized) {
                    ComputeResult::Done(result_ty)
                } else {
                    // 如果还需要归约，继续处理
                    ComputeResult::Pending(vec![result_ty])
                }
            }
            super::evaluator::EvalResult::Pending => ComputeResult::Pending(vec![ty.clone()]),
            super::evaluator::EvalResult::Error(msg) => ComputeResult::Error(msg),
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
                }),
            },
        )
    }
}

// ====================================================================
// TypeReducer
// ====================================================================
/// RFC-011 类型归约
///
/// 实现类型表达式的归约（Reduction）操作。
///
/// 支持的归约规则：
/// - Beta 归约：应用函数到参数
/// - Eta 归约：简化冗余抽象
/// - Delta 归约：展开类型别名
/// - Iota 归约：模式匹配求值
///
/// 归约状态
#[derive(Debug, Clone, PartialEq)]
pub enum ReductionResult {
    /// 成功归约
    Reduced(MonoType, ReductionStep),

    /// 无法继续归约
    Stuck,

    /// 归约失败
    Failed(String),
}

/// 归约器
///
/// 执行类型表达式的归约操作
#[derive(Debug, Clone)]
pub struct TypeReducer {
    /// 归约配置
    config: ReductionConfig,

    /// 类型别名映射
    type_aliases: HashMap<String, MonoType>,

    /// 归约步数计数器
    step_count: usize,
}

impl Default for TypeReducer {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeReducer {
    /// 创建新的归约器
    pub fn new() -> Self {
        Self {
            config: ReductionConfig::default(),
            type_aliases: HashMap::new(),
            step_count: 0,
        }
    }

    /// 创建带配置的归约器
    pub fn with_config(config: ReductionConfig) -> Self {
        Self {
            config,
            type_aliases: HashMap::new(),
            step_count: 0,
        }
    }

    /// 注册类型别名
    pub fn register_alias(
        &mut self,
        name: String,
        ty: MonoType,
    ) {
        self.type_aliases.insert(name, ty);
    }

    /// 批量注册类型别名
    pub fn register_aliases(
        &mut self,
        aliases: HashMap<String, MonoType>,
    ) {
        self.type_aliases.extend(aliases);
    }

    /// 执行归约
    pub fn reduce(
        &mut self,
        ty: &MonoType,
    ) -> ReductionResult {
        self.reduce_with_limit(ty, self.config.max_steps)
    }

    /// 带步数限制的归约
    pub fn reduce_with_limit(
        &mut self,
        ty: &MonoType,
        limit: usize,
    ) -> ReductionResult {
        self.step_count = 0;

        if self.step_count >= limit {
            return ReductionResult::Stuck;
        }

        self.reduce_internal(ty, limit)
    }

    /// 内部归约逻辑
    fn reduce_internal(
        &mut self,
        ty: &MonoType,
        limit: usize,
    ) -> ReductionResult {
        // 检查步数限制
        if self.step_count >= limit {
            return ReductionResult::Stuck;
        }

        // 类型别名展开（Delta 归约）
        if self.config.enable_delta {
            if let MonoType::TypeRef(name) = ty {
                if let Some(alias) = self.type_aliases.get(name).cloned() {
                    self.step_count += 1;
                    return self.reduce_internal(&alias, limit);
                }
            }
        }

        // 函数类型：尝试 Eta 归约
        if let MonoType::Fn {
            params,
            return_type,
            ..
        } = ty
        {
            return self.reduce_function(params, return_type, limit);
        }

        // 其他类型无法归约
        ReductionResult::Stuck
    }

    /// 归约函数类型
    fn reduce_function(
        &mut self,
        _params: &[MonoType],
        _ret: &MonoType,
        _limit: usize,
    ) -> ReductionResult {
        // Eta 归约: (\x. f x) => f (当 x 不在 f 中自由出现时)
        // 这里我们只是简单地检查函数是否可以简化
        // 实际实现需要更复杂的自由变量分析

        // 检查是否可以进行 Beta 归约
        // 例如: (fn(x: T) => U)[T'] => U[T'/T]
        if !_params.is_empty() {
            // 简化情况：单参数函数返回参数本身
            // fn(x: T) => x  => fn(x: T) => x (无法归约)
        }

        ReductionResult::Stuck
    }

    /// 获取步数
    pub fn step_count(&self) -> usize {
        self.step_count
    }
}

// ====================================================================
// TypeUnifier
// ====================================================================
/// RFC-011 类型级统一
///
/// 实现类型级统一算法，用于：
/// - 类型等价性检查
/// - 类型变量绑定
/// - 联合类型求解
///
/// 复用 core/type_system/substitute.rs 中的公共替换实现
/// 统一结果
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum UnificationResult {
    /// 统一成功，生成替换映射
    Success(Substitution),

    /// 统一失败
    Failure(String),

    /// 需要进一步归约
    NeedReduction(MonoType, MonoType),
}

/// 类型级统一器
///
/// 执行类型表达式的统一，复用 Substituter 进行替换操作
#[derive(Debug, Clone)]
pub struct TypeUnifier {
    /// 当前替换
    substitution: Substitution,

    /// 替换器实例
    substituter: Substituter,
}

impl Default for TypeUnifier {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeUnifier {
    /// 创建新的统一器
    pub fn new() -> Self {
        Self {
            substitution: Substitution::new(),
            substituter: Substituter::new(),
        }
    }

    /// 创建带配置的统一器
    pub fn with_config(_config: ReductionConfig) -> Self {
        Self {
            substitution: Substitution::new(),
            substituter: Substituter::new(),
        }
    }

    /// 统一两个类型
    pub fn unify(
        &mut self,
        ty1: &MonoType,
        ty2: &MonoType,
    ) -> UnificationResult {
        self.unify_internal(ty1, ty2)
    }

    /// 内部统一逻辑
    fn unify_internal(
        &mut self,
        ty1: &MonoType,
        ty2: &MonoType,
    ) -> UnificationResult {
        match (ty1, ty2) {
            // 两个类型变量
            (MonoType::TypeVar(tv1), MonoType::TypeVar(tv2)) => {
                if tv1.index() == tv2.index() {
                    UnificationResult::Success(self.substitution.clone())
                } else {
                    // 绑定 tv1 到 tv2
                    self.substitution.bind(*tv1, MonoType::TypeVar(*tv2));
                    UnificationResult::Success(self.substitution.clone())
                }
            }

            // 类型变量和具体类型
            (MonoType::TypeVar(tv), other) | (other, MonoType::TypeVar(tv)) => {
                self.substitution.bind(*tv, other.clone());
                UnificationResult::Success(self.substitution.clone())
            }

            // 基本类型
            _ if ty1 == ty2 => UnificationResult::Success(self.substitution.clone()),

            // 元组统一
            (MonoType::Tuple(types1), MonoType::Tuple(types2)) => {
                if types1.len() != types2.len() {
                    return UnificationResult::Failure("Tuple arity mismatch".to_string());
                }
                let mut result = self.substitution.clone();
                for (t1, t2) in types1.iter().zip(types2.iter()) {
                    let arg_result = self.unify_with_substitution(t1, t2, &result);
                    match arg_result {
                        UnificationResult::Success(sub) => result = sub,
                        _ => return arg_result,
                    }
                }
                UnificationResult::Success(result)
            }

            // 列表统一
            (MonoType::List(t1), MonoType::List(t2)) => self.unify_internal(t1, t2),

            // 函数统一
            (
                MonoType::Fn {
                    params: params1,
                    return_type: ret1,
                    ..
                },
                MonoType::Fn {
                    params: params2,
                    return_type: ret2,
                    ..
                },
            ) => {
                if params1.len() != params2.len() {
                    return UnificationResult::Failure("Function arity mismatch".to_string());
                }
                let mut result = self.substitution.clone();
                for (p1, p2) in params1.iter().zip(params2.iter()) {
                    let param_result = self.unify_with_substitution(p1, p2, &result);
                    match param_result {
                        UnificationResult::Success(sub) => result = sub,
                        _ => return param_result,
                    }
                }
                self.unify_with_substitution(ret1, ret2, &result)
            }

            // 其他情况失败
            _ => UnificationResult::Failure(format!("Cannot unify {:?} and {:?}", ty1, ty2)),
        }
    }

    /// 带替换的统一
    fn unify_with_substitution(
        &mut self,
        ty1: &MonoType,
        ty2: &MonoType,
        current: &Substitution,
    ) -> UnificationResult {
        // 应用当前替换
        let applied1 = self.substituter.substitute(ty1, current);
        let applied2 = self.substituter.substitute(ty2, current);

        self.unify_internal(&applied1, &applied2)
    }

    /// 获取当前替换
    pub fn substitution(&self) -> &Substitution {
        &self.substitution
    }

    /// 重置统一器
    pub fn reset(&mut self) {
        self.substitution = Substitution::new();
    }
}
