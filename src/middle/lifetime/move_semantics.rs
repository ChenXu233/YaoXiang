//! Move 语义检查
//!
//! 检查 UseAfterMove 错误：检测赋值和函数调用后对原值的使用。

use super::error::{OwnershipCheck, OwnershipError, ValueState, operand_to_string};
use crate::middle::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// Move 检查器
///
/// 检测以下错误：
/// - UseAfterMove: 使用已移动的值
#[derive(Debug)]
pub struct MoveChecker {
    pub state: HashMap<Operand, ValueState>,
    pub errors: Vec<OwnershipError>,
    pub location: (usize, usize),
}

impl MoveChecker {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            errors: Vec::new(),
            location: (0, 0),
        }
    }

    fn check_instruction(&mut self, instr: &Instruction) {
        match instr {
            Instruction::Move { dst, src } => self.check_move(dst, src),
            Instruction::Call { args, .. } => self.check_call(args),
            Instruction::Ret(Some(value)) => self.check_ret(value),
            Instruction::LoadIndex { src, .. }
            | Instruction::LoadField { src, .. }
            | Instruction::Neg { src, .. }
            | Instruction::Cast { src, .. } => self.check_used(src),
            Instruction::Add { lhs, rhs, .. }
            | Instruction::Sub { lhs, rhs, .. }
            | Instruction::Mul { lhs, rhs, .. }
            | Instruction::Div { lhs, rhs, .. }
            | Instruction::Mod { lhs, rhs, .. } => {
                self.check_used(lhs);
                self.check_used(rhs);
            }
            Instruction::Eq { lhs, rhs, .. }
            | Instruction::Ne { lhs, rhs, .. }
            | Instruction::Lt { lhs, rhs, .. }
            | Instruction::Le { lhs, rhs, .. }
            | Instruction::Gt { lhs, rhs, .. }
            | Instruction::Ge { lhs, rhs, .. } => {
                self.check_used(lhs);
                self.check_used(rhs);
            }
            _ => {}
        }
    }

    fn check_move(&mut self, dst: &Operand, src: &Operand) {
        if let Some(state) = self.state.get(src) {
            if *state == ValueState::Moved {
                self.report_use_after_move(src);
            } else {
                self.state.insert(src.clone(), ValueState::Moved);
            }
        } else {
            self.state.insert(src.clone(), ValueState::Moved);
        }
        self.state.insert(dst.clone(), ValueState::Owned);
    }

    fn check_call(&mut self, args: &[Operand]) {
        for arg in args {
            if let Some(state) = self.state.get(arg) {
                if *state == ValueState::Moved {
                    self.report_use_after_move(arg);
                } else {
                    self.state.insert(arg.clone(), ValueState::Moved);
                }
            } else {
                self.state.insert(arg.clone(), ValueState::Moved);
            }
        }
    }

    fn check_ret(&mut self, value: &Operand) {
        if let Some(state) = self.state.get(value) {
            if *state == ValueState::Moved {
                // 已移动的值被返回是合法的
            } else {
                self.state.insert(value.clone(), ValueState::Moved);
            }
        }
    }

    fn check_used(&mut self, operand: &Operand) {
        if let Some(state) = self.state.get(operand) {
            if *state == ValueState::Moved {
                self.report_use_after_move(operand);
            }
        }
    }

    fn report_use_after_move(&mut self, operand: &Operand) {
        self.errors.push(OwnershipError::UseAfterMove {
            value: operand_to_string(operand),
            location: self.location,
        });
    }
}

impl OwnershipCheck for MoveChecker {
    fn check_function(&mut self, func: &FunctionIR) -> &[OwnershipError] {
        self.clear();

        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.location = (block_idx, instr_idx);
                self.check_instruction(instr);
            }
        }

        &self.errors
    }

    fn errors(&self) -> &[OwnershipError] {
        &self.errors
    }

    fn state(&self) -> &HashMap<Operand, ValueState> {
        &self.state
    }

    fn clear(&mut self) {
        self.state.clear();
        self.errors.clear();
    }
}

impl Default for MoveChecker {
    fn default() -> Self {
        Self::new()
    }
}
