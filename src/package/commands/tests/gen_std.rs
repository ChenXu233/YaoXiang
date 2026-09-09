//! 测试 `yaoxiang gen-std` 命令（RFC-037 §标准库目录）
//!
//! 覆盖:
//! - 缺省输出到 <project>/.yaoxiang/vendor/std（查找链第一级）
//! - --out-dir 覆盖输出目录（发行包打包路径）
//! - 生成文件数与 write_interfaces_to_dir 返回值一致

use std::path::PathBuf;

use tempfile::TempDir;

use crate::package::commands::gen_std::exec_in;
use crate::package::commands::init;
use crate::std::gen_interfaces::generate_all_interfaces;

fn setup_project() -> (TempDir, PathBuf) {
    let tmp = TempDir::new().unwrap();
    init::exec_in(tmp.path(), &init::InitOptions { lib: false }, "test-proj").unwrap();
    let project_dir = tmp.path().join("test-proj");
    (tmp, project_dir)
}

#[test]
fn test_gen_std_writes_default_vendor_std_dir() {
    let (_tmp, project_dir) = setup_project();

    // Act: 不传 out_dir，应写入查找链第一级 .yaoxiang/vendor/std
    exec_in(&project_dir, None).unwrap();

    // Assert: 每个 std 模块都有对应接口文件
    let out_dir = project_dir.join(".yaoxiang/vendor/std");
    for (name, _) in generate_all_interfaces() {
        let file = out_dir.join(format!("{}.yx", name));
        assert!(
            file.exists(),
            "interface file for module '{}' must exist at {}",
            name,
            file.display()
        );
    }
}

#[test]
fn test_gen_std_out_dir_override_creates_files() {
    let (_tmp, project_dir) = setup_project();
    let out_dir = project_dir.join("dist/lib/yaoxiang/std");

    // Act: 显式 out_dir（发行包打包场景）
    exec_in(&project_dir, Some(out_dir.clone())).unwrap();

    // Assert
    assert!(
        out_dir.join("io.yx").exists(),
        "io.yx must be generated under explicit --out-dir {}",
        out_dir.display()
    );
}

#[test]
fn test_gen_std_returns_count_matching_generated_files() {
    let (_tmp, project_dir) = setup_project();
    let out_dir = project_dir.join("std-out");
    let expected = generate_all_interfaces().len();

    // Act
    let count = crate::std::gen_interfaces::write_interfaces_to_dir(&out_dir).unwrap();

    // Assert
    assert_eq!(
        count, expected,
        "write_interfaces_to_dir must report one file per std module"
    );
}
