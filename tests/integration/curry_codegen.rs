//! Curry IR 形态测试 — 基于 RFC-011 §4.1 函数级 const 泛型（curry 形式）
//!
//! 验证 curry 函数生成的 IR 指令序列正确。期望值来自设计 spec：
//! docs/superpowers/specs/2026-07-26-curry-desugaring-design.md
//!
//! RFC-011 §4.1: 编译期常量参数 curry 形式
//!   f: (N: Int) -> (n: N) -> Int 每层拆为独立 FunctionIR
//!   中间层: MakeClosure(下一层) + Ret
//!   最内层: LoadArg(env) + LoadArg(params) + 原 body + Ret

use yaoxiang::frontend::Compiler;
use yaoxiang::middle::core::ir::{FunctionIR, Instruction, Operand};

/// 编译源码并返回第一个函数的 IR
fn compile_first_func(src: &str) -> FunctionIR {
    // Arrange
    let mut compiler = Compiler::new();
    // Act
    let module = compiler
        .compile_with_source("test.yx", src)
        .expect("compile should succeed for curry function");
    // Assert

    module
        .functions
        .into_iter()
        .next()
        .expect("compiled module should have at least one function")
}

#[test]
fn two_layer_curry_generates_closure_chain() {
    // Arrange
    let f = compile_first_func(
        r#"
        f: (N: Int) -> (n: N) -> Int = (n) => n
    "#,
    );

    // Act
    let instrs = f.all_instructions().collect::<Vec<_>>();

    // Assert: 2 层 curry 的 f 应生成 LoadArg + MakeClosure + Ret
    assert_eq!(
        instrs.len(),
        3,
        "f 应生成 3 条指令 (LoadArg + MakeClosure + Ret)，实际: {:?}",
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
        "第 0 条应为 Load Arg(0) → Local(0)，实际: {:?}",
        instrs[0]
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
        "第 2 条应为 Ret(Local(1))，实际: {:?}",
        instrs[2]
    );
}

#[test]
fn three_layer_curry_generates_three_functions() {
    // Arrange
    let mut compiler = Compiler::new();

    // Act
    let module = compiler
        .compile_with_source(
            "test.yx",
            r#"
        add3: (a: Int) -> (b: Int) -> (c: Int) -> Int = (a, b, c) => a + b + c
    "#,
        )
        .expect("compile should succeed for 3-layer curry");

    // Assert: 3 层 curry 应生成 add3 + __add3_l0 + __add3_l1 三个函数
    let names: Vec<&str> = module.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"add3"),
        "应有最外层函数 add3，实际函数列表: {:?}",
        names
    );
    assert!(
        names.contains(&"__add3_l0"),
        "应有中间层函数 __add3_l0，实际函数列表: {:?}",
        names
    );
    assert!(
        names.contains(&"__add3_l1"),
        "应有最内层函数 __add3_l1，实际函数列表: {:?}",
        names
    );
}
