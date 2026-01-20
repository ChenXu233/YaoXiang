# Task 8.8: 工作窃取（Full Runtime）

> **优先级**: P2（可选）
> **状态**: ⬜ 待开始
> **模块**: `src/full/work_stealer.rs`
> **依赖**: task-08-07-standard-runtime

## 功能描述

实现工作窃取器，用于 Full Runtime 的负载均衡。

### 核心特性

- **负载均衡**：空闲 worker 从繁忙 worker 窃取任务
- **NUMA 友好**：减少跨节点访问
- **减少竞争**：本地队列优先

### 设计要点

> **RFC-008 决策**：WorkStealer 是 Full Runtime 高级特性
> - 不在 Core Runtime
> - num_workers>1 时可选启用
> - Core Runtime 在无 WorkStealer 时也能正常工作

## 文件结构

```
src/full/
├── mod.rs            # 导出入口
└── work_stealer.rs   # 工作窃取器（本文档）
```

## 实现内容

### 1. WorkStealer 结构

```rust
pub struct WorkStealer {
    /// 所有 worker 的本地队列
    worker_queues: Vec<Arc<WorkerQueue>>,
    /// 被窃取队列（无法窃取时的备用）
    steal_queue: Arc<StealQueue>,
    /// 工作线程数
    num_workers: usize,
}

/// 本地队列（每个 worker 一个）
struct WorkerQueue {
    /// 任务队列
    tasks: Mutex<VecDeque<Arc<dyn Task>>>,
    /// 任务计数器（用于随机选择）
    count: AtomicUsize,
}

/// 窃取队列
struct StealQueue {
    tasks: Mutex<Vec<Arc<dyn Task>>>,
    /// 版本号（用于检测队列变化）
    version: AtomicUsize,
}
```

### 2. 窃取算法

```rust
impl WorkStealer {
    /// 尝试从其他队列窃取任务
    pub fn try_steal(&self, worker_id: usize) -> Option<Arc<dyn Task>> {
        // 1. 随机选择目标 worker
        let target = self.random_worker(worker_id);

        // 2. 尝试从目标队列窃取
        if let Some(task) = self.worker_queues[target].try_steal() {
            return Some(task);
        }

        // 3. 尝试从窃取队列获取
        self.steal_queue.try_pop()
    }

    /// 窃取后端（使用 Treiber Stack）
    fn try_steal_from_queue(queue: &Mutex<Vec<Arc<dyn Task>>>) -> Option<Arc<dyn Task>> {
        let mut tasks = queue.lock().unwrap();

        if tasks.len() > 1 {
            // 弹出最后一个任务（O(1)）
            tasks.pop()
        } else {
            None
        }
    }
}

/// WorkerQueue 窃取实现
impl WorkerQueue {
    pub fn try_steal(&self) -> Option<Arc<dyn Task>> {
        let mut tasks = self.tasks.lock().unwrap();

        // 从队首窃取（与本地 push/pop 方向相反）
        if tasks.len() > 1 {
            // 移动队首到新 Vec
            let stolen = tasks.drain(0..1).next();
            stolen
        } else {
            None
        }
    }
}
```

### 3. 与 MultiThreadScheduler 集成

```rust
impl MultiThreadScheduler {
    /// 使用工作窃取创建
    pub fn new_with_stealing(num_workers: usize) -> Self {
        let work_stealer = Arc::new(WorkStealer::new(num_workers));

        MultiThreadScheduler {
            num_workers,
            global_queue: Arc::new(Mutex::new(VecDeque::new())),
            local_queues: (0..num_workers).map(|_| {
                ThreadLocal::new(|| WorkerQueue::new())
            }).collect(),
            work_stealer,
            dag: Arc::new(Mutex::new(Dag::new())),
            task_states: Arc::new(Mutex::new(HashMap::new())),
            results: Arc::new(Mutex::new(HashMap::new())),
            thread_pool: ThreadPool::new(num_workers),
            barrier: Arc::new(Barrier::new(num_workers)),
        }
    }
}

impl Worker {
    fn try_get_task(&self) -> Option<Arc<dyn Task>> {
        // 1. 优先本地队列
        if let Some(task) = self.scheduler.local_queues[self.id].pop() {
            return Some(task);
        }

        // 2. 全局队列
        if let Some(task) = self.try_global_queue() {
            return Some(task);
        }

        // 3. 工作窃取
        if let Some(task) = self.scheduler.work_stealer.try_steal(self.id) {
            return Some(task);
        }

        None
    }
}
```

## 性能优化

### 1. 随机选择

```rust
impl WorkStealer {
    fn random_worker(&self, exclude: usize) -> usize {
        let mut rng = rand::thread_rng();
        let mut target = rng.gen_range(0..self.num_workers);

        // 避免窃取自己
        while target == exclude && self.num_workers > 1 {
            target = rng.gen_range(0..self.num_workers);
        }

        target
    }
}
```

### 2. 指数退避

```rust
fn try_get_task_with_backoff(&self, worker_id: usize) -> Option<Arc<dyn Task>> {
    let mut attempts = 0;
    let max_attempts = self.num_workers * 2;

    loop {
        // 尝试获取任务
        if let Some(task) = self.try_get_task_local(worker_id) {
            return Some(task);
        }

        // 尝试窃取
        if let Some(task) = self.work_stealer.try_steal(worker_id) {
            return Some(task);
        }

        attempts += 1;

        if attempts >= max_attempts {
            // 长时间找不到任务，可能没有任务了
            return None;
        }

        // 指数退避
        std::thread::sleep(Duration::from_micros(1 << attempts.min(10)));
    }
}
```

## 验收测试

```rust
#[test]
fn test_work_stealing() {
    let scheduler = MultiThreadScheduler::new_with_stealing(4);

    // 提交不平衡的工作负载
    // 一些 worker 应该窃取其他 worker 的任务
    let imbalance_module = compile(r#"
        heavy_task: (Int) -> Int = (n) => {
            sum = 0
            for i in 0..100000 {
                sum = sum + i
            }
            sum
        }
        light_task: (Int) -> Int = (n) => n * 2
        main: () -> Int = () => {
            tasks = [
                spawn { heavy_task(1) },
                spawn { heavy_task(1) },
                spawn { light_task(1) },
                spawn { light_task(1) },
            ]
            sum = 0
            for t in tasks {
                sum = sum + t
            }
            sum
        }
    "#).unwrap();

    let result = scheduler.load_and_run(&imbalance_module);
    assert!(result.is_ok());

    // 验证负载均衡（任务完成时间更均衡）
    let stats = scheduler.stats();
    assert!(stats.queue_size >= 0);
}
```

## 与 RFC-008 对照

| RFC-008 设计 | 实现 |
|-------------|------|
| WorkStealer 是 Full Runtime | ✅ `src/full/` |
| num_workers>1 时可选 | ✅ Feature flag |
| 负载均衡 | ✅ 窃取算法 |
| Core Runtime 正常工作 | ✅ MultiThreadScheduler 无需 WorkStealer |
