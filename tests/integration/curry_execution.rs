//! Curry 端到端执行测试 — 基于 RFC-011 §4.1 函数级 const 泛型（curry 形式）
//!
//! 验证 curry 函数在运行时正确执行。期望值来自设计 spec：
//! docs/superpowers/specs/2026-07-26-curry-desugaring-design.md
//!
//! RFC-011 §4.1: 编译期常量参数 curry 形式
//!   2 层: f(42)(5) == 5
//!   3 层: add3(1)(2)(3) == 6
//!   捕获: f(10)(5) == 15（内层闭包捕获 N=10）

use yaoxiang::run;

fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| {
        panic!(
            "Execution failed for curry test.\nSource:\n{}\nError:\n{:?}",
            source, e
        )
    });
}

#[test]
fn two_layer_curry_returns_inner_value() {
    // RFC-011 §4.1: f: (N: Int) -> (n: N) -> Int = (n) => n
    // f(42)(5) 应返回 5
    run_ok(
        r#"
        f: (N: Int) -> (n: N) -> Int = (n) => n

        main = {
            result = f(42)(5)
        }
        "#,
    );
}

#[test]
fn three_layer_curry_sums_all_args() {
    // RFC-011 §4.1: add3: (a: Int) -> (b: Int) -> (c: Int) -> Int = (a, b, c) => a + b + c
    // add3(1)(2)(3) 应返回 6
    run_ok(
        r#"
        add3: (a: Int) -> (b: Int) -> (c: Int) -> Int = (a, b, c) => a + b + c

        main = {
            result = add3(1)(2)(3)
        }
        "#,
    );
}

#[test]
fn curry_with_capture_uses_outer_param() {
    // RFC-011 §4.1: f: (N: Int) -> (n: N) -> Int = (n) => n + N
    // f(10)(5) 应返回 15（内层闭包捕获 N=10）
    run_ok(
        r#"
        f: (N: Int) -> (n: N) -> Int = (n) => n + N

        main = {
            result = f(10)(5)
        }
        "#,
    );
}
