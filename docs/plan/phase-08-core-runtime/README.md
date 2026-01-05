# Phase 8: Core Runtime

> **模块路径**: `src/runtime/core/`
> **状态**: ⚠️ 需重构

## 概述

Core Runtime 提供运行时基础组件，包括值类型、内存分配器和 GC 接口。

## 文件结构

```
phase-08-core-runtime/
├── README.md                   # 本文档
├── task-08-01-value-type.md    # Value 类型定义
├── task-08-02-allocator.md     # 内存分配器
├── task-08-03-gc-interface.md  # GC 接口
└── task-08-04-object-model.md  # 对象模型
```

## 完成状态

| Task | 名称 | 状态 |
|------|------|------|
| task-08-01 | Value 类型定义 | ⚠️ 需重构（当前在 vm/mod.rs） |
| task-08-02 | 内存分配器 | ⚠️ 部分实现 |
| task-08-03 | GC 接口 | ⚠️ 部分实现 |
| task-08-04 | 对象模型 | ⚠️ 需重构 |

## 架构问题

**当前问题**：VM 模块中包含了 Runtime 应该负责的组件。

**期望架构**：
- `runtime/core/value.rs`: Value 类型定义
- `runtime/core/allocator.rs`: 内存分配器
- `runtime/core/gc.rs`: GC 接口
- `vm/executor.rs`: VM 执行器（使用 Runtime 提供的组件）

## 相关文件

- `src/vm/mod.rs` (当前 Value 定义位置)
- `src/runtime/memory/mod.rs` (当前内存管理位置)
