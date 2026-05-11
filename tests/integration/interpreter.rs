//! Interpreter integration tests
//!
//! Tests the full compilation pipeline for various YaoXiang programs.
//! Verifies that source code compiles and executes without errors.

use yaoxiang::run;

/// Helper: assert that source code compiles and executes successfully.
fn assert_run_ok(source: &str) {
    let result = run(source);
    match result {
        Ok(_) => {}
        Err(e) => {
            let msg = format!("{:?}", e);
            panic!("Execution failed:\n{msg}\n\nSource:\n{source}");
        }
    }
}

// ============================================================================
// 冒烟测试
// ============================================================================

#[test]
fn test_empty_module() {
    assert_run_ok("main = {}");
}

#[test]
fn test_hello_world() {
    assert_run_ok(r#"main = { print("hello") }"#);
}

// ============================================================================
// 变量声明
// ============================================================================

#[test]
fn test_variable_declaration() {
    assert_run_ok("main = { x = 42 }");
}

#[test]
fn test_typed_variable() {
    assert_run_ok("main = { x: Int = 42 }");
}

#[test]
fn test_mut_variable() {
    assert_run_ok("main = { mut x = 0; x = x + 1 }");
}

#[test]
fn test_multiple_variables() {
    assert_run_ok("main = { x = 1; y = 2; z = x + y }");
}

// ============================================================================
// 字面量和运算符
// ============================================================================

#[test]
fn test_literals() {
    assert_run_ok("main = { x = 42; y = 3.14; s = \"hi\"; b = true }");
}

#[test]
fn test_arithmetic() {
    assert_run_ok("main = { x = (10 + 20) * 3 - 5 / 2 }");
}

#[test]
fn test_comparison() {
    assert_run_ok("main = { x = 5 > 3; y = 10 <= 20; z = 42 == 42 }");
}

// ============================================================================
// Lambda 和函数调用
// ============================================================================

#[test]
fn test_lambda() {
    assert_run_ok("main = { double = (x) => x * 2; r = double(21) }");
}

#[test]
fn test_lambda_multi_args() {
    assert_run_ok("main = { add = (a, b) => a + b; r = add(3, 4) }");
}

#[test]
fn test_function_definition() {
    assert_run_ok(
        r#"
        add: (a: Int, b: Int) -> Int = (a, b) => { return a + b }
        main = { r = add(3, 4) }
        "#,
    );
}

// ============================================================================
// 控制流
// ============================================================================

#[test]
fn test_if_else() {
    assert_run_ok("main = { if true { x = 1 } }");
}

#[test]
fn test_if_elif_else() {
    assert_run_ok("main = { if false { x = 1 } elif true { x = 2 } else { x = 3 } }");
}

#[test]
fn test_while_loop() {
    assert_run_ok("main = { mut i = 3; while i > 0 { i = i - 1 } }");
}

#[test]
fn test_for_loop() {
    assert_run_ok("main = { mut items = [1, 2, 3]; for item in items { print(item) } }");
}

#[test]
fn test_match_expr() {
    assert_run_ok("main = { r = match 1 { 1 => 10, _ => 0 } }");
}

#[test]
fn test_match_string() {
    assert_run_ok(r#"main = { r = match "a" { "a" => 1, _ => 0 } }"#);
}

// ============================================================================
// 数据结构
// ============================================================================

#[test]
fn test_list() {
    assert_run_ok("main = { mut xs = [1, 2, 3]; first = xs[0] }");
}

#[test]
fn test_list_comp() {
    assert_run_ok("main = { mut xs = [1, 2, 3]; sq = [x * x for x in xs] }");
}

#[test]
fn test_tuple() {
    assert_run_ok("main = { mut t = (1, \"one\") }");
}

#[test]
fn test_dict() {
    assert_run_ok(r#"main = { mut d = {"k": 42} }"#);
}

// ============================================================================
// 高阶函数
// ============================================================================

#[test]
fn test_closure_map() {
    assert_run_ok(
        r#"
        use std.{io, list}
        main = { mut xs = [1, 2, 3]; ys = list.map(xs, x => x * 2) }
        "#,
    );
}

#[test]
fn test_closure_filter() {
    assert_run_ok(
        r#"
        use std.{io, list}
        main = { mut xs = [1, 2, 3, 4, 5]; ys = list.filter(xs, x => x > 2) }
        "#,
    );
}

#[test]
fn test_closure_reduce() {
    assert_run_ok(
        r#"
        use std.{io, list}
        main = { mut xs = [1, 2, 3]; s = list.reduce(xs, (a, x) => a + x, 0) }
        "#,
    );
}

// ============================================================================
// 模块导入
// ============================================================================

#[test]
fn test_use_module() {
    assert_run_ok("use std.io; main = { io.println(\"ok\") }");
}

#[test]
fn test_use_multi() {
    assert_run_ok("use std.{io, list}; main = { mut xs = [1]; io.println(xs) }");
}

// ============================================================================
// F-字符串
// ============================================================================

#[test]
fn test_fstring() {
    assert_run_ok(r#"main = { x = 42; s = f"value: {x}" }"#);
}
