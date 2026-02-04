//! REPL Execution Context
//!
//! Manages variable and function state for REPL sessions.

use std::collections::HashMap;
use std::time::Duration;

use crate::backends::common::RuntimeValue;
use crate::backends::dev::repl::backend_trait::{SymbolInfo, ExecutionStats};

/// REPL Execution Context
///
/// Stores variables, functions, and execution state across evaluations.
#[derive(Debug, Default)]
pub struct REPLContext {
    /// Variable environment: name -> value
    variables: HashMap<String, RuntimeValue>,
    /// Function environment: name -> bytecode function
    functions: HashMap<String, FunctionInfo>,
    /// Execution statistics
    stats: ExecutionStats,
    /// Eval count for this session
    eval_count: usize,
    /// Total execution time
    total_time: Duration,
}

#[derive(Debug, Clone)]
struct FunctionInfo {
    /// Function signature
    signature: String,
    /// Return type
    return_type: String,
}

impl REPLContext {
    /// Create a new context
    pub fn new() -> Self {
        Self::default()
    }

    /// Define a variable
    pub fn define_var(
        &mut self,
        name: String,
        value: RuntimeValue,
    ) {
        self.variables.insert(name, value);
    }

    /// Get a variable
    pub fn get_var(
        &self,
        name: &str,
    ) -> Option<&RuntimeValue> {
        self.variables.get(name)
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

        // Add variables
        for (name, value) in &self.variables {
            symbols.push(SymbolInfo {
                name: name.clone(),
                type_signature: format!("{:?}", value.value_type_simple()),
                doc: None,
            });
        }

        // Add functions
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
        if let Some(value) = self.variables.get(name) {
            Some(format!("{:?}", value.value_type_simple()))
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
        self.eval_count += 1;
        self.total_time += duration;
        self.stats.eval_count = self.eval_count;
        self.stats.total_time = self.total_time;
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.variables.clear();
        self.functions.clear();
        self.eval_count = 0;
        self.total_time = Duration::ZERO;
        self.stats = ExecutionStats::default();
    }

    /// Get statistics
    pub fn stats(&self) -> ExecutionStats {
        ExecutionStats {
            eval_count: self.eval_count,
            total_time: self.total_time,
        }
    }
}
