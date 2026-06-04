---
title: "运行时状态"
---

# 运行时（Runtime）

> **模块状态**：有缺口（4 项待改进，阶段 B 未开始）
> **位置**：`src/backends/runtime/`
> **最后更新**：2026-06-01

---

## 模块概述

运行时模块负责任务调度和并发执行。实现了 RFC-008 定义的三层运行时架构：Embedded / Standard / Full。

**代码量**：约 95KB（4 个源文件）

---

## 功能清单

### engine.rs — DAG 调度核心（已实现）

- ✅ **任务生命周期管理**：spawn / mark_running / complete / cancel / yield_now
- ✅ **DAG 依赖图**：硬依赖（hard_deps，传播失败/取消）+ 控制依赖（control_deps，仅排序不传播失败）
- ✅ **资源串行化**：ResourceKey 机制，同键任务严格串行，取消时等待控制依赖以保持资源顺序
- ✅ **环依赖检测**：add_dependency 时通过可达性检测环，返回 CycleDetected 错误
- ✅ **失败/取消传播**：失败沿硬依赖边 BFS 传播，多依赖同时失败时合并取消原因（primary + others）
- ✅ **协作式时间切片**：drive_until_polled 支持 TaskPoll::Pending 让出，多任务公平轮转
- ✅ **目标优先调度**：next_ready_for(target) 优先推进目标依赖链，孤岛任务不阻塞目标完成
- ✅ **统计数据**：RuntimeStats（pending/running/completed/failed/cancelled/total_spawned/avg_execution_time）

### facade.rs — 三层运行时门面（已实现）

- ✅ **Embedded Runtime**：即时执行，spawn 时立即运行闭包，无 DAG，不支持 deps/resources
- ✅ **Standard Runtime**：单线程 DAG 调度，支持普通 TaskFn 和协作式 CoopTaskFn
- ✅ **Full Runtime**：多线程执行，crossbeam channel 通信，worker 线程池
- ✅ **统一门面 Runtime**：通过 RuntimeConfig(mode, workers, work_stealing) 配置

### task.rs — 任务抽象（已实现）

- ✅ **TaskId**：唯一标识，支持 Display
- ✅ **TaskPriority**：Low/Normal/High/Critical 四级
- ✅ **TaskState**：Pending/Running/Completed/Failed/Cancelled
- ✅ **TaskConfig**：builder 模式配置（priority/name/stack_size/parent_id）
- ✅ **Task**：任务实体，持有 id/config/state/result
- ✅ **TaskContext**：任务执行上下文（registers/stack/locals/entry_ip）
- ✅ **Scheduler trait**：抽象调度器接口
- ✅ **TaskSpawner**：泛型任务调度器封装

---

## 测试覆盖

**约 22 个单元测试**，覆盖核心场景：

| 测试文件 | 测试数 | 覆盖场景 |
|----------|--------|----------|
| `engine.rs` | 14 | 线性依赖、菱形依赖、孤岛任务、目标调度、失败传播、取消、资源串行化、环检测、协作切片 |
| `facade.rs` | 5 | Standard/Full 一致性、并行执行、资源串行化、work-stealing 开关、协作切片 |
| `task.rs` | 3 | TaskId、TaskConfig、TaskContext |

---

## RFC 对比（RFC-008）

| RFC-008 要求 | 实现状态 | 说明 |
|-------------|---------|------|
| 三层架构 Embedded/Standard/Full | ✅ 已实现 | facade.rs 三种 RuntimeInner |
| 调度器脱耦（泛型 + 注入） | ⚠️ 部分实现 | task.rs 有 Scheduler trait，但 facade.rs 直接用 enum |
| 同步 = 调度的特例（num_workers=1） | ✅ 已实现 | Full workers=1 测试验证与 Standard 一致 |
| DAG 惰性求值 | ✅ 已实现 | engine.rs 的 LocalRuntime |
| 自底向上执行模型 | ✅ 已实现 | drive_until / next_ready_for 优先目标依赖链 |
| 孤岛 DAG 独立并行不阻塞 | ✅ 已实现 | 有专门测试 |
| WorkStealer | ⚠️ 声明支持但实际未独立实现 | FullRuntime 用 crossbeam channel，无真正 work-stealing 队列 |
| 编译期 DAG 分析 | ❌ 未实现（阶段 B） | 当前 DAG 在运行时构建 |
| 调度器静态库（200-500KB） | ❌ 未实现（阶段 B） | 属于 LLVM AOT 编译器范畴 |
| 反射元数据按需加载 | ❌ 未实现（阶段 B） | 属于后续规划 |

---

## 关键发现

1. **task.rs 中 Scheduler trait 与 facade.rs 的实际调度是分离的**：task.rs 定义了 Scheduler trait，但 facade.rs 并未使用这个 trait，而是直接用 enum 分发
2. **task.rs 有重复类型定义**：SyncValue、TaskResult、RuntimeError、SchedulerStats 在 engine.rs 和 task.rs 中各定义了一份
3. **WorkStealing 未真正实现**：RuntimeConfig 有 work_stealing 字段，但 FullRuntime 实际是简单的线程池 + channel 模型

---

## 代码质量评估

| 维度 | 评分 | 说明 |
|------|------|------|
| 未完成事项 | 4 | WorkStealing、重复类型、统一调度器、阶段 B |
| 测试覆盖 | 良好 | 22 个测试覆盖核心场景 |
| 文档质量 | 良好 | 模块级和方法级文档完整 |
| 代码架构 | 优秀 | 三层架构清晰，职责分离良好 |
| RFC 合规 | 高度符合 | 阶段 A 所有验收标准均已勾选 |

---

## 待改进项

1. **实现真正的 WorkStealing 队列**
2. **消除 task.rs 中的重复类型定义**
3. **统一 Scheduler trait 与 facade.rs 的调度实现**
4. **开始阶段 B：编译器接入**
