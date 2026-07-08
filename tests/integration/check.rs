//! CLI `yaoxiang check` 命令集成测试
//!
//! 测试 `check` 子命令对 .yx 文件的类型检查功能。
//! 覆盖正常文件、错误文件和无文件输入三种场景。

#![cfg(feature = "cli")]

use std::path::PathBuf;
use tempfile::TempDir;
use yaoxiang::util::diagnostic::run_check_command_once;

/// 在临时目录中创建 .yx 源文件
fn create_yx_file(
    dir: &TempDir,
    name: &str,
    content: &str,
) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).expect("Failed to write test file");
    path
}

/// 创建一个临时目录
fn temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

#[allow(clippy::ptr_arg)]
/// 辅助：对单个文件运行 check，返回 Ok(error_count) 或 Err
fn check_file(path: &PathBuf) -> Result<usize, anyhow::Error> {
    run_check_command_once(
        std::slice::from_ref(path),
        &[],
        false, // json
        false, // use_colors
        true,  // no_progress — 抑制进度输出
    )
}

// ============================================================================
// 正常路径：无错误的 .yx 文件
// ============================================================================

#[test]
fn test_check_valid_file() {
    let dir = temp_dir();
    let file = create_yx_file(&dir, "valid.yx", r#"main = { x = 42; print(x) }"#);
    let result = check_file(&file);
    assert!(result.is_ok(), "Valid file should pass check");
    assert_eq!(result.unwrap(), 0, "Valid file should report 0 errors");
}

#[test]
fn test_check_module_import() {
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "import.yx",
        r#"use std.io; main = { io.println("ok") }"#,
    );
    let result = check_file(&file);
    assert!(result.is_ok(), "File with module import should pass check");
    assert_eq!(result.unwrap(), 0);
}

#[test]
fn test_check_complex_program() {
    let dir = temp_dir();
    let file = create_yx_file(
        &dir,
        "complex.yx",
        r#"
use std.list

main = {
    mut xs = [1, 2, 3]
    ys = list.map(xs, (x) => x * 2)
    print(ys)
}
"#,
    );
    let result = check_file(&file);
    assert!(result.is_ok(), "Complex program should pass check");
    assert_eq!(result.unwrap(), 0);
}

// ============================================================================
// 错误路径：有类型错误的 .yx 文件
// ============================================================================
#[test]
fn test_check_syntax_error() {
    let dir = temp_dir();
    let file = create_yx_file(&dir, "syntax_error.yx", r#"main = { x =  }"#);
    let result = check_file(&file);
    assert!(
        result.is_err(),
        "Syntax error should fail at parse stage (return Err)"
    );
    let err_msg = format!("{}", result.unwrap_err());
    assert!(
        err_msg.contains("parse") || err_msg.contains("syntax") || err_msg.contains("expect"),
        "Error message should mention parse/syntax issue, got: {}",
        err_msg
    );
}

#[test]
fn test_check_type_mismatch() {
    let dir = temp_dir();
    let file = create_yx_file(&dir, "type_error.yx", r#"main = { x: Int = "hello" }"#);
    let result = check_file(&file);
    assert!(result.is_ok(), "check should not panic on type errors");
    let error_count = result.unwrap();
    assert!(
        error_count > 0,
        "Type mismatch file should report errors > 0"
    );
}

#[test]
fn test_check_undeclared_variable() {
    let dir = temp_dir();
    let file = create_yx_file(&dir, "undeclared.yx", r#"main = { x = undefined_var }"#);
    let result = check_file(&file);
    assert!(result.is_ok(), "check should not panic on undeclared var");
    let error_count = result.unwrap();
    assert!(
        error_count > 0,
        "Undeclared variable should report errors > 0"
    );
}

// ============================================================================
// 边界情况：空输入 / 路径不存在
// ============================================================================

#[test]
fn test_check_nonexistent_file() {
    let result = check_file(&PathBuf::from("nonexistent_file.yx"));
    assert!(result.is_err(), "Non-existent file should return error");
}

#[test]
fn test_check_file_no_yx_extension() {
    let dir = temp_dir();
    let file = create_yx_file(&dir, "hello.txt", "main = { print(42) }");
    let result = check_file(&file);
    // 非 .yx 文件路径会被 collect_yx_files_from_paths 过滤掉，返回 "No .yx files found"
    assert!(
        result.is_err(),
        "Non-yx file path should produce error (no .yx files found)"
    );
}
