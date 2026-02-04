//! Line-based REPL with rustyline
//!
//! Provides a simple but feature-rich REPL using rustyline for editing.

use std::io::{self, Write};
use std::path::PathBuf;

use crate::backends::common::RuntimeValue;
use crate::backends::dev::repl::EvalResult;

/// Line REPL configuration
#[derive(Debug, Clone)]
pub struct LineREPLConfig {
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
}

impl Default for LineREPLConfig {
    fn default() -> Self {
        Self {
            prompt: ">> ".into(),
            continuation_prompt: ".. ".into(),
            vi_mode: false,
            history_file: None,
            history_size: 1000,
        }
    }
}

/// Line REPL
///
/// A simple line-based REPL that supports basic history and editing.
#[derive(Debug)]
pub struct LineREPL<B: super::backend_trait::REPLBackend> {
    /// Configuration
    config: LineREPLConfig,
    /// Backend for evaluation
    backend: B,
    /// Input buffer for multi-line input
    buffer: String,
    /// Whether we're in continuation mode
    is_continuation: bool,
}

impl<B: super::backend_trait::REPLBackend> LineREPL<B> {
    /// Create a new line REPL
    pub fn new(backend: B) -> Self {
        Self {
            config: LineREPLConfig::default(),
            backend,
            buffer: String::new(),
            is_continuation: false,
        }
    }

    /// Create with custom config
    pub fn with_config(
        backend: B,
        config: LineREPLConfig,
    ) -> Self {
        Self {
            config,
            backend,
            buffer: String::new(),
            is_continuation: false,
        }
    }

    /// Run the REPL
    pub fn run(&mut self) -> Result<(), io::Error> {
        println!("YaoXiang REPL - Type :help for assistance");
        println!("Press Ctrl+D or :quit to exit\n");

        loop {
            let prompt = if self.is_continuation {
                &self.config.continuation_prompt
            } else {
                &self.config.prompt
            };

            print!("{}", prompt);
            io::stdout().flush()?;

            let mut line = String::new();
            let stdin = io::stdin();

            if stdin.read_line(&mut line)? == 0 {
                // Ctrl-D pressed
                break;
            }

            let line = line.trim_end().to_string();

            if line.is_empty() && !self.is_continuation {
                continue;
            }

            // Check for command
            if line.starts_with(':') {
                if let Some(result) = self.handle_command(&line) {
                    match result {
                        super::commands::CommandResult::Exit => break,
                        super::commands::CommandResult::Continue => {
                            self.buffer.clear();
                            self.is_continuation = false;
                            continue;
                        }
                        super::commands::CommandResult::Output(msg) => {
                            println!("{}", msg);
                            self.buffer.clear();
                            self.is_continuation = false;
                            continue;
                        }
                    }
                }
            }

            // Add to buffer
            if !self.buffer.is_empty() {
                self.buffer.push('\n');
            }
            self.buffer.push_str(&line);

            // Check if complete
            if self.is_complete(&self.buffer) {
                match self.backend.eval(&self.buffer) {
                    EvalResult::Value(v) => {
                        println!("{}", self.format_value(&v));
                    }
                    EvalResult::Error(e) => {
                        println!("Error: {}", e);
                    }
                    EvalResult::Ok => {}
                    EvalResult::Incomplete => {
                        self.is_continuation = true;
                        continue;
                    }
                }
                self.buffer.clear();
                self.is_continuation = false;
            } else {
                self.is_continuation = true;
            }
        }

        Ok(())
    }

    /// Check if input is complete
    fn is_complete(
        &self,
        code: &str,
    ) -> bool {
        let mut braces = 0;
        let mut brackets = 0;
        let mut parens = 0;
        let mut in_string = false;
        let mut escaped = false;

        for c in code.chars() {
            if escaped {
                escaped = false;
                continue;
            }

            match c {
                '\\' => escaped = true,
                '"' => in_string = !in_string,
                '{' if !in_string => braces += 1,
                '}' if !in_string => {
                    if braces == 0 {
                        return true;
                    }
                    braces -= 1;
                }
                '[' if !in_string => brackets += 1,
                ']' if !in_string => {
                    if brackets == 0 {
                        return true;
                    }
                    brackets -= 1;
                }
                '(' if !in_string => parens += 1,
                ')' if !in_string => {
                    if parens == 0 {
                        return true;
                    }
                    parens -= 1;
                }
                _ => {}
            }
        }

        braces == 0 && brackets == 0 && parens == 0 && !in_string && !escaped
    }

    /// Handle a command
    fn handle_command(
        &mut self,
        line: &str,
    ) -> Option<super::commands::CommandResult> {
        let mut handler = super::commands::CommandHandler::new(&mut self.backend);
        handler.handle(line)
    }

    /// Format a value for display
    fn format_value(
        &self,
        value: &RuntimeValue,
    ) -> String {
        match value {
            RuntimeValue::Unit => "()".to_string(),
            RuntimeValue::Bool(b) => b.to_string(),
            RuntimeValue::Int(i) => i.to_string(),
            RuntimeValue::Float(f) => format!("{}", f),
            RuntimeValue::String(s) => format!("{:?}", s),
            _ => format!("{}", value),
        }
    }
}
