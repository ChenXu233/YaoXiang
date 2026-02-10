# Task 8.7: 标准运行时入口

> **优先级**: P1
> **状态**: ⬜ 待开始
> **模块**: `src/middle/standard_runtime.rs`
> **依赖**: task-08-04-scheduler-trait, task-08-05-single-thread-scheduler, task-08-06-multi-thread-scheduler, phase-09-dag

## 功能描述

实现标准运行时入口，整合 DAG 调度器和泛型调度器接口。

### 核心职责

1. **VM 泛型实现**：`VM<S: Scheduler>` 通过泛型使用调度器
2. **字节码执行**：解释执行字节码指令
3. **并发支持**：spawn/await 指令处理
4. **DAG 集成**：与 phase-09-dag 集成

### 文件结构

```
src/middle/
├── mod.rs                  # 导出入口
├── standard_runtime.rs     # 标准运行时入口（本文档）
├── dag/                    # DAG 核心（phase-09）
│   ├── mod.rs
│   ├── graph.rs
│   └── ...
└── scheduler/
    ├── mod.rs
    ├── trait.rs
    ├── single_thread.rs
    └── multi_thread.rs
```

## 实现内容

### 1. StandardRuntime 结构

```rust
/// 标准运行时（泛型调度器）
pub struct StandardRuntime<S: Scheduler> {
    /// 调度器（泛型）
    scheduler: Arc<S>,
    /// 内存分配器
    allocator: SlabAllocator,
    /// 全局变量
    globals: HashMap<GlobalId, RuntimeValue>,
    /// 模块缓存
    modules: HashMap<ModuleId, CompiledModule>,
    /// 函数表
    functions: Vec<CompiledFunction>,
    /// VM（泛型）
    vm: VM<S>,
}
```

### 2. VM 结构（泛型约束）

```rust
/// 字节码虚拟机（泛型调度器）
struct VM<S: Scheduler> {
    /// 调度器
    scheduler: Arc<S>,
    /// 字节码
    bytecode: Bytecode,
    /// 寄存器
    registers: Vec<RuntimeValue>,
    /// 当前函数
    current_function: FunctionId,
    /// 指令指针
    ip: usize,
}

/// 字节码
struct Bytecode {
    instructions: Vec<BytecodeInstruction>,
    constants: Vec<RuntimeValue>,
    functions: Vec<FunctionInfo>,
}

/// 字节码指令
struct BytecodeInstruction {
    opcode: TypedOpcode,
    args: Vec<u32>,
}
```

### 3. 核心 API

```rust
impl<S: Scheduler> StandardRuntime<S> {
    /// 创建标准运行时
    pub fn new(scheduler: Arc<S>) -> Self {
        StandardRuntime {
            scheduler: scheduler.clone(),
            allocator: SlabAllocator::new(),
            globals: HashMap::new(),
            modules: HashMap::new(),
            functions: Vec::new(),
            vm: VM::new(scheduler),
        }
    }

    /// 加载并执行模块
    pub fn load_and_run(&mut self, module: &CompiledModule) -> Result<RuntimeValue, RuntimeError> {
        self.register_module(module);

        let main_func_id = self.find_function(module.id, "main")
            .ok_or(RuntimeError::MissingMain)?;

        let main_task = Arc::new(TaskImpl::new(main_func_id, vec![]));
        let task_id = self.scheduler.spawn(main_task);

        self.scheduler.await_task(task_id)
    }

    /// 执行函数
    pub fn execute(&mut self, func_id: FunctionId, args: Vec<RuntimeValue>) -> Result<RuntimeValue, RuntimeError> {
        self.vm.current_function = func_id;
        self.vm.registers = args;
        self.vm_run()
    }
}
```

### 4. 指令执行（关键指令）

```rust
impl<S: Scheduler> VM<S> {
    fn execute_instruction(&mut self, instr: &BytecodeInstruction) -> Result<(), RuntimeError> {
        match instr.opcode {
            TypedOpcode::Spawn => self.op_spawn(instr),
            TypedOpcode::Await => self.op_await(instr),
            // ... 其他指令
        }
    }

    /// Spawn 操作
    fn op_spawn(&mut self, instr: &BytecodeInstruction) -> Result<(), RuntimeError> {
        let func_id = FunctionId(instr.args[0]);
        let arg_count = instr.args[1] as usize;

        // 收集参数
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            args.push(self.registers.pop()
                .ok_or(RuntimeError::StackUnderflow)?);
        }
        args.reverse();

        // 创建任务
        let task = Arc::new(TaskImpl::new(func_id, args));

        // 提交到调度器
        let task_id = self.scheduler.spawn(task);

        // 将 Async 值压入栈
        let async_value = RuntimeValue::Async(AsyncValue {
            state: AsyncState::Pending(task_id),
            value_type: ValueType::Async(Box::new(...)),
        });

        self.registers.push(async_value);
        Ok(())
    }

    /// Await 操作
    fn op_await(&mut self, instr: &BytecodeInstruction) -> Result<(), RuntimeError> {
        let async_value = self.registers.pop()
            .ok_or(RuntimeError::StackUnderflow)?;

        let task_id = match &async_value {
            RuntimeValue::Async(AsyncValue { state, .. }) => match state {
                AsyncState::Pending(id) => *id,
                AsyncState::Ready(v) => {
                    self.registers.push(v.clone());
                    return Ok(());
                }
                AsyncState::Error(e) => return Err(RuntimeError::AsyncError(e.clone())),
            },
            _ => return Err(RuntimeError::TypeMismatch),
        };

        let result = self.scheduler.await_task(task_id)?;
        self.registers.push(result);
        Ok(())
    }
}
```

### 5. 创建辅助函数

```rust
/// 创建单线程运行时（异步调度）
pub fn create_single_thread_runtime() -> StandardRuntime<SingleThreadScheduler> {
    let scheduler = Arc::new(SingleThreadScheduler::new());
    StandardRuntime::new(scheduler)
}

/// 创建多线程运行时（并行调度）
pub fn create_multi_thread_runtime(num_workers: usize) -> StandardRuntime<MultiThreadScheduler> {
    let scheduler = Arc::new(MultiThreadScheduler::new(num_workers));
    StandardRuntime::new(scheduler)
}
```

## 验收测试

```rust
#[test]
fn test_single_thread_runtime() {
    let runtime = create_single_thread_runtime();
    let result = runtime.load_and_run(module);
    assert!(result.is_ok());
}

#[test]
fn test_multi_thread_runtime() {
    let runtime = create_multi_thread_runtime(4);
    let result = runtime.load_and_run(module);
    assert!(result.is_ok());
}

#[test]
fn test_spawn_await() {
    let runtime = create_single_thread_runtime();
    let result = runtime.load_and_run(spawn_await_module);
    assert_eq!(result, Ok(RuntimeValue::Int(42)));
}

#[test]
fn test_dag_dependencies() {
    let runtime = create_single_thread_runtime();
    let result = runtime.load_and_run(dag_module);
    // 验证按依赖顺序执行
    assert!(result.is_ok());
}
```

## 与 RFC-008 对照

| RFC-008 设计 | 实现 |
|-------------|------|
| 泛型 + 注入 | ✅ `StandardRuntime<S: Scheduler>` |
| VM 使用调度器 | ✅ `VM<S: Scheduler>` |
| spawn/await | ✅ 字节码指令 |
| DAG 集成 | ✅ 与 phase-09-dag 集成 |
| 单线程异步 | ✅ SingleThreadScheduler |
| 多线程并行 | ✅ MultiThreadScheduler |

## 依赖关系

```
task-08-04 ──► task-08-05 ──┐
                            ├──► task-08-07 ──► phase-09-dag
task-08-06 ────────────────┘
```
