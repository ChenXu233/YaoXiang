//! 控制流代码生成测试

use crate::middle::core::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::backends::common::Opcode;

/// 测试 if 语句标签生成
#[test]
fn test_if_label_generation() {
    let module = ModuleIR::default();
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);

    let end_label = ctx.next_label();
    let then_label = ctx.next_label();

    assert_ne!(
        end_label, then_label,
        "If branches should have different labels"
    );
}

/// 测试 while 循环标签生成
#[test]
fn test_while_label_generation() {
    let module = ModuleIR::default();
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);

    let loop_label = ctx.next_label();
    let end_label = ctx.next_label();

    assert_ne!(
        loop_label, end_label,
        "Loop should have different start and end labels"
    );
    assert!(
        loop_label < end_label,
        "Labels should be in increasing order"
    );
}

/// 测试 for 循环标签生成
#[test]
fn test_for_label_generation() {
    let module = ModuleIR::default();
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);

    let loop_label = ctx.next_label();
    let end_label = ctx.next_label();
    let iter_label = ctx.next_label();

    assert_eq!(3, vec![loop_label, end_label, iter_label].len());
}

/// 测试 match 表达式标签
#[test]
fn test_match_label_generation() {
    let module = ModuleIR::default();
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);

    let end_label = ctx.next_label();
    let arm1 = ctx.next_label();
    let arm2 = ctx.next_label();
    let arm3 = ctx.next_label();

    assert_eq!(end_label, 0);
    assert_eq!(arm1, 1);
    assert_eq!(arm2, 2);
    assert_eq!(arm3, 3);
}

/// 测试跳转指令操作数计数
#[test]
fn test_jump_operand_count() {
    // Test jump opcodes have correct operand counts
    assert_eq!(Opcode::Jmp.operand_count(), 0);
    assert_eq!(Opcode::JmpIf.operand_count(), 2);
    assert_eq!(Opcode::JmpIfNot.operand_count(), 2);
    assert_eq!(Opcode::Switch.operand_count(), 3);
}

/// 测试循环指令
#[test]
fn test_loop_opcodes() {
    // Test loop opcodes exist and have correct names
    assert_eq!(Opcode::LoopStart.name(), "LoopStart");
    assert_eq!(Opcode::LoopInc.name(), "LoopInc");
    // Test operand counts
    assert_eq!(Opcode::LoopStart.operand_count(), 4);
    assert_eq!(Opcode::LoopInc.operand_count(), 3);
}

/// 测试标签指令
#[test]
fn test_label_opcode() {
    // Test label opcode exists and has correct properties
    assert_eq!(Opcode::Label.name(), "Label");
    assert_eq!(Opcode::Label.operand_count(), 1);
}

/// 测试 break/continue 标签支持
#[test]
fn test_break_continue_labels() {
    let module = ModuleIR::default();

    // 模拟带有循环标签的代码生成上下文
    let mut ctx = crate::middle::passes::codegen::CodegenContext::new(module);

    // 设置循环标签
    let loop_label = 10;
    let end_label = 20;

    ctx.test_flow().set_loop_label(loop_label, end_label);

    assert_eq!(ctx.test_flow().loop_label().unwrap().0, loop_label);
    assert_eq!(ctx.test_flow().loop_label().unwrap().1, end_label);
}

/// 测试控制流指令分类
#[test]
fn test_control_flow_classification() {
    // Test jump instructions
    assert!(Opcode::Jmp.is_jump_op());
    assert!(Opcode::JmpIf.is_jump_op());
    assert!(Opcode::JmpIfNot.is_jump_op());
    assert!(Opcode::Switch.is_jump_op());
    assert!(!Opcode::Nop.is_jump_op());

    // Test loop instructions are jump instructions
    assert!(Opcode::LoopStart.is_jump_op());
    assert!(Opcode::LoopInc.is_jump_op());
}

/// 测试基本块指令生成
#[test]
fn test_basic_block_instruction_order() {
    let block = BasicBlock {
        label: 0,
        instructions: vec![
            Instruction::Move {
                dst: Operand::Temp(0),
                src: Operand::Const(ConstValue::Int(0)),
            },
            Instruction::Add {
                dst: Operand::Temp(1),
                lhs: Operand::Temp(0),
                rhs: Operand::Const(ConstValue::Int(1)),
            },
            Instruction::Ret(Some(Operand::Temp(1))),
        ],
        successors: vec![],
    };

    assert_eq!(block.instructions.len(), 3);
    assert!(matches!(block.instructions[0], Instruction::Move { .. }));
    assert!(matches!(block.instructions[1], Instruction::Add { .. }));
    assert!(matches!(block.instructions[2], Instruction::Ret(_)));
}
