//! Drop 语义检查
//!
//! 检查 Drop 相关错误：UseAfterDrop、DropMovedValue、DoubleDrop。

use super::error::{OwnershipError, operand_to_string};
use crate::middle::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// 所有权状态
#[derive(Debug, Clone, PartialEq, Eq)]
enum ValueState {
    /// 有效，所有者可用
    Owned,
    /// 已被移动，所有者不可用
    Moved,
    /// 已被释放
    Dropped,
}

/// Drop 检查器
///
/// 检测以下错误：
/// - UseAfterDrop: 使用已释放的值
/// - DropMovedValue: 释放已移动的值
/// - DoubleDrop: 双重释放
#[derive(Debug)]
pub struct DropChecker {
    /// 每个值的状态
    state: HashMap<Operand, ValueState>,
    /// 收集的错误
    errors: Vec<OwnershipError>,
}

impl DropChecker {
    /// 创建新的 Drop 检查器
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// 检查函数中的 Drop 语义
    pub fn check_function(&mut self, func: &FunctionIR) -> Vec<OwnershipError> {
        // 重置状态
        self.state.clear();
        self.errors.clear();

        // 遍历所有指令
        for (block_idx, block) in func.blocks.iter().enumerate() {
            for (instr_idx, instr) in block.instructions.iter().enumerate() {
                self.check_instruction(instr, block_idx, instr_idx);
            }
        }

        // 检查双重释放
        self.check_double_drop();

        self.errors.clone()
    }

    /// 检查单条指令
    fn check_instruction(
        &mut self,
        instr: &Instruction,
        block_idx: usize,
        instr_idx: usize,
    ) {
        match instr {
            // Drop: 显式释放值
            Instruction::Drop(value) => {
                let value_id = value.clone();
                let value_str = operand_to_string(&value_id);

                match self.state.get(&value_id) {
                    Some(ValueState::Moved) => {
                        // 错误：释放已移动的值
                        self.errors.push(OwnershipError::DropMovedValue {
                            value: value_str,
                        });
                    }
                    Some(ValueState::Dropped) => {
                        // 错误：双重释放
                        self.errors.push(OwnershipError::DoubleDrop {
                            value: value_str,
                        });
                    }
                    Some(ValueState::Owned) => {
                        // 正常释放
                        self.state.insert(value_id, ValueState::Dropped);
                    }
                    None => {
                        // 未跟踪的值：假设是新变量，第一次使用就被释放
                        self.state.insert(value_id, ValueState::Dropped);
                    }
                }
            }

            // Move: 目标值变为 Owned，src 变为 Moved
            Instruction::Move { dst, src } => {
                self.state.insert(dst.clone(), ValueState::Owned);
                // src 变为 Moved（如果之前是 Owned 或未跟踪）
                if let Some(state) = self.state.get(src) {
                    if matches!(state, ValueState::Owned) {
                        self.state.insert(src.clone(), ValueState::Moved);
                    }
                } else {
                    // 未跟踪的值，第一次使用就被移动
                    self.state.insert(src.clone(), ValueState::Moved);
                }
            }

            // LoadIndex: 使用值后不改变状态
            Instruction::LoadIndex { src, .. } => {
                if let Some(state) = self.state.get(src) {
                    if matches!(state, ValueState::Dropped) {
                        self.errors.push(OwnershipError::UseAfterDrop {
                            value: operand_to_string(src),
                            location: (block_idx, instr_idx),
                        });
                    }
                }
            }

            // LoadField: 使用值后不改变状态
            Instruction::LoadField { src, .. } => {
                if let Some(state) = self.state.get(src) {
                    if matches!(state, ValueState::Dropped) {
                        self.errors.push(OwnershipError::UseAfterDrop {
                            value: operand_to_string(src),
                            location: (block_idx, instr_idx),
                        });
                    }
                }
            }

            // Add/Sub/Mul/Div 等二元运算：使用值
            Instruction::Add { lhs, rhs, .. }
            | Instruction::Sub { lhs, rhs, .. }
            | Instruction::Mul { lhs, rhs, .. }
            | Instruction::Div { lhs, rhs, .. }
            | Instruction::Mod { lhs, rhs, .. } => {
                for operand in [lhs, rhs] {
                    if let Some(state) = self.state.get(operand) {
                        if matches!(state, ValueState::Dropped) {
                            self.errors.push(OwnershipError::UseAfterDrop {
                                value: operand_to_string(operand),
                                location: (block_idx, instr_idx),
                            });
                        }
                    }
                }
            }

            // 其他使用值的指令
            Instruction::Cast { src, .. } => {
                if let Some(state) = self.state.get(src) {
                    if matches!(state, ValueState::Dropped) {
                        self.errors.push(OwnershipError::UseAfterDrop {
                            value: operand_to_string(src),
                            location: (block_idx, instr_idx),
                        });
                    }
                }
            }

            // 函数调用：参数在调用后可能被释放
            Instruction::Call { args, .. } => {
                // 参数在函数调用后不再活跃，不需要特别处理
            }

            // Ret: 返回值
            Instruction::Ret(Some(value)) => {
                if let Some(state) = self.state.get(value) {
                    if matches!(state, ValueState::Dropped) {
                        self.errors.push(OwnershipError::UseAfterDrop {
                            value: operand_to_string(value),
                            location: (block_idx, instr_idx),
                        });
                    }
                }
            }

            // 其他指令不影响 Drop 状态
            _ => {}
        }
    }

    /// 检查双重释放
    fn check_double_drop(&mut self) {
        for (value, state) in &self.state {
            if *state == ValueState::Dropped {
                // 检查是否有其他引用指向此值（简化版本）
                // 完整实现需要引用计数分析
            }
        }
    }

    /// 获取收集的错误
    pub fn errors(&self) -> &[OwnershipError] {
        &self.errors
    }
}

impl Default for DropChecker {
    fn default() -> Self {
        Self::new()
    }
}
