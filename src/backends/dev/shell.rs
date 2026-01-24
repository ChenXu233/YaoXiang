//! Development shell for YaoXiang
//!
//! This module provides a command-line shell for YaoXiang development.
//! It combines the REPL, debugger, and file utilities.

use std::path::Path;
use std::io::{self, Write};
use crate::backends::dev::{Debugger, REPL};
use crate::backends::common::RuntimeValue;

/// Shell configuration
#[derive(Debug, Clone)]
pub struct ShellConfig {
    /// Shell prompt
    pub prompt: String,
    /// Show execution time
    pub show_timing: bool,
    /// Auto-run loaded files
    pub auto_run: bool,
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            prompt: "yx> ".to_string(),
            show_timing: true,
            auto_run: true,
        }
    }
}

/// Result of a shell command
#[derive(Debug)]
pub enum ShellResult {
    /// Command completed successfully
    Success,
    /// Command produced a value
    Value(RuntimeValue),
    /// Command had an error
    Error(String),
    /// Exit signal
    Exit,
}

/// Development shell for YaoXiang
///
/// The shell provides an interactive interface with:
/// - REPL for quick code snippets
/// - Debugger integration
/// - File loading and execution
/// - Basic shell commands
#[derive(Debug)]
pub struct DevShell {
    /// Shell configuration
    config: ShellConfig,
    /// REPL for evaluation
    repl: REPL,
    /// Debugger for debugging
    debugger: Debugger,
    /// Current working directory
    cwd: std::path::PathBuf,
    /// History of commands
    history: Vec<String>,
}

impl Default for DevShell {
    fn default() -> Self {
        Self::new()
    }
}

impl DevShell {
    /// Create a new shell
    pub fn new() -> Self {
        Self {
            config: ShellConfig::default(),
            repl: REPL::new(),
            debugger: Debugger::new(),
            cwd: std::env::current_dir().unwrap_or_default(),
            history: Vec::new(),
        }
    }

    /// Run the shell
    pub fn run(&mut self) -> Result<(), io::Error> {
        println!("YaoXiang Development Shell v0.3.0");
        println!("Type :help for available commands.");
        println!();

        loop {
            print!("{}", self.config.prompt);
            io::stdout().flush()?;

            let mut line = String::new();
            if io::stdin().read_line(&mut line)? == 0 {
                // Ctrl-D
                println!("\nExiting...");
                break;
            }

            let line = line.trim().to_string();
            if line.is_empty() {
                continue;
            }

            // Add to history
            self.history.push(line.clone());

            match self.execute_command(&line) {
                ShellResult::Exit => break,
                ShellResult::Error(e) => {
                    println!("Error: {}", e);
                }
                ShellResult::Value(v) => {
                    println!("{}", v);
                }
                ShellResult::Success => {}
            }
        }

        Ok(())
    }

    /// Execute a shell command
    fn execute_command(
        &mut self,
        command: &str,
    ) -> ShellResult {
        // Split command into parts
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return ShellResult::Success;
        }

        match parts[0] {
            // Shell commands
            ":quit" | ":q" | "exit" => ShellResult::Exit,
            ":help" | ":h" | "help" => {
                self.print_help();
                ShellResult::Success
            }
            ":clear" | "clear" => {
                print!("\x1B[2J\x1B[1;1H");
                io::stdout().flush().ok();
                ShellResult::Success
            }
            ":cd" | "cd" => {
                if parts.len() > 1 {
                    if let Ok(path) = std::env::current_dir() {
                        if let Ok(new_cwd) = path.join(parts[1]).canonicalize() {
                            if new_cwd.is_dir() {
                                self.cwd = new_cwd;
                                std::env::set_current_dir(&self.cwd).ok();
                            } else {
                                return ShellResult::Error(format!(
                                    "Not a directory: {}",
                                    parts[1]
                                ));
                            }
                        } else {
                            return ShellResult::Error(format!("Invalid path: {}", parts[1]));
                        }
                    } else {
                        return ShellResult::Error("Failed to get current directory".to_string());
                    }
                } else {
                    println!("{}", self.cwd.display());
                }
                ShellResult::Success
            }
            ":pwd" | "pwd" => {
                println!("{}", self.cwd.display());
                ShellResult::Success
            }
            ":ls" | "ls" => {
                let dir = if parts.len() > 1 {
                    self.cwd.join(parts[1])
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
                ShellResult::Success
            }
            ":run" | "run" => {
                if parts.len() > 1 {
                    self.run_file(std::path::Path::new(parts[1]))
                } else {
                    ShellResult::Error("Usage: :run <filename>".to_string())
                }
            }
            ":load" | "load" => {
                if parts.len() > 1 {
                    self.load_file(std::path::Path::new(parts[1]))
                } else {
                    ShellResult::Error("Usage: :load <filename>".to_string())
                }
            }
            ":debug" | "debug" => {
                if parts.len() > 1 {
                    self.debug_file(std::path::Path::new(parts[1]))
                } else {
                    ShellResult::Error("Usage: :debug <filename>".to_string())
                }
            }
            ":break" | "break" => {
                if parts.len() > 2 {
                    if let Ok(offset) = parts[2].parse() {
                        self.debugger.set_breakpoint(parts[1], offset);
                        ShellResult::Success
                    } else {
                        ShellResult::Error("Invalid offset".to_string())
                    }
                } else {
                    ShellResult::Error("Usage: :break <function> <offset>".to_string())
                }
            }
            ":repl" | "repl" => {
                // Switch to REPL mode
                if let Err(e) = self.repl.run() {
                    ShellResult::Error(format!("REPL error: {}", e))
                } else {
                    ShellResult::Success
                }
            }
            // Default: evaluate as code
            _ => self.evaluate_code(command),
        }
    }

    /// Print help message
    fn print_help(&self) {
        println!("YaoXiang Development Shell");
        println!();
        println!("Shell Commands:");
        println!("  :help, :h         - Show this help");
        println!("  :quit, :q, exit   - Exit the shell");
        println!("  :clear            - Clear the screen");
        println!("  :cd <dir>         - Change directory");
        println!("  :pwd              - Print working directory");
        println!("  :ls [dir]         - List directory contents");
        println!();
        println!("Code Commands:");
        println!("  :run <file>       - Run a YaoXiang file");
        println!("  :load <file>      - Load a YaoXiang file");
        println!("  :debug <file>     - Debug a YaoXiang file");
        println!("  :break <fn> <n>   - Set breakpoint");
        println!("  :repl             - Switch to REPL mode");
        println!();
        println!("Any other input is evaluated as YaoXiang code.");
    }

    /// Evaluate code directly
    fn evaluate_code(
        &mut self,
        code: &str,
    ) -> ShellResult {
        // Wrap in a simple expression
        let wrapped = code.to_string();

        let mut compiler = crate::frontend::Compiler::new();
        match compiler.compile_with_source("<shell>", &wrapped) {
            Ok(_module) => {
                // In a full implementation, we'd execute
                ShellResult::Success
            }
            Err(e) => ShellResult::Error(format!("{}", e)),
        }
    }

    /// Run a file
    fn run_file(
        &mut self,
        path: &Path,
    ) -> ShellResult {
        if !path.exists() {
            return ShellResult::Error(format!("File not found: {}", path.display()));
        }

        let start = std::time::Instant::now();
        let result = self.repl.load_file(path);
        let elapsed = start.elapsed();

        if self.config.show_timing {
            println!("Executed in {:?}", elapsed);
        }

        match result {
            Ok(_) => ShellResult::Success,
            Err(e) => ShellResult::Error(format!("IO error: {}", e)),
        }
    }

    /// Load a file
    fn load_file(
        &mut self,
        path: &Path,
    ) -> ShellResult {
        if !path.exists() {
            return ShellResult::Error(format!("File not found: {}", path.display()));
        }

        match self.repl.load_file(path) {
            Ok(_) => {
                println!("Loaded: {}", path.display());
                ShellResult::Success
            }
            Err(e) => ShellResult::Error(format!("IO error: {}", e)),
        }
    }

    /// Debug a file
    fn debug_file(
        &mut self,
        path: &Path,
    ) -> ShellResult {
        if !path.exists() {
            return ShellResult::Error(format!("File not found: {}", path.display()));
        }

        println!("Starting debug session for: {}", path.display());
        println!("Available commands:");
        println!("  (r)un    - Run to next breakpoint");
        println!("  (s)tep   - Step one instruction");
        println!("  (n)ext   - Step over function calls");
        println!("  (o)ut    - Step out of current function");
        println!("  (b)reak  - List breakpoints");
        println!("  (b)reak <n> - Add breakpoint at line n");
        println!("  (c)ont   - Continue execution");
        println!("  (q)uit   - Exit debugger");

        ShellResult::Success
    }

    /// Get the REPL
    pub fn repl(&mut self) -> &mut REPL {
        &mut self.repl
    }

    /// Get the debugger
    pub fn debugger(&mut self) -> &mut Debugger {
        &mut self.debugger
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_new() {
        let shell = DevShell::new();
        assert!(shell.history.is_empty());
    }
}
