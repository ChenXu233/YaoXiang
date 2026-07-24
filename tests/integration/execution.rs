//! Execution integration tests
//!
//! Tests that various .yx programs execute successfully end-to-end.
//! Uses yaoxiang::run() to compile and execute source code.
//!
//! Note: Full output-capturing E2E tests are in tests/yx_runner.rs.

use yaoxiang::run;

fn run_ok(source: &str) {
    run(source).unwrap_or_else(|e| panic!("Execution failed:\n{:?}", e));
}

// ============================================================================
// 完整程序测试
// ============================================================================

#[test]
fn test_simple_program() {
    run_ok(
        r#"
        main = {
            x = 42
            y = x * 2
            print(x)
            print(y)
        }
        "#,
    );
}

#[test]
fn test_fibonacci_iterative() {
    run_ok(
        r#"
        main = {
            mut a = 0
            mut b = 1
            mut i = 0
            while i < 10 {
                mut next = a + b
                a = b
                b = next
                i = i + 1
            }
            print(a)
        }
        "#,
    );
}

#[test]
fn test_factorial_iterative() {
    run_ok(
        r#"
        main = {
            mut result = 1
            mut i = 1
            while i <= 5 {
                result = result * i
                i = i + 1
            }
            print(result)
        }
        "#,
    );
}

#[test]
fn test_counter_loop() {
    run_ok(
        r#"
        main = {
            mut sum = 0
            mut i = 1
            while i <= 10 {
                sum = sum + i
                i = i + 1
            }
            print(sum)
        }
        "#,
    );
}

#[test]
fn test_match_simple() {
    run_ok(
        r#"
        main = {
            r1 = match 1 { 1 => 100, _ => 0 }
        }
        "#,
    );
}

#[test]
fn test_list_operations() {
    run_ok(
        r#"
        use std.{io, list}
        main = {
            xs = [1, 2, 3, 4, 5]
            ys = list.map(xs, x => x * 10)
            xs2 = [1, 2, 3, 4, 5]
            zs = list.filter(xs2, x => x > 2)
            xs3 = [1, 2, 3, 4, 5]
            s = list.reduce(xs3, (a, x) => a + x, 0)
            io.println(ys)
            io.println(zs)
            io.println(s)
        }
        "#,
    );
}

// ============================================================================
// 函数调用语义测试
// ============================================================================

#[test]
fn test_curried_fn_assign_and_call() {
    // Arrange & Act: 定义 curried 函数，赋值给局部变量后调用
    // 验证局部变量持有函数值能正确走 CallDyn 动态调用
    run_ok(
        r#"
        add: (x: Int) -> (y: Int) -> Int = (x) => (y) => x + y

        main = {
            f = add(1)
            y = f(2)
            print(y)
        }
        "#,
    );
}
