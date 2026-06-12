//! 测试 `main.yx` 模板生成
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
    assert!(content.contains("my-project"));
}

#[test]
fn test_generate_main_yx_contains_hello() {
    let content = generate_main_yx("test");
    assert!(content.contains("你好"));
}

#[test]
fn test_generate_main_yx_contains_main_fn() {
    let content = generate_main_yx("test");
    // YaoXiang 使用 `main = {...}` 语法而非 `fn main() {}`
    assert!(content.contains("main ="));
}

#[test]
fn test_generate_main_yx_contains_print() {
    let content = generate_main_yx("test");
    assert!(content.contains("print("));
}
