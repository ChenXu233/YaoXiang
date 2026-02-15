---
title: RFC-018：LLVM AOT 编译器与运行时调度器集成设计
---

# RFC-018：LLVM AOT 编译器与运行时调度器集成设计

> **状态**: 草案
> **作者**: 晨煦
> **创建日期**: 2026-02-15
> **最后更新**: 2026-02-15

> **参考**:
> - [RFC-001: 并作模型与错误处理系统](./accepted/001-concurrent-model-error-handling.md)
> - [RFC-008: Runtime 并发模型与调度器脱耦设计](./accepted/008-runtime-concurrency-model.md)
> - [RFC-009: 所有权模型设计](./accepted/009-ownership-model.md)

## 摘要

本文档设计 YaoXiang 语言的 LLVM AOT 编译器，目标是通过预先编译生成机器码 + DAG 元数据，由运行时调度器根据 DAG 依赖关系调度执行。此设计与 Rust async/await + tokio 运行时模式类似，能够解决颜色函数问题：默认并发，同步才是特例。

## 动机

### 为什么需要 LLVM AOT 编译器？

当前 YaoXiang 仅有解释器作为执行后端，存在以下问题：

| 问题 | 影响 |
|------|------|
| 性能瓶颈 | 解释执行比机器码慢 10-100x |
| 部署复杂 | 需要携带解释器和运行时 |
| 颜色函数问题 | 同步函数不能调用并发函数 |

### 颜色函数问题

**传统设计（当前）**：
- 同步函数（蓝色）→ 不能调用 → 并发函数（红色）
- 同步是默认，并发需要 `spawn` 标记
- 颜色会"传染"：一旦用了并发，同一调用链上都是并发

**翻转后的设计（目标）**：
- 并发是默认，同步才是特例（使用 `@block`）
- 解决颜色函数问题：同步函数可以直接调用"默认并发"的代码

### 与 Rust async 的对比

```
┌─────────────────────────────────────────────────────────────────┐
│                      Rust async 模式                            │
├─────────────────────────────────────────────────────────────────┤
│  编译时：生成状态机 + 机器码                                    │
│  运行时：tokio 调度器根据状态机调度                            │
│  特点：await 点明确，状态机管理执行                             │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                      YaoXiang LLVM AOT 模式                    │
├─────────────────────────────────────────────────────────────────┤
│  编译时：生成机器码 + DAG 元数据                               │
│  运行时：DAG 调度器根据依赖关系调度                             │
│  特点：惰性求值，DAG 自动分析依赖                               │
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
│  │  • 根据 DAG 依赖关系调度执行 │ │
│  │  • 支持并行/串                行执行                         │ │
│  └─────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────┘
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
    /// 是否需要 spawn（并行执行）
    pub spawn: bool,
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
}

impl DefaultDAGScheduler {
    pub fn new(artifact: CompiledArtifact, num_workers: usize) -> Self {
        Self {
            thread_pool: ThreadPool::new(num_workers),
            artifact,
        }
    }
}

impl DAGScheduler for DefaultDAGScheduler {
    fn schedule(&self, dag: &DAGMetadata, entries: &[EntryPoint]) -> RuntimeValue {
        // 1. 拓扑排序确定执行顺序
        // 2. 并行执行无依赖节点
        // 3. 等待依赖完成
        // 4. 返回结果
    }
}
```

### 与 Executor Trait 集成

```rust
/// LLVM 代码生成器实现 Executor trait
pub struct LLVMCodegen<'ctx> {
    context: &'ctx LLVMContext,
    module: &'ctx mut LLVMModule<'ctx>,
    builder: LLVMIRBuilder<'ctx>,
    types: TypeMap,
    values: ValueMap,
    dag_collector: DAGCollector,
}

impl Executor for LLVMCodegen {
    fn execute_module(&mut self, module: &BytecodeModule) -> ExecutorResult<()> {
        // 1. 收集 DAG 信息
        self.dag_collector.collect(module)?;

        // 2. 生成 LLVM IR
        self.translate_module(module)?;

        // 3. 运行优化 passes
        self.run_passes()?;

        // 4. 编译到目标文件
        self.compile_to_object()?;

        // 5. 链接运行时库
        self.link_runtime()?;

        Ok(())
    }

    fn execute_function(&mut self, func: &BytecodeFunction, args: &[RuntimeValue]) -> ExecutorResult<RuntimeValue> {
        // JIT 模式：直接执行编译好的函数
        let compiled = self.get_compiled_function(&func.name)?;
        unsafe { compiled(args) }
    }
}
```

### 编译命令示例

```bash
# AOT 编译到可执行文件（包含运行时）
yaoxiangc --output program program.yx

# AOT 编译到动态库（需要链接运行时）
yaoxiangc --lib --output libprogram.so program.yx

# JIT 模式（解释器 + 运行时）
yaoxiangc --jit program.yx
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
├── runtime.rs       # 运行时库接口
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

| 场景 | 调度策略 |
|------|----------|
| 无依赖函数 | 并行执行 |
| 有依赖函数 | 等待依赖完成后执行 |
| `@block` 标记 | 强制串行执行 |
| 循环依赖 | 运行时检测并报错 |

## 权衡

### 优点

1. **性能提升**：AOT 编译比解释执行快 10-100x
2. **解决颜色函数**：默认并发，同步是特例
3. **统一运行时**：解释器和 LLVM 共享同一调度器
4. **类似 Rust**：开发者熟悉范式
5. **惰性求值**：DAG 自动分析依赖

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
| 纯静态编译 | 无运行时调度 | 惰性求值需要在运行时 |
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
- [ ] 生成 DAG 元数据

#### 阶段 5：运行时库（3-5 天）

- [ ] 实现惰性求值包装
- [ ] 实现 DAG 调度器
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

## 开放问题

- [ ] DAG 元数据格式是否需要版本化？（@待讨论）
- [ ] 是否支持增量 AOT 编译？（@待讨论）
- [ ] 如何处理 FFI 调用？（@待讨论）

---

## 附录

### 附录 A：与 Rust async 对比详解

| 特性 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| 编译产物 | 状态机 + 机器码 | 机器码 + DAG |
| 运行时 | tokio | DAG Scheduler |
| 惰性求值 | 需要 await 点 | 自动 DAG 分析 |
| 并发控制 | 状态机状态 | DAG 依赖边 |
| 颜色函数 | async 传染 | **默认并发，sync 特例** |

### 附录 B：设计讨论记录

| 决策 | 决定 | 日期 |
|------|------|------|
| 采用 LLVM AOT | 直接 Codegen，不过度抽象 | 2026-02-15 |
| DAG 元数据格式 | 节点+边简单格式 | 2026-02-15 |
| 运行时调度器 | 与解释器共享接口 | 2026-02-15 |

---

## 参考文献

- [Rust async book](https://rust-lang.github.io/async-book/)
- [inkwell LLVM bindings](https://cranelift.dev/)
- [tokio 运行时设计](https://tokio.rs/)
- [RFC-001: 并作模型](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: Runtime 并发模型](./accepted/008-runtime-concurrency-model.md)
- [RFC-009: 所有权模型](./accepted/009-ownership-model.md)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 作者草稿，等待提交审核 |
| **审核中** | `docs/design/rfc/` | 开放社区讨论和反馈 |
| **已接受** | `docs/design/accepted/` | 成为正式设计文档 |
| **已拒绝** | `docs/design/rfc/` | 保留在 RFC 目录 |
