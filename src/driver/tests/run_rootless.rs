//! rootless（standalone / Script 语义）多文件 **run** 形态的编排钉测试 — 基于
//! WBS 4.10.3（4.2.5 裁决 A 的 run 半边）与 #413 T4 入口语义。
//!
//! 形态：无项目根（无 yaoxiang.toml）的入口文件经 Program{MultiFile} 编排运行。
//! 与 check 半边（check_rootless_mono.rs）同一家族的 run 侧判据：
//! - Script 无入口概念——Linking 不得给 rootless 合并 IR 设 entry_function
//!   （#413 T4：顶层语句即程序主体，自动调 main 会与显式 main() 双跑，#356）；
//! - Script 语义保持——顶层可执行语句不报 E3023（ir_gen 的 script_mode 由
//!   rootless 驱动，与限定名键解耦，同 3.4.8 ③ 的 check 半边）；
//! - Bin 形态对照——带 manifest 的项目入口语义不变（entry_function 照旧设置），
//!   证明 Script 侧断言不是空转。
//!
//! 判据来源：WBS 4.10.3；#413（T4 决议）；#356（防双跑）；RFC-029f 角色模型。

use std::path::Path;

use super::check_rootless_mono::{make_bin_project, make_rootless_project};
use crate::driver::{Driver, DriverError, DriverOutcome, Program, ProgramKind, Unit};
use crate::frontend::config::CompileConfig;

/// 以 MultiFile 形态跑 Driver（与 `yaoxiang run` 同一入口形态，即 compile_project）。
fn run_multifile(entry: &Path) -> Result<DriverOutcome, DriverError> {
    let program = Program::new(
        ProgramKind::MultiFile,
        vec![Unit::new("main", entry, "")],
        CompileConfig::default(),
    );
    Driver::new().run(program)
}

#[test]
fn test_run_rootless_merged_ir_has_no_entry_function() {
    // Arrange - 无 manifest：main.yx 定义函数形态 main
    let (_dir, entry) = make_rootless_project(&[("main.yx", "main = () => { x = 1 }\n")]);

    // Act
    let outcome = run_multifile(&entry).expect("rootless MultiFile 编排应成功");

    // Assert - #413 T4：Script 无入口概念，merged IR 不得携带 entry_function
    let module = outcome.module.expect("成功编排必须产 merged IR");
    assert!(
        module.entry_function.is_none(),
        "Script（rootless）不得自动调 main：entry_function 应为 None，实际 {:?}",
        module.entry_function
    );
    assert!(
        module.functions.iter().any(|f| f.name == "main.main"),
        "函数本体仍应限定名存在于 IR 中（不被删除）：{:?}",
        module.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
    );
}

#[test]
fn test_run_bin_project_merged_ir_keeps_entry_function() {
    // Arrange - 带 manifest 的 Bin 项目：入口语义不变（对照组）
    let (_dir, entry) = make_bin_project(&[("main.yx", "main = () => { x = 1 }\n")]);

    // Act
    let outcome = run_multifile(&entry).expect("Bin MultiFile 编排应成功");

    // Assert - Bin 照旧自动入口：entry_function 指向限定名 main
    let module = outcome.module.expect("成功编排必须产 merged IR");
    assert_eq!(
        module.entry_function.as_deref(),
        Some("main.main"),
        "Bin 项目入口应保持 {{entry_key}}.main（对照证明 Script 侧不是空转）"
    );
}

#[test]
fn test_run_rootless_top_level_statement_compiles() {
    // Arrange - 无 manifest：顶层可执行语句（Script 语义合法；Bin 语义是 E3023）
    let (_dir, entry) = make_rootless_project(&[("main.yx", "print(\"script-top-level\")\n")]);

    // Act
    let outcome = run_multifile(&entry);

    // Assert - Script 语义：顶层语句即程序主体，编排必须成功产 IR
    let outcome = outcome.unwrap_or_else(|e| {
        panic!("rootless 顶层可执行语句应正常编排（Script 语义），实际故障: {e:?}")
    });
    assert!(
        outcome.module.is_some(),
        "Script 顶层语句必须产出 merged IR（不得报 E3023）"
    );
}
