//! E2E Test Runner for YaoXiang (.yx) test files
//!
//! Discovers all `*.yx` files under `tests/yaoxiang/`, runs each through the
//! `yaoxiang run` binary, and verifies the output contains `ALL TESTS PASSED`.
//!
//! Directory structure (aligned with `docs/src/reference/language-spec/`):
//!
//! ```text
//! tests/yaoxiang/
//! ├── 00-smoke/             # 冒烟测试
//! ├── 01-syntax/            # 语法规范（对应 syntax.md）
//! │   ├── basics/           #   基本语法
//! │   ├── functions/        #   函数定义与调用
//! │   └── control-flow/     #   控制流
//! ├── 02-type-system/       # 类型系统（对应 type-system.md）
//! ├── 03-modules/           # 模块系统（对应 modules.md）
//! ├── 04-concurrency/       # 并发模型（对应 concurrency.md）
//! ├── 05-ownership/         # 所有权（独立章节）
//! ├── 06-compile-errors/    # 编译期错误检测
//! └── 99-demos/             # 论文演示（非规范测试）
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
        } else if path.extension().is_some_and(|e| e == "yx") {
            files.push(path);
        }
    }
}

/// Locate the `yaoxiang` binary.
///
/// Priority:
/// 1. `CARGO_BIN_EXE_yaoxiang` (set by `cargo test`)
/// 2. Build once with `cargo build` and use the output path
fn binary_name() -> String {
    // When running via `cargo test`, the binary should be discoverable
    // via CARGO_BIN_EXE_yaoxiang (set by cargo test --test).
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_yaoxiang") {
        return path;
    }
    // Build once and use the binary directly
    let build_output = Command::new("cargo")
        .args(["build", "--bin", "yaoxiang"])
        .output()
        .expect("Failed to build yaoxiang binary");
    if !build_output.status.success() {
        panic!(
            "Failed to build yaoxiang:\n{}",
            String::from_utf8_lossy(&build_output.stderr)
        );
    }
    // cargo build puts the binary in target/debug/yaoxiang
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let ext = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };
    let path = format!("{manifest_dir}/target/debug/yaoxiang{ext}");
    // Verify it exists
    assert!(
        std::path::Path::new(&path).exists(),
        "Built binary not found at {path}"
    );
    path
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_all_yx_files_pass() {
    let files = discover_yx_tests();
    assert!(!files.is_empty(), "No .yx test files found!");

    let binary = binary_name();
    let mut failed = Vec::new();

    for file in &files {
        let relative = file
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(file)
            .display()
            .to_string();

        let output = Command::new(&binary)
            .arg("run")
            .arg(file)
            .output()
            .unwrap_or_else(|e| panic!("Failed to run {binary} for {relative}: {e}"));

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let code = output.status.code().unwrap_or(-1);

        let is_error_test = relative.contains("06-compile-errors");

        if is_error_test {
            // Error test files should fail compilation
            if code == 0 {
                failed.push((relative, code, stdout, stderr));
            }
        } else if !stdout.contains("ALL TESTS PASSED") {
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
