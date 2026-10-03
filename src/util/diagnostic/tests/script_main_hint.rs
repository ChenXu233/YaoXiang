//! #413：Script 模式 main 提示判定测试

use crate::frontend::Compiler;

/// 单文件编译（script 模式同路径）后判定 main 是否被顶层调用
fn compile_and_check(src: &str) -> bool {
    let module = Compiler::new().compile("t.yx", src).unwrap();
    crate::util::diagnostic::script_main_never_called(&module)
}

#[test]
fn test_main_defined_without_call_triggers_hint() {
    // #413 原始复现形态：只定义 main，顶层不调用
    assert!(compile_and_check("main = () => {\n    print(\"x\")\n}\n"));
}

#[test]
fn test_explicit_main_call_suppresses_hint() {
    // 顶层显式调用 main()——T4 语义下程序正常执行，不应提示
    assert!(!compile_and_check(
        "main = () => {\n    print(\"x\")\n}\n\nmain()\n"
    ));
}

#[test]
fn test_no_main_no_hint() {
    // 纯顶层脚本（无 main 绑定）：不涉及该提示
    assert!(!compile_and_check("print(\"top\")\n"));
}

#[test]
fn test_unrelated_main_call_via_other_name_still_detected() {
    // main 内部调用别的函数、顶层调用别的函数——main 未被调用应提示
    assert!(compile_and_check(
        "helper = () => {\n    return 1\n}\n\nmain = () => {\n    return helper()\n}\n\nprint(helper())\n"
    ));
}
