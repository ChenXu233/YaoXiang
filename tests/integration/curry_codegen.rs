//! Curry 函数 IR 形态测试
//!
//! 验证 curry 函数生成的 IR 指令序列正确（issue #227）。
//! 直接编译源码并检查 `module.functions` 的指令，钉死形态。

#![allow(unused_imports)]
use yaoxiang::frontend::Compiler;
use yaoxiang::middle::core::ir::{FunctionIR, Instruction, Operand};

/// 编译源码并返回第一个函数的 IR
fn compile_first_func(src: &str) -> FunctionIR {
    let mut compiler = Compiler::new();
    let module = compiler
        .compile_with_source("test.yx", src)
        .expect("compile should succeed");
    module
        .functions
        .into_iter()
        .next()
        .expect("should have at least one function")
}

#[test]
fn two_layer_curry_generates_closure_chain() {
    let f = compile_first_func(
        r#"
        f: (N: Int) -> (n: N) -> Int = (n) => n
    "#,
    );

    let instrs = f.all_instructions().collect::<Vec<_>>();
    assert_eq!(
        instrs.len(),
        3,
        "f 应生成 LoadArg + MakeClosure + Ret，实际: {:?}",
        instrs
    );

    assert!(
        matches!(
            instrs[0],
            Instruction::Load {
                dst: Operand::Local(0),
                src: Operand::Arg(0)
            }
        ),
        "第 0 条应为 Load Arg(0) → Local(0)"
    );

    assert!(
        matches!(
            instrs[1],
            Instruction::MakeClosure {
                dst: Operand::Local(1),
                ref func,
                ref env
            } if func == "__f_l0" && env == &[Operand::Local(0)]
        ),
        "第 1 条应为 MakeClosure 到 __f_l0, env=[Local(0)]，实际: {:?}",
        instrs[1]
    );

    assert!(
        matches!(instrs[2], Instruction::Ret(Some(Operand::Local(1)))),
        "第 2 条应为 Ret(Local(1))"
    );
}

#[test]
fn three_layer_curry_generates_three_functions() {
    let mut compiler = Compiler::new();
    let module = compiler
        .compile_with_source(
            "test.yx",
            r#"
        add3: (a: Int) -> (b: Int) -> (c: Int) -> Int = (a, b, c) => a + b + c
    "#,
        )
        .expect("compile should succeed");

    // 应该有 3 个函数：add3, __add3_l0, __add3_l1
    let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"add3"), "应有 add3 函数，实际: {:?}", names);
    assert!(
        names.contains(&"__add3_l0"),
        "应有 __add3_l0 函数，实际: {:?}",
        names
    );
    assert!(
        names.contains(&"__add3_l1"),
        "应有 __add3_l1 函数，实际: {:?}",
        names
    );
}
