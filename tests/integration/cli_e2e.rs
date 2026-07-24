//! CLI 子命令端到端测试（子进程级）
//!
//! 通过 `CARGO_BIN_EXE_yaoxiang` 调用编译出来的二进制，
//! 验证用户真实路径：命令行参数解析、退出码、stdout/stderr。
//!
//! 规范来源：
//! - RFC-014: 包管理系统 (init/new/add/rm/install/list/update)
//! - docs/src/design/language-spec.md: 执行与编译章节
//! - docs/src/dev/test-specification.md §集成测试规范 规则 9.1: E2E 三条路径
//!
//! 覆盖命令：run / build / check / init
//! 函数级 API 契约见 `cli.rs`，本文件只验证"二进制入口"行为。

#![cfg(feature = "cli")]

use std::path::PathBuf;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// 编译产物路径（cargo 在测试构建时注入）
fn yx_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_yaoxiang"))
}

/// 在临时目录写一个 .yx 源文件
fn write_yx(
    dir: &std::path::Path,
    name: &str,
    content: &str,
) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap_or_else(|e| panic!("write {name}: {e}"));
    path
}

/// 跑一个子进程，捕获 stdout/stderr，返回 (退出码, stdout, stderr)
fn run_yx(
    args: &[&str],
    cwd: &std::path::Path,
) -> (i32, String, String) {
    let output = Command::new(yx_bin())
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap_or_else(|e| panic!("failed to spawn yaoxiang: {e}"));
    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

// ============================================================================
// run 命令 — 退出码契约：成功 0，编译/运行错误 1
// 规范来源：language-spec.md 执行章节
// ============================================================================

#[test]
fn test_e2e_run_valid_program_exits_zero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "ok.yx", "main = { print(42) }");

    // Act
    let (code, stdout, _stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "valid program should exit 0");
    assert!(
        stdout.contains("42"),
        "stdout should contain program output, got: {stdout:?}"
    );
}

#[test]
fn test_e2e_run_compile_error_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "bad.yx", "x: Int = ");

    // Act
    let (code, _stdout, stderr) = run_yx(&["run", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_ne!(code, 0, "compile error should exit non-zero");
    assert!(
        !stderr.is_empty(),
        "stderr should have error diagnostics on compile error"
    );
}

#[test]
fn test_e2e_run_nonexistent_file_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();

    // Act
    let (code, _stdout, _stderr) = run_yx(&["run", "/nonexistent/path.yx"], tmp.path());

    // Assert
    assert_ne!(code, 0, "missing file should exit non-zero");
}

// ============================================================================
// build 命令 — 退出码契约 + 输出文件存在性
// 规范来源：language-spec.md 编译章节
// ============================================================================

#[test]
fn test_e2e_build_valid_source_produces_bytecode_file() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "prog.yx", "main = { print(1) }");
    let out = tmp.path().join("prog.42");

    // Act
    let (code, _stdout, _stderr) = run_yx(
        &["build", src.to_str().unwrap(), "-o", out.to_str().unwrap()],
        tmp.path(),
    );

    // Assert
    assert_eq!(code, 0, "build on valid source should exit 0");
    assert!(out.exists(), "bytecode output file should exist");
    assert!(
        out.metadata().unwrap().len() > 0,
        "bytecode file should not be empty"
    );
}

#[test]
fn test_e2e_build_compile_error_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "bad.yx", "x: Int = ");
    let out = tmp.path().join("bad.42");

    // Act
    let (code, _stdout, _stderr) = run_yx(
        &["build", src.to_str().unwrap(), "-o", out.to_str().unwrap()],
        tmp.path(),
    );

    // Assert
    assert_ne!(code, 0, "build on compile error should exit non-zero");
    assert!(
        !out.exists(),
        "bytecode file should not be produced on compile error"
    );
}

// ============================================================================
// check 命令 — 退出码契约：无错 0，有错 1，无 .yx 文件 2
// 规范来源：language-spec.md 类型检查章节
// ============================================================================

#[test]
fn test_e2e_check_valid_file_exits_zero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "ok.yx", "main = { x = 1 }");

    // Act
    let (code, _stdout, _stderr) = run_yx(&["check", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 0, "check on valid file should exit 0");
}

#[test]
fn test_e2e_check_type_error_exits_one() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let src = write_yx(tmp.path(), "bad.yx", "x: Int = \"not an int\"");

    // Act
    let (code, _stdout, stderr) = run_yx(&["check", src.to_str().unwrap()], tmp.path());

    // Assert
    assert_eq!(code, 1, "check on type error should exit 1");
    assert!(
        !stderr.is_empty(),
        "stderr should contain diagnostics on type error"
    );
}

#[test]
fn test_e2e_check_nonexistent_file_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();

    // Act
    let (code, _stdout, _stderr) = run_yx(&["check", "/nonexistent/path.yx"], tmp.path());

    // Assert
    assert_ne!(code, 0, "check on missing file should exit non-zero");
}

// ============================================================================
// init 命令 — 目录结构契约 + 退出码
// 规范来源：RFC-014 包管理系统 — 项目初始化
// ============================================================================

#[test]
fn test_e2e_init_binary_project_creates_expected_files() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my_app");

    // Act
    let (code, _stdout, _stderr) = run_yx(&["init", "my_app"], tmp.path());

    // Assert
    assert_eq!(code, 0, "init should exit 0 on success");
    assert!(
        project_dir.join("src/main.yx").exists(),
        "should have src/main.yx"
    );
    assert!(
        project_dir.join("yaoxiang.toml").exists(),
        "should have yaoxiang.toml"
    );
    assert!(
        project_dir.join("yaoxiang.lock").exists(),
        "should have yaoxiang.lock"
    );
    assert!(
        project_dir.join(".gitignore").exists(),
        "should have .gitignore"
    );
    assert!(
        !project_dir.join("src/lib.yx").exists(),
        "binary project should not have src/lib.yx"
    );
}

#[test]
fn test_e2e_init_library_project_creates_lib_yx() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my_lib");

    // Act
    let (code, _stdout, _stderr) = run_yx(&["init", "my_lib", "--lib"], tmp.path());

    // Assert
    assert_eq!(code, 0, "init --lib should exit 0");
    assert!(
        project_dir.join("src/lib.yx").exists(),
        "library should have src/lib.yx"
    );
    assert!(
        !project_dir.join("src/main.yx").exists(),
        "library project should not have src/main.yx"
    );
}

#[test]
fn test_e2e_init_on_existing_directory_exits_nonzero() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let existing = tmp.path().join("dup");
    std::fs::create_dir_all(&existing).unwrap();
    std::fs::write(existing.join("placeholder.txt"), "").unwrap();

    // Act
    let (code, _stdout, stderr) = run_yx(&["init", "dup"], tmp.path());

    // Assert
    assert_ne!(code, 0, "init on existing dir should exit non-zero");
    assert!(!stderr.is_empty(), "stderr should explain why init failed");
}
