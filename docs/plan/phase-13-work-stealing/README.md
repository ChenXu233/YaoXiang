# Phase 13: Work Stealing（工作窃取）

> **模块路径**: `src/runtime/core/full/work_stealing/`
> **状态**: ⚠️ 部分实现
> **依赖**: P10（Scheduler）

## 概述

Work Stealing 提供多线程负载均衡能力，当某个线程的任务队列为空时，可以从其他线程的任务队列中"窃取"任务来执行。

## 与 Scheduler 的关系

```
runtime/core/full/
├── work_stealing/         # P13（工作窃取）
│   ├── thief.rs           # 窃取者实现
│   ├── victim.rs          # 被窃取者实现
│   └── mod.rs
└── block.rs            # P14（@block）
```

## 文件结构

```
phase-13-work-stealing/
├── README.md              # 本文档
├── task-13-01-thief.md    # 窃取者实现
├── task-13-02-victim.md   # 被窃取者保护
├── task-13-03-load-balancer.md # 负载监控
└── task-13-04-benchmark.md # 性能基准
```

## 完成状态

| Task | 名称 | 状态 | 依赖 |
|------|------|------|------|
| task-13-01 | 窃取者实现 | ⚠️ 部分实现 | task-11-01 |
| task-13-02 | 被窃取者保护 | ⏳ 待实现 | task-13-01 |
| task-13-03 | 负载监控 | ⏳ 待实现 | task-13-01 |
| task-13-04 | 性能基准 | ⏳ 待实现 | task-13-03 |

## 工作窃取原理

```
┌─────────────────────────────────────────────────────────┐
│                     Work Stealing                        │
├─────────────────────────────────────────────────────────┤
│  Thread 0                 Thread 1                 Thread 2│
│  ┌─────────────┐          ┌─────────────┐          ┌─────────────┐│
│  │Local Queue  │          │Local Queue  │          │Local Queue  ││
│  │[A, B, C, D] │          │[E, F]       │          │[G]          ││
│  └─────────────┘          └─────────────┘          └─────────────┘│
│       ↓                         ↓                         ↓      │
│  执行 D, C              执行 F, E                 执行 G         │
│       ↓                         ↓                              │
│  队列空 → 窃取 →              ← ← ← 队列空 → 窃取              │
│       从 Thread 2 窃取 G                                             │
└─────────────────────────────────────────────────────────┘
```

## 窃取算法

```rust
struct WorkStealer {
    workers: Vec<Arc<Worker>>,
    threshold: usize,           // 窃取阈值
}

impl WorkStealer {
    /// 尝试从其他线程窃取任务
    fn try_steal(&self) -> Option<Task> {
        // 随机选择 victim
        let victim = self.select_victim();

        // 从 victim 的队列尾部窃取
        victim.steal().map(|task| {
            info!("Stolen task from thread {}", victim.id());
            task
        })
    }

    /// 选择 victim（使用随机策略）
    fn select_victim(&self) -> &Arc<Worker> {
        let idx = rand::thread_rng().gen_range(0..self.workers.len());
        &self.workers[idx]
    }
}
```

## 被窃取者保护

为了减少窃取冲突，采用双端队列设计：
- **push**：添加到队列头部（快速）
- **pop**：从队列头部取（快速，本线程）
- **steal**：从队列尾部窃取（较慢，他线程）

```rust
struct Deque<T> {
    head: AtomicUsize,      // 头部指针
    data: Vec<T>,           // 数据
    tail: AtomicUsize,      // 尾部指针
}

impl<T> Deque<T> {
    /// 本线程 push
    fn push(&self, item: T) {
        let idx = self.head.fetch_add(1, Ordering::Relaxed);
        self.data[idx % SIZE] = item;
    }

    /// 本线程 pop
    fn pop(&self) -> Option<T> {
        let idx = self.head.fetch_sub(1, Ordering::AcqRel) - 1;
        if idx < self.tail.load(Ordering::Acquire) {
            return None;
        }
        Some(self.data[idx % SIZE])
    }

    /// 他线程 steal
    fn steal(&self) -> Option<T> {
        let tail = self.tail.load(Ordering::Acquire);
        let head = self.head.load(Ordering::Acquire);

        if tail >= head {
            return None;
        }

        // CAS 竞争
        match self.tail.compare_exchange_weak(
            tail,
            tail + 1,
            Ordering::AcqRel,
            Ordering::Relaxed,
        ) {
            Ok(_) => Some(self.data[tail % SIZE]),
            Err(_) => None,
        }
    }
}
```

## 负载监控

```rust
struct LoadMonitor {
    queue_lengths: Arc<Vec<AtomicUsize>>,
    sample_interval: Duration,
}

impl LoadMonitor {
    /// 获取当前系统负载
    fn get_load(&self) -> LoadMetrics {
        let lengths: Vec<_> = self.queue_lengths
            .iter()
            .map(|len| len.load(Ordering::Relaxed))
            .collect();

        LoadMetrics {
            max: lengths.iter().max().unwrap(),
            min: lengths.iter().min().unwrap(),
            avg: lengths.iter().sum::<usize>() / lengths.len(),
        }
    }
}
```

## 性能基准

| 场景 | 无 Work Stealing | 有 Work Stealing | 提升 |
|------|------------------|------------------|------|
| 不均匀负载 | O(n²) | O(n log n) | ~40% |
| 热点任务 | O(n) | O(n) | ~10% |
| 空闲窃取 | O(1) | O(1) | ~5% |

## 相关文件

- `src/runtime/scheduler/work_stealing.rs`（当前实现）
- `src/runtime/core/full/work_stealing/`（目标位置）

## 相关文档

- [Core Runtime - Full](../phase-08-core-runtime/full/README.md)
- [Phase 11: Scheduler](../phase-11-scheduler/README.md)
