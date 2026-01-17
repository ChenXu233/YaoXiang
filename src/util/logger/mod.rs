//! Logger module for YaoXiang
//!
//! Go-style simple logging: `[LEVEL] message`
//!
//! # Usage
//!
//! ```rust
//! use yaoxiang::util::logger;
//!
//! logger::init();
//! tracing::info!("Hello, {}", "world");
//! ```

use std::sync::atomic::{AtomicU8, Ordering};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry};

use crate::util::i18n::current_lang;

/// Global language setting for i18n (stored as atomic u8 for thread-safe access)
static CURRENT_LANG: AtomicU8 = AtomicU8::new(0);

/// Set the current language for i18n (for backward compatibility)
pub fn set_lang(lang_code: &str) {
    // Map lang code to u8: en=0, zh=1, zh-x-miao=2, others=0
    let val = match lang_code {
        "zh" => 1,
        "zh-x-miao" | "zh-miao" => 2,
        _ => 0,
    };
    CURRENT_LANG.store(val, Ordering::SeqCst);
}

/// Get the current language for i18n (for backward compatibility)
pub fn get_lang() -> &'static str {
    let val = CURRENT_LANG.load(Ordering::SeqCst);
    match val {
        1 => "zh",
        2 => "zh-x-miao",
        _ => current_lang(),
    }
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Initialize logger with default configuration (INFO level)
pub fn init() {
    init_with_level(LogLevel::Info);
}

/// Initialize logger with custom level (Go style: `[LEVEL] message`)
pub fn init_with_level(level: LogLevel) {
    let filter = tracing_subscriber::filter::LevelFilter::from_level(level.into());

    // Go 风格：显示 [LEVEL] 前缀，不显示时间、不显示模块路径、无颜色
    let layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_target(false)
        .with_level(true)
        .with_ansi(false)
        .compact()
        .with_filter(filter);

    Registry::default().with(layer).init();
}

/// Initialize logger for CLI use (INFO level)
pub fn init_cli() {
    init_with_level(LogLevel::Info);
}

/// Initialize logger for debug use (DEBUG level)
pub fn init_debug() {
    init_with_level(LogLevel::Debug);
}
