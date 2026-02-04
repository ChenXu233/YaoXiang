//! 重载解析器
//!
//! 实现函数重载的解析和选择逻辑。
//! 当存在多个同名函数时，根据调用参数的类型选择最佳匹配。

use std::collections::HashMap;
use std::fmt;

use crate::frontend::typecheck::MonoType;
use crate::middle::passes::mono::instance::{FunctionId, GenericFunctionId, SpecializationKey};

/// 重载候选函数
#[derive(Debug, Clone)]
pub struct OverloadCandidate {
    /// 函数名称
    pub name: String,
    /// 参数类型列表
    pub param_types: Vec<MonoType>,
    /// 返回类型
    pub return_type: MonoType,
    /// 泛型参数列表
    pub type_params: Vec<String>,
    /// 是否是泛型函数
    pub is_generic: bool,
}

impl OverloadCandidate {
    /// 创建新的重载候选
    pub fn new(
        name: String,
        param_types: Vec<MonoType>,
        return_type: MonoType,
        type_params: Vec<String>,
    ) -> Self {
        let is_generic = !type_params.is_empty();
        Self {
            name,
            param_types,
            return_type,
            type_params,
            is_generic,
        }
    }

    /// 检查参数数量是否匹配
    pub fn param_count_matches(
        &self,
        arg_count: usize,
    ) -> bool {
        self.param_types.len() == arg_count
    }

    /// 获取特化键
    pub fn specialization_key(&self) -> SpecializationKey {
        SpecializationKey::new_overload(self.name.clone(), self.param_types.clone(), Vec::new())
    }

    /// 获取泛型函数ID
    pub fn generic_id(&self) -> GenericFunctionId {
        GenericFunctionId::new_overload(
            self.name.clone(),
            self.param_types.clone(),
            self.type_params.clone(),
        )
    }
}

/// 重载解析结果
#[derive(Debug)]
pub enum OverloadResolution {
    /// 精确匹配
    Exact(FunctionId),
    /// 需要泛型实例化
    Generic(GenericFunctionId, Vec<MonoType>),
    /// 多个候选匹配（歧义）
    Ambiguous(Vec<OverloadCandidate>),
    /// 无匹配
    NoMatch,
}

impl fmt::Display for OverloadResolution {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            OverloadResolution::Exact(id) => write!(f, "Exact({})", id),
            OverloadResolution::Generic(id, args) => {
                let args_str: Vec<String> = args.iter().map(|t| t.type_name()).collect();
                write!(f, "Generic({}<{}>)", id.name(), args_str.join(", "))
            }
            OverloadResolution::Ambiguous(candidates) => {
                write!(f, "Ambiguous({} candidates)", candidates.len())
            }
            OverloadResolution::NoMatch => write!(f, "NoMatch"),
        }
    }
}

/// 重载解析错误
#[derive(Debug, Clone)]
pub enum OverloadError {
    /// 无匹配的定义
    NoMatchingDefinition {
        func_name: String,
        arg_types: Vec<MonoType>,
    },
    /// 多个匹配（歧义）
    AmbiguousCall {
        func_name: String,
        arg_types: Vec<MonoType>,
        candidates: Vec<MonoType>,
    },
    /// 参数数量不匹配
    ArgCountMismatch {
        func_name: String,
        expected: usize,
        actual: usize,
    },
}

impl fmt::Display for OverloadError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            OverloadError::NoMatchingDefinition {
                func_name,
                arg_types,
            } => {
                write!(
                    f,
                    "No matching definition for '{}' with arguments of types ({})",
                    func_name,
                    arg_types
                        .iter()
                        .map(|t| t.type_name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            OverloadError::AmbiguousCall {
                func_name,
                arg_types,
                candidates,
            } => {
                write!(
                    f,
                    "Ambiguous call to '{}' with arguments of types ({}). Possible matches: ({})",
                    func_name,
                    arg_types
                        .iter()
                        .map(|t| t.type_name())
                        .collect::<Vec<_>>()
                        .join(", "),
                    candidates
                        .iter()
                        .map(|t| t.type_name())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            OverloadError::ArgCountMismatch {
                func_name,
                expected,
                actual,
            } => {
                write!(
                    f,
                    "Argument count mismatch for '{}': expected {}, got {}",
                    func_name, expected, actual
                )
            }
        }
    }
}

/// 重载解析器
#[derive(Debug, Default)]
pub struct OverloadResolver {
    /// 收集的重载候选
    candidates: Vec<OverloadCandidate>,
    /// 函数名到候选索引的映射
    candidate_map: HashMap<String, Vec<usize>>,
}

impl OverloadResolver {
    /// 创建新的解析器
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
            candidate_map: HashMap::new(),
        }
    }

    /// 添加重载候选
    pub fn add_candidate(
        &mut self,
        candidate: OverloadCandidate,
    ) {
        let idx = self.candidates.len();
        self.candidates.push(candidate.clone());
        self.candidate_map
            .entry(candidate.name.clone())
            .or_default()
            .push(idx);
    }

    /// 批量添加候选
    pub fn add_candidates(
        &mut self,
        candidates: Vec<OverloadCandidate>,
    ) {
        for candidate in candidates {
            self.add_candidate(candidate);
        }
    }

    /// 获取函数的所有候选
    pub fn get_candidates(
        &self,
        name: &str,
    ) -> Option<&[usize]> {
        self.candidate_map.get(name).map(|v| v.as_slice())
    }

    /// 解析重载调用
    ///
    /// # Arguments
    /// * `name` - 函数名
    /// * `arg_types` - 参数类型列表
    ///
    /// # Returns
    /// 解析结果，包含最佳匹配或错误信息
    pub fn resolve(
        &self,
        name: &str,
        arg_types: &[MonoType],
    ) -> Result<&OverloadCandidate, OverloadError> {
        let candidate_indices = match self.candidate_map.get(name) {
            Some(indices) => indices,
            None => {
                return Err(OverloadError::NoMatchingDefinition {
                    func_name: name.to_string(),
                    arg_types: arg_types.to_vec(),
                });
            }
        };

        // 筛选参数数量匹配的候选
        let matching: Vec<&OverloadCandidate> = candidate_indices
            .iter()
            .map(|idx| &self.candidates[*idx])
            .filter(|c| c.param_count_matches(arg_types.len()))
            .collect();

        if matching.is_empty() {
            // 参数数量不匹配
            let expected = candidate_indices
                .first()
                .map(|idx| self.candidates[*idx].param_types.len())
                .unwrap_or(0);
            return Err(OverloadError::ArgCountMismatch {
                func_name: name.to_string(),
                expected,
                actual: arg_types.len(),
            });
        }

        // 评估类型匹配
        let scored: Vec<(f64, &OverloadCandidate)> = matching
            .iter()
            .map(|c| (self.score_match(c, arg_types), *c))
            .filter(|(score, _)| *score >= 0.0) // 负分表示不匹配
            .collect();

        if scored.is_empty() {
            return Err(OverloadError::NoMatchingDefinition {
                func_name: name.to_string(),
                arg_types: arg_types.to_vec(),
            });
        }

        // 找出最高分
        let max_score = scored.iter().map(|(s, _)| *s).fold(f64::MIN, f64::max);
        let best: Vec<&OverloadCandidate> = scored
            .iter()
            .filter(|(s, _)| *s == max_score)
            .map(|(_, c)| *c)
            .collect();

        if best.len() == 1 {
            Ok(best[0])
        } else {
            // 歧义：多个候选有相同分数
            Err(OverloadError::AmbiguousCall {
                func_name: name.to_string(),
                arg_types: arg_types.to_vec(),
                candidates: best.iter().map(|c| c.return_type.clone()).collect(),
            })
        }
    }

    /// 评估候选函数的匹配程度
    ///
    /// 返回分数：
    /// - 精确匹配：1.0
    /// - 子类型匹配：0.8
    /// - 泛型实例化：0.5
    /// - 不匹配：-1.0
    fn score_match(
        &self,
        candidate: &OverloadCandidate,
        arg_types: &[MonoType],
    ) -> f64 {
        let mut score = 1.0;

        for (param, arg) in candidate.param_types.iter().zip(arg_types.iter()) {
            match self.type_match_score(param, arg) {
                s if s < 0.0 => return -1.0, // 不匹配
                s => score *= s,
            }
        }

        score
    }

    /// 评估单个类型的匹配分数
    #[allow(clippy::only_used_in_recursion)]
    fn type_match_score(
        &self,
        param: &MonoType,
        arg: &MonoType,
    ) -> f64 {
        // 精确匹配
        if param == arg {
            return 1.0;
        }

        // 泛型参数匹配（使用类型变量）
        if let MonoType::TypeVar(_) = param {
            return 0.8;
        }

        // 结构体匹配（检查名称）
        if let (MonoType::Struct(p), MonoType::Struct(a)) = (param, arg) {
            if p.name == a.name {
                return 0.9;
            }
        }

        // 枚举匹配
        if let (MonoType::Enum(p), MonoType::Enum(a)) = (param, arg) {
            if p.name == a.name {
                return 0.9;
            }
        }

        // 容器类型匹配
        match (param, arg) {
            (MonoType::List(p), MonoType::List(a)) => 0.9 * self.type_match_score(p, a),
            (MonoType::Dict(pk, pv), MonoType::Dict(ak, av)) => {
                0.9 * self.type_match_score(pk, ak) * self.type_match_score(pv, av)
            }
            (MonoType::Set(p), MonoType::Set(a)) => 0.9 * self.type_match_score(p, a),
            (MonoType::Tuple(ps), MonoType::Tuple(as_)) if ps.len() == as_.len() => ps
                .iter()
                .zip(as_.iter())
                .map(|(p, a)| self.type_match_score(p, a))
                .product(),
            (
                MonoType::Fn {
                    params: pp,
                    return_type: pr,
                    ..
                },
                MonoType::Fn {
                    params: ap,
                    return_type: ar,
                    ..
                },
            ) if pp.len() == ap.len() => {
                0.9 * pp
                    .iter()
                    .zip(ap.iter())
                    .map(|(p, a)| self.type_match_score(p, a))
                    .product::<f64>()
                    * self.type_match_score(pr, ar)
            }
            _ => -1.0,
        }
    }

    /// 检查类型是否兼容
    pub fn is_compatible(
        &self,
        param: &MonoType,
        arg: &MonoType,
    ) -> bool {
        self.type_match_score(param, arg) >= 0.0
    }

    /// 获取候选数量
    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// 清空候选
    pub fn clear(&mut self) {
        self.candidates.clear();
        self.candidate_map.clear();
    }
}

/// 重载管理器
///
/// 管理全局重载解析器，支持跨模块重载
#[derive(Debug, Default)]
pub struct OverloadManager {
    /// 全局解析器
    global_resolver: OverloadResolver,
    /// 模块级解析器
    module_resolvers: HashMap<String, OverloadResolver>,
}

impl OverloadManager {
    /// 创建新的管理器
    pub fn new() -> Self {
        Self {
            global_resolver: OverloadResolver::new(),
            module_resolvers: HashMap::new(),
        }
    }

    /// 添加全局重载候选
    pub fn add_global_candidate(
        &mut self,
        candidate: OverloadCandidate,
    ) {
        self.global_resolver.add_candidate(candidate);
    }

    /// 添加模块级重载候选
    pub fn add_module_candidate(
        &mut self,
        module_name: &str,
        candidate: OverloadCandidate,
    ) {
        self.module_resolvers
            .entry(module_name.to_string())
            .or_default()
            .add_candidate(candidate);
    }

    /// 解析调用（优先本地，后全局）
    pub fn resolve(
        &self,
        module_name: Option<&str>,
        name: &str,
        arg_types: &[MonoType],
    ) -> Result<&OverloadCandidate, OverloadError> {
        // 先尝试模块级解析器
        if let Some(mod_name) = module_name {
            if let Some(resolver) = self.module_resolvers.get(mod_name) {
                if let Ok(candidate) = resolver.resolve(name, arg_types) {
                    return Ok(candidate);
                }
            }
        }

        // 回退到全局解析器
        self.global_resolver.resolve(name, arg_types)
    }

    /// 清空所有候选
    pub fn clear(&mut self) {
        self.global_resolver.clear();
        self.module_resolvers.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int_type() -> MonoType {
        MonoType::Int(32)
    }

    fn float_type() -> MonoType {
        MonoType::Float(64)
    }

    fn string_type() -> MonoType {
        MonoType::String
    }

    #[test]
    fn test_overload_candidate_creation() {
        let candidate = OverloadCandidate::new(
            "add".to_string(),
            vec![int_type(), int_type()],
            int_type(),
            vec![],
        );

        assert_eq!(candidate.name, "add");
        assert_eq!(candidate.param_types.len(), 2);
        assert!(!candidate.is_generic);
    }

    #[test]
    fn test_overload_resolution_exact() {
        let mut resolver = OverloadResolver::new();

        // 添加重载候选
        resolver.add_candidate(OverloadCandidate::new(
            "add".to_string(),
            vec![int_type(), int_type()],
            int_type(),
            vec![],
        ));

        resolver.add_candidate(OverloadCandidate::new(
            "add".to_string(),
            vec![float_type(), float_type()],
            float_type(),
            vec![],
        ));

        // 精确匹配 Int 版本
        let result = resolver.resolve("add", &[int_type(), int_type()]);
        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert_eq!(candidate.param_types[0], int_type());
    }

    #[test]
    fn test_overload_resolution_ambiguous() {
        let mut resolver = OverloadResolver::new();

        // 添加两个兼容的候选
        resolver.add_candidate(OverloadCandidate::new(
            "identity".to_string(),
            vec![int_type()],
            int_type(),
            vec!["T".to_string()],
        ));

        resolver.add_candidate(OverloadCandidate::new(
            "identity".to_string(),
            vec![float_type()],
            float_type(),
            vec!["T".to_string()],
        ));

        // 使用 int_type 调用，两者都匹配
        let result = resolver.resolve("identity", &[int_type()]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_overload_resolution_no_match() {
        let mut resolver = OverloadResolver::new();

        resolver.add_candidate(OverloadCandidate::new(
            "add".to_string(),
            vec![int_type(), int_type()],
            int_type(),
            vec![],
        ));

        // 使用不兼容的类型
        let result = resolver.resolve("add", &[string_type(), int_type()]);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            OverloadError::NoMatchingDefinition { .. }
        ));
    }

    #[test]
    fn test_type_match_score() {
        let resolver = OverloadResolver::new();

        // 精确匹配
        assert_eq!(resolver.type_match_score(&int_type(), &int_type()), 1.0);

        // 不匹配
        assert_eq!(resolver.type_match_score(&int_type(), &float_type()), -1.0);
    }
}
