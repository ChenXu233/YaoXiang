# Full Runtime（完整运行时）

> **路径**: `src/runtime/core/full/`
> **Phase**: P8 - P14
> **状态**: ⚠️ 部分实现
> **依赖**: 需要先实现 Standard Runtime（P8-P11）

## 概述

Full Runtime 是完整运行时环境，在 Standard 基础上增加 Work Stealing 负载均衡和 @blocking 同步执行保证。

## 特性

- **Work Stealing**：多线程负载均衡
- **@blocking 注解**：同步执行保证
- **高性能并发**：最优资源利用率

## 文件结构

```
full/
├── work_stealing.rs  # 工作窃取（P13）
├── blocking.rs       # @blocking 注解（P14）
└── README.md         # 本文档
```

## 架构

```
┌─────────────────────────────────────┐
│          Full Runtime               │
├─────────────────────────────────────┤
│  Standard Runtime（P8-P11）         │
│  ├── DAG 任务管理                   │
│  ├── Scheduler                      │
│  └── VM                             │
├─────────────────────────────────────┤
│  Full 扩展                          │
│  ├── Work Stealing                  │
│  │   ├── 线程本地队列               │
│  │   ├── 窃取算法                   │
│  │   └── 负载监控                   │
│  └── @blocking                      │
│      ├── 同步执行                   │
│      └── 阻塞保证                   │
└─────────────────────────────────────┘
```

## 组件

### Work Stealing（P13）

| Task | 名称 | 状态 |
|------|------|------|
| task-13-01 | 线程本地队列 | ⚠️ 部分实现 |
| task-13-02 | 窃取算法 | ⏳ 待实现 |
| task-13-03 | 负载监控 | ⏳ 待实现 |

### @blocking（P14）

| Task | 名称 | 状态 |
|------|------|------|
| task-14-01 | 同步执行器 | ⏳ 待实现 |
| task-14-02 | 阻塞语义 | ⏳ 待实现 |

## 工作窃取原理

```
┌────────────────────────────────────────────┐
│  Thread 0          Thread 1          Thread 2 │
│  ┌──────┐          ┌──────┐          ┌──────┐│
│  │Task A│          │Task D│          │Task G││
│  │Task B│          │Task E│          │Task H││
│  └──────┘          └──────┘          └──────┘│
│     ↓                                    ↑   │
│     └───────── 窃取 Task H ───────────────┘   │
└────────────────────────────────────────────┘
```

## @blocking 使用示例

```yaoxiang
# 同步函数：完全顺序执行，无并发优化
main: () -> Void @blocking = () => {
    # 所有 spawn 调用将顺序执行
    result_a = spawn compute_a()
    result_b = spawn compute_b()
    # result_a 和 result_b 顺序计算
}
```

## 依赖

- `runtime/core/standard/dag.rs` (P9)
- `runtime/core/standard/scheduler.rs` (P10-P11)
- `runtime/core/standard/vm.rs` (P12)

## 使用场景

1. **高性能计算**：CPU 密集型任务
2. **科学计算**：需要最优负载均衡
3. **实时系统**：需要确定性执行
