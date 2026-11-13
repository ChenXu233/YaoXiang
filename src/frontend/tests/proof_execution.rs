//! 证明执行路径：被调证明函数自身的形参精化义务（RFC-027 §3.3 / issue #537）
//!
//! 规范来源：RFC-027 §3.1（「`b: Positive(b)` 和 `f(5)` 一样，就是函数调用」）+
//! §3.3（「函数前置条件：调用 `divide(a, b)` 时，`b` 必须满足 `Positive` 的证据
//! 要么来自当前假设，要么来自实参自身的精化类型标注」）+ §12.2（Unproven 不得
//! 静默放行）。证明函数的形参带精化时，对该证明函数的每次编译期调用都须先满足
//! 其形参精化（以调用实参代入）——义务链终点是**被调证明函数自己的形参精化**。
//!
//! 历史：本文件曾以 tripwire 钉住「该义务被静默丢弃」的现状（6.9.2-A F14-B 登记为
//! issue #537），2026-11-14 的 3.20.22 修复轮翻面为真锚。隔离结论（探针矩阵见该轮
//! 实录）：静默丢义务的根因是证明执行器只求值被调函数**体**、从不检查其形参精化，
//! 与 IR 生成器的谓词命名空间无关（负literal实参不可折叠另报 E2031，属另一既有边界）。

use crate::frontend::compiler::Compiler;

/// 证明函数 `Check` 的形参 `p` 带精化 `Eq(n, 0)`；使用点给 `n = 7` ⇒ `Eq(7, 0)` 为假。
const PROOF_FN_REFINED_PARAM: &str = concat!(
    "Eq: (a: Int, b: Int) -> Type = { a == b }\n",
    "Check: (n: Int, p: Eq(n, 0)) -> Type = { n > 0 }\n",
    "f: (n: Int, p: Check(7, n)) -> Int = { n }\n",
    "main: () -> Void = {\n",
    "    y: Int = f(42, 7)\n",
    "}\n",
);

/// 普通函数的同形精化参数（对照面：义务存在且会被判定）。
const PLAIN_FN_REFINED_PARAM: &str = concat!(
    "Eq: (a: Int, b: Int) -> Type = { a == b }\n",
    "g: (n: Int, p: Eq(n, 0)) -> Int = { p }\n",
    "main: () -> Void = {\n",
    "    y: Int = g(7, 42)\n",
    "}\n",
);

/// 证明函数形参精化的**嵌套义务**必须被判定：`f(42, 7)` 的实参 `p` 须满足
/// `Check(7, 42)`，而 `Check` 自己的形参精化 `p: Eq(n, 0)` 以实参 `(7, 42)` 代入
/// 得 `Eq(7, 0)` 为假——编译必须报 E4018（issue #537 修复前此处静默通过）。
#[test]
fn test_proof_fn_param_refinement_obligation_is_reported() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("proof_fn_refined_param.yx", PROOF_FN_REFINED_PARAM);

    // Assert: 义务链终点 = 被调证明函数自己的形参精化 Eq(7, 0)，违反报 E4018
    let error = result.expect_err(
        "证明函数形参精化的嵌套义务（Eq(7, 0) 为假）必须被拒绝（E4018）；零诊断 = 义务被静默丢弃（issue #537 回归）",
    );
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E4018"),
        "证明函数面的嵌套义务违反应报 E4018（与普通函数对照面同码同判）；实际: {}",
        error.message()
    );
}

/// 对照：**普通函数**的同形精化参数会被判定（E4018）——证明夹具确实违反了义务，
/// 第一条测试的「零诊断」不是夹具太弱。
#[test]
fn test_plain_fn_param_refinement_obligation_is_reported() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("plain_fn_refined_param.yx", PLAIN_FN_REFINED_PARAM);

    // Assert
    let error = result.expect_err("普通函数的精化形参义务应被拒绝（E4018）");
    assert_eq!(
        error.diagnostic().map(|d| d.code.as_str()),
        Some("E4018"),
        "普通函数面应报 E4018（证明义务存在）；实际: {}",
        error.message()
    );
}
