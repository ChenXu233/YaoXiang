//! Unit 测试 — 基于 02-stage-contract §编译器改动清单（driver/unit.rs）
//!
//! Unit 是 PerModule 阶段的执行粒度：单文件 1 个 / 多文件 N 个 / 嵌入模块虚拟路径。

use crate::driver::unit::Unit;

#[test]
fn test_unit_new_stores_fields() {
    // Arrange / Act
    let unit = Unit::new("std.test", "<std/test>", "assert_eq: ...");

    // Assert
    assert_eq!(unit.key, "std.test");
    assert_eq!(unit.path.to_string_lossy(), "<std/test>");
    assert_eq!(unit.source, "assert_eq: ...");
}
