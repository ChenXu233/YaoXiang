//! Logger module for YaoXiang
//!
//! Go-style simple logging: `[LEVEL] message`

use std::sync::atomic::{AtomicU8, Ordering};

use crate::util::i18n::current_lang;

/// Global language setting for i18n (stored as atomic u8 for thread-safe access)
static CURRENT_LANG: AtomicU8 = AtomicU8::new(0);

/// Set the current language for i18n
pub fn set_lang(lang_code: &str) {
    let val = match lang_code {
        "zh" => 1,
        "zh-x-miao" | "zh-miao" => 2,
        _ => 0,
    };
    CURRENT_LANG.store(val, Ordering::SeqCst);
}

/// Get the current language for i18n
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
#[cfg(feature = "cli")]
pub fn init() {
    init_with_level(LogLevel::Info);
}

/// Initialize logger with custom level (Go style: `[LEVEL] message`)
#[cfg(feature = "cli")]
pub fn init_with_level(level: LogLevel) {
    use tracing_subscriber::{
        fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry,
    };

    let filter = tracing_subscriber::filter::LevelFilter::from_level(level.into());

    let layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_target(false)
        .with_level(true)
        .with_ansi(true)
        .with_filter(filter);

    Registry::default().with(layer).init();
}

/// Initialize logger for CLI use (INFO level)
#[cfg(feature = "cli")]
pub fn init_cli() {
    init_with_level(LogLevel::Info);
}

/// Initialize logger for LSP use (stderr only)
#[cfg(feature = "cli")]
pub fn init_lsp() {
    init_lsp_with_level(LogLevel::Info);
}

/// Initialize logger for LSP use with custom level
#[cfg(feature = "cli")]
pub fn init_lsp_with_level(level: LogLevel) {
    use tracing_subscriber::{
        fmt::writer::MakeWriterExt, layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry,
    };

    let filter = tracing_subscriber::filter::LevelFilter::from_level(level.into());

    let layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_target(false)
        .with_level(true)
        .with_ansi(false)
        .with_writer(std::io::stderr.with_max_level(tracing::Level::TRACE))
        .with_filter(filter);

    Registry::default().with(layer).init();
}

/// Initialize logger for debug use (DEBUG level)
#[cfg(feature = "cli")]
pub fn init_debug() {
    init_with_level(LogLevel::Debug);
}
