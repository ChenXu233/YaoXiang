//! 控制流代码生成测试

use crate::middle::ir::{BasicBlock, ConstValue, FunctionIR, Instruction, ModuleIR, Operand};
use crate::vm::opcode::TypedOpcode;

/// 测试 if 语句标签生成
#[test]
fn test_if_label_generation() {
    let module = ModuleIR::default();
    let mut ctx = crate::middle::codegen::CodegenContext::new(module);

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
    let mut ctx = crate::middle::codegen::CodegenContext::new(module);

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
    let mut ctx = crate::middle::codegen::CodegenContext::new(module);

    let loop_label = ctx.next_label();
    let end_label = ctx.next_label();
    let iter_label = ctx.next_label();

    assert_eq!(3, vec![loop_label, end_label, iter_label].len());
}

/// 测试 match 表达式标签
#[test]
fn test_match_label_generation() {
    let module = ModuleIR::default();
    let mut ctx = crate::middle::codegen::CodegenContext::new(module);

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
    assert_eq!(TypedOpcode::Jmp.operand_count(), 0);
    assert_eq!(TypedOpcode::JmpIf.operand_count(), 2);
    assert_eq!(TypedOpcode::JmpIfNot.operand_count(), 2);
    assert_eq!(TypedOpcode::Switch.operand_count(), 3);
}

/// 测试循环指令
#[test]
fn test_loop_opcodes() {
    assert_eq!(TypedOpcode::LoopStart.name(), "LoopStart");
    assert_eq!(TypedOpcode::LoopInc.name(), "LoopInc");
    assert_eq!(TypedOpcode::LoopStart.operand_count(), 4);
    assert_eq!(TypedOpcode::LoopInc.operand_count(), 3);
}

/// 测试标签指令
#[test]
fn test_label_opcode() {
    assert_eq!(TypedOpcode::Label.name(), "Label");
    assert_eq!(TypedOpcode::Label.operand_count(), 1);
}

/// 测试 break/continue 标签支持
#[test]
fn test_break_continue_labels() {
    let module = ModuleIR::default();

    // 模拟带有循环标签的代码生成上下文
    let mut ctx = crate::middle::codegen::CodegenContext::new(module);

    // 设置循环标签
    let loop_label = 10;
    let end_label = 20;

    ctx.current_loop_label = Some((loop_label, end_label));

    assert_eq!(ctx.current_loop_label.unwrap().0, loop_label);
    assert_eq!(ctx.current_loop_label.unwrap().1, end_label);
}

/// 测试控制流指令分类
#[test]
fn test_control_flow_classification() {
    // 跳转指令
    assert!(TypedOpcode::Jmp.is_jump_op());
    assert!(TypedOpcode::JmpIf.is_jump_op());
    assert!(TypedOpcode::JmpIfNot.is_jump_op());
    assert!(TypedOpcode::Switch.is_jump_op());
    assert!(!TypedOpcode::Nop.is_jump_op());

    // 循环指令
    assert!(TypedOpcode::LoopStart.is_jump_op());
    assert!(TypedOpcode::LoopInc.is_jump_op());
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
