//! E2E Test Runner for YaoXiang (.yx) test files
//!
//! Discovers all `*.yx` test files across the two tiers (RFC-036 §9) and
//! executes each against the `yaoxiang` binary with expectation-driven
//! judgment:
//!
//! - 语言可用性语料：`tests/yaoxiang/`（被测对象是语言本身，目录对齐语言规范章节）
//! - 库测试：`src/std/tests/`（被测对象是 std 的 API 契约，随库走；
//!   CLI 侧经显式路径 `yaoxiang test src/std/tests` 发现，不混入默认扫描）
//!
//! 判定约定与 `yaoxiang test` CLI 共用（RFC-036 §8.2，#319 收口 + 2026-09-06
//! 指令文法定案）：头部指令经 `yaoxiang::util::test_markers::TestFileSpec`
//! 解析——
//! - `// expect: compile-error EXXXX`：单步 `check`，退出码非 0 且全部预期码
//!   出现 = PASS（run 不执行）
//! - `// expect: runtime-error EXXXX`：`check` 必须通过 + `run` 必须失败且
//!   全部预期码出现 = PASS
//! - 无 expect 指令 = 行为测试：`run` 退出码 0 = PASS
//! - 指令解析失败 = panic（构造期拒绝）
//! - `// skip: <原因>` 跳过，`// mode: <模式>` 指定子进程运行时模式
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
//! ├── 03-semantics/         # 语义（return/尾表达式）
//! ├── 04-concurrency/       # 并发模型（对应 concurrency.md）
//! ├── 05-ownership/         # 所有权（独立章节）
//! ├── 06-compile-errors/    # 编译期错误检测
//! └── 99-demos/             # 论文演示（非规范测试）
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;

use yaoxiang::util::test_markers::{Expectation, TestFileSpec};

/// Find all `.yx` test files across both tiers (RFC-036 §9), sorted.
fn discover_yx_tests() -> Vec<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    // 第一层：语言可用性语料
    collect_yx_files(&manifest.join("tests").join("yaoxiang"), &mut files);
    // 第二层：库测试（std，随库走；目录与 Rust 单元测试共存，文件类型不相交）
    collect_yx_files(&manifest.join("src").join("std").join("tests"), &mut files);
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

/// Locate the `yaoxiang-rs` binary.
///
/// Priority:
/// 1. `CARGO_BIN_EXE_yaoxiang-rs` (set by `cargo test`)
/// 2. Build once with `cargo build` and use the output path
fn binary_name() -> String {
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

/// `check` 单文件（编译期判定步）。
fn spawn_check(
    binary: &str,
    file: &Path,
) -> std::process::Output {
    Command::new(binary)
        .arg("check")
        .arg(file)
        .output()
        .unwrap_or_else(|e| panic!("Failed to run {binary} check for {}: {e}", file.display()))
}

/// `run` 单文件（`// mode:` 透传 `--runtime`）。
fn spawn_run(
    binary: &str,
    file: &Path,
    mode: Option<&String>,
) -> std::process::Output {
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

// Tests

#[test]
fn test_all_yx_files_pass() {
    let files = discover_yx_tests();
    assert!(!files.is_empty(), "No .yx test files found!");

    let binary = binary_name();

    for file in &files {
        let relative = file
            .strip_prefix(env!("CARGO_MANIFEST_DIR"))
            .unwrap_or(file)
            .display()
            .to_string();

        // 头部指令经 TestFileSpec 统一解析（与 yaoxiang test CLI 同一实现）
        let spec = TestFileSpec::parse(file);

        // // skip: 文件跳过执行
        if let Some(reason) = &spec.skip_reason {
            eprintln!("  [SKIP] {relative}: {reason}");
            continue;
        }
        // 指令解析失败 = 语料缺陷，构造期拒绝（RFC-036 §8.2）
        if let Some(reason) = &spec.invalid {
            panic!("Invalid header directive: {relative}: {reason}");
        }

        match &spec.expectation {
            Expectation::Behavior => {
                let output = spawn_run(&binary, file, spec.mode.as_ref());
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let code = output.status.code().unwrap_or(-1);
                assert!(
                    code == 0,
                    "Test failed: {relative} (exit: {code})\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
                );
            }
            Expectation::CompileError(_) => {
                let output = spawn_check(&binary, file);
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let code = output.status.code().unwrap_or(-1);
                assert!(
                    code != 0,
                    "Expected compile error, but check passed: {relative}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
                );
                if let Err(note) = spec.check_expected_codes(&stderr) {
                    panic!("预期错误码比对失败: {relative}\n{note}\nSTDERR:\n{stderr}");
                }
            }
            Expectation::RuntimeError(_) => {
                // 编译必须先通过——check 失败说明语料分类错了（编译期就该报）
                let check = spawn_check(&binary, file);
                let check_stderr = String::from_utf8_lossy(&check.stderr).to_string();
                assert!(
                    check.status.code().unwrap_or(-1) == 0,
                    "Expected runtime error, but compilation rejected the file: {relative}\nSTDERR:\n{check_stderr}"
                );
                let output = spawn_run(&binary, file, spec.mode.as_ref());
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let code = output.status.code().unwrap_or(-1);
                assert!(
                    code != 0,
                    "Expected runtime error, but run exited successfully: {relative}\nSTDOUT:\n{stdout}\nSTDERR:\n{stderr}"
                );
                if let Err(note) = spec.check_expected_codes(&stderr) {
                    panic!("预期错误码比对失败: {relative}\n{note}\nSTDERR:\n{stderr}");
                }
            }
        }
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
