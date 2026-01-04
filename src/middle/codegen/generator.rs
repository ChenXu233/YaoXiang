//! Bytecode Generator
//!
//! Translates Middle IR to Typed Bytecode.

use crate::middle::ir::{FunctionIR, Instruction, Operand, ConstValue, BasicBlock};
use crate::middle::codegen::bytecode::{BytecodeInstruction, FunctionCode};
use crate::vm::opcode::TypedOpcode;
use crate::frontend::typecheck::MonoType;
use std::collections::HashMap;

pub struct BytecodeGenerator<'a> {
    ir: &'a FunctionIR,
    instructions: Vec<BytecodeInstruction>,
    label_map: HashMap<usize, usize>, // IR Label -> Bytecode Offset (instruction index)
    jumps_to_patch: Vec<(usize, usize)>, // (Instruction Index, Target Label)
    next_reg: u8,
}

impl<'a> BytecodeGenerator<'a> {
    pub fn new(ir: &'a FunctionIR) -> Self {
        let param_count = ir.params.len();
        let local_count = ir.locals.len();
        Self {
            ir,
            instructions: Vec::new(),
            label_map: HashMap::new(),
            jumps_to_patch: Vec::new(),
            next_reg: (param_count + local_count) as u8,
        }
    }

    pub fn generate(mut self) -> FunctionCode {
        // First pass: generate instructions
        for block in &self.ir.blocks {
            self.label_map.insert(block.label, self.instructions.len());
            for instr in &block.instructions {
                self.translate_instruction(instr);
            }
        }

        // Patch jumps
        for (instr_idx, target_label) in self.jumps_to_patch {
            if let Some(&target_offset) = self.label_map.get(&target_label) {
                let offset = target_offset as i32 - instr_idx as i32;
                let instr = &mut self.instructions[instr_idx];
                match TypedOpcode::try_from(instr.opcode).unwrap() {
                    TypedOpcode::Jmp => {
                        instr.operands = offset.to_le_bytes().to_vec();
                    }
                    TypedOpcode::JmpIf | TypedOpcode::JmpIfNot => {
                        let offset16 = offset as i16;
                        let bytes = offset16.to_le_bytes();
                        instr.operands[1] = bytes[0];
                        instr.operands[2] = bytes[1];
                    }
                    _ => {}
                }
            }
        }

        FunctionCode {
            name: self.ir.name.clone(),
            params: self.ir.params.clone(),
            return_type: self.ir.return_type.clone(),
            instructions: self.instructions,
            local_count: self.ir.locals.len(),
        }
    }

    fn translate_instruction(&mut self, instr: &Instruction) {
        match instr {
            Instruction::Add { dst, lhs, rhs } => {
                let type_ = self.get_operand_type(lhs);
                let opcode = match type_ {
                    MonoType::Int(_) => TypedOpcode::I64Add,
                    MonoType::Float(_) => TypedOpcode::F64Add,
                    _ => TypedOpcode::I64Add,
                };
                self.emit_arithmetic(opcode, dst, lhs, rhs);
            }
            Instruction::Sub { dst, lhs, rhs } => {
                let type_ = self.get_operand_type(lhs);
                let opcode = match type_ {
                    MonoType::Int(_) => TypedOpcode::I64Sub,
                    MonoType::Float(_) => TypedOpcode::F64Sub,
                    _ => TypedOpcode::I64Sub,
                };
                self.emit_arithmetic(opcode, dst, lhs, rhs);
            }
            Instruction::Mul { dst, lhs, rhs } => {
                let type_ = self.get_operand_type(lhs);
                let opcode = match type_ {
                    MonoType::Int(_) => TypedOpcode::I64Mul,
                    MonoType::Float(_) => TypedOpcode::F64Mul,
                    _ => TypedOpcode::I64Mul,
                };
                self.emit_arithmetic(opcode, dst, lhs, rhs);
            }
            Instruction::Div { dst, lhs, rhs } => {
                let type_ = self.get_operand_type(lhs);
                let opcode = match type_ {
                    MonoType::Int(_) => TypedOpcode::I64Div,
                    MonoType::Float(_) => TypedOpcode::F64Div,
                    _ => TypedOpcode::I64Div,
                };
                self.emit_arithmetic(opcode, dst, lhs, rhs);
            }
            Instruction::Eq { dst, lhs, rhs } => {
                let type_ = self.get_operand_type(lhs);
                let opcode = match type_ {
                    MonoType::Int(_) => TypedOpcode::I64Eq,
                    MonoType::Float(_) => TypedOpcode::F64Eq,
                    _ => TypedOpcode::I64Eq,
                };
                self.emit_arithmetic(opcode, dst, lhs, rhs);
            }
            Instruction::Ne { dst, lhs, rhs } => {
                let type_ = self.get_operand_type(lhs);
                let opcode = match type_ {
                    MonoType::Int(_) => TypedOpcode::I64Ne,
                    MonoType::Float(_) => TypedOpcode::F64Ne,
                    _ => TypedOpcode::I64Ne,
                };
                self.emit_arithmetic(opcode, dst, lhs, rhs);
            }
            Instruction::Lt { dst, lhs, rhs } => {
                let type_ = self.get_operand_type(lhs);
                let opcode = match type_ {
                    MonoType::Int(_) => TypedOpcode::I64Lt,
                    MonoType::Float(_) => TypedOpcode::F64Lt,
                    _ => TypedOpcode::I64Lt,
                };
                self.emit_arithmetic(opcode, dst, lhs, rhs);
            }
            Instruction::Le { dst, lhs, rhs } => {
                let type_ = self.get_operand_type(lhs);
                let opcode = match type_ {
                    MonoType::Int(_) => TypedOpcode::I64Le,
                    MonoType::Float(_) => TypedOpcode::F64Le,
                    _ => TypedOpcode::I64Le,
                };
                self.emit_arithmetic(opcode, dst, lhs, rhs);
            }
            Instruction::Gt { dst, lhs, rhs } => {
                let type_ = self.get_operand_type(lhs);
                let opcode = match type_ {
                    MonoType::Int(_) => TypedOpcode::I64Gt,
                    MonoType::Float(_) => TypedOpcode::F64Gt,
                    _ => TypedOpcode::I64Gt,
                };
                self.emit_arithmetic(opcode, dst, lhs, rhs);
            }
            Instruction::Ge { dst, lhs, rhs } => {
                let type_ = self.get_operand_type(lhs);
                let opcode = match type_ {
                    MonoType::Int(_) => TypedOpcode::I64Ge,
                    MonoType::Float(_) => TypedOpcode::F64Ge,
                    _ => TypedOpcode::I64Ge,
                };
                self.emit_arithmetic(opcode, dst, lhs, rhs);
            }
            Instruction::Move { dst, src } => {
                let dst_reg = self.resolve_dst(dst);
                let src_reg = self.load_operand(src);
                self.emit(TypedOpcode::Mov, vec![dst_reg, src_reg]);
            }
            Instruction::Jmp(label) => {
                self.jumps_to_patch.push((self.instructions.len(), *label));
                self.emit(TypedOpcode::Jmp, vec![0, 0, 0, 0]);
            }
            Instruction::JmpIf(cond, label) => {
                let cond_reg = self.load_operand(cond);
                self.jumps_to_patch.push((self.instructions.len(), *label));
                self.emit(TypedOpcode::JmpIf, vec![cond_reg, 0, 0]);
            }
            Instruction::Ret(val) => {
                if let Some(val) = val {
                    let reg = self.load_operand(val);
                    self.emit(TypedOpcode::ReturnValue, vec![reg]);
                } else {
                    self.emit(TypedOpcode::Return, vec![]);
                }
            }
            Instruction::Call { dst, func, args } => {
                // CallStatic: dst, func_id, base_arg_reg, arg_count
                // We need to move args to contiguous registers if they aren't already.
                // For simplicity, let's allocate a block of temp registers for args.
                let arg_count = args.len();
                let base_arg_reg = self.next_temp_reg();
                // Reserve more regs
                for _ in 1..arg_count {
                    self.next_temp_reg();
                }
                
                // Move args to these regs
                for (i, arg) in args.iter().enumerate() {
                    let arg_reg = self.load_operand(arg);
                    let target_reg = base_arg_reg + i as u8;
                    if arg_reg != target_reg {
                        self.emit(TypedOpcode::Mov, vec![target_reg, arg_reg]);
                    }
                }

                let dst_reg = if let Some(d) = dst {
                    self.resolve_dst(d)
                } else {
                    0 // Discard result
                };

                // Resolve func
                // Assuming func is a Const(Int) representing ID or Label
                // Or a Global.
                // For now, placeholder ID 0.
                let func_id = 0u32; 
                
                let mut operands = vec![dst_reg];
                operands.extend_from_slice(&func_id.to_le_bytes());
                operands.push(base_arg_reg);
                operands.push(arg_count as u8);
                
                self.emit(TypedOpcode::CallStatic, operands);
            }
            _ => {
                // TODO: Handle other instructions
            }
        }
    }

    fn emit_arithmetic(&mut self, opcode: TypedOpcode, dst: &Operand, lhs: &Operand, rhs: &Operand) {
        let dst_reg = self.resolve_dst(dst);
        let lhs_reg = self.load_operand(lhs);
        let rhs_reg = self.load_operand(rhs);
        self.emit(opcode, vec![dst_reg, lhs_reg, rhs_reg]);
    }

    fn emit(&mut self, opcode: TypedOpcode, operands: Vec<u8>) {
        self.instructions.push(BytecodeInstruction {
            opcode: opcode as u8,
            operands,
        });
    }

    fn next_temp_reg(&mut self) -> u8 {
        let reg = self.next_reg;
        self.next_reg = self.next_reg.wrapping_add(1);
        reg
    }

    fn resolve_dst(&mut self, op: &Operand) -> u8 {
        match op {
            Operand::Register(r) => *r,
            Operand::Local(idx) => (*idx + self.ir.params.len()) as u8,
            Operand::Temp(idx) => (*idx + self.ir.params.len() + self.ir.locals.len()) as u8,
            Operand::Arg(idx) => *idx as u8,
            _ => 0, // Should not happen for dst
        }
    }

    fn load_operand(&mut self, op: &Operand) -> u8 {
        match op {
            Operand::Register(r) => *r,
            Operand::Local(idx) => (*idx + self.ir.params.len()) as u8,
            Operand::Temp(idx) => (*idx + self.ir.params.len() + self.ir.locals.len()) as u8,
            Operand::Arg(idx) => *idx as u8,
            Operand::Const(c) => {
                let reg = self.next_temp_reg();
                match c {
                    ConstValue::Int(v) => {
                        let val = *v as i64;
                        let mut operands = vec![reg];
                        operands.extend_from_slice(&val.to_le_bytes());
                        self.emit(TypedOpcode::I64Const, operands);
                    }
                    ConstValue::Float(v) => {
                        let val = *v;
                        let mut operands = vec![reg];
                        operands.extend_from_slice(&val.to_le_bytes());
                        self.emit(TypedOpcode::F64Const, operands);
                    }
                    _ => {}
                }
                reg
            }
            _ => 0,
        }
    }

    fn get_operand_type(&self, op: &Operand) -> MonoType {
        match op {
            Operand::Local(idx) => self.ir.locals.get(*idx).cloned().unwrap_or(MonoType::Void),
            Operand::Arg(idx) => self.ir.params.get(*idx).cloned().unwrap_or(MonoType::Void),
            Operand::Temp(_) => MonoType::Int(64), // TODO: Track temp types
            Operand::Const(c) => match c {
                ConstValue::Int(_) => MonoType::Int(64),
                ConstValue::Float(_) => MonoType::Float(64),
                ConstValue::Bool(_) => MonoType::Bool,
                _ => MonoType::Void,
            },
            _ => MonoType::Void,
        }
    }
}


