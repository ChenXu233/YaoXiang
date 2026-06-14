//! Wasm Playground entry point.
//!
//! Exposes `run_code()` to JavaScript via wasm-bindgen,
//! allowing the YaoXiang compiler + interpreter to run in the browser.

use wasm_bindgen::prelude::*;
use crate::backends::Executor;

/// Initialize panic hook for better error messages in the browser console.
#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

/// Simple test function to verify wasm module loads
#[wasm_bindgen]
pub fn ping() -> String {
    "pong".to_string()
}

/// Test: just create a compiler
#[wasm_bindgen]
pub fn test_compiler() -> String {
    let _compiler = crate::frontend::Compiler::new();
    "compiler created".to_string()
}

/// Test: compile only
#[wasm_bindgen]
pub fn test_compile(source: &str) -> String {
    let mut compiler = crate::frontend::Compiler::new();
    match compiler.compile_with_source("<test>", source) {
        Ok(_) => "compiled ok".to_string(),
        Err(e) => format!("error: {}", e),
    }
}

/// Compile and execute YaoXiang source code.
///
/// Returns the captured output (from `print`/`println`) or error messages.
#[wasm_bindgen]
pub fn run_code(source: &str) -> String {
    // Clear output buffer
    crate::std::io::wasm_output::clear();

    // Compile
    let mut compiler = crate::frontend::Compiler::new();
    let module = match compiler.compile_with_source("<playground>", source) {
        Ok(m) => m,
        Err(e) => return format!("Compilation Error:\n{}", e),
    };

    // Generate bytecode
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);
    let bytecode_file = match ctx.generate() {
        Ok(b) => b,
        Err(e) => return format!("Codegen Error:\n{:?}", e),
    };

    // Convert to BytecodeModule
    let bytecode_module = crate::middle::bytecode::BytecodeModule::from(bytecode_file);

    // Execute
    let mut interpreter = crate::backends::interpreter::Interpreter::new();
    if let Err(e) = interpreter.execute_module(&bytecode_module) {
        let mut output = crate::std::io::wasm_output::take();
        output.push_str(&format!("Runtime Error:\n{}", e));
        return output;
    }

    // Return captured output
    crate::std::io::wasm_output::take()
}
