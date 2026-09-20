//! Call frames for the interpreter
//!
//! This module provides the call frame structure used for function calls.

use crate::backends::common::value::TaskId;
use crate::backends::common::RuntimeValue;

/// Maximum number of local variables
pub const MAX_LOCALS: usize = 256;

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
}
