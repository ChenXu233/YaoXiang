//! REPL Evaluation Engine
//!
//! Core engine for compiling and executing REPL input.

use std::time::Instant;

use crate::backends::interpreter::Interpreter;
use crate::backends::Executor;
use crate::frontend::Compiler;
use crate::middle::passes::codegen::CodegenContext;
use crate::middle::bytecode::BytecodeModule;

use super::context::REPLContext;
use crate::backends::dev::repl::backend_trait::{EvalResult, REPLBackend, SymbolInfo, ExecutionStats};

/// Evaluation Engine
///
/// The core engine that compiles and executes REPL input.
/// It wraps code in a main function, compiles it, and executes it.
#[derive(Debug)]
pub struct Evaluator {
    /// Compiler instance
    compiler: Compiler,
    /// Interpreter for execution
    interpreter: Interpreter,
    /// Execution context
    context: REPLContext,
}

impl Default for Evaluator {
    fn default() -> Self {
        Self::new()
    }
}

impl Evaluator {
    /// Create a new evaluator
    pub fn new() -> Self {
        Self {
            compiler: Compiler::new(),
            interpreter: Interpreter::new(),
            context: REPLContext::new(),
        }
    }

    /// Evaluate code
    pub fn evaluate(
        &mut self,
        code: &str,
    ) -> EvalResult {
        let start = Instant::now();
        let trimmed = code.trim();

        if trimmed.is_empty() {
            return EvalResult::Ok;
        }

        // Check if input is complete
        if !self.is_complete(trimmed) {
            return EvalResult::Incomplete;
        }

        // Wrap code for REPL evaluation
        let wrapped = self.wrap_code(trimmed);

        // Compile
        match self.compiler.compile("<repl>", &wrapped) {
            Ok(module_ir) => {
                // Generate bytecode
                match CodegenContext::new(module_ir).generate() {
                    Ok(bytecode_file) => {
                        let bytecode_module = BytecodeModule::from(bytecode_file);

                        // Execute
                        match self.interpreter.execute_module(&bytecode_module) {
                            Ok(_) => {
                                // Update stats
                                self.context.increment_eval(start.elapsed());

                                // Extract definitions to context
                                self.extract_definitions(&bytecode_module);

                                // Return Ok (module executed successfully)
                                EvalResult::Ok
                            }
                            Err(e) => EvalResult::Error(format!("Runtime error: {:?}", e)),
                        }
                    }
                    Err(e) => EvalResult::Error(format!("Codegen error: {:?}", e)),
                }
            }
            Err(e) => {
                // Format error nicely
                let error_msg = format!("{}", e);
                let lines: Vec<&str> = error_msg.lines().collect();
                if lines.len() > 2 {
                    EvalResult::Error(lines[lines.len() - 2..].join("\n").to_string())
                } else {
                    EvalResult::Error(error_msg)
                }
            }
        }
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

    /// Wrap code for REPL evaluation
    fn wrap_code(
        &self,
        code: &str,
    ) -> String {
        let trimmed = code.trim();

        // Check if it's an expression (not a statement)
        if self.is_expression(trimmed) {
            format!("main() -> _ = () => {{ {}; }}", code)
        } else {
            format!("main() -> () = () => {{ {} }}", code)
        }
    }

    /// Check if code is an expression
    fn is_expression(
        &self,
        code: &str,
    ) -> bool {
        let first_word = code.split_whitespace().next();
        matches!(
            first_word,
            Some("let") | Some("if") | Some("match") | Some("for")
        )
    }

    /// Extract defined variables and functions to context
    fn extract_definitions(
        &mut self,
        module: &BytecodeModule,
    ) {
        // Extract function definitions (skip main wrapper)
        for func in &module.functions {
            // Skip the main wrapper function
            if func.name == "main" {
                continue;
            }

            // Build signature string
            let params: Vec<String> = func.params.iter().map(|p| format!("{:?}", p)).collect();
            let signature = format!("fn({}) -> {:?}", params.join(", "), func.return_type);

            self.context.define_function(
                func.name.clone(),
                signature,
                format!("{:?}", func.return_type),
            );
        }

        // Extract global variable definitions
        for global in &module.globals {
            // Skip internal variables
            if global.name.starts_with("$") {
                continue;
            }

            // Get type from type table
            let type_str = if (global.type_id as usize) < module.type_table.len() {
                format!("{:?}", &module.type_table[global.type_id as usize])
            } else {
                "unknown".to_string()
            };

            self.context.define_variable(global.name.clone(), type_str);
        }
    }

    /// Get context reference
    pub fn context(&self) -> &REPLContext {
        &self.context
    }

    /// Get mutable context reference
    pub fn context_mut(&mut self) -> &mut REPLContext {
        &mut self.context
    }
}

impl REPLBackend for Evaluator {
    fn eval(
        &mut self,
        code: &str,
    ) -> EvalResult {
        self.evaluate(code)
    }

    fn complete(
        &self,
        line: &str,
        _pos: usize,
    ) -> Vec<String> {
        self.context
            .get_all_symbols()
            .iter()
            .filter(|s| s.name.starts_with(line))
            .map(|s| s.name.clone())
            .collect()
    }

    fn get_symbols(&self) -> Vec<SymbolInfo> {
        self.context.get_all_symbols()
    }

    fn get_type(
        &self,
        name: &str,
    ) -> Option<String> {
        self.context.get_symbol_type(name)
    }

    fn clear(&mut self) {
        self.context.clear();
    }

    fn stats(&self) -> ExecutionStats {
        self.context.stats()
    }
}
