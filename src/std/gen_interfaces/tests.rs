//! 标准库接口文件三级查找链测试（RFC-037 §标准库目录）
//!
//! 覆盖 find_std_interface_file 的优先级语义：
//! 1. 项目 `.yaoxiang/vendor/std/` 覆盖最高
//! 2. 发行包 exe 相对 `../lib/yaoxiang/std/` 次之（便携/托管/deb 同构）
//! 3. 全局目录（`YAOXIANG_HOME/std`，缺省 `~/.yaoxiang/std`）兜底
//!
//! 全部走注入式纯函数 `find_std_interface_file_in`，不触碰进程环境变量。

use std::path::{Path, PathBuf};

use tempfile::TempDir;

use crate::std::gen_interfaces::find_std_interface_file_in;

/// 在 `dir` 下写一个接口文件，返回其路径（模拟查找链中的某一环）
fn put_interface(
    dir: &Path,
    name: &str,
) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let file = dir.join(format!("{}.yx", name));
    std::fs::write(&file, "// stub").unwrap();
    file
}

#[test]
fn test_find_project_override_beats_bundled_and_global() {
    // Arrange: 三级链各放一个同名接口文件
    let proj = TempDir::new().unwrap();
    let pkg = TempDir::new().unwrap();
    let global = TempDir::new().unwrap();
    let project_file = put_interface(&proj.path().join(".yaoxiang/vendor/std"), "io");
    put_interface(&pkg.path().join("lib/yaoxiang/std"), "io");
    put_interface(global.path(), "io");

    // Act
    let found = find_std_interface_file_in(
        Some(proj.path()),
        Some(&pkg.path().join("bin")),
        Some(global.path()),
        "io",
    );

    // Assert: 项目覆盖必须优先于发行包与全局
    assert_eq!(
        found.as_deref(),
        Some(project_file.as_path()),
        "project vendor/std override must win over bundled and global"
    );
}

#[test]
fn test_find_exe_relative_bundled_std_resolves_pkg_layout() {
    // Arrange: 只有发行包目录有该文件（bin/ 引擎 + lib/yaoxiang/std/）
    let pkg = TempDir::new().unwrap();
    let global = TempDir::new().unwrap();
    let bundled = put_interface(&pkg.path().join("lib/yaoxiang/std"), "math");

    // Act: exe 位于 <pkg>/bin/，接口应解析到 <pkg>/lib/yaoxiang/std/
    let found = find_std_interface_file_in(
        None,
        Some(&pkg.path().join("bin")),
        Some(global.path()),
        "math",
    );

    // Assert
    assert_eq!(
        found.as_deref(),
        Some(bundled.as_path()),
        "exe-relative ../lib/yaoxiang/std must be found from bin/"
    );
}

#[test]
fn test_find_global_dir_is_last_fallback() {
    // Arrange: 只有全局目录有该文件
    let global = TempDir::new().unwrap();
    let global_file = put_interface(global.path(), "time");

    // Act
    let found = find_std_interface_file_in(None, None, Some(global.path()), "time");

    // Assert
    assert_eq!(
        found.as_deref(),
        Some(global_file.as_path()),
        "global dir must be used when project and bundled are absent"
    );
}

#[test]
fn test_find_unknown_module_returns_none() {
    // Arrange: 空目录
    let empty = TempDir::new().unwrap();

    // Act
    let found = find_std_interface_file_in(
        Some(empty.path()),
        Some(&empty.path().join("bin")),
        Some(empty.path()),
        "no_such_module",
    );

    // Assert
    assert!(
        found.is_none(),
        "missing module must return None, got {:?}",
        found
    );
}
