---
title: RFC-008：Runtime 并发模型与调度器脱耦设计
---

# RFC-008：Runtime 并发模型与调度器脱耦设计

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-05
> **最后更新**: 2026-03-10

> **参考**:
> - [RFC-001: 并作模型与错误处理系统](../rfc/001-concurrent-model-error-handling.md)
> - [RFC-003: 版本规划与实现建议](../rfc/003-version-planning.md)
> - [RFC-011: 泛型系统设计](../rfc/011-generic-type-system.md) **类型约束已定义**

## 摘要

本文档讨论 Runtime 架构中的关键设计问题：
1. **运行时分层设计**：Embedded Runtime（嵌入式） vs Standard Runtime（标准） vs Full Runtime（完整）
2. **编译与运行分离**：编译阶段完全相同，区别仅在运行时执行方式
3. **调度器脱耦设计**：通过 **泛型 + 注入** 实现解耦，保证工作窃取等特性不启用时解释器仍能正常运行
4. **YaoXiang 泛型 vs Rust Trait**：利用 YaoXiang 语言的泛型系统直接实现调度器脱耦，无需在语言层面引入 Trait 概念
5. **DAG 的核心地位**：作为惰性求值的依赖图，DAG 属于 Standard Runtime，嵌入式场景可选择不使用

## 动机

### 当前架构问题

在设计 YaoXiang 运行时架构时，存在以下关键问题需要解决：

```
┌─────────────────────────────────────────────────────────────────┐
│                    当前架构的困惑                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  问题 1：DAG 是核心还是可选？                                   │
│  ├── 如果是核心，它应该在 Core Runtime                         │
│  ├── 如果是可选，如何实现"无并发"的同步执行？                   │
│  └── 惰性求值依赖 DAG，无法禁用                                │
│                                                                 │
│  问题 2：调度器如何脱耦？                                       │
│  ├── WorkStealer 是否可以禁用？                                 │
│  ├── 单线程模式如何实现？                                       │
│  └── 如何保证"无调度器特性"时 VM 仍能运行？                    │
│                                                                 │
│  问题 3：异步/并发的实现层级                                   │
│  ├── @block（L1 同步）                                         │
│  ├── spawn（L2 显式并发）                                      │
│  └── 无标记（L3 透明并发）                                     │
│  └── 这些特性应该在哪一层实现？                                │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 核心矛盾

| 矛盾 | 描述 |
|------|------|
| 透明性 vs 可控性 | 并发应该是默认行为，但用户应该能控制 |
| 核心 vs 可选 | DAG 是核心，但 WorkStealing 是 num_workers>1 的高级特性 |
| 单线程 vs 并发 | 单线程模式下并发表现为异步，同步只是调度的特例 |

## 提案

### 1. Runtime 三层架构设计

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                           YaoXiang 运行时三层架构                                         │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│  ┌──────────────────────────────────────────────────────────────────────────────────┐   │
│  │                           📦 编译阶段（所有模式相同）                              │   │
│  │                                                                                  │   │
│  │   Source Code                                                                    │   │
│  │       │                                                                          │   │
│  │       ▼                                                                          │   │
│  │   ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────────┐  │   │
│  │   │  Lexer  │─▶│ Parser  │─▶│TypeCheck│─▶│ Codegen │─▶│  IR / Bytecode      │  │   │
│  │   └─────────┘  └─────────┘  └─────────┘  └─────────┘  └─────────────────────┘  │   │
│  │                                                                                  │   │
│  │   ⚠️ 重要：编译阶段完全相同！共享同一套代码                                       │   │
│  │       • 同一套语法解析（Lexer + Parser）                                         │   │
│  │       • 同一套类型检查（TypeCheck）                                              │   │
│  │       • 同一套代码生成（Codegen）                                                │   │
│  │       • 同一套 IR/Bytecode 输出                                                  │   │
│  │                                                                                  │   │
│  └──────────────────────────────────────────────────────────────────────────────────┘   │
│                                         │                                              │
│                    ┌────────────────────┼────────────────────┐                        │
│                    ▼                    ▼                    ▼                        │
│  ┌──────────────────┐   ┌───────────────┐   ┌──────────────────┐                     │
│  │ 🟢 Embedded      │   │ 🔵 Standard   │   │ 🟣 Full          │                     │
│  │                  │   │               │   │                  │                     │
│  │ 即时执行器        │   │ DAG 调度器    │   │ Full 调度器      │                     │
│  │ 同步执行         │   │ 惰性求值      │   │ 并行优化         │                     │
│  │ 无调度器         │   │ 自动并发      │   │ 工作窃取         │                     │
│  │ 无 DAG 调度      │   │               │   │                  │                     │
│  │                  │   │               │   │                  │                     │
│  └──────────────────┘   └───────────────┘   └──────────────────┘                     │
│                                                                                          │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                              各运行时详细说明                                            │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│  ┌───────────────────────────────────────────────────────────────────────────────┐      │
│  │  🟢 Embedded Runtime（嵌入式）                                                 │      │
│  │                                                                               │      │
│  │  目标场景：WASM / 游戏脚本 / 规则引擎 / 数据库存储过程                          │      │
│  │                                                                               │      │
│  │  组件：                                                                       │      │
│  │  • Value + Allocator + Ownership（内存安全）                                  │      │
│  │  • 即时执行器（Immediate Executor）                                           │      │
│  │  • ⚠️ 无 DAG 调度器                                                           │      │
│  │                                                                               │      │
│  │  执行模式：                                                                   │      │
│  │  • 编译产物：IR / Bytecode（含惰性求值标记）                                   │      │
│  │  • 即时执行器读取 IR，按顺序**同步执行**                                       │      │
│  │  • 虽然代码标记了"惰性求值"，但因为没有调度器，默认同步执行                    │      │
│  │                                                                               │      │
│  │  优势：                                                                       │      │
│  │  • 高性能：即时执行，无调度开销                                                │      │
│  │  • 低内存：无 DAG 内存占用                                                    │      │
│  │  • 快速启动：无需初始化调度器                                                  │      │
│  │                                                                               │      │
│  │  代价：                                                                       │      │
│  │  • 放弃惰性求值                                                                │      │
│  │  • 放弃自动并发/异步                                                           │      │
│  │                                                                               │      │
│  └───────────────────────────────────────────────────────────────────────────────┘      │
│                                                                                          │
│  ┌───────────────────────────────────────────────────────────────────────────────┐      │
│  │  🔵 Standard Runtime（标准运行时 - 语言核心）                                  │      │
│  │                                                                               │      │
│  │  目标场景：Web 服务 / 数据管道 / 事件驱动应用                                   │      │
│  │                                                                               │      │
│  │  组件：                                                                       │      │
│  │  • Value + Allocator + Ownership（内存安全）                                  │      │
│  │  • DAG（惰性求值核心 ★）                                                      │      │
│  │  • Scheduler（任务调度器）                                                    │      │
│  │  • 并作模型（RFC-001）原生支持异步/并发                                        │      │
│  │                                                                               │      │
│  │  执行模式：                                                                   │   ̃  │
│  │  • 编译产物：IR / Bytecode（含惰性求值标记）                                   │      │
│  │  • DAG 调度器解析依赖关系，按依赖顺序**惰性执行**                               │      │
│  │  • 并作模型自动处理并发/异步                                                   │      │
│  │  • ⚠️ 无需 async/await 语法糖！                                               │      │
│  │                                                                               │      │
│  │  配置选项：                                                                   │      │
│  │  • num_workers = 1 → 单线程异步调度                                           │      │
│  │  • num_workers > 1 → 多线程并行调度                                           │      │
│  │                                                                               │      │
│  └───────────────────────────────────────────────────────────────────────────────┘      │
│                                                                                          │
│  ┌───────────────────────────────────────────────────────────────────────────────┐      │
│  │  🟣 Full Runtime（完整运行时 - 并行优化）                                      │      │
│  │                                                                               │      │
│  │  目标场景：科学计算 / 大规模并行处理                                           │      │
│  │                                                                               │      │
│  │  组件：                                                                       │      │
│  │  • Standard Runtime 所有组件                                                  │      │
│  │  • WorkStealer（工作窃取 - 负载均衡）                                          │      │
│  │  • @block 标准库（强制同步）                                               │      │
│  │                                                                               │      │
│  │  执行模式：                                                                   │      │
│  │  • 编译产物：IR / Bytecode（含惰性求值标记）                                   │      │
│  │  • Full 调度器：DAG + WorkStealer                                             │      │
│  │  • 多线程并行执行 + 负载均衡                                                   │      │
│  │                                                                               │      │
│  │  配置选项：                                                                   │      │
│  │  • use_work_stealing = true → 启用工作窃取                                    │      │
│  │  • @block 可用于强制同步执行，确保线程安全                                     │      │
│  │                                                                               │      │
│  └───────────────────────────────────────────────────────────────────────────────┘      │
│                                                                                          │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                              关键对比表                                                  │
├─────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                          │
│  阶段              │ Embedded   │ Standard   │ Full                                    │
│  ──────────────────┼────────────┼────────────┼──────────────────                       │
│  编译              │   相同     │   相同     │   相同                                  │
│  语法解析          │   相同     │   相同     │   相同                                  │
│  类型检查          │   相同     │   相同     │   相同                                  │
│  IR 生成           │   相同     │   相同     │   相同                                  │
│  ──────────────────┼────────────┼────────────┼──────────────────                       │
│  运行时            │ 即时执行   │ DAG 调度   │ Full 调度                               │
│  执行模式          │ 同步       │ 惰性+并发   │ 并行                                    │
│  内存占用          │ 低         │ 中         │ 高                                      │
│  启动速度          │ 快         │ 中         │ 中                                      │
│  并发能力          │ 无         │ 自动       │ 自动+并行                               │
│  DAG 惰性求值      │ 无         │ ✅         │ ✅                                      │
│  WorkStealer       │ 无         │ 无         │ ✅                                      │
│  @block             │ 无         │ 无         │ ✅                                      │
│                                                                                          │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

### 2. 调度器脱耦设计

#### 核心原则

```
┌─────────────────────────────────────────────────────────────────┐
│                    调度器脱耦核心原则                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  原则 1：调度器必须存在，但可以配置                              │
│  ├── num_workers=1   → 单线程模式，并发表现为异步               │
│  ├── num_workers=N   → 多线程模式，自动启用并发                 │
│  └── WorkStealer     → num_workers>1 时的高级特性               │
│                                                                 │
│  原则 2：同步执行是调度的特例                                    │
│  ├── 不是"禁用调度器"                                           │
│  ├── 而是"使用单 worker 的调度器"                               │
│  └── DAG 仍用于惰性求值                                         │
│                                                                 │
│  原则 3：VM 与调度器通过 **泛型** 解耦                           │
│  ├── VM 不直接依赖具体调度器                                     │
│  ├── 而是通过泛型参数 `[S]` 调用（类型约束需语言扩展）           │
│  └── 可以注入不同的调度器实现（见[附录A讨论3](#附录a设计讨论记录)）│
│                                                                 │
│  原则 4：异步是 DAG 的天然特性                                   │
│  ├── num_workers=1 时：任务按依赖顺序异步执行                   │
│  ├── num_workers>1 时：任务并行执行                             │
│  └── 同一个核心上也能保持异步，与同步语义一致                   │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### 调度器接口设计：泛型 + 函数类型

> **关键洞察**：YaoXiang 语言本身支持泛型，因此调度器脱耦可以直接使用 **泛型参数** 实现，无需引入 Trait 概念。

##### 方案 A：YaoXiang 语言层面（推荐）

```yaoxiang
# 调度器接口 - 使用 YaoXiang 函数类型定义
Scheduler: Type = {
    spawn: (Task) -> TaskId,
    await: (TaskId) -> Result,
    spawn_with_deps: (Task, List[TaskId]) -> TaskId,
    await_all: (List[TaskId]) -> List[Result],
    stats: () -> SchedulerStats,
}

# 单线程调度器实现
SingleThreadScheduler: Scheduler = {
    spawn: (task) => {
        task_queue.push(task)
        generate_task_id()
    },
    await: (task_id) => {
        # 单线程：按依赖顺序异步执行
        if task = task_queue.pop() {
            execute_task(task)
        } else {
            err("No tasks")
        }
    },
    spawn_with_deps: (task, deps) => {
        for dep in deps { dag.add_edge(dep, task.id) }
        task_queue.push(task)
        task.id
    },
    await_all: (task_ids) => { ... },
    stats: () => { queue_size: task_queue.len() },
}

# 多线程调度器实现
MultiThreadScheduler: Scheduler = {
    spawn: (task) => {
        work_queue.push(task)  # 线程安全队列
        generate_task_id()
    },
    await: (task_id) => {
        # 等待任务完成
        wait_for_completion(task_id)
    },
    spawn_with_deps: (task, deps) => {
        dag.add_dependencies(task.id, deps)
        work_queue.push(task)
    },
    await_all: (task_ids) => { ... },
    stats: () => { workers: get_worker_stats() },
}

# 使用泛型创建 VM
# 注意：当前 YaoXiang 泛型语法不支持类型约束 `S: Scheduler`
# 需要扩展语言特性或使用其他方式实现调度器脱耦
# 方案1：使用类型参数，依赖类型检查保证正确性
create_vm: [S](scheduler: S) -> VM = (scheduler) => {
    VM(scheduler: scheduler, memory: create_memory(), dag: create_dag())
}

# 方案2：使用配置参数，编译期选择调度器类型
create_vm_with_config: (config: VMConfig) -> VM = (config) => {
    match config.scheduler_type of
        "single" => VM(scheduler: SingleThreadScheduler, ...)
        "multi"  => VM(scheduler: MultiThreadScheduler(num_workers: config.workers), ...)
}
```

> **⚠️ 语言特性待定**：YaoXiang 当前不支持泛型类型约束（如 `S: Scheduler`）。若需此特性，需要扩展语言规范或采用替代方案。

##### 方案 B：Rust 实现层面（编译器的 Rust 实现）

```rust
// src/middle/scheduler/mod.rs

/// 调度器接口 - 通过泛型约束实现解耦
/// 注意：这是 Rust 层面的实现细节，非 YaoXiang 语言特性
trait Scheduler: Send + Sync {
    fn spawn(&self, task: Arc<dyn Task>) -> TaskId;
    fn await_task(&self, task_id: TaskId) -> Result<Value, RuntimeError>;
    fn spawn_with_deps(&self, task: Arc<dyn Task>, deps: &[NodeId]) -> TaskId;
    fn stats(&self) -> SchedulerStats;
}

/// VM 使用泛型约束调度器
struct VM<S: Scheduler> {
    scheduler: Arc<S>,
    bytecode: Bytecode,
    registers: Vec<Value>,
    // ...
}

impl<S: Scheduler> VM<S> {
    fn exec_spawn(&mut self, func_id: FuncId, args: &[Reg]) -> TaskId {
        let task = Task::new(func_id, args);
        self.scheduler.spawn(Arc::new(task))
    }
}

/// 单线程调度器（用于单线程模式，异步执行）
struct SingleThreadScheduler {
    task_queue: VecDeque<Arc<dyn Task>>,
    dag: Arc<Mutex<ComputationDAG>>,
}

impl Scheduler for SingleThreadScheduler {
    fn spawn(&self, task: Arc<dyn Task>) -> TaskId {
        self.task_queue.push_back(task);
        task.id()
    }

    fn await_task(&self, _task_id: TaskId) -> Result<Value, RuntimeError> {
        // 单线程：按依赖顺序异步执行
        if let Some(task) = self.task_queue.pop_front() {
            task.execute()
        } else {
            Err(RuntimeError::NoTasks)
        }
    }

    fn spawn_with_deps(&self, task: Arc<dyn Task>, deps: &[NodeId]) -> TaskId {
        for &dep in deps {
            self.dag.lock().unwrap().add_edge(dep, task.id());
        }
        self.task_queue.push_back(task);
        task.id()
    }

    fn stats(&self) -> SchedulerStats {
        SchedulerStats { queue_size: self.task_queue.len() }
    }
}

/// 多线程调度器（用于并发模式）
struct MultiThreadScheduler {
    workers: Vec<Worker>,
    dag: Arc<Mutex<ComputationDAG>>,
    #[cfg(feature = "work-stealing")]
    work_stealer: Arc<WorkStealer>,
}

impl Scheduler for MultiThreadScheduler {
    fn spawn(&self, task: Arc<dyn Task>) -> TaskId {
        #[cfg(feature = "work-stealing")]
        {
            self.work_stealer.push_local(task);
        }
        #[cfg(not(feature = "work-stealing"))]
        {
            self.global_queue.push(task);
        }
        task.id()
    }

    // ... 其他方法实现
}
```

> **核心要点**：
> - **YaoXiang 层面**：使用泛型参数 `[S]` 实现调度器解耦（类型约束待语言扩展）
> - **Rust 层面**：使用 trait `Scheduler` 作为泛型约束
> - 两者语义一致：编译期多态，无运行时开销
>
> **✅ 已解决**：YaoXiang 泛型类型约束已在RFC-011中定义，调度器脱耦方案：
> - **方案 A**：使用RFC-011的类型约束语法 `[S: Scheduler]`
> - 方案 B：使用配置参数 + match 分支选择调度器（备用方案）
> - 方案 C：使用 Rust trait 在编译器实现层处理（备用方案）

#### VM 使用泛型调度器

```yaoxiang
# 方案 A：RFC-011设计（已定义类型约束）
# VM: [S: Scheduler](scheduler: S) -> VM = (scheduler) => { ... }
# single_vm = VM[SingleThreadScheduler](SingleThreadScheduler)
# multi_vm = VM[MultiThreadScheduler](MultiThreadScheduler(num_workers: 4))

# 方案 B：当前可行设计（使用配置 + match）
VM: (scheduler: Scheduler) -> VM = (scheduler) => {
    {
        scheduler: scheduler,
        memory: Memory(),
        registers: [],
        execute: ( bytecode ) => { ... },
    }
}

# 使用配置创建不同类型的 VM
create_vm: (config: VMConfig) -> VM = (config) => {
    scheduler = match config.mode of
        "single" => SingleThreadScheduler()
        "multi"  => MultiThreadScheduler(num_workers: config.workers)
    VM(scheduler: scheduler)
}
```

```rust
// Rust: VM 通过泛型约束使用调度器
struct VM<S: Scheduler> {
    scheduler: Arc<S>,
    bytecode: Bytecode,
    registers: Vec<Value>,
}

impl<S: Scheduler> VM<S> {
    fn exec_spawn(&mut self, func_id: FuncId, args: &[Reg]) -> TaskId {
        let task = Task::new(func_id, args);
        // 通过泛型接口提交任务
        self.scheduler.spawn(Arc::new(task))
    }

    fn exec_await(&mut self, task_id: Reg, result_reg: Reg) {
        let task_id = self.registers[task_id].as_task_id();
        let result = self.scheduler.await_task(task_id)
            .expect("await failed");
        self.registers[result_reg] = result;
    }
}
```

### 3. 异步与无感并发的实现位置

#### 并发三层模型（RFC-001 扩展）

> **详见**：[附录A讨论1 - @block 与异步实现位置](#讨论1-block-与异步实现位置)

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                           并发三层模型实现层级                                   │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│  层级  │   模式            │  语法标记      │  实现位置      │  说明            │
│  ──────┼──────────────────┼────────────────┼───────────────┼───────────────── │
│  L1    │ @block 同步       │ `@block`       │ Full Runtime  │ 标准库提供       │
│  L2    │ @block 内并发    │ `spawn`        │ Full Runtime  │ @block内可控并发 │
│  L3    │ 透明并发（默认）  │ 无/@auto       │ Core Runtime  │ 自动最优并行     │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  核心要点：                                                              │   │
│  │  ├── L2 spawn 只能在 @block 函数内部使用                                  │   │
│  │  ├── 用于在同步代码中插入部分并发                                        │   │
│  │  └── L3 默认自动并发，编译器自动分析 DAG                                 │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
├─────────────────────────────────────────────────────────────────────────────────┤
│                           Full Runtime（可选）                                  │
│                                                                                 │
│  层级  │   模式        │  实现位置      │  说明                               │
│  ──────┼──────────────┼────────────────┼─────────────────────────────────── │
│  L1    │ @block        │ Full Runtime   │ 强制急切求值，由标准库提供         │
│  L2    │ spawn         │ Full Runtime   │ @block 内可控并发                 │
│  L4    │ WorkStealer  │ Full Runtime   │ num_workers>1 时的负载均衡         │
│                                                                                 │
│  ┌─────────────────────────────────────────────────────────────────────────┐   │
│  │  核心要点：                                                              │   │
│  │  ├── @block 是标准库功能，不是语言内置                               │   │
│  │  ├── spawn 是 @block 作用域内的可控并发                                │   │
│  │  └── WorkStealer 是 num_workers>1 的高级特性                           │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### 无感并发实现

```yaoxiang
# L3: 透明并发（默认行为）
# 代码看起来是同步的，但运行时自动并行/异步执行

# num_workers=1 时：异步执行
# num_workers>1 时：并行执行

data1 = fetch_data("api1")  # 创建 DAG 节点，不阻塞
data2 = fetch_data("api2")  # 创建 DAG 节点，不阻塞

# 当使用 data1 或 data2 时，触发求值
# 运行时自动选择异步或并行策略

result = process(data1, data2)  # 完成后处理
```

#### 自底向上执行模型

> **设计更新（2026-03-10）**：基于 RFC-001 的新设计，调度器采用自底向上执行模型。

```
用户代码（同步语法）：
    a = fetch(url0)
    b = fetch(url1)
    print(a)  ← 只需要 a

编译时分析（自底向上）：
    print(a) 需要 a → 依赖 fetch(url0)
    fetch(url1) 没有人需要 → 孤岛 DAG

运行时调度（从叶子开始）：
    fetch(url0) ──→ print(a)    ← 依赖链，按序
    fetch(url1)                  ← 孤岛，独立并行
```

**核心要点**：
- 不是"遇到函数调用就生成 Future"，而是**从"需要结果的地方"反向分析依赖**
- **叶子节点优先并行执行**，依赖链按序向上遍历
- **孤岛 DAG 独立并行**：没有消费者的节点不阻塞主流程
- **无限循环作为后台 DAG**：调度器切片执行，不会卡死

```
执行流程：
    1. 从"最终结果"反向分析依赖
    2. 构建全局 DAG：叶子 → 内部节点 → 根节点
    3. 叶子节点并行执行（控制并发数）
    4. 向上遍历，需要值时等待依赖完成
    5. 孤岛 DAG 独立并行，不影响主流程
```

#### 无限循环处理

```
场景 1：单个 while/for（无调度开销）
──────────────────────────────────────────────
main: () -> () = {
    while True {
        update_ui()
        fetch_data()
    }
}
→ 只有一个无限循环
→ 直接同步执行，和普通代码没区别

场景 2：多个 while（自动切片）
──────────────────────────────────────────────
main: () -> () = {
    while True { update_ui() }      # 后台任务1
    while True { network_poll() }  # 后台任务2
    server_loop()                   # 主任务
}
→ 3 个独立任务
→ 调度器切片切换
→ 真正的并发

调度器自适应：
──────────────────────────────────────────────
if 任务数 == 1:
    直接执行（同步，无开销）
else:
    切片调度（并发）
```

#### @block 实现（标准库）

```yaoxiang
# @block 由标准库提供
use std::runtime::block

# 使用 @block 强制急切求值
@block
fetch_sync: (String) -> JSON = (url) => {
    HTTP.get(url).json()  # 强制急切求值，不进入 DAG 惰性队列
    HTTP.get(url).json()  # 强制急切求值
    HTTP.get(url).json()
    HTTP.get(url).json()
}
```

### 4. 核心设计决策

#### 决策 1：调度器脱耦方案选择

> **决议**：采用方案 C'（**泛型 + 注入**）
> **详见**：[附录A讨论3 - 调度器脱耦方案选择](#讨论3调度器脱耦方案选择)

| 方案 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| ~~A: 条件编译~~ | ~~#[cfg] 控制代码路径~~ | ~~零运行时开销~~ | ~~维护两份代码~~ |
| ~~B: 运行时分支~~ | ~~if enabled { ... } else { ... }~~ | ~~单二进制~~ | ~~一点开销~~ |
| **C': 泛型 + 注入** | **使用 YaoXiang 泛型参数** | **清晰解耦、单二进制、零开销、无需 Trait** | **编译期确定调度器类型** |

> **关键洞察**（2025-01-05 @晨煦）：
> - YaoXiang 语言本身支持泛型，无需引入 Trait 概念
> - 使用泛型参数 `[S]` 实现调度器脱耦（类型约束需语言扩展）
> - 编译期多态，无运行时开销

#### 决策 2：同步执行的实现

> **决议**：同步只是调度的特例
> **详见**：[附录A讨论2 - 单线程模式实现](#讨论2单线程模式实现)

```
┌─────────────────────────────────────────────────────────────────┐
│                    同步 vs 调度的关系                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  误解：禁用调度器                                                │
│  ❌ scheduler.disabled = true                                   │
│  ❌ if concurrency_disabled { 顺序执行 }                        │
│                                                                 │
│  正确：同步是调度的特例                                         │
│  ✅ scheduler = SingleThreadScheduler::new()                    │
│  ✅ VM 通过 Trait 接口使用调度器                                │
│  ✅ 调度器始终存在，只是 num_workers=1                          │
│  ✅ 单线程模式下并发表现为异步                                  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 5. 核心问题讨论

#### 问题 1：DAG 应该在哪一层？

> **已决议**：DAG 属于 Core Runtime

| 层级 | 包含 | 说明 |
|------|------|------|
| Core Runtime | ✅ DAG | 惰性求值核心，必须存在 |
| Core Runtime | ✅ Scheduler | 任务调度核心，必须存在 |
| Full Runtime | ❌ WorkStealer | num_workers>1 时的负载均衡，可选 |

---

#### 问题 2：异步支持如何实现？

> **已决议**：DAG 天然支持异步

| 配置 | 行为 |
|------|------|
| num_workers=1 | 任务按 DAG 依赖顺序**异步**执行 |
| num_workers>1 | 任务**并行**执行，可选 WorkStealer |

---

#### 问题 3：@block 的实现位置？

> **已决议**：@block 由标准库提供

```
┌─────────────────────────────────────────────────────────────────┐
│                    @block 实现位置                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ❌ 不是语言内置语法                                            │
│  ✅ 是标准库提供的功能                                          │
│  ✅ 使用方式：blocking::block(|| { ... })                       │
│                                                                 │
│  效果：                                                          │
│  ├── 强制函数内所有语句急切求值                                 │
│  ├── 不会进入 DAG 惰性队列                                      │
│  └── 表现为同步执行，不会进行调度                               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

---

#### 问题 4：WorkStealer 是否在 Core Runtime？

> **已决议**：WorkStealer 是 Full Runtime 高级特性

| 组件 | 层级 | 原因 |
|------|------|------|
| DAG | Core Runtime | 惰性求值必需 |
| Scheduler | Core Runtime | 任务调度必需 |
| WorkStealer | Full Runtime | num_workers>1 时的负载均衡，可选 |

---

## 详细设计

### 文件结构

```
src/
├── core/                          # Core 组件（所有运行时共享）
│   ├── mod.rs
│   ├── value.rs                   # 值类型
│   ├── allocator.rs               # 内存分配
│   └── ownership.rs               # 所有权模型
│
├── frontend/                      # 前端组件（所有运行时共享）
│   ├── lexer/                     # 词法分析
│   ├── parser/                    # 语法解析
│   └── typecheck/                 # 类型检查
│
├── codegen/                       # 代码生成（所有运行时共享）
│   ├── mod.rs
│   ├── ir.rs                      # IR 定义
│   └── bytecode.rs                # 字节码生成
│
├── embedded/                      # 🟢 Embedded Runtime（可选）
│   ├── mod.rs
│   └── executor.rs                # 即时执行器（无 DAG 调度）
│
├── runtime/                       # 🔵 Standard Runtime（标准）
│   ├── mod.rs
│   ├── dag/                       # DAG 核心（惰性求值）
│   │   ├── mod.rs
│   │   ├── trait.rs               # DAG Trait
│   │   └── computation.rs
│   └── scheduler/                 # 调度器
│       ├── mod.rs
│       ├── trait.rs               # Scheduler Trait
│       ├── single_thread.rs       # 单线程实现（异步调度）
│       └── multi_thread.rs        # 多线程实现（并行调度）
│
├── full/                          # 🟣 Full Runtime（可选）
│   ├── mod.rs
│   ├── work_stealer.rs            # 工作窃取（高级特性）
│   └── block.rs                # @block 标准库实现
│
└── vm/                            # 字节码解释器
    ├── mod.rs
    └── executor.rs                # 通过 Trait 调用运行时
```

### 特性配置

```rust
// Cargo.toml
[features]
default = ["runtime"]              # 默认启用标准运行时
embedded = []                      # 嵌入式模式（无 DAG 调度）
full-runtime = ["runtime"]         # 完整运行时（需要 runtime）
work-stealing = ["full-runtime"]   # 工作窃取（需要 full-runtime）

// 使用示例
// 嵌入式：yaoxiang --embedded script.yx
// 标准：yaoxiang script.yx
// 完整：yaoxiang --full-runtime script.yx
```

### 配置方式

```rust
// Cargo.toml
[features]
default = ["core-runtime", "vm", "stdlib"]
core-runtime = []      # Core Runtime 始终启用
full-runtime = []      # Full Runtime 可选
work-stealing = []     # 工作窃取（需要 full-runtime）

// 调度器配置
pub enum SchedulerConfig {
    /// 单线程模式：num_workers=1，并发表现为异步
    SingleThread {
        /// 是否启用 DAG（始终启用）
        enable_dag: bool = true,
    },
    /// 多线程模式：num_workers>1，可选启用工作窃取
    MultiThread {
        num_workers: usize,
        #[cfg(feature = "work-stealing")]
        use_work_stealing: bool,
    },
}
```

## 权衡

### 优点

- **清晰的三层分层**：Embedded / Standard / Full
- **编译与运行分离**：前端代码完全复用，仅运行时不同
- **良好的解耦**：通过 **泛型** 接口（方案 C'）
- **灵活性**：单二进制支持多种模式
- **零运行时开销**：编译期多态，无需 trait 对象
- **一致性**：同步只是调度的特例
- **高性能**：嵌入式即时执行，无调度开销
- **可扩展**：WorkStealer 作为 Full Runtime 可选特性
- **嵌入式友好**：高性能 + 低内存占用 + 快速启动

### 缺点

- **初始复杂度**：需要定义调度器接口和多种运行时变体
- **运行时配置**：需要处理不同的运行时模式
- **嵌入式代价**：放弃惰性求值和自动并发
- **编译期绑定**：调度器类型在编译期确定，运行时无法切换

## 替代方案

### 方案 1：纯条件编译

```rust
#[cfg(not(feature = "concurrency"))]
fn spawn_task(task: Arc<Task>) {
    // 单线程：直接执行
    task.execute();
}

#[cfg(feature = "concurrency")]
fn spawn_task(task: Arc<Task>) {
    // 多线程：提交到调度器
    SCHEDULER.spawn(task);
}
```

**缺点**：需要维护两份代码，配置更改需要重新编译

### 方案 2：禁用调度器

```rust
struct VM {
    #[cfg(not(feature = "concurrency"))]
    scheduler: (),

    #[cfg(feature = "concurrency")]
    scheduler: Arc<Scheduler>,
}
```

**缺点**：引入条件编译的复杂度，破坏架构一致性

## 实现策略

### 阶段 1：基础架构（Standard Runtime）

1. 实现前端组件共享（Lexer + Parser + TypeCheck）
2. 定义 `Scheduler` 接口（**泛型约束**，方案 C'）
3. 实现 `SingleThreadScheduler`（异步调度）
4. 迁移 DAG 到 Runtime 层
5. **决策**：同步是调度的特例（方案 B）
6. 实现 Standard Runtime（Web 服务/数据管道场景）

### 阶段 2：Embedded Runtime（嵌入式）

1. 实现即时执行器（Immediate Executor）
2. 实现 Embedded Runtime 组件
3. 配置系统支持 `--embedded` 模式
4. **决策**：嵌入式使用同一套前端代码，即时执行
5. **决策**：嵌入式放弃 DAG 调度，获得高性能 + 低占用
6. 目标场景：WASM / 游戏脚本 / 规则引擎

### 阶段 3：Full Runtime（并行优化）

1. 实现 `MultiThreadScheduler`（并行调度）
2. **决策**：WorkStealer 是高级特性
3. 实现 WorkStealer（Full Runtime）
4. 实现 @block 标准库
5. 配置系统支持 `--full-runtime` 模式
6. 目标场景：科学计算 / 大规模并行处理

## 开放问题

```markdown
# 已决议问题
- [x] DAG 是否应该属于 Standard Runtime？（决议：是，嵌入式除外）
- [x] WorkStealer 如何保证不启用时解释器仍能运行？（决议：**泛型 + 注入**，方案 C'）
- [x] 单线程模式的正确实现方式？（决议：同步是调度的特例，方案 B）
- [x] 异步/并发的实现位置？（决议：DAG 天然支持异步）
- [x] @block 的实现位置？（决议：标准库提供）
- [x] WorkStealer 是否在 Core Runtime？（决议：Full Runtime）
- [x] 嵌入式运行时如何设计？（决议：即时执行，无 DAG 调度）
- [x] 编译阶段是否相同？（决议：是，所有运行时共享同一套前端）
- [x] 调度器脱耦是否需要 Trait？（决议：**不需要**，使用 YaoXiang 泛型直接实现）
- [x] 运行时切换机制设计（决议：泛型 + 条件编译 + 向下切换）
- [x] 泛型类型约束 `[S: Scheduler]`（RFC-011已定义）
- [x] **运行时切换 API**：运行时切换在标准库内实现
- [x] **单线程模式优化**：DAG 本身就是额外开销，num_workers=1 与 >=1 时开销一致
- [x] **调度模式切换**：num_workers 切换时自动增加调度器核心，同步/异步/并行语法无区别，由调度器自动处理
- [x] **惰性求值标记**：编译器自动处理，确定无高级运行时参与时简化编译产物（删除无用标记）


# 待讨论问题

> **说明**：以下问题应在其他 RFC 讨论，不在本 RFC 范围内：
> - 泛型类型约束语法 → RFC-011已定义（已决议）
> - @block API 设计 → RFC-001 或标准库 RFC
> - 异常/错误传播 → RFC-001 错误处理
> - 调试器支持 → 调试器 RFC
> - 性能基准 → 实现文档

- 暂无

---

## 附录A：设计讨论记录

### 讨论 1：@block 与异步实现位置

> **讨论状态**: 已解决
> **发起者**: 晨煦
> **日期**: 2025-01-05

#### 问题描述

异步支持是否需要在 Core Runtime 实现？@block 注解应该放在哪一层？

#### 备选方案

| 方案 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| A | @block 是语言内置 | 用户友好 | 增加语言复杂度 |
| B | @block 在 Core Runtime | 性能好 | Core Runtime 膨胀 |
| C | @block 由标准库提供 | 解耦、灵活 | 需要导入 |

#### 讨论记录

- 2025-01-05 @晨煦：异步支持本身核心的 DAG 依赖图就可以支持了
- 2025-01-05 @晨煦：num_workers=1 时自动进行异步调度，多线程情况下自动并发调度
- 2025-01-05 @晨煦：同一个核心上也能异步，保持与同步语义一致
- 2025-01-05 @晨煦：@block 由标准库提供，强制函数内的所有语句急切求值，表现为同步

#### 决议

采用方案 C：
- ✅ 异步支持由 DAG 天然支持，在 Core Runtime
- ✅ @block 由标准库提供，不在语言内置
- ✅ 保持 Core Runtime 简洁

---

### 讨论 2：WorkStealer 位置

> **讨论状态**: 已解决
> **发起者**: 晨煦
> **日期**: 2025-01-05

#### 问题描述

WorkStealer 应该放在 Core Runtime 还是 Full Runtime？

#### 备选方案

| 方案 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| A | WorkStealer 在 Core Runtime | 负载均衡自动启用 | Core Runtime 膨胀 |
| B | WorkStealer 在 Full Runtime | 核心简洁 | 需要启用特性 |

#### 讨论记录

- 2025-01-05 @晨煦：工作窃取可以作为 num_workers>1 之后的高级特性

#### 决议

采用方案 B：
- ✅ WorkStealer 是 num_workers>1 时的负载均衡策略
- ✅ 不在 Core Runtime，作为 Full Runtime 可选特性
- ✅ Core Runtime 在 num_workers=1 时也能正常工作

---

### 讨论 3：调度器脱耦方案选择

> **讨论状态**: 已解决
> **发起者**: 晨煦
> **日期**: 2025-01-05
> **更新**: 2025-01-05 - 修正为「泛型 + 注入」方案 C'

#### 问题描述

如何设计调度器，使其在不启用工作窃取时，解释器仍能正常运行？同时，YaoXiang 语言本身支持泛型，是否可以利用这一点避免引入 Trait 概念？

#### 备选方案

| 方案 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| A: 条件编译 | #[cfg] 控制代码路径 | 零运行时开销 | 维护两份代码 |
| B: 运行时分支 | if enabled { ... } else { ... } | 单二进制 | 一点开销 |
| C: Trait + 注入 | 注入不同实现 | 清晰解耦 | 需要 Trait 定义 |
| **C': 泛型 + 注入** | **使用 YaoXiang 泛型参数** | **清晰解耦、单二进制、零开销、无需 Trait** | **编译期确定类型** |

#### 讨论记录

- 2025-01-05 @沫郁酱：建议使用方案 C（Trait + 注入），因为最清晰且单二进制
- 2025-01-05 @晨煦：同意方案 C
- 2025-01-05 @晨煦：**关键洞察**：我的语言支持泛型，我直接用泛型实现不就行了！
- 2025-01-05 @沫郁酱：对呀！这样就不需要 Trait 了喵～使用泛型参数 `[S]` 实现调度器脱耦

#### 决议

采用方案 C'（**泛型 + 注入**）：
- ✅ 清晰解耦
- ✅ 单二进制支持多种模式
- ✅ 无条件编译复杂度
- ✅ **零运行时开销**（编译期多态）
- ✅ **无需 Trait 概念**（利用 YaoXiang 泛型）
- ✅ 调度器类型在编译期确定，运行时无开销
- ✅ **已支持**：YaoXiang 泛型类型约束已在 RFC-011 中定义（如 `S: Scheduler`）

---

### 讨论 4：嵌入式运行时设计

> **讨论状态**: 已解决
> **发起者**: 晨煦
> **日期**: 2025-01-05

#### 问题描述

嵌入式场景（如 WASM/游戏脚本）需要什么运行时支持？是否需要 DAG 调度？

#### 备选方案

| 方案 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| A | 嵌入式使用 Standard Runtime | 代码复用、特性完整 | 内存占用高、启动慢 |
| B | 嵌入式使用即时执行器，无 DAG | 高性能、低占用、快速启动 | 放弃惰性求值、放弃并发 |
| C | 嵌入式可选启用 DAG | 灵活 | 增加复杂度 |

#### 讨论记录

- 2025-01-05 @晨煦：嵌入式需要高性能和低内存占用
- 2025-01-05 @晨煦：YaoXiang 是编译型语言，编译阶段完全相同
- 2025-01-05 @晨煦：嵌入式场景使用即时执行器，按顺序同步执行
- 2025-01-05 @晨煦：虽然代码标记了惰性求值，但嵌入式没有调度器，默认同步执行
- 2025-01-05 @沫郁酱：编译阶段完全相同，区别仅在运行时执行方式

#### 决议

采用方案 B：
- ✅ 编译阶段完全相同（共享同一套前端）
- ✅ 嵌入式使用即时执行器，按顺序同步执行
- ✅ 嵌入式放弃 DAG 调度，获得高性能 + 低占用 + 快速启动
- ✅ 嵌入式放弃惰性求值和自动并发
- ✅ 目标场景：WASM / 游戏脚本 / 规则引擎 / 数据库存储过程

---

### 讨论 5：单线程模式实现

> **讨论状态**: 已解决
> **发起者**: 沫郁酱
> **日期**: 2025-01-05

#### 问题描述

如何正确实现"单线程模式"？是禁用调度器还是使用单 Worker？

#### 备选方案

| 方案 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| A: 禁用调度器 | scheduler.disabled = true | 简单 | 破坏架构一致性 |
| B: 单 Worker | scheduler = SingleThreadScheduler | 一致性好 | 需要单独实现 |

#### 讨论记录

- 2025-01-05 @沫郁酱：建议方案 B，同步只是调度的特例
- 2025-01-05 @晨煦：同意！同步只是调度的特例
- 2025-01-05 @晨煦：单线程模式下的并发应该表现为异步

#### 决议

采用方案 B：
- ✅ 同步只是调度的特例
- ✅ 单线程模式下并发表现为异步
- ✅ 调度器始终存在，只是配置不同
- ✅ 保持架构一致性

---

### 讨论 6：运行时切换机制设计

> **讨论状态**: 已解决
> **发起者**: 晨煦
> **日期**: 2025-01-05

#### 问题描述

运行时切换是否支持泛型类型的条件编译？如何设计三层运行时的切换机制？

#### 备选方案

| 方案 | 描述 | 优点 | 缺点 |
|------|------|------|------|
| A: 条件编译 | 通过 `#[cfg]` 控制代码路径 | 零运行时开销 | 需要为每种模式编译不同版本 |
| B: 运行时分支 | if runtime_mode == Full { ... } | 单二进制 | 一点运行时开销 |
| C: 泛型 + 条件编译 + 向下切换 | 编译时选择层级，运行时选择性禁用 | 灵活、可扩展 | 实现复杂度较高 |

#### 讨论记录

- 2025-01-05 @晨煦：运行时切换是不是不支持泛型类型的条件编译，需要导致三层全部编译？
- 2025-01-05 @晨煦：设计为通过泛型判断需要编译几层，提供运行时向下切换的选型
- 2025-01-05 @晨煦：比如编译产生标准运行时可以切换为嵌入式，但是由于没有接入完整运行时，无法切换为完整运行时
- 2025-01-05 @晨煦：编译时标记保留，使之支持运行时向下切换还能切换回去

#### 运行时切换规则

```
┌─────────────────────────────────────────────────────────────────┐
│                    运行时向下切换规则                            │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐        │
│  │  Full       │───▶│  Standard   │───▶│  Embedded   │        │
│  │  完整运行时  │    │  标准运行时  │    │  嵌入式运行时│        │
│  └─────────────┘    └─────────────┘    └─────────────┘        │
│       │                   │                   │                 │
│       ▼                   ▼                   ▼                 │
│    ✅ Full           ✅ Standard           ✅ Embedded         │
│    ✅ Standard       ✅ Standard           ✅ Embedded         │
│    ❌ Embedded       ❌ Embedded           ✅ Embedded         │
│                                                                 │
│  切换规则：                                                          │
│  ├── Full → Standard：启用标准调度，禁用 WorkStealer              │
│  ├── Full → Embedded：启用即时执行，禁用 DAG 和调度器              │
│  ├── Standard → Embedded：启用即时执行，禁用 DAG                   │
│  └── **向上切换不可能**：Embedded 无法切换到 Standard 或 Full     │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### 决议

采用方案 C（**泛型 + 条件编译 + 向下切换**）：
- ✅ 通过泛型判断需要编译几层
- ✅ 提供运行时向下切换的选型
- ✅ 编译产生 Standard 运行时可切换为 Embedded，但无法切换为 Full
- ✅ 编译时标记保留（DAG 标记），支持运行时切换
- ✅ 向下切换后可随时再启用（如 Standard → Embedded 后可切回 Standard）

---

### 讨论 7：泛型类型约束与调度器接口标准化

> **讨论状态**: 已解决
> **发起者**: 晨煦
> **日期**: 2025-01-05

#### 问题描述

1. `[S: Scheduler]` 类型约束语法是否需要加入语言规范？
2. 调度器接口的标准化应该在哪个 RFC 讨论？

#### 讨论记录

- 2025-01-05 @晨煦：不支持类型约束 `S: Scheduler` 是否要开一个新的 RFC 来讨论？
- 2025-01-05 @晨煦：我倒是很想进行支持，毕竟一切皆是类型
- 2025-01-05 @晨煦：调度器接口标准化在 RFC-003 讨论

#### 决议

- ✅ **泛型类型约束**：已在 RFC-011 中详细定义 `[S: Scheduler]` 语法
- ✅ **调度器接口标准化**：移到 RFC-003 版本规划中讨论
- ✅ 遵循"一切皆是类型"的设计哲学

---

### 讨论 8：运行时切换与嵌入式编译产物处理

> **讨论状态**: 已解决
> **发起者**: 晨煦
> **日期**: 2025-01-05

#### 问题描述

1. 运行时切换应该在语言层面还是标准库实现？
2. DAG 在单线程模式下的开销如何？
3. 同步/异步/并行如何平滑切换？
4. 嵌入式模式下编译产物如何处理？

#### 讨论记录

- 2025-01-05 @晨煦：运行时切换可以在标准库内实现
- 2025-01-05 @晨煦：DAG 在 num_workers=1 时是否有额外开销？实际上 DAG 本身就是额外开销，开销情况和 num_workers>=1 时一致
- 2025-01-05 @晨煦：单线程异步与多线程并行如何平滑切换？在 num_workers 切换时自动将调度器的核心增加，开始并行处理任务
- 2025-01-05 @晨煦：在我们语言的语境下面，同步，异步，并行语法没有区别，由调度器解决
- 2025-01-05 @晨煦：嵌入式模式下编译产物可以简化，删除无用标记，但是语法解析和代码层面不用做任何改变，这一切都是由编译器自动处理

#### 决议

- ✅ **运行时切换**：在标准库内实现
- ✅ **DAG 开销**：DAG 本身就是额外开销，与 num_workers 无关
- ✅ **调度模式切换**：num_workers 切换时自动调整，同步/异步/并行语法无区别，由调度器自动处理
- ✅ **编译产物简化**：编译器自动处理，确定无高级运行时参与时删除无用标记
- ✅ **依赖图构建**：静态依赖图，编译期确定所有依赖，运行时无额外负担（内存换性能）
- ✅ **Task 生命周期**：移到 RFC-001 并作模型中讨论

---

## 附录B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 调度器脱耦方案 | **泛型 + 注入**（方案 C'） | 2025-01-05 | 晨煦、沫郁酱 |
| 单线程模式 | 同步是调度的特例（方案 B） | 2025-01-05 | 沫郁酱、晨煦 |
| 异步实现 | DAG 天然支持 | 2025-01-05 | 晨煦 |
| @block | 标准库提供 | 2025-01-05 | 晨煦 |
| WorkStealer | Full Runtime 高级特性 | 2025-01-05 | 晨煦 |
| 并发层级 | L1: Full Runtime, L2-L3: Standard Runtime | 2025-01-05 | 沫郁酱 |
| 嵌入式设计 | 即时执行，无 DAG 调度 | 2025-01-05 | 晨煦 |
| 编译阶段 | 所有运行时共享同一套前端 | 2025-01-05 | 晨煦 |
| 运行时分层 | Embedded / Standard / Full 三层 | 2025-01-05 | 沫郁酱 |
| **类型约束 RFC** | RFC-011 已定义 `[S: Scheduler]` 语法 | 2025-01-25 | 晨煦 |
| **调度器接口** | 移到 RFC-003 讨论 | 2025-01-05 | 晨煦 |
| **编译标记保留** | 保留 DAG 标记支持运行时切换 | 2025-01-05 | 晨煦 |
| **运行时切换** | 在标准库内实现（泛型 + 条件编译 + 向下切换） | 2025-01-05 | 晨煦 |
| **DAG 开销** | 本身就是额外开销，与 num_workers 无关 | 2025-01-05 | 晨煦 |
| **调度模式切换** | num_workers 切换时自动调整，由调度器处理 | 2025-01-05 | 晨煦 |
| **编译产物简化** | 编译器自动处理，确定无高级运行时时删除标记 | 2025-01-05 | 晨煦 |
| **依赖图构建** | 静态依赖图，编译期确定，运行时无负担（内存换性能） | 2025-01-05 | 晨煦 |

---

## 参考文献

### YaoXiang 官方文档
- [RFC-001: 并作模型与错误处理系统](../rfc/001-concurrent-model-error-handling.md)
- [RFC-003: 版本规划与实现建议](../rfc/003-version-planning.md)
- [语言规范](../language-spec.md)
- [YaoXiang 指南](../guides/YaoXiang-book.md)

### 外部参考
- [Rust async 运行时设计](https://tokio.rs/)
- [Go 调度器设计](https://golang.org/src/middle/proc.go)
- [Swift Actor 模型](https://docs.swift.org/swift-book/LanguageGuide/Concurrency.html)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 作者草稿，等待提交审核 |
| **审核中** | `docs/design/rfc/` | 开放社区讨论和反馈 |
| **已接受** | `docs/design/accepted/` | 成为正式设计文档 |
| **已拒绝** | `docs/design/rfc/` | 保留在RFC目录 |
