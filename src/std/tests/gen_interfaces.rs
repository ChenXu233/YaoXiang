//! 标准库接口文件测试（RFC-037 §标准库目录）
//!
//! 覆盖 gen_interfaces.rs 的两块行为：
//! - 接口视图生成（`StdModule::exports()` → `.yx` 签名视图，发行包
//!   `lib/yaoxiang/std/` 的 native 层来源）
//! - 三级查找链（项目 vendor → 发行包 exe 相对 → 全局回退）
//! - 预生成文件同步门禁（生成物入库，漂移即红，同 RFC-013 码表模式）

use tempfile::TempDir;

use crate::std::gen_interfaces::{
    find_std_interface_file_in, generate_all_interfaces, write_interfaces_to_dir,
};
use std::path::{Path, PathBuf};

#[test]
fn test_write_interfaces_to_temp_dir() {
    // Arrange
    let temp = TempDir::new().unwrap();

    // Act
    let count = write_interfaces_to_dir(temp.path()).unwrap();

    // Assert: 每个 std 模块都有对应接口文件落盘
    assert!(
        count >= 3,
        "至少应写出 3 个接口文件（io/math/dict 等），实际 {count}"
    );
    // std.list 已迁到纯 yx，其接口由源码承载，不再落盘为生成物。
    for name in ["io", "math", "dict"] {
        let file = temp.path().join(format!("{}.yx", name));
        assert!(file.exists(), "接口文件 {} 应存在", file.display());
    }
}

/// 在 `dir` 下写一个接口文件，返回其路径（模拟查找链中的某一环）
fn put_interface(
    dir: &Path,
    name: &str,
) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let file = dir.join(format!("{}.yx", name));
    std::fs::write(&file, "// stub")
        .unwrap_or_else(|e| panic!("写接口桩文件 {} 失败: {e}", file.display()));
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

/// 仓库预生成接口目录（发行包 lib/yaoxiang/std 的 native 层来源，
/// 打包时纯复制，无运行时生成入口）
const COMMITTED_INTERFACES_DIR: &str = "src/std/interfaces";

#[test]
fn test_committed_interface_files_match_generation() {
    // Arrange: 预生成目录与 StdModule::exports() 是"单一源 → 派生文件"关系
    // （同 RFC-013 码表：漂移即红，治愈走 example 工具）
    let committed = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(COMMITTED_INTERFACES_DIR);
    let expected = generate_all_interfaces();

    // Act & Assert: 每个模块的预生成文件与当前生成逐字节一致
    for (name, content) in &expected {
        let path = committed.join(format!("{}.yx", name));
        let on_disk = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "预生成接口文件 {} 缺失: {}；治愈：cargo run --example gen-std-interfaces",
                path.display(),
                e
            )
        });
        assert_eq!(
            &on_disk, content,
            "预生成接口 {} 与 StdModule::exports() 漂移；治愈：cargo run --example gen-std-interfaces",
            name
        );
    }
}
