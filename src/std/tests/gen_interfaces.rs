//! 标准库接口文件生成器测试
//!
//! 验证 `gen_interfaces` 模块从 `StdModule` trait 自动生成 `.yx` 接口文件的功能。

use crate::std::gen_interfaces::{
    generate_all_interfaces, write_interfaces_to_dir, find_std_interface_file,
};

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

/// 仓库预生成接口目录（RFC-037：发行包 lib/yaoxiang/std 的 native 层来源，
/// 打包时纯复制，不再有运行时生成入口）
const COMMITTED_INTERFACES_DIR: &str = "src/std/interfaces";

#[test]
fn test_committed_interface_files_match_generation() {
    // Arrange: 预生成目录与 StdModule::exports() 是"单一源 → 派生文件"关系
    // （同 RFC-013 码表：漂移即红，治愈走 bless 入口）
    let committed = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(COMMITTED_INTERFACES_DIR);
    let expected = generate_all_interfaces();

    // Act & Assert: 每个模块的预生成文件与当前生成逐字节一致
    for (name, content) in &expected {
        let path = committed.join(format!("{}.yx", name));
        let on_disk = std::fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!(
                "预生成接口文件 {} 缺失: {}；治愈：cargo test update_committed_interface_files -- --ignored",
                path.display(),
                e
            )
        });
        assert_eq!(
            &on_disk, content,
            "预生成接口 {} 与 StdModule::exports() 漂移；治愈：cargo test update_committed_interface_files -- --ignored",
            name
        );
    }

    // 反向：目录中不允许存在生成器不再产出的孤儿文件
    let valid: Vec<String> = expected.iter().map(|(n, _)| format!("{}.yx", n)).collect();
    for entry in std::fs::read_dir(&committed).unwrap() {
        let file_name = entry.unwrap().file_name().to_string_lossy().to_string();
        assert!(
            valid.contains(&file_name),
            "预生成目录存在孤儿文件 {}（生成器已不再产出）；治愈：删除后重跑 bless",
            file_name
        );
    }
}

#[test]
#[ignore = "bless 入口：std 接口变更后显式运行重写预生成文件（cargo test update_committed_interface_files -- --ignored）"]
fn update_committed_interface_files() {
    // Act: 以 StdModule::exports() 为唯一源重写仓库预生成目录
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(COMMITTED_INTERFACES_DIR);
    let count = write_interfaces_to_dir(&dir).unwrap();

    // Assert: 数量可感知（bless 不做内容断言，正确性由同步测试把关）
    assert!(count > 0, "bless 应至少写出一个接口文件");
}
