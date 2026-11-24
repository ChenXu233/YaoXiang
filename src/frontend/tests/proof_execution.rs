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
//!
//! 追加（issue #509 残留②，WBS 3.21.5）：E4018 proof_false/proof_error 形态的
//! constraint 参数位句式在 locales（E4018.proof_*.constraint）随 error_lang
//! 渲染——端到端渲染级锚钉住 en 逐字形态。

use crate::frontend::compiler::Compiler;
use crate::frontend::core::sema::proof::verdict::ProofFunctionCall;
use crate::frontend::core::types::const_data::ConstValue;
use crate::frontend::proof_execution::execute_proof_calls;
use crate::util::span::Span;

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

/// 判据（issue #509 残留②）：E4018 proof_false 形态的 constraint 参数位句式
/// 在 locales（E4018.proof_false.constraint）随 error_lang 渲染——经生产入口
/// `Compiler::compile_with_source` 的端到端锚，钉住 en 逐字形态（默认即 en，
/// 显式设定使断言与环境配置无关，arg_binding.rs 先例）。
///
/// 夹具取返回位义务形态（proof_obligations.rs 的 RETURN_POSITION 同源）：
/// `SumUpTo(3, 7)` 的体 `s == n * 2` 判 false ⇒ 证明执行器 proof_false 臂
/// （`constraint_is_terminates` 不命中、前三级不闭合，义务入第四级）。
/// 绑定注解形态（`IsPositive(-1)` 类）走 refinement 的闭合约束支，不经本臂。
#[test]
fn test_proof_fn_false_message_renders_constraint_from_locales() {
    // Arrange：SumUpTo(3, 7) 判 false ⇒ E4018（proof_false 路径）
    crate::util::i18n::set_lang_from_string("en".to_string());
    let source = concat!(
        "SumUpTo: (n: Int, s: Int) -> Type = { s == n * 2 }
",
        "f: () -> (r: SumUpTo(3, r)) = {
",
        "    return 7
",
        "}
",
        "
",
        "main: () -> Void = {}
",
    );
    let mut compiler = Compiler::new();

    // Act
    let error = compiler
        .compile_with_source("test.yao", source)
        .expect_err("SumUpTo(3, 7) 判 false，必须编译失败（E4018）");

    // Assert：constraint 参数位来自 locales 键渲染（en 逐字），非硬编中文
    let diag = error.diagnostic().expect("编译失败必须带诊断");
    assert_eq!(
        diag.code,
        "E4018",
        "证明函数判伪必须报 E4018；实际: {}",
        error.message()
    );
    assert!(
        diag.message
            .contains("Proof function 'SumUpTo' returned false: constraint not satisfied (args: [Int(3), Int(7)]); check the constraint or change the arguments"),
        "constraint 参数位必须来自 E4018.proof_false.constraint 键渲染（en 逐字，issue #509 残留②）。Got: {}",
        diag.message
    );
    assert!(
        diag.message
            .contains("(Proof function returned false; no concrete counterexample.)"),
        "反例段注记必须来自 E4018.proof_false.note 键渲染。Got: {}",
        diag.message
    );
}

/// 判据（issue #509 残留②）：E4018 proof_error 形态的 wrapper 句式在 locales
/// （E4018.proof_error.constraint）——经生产入口 `execute_proof_calls` 直驱
/// （func 不在 AST ⇒ 执行失败路径）。{error} 是内部 Err 文本的插值位（登记
/// 不迁移，issue 原意），不在断言面；wrapper 与注记为迁移面，逐字断言。
#[test]
fn test_proof_fn_execution_failure_message_renders_wrapper_from_locales() {
    // Arrange：AST 中不存在名为 Missing 的函数 ⇒ execute_single_proof_fn 返回 Err
    crate::util::i18n::set_lang_from_string("en".to_string());
    let ast = crate::ast::Module {
        items: vec![],
        span: Span::dummy(),
    };
    let call = ProofFunctionCall {
        func_name: "Missing".to_string(),
        args: vec![ConstValue::Int(1)],
    };

    // Act
    let errors = execute_proof_calls(std::slice::from_ref(&call), &ast, &Default::default());

    // Assert
    assert_eq!(
        errors.len(),
        1,
        "执行失败必须产出恰好一条 E4018 诊断；实际: {:?}",
        errors.iter().map(|d| d.code.clone()).collect::<Vec<_>>()
    );
    assert_eq!(errors[0].code, "E4018", "执行失败形态保持 E4018 码");
    assert!(
        errors[0]
            .message
            .contains("Proof function 'Missing' failed to execute: "),
        "wrapper 必须来自 E4018.proof_error.constraint 键渲染（en 逐字，issue #509 残留②）。Got: {}",
        errors[0].message
    );
    assert!(
        errors[0]
            .message
            .ends_with("(Proof function failed to execute; no concrete counterexample.)"),
        "反例段注记必须来自 E4018.proof_error.note 键渲染。Got: {}",
        errors[0].message
    );
}
