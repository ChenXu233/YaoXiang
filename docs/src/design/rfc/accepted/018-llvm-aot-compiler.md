---
title: "RFC-018：LLVM AOT 编译器设计"
status: "已接受"
author: "晨煦"
created: "2026-02-15"
updated: "2026-07-05（同步 GitHub Issue #14、#134；添加实施状态分析）"
issue: "#14"
tracking_issue: "https://github.com/ChenXu233/YaoXiang/issues/134"
---

# RFC-018：LLVM AOT 编译器设计

> **参考**:
> - [RFC-024：基于 spawn 块的并发模型](../accepted/024-concurrency-model.md)
> - [RFC-008：Runtime 并发模型与调度器脱耦设计](../accepted/008-runtime-concurrency-model.md)
> - [RFC-009：所有权模型设计](../accepted/009-ownership-model.md)
> - [RFC-026：FFI 核心机制](./026-ffi-core-mechanism.md)
> - [RFC-010：统一类型语法](../accepted/010-unified-type-syntax.md)

> **废弃**:
> - 旧版"自底向上自动 DAG 分析"模型 — 已被 RFC-024 spawn 块直接子表达式模型取代
> - `@IO`/`@Pure` 隐式副作用推断 — 已被 RFC-024 资源类型机制取代
> - `Arc(T)` 类型映射 — 已被 RFC-009 v9 `ref` 关键字取代

## 摘要

本文档设计 YaoXiang 语言的 LLVM AOT（Ahead-of-Time）编译器。LLVM 后端与 VM 后端（解释器）共享同一套编译前端，构成[RFC-008](../accepted/008-runtime-concurrency-model.md) 定义的双后端架构：VM 用于开发调试，LLVM 用于生产发布。

**核心职责**：

```
源码 → 前端（共享）→ IR → LLVM Codegen → .o → 链接调度器静态库 → exe
```

编译器将 YaoXiang 源码编译为原生机器码，其中：

| 语言特性 | 编译策略 |
|----------|----------|
| 普通代码 | 顺序机器码，零调度开销 |
| `spawn { }` 块 | 直接子表达式 → 任务分发 + 同步等待（对齐 [RFC-024](../accepted/024-concurrency-model.md)） |
| `native("symbol")` | LLVM `declare external` + 参数 marshalling（对齐 [RFC-026](./026-ffi-core-mechanism.md)） |
| `.drop` 析构 | RAII cleanup 代码插入（对齐 [RFC-009](../accepted/009-ownership-model.md)） |
| `&T` / `&mut T` 令牌 | 零大小类型，编译后消失 |
| `ref T` 共享 | `{ refcount_ptr, data_ptr }` 胖指针，编译器自动选 Rc/Arc |

**与 RFC-024 的关系**：RFC-024 定义了 spawn 块的**用户语义**（直接子表达式创建任务、同步阻塞等待）。本文档定义这些语义**如何编译为机器码**。

**与 RFC-026 的关系**：RFC-026 定义了 FFI 的**用户语法**（`native()`、`[0]` 方法绑定、`.drop`）。本文档定义 FFI 调用**如何生成 LLVM IR**。

---

## 动机

### 为什么需要 LLVM AOT 编译器？

当前 YaoXiang 仅有解释器作为执行后端：

| 问题 | 影响 |
|------|------|
| 性能瓶颈 | 解释执行比机器码慢 10-100x |
| 部署复杂 | 需要携带解释器和运行时 |
| 生产环境 | 解释器不适合对性能敏感的场景 |

### 双后端模型中的 LLVM

[RFC-008](../accepted/008-runtime-concurrency-model.md) §6 定义了双后端架构：

```
                    ┌─────────────────────┐
                    │   编译前端（统一）     │
                    │   Lexer → Parser     │
                    │   → TypeCheck        │
                    │   → spawn 分析       │
                    │   → 逃逸分析          │
                    └──────────┬──────────┘
                               │
                  ┌────────────┴────────────┐
                  ▼                         ▼
      ┌───────────────────┐     ┌───────────────────┐
      │   VM 后端（开发）   │     │  LLVM 后端（生产）  │
      │   IR → 解释执行    │     │  IR → 原生代码      │
      │   步进调试         │     │  链接调度器静态库   │
      │   快速迭代         │     │  输出 .exe         │
      └───────────────────┘     └───────────────────┘
```

两个后端的**行为完全一致**——区别仅在执行方式。同一份源码、同一套类型检查、同一个 spawn 分析结果。

---

## 提案

### 1. 编译器架构

LLVM 后端位于编译流水线的最后阶段，从前端接收 IR，生成原生代码：

```
源码
  → Lexer / Parser（frontend/core/）
  → TypeCheck + spawn 分析（frontend/core/typecheck/）
  → IR 生成（middle/core/ir_gen.rs）
  → LLVM Codegen（backends/llvm/）
      ├── 类型映射：YaoXiang 类型 → LLVM IR 类型
      ├── 函数翻译：IR 指令 → LLVM IR 指令
      ├── spawn 展开：直接子表达式 → 任务函数 + 调度调用
      ├── FFI 展开：native() 调用 → declare + marshalling
      └── 析构插入：作用域结束 → .drop() 调用
  → LLVM 优化 + 目标代码生成
  → 链接运行时静态库 → 可执行文件
```

### 2. 编译流程

```
Phase 1: 前端（与 VM 后端共享）
  - 解析、类型检查、spawn 块分析、逃逸分析
  - 输出：类型标注的 IR

Phase 2: LLVM IR 生成
  - 类型映射、函数声明、指令翻译
  - 输出：LLVM Module

Phase 3: LLVM 优化
  - 标准 LLVM 优化 pipeline（O0/O1/O2/O3）
  - 内联、常量折叠、死代码消除

Phase 4: 目标代码生成
  - LLVM TargetMachine → .o 文件
  - 平台：Linux (ELF)、macOS (Mach-O)、Windows (COFF)

Phase 5: 链接
  - 链接运行时静态库（调度器、分配器）
  - 输出：可执行文件
```

### 3. 类型映射

#### 3.1 YaoXiang → LLVM IR 类型映射

| YaoXiang 类型 | LLVM IR 类型 | 说明 |
|---------------|-------------|------|
| `Int` | `i64` | 默认 64 位有符号整数 |
| `Int32` | `i32` | 显式 32 位整数（主要用于 FFI） |
| `Float` | `f64` | 默认 64 位浮点 |
| `Float32` | `f32` | 显式 32 位浮点（主要用于 FFI） |
| `Bool` | `i1` | 布尔值 |
| `Char` | `i32` | Unicode 码点 |
| `String` | `{ i8*, i64 }` | 指针 + 字节长度 |
| `Void` | `{}` | 零大小空类型 |
| `&T` | — | 零大小令牌，编译后消失，不产生任何 IR |
| `&mut T` | — | 零大小令牌，编译后消失，不产生任何 IR |
| `ref T` | `{ i64*, T* }` | 胖指针（引用计数指针 + 数据指针） |
| `*T` | `T*` | 裸指针 |
| `[T; N]` | `[N x T]` | 定长数组 |
| `List(T)` | `{ T*, i64, i64 }` | 数据指针 + 长度 + 容量 |
| 结构体 | 对应 LLVM struct | 字段按定义顺序布局 |
| 记录枚举 | `{ i64, [max_payload_size] }` | 标签 + 最大 payload 的 union |
| `?T` | `{ i1, T }` | 有值标记 + 数据（通用表示） |
| FFI 不透明类型 | `{ i8* }` | 包装 C 指针 |
| 函数指针 | `T (...)*` | 函数指针类型 |

> **`&T` / `&mut T` 零运行时开销**：[RFC-009](../accepted/009-ownership-model.md) §2.7 定义编译器内部为令牌分配品牌标识（编译期唯一整数），单态化和内联后品牌完全消失——生成的机器码中不存在任何令牌痕迹。

#### 3.2 FFI 参数类型映射

对齐 [RFC-026](./026-ffi-core-mechanism.md) §2.2，补充 LLVM IR 一列：

| C 类型 | YaoXiang 类型 | LLVM IR | 说明 |
|--------|---------------|---------|------|
| `int` | `Int32` | `i32` | |
| `long` | `Int64` | `i64` | |
| `float` | `Float32` | `f32` | |
| `double` | `Float64` | `f64` | |
| `char` | `Char` | `i32` | C char → YaoXiang Char（Unicode 兼容） |
| `char*` | `String` | `{ i8*, i64 }` | marshalling：C string → YaoXiang String |
| `bool` | `Bool` | `i1` | |
| `size_t` | `Uint` | `i64` | |
| `void*` | `*Void` | `i8*` | |
| `struct T*` | `T`（透明类型） | `T*` | 传递指针 |
| `typedef struct T T` | `T`（不透明类型） | `{ i8* }` | 包装 C 指针 |

### 4. IR 规范化与指令翻译

#### 4.0 IR 规范化（栈 → 寄存器）

当前 IR（`src/middle/core/ir.rs`）包含栈操作指令（`Push`/`Pop`/`Dup`/`Swap`），这是为字节码 VM 设计的。LLVM IR 是 SSA 形式，不接受栈操作。

**处理策略**：LLVM 路径在指令翻译之前，先经过一个轻量规范化 pass：

| 栈指令 | 规范化策略 |
|--------|-----------|
| `Push(r)` | 记录 `stack.push(r)`，不产生 IR |
| `Pop(r)` | `r = stack.pop()`，产生 `load`（从栈槽位） |
| `Dup` | `stack.push(stack.top())`，不产生 IR |
| `Swap` | 交换栈顶两个元素，不产生 IR |

规范化后，所有操作数变为寄存器/局部变量引用，栈操作全部消除。该 pass 作为 `translator.rs` 的第一步执行。

> **为什么不在 IR 层面消除栈指令？** 因为 VM 后端需要栈语义。在 LLVM 翻译入口处规范化，保持了 IR 对两个后端的共享——每个后端按自己的需求消费同一个 IR。
>
> **前提**：IR 生成阶段保证栈平衡——所有控制流路径到达同一程序点时栈深度一致（VM 字节码后端依赖同一前提，否则字节码执行会出错）。规范化 pass 不检查此前提；违反时 LLVM 后端产生未定义行为。

#### 4.1 指令翻译表

以下逐条列出 `Instruction` 枚举中每个变体的 LLVM IR 翻译策略。指令名与 `src/middle/core/ir.rs` 完全一致。

**算术指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `Add { dst, lhs, rhs }` | `add`（整数）/ `fadd`（浮点） | 按类型选择整数或浮点加法 |
| `Sub { dst, lhs, rhs }` | `sub` / `fsub` | |
| `Mul { dst, lhs, rhs }` | `mul` / `fmul` | |
| `Div { dst, lhs, rhs }` | `sdiv` / `udiv` / `fdiv` | 有符号/无符号/浮点除法 |
| `Mod { dst, lhs, rhs }` | `srem` / `urem` | 有符号/无符号取模 |
| `Neg { dst, src }` | `sub 0, src`（整数）/ `fneg`（浮点） | |

**位运算指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `And { dst, lhs, rhs }` | `and` | |
| `Or { dst, lhs, rhs }` | `or` | |
| `Xor { dst, lhs, rhs }` | `xor` | |
| `Shl { dst, lhs, rhs }` | `shl` | 左移 |
| `Shr { dst, lhs, rhs }` | `lshr` | 逻辑右移 |
| `Sar { dst, lhs, rhs }` | `ashr` | 算术右移 |

**比较指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `Eq { dst, lhs, rhs }` | `icmp eq` / `fcmp oeq` | |
| `Ne { dst, lhs, rhs }` | `icmp ne` / `fcmp one` | |
| `Lt { dst, lhs, rhs }` | `icmp slt` / `fcmp olt` | |
| `Le { dst, lhs, rhs }` | `icmp sle` / `fcmp ole` | |
| `Gt { dst, lhs, rhs }` | `icmp sgt` / `fcmp ogt` | |
| `Ge { dst, lhs, rhs }` | `icmp sge` / `fcmp oge` | |

**控制流指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `Jmp(label)` | `br label %L` | 无条件跳转 |
| `JmpIf(cond, label)` | `br i1 %cond, label %L, label %fallthrough` | 条件跳转 |
| `JmpIfNot(cond, label)` | `br i1 %cond, label %fallthrough, label %L` | 条件不跳转 |
| `Ret(Some(v))` | `ret T %v` | 有返回值 |
| `Ret(None)` | `ret void` | 无返回值 |

**调用指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `Call { dst, func, args }` | `%r = call T @func(...)` | 静态调用 |
| `CallVirt { dst, obj, method_name, args }` | vtable GEP + `call`（函数指针） | 虚方法调用，通过 vtable 查找 |
| `CallDyn { dst, func, args }` | `%r = call T %func(...)` | 动态调用（闭包/函数指针） |
| `TailCall { func, args }` | `musttail call` / `tail call` | 尾调用优化 |

**内存指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `Move { dst, src }` | — | 规范化后变为寄存器复制，SSA 构造可消除大部分 |
| `Load { dst, src }` | `%v = load T, T* %src` | |
| `Store { dst, src }` | `store T %src, T* %dst` | |
| `Alloc { dst, size }` | `%p = alloca T`（栈）/ `call @malloc`（逃逸到堆） | 逃逸分析决定分配位置 |
| `Free(ptr)` | `call @free(%ptr)`（堆）/ —（栈，自动回收） | |
| `AllocArray { dst, size, elem_size }` | `%p = alloca [N x T]`（栈）/ `call @malloc`（堆） | |

**结构体/数组访问指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `LoadField { dst, src, field }` | `%ptr = getelementptr T, T* %src, 0, field` + `load` | |
| `StoreField { dst, field, src }` | `%ptr = getelementptr T, T* %dst, 0, field` + `store` | |
| `LoadIndex { dst, src, index }` | `%ptr = getelementptr T, T* %src, 0, %index` + `load` | |
| `StoreIndex { dst, index, src }` | `%ptr = getelementptr T, T* %dst, 0, %index` + `store` | |
| `CreateStruct { dst, type_name, fields }` | `insertvalue` 链 | 按字段顺序构造 LLVM struct |

**类型转换指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `Cast { dst, src, target_type }` | `bitcast` / `trunc` / `zext` / `sext` / `fptrunc` / `fpext` / `sitofp` / `fptosi` / `inttoptr` / `ptrtoint` | 按源/目标类型组合选择合适的 cast 指令 |
| `TypeTest(val, type)` | — | 编译期类型测试，生成 `icmp eq` 比较类型标签 |

**所有权与借用指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `Borrow { dst, src, mutable }` | — | **零大小令牌，编译后完全消失**，不产生任何 IR |
| `Release(val)` | — | **零大小令牌，编译后完全消失** |
| `Move { dst, src }` | — | 所有权转移，规范化后变为寄存器复制 |
| `Drop(val)` | `call void @T.drop(T* %val)` | 调用类型的析构函数（见 §7） |
| `ShareRef { dst, src }` | `call %T* @Arc_new(%src)` / `call %T* @Rc_new(%src)` | 编译器根据跨线程与否自动选 Arc/Rc |
| `ArcNew { dst, src }` | `call %T* @Arc_new(%src)` | 原子引用计数 = 1 |
| `ArcClone { dst, src }` | `call %T* @Arc_clone(%src)` | 原子递增引用计数 |
| `ArcDrop(val)` | `call void @Arc_drop(%val)` | 原子递减 + 条件释放 |

**并发指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `Spawn { closures, plan, result }` | 展开为调度器调用序列 | 详见 §5，运行时 `task_spawn` + `task_wait_all` |
| `Yield` | — | AOT 路径上 spawn 块同步等待，不需要 yield；忽略 |

**unsafe 块与裸指针指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `UnsafeBlockStart` | — | **编译期标记，不产生 IR** |
| `UnsafeBlockEnd` | — | **编译期标记，不产生 IR** |
| `PtrFromRef { dst, src }` | `%p = ptrtoint T* %src to i64`（或直接复制指针） | |
| `PtrDeref { dst, src }` | `%v = load T, T* %src` | |
| `PtrStore { dst, src }` | `store T %src, T* %dst` | |
| `PtrLoad { dst, src }` | `%v = load T, T* %src` | |

**字符串指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `StringLength { dst, src }` | `%len = extractvalue { i8*, i64 } %src, 1` | String 是 `{ ptr, len }`，长度在字段 1 |
| `StringConcat { dst, lhs, rhs }` | `call String @yx_string_concat(%lhs, %rhs)` | 运行时辅助函数 |
| `StringGetChar { dst, src, index }` | `getelementptr` + `load i32` | 含边界检查 |
| `StringFromInt { dst, src }` | `call String @yx_string_from_int(%src)` | 运行时辅助函数 |
| `StringFromFloat { dst, src }` | `call String @yx_string_from_f64(%src)` | 运行时辅助函数 |

**闭包指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `MakeClosure { dst, func: String, env }` | 分配闭包结构体 + 填充函数指针（按函数名查找）和环境 | `{ fn_ptr, env_fields... }` |
| `LoadUpvalue { dst, upvalue_idx }` | `%v = extractvalue %env, upvalue_idx` | 从闭包环境读 upvalue |
| `StoreUpvalue { src, upvalue_idx }` | `%env = insertvalue %env, %src, upvalue_idx` | 写入闭包环境 |
| `CloseUpvalue(val)` | 将栈上 upvalue 复制到堆 | |

**其他指令**：

| IR 指令 | LLVM IR | 说明 |
|---------|---------|------|
| `HeapAlloc { dst, type_id }` | `call i8* @malloc(i64 size)` + 类型标签写入 | 堆分配 + 类型信息 |
| `NewDict { dst, keys, values }` | `call Dict @yx_dict_new(%keys, %values)` | 运行时辅助函数 |

> **注意**：`Push`/`Pop`/`Dup`/`Swap` 已在 §4.0 规范化阶段消除，不出现在翻译表中。`Borrow`/`Release` 是零大小编译期令牌，不产生任何机器码。

### 5. spawn 块代码生成

对齐 [RFC-024](../accepted/024-concurrency-model.md)，spawn 块的编译分为以下步骤。

#### 5.1 语义回顾

```yaoxiang
(r1, r2) = spawn {
    t1 = fetch("url1"),   // 直接子表达式 → 任务 1
    t2 = fetch("url2"),   // 直接子表达式 → 任务 2
    return (t1, t2)       // 同步等待，组装结果
}
```

**规则**（RFC-024 §2.1）：
- spawn 块的**直接子表达式**（顶层逗号分隔的语句）创建并行任务
- 嵌套 `{}` 内的表达式不算直接子表达式，不成为独立任务
- 整个 spawn 块同步阻塞，等待所有任务完成后返回

#### 5.2 编译步骤

```
Step 1: 识别直接子表达式
  遍历 spawn 块体，收集顶层语句

Step 2: 依赖分析
  对每个直接子表达式，分析它引用了哪些由前面任务产生的变量
  无依赖 → 可立即并行调度
  有依赖 → 排队等待依赖任务完成

Step 3: 资源冲突检测（RFC-024 §2.5）
  检查同一资源类型实例是否被多个任务使用
  同实例冲突 → 标记串行执行顺序

Step 4: 生成任务函数
  每个直接子表达式生成一个独立的 LLVM 函数（闭包）

Step 5: 生成调度代码
  调用运行时 scheduler 的 task_spawn / task_wait

Step 6: 结果组装
  收集所有任务输出，拼装 return 元组
```

#### 5.3 LLVM IR 生成模式

```llvm
; spawn 块入口
%task_count = 2
%tasks = alloca [2 x %TaskHandle]

; 创建任务 1：fetch("url1")
%task1_fn = @spawn_closure_1
call @runtime_task_spawn(%tasks[0], %task1_fn, ...)

; 创建任务 2：fetch("url2")
%task2_fn = @spawn_closure_2
call @runtime_task_spawn(%tasks[1], %task2_fn, ...)

; 同步等待所有任务
call @runtime_task_wait_all(%tasks, %task_count)

; 组装返回值
%r1 = call @runtime_task_result(%tasks[0])
%r2 = call @runtime_task_result(%tasks[1])
ret { %r1, %r2 }
```

#### 5.4 依赖任务

```yaoxiang
result = spawn {
    data = fetch("url"),       // 任务 1：无依赖
    processed = parse(data),   // 任务 2：依赖任务 1 的 data
    return processed
}
```

编译器检测到 `parse(data)` 引用了任务 1 产生的 `data`，在生成调度代码时标记依赖：

```llvm
; 任务 2 带着对任务 1 的依赖创建
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
;                                                              ↑
;                                                 依赖任务 0（fetch）完成
```

#### 5.5 资源类型自动串行

[RFC-024 §2.5](../accepted/024-concurrency-model.md) 定义的资源类型（`FilePath`、`HttpUrl`、`DBUrl`、`Console` 及用户自定义资源类型）在 spawn 块中自动串行：

```yaoxiang
(a, b) = spawn {
    r1 = db.exec("SELECT ..."),   // 使用 SqliteDb（资源类型）
    r2 = db.exec("INSERT ...")    // 同一实例 → 自动串行
}
```

编译器检测到同一资源实例被两个任务使用，生成串行依赖：

```llvm
; 任务 2 依赖任务 1（同资源自动串行）
call @runtime_task_spawn_with_dep(%tasks[1], %task2_fn, %tasks[0])
```

#### 5.6 spawn for 数据并行

```yaoxiang
results = spawn for item in items {
    process(item)
}
```

编译器展开为 N 个独立任务（N = items 长度），受最大并发数限制。

### 6. FFI 代码生成

> ⚠️ **依赖说明**：本节定义的 FFI 代码生成**架构**（`native("x")` → `declare external @x` → marshalling 包装函数 → call）是稳定的，不随 RFC-026 语法变更而变。具体的参数 marshalling 规则表（§6.2）和不透明类型布局（§6.3）引用 RFC-026 的定义——若 RFC-026 的 `native()` 语法或 marshalling 规则发生变更，只需更新本文档中对应的映射表，架构层不受影响。RFC-026 当前状态：**审核中**，与本文档同在 `review/` 目录。
>
> **接受前置条件**：本 RFC 接受前，RFC-026 中与本文档 §6 相关的部分（`native()` 声明语法、参数 marshalling 规则、不透明类型 `{ i8* }` 布局、`.drop` 绑定约定）应先冻结或随 026 一同接受。否则 §6.2/§6.3/§7 的映射表可能在实现前就过时。

对齐 [RFC-026](./026-ffi-core-mechanism.md)，本节定义 FFI 调用的 LLVM IR 生成策略。

#### 6.1 native() 函数声明

```yaoxiang
sqlite3_open: (filename: String) -> SqliteDb = native("sqlite3_open")
```

编译为 LLVM IR：

```llvm
; 声明外部 C 函数
declare i8* @sqlite3_open(i8*)

; YaoXiang 包装函数（处理 marshalling）
define { i8* } @__yx_sqlite3_open({ i8*, i64 } %filename) {
    ; marshalling: YaoXiang String → C string
    %c_str = extractvalue { i8*, i64 } %filename, 0
    ; 调用 C 函数
    %raw = call i8* @sqlite3_open(i8* %c_str)
    ; unmarshalling: C 指针 → 不透明类型
    %result = insertvalue { i8* } undef, i8* %raw, 0
    ret { i8* } %result
}
```

**关键点**：
- `native("sqlite3_open")` → `declare external @sqlite3_open`
- 编译器自动生成 marshalling 包装函数
- 包装函数的签名使用 YaoXiang 类型，内部转换到 C 类型

#### 6.2 参数 Marshalling

| 方向 | 转换 |
|------|------|
| YaoXiang `String` → C `char*` | 提取 `.ptr` 字段传递 |
| YaoXiang `Int32` → C `int` | 直接传递（`i32`） |
| YaoXiang `*Void` → C `void*` | 直接传递（`i8*`） |
| YaoXiang `T`（透明类型） → C `struct T*` | 取地址传递 |
| YaoXiang `T`（不透明类型） → C `struct T*` | 提取 `{ i8* }` 中的指针传递 |

#### 6.3 不透明类型的 LLVM 布局

[RFC-026](./026-ffi-core-mechanism.md) §4.1 定义的不透明类型：

```yaoxiang
SqliteDb = unsafe {
    SqliteDb: Type = {
        handle: *Void
    }
    return SqliteDb
}
```

LLVM 布局：`{ i8* }` — 一个包含 C 指针的结构体。

**布局优化**：当不透明类型只有一个 `handle: *Void` 字段时，可优化为直接使用 `i8*`（省略外层 struct）。优化后的 ABI 与 C 指针完全一致，零 marshalling 开销。编译器默认启用此优化，用户无感知。

#### 6.4 ?T 可空返回值的 LLVM 表示

[RFC-026](./026-ffi-core-mechanism.md) §7.6 定义的 FFI 可空返回值：

```yaoxiang
sqlite3_open: (filename: String) -> ?SqliteDb = native("sqlite3_open")
```

通用 LLVM 表示：`{ i1, { i8* } }` — 有值标记 + 数据。

**针对 FFI null 指针的优化**：如果 `?T` 的 `T` 是不透明类型（内部为指针），编译器使用 **null 指针 = None** 优化：

```llvm
; 优化后的 LLVM 表示：直接使用可为 null 的指针
define i8* @__yx_sqlite3_open(...) {
    %raw = call i8* @sqlite3_open(...)
    ; null → None，非 null → Some(包装为不透明类型)
    ret i8* %raw
}
```

调用方：
```llvm
%raw = call i8* @__yx_sqlite3_open(...)
%is_null = icmp eq i8* %raw, null
br i1 %is_null, label %none_branch, label %some_branch
```

此优化使得 `?SqliteDb` 的 FFI 调用**零额外开销**——与 C 的 null 检查完全等价。

#### 6.5 yx-bindgen 集成

[yx-bindgen](./026-ffi-core-mechanism.md) §6 自动生成的绑定文件在编译时被当作普通 YaoXiang 源码处理。编译器不需要知道代码来自 bindgen——`native()` 声明和 `unsafe {}` 类型定义的处理方式完全一致。

### 7. 析构函数代码生成

对齐 [RFC-009](../accepted/009-ownership-model.md) 的 RAII 语义和 [RFC-026](./026-ffi-core-mechanism.md) §7 的 `.drop` 约定。

#### 7.1 .drop 绑定识别

```yaoxiang
SqliteDb.drop = sqlite3_close[0]
```

编译器识别 `.drop` 绑定，在类型元数据中标记析构函数指针。

#### 7.2 作用域结束时 Cleanup 插入

```
用户代码：
{
    db = SqliteDb.open("test.db")
    stmt = db.prepare("SELECT ...")
    stmt.step()
    // ← 作用域结束
}

编译器插入的 cleanup（逆序）：
    call @sqlite3_finalize(%stmt)    // stmt.drop()
    call @sqlite3_close(%db)          // db.drop()
```

**插入位置**：
- 正常作用域结束（`}`）
- 提前返回（`return` 前）
- `?` 错误传播路径（`?` 前）
- spawn 块结束（任务内变量的析构）

#### 7.3 Move 与析构

```yaoxiang
db = SqliteDb.open("test.db")
db2 = db                // Move：所有权转移给 db2
// db 已失效，此处不为 db 插入 drop
// ← 作用域结束：只为 db2 插入 drop
```

编译器追踪 Move 语义（[RFC-009](../accepted/009-ownership-model.md) §1），只在变量的最终持有者处插入析构调用。

#### 7.4 析构失败处理

```llvm
; debug 模式：检查析构返回值
%ret = call i32 @sqlite3_close(i8* %handle)
%ok = icmp eq i32 %ret, 0
br i1 %ok, label %done, label %panic
panic:
  call @__yx_panic("destructor failed")
  unreachable
done:
  ret void

; release 模式：忽略返回值
call i32 @sqlite3_close(i8* %handle)
ret void
```

### 8. 编译产物结构

编译产物包含以下组成部分（具体 struct 定义在实现阶段确定）：

- **机器码**：LLVM 编译的目标文件（`.o`），包含所有函数翻译结果
- **spawn 元数据**：每个 spawn 块的任务函数指针、依赖关系、资源冲突串行化对
- **FFI 符号表**：外部 C 符号引用（符号名 + 是否弱引用）
- **入口点表**：可执行文件的入口函数列表
- **类型信息**：反射元数据，写入 `.reflect` 段，运行时按需 mmap

### 9. 运行时库

对齐 [RFC-008 §6.2](../accepted/008-runtime-concurrency-model.md)，运行时以**静态库**形式链接进最终 exe。

```
最终 exe 内部结构：

┌────────────────────────────────────────────┐
│  用户代码（原生机器码）                       │
│  ├── 普通函数（顺序执行）                    │
│  ├── spawn 块展开（任务函数 + 调度调用）     │
│  ├── FFI marshalling 包装函数               │
│  └── RAII 析构代码                          │
├────────────────────────────────────────────┤
│  运行时静态库（约 500KB-1MB，取决于平台和功能选择）  │
│  ├── 线程池（num_workers）                  │
│  ├── 事件循环（libuv / io_uring）           │
│  ├── 工作窃取队列（仅 Full Runtime）         │
│  ├── 内存分配器（jemalloc / mimalloc）      │
│  └── 反射元数据（.reflect 段，按需 mmap）    │
│                                              │
│  没有：                                      │
│  ❌ 字节码解释器                             │
│  ❌ JIT 编译器                               │
│  ❌ GC                                      │
│  ❌ 虚拟机                                    │
└────────────────────────────────────────────┘
```

**关键设计**：编译期完成 spawn 块的任务识别和依赖分析，运行时只做"创建任务 → 分发到线程池 → 等待完成"——数据结构固定，行为可预测。

> **与 RFC-008 大小估计的差异**：RFC-008 §4 估计调度器约 200-500KB，仅含任务调度核心。本文档的 500KB-1MB 估计额外包含内存分配器（jemalloc/mimalloc）、事件循环（libuv/io_uring）和反射元数据段。实际大小取决于平台和功能选择，实现阶段会给出精确数字。

**三层运行时与 LLVM 的关系**（对齐 RFC-008 §1）：

| 运行时 | LLVM AOT 行为 |
|--------|---------------|
| **Embedded** | 无 spawn 支持，直接生成顺序机器码 |
| **Standard** | 支持 spawn 块，spawn 块内 DAG + 单线程调度（num_workers=1） |
| **Full** | 支持 spawn 块，spawn 块内 DAG + 多线程调度（num_workers>1），支持 WorkStealing |

---

## 详细设计

### 模块目录结构

对齐 [RFC-008](../accepted/008-runtime-concurrency-model.md) §6 的目录布局。`[! 规划中]` 标记表示该文件/目录尚未创建，由本 RFC 的实现阶段引入。

```
src/
├── frontend/                          # 编译前端（所有后端共享）
│   ├── core/
│   │   ├── spawn/                     # spawn 模块（VM 和 LLVM 后端共享的并发分析）
│   │   │   ├── mod.rs                 # spawn 模块入口
│   │   │   ├── placement.rs           # spawn 出现位置合法性检查
│   │   │   └── analysis.rs            # [! 规划中] 任务识别、依赖分析、资源冲突检测
│   │   └── typecheck/
│   │       └── ...
│
├── middle/
│   ├── core/
│   │   ├── ir.rs                      # IR 定义（VM 和 LLVM 共用）
│   │   └── ir_gen.rs                  # IR 生成
│   └── passes/
│       ├── codegen/
│       │   ├── mod.rs                 # 编排层（当前输出 BytecodeFile）
│       │   ├── translator.rs          # IR → 字节码翻译（VM 后端用）
│       │   ├── emitter.rs             # 字节码发射 + 跳转回填（VM 后端用）
│       │   ├── buffer.rs              # 常量池 + 字节码缓冲区（VM 后端用）
│       │   ├── bytecode.rs            # 字节码格式定义 + 序列化（VM 后端用）
│       │   ├── flow.rs                # 寄存器分配 + 标签生成 + 符号表（VM 后端用）
│       │   └── operand.rs             # 操作数解析（VM 后端用）
│       ├── lifetime/                  # 生命周期/令牌活性分析
│       └── mono/                      # 单态化
│
├── backends/
│   ├── common/                        # 共享值/堆/操作码
│   ├── interpreter/                   # 树遍历解释器（VM 后端）
│   ├── llvm/                          # [! 规划中] LLVM 后端代码生成（见下方文件列表）
│   │   ├── mod.rs                     # [! 规划中] LLVM 后端入口
│   │   ├── context.rs                 # [! 规划中] LLVM 上下文管理
│   │   ├── types.rs                   # [! 规划中] 类型映射（YaoXiang → LLVM IR）
│   │   ├── values.rs                  # [! 规划中] 值映射
│   │   ├── func.rs                    # [! 规划中] 函数翻译
│   │   ├── spawn.rs                   # [! 规划中] spawn 块展开
│   │   ├── ffi.rs                     # [! 规划中] FFI 调用代码生成
│   │   └── drop.rs                    # [! 规划中] 析构函数插入
│   └── runtime/                       # 编译型运行时（静态库链接进 exe）
│       ├── engine.rs                  # 任务调度引擎
│       ├── facade.rs                  # 对外接口
│       └── task.rs                    # 任务表示
│
└── util/
    └── diagnostic/                    # 错误诊断（共享）
```

> **关键变更**：spawn 块分析（任务识别、依赖分析、资源冲突检测）将在 `frontend/core/spawn/`（前端共享）中实现。现有的 `frontend/core/typecheck/passes/spawn_placement.rs`（spawn 出现位置检查）将迁移至 `frontend/core/spawn/placement.rs`，详见 RFC-024。LLVM 后端只消费分析结果，生成对应的调度代码。
>
> **现状说明**：当前 `middle/passes/codegen/` 下的 `buffer.rs`、`emitter.rs`、`bytecode.rs`、`flow.rs`、`operand.rs` 服务于 VM 后端的字节码生成（`CodegenContext::generate()` → `BytecodeFile`）。LLVM 后端将在 `backends/llvm/` 中实现，与 interpreter 后端和 runtime 平级——两者共享同一个 `ModuleIR` 输入，输出不同的目标格式（字节码 vs 原生代码）。

### 平台 ABI 支持

| 平台 | 目标三元组 | 输出格式 | 调用约定（FFI 默认） |
|------|-----------|----------|---------------------|
| Linux x86_64 | `x86_64-unknown-linux-gnu` | ELF | System V AMD64 |
| macOS x86_64 | `x86_64-apple-darwin` | Mach-O | System V AMD64 |
| macOS ARM64 | `aarch64-apple-darwin` | Mach-O | ARM64 AAPCS |
| Windows x86_64 | `x86_64-pc-windows-msvc` | COFF | Microsoft x64 |

FFI 调用默认使用平台的 C 调用约定。用户可通过 `native("symbol", cc = "stdcall")` 等选项覆盖（对齐 [RFC-026](./026-ffi-core-mechanism.md) 的未来扩展）。

### 浮点语义一致性（VM ↔ LLVM）

双后端架构的核心承诺是 VM（开发调试）和 LLVM（生产发布）行为一致。浮点运算在两种执行模式下存在潜在的不一致点：

| 场景 | 风险 | 策略 |
|------|------|------|
| NaN 传播 | VM 和 LLVM 可能对 NaN 的符号位和 payload 处理不同 | 编译器在 IR 层面规范化 NaN 表示，NaN 比较统一使用 `fcmp uno` |
| 舍入模式 | LLVM 默认 round-to-nearest-even，VM 取决于宿主 CPU | 不暴露非默认舍入模式，VM 和 LLVM 统一使用 RTNE |
| 除零 | IEEE 754 定义 ±Inf，但某些平台可能 trap | debug 模式检查除零并报告诊断；release 模式遵循 IEEE 754 |
| `-0.0` vs `+0.0` | 比较操作可能不等价 | 统一使用 IEEE 754 规则：`+0.0 == -0.0` |
| 非规格化数 | 某些平台 flush-to-zero | LLVM 不启用 `denormal-fp-math` 属性，保留完整 IEEE 754 语义 |

> **测试策略**：实现一套跨后端的浮点一致性测试套件——相同的 YaoXiang 源码分别在 VM 和 LLVM 后端执行，逐值比对输出。这组测试是 CI 的强制门禁。

---

## 权衡

### 优点

1. **性能**：AOT 编译比解释执行快 10-100x
2. **统一前端**：VM 和 LLVM 共享同一套前端，行为完全一致
3. **零调度开销**：普通代码直接生成顺序机器码，spawn 块外无 DAG 开销
4. **静态链接**：没有外部运行时依赖，单个 exe 即可部署
5. **零 GC**：RAII 确定性析构，无暂停
6. **FFI 零开销**：`?T` null 指针优化、不透明类型布局优化，FFI 调用成本等同于 C
7. **编译期分析**：spawn 块任务识别和依赖分析在编译期完成，运行时只执行

### 缺点

1. **LLVM 集成复杂度**：需要深入理解 inkwell API 和 LLVM IR
2. **编译时间**：AOT 编译比解释器慢（一次性的代价）
3. **调试体验**：原生代码调试需要 DWARF/PDB 符号支持（编译器需生成调试信息）
4. **增量编译**：大型项目的增量编译需要额外设计
5. **浮点语义一致性**：VM 和 LLVM 在 NaN 传播、舍入模式、除零等边界行为上可能存在差异，需通过规范化策略保证双后端行为一致（见 §10）

### 与相关 RFC 的一致性

| RFC | 一致性 |
|-----|--------|
| RFC-024 spawn 块并发模型 | ✅ spawn 块直接子表达式 → 任务分发 |
| RFC-008 运行时架构 | ✅ 双后端 + 调度器静态库 + 模块目录结构 |
| RFC-009 所有权模型 v9 | ✅ `&T`/`&mut T` 令牌（零大小）、`ref T`（胖指针）、`?T`（Option） |
| RFC-026 FFI 核心机制 | ✅ `native()` → declare + marshalling、`.drop` → RAII cleanup |

---

## 替代方案

| 方案 | 描述 | 为什么不选 |
|------|------|-----------|
| 仅用解释器 | 不需要 AOT | 性能不足 |
| 纯静态编译（无运行时） | 不链接调度器 | spawn 块需要运行时任务调度 |
| Cranelift 后端 | 更快的编译速度 | 运行时性能不如 LLVM，作为未来可选后端 |
| 链接外部 LLVM runtime | 使用 LLVM 内置运行时 | 引入不需要的依赖 |

---

## 实现策略

### 阶段划分

#### 阶段 1：基础框架
- [ ] 添加 inkwell 依赖
- [ ] 实现 LLVM 上下文初始化（`context.rs`）
- [ ] 实现基础类型映射（`types.rs`）

#### 阶段 2：函数翻译
- [ ] 实现函数声明翻译（`func.rs`）
- [ ] 实现基础指令翻译（算术、控制流、调用）（`translator.rs`）
- [ ] 实现值映射（`values.rs`）

#### 阶段 3：所有权类型翻译
- [ ] 实现 `&T`/`&mut T` 令牌（零大小，编译后消失）
- [ ] 实现 `ref T`（胖指针 `{ i64*, T* }`）
- [ ] 实现 `?T`（`{ i1, T }` tagged union）
- [ ] 实现 `List(T)`（`{ T*, i64, i64 }`）
- [ ] 实现 Move 语义追踪（用于析构插入判断）

#### 阶段 4：spawn 块代码生成
- [ ] 消费 `spawn_placement.rs` 的分析结果
- [ ] 直接子表达式 → 任务函数生成
- [ ] 依赖任务调度代码生成
- [ ] 资源冲突串行化
- [ ] spawn for 展开

#### 阶段 5：FFI 代码生成
- [ ] `native()` → `declare external`（`ffi.rs`）
- [ ] 参数 marshalling / 返回值 unmarshalling
- [ ] 不透明类型布局（含单字段优化）
- [ ] `?T` null 指针优化（FFI 专用）

#### 阶段 6：析构函数代码生成
- [ ] `.drop` 绑定识别
- [ ] 作用域结束 cleanup 插入（逆序）（`drop.rs`）
- [ ] 提前返回路径 cleanup
- [ ] `?` 错误传播路径 cleanup

#### 阶段 7：运行时库链接
- [ ] 实现 `runtime_task_spawn` / `runtime_task_wait_all` 等运行时函数
- [ ] 链接运行时静态库
- [ ] 端到端集成测试

### 依赖关系

- RFC-024（spawn 块并发）→ 阶段 4 的输入
- RFC-009 v9（所有权）→ 阶段 3、6 的输入
- RFC-008（运行时架构）→ 阶段 7 的输入
- RFC-026（FFI 机制）→ 阶段 5 的输入

---

## 相关工作

### Lazy Task Creation (1990)[^1]

| 属性 | 说明 |
|------|------|
| 机构 | MIT |
| 作者 | James R. Larus, Robert H. Halstead Jr. |
| 核心 | 延迟创建子任务，按需创建 |
| 参考价值 | spawn 块内任务按需调度的理论基础 |

**核心思想**：不是立即创建任务，而是延迟创建。当父任务需要子任务的值时，才创建子任务。这解决了细粒度并行任务的性能开销问题[^1]。YaoXiang 的 spawn 块调度借鉴了这一思想——任务在编译期识别，但运行时按需分发到线程池。

### Lazy Scheduling (2014)[^2]

| 属性 | 说明 |
|------|------|
| 机构 | University of Maryland |
| 作者 | Tzannes, Caragea |
| 核心 | 运行时自适应调度，无额外状态 |
| 参考价值 | Full Runtime WorkStealing 调度器设计参考 |

### SISAL 语言[^3]

| 属性 | 说明 |
|------|------|
| 机构 | Lawrence Livermore National Laboratory (LLNL) |
| 核心 | 单赋值语言，Dataflow 图，隐式并行 |
| 参考价值 | Dataflow 模型在工业级应用的可行性证明 |

**关键区别**：SISAL 的并行性是**隐式的**——语言是单赋值语义，编译器自动分析全程序的数据依赖图决定并行。YaoXiang 的并行性是**显式的**——用户用 `spawn {}` 块标记并行区域，编译器只在 spawn 块内分析依赖。这避免了 SISAL 的全程序分析复杂度，同时保留了用户对并行行为的控制。

### Mul-T 并行 Scheme[^4]

| 属性 | 说明 |
|------|------|
| 机构 | MIT |
| 核心 | Future 构造，Lazy Task Creation 实现 |
| 参考价值 | 具体实现参考 |

### 对比总结

| 技术 | 延迟创建 | 并行标记 | 分析范围 | 所有权 |
|------|----------|----------|----------|--------|
| Lazy Task Creation[^1] | ✅ | 隐式 | 全程序 | N/A |
| Lazy Scheduling[^2] | ✅ | 隐式 | 全程序 | N/A |
| SISAL[^3] | ✅ | 隐式（单赋值） | 全程序 | N/A |
| Mul-T[^4] | ✅ | 显式（future） | 调用点 | N/A |
| **YaoXiang** | ✅ | **显式（spawn 块）** | **spawn 块内** | **✅（Move + 令牌 + ref）** |

**YaoXiang 的创新**：将并行标记从"每个函数调用"（future）提升到"结构化块"（spawn），用户写普通代码，在需要并行的位置放 spawn 块。分析范围约束在 spawn 块内，编译高效且行为可控。

---

## 附录

### 附录 A：与 Rust async 对比

| 特性 | Rust async | YaoXiang LLVM AOT |
|------|-----------|-------------------|
| 编译产物 | 状态机 + 机器码 | 机器码 + spawn 任务元数据 |
| 运行时 | tokio | 静态链接调度器（约 500KB-1MB） |
| 并发标记 | async/await 关键字 | `spawn { }` 块 |
| 任务创建 | 编译期生成状态机 | 编译期识别直接子表达式 → 任务函数 |
| 颜色函数 | async 传染 | **无函数着色** |
| 同步等待 | `.await` | spawn 块自动同步阻塞 |
| 内存管理 | GC（运行时） | **RAII（确定）** |
| 共享机制 | `Arc::new()` + 手动 Weak | **`ref` 关键字（编译器自动选 Rc/Arc）** |

### 附录 B：设计决策记录

| 决策 | 决定 | 日期 |
|------|------|------|
| 采用 LLVM AOT | 直接 Codegen，不过度抽象 | 2026-02-15 |
| 并发模型对齐 | 对齐 RFC-024 spawn 块直接子表达式模型 | 2026-06-10 |
| DAG 分析范围 | spawn 块内，不跨 spawn 块（对齐 RFC-024） | 2026-06-05 |
| 所有权模型对齐 | 对齐 RFC-009 v9：`&T`/`&mut T` 令牌 + `ref` 关键字 | 2026-06-10 |
| 双后端模型 | VM（开发）+ LLVM（生产），对齐 RFC-008 | 2026-05-11 |
| 调度器形态 | 静态库链接进 exe，约 500KB-1MB（取决于平台与功能），无 GC | 2026-05-11 |
| FFI 代码生成 | 整合 RFC-026：`native()` declare + marshalling | 2026-06-10 |
| 析构函数 | `.drop` → RAII cleanup 插入，对齐 RFC-026 §7 | 2026-06-10 |
| 副作用处理 | 删除 `@IO`/`@Pure` 推断，改用 RFC-024 资源类型 | 2026-06-10 |
| 反射元数据 | 编译进 exe .reflect 段，mmap 按需加载 | 2026-05-11 |
| 论文引用 | 保留 Lazy Task Creation 等，明确 YaoXiang 的区别 | 2026-02-16 |

---

## 参考文献

[^1]: Larus, J. R., & Halstead, R. H. (1990). *Lazy Task Creation: A Technique for Increasing the Granularity of Parallel Programs*. MIT.

[^2]: Tzannes, A., & Caragea, G. (2014). *Lazy Scheduling: A Runtime Adaptive Scheduler for Declarative Parallelism*. University of Maryland.

[^3]: Feo, J. T., et al. (1990). *A report on the SISAL language project*. Lawrence Livermore National Laboratory.

[^4]: Mohr, E., et al. (1991). *Mul-T: A high-performance parallel lisp*. MIT.

- [inkwell LLVM bindings](https://github.com/TheDan64/inkwell)
- [RFC-024：基于 spawn 块的并发模型](../accepted/024-concurrency-model.md)
- [RFC-008：Runtime 并发模型与调度器脱耦设计](../accepted/008-runtime-concurrency-model.md)
- [RFC-009：所有权模型设计](../accepted/009-ownership-model.md)
- [RFC-026：FFI 核心机制](./026-ffi-core-mechanism.md)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 作者草稿，等待提交审核 |
| **审核中** | `docs/design/rfc/review/` | 开放社区讨论和反馈 |
| **已接受** | `docs/design/rfc/accepted/` | 成为正式设计文档 |
| **已拒绝** | `docs/design/rfc/` | 保留在 RFC 目录 |

> 当前状态：**已接受** — 已对齐 RFC-024 spawn 块并发模型、RFC-009 v9 所有权模型、RFC-026 FFI 机制
