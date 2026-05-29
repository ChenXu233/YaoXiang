//! Send/Sync 约束检查
//!
//! 检查类型是否满足 Send/Sync 约束，确保并发安全：
//! - Send: 类型可以安全地跨线程传输（值传递）
//! - Sync: 类型可以安全地跨线程共享引用
//!
//! 设计原则：
//! 1. YaoXiang 优先使用值传递，Sync 很少需要
//! 2. 基本类型自动满足 Send + Sync
//! 3. Arc 自动满足 Send + Sync
//! 4. Rc 既不是 Send 也不是 Sync
//!
//! 注意：此模块不再被 OwnershipChecker 使用，
//! 但保留给 mono/constraint.rs 的约束传播使用。

use crate::frontend::core::typecheck::MonoType;

/// Send/Sync 检查器
///
/// 检测类型是否满足 Send/Sync 约束。
/// 主要用于约束传播阶段的类型检查。
#[derive(Debug)]
pub struct SendSyncChecker {
    // 闭包定义映射保留用于未来扩展
}

impl SendSyncChecker {
    /// 创建新的 Send/Sync 检查器
    pub fn new() -> Self {
        Self {}
    }

    /// 检查类型是否 Send
    #[allow(clippy::only_used_in_recursion)]
    pub fn is_send(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            // 基本类型总是 Send
            MonoType::Void => true,
            MonoType::Bool => true,
            MonoType::Int(_) => true,
            MonoType::Float(_) => true,
            MonoType::Char => true,
            MonoType::String => true,
            MonoType::Bytes => true,

            // 列表、字典、集合：元素类型必须 Send
            MonoType::List(elem) => self.is_send(elem),
            MonoType::Dict(key, value) => self.is_send(key) && self.is_send(value),
            MonoType::Set(elem) => self.is_send(elem),

            // Option：内部类型必须 Send
            MonoType::Option(inner) => self.is_send(inner),

            // Result：Ok/Err 必须 Send
            MonoType::Result(ok, err) => self.is_send(ok) && self.is_send(err),

            // 元组：所有元素必须 Send
            MonoType::Tuple(types) => types.iter().all(|t| self.is_send(t)),

            // 函数类型：参数和返回类型必须 Send
            MonoType::Fn {
                params,
                return_type,
                ..
            } => params.iter().all(|p| self.is_send(p)) && self.is_send(return_type),

            // Arc: 总是 Send（原子引用计数）
            MonoType::Arc(inner) => self.is_send(inner),

            // Weak: 基于 Arc，总是 Send
            MonoType::Weak(inner) => self.is_send(inner),

            // Range: 元素类型必须 Send
            MonoType::Range { elem_type } => self.is_send(elem_type),

            // 联合/交集类型：所有成员必须 Send
            MonoType::Union(types) => types.iter().all(|t| self.is_send(t)),
            MonoType::Intersection(types) => types.iter().all(|t| self.is_send(t)),

            // 结构体：所有字段必须 Send
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.is_send(f)),

            // 枚举：所有变体必须 Send
            MonoType::Enum(_) => true, // 枚举只是标签，视为 Send

            // 类型变量和类型引用：保守假设为 Send（类型检查已验证）
            MonoType::TypeVar(_) => true,
            MonoType::TypeRef(_) => true,
            // 关联类型需要检查宿主类型
            MonoType::AssocType { host_type, .. } => self.is_send(host_type),
            // 字面量类型：检查基础类型
            MonoType::Literal { base_type, .. } => self.is_send(base_type),
            // 元类型：编译期概念，总是 Send
            MonoType::MetaType { .. } => true,
        }
    }

    /// 检查类型是否 Sync
    #[allow(clippy::only_used_in_recursion)]
    pub fn is_sync(
        &self,
        ty: &MonoType,
    ) -> bool {
        match ty {
            // 基本类型总是 Sync
            MonoType::Void => true,
            MonoType::Bool => true,
            MonoType::Int(_) => true,
            MonoType::Float(_) => true,
            MonoType::Char => true,
            MonoType::String => true,
            MonoType::Bytes => true,

            // 列表：必须是 RefCell[T] 或类似包装才有意义，保守返回 false
            MonoType::List(_) => false,

            // 字典：保守返回 false
            MonoType::Dict(_, _) => false,

            // 集合：保守返回 false
            MonoType::Set(_) => false,

            // Option：内部类型必须 Sync
            MonoType::Option(inner) => self.is_sync(inner),

            // Result：Ok/Err 必须 Sync
            MonoType::Result(ok, err) => self.is_sync(ok) && self.is_sync(err),

            // 元组：如果是 (T, &T) 形式可能 Sync，保守返回 false
            MonoType::Tuple(types) => types.iter().all(|t| self.is_sync(t)),

            // 函数类型：通常不用于共享
            MonoType::Fn { .. } => false,

            // Arc: 总是 Sync（安全共享）
            MonoType::Arc(inner) => self.is_sync(inner),

            // Weak: 基于 Arc，总是 Sync
            MonoType::Weak(inner) => self.is_sync(inner),

            // Range: 元素类型必须 Sync
            MonoType::Range { elem_type } => self.is_sync(elem_type),

            // 联合/交集：所有成员必须 Sync
            MonoType::Union(types) => types.iter().all(|t| self.is_sync(t)),
            MonoType::Intersection(types) => types.iter().all(|t| self.is_sync(t)),

            // 结构体：所有字段必须 Sync
            MonoType::Struct(s) => s.fields.iter().all(|(_, f)| self.is_sync(f)),

            // 枚举：只是标签
            MonoType::Enum(_) => true,

            // 类型变量和类型引用：保守假设为 Sync
            MonoType::TypeVar(_) => true,
            MonoType::TypeRef(_) => true,
            // 关联类型需要检查宿主类型
            MonoType::AssocType { host_type, .. } => self.is_sync(host_type),
            // 字面量类型：检查基础类型
            MonoType::Literal { base_type, .. } => self.is_sync(base_type),
            // 元类型：编译期概念，总是 Sync
            MonoType::MetaType { .. } => true,
        }
    }
}

impl Default for SendSyncChecker {
    fn default() -> Self {
        Self::new()
    }
}

// =========================================================================
// 约束传播支持
// =========================================================================

use crate::frontend::core::typecheck::{SendSyncConstraint, SendSyncSolver};

/// Send/Sync 约束传播结果
#[derive(Debug, Clone, Default)]
pub struct SendSyncPropagationResult {
    /// 需要生成的 Send 特化版本
    pub require_send_specialization: bool,
    /// 需要生成的 Sync 特化版本
    pub require_sync_specialization: bool,
    /// 无法满足约束的类型
    pub unsatisfied_types: Vec<(MonoType, SendSyncConstraint)>,
}

impl SendSyncPropagationResult {
    /// 创建新的结果
    pub fn new() -> Self {
        Self {
            require_send_specialization: false,
            require_sync_specialization: false,
            unsatisfied_types: Vec::new(),
        }
    }

    /// 添加未满足的约束
    pub fn add_unsatisfied(
        &mut self,
        ty: MonoType,
        constraint: SendSyncConstraint,
    ) {
        if constraint.require_send {
            self.require_send_specialization = true;
        }
        if constraint.require_sync {
            self.require_sync_specialization = true;
        }
        self.unsatisfied_types.push((ty, constraint));
    }

    /// 是否可以满足约束
    pub fn can_satisfy(&self) -> bool {
        self.unsatisfied_types.is_empty()
    }
}

/// Send/Sync 约束传播器
///
/// 负责：
/// 1. 从 spawn 点收集 Send/Sync 约束
/// 2. 将约束传播到泛型参数
/// 3. 生成特化请求
#[derive(Debug, Default)]
pub struct SendSyncPropagator {
    /// 收集的约束
    constraints: Vec<(MonoType, SendSyncConstraint)>,
}

impl SendSyncPropagator {
    /// 创建新的传播器
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
        }
    }

    /// 重置状态
    pub fn reset(&mut self) {
        self.constraints.clear();
    }

    /// 添加 Send 约束
    pub fn add_send_constraint(
        &mut self,
        ty: &MonoType,
    ) {
        self.constraints
            .push((ty.clone(), SendSyncConstraint::send_only()));
    }

    /// 添加 Sync 约束
    pub fn add_sync_constraint(
        &mut self,
        ty: &MonoType,
    ) {
        self.constraints
            .push((ty.clone(), SendSyncConstraint::sync_only()));
    }

    /// 添加 Send + Sync 约束
    pub fn add_send_sync_constraint(
        &mut self,
        ty: &MonoType,
    ) {
        self.constraints
            .push((ty.clone(), SendSyncConstraint::send_sync()));
    }

    /// 从约束求解器收集约束
    pub fn collect_from_solver(
        &mut self,
        solver: &SendSyncSolver,
        type_args: &[MonoType],
    ) {
        for ty in type_args {
            let constraint = solver.get_constraint(ty);
            if constraint.require_send || constraint.require_sync {
                self.constraints.push((ty.clone(), constraint));
            }
        }
    }

    /// 传播约束到类型参数
    ///
    /// 约束传播规则：
    /// - `Vec[T]` 约束 Send → T 约束 Send
    /// - `(T, U)` 约束 Send → T 和 U 都约束 Send
    /// - `fn(T) -> U` 约束 Send → T 和 U 都约束 Send
    pub fn propagate(&self) -> Vec<(MonoType, SendSyncConstraint)> {
        let mut propagated = Vec::new();
        let mut visited = std::collections::HashSet::new();

        for (ty, constraint) in &self.constraints {
            let key = format!("{:?}-{:?}", ty, constraint);
            if visited.insert(key) {
                self.propagate_type(ty, constraint, &mut propagated, &mut visited);
            }
        }

        propagated
    }

    /// 递归传播约束到类型
    #[allow(clippy::only_used_in_recursion)]
    fn propagate_type(
        &self,
        ty: &MonoType,
        constraint: &SendSyncConstraint,
        result: &mut Vec<(MonoType, SendSyncConstraint)>,
        visited: &mut std::collections::HashSet<String>,
    ) {
        // 添加当前类型的约束
        result.push((ty.clone(), constraint.clone()));

        // 递归传播到类型参数
        match ty {
            MonoType::Struct(s) => {
                for (_, field_ty) in &s.fields {
                    self.propagate_type(field_ty, constraint, result, visited);
                }
            }
            MonoType::Tuple(types) => {
                for elem_ty in types {
                    self.propagate_type(elem_ty, constraint, result, visited);
                }
            }
            MonoType::List(elem) | MonoType::Set(elem) => {
                self.propagate_type(elem, constraint, result, visited);
            }
            MonoType::Dict(key, value) => {
                self.propagate_type(key, constraint, result, visited);
                self.propagate_type(value, constraint, result, visited);
            }
            MonoType::Range { elem_type } => {
                self.propagate_type(elem_type, constraint, result, visited);
            }
            MonoType::Fn {
                params,
                return_type,
                ..
            } => {
                for param_ty in params {
                    self.propagate_type(param_ty, constraint, result, visited);
                }
                self.propagate_type(return_type, constraint, result, visited);
            }
            MonoType::Union(types) | MonoType::Intersection(types) => {
                for member_ty in types {
                    self.propagate_type(member_ty, constraint, result, visited);
                }
            }
            MonoType::Arc(inner) => {
                self.propagate_type(inner, constraint, result, visited);
            }
            _ => {}
        }
    }

    /// 验证约束是否可满足
    ///
    /// 返回不可满足的类型及其约束
    pub fn verify_constraints(
        &self,
        checker: &SendSyncChecker,
    ) -> SendSyncPropagationResult {
        let mut result = SendSyncPropagationResult::new();
        let propagated = self.propagate();

        for (ty, constraint) in propagated {
            let is_send = constraint.require_send && !checker.is_send(&ty);
            let is_sync = constraint.require_sync && !checker.is_sync(&ty);

            if is_send || is_sync {
                result.add_unsatisfied(ty, constraint);
            }
        }

        result
    }

    /// 获取所有收集的约束
    pub fn constraints(&self) -> &[(MonoType, SendSyncConstraint)] {
        &self.constraints
    }
}
