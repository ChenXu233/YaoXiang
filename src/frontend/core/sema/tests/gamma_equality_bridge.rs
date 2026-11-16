//! Γ 常量等式桥（RFC-039 D62 / 6.9.4，issue #534）
//!
//! 覆盖: 单收集点 `check_if_stmt` 把常量等式事实注入流敏感 Γ；`frame_term_env` 把
//! 这些等式并入 `TermEnv` 的**值**侧，成为 q=0 槽的第四证据源（前三源：字面量 /
//! 值环境 / 单值表达式）。等式形如 `BinOp(Eq, 一侧闭常量, 一侧唯一 NamedVar)`。
//!
//! 同时钉住 **barrier 三性质**（体内自产可见 / 体外事实不入体 / 体内事实随体亡）：
//! 函数体、lambda 体、spawn 体各自是 Γ 的证据边界——守卫事实不跨体传入，体内
//! 自产的事实（含裸 `assert` 的第二来源）只在体内有效。
//!
//! 边界（不宣称）：只覆盖**裸** `assert(...)`；`assert.assert(...)`（字段访问形态）
//! 不扩面。`f(y)(y)` 的第二值位不属本项（E1002 另案）。

use crate::frontend::compiler::Compiler;

/// 编译一段 .yx 源码，返回首个错误的诊断码（成功返回 None）。
///
/// 只看码不看文案：判据是「有没有编译期证据」，不是措辞（措辞归 locales 单测）。
fn compile_error_code(source: &str) -> Option<String> {
    let mut compiler = Compiler::new();
    match compiler.compile("gamma_equality_bridge.yx", source) {
        Ok(_) => None,
        Err(error) => Some(
            error
                .diagnostic()
                .map(|diagnostic| diagnostic.code.as_str().to_string())
                .unwrap_or_else(|| format!("<无结构化诊断: {}>", error.message())),
        ),
    }
}

// ── 测试语料（.yx 源码；raw string 保持与真实语料同形）──────────────────

/// 分支内正控：守卫 `y == 42` 成立的分支内，`y` 的编译期值即 42。
/// 与 `f(42)(42)` 同形（同一特化值 42、同一证据出口）。
const BRANCH_IN_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

guarded: (y: Int) -> Int = {
    if y == 42 {
        f(y)(42)
    } else {
        0
    }
}

main: () -> Void = {
    a: Int = guarded(42)
    b: Int = f(42)(42)
    assert.assert(a == b, "分支内等式与字面量同形")
}

main()
"#;

/// 反向书写：`42 == y` 与 `y == 42` 同义。
const REVERSED_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

reversed: (y: Int) -> Int = {
    if 42 == y {
        f(y)(42)
    } else {
        0
    }
}

main: () -> Void = {
    assert.assert(reversed(42) == 42, "反向等式")
}

main()
"#;

/// 合取逐项分解：`y == 42 and z == 7` 两项都进 Γ。
const CONJUNCTION_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

both: (y: Int, z: Int) -> Int = {
    if y == 42 and z == 7 {
        f(z)(7)
    } else {
        0
    }
}

main: () -> Void = {
    assert.assert(both(42, 7) == 7, "合取第二项")
}

main()
"#;

/// 嵌套分支：内层分支同时持有外层与内层守卫。
const NESTED_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

nested: (y: Int) -> Int = {
    if y > 0 {
        if y == 42 {
            f(y)(42)
        } else {
            0
        }
    } else {
        0
    }
}

main: () -> Void = {
    assert.assert(nested(42) == 42, "嵌套内层")
}

main()
"#;

/// T18(b) 正控：函数体内**裸 assert** 的等式事实对体内可见
/// （生产 Γ 从 None 变 Some 前，这里必然 E1113）。
const ASSERT_IN_BODY_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

asserted: (y: Int) -> Int = {
    assert(y == 42, "y 应为 42")
    f(y)(42)
}

main: () -> Void = {
    assert.assert(asserted(42) == 42, "体内裸 assert")
}

main()
"#;

/// 分支合流后的调用：守卫事实出栈即失效。
const OUTSIDE_BRANCH_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

outside: (y: Int) -> Int = {
    if y == 42 {
        0
    } else {
        0
    }
    f(y)(42)
}

main: () -> Void = {
    outside(42)
}

main()
"#;

/// else 分支里等式取反（`!(y == 42)`）不是等式证据。
const ELSE_BRANCH_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

else_branch: (y: Int) -> Int = {
    if y == 42 {
        0
    } else {
        f(y)(42)
    }
}

main: () -> Void = {
    else_branch(7)
}

main()
"#;

/// mut 重赋值 kill 依赖该名的等式事实。
const KILLED_BY_REASSIGN_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

killed: (y0: Int) -> Int = {
    mut y: Int = y0
    if y == 42 {
        y = 43
        f(y)(42)
    } else {
        0
    }
}

main: () -> Void = {
    killed(42)
}

main()
"#;

/// `&mut y` 借出后 y 可能被写坏，等式事实按名保守作废。
const KILLED_BY_MUT_BORROW_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

take_mut: (p: &mut Int) -> Void = {}

borrowed: (y0: Int) -> Int = {
    mut y: Int = y0
    if y == 42 {
        take_mut(&mut y)
        f(y)(42)
    } else {
        0
    }
}

main: () -> Void = {
    borrowed(42)
}

main()
"#;

/// 只有**等式**是第四证据源；谓词守卫（`y > 0`）不产值。
const PREDICATE_GUARD_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

predicate: (y: Int) -> Int = {
    if y > 0 {
        f(y)(42)
    } else {
        0
    }
}

main: () -> Void = {
    predicate(42)
}

main()
"#;

/// T17(a)：守卫事实不入**函数体**。体内同名形参是新的运行期绑定，拿外层
/// 同名等式当证据会把运行期值错当编译期常量（unsound）。
const BARRIER_FUNCTION_BODY_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

outer: (y: Int) -> Int = {
    if y == 42 {
        inner: (y: Int) -> Int = f(y)(42)
        inner(42)
    } else {
        0
    }
}

main: () -> Void = {
    outer(42)
}

main()
"#;

/// T17(b)：守卫事实不入 **lambda 体**。闭包体延迟执行，外层事实在调用时
/// 未必仍成立。
const BARRIER_LAMBDA_BODY_SOURCE: &str = r#"use std.{assert, list}

f: (N: Int) -> (n: N) -> Int = (n) => n

closure_body: (y: Int) -> Int = {
    if y == 42 {
        r = list.map([1], (x: Int) => f(y)(42))
        r[0]
    } else {
        0
    }
}

main: () -> Void = {
    closure_body(42)
}

main()
"#;

/// T17(c)：守卫事实不入 **spawn 体**。
const BARRIER_SPAWN_BODY_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

spawned: (y: Int) -> Int = {
    if y == 42 {
        r = spawn { return f(y)(42) }
        r
    } else {
        0
    }
}

main: () -> Void = {
    spawned(42)
}

main()
"#;

/// T19：体内自产事实随体亡——spawn 体内裸 assert 的等式出体即失效。
const BARRIER_BODY_FACT_DIES_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

dies: (y: Int) -> Int = {
    r = spawn {
        assert(y == 42, "体内自产")
        return 0
    }
    f(y)(42)
}

main: () -> Void = {
    dies(42)
}

main()
"#;

/// T20：嵌套体退出后 barrier 还原到**外层体**深度（不是 0），外层体自产事实
/// 仍在体内可见。
const BARRIER_NESTED_SOURCE: &str = r#"use std.{assert, list}

f: (N: Int) -> (n: N) -> Int = (n) => n

nested_barrier: (y: Int) -> Int = {
    assert(y == 42, "体内自产")
    r = list.map([1], (x: Int) => x)
    f(y)(42)
}

main: () -> Void = {
    assert.assert(nested_barrier(42) == 42, "嵌套体退出后外层体事实仍可见")
}

main()
"#;

/// 本批只覆盖**裸** assert；`assert.assert`（字段访问形态）不扩面。
const ASSERT_DOT_ASSERT_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

field_access_form: (y: Int) -> Int = {
    assert.assert(y == 42, "字段访问形态不扩面")
    f(y)(42)
}

main: () -> Void = {
    field_access_form(42)
}

main()
"#;

/// 经典类型路径（字面量 / 值环境两类既有证据源）不受 Γ 桥影响。
const CLASSIC_PATH_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

main: () -> Void = {
    x: Int = 42
    a: Int = f(42)(42)
    b: Int = f(x)(42)
    assert.assert(a == 42 and b == 42, "既有证据源不变")
}

main()
"#;

/// 体内重赋值 kill：spawn 体共享外层帧，`y = …` 走表达式侧 walker 的同一
/// `classify_binding` 判定 ⇒ 体内自产的等式事实必须同时作废。
const KILLED_BY_BODY_REASSIGN_SOURCE: &str = r#"use std.assert

f: (N: Int) -> (n: N) -> Int = (n) => n

reassigned: (y0: Int) -> Int = {
    mut y: Int = y0
    r = spawn {
        assert(y == 42, "体内自产")
        y = 43
        return f(y)(42)
    }
    r
}

main: () -> Void = {
    reassigned(42)
}

main()
"#;

// ── 分支内：等式事实成为 q=0 证据（今日 E1113 ⇒ 绿）──────────────────────

#[test]
fn test_gamma_equality_in_branch_provides_q0_evidence() {
    // Arrange — 守卫成立的分支内，`y` 的编译期值即 42
    let source = BRANCH_IN_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code, None,
        "分支内 `f(y)(42)` 应凭 Γ 等式证据通过（与 `f(42)(42)` 同形）"
    );
}

#[test]
fn test_gamma_equality_reversed_operand_provides_q0_evidence() {
    // Arrange — `42 == y` 与 `y == 42` 同义
    let source = REVERSED_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(code, None, "`42 == y` 应给出与 `y == 42` 同一等式证据");
}

#[test]
fn test_gamma_equality_conjunction_provides_each_fact() {
    // Arrange — 合取逐项注入
    let source = CONJUNCTION_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(code, None, "合取应逐项注入，`z == 7` 的等式应可用");
}

#[test]
fn test_gamma_equality_nested_branch_provides_q0_evidence() {
    // Arrange — 嵌套 if 的内层分支
    let source = NESTED_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(code, None, "嵌套分支内的等式应可用");
}

#[test]
fn test_gamma_equality_assert_in_body_provides_q0_evidence() {
    // Arrange — T18(b) 正控：函数体内裸 assert 的等式事实对体内可见
    let source = ASSERT_IN_BODY_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code, None,
        "函数体内裸 assert 的等式事实应成为 q=0 证据（生产 Γ 可达）"
    );
}

#[test]
fn test_gamma_equality_outside_branch_reports_missing_evidence() {
    // Arrange — 分支合流后的调用
    let source = OUTSIDE_BRANCH_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "分支合流后的调用必须仍报 E1113"
    );
}

#[test]
fn test_gamma_equality_else_branch_reports_missing_evidence() {
    // Arrange — else 分支的取反守卫
    let source = ELSE_BRANCH_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "else 分支的取反守卫不构成等式证据，必须仍报 E1113"
    );
}

#[test]
fn test_gamma_equality_killed_by_reassign_reports_missing_evidence() {
    // Arrange — mut 重赋值 kill
    let source = KILLED_BY_REASSIGN_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "重赋值后旧等式不再成立，必须仍报 E1113"
    );
}

#[test]
fn test_gamma_equality_killed_by_mut_borrow_reports_missing_evidence() {
    // Arrange — `&mut y` 借出
    let source = KILLED_BY_MUT_BORROW_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "`&mut` 借出后等式事实应作废，必须仍报 E1113"
    );
}

#[test]
fn test_gamma_inequality_guard_reports_missing_evidence() {
    // Arrange — 谓词守卫（非等式）
    let source = PREDICATE_GUARD_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "谓词守卫不是常量等式，必须仍报 E1113"
    );
}

#[test]
fn test_gamma_barrier_function_body_blocks_outer_fact() {
    // Arrange — T17(a)：守卫事实不入函数体
    let source = BARRIER_FUNCTION_BODY_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "外层守卫事实不得进入嵌套函数体，体内 `f(y)(42)` 必须报 E1113"
    );
}

#[test]
fn test_gamma_barrier_lambda_body_blocks_outer_fact() {
    // Arrange — T17(b)：守卫事实不入 lambda 体
    let source = BARRIER_LAMBDA_BODY_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "外层守卫事实不得进入 lambda 体，体内 `f(y)(42)` 必须报 E1113"
    );
}

#[test]
fn test_gamma_barrier_spawn_body_blocks_outer_fact() {
    // Arrange — T17(c)：守卫事实不入 spawn 体
    let source = BARRIER_SPAWN_BODY_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "外层守卫事实不得进入 spawn 体，体内 `f(y)(42)` 必须报 E1113"
    );
}

#[test]
fn test_gamma_barrier_body_fact_dies_with_spawn_body() {
    // Arrange — T19：体内自产事实随体亡
    let source = BARRIER_BODY_FACT_DIES_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "spawn 体内自产的事实应随体亡，出体后的调用必须报 E1113"
    );
}

#[test]
fn test_gamma_barrier_nested_body_keeps_outer_body_fact() {
    // Arrange — T20：嵌套体退出后 barrier 还原到外层体深度
    let source = BARRIER_NESTED_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code, None,
        "嵌套体退出应还原到外层体 barrier，外层体事实仍可见"
    );
}

#[test]
fn test_gamma_equality_assert_dot_assert_form_reports_missing_evidence() {
    // Arrange — `assert.assert` 形态不扩面
    let source = ASSERT_DOT_ASSERT_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "`assert.assert` 不属本批覆盖面，必须仍报 E1113"
    );
}

#[test]
fn test_gamma_equality_literal_evidence_unchanged() {
    // Arrange — 经典类型路径
    let source = CLASSIC_PATH_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(code, None, "字面量 / 值环境两类既有证据必须照旧通过");
}
#[test]
fn test_gamma_equality_killed_by_body_reassign_reports_missing_evidence() {
    // Arrange — spawn 体内先立等式、再重赋值、再用旧等式要证据
    let source = KILLED_BY_BODY_REASSIGN_SOURCE;

    // Act
    let code = compile_error_code(source);

    // Assert
    assert_eq!(
        code,
        Some("E1113".to_string()),
        "体内重赋值后旧等式不得再作证据，必须报 E1113"
    );
}
