//! Standard Dict library (YaoXiang)
//!
//! This module provides dictionary manipulation functions for YaoXiang programs.

use crate::backends::common::RuntimeValue;
use crate::backends::ExecutorError;
use crate::std::{NativeExport, StdModule, NativeHandler};

// ============================================================================
// DictModule - StdModule Implementation
// ============================================================================

/// Dict module implementation.
pub struct DictModule;

impl Default for DictModule {
    fn default() -> Self {
        Self
    }
}

impl StdModule for DictModule {
    fn module_path(&self) -> &str {
        "std.dict"
    }

    fn exports(&self) -> Vec<NativeExport> {
        vec![
            NativeExport::new(
                "get",
                "std.dict.get",
                "(dict: Dict, key: Any) -> Any",
                native_get as NativeHandler,
            ),
            NativeExport::new(
                "set",
                "std.dict.set",
                "(dict: Dict, key: Any, value: Any) -> Dict",
                native_set as NativeHandler,
            ),
            NativeExport::new(
                "has",
                "std.dict.has",
                "(dict: Dict, key: Any) -> Bool",
                native_has as NativeHandler,
            ),
            NativeExport::new(
                "values",
                "std.dict.values",
                "(dict: Dict) -> List",
                native_values as NativeHandler,
            ),
            NativeExport::new(
                "keys",
                "std.dict.keys",
                "(dict: Dict) -> List",
                native_keys as NativeHandler,
            ),
            NativeExport::new(
                "entries",
                "std.dict.entries",
                "(dict: Dict) -> List",
                native_entries as NativeHandler,
            ),
            NativeExport::new(
                "delete",
                "std.dict.delete",
                "(dict: Dict, key: Any) -> Dict",
                native_delete as NativeHandler,
            ),
            NativeExport::new(
                "len",
                "std.dict.len",
                "(dict: Dict) -> Int",
                native_len as NativeHandler,
            ),
            NativeExport::new(
                "is_empty",
                "std.dict.is_empty",
                "(dict: Dict) -> Bool",
                native_is_empty as NativeHandler,
            ),
            NativeExport::new(
                "merge",
                "std.dict.merge",
                "(a: Dict, b: Dict) -> Dict",
                native_merge as NativeHandler,
            ),
        ]
    }
}

/// Singleton instance for std.dict module.
pub const DICT_MODULE: DictModule = DictModule;

// ============================================================================
// Native function implementations
// ============================================================================

/// Native implementation: get - get value by key
fn native_get(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::Unit)
}

/// Native implementation: set - set key-value pair (consumes original dict)
/// Returns new dict with the key set
fn native_set(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return new Dict
    Ok(RuntimeValue::String("[Dict]".into()))
}

/// Native implementation: has - check if key exists
fn native_has(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::Bool(false))
}

/// Native implementation: values - get all values as list
fn native_values(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return List
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: keys - get all keys as list
fn native_keys(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return List
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: entries - get all key-value pairs as list of tuples
fn native_entries(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return List
    Ok(RuntimeValue::String("[List]".into()))
}

/// Native implementation: delete - remove key-value pair (consumes original)
/// Returns new dict without the key
fn native_delete(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return new Dict
    Ok(RuntimeValue::String("[Dict]".into()))
}

/// Native implementation: len - get number of entries
fn native_len(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::Int(0))
}

/// Native implementation: is_empty - check if dict is empty
fn native_is_empty(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access
    Ok(RuntimeValue::Bool(true))
}

/// Native implementation: merge - merge two dicts (consumes both)
/// Second dict values override first dict values for duplicate keys
fn native_merge(args: &[RuntimeValue]) -> Result<RuntimeValue, ExecutorError> {
    // TODO: Requires heap access to return new Dict
    Ok(RuntimeValue::String("[Dict]".into()))
}
