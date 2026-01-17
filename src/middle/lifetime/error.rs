//! 所有权分析错误类型
//!
//! 定义所有权的语义错误，包括 UseAfterMove、UseAfterDrop 等。

use crate::middle::ir::Operand;

/// 所有权错误类型
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
