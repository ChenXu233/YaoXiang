//! REPL Evaluation Engine
//!
//! Core engine for compiling and executing REPL input.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::backends::common::RuntimeValue;
use crate::backends::interpreter::Interpreter;
use crate::backends::Executor;
use crate::frontend::Compiler;
use crate::middle::bytecode::BytecodeModule;
use crate::middle::passes::codegen::CodegenContext;

use super::backend::{EvalResult, ExecutionStats, REPLBackend, SymbolInfo};

// =============================================================================
// REPL Context
// =============================================================================

/// Variable info stored in context
#[derive(Debug, Clone)]
pub enum VariableInfo {
    /// Runtime value known
    Value(RuntimeValue),
    /// Only type known (from bytecode)
    TypeOnly(String),
}

/// Function info stored in context
#[derive(Debug, Clone)]
struct FunctionInfo {
    /// Function signature
    signature: String,
    /// Return type
    return_type: String,
}

/// REPL Execution Context
///
/// Stores variables, functions, and execution state across evaluations.
#[derive(Debug, Default)]
pub struct REPLContext {
    /// Variable environment: name -> info
    variables: HashMap<String, VariableInfo>,
    /// Function environment: name -> info
    functions: HashMap<String, FunctionInfo>,
    /// Execution statistics
    stats: ExecutionStats,
}

impl REPLContext {
    /// Create a new context
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a variable with runtime value
    pub fn define_var(
        &mut self,
        name: String,
        value: RuntimeValue,
    ) {
        self.variables.insert(name, VariableInfo::Value(value));
    }

    /// Define a variable by type only (for bytecode extraction)
    pub fn define_variable(
        &mut self,
        name: String,
        type_signature: String,
    ) {
        self.variables
            .insert(name, VariableInfo::TypeOnly(type_signature));
    }

    /// Get a variable
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&RuntimeValue> {
        match self.variables.get(name) {
            Some(VariableInfo::Value(v)) => Some(v),
            _ => None,
        }
    }

    /// Get variable type
    pub fn get_var_type(
        &self,
        name: &str,
    ) -> Option<String> {
        match self.variables.get(name) {
            Some(VariableInfo::Value(v)) => Some(format!("{:?}", v.value_type_simple())),
            Some(VariableInfo::TypeOnly(t)) => Some(t.clone()),
            None => None,
        }
    }

    /// Define a function
    pub fn define_function(
        &mut self,
        name: String,
        signature: String,
        return_type: String,
    ) {
        self.functions.insert(
            name,
            FunctionInfo {
                signature,
                return_type,
            },
        );
    }

    /// Get all defined symbols
    pub fn get_all_symbols(&self) -> Vec<SymbolInfo> {
        let mut symbols = Vec::new();

        for (name, info) in &self.variables {
            let type_sig = match info {
                VariableInfo::Value(v) => format!("{:?}", v.value_type_simple()),
                VariableInfo::TypeOnly(t) => t.clone(),
            };
            symbols.push(SymbolInfo {
                name: name.clone(),
                type_signature: type_sig,
                doc: None,
            });
        }

        for (name, func) in &self.functions {
            symbols.push(SymbolInfo {
                name: name.clone(),
                type_signature: func.signature.clone(),
                doc: None,
            });
        }

        symbols
    }

    /// Get type of a symbol
    pub fn get_symbol_type(
        &self,
        name: &str,
    ) -> Option<String> {
        if let Some(t) = self.get_var_type(name) {
            Some(t)
        } else {
            self.functions
                .get(name)
                .map(|func| func.return_type.clone())
        }
    }

    /// Increment eval count
    pub fn increment_eval(
        &mut self,
        duration: Duration,
    ) {
        self.stats.eval_count += 1;
        self.stats.total_time += duration;
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.variables.clear();
        self.functions.clear();
        self.stats = ExecutionStats::default();
    }

    /// Get statistics
    pub fn stats(&self) -> ExecutionStats {
        self.stats.clone()
    }
}

// =============================================================================
// Evaluator
// =============================================================================

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
            Ok(module_ir) => match CodegenContext::new(module_ir).generate() {
                Ok(bytecode_file) => {
                    let bytecode_module = BytecodeModule::from(bytecode_file);

                    match self.interpreter.execute_module(&bytecode_module) {
                        Ok(_) => {
                            self.context.increment_eval(start.elapsed());
                            self.extract_definitions(&bytecode_module);
                            EvalResult::Ok
                        }
                        Err(e) => EvalResult::Error(format!("Runtime error: {:?}", e)),
                    }
                }
                Err(e) => EvalResult::Error(format!("Codegen error: {:?}", e)),
            },
            Err(e) => {
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

    /// Wrap code for REPL evaluation
    fn wrap_code(
        &self,
        code: &str,
    ) -> String {
        let trimmed = code.trim();

        if self.is_statement(trimmed) {
            format!("main() -> () = () => {{ {} }}", code)
        } else {
            format!("main() -> _ = () => {{ {} }}", code)
        }
    }

    /// Check if code is a statement
    fn is_statement(
        &self,
        code: &str,
    ) -> bool {
        let first_word = code.split_whitespace().next();
        matches!(
            first_word,
            Some("let")
                | Some("if")
                | Some("match")
                | Some("for")
                | Some("while")
                | Some("fn")
                | Some("struct")
                | Some("enum")
                | Some("trait")
                | Some("impl")
                | Some("return")
                | Some("break")
                | Some("continue")
        )
    }

    /// Extract defined variables and functions to context
    fn extract_definitions(
        &mut self,
        module: &BytecodeModule,
    ) {
        for func in &module.functions {
            if func.name == "main" {
                continue;
            }

            let params: Vec<String> = func.params.iter().map(|p| format!("{:?}", p)).collect();
            let signature = format!("fn({}) -> {:?}", params.join(", "), func.return_type);

            self.context.define_function(
                func.name.clone(),
                signature,
                format!("{:?}", func.return_type),
            );
        }

        for global in &module.globals {
            if global.name.starts_with('$') {
                continue;
            }

            let type_str = if (global.type_id as usize) < module.type_table.len() {
                format!("{:?}", module.type_table[global.type_id as usize])
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
