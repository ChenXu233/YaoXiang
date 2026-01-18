# Phase 7: 单态化

> **模块路径**: `src/middle/monomorphize/`
> **状态**: 🔄 开发中

## 概述

单态化将泛型代码转换为具体类型的非泛型代码。

## 文件结构

```
phase-07-monomorphize/
├── README.md                      # 本文档
├── task-07-01-datatype.md         # 单态化数据结构 ✅
├── task-07-02-type-monomorphize.md # 类型单态化 ✅
├── task-07-03-01-fn-monomorphize.md # 函数单态化 ✅
├── task-07-03-02-closure-monomorphize-plan.md # 闭包单态化 ✅
├── task-07-04-constraint.md       # Send/Sync 特化 ⏳
├── task-07-05-cross-module.md     # 跨模块实例化 ✅
├── task-07-06-strategy.md         # 实例化策略 ⏳
├── task-07-07-cache.md            # 实例缓存 ⏳
└── task-07-08-error.md            # 错误处理 ⏳
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
| task-07-03-01 | 函数单态化 | P0 | 07-01, 07-02 | ✅ 已完成 |
| task-07-03-02 | 闭包单态化 | P0 | 07-03-01 | ✅ 已完成 |
| task-07-04 | Send/Sync 特化 | P0 | 07-02, 07-03 | ⏳ 待实现 |
| task-07-05 | 跨模块实例化 | P1 | 07-03 | ✅ 已完成 |
| task-07-06 | 实例化策略 | P1 | 07-03 | ⏳ 待实现 |
| task-07-07 | 实例缓存 | P1 | 07-03, 07-04, 07-05 | ⏳ 待实现 |
| task-07-08 | 错误处理 | P2 | 07-01 ~ 07-07 | ⏳ 待实现 |

## 相关文件

| 文件 | 描述 |
|------|------|
| [mod.rs](../../../../src/middle/monomorphize/mod.rs) | 单态化主模块 |
| [cross_module.rs](../../../../src/middle/monomorphize/cross_module.rs) | 跨模块单态化器 |
| [global.rs](../../../../src/middle/monomorphize/global.rs) | 类型替换工具函数 |
| [module_state.rs](../../../../src/middle/monomorphize/module_state.rs) | 模块单态化状态 |
| [instance.rs](../../../../src/middle/monomorphize/instance.rs) | 实例数据结构 |
| [function.rs](../../../../src/middle/monomorphize/function.rs) | 函数单态化 |
| [closure.rs](../../../../src/middle/monomorphize/closure.rs) | 闭包单态化 |
| [type_mono.rs](../../../../src/middle/monomorphize/type_mono.rs) | 类型单态化 |

## 测试文件

| 文件 | 描述 |
|------|------|
| [fn_monomorphize.rs](../../../../src/middle/monomorphize/tests/fn_monomorphize.rs) | 函数单态化测试 |
| [closure_monomorphize.rs](../../../../src/middle/monomorphize/tests/closure_monomorphize.rs) | 闭包单态化测试 |
| [cross_module.rs](../../../../src/middle/monomorphize/tests/cross_module.rs) | 跨模块测试 |
| [type_monomorphize.rs](../../../../src/middle/monomorphize/tests/type_monomorphize.rs) | 类型单态化测试 |
