//! REPL (Read-Eval-Print Loop) for YaoXiang
//!
//! This module provides a simple interactive REPL for YaoXiang.
//! It allows users to enter code snippets and see results immediately.

use std::io::{self, Write};
use std::path::Path;
use crate::tlog;
use crate::util::i18n::MSG;
use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::Interpreter;
use crate::backends::Executor;

/// REPL configuration
#[derive(Debug, Clone)]
pub struct REPLConfig {
    /// Prompt to display
    pub prompt: String,
    /// Multi-line prompt
    pub multi_line_prompt: String,
    /// Enable syntax highlighting (if available)
    pub syntax_highlight: bool,
    /// Enable auto-indent
    pub auto_indent: bool,
    /// Maximum history size
    pub history_size: usize,
}

impl Default for REPLConfig {
    fn default() -> Self {
        Self {
            prompt: ">> ".to_string(),
            multi_line_prompt: ".. ".to_string(),
            syntax_highlight: true,
            auto_indent: true,
            history_size: 1000,
        }
    }
}

/// Result of a REPL evaluation
#[derive(Debug)]
pub enum REPLResult {
    /// Evaluation produced a value
    Value(RuntimeValue),
    /// Evaluation produced no value (unit)
    Ok,
    /// Evaluation had an error
    Error(String),
    /// User entered :quit or Ctrl-D
    Exit,
}

/// REPL for YaoXiang
///
/// The REPL reads code from stdin, evaluates it, and prints the result.
/// It supports:
/// - Multi-line input (for blocks)
/// - Basic history (up arrow)
/// - Special commands (`:quit`, `:help`, etc.)
#[derive(Debug)]
pub struct REPL {
    /// REPL configuration
    config: REPLConfig,
    /// Interpreter for evaluation
    interpreter: Interpreter,
    /// Input history
    history: Vec<String>,
    /// Current input buffer
    buffer: String,
    /// Line count for multi-line input
    line_count: usize,
}

impl Default for REPL {
    fn default() -> Self {
        Self::new()
    }
}

impl REPL {
    /// Create a new REPL
    pub fn new() -> Self {
        Self::with_config(REPLConfig::default())
    }

    /// Create a REPL with custom configuration
    pub fn with_config(config: REPLConfig) -> Self {
        Self {
            config,
            interpreter: Interpreter::new(),
            history: Vec::new(),
            buffer: String::new(),
            line_count: 0,
        }
    }

    /// Run the REPL
    pub fn run(&mut self) -> Result<(), io::Error> {
        tlog!(info, MSG::ReplWelcome);
        tlog!(info, MSG::ReplHelp);

        loop {
            match self.read_line()? {
                REPLResult::Exit => break,
                REPLResult::Error(e) => {
                    tlog!(info, MSG::ReplError, &e.to_string());
                    self.buffer.clear();
                    self.line_count = 0;
                }
                REPLResult::Ok => {
                    self.buffer.clear();
                    self.line_count = 0;
                }
                REPLResult::Value(v) => {
                    tlog!(info, MSG::ReplValue, &v.to_string());
                    self.buffer.clear();
                    self.line_count = 0;
                }
            }
        }

        Ok(())
    }

    /// Read a line of input
    fn read_line(&mut self) -> Result<REPLResult, io::Error> {
        let prompt = if self.line_count == 0 {
            &self.config.prompt
        } else {
            &self.config.multi_line_prompt
        };

        tlog!(debug, MSG::ReplPrompt, &prompt.to_string());
        io::stdout().flush()?;

        let mut line = String::new();
        let stdin = io::stdin();

        if stdin.read_line(&mut line)? == 0 {
            // Ctrl-D pressed
            return Ok(REPLResult::Exit);
        }

        // Remove trailing newline
        line = line.trim_end().to_string();

        // Check for special commands
        if line.starts_with(':') {
            return self.handle_command(&line);
        }

        // Add to history
        if !line.is_empty() {
            self.history.push(line.clone());
        }

        // Add to buffer
        self.buffer.push_str(&line);
        self.buffer.push('\n');
        self.line_count += 1;

        // Check if we have a complete expression (clone buffer to avoid borrow conflict)
        let buffer_ref = self.buffer.clone();
        if self.is_complete(&buffer_ref) {
            self.evaluate(&buffer_ref)
        } else {
            // Continue reading
            Ok(REPLResult::Ok)
        }
    }

    /// Check if the input is a complete expression
    fn is_complete(
        &self,
        code: &str,
    ) -> bool {
        let code = code.trim();
        if code.is_empty() {
            return true;
        }

        // Count braces, brackets, and parens
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
                '{' => {
                    if !in_string {
                        braces += 1;
                    }
                }
                '}' => {
                    if !in_string {
                        if braces == 0 {
                            return true;
                        }
                        braces -= 1;
                    }
                }
                '[' => {
                    if !in_string {
                        brackets += 1;
                    }
                }
                ']' => {
                    if !in_string {
                        if brackets == 0 {
                            return true;
                        }
                        brackets -= 1;
                    }
                }
                '(' => {
                    if !in_string {
                        parens += 1;
                    }
                }
                ')' => {
                    if !in_string {
                        if parens == 0 {
                            return true;
                        }
                        parens -= 1;
                    }
                }
                _ => {}
            }
        }

        braces == 0 && brackets == 0 && parens == 0 && !in_string && !escaped
    }

    /// Handle a special command
    fn handle_command(
        &mut self,
        command: &str,
    ) -> Result<REPLResult, io::Error> {
        match command {
            ":quit" | ":q" => Ok(REPLResult::Exit),
            ":help" | ":h" => {
                tlog!(info, MSG::ReplAvailableCommands);
                tlog!(info, MSG::ReplExitCommand);
                tlog!(info, MSG::ReplHelpCommand);
                tlog!(info, MSG::ReplClearCommand);
                tlog!(info, MSG::ReplHistoryCommand);
                Ok(REPLResult::Ok)
            }
            ":clear" | ":c" => {
                self.buffer.clear();
                self.line_count = 0;
                Ok(REPLResult::Ok)
            }
            ":history" | ":hist" => {
                for (i, line) in self.history.iter().enumerate() {
                    tlog!(
                        info,
                        MSG::ReplHistoryEntry,
                        &i.to_string(),
                        &line.to_string()
                    );
                }
                Ok(REPLResult::Ok)
            }
            _ => {
                tlog!(info, MSG::ReplUnknownCommand, &command.to_string());
                Ok(REPLResult::Ok)
            }
        }
    }

    /// Evaluate code
    fn evaluate(
        &mut self,
        code: &str,
    ) -> Result<REPLResult, io::Error> {
        // Wrap the code in a function
        let wrapped = format!("main() -> () = () => {{\n{}\n}}", code);

        // Compile
        let mut compiler = crate::frontend::Compiler::new();
        match compiler.compile("<repl>", &wrapped) {
            Ok(module_ir) => {
                // Generate bytecode
                match crate::middle::passes::codegen::CodegenContext::new(module_ir).generate() {
                    Ok(bytecode_file) => {
                        let bytecode_module =
                            crate::middle::bytecode::BytecodeModule::from(bytecode_file);

                        // Execute with interpreter
                        match self.interpreter.execute_module(&bytecode_module) {
                            Ok(_) => {
                                // Note: execute_module doesn't return the value yet
                                Ok(REPLResult::Ok)
                            }
                            Err(e) => Ok(REPLResult::Error(format!("Runtime error: {:?}", e))),
                        }
                    }
                    Err(e) => Ok(REPLResult::Error(format!("Codegen error: {:?}", e))),
                }
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                let lines: Vec<&str> = error_msg.lines().collect();
                if lines.len() > 2 {
                    Ok(REPLResult::Error(lines[lines.len() - 2..].join("\n")))
                } else {
                    Ok(REPLResult::Error(error_msg))
                }
            }
        }
    }

    /// Load and execute a file
    pub fn load_file(
        &mut self,
        path: &Path,
    ) -> Result<REPLResult, io::Error> {
        let source = std::fs::read_to_string(path)?;

        // Compile
        let mut compiler = crate::frontend::Compiler::new();
        match compiler.compile(&path.display().to_string(), &source) {
            Ok(module_ir) => {
                match crate::middle::passes::codegen::CodegenContext::new(module_ir).generate() {
                    Ok(bytecode_file) => {
                        let bytecode_module =
                            crate::middle::bytecode::BytecodeModule::from(bytecode_file);
                        match self.interpreter.execute_module(&bytecode_module) {
                            Ok(_) => Ok(REPLResult::Ok),
                            Err(e) => Ok(REPLResult::Error(format!("Runtime error: {:?}", e))),
                        }
                    }
                    Err(e) => Ok(REPLResult::Error(format!("Codegen error: {:?}", e))),
                }
            }
            Err(e) => Ok(REPLResult::Error(format!("{}", e))),
        }
    }

    /// Get the interpreter for external use
    pub fn interpreter(&mut self) -> &mut Interpreter {
        &mut self.interpreter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_repl_new() {
        let repl = REPL::new();
        assert!(repl.history.is_empty());
    }

    #[test]
    fn test_repl_is_complete() {
        let repl = REPL::new();

        // Complete expressions
        assert!(repl.is_complete("1 + 2"));
        assert!(repl.is_complete("let x = 42"));
        assert!(repl.is_complete("fn foo() { 1 }"));

        // Incomplete expressions (checked by balanced delimiters)
        assert!(!repl.is_complete("fn foo() {"));
        assert!(!repl.is_complete("if true {"));
        assert!(!repl.is_complete("{"));
    }
}
