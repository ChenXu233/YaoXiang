//! Execution integration tests
//!
//! Tests that various .yx programs execute successfully end-to-end.
//! Uses yaoxiang::run() to compile and execute source code.
//!
//! Note: Full output-capturing E2E tests are in tests/yx_runner.rs.
//!
//! #468（WBS 3.21.1）：一元取负 i64::MIN 的运行时溢出诊断——与二元溢出
//! （#281，checked 整数运算报 E6007）同码同构造，覆盖 language-spec 的
//! 运行时错误类（整数运算溢出属运行期诊断，不 panic）。

use yaoxiang::run;

fn run_ok(source: &str) {
    let source = crate::fixture::with_main_invoked(source);
    run(&source).unwrap_or_else(|e| panic!("Execution failed:\n{:?}\n\nSource:\n{source}", e));
}

/// 断言源码执行失败并返回完整错误串（含栈帧）。
///
/// 与 [`run_ok`] 对偶：运行期失败路径（如整数溢出）的值级 oracle——
/// 溢出必须以 `Err`（Runtime error + 栈帧）返回，而不是 panic。
fn run_err(source: &str) -> String {
    let source = crate::fixture::with_main_invoked(source);
    run(&source)
        .map_err(|e| format!("{e:#}"))
        .expect_err(&format!(
            "Execution should fail with an error, not succeed:\n\nSource:\n{source}"
        ))
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

// 运行时整数溢出诊断（#468 / WBS 3.21.1）
//
// #281 定下二元整数运算走 checked 路径、溢出报 E6007；#468 补齐一元取负：
// i64::MIN 取负在 i64 域不可表示，必须以运行时错误（Runtime error + 栈帧）
// 返回，不得 panic。两形态：字面量（常量装载按 i64 截断为 i64::MIN 后取负）
// 与变量（运行期算出 i64::MIN 后取负）。

/// #468：字面量形态——`-9223372036854775808` 中常量 2^63 装载时截断为
/// i64::MIN，取负溢出必须报运行时错误并携带栈帧，而不是 panic。
#[test]
fn test_unary_neg_i64_min_literal_reports_integer_overflow() {
    // Arrange
    let source = r#"
        main = () => {
            x = -9223372036854775808
            print(x)
        }
        "#;

    // Act
    let err = run_err(source);

    // Assert
    assert!(
        err.contains("Runtime error: integer overflow"),
        "unary neg of i64::MIN literal should report runtime integer overflow, got: {err}"
    );
    assert!(
        err.contains("at main"),
        "integer overflow error should carry a stack trace (frame of `main`), got: {err}"
    );
}

/// #468：变量形态——运行期算出 i64::MIN 后对变量取负，同样必须报
/// 运行时错误（与二元溢出同码同构造），而不是 panic。
#[test]
fn test_unary_neg_i64_min_variable_reports_integer_overflow() {
    // Arrange
    let source = r#"
        main = () => {
            m = -9223372036854775807 - 1
            y = -m
            print(y)
        }
        "#;

    // Act
    let err = run_err(source);

    // Assert
    assert!(
        err.contains("Runtime error: integer overflow"),
        "unary neg of i64::MIN variable should report runtime integer overflow, got: {err}"
    );
    assert!(
        err.contains("at main"),
        "integer overflow error should carry a stack trace (frame of `main`), got: {err}"
    );
}

/// #468 对照：i64::MAX 取负（结果 -i64::MAX 可表示）不受修复影响，仍成功。
#[test]
fn test_unary_neg_i64_max_still_succeeds() {
    // Arrange & Act & Assert: 正常路径回归——取负不溢出的边界值仍能执行
    run_ok(
        r#"
        main = () => {
            m = 9223372036854775807
            y = -m
            print(y)
        }
        "#,
    );
}

/// #468 对照：普通值取负不受修复影响，仍成功。
#[test]
fn test_unary_neg_normal_value_still_succeeds() {
    // Arrange & Act & Assert: 正常路径回归——常规取负仍能执行
    run_ok(
        r#"
        main = () => {
            y = -42
            print(y)
        }
        "#,
    );
}

/// #468 对照：二元溢出（#281 既有行为）不受修复影响，仍报运行时整数溢出。
#[test]
fn test_binary_int_overflow_still_reports_integer_overflow() {
    // Arrange
    let source = r#"
        main = () => {
            b = 9223372036854775807 + 1
            print(b)
        }
        "#;

    // Act
    let err = run_err(source);

    // Assert
    assert!(
        err.contains("Runtime error: integer overflow in `9223372036854775807 + 1`"),
        "binary overflow should keep reporting integer overflow with operands, got: {err}"
    );
}

/// #512 端到端：约束泛型以 **Ref 形态实参**实例化，mono 派发名归底类型。
///
/// `sum(&a, &b)`（a/b: Vec3）把 T 解成 `Ref{inner: Vec3}`；方法/背书登记名
/// 一律按底类型（`Vec3.add` 的 `self: &Vec3` 是参数形态不是登记名），mono
/// 改写必须剥 Ref 才能在合并函数表命中 `Vec3.add`——否则 run 报 E6006
/// `&Vec3.add` not found（RFC-011b 约束泛型运算符派发，issue #512）。
/// 值传形态（T=Vec3 / T=Int）是既有绿态的回归对照。
#[test]
fn test_constrained_generic_ref_args_dispatch_to_base_type_method() {
    run_ok(
        r#"
use std.assert

Vec3: Type = {
    x: Float,
    y: Float,
    z: Float,
    Add(Vec3, Vec3, Vec3),
}

Vec3.add: (self: &Vec3, other: &Vec3) -> Vec3 = (self, other) => {
    return Vec3(self.x + other.x, self.y + other.y, self.z + other.z)
}

sum: (T: Add) -> (a: T, b: T) -> T = a + b

main = () => {
    a = Vec3(1.0, 2.0, 3.0)
    b = Vec3(4.0, 5.0, 6.0)
    by_ref = sum(&a, &b)
    assert(by_ref.x == 5.0, "ref-shaped instantiation dispatches to Vec3.add")
    by_value = sum(a, b)
    assert(by_value.x == 5.0, "value-shaped instantiation still dispatches to Vec3.add")
    assert(sum(1, 2) == 3, "Int instantiation still dispatches to native add")
}
        "#,
    );
}

/// #512 端到端：`&mut` 形态实参走同一条剥 Ref 链。
///
/// `type_name` 对可变借用渲染 `&mut Vec3`，剥链不分可变性（函数表键空间
/// 同样不存在 `&mut Type.add`），两侧同 `&mut` 的实例化派发归底类型
/// （RFC-011b，issue #512）。
#[test]
fn test_constrained_generic_mut_ref_args_dispatch_to_base_type_method() {
    run_ok(
        r#"
use std.assert

Vec3: Type = {
    x: Float,
    y: Float,
    z: Float,
    Add(Vec3, Vec3, Vec3),
}

Vec3.add: (self: &Vec3, other: &Vec3) -> Vec3 = (self, other) => {
    return Vec3(self.x + other.x, self.y + other.y, self.z + other.z)
}

sum: (T: Add) -> (a: T, b: T) -> T = a + b

main = () => {
    mut a = Vec3(1.0, 2.0, 3.0)
    mut b = Vec3(4.0, 5.0, 6.0)
    v = sum(&mut a, &mut b)
    assert(v.x == 5.0, "mut-ref-shaped instantiation dispatches to Vec3.add")
}
        "#,
    );
}

// ============ 3.20.35 / #441：调用形谓词体的精化入口（运行时面） ============
//
// 谓词体是用户函数调用（`{ check_nn(x) }` 形 / lambda 形 `(x) => check_nn(x)`）
// 时，满足值经第 4 级证明调用编译期执行为真 → 程序编译并通过运行；
// 精化变量的运行时类型是 base（Int），可原样参与运算。编译期拒绝面（违反值
// E4018 / 缺失被调）由 check.rs 的子进程锚钉住
// （test_check_call_form_predicate_body_*：语料夹具因基线红线改挂集成测试）。

/// #441 端到端：TypeDefinition 形调用形谓词体（`= { check_nn(x) }`）的
/// 满足值编译通过且运行正确——精化变量按 base 类型参与 Int 运算。
/// 修前假拒绝：E1003（实参落类型命名空间）+ E1002。
#[test]
fn test_call_form_predicate_body_satisfied_value_runs_and_uses_base_type() {
    run_ok(
        r#"
use std.assert

check_nn: (v: Int) -> Bool = { v >= 0 }
NonNegative: (x: Int) -> Type = { check_nn(x) }

main = () => {
    mut y: Int = 5
    z: NonNegative(y) = y
    w: NonNegative(7) = 7
    assert.assert(z + w == 12, "call-form predicate refinement erases to Int base")
}
        "#,
    );
}

/// #441 端到端：lambda 声明形调用形谓词体（`= (x) => check_nn(x)`）与
/// TypeDefinition 形同判——满足值编译通过且运行正确。
/// 修前该形态经证明函数面 IR 路径执行，链接不了用户被调
/// （「Native function not found: check_nn」→ E4018 执行失败）。
#[test]
fn test_call_form_predicate_lambda_body_satisfied_value_runs() {
    run_ok(
        r#"
use std.assert

check_nn: (v: Int) -> Bool = { v >= 0 }
NonNegativeA: (x: Int) -> Type = (x) => check_nn(x)

main = () => {
    mut y: Int = 5
    z: NonNegativeA(y) = y
    assert.assert(z == 5, "lambda-form call-body predicate refinement passes")
}
        "#,
    );
}
