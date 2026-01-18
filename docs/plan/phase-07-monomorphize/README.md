# Phase 7: 单态化

> **模块路径**: `src/middle/monomorphize/`
> **状态**: ⏳ 待实现

## 概述

单态化将泛型代码转换为具体类型的非泛型代码。

## 文件结构

```
phase-07-monomorphize/
├── README.md                      # 本文档
├── task-07-01-datatype.md         # 单态化数据结构 ⬅️ P0
├── task-07-02-type-monomorphize.md # 类型单态化 ⬅️ P0
├── task-07-03-fn-monomorphize.md  # 函数单态化 ⬅️ P0
├── task-07-04-constraint.md       # Send/Sync 特化 ⬅️ P0
├── task-07-05-cross-module.md     # 跨模块实例化 ⬅️ P1
├── task-07-06-strategy.md         # 实例化策略 ⬅️ P1
├── task-07-07-cache.md            # 实例缓存 ⬅️ P1
└── task-07-08-error.md            # 错误处理 ⬅️ P2
```

## 任务依赖关系

```
task-07-01 (数据结构)
     │
     ▼
task-07-02 (类型单态化) ──────┐
     │                       │
     ▼                       ▼
task-07-03 (函数单态化) ──→ task-07-04 (Send/Sync 特化)
     │                       │
     ▼                       │
task-07-05 (跨模块) ←────────┘
     │
     ▼
task-07-06 (策略) ───────┐
     │                   │
     ▼                   ▼
task-07-07 (缓存) ──→ task-07-08 (错误处理)
```

## 完成状态

| Task | 名称 | 优先级 | 依赖 | 状态 |
|------|------|--------|------|------|
| task-07-01 | 单态化数据结构 | P0 | - | ✅ 已完成 |
| task-07-02 | 类型单态化 | P0 | 07-01 | ✅ 已完成 |
| task-07-03 | 函数单态化 | P0 | 07-01, 07-02 | ⏳ 待实现 |
| task-07-04 | Send/Sync 特化 | P0 | 07-02, 07-03 | ⏳ 待实现 |
| task-07-05 | 跨模块实例化 | P1 | 07-03 | ⏳ 待实现 |
| task-07-06 | 实例化策略 | P1 | 07-03 | ⏳ 待实现 |
| task-07-07 | 实例缓存 | P1 | 07-03, 07-04, 07-05 | ⏳ 待实现 |
| task-07-08 | 错误处理 | P2 | 07-01 ~ 07-07 | ⏳ 待实现 |

## 相关文件

- **mod.rs**: 单态化主模块
- **types.rs**: MonoType, InstanceKey, MonoState
- **instantiate.rs**: 实例化逻辑
- **functions.rs**: 函数单态化
- **constraints.rs**: Send/Sync 推导
- **cross_module.rs**: 跨模块处理
- **strategy.rs**: 策略选择
- **cache.rs**: 缓存管理
- **error.rs**: 错误处理
