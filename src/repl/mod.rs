//! REPL for YaoXiang
//!
//! Unified interactive development environment combining REPL, debugger, and shell.
//!
//! # Usage
//!
//! ```no_run
//! use yaoxiang::repl::Repl;
//!
//! let mut repl = Repl::new().expect("Failed to create REPL");
//! repl.run().expect("REPL error");
//! ```

pub mod backend;
pub mod completer;
pub mod eval;

use std::cell::RefCell;
use std::collections::HashSet;
use std::io;
use std::path::PathBuf;
use std::rc::Rc;

use rustyline::config::Config;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, EditMode, Editor};

pub use backend::{EvalResult, ExecutionStats, REPLBackend, SymbolInfo};
pub use completer::ReplCompleter;
pub use eval::{Evaluator, REPLContext};

use crate::backends::common::RuntimeValue;

// =============================================================================
// Configuration
// =============================================================================

/// REPL configuration
#[derive(Debug, Clone)]
pub struct ReplConfig {
    /// Prompt to display
    pub prompt: String,
    /// Multi-line prompt
    pub continuation_prompt: String,
    /// Enable VI mode
    pub vi_mode: bool,
    /// History file path
    pub history_file: Option<PathBuf>,
    /// Maximum history size
    pub history_size: usize,
    /// Show execution time for :run
    pub show_timing: bool,
}

impl Default for ReplConfig {
    fn default() -> Self {
        let history_file = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(|home| PathBuf::from(home).join(".yaoxiang_history"));
        Self {
            prompt: ">> ".into(),
            continuation_prompt: ".. ".into(),
            vi_mode: false,
            history_file,
            history_size: 1000,
            show_timing: true,
        }
    }
}

// =============================================================================
// Command Result
// =============================================================================

/// Result of a command
enum CommandResult {
    /// Exit the REPL
    Exit,
    /// Continue to next input
    Continue,
    /// Output a message
    Output(String),
}

// =============================================================================
// Repl
// =============================================================================

/// Unified REPL for YaoXiang
///
/// Combines REPL evaluation, debugger, and shell into a single interactive environment.
pub struct Repl {
    /// Configuration
    config: ReplConfig,
    /// rustyline editor
    editor: Editor<ReplCompleter, rustyline::history::FileHistory>,
    /// Shared evaluator
    evaluator: Rc<RefCell<Evaluator>>,
    /// Current working directory
    cwd: PathBuf,
    /// Breakpoints: (function_name, instruction_offset)
    breakpoints: HashSet<(String, usize)>,
}

impl Repl {
    /// Create a new REPL with default configuration
    pub fn new() -> io::Result<Self> {
        Self::with_config(ReplConfig::default())
    }

    /// Create a REPL with custom configuration
    pub fn with_config(config: ReplConfig) -> io::Result<Self> {
        let rl_config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(if config.vi_mode {
                EditMode::Vi
            } else {
                EditMode::Emacs
            })
            .build();

        let evaluator = Rc::new(RefCell::new(Evaluator::new()));
        let completer = ReplCompleter::new(Rc::clone(&evaluator));

        let mut editor = Editor::with_config(rl_config)
            .map_err(|e| io::Error::other(format!("Readline error: {:?}", e)))?;
        editor.set_helper(Some(completer));

        // Load history if file exists
        if let Some(ref history_file) = config.history_file {
            if history_file.exists() {
                let _ = editor.load_history(history_file);
            }
        }

        Ok(Self {
            config,
            editor,
            evaluator,
            cwd: std::env::current_dir().unwrap_or_default(),
            breakpoints: HashSet::new(),
        })
    }

    /// Run the REPL
    pub fn run(&mut self) -> io::Result<()> {
        println!("YaoXiang REPL - Type :help for assistance");
        println!("Press Ctrl+D or :quit to exit\n");

        let mut in_continuation = false;
        let mut buffer = String::new();

        loop {
            let prompt = if in_continuation {
                &self.config.continuation_prompt
            } else {
                &self.config.prompt
            };

            match self.editor.readline(prompt) {
                Ok(line) => {
                    let _ = self.editor.add_history_entry(&line);

                    // Handle commands (only in non-continuation mode)
                    if !in_continuation && line.starts_with(':') {
                        match self.handle_command(&line) {
                            CommandResult::Exit => break,
                            CommandResult::Continue => continue,
                            CommandResult::Output(msg) => {
                                println!("{}", msg);
                                continue;
                            }
                        }
                    }

                    // Accumulate input
                    buffer.push_str(&line);
                    buffer.push('\n');

                    // Evaluate
                    let eval_result = {
                        let mut eval = self.evaluator.borrow_mut();
                        eval.eval(&buffer)
                    };

                    match eval_result {
                        EvalResult::Value(v) => {
                            println!("{}", Self::format_value(&v));
                            buffer.clear();
                            in_continuation = false;
                        }
                        EvalResult::Error(e) => {
                            println!("Error: {}", e);
                            buffer.clear();
                            in_continuation = false;
                        }
                        EvalResult::Incomplete => {
                            in_continuation = true;
                        }
                        EvalResult::Ok => {
                            buffer.clear();
                            in_continuation = false;
                        }
                    }
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl-D pressed
                    break;
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl-C pressed
                    println!("(Interrupted)");
                    buffer.clear();
                    in_continuation = false;
                    let _ = self.editor.clear_screen();
                    continue;
                }
                Err(e) => {
                    return Err(io::Error::other(e.to_string()));
                }
            }
        }

        // Save history
        if let Some(ref history_file) = self.config.history_file {
            let _ = self.editor.save_history(history_file);
        }

        Ok(())
    }

    // =========================================================================
    // Command Handling
    // =========================================================================

    /// Handle a command
    fn handle_command(
        &mut self,
        line: &str,
    ) -> CommandResult {
        let cmd = line.trim_start_matches(':').trim();
        let parts: Vec<&str> = cmd.split_whitespace().collect();

        if parts.is_empty() {
            return CommandResult::Continue;
        }

        match parts[0] {
            // Exit
            "quit" | "q" | "exit" => CommandResult::Exit,

            // Help
            "help" | "h" => {
                self.print_help();
                CommandResult::Continue
            }

            // Clear context
            "clear" | "c" => {
                self.evaluator.borrow_mut().clear();
                println!("Context cleared");
                CommandResult::Continue
            }

            // Show type of symbol
            "type" | "t" => {
                if let Some(name) = parts.get(1) {
                    if let Some(ty) = self.evaluator.borrow().get_type(name) {
                        println!("{}: {}", name, ty);
                    } else {
                        println!("Unknown symbol: {}", name);
                    }
                } else {
                    println!("Usage: :type <name>");
                }
                CommandResult::Continue
            }

            // List symbols
            "symbols" | "info" | "i" => {
                for sym in self.evaluator.borrow().get_symbols() {
                    println!("{}: {}", sym.name, sym.type_signature);
                }
                CommandResult::Continue
            }

            // Show stats
            "stats" => {
                let stats = self.evaluator.borrow().stats();
                println!("Eval count: {}", stats.eval_count);
                println!("Total time: {:?}", stats.total_time);
                CommandResult::Continue
            }

            // Run a file
            "run" => {
                if let Some(path) = parts.get(1) {
                    self.run_file(path)
                } else {
                    CommandResult::Output("Usage: :run <filename>".to_string())
                }
            }

            // Load a file
            "load" => {
                if let Some(path) = parts.get(1) {
                    self.load_file(path)
                } else {
                    CommandResult::Output("Usage: :load <filename>".to_string())
                }
            }

            // Debug a file
            "debug" => {
                if let Some(path) = parts.get(1) {
                    self.debug_file(path)
                } else {
                    CommandResult::Output("Usage: :debug <filename>".to_string())
                }
            }

            // Set breakpoint
            "break" | "b" => {
                if parts.len() > 2 {
                    if let Ok(offset) = parts[2].parse() {
                        self.breakpoints.insert((parts[1].to_string(), offset));
                        println!("Breakpoint set at {}:{}", parts[1], offset);
                        CommandResult::Continue
                    } else {
                        CommandResult::Output("Invalid offset".to_string())
                    }
                } else {
                    CommandResult::Output("Usage: :break <function> <offset>".to_string())
                }
            }

            // List breakpoints
            "breakpoints" | "bp" => {
                if self.breakpoints.is_empty() {
                    println!("No breakpoints set");
                } else {
                    for (func, offset) in &self.breakpoints {
                        println!("  {}:{}", func, offset);
                    }
                }
                CommandResult::Continue
            }

            // Change directory
            "cd" => {
                if let Some(path) = parts.get(1) {
                    if let Ok(new_cwd) = self.cwd.join(path).canonicalize() {
                        if new_cwd.is_dir() {
                            self.cwd = new_cwd;
                            std::env::set_current_dir(&self.cwd).ok();
                        } else {
                            return CommandResult::Output(format!("Not a directory: {}", path));
                        }
                    } else {
                        return CommandResult::Output(format!("Invalid path: {}", path));
                    }
                } else {
                    println!("{}", self.cwd.display());
                }
                CommandResult::Continue
            }

            // Print working directory
            "pwd" => CommandResult::Output(self.cwd.display().to_string()),

            // List directory
            "ls" => {
                let dir = if let Some(path) = parts.get(1) {
                    self.cwd.join(path)
                } else {
                    self.cwd.clone()
                };
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let path = entry.path();
                        let is_dir = path.is_dir();
                        println!(
                            "{} {}",
                            if is_dir { "[DIR]" } else { "    " },
                            entry.file_name().to_string_lossy()
                        );
                    }
                }
                CommandResult::Continue
            }

            // History
            "history" | "hist" => {
                println!("History command not yet implemented");
                CommandResult::Continue
            }

            // Unknown
            "" => CommandResult::Continue,
            _ => {
                println!("Unknown command: {}", line);
                CommandResult::Continue
            }
        }
    }

    // =========================================================================
    // File Operations
    // =========================================================================

    /// Run a file
    fn run_file(
        &mut self,
        path: &str,
    ) -> CommandResult {
        let file_path = std::path::Path::new(path);
        if !file_path.exists() {
            return CommandResult::Output(format!("File not found: {}", file_path.display()));
        }

        let start = std::time::Instant::now();

        match std::fs::read_to_string(file_path) {
            Ok(source) => {
                let result = self.evaluator.borrow_mut().eval(&source);
                if self.config.show_timing {
                    println!("Executed in {:?}", start.elapsed());
                }
                match result {
                    EvalResult::Error(e) => CommandResult::Output(format!("Error: {}", e)),
                    _ => CommandResult::Continue,
                }
            }
            Err(e) => CommandResult::Output(format!("IO error: {}", e)),
        }
    }

    /// Load a file
    fn load_file(
        &mut self,
        path: &str,
    ) -> CommandResult {
        let file_path = std::path::Path::new(path);
        if !file_path.exists() {
            return CommandResult::Output(format!("File not found: {}", file_path.display()));
        }

        match std::fs::read_to_string(file_path) {
            Ok(source) => {
                let result = self.evaluator.borrow_mut().eval(&source);
                match result {
                    EvalResult::Error(e) => CommandResult::Output(format!("Error: {}", e)),
                    _ => CommandResult::Output(format!("Loaded {}", file_path.display())),
                }
            }
            Err(e) => CommandResult::Output(format!("IO error: {}", e)),
        }
    }

    /// Debug a file
    fn debug_file(
        &mut self,
        path: &str,
    ) -> CommandResult {
        let file_path = std::path::Path::new(path);
        if !file_path.exists() {
            return CommandResult::Output(format!("File not found: {}", file_path.display()));
        }

        println!("Debugging {}...", file_path.display());
        println!("Use :break <function> <offset> to set breakpoints");
        CommandResult::Continue
    }

    // =========================================================================
    // Helpers
    // =========================================================================

    /// Print help message
    fn print_help(&self) {
        println!("Available commands:");
        println!("  :quit, :q, :exit       - Exit the REPL");
        println!("  :help, :h              - Show this help");
        println!("  :clear, :c             - Clear all state");
        println!("  :type, :t <name>       - Show type of symbol");
        println!("  :symbols, :info, :i    - List all symbols");
        println!("  :stats                 - Show execution statistics");
        println!("  :run <file>            - Run a file");
        println!("  :load <file>           - Load a file");
        println!("  :debug <file>          - Debug a file");
        println!("  :break, :b <fn> <off>  - Set breakpoint");
        println!("  :breakpoints, :bp      - List breakpoints");
        println!("  :cd <dir>              - Change directory");
        println!("  :pwd                   - Print working directory");
        println!("  :ls [dir]              - List directory contents");
        println!("  :history, :hist        - Show command history");
    }

    /// Format a value for display
    fn format_value(value: &RuntimeValue) -> String {
        match value {
            RuntimeValue::Unit => "()".to_string(),
            RuntimeValue::Bool(b) => b.to_string(),
            RuntimeValue::Int(i) => i.to_string(),
            RuntimeValue::Float(f) => format!("{}", f),
            RuntimeValue::String(s) => format!("{:?}", s),
            _ => format!("{}", value),
        }
    }

    /// Get the evaluator reference
    pub fn evaluator(&self) -> std::cell::Ref<'_, Evaluator> {
        self.evaluator.borrow()
    }

    /// Get the evaluator mutable reference
    pub fn evaluator_mut(&self) -> std::cell::RefMut<'_, Evaluator> {
        self.evaluator.borrow_mut()
    }

    /// Get the current working directory
    pub fn cwd(&self) -> &std::path::Path {
        &self.cwd
    }

    /// Get breakpoints
    pub fn breakpoints(&self) -> &HashSet<(String, usize)> {
        &self.breakpoints
    }
}
