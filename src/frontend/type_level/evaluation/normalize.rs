//! RFC-011 类型范式化
//!
//! 实现类型范式化算法，将类型表达式转换为标准形式。
//!
//! 范式化是类型级计算的基础，确保：
//! - 类型表达式的唯一表示
//! - 冗余结构被消除
//! - 嵌套类型被扁平化
//!
//! 复用 core/type_system/substitute.rs 中的公共替换实现

use crate::frontend::core::type_system::{MonoType, Substitution, Substituter};
use crate::frontend::type_level::evaluation::{NormalForm, ReductionConfig};
use crate::frontend::typecheck::type_eval::TypeEvaluator;
use std::collections::HashMap;

/// 范式化上下文
#[derive(Debug, Clone, Default)]
pub struct NormalizationContext {
    /// 类型替换映射
    substitutions: Substitution,

    /// 替换器实例
    substituter: Substituter,

    /// 范式化缓存
    cache: HashMap<MonoType, NormalForm>,
}

impl NormalizationContext {
    /// 创建新的范式化上下文
    pub fn new() -> Self {
        Self {
            substitutions: Substitution::new(),
            substituter: Substituter::new(),
            cache: HashMap::new(),
        }
    }

    /// 获取缓存的可变引用（用于同步）
    pub fn cache_mut(&mut self) -> &mut HashMap<MonoType, NormalForm> {
        &mut self.cache
    }

    /// 获取缓存的不可变引用
    pub fn cache(&self) -> &HashMap<MonoType, NormalForm> {
        &self.cache
    }

    /// 添加类型变量替换
    pub fn add_substitution(
        &mut self,
        index: usize,
        ty: MonoType,
    ) {
        self.substitutions.insert(index, ty);
    }

    /// 批量添加替换
    pub fn add_substitutions(
        &mut self,
        subs: HashMap<usize, MonoType>,
    ) {
        for (k, v) in subs {
            self.substitutions.insert(k, v);
        }
    }

    /// 应用替换到类型
    pub fn apply_substitution(
        &self,
        ty: &MonoType,
    ) -> MonoType {
        self.substituter.substitute(ty, &self.substitutions)
    }
}

/// 类型范式化器
///
/// 将类型表达式转换为范式形式
#[derive(Debug)]
pub struct TypeNormalizer {
    /// 范式化配置
    config: ReductionConfig,

    /// 上下文
    context: NormalizationContext,

    /// 类型求值器（用于条件类型求值）
    evaluator: TypeEvaluator,
}

impl Default for TypeNormalizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for TypeNormalizer {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            context: self.context.clone(),
            // 重新创建 evaluator，因为原始指针不能克隆
            evaluator: TypeEvaluator::new(),
        }
    }
}

impl TypeNormalizer {
    /// 创建新的范式化器
    pub fn new() -> Self {
        Self {
            config: ReductionConfig::default(),
            context: NormalizationContext::new(),
            evaluator: TypeEvaluator::new(),
        }
    }

    /// 创建带配置的范式化器
    pub fn with_config(config: ReductionConfig) -> Self {
        Self {
            config,
            context: NormalizationContext::new(),
            evaluator: TypeEvaluator::new(),
        }
    }

    /// 范式化类型
    pub fn normalize(
        &mut self,
        ty: &MonoType,
    ) -> NormalForm {
        // 检查缓存
        if let Some(cached) = self.context.cache.get(ty).cloned() {
            return cached;
        }

        let result = self.normalize_internal(ty);

        // 缓存结果
        self.context.cache.insert(ty.clone(), result.clone());

        result
    }

    /// 内部范式化逻辑
    fn normalize_internal(
        &mut self,
        ty: &MonoType,
    ) -> NormalForm {
        match ty {
            // 基本类型已经是范式
            MonoType::Void
            | MonoType::Bool
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes => NormalForm::Normalized,
            MonoType::Int(_) | MonoType::Float(_) => NormalForm::Normalized,
            MonoType::TypeVar(_) => NormalForm::NeedsReduction,
            MonoType::TypeRef(name) => {
                // 检查是否是条件类型（If, Match 等）
                if let Some(args) = self.parse_conditional_args(name) {
                    self.eval_conditional(name, &args)
                } else {
                    NormalForm::Normalized
                }
            }
            MonoType::Struct(_) | MonoType::Enum(_) => NormalForm::Normalized,
            MonoType::Tuple(types) => {
                if types
                    .iter()
                    .all(|t| matches!(self.normalize(t), NormalForm::Normalized))
                {
                    NormalForm::Normalized
                } else {
                    NormalForm::NeedsReduction
                }
            }
            MonoType::List(t) => {
                if matches!(self.normalize(t), NormalForm::Normalized) {
                    NormalForm::Normalized
                } else {
                    NormalForm::NeedsReduction
                }
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                let params_normalized = params
                    .iter()
                    .all(|p| matches!(self.normalize(p), NormalForm::Normalized));
                let ret_normalized = matches!(self.normalize(return_type), NormalForm::Normalized);

                if params_normalized && ret_normalized {
                    NormalForm::Normalized
                } else {
                    NormalForm::NeedsReduction
                }
            }
            _ => NormalForm::Normalized,
        }
    }

    /// 解析条件类型参数
    fn parse_conditional_args(
        &self,
        type_name: &str,
    ) -> Option<Vec<String>> {
        if !type_name.starts_with("If<") && !type_name.starts_with("Match<") {
            return None;
        }

        let args_str = &type_name[type_name.find('<')? + 1..type_name.rfind('>')?];
        Self::parse_type_args(args_str)
    }

    /// 解析类型参数字符串
    fn parse_type_args(args_str: &str) -> Option<Vec<String>> {
        let mut args = Vec::new();
        let mut current = String::new();
        let mut depth = 0;

        for c in args_str.chars() {
            match c {
                ',' if depth == 0 => {
                    if !current.trim().is_empty() {
                        args.push(current.trim().to_string());
                    }
                    current = String::new();
                }
                '<' => {
                    depth += 1;
                    current.push(c);
                }
                '>' => {
                    if depth == 0 {
                        return None;
                    }
                    depth -= 1;
                    current.push(c);
                }
                _ => current.push(c),
            }
        }

        if !current.trim().is_empty() {
            args.push(current.trim().to_string());
        }

        if args.is_empty() {
            None
        } else {
            Some(args)
        }
    }

    /// 求值条件类型
    fn eval_conditional(
        &mut self,
        type_name: &str,
        args: &[String],
    ) -> NormalForm {
        // 解析参数为 MonoType
        let parsed_args: Vec<MonoType> = args
            .iter()
            .filter_map(|arg| {
                self.evaluator
                    .parse_type(arg)
                    .or_else(|| Some(MonoType::TypeRef(arg.clone())))
            })
            .collect();

        if parsed_args.len() < 2 {
            return NormalForm::Normalized;
        }

        // 根据类型名称调用对应的求值方法
        match type_name {
            _ if type_name.starts_with("If<") => {
                // If<Condition, TrueBranch, FalseBranch>
                if parsed_args.len() >= 3 {
                    let result =
                        self.evaluator
                            .eval_if(&parsed_args[0], &parsed_args[1], &parsed_args[2]);
                    match result {
                        crate::frontend::typecheck::type_eval::EvalResult::Value(_) => {
                            NormalForm::Normalized
                        }
                        crate::frontend::typecheck::type_eval::EvalResult::Pending => {
                            NormalForm::NeedsReduction
                        }
                        crate::frontend::typecheck::type_eval::EvalResult::Error(_) => {
                            NormalForm::Normalized
                        }
                    }
                } else {
                    NormalForm::Normalized
                }
            }
            _ if type_name.starts_with("Match<") => {
                // Match<Target, Arm1, Arm2, ...>
                let target = &parsed_args[0];
                let arms: Vec<(MonoType, MonoType)> = parsed_args[1..]
                    .chunks(2)
                    .filter_map(|chunk| {
                        if chunk.len() == 2 {
                            Some((chunk[0].clone(), chunk[1].clone()))
                        } else {
                            None
                        }
                    })
                    .collect();

                let result = self.evaluator.eval_match(target, arms);
                match result {
                    crate::frontend::typecheck::type_eval::EvalResult::Value(_) => {
                        NormalForm::Normalized
                    }
                    crate::frontend::typecheck::type_eval::EvalResult::Pending => {
                        NormalForm::NeedsReduction
                    }
                    crate::frontend::typecheck::type_eval::EvalResult::Error(_) => {
                        NormalForm::Normalized
                    }
                }
            }
            _ => NormalForm::Normalized,
        }
    }

    /// 获取求值器（用于外部访问）
    pub fn evaluator(&mut self) -> &mut TypeEvaluator {
        &mut self.evaluator
    }

    /// 获取上下文
    pub fn context(&self) -> &NormalizationContext {
        &self.context
    }
}
