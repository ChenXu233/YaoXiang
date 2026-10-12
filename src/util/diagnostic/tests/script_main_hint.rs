//! #413：Script 模式 main 提示判定测试
//!
//! 规范来源：RFC-029 §角色推断（Script 角色——无 manifest 即无入口概念，
//! 顶层语句即程序主体）与 codegen `find_entry_point` 的 T4 决议（自动执行
//! main 会与显式 main() 双跑，#356）。本文件锚定提示判定
//! `script_main_never_called` 的行为契约：文件绑定 main 且模块初始化序列
//! 无对它的直接调用 → 判定命中（提示输出）。

use crate::frontend::Compiler;

/// 单文件编译（script 模式同路径）后判定 main 是否被顶层调用
fn compile_and_check(src: &str) -> bool {
    let module = Compiler::new()
        .compile("t.yx", src)
        .unwrap_or_else(|e| panic!("compile failed:\nSource:\n{src}\nError:\n{e:?}"));
    crate::util::diagnostic::script_main_never_called(&module)
}

#[test]
fn test_main_defined_without_call_triggers_hint() {
    // Arrange：#413 原始复现形态——只定义 main，顶层不调用
    let src = "main = () => {\n    print(\"x\")\n}\n";

    // Act
    let hinted = compile_and_check(src);

    // Assert
    assert!(hinted, "定义 main 未调用应命中提示判定");
}

#[test]
fn test_explicit_main_call_suppresses_hint() {
    // Arrange：顶层显式调用 main()——T4 语义下程序正常执行
    let src = "main = () => {\n    print(\"x\")\n}\n\nmain()\n";

    // Act
    let hinted = compile_and_check(src);

    // Assert
    assert!(!hinted, "顶层显式调用 main() 不应命中提示判定");
}

#[test]
fn test_no_main_no_hint() {
    // Arrange：纯顶层脚本（无 main 绑定）
    let src = "print(\"top\")\n";

    // Act
    let hinted = compile_and_check(src);

    // Assert
    assert!(!hinted, "无 main 绑定不应命中提示判定");
}

#[test]
fn test_top_level_calls_other_fn_main_still_hinted() {
    // Arrange：顶层调用 helper 而非 main——main 定义了但未被调用
    let src = "helper = () => {\n    return 1\n}\n\nmain = () => {\n    return helper()\n}\n\nprint(helper())\n";

    // Act
    let hinted = compile_and_check(src);

    // Assert
    assert!(hinted, "顶层只调用别的函数、main 未被调用应命中提示判定");
}

#[test]
fn test_value_block_main_silences_hint() {
    // Arrange：#422 UX 对账——值型块体在初始化期内联求值，main 的效果已发生；
    // 此时提示"未被调用"是误导（照提示补 main() 会双跑）
    let src = "main = {\n    print(\"x\")\n}\n";

    // Act
    let hinted = compile_and_check(src);

    // Assert
    assert!(!hinted, "值型块体 main 已在初始化期求值，不应命中提示判定");
}

#[test]
fn test_value_scalar_main_silences_hint() {
    // Arrange：标量值 main（`main = 5`）——初始化期求值，无观察效果但无惰性体可跑
    let src = "main = 5\n";

    // Act
    let hinted = compile_and_check(src);

    // Assert
    assert!(!hinted, "标量值 main 已在初始化期求值，不应命中提示判定");
}

#[test]
fn test_block_tail_lambda_main_still_hinted() {
    // Arrange：块值求值的结果是**新闭包**（`main = { () => {...} }`）——
    // main 的值惰性，体从未运行，提示成立
    let src = "main = { () => {\n    print(\"x\")\n} }\n";

    // Act
    let hinted = compile_and_check(src);

    // Assert
    assert!(hinted, "块尾 lambda（存入新闭包）体未运行，应命中提示判定");
}
