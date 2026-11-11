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
/// 6.9.2-C2c：闭项 const 实参被擦除后，体内读 N 仍取到**该实例**的值。
///
/// 两个实例（42 / 7）各自把 N 物化成常量，`k` 仍按运行期实参传入：
/// `f(42)(42) == 84`、`f(7)(7) == 14`。擦除若把值算错（41 / 121 一类）或让两个实例
/// 串味，这里的 assert 会直接抛。
#[test]
fn test_closed_const_arg_erasure_keeps_instance_value() {
    run_ok(
        r#"
        use std.assert

        f: (N: Int) -> (k: N) -> Int = {
            x: Int = N
            return x + k
        }

        main = () => {
            a: Int = f(42)(42)
            assert.assert(a == 84, "f(42)(42) should be 84")
            b: Int = f(7)(7)
            assert.assert(b == 14, "f(7)(7) should be 14")
        }
        "#,
    );
}
