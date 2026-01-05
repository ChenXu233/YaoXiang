# YaoXiang 异步实现方案

> 版本：v1.0.0
> 状态：草稿
> 日期：2025-01-03
> 基于：《象流：基于惰性求值的无感异步并发模型》技术白皮书

---

## 一、概述

本文档基于《象流》白皮书的设计理念，结合 YaoXiang 项目现有的编译器架构，制定完整的异步实现方案。目标是在现有代码基础上，逐步实现"同步语法、异步本质"的无感并发模型。

### 1.1 象流模型核心目标

| 目标 | 当前状态 | 实现难度 |
|------|----------|----------|
| 惰性计算图构建 | 未实现 | ⭐⭐⭐⭐ |
| Async[T] 代理类型 | 部分支持 | ⭐⭐⭐ |
| 自动等待机制 | 未实现 | ⭐⭐⭐⭐ |
| 并发调度器 | 基础框架 | ⭐⭐ |
| 工作窃取优化 | 基础框架 | ⭐⭐⭐ |

---

## 二、现有架构分析

### 2.1 编译器前端

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         YaoXiang 编译器前端                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  lexer/mod.rs ───► tokens.rs                                            │
│      │                                                                   │
│      ▼                                                                   │
│  parser/mod.rs ───► ast.rs (已含 is_async 字段)                         │
│      │                                                                   │
│      ├──► nud.rs (表达式解析)                                           │
│      ├──► led.rs (中缀解析)                                             │
│      ├──► stmt.rs (语句解析)                                            │
│      └──► type_parser.rs (类型解析)                                     │
│                                                                          │
│      ▼                                                                   │
│  typecheck/mod.rs ───► types.rs (MonoType 含 is_async)                 │
│      │                                                                   │
│      ├──► infer.rs (类型推断)                                           │
│      ├──► check.rs (类型检查)                                           │
│      └──► specialize.rs (泛型特化)                                       │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

**现有异步支持**：
- [lexer/mod.rs:568](src/frontend/lexer/mod.rs#L568) - `spawn` 关键字已定义
- [ast.rs:32](src/frontend/parser/ast.rs#L32) - `Expr::FnDef` 包含 `is_async: bool`
- [types.rs:89](src/frontend/typecheck/types.rs#L89) - `MonoType::Fn` 包含 `is_async: bool`

### 2.2 编译器中端

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         YaoXiang 编译器中端                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  middle/ir.rs ───► IR 中间表示 (含 CallAsync)                           │
│      │                                                                   │
│      ├──► codegen/ (字节码生成)                                         │
│      │       ├──► expr.rs                                               │
│      │       ├──► stmt.rs                                               │
│      │       └──► control_flow.rs                                       │
│      │                                                                   │
│      ├──► monomorphize/ (单态化)                                        │
│      ├──► optimizer/ (优化器)                                           │
│      ├──► lifetime/ (生命周期)                                          │
│      └──► escape_analysis/ (逃逸分析)                                   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### 2.3 虚拟机与运行时

```
┌─────────────────────────────────────────────────────────────────────────┐
│                      YaoXiang VM 与运行时                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  vm/mod.rs ───► 虚拟机核心                                               │
│      │                                                                   │
│      ├──► executor.rs (指令执行，含 CallAsync)                          │
│      ├──► opcode.rs (操作码定义)                                        │
│      ├──► instructions.rs (指令实现)                                    │
│      ├──► frames.rs (调用帧管理)                                        │
│      └──► inline_cache.rs (内联缓存)                                    │
│                                                                          │
│  runtime/mod.rs ───► 运行时核心                                         │
│      │                                                                   │
│      ├──► scheduler/mod.rs (任务调度器)                                 │
│      │       └──► work-stealing 队列实现                                │
│      │                                                                   │
│      ├──► memory/mod.rs (内存管理)                                      │
│      └──► gc/mod.rs (垃圾回收)                                          │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## 三、异步实现方案

### 3.1 第一阶段：基础框架完善

#### 3.1.1 任务描述符设计

```rust
// runtime/scheduler/task.rs (新建)

use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::collections::LinkedList;

/// 任务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Created,      // 已创建
    Ready,        // 就绪待执行
    Running,      // 正在执行
    Waiting,      // 等待资源
    Suspended,    // 挂起（等待异步结果）
    Finished,     // 完成
    Failed,       // 失败
}

/// 任务标识
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(usize);

impl TaskId {
    pub fn new(id: usize) -> Self { Self(id) }
    pub fn as_usize(&self) -> usize { self.0 }
}

/// 任务描述符 - 象流模型的核心数据结构
#[derive(Debug)]
pub struct TaskDescriptor {
    /// 任务 ID
    id: TaskId,
    /// 任务状态
    state: AtomicTaskState,
    /// 入口函数 (编译后的闭包 ID)
    entry_fn: usize,
    /// 参数槽
    args: Vec<Value>,
    /// 依赖任务列表 (用于惰性计算图)
    dependencies: Vec<TaskId>,
    /// 父任务 ID
    parent: Option<TaskId>,
    /// 栈大小
    stack_size: usize,
    /// 优先级
    priority: TaskPriority,
    /// 调度策略
    schedule_policy: SchedulePolicy,
}

impl TaskDescriptor {
    pub fn new(id: TaskId, entry_fn: usize, args: Vec<Value>) -> Self {
        Self {
            id,
            state: AtomicTaskState::new(TaskState::Created),
            entry_fn,
            args,
            dependencies: Vec::new(),
            parent: None,
            stack_size: DEFAULT_STACK_SIZE,
            priority: TaskPriority::Normal,
            schedule_policy: SchedulePolicy::Lazy,
        }
    }

    /// 添加依赖
    pub fn add_dependency(&mut self, task_id: TaskId) {
        self.dependencies.push(task_id);
    }
}

/// 原子任务状态
#[derive(Debug)]
pub struct AtomicTaskState(AtomicUsize);

impl AtomicTaskState {
    pub fn new(state: TaskState) -> Self {
        Self(AtomicUsize::new(state as usize))
    }

    pub fn load(&self) -> TaskState {
        TaskState::from_usize(self.0.load(Ordering::SeqCst))
    }

    pub fn store(&self, state: TaskState) {
        self.0.store(state as usize, Ordering::SeqCst);
    }
}

/// 调度策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulePolicy {
    Lazy,        // 惰性调度 (按需)
    Eager,       // 急切调度 (立即执行)
    Parallel,    // 并行调度 (强制并行)
    Blocking,    // 阻塞调度 (同步执行)
}

/// 任务优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum TaskPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}
```

#### 3.1.2 惰性值类型设计

```rust
// runtime/async/lazy_value.rs (新建)

use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::fmt::Debug;

/// 惰性值状态
enum LazyState<T> {
    Pending(TaskId),           // 等待任务完成
    Computing,                 // 正在计算中
    Ready(T),                  // 已就绪
    Failed(Arc<dyn std::error::Error + Send + Sync>),  // 计算失败
}

/// Async[T] - 惰性代理类型
///
/// 这是象流模型的核心！它是一个透明代理：
/// - 类型系统中是 T 的子类型
/// - 实际使用时触发自动等待
/// - 实现了 Send + Sync (线程安全)
pub struct Async<T: Send + Sync> {
    state: Arc<Mutex<LazyState<T>>>,
    ready: Arc<AtomicBool>,
}

impl<T: Send + Sync> Async<T> {
    /// 创建异步任务
    pub fn new(task_id: TaskId) -> Self {
        Self {
            state: Arc::new(Mutex::new(LazyState::Pending(task_id))),
            ready: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 标记为正在计算
    pub fn set_computing(&self) {
        let mut state = self.state.lock().unwrap();
        if let LazyState::Pending(_) = *state {
            *state = LazyState::Computing;
        }
    }

    /// 设置结果
    pub fn set_result(&self, value: T) {
        let mut state = self.state.lock().unwrap();
        *state = LazyState::Ready(value);
        self.ready.store(true, Ordering::SeqCst);
    }

    /// 设置错误
    pub fn set_error(&self, error: Arc<dyn std::error::Error + Send + Sync>) {
        let mut state = self.state.lock().unwrap();
        *state = LazyState::Failed(error);
        self.ready.store(true, Ordering::SeqCst);
    }

    /// 等待并获取值 (阻塞调用)
    ///
    /// 这是自动等待机制的关键！
    /// 当代码需要具体值时，调用此方法会阻塞当前协程，
    /// 调度器会自动切换到其他就绪任务。
    pub fn await(&self) -> Result<T, Arc<dyn std::error::Error + Send + Sync>> {
        // 第一次调用时，注册到调度器
        let mut first_access = true;

        loop {
            let state = self.state.lock().unwrap();
            match &*state {
                LazyState::Ready(value) => {
                    return Ok(value.clone());
                }
                LazyState::Failed(error) => {
                    return Err(error.clone());
                }
                LazyState::Pending(task_id) | LazyState::Computing => {
                    // 注册等待并让出控制权
                    if first_access {
                        Scheduler::current().register_waiter(*task_id, self);
                        first_access = false;
                    }
                }
            }
            drop(state);

            // 让出控制权给调度器
            Scheduler::current().yield_now();
        }
    }

    /// 非阻塞检查是否就绪
    pub fn is_ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }
}

/// 从普通值创建 Async
impl<T: Send + Sync> From<T> for Async<T> {
    fn from(value: T) -> Self {
        let this = Self::new(TaskId::new(0));
        this.set_result(value);
        this
    }
}
```

#### 3.1.3 调度器增强

```rust
// runtime/scheduler/mod.rs (增强)

use super::{TaskDescriptor, TaskId, TaskState, TaskPriority, SchedulePolicy};
use std::collections::{VecDeque, HashMap, HashSet};
use std::sync::{Arc, Mutex, Condvar};
use std::thread;

/// 工作窃取调度器
pub struct WorkStealingScheduler {
    /// 全局配置
    config: SchedulerConfig,
    /// 全局就绪队列
    global_queue: Arc<Mutex<VecDeque<Arc<TaskDescriptor>>>>,
    /// 工作者本地队列
    local_queues: Vec<Arc<Mutex<VecDeque<Arc<TaskDescriptor>>>>>,
    /// 等待队列 (按任务 ID 索引)
    waiting_queue: Arc<Mutex<HashMap<TaskId, Vec<Arc<dyn Waitable + Send + Sync>>>>>,
    /// 任务状态查询
    task_states: Arc<Mutex<HashMap<TaskId, Arc<AtomicTaskState>>>>,
    /// 完成通知
    completion_notifier: Arc<Condvar>,
    /// 运行标志
    running: Arc<AtomicBool>,
    /// 工作者线程
    workers: Vec<JoinHandle<()>>,
}

impl WorkStealingScheduler {
    /// 注册等待者
    pub fn register_waiter(&self, task_id: TaskId, waiter: &Arc<dyn Waitable + Send + Sync>) {
        let mut waiting = self.waiting_queue.lock().unwrap();
        waiting
            .entry(task_id)
            .or_insert_with(Vec::new)
            .push(waiter.clone());
    }

    /// 通知任务完成
    pub fn notify_task_completed(&self, task_id: TaskId) {
        // 唤醒所有等待此任务的等待者
        let mut waiting = self.waiting_queue.lock().unwrap();
        if let Some(waiters) = waiting.remove(&task_id) {
            for waiter in waiters {
                waiter.wake();
            }
        }

        // 如果有父任务，更新依赖
        if let Some(parent_id) = self.get_parent(task_id) {
            self.update_dependency(parent_id);
        }
    }

    /// 更新依赖计数
    fn update_dependency(&self, parent_id: TaskId) {
        // 检查父任务的所有依赖是否都已完成
        // 如果都完成，将父任务加入就绪队列
    }

    /// 让出控制权 (协程切换)
    pub fn yield_now(&self) {
        // 将当前协程挂起，加入等待队列
        // 唤醒一个就绪任务
    }

    /// 惰性获取任务 (支持工作窃取)
    fn steal_or_get(
        &self,
        worker_id: usize,
        local_queue: &Arc<Mutex<VecDeque<Arc<TaskDescriptor>>>>,
    ) -> Option<Arc<TaskDescriptor>> {
        // 1. 尝试从本地队列获取
        if let Some(task) = local_queue.lock().unwrap().pop_front() {
            return Some(task);
        }

        // 2. 尝试从全局队列获取
        if let Some(task) = self.global_queue.lock().unwrap().pop_front() {
            return Some(task);
        }

        // 3. 工作窃取 - 随机选择其他工作者的本地队列
        let num_workers = self.local_queues.len();
        let mut attempts = 0;
        while attempts < num_workers {
            let victim_id = (worker_id + 1 + attempts) % num_workers;
            let victim_queue = &self.local_queues[victim_id];

            if let Some(task) = Self::steal_from(victim_queue) {
                return Some(task);
            }
            attempts += 1;
        }

        None
    }

    /// 从队列窃取任务 (从队尾获取，保持 LIFO)
    fn steal_from(queue: &Arc<Mutex<VecDeque<Arc<TaskDescriptor>>>>) -> Option<Arc<TaskDescriptor>> {
        let mut queue = queue.lock().unwrap();
        queue.pop_back()
    }
}

/// 等待者 trait
pub trait Waitable {
    fn wake(&self);
}
```

---

### 3.2 第二阶段：编译器集成

#### 3.2.1 词法与语法分析

```rust
// frontend/lexer/tokens.rs (扩展)

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenKind {
    // ... 现有关键字 ...

    /// 异步标记 (现有)
    KwSpawn,

    /// 并发构造块 (新增)
    KwSpawnBlock,

    /// 阻塞注解 (新增)
    KwBlocking,

    /// 急切求值注解 (新增)
    KwEager,
}
```

```rust
// frontend/parser/expr.rs (扩展)

/// 解析 spawn 表达式
fn parse_spawn_expr(&mut self) -> Result<Expr> {
    // 语法: spawn { ... } | spawn for ... | spawn fn ...

    if self.consume_if(TokenKind::KwFn) {
        // spawn fn - 异步函数定义
        return self.parse_async_fn_def();
    }

    if self.consume_if(TokenKind::KwFor) {
        // spawn for - 数据并行循环
        return self.parse_parallel_for();
    }

    if self.consume_if(TokenKind::LBrace) {
        // spawn { ... } - 并发构造块
        return self.parse_spawn_block();
    }

    Err(ParserError::Expected("spawn { } | spawn fn | spawn for"))
}

/// 解析并发构造块
fn parse_spawn_block(&mut self) -> Result<Expr> {
    let mut expressions = Vec::new();

    while !self.consume_if(TokenKind::RBrace) {
        if self.at_end() {
            return Err(ParserError::UnclosedBlock);
        }
        expressions.push(self.parse_expr()?);

        if !self.consume_if(TokenKind::Comma) {
            break;
        }
    }

    Ok(Expr::SpawnBlock {
        expressions,
        span: self.current_span(),
    })
}
```

#### 3.2.2 类型系统增强

```rust
// frontend/typecheck/types.rs (增强)

/// MonoType - 单态化后的类型
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum MonoType {
    /// 原类型
    Primitive(PrimitiveType),
    /// 构造器类型
    Constructor {
        name: Symbol,
        args: Box<Vec<MonoType>>,
    },
    /// 函数类型 (扩展：添加 spawn 信息)
    Fn {
        params: Box<Vec<MonoType>>,
        return_type: Box<MonoType>,
        is_async: bool,          // 标记是否为 spawn 函数
        spawn_policy: Option<SpawnPolicy>,  // 并发策略
    },
    /// 元类型 (类型作为值)
    MetaType(Box<MonoType>),
    /// 异步代理类型 (新增)
    Async {
        inner: Box<MonoType>,
        policy: SpawnPolicy,
    },
    /// 引用类型
    Ref {
        inner: Box<MonoType>,
        mutable: bool,
    },
}

/// 并发策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpawnPolicy {
    Lazy,        // 惰性调度
    Eager,       // 急切调度
    Parallel,    // 强制并行
    Blocking,    // 阻塞执行
}

/// Async[T] 子类型规则
impl MonoType {
    /// 检查类型是否可隐式转换为目标类型
    pub fn is_subtype_of(&self, target: &MonoType) -> bool {
        match (self, target) {
            // Async<T> 是 T 的子类型 (象流模型核心!)
            (MonoType::Async { inner, .. }, other) => inner.as_ref().is_subtype_of(other),
            // 同类型直接匹配
            _ => self == target,
        }
    }
}
```

#### 3.2.3 中间表示扩展

```rust
// middle/ir.rs (扩展)

/// IR 指令
#[derive(Debug, Clone, PartialEq)]
pub enum IRInstruction {
    // ... 现有指令 ...

    /// 创建异步任务 (新增)
    Spawn {
        /// 任务函数
        func: IRValue,
        /// 参数
        args: Vec<IRValue>,
        /// 产出值的目标位置
        target: IRTarget,
        /// 调度策略
        policy: SchedulePolicy,
    },

    /// 等待异步结果 (新增)
    Await {
        /// 要等待的 Async 值
        value: IRValue,
        /// 结果存放位置
        target: IRTarget,
    },

    /// 并发块 (新增)
    SpawnBlock {
        /// 块中的表达式
        expressions: Vec<IRInstruction>,
        /// 结果类型
        result_type: MonoType,
    },

    /// 数据并行循环 (新增)
    ParallelFor {
        /// 迭代变量
        iterator: Symbol,
        /// 范围
        range: (IRValue, IRValue),
        /// 循环体
        body: Box<IRInstruction>,
        /// 调度策略
        policy: SchedulePolicy,
    },

    /// 挂起当前协程 (新增)
    Suspend,

    /// 恢复协程 (新增)
    Resume {
        task_id: IRValue,
    },
}

/// 惰性计算图节点
#[derive(Debug, Clone)]
pub struct ComputationNode {
    /// 节点 ID
    id: NodeId,
    /// 节点类型
    kind: NodeKind,
    /// 依赖节点
    dependencies: Vec<NodeId>,
    /// 是否可并行
    parallelizable: bool,
    /// 估计计算成本
    estimated_cost: u64,
}

/// 节点类型
#[derive(Debug, Clone)]
pub enum NodeKind {
    /// 计算节点
    Computation(IRInstruction),
    /// I/O 节点
    IO(IOOperation),
    /// 同步屏障
    Barrier,
    /// 条件分支
    Branch {
        condition: NodeId,
        then_branch: Vec<NodeId>,
        else_branch: Vec<NodeId>,
    },
    /// 循环
    Loop {
        body: Vec<NodeId>,
        max_iterations: Option<usize>,
    },
}
```

#### 3.2.4 字节码生成

```rust
// middle/codegen/mod.rs (增强)

impl CodeGenerator {
    /// 生成 spawn 表达式的字节码
    fn compile_spawn(&mut self, expr: &Expr) -> Result<Vec<ByteCode>> {
        match expr {
            Expr::SpawnBlock { expressions, .. } => {
                // 并发构造块生成
                self.compile_spawn_block(expressions)
            }
            Expr::ParallelFor { iterator, range, body, .. } => {
                // 数据并行循环生成
                self.compile_parallel_for(iterator, range, body)
            }
            _ => Err(CodegenError::InvalidSpawnExpr),
        }
    }

    /// 编译并发构造块
    fn compile_spawn_block(&mut self, expressions: &[Expr]) -> Result<Vec<ByteCode>> {
        let mut codes = Vec::new();

        // 1. 为每个表达式创建异步任务
        for (idx, expr) in expressions.iter().enumerate() {
            codes.push(ByteCode::Spawn {
                func: self.compile_expr(expr)?,
                target: idx,  // 结果存入指定槽位
                policy: SchedulePolicy::Eager,
            });
        }

        // 2. 生成同步屏障 (等待所有任务完成)
        codes.push(ByteCode::Barrier {
            count: expressions.len(),
        });

        // 3. 收集结果
        codes.push(ByteCode::CollectResults {
            count: expressions.len(),
        });

        Ok(codes)
    }

    /// 编译并行 for 循环
    fn compile_parallel_for(
        &mut self,
        iterator: &Symbol,
        range: &(Expr, Expr),
        body: &Expr,
    ) -> Result<Vec<ByteCode>> {
        let mut codes = Vec::new();

        // 1. 计算范围
        let start = self.compile_expr(&range.0)?;
        let end = self.compile_expr(&range.1)?;

        // 2. 计算迭代次数和分片
        let iterations = self.calculate_iterations(&start, &end)?;

        // 3. 工作窃取并行化
        codes.push(ByteCode::ParallelFor {
            iterator: *iterator,
            start,
            end,
            body: self.compile_expr(body)?,
            chunk_size: self.calculate_optimal_chunk_size(iterations),
            policy: SchedulePolicy::Parallel,
        });

        Ok(codes)
    }
}
```

---

### 3.3 第三阶段：虚拟机与运行时

#### 3.3.1 VM 指令扩展

```rust
// vm/opcode.rs (扩展)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Opcode {
    // ... 现有指令 ...

    // === 异步指令 (新增) ===

    /// 创建异步任务
    ///
    /// 操作数: [func_id: u32, args_count: u16, target_slot: u16, policy: u8]
    /// 行为:
    ///   1. 创建任务描述符
    ///   2. 根据 policy 决定调度时机
    ///   3. 返回 Async[T] 句柄
    Spawn,

    /// 等待异步结果
    ///
    /// 操作数: [async_handle: u32, target_slot: u16]
    /// 行为:
    ///   1. 检查 Async[T] 是否就绪
    ///   2. 如果未就绪，挂起当前协程
    ///   3. 调度器切换到其他就绪任务
    Await,

    /// 并发屏障 (等待所有任务)
    ///
    /// 操作数: [task_handles: u32...]
    Barrier,

    /// 挂起当前协程
    ///
    /// 无操作数
    /// 行为:
    ///   1. 保存协程状态到帧
    ///   2. 将协程加入等待队列
    ///   3. 让出控制权
    Suspend,

    /// 恢复协程
    ///
    /// 操作数: [task_id: u32]
    Resume,

    /// 并行 for 循环
    ///
    /// 操作数: [iterator_slot: u16, start_slot: u16, end_slot: u16, body_func_id: u32]
    ParallelFor,

    /// 检查并切换 (用于轮询 I/O)
    ///
    /// 操作数: [async_handle: u32]
    Poll,

    /// 设置 Async 结果
    ///
    /// 操作数: [async_handle: u32, value_slot: u16]
    SetAsyncResult,

    /// 设置 Async 错误
    ///
    /// 操作数: [async_handle: u32, error_slot: u16]
    SetAsyncError,
}
```

```rust
// vm/executor.rs (扩展)

impl Executor {
    /// 执行 Spawn 指令
    fn exec_spawn(&mut self, func_id: u32, args_count: u16, target_slot: u16, policy: u8) -> VMResult<()> {
        // 1. 获取函数闭包
        let func = self.get_closure(func_id)?;

        // 2. 收集参数
        let args = self.pop_args(args_count)?;

        // 3. 创建任务描述符
        let task_id = self.scheduler.next_task_id();
        let task = TaskDescriptor::new(task_id, func_id, args);

        // 4. 设置调度策略
        task.set_schedule_policy(SchedulePolicy::from_u8(policy));

        // 5. 创建 Async[T] 句柄
        let async_handle = self.vm.alloc_async(task_id);

        // 6. 提交给调度器
        match SchedulePolicy::from_u8(policy) {
            SchedulePolicy::Eager => {
                self.scheduler.spawn_task(task);
            }
            SchedulePolicy::Lazy => {
                // 惰性调度：只创建任务，延迟执行
                self.vm.add_lazy_task(async_handle, task);
            }
            SchedulePolicy::Parallel => {
                // 强制并行：立即并行执行
                self.scheduler.spawn_parallel(task);
            }
            SchedulePolicy::Blocking => {
                // 阻塞执行：同步执行
                self.scheduler.execute_blocking(task);
            }
        }

        // 7. 将 Async 句柄存入目标槽位
        self.set_slot(target_slot, Value::Async(async_handle));

        Ok(())
    }

    /// 执行 Await 指令
    fn exec_await(&mut self, async_handle: u32, target_slot: u16) -> VMResult<()> {
        // 1. 获取 Async 结构
        let async_obj = self.vm.get_async(async_handle)?;

        // 2. 非阻塞检查
        if async_obj.is_ready() {
            // 已就绪，直接获取结果
            let result = async_obj.get_result()?;
            self.set_slot(target_slot, result);
            return Ok(());
        }

        // 3. 未就绪，挂起当前协程
        self.current_frame().state = FrameState::Suspended;

        // 4. 注册回调 (任务完成时恢复)
        async_obj.on_ready(Box::new(move |result| {
            // 恢复挂起的协程
            scheduler.resume_task(task_id, result);
        }));

        // 5. 调度其他任务
        self.scheduler.schedule_next();

        Ok(())
    }

    /// 执行 Barrier 指令
    fn exec_barrier(&mut self, task_handles: &[u32]) -> VMResult<()> {
        // 1. 收集所有任务
        let tasks: Vec<_> = task_handles
            .iter()
            .map(|&h| self.vm.get_async(h))
            .collect::<Result<_, _>>()?;

        // 2. 等待所有任务完成
        for task in tasks {
            if !task.is_ready() {
                self.current_frame().state = FrameState::Suspended;
                task.on_ready(Box::new(|_| {
                    // 恢复时重新检查
                }));
                self.scheduler.schedule_next();
            }
        }

        // 3. 收集所有结果
        let results: Vec<Value> = task_handles
            .iter()
            .map(|&h| self.vm.get_async(h)?.get_result())
            .collect::<Result<_, _>>()?;

        // 4. 将结果打包为元组
        self.set_slot_from_values(target_slot, results);

        Ok(())
    }

    /// 执行 ParallelFor 指令
    fn exec_parallel_for(
        &mut self,
        iterator_slot: u16,
        start_slot: u16,
        end_slot: u16,
        body_func_id: u32,
    ) -> VMResult<()> {
        // 1. 获取范围
        let start = self.get_slot::<i64>(start_slot)?;
        let end = self.get_slot::<i64>(end_slot)?;

        // 2. 计算迭代次数
        let iterations = (end - start) as usize;

        // 3. 创建工作窃取任务
        let chunk_size = self.calculate_optimal_chunk_size(iterations);
        let chunks = (iterations + chunk_size - 1) / chunk_size;

        // 4. 为每个分片创建任务
        for chunk_id in 0..chunks {
            let chunk_start = start + (chunk_id * chunk_size) as i64;
            let chunk_end = (chunk_start + chunk_size as i64).min(end);

            self.scheduler.spawn_task(TaskDescriptor::new(
                self.scheduler.next_task_id(),
                body_func_id,
                vec![
                    Value::Int(chunk_start),
                    Value::Int(chunk_end),
                ],
            ));
        }

        // 5. 等待所有分片完成
        self.exec_barrier(&self.vm.get_current_chunk_handles())?;

        Ok(())
    }
}
```

#### 3.3.2 协程状态管理

```rust
// vm/frames.rs (扩展)

/// 调用帧状态
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameState {
    /// 正常运行
    Running,
    /// 挂起等待 (新增)
    Suspended,
    /// 完成
    Finished,
}

/// 协程上下文
#[derive(Debug)]
pub struct CoroutineContext {
    /// 所属任务 ID
    task_id: TaskId,
    /// 当前帧
    current_frame: Frame,
    /// 帧栈
    frame_stack: Vec<Frame>,
    /// 协程局部存储
    local_storage: HashMap<Symbol, Value>,
    /// 挂起点列表 (用于恢复)
    suspend_points: Vec<SuspendPoint>,
}

/// 挂起点
#[derive(Debug, Clone)]
pub struct SuspendPoint {
    /// 指令指针
    ip: usize,
    /// 局部变量快照
    locals: Vec<(Symbol, Value)>,
    /// 恢复后继续执行的指令
    resume_ip: usize,
    /// 等待的 async 句柄
    waiting_for: Vec<u32>,
}

impl CoroutineContext {
    /// 创建挂起点
    pub fn create_suspend_point(&mut self, ip: usize) -> SuspendPoint {
        let locals = self.capture_locals();
        let suspend_point = SuspendPoint {
            ip,
            locals,
            resume_ip: ip + 1,
            waiting_for: Vec::new(),
        };
        self.suspend_points.push(suspend_point.clone());
        suspend_point
    }

    /// 捕获局部变量
    fn capture_locals(&self) -> Vec<(Symbol, Value)> {
        self.current_frame.locals.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// 恢复执行
    pub fn resume(&mut self, result: Value) -> VMResult<()> {
        // 1. 恢复局部变量
        for (symbol, value) in &self.suspend_points.last().unwrap().locals {
            self.current_frame.locals.insert(symbol.clone(), value.clone());
        }

        // 2. 恢复指令指针
        self.current_frame.ip = self.suspend_points.last().unwrap().resume_ip;

        // 3. 将结果存入指定位置
        let slot = self.current_frame.ip;  // 约定结果在 Await 后面的槽位
        self.current_frame.slots[slot] = result;

        // 4. 清除挂起点
        self.suspend_points.pop();

        Ok(())
    }
}
```

---

### 3.4 第四阶段：标准库与工具

#### 3.4.1 并发标准库

```yaoxiang
# std/concurrent.yx

# === 基础并发原语 ===

# 并行执行多个异步任务
pub parallel[T](List[Async[T]]) -> List[T] = (tasks) => {
    # 等待所有任务完成
    # 返回结果列表
}

# 等待任意一个任务完成
pub await_any[T](List[Async[T]]) -> T = (tasks) => {
    # 返回第一个完成的任务结果
}

# 超时等待
pub with_timeout[T](Async[T], Duration) -> Option[T] = (task, timeout) => {
    # 在指定时间内等待任务完成
    # 超时返回 None
}

# === 异步工具 ===

# 延迟执行
pub sleep(Duration) -> Void = (duration) => {
    # 异步睡眠，不阻塞线程
}

# 异步读取文件
pub read_file(String) -> String spawn = (path) => {
    # 异步读取文件内容
}

# 异步 HTTP 请求
pub http_get(String) -> JSON spawn = (url) => {
    # 异步 HTTP GET 请求
}
```

```rust
// std/concurrent.rs (增强)

/// 并行执行多个异步任务
pub async fn parallel<T: Send + Sync>(
    tasks: Vec<Async<T>>,
) -> Vec<T> {
    // 使用 Barrier 等待所有任务
    let barrier = Arc::new(Barrier::new(tasks.len()));
    let results = Arc::new(Mutex::new(Vec::with_capacity(tasks.len())));

    for (i, task) in tasks.into_iter().enumerate() {
        let barrier = barrier.clone();
        let results = results.clone();

        thread::spawn(move || {
            let result = task.await();
            let mut results = results.lock().unwrap();
            results.insert(i, result);
            barrier.wait();
        });
    }

    barrier.wait();

    // 返回排序后的结果
    results.into_inner().unwrap()
}

/// 超时等待
pub async fn with_timeout<T>(
    task: Async<T>,
    timeout: Duration,
) -> Option<T> {
    // 使用 select! 模式
    // 超时返回 None
}
```

#### 3.4.2 调试与可视化

```rust
// tools/debug/computation_graph.rs (新建)

/// 计算图可视化工具
pub struct ComputationGraphDebugger {
    /// 所有节点
    nodes: HashMap<NodeId, ComputationNode>,
    /// 节点状态
    node_states: HashMap<NodeId, NodeState>,
    /// 任务到节点的映射
    task_to_node: HashMap<TaskId, NodeId>,
}

impl ComputationGraphDebugger {
    /// 生成 DOT 格式图
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph ComputationGraph {\n");
        dot += "    rankdir=TB;\n";
        dot += "    node [shape=box, style=filled];\n\n";

        for (id, node) in &self.nodes {
            let state = self.node_states.get(id).unwrap_or(&NodeState::Pending);
            let color = state.color();
            dot += &format!("    n{} [label=\"{}\", fillcolor=\"{}\"];\n", id, node.name(), color);
        }

        for (id, node) in &self.nodes {
            for dep in &node.dependencies {
                dot += &format!("    n{} -> n{};\n", dep, id);
            }
        }

        dot += "}\n";
        dot
    }

    /// 生成 HTML 可视化页面
    pub fn to_html(&self) -> String {
        // 使用 D3.js 渲染交互式计算图
        html_template(&self.to_dot())
    }
}
```

---

## 四、实现路线图

### 4.1 版本规划

| 版本 | 阶段 | 主要内容 | 状态 |
|------|------|----------|------|
| v1.0.0 | 基础 | 现有框架 (spawn 关键字、is_async 标志) | ✅ 完成 |
| v1.1.0 | 第一阶段 | 任务描述符、Async[T] 类型、调度器增强 | 🚧 进行中 |
| v1.2.0 | 第二阶段 | 词法/语法分析增强、类型系统扩展、IR 扩展 | ⏳ 待开始 |
| v1.3.0 | 第三阶段 | VM 指令扩展、协程状态管理 | ⏳ 待开始 |
| v1.4.0 | 第四阶段 | 标准库完善、调试工具 | ⏳ 待开始 |
| v2.0.0 | 完整版 | 惰性计算图、工作窃取优化 | ⏳ 待开始 |

### 4.2 关键里程碑

```
里程碑 1: 基础框架 (v1.1.0)
├── [ ] 任务描述符 TaskDescriptor
├── [ ] 原子任务状态 AtomicTaskState
├── [ ] Async[T] 惰值类型
├── [ ] 调度器增强 (注册等待者、通知完成)
└── [ ] 单元测试覆盖率 > 60%

里程碑 2: 编译器集成 (v1.2.0)
├── [ ] 解析 spawn { } 和 spawn for
├── [ ] 类型系统支持 Async[T] 子类型
├── [ ] IR 扩展 (Spawn, Await, Barrier)
├── [ ] 字节码生成
└── [ ] 集成测试覆盖率 > 70%

里程碑 3: VM 运行时 (v1.3.0)
├── [ ] VM 异步指令实现
├── [ ] 协程上下文与挂起/恢复
├── [ ] 工作窃取调度实现
├── [ ] 性能测试
└── [ ] 基准测试 > 现有实现 2x

里程碑 4: 工具生态 (v1.4.0)
├── [ ] 并发标准库
├── [ ] 计算图可视化调试器
├── [ ] 性能分析工具
└── [ ] 文档完善
```

---

## 五、方案评估

### 5.1 优势分析

#### 5.1.1 设计层面的优势

| 特性 | 象流模型 | 传统 async/await | 评估 |
|------|----------|------------------|------|
| **语法简洁度** | ⭐⭐⭐⭐⭐ 无需额外关键字 | ⭐⭐⭐ 需要 async/await | 象流胜出 |
| **类型透明性** | ⭐⭐⭐⭐⭐ Async[T] 即 T | ⭐⭐⭐ Future<T> ≠ T | 象流胜出 |
| **心智模型** | ⭐⭐⭐⭐ 同步思维 | ⭐⭐⭐⭐ 需理解异步流 | 象流胜出 |
| **学习曲线** | ⭐⭐⭐⭐ 接近自然思维 | ⭐⭐⭐ 需学习异步概念 | 象流胜出 |
| **性能优化** | ⭐⭐⭐⭐⭐ 惰性优化 | ⭐⭐⭐ 需手动优化 | 象流胜出 |

#### 5.1.2 技术层面的优势

1. **零异步传染**：
   - `spawn fn` 函数与普通函数类型签名完全一致
   - 调用方无需知道被调用方是否为异步
   - 彻底消除 JavaScript/TypeScript 中的 "async 传染" 问题

2. **自动并行优化**：
   - 编译器分析数据依赖，构建计算图
   - 运行时根据依赖关系自动并行化
   - 无需开发者显式使用 `Promise.all()` 等

3. **惰性求值优势**：
   - 只执行真正需要的计算
   - 短路不必要的工作
   - 适合深层嵌套的表达式

#### 5.1.3 现有实现基础

| 组件 | 已有实现 | 需要扩展 | 评估 |
|------|----------|----------|------|
| 词法分析器 | ✅ `spawn` 关键字 | `spawn { }`/`spawn for` | 较小改动 |
| 语法分析器 | ✅ AST 含 `is_async` | 并发块解析 | 中等改动 |
| 类型系统 | ✅ `MonoType::Fn` 含 `is_async` | `Async[T]` 子类型规则 | 中等改动 |
| IR | ✅ `CallAsync` | `Spawn`/`Await`/`Barrier` | 较大改动 |
| VM | ✅ `CallAsync` 指令 | 完整异步指令集 | 较大改动 |
| 调度器 | ✅ 基础框架 | 工作窃取实现 | 较大改动 |

### 5.2 挑战与风险

#### 5.2.1 技术挑战

| 挑战 | 描述 | 应对策略 |
|------|------|----------|
| **类型系统** | Async[T] 子类型规则复杂 | 逐步实现，先支持简单场景 |
| **调度器** | 工作窃取实现复杂 | 参考 Tokio/Actix，简化首版本 |
| **调试** | 并发调试困难 | 先提供计算图可视化 |
| **性能** | 惰性求值可能增加开销 | 提供 @eager 注解，急切执行 |

#### 5.2.2 实现风险

1. **风险：类型系统复杂度**
   - 影响：Async[T] 子类型规则可能导致类型检查复杂
   - 缓解：分阶段实现，先支持基础场景

2. **风险：调度器死锁**
   - 影响：循环依赖可能导致死锁
   - 缓解：添加超时机制和死锁检测

3. **风险：性能退化**
   - 影响：惰性求值和自动等待可能增加运行时开销
   - 缓解：提供性能分析工具和优化编译通道

4. **风险：调试困难**
   - 影响：异步代码调试比同步代码困难
   - 缓解：提供计算图可视化，逐步完善工具链

### 5.3 性能评估

#### 5.3.1 基准测试场景

| 场景 | 描述 | 预期性能 |
|------|------|----------|
| **串行请求** | 多次顺序 HTTP 请求 | 与 Go  goroutine 相近 |
| **并行请求** | 多次独立 HTTP 请求 | 与 Promise.all 相近 |
| **CPU 密集** | 计算密集型任务 | 与 Rust rayon 相近 |
| **混合负载** | I/O + CPU 混合 | 优于传统 async/await |

#### 5.3.2 优化目标

```
目标性能指标:
├── 简单 HTTP 请求延迟 < 1.5x 手动优化版
├── 并行吞吐量 > 现有实现的 2x
├── 内存占用 < 100KB/协程
└── 调度开销 < 1μs/切换
```

### 5.4 与竞品对比

| 语言 | 异步模型 | 语法 | 性能 | 复杂度 |
|------|----------|------|------|--------|
| **YaoXiang** | 象流 (惰性) | 同步语法 | 高 | 中 |
| Rust | async/await | 需标记 | 高 | 高 |
| Go | goroutine | 同步语法 | 高 | 低 |
| Kotlin | coroutine | 需标记 | 中 | 中 |
| JavaScript | Promise/async | 需标记 | 中 | 中 |

### 5.5 总结评估

#### 5.5.1 总体评价

象流模型在**设计理念上具有创新性**，通过将异步转化为数据流依赖解析，实现了"同步语法、异步本质"的目标。相比传统 async/await 模型：

| 维度 | 得分 (1-5) | 说明 |
|------|------------|------|
| **创新性** | 5 | 惰性求值 + 自动依赖解析 |
| **实用性** | 4 | 简化开发，提升可维护性 |
| **可行性** | 4 | 技术路线清晰，挑战可控 |
| **性能** | 5 | 惰性优化 + 工作窃取 |
| **生态** | 3 | 需逐步建设工具链 |

#### 5.5.2 建议

1. **优先级**：先实现基础框架，再逐步完善高级特性
2. **策略**：采用 MVP (最小可行产品) 策略，快速验证核心思想
3. **风险控制**：通过分阶段实现降低技术风险
4. **工具先行**：优先开发调试工具，降低使用门槛

---

## 六、附录

### 6.1 术语对照表

| 英文 | 中文 | 说明 |
|------|------|------|
| Lazy Evaluation | 惰性求值 | 按需计算，不提前执行 |
| Computation Graph | 计算图 | 表达数据依赖的有向无环图 |
| Async[T] | 异步代理类型 | 透明的异步值包装器 |
| Work Stealing | 工作窃取 | 空闲线程从其他线程队列偷任务 |
| Coroutine | 协程 | 轻量级用户态线程 |
| Spawn | 异步标记 | 创建异步任务的关键字 |

### 6.2 参考实现

- **Rust Tokio**: 调度器设计参考
- **Go runtime**: 工作窃取队列实现
- **Swift SwiftUI**: 响应式数据流参考
- **Kotlin Coroutines**: 协程状态管理参考

---

> 象流模型的核心价值在于：**让开发者以同步的思维编写代码，同时享受异步并行的性能优势**。
>
> —— 晨煦
