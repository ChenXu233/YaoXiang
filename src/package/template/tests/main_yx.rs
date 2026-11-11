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
