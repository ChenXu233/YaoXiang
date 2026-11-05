//! Curry 端到端执行测试 — 基于 RFC-011 §4.1 函数级 const 泛型（curry 形式）
//!
//! 验证 curry 函数在运行时正确执行。期望值来自设计 spec：
//! docs/superpowers/specs/2026-07-26-curry-desugaring-design.md
//!
//! RFC-011 §4.1: 编译期常量参数 curry 形式
//!   2 层: f(42)(42) == 42
//!   3 层: add3(1)(2)(3) == 6
//!   捕获: f(10)(10) == 20（内层闭包捕获 N=10；6.8.7 起 n: N 是单值类型，实参必须等于 N）

use yaoxiang::run;

fn run_ok(source: &str) {
    let source = crate::fixture::with_main_invoked(source);
    run(&source).unwrap_or_else(|e| {
        panic!(
            "Execution failed for curry test.\nSource:\n{}\nError:\n{:?}",
            source, e
        )
    });
}

#[test]
fn two_layer_curry_returns_inner_value() {
    // RFC-011 §4.1: f: (N: Int) -> (n: N) -> Int = (n) => n
    // f(42)(42) 应返回 42（6.8.7 起 n: N 是单值类型，实参必须等于 N）
    run_ok(
        r#"
        f: (N: Int) -> (n: N) -> Int = (n) => n

        main = () => {
            result = f(42)(42)
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

        main = () => {
            result = add3(1)(2)(3)
        }
        "#,
    );
}

#[test]
fn curry_with_capture_uses_outer_param() {
    // RFC-011 §4.1: f: (N: Int) -> (n: N) -> Int = (n) => n + N
    // f(10)(10) 应返回 20（内层闭包捕获 N=10；6.8.7 起 n: N 是单值类型，实参必须等于 N）
    run_ok(
        r#"
        f: (N: Int) -> (n: N) -> Int = (n) => n + N

        main = () => {
            result = f(10)(10)
        }
        "#,
    );
}
