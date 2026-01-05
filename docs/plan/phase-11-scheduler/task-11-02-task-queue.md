# Task 11.2: 任务队列

> **优先级**: P0
> **状态**: ⚠️ 部分实现

## 功能描述

实现高效的任务队列，支持多生产者多消费者。

## 任务队列实现

```rust
/// 任务队列
struct WorkQueue<T> {
    /// 内部存储（使用 mpsc 或数组）
    queue: CrossbeamQueue<T>,
    /// 队列状态
    state: AtomicU8,
}

enum QueueState {
    Active,
    Idle,
    Shutdown,
}

impl<T> WorkQueue<T> {
    /// 推送任务（多生产者安全）
    pub fn push(&self, item: T) {
        self.queue.push(item);
    }

    /// 弹出任务（单消费者）
    pub fn pop(&self) -> Option<T> {
        self.queue.pop()
    }

    /// 窃取任务（多消费者）
    pub fn steal(&self) -> Option<T> {
        // 尝试从队列头部窃取
        self.try_steal()
    }
}
```

## 相关文件

- **queue.rs**: WorkQueue
