//! Call frames for the interpreter
//!
//! This module provides the call frame structure used for function calls.

use crate::backends::common::value::{FunctionId, TaskId};
use crate::backends::common::RuntimeValue;
use crate::middle::bytecode::Reg;
use std::collections::VecDeque;

/// Maximum number of local variables
pub const MAX_LOCALS: usize = 256;

/// 返回值去向（蹦床调度，WBS 3.9.1）。
///
/// 解释型调用不再递归进入 `execute_by_id`：被调帧由外层 `step_one` 循环驱动，
/// 故其返回值无法经 Rust 函数的返回值带出——本类型指明它该落到哪里。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnTarget {
    /// 宿主入口帧（`execute_by_id` / `call_function_by_id` / `call_closure` 压入）：
    /// 返回值经 `Interpreter::last_return_value` 交还宿主循环。
    Host,
    /// 解释型调用帧：写回调用者帧（弹栈后的栈顶）的该槽位。
    CallerSlot(u16),
    /// 解释型调用帧且指令不带 dst：返回值直接丢弃。
    Discard,
}

impl ReturnTarget {
    /// 从指令的 `dst` 得出返回值去向（解释型调用帧）。
    pub fn from_dst(dst: Option<Reg>) -> Self {
        match dst {
            Some(reg) => ReturnTarget::CallerSlot(reg.0),
            None => ReturnTarget::Discard,
        }
    }
}

/// 一次待启动的解释型调用（蹦床）。
///
/// 一条指令可能发起多个调用（`Spawn` 的多闭包、`SpawnFromList` 的列表闭包），
/// 而蹦床一次只压一帧：第一个调用立即压帧，其余排进调用者帧的
/// `pending_calls`，由 `Interpreter::return_from_frame` 在被调返回后逐个启动——
/// 全程不递归进入 `execute_by_id`，故解释型深度不消耗原生栈。
#[derive(Debug, Clone)]
pub struct PendingCall {
    /// 函数表索引
    pub func_id: FunctionId,
    /// 实参（闭包调用含 env 前置段）
    pub args: Vec<RuntimeValue>,
    /// 上值表（闭包调用为 env；静态/虚表调用为空）
    pub upvalues: Vec<RuntimeValue>,
    /// 返回值落点
    pub target: ReturnTarget,
}

impl PendingCall {
    /// 按函数表索引的调用（`CallStatic` / `CallVirt` / `SpawnFromList` 内层）：
    /// 不注入上值。
    pub fn function(
        func_id: FunctionId,
        args: Vec<RuntimeValue>,
        dst: Option<Reg>,
    ) -> Self {
        Self {
            func_id,
            args,
            upvalues: Vec::new(),
            target: ReturnTarget::from_dst(dst),
        }
    }

    /// 闭包调用（`CallDyn` / `Spawn` 内层）：env 同时作上值（`LoadUpvalue` 读）
    /// 与实参前段（`LoadArg` 读）。
    pub fn closure(
        func_id: FunctionId,
        args: Vec<RuntimeValue>,
        upvalues: Vec<RuntimeValue>,
        dst: Option<Reg>,
    ) -> Self {
        Self {
            func_id,
            args,
            upvalues,
            target: ReturnTarget::from_dst(dst),
        }
    }
}

/// Call frame for function execution
///
/// A call frame contains all the state needed to execute a function,
/// including its registers, instruction pointer, and local variables.
///
/// ## 为什么持 `func_id` 而非 `BytecodeFunction`
///
/// 此前 `Frame` 按值持有整个 `BytecodeFunction`（含指令 `Vec`、`labels`
/// `HashMap`、`debug_map` `HashMap`、函数名 `String`）。这带来两个问题：
///
/// 1. **每次函数调用深拷贝整个函数体**——`call_function_by_id` 先从
///    `functions_by_id` 克隆一份再交给 `Frame::new`，而 `Frame` 又克隆一次。
///    递归场景（如 `fib(28)` 的 63 万次调用）下这是主要开销。
/// 2. **指令读取受调用栈借用限制**——指令存在 frame 里，而 frame 又要可变，
///    导致执行一条指令必须把 frame 从 `call_stack` 弹出（见 `step_one`）。
///
/// 改持 `func_id` 后，函数体统一由只读的 `Image` 持有，指令可经
/// `&Image` 读取，执行状态与指令数据彻底解耦。
#[derive(Debug, Clone)]
pub struct Frame {
    /// 函数表索引（指向 `Image::functions_by_id`）
    pub func_id: u32,
    /// Instruction pointer (index into instructions)
    pub ip: usize,
    /// Unified slot array (registers + locals merged)
    slots: Vec<RuntimeValue>,
    /// Upvalue capture values
    upvalues: Vec<RuntimeValue>,
    /// Spawn task groups (RFC-024: only meaningful inside spawn scopes).
    spawn_groups: Vec<Vec<TaskId>>,
    /// 返回值去向（蹦床）。宿主入口压入的帧默认 `Host`。
    return_target: ReturnTarget,
    /// 本帧发起、尚未启动的解释型调用（多调用指令的后续项）。
    pending_calls: VecDeque<PendingCall>,
}

impl Frame {
    /// Create a new frame for the function at `func_id`.
    ///
    /// `local_count` 由调用方从 `Image` 查出后传入——`Frame` 不再持有函数体，
    /// 无法自行推导槽位数量。
    pub fn new(
        func_id: u32,
        local_count: usize,
    ) -> Self {
        Self {
            func_id,
            ip: 0,
            slots: vec![RuntimeValue::Void; local_count.max(1)],
            upvalues: Vec::new(),
            spawn_groups: Vec::new(),
            return_target: ReturnTarget::Host,
            pending_calls: VecDeque::new(),
        }
    }

    /// Create a new frame with arguments
    pub fn with_args(
        func_id: u32,
        local_count: usize,
        args: &[RuntimeValue],
    ) -> Self {
        let mut frame = Self::new(func_id, local_count);
        for (i, arg) in args.iter().enumerate() {
            if i < frame.slots.len() {
                frame.slots[i] = arg.clone();
            }
        }
        frame
    }

    /// Advance the instruction pointer
    pub fn advance(&mut self) {
        self.ip += 1;
    }

    /// Get a slot value (unified for locals + registers)
    pub fn get_slot(
        &self,
        index: usize,
    ) -> Option<&RuntimeValue> {
        self.slots.get(index)
    }

    /// Get a mutable slot value
    pub fn get_slot_mut(
        &mut self,
        index: usize,
    ) -> Option<&mut RuntimeValue> {
        self.slots.get_mut(index)
    }

    /// Set a slot value, extending the slot array if necessary
    pub fn set_slot(
        &mut self,
        index: usize,
        value: RuntimeValue,
    ) {
        if index >= self.slots.len() {
            self.slots.resize(index + 1, RuntimeValue::Void);
        }
        self.slots[index] = value;
    }

    /// Record a spawned task into the innermost spawn group (RFC-024).
    ///
    /// 仅在有活跃 spawn 作用域（`push_spawn_group` 已调用）时生效——
    /// 作用域外的 spawn 不产生待 join 的任务组。
    pub fn record_spawned_task(
        &mut self,
        task_id: TaskId,
    ) {
        if let Some(group) = self.spawn_groups.last_mut() {
            group.push(task_id);
        }
    }

    /// 开始一个 spawn 作用域（RFC-024）
    pub fn push_spawn_group(&mut self) {
        self.spawn_groups.push(Vec::new());
    }

    /// Take all spawned tasks recorded in this frame, draining the groups.
    pub fn take_all_spawned_tasks(&mut self) -> Vec<TaskId> {
        let mut out = Vec::new();
        for group in self.spawn_groups.drain(..) {
            out.extend(group);
        }
        out
    }

    /// Get an upvalue
    pub fn get_upvalue(
        &self,
        index: usize,
    ) -> Option<&RuntimeValue> {
        self.upvalues.get(index)
    }

    /// Set an upvalue
    pub fn set_upvalue(
        &mut self,
        index: usize,
        value: RuntimeValue,
    ) {
        if index >= self.upvalues.len() {
            self.upvalues.resize(index + 1, RuntimeValue::Void);
        }
        self.upvalues[index] = value;
    }

    /// Get the number of local variables
    pub fn local_count(&self) -> usize {
        self.slots.len()
    }

    /// Get mutable access to upvalues (for closure capture)
    pub fn upvalues_mut(&mut self) -> &mut Vec<RuntimeValue> {
        &mut self.upvalues
    }

    /// 本帧的返回值去向（蹦床）。
    pub fn return_target(&self) -> ReturnTarget {
        self.return_target
    }

    /// 设置返回值去向。解释型调用帧在压栈前设置；宿主入口帧保持 Host。
    pub fn set_return_target(
        &mut self,
        target: ReturnTarget,
    ) {
        self.return_target = target;
    }

    /// 排入一个待启动的解释型调用（多调用指令的后续项）。
    pub fn push_pending_call(
        &mut self,
        call: PendingCall,
    ) {
        self.pending_calls.push_back(call);
    }

    /// 取出下一个待启动的解释型调用（无则返回 None）。
    pub fn pop_pending_call(&mut self) -> Option<PendingCall> {
        self.pending_calls.pop_front()
    }
}
