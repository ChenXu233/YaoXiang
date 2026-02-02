//! 编译配置
//!
//! 管理编译器配置选项，包括优化级别、诊断级别、RFC特性开关等。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 优化级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum OptLevel {
    /// 不优化
    O0,
    /// 基本优化
    #[default]
    O1,
    /// 标准优化
    O2,
    /// 激进优化
    O3,
    /// 自动选择
    Auto,
}

impl std::fmt::Display for OptLevel {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            OptLevel::O0 => write!(f, "O0"),
            OptLevel::O1 => write!(f, "O1"),
            OptLevel::O2 => write!(f, "O2"),
            OptLevel::O3 => write!(f, "O3"),
            OptLevel::Auto => write!(f, "Auto"),
        }
    }
}

/// 诊断级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub enum DiagLevel {
    /// 不显示诊断
    None,
    /// 只显示错误
    Errors,
    /// 显示错误和警告
    Warnings,
    /// 显示所有诊断信息
    #[default]
    All,
}

impl std::fmt::Display for DiagLevel {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            DiagLevel::None => write!(f, "none"),
            DiagLevel::Errors => write!(f, "errors"),
            DiagLevel::Warnings => write!(f, "warnings"),
            DiagLevel::All => write!(f, "all"),
        }
    }
}

/// RFC 特性标志（预留）
///
/// 这些字段预留给未来 RFC 特性开关的启用/禁用控制。
/// 当前版本强制默认开启所有特性。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// RFC-004: 多位置绑定语法
    #[serde(default = "default_true")]
    pub rfc004_bindings: bool,

    /// RFC-010: 统一语法
    #[serde(default = "default_true")]
    pub rfc010_unified_syntax: bool,

    /// RFC-011: 泛型系统
    #[serde(default = "default_true")]
    pub rfc011_generics: bool,

    /// RFC-011: 类型级计算
    #[serde(default = "default_true")]
    pub rfc011_type_level: bool,

    /// RFC-011: Const 泛型
    #[serde(default = "default_true")]
    pub rfc011_const_generics: bool,

    /// RFC-011: 特质系统
    #[serde(default = "default_true")]
    pub rfc011_traits: bool,

    /// 未来 RFC 预留字段
    #[serde(default)]
    pub _future: (),
}

fn default_true() -> bool {
    true
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            rfc004_bindings: true,
            rfc010_unified_syntax: true,
            rfc011_generics: true,
            rfc011_type_level: true,
            rfc011_const_generics: true,
            rfc011_traits: true,
            _future: (),
        }
    }
}

impl FeatureFlags {
    /// 检查 RFC-004 是否启用
    #[inline]
    pub fn has_rfc004(&self) -> bool {
        self.rfc004_bindings
    }

    /// 检查 RFC-010 是否启用
    #[inline]
    pub fn has_rfc010(&self) -> bool {
        self.rfc010_unified_syntax
    }

    /// 检查 RFC-011 是否启用
    #[inline]
    pub fn has_rfc011(&self) -> bool {
        self.rfc011_generics
    }

    /// 检查类型级计算是否启用
    #[inline]
    pub fn has_type_level(&self) -> bool {
        self.rfc011_type_level
    }

    /// 检查 Const 泛型是否启用
    #[inline]
    pub fn has_const_generics(&self) -> bool {
        self.rfc011_const_generics
    }

    /// 检查特质系统是否启用
    #[inline]
    pub fn has_traits(&self) -> bool {
        self.rfc011_traits
    }
}

/// 错误恢复策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ErrorRecoveryStrategy {
    /// 不恢复，遇到错误立即停止
    None,
    /// 跳过错误行继续编译
    #[default]
    SkipLine,
    /// 跳过错误函数继续编译
    SkipFunction,
    /// 跳过错误文件继续编译
    SkipFile,
    /// 尽可能恢复，继续编译
    Aggressive,
}

/// 增量编译配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncrementalConfig {
    /// 是否启用增量编译
    #[serde(default)]
    pub enabled: bool,

    /// 缓存目录
    #[serde(default)]
    pub cache_dir: Option<PathBuf>,

    /// 缓存最大大小（字节）
    #[serde(default = "default_cache_size")]
    pub max_cache_size: u64,

    /// 缓存过期时间（秒）
    #[serde(default = "default_cache_ttl")]
    pub cache_ttl: u64,
}

fn default_cache_size() -> u64 {
    100 * 1024 * 1024 // 100MB
}

fn default_cache_ttl() -> u64 {
    24 * 60 * 60 // 24小时
}

impl Default for IncrementalConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_dir: None,
            max_cache_size: default_cache_size(),
            cache_ttl: default_cache_ttl(),
        }
    }
}

/// 编译配置
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct CompileConfig {
    /// 优化级别
    #[serde(default)]
    pub optimization_level: OptLevel,

    /// 诊断级别
    #[serde(default)]
    pub diagnostic_level: DiagLevel,

    /// RFC 特性标志
    #[serde(default)]
    pub features: FeatureFlags,

    /// 错误恢复策略
    #[serde(default)]
    pub error_recovery: ErrorRecoveryStrategy,

    /// 增量编译配置
    #[serde(default)]
    pub incremental: IncrementalConfig,

    /// 是否启用详细日志
    #[serde(default)]
    pub verbose: bool,

    /// 源文件根目录（用于解析导入）
    #[serde(default)]
    pub source_root: Option<PathBuf>,

    /// 导入搜索路径
    #[serde(default)]
    pub import_paths: Vec<PathBuf>,

    /// 允许的危险特性（需用户确认）
    #[serde(default)]
    pub allow_unsafe: bool,

    /// 未来扩展字段
    #[serde(default)]
    pub _future: (),
}

impl CompileConfig {
    /// 创建默认配置
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置优化级别
    #[inline]
    pub fn with_opt_level(
        mut self,
        level: OptLevel,
    ) -> Self {
        self.optimization_level = level;
        self
    }

    /// 设置诊断级别
    #[inline]
    pub fn with_diag_level(
        mut self,
        level: DiagLevel,
    ) -> Self {
        self.diagnostic_level = level;
        self
    }

    /// 设置错误恢复策略
    #[inline]
    pub fn with_error_recovery(
        mut self,
        strategy: ErrorRecoveryStrategy,
    ) -> Self {
        self.error_recovery = strategy;
        self
    }

    /// 启用/禁用增量编译
    #[inline]
    pub fn with_incremental(
        mut self,
        enabled: bool,
    ) -> Self {
        self.incremental.enabled = enabled;
        self
    }

    /// 设置缓存目录
    #[inline]
    pub fn with_cache_dir(
        mut self,
        dir: PathBuf,
    ) -> Self {
        self.incremental.cache_dir = Some(dir);
        self
    }

    /// 添加导入路径
    #[inline]
    pub fn add_import_path(
        mut self,
        path: PathBuf,
    ) -> Self {
        self.import_paths.push(path);
        self
    }

    /// 设置源文件根目录
    #[inline]
    pub fn with_source_root(
        mut self,
        root: PathBuf,
    ) -> Self {
        self.source_root = Some(root);
        self
    }

    /// 启用详细日志
    #[inline]
    pub fn verbose(
        mut self,
        verbose: bool,
    ) -> Self {
        self.verbose = verbose;
        self
    }

    /// 启用所有 RFC 特性（默认）
    #[inline]
    pub fn all_features(self) -> Self {
        self
    }

    /// 检查是否应该显示诊断
    #[inline]
    pub fn should_show_diagnostics(&self) -> bool {
        self.diagnostic_level >= DiagLevel::Errors
    }

    /// 检查是否应该显示警告
    #[inline]
    pub fn should_show_warnings(&self) -> bool {
        self.diagnostic_level >= DiagLevel::Warnings
    }

    /// 检查是否应该显示所有诊断
    #[inline]
    pub fn should_show_all(&self) -> bool {
        self.diagnostic_level >= DiagLevel::All
    }
}

/// 从外部配置继承的配置适配器
///
/// 如果未来有全局配置，可以实现这个 trait 来从全局配置创建 CompileConfig。
pub trait ConfigAdapter {
    /// 从源配置创建编译配置
    fn adapt(&self) -> CompileConfig;
}

/// 空的配置适配器（使用默认值）
impl ConfigAdapter for () {
    fn adapt(&self) -> CompileConfig {
        CompileConfig::new()
    }
}

/// JSON 配置文件的配置适配器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonConfig {
    #[serde(default)]
    pub optimization_level: OptLevel,

    #[serde(default)]
    pub diagnostic_level: DiagLevel,

    #[serde(default)]
    pub features: FeatureFlags,

    #[serde(default)]
    pub error_recovery: ErrorRecoveryStrategy,

    #[serde(default)]
    pub incremental: IncrementalConfig,

    #[serde(default)]
    pub import_paths: Vec<PathBuf>,
}

impl ConfigAdapter for JsonConfig {
    fn adapt(&self) -> CompileConfig {
        CompileConfig {
            optimization_level: self.optimization_level,
            diagnostic_level: self.diagnostic_level,
            features: self.features.clone(),
            error_recovery: self.error_recovery,
            incremental: self.incremental.clone(),
            verbose: false,
            source_root: None,
            import_paths: self.import_paths.clone(),
            allow_unsafe: false,
            _future: (),
        }
    }
}

impl From<JsonConfig> for CompileConfig {
    fn from(config: JsonConfig) -> Self {
        config.adapt()
    }
}
