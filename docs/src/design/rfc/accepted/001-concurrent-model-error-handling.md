---
title: "RFC-001：并作模型与错误处理系统"
---

# RFC-001：并作模型与错误处理系统

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-05
> **最后更新**: 2026-05-11（修剪：移除 @auto、L1 回退启发式，精简讨论记录）

## 设计来源

| 文档 | 关系 |
|------|------|
| [async-whitepaper](/src/archive/async-whitepaper) | 设计源头，理论基础 |
| [language-spec](/src/design/language-spec) | 规范目标 |

## 摘要

提出 YaoXiang 的并作模型：以同步语法描述逻辑，运行时自动并发执行。核心机制：三层并发架构 + DAG 依赖分析 + Result 类型系统。

## 快速选择

| 场景 | 写法 | 说明 |
|------|------|------|
| 自动并行 | 不写注解（默认） | 最大化并行 |
| 同步等待 | `@eager` | 等待依赖完成 |
| 完全顺序 | `@block` | 无并发，调试用 |
| 局部并发 | `spawn` | @block 作用域内并发 |

## 动机

当前主流语言的并发模型存在明显缺陷：

| 语言 | 并发模型 | 问题 |
|------|----------|------|
| Rust | async/await + tokio | 异步传染、学习曲线陡峭 |
| Go | goroutine | 无类型安全 |
| Python | asyncio | GIL 限制 |
| JavaScript | Promise/async | 回调复杂 |

### 核心矛盾

1. **透明性 vs 可控性**：完全透明但不可控 vs 完全可控但不透明
2. **并发 vs 可调试**：并发程序难调试 vs 可调试程序难并发

---

## 提案

### 1. 并作模型：三层并发架构

> **说明**：L1/L2/L3 是心智模型，帮助用户理解不同场景。实际实现只有一套机制：DAG 自动分析 + 注解控制。

| 层级 | 心智模型 | 语法 | 执行方式 | 并行度 |
|------|----------|------|----------|--------|
| **L1** | 禁止并发 | `@block` | 纯顺序执行 | ❌ 无 |
| **L2** | @block 内并发 | `spawn` | @block 作用域内可控并发 | ⚠️ 部分 |
| **L3** | 完全并发 | 默认（无注解） | 自动分析 DAG | ✅ 完整 |

#### L1: @block 同步模式

```yaoxiang
main: () -> Void @block = {
    data1 = fetch_sync("api1")
    data2 = fetch_sync("api2")
    process(data1, data2)    # 严格顺序，无并发
}
```

#### L2: @block 内可控并发

```yaoxiang
# spawn 只能在 @block 函数内部使用
main: () -> Void @block = {
    spawn { data1 = fetch_data("api1") }
    spawn { data2 = fetch_data("api2") }
    # 等待所有 spawn 完成（标准库控制）
    process(data1, data2)
}
```

#### L3: 完全透明（默认）

```yaoxiang
# 不需要任何注解，编译器自动分析 DAG
heavy_calc: (n: Int) -> Int = fibonacci(n)

auto_parallel: (n: Int) -> Int = {
    a = heavy_calc(1)    # 自动并行
    b = heavy_calc(2)    # 自动并行
    c = heavy_calc(3)    # 自动并行
    a + b + c            # 需要值时等待所有结果
}
```

### 2. 注解完整对比

| 维度 | 默认（无注解） | `@eager` | `@block` | `spawn` |
|------|---------------|----------|----------|---------|
| **执行方式** | 自动 DAG 分析 | 同步等待依赖 | 纯顺序 | @block 内并发 |
| **并行度** | ✅ 完全 | ⚠️ 按依赖顺序 | ❌ 无 | ⚠️ 部分 |
| **DAG 构建** | ✅ | ✅ | ❌ | ✅ |

**选择指南**：
- 最大并发 → 不写注解（默认）
- 需要有序副作用 → `@eager`
- 调试/新手/关键代码 → `@block`
- @block 内需要并发 → `spawn`

```yaoxiang
# 默认：最大化并行
calc_all: () -> Int = {
    a = heavy_calc(1)    # 自动并行
    b = heavy_calc(2)    # 自动并行
    a + b
}

# @eager：同步等待
calc_seq: () -> Int @eager = {
    a = heavy_calc(1)    # 同步执行
    b = heavy_calc(2)    # 同步执行
    a + b
}

# @block：纯顺序
calc_simple: () -> Int @block = {
    a = heavy_calc(1)    # 强制同步
    b = heavy_calc(2)    # 同步
    a + b
}

# spawn：@block 内并发
calc_mixed: () -> Int @block = {
    spawn { heavy_calc(1) }
    spawn { heavy_calc(2) }
    heavy_calc(3)        # 同步
}
```

### 3. DAG 依赖分析

#### 3.1 核心原则：自底向上执行

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

**关键洞察**：不是"自顶向下"生成 Future，而是"自底向上"从结果反向分析依赖。

#### 3.2 孤岛 DAG：独立并行

```
主流程：fetch(url0) → process → print
孤岛：  fetch(url1)  ← 没人要结果，独立并行

调度器：主流程按依赖链执行，孤岛用另一核心并行
```

#### 3.3 资源类型与副作用

**核心思想**：资源操作通过类型标记，DAG 自动构建依赖。同一资源自动串行，不同资源自动并行。

**资源类型边界——明确定义**：

资源类型是编译器内置标记的类型。以下类型被编译器识别为资源：

| 资源类型 | 说明 | 编译器行为 |
|----------|------|-----------|
| `FilePath` | 文件系统路径 | 同路径操作自动串行 |
| `HttpUrl` | HTTP 端点 | 同 URL 操作自动串行 |
| `DBUrl` | 数据库连接 | 同连接操作自动串行 |
| `Console` | 标准输出 | 所有 Console 操作自动串行 |

用户自定义资源类型需显式标记：
```yaoxiang
Database: Resource              # 显式标记为资源类型
query: (Database, String) -> Result(Row, Error)
# 参数 Database 是 Resource，自动识别为资源操作
```

非 Resource 标记的类型不会被编译器追踪资源依赖。

**使用规则**：
- 通过变量传递资源句柄，DAG 自动管理顺序
- 字面量直接使用同一资源是用户设计问题，非语言责任

```yaoxiang
# ✅ 正确：变量传递，DAG 自动串行
filename: String = "data.txt"
File.write(filename, x)
File.write(filename, y)    # DAG 串行

# ⚠️ 用户责任：字面量
File.write("data.txt", x)
File.write("data.txt", y)  # 可能并行，用户自己负责
```

#### 3.4 无限循环处理

```
1 个循环 → 直接同步执行，零调度开销
多个循环 → 调度器切片切换，真正并发
```

### 4. Result 类型与错误处理

```yaoxiang
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Self, err: (E) -> Self }

# ? 运算符透明传播
process: () -> Result(Data, Error) = {
    data = fetch_data()?
    processed = transform(data)?
    save(processed)?
}
```

### 5. DAG 节点设计

```rust
enum NodeKind {
    Task,      // 任务节点
    Value,     // 值节点
    Control,   // 控制流节点
}

struct Node {
    id: NodeId,
    kind: NodeKind,
    inputs: Vec<ValueNodeId>,   // 输入依赖
    outputs: Vec<ValueNodeId>,  // 输出值
    span: Span,                 // 源码位置
}
```

| 边类型 | 符号 | 语义 |
|--------|------|------|
| DataEdge | → | 数据依赖（值流动） |
| ControlEdge | ● | 控制依赖（顺序执行） |
| SpawnEdge | ◎ | 并发入口（可并行起点） |

### 6. 类型系统

```
Send → 可安全跨线程传输
Sync → 可安全跨线程共享
Arc(T) 实现 Send + Sync（线程安全引用计数）
```

---

## 权衡

### 优点

1. **渐进式采用**：三层模型适应不同技能水平
2. **自然语法**：同步代码获得并行性能
3. **编译时安全**：Send/Sync 约束消除数据竞争
4. **可调试**：错误图提供清晰的错误传播视图

### 缺点

1. **学习曲线**：需要理解 DAG 依赖概念
2. **编译时间**：全程序 DAG 分析可能较慢
3. **工具链复杂度**：需要全新的调试和可视化工具

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 仅支持显式 async/await | 无法实现透明并发 |
| 仅支持完全透明并发 | 用户失去控制权 |
| Go 式 goroutine | 无类型安全、无法编译时检查 |
| 仅 L1 模式 | 放弃并作模型核心价值 |

## 实现策略

### 阶段划分

1. **阶段 1 (v0.1)**：@block 同步模式、基础类型
2. **阶段 2 (v0.2)**：FlowScheduler 调度器
3. **阶段 3 (v0.3)**：spawn 块、显式并发
4. **阶段 4 (v0.5)**：L3 完全透明、DAG 自动分析
5. **阶段 5 (v0.6)**：错误图、图调试器
6. **阶段 6 (v1.0)**：生产可用优化

### 依赖关系

- RFC-001 无外部依赖（基础核心）
- RFC-008（Runtime 并发模型）→ 设计完成
- RFC-011（泛型系统）→ 设计完成

### 风险

1. **DAG 分析性能**：全程序分析可能 O(n²)，需要优化
2. **工具链缺失**：调试器需要从零开发
3. **用户接受度**：透明并发需要良好文档

---

## 设计决策记录

| 决策 | 决定 | 日期 |
|------|------|------|
| 三层并发架构 | L1/L2/L3 渐进式 | 2025-01-05 |
| @block 注解位置 | 返回类型后 | 2025-01-05 |
| DAG 错误传播 | 沿依赖边向上游传播 | 2025-01-06 |
| DAG 性能优化 | 增量构建 + 缓存 | 2025-01-06 |
| 运行时选择 | 泛型 + 编译时注入 | 2025-01-06 |
| 节点接口 | 泛型 + 函数注入（无 trait） | 2025-01-06 |
| 错误图内存 | DAG 仅在单函数内构建 | 2025-01-06 |
| 资源冲突检测 | DAG 数据流依赖，用户变量传递 | 2025-01-06 |
| 资源类型系统 | Resource 标记 + DAG 自动依赖 | 2026-01-06 |
| L1/L2/L3 心智模型 | 三层抽象，非实现机制 | 2026-01-06 |
| @auto 注解 | 删除，与默认行为重复 | 2026-05-11 |
| L1 自动回退 | 删除，行为不可预测 | 2026-05-11 |

---

## 附录：术语表

| 术语 | 定义 |
|------|------|
| 并作模型 | YaoXiang 的并发范式：同步语法，异步本质 |
| DAG | 有向无环图，描述计算依赖关系 |
| spawn | @block 作用域内可控并发 |
| @block | 同步注解，禁用并发优化 |
| @eager | 急切求值，等待依赖完成 |
| Resource | 资源类型标记，操作自动构建 DAG 依赖 |
| 错误图 | 可视化的错误传播路径 |

## 参考文献

- [Rust async book](https://rust-lang.github.io/async-book/)
- [Go 并发模式](https://golang.org/doc/effective_go#concurrency)
- [工作窃取调度](https://en.wikipedia.org/wiki/Work_stealing)
- [并作模型白皮书](/src/archive/async-whitepaper)
- [YaoXiang 语言规范](/src/design/language-spec)
