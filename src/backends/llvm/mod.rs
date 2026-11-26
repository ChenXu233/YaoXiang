//! LLVM AOT 后端（RFC-018，AOT-0/AOT-1）。
//!
//! 输入是既有 Driver 已完成单态化的 `ModuleIR`，输出是目标文件与原生可执行文件。
//! 不读取 `.42`，不实现字节码 `Executor`，也不反查 AST。

mod analysis;
mod codegen;
mod linker;

#[cfg(test)]
mod tests;

use std::ffi::OsString;
use std::path::Path;

use anyhow::{anyhow, Context, Result};
use inkwell::passes::PassBuilderOptions;
use inkwell::targets::{
    CodeModel, FileType, InitializationConfig, RelocMode, Target, TargetTriple,
};
use inkwell::OptimizationLevel;
use tempfile::TempDir;

use crate::middle::ir::ir::ModuleIR;

use analysis::analyze_module;
use codegen::Emitter;
use linker::link_executable;

const TARGET_TRIPLE: &str = "x86_64-unknown-linux-gnu";

/// 将最终模块 IR 编译为 Linux x86-64 可执行文件。
///
/// `optimize` 为 false 时保留 O0，true 时运行 LLVM `default<O2>`。
/// 所有输出在链接成功后原子替换既有文件。
pub fn compile_ir_to_executable(
    ir: &ModuleIR,
    source_name: &str,
    output: &Path,
    optimize: bool,
) -> Result<()> {
    let analysis = analyze_module(ir, source_name)?;
    let context = inkwell::context::Context::create();
    let emitter = Emitter::new(&context, analysis, source_name);
    let module = emitter.emit(ir)?;

    Target::initialize_x86(&InitializationConfig::default());
    let triple = TargetTriple::create(TARGET_TRIPLE);
    let target = Target::from_triple(&triple)
        .map_err(|e| anyhow!("failed to select LLVM target {TARGET_TRIPLE}: {e}"))?;
    let level = if optimize {
        OptimizationLevel::Aggressive
    } else {
        OptimizationLevel::None
    };
    let machine = target
        .create_target_machine(
            &triple,
            "x86-64",
            "",
            level,
            RelocMode::PIC,
            CodeModel::Default,
        )
        .ok_or_else(|| anyhow!("failed to create LLVM target machine for {TARGET_TRIPLE}"))?;
    module.set_triple(&triple);
    let data_layout = machine.get_target_data().get_data_layout();
    module.set_data_layout(&data_layout);
    module.verify().map_err(|_| {
        anyhow!(
            "native AOT LLVM verification failed for {source_name}; the module is outside the native AOT support set"
        )
    })?;
    if optimize {
        let passes = PassBuilderOptions::create();
        passes.set_verify_each(true);
        module
            .run_passes("default<O2>", &machine, passes)
            .map_err(|e| anyhow!("LLVM O2 pipeline failed: {e}"))?;
        module.verify().map_err(|_| {
            anyhow!(
                "native AOT LLVM verification failed for {source_name} after optimization; the module is outside the AOT-3.1 support set"
            )
        })?;
    }

    let work = TempDir::new().context("failed to create LLVM object scratch directory")?;
    let object = work.path().join("module.o");
    machine
        .write_to_file(&module, FileType::Object, &object)
        .map_err(|e| anyhow!("failed to emit native object {}: {e}", object.display()))?;
    let linker = std::env::var_os("YAOXIANG_CC").unwrap_or_else(|| OsString::from("cc"));
    link_executable(&object, output, &linker)
}
