//! 字节码发射器单元测试
//!
//! 测试 Emitter 的指令发射、映射记录和待回填跳转功能。

use crate::middle::bytecode::opcode;
use crate::middle::passes::codegen::emitter::Emitter;
use crate::middle::bytecode::file::BytecodeInstruction;

#[test]
fn test_emit_with_mapping() {
    let mut emitter = Emitter::new();

    let instr = BytecodeInstruction::new(opcode::MOV, vec![0, 1]);
    emitter.emit_with_mapping(instr, 0);

    assert_eq!(emitter.ir_mapping().get(&0), Some(&0));
}
