//! 所有权分析错误类型
//!
//! 定义所有权的语义错误，包括 UseAfterMove、UseAfterDrop 等。

use crate::middle::ir::{FunctionIR, Operand};
use std::collections::HashMap;

/// 所有权状态（Move/Drop 检查器共用）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueState {
    /// 有效，所有者可用
    Owned,
    /// 已被移动，所有者不可用
    Moved,
    /// 已被释放（仅 DropChecker 使用）
    Dropped,
}

/// 所有权检查器 Trait
///
/// 提取公共接口，减少 MoveChecker 和 DropChecker 的重复代码。
pub trait OwnershipCheck {
    /// 检查函数的所有权语义
    fn check_function(&mut self, func: &FunctionIR) -> &[OwnershipError];

    /// 获取收集的错误
    fn errors(&self) -> &[OwnershipError];

    /// 获取状态
    fn state(&self) -> &HashMap<Operand, ValueState>;

    /// 清除状态
    fn clear(&mut self);
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OwnershipError {
    /// 使用已移动的值
    UseAfterMove {
        /// 值标识
        value: String,
        /// 发生位置 (block_idx, instr_idx)
        location: (usize, usize),
    },
    /// 使用已释放的值
    UseAfterDrop {
        /// 值标识
        value: String,
        /// 发生位置
        location: (usize, usize),
    },
    /// 释放已移动的值
    DropMovedValue {
        /// 值标识
        value: String,
    },
    /// 双重释放
    DoubleDrop {
        /// 值标识
        value: String,
    },
}

impl std::fmt::Display for OwnershipError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnershipError::UseAfterMove { value, location } => {
                write!(f, "UseAfterMove: value '{}' used after move at {:?}", value, location)
            }
            OwnershipError::UseAfterDrop { value, location } => {
                write!(f, "UseAfterDrop: value '{}' used after drop at {:?}", value, location)
            }
            OwnershipError::DropMovedValue { value } => {
                write!(f, "DropMovedValue: cannot drop value '{}' that has been moved", value)
            }
            OwnershipError::DoubleDrop { value } => {
                write!(f, "DoubleDrop: value '{}' dropped twice", value)
            }
        }
    }
}

/// 将 Operand 转换为字符串标识
pub fn operand_to_string(operand: &Operand) -> String {
    match operand {
        Operand::Local(idx) => format!("local_{}", idx),
        Operand::Arg(idx) => format!("arg_{}", idx),
        Operand::Temp(idx) => format!("temp_{}", idx),
        Operand::Global(idx) => format!("global_{}", idx),
        Operand::Const(c) => format!("const_{:?}", c),
        Operand::Label(idx) => format!("label_{}", idx),
        Operand::Register(idx) => format!("reg_{}", idx),
    }
}
