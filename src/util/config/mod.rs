//! YaoXiang configuration system
//!
//! Supports user-level and project-level configuration with merge semantics.
//!
//! # Configuration hierarchy
//!
//! ```text
//! Priority (high → low):
//! 1. CLI arguments
//! 2. Environment variables
//! 3. Project-level (yaoxiang.toml)
//! 4. User-level (~/.config/yaoxiang/config.toml)
//! 5. Default values
//! ```
//!
//! # Usage
//!
//! ```rust
//! use yaoxiang::util::config::{load_user_config, UserConfig};
//!
//! // Load user-level config (auto-creates if not exists)
//! let config = load_user_config().unwrap();
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[cfg(test)]
mod tests;

/// User-level configuration for YaoXiang
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserConfig {
    /// Internationalization settings
    #[serde(default)]
    pub i18n: I18nConfig,
    /// REPL settings
    #[serde(default)]
    pub repl: ReplConfig,
    /// Format settings
    #[serde(default)]
    pub fmt: FmtConfig,
    /// Global package cache settings (RFC-014 Phase 3)
    #[serde(default)]
    pub cache: CacheConfig,
    /// 构建信任记录（RFC-014b 5e 信任门）
    #[serde(default)]
    pub trust: TrustConfig,
}

/// 构建信任记录（RFC-014b 2026-09-15 决议 1）
///
/// 记录用户已确认信任的 build.yx（`name@version` 键）。持久化位置随用户
/// 配置体系（`~/.config/yaoxiang/config.toml`；RFC 草拟时的
/// `~/.yaoxiang/config.toml` 未曾存在，与 `[cache] dir` 同款并入）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrustConfig {
    /// 已信任的构建脚本（`name@version`）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub build_scripts: Vec<String>,
}

/// Global package cache configuration (RFC-014 Phase 3)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CacheConfig {
    /// Cache root directory (default `~/.yaoxiang/cache`)
    #[serde(default)]
    pub dir: Option<String>,
}

/// I18n configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct I18nConfig {
    /// Default language (fallback for error-lang and local-lang)
    #[serde(default = "default_lang")]
    pub lang: String,
    /// Fallback language
    #[serde(default = "default_lang")]
    pub fallback: String,
    /// Language for diagnostic error messages (src/util/diagnostic)
    #[serde(default)]
    pub error_lang: Option<String>,
    /// Language for local/misc messages (src/util/i18n)
    #[serde(default)]
    pub local_lang: Option<String>,
}

fn default_lang() -> String {
    "en".to_string()
}

impl Default for I18nConfig {
    fn default() -> Self {
        Self {
            lang: "en".to_string(),
            fallback: "en".to_string(),
            error_lang: None,
            local_lang: None,
        }
    }
}

/// REPL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplConfig {
    /// History file path
    #[serde(default)]
    pub history_file: Option<PathBuf>,
    /// Prompt string
    #[serde(default = "default_prompt")]
    pub prompt: String,
    /// Enable syntax highlighting
    #[serde(default = "default_colors")]
    pub colors: bool,
    /// Auto-import modules
    #[serde(default)]
    pub auto_imports: Vec<String>,
}

fn default_prompt() -> String {
    "yx> ".to_string()
}

fn default_colors() -> bool {
    true
}

impl Default for ReplConfig {
    fn default() -> Self {
        Self {
            history_file: None,
            prompt: "yx> ".to_string(),
            colors: true,
            auto_imports: Vec::new(),
        }
    }
}

/// Format configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FmtConfig {
    /// Line width
    #[serde(default)]
    pub line_width: Option<usize>,
    /// Indent width
    #[serde(default)]
    pub indent_width: Option<usize>,
    /// Use tabs for indentation
    #[serde(default)]
    pub use_tabs: Option<bool>,
    /// Use single quotes
    #[serde(default)]
    pub single_quote: Option<bool>,
    /// Sort import statements
    #[serde(default)]
    pub sort_imports: Option<bool>,
}

/// Project-level configuration (yaoxiang.toml)
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
    /// Format configuration
    #[serde(default)]
    pub fmt: FmtConfig,
    /// Runtime configuration
    #[serde(default)]
    pub runtime: RuntimeConfig,
    /// Tool configuration ([tool.*] extension namespace, RFC-015)
    #[serde(default)]
    pub tool: ToolConfig,
}

/// [tool.*] 扩展命名空间（RFC-015）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolConfig {
    /// [tool.test] — yaoxiang test（RFC-036）
    #[serde(default)]
    pub test: TestConfig,
}

/// [tool.test] 配置（RFC-036）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    /// 测试文件发现 pattern（字面路径或 `root/**/*.yx` 形式）
    #[serde(default = "default_test_patterns")]
    pub patterns: Vec<String>,
    /// 排除 pattern（同 patterns 形式），命中的文件从发现集剔除（RFC-036 Phase 3；
    /// 显式路径模式不读配置，故 exclude 只作用于配置发现）
    #[serde(default)]
    pub exclude: Vec<String>,
    /// 并行执行（与 CLI `--parallel` 取或；RFC-036 Phase 3）
    #[serde(default)]
    pub parallel: bool,
}

fn default_test_patterns() -> Vec<String> {
    vec!["tests/**/*.yx".to_string()]
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            patterns: default_test_patterns(),
            exclude: Vec::new(),
            parallel: false,
        }
    }
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime mode: embedded, standard, full
    #[serde(default = "default_runtime_mode")]
    pub mode: String,
    /// Number of worker threads (0 = auto-detect CPU cores)
    #[serde(default)]
    pub workers: usize,
}

fn default_runtime_mode() -> String {
    "embedded".to_string()
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            mode: "embedded".to_string(),
            workers: 0,
        }
    }
}

/// Load user-level configuration
/// Returns default config if file doesn't exist
pub fn load_user_config() -> Result<UserConfig, ConfigError> {
    let path = user_config_path();
    let path = match path {
        Some(p) => p,
        None => return Ok(UserConfig::default()),
    };
    load_user_config_from(&path)
}

/// 从指定路径加载用户配置（信任门测试注入用）
pub fn load_user_config_from(path: &std::path::Path) -> Result<UserConfig, ConfigError> {
    if !path.exists() {
        return Ok(UserConfig::default());
    }
    let content = fs::read_to_string(path).map_err(ConfigError::IoError)?;
    toml::from_str(&content).map_err(ConfigError::ParseError)
}

/// 用户配置文件路径（XDG → HOME → APPDATA；都不可用则 None）
pub fn user_config_path() -> Option<std::path::PathBuf> {
    std::env::var("XDG_CONFIG_HOME")
        .map(|xdg| {
            std::path::PathBuf::from(xdg)
                .join("yaoxiang")
                .join("config.toml")
        })
        .or_else(|_| {
            std::env::var("HOME").map(|home| {
                std::path::PathBuf::from(home)
                    .join(".config")
                    .join("yaoxiang")
                    .join("config.toml")
            })
        })
        .or_else(|_| {
            std::env::var("APPDATA").map(|appdata| {
                std::path::PathBuf::from(appdata)
                    .join("yaoxiang")
                    .join("config.toml")
            })
        })
        .ok()
}

/// 保存用户配置（目录不存在则创建）
pub fn save_user_config(config: &UserConfig) -> Result<(), ConfigError> {
    let path = user_config_path().ok_or(ConfigError::IoError(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "无法定位用户配置目录（HOME / APPDATA 均未设置）",
    )))?;
    save_user_config_to(&path, config)
}

/// 保存用户配置到指定路径（信任门测试注入用）
pub fn save_user_config_to(
    path: &std::path::Path,
    config: &UserConfig,
) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(ConfigError::IoError)?;
    }
    let content = toml::to_string_pretty(config).map_err(|e| {
        ConfigError::IoError(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    })?;
    fs::write(path, content).map_err(ConfigError::IoError)
}

/// 该构建脚本（`name@version`）是否已被用户信任
pub fn is_trusted_build_script(key: &str) -> bool {
    load_user_config()
        .map(|c| c.trust.build_scripts.iter().any(|s| s == key))
        .unwrap_or(false)
}

/// 记录信任（幂等；落盘用户配置）
pub fn add_trusted_build_script(key: &str) -> Result<(), ConfigError> {
    let mut config = load_user_config()?;
    if !config.trust.build_scripts.iter().any(|s| s == key) {
        config.trust.build_scripts.push(key.to_string());
    }
    save_user_config(&config)
}

/// Configuration errors
#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Config parse error: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}
