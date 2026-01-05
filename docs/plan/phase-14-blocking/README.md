# Phase 14: @blocking 注解

> **模块路径**: `src/runtime/core/full/blocking/`
> **状态**: ⏳ 待实现
> **依赖**: P10-P11（Scheduler + VM）

## 概述

@blocking 注解提供同步执行保证，禁用所有并发优化，确保函数按顺序执行。

## 与 Standard Runtime 的关系

```
runtime/core/full/
├── work_stealing/         # P13
├── blocking.rs            # P14（@blocking）
└── mod.rs
```

## 文件结构

```
phase-14-blocking/
├── README.md              # 本文档
├── task-14-01-sync-executor.md # 同步执行器
├── task-14-02-semantics.md     # 阻塞语义
├── task-14-03-error-handling.md# 错误处理
└── task-14-04-examples.md      # 使用示例
```

## 完成状态

| Task | 名称 | 状态 | 依赖 |
|------|------|------|------|
| task-14-01 | 同步执行器 | ⏳ 待实现 | task-11-03 |
| task-14-02 | 阻塞语义 | ⏳ 待实现 | task-14-01 |
| task-14-03 | 错误处理 | ⏳ 待实现 | task-14-01 |
| task-14-04 | 使用示例 | ⏳ 待实现 | task-14-02 |

## @blocking 语义

```
┌─────────────────────────────────────────────────────────┐
│              @blocking 执行模型                           │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  spawn f()  ──► 直接调用 f()（非并发）                   │
│                                                         │
│  spawn {   ──► 顺序执行块内表达式                        │
│      a,                                                    │
│      b,                                                    │
│  }                                                        │
│                                                         │
│  spawn for  ──► 顺序迭代，无并行                         │
│      item in list {                                      │
│          process(item)                                   │
│      }                                                   │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

## 同步执行器

```rust
/// 同步执行器（用于 @blocking）
struct SyncExecutor;

impl Executor for SyncExecutor {
    fn spawn(&self, task: impl Future<Output = ()> + Send + 'static) {
        // 直接运行任务，不放入调度队列
        let mut task = task;
        let _ = task.poll(&mut Context::from_waker(noop_waker()));
    }

    fn spawn_blocking<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // 在当前线程同步执行
        f();
    }
}
```

## @blocking 注解处理

```rust
/// @blocking 注解处理器
struct BlockingAnnotator;

impl Annotator for BlockingAnnotator {
    fn annotate_fn(&self, fn_decl: &FnDecl) -> AnnotatedFn {
        // 检查 @blocking 注解
        if fn_decl.has_annotation("blocking") {
            AnnotatedFn {
                executor: ExecutorType::Sync,
                error_handling: ErrorHandling::Propagate,
                spawn_behavior: SpawnBehavior::Sequential,
            }
        } else {
            AnnotatedFn::default()
        }
    }
}
```

## 与并发的交互

```yaoxiang
# @blocking 函数内部调用普通 spawn
main: () -> Void @blocking = () => {
    # 这些 spawn 顺序执行
    a = spawn compute_heavy_task()  # 先执行
    b = spawn compute_heavy_task()  # 后执行

    # 结果按顺序获取
    result_a = await a
    result_b = await b
}
```

## 使用场景

1. **I/O 密集型操作**：需要确定性执行顺序
2. **测试代码**：简化测试逻辑
3. **单线程环境**：不支持并发的环境
4. **关键代码段**：需要避免并发复杂性

## 性能影响

| 操作 | 无 @blocking | 有 @blocking |
|------|--------------|--------------|
| spawn 开销 | 队列操作 | 直接调用 |
| 并发优化 | DAG 调度 | 禁用 |
| 内存占用 | 任务队列 | 最小 |
| 确定性 | 非确定 | 完全确定 |

## 相关文件

- `src/runtime/executor/blocking.rs`

## 相关文档

- [Core Runtime - Full](../phase-08-core-runtime/full/README.md)
- [Phase 11: Scheduler](../phase-11-scheduler/README.md)
