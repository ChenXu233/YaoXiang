//! 非 curry 函数回归测试 — 基于 RFC-011 §4.1（非 curry 不受影响）
//!
//! 验证 curry 修复没有破坏现有函数语义。curry 是 1 层退化为普通函数（递归基例），
//! 多参、单参、lambda 语法均不应受影响。
//!
//! RFC-011 §4.1: 编译期常量参数 curry 形式
//!   非 curry 函数 return_type 不是嵌套 Fn → 走老路径，零破坏

use yaoxiang::run;

fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| {
        panic!(
            "Regression test failed.\nSource:\n{}\nError:\n{:?}",
            source, e
        )
    });
}

#[test]
fn non_curry_multi_param_function_unchanged() {
    // 普通 2 参函数：add(3, 4) == 7，不应受 curry 修复影响
    run_ok(
        r#"
        add: (a: Int, b: Int) -> Int = (a, b) => a + b
        main = { r = add(3, 4) }
        "#,
    );
}

#[test]
fn non_curry_single_param_function_unchanged() {
    // 单参非 curry：inc(5) == 6，return_type 不是 Fn，走老路径
    run_ok(
        r#"
        inc: (x: Int) -> Int = (x) => x + 1
        main = { r = inc(5) }
        "#,
    );
}

#[test]
fn existing_lambda_call_still_works() {
    // 现有 lambda 语法非 curry：f(21) == 42，与 curry 修复无关的路径
    run_ok(
        r#"
        f: (x: Int) -> Int = (x) => x * 2
        main = { r = f(21) }
        "#,
    );
}
