//! 标准库接口文件测试（RFC-037 §标准库目录）
//!
//! 覆盖 gen_interfaces.rs 的两块行为：
//! - 接口视图生成（`StdModule::exports()` → `.yx` 签名视图，发行包
//!   `lib/yaoxiang/std/` 的 native 层来源）
//! - 三级查找链（项目 vendor → 发行包 exe 相对 → 全局回退）
//! - 预生成文件同步门禁（生成物入库，漂移即红，同 RFC-013 码表模式）

use tempfile::TempDir;

use crate::std::gen_interfaces::{
    find_std_interface_file, find_std_interface_file_in, generate_all_interfaces,
    write_interfaces_to_dir,
};
use std::path::{Path, PathBuf};

#[test]
fn test_generate_all_interfaces() {
    let interfaces = generate_all_interfaces();
    assert!(!interfaces.is_empty(), "应至少生成一个接口文件");

    // 检查包含关键模块
    let names: Vec<&str> = interfaces.iter().map(|(n, _)| n.as_str()).collect();
    assert!(names.contains(&"io"), "应包含 io 模块");
    assert!(names.contains(&"list"), "应包含 list 模块");
    assert!(names.contains(&"math"), "应包含 math 模块");
    assert!(names.contains(&"dict"), "应包含 dict 模块");
    assert!(names.contains(&"string"), "应包含 string 模块");
}

#[test]
fn test_io_interface_content() {
    let interfaces = generate_all_interfaces();
    let io = interfaces.iter().find(|(n, _)| n == "io").unwrap();
    let content = &io.1;

    assert!(content.contains("print:"), "io 接口应包含 print");
    assert!(content.contains("println:"), "io 接口应包含 println");
    assert!(content.contains("read_line:"), "io 接口应包含 read_line");
    assert!(content.contains("read_file:"), "io 接口应包含 read_file");
    assert!(content.contains("..."), "接口函数体应包含 ...");
}

#[test]
fn test_math_interface_has_constants() {
    let interfaces = generate_all_interfaces();
    let math = interfaces.iter().find(|(n, _)| n == "math").unwrap();
    let content = &math.1;

    assert!(content.contains("PI:"), "math 接口应包含 PI 常量");
    assert!(content.contains("E:"), "math 接口应包含 E 常量");
}

#[test]
fn test_list_interface_content() {
    let interfaces = generate_all_interfaces();
    let list = interfaces.iter().find(|(n, _)| n == "list").unwrap();
    let content = &list.1;

    assert!(content.contains("push:"), "list 接口应包含 push");
    assert!(content.contains("pop:"), "list 接口应包含 pop");
    assert!(content.contains("map:"), "list 接口应包含 map");
    assert!(content.contains("filter:"), "list 接口应包含 filter");
}

#[test]
fn test_write_interfaces_to_temp_dir() {
    let temp_dir = std::env::temp_dir().join("yaoxiang_test_interfaces");
    let _ = std::fs::remove_dir_all(&temp_dir);

    let result = write_interfaces_to_dir(&temp_dir);
    assert!(result.is_ok(), "写入接口文件应成功");

    // 验证文件存在
    assert!(temp_dir.join("io.yx").exists());
    assert!(temp_dir.join("list.yx").exists());
    assert!(temp_dir.join("math.yx").exists());

    // 清理
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_find_std_interface_file() {
    // 不指定项目目录，且全局目录可能不存在 → 返回 None
    let result = find_std_interface_file(None, "nonexistent_module");
    assert!(result.is_none());
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

    // 反向：目录中不允许存在生成器不再产出的孤儿文件
    let valid: Vec<String> = expected.iter().map(|(n, _)| format!("{}.yx", n)).collect();
    for entry in std::fs::read_dir(&committed).unwrap() {
        let file_name = entry.unwrap().file_name().to_string_lossy().to_string();
        assert!(
            valid.contains(&file_name),
            "预生成目录存在孤儿文件 {}（生成器已不再产出）；治愈：删除后重跑 gen-std-interfaces example",
            file_name
        );
    }
}
