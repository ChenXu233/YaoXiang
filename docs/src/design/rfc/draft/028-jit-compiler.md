---
title: "RFC-028：JIT 编译器 — VM 内多级执行引擎"
status: "草案"
author: "晨煦"
created: "2026-06-11"
---

# RFC-028：JIT 编译器 — VM 内多级执行引擎

> **参考**:
> - [RFC-018：LLVM AOT 编译器设计](../review/018-llvm-aot-compiler.md)
> - [RFC-024：基于 spawn 块的并发模型](../accepted/024-concurrency-model.md)
> - [RFC-008：Runtime 并发模型与调度器脱耦设计](../accepted/008-runtime-concurrency-model.md)

## 摘要

本文档提出为 YaoXiang 的 VM 后端引入 Cranelift JIT 编译器，将 VM 从纯解释器升级为**多级执行引擎**：冷代码解释执行，热函数经 Cranelift 编译为原生代码。JIT 路径与 RFC-018 的 LLVM AOT 路径共享 IR 规范化 pass，Cranelift 负责 JIT 的快速编译，LLVM 负责 AOT 的深度优化，各取所长。

**核心定位：JIT 服务 VM，不是替代 VM。**

## 动机

### 为什么需要 JIT？

当前 VM 后端是纯解释器，执行速度比原生代码慢 10-100 倍。开发时频繁运行测试、脚本、本地调试——这些场景不需要 AOT 的极致优化，但需要比解释器明显更快的执行速度。

### 为什么不是只用 LLVM AOT？

LLVM AOT 编译耗时长（秒级），不适合开发迭代。开发需要"改了就跑"的体验：改动一行代码 → 重新运行 → 几乎即时看到结果。Cranelift JIT 编译单个函数只需 1-5ms，用户感知不到编译延迟。

### 为什么是 Cranelift 不是 LLVM ORC JIT？

| 维度 | Cranelift JIT | LLVM ORC JIT |
|------|--------------|--------------|
| 编译速度 | 1-5ms/函数 | 10-100ms/函数 |
| 依赖体积 | 小 | 大（需完整 LLVM） |
| 代码质量 | LLVM -O2 的 70-80% | 极高 |
| 适用场景 | 开发调试，快速迭代 | 不适用（见本文权衡） |

Cranelift 编译快，代码质量足够。LLVM 留给 AOT 做离线深度优化。一个工具做好一件事。

## 提案

### 核心架构

```
VM 执行引擎
├── 解释器层
│   ├── 执行字节码指令
│   ├── 收集热度数据（invocation count + loop backedge count）
│   └── 达到阈值 → 提交编译任务
│
├── JIT 编译层（Cranelift Backend）
│   ├── 编译队列（后台线程，不阻塞解释器）
│   ├── IR → 规范化 → Cranelift IR → 原生代码
│   └── 复用 RFC-018 §4.0 的 IR 规范化 pass（栈→SSA）
│
├── 代码缓存
│   ├── 函数表：函数 ID → {解释器入口, JIT入口(可选)}
│   ├── 编译后函数入口原子替换
│   └── 按模块分组（预留热重载接口）
│
└── 热度分析
    ├── 每函数调用计数 + 循环回边计数
    ├── 定时衰减（避免一次性预热触发编译）
    └── 三级热度：Cold → Warm → Hot → Compiled
```

### 与现有架构对接

```
源码 → 前端（共享）→ IR → ┬→ 字节码 codegen → VM 解释器 → [热函数] → Cranelift JIT
                           │
                           └→ LLVM AOT codegen → .o → 链接 → exe（生产）
```

JIT 和 AOT 共享 **IR 规范化 pass**（`middle/passes/ir_normalize.rs`），底层 codegen 从 LLVM 换成 Cranelift。

### 执行流程

```
函数调用
  → fn_entry.code_ptr.load()
  → ┬─ 解释器 stub（冷状态）：逐条解释字节码
    └─ JIT 原生代码（热状态）：直接执行机器码
  → 返回
```

## 详细设计

### 1. 目录结构

```
src/
├── backends/
│   ├── interpreter/              # 现有 — VM 解释器
│   │   └── executor/
│   │       ├── engine.rs         # 改动 — 调用入口从直接解释改为 FunctionEntry 分发
│   │       └── ...
│   │
│   ├── jit/                      # 新增 — JIT 编译层
│   │   ├── mod.rs                # JIT 模块入口，初始化 Cranelift 上下文
│   │   ├── profiler.rs           # 热度计数 + 衰减 + 阈值决策
│   │   ├── entry.rs              # FunctionEntry + AtomicPtr 管理
│   │   ├── cache.rs              # 代码缓存（mmap 可执行页管理）
│   │   ├── compiler.rs           # IR → Cranelift IR → 原生代码
│   │   ├── types.rs              # YaoXiang 类型 → Cranelift 类型映射
│   │   └── abi.rs                # 函数调用约定（System V / Microsoft x64）
│   │
│   ├── llvm/                     # 规划中 — LLVM AOT（RFC-018）
│   ├── common/                   # 现有
│   └── runtime/                  # 现有
│
└── middle/
    └── passes/
        └── ir_normalize.rs       # 新增 — 共享 IR 规范化（栈→SSA）
                                  #   JIT 和 LLVM AOT 共用
```

**关键约束**：
- `backends/jit/` 只依赖 `middle/`（IR 定义、规范化 pass）、标准库和 Cranelift crate
- `backends/jit/` 不依赖 `backends/llvm/`，两者是平级后端
- `backends/jit/` 不依赖 `backends/interpreter/`，通过 `FunctionEntry` 接口交互

### 2. 热度分析与分层触发

#### 2.1 热度状态机

```
Cold ──(invocation > 50 或 backedge > 500)──→ Warm
Warm ──(invocation > 200)────────────────────→ Hot
Hot ──(提交编译队列，编译完成)──────────────────→ Compiled
```

> 阈值为可配置项，以上为默认值。参考 LuaJIT、JVM C1、V8 Sparkplug 的实际阈值范围（50-1000）。

#### 2.2 计数器

每个函数在 `FunctionEntry`（详见 §4.1）中维护两个原子计数器：

```rust
// FunctionEntry 的热度字段（完整定义见 §4.1）
invocation_count: AtomicU32,   // 函数被调用次数
backedge_count: AtomicU32,     // 循环回边跳转次数
state: AtomicU8,              // Cold | Warm | Hot | Compiled
```

#### 2.3 衰减机制

每 5 秒所有计数器右移 1 位（乘以 0.5）。防止启动时高频但只跑一次的代码（如初始化遍历）触发无意义的 JIT 编译。

```rust
fn decay(entry: &FunctionEntry) {
    entry.invocation_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
    entry.backedge_count.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| Some(v >> 1));
}
```

用位运算，零除法开销。

#### 2.4 编译队列

```
解释器线程                         后台 JIT 线程
    │                                  │
    ├─ 热度达到 Hot                     │
    ├─ 推送编译请求 ─────────────────→  │
    │  (不阻塞解释器)                    ├─ 取出函数 IR
    │                                  ├─ IR 规范化 (栈→SSA)
    │                                  ├─ Cranelift 编译
    │                                  ├─ 写入代码缓存
    │                                  └─ 原子更新函数入口指针
    │  下次调用该函数 ←─────────────────  │
    │  直接走原生代码                     │
```

编译期间函数仍通过解释器执行。编译完成后下一次调用原子切换到 JIT 代码。

### 3. IR → Cranelift 编译管线

#### 3.1 管线

```
YaoXiang IR (栈形式)
  → IR 规范化 pass (栈 → 寄存器/SSA)    ← 复用 RFC-018 §4.0
  → Cranelift IR 构建
  → Cranelift 优化 + 机器码生成
  → 写入代码缓存
```

#### 3.2 YaoXiang 类型 → Cranelift 类型

| YaoXiang 类型 | Cranelift 类型 | 说明 |
|---------------|---------------|------|
| `Int` | `i64` | |
| `Int32` | `i32` | |
| `Float` | `f64` | |
| `Float32` | `f32` | |
| `Bool` | `i8` | Cranelift 无 `i1`，用 `i8` |
| `Char` | `i32` | Unicode 码点 |
| `String` | `{ i64, i64 }` | 指针 + 长度 |
| `Void` | 空元组 | |
| `&T` | — | 零大小，编译后消失 |
| `&mut T` | — | 零大小，编译后消失 |
| `ref T` | `{ i64, i64 }` | 引用计数指针 + 数据指针 |
| `*T` | `i64` | 裸指针 |
| `List(T)` | `{ i64, i64, i64 }` | 数据指针 + 长度 + 容量 |
| 结构体 | Cranelift struct | |
| 记录枚举 | `{ i64, [max_payload] }` | 标签 + union |
| `?T` | `{ i8, T }` | 有值标记 + 数据 |

> 与 RFC-018 §3 的 LLVM 类型表对比：Cranelift 不区分指针类型、无 `i1`，整体更简洁。

#### 3.3 关键指令翻译

| IR 指令 | Cranelift IR |
|---------|-------------|
| `Add { dst, lhs, rhs }` | `iadd`（整数）/ `fadd`（浮点） |
| `Sub { dst, lhs, rhs }` | `isub` / `fsub` |
| `Mul { dst, lhs, rhs }` | `imul` / `fmul` |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv` |
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp eq` |
| `Jmp(label)` | `jump` |
| `JmpIf(cond, label)` | `brnz` |
| `Ret(Some(v))` | `return` |
| `Call { dst, func, args }` | `call` |
| `Load { dst, src }` | `load` |
| `Store { dst, src }` | `store` |
| `Spawn { ... }` | 调用运行时 `task_spawn` + `task_wait_all` |

> 完整翻译表见 RFC 正文。核心原则：Cranelift 指令集覆盖 YaoXiang IR 所有操作，不存在语义缺口。

#### 3.4 两种规范化共存

VM 解释器需要栈语义（`Push`/`Pop`/`Dup`/`Swap`），Cranelift JIT 和 LLVM AOT 需要寄存器/SSA。IR 规范化 pass 做一次转换（RFC-018 §4.0），JIT 和 AOT 共用，不改变 IR 本身的表示。每个后端按自己的需求消费同一个 IR。

### 4. 函数入口表与原子替换

#### 4.1 FunctionEntry

```rust
struct FunctionEntry {
    /// 原子可替换的执行目标
    code_ptr: AtomicPtr<u8>,
    /// 不变元数据
    bytecode: &'static [u8],        // 解释器 fallback
    ir: &'static FunctionIR,        // JIT 编译的输入
    /// 运行时统计
    invocation_count: AtomicU32,
    backedge_count: AtomicU32,
    state: AtomicU8,                // Cold | Warm | Hot | Compiled
}
```

#### 4.2 入口分发

```
调用方
  → fn_entry.code_ptr.load(Ordering::Acquire)
  → ┬─ 解释器 stub 地址 → 执行解释器，逐条解释字节码
    └─ JIT 代码地址      → 直接跳转原生代码
```

一次指针解引用。现代 CPU 分支预测器对间接跳转的处理：首次预测错误，之后全对。开销约 1 cycle。

#### 4.3 原子切换

编译完成后一次 CAS：

```rust
fn install_jit_code(entry: &FunctionEntry, jit_code: *mut u8) -> bool {
    entry.code_ptr.compare_exchange(
        INTERPRETER_STUB,      // 期望：仍指向解释器
        jit_code,              // 替换为：JIT 代码
        Ordering::AcqRel,
        Ordering::Acquire,
    ).is_ok()
}
```

无暂停解释器，无安全点等待，无调用点遍历。一个原子操作完成切换。

### 5. 代码缓存

#### 5.1 结构

```
CodeCache:
  modules:
    "main.yao":
      functions:
        "compute"    → FunctionEntry (state: Compiled)
        "process"    → FunctionEntry (state: Cold)
        "init"       → FunctionEntry (state: Compiled)
      native_pages:   [ mmap'd executable memory pages ]
    "lib.yao":
      functions:
        "helper"     → FunctionEntry (state: Compiled)
      native_pages:   [ mmap'd executable memory pages ]
```

#### 5.2 可执行内存管理

```rust
struct NativePage {
    ptr: *mut u8,
    size: usize,
    used: AtomicUsize,     // 已用字节数
    remaining: usize,       // 剩余容量
}

impl CodeCache {
    fn allocate(&self, code_size: usize) -> *mut u8;
    fn deallocate(&self, ptr: *mut u8, code_size: usize);  // 仅在模块失效时调用
}
```

每个模块分配连续的 mmap 可执行页，模块内的所有 JIT 函数从同一页分配。模块失效时整页回收，无需逐函数释放。

### 6. 热重载预留扩展点

以下接口编译通过但热重载实现前不调用。接口设计原则：**JIT 实现时只需 `insert` 和单函数 `compare_exchange`，模块级操作留给热重载。**

```rust
/// 代码缓存扩展接口（预留，不实现）
trait CodeCacheExt {
    /// 失效整个模块的所有 JIT 代码，回退到解释器
    fn invalidate_module(&self, module_path: &str);

    /// 根据源码位置范围失效特定函数
    fn invalidate_range(&self, file: &str, start: u32, end: u32);

    /// 原子替换整个模块的函数表
    fn swap_module(&self, module_path: &str, new_functions: HashMap<String, FunctionEntry>);
}

/// 编译队列扩展接口（预留，不实现）
trait CompileQueueExt {
    /// 优先级插队（热重载编译高于普通 JIT 编译）
    fn submit_priority(&self, task: CompileTask);
}
```

**为什么按模块分组？** JIT 本身只需要函数。按模块组织完全是为热重载服务的：模块重编译后，可以原子性地替换整个模块的函数集，而不是逐个函数 CAS——后者在函数间有循环依赖时会导致不一致状态。

## 权衡

### 优点

1. **零感知编译延迟**：Cranelift 1-5ms/函数，后台线程编译，解释器不暂停
2. **共享基础设施**：JIT 和 AOT 共享 IR 规范化 pass（RFC-018 §4.0），不重复造轮子
3. **无破坏性**：纯增量功能。VM 不变，解释器不变，只是多了一条更快的热路径
4. **无 LLVM 依赖**：VM 不引入 LLVM，保持轻量
5. **天然支持多平台**：Cranelift 原生支持 x86_64 和 ARM64，覆盖所有目标平台
6. **热重载预留**：代码缓存按模块分组 + 函数入口间接跳转，为未来热重载打下结构基础

### 缺点

1. **Cranelift 新依赖**：引入新的外部 crate，需要熟悉其 API
2. **调试复杂度**：JIT 生成的代码栈帧需要与解释器栈帧兼容，调试信息映射需要额外处理
3. **冷启动热度延迟**：程序启动后前几秒没有 JIT 加速，需要热度积累
4. **平台 ABI**：不同平台（Linux/macOS/Windows）的 mmap 和调用约定需要分别适配

### 与相关 RFC 的一致性

| RFC | 一致性 |
|-----|--------|
| RFC-018 LLVM AOT | ✅ 共享 IR 规范化 pass，JIT 和 AOT 是平级后端 |
| RFC-024 spawn 块并发 | ✅ spawn 块编译为运行时函数调用 |
| RFC-008 运行时架构 | ✅ 三层运行时（Embedded/Standard/Full）均支持 JIT |

## 替代方案

| 方案 | 为什么不选 |
|------|-----------|
| 仅用 LLVM AOT，不做 JIT | 开发时需要重新编译整个程序，丧失快速迭代体验 |
| LLVM ORC JIT | 编译延迟高（10-100ms），LLVM 依赖大，不适合嵌入 VM |
| 自定义轻量 JIT（dynasm） | 手写后端的维护成本高，不如 Cranelift 成熟 |
| 模板 JIT | 零优化，代码质量差，白白浪费 JIT 编译的时间 |
| 全程序 JIT（无解释器） | 冷启动慢，简单脚本不值得编译 |

## 依赖关系

- RFC-018（LLVM AOT）→ 共享 IR 规范化 pass
- RFC-024（spawn 块并发）→ spawn 块的 JIT 编译
- RFC-008（运行时架构）→ 三层运行时 JIT 支持
- Cranelift crate → JIT 后端

## 参考文献

- [Cranelift IR Documentation](https://github.com/bytecodealliance/wasmtools/tree/main/cranelift)
- [RFC-018：LLVM AOT 编译器设计](../review/018-llvm-aot-compiler.md)
- [RFC-024：基于 spawn 块的并发模型](../accepted/024-concurrency-model.md)
- [RFC-008：Runtime 并发模型与调度器脱耦设计](../accepted/008-runtime-concurrency-model.md)
- Hölzle, U. (1994). *Adaptive Optimization for Self: Reconciling High Performance with Exploratory Programming*. Stanford.

---
## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/src/design/rfc/draft/` | 作者草稿，等待提交审核 |
| **审核中** | `docs/src/design/rfc/review/` | 开放社区讨论和反馈 |
| **已接受** | `docs/src/design/rfc/accepted/` | 成为正式设计文档 |
