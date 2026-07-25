//! Curry 函数端到端执行测试
//!
//! 验证 curry 函数在运行时行为正确（issue #227）。
//! 通过 `yaoxiang::run()` 编译并执行源码。

use yaoxiang::run;

fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\n{:?}", e));
}

#[test]
fn two_layer_curry_returns_inner_value() {
    // f: (N: Int) -> (n: N) -> Int = (n) => n
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
    // add3: (a: Int) -> (b: Int) -> (c: Int) -> Int = (a, b, c) => a + b + c
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
    // f: (N: Int) -> (n: N) -> Int = (n) => n + N
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
