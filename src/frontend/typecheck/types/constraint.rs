//! 类型约束定义
//!
//! 实现类型系统中的约束：
//! - TypeConstraint: 类型约束
//! - SendSyncConstraint: Send/Sync 约束
//! - SendSyncSolver: Send/Sync 约束求解器

use super::mono::MonoType;
use crate::util::span::Span;
use std::collections::HashMap;

/// 类型约束
///
/// 在类型推断过程中收集的约束条件
#[derive(Debug, Clone)]
pub struct TypeConstraint {
    /// 约束的左边
    pub left: MonoType,
    /// 约束的右边
    pub right: MonoType,
    /// 约束的来源位置
    pub span: Span,
}

impl TypeConstraint {
    /// 创建新的类型约束
    pub fn new(
        left: MonoType,
        right: MonoType,
        span: Span,
    ) -> Self {
        TypeConstraint { left, right, span }
    }
}

/// Send/Sync 约束
///
/// 用于标记类型变量必须满足的 Send/Sync 约束：
/// - Send: 类型可以安全地跨线程传输
/// - Sync: 类型可以安全地跨线程共享
///
/// 根据 RFC-009：
/// - 值类型默认 Send + Sync
/// - ref T (Arc) 默认 Send + Sync
/// - *T (裸指针) 既不是 Send 也不是 Sync
/// - Rc[T] 既不是 Send 也不是 Sync
#[derive(Debug, Clone, PartialEq, Eq, Default, Hash)]
pub struct SendSyncConstraint {
    /// 是否必须 Send
    pub require_send: bool,
    /// 是否必须 Sync
    pub require_sync: bool,
}

impl SendSyncConstraint {
    /// 创建新的约束
    pub fn new(
        require_send: bool,
        require_sync: bool,
    ) -> Self {
        Self {
            require_send,
            require_sync,
        }
    }

    /// 只有 Send 约束
    pub fn send_only() -> Self {
        Self {
            require_send: true,
            require_sync: false,
        }
    }

    /// Send + Sync 约束
    pub fn send_sync() -> Self {
        Self {
            require_send: true,
            require_sync: true,
        }
    }

    /// 只有 Sync 约束
    pub fn sync_only() -> Self {
        Self {
            require_send: false,
            require_sync: true,
        }
    }

    /// 无约束
    pub fn none() -> Self {
        Self::default()
    }

    /// 合并两个约束（取并集）
    pub fn merge(
        &self,
        other: &Self,
    ) -> Self {
        Self {
            require_send: self.require_send || other.require_send,
            require_sync: self.require_sync || other.require_sync,
        }
    }

    /// 检查约束是否满足给定要求
    pub fn is_satisfied(
        &self,
        is_send: bool,
        is_sync: bool,
    ) -> bool {
        (self.require_send || !is_send) && (self.require_sync || !is_sync)
    }
}

/// Send/Sync 约束求解器
///
/// 负责管理类型变量的 Send/Sync 约束收集和求解
#[derive(Debug, Default)]
pub struct SendSyncSolver {
    /// 类型变量的 Send/Sync 约束
    constraints: HashMap<usize, SendSyncConstraint>,
}

impl SendSyncSolver {
    /// 创建新的求解器
    pub fn new() -> Self {
        SendSyncSolver {
            constraints: HashMap::new(),
        }
    }

    /// 添加约束到类型
    pub fn add_constraint(
        &mut self,
        ty: &MonoType,
        require_send: bool,
        require_sync: bool,
    ) {
        let constraint = SendSyncConstraint::new(require_send, require_sync);

        match ty {
            MonoType::TypeVar(v) => {
                // 为类型变量添加约束
                self.constraints
                    .entry(v.index())
                    .and_modify(|c| *c = c.merge(&constraint))
                    .or_insert(constraint);
            }
            MonoType::Struct(_) | MonoType::Enum(_) => {
                // 结构体/枚举需要检查字段或变体的约束
                // TODO: 需要更复杂的实现
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                // 函数：约束传播到参数和返回类型
                for param_ty in params {
                    self.add_constraint(param_ty, require_send, require_sync);
                }
                self.add_constraint(return_type, require_send, require_sync);
            }
            MonoType::Union(types) | MonoType::Intersection(types) => {
                // 联合/交集：约束传播到所有成员类型
                for member_ty in types {
                    self.add_constraint(member_ty, require_send, require_sync);
                }
            }
            MonoType::Arc(inner) => {
                // Arc 内部类型需要满足约束（因为 Arc 可以跨线程共享）
                self.add_constraint(inner, require_send, require_sync);
            }
            // 基本类型、类型引用等不需要额外处理
            // 它们默认满足 Send/Sync
            _ => {}
        }
    }

    /// 获取类型的 Send/Sync 约束
    pub fn get_constraint(
        &self,
        ty: &MonoType,
    ) -> SendSyncConstraint {
        match ty {
            MonoType::TypeVar(v) => self
                .constraints
                .get(&v.index())
                .cloned()
                .unwrap_or_default(),
            _ => SendSyncConstraint::none(),
        }
    }

    /// 检查类型是否满足 Send 约束
    pub fn is_send(
        &self,
        ty: &MonoType,
    ) -> bool {
        let constraint = self.get_constraint(ty);
        constraint.require_send || self.is_type_inherently_send(ty)
    }

    /// 检查类型是否满足 Sync 约束
    pub fn is_sync(
        &self,
        ty: &MonoType,
    ) -> bool {
        let constraint = self.get_constraint(ty);
        constraint.require_sync || self.is_type_inherently_sync(ty)
    }

    /// 检查类型是否固有地满足 Send（RFC-009）
    ///
    /// 根据 RFC-009：
    /// - 值类型（Int, Float, Bool, Char, String, Bytes）默认 Send
    /// - ref T (Arc) 默认 Send
    /// - Arc[T] 默认 Send
    /// - Rc[T] 不是 Send
    /// - *T 不是 Send
    fn is_type_inherently_send(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            // 基本类型默认 Send
            MonoType::Void
            | MonoType::Bool
            | MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes => true,
            // Arc 默认 Send
            MonoType::Arc(_) => true,
            // 枚举默认 Send（只是标签）
            MonoType::Enum(_) => true,
            // 联合/交集类型需要所有成员 Send
            MonoType::Union(types) | MonoType::Intersection(types) => {
                types.iter().all(|t| self.is_send(t))
            }
            // 结构体需要所有字段 Send
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.is_send(f)),
            // 元组需要所有元素 Send
            MonoType::Tuple(types) => types.iter().all(|t| self.is_send(t)),
            // 列表/集合/Range 需要元素 Send
            MonoType::List(elem) | MonoType::Set(elem) | MonoType::Range { elem_type: elem } => {
                self.is_send(elem)
            }
            // 字典需要键和值 Send
            MonoType::Dict(k, v) => self.is_send(k) && self.is_send(v),
            // 函数类型需要参数和返回类型 Send
            MonoType::Fn {
                params,
                return_type,
                ..
            } => params.iter().all(|p| self.is_send(p)) && self.is_send(return_type),
            // Rc 不是 Send（无法跨线程安全共享引用计数）
            MonoType::TypeRef(name) if name.starts_with("Rc") => false,
            // 其他类型引用保守假设为 Send
            MonoType::TypeRef(_) => true,
            // 类型变量需要根据约束判断
            MonoType::TypeVar(_) => false,
            // 其他类型默认不是 Send
            _ => false,
        }
    }

    /// 检查类型是否固有地满足 Sync（RFC-009）
    ///
    /// 根据 RFC-009：
    /// - 值类型默认 Sync
    /// - ref T (Arc) 默认 Sync
    /// - Arc[T] 默认 Sync
    /// - Rc[T] 不是 Sync
    /// - *T 不是 Sync
    fn is_type_inherently_sync(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            // 基本类型默认 Sync
            MonoType::Void
            | MonoType::Bool
            | MonoType::Int(_)
            | MonoType::Float(_)
            | MonoType::Char
            | MonoType::String
            | MonoType::Bytes => true,
            // Arc 默认 Sync
            MonoType::Arc(_) => true,
            // 枚举默认 Sync（只是标签）
            MonoType::Enum(_) => true,
            // 联合/交集类型需要所有成员 Sync
            MonoType::Union(types) | MonoType::Intersection(types) => {
                types.iter().all(|t| self.is_sync(t))
            }
            // 结构体需要所有字段 Sync
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.is_sync(f)),
            // 元组需要所有元素 Sync
            MonoType::Tuple(types) => types.iter().all(|t| self.is_sync(t)),
            // 列表/集合/Range 需要元素 Sync
            MonoType::List(elem) | MonoType::Set(elem) | MonoType::Range { elem_type: elem } => {
                self.is_sync(elem)
            }
            // 字典需要键和值 Sync
            MonoType::Dict(k, v) => self.is_sync(k) && self.is_sync(v),
            // 函数类型需要参数和返回类型 Sync
            MonoType::Fn {
                params,
                return_type,
                ..
            } => params.iter().all(|p| self.is_sync(p)) && self.is_sync(return_type),
            // Rc 不是 Sync（无法跨线程安全共享引用计数）
            MonoType::TypeRef(name) if name.starts_with("Rc") => false,
            // 其他类型引用保守假设为 Sync
            MonoType::TypeRef(_) => true,
            // 类型变量需要根据约束判断
            MonoType::TypeVar(_) => false,
            // 其他类型默认不是 Sync
            _ => false,
        }
    }

    /// 添加 Send 约束
    pub fn add_send_constraint(
        &mut self,
        ty: &MonoType,
    ) {
        self.add_constraint(ty, true, false);
    }

    /// 添加 Sync 约束
    pub fn add_sync_constraint(
        &mut self,
        ty: &MonoType,
    ) {
        self.add_constraint(ty, false, true);
    }

    /// 获取所有约束（用于测试）
    pub fn constraints(&self) -> &HashMap<usize, SendSyncConstraint> {
        &self.constraints
    }
}
