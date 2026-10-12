//! 测试 `main.yx` 模板生成
//! 规范来源：docs/src/reference/package/commands.md（init 模板约定）
//!
//! 覆盖:
//! - 生成内容包含项目名
//! - 生成内容包含中文问候
//! - 生成内容包含 `main =` 语法
//! - 生成内容包含 `print(`

use crate::package::template::main_yx::generate_main_yx;

#[test]
fn test_generate_main_yx_contains_project_name() {
    let content = generate_main_yx("my-project");
    assert!(
        content.contains("my-project"),
        "模板应包含项目名: {content}"
    );
}

#[test]
fn test_generate_main_yx_contains_hello() {
    let content = generate_main_yx("test");
    assert!(content.contains("你好"), "模板应包含中文问候: {content}");
}

#[test]
fn test_generate_main_yx_contains_main_fn() {
    let content = generate_main_yx("test");
    // Bin 模式入口必须是函数：`main = () => { ... }`（值块形态会在 E3021 被拒）
    assert!(
        content.contains("main ="),
        "模板应包含函数形态入口: {content}"
    );
}

#[test]
fn test_generate_main_yx_contains_print() {
    let content = generate_main_yx("test");
    assert!(
        content.contains("print("),
        "模板应包含 print 调用: {content}"
    );
}
