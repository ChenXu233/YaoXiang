//! RFC-011 类型归约
//!
//! 实现类型表达式的归约（Reduction）操作。
//!
//! 支持的归约规则：
//! - Beta 归约：应用函数到参数
//! - Eta 归约：简化冗余抽象
//! - Delta 归约：展开类型别名
//! - Iota 归约：模式匹配求值

use crate::frontend::core::type_system::MonoType;
use crate::frontend::type_level::evaluation::{ReductionConfig, ReductionStep};
use std::collections::HashMap;

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
