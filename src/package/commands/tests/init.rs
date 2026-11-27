//! 项目初始化命令测试 — 基于 RFC-014 包管理系统设计 & 设计文档
//!
//! 设计文档: docs/superpowers/specs/2026-07-02-yaoxiang-new-init-design.md
//! RFC-014: 包管理系统（manifest 格式对齐）
//!
//! 测试覆盖:
//! - `exec_in`: 在指定目录创建项目子目录（binary / --lib）
//! - `exec_here_in`: 在**指定目录**就地初始化项目（`exec_here` 的参数化入口）
//! - 错误处理: 目录已存在、项目已存在、文件跳过
//!
//! 为什么用 `exec_here_in` 而不是 `exec_here`：`exec_here` 走 `std::env::current_dir()`，
//! 而切换**进程级**工作目录是全进程共享的——并行测试下
//! 会把同进程其它线程的 `current_dir()` 指到临时目录（曾实际把 init 跑在仓库根、留下
//! 项目文件并把显式路径的集成判据顶成 E3023）。测试因此一律给目录参数，不改进程状态；
//! CLI 的 `exec_here` 路径由二进制级 e2e 覆盖（子进程有自己的 cwd）。

use std::fs;

use crate::package::commands::init::{exec_here_in, exec_in, InitOptions};
use crate::package::error::PackageError;
use crate::package::manifest::PackageManifest;
use tempfile::TempDir;

fn default_opts() -> InitOptions {
    InitOptions { lib: false }
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

/// 第二次就地初始化必须报 `ProjectExists`，且错误里带的就是**该项目根**。
#[test]
fn test_init_here_fails_when_project_already_exists() {
    // Arrange：临时目录下的项目根（第一次初始化由 `exec_here_in` 直接落在这里）
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my-here");
    fs::create_dir(&project_dir).unwrap();
    exec_here_in(&project_dir, &default_opts()).unwrap();

    // Act
    let result = exec_here_in(&project_dir, &default_opts());

    // Assert
    assert!(result.is_err(), "second init in same directory should fail");
    let err = result.unwrap_err();
    assert!(
        matches!(err, PackageError::ProjectExists(_)),
        "expected ProjectExists error for already-initialized project, got: {err:?}"
    );
    let PackageError::ProjectExists(reported) = err else {
        unreachable!("上面已断言是 ProjectExists")
    };
    assert_eq!(
        reported, project_dir,
        "ProjectExists 必须报出被初始化过的那个项目根（不是别的目录）"
    );
}

/// 预存文件不被覆盖，且其余产物照常生成——断言全部直接落在 `project_dir` 上。
#[test]
fn test_init_here_preserves_preexisting_files() {
    // Arrange：项目根里先放一份 src/main.yx
    let tmp = TempDir::new().unwrap();
    let project_dir = tmp.path().join("my-here");
    fs::create_dir(&project_dir).unwrap();
    fs::create_dir_all(project_dir.join("src")).unwrap();

    let main_path = project_dir.join("src").join("main.yx");
    let preexisting_content = "// existing content";
    fs::write(&main_path, preexisting_content).unwrap();

    // Act
    exec_here_in(&project_dir, &default_opts()).unwrap();

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
    let manifest = PackageManifest::load(&project_dir).unwrap();
    assert_eq!(
        manifest.package.name, "my-here",
        "项目名取所在目录名（这里就是 project_dir 的末段）"
    );
}
