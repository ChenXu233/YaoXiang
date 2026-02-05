//! REPL Execution Context
//!
//! Manages variable and function state for REPL sessions.

use std::collections::HashMap;
use std::time::Duration;

use crate::backends::common::RuntimeValue;
use crate::backends::dev::repl::backend_trait::{SymbolInfo, ExecutionStats};

/// Variable info stored in context
#[derive(Debug, Clone)]
pub enum VariableInfo {
    /// Runtime value known
    Value(RuntimeValue),
    /// Only type known (from bytecode)
    TypeOnly(String),
}

/// REPL Execution Context
///
/// Stores variables, functions, and execution state across evaluations.
#[derive(Debug, Default)]
pub struct REPLContext {
    /// Variable environment: name -> info
    variables: HashMap<String, VariableInfo>,
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
        self.variables.insert(name, VariableInfo::TypeOnly(type_signature));
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

        // Add variables
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
