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
//!
//! 规范依据（WBS 3.6.5 / #479，选择式导入 + 顶层重绑）：
//! - 语言规范 `modules.md` §2.2：`use path.{x}` 把 `x` 绑定到当前作用域，
//!   绑定的是**同一个顶层绑定**（函数仍是函数、值仍是值），不是调用结果。
//! - 语言规范 `type-system.md` §3.5：函数类型是一等类型，函数值是值。
//!   故 `f = greet` 之后 `f` 装的是函数值本身；`f()` 间接调用运行 greet 的体。
//!
//! 红态（修复前，revision eb4124cf0 主树）：`use helper.{greet}` + `f = greet`
//! 在顶层把导入函数当 native 常量**零参调用**——`f` 装的是 greet() 的返回值，
//! `f()` 得到 void、退出码 0（静默错值）。

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

// === WBS 3.6.5 / #479：选择式导入（`use lib.{f}`）后顶层重绑的函数值调用 ===
//
// 跨文件形态必须在真实项目上取：入口的 use 图决定模块发现，帮助函数只做机械
// 落盘与入口调用，不掺任何语义改写（判据同 tests/integration/multifile.rs）。

/// 在临时目录写一个带清单的多文件项目，运行入口文件；失败返回错误文本。
fn run_multifile_project(
    files: &[(&str, &str)],
    entry: &str,
) -> Result<(), String> {
    let dir = tempfile::tempdir().expect("create tempdir for multifile fixture");
    std::fs::write(
        dir.path().join("yaoxiang.toml"),
        "[package]\nname = \"t\"\nversion = \"0.1.0\"\n",
    )
    .expect("write manifest");
    for (name, content) in files {
        std::fs::write(dir.path().join(name), content)
            .unwrap_or_else(|e| panic!("write source file {name}: {e}"));
    }
    yaoxiang::run_project(&dir.path().join(entry)).map_err(|e| format!("{e:?}"))
}

#[test]
fn test_fn_value_selective_import_rebound_call_runs_imported_function() {
    // Arrange：选择式导入 greet（函数绑定，不是值绑定），顶层 f = greet 重绑
    let helper = "greet: () -> String = () => { \"yo\" }\n";
    let main = r#"
use std.assert
use helper.{greet}

f = greet

main = () => {
    assert.assert(f() == "yo", "f = greet 之后 f() 必须运行 helper.greet 的体并返回 yo（#479）")
}
"#;

    // Act
    let result = run_multifile_project(&[("helper.yx", helper), ("main.yx", main)], "main.yx");

    // Assert：修复前 f 装的是 greet() 的返回值，f() 得 void——此处运行期 Eq 失配
    assert!(
        result.is_ok(),
        "选择式导入的函数重绑后按名调用必须运行导入的函数体（#479）：{result:?}"
    );
}

#[test]
fn test_fn_value_selective_import_direct_call_runs_imported_function() {
    // Arrange：对照①——选择式导入后**直接调用**导入名（不经重绑）
    let helper = "greet: () -> String = () => { \"yo\" }\n";
    let main = r#"
use std.assert
use helper.{greet}

main = () => {
    assert.assert(greet() == "yo", "选择式导入的函数直接调用必须运行 helper.greet（#479 对照）")
}
"#;

    // Act
    let result = run_multifile_project(&[("helper.yx", helper), ("main.yx", main)], "main.yx");

    // Assert：直接调用一直是静态调用，本批不得改变其形态与结果
    assert!(
        result.is_ok(),
        "选择式导入的函数直接调用必须保持原行为（#479 对照）：{result:?}"
    );
}

#[test]
fn test_fn_value_whole_import_rebound_call_runs_imported_function() {
    // Arrange：对照②——整体导入（use helper）后限定名取值再重绑
    let helper = "greet: () -> String = () => { \"yo\" }\n";
    let main = r#"
use std.assert
use helper

f = helper.greet

main = () => {
    assert.assert(f() == "yo", "整体导入后 f = helper.greet 的 f() 必须运行 helper.greet（#479 对照）")
}
"#;

    // Act
    let result = run_multifile_project(&[("helper.yx", helper), ("main.yx", main)], "main.yx");

    // Assert：整体导入的命名空间取值路径已由 #396 修好，本批不得回退
    assert!(
        result.is_ok(),
        "整体导入的函数值重绑必须保持原行为（#479 对照）：{result:?}"
    );
}
