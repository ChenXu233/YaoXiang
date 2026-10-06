//! 集成测试共享助手（tests/ 各 target 经 `mod common;` 引入）。
//!
//! cargo 约定：`tests/` 的一级子目录不会被当作独立 test target，
//! 故本文件是放置跨 target 共享代码的标准位置（与 tests/integration/ 同构）。
//!
//! 禁令三合规：`yaoxiang-rs` 二进制定位与 check/run 子进程拉起全仓只有这一处
//! 实现，单文件语料（yx_runner）与多文件语料（yx_multifile_runner，RFC-039 D48）
//! 两个运行器共用；判定契约（头部指令）则由 `util::test_markers::TestFileSpec`
//! 在库侧单点实现。
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// Locate the `yaoxiang-rs` binary.
///
/// Priority:
/// 1. `CARGO_BIN_EXE_yaoxiang-rs` (set by `cargo test`)
/// 2. Build once with `cargo build` and use the output path
pub fn binary_name() -> String {
    // When running via `cargo test`, the binary should be discoverable
    // via CARGO_BIN_EXE_yaoxiang-rs (set by cargo test --test).
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_yaoxiang-rs") {
        return path;
    }
    // Build once and use the binary directly
    let build_output = Command::new("cargo")
        .args(["build", "--bin", "yaoxiang-rs"])
        .output()
        .expect("Failed to build yaoxiang-rs binary");
    if !build_output.status.success() {
        panic!(
            "Failed to build yaoxiang-rs:\n{}",
            String::from_utf8_lossy(&build_output.stderr)
        );
    }
    // cargo build puts the binary in target/debug/yaoxiang-rs
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let ext = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    let path = format!("{manifest_dir}/target/debug/yaoxiang-rs{ext}");
    // Verify it exists
    assert!(
        std::path::Path::new(&path).exists(),
        "Built binary not found at {path}"
    );
    path
}

/// `check` 子进程（编译期判定步）。file 可以是单文件，也可以是项目入口
/// （项目内文件自动走多文件编排路径，RFC-029）。
pub fn spawn_check(
    binary: &str,
    file: &Path,
) -> Output {
    Command::new(binary)
        .arg("check")
        .arg(file)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run {binary} check for {}: {e}", file.display()))
}

/// `run` 子进程（`mode` 透传 `--runtime`）。
pub fn spawn_run(
    binary: &str,
    file: &Path,
    mode: Option<&String>,
) -> Output {
    let mut command = Command::new(binary);
    command.arg("run");
    if let Some(mode) = mode {
        command.arg("--runtime").arg(mode);
    }
    command.arg(file);
    command
        .output()
        .unwrap_or_else(|e| panic!("Failed to run {binary} for {}: {e}", file.display()))
}

/// 递归收集 dir 下全部 .yx 文件（路径排序由调用方负责）。
pub fn collect_yx_files(
    dir: &Path,
    files: &mut Vec<PathBuf>,
) {
    if !dir.is_dir() {
        return;
    }
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_yx_files(&path, files);
        } else if path.extension().is_some_and(|e| e == "yx") {
            files.push(path);
        }
    }
}
