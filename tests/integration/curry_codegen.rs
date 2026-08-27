//! Curry 分层结构测试 — 基于 RFC-011 §4.1 编译期常量参数（curry 形式）
//!
//! RFC-011 §4.1: `f: (N: Int) -> (n: N) -> Int` 每层拆为独立 FunctionIR，
//! 中间层生成闭包链（MakeClosure 指向内层函数），逐层部分应用固化。
//!
//! 原则 4（测试行为，不测试实现）：本文件断言 curry 分层的**结构行为**
//! （生成了几层函数、中间层是否产出闭包链），不断言具体指令序列——
//! LoadArg / 参数原位注册等是编译器实现细节，可随实现演进变化。

use yaoxiang::frontend::Compiler;
use yaoxiang::middle::core::ir::{FunctionIR, Instruction};

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

    // Assert: 中间层 f 必须生成 MakeClosure 闭包，指向内层函数 __f_l0（闭包链结构）
    let makes_closure_to_inner = instrs
        .iter()
        .any(|instr| matches!(instr, Instruction::MakeClosure { func, .. } if func == "__f_l0"));
    assert!(
        makes_closure_to_inner,
        "f 应生成指向内层 __f_l0 的 MakeClosure 闭包（curry 分层闭包链），实际指令: {:?}",
        instrs
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
