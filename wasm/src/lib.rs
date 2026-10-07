//! YaoXiang Wasm Playground
//!
//! Exposes `run_code()` to JavaScript via wasm-bindgen,
//! allowing the YaoXiang compiler + interpreter to run in the browser.

use wasm_bindgen::prelude::*;

// 裸导出带出（#435 观测口）：terminated 后 JS 直读内存的唯一通道。
pub use yaoxiang::util::{shim_log_len, shim_log_ptr};
use yaoxiang::backends::Executor;

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
    let _compiler = yaoxiang::frontend::Compiler::new();
    "compiler created".to_string()
}

/// Test: SMT 求解器在 wasm 下真实可用（#435——wasm 形态必须带 Z3）
///
/// 构造 `x > 5 ∧ x < 3`（不可满足）送 Z3：返回 Unsat 证明求解器真在跑；
/// 任何降级形态（solver 缺失 / stub / 假链接）都会得到不同结果。
#[wasm_bindgen]
pub fn test_smt() -> String {
    use yaoxiang::frontend::core::typecheck::proof::smt::ast::{SMTCommand, SMTExpr, SMTSort};
    use yaoxiang::frontend::core::typecheck::proof::smt::backend::default_solver;

    let Some(solver) = default_solver() else {
        return "solver unavailable".to_string();
    };
    let commands = vec![
        SMTCommand::DeclareConst("x".to_string(), SMTSort::Int),
        SMTCommand::Assert(SMTExpr::App(
            ">".to_string(),
            vec![SMTExpr::Atom("x".to_string()), SMTExpr::Atom("5".to_string())],
        )),
        SMTCommand::Assert(SMTExpr::App(
            "<".to_string(),
            vec![SMTExpr::Atom("x".to_string()), SMTExpr::Atom("3".to_string())],
        )),
        SMTCommand::CheckSat,
    ];
    match solver.solve(&commands, 5000) {
        yaoxiang::frontend::core::typecheck::proof::smt::ast::SMTResult::Unsat => {
            "unsat (z3 alive)".to_string()
        }
        other => format!("unexpected: {:?}", other),
    }
}

/// Test: compile only
#[wasm_bindgen]
pub fn test_compile(source: &str) -> String {
    let mut compiler = yaoxiang::frontend::Compiler::new();
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
    yaoxiang::std::io::wasm_output::clear();

    // Compile
    let mut compiler = yaoxiang::frontend::Compiler::new();
    let module = match compiler.compile_with_source("<playground>", source) {
        Ok(m) => m,
        Err(e) => return format!("Compilation Error:\n{}", e),
    };

    // Generate bytecode
    let mut ctx = yaoxiang::middle::passes::codegen::CodegenContext::new(module);
    let bytecode_file = match ctx.generate() {
        Ok(b) => b,
        Err(e) => return format!("Codegen Error:\n{:?}", e),
    };

    // Convert to BytecodeModule
    let bytecode_module = yaoxiang::middle::bytecode::BytecodeModule::from(bytecode_file);

    // Execute
    let mut interpreter = yaoxiang::backends::interpreter::Interpreter::new();
    if let Err(e) = interpreter.execute_module(&bytecode_module) {
        let mut output = yaoxiang::std::io::wasm_output::take();
        output.push_str(&format!("Runtime Error:\n{}", e));
        return output;
    }

    // Return captured output
    yaoxiang::std::io::wasm_output::take()
}
