//! 项目初始化命令测试 — 基于 RFC-014 包管理系统设计 & 设计文档
//!
//! 设计文档: docs/superpowers/specs/2026-07-02-yaoxiang-new-init-design.md
//! RFC-014: 包管理系统（manifest 格式对齐）
//!
//! 测试覆盖:
//! - `exec_in`: 在指定目录创建项目子目录（binary / --lib）
//! - `exec_here`: 在当前目录初始化项目
//! - 错误处理: 目录已存在、项目已存在、文件跳过

use std::fs;

use crate::package::commands::init::{exec_in, exec_here, InitOptions};
use crate::package::error::PackageError;
use crate::package::manifest::PackageManifest;
use tempfile::TempDir;

fn default_opts() -> InitOptions {
    InitOptions { lib: false }
}

fn lib_opts() -> InitOptions {
    InitOptions { lib: true }
}

// ===================================================================
// exec_in 基础测试
// ===================================================================

#[test]
fn test_init_creates_project_directory_and_files() {
    // Arrange & Act
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &default_opts(), "test-project").unwrap();

    // Assert
    let project_path = tmp.path().join("test-project");
    assert!(
        project_path.join("yaoxiang.toml").exists(),
        "yaoxiang.toml should be created"
    );
    assert!(
        project_path.join("yaoxiang.lock").exists(),
        "yaoxiang.lock should be created"
    );
    assert!(
        project_path.join("src/main.yx").exists(),
        "src/main.yx should be created for binary project"
    );
    assert!(
        project_path.join(".gitignore").exists(),
        ".gitignore should be created"
    );
    assert!(
        project_path.join("tests").is_dir(),
        "tests/ directory should be created"
    );
    assert!(
        project_path.join(".yaoxiang/std/io.yx").exists(),
        ".yaoxiang/std/ should contain interface files"
    );
}

#[test]
fn test_init_manifest_has_correct_metadata() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &default_opts(), "my-app").unwrap();

    // Act
    let manifest = PackageManifest::load(&tmp.path().join("my-app")).unwrap();

    // Assert
    assert_eq!(
        manifest.package.name, "my-app",
        "package name should match project name"
    );
    assert_eq!(
        manifest.package.version, "0.1.0",
        "default version should be 0.1.0"
    );
}

#[test]
fn test_init_main_yx_contains_entry_point() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &default_opts(), "my-app").unwrap();

    // Act
    let content = fs::read_to_string(tmp.path().join("my-app/src/main.yx")).unwrap();

    // Assert
    assert!(
        content.contains("my-app"),
        "main.yx should contain project name 'my-app', got: {content}"
    );
    assert!(
        content.contains("main ="),
        "main.yx should define entry point with 'main =', got: {content}"
    );
}

#[test]
fn test_init_existing_directory_returns_project_exists_error() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &default_opts(), "existing").unwrap();

    // Act
    let result = exec_in(tmp.path(), &default_opts(), "existing");

    // Assert
    assert!(
        result.is_err(),
        "should return error when directory already exists"
    );
    let err = result.unwrap_err();
    assert!(
        matches!(err, PackageError::ProjectExists(_)),
        "expected ProjectExists error, got: {err:?}"
    );
}

// ===================================================================
// --lib 库项目测试
// ===================================================================

#[test]
fn test_init_lib_creates_lib_yx_not_main_yx() {
    // Arrange & Act
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &lib_opts(), "my-lib").unwrap();

    let project_path = tmp.path().join("my-lib");

    // Assert
    assert!(
        project_path.join("src/lib.yx").exists(),
        "src/lib.yx should be created for library project"
    );
    assert!(
        !project_path.join("src/main.yx").exists(),
        "src/main.yx should NOT exist for library project"
    );
}

#[test]
fn test_init_lib_yx_contains_no_main_entry_point() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    exec_in(tmp.path(), &lib_opts(), "my-lib").unwrap();

    // Act
    let content = fs::read_to_string(tmp.path().join("my-lib/src/lib.yx")).unwrap();

    // Assert
    assert!(
        content.contains("my-lib"),
        "lib.yx should contain project name 'my-lib', got: {content}"
    );
    assert!(
        content.contains("库项目"),
        "lib.yx should indicate library project type, got: {content}"
    );
    assert!(
        !content.contains("main ="),
        "lib.yx should NOT contain main entry point, got: {content}"
    );
}

// ===================================================================
// exec_here 当前目录初始化测试
// ===================================================================

#[test]
fn test_init_here_creates_project_in_current_directory() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my-here");
    fs::create_dir(&project_dir).unwrap();

    // Act
    let _guard = std::env::set_current_dir(&project_dir);
    exec_here(&default_opts()).unwrap();

    // Assert
    assert!(
        project_dir.join("yaoxiang.toml").exists(),
        "yaoxiang.toml should be created in current directory"
    );
    assert!(
        project_dir.join("src/main.yx").exists(),
        "src/main.yx should be created in current directory"
    );
    assert!(
        project_dir.join("tests").is_dir(),
        "tests/ directory should be created in current directory"
    );
}

#[test]
fn test_init_here_fails_when_project_already_exists() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my-here");
    fs::create_dir(&project_dir).unwrap();

    let _guard = std::env::set_current_dir(&project_dir);
    exec_here(&default_opts()).unwrap();

    // Act
    let result = exec_here(&default_opts());

    // Assert
    assert!(result.is_err(), "second init in same directory should fail");
    let err = result.unwrap_err();
    assert!(
        matches!(err, PackageError::ProjectExists(_)),
        "expected ProjectExists error for already-initialized project, got: {err:?}"
    );
}

#[test]
fn test_init_here_preserves_preexisting_files() {
    // Arrange
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my-here");
    fs::create_dir(&project_dir).unwrap();
    fs::create_dir_all(project_dir.join("src")).unwrap();

    let main_path = project_dir.join("src").join("main.yx");
    let preexisting_content = "// existing content";
    fs::write(&main_path, preexisting_content).unwrap();

    // Act
    let _guard = std::env::set_current_dir(&project_dir);
    exec_here(&default_opts()).unwrap();

    // Assert
    let content = fs::read_to_string(&main_path).unwrap();
    assert_eq!(
        content, preexisting_content,
        "preexisting src/main.yx should NOT be overwritten by init here"
    );
    assert!(
        project_dir.join("yaoxiang.toml").exists(),
        "yaoxiang.toml should still be created even when some files are skipped"
    );
}

// ===================================================================
// new 和 init <name> 等价性测试
// ===================================================================

#[test]
fn test_new_and_init_name_produce_identical_output() {
    // Arrange
    let tmp_new = TempDir::new().unwrap();
    let tmp_init = TempDir::new().unwrap();
    let opts = default_opts();

    // Act
    exec_in(tmp_new.path(), &opts, "eq-test").unwrap();
    exec_in(tmp_init.path(), &opts, "eq-test").unwrap();

    // Assert
    let new_toml_path = tmp_new.path().join("eq-test/yaoxiang.toml");
    let init_toml_path = tmp_init.path().join("eq-test/yaoxiang.toml");
    let new_toml = fs::read_to_string(&new_toml_path).expect("should read new yaoxiang.toml");
    let init_toml = fs::read_to_string(&init_toml_path).expect("should read init yaoxiang.toml");
    assert_eq!(
        new_toml, init_toml,
        "yaoxiang.toml from 'new' and 'init <name>' should be identical"
    );

    let new_main_path = tmp_new.path().join("eq-test/src/main.yx");
    let init_main_path = tmp_init.path().join("eq-test/src/main.yx");
    let new_main = fs::read_to_string(&new_main_path).expect("should read new main.yx");
    let init_main = fs::read_to_string(&init_main_path).expect("should read init main.yx");
    assert_eq!(
        new_main, init_main,
        "src/main.yx from 'new' and 'init <name>' should be identical"
    );
}
