//! Line-based REPL with rustyline
//!
//! Provides a feature-rich REPL using rustyline for editing and history.

use std::io::{self, Write};
use std::path::PathBuf;

use rustyline::config::Config;
use rustyline::error::ReadlineError;
use rustyline::{Editor, CompletionType, EditMode};

use crate::backends::common::RuntimeValue;
use crate::backends::dev::repl::backend_trait::{REPLBackend, EvalResult};
use crate::backends::dev::repl::commands::{CommandHandler, CommandResult};

mod completer;
pub use completer::REPLCompleter;

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
/// A line-based REPL with rustyline support for editing and history.
#[derive(Debug)]
pub struct LineREPL<B: REPLBackend> {
    /// Configuration
    config: LineREPLConfig,
    /// rustyline editor
    editor: Editor<(), rustyline::history::FileHistory>,
    /// Backend for evaluation
    backend: B,
}

impl<B: REPLBackend + 'static> LineREPL<B> {
    /// Create a new line REPL
    pub fn new(backend: B) -> Result<Self> {
        Self::with_config(backend, LineREPLConfig::default())
    }

    /// Create with custom config
    pub fn with_config(backend: B, config: LineREPLConfig) -> Result<Self> {
        use std::io;

        let rl_config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(if config.vi_mode {
                EditMode::Vi
            } else {
                EditMode::Emacs
            })
            .build();

        let mut editor = Editor::with_config(rl_config)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Readline error: {:?}", e)))?;

        // Load history if file exists
        if let Some(ref history_file) = config.history_file {
            if history_file.exists() {
                let _ = editor.load_history(history_file);
            }
        }

        Ok(Self {
            config,
            editor,
            backend,
        })
    }

    /// Run the REPL
    pub fn run(&mut self) -> Result<()> {
        println!("YaoXiang REPL - Type :help for assistance");
        println!("Press Ctrl+D or :quit to exit\n");

        let mut in_continuation = false;

        loop {
            let prompt = if in_continuation {
                &self.config.continuation_prompt
            } else {
                &self.config.prompt
            };

            match self.editor.readline(prompt) {
                Ok(line) => {
                    in_continuation = false;
                    self.editor.add_history_entry(&line);

                    // Handle commands
                    if line.starts_with(':') {
                        let mut command_handler = CommandHandler::new(&mut self.backend);
                        if let Some(result) = command_handler.handle(&line) {
                            match result {
                                CommandResult::Exit => break,
                                CommandResult::Continue => {
                                    continue;
                                }
                                CommandResult::Output(msg) => {
                                    println!("{}", msg);
                                    continue;
                                }
                            }
                        }
                    }

                    // Evaluate
                    match self.backend.eval(&line) {
                        EvalResult::Value(v) => {
                            println!("{}", self.format_value(&v));
                        }
                        EvalResult::Error(e) => {
                            println!("Error: {}", e);
                        }
                        EvalResult::Incomplete => {
                            // Continue multi-line input
                            continue;
                        }
                        EvalResult::Ok => {}
                    }
                }
                Err(ReadlineError::Eof) => {
                    // Ctrl-D pressed
                    break;
                }
                Err(ReadlineError::Interrupted) => {
                    // Ctrl-C pressed
                    println!("(Interrupted)");
                    let _ = self.editor.clear_screen();
                    continue;
                }
                Err(e) => {
                    return Err(io::Error::new(io::ErrorKind::Other, e.to_string()));
                }
            }
        }

        // Save history
        if let Some(ref history_file) = self.config.history_file {
            let _ = self.editor.save_history(history_file);
        }

        Ok(())
    }

    /// Format a value for display
    fn format_value(&self, value: &RuntimeValue) -> String {
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

impl<B: REPLBackend> LineREPL<B> {
    /// Get the backend reference
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Get the backend mut reference
    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }
}

type Result<T> = std::result::Result<T, io::Error>;
