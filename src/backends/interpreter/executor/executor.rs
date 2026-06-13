//! Interpreter executor for YaoXiang bytecode
//!
//! This module implements the main interpreter that executes bytecode.
//! It follows the standard fetch-decode-execute cycle.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use crate::backends::{Executor, ExecutorResult, ExecutorError, ExecutionState, ExecutorConfig};
use crate::backends::common::{RuntimeValue, Heap, HeapValue};
use crate::backends::common::value::{
    AsyncState, AsyncValue, FunctionValue, FunctionId, TaskId, ValueType,
};
use crate::middle::bytecode::{BytecodeFunction, Reg, Label, BinaryOp, CompareOp, ConstValue};
use crate::backends::interpreter::Frame;
use crate::backends::interpreter::ffi::FfiRegistry;
use crate::backends::interpreter::runtime::InterpreterRuntimeConfig;
use crate::backends::runtime::Runtime;
use crate::backends::runtime::facade::RuntimeConfig;
use crate::backends::runtime::engine::{
    SyncValue, TaskCancelReason, TaskMeta, TaskOutcome, TaskResult, sv,
};
use crate::util::i18n::MSG;
use crate::tlog;
use crate::std::NativeContext;

/// Maximum call stack depth
const DEFAULT_MAX_STACK_DEPTH: usize = 1024;

/// Read-only shared state, shared across threads via raw pointer.
///
/// Safety: `drive_until` blocks until all tasks complete, so the data outlives all tasks.
/// Data is read-only after creation, so no data races.
pub(super) struct SharedState {
    pub functions: HashMap<String, BytecodeFunction>,
    pub functions_by_id: Vec<BytecodeFunction>,
    pub constants: Vec<ConstValue>,
    pub type_table: Vec<crate::middle::core::ir::Type>,
    pub ffi: FfiRegistry,
}

/// Wrapper around a raw pointer to make it `Send`.
///
/// # Safety
///
/// The pointer must remain valid for the entire duration of the task execution.
/// `drive_until` blocks until all tasks complete, guaranteeing the data outlives all tasks.
/// Data behind the pointer is read-only after creation, so no data races occur.
#[derive(Clone, Copy)]
struct SendPtr(*const SharedState);

impl SendPtr {
    /// Get the raw pointer.
    ///
    /// # Safety
    ///
    /// The caller must ensure the pointer is used safely (read-only, valid lifetime).
    unsafe fn get(self) -> *const SharedState {
        self.0
    }
}

// SAFETY: See Safety comment above.
unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

#[derive(Debug)]
pub enum InterpreterTask {
    Static {
        func_name: String,
        args: Vec<RuntimeValue>,
    },
    Native {
        func_name: String,
        args: Vec<RuntimeValue>,
    },
    Dyn {
        func: FunctionValue,
        args: Vec<RuntimeValue>,
    },
}

/// The YaoXiang bytecode interpreter
///
/// The interpreter loads bytecode modules and executes them instruction by instruction.
/// It maintains:
/// - A heap for dynamically allocated objects
/// - A call stack for function calls
/// - A constant pool for literals
pub struct Interpreter {
    /// Heap for dynamic allocation
    pub(super) heap: Heap,
    /// Call stack
    pub(super) call_stack: Vec<Frame>,
    /// Constant pool (shared across modules)
    pub(super) constants: Vec<ConstValue>,
    /// Function table (name -> function)
    pub(super) functions: HashMap<String, BytecodeFunction>,
    /// Function table by index (for closure calls via func_id)
    pub(super) functions_by_id: Vec<BytecodeFunction>,
    /// Type table
    pub(super) type_table: Vec<crate::middle::core::ir::Type>,
    /// Current execution state
    pub(super) state: ExecutionState,
    /// Configuration
    pub(super) config: ExecutorConfig,
    /// Breakpoints
    pub(super) breakpoints: HashMap<usize, ()>,
    /// FFI Registry for native function calls
    pub(super) ffi: FfiRegistry,
    /// Standard output
    #[allow(dead_code)] // Might be unused if only accessed via write!
    stdout: Option<std::sync::Arc<std::sync::Mutex<dyn std::io::Write + Send>>>,
    /// Interpreter-side runtime configuration (defaults to current behavior).
    pub(super) runtime_config: InterpreterRuntimeConfig,
    /// Runtime facade used for task scheduling (Embedded / Standard / Full).
    pub(super) rt: Runtime,
    /// Read-only shared state, shared across threads via raw pointer.
    /// Set in `execute_module`; null when not yet initialized.
    pub(super) shared: *const SharedState,
}

impl fmt::Debug for Interpreter {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        f.debug_struct("Interpreter")
            .field("heap", &self.heap)
            .field("call_stack", &self.call_stack)
            .field("constants", &self.constants)
            .field("functions", &self.functions)
            .field("functions_by_id", &self.functions_by_id)
            .field("type_table", &self.type_table)
            .field("state", &self.state)
            .field("config", &self.config)
            .field("breakpoints", &self.breakpoints)
            .field("ffi", &self.ffi)
            .field(
                "stdout",
                &if self.stdout.is_some() {
                    "Some(...)"
                } else {
                    "None"
                },
            )
            .field("shared", &self.shared)
            .finish()
    }
}

impl Default for Interpreter {
    fn default() -> Self {
        Self::new()
    }
}

impl Interpreter {
    /// Create a new interpreter with default configuration
    pub fn new() -> Self {
        Self::with_config(ExecutorConfig::default())
    }

    /// Create an interpreter with custom configuration
    pub fn with_config(config: ExecutorConfig) -> Self {
        let runtime_config = InterpreterRuntimeConfig::default();
        let rt = Runtime::new(RuntimeConfig {
            mode: runtime_config.runtime,
            workers: runtime_config.workers,
            work_stealing: runtime_config.work_stealing,
        })
        .unwrap_or_else(|_| Runtime::new(RuntimeConfig::default()).unwrap());

        Self {
            heap: Heap::new(),
            call_stack: Vec::with_capacity(DEFAULT_MAX_STACK_DEPTH),
            constants: Vec::new(),
            functions: HashMap::new(),
            functions_by_id: Vec::new(),
            type_table: Vec::new(),
            state: ExecutionState::default(),
            config,
            breakpoints: HashMap::new(),
            ffi: FfiRegistry::with_std(),
            stdout: None, // Default to stdout (handled by None check)
            runtime_config,
            rt,
            shared: std::ptr::null(),
        }
    }

    pub fn runtime_config(&self) -> &InterpreterRuntimeConfig {
        &self.runtime_config
    }

    /// Create an interpreter that shares read-only state via a raw pointer.
    ///
    /// The caller must ensure that the `SharedState` outlives this interpreter.
    /// Typically used when spawning per-task interpreters inside `execute_module`.
    pub(super) fn from_shared(shared: *const SharedState) -> Self {
        let rt = Runtime::new(RuntimeConfig::default()).unwrap();

        // SAFETY: 共享状态由主解释器管理生命周期（通过 execute_module 中的 Box::into_raw）。
        // 主解释器通过 drive_until 阻塞直到所有任务完成，保证数据在任务期间有效。
        // 数据在创建后只读，无数据竞争。
        // 如果 shared 为空（例如 execute_module 未调用），使用空数据。
        let (constants, functions, functions_by_id, type_table, ffi) = if shared.is_null() {
            (
                Vec::new(),
                HashMap::new(),
                Vec::new(),
                Vec::new(),
                FfiRegistry::new(),
            )
        } else {
            let shared_ref = unsafe { &*shared };
            (
                shared_ref.constants.clone(),
                shared_ref.functions.clone(),
                shared_ref.functions_by_id.clone(),
                shared_ref.type_table.clone(),
                shared_ref.ffi.clone(),
            )
        };

        Self {
            heap: Heap::new(),
            call_stack: Vec::with_capacity(DEFAULT_MAX_STACK_DEPTH),
            constants,
            functions,
            functions_by_id,
            type_table,
            state: ExecutionState::default(),
            config: ExecutorConfig::default(),
            breakpoints: HashMap::new(),
            ffi,
            stdout: None,
            runtime_config: InterpreterRuntimeConfig::default(),
            rt,
            // 不设置 shared 字段，避免 Drop 时双重释放。
            // 共享数据已拷贝到上方的字段中。
            shared: std::ptr::null(),
        }
    }

    pub fn set_runtime_config(
        &mut self,
        runtime_config: InterpreterRuntimeConfig,
    ) {
        self.runtime_config = runtime_config;
        // Rebuild Runtime facade to match new config
        self.rt = Runtime::new(RuntimeConfig {
            mode: self.runtime_config.runtime,
            workers: self.runtime_config.workers,
            work_stealing: self.runtime_config.work_stealing,
        })
        .unwrap_or_else(|_| Runtime::new(RuntimeConfig::default()).unwrap());
    }

    /// Set standard output redirect
    pub fn set_stdout(
        &mut self,
        stdout: std::sync::Arc<std::sync::Mutex<dyn std::io::Write + Send>>,
    ) {
        self.stdout = Some(stdout);
    }

    /// Get mutable reference to the FFI registry for registering native functions
    pub fn ffi_registry_mut(&mut self) -> &mut FfiRegistry {
        &mut self.ffi
    }

    /// Get reference to the FFI registry
    pub fn ffi_registry(&self) -> &FfiRegistry {
        &self.ffi
    }

    /// Build vtable for a struct type at runtime
    ///
    /// This method looks up methods in the function table by matching the type name prefix.
    /// Functions are stored with keys like "TypeName.method_name".
    pub(super) fn build_vtable(
        &mut self,
        type_name: &str,
    ) -> Vec<(String, FunctionValue)> {
        let mut vtable = Vec::new();
        let method_prefix = format!("{}.", type_name);

        // Find all functions that match the type name prefix
        for (func_name, bytecode_func) in &self.functions {
            if func_name.starts_with(&method_prefix) {
                // Extract method name (everything after the prefix)
                let method_name = func_name[method_prefix.len()..].to_string();

                // Create a new function ID for this method
                let func_id = FunctionId(self.functions_by_id.len() as u32);

                // Add to functions_by_id for runtime lookup
                self.functions_by_id.push(bytecode_func.clone());

                vtable.push((
                    method_name,
                    FunctionValue {
                        func_id,
                        env: Vec::new(), // Methods don't need closure env
                    },
                ));
            }
        }

        vtable
    }

    /// Call a YaoXiang function by its FunctionId.
    /// This is used by native functions (like map/filter/reduce) to invoke closures.
    pub fn call_function_by_id(
        &mut self,
        func_id: crate::backends::common::value::FunctionId,
        args: &[RuntimeValue],
    ) -> Result<RuntimeValue, ExecutorError> {
        let idx = func_id.0 as usize;
        if idx >= self.functions_by_id.len() {
            let stack = self.capture_stack();
            return Err(ExecutorError::function_not_found(
                format!(
                    "Function with id {} not found (total functions: {})",
                    idx,
                    self.functions_by_id.len()
                ),
                stack,
            ));
        }
        // Clone the function to avoid borrow issues
        let func = self.functions_by_id[idx].clone();
        self.execute_function(&func, args)
    }

    /// Push a frame onto the call stack
    pub(super) fn push_frame(
        &mut self,
        frame: Frame,
    ) -> ExecutorResult<()> {
        if self.call_stack.len() >= self.config.max_stack_depth {
            let stack = self.capture_stack();
            return Err(ExecutorError::stack_overflow(stack));
        }
        self.call_stack.push(frame);
        Ok(())
    }

    /// Pop a frame from the call stack
    pub(super) fn pop_frame(&mut self) -> Option<Frame> {
        self.call_stack.pop()
    }

    /// Get the current frame
    pub fn current_frame(&mut self) -> Option<&mut Frame> {
        self.call_stack.last_mut()
    }

    /// Get the current function
    pub fn current_function(&self) -> Option<&BytecodeFunction> {
        self.call_stack.last().map(|f| &f.function)
    }

    /// Capture the current call stack as a vector of StackFrame
    pub fn capture_stack(&self) -> Vec<crate::backends::StackFrame> {
        self.call_stack
            .iter()
            .rev()
            .map(|frame| crate::backends::StackFrame {
                function_name: frame.function.name.clone(),
                ip: frame.ip,
            })
            .collect()
    }

    /// Resolve a label to an instruction offset
    pub fn resolve_label(
        &mut self,
        label: Label,
    ) -> Option<usize> {
        self.current_frame()
            .and_then(|f| f.function.labels.get(&label).copied())
    }

    /// Load a constant by index
    pub(super) fn load_constant(
        &self,
        idx: u16,
    ) -> RuntimeValue {
        self.constants
            .get(idx as usize)
            .map(|c| match c {
                ConstValue::Void => RuntimeValue::Unit,
                ConstValue::Bool(b) => RuntimeValue::Bool(*b),
                ConstValue::Int(i) => RuntimeValue::Int((*i) as i64),
                ConstValue::Float(f) => RuntimeValue::Float(*f),
                ConstValue::Char(c) => RuntimeValue::Char((*c) as u32),
                ConstValue::String(s) => RuntimeValue::String(s.as_str().into()),
                ConstValue::Bytes(b) => RuntimeValue::Bytes(b.as_slice().into()),
            })
            .unwrap_or(RuntimeValue::Unit)
    }

    pub(super) fn make_async_pending(
        &self,
        task_id: TaskId,
    ) -> RuntimeValue {
        RuntimeValue::Async(Box::new(AsyncValue {
            state: Box::new(AsyncState::Pending(task_id)),
            value_type: ValueType::Unit,
        }))
    }

    pub(super) fn deps_from_args(
        &self,
        args: &[RuntimeValue],
    ) -> Vec<TaskId> {
        let mut deps = Vec::new();
        for arg in args {
            let RuntimeValue::Async(av) = arg else {
                continue;
            };
            if let AsyncState::Pending(id) = av.state.as_ref() {
                deps.push(*id);
            }
        }
        deps
    }

    pub(super) fn schedule_task(
        &mut self,
        task: InterpreterTask,
        meta: TaskMeta,
    ) -> ExecutorResult<TaskId> {
        let sp = SendPtr(self.shared);
        let task_fn: crate::backends::runtime::TaskFn = Box::new(move |_spawn_handle| {
            let mut task_interp = Interpreter::from_shared(unsafe { sp.get() });
            task_interp.execute_scheduled_task_from_data(task)
        });

        let id = self.rt.spawn(meta, task_fn).map_err(|e| {
            let stack = self.capture_stack();
            ExecutorError::runtime(format!("{e}"), stack)
        })?;
        Ok(id)
    }

    pub(super) fn drive_dag_until(
        &mut self,
        target: Option<TaskId>,
    ) -> ExecutorResult<()> {
        self.rt.drive_until(target).map_err(|e| {
            let stack = self.capture_stack();
            ExecutorError::runtime(format!("{e}"), stack)
        })
    }

    pub(super) fn execute_scheduled_task_from_data(
        &mut self,
        task: InterpreterTask,
    ) -> TaskResult {
        let exec_result = match task {
            InterpreterTask::Static { func_name, args } => {
                self.call_static_by_name(&func_name, &args)
            }
            InterpreterTask::Native { func_name, args } => {
                self.call_native_by_name(&func_name, &args)
            }
            InterpreterTask::Dyn { func, args } => {
                let mut resolved = Vec::with_capacity(args.len());
                for arg in &args {
                    match self.force_value_clone(arg) {
                        Ok(v) => resolved.push(v),
                        Err(e) => return Err(sv(RuntimeValue::String(format!("{e}").into()))),
                    }
                }
                let mut final_args = func.env.clone();
                final_args.extend(resolved);
                self.call_function_by_id(func.func_id, &final_args)
            }
        };

        match exec_result {
            Ok(v) => Ok(sv(v)),
            Err(e) => Err(sv(RuntimeValue::String(format!("{e}").into()))),
        }
    }

    pub(super) fn format_cancel_reason(
        &self,
        task_id: TaskId,
        reason: &TaskCancelReason,
    ) -> String {
        let task = self.format_task_id(task_id);
        match reason {
            TaskCancelReason::Explicit => format!("Task {task} cancelled"),
            TaskCancelReason::DependencyFailed { primary, others } => {
                let mut deps = Vec::with_capacity(1 + others.len());
                deps.push(*primary);
                deps.extend(others.iter().copied());

                let mut msg = format!("Task {task} cancelled: dependency failed");
                let summaries: Vec<String> = deps
                    .iter()
                    .map(|dep| self.format_dependency_summary(*dep))
                    .collect();
                if !summaries.is_empty() {
                    msg.push_str(&format!("; causes: {}", summaries.join("; ")));
                }
                msg
            }
            TaskCancelReason::DependencyCancelled { primary, others } => {
                let mut deps = Vec::with_capacity(1 + others.len());
                deps.push(*primary);
                deps.extend(others.iter().copied());

                let mut msg = format!("Task {task} cancelled: dependency cancelled");
                let summaries: Vec<String> = deps
                    .iter()
                    .map(|dep| self.format_dependency_summary(*dep))
                    .collect();
                if !summaries.is_empty() {
                    msg.push_str(&format!("; causes: {}", summaries.join("; ")));
                }
                msg
            }
        }
    }

    fn format_task_id(
        &self,
        task_id: TaskId,
    ) -> String {
        format!("{task_id:?}")
    }

    fn format_dependency_summary(
        &self,
        task_id: TaskId,
    ) -> String {
        let task = self.format_task_id(task_id);
        match self.rt.outcome(task_id) {
            Some(TaskOutcome::Err(payload)) => {
                format!("{task}: {}", self.format_sync_value(&payload))
            }
            Some(TaskOutcome::Cancelled(reason)) => {
                format!(
                    "{task}: cancelled ({})",
                    self.format_cancel_reason_brief(&reason)
                )
            }
            Some(TaskOutcome::Ok(_)) => format!("{task}: ok"),
            None => format!("{task}: pending"),
        }
    }

    fn format_cancel_reason_brief(
        &self,
        reason: &TaskCancelReason,
    ) -> String {
        match reason {
            TaskCancelReason::Explicit => "explicit".to_string(),
            TaskCancelReason::DependencyFailed { primary, others } => {
                if others.is_empty() {
                    format!("dependency failed {primary:?}")
                } else {
                    format!("dependency failed {primary:?} (+{} others)", others.len())
                }
            }
            TaskCancelReason::DependencyCancelled { primary, others } => {
                if others.is_empty() {
                    format!("dependency cancelled {primary:?}")
                } else {
                    format!(
                        "dependency cancelled {primary:?} (+{} others)",
                        others.len()
                    )
                }
            }
        }
    }

    fn format_sync_value(
        &self,
        payload: &SyncValue,
    ) -> String {
        if let Some(rv) = payload.downcast_ref::<RuntimeValue>() {
            return rv.to_string();
        }
        if let Some(s) = payload.downcast_ref::<String>() {
            return s.clone();
        }
        if let Some(s) = payload.downcast_ref::<Arc<str>>() {
            return s.to_string();
        }
        if let Some(s) = payload.downcast_ref::<&'static str>() {
            return s.to_string();
        }
        "Unknown task payload".to_string()
    }

    pub(super) fn force_value_in_place(
        &mut self,
        value: &mut RuntimeValue,
    ) -> ExecutorResult<()> {
        let RuntimeValue::Async(av) = value else {
            return Ok(());
        };

        match av.state.as_ref() {
            AsyncState::Ready(inner) => {
                *value = (**inner).clone();
                Ok(())
            }
            AsyncState::Error(err) => {
                let stack = self.capture_stack();
                Err(ExecutorError::runtime(
                    format!("Async error: {err:?}"),
                    stack,
                ))
            }
            AsyncState::Pending(task_id) => {
                self.drive_dag_until(Some(*task_id))?;
                let outcome = self.rt.outcome(*task_id).ok_or_else(|| {
                    let stack = self.capture_stack();
                    ExecutorError::runtime(format!("Task has no outcome: {task_id:?}"), stack)
                })?;

                match outcome {
                    TaskOutcome::Ok(payload) => {
                        let rv = payload
                            .downcast_ref::<RuntimeValue>()
                            .cloned()
                            .unwrap_or(RuntimeValue::Unit);
                        *value = rv;
                        Ok(())
                    }
                    TaskOutcome::Err(payload) => {
                        let stack = self.capture_stack();
                        Err(ExecutorError::runtime(
                            self.format_sync_value(&payload),
                            stack,
                        ))
                    }
                    TaskOutcome::Cancelled(reason) => {
                        let stack = self.capture_stack();
                        Err(ExecutorError::runtime(
                            self.format_cancel_reason(*task_id, &reason),
                            stack,
                        ))
                    }
                }
            }
        }
    }

    pub(super) fn force_register(
        &mut self,
        frame: &mut Frame,
        reg: Reg,
    ) -> ExecutorResult<RuntimeValue> {
        if let Some(v) = frame.registers.get_mut(reg.0 as usize) {
            self.force_value_in_place(v)?;
            Ok(v.clone())
        } else {
            Ok(RuntimeValue::Unit)
        }
    }

    pub(super) fn force_value_clone(
        &mut self,
        value: &RuntimeValue,
    ) -> ExecutorResult<RuntimeValue> {
        let mut cloned = value.clone();
        self.force_value_in_place(&mut cloned)?;
        Ok(cloned)
    }

    pub(super) fn call_native_by_name(
        &mut self,
        func_name: &str,
        call_args: &[RuntimeValue],
    ) -> ExecutorResult<RuntimeValue> {
        let mut resolved = Vec::with_capacity(call_args.len());
        for arg in call_args {
            resolved.push(self.force_value_clone(arg)?);
        }

        let stack = self.capture_stack();
        let interp_ptr = std::ptr::addr_of_mut!(*self);
        let mut call_fn = move |func: &RuntimeValue,
                                args: &[RuntimeValue]|
              -> Result<RuntimeValue, ExecutorError> {
            if let RuntimeValue::Function(fv) = func {
                // SAFETY: The interpreter lives as long as the callback.
                let interpreter = unsafe { &mut *interp_ptr };
                interpreter.call_function_by_id(fv.func_id, args)
            } else {
                Err(ExecutorError::type_error(
                    "Expected function value".to_string(),
                    vec![],
                ))
            }
        };
        let mut ctx = NativeContext::with_call_fn(&mut self.heap, &mut call_fn);
        self.ffi
            .call(func_name, &resolved, &mut ctx)
            .map_err(|e| e.with_stack(stack))
    }

    pub(super) fn call_static_by_name(
        &mut self,
        func_name: &str,
        call_args: &[RuntimeValue],
    ) -> ExecutorResult<RuntimeValue> {
        let mut resolved = Vec::with_capacity(call_args.len());
        for arg in call_args {
            resolved.push(self.force_value_clone(arg)?);
        }

        if self.ffi.has(func_name) {
            return self.call_native_by_name(func_name, &resolved);
        }

        let mut lookup_name = func_name.to_string();
        if !self.functions.contains_key(func_name) {
            let constructor_name = format!("{}_constructor", func_name);
            if self.functions.contains_key(&constructor_name) {
                lookup_name = constructor_name;
            }
        }

        if let Some(target_func) = self.functions.get(&lookup_name).cloned() {
            self.execute_function(&target_func, &resolved)
        } else {
            let stack = self.capture_stack();
            Err(ExecutorError::function_not_found(
                func_name.to_string(),
                stack,
            ))
        }
    }

    /// Execute a binary operation
    pub(super) fn exec_binary_op(
        &mut self,
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
        op: BinaryOp,
        frame: &mut Frame,
    ) -> ExecutorResult<()> {
        tlog!(
            debug,
            MSG::DebugRegisters,
            &frame.registers.len(),
            &(lhs.0 as usize),
            &(rhs.0 as usize)
        );
        let a = self.force_register(frame, lhs)?;
        let b = self.force_register(frame, rhs)?;

        tlog!(debug, MSG::DebugBinaryOp, &a, &b);

        tlog!(
            debug,
            MSG::DebugExecBinaryOp,
            &format!("{:?}, {:?}, {:?}", &a, &b, &op)
        );

        let result = match (op, a, b) {
            (BinaryOp::Add, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                tlog!(debug, MSG::DebugAddingNumbers, &l, &r);
                tlog!(debug, MSG::VmI64Add, &l, &r);
                RuntimeValue::Int(l + r)
            }
            (BinaryOp::Sub, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l - r),
            (BinaryOp::Mul, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l * r),
            (BinaryOp::Div, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                if r == 0 {
                    let stack = self.capture_stack();
                    return Err(ExecutorError::division_by_zero(stack));
                }
                RuntimeValue::Int(l / r)
            }
            (BinaryOp::Rem, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                if r == 0 {
                    let stack = self.capture_stack();
                    return Err(ExecutorError::division_by_zero(stack));
                }
                RuntimeValue::Int(l % r)
            }
            (BinaryOp::And, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l & r),
            (BinaryOp::Or, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l | r),
            (BinaryOp::Xor, RuntimeValue::Int(l), RuntimeValue::Int(r)) => RuntimeValue::Int(l ^ r),
            (BinaryOp::Shl, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(l << r)
            }
            (BinaryOp::Sar, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(l >> r)
            }
            (BinaryOp::Shr, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Int(l >> r)
            }
            (BinaryOp::Add, RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
                RuntimeValue::Float(l + r)
            }
            (BinaryOp::Sub, RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
                RuntimeValue::Float(l - r)
            }
            (BinaryOp::Mul, RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
                RuntimeValue::Float(l * r)
            }
            (BinaryOp::Div, RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
                RuntimeValue::Float(l / r)
            }
            (BinaryOp::Rem, RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
                RuntimeValue::Float(l % r)
            }
            (BinaryOp::Add, RuntimeValue::List(lhs_handle), RuntimeValue::List(rhs_handle)) => {
                let mut merged = Vec::new();

                if let Some(HeapValue::List(items)) = self.heap.get(lhs_handle) {
                    merged.extend(items.iter().cloned());
                }
                if let Some(HeapValue::List(items)) = self.heap.get(rhs_handle) {
                    merged.extend(items.iter().cloned());
                }

                let handle = self.heap.allocate(HeapValue::List(merged));
                RuntimeValue::List(handle)
            }
            _ => RuntimeValue::Unit,
        };

        frame.set_register(dst.0 as usize, result);
        Ok(())
    }

    /// Execute a comparison
    pub(super) fn exec_compare(
        &mut self,
        dst: Reg,
        lhs: Reg,
        rhs: Reg,
        cmp: CompareOp,
        frame: &mut Frame,
    ) -> ExecutorResult<()> {
        let a = self.force_register(frame, lhs)?;
        let b = self.force_register(frame, rhs)?;

        let result = match (cmp, &a, &b) {
            // Integer comparison
            (CompareOp::Eq, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l == r)
            }
            (CompareOp::Ne, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l != r)
            }
            (CompareOp::Lt, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l < r)
            }
            (CompareOp::Le, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l <= r)
            }
            (CompareOp::Gt, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l > r)
            }
            (CompareOp::Ge, RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
                RuntimeValue::Bool(l >= r)
            }
            // String comparison
            (CompareOp::Eq, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l == r)
            }
            (CompareOp::Ne, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l != r)
            }
            (CompareOp::Lt, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l < r)
            }
            (CompareOp::Le, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l <= r)
            }
            (CompareOp::Gt, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l > r)
            }
            (CompareOp::Ge, RuntimeValue::String(l), RuntimeValue::String(r)) => {
                RuntimeValue::Bool(l >= r)
            }
            _ => RuntimeValue::Bool(false),
        };

        frame.set_register(dst.0 as usize, result);
        Ok(())
    }
}

impl Drop for Interpreter {
    fn drop(&mut self) {
        if !self.shared.is_null() {
            unsafe {
                drop(Box::from_raw(self.shared as *mut SharedState));
            }
        }
    }
}
