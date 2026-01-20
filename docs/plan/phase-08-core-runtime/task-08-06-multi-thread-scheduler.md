# Task 8.6: 多线程调度器

> **优先级**: P1
> **状态**: ⬜ 待开始
> **模块**: `src/runtime/scheduler/multi_thread.rs`
> **依赖**: task-08-05-single-thread-scheduler

## 功能描述

实现多线程调度器，用于 `num_workers>1` 模式。

### 核心特性

- **并行执行**：多个 worker 线程并发执行
- **全局队列**：工作分发
- **DAG 依赖**：管理任务依赖关系
- **线程安全**：所有操作原子化

### 设计要点

> **RFC-008**：num_workers>1 时自动启用并行
> - 工作窃取是 Full Runtime 高级特性
> - 多线程调度器是 Standard Runtime 的一部分

## 实现内容

### 1. MultiThreadScheduler 结构

```rust
pub struct MultiThreadScheduler {
    /// 工作线程数
    num_workers: usize,
    /// 全局任务队列
    global_queue: Arc<Mutex<VecDeque<Arc<dyn Task>>>>,
    /// 每个 worker 的本地队列
    local_queues: Vec<ThreadLocal<VecDeque<Arc<dyn Task>>>>,
    /// DAG（用于依赖管理）
    dag: Arc<Mutex<Dag>>,
    /// 任务状态
    task_states: Arc<Mutex<HashMap<TaskId, TaskState>>>,
    /// 任务结果
    results: Arc<Mutex<HashMap<TaskId, RuntimeValue>>>,
    /// 线程池
    thread_pool: ThreadPool,
    /// 同步屏障
    barrier: Arc<Barrier>,
}
```

### 2. Scheduler Trait 实现

```rust
impl Scheduler for MultiThreadScheduler {
    fn spawn(&self, task: Arc<dyn Task>) -> TaskId {
        let task_id = task.id();
        self.task_states.lock().unwrap()
            .insert(task_id, TaskState::Ready);

        // 加入全局队列
        self.global_queue.lock().unwrap()
            .push_back(task);

        task_id
    }

    fn await_task(&self, task_id: TaskId) -> Result<RuntimeValue, RuntimeError> {
        loop {
            {
                let states = self.task_states.lock().unwrap();
                match states.get(&task_id) {
                    Some(TaskState::Completed) => {
                        let results = self.results.lock().unwrap();
                        return Ok(results[&task_id].clone());
                    }
                    Some(TaskState::Error(e)) => return Err(e.clone()),
                    Some(_) => {
                        // 仍在运行，等待
                    }
                    None => return Err(SchedulerError::TaskNotFound(task_id)),
                }
            }
            // 短暂休眠，避免忙等待
            std::thread::sleep(std::time::Duration::from_micros(100));
        }
    }

    fn spawn_with_deps(&self, task: Arc<dyn Task>, deps: &[NodeId]) -> TaskId {
        let task_id = task.id();
        let mut states = self.task_states.lock().unwrap();

        for &dep in deps {
            self.dag.lock().unwrap().add_edge(dep, task_id);
        }

        states.insert(task_id, TaskState::Pending);

        // 检查依赖是否都已完成
        let all_done = deps.iter().all(|&d| {
            matches!(states.get(&d), Some(TaskState::Completed))
        });

        if all_done {
            states.insert(task_id, TaskState::Ready);
            drop(states);
            self.global_queue.lock().unwrap()
                .push_back(task);
        }

        task_id
    }

    fn await_all(&self, task_ids: &[TaskId]) -> Vec<Result<RuntimeValue, RuntimeError>> {
        task_ids.iter().map(|id| self.await_task(*id)).collect()
    }

    fn stats(&self) -> SchedulerStats {
        let states = self.task_states.lock().unwrap();
        SchedulerStats {
            queue_size: self.global_queue.lock().unwrap().len(),
            running_tasks: states.values()
                .filter(|s| matches!(s, TaskState::Running))
                .count(),
            completed_tasks: states.values()
                .filter(|s| matches!(s, TaskState::Completed))
                .count(),
        }
    }
}
```

### 3. Worker 线程

```rust
struct Worker {
    id: usize,
    scheduler: Arc<MultiThreadScheduler>,
}

impl Worker {
    fn run(&self) {
        loop {
            // 同步开始
            self.scheduler.barrier.wait();

            // 从队列获取任务
            while let Some(task) = self.try_get_task() {
                self.execute_task(task);
            }

            // 检查是否应该退出
            if self.scheduler.is_shutdown() {
                break;
            }
        }
    }

    fn try_get_task(&self) -> Option<Arc<dyn Task>> {
        // 1. 先尝试本地队列
        if let Some(task) = self.scheduler.local_queues[self.id].pop_back() {
            return Some(task);
        }

        // 2. 尝试从全局队列获取
        {
            let mut global = self.scheduler.global_queue.lock().unwrap();
            if let Some(task) = global.pop_front() {
                return Some(task);
            }
        }

        // 3. 从其他本地队列窃取（Full Runtime 特性）
        #[cfg(feature = "work-stealing")]
        if let Some(task) = self.steal_task() {
            return Some(task);
        }

        None
    }

    fn execute_task(&self, task: Arc<dyn Task>) {
        let task_id = task.id();

        // 更新状态
        self.scheduler.task_states.lock().unwrap()
            .insert(task_id, TaskState::Running);

        // 执行
        let result = task.execute();

        // 更新状态和结果
        let mut states = self.scheduler.task_states.lock().unwrap();
        let mut results = self.scheduler.results.lock().unwrap();

        match result {
            Ok(value) => {
                states.insert(task_id, TaskState::Completed);
                results.insert(task_id, value);
            }
            Err(e) => {
                states.insert(task_id, TaskState::Error(e));
            }
        }
    }
}
```

## 验收测试

```rust
#[test]
fn test_multi_thread_spawn() {
    let scheduler = Arc::new(MultiThreadScheduler::new(4));
    let task: Arc<dyn Task> = Arc::new(TaskImpl::new(...));

    let task_id = scheduler.spawn(task);
    assert!(task_id != TaskId(0));
}

#[test]
fn test_multi_thread_parallel() {
    // 验证并行执行
    let scheduler = Arc::new(MultiThreadScheduler::new(4));

    // 提交多个独立任务
    let tasks: Vec<Arc<dyn Task>> = (0..10).map(|i| {
        Arc::new(TaskImpl::new(slow_func_id, vec![RuntimeValue::Int(i)]))
    }).collect();

    let start = std::time::Instant::now();
    for task in &tasks {
        scheduler.spawn(task.clone());
    }

    // 并行执行应该比串行快
    let results: Vec<_> = tasks.iter()
        .map(|t| scheduler.await_task(t.id()))
        .collect();

    let elapsed = start.elapsed();
    // 并行执行时间应该明显小于 10 * 串行时间
    assert!(elapsed < std::time::Duration::from_secs(5));
}

#[test]
fn test_multi_thread_dependencies() {
    let scheduler = Arc::new(MultiThreadScheduler::new(4));

    // 提交有依赖的任务链
    let chain = create_dependency_chain(&scheduler, 5);
    let result = scheduler.await_all(&chain);
    assert_eq!(result.len(), 5);
    assert!(result.iter().all(|r| r.is_ok()));
}
```

## 与 RFC-008 对照

| RFC-008 设计 | 实现 |
|-------------|------|
| 多线程模式 | ✅ MultiThreadScheduler |
| num_workers>1 并行 | ✅ 线程池 |
| 任务调度 | ✅ 全局队列 + 本地队列 |
| 工作窃取 | ⚠️ Full Runtime 特性 |
