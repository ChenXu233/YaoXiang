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

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry};

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

    // Go 风格：只显示级别和消息，无时间戳、无模块路径
    let layer = tracing_subscriber::fmt::layer()
        .without_time()
        .with_target(false)
        .with_level(false)
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
