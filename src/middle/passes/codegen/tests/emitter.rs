//! 字节码发射器单元测试
//!
//! 测试 Emitter 的指令发射、映射记录和待回填跳转功能。

use crate::backends::common::Opcode;
use crate::middle::passes::codegen::emitter::Emitter;
use crate::middle::passes::codegen::BytecodeInstruction;

#[test]
fn test_emit_with_mapping() {
    let mut emitter = Emitter::new();

    let instr = BytecodeInstruction::new(Opcode::Mov, vec![0, 1]);
    emitter.emit_with_mapping(instr, 0);

    assert_eq!(emitter.ir_mapping().get(&0), Some(&0));
}

#[test]
fn test_pending_jumps() {
    let mut emitter = Emitter::new();

    for i in 0..5 {
        let instr = BytecodeInstruction::new(Opcode::Mov, vec![i as u8, 0]);
        emitter.emit_with_mapping(instr, i);
    }

    let jmp_instr = BytecodeInstruction::new(Opcode::Jmp, vec![0, 0, 0, 0]);
    emitter.emit_jump_with_pending(jmp_instr, 5, 10, Opcode::Jmp);

    emitter.mark_end(11);

    assert_eq!(emitter.ir_mapping().len(), 7);
}
