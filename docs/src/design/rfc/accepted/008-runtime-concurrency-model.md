---
title: RFC-008：Runtime 并发模型与调度器脱耦设计
---

# RFC-008：Runtime 并发模型与调度器脱耦设计

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-05
> **最后更新**: 2026-05-11（修剪 + 新增双后端模型、编译器与运行时分离、调度器静态库、反射按需加载）

> **参考**:
> - [RFC-001: 并作模型与错误处理系统](./001-concurrent-model-error-handling.md)
> - [RFC-011: 泛型系统设计](./011-generic-type-system.md)

## 摘要

本文档定义 Runtime 架构的关键设计：

1. **运行时三层架构**：Embedded（即时执行） → Standard（DAG 调度） → Full（工作窃取）
2. **编译与运行分离**：编译阶段完全相同，区别仅在运行时执行方式
3. **双后端模型**：VM（开发调试）与 LLVM AOT（生产发布），行为完全一致
4. **调度器 = 静态库**：AOT 编译时调度器链接进 exe，约 200-500KB，无 GC
5. **同步只是调度的特例**：num_workers=1 即同步模式

### 关键澄清：这不是 Java

```
Java:   .java → .class → JVM（解释/JIT + GC）        ← 永远需要虚拟机
YaoXiang 开发: .yx → IR → VM 执行（快速迭代、步进调试）
YaoXiang 生产: .yx → IR → LLVM → 原生 exe（调度器链接进去）

VM 是开发工具，不是运行时本质。跟 Go 的 go run vs go build 一样。
最终 exe = 你的原生代码 + 调度器静态库 + 反射元数据。没有解释器，没有 JIT，没有 GC。
```

## 动机

### 核心矛盾

| 矛盾 | 描述 |
|------|------|
| 透明性 vs 可控性 | 并发应该是默认行为，但用户应该能控制 |
| 核心 vs 可选 | DAG 是核心，但 WorkStealing 是 num_workers>1 的高级特性 |
| 单线程 vs 并发 | 单线程模式下并发表现为异步，同步只是调度的特例 |

---

## 提案

### 1. Runtime 三层架构

```
┌──────────────────────────────────────────────────────────────────┐
│                    编译阶段（所有模式相同）                        │
│                                                                  │
│  Source Code → Lexer → Parser → TypeCheck → Codegen → IR        │
│                                                                  │
│  ⚠️ 同一套语法解析、类型检查、代码生成、IR 输出                    │
└──────────────────────────────────────────────────────────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          ▼                    ▼                    ▼
┌──────────────────┐ ┌───────────────┐ ┌──────────────────┐
│ 🟢 Embedded      │ │ 🔵 Standard   │ │ 🟣 Full          │
│ 即时执行器        │ │ DAG 调度器    │ │ Full 调度器      │
│ 同步执行         │ │ 惰性求值      │ │ 并行优化         │
│ 无 DAG 调度      │ │ 自动并发      │ │ 工作窃取         │
└──────────────────┘ └───────────────┘ └──────────────────┘
```

| 阶段 | Embedded | Standard | Full |
|------|----------|----------|------|
| 编译 | 相同 | 相同 | 相同 |
| 执行模式 | 同步 | 惰性+并发 | 并行 |
| 内存占用 | 低 | 中 | 高 |
| 并发能力 | 无 | 自动 | 自动+并行 |
| DAG 惰性求值 | 无 | ✅ | ✅ |
| WorkStealer | 无 | 无 | ✅ |

**Embedded Runtime**：目标 WASM/游戏脚本/规则引擎。即时执行器，无 DAG，高性能低占用。

**Standard Runtime**：目标 Web 服务/数据管道。DAG 惰性求值 + 自动并发。num_workers=1 即单线程异步。

**Full Runtime**：目标科学计算/大规模并行。Standard + WorkStealer 负载均衡。

### 2. 调度器脱耦：泛型 + 注入

核心原则：VM 不直接依赖具体调度器，通过泛型参数 `[S]` 调用。

```yaoxiang
# 调度器接口定义
Scheduler: Type = {
    spawn: (Task) -> TaskId,
    await: (TaskId) -> Result,
    spawn_with_deps: (Task, List(TaskId)) -> TaskId,
    await_all: (List(TaskId)) -> List(Result),
    stats: () -> SchedulerStats,
}

# 单线程调度器
SingleThreadScheduler: Scheduler = {
    spawn: (task) => { task_queue.push(task); generate_task_id() },
    await: (task_id) => { ... },
    spawn_with_deps: (task, deps) => { ... },
    await_all: (task_ids) => { ... },
    stats: () => { queue_size: task_queue.len() },
}

# 多线程调度器
MultiThreadScheduler: Scheduler = {
    spawn: (task) => { work_queue.push(task); generate_task_id() },
    await: (task_id) => { wait_for_completion(task_id) },
    spawn_with_deps: (task, deps) => { ... },
    await_all: (task_ids) => { ... },
    stats: () => { workers: get_worker_stats() },
}

# VM 通过泛型使用调度器
create_vm: [S: Scheduler](scheduler: S) -> VM = (scheduler) => {
    VM(scheduler: scheduler, memory: create_memory(), dag: create_dag())
}
```

**核心要点**：
- 编译期多态，零运行时开销
- 无需 Trait 对象
- 泛型类型约束 `[S: Scheduler]` 已在 RFC-011 中定义

### 3. 同步 = 调度的特例

```
❌ 误解：禁用调度器
✅ 正确：使用单 worker 的调度器

num_workers = 1 → 单线程异步调度
num_workers > 1 → 多线程并行调度

同一个调度器接口，只是配置不同。消除特殊情况。
```

### 4. DAG 的地位

| 层级 | 包含 DAG | 说明 |
|------|----------|------|
| Core Runtime | ✅ | 惰性求值核心 |
| Standard Runtime | ✅ | DAG 调度器 |
| Embedded Runtime | ❌ | 即时执行，无 DAG |

### 5. 自底向上执行模型

```
用户代码（同步语法）：
    a = fetch(url0)
    b = fetch(url1)
    print(a)

编译时分析（自底向上）：
    print(a) 需要 a → 依赖 fetch(url0)
    fetch(url1) 没有人需要 → 孤岛 DAG

运行时调度（从叶子开始）：
    fetch(url0) → print(a)    ← 依赖链，按序
    fetch(url1)                ← 孤岛，独立并行
```

**核心要点**：
- 从"需要结果的地方"反向分析依赖
- 叶子节点优先并行执行
- 孤岛 DAG 独立并行，不阻塞主流程

---

### 6. 编译模型：双后端 + 静态链接运行时

#### 6.1 两个后端，一种行为

```
                      ┌─────────────────────┐
                      │   编译前端（统一）     │
                      │   Lexer → Parser     │
                      │   → TypeCheck        │
                      │   → DAG 分析         │
                      │   → 逃逸分析          │
                      │   → 环检测            │
                      └──────────┬──────────┘
                                 │
                    ┌────────────┴────────────┐
                    ▼                         ▼
        ┌───────────────────┐     ┌───────────────────┐
        │   VM 后端（开发）   │     │  LLVM 后端（生产）  │
        │                   │     │                   │
        │  生成 IR/字节码    │     │  生成原生代码       │
        │  VM 解释执行       │     │  链接运行时静态库   │
        │  支持步进调试       │     │  输出 .exe         │
        │  快速迭代          │     │  零解释开销         │
        └───────────────────┘     └───────────────────┘
                 │                         │
                 ▼                         ▼
           行为完全一致                 行为完全一致
```

**VM 后端**：开发时使用。修改代码 → 即时运行 → 步进调试 → 快速迭代。行为和最终 exe 完全一致。

**LLVM 后端**：发布时使用。AOT 编译到原生代码，调度器作为静态库链接进去。没有解释器，没有 JIT。

#### 6.2 调度器 = 静态库，不是虚拟机

```
最终 exe 的内部结构：

┌────────────────────────────────────────────┐
│  你的代码（原生机器码）                       │
│  ├── 编译期已确定的 DAG 执行计划              │
│  ├── 内联的 Move/ref/clone 操作              │
│  └── RAII 释放代码                          │
├────────────────────────────────────────────┤
│  运行时静态库（~200-500KB）                   │
│  ├── 线程池（固定大小 = num_workers）         │
│  ├── 事件循环（libuv / io_uring）            │
│  ├── 工作窃取队列（仅 Full Runtime）          │
│  ├── 内存分配器（jemalloc / mimalloc）       │
│  └── 反射元数据（按需加载，不常驻内存）         │
├────────────────────────────────────────────┤
│  没有：                                      │
│  ❌ 字节码解释器                             │
│  ❌ JIT 编译器                               │
│  ❌ GC                                      │
│  ❌ 虚拟机                                    │
└────────────────────────────────────────────┘
```

对比：

| | Java | Go | YaoXiang |
|------|------|-----|-----------|
| 编译产物 | 字节码 | 原生代码 | 原生代码 |
| 执行方式 | JVM 解释/JIT | 直接执行 | 直接执行 |
| 运行时大小 | ~200MB（JVM） | ~1-2MB（含 GC） | **~200-500KB（无 GC）** |
| 内存管理 | GC | GC | **RAII（确定）** |
| 反射 | 常驻内存 | 常驻内存 | **exe 中储存，按需加载** |

#### 6.3 为什么调度器性能恒定

**关键洞察**：大部分工作在编译期完成，运行时只做"执行"。

```
编译期（一次性，不进入运行时）：
    ├── 构建全局 DAG：谁依赖谁
    ├── 拓扑排序：确定执行顺序
    ├── 识别孤岛：可并行的子树
    ├── 逃逸分析：ref → Rc 还是 Arc
    ├── 环检测：自动降级 Weak 或报错
    └── 内联：小函数直接展开

运行时（每次执行，数据结构固定）：
    ├── 按编译期确定的 DAG 顺序分发任务到线程池
    ├── 遇到 I/O → 挂起当前任务，事件循环接管
    ├── 任务就绪 → 放回就绪队列
    └── 就这些。
```

**调度器本身是固定大小的数据结构**：线程池、事件循环、工作队列。没有动态增长，没有自适应重优化，没有 GC 扫描。行为完全可预测。

编译期已经把"调度什么"算完了，运行时只做"执行"。这跟 tokio 不同——tokio 在运行时动态构建 Future 链。YaoXiang 的 DAG 是静态的。

#### 6.4 反射：储存不常驻

反射元数据在编译期生成，储存在 exe 的独立段（section）中。程序启动时不加载。首次请求反射时，按需 mmap 进内存。类似于：

```
exe 布局：
  .text     ← 你的代码
  .rodata   ← 常量
  .reflect  ← 反射元数据（类型信息、函数签名等）
              mmap 按需加载，不访问不占内存
```

**取舍**：exe 体积增大（含反射数据），但运行时不访问则零内存开销。首次访问有加载延迟（类似 JIT 预热），后续零开销。



```
src/
├── core/                    # 所有运行时共享
│   ├── value.rs
│   ├── allocator.rs
│   └── ownership.rs
├── frontend/                # 所有后端共享
│   ├── lexer/
│   ├── parser/
│   ├── typecheck/
│   └── dag/                 # ★ DAG 分析（编译期）
│       ├── builder.rs       #   构建依赖图
│       ├── escape.rs        #   逃逸分析（ref → Rc/Arc）
│       ├── cycle.rs         #   环检测 + 自动降级
│       └── topology.rs      #   拓扑排序
├── codegen/                 # 代码生成
│   ├── ir.rs                # IR 定义（VM 和 LLVM 共用）
│   ├── vm_backend/          # VM 后端（开发调试）
│   │   ├── bytecode.rs
│   │   └── compiler.rs
│   └── llvm_backend/        # LLVM 后端（生产发布）
│       └── compiler.rs
├── embedded/                # 🟢 Embedded Runtime
│   └── executor.rs
├── runtime/                 # 🔵 运行时静态库（链接进 exe）
│   ├── thread_pool.rs       #   固定大小线程池
│   ├── event_loop.rs        #   I/O 事件循环（libuv/io_uring）
│   ├── dag_executor.rs      #   按编译期 DAG 执行
│   └── scheduler/
│       ├── single_thread.rs
│       └── multi_thread.rs
├── full/                    # 🟣 Full Runtime（可选链接）
│   └── work_stealer.rs      #   工作窃取
├── reflect/                 # 反射元数据
│   ├── metadata.rs          #   元数据生成（编译期）
│   └── loader.rs            #   按需加载（运行时）
└── vm/                      # VM 解释器（仅开发用）
    └── executor.rs
```

---

## 权衡

### 优点

- **清晰分层**：Embedded / Standard / Full 三层
- **编译复用**：前端代码完全共享
- **泛型脱耦**：编译期多态，零开销
- **一致性**：同步只是调度的特例
- **嵌入式友好**：高性能 + 低内存 + 快速启动

### 缺点

- **初始复杂度**：需要定义调度器接口和多种运行时变体
- **编译期绑定**：调度器类型在编译期确定

---

## 设计决策记录

| 决策 | 决定 | 日期 |
|------|------|------|
| 调度器脱耦方案 | 泛型 + 注入 | 2025-01-05 |
| 单线程模式 | 同步是调度的特例 | 2025-01-05 |
| 异步实现 | DAG 天然支持 | 2025-01-05 |
| WorkStealer | Full Runtime 高级特性 | 2025-01-05 |
| 嵌入式设计 | 即时执行，无 DAG 调度 | 2025-01-05 |
| 编译阶段 | 所有运行时共享同一套前端 | 2025-01-05 |
| 运行时分层 | Embedded / Standard / Full | 2025-01-05 |
| 类型约束 | RFC-011 已定义 | 2025-01-25 |
| 依赖图构建 | 静态依赖图，编译期确定 | 2025-01-05 |
| 双后端模型 | VM（开发调试）+ LLVM AOT（生产），行为一致 | 2026-05-11 |
| 调度器形态 | 静态库链接进 exe，~200-500KB，无 GC | 2026-05-11 |
| 反射元数据 | 编译进 exe 独立段，mmap 按需加载 | 2026-05-11 |
| 调度器性能 | 编译期完成 DAG 分析，运行时仅执行 | 2026-05-11 |

---

## 参考文献

- [RFC-001: 并作模型与错误处理系统](./001-concurrent-model-error-handling.md)
- [RFC-011: 泛型系统设计](./011-generic-type-system.md)
- [Rust async 运行时设计](https://tokio.rs/)
- [Go 调度器设计](https://golang.org/src/runtime/proc.go)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 作者草稿 |
| **审核中** | `docs/design/rfc/` | 开放社区讨论 |
| **已接受** | `docs/design/accepted/` | 正式设计文档 |
| **已拒绝** | `docs/design/rfc/` | 保留在 RFC 目录 |
