//! Logger module for YaoXiang
//!
//! Provides structured logging with customizable formatters and output targets.
//!
//! # Usage
//!
//! ```rust
//! use yaoxiang_util::logger;
//!
//! fn main() {
//!     logger::init();
//!     tracing::info!("Hello, {}", "world");
//! }
//! ```

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, Layer, Registry};

/// Logger configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Log level filter
    level: tracing::Level,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            level: tracing::Level::INFO,
        }
    }
}

/// Initialize logger with default configuration
pub fn init() {
    init_with_config(Config::default());
}

/// Initialize logger with custom configuration
pub fn init_with_config(config: Config) {
    let filter = tracing_subscriber::filter::LevelFilter::from_level(config.level);
    let subscriber = tracing_subscriber::fmt::layer().with_filter(filter);
    Registry::default().with(subscriber).init();
}

/// Initialize logger for CLI use (info level)
pub fn init_cli() {
    init_with_config(Config {
        level: tracing::Level::INFO,
    });
}

/// Initialize logger for debug use (debug level)
pub fn init_debug() {
    init_with_config(Config {
        level: tracing::Level::DEBUG,
    });
}
