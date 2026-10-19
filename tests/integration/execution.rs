//! Execution integration tests
//!
//! Tests that various .yx programs execute successfully end-to-end.
//! Uses yaoxiang::run() to compile and execute source code.
//!
//! Note: Full output-capturing E2E tests are in tests/yx_runner.rs.

use yaoxiang::run;

fn run_ok(source: &str) {
    let source = crate::fixture::with_main_invoked(source);
    run(&source).unwrap_or_else(|e| panic!("Execution failed:\n{:?}\n\nSource:\n{source}", e));
}

// 完整程序测试

/// #452 端到端：方法调用的**命名实参加入重载决议**（WBS 3.14.1）。
///
/// 值级 oracle 用程序内 assert 表达（跑通即算对）：
///   ① t.pick(1, 2)        = 102   纯位置（回归）
///   ② t.pick(1, a = 2)    = 102   混合（位置占前导槽位、命名按名入槽）
///   ③ t.pick(a = 2, n = 1) = 102  纯命名（书写序无关）
/// 改动前 ②③ 会**丢实参**（运行期 E6007 或算错），① 是回归对照。
#[test]
fn test_named_args_in_method_overload_decide_candidate() {
    run_ok(
        r#"
use std.assert

Temp: Type = { v: Int }

Temp.pick: (self: &Temp, n: Int, a: Int) -> Int = (self, n, a) => { return n * 100 + a }

main = () => {
    t = Temp(1)
    assert.assert(t.pick(1, 2) == 102, "positional")
    assert.assert(t.pick(1, a = 2) == 102, "mixed positional + named")
    assert.assert(t.pick(a = 2, n = 1) == 102, "named only, reversed order")
}
        "#,
    );
}

#[test]
fn test_simple_program() {
    run_ok(
        r#"
        main = () => {
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
        main = () => {
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
        main = () => {
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
        main = () => {
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
        main = () => {
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
        main = () => {
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

// 函数调用语义测试

#[test]
fn test_curried_fn_assign_and_call() {
    // Arrange & Act: 定义 curried 函数，赋值给局部变量后调用
    // 验证局部变量持有函数值能正确走 CallDyn 动态调用
    run_ok(
        r#"
        add: (x: Int) -> (y: Int) -> Int = (x) => (y) => x + y

        main = () => {
            f = add(1)
            y = f(2)
            print(y)
        }
        "#,
    );
}
