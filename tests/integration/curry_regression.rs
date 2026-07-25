//! 非 curry 函数回归测试
//!
//! 验证 curry 修复没有破坏现有函数语义（issue #227）。
//! 钉死"非 curry 不受影响"：多参非 curry、单参非 curry、Lambda 语法。

#![allow(unused_imports)]
use yaoxiang::run;

#[test]
fn non_curry_multi_param_function_unchanged() {
    run(r#"
        add: (a: Int, b: Int) -> Int = (a, b) => a + b
        main = { r = add(3, 4) }
        "#)
    .expect("non-curry multi-param function should still work");
}

#[test]
fn non_curry_single_param_function_unchanged() {
    run(r#"
        inc: (x: Int) -> Int = (x) => x + 1
        main = { r = inc(5) }
        "#)
    .expect("non-curry single-param function should still work");
}

#[test]
fn existing_lambda_call_still_works() {
    run(r#"
        f: (x: Int) -> Int = (x) => x * 2
        main = { r = f(21) }
        "#)
    .expect("existing lambda call should still work");
}
