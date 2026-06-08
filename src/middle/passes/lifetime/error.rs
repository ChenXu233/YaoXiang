//! 所有权分析错误类型
//!
//! 定义所有权的语义错误，包括 UseAfterMove、UseAfterDrop 等。
//! 统一使用 Diagnostic 错误码系统。

use crate::middle::core::ir::{FunctionIR, Operand};
use crate::util::diagnostic::{Diagnostic, ErrorCodeDefinition};
use std::collections::HashMap;

/// 所有权状态（Move/Drop 检查器共用）
///
/// # 状态转换图
///
/// ```text
///     ┌──────────────────────────────────────────────────────┐
///     │                                                      │
///     │    ┌─────────┐    Move     ┌─────────┐               │
///     │    │  Owned  │ ──────────► │  Moved  │               │
///     │    └─────────┘             └─────────┘               │
///     │         │                       │                    │
///     │         │ Store                 │                    │
///     │         ▼                       │                    │
///     │    ┌─────────┐                  │                    │
///     │    │  Empty  │ ◄────────────────┘                    │
///     │    └─────────┘    (仅当 Moved 时)                     │
///     │         │                                            │
///     │         │ Store (类型一致)                            │
///     │         ▼                                            │
///     │    ┌─────────┐    Drop     ┌─────────┐               │
///     │    │  Owned  │ ──────────► │ Dropped │               │
///     │    └─────────┘             └─────────┘               │
///     └──────────────────────────────────────────────────────┘
/// ```
///
/// # 状态语义
///
/// - **Owned**: 值有效，所有者可用。可以被使用或 Move。
/// - **Moved**: 值已被移动，所有者不可用。必须重新赋值才能使用。
/// - **Empty**: 值处于空状态（仅在 Moved 后触发）。可以重新赋值复用变量。
/// - **Dropped**: 值已被释放（仅 DropChecker 使用）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueState {
    /// 有效，所有者可用，可选携带类型信息用于重赋值检查
    Owned(Option<TypeId>),
    /// 已被移动，所有者不可用
    Moved,
    /// 空状态，可重新赋值（Move 后进入）
    Empty,
    /// 已被释放（仅 DropChecker 使用）
    Dropped,
}

/// 类型标识符（用于空状态重赋值时的类型检查）
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TypeId(pub String);

/// 所有权检查器 Trait
///
/// 提取公共接口，减少 MoveChecker 和 DropChecker 的重复代码。
pub trait OwnershipCheck {
    /// 检查函数的所有权语义
    fn check_function(
        &mut self,
        func: &FunctionIR,
    ) -> &[Diagnostic];

    /// 获取收集的错误
    fn errors(&self) -> &[Diagnostic];

    /// 获取状态
    fn state(&self) -> &HashMap<Operand, ValueState>;

    /// 清除状态
    fn clear(&mut self);
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

/// 获取操作数的用户可见名称
///
/// 优先使用源码变量名（local_names），回退到内部名（operand_to_string）。
/// 这确保错误信息显示 `'p' has been moved` 而不是 `'local_0' has been moved`。
pub fn operand_display_name(
    operand: &Operand,
    local_names: Option<&Vec<String>>,
) -> String {
    if let Operand::Local(idx) = operand {
        if let Some(names) = local_names {
            if *idx < names.len() && !names[*idx].is_empty() {
                return names[*idx].clone();
            }
        }
    }
    operand_to_string(operand)
}

/// OwnershipError 变体到错误码的映射辅助函数
///
/// 用于将旧的 OwnershipError 语义转换为统一的 Diagnostic 错误码。
pub mod codes {
    use super::*;

    pub fn use_after_move(value: &str) -> Diagnostic {
        ErrorCodeDefinition::use_after_move(value).build()
    }

    pub fn use_after_drop(value: &str) -> Diagnostic {
        ErrorCodeDefinition::use_after_drop(value).build()
    }

    pub fn drop_moved_value(value: &str) -> Diagnostic {
        ErrorCodeDefinition::drop_moved_value(value).build()
    }

    pub fn double_drop(value: &str) -> Diagnostic {
        ErrorCodeDefinition::double_drop(value).build()
    }

    pub fn immutable_assign(value: &str) -> Diagnostic {
        ErrorCodeDefinition::immutable_assign(value).build()
    }

    pub fn immutable_mutation(
        value: &str,
        method: &str,
    ) -> Diagnostic {
        ErrorCodeDefinition::immutable_mutation(value, method).build()
    }

    pub fn immutable_field_assign(
        struct_name: &str,
        field: &str,
    ) -> Diagnostic {
        ErrorCodeDefinition::immutable_field_assign(struct_name, field).build()
    }

    pub fn ref_non_owner(target_value: &str) -> Diagnostic {
        ErrorCodeDefinition::ref_non_owner(target_value).build()
    }

    pub fn clone_moved_value(value: &str) -> Diagnostic {
        ErrorCodeDefinition::clone_moved_value(value).build()
    }

    pub fn cross_spawn_cycle(details: &str) -> Diagnostic {
        ErrorCodeDefinition::ownership_violation(details).build()
    }

    pub fn reassign_non_empty(value: &str) -> Diagnostic {
        ErrorCodeDefinition::reassign_non_empty(value).build()
    }

    pub fn consumed_not_returned(param: &str) -> Diagnostic {
        ErrorCodeDefinition::consumed_not_returned(param).build()
    }

    pub fn intra_task_cycle(details: &str) -> Diagnostic {
        ErrorCodeDefinition::ownership_violation(details).build()
    }

    pub fn unsafe_bypass_cycle(details: &str) -> Diagnostic {
        ErrorCodeDefinition::ownership_violation(details).build()
    }

    pub fn unsafe_deref() -> Diagnostic {
        ErrorCodeDefinition::unsafe_deref().build()
    }

    pub fn mutable_borrow_conflict(source: &str) -> Diagnostic {
        ErrorCodeDefinition::mutable_borrow_conflict(source).build()
    }

    pub fn borrow_after_move(source: &str) -> Diagnostic {
        ErrorCodeDefinition::borrow_after_move(source).build()
    }

    pub fn use_while_frozen(source: &str) -> Diagnostic {
        ErrorCodeDefinition::mutable_immutable_borrow_conflict(source).build()
    }
}
