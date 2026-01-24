//! REPL 模块
//!
//! 提供简单 REPL 和高级 TUI REPL 两种交互式编程环境

pub mod app;
pub mod components;
pub mod engine;
pub mod screens;
pub mod tui;
pub mod widgets;

// 重新导出 TUI REPL
pub use tui::TuiREPL;

// 重新导出引擎组件
pub use engine::{IncrementalCompiler, ModuleBuilder, SymbolCache, Profiler};

// 重新导出组件
pub use components::{HistoryPanel, InputWindow, DebugPanel, OutputConsole};

// 重新导出屏幕
pub use screens::{MainScreen, HistoryScreen, DebugScreen, HelpScreen};
