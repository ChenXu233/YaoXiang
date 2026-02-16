//! 字节码发射器
//!
//! 负责字节码指令发射和跳转偏移回填。

use crate::backends::common::Opcode;
use crate::middle::passes::codegen::buffer::BytecodeBuffer;
use crate::middle::passes::codegen::BytecodeInstruction;
use std::collections::HashMap;

/// 待回填的跳转指令
#[derive(Debug)]
struct PendingJump {
    /// 指令在字节码中的索引
    instr_idx: usize,
    /// 跳转目标的 IR 索引
    target_ir_idx: usize,
    /// 操作码类型（用于确定回填位置）
    opcode: Opcode,
}

/// 字节码发射器
///
/// 职责：
/// - 发射字节码指令
/// - 记录跳转目标映射
/// - 回填跳转偏移量
#[derive(Debug)]
pub struct Emitter {
    /// 字节码缓冲区
    buffer: BytecodeBuffer,
    /// IR 索引 → 字节码索引 映射
    ir_to_bytecode_map: HashMap<usize, usize>,
    /// 待回填的跳转指令
    pending_jumps: Vec<PendingJump>,
}

impl Emitter {
    /// 创建新的发射器
    pub fn new() -> Self {
        Emitter {
            buffer: BytecodeBuffer::new(),
            ir_to_bytecode_map: HashMap::new(),
            pending_jumps: Vec::new(),
        }
    }

    /// 发射指令并记录位置映射
    pub fn emit_with_mapping(
        &mut self,
        instr: BytecodeInstruction,
        ir_index: usize,
    ) -> usize {
        // 记录 IR 索引到字节码指令索引的映射
        let bytecode_idx = self.buffer.bytecode().len() / instr.encoded_size();
        self.ir_to_bytecode_map.insert(ir_index, bytecode_idx);

        // 发射指令
        self.buffer.emit(&instr.encode());

        bytecode_idx
    }

    /// 发射跳转指令并记录待回填
    pub fn emit_jump_with_pending(
        &mut self,
        instr: BytecodeInstruction,
        ir_index: usize,
        target_ir_idx: usize,
        opcode: Opcode,
    ) {
        let bytecode_idx = self.buffer.bytecode().len();

        // 记录 IR 索引到字节码指令索引的映射
        let bytecode_instr_idx = bytecode_idx / instr.encoded_size();
        self.ir_to_bytecode_map.insert(ir_index, bytecode_instr_idx);

        self.pending_jumps.push(PendingJump {
            instr_idx: bytecode_idx,
            target_ir_idx,
            opcode,
        });

        self.buffer.emit(&instr.encode());
    }

    /// 记录结束位置映射（用于跳到函数末尾）
    pub fn mark_end(
        &mut self,
        ir_index: usize,
    ) {
        let bytecode_idx = self.buffer.bytecode().len();
        self.ir_to_bytecode_map.insert(ir_index, bytecode_idx);
    }

    /// 回填跳转偏移（直接操作字节码）
    pub fn backfill_jumps(&mut self) {
        let bytecode = self.buffer.bytecode_mut();

        for pending in &self.pending_jumps {
            if let Some(&target_bytecode_idx) = self.ir_to_bytecode_map.get(&pending.target_ir_idx)
            {
                let offset = (target_bytecode_idx as i32) - (pending.instr_idx as i32);
                let bytes = offset.to_le_bytes();

                match pending.opcode {
                    Opcode::Jmp => {
                        bytecode[pending.instr_idx + 1] = bytes[0];
                        bytecode[pending.instr_idx + 2] = bytes[1];
                        bytecode[pending.instr_idx + 3] = bytes[2];
                        bytecode[pending.instr_idx + 4] = bytes[3];
                    }
                    Opcode::JmpIf | Opcode::JmpIfNot => {
                        bytecode[pending.instr_idx + 2] = bytes[0];
                        bytecode[pending.instr_idx + 3] = bytes[1];
                        bytecode[pending.instr_idx + 4] = bytes[2];
                        bytecode[pending.instr_idx + 5] = bytes[3];
                    }
                    _ => {}
                }
            }
        }
    }

    /// 获取字节码缓冲区可变引用
    pub fn buffer_mut(&mut self) -> &mut BytecodeBuffer {
        &mut self.buffer
    }

    /// 获取字节码内容
    pub fn bytecode(&self) -> &[u8] {
        self.buffer.bytecode()
    }

    /// 获取常量池
    pub fn take_constant_pool(&mut self) -> Vec<crate::middle::core::ir::ConstValue> {
        self.buffer.take_constant_pool()
    }

    /// 添加常量
    pub fn add_constant(
        &mut self,
        value: crate::middle::core::ir::ConstValue,
    ) -> usize {
        self.buffer.add_constant(value)
    }

    /// 获取 IR → 字节码映射
    pub fn ir_mapping(&self) -> &HashMap<usize, usize> {
        &self.ir_to_bytecode_map
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Emitter::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
