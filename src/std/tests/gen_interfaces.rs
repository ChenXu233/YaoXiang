//! 标准库接口文件生成器测试
//!
//! 验证 `gen_interfaces` 模块从 `StdModule` trait 自动生成 `.yx` 接口文件的功能。

use crate::std::gen_interfaces::{generate_all_interfaces, write_interfaces_to_dir, find_std_interface_file};

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
