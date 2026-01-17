//! Logger module for YaoXiang
//!
//! Provides structured logging with customizable formatters and output targets.
//!
//! # Usage
//!
//! ```rust
//! use yaoxiang_util::logger;
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

/// Initialize logger with custom level
pub fn init_with_level(level: LogLevel) {
    let filter = tracing_subscriber::filter::LevelFilter::from_level(level.into());
    let subscriber = tracing_subscriber::fmt::layer().with_filter(filter);
    Registry::default().with(subscriber).init();
}

/// Initialize logger for CLI use (INFO level, pretty format)
pub fn init_cli() {
    init_with_level(LogLevel::Info);
}

/// Initialize logger for debug use (DEBUG level)
pub fn init_debug() {
    init_with_level(LogLevel::Debug);
}
