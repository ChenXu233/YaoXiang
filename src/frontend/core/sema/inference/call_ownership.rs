//! 调用点所有权解析表（#251 0.8.0 门槛 G3：类型信息流接口）
//!
//! 推断层在 Call 解析完成点（已持有单态化后的函数类型）记录「该调用每个
//! 实参/接收者的所有权」，按调用 span 键控；ownership 层直接查表消费，
//! 取代跨层反查签名。Ref 拆解逻辑（own_of）留在推断层——单态化结果只在
//! 推断层可见（#335 路径 A 修复的 TypeVar 失明问题由此根治）。

use crate::frontend::core::types::MonoType;
use crate::util::span::Span;
use std::collections::HashMap;

/// 参数/接收者所有权（推断层解析结果，ownership 层消费）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamOwnership {
    /// 按值移动
    Move,
    /// 不可变借用
    ReadBorrow,
    /// 可变借用
    WriteBorrow,
}

impl ParamOwnership {
    /// 从签名参数类型推导：`&T`→Read、`&mut T`→Write、其余→Move
    pub fn from_param_type(ty: &MonoType) -> Self {
        match ty {
            MonoType::Ref {
                mutable: true,
                inner: _,
            } => ParamOwnership::WriteBorrow,
            MonoType::Ref { mutable: false, .. } => ParamOwnership::ReadBorrow,
            _ => ParamOwnership::Move,
        }
    }
}

/// 一次调用的所有权解析结果
#[derive(Debug, Clone, Default)]
pub struct CallOwnership {
    /// 与实参按位对齐的所有权
    pub args: Vec<ParamOwnership>,
    /// 方法接收者的所有权（impl 绑定方法 params[0]；std 容器方法的首实参）
    pub receiver: Option<ParamOwnership>,
}

/// 调用 span → 所有权解析结果
pub type CallOwnershipTable = HashMap<Span, CallOwnership>;

/// std 容器「可变方法」名单（首实参 = 容器，方法原地修改容器内容）。
/// 未列出的 std 方法缺省按只读借用处理（保守：宁漏检不误报，校准阶段修正）。
const STD_MUTATING_METHODS: &[&str] = &[
    "push",
    "pop",
    "insert",
    "remove",
    "clear",
    "set",
    "append",
    "retain",
    "swap_remove",
];

/// std 容器方法的接收者所有权：可变方法 → WriteBorrow（写令牌与活跃借用
/// 冲突，RFC-009 §2.8「只读自动借用」的对偶）；未知名缺省 ReadBorrow。
/// `method` 为方法名字段（如 `push`）；调用形如 `list.push(x)`，首实参是容器。
pub fn std_native_receiver_ownership(method: &str) -> ParamOwnership {
    if STD_MUTATING_METHODS.contains(&method) {
        ParamOwnership::WriteBorrow
    } else {
        ParamOwnership::ReadBorrow
    }
}
