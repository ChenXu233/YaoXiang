# Phase 8: Core Runtime（运行时核心）

> **模块路径**: `src/runtime/core/`
> **状态**: ⚠️ 需重构
> **RFC对齐**: RFC-008 三层运行时架构

## 概述

Core Runtime 是运行时核心，提供值类型、内存分配器和 GC 接口。根据 RFC-008 三层架构，内部划分为：

```
runtime/core/
├── embedded/     # 🟢 Embedded Runtime（立即执行器，无 DAG）
├── standard/     # 🔵 Standard Runtime（DAG + Scheduler）
└── full/         # 🟣 Full Runtime（Standard + WorkStealing + @block）
                  #     ↓ 链接到 P13/P14
```

## 三层运行时架构（RFC-008）

| 层级 | Phase | 特性 | 适用场景 |
|------|-------|------|----------|
| **Embedded** | P8+P12 | 立即执行器，无 DAG | 资源受限环境、脚本嵌入 |
| **Standard** | P8-P11 | DAG + Scheduler + VM | 标准应用（v0.3+） |
| **Full** | P8-P14 | Standard + WorkStealing + @block | 高性能计算（v0.5+） |

### Embedded Runtime（立即执行器）

```
embedded/
├── executor.rs       # 立即执行器
├── mod.rs            # 模块入口
└── README.md         # 嵌入式运行时说明
```

**特性**：
- 无任务图（DAG），直接顺序/并行执行
- 最小内存占用
- 适合嵌入式场景

**相关 Task**：task-08-05-embedded（新建）

### Standard Runtime（标准运行时）

```
standard/
├── dag.rs            # 任务依赖图
├── scheduler.rs      # 调度器（P10-P11）
├── vm.rs             # 虚拟机（P12）
└── README.md         # 标准运行时说明
```

**特性**：
- DAG 任务依赖管理
- 并发调度优化
- 完整 VM 支持

**相关 Task**：task-09-01 至 task-12-02

### Full Runtime（完整运行时）

```
full/
├── work_stealing.rs  # 工作窃取（P13）
├── block.rs       # @block 注解（P14）
└── README.md         # 完整运行时说明
```

**特性**：
- Work-stealing 负载均衡
- @block 同步执行保证
- 高性能并发

**相关 Task**：task-13-01 至 task-14-02

## 文件结构

```
phase-08-core-runtime/
├── README.md                       # 本文档
├── embedded/
│   └── README.md                   # Embedded Runtime 说明
├── standard/
│   └── README.md                   # Standard Runtime 说明
├── full/
│   └── README.md                   # Full Runtime 说明（P13/P14 入口）
├── task-08-01-value-type.md        # Value 类型定义
├── task-08-02-allocator.md         # 内存分配器
├── task-08-03-gc-interface.md      # GC 接口
├── task-08-04-object-model.md      # 对象模型
└── task-08-05-embedded.md          # Embedded Runtime（即时执行）
```

## 完成状态

### Core 组件（P8）

| Task | 名称 | 状态 | 依赖 |
|------|------|------|------|
| task-08-01 | Value 类型定义 | ⚠️ 需重构 | - |
| task-08-02 | 内存分配器 | ⚠️ 部分实现 | task-08-01 |
| task-08-03 | GC 接口 | ⚠️ 部分实现 | task-08-02 |
| task-08-04 | 对象模型 | ⚠️ 需重构 | task-08-01 |
| task-08-05 | Embedded Runtime | ⏳ 待实现 | task-08-01 |

### Full 扩展（P13-P14）

| Task | 名称 | 状态 | 位置 |
|------|------|------|------|
| task-13-01 | Work Stealing | ⚠️ 部分实现 | full/work_stealing.md |
| task-14-01 | @block 注解 | ⏳ 待实现 | full/block.md |

## 架构问题

**当前问题**：VM 模块中包含了 Runtime 应该负责的组件。

**期望架构**：
- `runtime/core/value.rs`: Value 类型定义
- `runtime/core/allocator.rs`: 内存分配器
- `runtime/core/gc.rs`: GC 接口
- `runtime/core/embedded/executor.rs`: Embedded 执行器
- `runtime/core/standard/dag.rs`: DAG 管理
- `runtime/core/full/work_stealing.rs`: Work stealing
- `vm/executor.rs`: VM 执行器（使用 Runtime 提供的组件）

## 依赖链

```
P1-P4 (编译前端) → P5-P7 (优化) → P8 (Core Runtime)
                                        ↓
                    ┌───────────────────┼───────────────────┐
                    ↓                   ↓                   ↓
              P12 (Embedded)    P9-P11 (Standard)    P13-P14 (Full)
                    ↓                   ↓                   ↓
              脚本嵌入          标准应用            高性能计算
                                        ↓
                                  P15-P17 (JIT/Debugger/Stdlib)
```

## 相关文件

- `src/vm/mod.rs` (当前 Value 定义位置)
- `src/runtime/memory/mod.rs` (当前内存管理位置)
- `src/runtime/scheduler/` (调度器 - P10-P11)
- `src/runtime/dag/` (DAG - P9)
