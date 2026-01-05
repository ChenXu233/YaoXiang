# Phase 12: 虚拟机

> **模块路径**: `src/vm/`
> **状态**: ⚠️ 需重构

## 概述

虚拟机执行字节码指令，实现 YaoXiang 程序的运行时解释。

## 文件结构

```
phase-12-vm/
├── README.md                    # 本文档
├── task-12-01-executor.md       # 执行器
├── task-12-02-frames.md         # 栈帧管理
├── task-12-03-opcodes.md        # 指令集
└── task-12-04-interrupt.md      # 中断处理
```

## 完成状态

| Task | 名称 | 状态 |
|------|------|------|
| task-12-01 | 执行器 | ⚠️ 需重构 |
| task-12-02 | 栈帧管理 | ⚠️ 需重构 |
| task-12-03 | 指令集 | ⚠️ 需重构 |
| task-12-04 | 中断处理 | ⏳ 待实现 |

## 架构问题

**当前问题**：VM 模块中包含了 Runtime 组件（Value、Heap 等）。

**期望架构**：
- `vm/executor.rs`: VM 执行器（使用 Runtime 提供的 Value、Allocator）
- `runtime/core/value.rs`: Value 类型（迁移自 vm/mod.rs）
- `runtime/core/allocator.rs`: 内存分配器

## 相关文件

- `src/vm/mod.rs` (当前混乱状态)
- `src/vm/executor.rs` (当前实现)
