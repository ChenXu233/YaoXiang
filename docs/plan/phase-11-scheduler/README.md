# Phase 11: 调度器

> **模块路径**: `src/runtime/scheduler/`
> **状态**: ⚠️ 部分实现

## 概述

调度器负责任务的调度和执行，支持 work-stealing 负载均衡。

## 文件结构

```
phase-11-scheduler/
├── README.md                     # 本文档
├── task-11-01-work-stealing.md   # Work Stealing
├── task-11-02-task-queue.md      # 任务队列
└── task-11-03-fiber.md           # Fiber 调度
```

## 完成状态

| Task | 名称 | 状态 |
|------|------|------|
| task-11-01 | Work Stealing | ⚠️ 部分实现 |
| task-11-02 | 任务队列 | ⚠️ 部分实现 |
| task-11-03 | Fiber 调度 | ⏳ 待实现 |

## 相关文件

- **mod.rs**: 调度器主模块
- **work_stealing.rs**: Work stealing 实现
- **fiber.rs**: Fiber 实现
