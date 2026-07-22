//! CLI 子命令集成测试
//!
//! 覆盖 Issue #118 列出的 CLI 子命令端到端行为验证。
//!
//! 规范来源：
//! - RFC-014: 包管理系统 (init/new/add/rm/install/list/update)
//! - RFC-010: 统一类型语法 — name: type = value 模型
//! - docs/src/design/language-spec.md: 执行与编译
//!
//! 注意：check 命令已有 tests/integration/check.rs 覆盖，此处不重复。

#![cfg(feature = "cli")]

use std::path::{Path, PathBuf};
use tempfile::TempDir;
use yaoxiang::package::commands::init::{exec_in, InitOptions};
use yaoxiang::package::commands::{add, rm, install, list};
use yaoxiang::package::manifest::PackageManifest;
use yaoxiang::package::error::PackageError;
use yaoxiang::formatter::{format_source, FormatOptions, run_format_command};
use yaoxiang::{run, run_file, build_bytecode, build_bytecode_with_options, eval_code};

// ============================================================================
// 辅助函数
// ============================================================================

fn temp_dir() -> TempDir {
    TempDir::new().expect("Failed to create temp dir")
}

fn write_yx_file(
    dir: &Path,
    name: &str,
    content: &str,
) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).unwrap_or_else(|e| panic!("Failed to write {}: {}", name, e));
    path
}

fn init_project(
    tmp: &TempDir,
    name: &str,
    lib: bool,
) -> PathBuf {
    let dir = tmp.path().join(name);
    exec_in(tmp.path(), &InitOptions { lib }, name)
        .unwrap_or_else(|e| panic!("Failed to init project {}: {:?}", name, e));
    dir
}

// ============================================================================
// init 命令 — RFC-014 包管理：项目初始化
// ============================================================================

#[test]
fn test_init_binary_project_creates_main_yx() {
    // Arrange
    let tmp = temp_dir();
    // Act
    let dir = init_project(&tmp, "my_app", false);
    // Assert
    assert!(
        dir.join("src/main.yx").exists(),
        "binary project should have src/main.yx"
    );
    assert!(
        dir.join("yaoxiang.toml").exists(),
        "should have yaoxiang.toml"
    );
    assert!(
        dir.join("yaoxiang.lock").exists(),
        "should have yaoxiang.lock"
    );
    assert!(dir.join(".gitignore").exists(), "should have .gitignore");
    assert!(
        !dir.join("src/lib.yx").exists(),
        "binary project should not have src/lib.yx"
    );
}

#[test]
fn test_init_library_project_creates_lib_yx() {
    // Arrange
    let tmp = temp_dir();
    // Act
    let dir = init_project(&tmp, "my_lib", true);
    // Assert
    assert!(
        dir.join("src/lib.yx").exists(),
        "library project should have src/lib.yx"
    );
    assert!(
        !dir.join("src/main.yx").exists(),
        "library project should not have src/main.yx"
    );
}

#[test]
fn test_init_on_existing_directory_returns_project_exists_error() {
    // Arrange
    let tmp = temp_dir();
    let dir = tmp.path().join("dup_app");
    std::fs::create_dir_all(&dir).unwrap();
    write_yx_file(&dir, "placeholder.txt", "");
    // Act
    let result = exec_in(tmp.path(), &InitOptions { lib: false }, "dup_app");
    // Assert
    let err = result.expect_err("init on existing dir should fail");
    assert!(
        matches!(&err, PackageError::ProjectExists(p) if p == &dir),
        "Expected ProjectExists({:?}), got: {:?}",
        dir,
        err
    );
}

// ============================================================================
// run 命令 — 执行 .yx 源文件
// ============================================================================

#[test]
fn test_run_program_with_variable_declaration_and_print() {
    // Act
    let result = run(r#"
        main = {
            x = 1 + 2
            print(x)
        }
        "#);
    // Assert
    assert!(
        result.is_ok(),
        "executing a valid program should succeed, got: {:?}",
        result
    );
}

#[test]
fn test_run_syntax_error_returns_error() {
    // Act
    let result = run("main = { print('unclosed }");
    // Assert
    assert!(result.is_err(), "syntax error should return compile error");
}

#[test]
fn test_run_nonexistent_file_returns_error() {
    // Act
    let result = yaoxiang::run_file(Path::new("/nonexistent/path.yx"));
    // Assert
    assert!(result.is_err(), "run_file on nonexistent path should error");
}

// ============================================================================

#[test]
fn test_run_file_compile_error_returns_error() {
    // Arrange
    let dir = temp_dir();
    let path = write_yx_file(dir.path(), "compile_error.yx", "x: Int = "); // incomplete expression = compile error

    // Act
    let result = run_file(&path);

    // Assert
    assert!(
        result.is_err(),
        "run_file on compile error should return error"
    );
}
// build 命令 — 字节码编译
// ============================================================================

#[test]
fn test_build_bytecode_produces_nonempty_output_file() {
    // Arrange
    let tmp = temp_dir();
    let src = write_yx_file(tmp.path(), "test.yx", "main = { print(42) }");
    let output = tmp.path().join("test.42");
    // Act
    build_bytecode(&src, &output).unwrap_or_else(|e| panic!("Failed to build bytecode: {:?}", e));
    // Assert
    assert!(output.exists(), "bytecode output file should exist");
    assert!(
        output.metadata().unwrap().len() > 0,
        "bytecode output should not be empty"
    );
}

#[test]
fn test_build_bytecode_with_debug_info_succeeds() {
    // Arrange
    let tmp = temp_dir();
    let src = write_yx_file(tmp.path(), "debug.yx", "main = { print(1) }");
    let output = tmp.path().join("debug.42");
    // Act
    let result = build_bytecode_with_options(&src, &output, true);
    // Assert
    assert!(
        result.is_ok(),
        "build with debug info should succeed, got: {:?}",
        result
    );
    assert!(output.exists(), "bytecode file should exist");
}

#[test]
fn test_build_nonexistent_source_returns_error() {
    // Arrange
    let tmp = temp_dir();
    let src = tmp.path().join("no_such_file.yx");
    let output = tmp.path().join("out.42");
    // Act
    let result = build_bytecode(&src, &output);
    // Assert
    assert!(result.is_err(), "build on nonexistent source should fail");
}

// ============================================================================
// eval 命令 — 代码求值
// ============================================================================

#[test]
fn test_eval_code_single_expression() {
    // Act
    let result = eval_code("print(99)");
    // Assert
    assert!(
        result.is_ok(),
        "eval should handle a single print expression, got: {:?}",
        result
    );
}

#[test]
fn test_eval_code_with_explicit_main_block() {
    // Act
    let result = eval_code("main = { print(\"with explicit main\") }");
    // Assert
    assert!(
        result.is_ok(),
        "eval should handle code that defines main, got: {:?}",
        result
    );
}

// ============================================================================
// format 命令 — RFC-010 代码格式化
// ============================================================================

#[test]
fn test_format_source_adds_trailing_newline() {
    // Arrange
    let options = FormatOptions::default();
    // Act
    let result =
        format_source("x = 1", &options).expect("format_source should succeed on valid code");
    // Assert: 规范要求 output 以换行结尾
    assert!(
        result.ends_with('\n'),
        "formatted output should end with newline"
    );
}

#[test]
fn test_format_check_mode_reports_already_formatted_as_pass() {
    // Arrange
    let tmp = temp_dir();
    let path = write_yx_file(tmp.path(), "test.yx", "x = 1\n");
    // Act
    let result = run_format_command(&path, &FormatOptions::default(), true, false)
        .expect("format check should succeed on valid file");
    // Assert
    assert!(
        !result.needs_formatting,
        "already-formatted file should pass check"
    );
}

#[test]
fn test_format_write_overwrites_file_with_formatted_content() {
    // Arrange
    let tmp = temp_dir();
    let path = write_yx_file(tmp.path(), "write_test.yx", "x   = 1\n");
    // Act
    run_format_command(&path, &FormatOptions::default(), false, true)
        .expect("format write should succeed");
    // Assert
    let content = std::fs::read_to_string(&path).expect("should read file after format write");
    assert_eq!(
        content, "x = 1\n",
        "file content should be normalized to 'x = 1\\n'"
    );
}

#[test]
fn test_format_invalid_path_returns_error() {
    // Act
    let result = run_format_command(
        Path::new("/nonexistent/path.yx"),
        &FormatOptions::default(),
        false,
        false,
    );
    // Assert
    assert!(result.is_err(), "format on nonexistent path should fail");
}

// ============================================================================
// add / rm 命令 — RFC-014 包管理：依赖操作
// ============================================================================

#[test]
fn test_add_and_remove_dependency_persists_manifest_changes() {
    // Arrange
    let tmp = temp_dir();
    let dir = init_project(&tmp, "pkg_test", false);
    // Act: add
    add::exec_in(&dir, "std", Some("1.0.0"), false).expect("adding a dependency should succeed");
    // Assert: add
    let manifest = PackageManifest::load(&dir).expect("manifest should be readable");
    assert!(
        manifest.has_dependency("std"),
        "std should be listed in dependencies"
    );
    // Act: remove
    rm::exec_in(&dir, "std", false).expect("removing an existing dependency should succeed");
    // Assert: remove
    let manifest = PackageManifest::load(&dir).expect("manifest should be readable after remove");
    assert!(
        !manifest.has_dependency("std"),
        "std should be removed from dependencies"
    );
}

#[test]
fn test_add_dev_dependency_is_listed_in_dev_section() {
    // Arrange
    let tmp = temp_dir();
    let dir = init_project(&tmp, "dev_test", false);
    // Act
    add::exec_in(&dir, "test_utils", Some("0.1.0"), true)
        .expect("adding a dev-dependency should succeed");
    // Assert
    let manifest = PackageManifest::load(&dir).expect("manifest should be readable");
    assert!(
        manifest
            .dev_dependencies
            .iter()
            .any(|(k, _)| k == "test_utils"),
        "test_utils should be in dev-dependencies"
    );
}

#[test]
fn test_add_duplicate_dependency_returns_already_exists_error() {
    // Arrange
    let tmp = temp_dir();
    let dir = init_project(&tmp, "dup_dep_test", false);
    add::exec_in(&dir, "std", None, false).expect("first addition should succeed");
    // Act
    let result = add::exec_in(&dir, "std", None, false);
    // Assert
    let err = result.expect_err("adding a duplicate dependency should fail");
    assert!(
        matches!(&err, PackageError::DependencyAlreadyExists(name) if name == "std"),
        "Expected DependencyAlreadyExists(\"std\"), got: {:?}",
        err
    );
}

#[test]
fn test_remove_nonexistent_dependency_returns_not_found_error() {
    // Arrange
    let tmp = temp_dir();
    let dir = init_project(&tmp, "rm_fail_test", false);
    // Act
    let result = rm::exec_in(&dir, "no_such_pkg", false);
    // Assert
    let err = result.expect_err("removing nonexistent dependency should fail");
    assert!(
        matches!(&err, PackageError::DependencyNotFound(name) if name == "no_such_pkg"),
        "Expected DependencyNotFound(\"no_such_pkg\"), got: {:?}",
        err
    );
}

// ============================================================================
// install / list 命令 — RFC-014 包管理：依赖生命周期
// ============================================================================

#[test]
fn test_install_on_project_with_no_dependencies_is_noop() {
    // Arrange
    let tmp = temp_dir();
    let dir = init_project(&tmp, "empty_install", false);
    // Act
    let result = install::exec_in(&dir);
    // Assert
    assert!(
        result.is_ok(),
        "install on empty project should succeed, got: {:?}",
        result
    );
}

#[test]
fn test_list_on_empty_project_succeeds() {
    // Arrange
    let tmp = temp_dir();
    let dir = init_project(&tmp, "empty_list", false);
    // Act
    let result = list::exec_in(&dir);
    // Assert
    assert!(
        result.is_ok(),
        "list on empty project should succeed, got: {:?}",
        result
    );
}

#[test]
fn test_list_after_adding_dependency_succeeds() {
    // Arrange
    let tmp = temp_dir();
    let dir = init_project(&tmp, "list_deps", false);
    add::exec_in(&dir, "foo", None, false).expect("adding dependency should succeed");
    // Act
    let result = list::exec_in(&dir);
    // Assert
    assert!(
        result.is_ok(),
        "list after adding a dep should succeed, got: {:?}",
        result
    );
}
