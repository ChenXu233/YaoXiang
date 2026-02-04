//! 平台特化器
//!
//! 根据目标平台过滤和选择泛型实例化
//!
//! # RFC-011 平台特化设计
//!
//! - `P` 是预定义泛型参数名，被解析器占用
//! - `[P: X86_64]` 表示当前平台是 X86_64 时的特化
//! - 编译器自动选择匹配的特化
//!
//! # 使用示例
//!
//! ```yaoxiang
//! # 通用实现（所有平台可用）
//! sum: [T: Add](arr: Array[T]) -> T = { ... }
//!
//! # 平台特化：P 是预定义泛型参数，代表当前平台
//! sum: [P: X86_64](arr: Array[Float]) -> Float = {
//!     return avx2_sum(arr.data, arr.length)
//! }
//!
//! sum: [P: AArch64](arr: Array[Float]) -> Float = {
//!     return neon_sum(arr.data, arr.length)
//! }
//! ```

use super::platform_info::{PlatformInfo, TargetPlatform};
use crate::frontend::core::parser::ast::GenericParam;
use crate::frontend::typecheck::MonoType;
use std::collections::HashMap;

/// 平台约束
///
/// 表示泛型函数上的平台约束条件
/// 例如 `[P: X86_64]` 表示该特化仅在 X86_64 平台上有效
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PlatformConstraint {
    /// 约束的平台类型名（如 "X86_64"）
    platform_type: String,
}

impl PlatformConstraint {
    /// 创建新的平台约束
    pub fn new(platform_type: String) -> Self {
        PlatformConstraint { platform_type }
    }

    /// 获取约束的平台类型名
    pub fn platform_type(&self) -> &str {
        &self.platform_type
    }

    /// 检查约束是否匹配给定平台
    pub fn matches(
        &self,
        platform: &TargetPlatform,
    ) -> bool {
        // 通配符约束匹配所有平台
        if self.is_any() {
            return true;
        }
        self.platform_type == platform.as_str()
    }

    /// 创建匹配任意平台的约束
    pub fn any() -> Self {
        PlatformConstraint {
            platform_type: "*".to_string(),
        }
    }

    /// 检查是否是通配符约束
    pub fn is_any(&self) -> bool {
        self.platform_type == "*"
    }
}

impl From<&str> for PlatformConstraint {
    fn from(s: &str) -> Self {
        PlatformConstraint::new(s.to_string())
    }
}

impl From<String> for PlatformConstraint {
    fn from(s: String) -> Self {
        PlatformConstraint::new(s)
    }
}

/// 函数平台的特化信息
///
/// 存储单个函数的平台特化相关信息
#[derive(Debug, Clone)]
pub struct FunctionPlatformInfo {
    /// 函数名称
    name: String,

    /// 泛型参数列表
    generic_params: Vec<GenericParam>,

    /// 平台约束（如果有）
    platform_constraint: Option<PlatformConstraint>,

    /// 优先级（用于多个约束匹配时的选择）
    priority: i32,
}

impl FunctionPlatformInfo {
    /// 创建新的函数平台信息
    pub fn new(
        name: String,
        generic_params: Vec<GenericParam>,
        platform_constraint: Option<PlatformConstraint>,
    ) -> Self {
        let priority = match &platform_constraint {
            Some(c) if !c.is_any() => 10, // 具体平台约束优先级高
            Some(c) if c.is_any() => 0,   // 通配符优先级低
            Some(_) => 0,                 // 其他 Some 情况
            None => 0,                    // 无约束优先级最低
        };

        FunctionPlatformInfo {
            name,
            generic_params,
            platform_constraint,
            priority,
        }
    }

    /// 获取函数名称
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 获取泛型参数
    pub fn generic_params(&self) -> &[GenericParam] {
        &self.generic_params
    }

    /// 获取平台约束
    pub fn platform_constraint(&self) -> Option<&PlatformConstraint> {
        self.platform_constraint.as_ref()
    }

    /// 获取优先级
    pub fn priority(&self) -> i32 {
        self.priority
    }
}

/// 平台特化器
///
/// 根据目标平台选择正确的泛型特化版本
#[derive(Debug)]
pub struct PlatformSpecializer {
    /// 当前目标平台信息
    current_platform: PlatformInfo,

    /// 函数平台信息缓存
    function_infos: HashMap<String, Vec<FunctionPlatformInfo>>,
}

impl PlatformSpecializer {
    /// 创建新的平台特化器
    pub fn new(platform_info: PlatformInfo) -> Self {
        PlatformSpecializer {
            current_platform: platform_info,
            function_infos: HashMap::new(),
        }
    }

    /// 创建从配置的平台特化器
    pub fn from_config(platform_info: &PlatformInfo) -> Self {
        Self::new(platform_info.clone())
    }

    /// 注册函数的平台特化信息
    pub fn register_function(
        &mut self,
        info: FunctionPlatformInfo,
    ) {
        self.function_infos
            .entry(info.name.clone())
            .or_default()
            .push(info);
    }

    /// 从泛型参数中提取平台约束
    ///
    /// # Arguments
    ///
    /// * `generic_params` - 泛型参数列表
    ///
    /// # Returns
    ///
    /// 平台约束（如果有），以及不包含平台参数的泛型参数列表
    pub fn extract_platform_constraint(
        generic_params: &[GenericParam]
    ) -> (Option<PlatformConstraint>, Vec<GenericParam>) {
        use crate::frontend::core::parser::ast::GenericParamKind;

        let mut platform_constraint: Option<PlatformConstraint> = None;
        let mut filtered_params: Vec<GenericParam> = Vec::new();

        for param in generic_params {
            // RFC-011: 检查是否是预定义平台参数 P
            // P 是预定义泛型参数，用于平台特化
            if param.name == "P" || matches!(param.kind, GenericParamKind::Platform) {
                // 约束类型即为平台类型
                if let Some(constraint) = param.constraints.first() {
                    let platform_type = Self::extract_platform_type(constraint);
                    platform_constraint = Some(PlatformConstraint::new(platform_type));
                } else {
                    // 没有约束，表示匹配任意平台
                    platform_constraint = Some(PlatformConstraint::any());
                }
            } else {
                // 普通泛型参数，保留
                filtered_params.push(param.clone());
            }
        }

        (platform_constraint, filtered_params)
    }

    /// 从类型表达式中提取平台类型名
    fn extract_platform_type(ty: &crate::frontend::core::parser::ast::Type) -> String {
        use crate::frontend::core::parser::ast::Type;

        match ty {
            Type::Name(name) => name.clone(),
            Type::Generic { name, args: _ } => name.clone(),
            _ => {
                // 尝试获取字符串表示
                format!("{:?}", ty)
            }
        }
    }

    /// 获取当前目标平台
    pub fn current_platform(&self) -> &PlatformInfo {
        &self.current_platform
    }

    /// 获取当前目标平台的类型名
    pub fn current_platform_type(&self) -> String {
        self.current_platform.platform_type_name()
    }

    /// 获取当前目标平台
    pub fn target_platform(&self) -> &TargetPlatform {
        self.current_platform.target()
    }

    /// 为函数选择最佳特化版本
    ///
    /// # Arguments
    ///
    /// * `func_name` - 函数名称
    /// * `platform_args` - 平台相关的类型参数
    ///
    /// # Returns
    ///
    /// 最佳匹配的特化版本信息（如果有）
    pub fn select_specialization(
        &self,
        func_name: &str,
        _platform_args: &[MonoType],
    ) -> Option<&FunctionPlatformInfo> {
        let infos = self.function_infos.get(func_name)?;
        let current_platform = self.target_platform();

        // 找到所有匹配的特化版本
        let matches: Vec<&FunctionPlatformInfo> = infos
            .iter()
            .filter(|info| {
                if let Some(constraint) = &info.platform_constraint {
                    constraint.matches(current_platform)
                } else {
                    // 没有平台约束的版本始终匹配
                    true
                }
            })
            .collect();

        if matches.is_empty() {
            return None;
        }

        // 按优先级排序，返回优先级最高的
        matches.into_iter().max_by_key(|info| info.priority())
    }

    /// 检查函数是否有平台特化版本
    pub fn has_platform_specialization(
        &self,
        func_name: &str,
    ) -> bool {
        self.function_infos
            .get(func_name)
            .map(|infos| infos.iter().any(|i| i.platform_constraint.is_some()))
            .unwrap_or(false)
    }

    /// 获取所有平台特化版本
    pub fn get_specializations(
        &self,
        func_name: &str,
    ) -> Option<&[FunctionPlatformInfo]> {
        self.function_infos.get(func_name).map(|v| v.as_slice())
    }

    /// 设置目标平台（用于测试）
    pub fn set_target_platform(
        &mut self,
        platform: TargetPlatform,
    ) {
        let platform_str = platform.as_str().to_string();
        self.current_platform = super::platform_info::PlatformInfo::new(platform, platform_str);
    }
}

/// 平台约束求解器
///
/// 用于检查泛型实例化是否满足平台约束
#[derive(Debug, Clone)]
pub struct PlatformConstraintSolver {
    /// 当前目标平台
    current_platform: TargetPlatform,
}

impl PlatformConstraintSolver {
    /// 创建新的约束求解器
    pub fn new(platform: &PlatformInfo) -> Self {
        PlatformConstraintSolver {
            current_platform: platform.target().clone(),
        }
    }

    /// 检查约束是否满足
    pub fn satisfies(
        &self,
        constraint: &PlatformConstraint,
    ) -> bool {
        constraint.matches(&self.current_platform)
    }

    /// 批量检查约束
    pub fn satisfies_all<'a>(
        &self,
        mut constraints: impl Iterator<Item = &'a PlatformConstraint>,
    ) -> bool {
        constraints.all(|c| self.satisfies(c))
    }
}

/// 特化决策结果
///
/// 表示泛型实例化的特化决策
#[derive(Debug, Clone)]
pub struct SpecializationDecision {
    /// 是否应该特化
    should_specialize: bool,

    /// 使用的平台约束（如果有）
    applied_constraint: Option<PlatformConstraint>,

    /// 原因说明
    reason: String,
}

impl SpecializationDecision {
    /// 创建应该特化的决策
    pub fn specialize(
        constraint: Option<PlatformConstraint>,
        reason: String,
    ) -> Self {
        SpecializationDecision {
            should_specialize: true,
            applied_constraint: constraint,
            reason,
        }
    }

    /// 创建不应该特化的决策
    pub fn skip(reason: String) -> Self {
        SpecializationDecision {
            should_specialize: false,
            applied_constraint: None,
            reason,
        }
    }

    /// 检查是否应该特化
    pub fn should_specialize(&self) -> bool {
        self.should_specialize
    }

    /// 获取应用约束
    pub fn applied_constraint(&self) -> Option<&PlatformConstraint> {
        self.applied_constraint.as_ref()
    }

    /// 获取原因
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

/// 平台特化决策器
///
/// 根据平台信息做出特化决策
#[derive(Debug)]
pub struct SpecializationDecider {
    /// 目标平台
    target_platform: TargetPlatform,
}

impl SpecializationDecider {
    /// 创建新的决策器
    pub fn new(platform_info: &PlatformInfo) -> Self {
        SpecializationDecider {
            target_platform: platform_info.target().clone(),
        }
    }

    /// 决定是否应该应用特化
    pub fn decide(
        &self,
        constraint: &PlatformConstraint,
    ) -> SpecializationDecision {
        if constraint.matches(&self.target_platform) {
            SpecializationDecision::specialize(
                Some(constraint.clone()),
                format!(
                    "Platform '{}' matches constraint '{}'",
                    self.target_platform,
                    constraint.platform_type()
                ),
            )
        } else {
            SpecializationDecision::skip(format!(
                "Platform '{}' does not match constraint '{}'",
                self.target_platform,
                constraint.platform_type()
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_constraint_matches() {
        let constraint = PlatformConstraint::new("X86_64".to_string());

        assert!(constraint.matches(&TargetPlatform::X86_64));
        assert!(!constraint.matches(&TargetPlatform::AArch64));
    }

    #[test]
    fn test_wildcard_constraint() {
        let constraint = PlatformConstraint::any();

        assert!(constraint.is_any());
        assert!(constraint.matches(&TargetPlatform::X86_64));
        assert!(constraint.matches(&TargetPlatform::AArch64));
    }

    #[test]
    fn test_extract_platform_constraint() {
        use crate::frontend::core::parser::ast::GenericParamKind;

        let param_p = GenericParam {
            name: "P".to_string(),
            kind: GenericParamKind::Type,
            constraints: vec![crate::frontend::core::parser::ast::Type::Name(
                "X86_64".to_string(),
            )],
        };

        let param_t = GenericParam {
            name: "T".to_string(),
            kind: GenericParamKind::Type,
            constraints: vec![crate::frontend::core::parser::ast::Type::Name(
                "Clone".to_string(),
            )],
        };

        let params = vec![param_p.clone(), param_t.clone()];

        let (constraint, filtered) = PlatformSpecializer::extract_platform_constraint(&params);

        assert!(constraint.is_some());
        assert_eq!(constraint.unwrap().platform_type(), "X86_64");
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "T");
    }

    #[test]
    fn test_select_specialization() {
        let platform_info =
            super::super::platform_info::PlatformDetector::detect_from_target("x86_64-linux-gnu");
        let mut specializer = PlatformSpecializer::new(platform_info);

        // 注册两个特化版本
        specializer.register_function(FunctionPlatformInfo::new(
            "sum".to_string(),
            vec![],
            Some(PlatformConstraint::new("X86_64".to_string())),
        ));

        specializer.register_function(FunctionPlatformInfo::new(
            "sum".to_string(),
            vec![],
            Some(PlatformConstraint::new("AArch64".to_string())),
        ));

        // 在 X86_64 平台上，应该选择 X86_64 版本
        let selected = specializer.select_specialization("sum", &[]);
        assert!(selected.is_some());
        assert_eq!(
            selected
                .unwrap()
                .platform_constraint()
                .unwrap()
                .platform_type(),
            "X86_64"
        );
    }
}
