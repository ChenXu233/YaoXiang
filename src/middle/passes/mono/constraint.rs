//! Send/Sync 约束传播模块
//!
//! 核心功能：
//! 1. 从 spawn 点收集 Send/Sync 约束
//! 2. 约束沿类型变量传播到泛型参数
//! 3. 生成特化请求，驱动单态化器生成 Send/Sync 特化版本
//!
//! 设计原则：
//! - 简单：约束传播逻辑清晰直接
//! - 实用：约束从 spawn 点自然产生，不需要显式标注
//! - 零负担：用户代码不需要任何改变

use crate::frontend::typecheck::{MonoType, SendSyncConstraint, SendSyncConstraintSolver};
use crate::middle::passes::lifetime::send_sync::{SendSyncChecker, SendSyncPropagator};
use std::collections::HashSet;
use crate::util::span::Span;

/// 约束传播结果
#[derive(Debug, Clone)]
pub struct ConstraintPropagationResult {
    /// 需要生成的 Send 特化版本
    pub require_send_specialization: bool,
    /// 需要生成的 Sync 特化版本
    pub require_sync_specialization: bool,
    /// 无法满足约束的类型
    pub unsatisfied_types: Vec<(MonoType, SendSyncConstraint)>,
    /// 约束传播详情
    pub details: Vec<ConstraintDetail>,
}

impl Default for ConstraintPropagationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstraintPropagationResult {
    /// 创建新的结果
    pub fn new() -> Self {
        Self {
            require_send_specialization: false,
            require_sync_specialization: false,
            unsatisfied_types: Vec::new(),
            details: Vec::new(),
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

    /// 添加传播详情
    pub fn add_detail(
        &mut self,
        detail: ConstraintDetail,
    ) {
        self.details.push(detail);
    }

    /// 是否可以满足约束
    pub fn can_satisfy(&self) -> bool {
        self.unsatisfied_types.is_empty()
    }

    /// 是否需要任何特化
    pub fn needs_specialization(&self) -> bool {
        self.require_send_specialization || self.require_sync_specialization
    }
}

/// 约束传播详情
#[derive(Debug, Clone)]
pub struct ConstraintDetail {
    /// 约束来源类型
    pub source_type: MonoType,
    /// 约束目标类型
    pub target_type: MonoType,
    /// 约束类型
    pub constraint: SendSyncConstraint,
    /// 传播路径
    pub path: String,
    /// 位置
    pub span: Span,
}

/// 约束收集器
///
/// 从代码中收集 Send/Sync 约束：
/// - spawn 闭包捕获的变量必须 Send
/// - 跨线程传递的参数必须 Send
/// - 函数返回值必须 Send（如果用于 spawn）
#[derive(Debug, Default)]
pub struct ConstraintCollector {
    /// 收集的约束
    constraints: Vec<(MonoType, SendSyncConstraint, Span)>,
    /// 已访问的约束（避免重复）
    visited: HashSet<String>,
}

impl ConstraintCollector {
    /// 创建新的收集器
    pub fn new() -> Self {
        Self {
            constraints: Vec::new(),
            visited: HashSet::new(),
        }
    }

    /// 重置状态
    pub fn reset(&mut self) {
        self.constraints.clear();
        self.visited.clear();
    }

    /// 添加 Send 约束
    pub fn add_send_constraint(
        &mut self,
        ty: &MonoType,
        span: Span,
    ) {
        let key = format!("send-{:?}", ty);
        if self.visited.insert(key) {
            self.constraints
                .push((ty.clone(), SendSyncConstraint::send_only(), span));
        }
    }

    /// 添加 Sync 约束
    pub fn add_sync_constraint(
        &mut self,
        ty: &MonoType,
        span: Span,
    ) {
        let key = format!("sync-{:?}", ty);
        if self.visited.insert(key) {
            self.constraints
                .push((ty.clone(), SendSyncConstraint::sync_only(), span));
        }
    }

    /// 添加 Send + Sync 约束
    pub fn add_send_sync_constraint(
        &mut self,
        ty: &MonoType,
        span: Span,
    ) {
        let key = format!("send_sync-{:?}", ty);
        if self.visited.insert(key) {
            self.constraints
                .push((ty.clone(), SendSyncConstraint::send_sync(), span));
        }
    }

    /// 获取所有收集的约束
    pub fn constraints(&self) -> &[(MonoType, SendSyncConstraint, Span)] {
        &self.constraints
    }
}

/// 约束传播引擎
///
/// 核心算法：
/// 1. 从收集的约束开始
/// 2. 沿类型结构递归传播约束到类型参数
/// 3. 验证约束是否可以满足
/// 4. 生成特化请求
#[derive(Debug)]
pub struct ConstraintPropagationEngine {
    /// 约束收集器
    collector: ConstraintCollector,
    /// 约束传播器
    propagator: SendSyncPropagator,
    /// Send/Sync 检查器
    checker: SendSyncChecker,
}

impl Default for ConstraintPropagationEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ConstraintPropagationEngine {
    /// 创建新的引擎
    pub fn new() -> Self {
        Self {
            collector: ConstraintCollector::new(),
            propagator: SendSyncPropagator::new(),
            checker: SendSyncChecker::new(),
        }
    }

    /// 重置状态
    pub fn reset(&mut self) {
        self.collector.reset();
        self.propagator.reset();
    }

    /// 添加 Send 约束
    pub fn add_send_constraint(
        &mut self,
        ty: &MonoType,
        span: Span,
    ) {
        self.collector.add_send_constraint(ty, span);
    }

    /// 添加 Sync 约束
    pub fn add_sync_constraint(
        &mut self,
        ty: &MonoType,
        span: Span,
    ) {
        self.collector.add_sync_constraint(ty, span);
    }

    /// 添加 spawn 约束
    ///
    /// 当检测到 spawn 调用时：
    /// 1. 闭包参数必须 Send
    /// 2. 闭包返回值必须 Send
    /// 3. 闭包捕获的变量必须 Send
    pub fn add_spawn_constraint(
        &mut self,
        closure_type: &MonoType,
        span: Span,
    ) {
        if let MonoType::Fn {
            params,
            return_type,
            ..
        } = closure_type
        {
            // 闭包参数必须 Send
            for param_ty in params {
                self.collector.add_send_constraint(param_ty, span);
            }
            // 闭包返回值必须 Send
            self.collector.add_send_constraint(return_type, span);
        }
    }

    /// 添加捕获变量约束
    ///
    /// 闭包捕获的自由变量必须 Send
    pub fn add_captured_var_constraint(
        &mut self,
        var_type: &MonoType,
        span: Span,
    ) {
        self.collector.add_send_constraint(var_type, span);
    }

    /// 传播约束
    pub fn propagate(&mut self) -> ConstraintPropagationResult {
        let mut result = ConstraintPropagationResult::new();

        // 从收集器收集约束到传播器
        for (ty, constraint, span) in self.collector.constraints() {
            if constraint.require_send {
                self.propagator.add_send_constraint(ty);
            }
            if constraint.require_sync {
                self.propagator.add_sync_constraint(ty);
            }

            // 记录传播详情
            result.add_detail(ConstraintDetail {
                source_type: ty.clone(),
                target_type: ty.clone(),
                constraint: constraint.clone(),
                path: "initial".to_string(),
                span: *span,
            });
        }

        // 执行约束传播
        let propagated = self.propagator.propagate();

        // 验证约束是否可满足
        for (ty, constraint) in propagated {
            let is_send_ok = !constraint.require_send || self.checker.is_send(&ty);
            let is_sync_ok = !constraint.require_sync || self.checker.is_sync(&ty);

            if !is_send_ok || !is_sync_ok {
                result.add_unsatisfied(ty, constraint);
            }
        }

        // 如果有未满足的约束，标记需要特化
        if !result.can_satisfy() {
            result.require_send_specialization = true;
        }

        result
    }

    /// 从类型推断的约束求解器收集约束
    ///
    /// 用于集成类型检查阶段的 Send/Sync 约束
    pub fn collect_from_type_solver(
        &mut self,
        solver: &SendSyncConstraintSolver,
        type_args: &[MonoType],
    ) {
        for ty in type_args {
            let constraint = solver.get_constraint(ty);
            if constraint.require_send {
                self.collector.add_send_constraint(ty, Span::default());
            }
            if constraint.require_sync {
                self.collector.add_sync_constraint(ty, Span::default());
            }
        }
    }

    /// 获取收集器
    pub fn collector(&self) -> &ConstraintCollector {
        &self.collector
    }
}

/// 特化请求
///
/// 描述需要生成的特化版本
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpecializationRequest {
    /// 泛型函数/类型的标识
    pub generic_name: String,
    /// 类型参数
    pub type_args: Vec<MonoType>,
    /// 需要的约束
    pub constraints: SendSyncConstraint,
    /// 位置
    pub span: Span,
}

impl SpecializationRequest {
    /// 创建新的特化请求
    pub fn new(
        generic_name: String,
        type_args: Vec<MonoType>,
        constraints: SendSyncConstraint,
        span: Span,
    ) -> Self {
        Self {
            generic_name,
            type_args,
            constraints,
            span,
        }
    }

    /// 是否需要 Send 特化
    pub fn needs_send(&self) -> bool {
        self.constraints.require_send
    }

    /// 是否需要 Sync 特化
    pub fn needs_sync(&self) -> bool {
        self.constraints.require_sync
    }
}

/// 特化请求收集器
///
/// 收集所有需要生成的特化请求
#[derive(Debug, Default)]
pub struct SpecializationRequestCollector {
    /// 收集的请求
    requests: Vec<SpecializationRequest>,
}

impl SpecializationRequestCollector {
    /// 创建新的收集器
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    /// 添加特化请求
    pub fn add_request(
        &mut self,
        request: SpecializationRequest,
    ) {
        self.requests.push(request);
    }

    /// 获取所有 Send 特化请求
    pub fn send_requests(&self) -> impl Iterator<Item = &SpecializationRequest> {
        self.requests.iter().filter(|r| r.needs_send())
    }

    /// 获取所有 Sync 特化请求
    pub fn sync_requests(&self) -> impl Iterator<Item = &SpecializationRequest> {
        self.requests.iter().filter(|r| r.needs_sync())
    }

    /// 获取所有请求
    pub fn requests(&self) -> &[SpecializationRequest] {
        &self.requests
    }

    /// 清空
    pub fn clear(&mut self) {
        self.requests.clear();
    }
}
