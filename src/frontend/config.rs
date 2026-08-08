//! 编译配置
//!
//! 管理编译器配置选项。仅保留实际被 pipeline 读取的配置。

use serde::{Deserialize, Serialize};

/// 单态化配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MonoConfig {
    /// 是否启用单态化
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// 最大递归深度
    #[serde(default = "default_max_mono_depth")]
    pub max_depth: usize,
}

fn default_max_mono_depth() -> usize {
    100
}

impl Default for MonoConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_depth: 100,
        }
    }
}

/// 死代码分析配置
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeadCodeConfig {
    /// 是否启用死代码分析
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for DeadCodeConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

fn default_true() -> bool {
    true
}

/// 编译配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CompileConfig {
    /// 死代码分析配置
    #[serde(default)]
    pub dead_code: DeadCodeConfig,

    /// 单态化配置
    #[serde(default)]
    pub mono: MonoConfig,
}

impl CompileConfig {
    /// 创建默认配置
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// 启用/禁用死代码分析
    #[inline]
    pub fn with_dead_code_enabled(
        mut self,
        enabled: bool,
    ) -> Self {
        self.dead_code.enabled = enabled;
        self
    }
}
