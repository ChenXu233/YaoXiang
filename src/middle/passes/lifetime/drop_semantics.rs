//! Drop 语义检查
//!
//! 检查 Drop 相关错误：UseAfterDrop、DropMovedValue、DoubleDrop。

use super::error::{Checker, ValueStateProvider, ValueState, operand_display_name};
use crate::middle::core::ir::{FunctionIR, Instruction, Operand};
use crate::util::diagnostic::{ErrorCodeDefinition, Diagnostic};
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
    pub errors: Vec<Diagnostic>,
    pub location: (usize, usize),
    /// 局部变量名列表（用于错误报告中显示源码变量名）
    local_names: Option<Vec<String>>,
}

impl DropChecker {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            errors: Vec::new(),
            location: (0, 0),
            local_names: None,
        }
    }

    /// 设置局部变量名列表
    pub fn set_local_names(
        &mut self,
        local_names: Option<Vec<String>>,
    ) {
        self.local_names = local_names;
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
                let name = operand_display_name(value, self.local_names.as_ref());
                self.errors
                    .push(ErrorCodeDefinition::drop_moved_value(&name).build());
            }
            Some(ValueState::Dropped) => {
                let name = operand_display_name(value, self.local_names.as_ref());
                self.errors
                    .push(ErrorCodeDefinition::double_drop(&name).build());
            }
            Some(ValueState::Owned(_)) => {
                self.state.insert(value.clone(), ValueState::Dropped);
            }
            Some(ValueState::Dup) => {
                // Dup 类型（如 &T）不会被 Drop，保持 Dup 状态
            }
            Some(ValueState::Empty) => {
                // Empty 状态的值也可以被 Drop，保持 Empty
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
        self.state.insert(dst.clone(), ValueState::Owned(None));
        if let Some(ValueState::Owned(_)) = self.state.get(src) {
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
            let name = operand_display_name(operand, self.local_names.as_ref());
            self.errors
                .push(ErrorCodeDefinition::use_after_drop(&name).build());
        }
    }
}

impl Checker for DropChecker {
    fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[Diagnostic] {
        self.clear();

        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.location = (block_idx, instr_idx);
                self.check_instruction(instr);
            }
        }

        &self.errors
    }

    fn errors(&self) -> &[Diagnostic] {
        &self.errors
    }

    fn clear(&mut self) {
        self.state.clear();
        self.errors.clear();
    }
}

impl ValueStateProvider for DropChecker {
    fn state(&self) -> &HashMap<Operand, ValueState> {
        &self.state
    }
}

impl Default for DropChecker {
    fn default() -> Self {
        Self::new()
    }
}
