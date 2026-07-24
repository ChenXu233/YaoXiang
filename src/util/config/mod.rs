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
    /// Lint settings
    #[serde(default)]
    pub lint: LintConfig,
    /// Install settings
    #[serde(default)]
    pub install: InstallConfig,
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
    /// History size
    #[serde(default = "default_history_size")]
    pub history_size: usize,
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

fn default_history_size() -> usize {
    1000
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
            history_size: 1000,
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

/// Warning level for lints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WarningLevel {
    /// Disable the warning
    Off,
    /// Show as warning (default)
    #[default]
    Warn,
    /// Treat as error
    Deny,
}

impl WarningLevel {
    pub fn is_enabled(&self) -> bool {
        !matches!(self, WarningLevel::Off)
    }

    pub fn is_deny(&self) -> bool {
        matches!(self, WarningLevel::Deny)
    }
}

/// Lint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LintConfig {
    /// Rule sets
    #[serde(default = "default_lint_rules")]
    pub rules: Vec<String>,
    /// Strict mode
    #[serde(default)]
    pub strict: bool,
    /// Dead code analysis level
    #[serde(default)]
    pub dead_code: WarningLevel,
}

fn default_lint_rules() -> Vec<String> {
    vec!["recommended".to_string()]
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            rules: vec!["recommended".to_string()],
            strict: false,
            dead_code: WarningLevel::default(),
        }
    }
}

/// Install configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstallConfig {
    /// Global install directory
    #[serde(default)]
    pub dir: Option<PathBuf>,
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
    let path = std::env::var("XDG_CONFIG_HOME")
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
        .ok();
    let path = match path {
        Some(p) => p,
        None => return Ok(UserConfig::default()),
    };
    if !path.exists() {
        return Ok(UserConfig::default());
    }
    let content = fs::read_to_string(&path).map_err(ConfigError::IoError)?;
    toml::from_str(&content).map_err(ConfigError::ParseError)
}

/// Configuration errors
#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(toml::de::Error),
    SerializeError(toml::ser::Error),
    NoConfigDir,
}

impl std::fmt::Display for ConfigError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            ConfigError::IoError(e) => write!(f, "IO error: {}", e),
            ConfigError::ParseError(e) => write!(f, "Config parse error: {}", e),
            ConfigError::SerializeError(e) => write!(f, "Config serialize error: {}", e),
            ConfigError::NoConfigDir => write!(f, "Cannot determine config directory"),
        }
    }
}

impl std::error::Error for ConfigError {}
