---
title: RFC-018：LLVM AOT 编译器与 L3 透明并发设计
---

# RFC-018：LLVM AOT 编译器与 L3 透明并发设计

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2026-02-15
> **最后更新**: 2026-02-16

> **参考**:
> - [RFC-001: 并作模型与错误处理系统](./accepted/001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 并发模型与调度器脱耦设计](./accepted/008-runtime-concurrency-model.md)
> - [RFC-009: 所有权模型设计](./accepted/009-ownership-model.md)

## 摘要

本文档设计 YaoXiang 语言的 LLVM AOT 编译器，目标是通过预先编译生成机器码 + DAG 元数据，由运行时调度器根据 DAG 依赖关系**延迟调度**执行。此设计与 Rust async/await + tokio 运行时模式有本质区别：Rust 在编译期确定 await 点，而 YaoXiang 在运行时按需调度[^1]。遵循 RFC-001 的 L3 透明并发设计：默认 @auto（自动并行），@block 同步是特例，解决颜色函数问题。

## 动机

### 为什么需要 LLVM AOT 编译器？

当前 YaoXiang 仅有解释器作为执行后端，存在以下问题：

| 问题 | 影响 |
|------|------|
| 性能瓶颈 | 解释执行比机器码慢 10-100x |
| 部署复杂 | 需要携带解释器和运行时 |
| 颜色函数问题 | 同步函数不能调用并发函数 |

### 颜色函数问题与 L3 透明并发

**传统设计（当前）**：
- 同步函数（蓝色）→ 不能调用 → 并发函数（红色）
- 同步是默认，并发需要 `spawn` 标记
- 颜色会"传染"：一旦用了并发，同一调用链上都是并发

**RFC-001 L3 透明并发（目标）**：
- L3：默认透明并发（@auto）
- L2：显式 spawn 并发
- L1：@block 同步模式

**翻转后的设计（RFC-018）**：
- 默认 L3 透明并发，编译时自动分析 DAG 依赖
- 解决颜色函数问题：同步函数可以直接调用"默认并发"的代码
- @block 仅作为特例强制串行执行

### 核心创新：延迟调度

本设计的核心创新在于**延迟调度**（Lazy Scheduling）[^2]：

```
传统函数调用：
  call fetch(url) → 执行 → 返回结果

延迟调度：
  call fetch(url) → 跳过执行，挂起函数 → 记录"需要执行"
                  → 继续执行下一行
                  → 当需要结果时才执行
```

**关键区别**：
- 不返回"懒迭代器"或其他中间数据结构
- 直接挂起函数，不占用额外周期
- 调度器按需触发执行

### 与 Rust async 的对比

```
┌─────────────────────────────────────────────────────────────────┐
│                      Rust async 模式                            │
├─────────────────────────────────────────────────────────────────┤
│  编译时：生成状态机 + 机器码                                    │
│  运行时：tokio 调度器根据状态机调度                            │
│  特点：await 点在编译期确定，状态机管理执行                     │
│  粒度：函数级别                                                │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      YaoXiang LLVM AOT 模式                    │
├─────────────────────────────────────────────────────────────────┤
│  编译时：生成机器码 + DAG 元数据                               │
│  运行时：DAG 调度器根据依赖关系延迟调度                       │
│  特点：调用在需要时才执行，按需调度                            │
│  粒度：函数块内 DAG                                            │
└─────────────────────────────────────────────────────────────────┘
```

## 提案

### 核心设计

```
┌─────────────────────────────────────────────────────┐
│  编译时                                              │
│  ┌─────────┐  ┌─────────┐  ┌─────────┐           │
│  │ Parser  │→│DAG分析  │→│LLVM Codegen│→ 机器码  │
│  └─────────┘  └─────────┘  └─────────┘           │
│                      ↓                           │
│              生成：DAG 元数据                         │
└─────────────────────────────────────────────────────┘
                      ↓
┌─────────────────────────────────────────────────────┐
│  运行时                                              │
│  ┌─────────────────────────────────────────────┐ │
│  │  DAG 调度器库                                 │ │
│  │  • 加载机器码                                │ │
│  │  • 读取 DAG 元数据                           │ │
│  │  • 延迟调度：挂起调用，按需执行              │ │
│  │  • 支持并行/串行执行                         │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
```

### 延迟调度执行流程

```
Phase 1: 遍历函数体，挂起调用
─────────────────────────────────────────
遇到 fetch(url0) → 跳过（挂起），记录"需要执行"
遇到 fetch(url1) → 跳过（挂起），记录"需要执行"
遇到 fetch(url2) → 跳过（挂起），记录"需要执行"
    ↓
此时不执行任何 fetch，只构建待执行列表

Phase 2: 并发执行（控制并发数）
─────────────────────────────────────────
调度器从待执行列表中取任务
    ↓
    ↓ 控制并发数（比如 16 个）
    ↓
执行 fetch(url0), fetch(url1), ... fetch(url15)

Phase 3: 需要值时触发
─────────────────────────────────────────
当 parse_page(page0) 需要 page0 时
    ↓
检查 page0 是否已就绪
    ↓
就绪 → 执行 parse_page
未就绪 → 等待，完成后继续
```

### 编译产物结构

```rust
/// 编译产物：机器码 + DAG 元数据
pub struct CompiledArtifact {
    /// LLVM 编译的机器码（ELF/Mach-O/COFF）
    machine_code: Vec<u8>,

    /// DAG 元数据：描述函数依赖关系
    dag: DAGMetadata,

    /// 入口点表
    entries: Vec<EntryPoint>,

    /// 类型信息（用于 FFI）
    type_info: TypeInfo,
}

/// DAG 元数据
pub struct DAGMetadata {
    /// 节点：函数调用
    nodes: Vec<DAGNode>,
    /// 边：依赖关系 (from, to)
    edges: Vec<(usize, usize)>,
}

/// 单个 DAG 节点
pub struct DAGNode {
    /// 函数 ID
    pub function_id: usize,
    /// 依赖的节点 ID
    pub deps: Vec<usize>,
    /// 副作用标记（@IO / @Pure）
    pub effect: EffectTag,
}
```

### 运行时调度器接口

```rust
/// DAG 调度器 trait
pub trait DAGScheduler: Send + Sync {
    /// 调度执行
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue;

    /// 单函数执行
    fn execute(&self, func: &CompiledFunction, args: &[RuntimeValue]) -> RuntimeValue;
}

/// 调度器实现
pub struct DefaultDAGScheduler {
    /// 线程池
    thread_pool: ThreadPool,
    /// 编译产物
    artifact: CompiledArtifact,
    /// 最大并发数
    max_parallelism: usize,
}

impl DefaultDAGScheduler {
    pub fn new(artifact: CompiledArtifact, num_workers: usize) -> Self {
        Self {
            thread_pool: ThreadPool::new(num_workers),
            artifact,
            max_parallelism: num_workers * 2, // 自适应粒度控制
        }
    }
}

impl DAGScheduler for DefaultDAGScheduler {
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue {
        // 1. 遍历函数体，挂起所有调用
        // 2. 构建待执行任务列表
        // 3. 按依赖顺序调度执行（控制并发数）
        // 4. 需要值时触发执行
        // 5. 返回结果
    }
}
```

### 语法设计（统一语法：name: type = expression）

```yaoxiang
# 变量
x: Int = 42

# 函数（参数名在签名中）
add: (a: Int, b: Int) -> Int = a + b

# 主函数（默认 @auto 并发）
main: (urls: Vec[String]) -> () = {
    # 并发下载所有页面（延迟调度）
    let pages = urls.map(|url| fetch(url));

    # 并发解析所有页面
    let results = pages.map(|page| parse_page(page));

    # 过滤链接（纯函数，可并行）
    let all_links = results.flat_map(|r| filter_links(r.links));

    # 顺序保存（@IO 保证顺序）
    for result in results {
        save_result(result);
    }

    print(`Fetched ${results.len()} pages`);
}

# 纯函数：解析页面
parse_page: (page: Page) -> Result = {
    title = extract_title(page.content);
    links = extract_links(page.content);
    Result { title, links }
}

# 纯函数：过滤有效链接
filter_links: (links: Vec[String]) -> Vec[String] =
    links.filter(|l| l.starts_with("http"))

# 外部 I/O：下载页面（隐式 @IO，用户无感知）
fetch: (url: String) -> Page = {
    content = http_get(url);
    Page { url, content }
}

# 外部 I/O：保存结果
save_result: (result: Result) -> () = {
    database.save(result);
}

# L2 显式并发
spawn_main: () -> () = {
    spawn { fetch(url0) };
    spawn { fetch(url1) };
}

# L1 强制串行
serial_main: () -> () = {
    block {
        db.begin();
        db.write(data1);
        db.write(data2);
        db.commit();
    }
}
```

### DAG 示例：网页爬虫

```
main 函数 DAG：
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  fetch(url0) ──┐                                           │
│  fetch(url1) ──┼──→ parse_page ──→ filter_links ──┐      │
│  fetch(url2) ──┘                                       │      │
│                                                          │      │
│                     save_result ──→ print              │      │
│                          ↑                              │      │
│                          └──────────────────────────────┘      │
│                                                             │
└─────────────────────────────────────────────────────────────┘

节点说明：
┌──────────────────┬────────────┬────────────────────────────┐
│ 节点              │ 副作用     │ 说明                       │
├──────────────────┼────────────┼────────────────────────────┤
│ fetch(url0)      │ @IO       │ 并发下载                   │
│ fetch(url1)      │ @IO       │ 并发下载                   │
│ fetch(url2)      │ @IO       │ 并发下载                   │
│ parse_page       │ @Pure     │ 并行解析                   │
│ filter_links     │ @Pure     │ 并行过滤                   │
│ save_result      │ @IO       │ 顺序保存（I/O保证顺序）    │
│ print            │ @IO       │ 最后执行                   │
└──────────────────┴────────────┴────────────────────────────┘
```

### 调度器执行阶段

```
Phase 1: 并发下载
─────────────────────────────────────────
线程1: fetch(url0) ──────────┐
线程2: fetch(url1) ─────────┼──→ 3个并发任务（限制最大并发数）
线程3: fetch(url2) ──────────┘

Phase 2: 并发解析
─────────────────────────────────────────
线程1: parse_page(page0) ──┐
线程2: parse_page(page1) ──┼──→ 3个并发任务
线程3: parse_page(page2) ──┘

Phase 3: 并发过滤
─────────────────────────────────────────
线程1: filter_links(result0) ──┐
线程2: filter_links(result1) ──┼──→ 3个并发任务
线程3: filter_links(result2) ──┘

Phase 4: 顺序保存
─────────────────────────────────────────
线程1: save_result(result0) → 等待完成
线程1: save_result(result1) → 等待完成
线程1: save_result(result2) → 等待完成

Phase 5: 输出
─────────────────────────────────────────
线程1: print("Fetched 3 pages")
```

## 详细设计

### 模块结构

```
src/backends/llvm/
├── mod.rs           # 模块入口 + Executor 实现
├── context.rs       # LLVM 上下文管理
├── types.rs         # 类型映射 (YaoXiang → LLVM)
├── values.rs        # 值映射 (寄存器 → LLVM Value)
├── codegen.rs       # 核心代码生成
├── dag.rs           # DAG 分析与生成
├── scheduler.rs      # 运行时调度器
└── tests.rs         # 测试
```

### 类型映射

| YaoXiang 类型 | LLVM 类型 |
|---------------|----------|
| `Int` | `i64` |
| `Float` | `f64` |
| `Bool` | `i1` |
| `String` | `ptr` (结构体) |
| `Arc[T]` | `{ i32, T }` (引用计数结构体) |
| `ref T` | `ptr` (Arc 指针) |
| `List[T]` | `ptr` (动态数组) |
| `Struct` | `struct` (对应结构体) |

### 指令翻译

每个 `BytecodeInstr` 直接翻译为对应的 LLVM IR 指令：

| BytecodeInstr | LLVM IR |
|---------------|---------|
| `BinaryOp { add }` | `llvm.add` |
| `CallStatic` | `llvm.call` |
| `ArcNew` | `call @Arc_new` |
| `LoadElement` | `llvm.getelementptr` + `llvm.load` |

### 运行时库

```rust
// 核心运行时函数
extern "C" {
    // 引用计数
    fn Arc_new(ptr: *mut u8) -> i32;
    fn Arc_clone(ref_count: *mut i32) -> i32;
    fn Arc_drop(ref_count: *mut i32);

    // 堆分配
    fn Alloc(size: usize) -> *mut u8;
    fn Dealloc(ptr: *mut u8);

    // DAG 调度
    fn dag_schedule(dag: *const DAGMetadata, entry: usize) -> RuntimeValue;
}
```

### 调度策略

| 注解 | 场景 | 调度策略 |
|------|------|----------|
| `@auto`（默认，L3） | 透明并发 | DAG 延迟调度，无依赖并行执行 |
| `@eager` | 急切求值 | 等待依赖完成后执行，保证顺序 |
| `@spawn`（L2） | 手动并发 | 强制后台执行 |
| `@block`（L1） | 强制同步 | 无 DAG，纯串行执行 |
| 循环依赖 | 运行时检测 | 报错 |

### 副作用处理：隐式 Effect System

用户无感知副作用处理，编译器自动推断：

```
用户代码：
  print("a");
  print("b");
  let x = compute(1);
  let y = compute(2);

编译器推断：
  print → @IO（外部调用）
  compute → @Pure（纯函数）

调度器执行：
  print("a") ──→ 顺序（都是 @IO）
  print("b") ──→ 顺序
  compute(1) ─┬─→ 并行（DAG 调度）
  compute(2) ─┘
```

### 粒度控制：解决任务爆炸问题

当 `fetch` 任务过多时，可能创建无数并发任务导致卡顿。解决方案：

**1. 并发数限制**

```rust
// 调度器策略
scheduler.max_parallelism = num_cores * 2; // 例如 8 核 = 16 并发
```

**2. 自适应粒度**[^2]

```
负载高时：合并多个小任务为一个大任务
负载低时：保持细粒度并行
```

**3. Lazy Task Creation**[^1]

```
传统 eager 创建：
  urls.map(|url| fetch(url))
  → 立即创建 10000 个任务

延迟创建：
  只在"需要值"时才创建任务
  → 内存占用 O(1) 或 O(并发数)
```

### 与三层运行时的关系

RFC-008 定义了 Embedded / Standard / Full 三层运行时架构。LLVM AOT 编译器与三层运行时的对应关系：

| 运行时 | LLVM AOT 行为 |
|--------|---------------|
| **Embedded** | 无 DAG 调度，直接生成顺序机器码 |
| **Standard** | DAG + 单线程调度（num_workers=1） |
| **Full** | DAG + 多线程调度（num_workers>1），支持 WorkStealing |

### 调度器接口设计

```rust
/// 调度策略
pub enum ScheduleStrategy {
    /// @block：强制串行，无 DAG
    Serial,
    /// @eager：急切求值，等待依赖完成
    Eager,
    /// @auto（默认）：延迟调度，DAG 自动调度
    Lazy,
}

/// 副作用标签
pub enum EffectTag {
    /// 纯函数，无副作用
    Pure,
    /// 有 I/O 副作用
    IO,
}

/// DAG 调度器 trait
pub trait DAGScheduler: Send + Sync {
    /// 调度执行（带策略参数）
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint], strategy: ScheduleStrategy) -> RuntimeValue;

    /// 单函数执行
    fn execute(&self, func: &CompiledFunction, args: &[RuntimeValue]) -> RuntimeValue;
}
```

## 权衡

### 优点

1. **性能提升**：AOT 编译比解释执行快 10-100x
2. **解决颜色函数**：默认并发，同步是特例
3. **统一运行时**：解释器和 LLVM 共享同一调度器
4. **延迟调度**：调用不立即执行，按需调度[^1][^2]
5. **隐式副作用**：用户无感知，编译器自动处理
6. **所有权安全**：依赖 Rust 风格的所有权模型，无数据竞争

### 缺点

1. **实现复杂度**：需要 LLVM 集成经验
2. **编译时间**：AOT 编译比解释器慢
3. **调试困难**：AOT 代码调试比解释器复杂

### 与 RFC 设计的一致性

| RFC | 一致性 |
|-----|--------|
| RFC-001 并作模型 | ✅ DAG 依赖分析是核心 |
| RFC-008 运行时架构 | ✅ 运行时调度器设计一致 |
| RFC-009 所有权模型 | ✅ ARC 运行时正确实现 |

## 替代方案

| 方案 | 描述 | 为什么不选 |
|------|------|-----------|
| 仅用解释器 | 不需要 AOT | 性能不足，颜色函数问题 |
| 纯静态编译 | 无运行时调度 | 延迟调度需要在运行时 |
| 链接外部 LLVM runtime | 使用 LLVM 的 runtime | 需要额外依赖 |

## 实现策略

### 阶段划分

#### 阶段 1：基础框架（1-2 天）

- [ ] 添加 inkwell 依赖到 `Cargo.toml`
- [ ] 创建 `src/backends/llvm/` 模块
- [ ] 实现 LLVM 上下文初始化

#### 阶段 2：类型映射（2-3 天）

- [ ] 实现 `TypeMap`：YaoXiang 类型 → LLVM 类型
- [ ] 基础类型：i32, i64, f32, f64, bool
- [ ] 复合类型：struct, array, tuple
- [ ] 特殊类型：Arc, ref, Option

#### 阶段 3：指令翻译（3-5 天）

- [ ] 实现 `codegen_instruction()`
- [ ] 算术指令：add, sub, mul, div
- [ ] 控制流：jmp, jmp_if, ret
- [ ] 函数调用：call, call_virt, call_dyn

#### 阶段 4：DAG 收集（2-3 天）

- [ ] 在代码生成时收集 DAG 信息
- [ ] 记录函数依赖关系
- [ ] 副作用推断（@IO / @Pure）
- [ ] 生成 DAG 元数据

#### 阶段 5：运行时库（3-5 天）

- [ ] 实现延迟调度
- [ ] 实现 DAG 调度器
- [ ] 实现粒度控制
- [ ] 实现 ARC 运行时

#### 阶段 6：集成与测试（2-3 天）

- [ ] 链接运行时库
- [ ] 端到端测试
- [ ] 性能基准

### 依赖关系

- RFC-001：并作模型（已接受）
- RFC-008：Runtime 并发模型（已接受）
- RFC-009：所有权模型（已接受）

### 风险

1. **LLVM 集成复杂度**：需要深入理解 inkwell API
2. **调度器与 AOT 代码集成**：需要精心设计接口
3. **ABI 兼容性**：需要确保与解释器运行时 ABI 兼容

## 相关工作

### Lazy Task Creation (1990)[^1]

| 属性 | 说明 |
|------|------|
| 机构 | MIT |
| 作者 | James R. Larus, Robert H. Halstead Jr. |
| 核心 | 延迟创建子任务，按需创建 |
| 参考价值 | 技术基础，延迟调度概念起源 |

**核心思想**：不是立即创建任务，而是延迟创建。当父任务需要子任务的值时，才创建子任务。这解决了细粒度并行任务的性能开销问题[^1]。

### Lazy Scheduling (2014)[^2]

| 属性 | 说明 |
|------|------|
| 机构 | University of Maryland |
| 作者 | Tzannes, Caragea |
| 核心 | 运行时自适应调度，无额外状态 |
| 参考价值 | 调度器设计，自适应粒度控制 |

**核心思想**：通过"延迟执行"自动控制粒度，不需要维护复杂状态。当系统忙时任务自动合并，闲时自动拆分[^2]。

### SISAL 语言[^3]

| 属性 | 说明 |
|------|------|
| 机构 | Lawrence Livermore National Laboratory (LLNL) |
| 核心 | 单赋值语言，Dataflow 图，隐式并行 |
| 参考价值 | 可行性证明，性能接近 Fortran |

**核心贡献**：SISAL 证明了 Dataflow 模型在工业级应用中可以达到接近 Fortran 的性能[^3]。

### Mul-T 并行 Scheme[^4]

| 属性 | 说明 |
|------|------|
| 机构 | MIT |
| 核心 | Future 构造，Lazy Task Creation 实现 |
| 参考价值 | 具体实现参考 |

**核心机制**：
```scheme
;; Multilisp / Mul-T 语法
(let ((a (future compute-a))      ;; 立即返回 future
      (b (future compute-b)))      ;; 立即返回 future
  (join a b))                      ;; 等待完成
```

### 对比总结

| 技术 | 延迟创建 | DAG 分析 | 副作用处理 | 所有权 |
|------|----------|----------|------------|--------|
| Lazy Task Creation[^1] | ✅ | ❌ | ❌ | N/A |
| Lazy Scheduling[^2] | ✅ | ❌ | ❌ | N/A |
| SISAL[^3] | ✅ | ✅ (全局) | N/A (单赋值) | N/A |
| Mul-T[^4] | ✅ | ❌ | ❌ | N/A |
| **YaoXiang** | ✅ | ✅ (函数内) | ✅ (隐式) | ✅ (ARC) |

**YaoXiang 的创新**：用现代语言特性（所有权 + 隐式副作用）简化传统设计，将 DAG 约束在函数块内降低复杂度。

## 与传统自动并行方法的对比

### 传统编译器：循环级并行化

商业编译器（如 Intel Fortran、Oracle Fortran）采用**循环级自动并行化**[^5]：

**核心流程**：
```
1. 识别可并行的循环
2. 对循环内的数组访问做依赖分析
3. 确定循环迭代之间是否有依赖
4. 如果没有依赖，生成多线程代码
```

**依赖分析技术**：

| 技术 | 说明 |
|------|------|
| **数据依赖** | 两个访问是否访问同一内存位置 |
| **Use-Def** | 变量的定义和使用关系 |
| **别名分析** | 指针是否指向同一内存 |

**循环可并行的条件**：
```fortran
! 可以并行
DO I = 1, N
  A(I) = C(I)
END DO

!  B(I) +不可并行（依赖前一个迭代）
DO I = 2, N
  A(I) = A(I-1) + B(I)
END DO
```

### Haskell：Spark 机制

GHC (Glasgow Haskell Compiler) 采用 **Spark 机制**实现纯函数并行[^6]：

```haskell
-- rpar: 并行执行，创建 spark
-- rseq: 串行执行，等待完成

example = do
  a <- rpar (f x)   -- 创建 spark，并行执行 f x
  b <- rpar (g y)   -- 创建 spark，并行执行 g y
  rseq a            -- 等待 a 完成
  rseq b            -- 等待 b 完成
  return (a, b)
```

**Spark 池机制**：
- 从池中取 spark 分配给空闲处理核
- 如果 spark 未被使用（无人等待结果），则被 GC 回收
- 这解决了粒度问题：太小的 spark 会被丢弃

### Clean 语言：唯一性类型

Clean 语言通过**唯一性类型（Uniqueness Types）**实现并行安全[^7]：

```clean
-- *Array 表示唯一性，可以安全修改
modify :: *Array Int -> *Array Int
```

**核心思想**：如果一个值是唯一引用的，可以安全地在并行环境中修改，因为没有其他引用会看到中间状态。

### 程序切片与依赖图

**程序依赖图 (PDG)** 是并行性检测的基础：

```
节点：语句
边：数据依赖 + 控制依赖

并行性检测：
  如果两个节点之间没有路径可达 → 可以并行
```

### 综合对比

| 方法 | 依赖分析 | 粒度 | 副作用处理 | 典型场景 |
|------|----------|------|------------|----------|
| Intel/Oracle Fortran[^5] | 复杂数组分析 | 循环迭代 | N/A | 科学计算 |
| GHC Spark[^6] | 纯函数假设 | 表达式 | N/A | 函数式编程 |
| Clean[^7] | 唯一性类型 | 图重写 | N/A | 函数式编程 |
| **YaoXiang** | 所有权保证 | 函数调用 | 隐式推断 | 通用 |

### 你的设计的独特优势

**相比传统方法，YaoXiang 的优势**：

```
1. 简化依赖分析
   传统：需要复杂指针/别名分析，保守策略（不确定就串行）
   YaoXiang：所有权保证安全，不需要复杂分析

2. 更粗粒度的并行
   传统：循环迭代级，需要精确分析
   YaoXiang：函数调用级，DAG 调度

3. 隐式副作用处理
   传统：需要手动标记或假设纯函数
   YaoXiang：编译器自动推断，用户无感知

4. 函数级作用域
   传统：全局依赖图，复杂度高
   YaoXiang：DAG 约束在函数块内，降低复杂度
```

### 开放问题

- [ ] DAG 元数据格式是否需要版本化？
- [ ] 是否支持增量 AOT 编译？
- [ ] 如何处理 FFI 调用？
- [ ] 性能基准测试计划？

---

## 附录

### 附录 A：与 Rust async 对比详解

| 特性 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| 编译产物 | 状态机 + 机器码 | 机器码 + DAG |
| 运行时 | tokio | DAG Scheduler |
| 调度时机 | 编译期确定 await 点 | 运行时按需调度 |
| 并发控制 | 状态机状态 | DAG 依赖边 |
| 颜色函数 | async 传染 | **L3 透明并发，@block 特例** |
| 注解 | async/await | @auto/@eager/@block |

### 附录 B：调度器优化示例

**场景 1：调度器检测到可以合并执行**

```
原始 DAG:
  compute_a() ──┐
  compute_b() ──┼──→ compute_c()

调度器优化后:
  合并 compute_a + compute_b 为单个任务
  → 减少调度开销
```

**场景 2：依赖未被使用**

```
let a = expensive_compute(); // 计算了
let b = other_thing();       // 不需要 a
print(b);                    // 直接返回 b，跳过 a
```

### 附录 C：设计讨论记录

| 决策 | 决定 | 日期 |
|------|------|------|
| 采用 LLVM AOT | 直接 Codegen，不过度抽象 | 2026-02-15 |
| DAG 作用域 | 函数块内，不跨函数 | 2026-02-15 |
| 延迟调度 | 跳过执行，挂起函数，按需调度 | 2026-02-15 |
| 副作用处理 | 隐式 Effect System，用户无感知 | 2026-02-15 |
| 粒度控制 | 并发数限制 + 自适应 | 2026-02-16 |
| 论文引用 | 添加 Lazy Task Creation 等 | 2026-02-16 |

---

## 参考文献

[^1]: Larus, J. R., & Halstead, R. H. (1990). *Lazy Task Creation: A Technique for Increasing the Granularity of Parallel Programs*. MIT. Retrieved from https://people.csail.mit.edu/riastradh/t/halstead90lazy-task.pdf

[^2]: Tzannes, A., & Caragea, G. (2014). *Lazy Scheduling: A Runtime Adaptive Scheduler for Declarative Parallelism*. University of Maryland. Retrieved from https://user.eng.umd.edu/~barua/tzannes-TOPLAS-2014.pdf

[^3]: Feo, J. T., et al. (1990). *A report on the SISAL language project*. Lawrence Livermore National Laboratory. Retrieved from https://www.sciencedirect.com/science/article/abs/pii/074373159090035N

[^4]: Mohr, E., et al. (1991). *Mul-T: A high-performance parallel lisp*. MIT. Retrieved from https://link.springer.com/content/pdf/10.1007/bfb0024163.pdf

[^5]: Intel Corporation. *Automatic Parallelization with Intel Compilers*. Retrieved from https://www.intel.com/content/www/us/en/developer/articles/technical/automatic-parallelization-with-intel-compilers.html

[^6]: Marlow, S. (2010). *Parallel and Concurrent Programming in Haskell*. Retrieved from https://www.cse.chalmers.se/edu/year/2015/course/pfp/Papers/strategies-tutorial-v2.pdf

[^7]: Plasmeijer, R., & van Eekelen, M. (2011). *Clean Language Documentation*. University of Nijmegen. Retrieved from https://clean.cs.ru.nl/Documentation

- [Rust async book](https://rust-lang.github.io/async-book/)
- [inkwell LLVM bindings](https://cranelift.dev/)
- [tokio 运行时设计](https://tokio.rs/)
- [RFC-001: 并作模型](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 并发模型](./accepted/008-runtime-concurrency-model.md)
- [RFC-009: 所有权模型](./accepted/009-ownership-model.md)
- [Implicit Parallelism - Wikipedia](https://en.wikipedia.org/wiki/Implicit_parallelism)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 作者草稿，等待提交审核 |
| **审核中** | `docs/design/rfc/` | 开放社区讨论和反馈 |
| **已接受** | `docs/design/accepted/` | 成为正式设计文档 |
| **已拒绝** | `docs/design/rfc/` | 保留在 RFC 目录 |
