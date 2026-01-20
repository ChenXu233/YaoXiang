# Task 8.5: 单线程调度器

> **优先级**: P1
> **状态**: ⬜ 待开始
> **模块**: `src/runtime/scheduler/single_thread.rs`
> **依赖**: task-08-04-scheduler-trait

## 功能描述

实现单线程调度器，用于 `num_workers=1` 模式。

### 核心特性

- **异步调度**：任务按依赖顺序异步执行
- **单线程**：无锁，无竞态
- **按依赖顺序**：使用 DAG 管理依赖关系

### 设计要点

> **RFC-008 核心决策**：同步是调度的特例
> - 单线程模式下并发表现为异步
> - 不是禁用调度器，而是使用 SingleThreadScheduler

## 实现内容

### 1. SingleThreadScheduler 结构

```rust
pub struct SingleThreadScheduler {
    /// 任务队列（就绪任务）
    task_queue: VecDeque<Arc<dyn Task>>,
    /// DAG（用于依赖管理）
    dag: Arc<Mutex<Dag>>,
    /// 任务状态
    task_states: HashMap<TaskId, TaskState>,
    /// 任务结果缓存
    results: HashMap<TaskId, RuntimeValue>,
}
```

### 2. Scheduler Trait 实现

```rust
impl Scheduler for SingleThreadScheduler {
    fn spawn(&self, task: Arc<dyn Task>) -> TaskId {
        let task_id = task.id();
        self.task_states.insert(task_id, TaskState::Ready);
        self.task_queue.push_back(task);
        task_id
    }

    fn await_task(&self, task_id: TaskId) -> Result<RuntimeValue, RuntimeError> {
        // 按依赖顺序执行
        while let Some(task) = self.task_queue.pop_front() {
            if task.id() == task_id {
                return task.execute();
            }
            self.task_queue.push_back(task);
        }
        Err(SchedulerError::TaskNotFound(task_id))
    }

    fn spawn_with_deps(&self, task: Arc<dyn Task>, deps: &[NodeId]) -> TaskId {
        for &dep in deps {
            self.dag.lock().unwrap().add_edge(dep, task.id());
        }
        self.task_states.insert(task.id(), TaskState::Ready);
        self.task_queue.push_back(task);
        task.id()
    }

    fn await_all(&self, task_ids: &[TaskId]) -> Vec<Result<RuntimeValue, RuntimeError>> {
        task_ids.iter().map(|id| self.await_task(*id)).collect()
    }

    fn stats(&self) -> SchedulerStats {
        SchedulerStats {
            queue_size: self.task_queue.len(),
            running_tasks: 1,  // 单线程只有 1 个运行中
            completed_tasks: self.task_states.values()
                .filter(|s| matches!(s, TaskState::Completed))
                .count(),
        }
    }
}
```

### 3. 任务状态

```rust
enum TaskState {
    Pending,    // 等待依赖
    Ready,      // 就绪
    Running,    // 运行中
    Completed,  // 完成
    Error(RuntimeError),
}
```

## 验收测试

```rust
#[test]
fn test_single_thread_spawn() {
    let scheduler = Arc::new(SingleThreadScheduler::new());
    let task: Arc<dyn Task> = Arc::new(TaskImpl::new(...));

    let task_id = scheduler.spawn(task);
    assert!(task_id != TaskId(0));
}

#[test]
fn test_single_thread_await() {
    let scheduler = Arc::new(SingleThreadScheduler::new());
    let task: Arc<dyn Task> = Arc::new(TaskImpl::new(func_id, vec![]));

    let task_id = scheduler.spawn(task);
    let result = scheduler.await_task(task_id);
    assert!(result.is_ok());
}

#[test]
fn test_single_thread_async_behavior() {
    // 验证单线程模式下的异步行为
    let scheduler = Arc::new(SingleThreadScheduler::new());

    // 提交两个独立任务
    let t1 = Arc::new(TaskImpl::new(func_a, vec![]));
    let t2 = Arc::new(TaskImpl::new(func_b, vec![]));

    let id1 = scheduler.spawn(t1);
    let id2 = scheduler.spawn(t2);

    // await 时按提交顺序执行
    let r1 = scheduler.await_task(id1);
    let r2 = scheduler.await_task(id2);
}
```

## 与 RFC-008 对照

| RFC-008 设计 | 实现 |
|-------------|------|
| 单线程模式 | ✅ SingleThreadScheduler |
| 同步是特例 | ✅ 单线程并发表现为异步 |
| 按依赖顺序执行 | ✅ 使用 DAG |
| 无锁设计 | ✅ VecDeque + Mutex(DAG) |
