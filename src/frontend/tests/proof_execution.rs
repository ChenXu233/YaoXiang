//! 证明执行路径的命名空间缺口（F14-B / RFC-039 D62 修复轮）
//!
//! # tripwire（勿把本文件的断言读成「正确行为」）
//!
//! 下面第一条测试断言的是**当前事实**：证明函数签名上的**形参精化义务**在调用点
//! 被静默丢弃（零诊断）。这是既有缺口，不是 D62 引入的（改前 `ir_gen` 的静态
//! `SemanticProbe` 对用户声明谓词同样答「不是谓词」，两版对该路径裁决逐字相同）。
//!
//! 根因**未完全隔离**：t14 复审实测「把精化引用换成模块级常量同样丢义务」，
//! 故不能把根因坐实到 `frontend/proof_execution.rs` 的空谓词命名空间。本轮只做两件：
//! ① 生成器的谓词命名空间改为构造期**必需输入**（`TypeLevelNamespace`），空集不再
//! 由默认值静默提供；② 用本文件钉住症状，修好后把 `is_ok` 断言改为「必须报 E4018」。
//!
//! 触发条件（复现形态）：证明函数（`-> Type` 的声明）的形参带精化，且该精化在
//! **调用点可由已知实参求值**却被跳过——
//! `Check: (n: Int, p: Eq(n, 0)) -> Type` + 调用点 `Check(7, n)`：`Eq(7, 0)` 为假，
//! 该义务应当被拒绝。对照：**普通函数**的同形精化参数会正确报 E4018（第二条测试）。

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

/// tripwire：**当前零诊断**——证明函数形参精化的调用方义务被静默丢弃。
///
/// 修好后本条必须改成「报 E4018」；在那之前不得把它当作「该形态合法」的证据。
#[test]
fn test_proof_fn_param_refinement_obligation_is_currently_dropped() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let result = compiler.compile("proof_fn_refined_param.yx", PROOF_FN_REFINED_PARAM);

    // Assert — tripwire：当前没有诊断（根因未隔离，本轮不盲改行为）
    assert!(
        result.is_ok(),
        "tripwire：证明函数形参精化的调用方义务当前被静默丢弃（应为零诊断）；若这里开始失败，说明该义务已被接管——请把断言改成「必须报 E4018」并删掉本 tripwire。实际: {:?}",
        result.err().map(|e| e.message().to_string())
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
