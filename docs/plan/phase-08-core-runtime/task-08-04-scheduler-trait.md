# Task 8.4: 调度器接口（Trait）

> **优先级**: P1
> **状态**: ⬜ 待开始
> **模块**: `src/runtime/scheduler/trait.rs`
> **依赖**: task-08-01-value-type, task-08-02-allocator

## 功能描述

定义调度器接口（Trait），用于实现调度器脱耦（RFC-008 方案 C'）。

### 核心职责

1. **任务管理**：spawn, await, await_all
2. **依赖管理**：spawn_with_deps
3. **统计信息**：获取调度器状态

### 文件结构

```
src/runtime/scheduler/
├── mod.rs              # 导出入口
├── trait.rs            # Scheduler Trait（本文档）
├── single_thread.rs    # 单线程实现
└── multi_thread.rs     # 多线程实现
```

## 实现内容

### 1. Scheduler Trait

```rust
/// 调度器 Trait（零运行时开销，编译期多态）
pub trait Scheduler: Send + Sync {
    /// 提交任务
    fn spawn(&self, task: Arc<dyn Task>) -> TaskId;

    /// 等待任务完成
    fn await_task(&self, task_id: TaskId) -> Result<RuntimeValue, RuntimeError>;

    /// 提交带依赖的任务
    fn spawn_with_deps(&self, task: Arc<dyn Task>, deps: &[NodeId]) -> TaskId;

    /// 等待所有任务完成
    fn await_all(&self, task_ids: &[TaskId]) -> Vec<Result<RuntimeValue, RuntimeError>>;

    /// 获取调度器统计
    fn stats(&self) -> SchedulerStats;
}
```

### 2. Task Trait

```rust
/// 任务 Trait
pub trait Task: Send + Sync {
    fn id(&self) -> TaskId;
    fn func_id(&self) -> FunctionId;
    fn args(&self) -> &[RuntimeValue];
    fn execute(&self) -> Result<RuntimeValue, RuntimeError>;
}
```

### 3. TaskImpl 实现

```rust
struct TaskImpl {
    id: TaskId,
    func_id: FunctionId,
    args: Vec<RuntimeValue>,
}

impl Task for TaskImpl { ... }
```

### 4. 调度器统计

```rust
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    pub queue_size: usize,
    pub running_tasks: usize,
    pub completed_tasks: usize,
}
```

### 5. 错误类型

```rust
#[derive(Debug, Clone)]
pub enum SchedulerError {
    TaskNotFound(TaskId),
    TaskExecutionFailed(String),
    DependencyNotMet(NodeId),
    // ...
}
```

## 验收测试

```rust
#[test]
fn test_scheduler_trait_object() {
    // 可以将任意调度器作为 trait 对象使用
    let scheduler: Arc<dyn Scheduler> = Arc::new(SingleThreadScheduler::new());
    let task: Arc<dyn Task> = Arc::new(TaskImpl::new(...));
    let id = scheduler.spawn(task);
    assert!(id != TaskId(0));
}
```

## 与 RFC-008 对照

| RFC-008 设计 | 实现 |
|-------------|------|
| 调度器脱耦 | ✅ Scheduler Trait |
| 泛型 + 注入 | ✅ Trait + Arc<dyn Scheduler> |
| 零运行时开销 | ✅ 编译期多态 |
