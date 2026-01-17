//! ref 语义检查
//!
//! 检查 ref 表达式的所有权语义：
//! - ref 只能应用于有效的所有者（不能是已移动或已释放的值）

use super::error::{OwnershipCheck, OwnershipError, ValueState};
use crate::middle::ir::{FunctionIR, Instruction, Operand};
use std::collections::HashMap;

/// Ref 检查器
///
/// 检测以下错误：
/// - RefNonOwner: ref 应用于已移动或已释放的值
#[derive(Debug)]
pub struct RefChecker {
    /// 值状态追踪
    state: HashMap<Operand, ValueState>,
    /// 定义位置追踪
    definitions: HashMap<Operand, (usize, usize)>,
    /// 收集的错误
    errors: Vec<OwnershipError>,
    /// 当前位置 (block_idx, instr_idx)
    location: (usize, usize),
}

impl RefChecker {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            definitions: HashMap::new(),
            errors: Vec::new(),
            location: (0, 0),
        }
    }

    fn check_instruction(
        &mut self,
        instr: &Instruction,
    ) {
        match instr {
            Instruction::Move { dst, src } => {
                // src 被移动
                self.state.insert(src.clone(), ValueState::Moved);
                // dst 成为新所有者
                self.state.insert(dst.clone(), ValueState::Owned);
                // 记录 dst 的定义位置
                self.definitions.insert(dst.clone(), self.location);
            }
            Instruction::Call { args, dst, .. } => {
                // 参数被移动（所有权转移给函数）
                for arg in args {
                    self.state.insert(arg.clone(), ValueState::Moved);
                }
                // 如果有返回值，dst 成为新所有者
                if let Some(d) = dst {
                    self.state.insert(d.clone(), ValueState::Owned);
                    self.definitions.insert(d.clone(), self.location);
                }
            }
            Instruction::Ret(Some(value)) => {
                // 返回值被移动
                self.state.insert(value.clone(), ValueState::Moved);
            }
            Instruction::Drop(operand) => {
                // 标记为已释放
                self.state.insert(operand.clone(), ValueState::Dropped);
            }
            Instruction::HeapAlloc { dst, .. } => {
                // 新分配的值是有效的所有者
                self.state.insert(dst.clone(), ValueState::Owned);
                self.definitions.insert(dst.clone(), self.location);
            }
            Instruction::MakeClosure { dst, env, .. } => {
                // 闭包捕获的环境变量被移动
                for var in env {
                    self.state.insert(var.clone(), ValueState::Moved);
                }
                // dst 是新所有者
                self.state.insert(dst.clone(), ValueState::Owned);
                self.definitions.insert(dst.clone(), self.location);
            }
            // ArcNew: 创建 Arc，不影响原值的状态（原值仍有效）
            Instruction::ArcNew { dst, .. } => {
                self.state.insert(dst.clone(), ValueState::Owned);
                self.definitions.insert(dst.clone(), self.location);
            }
            // ArcClone: 克隆 Arc，不影响原值的状态
            Instruction::ArcClone { dst, .. } => {
                self.state.insert(dst.clone(), ValueState::Owned);
                self.definitions.insert(dst.clone(), self.location);
            }
            // ArcDrop: 释放 Arc，不影响原值的状态
            Instruction::ArcDrop(_) => {
                // ArcDrop 不改变底层值的状态，只改变引用计数
            }
            _ => {}
        }
    }
}

impl OwnershipCheck for RefChecker {
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
        self.definitions.clear();
        self.errors.clear();
    }
}

impl Default for RefChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl RefChecker {
    /// 获取收集的错误
    pub fn errors(&self) -> &[OwnershipError] {
        &self.errors
    }
}
