//! E2E Test Runner for YaoXiang (.yx) test files
//!
//! Discovers all `*.yx` files under `tests/yaoxiang/`, runs each through the
//! `yaoxiang run` binary, and verifies the output contains `ALL TESTS PASSED`.
//!
//! Directory structure:
//!
//! ```text
//! tests/yaoxiang/
//! ├── 00-smoke/         # 冒烟测试
//! ├── 01-basics/        # 基本语法
//! ├── 02-functions/     # 函数定义与调用
//! ├── 03-control-flow/  # 控制流
//! ├── 04-types/         # 类型系统
//! ├── 05-data-structures/
//! ├── 06-modules/       # 模块系统
//! └── 07-errors/        # 错误处理
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

/// Find all `.yx` test files under `tests/yaoxiang/`, excluding `.skip` files.
fn discover_yx_tests() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("yaoxiang");
    let mut files = Vec::new();
    collect_yx_files(&root, &mut files);
    files.sort();
    files
}

fn collect_yx_files(
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
        } else if path.extension().map_or(false, |e| e == "yx") {
            files.push(path);
        }
    }
}

/// Locate the `yaoxiang` binary (either from `cargo run` or pre-built).
fn binary_name() -> String {
    // When running via `cargo test`, the binary should be discoverable
    // via CARGO_BIN_EXE_yaoxiang (set by cargo test --test).
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_yaoxiang") {
        return path;
    }
    // Fallback: use `cargo run -- `
    "cargo".to_string()
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_all_yx_files_pass() {
    let files = discover_yx_tests();
    assert!(!files.is_empty(), "No .yx test files found!");

    let binary = binary_name();
    let using_cargo = binary == "cargo";
    let mut failed = Vec::new();

    for file in &files {
        let relative = file
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(file)
            .display()
            .to_string();

        let output = if using_cargo {
            // FIXME: This approach is slower. For CI, prefer CARGO_BIN_EXE_yaoxiang.
            Command::new("cargo")
                .arg("run")
                .arg("--")
                .arg("run")
                .arg(file)
                .output()
                .unwrap_or_else(|e| panic!("Failed to run cargo for {relative}: {e}"))
        } else {
            Command::new(&binary)
                .arg("run")
                .arg(file)
                .output()
                .unwrap_or_else(|e| panic!("Failed to run {binary} for {relative}: {e}"))
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        if !stdout.contains("ALL TESTS PASSED") {
            failed.push((relative, code, stdout, stderr));
        }
    }

    if !failed.is_empty() {
        let mut msg = String::from("\n\n========== FAILED YX TESTS ==========\n");
        for (name, code, stdout, stderr) in &failed {
            msg.push_str(&format!("\n--- {name} (exit code: {code}) ---\n"));
            if !stdout.is_empty() {
                msg.push_str(&format!("STDOUT:\n{stdout}\n"));
            }
            if !stderr.is_empty() {
                msg.push_str(&format!("STDERR:\n{stderr}\n"));
            }
        }
        panic!("{msg}");
    }
}

/// Verify that each `.yx` file contains the required metadata header.
#[test]
fn test_yx_file_headers() {
    let files = discover_yx_tests();
    assert!(!files.is_empty(), "No .yx test files found!");

    for file in &files {
        let content =
            std::fs::read_to_string(file).unwrap_or_else(|e| panic!("Cannot read {:?}: {e}", file));
        let first_line = content.lines().next().unwrap_or("");

        // Every .yx file should start with `//` comment (module header)
        assert!(
            first_line.starts_with("//"),
            "Test file {:?} must start with a '//' header comment\n\
             Found: {first_line:?}",
            file
        );
    }
}
