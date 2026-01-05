# Phase 11: 调度器

> **模块路径**: `src/runtime/core/standard/scheduler/`
> **状态**: ⚠️ 部分实现
> **依赖**: task-09-01（DAG）

## 概述

调度器负责任务的调度和执行，管理任务队列和 Fiber 生命周期。

## 文件结构

```
phase-11-scheduler/
├── README.md                     # 本文档
├── task-11-01-task-queue.md      # 任务队列
├── task-11-02-fiber.md           # Fiber 调度
└── task-11-03-scheduler.md       # 调度器核心
```

## 完成状态

| Task | 名称 | 状态 | 依赖 |
|------|------|------|------|
| task-11-01 | 任务队列 | ⚠️ 部分实现 | task-09-01 |
| task-11-02 | Fiber 调度 | ⏳ 待实现 | task-11-01 |
| task-11-03 | 调度器核心 | ⏳ 待实现 | task-11-02 |

## 架构

```
┌─────────────────────────────────────────┐
│              Scheduler                   │
├─────────────────────────────────────────┤
│  Global Queue                           │
│  ├── 新任务入队                          │
│  └── 工作窃取源（Full Runtime）          │
├─────────────────────────────────────────┤
│  Per-Thread Queue（每个线程本地）         │
│  ├── FIFO 任务队列                       │
│  └── 快速出队                            │
├─────────────────────────────────────────┤
│  Fiber Manager                           │
│  ├── Fiber 创建/销毁                     │
│  ├── Fiber 状态管理                      │
│  └── 上下文切换                          │
└─────────────────────────────────────────┘
```

## 任务队列

```rust
struct TaskQueue {
    local_queue: VecDeque<Task>,    // 线程本地队列
    global_queue: Arc<TaskQueue>,   // 全局共享队列
    stealing: bool,                 // 是否允许被窃取
}
```

## Fiber 调度

Fiber 是轻量级执行单元，类似于 Go 的 goroutine：

```rust
struct Fiber {
    stack: Vec<u8>,         // 栈空间
    context: Context,       // 寄存器上下文
    state: FiberState,      // 状态：Running/Ready/Suspended
    parent: Option<Arc<Fiber>>,
}
```

## 相关文件

- **mod.rs**: 调度器主模块
- **queue.rs**: 任务队列实现
- **fiber.rs**: Fiber 实现

## 与 Work Stealing 的关系

> **注意**：Work Stealing 已移至 Phase 13（Full Runtime）
>
> 基础调度器提供任务队列和 Fiber 管理，Work Stealing 在此基础上增加窃取能力。

## 相关文档

- [Full Runtime - Work Stealing](../phase-08-core-runtime/full/work_stealing.md)
- [Phase 13: Work Stealing](../phase-13-work-stealing/README.md)
