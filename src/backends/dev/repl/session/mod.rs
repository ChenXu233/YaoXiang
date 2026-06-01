//! Session-based REPL with rustyline
//!
//! Provides a feature-rich REPL using rustyline for editing and history.

use std::cell::RefCell;
use std::fmt;
use std::io;
use std::path::Path;
use std::rc::Rc;

use rustyline::config::Config;
use rustyline::error::ReadlineError;
use rustyline::{Editor, CompletionType, EditMode};

use crate::backends::common::RuntimeValue;
use crate::backends::dev::repl::backend_trait::{REPLBackend, EvalResult};
use crate::backends::dev::repl::commands::{CommandHandler, CommandResult};

mod completer;
pub use completer::REPLCompleter;

/// Session REPL configuration
#[derive(Debug, Clone)]
pub struct SessionREPLConfig {
    /// Prompt to display
    pub prompt: String,
    /// Multi-line prompt
    pub continuation_prompt: String,
    /// Enable VI mode
    pub vi_mode: bool,
    /// History file path
    pub history_file: Option<std::path::PathBuf>,
    /// Maximum history size
    pub history_size: usize,
}

impl Default for SessionREPLConfig {
    fn default() -> Self {
        let history_file = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .ok()
            .map(|home| std::path::PathBuf::from(home).join(".yaoxiang_history"));
        Self {
            prompt: ">> ".into(),
            continuation_prompt: ".. ".into(),
            vi_mode: false,
            history_file,
            history_size: 1000,
        }
    }
}

/// Session REPL
///
/// A session-based REPL with rustyline support for editing, history, and completion.
pub struct SessionREPL<B: REPLBackend + 'static> {
    /// Configuration
    config: SessionREPLConfig,
    /// rustyline editor
    editor: Editor<REPLCompleter<B>, rustyline::history::FileHistory>,
    /// Shared backend for evaluation
    backend: Rc<RefCell<B>>,
}

impl<B: REPLBackend + 'static> fmt::Debug for SessionREPL<B> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionREPL")
            .field("config", &self.config)
            .finish()
    }
}

impl<B: REPLBackend + 'static> SessionREPL<B> {
    /// Create a new session REPL
    pub fn new(backend: B) -> io::Result<Self> {
        Self::with_config(backend, SessionREPLConfig::default())
    }

    /// Create with custom config
    pub fn with_config(backend: B, config: SessionREPLConfig) -> io::Result<Self> {
        let rl_config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .edit_mode(if config.vi_mode {
                EditMode::Vi
            } else {
                EditMode::Emacs
            })
            .build();

        let shared_backend = Rc::new(RefCell::new(backend));
        let completer = REPLCompleter::new(Rc::clone(&shared_backend));

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
            backend: shared_backend,
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
                        let mut backend = self.backend.borrow_mut();
                        let mut command_handler = CommandHandler::new(&mut *backend);
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

                    // Accumulate input
                    buffer.push_str(&line);
                    buffer.push('\n');

                    // Evaluate
                    let eval_result = {
                        let mut backend = self.backend.borrow_mut();
                        backend.eval(&buffer)
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

    /// Load and execute a file
    pub fn load_file(&mut self, path: &Path) -> io::Result<()> {
        let source = std::fs::read_to_string(path)?;
        let eval_result = self.backend.borrow_mut().eval(&source);
        match eval_result {
            EvalResult::Error(e) => {
                eprintln!("Error: {}", e);
            }
            _ => {}
        }
        Ok(())
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
}

impl<B: REPLBackend> SessionREPL<B> {
    /// Get the backend reference
    pub fn backend(&self) -> std::cell::Ref<'_, B> {
        self.backend.borrow()
    }

    /// Get the backend mutable reference
    pub fn backend_mut(&self) -> std::cell::RefMut<'_, B> {
        self.backend.borrow_mut()
    }
}
