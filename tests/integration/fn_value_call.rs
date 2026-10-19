//! 局部函数类型值按名调用的间接调用路径 — RFC-039 WBS 3.6.1（2026-10-10 用户裁决 (i)）
//!
//! 规范依据：
//! - 语言规范 `type-system.md` §3.5 函数类型：函数类型是一等类型，函数值是值。
//! - 语言规范 `syntax.md` §2.x 绑定语义（`name = { ... }` 是函数还是块值）：
//!   `f = double`（值是变量）是**值绑定**——它是变量，不是函数定义；只有
//!   「值是 Lambda / 注解是函数类型 / 无注解块」才进函数表。
//! - 语义裁 (i)：函数类型值按**间接调用**运行，判据是「该程序正常运行
//!   （退出码 0、结果正确）」，不是改成响亮诊断。
//!
//! 红态（修复前，revision 8d49e9cc4）：三条用例都在运行期报
//! `E6006 Native function not found: f`——调用被降格为按名 `Call`，
//! 运行期按该名查扁平函数表落空。

use yaoxiang::run;

/// 执行源码；失败时把错误连源码一起打出来（辅助函数的 panic 必须带上下文）。
fn run_source(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\n{:?}\n\nSource:\n{source}", e));
}

#[test]
fn test_fn_value_binding_call_by_name_uses_indirect_call() {
    // Arrange：顶层值绑定 `f = double` 只占全局槽位，函数表里没有 `f`
    let source = r#"
use std.assert

double: (x: Int) -> Int = (x) => x * 2

f = double

assert.assert(f(3) == 6, "call of a top-level function value must run the bound function")
"#;

    // Act
    run_source(source);

    // Assert：值正确性由程序内 assert 表达；E6006 会让 run_source panic
}

#[test]
fn test_fn_value_binding_call_by_name_from_function_body_uses_indirect_call() {
    // Arrange：调用点在函数体内（不在模块初始化序列里），名字仍解析到全局槽位
    let source = r#"
use std.assert

double: (x: Int) -> Int = (x) => x * 2

f = double

main: () -> Void = {
    assert.assert(f(4) == 8, "call from a function body must run the bound function")
}

main()
"#;

    // Act
    run_source(source);

    // Assert：值正确性由程序内 assert 表达；E6006 会让 run_source panic
}

#[test]
fn test_fn_value_binding_forward_reference_call_by_name_uses_indirect_call() {
    // Arrange：调用语句写在绑定之前——槽位由 register_global_slots 预分配，
    // 初始化按拓扑序排在顶层语句之前执行
    let source = r#"
use std.assert

assert.assert(f(5) == 10, "forward-referenced call must run the bound function")

double: (x: Int) -> Int = (x) => x * 2
f = double
"#;

    // Act
    run_source(source);

    // Assert：值正确性由程序内 assert 表达；E6006 会让 run_source panic
}
