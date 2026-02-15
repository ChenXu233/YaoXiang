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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FmtConfig {
    /// Line width
    #[serde(default = "default_line_width")]
    pub line_width: usize,
    /// Indent width
    #[serde(default = "default_indent_width")]
    pub indent_width: usize,
    /// Use tabs for indentation
    #[serde(default)]
    pub use_tabs: bool,
    /// Use single quotes
    #[serde(default)]
    pub single_quote: bool,
}

fn default_line_width() -> usize {
    120
}

fn default_indent_width() -> usize {
    4
}

impl Default for FmtConfig {
    fn default() -> Self {
        Self {
            line_width: 120,
            indent_width: 4,
            use_tabs: false,
            single_quote: false,
        }
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
}

fn default_lint_rules() -> Vec<String> {
    vec!["recommended".to_string()]
}

impl Default for LintConfig {
    fn default() -> Self {
        Self {
            rules: vec!["recommended".to_string()],
            strict: false,
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

/// Get the user config directory
pub fn get_config_dir() -> Option<PathBuf> {
    // Try XDG config directory on Unix
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg_config).join("yaoxiang"));
    }

    // Fallback to ~/.config/yaoxiang
    if let Ok(home) = std::env::var("HOME") {
        return Some(PathBuf::from(home).join(".config").join("yaoxiang"));
    }

    // On Windows, try %APPDATA%
    if let Ok(appdata) = std::env::var("APPDATA") {
        return Some(PathBuf::from(appdata).join("yaoxiang"));
    }

    None
}

/// Get the user config file path (~/.config/yaoxiang/config.toml)
pub fn get_config_path() -> Option<PathBuf> {
    get_config_dir().map(|dir| dir.join("config.toml"))
}

/// Check if user config exists
pub fn config_exists() -> bool {
    get_config_path().map(|p| p.exists()).unwrap_or(false)
}

/// Load user-level configuration
/// Returns default config if file doesn't exist
pub fn load_user_config() -> Result<UserConfig, ConfigError> {
    let path = match get_config_path() {
        Some(p) => p,
        None => return Ok(UserConfig::default()),
    };

    if !path.exists() {
        return Ok(UserConfig::default());
    }

    let content = fs::read_to_string(&path).map_err(ConfigError::IoError)?;

    toml::from_str(&content).map_err(ConfigError::ParseError)
}

/// Load user-level config, creating default if not exists
pub fn load_or_create_user_config() -> Result<UserConfig, ConfigError> {
    let path = match get_config_path() {
        Some(p) => p,
        None => return Ok(UserConfig::default()),
    };

    if !path.exists() {
        // Create default config
        let config = UserConfig::default();
        save_user_config(&config)?;
        return Ok(config);
    }

    load_user_config()
}

/// Save user-level configuration
pub fn save_user_config(config: &UserConfig) -> Result<(), ConfigError> {
    let dir = get_config_dir().ok_or(ConfigError::NoConfigDir)?;
    let path = dir.join("config.toml");

    // Create directory if not exists
    if !dir.exists() {
        fs::create_dir_all(&dir).map_err(ConfigError::IoError)?;
    }

    let content = toml::to_string_pretty(config).map_err(ConfigError::SerializeError)?;
    fs::write(&path, content).map_err(ConfigError::IoError)?;

    Ok(())
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
