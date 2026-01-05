# Task 11.1: Work Stealing

> **优先级**: P0
> **状态**: ⚠️ 部分实现

## 功能描述

实现 work-stealing 调度算法，实现负载均衡。

## Work Stealing 算法

```rust
/// 工作窃取调度器
struct WorkStealingScheduler {
    /// 工作者线程
    workers: Vec<Worker>,
    /// 全局任务队列
    global_queue: WorkQueue<Task>,
    /// 窃取阈值
    steal_threshold: usize,
}

struct Worker {
    /// 工作者 ID
    id: WorkerId,
    /// 本地任务队列
    local_queue: WorkQueue<Task>,
    ///线程句柄
    thread : Option<JoinHandle<()>>,
}

impl WorkStealingScheduler {
    /// 调度任务
    pub fn schedule(&self, task: Task) {
        if let Some(worker) = self.current_worker() {
            worker.local_queue.push(task);
        } else {
            self.global_queue.push(task);
        }
    }

    /// 窃取任务
    pub fn steal(&self, thief: &Worker) -> Option<Task> {
        // 从全局队列窃取
        if let Some(task) = self.global_queue.pop() {
            return Some(task);
        }

        // 从其他工作者窃取
        for victim in &self.workers {
            if victim.id != thief.id {
                if let Some(task) = victim.local_queue.steal() {
                    return Some(task);
                }
            }
        }

        None
    }
}
```

## 相关文件

- **work_stealing.rs**: WorkStealingScheduler
