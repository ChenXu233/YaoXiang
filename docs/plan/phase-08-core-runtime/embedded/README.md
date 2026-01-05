# Embedded Runtime（嵌入式运行时）

> **路径**: `src/runtime/core/embedded/`
> **Phase**: P8 + P12
> **状态**: ⏳ 待实现

## 概述

Embedded Runtime 提供轻量级运行时环境，适用于资源受限环境或脚本嵌入场景。

## 特性

- **无 DAG**：立即执行器，直接顺序/并行执行
- **最小内存占用**：专为嵌入式设计
- **快速启动**：无复杂调度开销

## 文件结构

```
embedded/
├── executor.rs       # 立即执行器
├── mod.rs            # 模块入口
└── README.md         # 本文档
```

## 架构

```
┌─────────────────────────────────────┐
│         Embedded Runtime            │
├─────────────────────────────────────┤
│  Immediate Executor                 │
│  ├── spawn fn 执行                  │
│  ├── spawn {} 块执行                │
│  └── spawn for 执行                 │
├─────────────────────────────────────┤
│  无 DAG 依赖管理                    │
│  无 Work Stealing                   │
└─────────────────────────────────────┘
```

## 与 Standard/Full 对比

| 特性 | Embedded | Standard | Full |
|------|----------|----------|------|
| DAG | ❌ | ✅ | ✅ |
| Work Stealing | ❌ | ❌ | ✅ |
| @blocking | ❌ | ❌ | ✅ |
| 内存占用 | 最小 | 中等 | 最大 |
| 启动速度 | 最快 | 中等 | 最慢 |

## 相关 Task

| Task | 名称 | 状态 |
|------|------|------|
| task-08-05 | Embedded Runtime 核心 | ⏳ 待实现 |
| task-12-01 | 嵌入式 API 绑定 | ⏳ 待实现 |

## 依赖

- `runtime/core/value.rs` (P8)
- `runtime/core/allocator.rs` (P8)

## 使用场景

1. **嵌入式系统**：资源受限的运行环境
2. **脚本嵌入**：作为其他应用的脚本引擎
3. **快速原型**：开发初期快速验证
