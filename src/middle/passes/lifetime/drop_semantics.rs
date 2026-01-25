//! Drop 语义检查
//!
//! 检查 Drop 相关错误：UseAfterDrop、DropMovedValue、DoubleDrop。

use super::error::{OwnershipCheck, OwnershipError, ValueState, operand_to_string};
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// Drop 检查器
///
/// 检测以下错误：
/// - UseAfterDrop: 使用已释放的值
/// - DropMovedValue: 释放已移动的值
/// - DoubleDrop: 双重释放
#[derive(Debug)]
pub struct DropChecker {
    pub state: HashMap<Operand, ValueState>,
    pub errors: Vec<OwnershipError>,
    pub location: (usize, usize),
}

impl DropChecker {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            errors: Vec::new(),
            location: (0, 0),
        }
    }

    fn check_instruction(
        &mut self,
        instr: &Instruction,
    ) {
        match instr {
            Instruction::Drop(value) => self.check_drop(value),
            Instruction::Move { dst, src } => self.check_move(dst, src),
            Instruction::LoadIndex { src, .. }
            | Instruction::LoadField { src, .. }
            | Instruction::Cast { src, .. }
            | Instruction::Ret(Some(src)) => self.check_used(src),
            Instruction::Add { lhs, rhs, .. }
            | Instruction::Sub { lhs, rhs, .. }
            | Instruction::Mul { lhs, rhs, .. }
            | Instruction::Div { lhs, rhs, .. }
            | Instruction::Mod { lhs, rhs, .. } => {
                self.check_used(lhs);
                self.check_used(rhs);
            }
            _ => {}
        }
    }

    fn check_drop(
        &mut self,
        value: &Operand,
    ) {
        match self.state.get(value) {
            Some(ValueState::Moved) => {
                self.errors.push(OwnershipError::DropMovedValue {
                    value: operand_to_string(value),
                });
            }
            Some(ValueState::Dropped) => {
                self.errors.push(OwnershipError::DoubleDrop {
                    value: operand_to_string(value),
                });
            }
            Some(ValueState::Owned) => {
                self.state.insert(value.clone(), ValueState::Dropped);
            }
            None => {
                self.state.insert(value.clone(), ValueState::Dropped);
            }
        }
    }

    fn check_move(
        &mut self,
        dst: &Operand,
        src: &Operand,
    ) {
        self.state.insert(dst.clone(), ValueState::Owned);
        if let Some(ValueState::Owned) = self.state.get(src) {
            self.state.insert(src.clone(), ValueState::Moved);
        } else if !self.state.contains_key(src) {
            self.state.insert(src.clone(), ValueState::Moved);
        }
    }

    fn check_used(
        &mut self,
        operand: &Operand,
    ) {
        if let Some(ValueState::Dropped) = self.state.get(operand) {
            self.errors.push(OwnershipError::UseAfterDrop {
                value: operand_to_string(operand),
                location: self.location,
            });
        }
    }
}

impl OwnershipCheck for DropChecker {
    fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[OwnershipError] {
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

impl Default for DropChecker {
    fn default() -> Self {
        Self::new()
    }
}
