#[path = "integration/backends.rs"]
mod backends;
#[path = "integration/check.rs"]
mod check;
#[path = "integration/codegen.rs"]
mod codegen;
#[path = "integration/codegen_extended.rs"]
mod codegen_extended;
#[path = "integration/curry_codegen.rs"]
mod curry_codegen;
#[path = "integration/curry_execution.rs"]
mod curry_execution;
#[path = "integration/curry_regression.rs"]
mod curry_regression;
#[path = "integration/eval.rs"]
mod eval;
#[path = "integration/execution.rs"]
mod execution;
#[path = "integration/feature_flags.rs"]
mod feature_flags;
#[path = "integration/fstring.rs"]
mod fstring;
#[path = "integration/interpreter.rs"]
mod interpreter;
#[path = "integration/token_system.rs"]
mod token_system;

/// `yaoxiang` CLI 子命令集成测试
#[path = "integration/cli.rs"]
mod cli;

/// `yaoxiang` CLI 子命令端到端测试（子进程级）
#[path = "integration/cli_e2e.rs"]
mod cli_e2e;
