//! Development tools for YaoXiang
//!
//! This module provides debugging and development utilities:
//! - Debugger: breakpoint debugging, stepping
//! - REPL: interactive evaluation
//! - Shell: command-line interface

pub mod debugger;
pub mod repl;
pub mod shell;
#[cfg(feature = "tui")]
pub mod tui_repl;

pub use debugger::{Debugger, DebuggerState};
pub use repl::{REPL, REPLConfig};
pub use shell::DevShell;
#[cfg(feature = "tui")]
pub use tui_repl::TuiREPL;
