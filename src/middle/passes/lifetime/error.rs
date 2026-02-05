//! 所有权分析错误类型
//!
//! 定义所有权的语义错误，包括 UseAfterMove、UseAfterDrop 等。

use crate::middle::core::ir::{FunctionIR, Operand};
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
    fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[OwnershipError];

    /// 获取收集的错误
    fn errors(&self) -> &[OwnershipError];

    /// 获取状态
    fn state(&self) -> &HashMap<Operand, ValueState>;

    /// 清除状态
    fn clear(&mut self);
}
/// 所有权检查错误类型
///
/// 包含 Move/Drop/Mut 三种检查的错误。
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
    /// 不可变赋值：对不可变变量进行赋值
    ImmutableAssign {
        /// 值标识
        value: String,
        /// 发生位置
        location: (usize, usize),
    },
    /// 不可变变异：调用不可变对象上的变异方法
    ImmutableMutation {
        /// 值标识
        value: String,
        /// 变异方法名
        method: String,
        /// 发生位置
        location: (usize, usize),
    },
    /// 不可变字段赋值：对不可变字段进行赋值
    ImmutableFieldAssign {
        /// 结构体类型名
        struct_name: String,
        /// 字段名
        field: String,
        /// 发生位置
        location: (usize, usize),
    },
    /// ref 应用于非所有者（已移动或已释放的值）
    RefNonOwner {
        /// ref 表达式位置
        ref_span: (usize, usize),
        /// 目标值位置
        target_span: (usize, usize),
        /// 目标值标识
        target_value: String,
    },
    /// clone 已移动的值
    CloneMovedValue {
        /// 值标识
        value: String,
        /// 发生位置
        location: (usize, usize),
    },
    /// clone 已释放的值
    CloneDroppedValue {
        /// 值标识
        value: String,
        /// 发生位置
        location: (usize, usize),
    },
    /// 非 Send 类型用于跨线程操作
    NotSend {
        /// 值标识
        value: String,
        /// 原因说明
        reason: String,
        /// 发生位置
        location: (usize, usize),
    },
    /// 非 Sync 类型用于跨线程共享
    NotSync {
        /// 值标识
        value: String,
        /// 原因说明
        reason: String,
        /// 发生位置
        location: (usize, usize),
    },
    /// 跨 spawn 循环引用（Task 5.6）
    CrossSpawnCycle {
        /// 详细信息
        details: String,
        /// 发生位置
        span: (usize, usize),
    },
}

impl std::fmt::Display for OwnershipError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            OwnershipError::UseAfterMove { value, location } => {
                write!(
                    f,
                    "UseAfterMove: value '{}' used after move at {:?}",
                    value, location
                )
            }
            OwnershipError::UseAfterDrop { value, location } => {
                write!(
                    f,
                    "UseAfterDrop: value '{}' used after drop at {:?}",
                    value, location
                )
            }
            OwnershipError::DropMovedValue { value } => {
                write!(
                    f,
                    "DropMovedValue: cannot drop value '{}' that has been moved",
                    value
                )
            }
            OwnershipError::DoubleDrop { value } => {
                write!(f, "DoubleDrop: value '{}' dropped twice", value)
            }
            OwnershipError::ImmutableAssign { value, location } => {
                write!(
                    f,
                    "ImmutableAssign: cannot assign to immutable value '{}' at {:?}",
                    value, location
                )
            }
            OwnershipError::ImmutableMutation {
                value,
                method,
                location,
            } => {
                write!(
                    f,
                    "ImmutableMutation: cannot mutate '{}' via method '{}' at {:?}",
                    value, method, location
                )
            }
            OwnershipError::ImmutableFieldAssign {
                struct_name,
                field,
                location,
            } => {
                write!(
                    f,
                    "ImmutableFieldAssign: cannot assign to immutable field '{}.{}' at {:?}",
                    struct_name, field, location
                )
            }
            OwnershipError::RefNonOwner {
                ref_span,
                target_span,
                target_value,
            } => {
                write!(
                    f,
                    "RefNonOwner: cannot create ref for value '{}' at {:?} (target defined at {:?})",
                    target_value, ref_span, target_span
                )
            }
            OwnershipError::CloneMovedValue { value, location } => {
                write!(
                    f,
                    "CloneMovedValue: cannot clone value '{}' that has been moved at {:?}",
                    value, location
                )
            }
            OwnershipError::CloneDroppedValue { value, location } => {
                write!(
                    f,
                    "CloneDroppedValue: cannot clone value '{}' that has been dropped at {:?}",
                    value, location
                )
            }
            OwnershipError::NotSend {
                value,
                reason,
                location,
            } => {
                write!(
                    f,
                    "NotSend: value '{}' cannot be sent between threads: {} at {:?}",
                    value, reason, location
                )
            }
            OwnershipError::NotSync {
                value,
                reason,
                location,
            } => {
                write!(
                    f,
                    "NotSync: value '{}' cannot be shared between threads: {} at {:?}",
                    value, reason, location
                )
            }
            OwnershipError::CrossSpawnCycle { details, span } => {
                write!(f, "CrossSpawnCycle: {} at {:?}", details, span)
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
