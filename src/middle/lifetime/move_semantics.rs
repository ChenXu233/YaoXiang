//! Move 语义检查
//!
//! 检查 UseAfterMove 错误：检测赋值和函数调用后对原值的使用。

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
}

/// Move 检查器
///
/// 检测以下错误：
/// - UseAfterMove: 使用已移动的值
#[derive(Debug)]
pub struct MoveChecker {
    /// 每个值的状态
    state: HashMap<Operand, ValueState>,
    /// 收集的错误
    errors: Vec<OwnershipError>,
}

impl MoveChecker {
    /// 创建新的 Move 检查器
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// 检查函数中的 Move 语义
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
            // Move: dst = src，src 被移动
            Instruction::Move { dst, src } => {
                let src_id = src.clone();
                let src_str = operand_to_string(&src_id);

                // 检查 src 是否已被移动
                if let Some(state) = self.state.get(&src_id) {
                    match state {
                        ValueState::Moved => {
                            self.errors.push(OwnershipError::UseAfterMove {
                                value: src_str,
                                location: (block_idx, instr_idx),
                            });
                            // src 仍然是 Moved（不改变）
                        }
                        ValueState::Owned => {
                            // 正常 Move：标记原值已移动
                            self.state.insert(src_id, ValueState::Moved);
                        }
                    }
                } else {
                    // 首次出现，标记为已拥有（然后立即移动）
                    // 这是一个新的变量，第一次使用就被移动
                    self.state.insert(src_id, ValueState::Moved);
                }

                // 目标值状态 - 拥有所有权
                self.state.insert(dst.clone(), ValueState::Owned);
            }

            // 函数调用：参数移动进函数
            Instruction::Call { args, .. } => {
                for arg in args {
                    let arg_id = arg.clone();
                    let arg_str = operand_to_string(&arg_id);

                    if let Some(state) = self.state.get(&arg_id) {
                        match state {
                            ValueState::Moved => {
                                self.errors.push(OwnershipError::UseAfterMove {
                                    value: arg_str,
                                    location: (block_idx, instr_idx),
                                });
                            }
                            ValueState::Owned => {
                                // 参数移动进函数，标记为已移动
                                self.state.insert(arg_id, ValueState::Moved);
                            }
                        }
                    }
                    // 未跟踪的值：假设是新变量，第一次使用就被移动
                    else {
                        self.state.insert(arg_id, ValueState::Moved);
                    }
                }
            }

            // Ret: 返回值可能移动
            Instruction::Ret(Some(value)) => {
                let value_id = value.clone();
                let value_str = operand_to_string(&value_id);

                if let Some(state) = self.state.get(&value_id) {
                    match state {
                        ValueState::Moved => {
                            self.errors.push(OwnershipError::UseAfterMove {
                                value: value_str,
                                location: (block_idx, instr_idx),
                            });
                        }
                        ValueState::Owned => {
                            self.state.insert(value_id, ValueState::Moved);
                        }
                    }
                }
                // 未跟踪的值：假设是新变量
            }

            // LoadIndex: 加载元素，容器可能被移动
            Instruction::LoadIndex { src, .. } => {
                let src_id = src.clone();
                if let Some(state) = self.state.get(&src_id) {
                    if matches!(state, ValueState::Moved) {
                        self.errors.push(OwnershipError::UseAfterMove {
                            value: operand_to_string(&src_id),
                            location: (block_idx, instr_idx),
                        });
                    }
                    // Loaded values don't change ownership state
                }
                // 未跟踪的值：假设是新变量，不需要报错
            }

            // LoadField: 加载字段
            Instruction::LoadField { src, .. } => {
                let src_id = src.clone();
                if let Some(state) = self.state.get(&src_id) {
                    if matches!(state, ValueState::Moved) {
                        self.errors.push(OwnershipError::UseAfterMove {
                            value: operand_to_string(&src_id),
                            location: (block_idx, instr_idx),
                        });
                    }
                }
                // 未跟踪的值：假设是新变量
            }

            // Store/StoreIndex/StoreField: 存储不涉及 Move
            Instruction::Store { .. }
            | Instruction::StoreIndex { .. }
            | Instruction::StoreField { .. } => {}

            // 二元运算：检查操作数是否被移动
            Instruction::Add { lhs, rhs, .. }
            | Instruction::Sub { lhs, rhs, .. }
            | Instruction::Mul { lhs, rhs, .. }
            | Instruction::Div { lhs, rhs, .. }
            | Instruction::Mod { lhs, rhs, .. } => {
                for operand in [lhs, rhs] {
                    if let Some(state) = self.state.get(operand) {
                        if matches!(state, ValueState::Moved) {
                            self.errors.push(OwnershipError::UseAfterMove {
                                value: operand_to_string(operand),
                                location: (block_idx, instr_idx),
                            });
                        }
                    }
                }
            }

            // 一元运算：检查操作数
            Instruction::Neg { src, .. } => {
                if let Some(state) = self.state.get(src) {
                    if matches!(state, ValueState::Moved) {
                        self.errors.push(OwnershipError::UseAfterMove {
                            value: operand_to_string(src),
                            location: (block_idx, instr_idx),
                        });
                    }
                }
            }

            // 比较运算：检查操作数
            Instruction::Eq { lhs, rhs, .. }
            | Instruction::Ne { lhs, rhs, .. }
            | Instruction::Lt { lhs, rhs, .. }
            | Instruction::Le { lhs, rhs, .. }
            | Instruction::Gt { lhs, rhs, .. }
            | Instruction::Ge { lhs, rhs, .. } => {
                for operand in [lhs, rhs] {
                    if let Some(state) = self.state.get(operand) {
                        if matches!(state, ValueState::Moved) {
                            self.errors.push(OwnershipError::UseAfterMove {
                                value: operand_to_string(operand),
                                location: (block_idx, instr_idx),
                            });
                        }
                    }
                }
            }

            // Cast: 检查源值
            Instruction::Cast { src, .. } => {
                if let Some(state) = self.state.get(src) {
                    if matches!(state, ValueState::Moved) {
                        self.errors.push(OwnershipError::UseAfterMove {
                            value: operand_to_string(src),
                            location: (block_idx, instr_idx),
                        });
                    }
                }
            }

            // 其他指令不影响 Move 状态
            _ => {}
        }
    }

    /// 获取收集的错误
    pub fn errors(&self) -> &[OwnershipError] {
        &self.errors
    }
}

impl Default for MoveChecker {
    fn default() -> Self {
        Self::new()
    }
}
