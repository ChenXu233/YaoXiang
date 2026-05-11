---
title: RFC-009：所有权模型设计
---

# RFC-009: 所有权模型设计

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-08
> **最后更新**: 2026-05-11（大幅修剪：消除过度设计，ref 编译器自动优化）

## 摘要

本文档定义 YaoXiang 编程语言的**所有权模型（Ownership Model）**。

**核心设计——只有四个概念**：

- **Move（默认）**：赋值/传参/返回 = 所有权转移，零拷贝，RAII 自动释放
- **`ref` 关键字**：显式共享。**编译器自动选择底层实现**（Rc/Arc），用户永远只写 `ref`
- **`clone()`**：显式深拷贝
- **`unsafe` + `*T`**：裸指针，系统级编程逃生舱

**`ref` 的编译器自动优化**：

```
用户写 ref → 编译器分析：
  ├── 数据不逃逸到其他任务 → 编译为 Rc（非原子，开销低）
  ├── 数据逃逸到其他任务   → 编译为 Arc（原子操作，线程安全）
  ├── 任务内检测到环       → 不做警告（用户的权限，任务结束时统一释放）
  └── 跨任务检测到环       → 警告（提醒是否真的需要跨任务强引用）
```

用户只需要知道 `ref` = 共享。编译器自动选 Rc 还是 Arc。

**消除的复杂性**：
- ❌ 无生命周期 `'a`
- ❌ 无借用检查器
- ❌ 无 GC
- ❌ 无消费分析/所有权回流等"迷你借用检查器"
- ❌ 用户不需要知道 Rc/Arc 的区别（编译器自动选）

> **编程负担**：一个关键字（`ref`），编译器全自动。
> **性能保证**：Move 零开销，ref 按需付费，无 GC 暂停。

## 动机

### 为什么需要所有权模型？

| 语言 | 内存管理 | 问题 |
|------|----------|------|
| C/C++ | 手动管理 | 内存泄漏、野指针、双重释放 |
| Java/Python | GC | 延迟波动、内存开销、无法预测的暂停 |
| Rust | 所有权 + 借用检查 | 生命周期 `'a` 学习曲线陡峭 |
| **YaoXiang** | **Move + ref** | **简单、确定、无 GC** |

### 设计目标

```yaoxiang
# 1. 默认 Move（零拷贝）
p = Point(1.0, 2.0)
p2 = p                         # Move，p 不可再读
# 作用域结束，p2 自动释放

# 2. ref = 共享（编译器自动选 Rc/Arc）
shared = ref p                 # 编译器分析：跨任务→Arc，不跨→Rc

# 3. clone() = 显式复制
p2 = p.clone()                 # 深拷贝，p 和 p2 独立

# 4. unsafe + *T = 系统级
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### 与 Rust 的核心区别

| 特性 | Rust | YaoXiang |
|------|------|----------|
| 默认语义 | 借用 `&T` | **Move（值传递）** |
| 共享机制 | `Arc::new()` | **`ref` 关键字（编译器自动优化）** |
| 复制 | `clone()` | `clone()` |
| 裸指针 | `*T` | `*T` |
| 生命周期 | `'a` | ❌ 无 |
| 借用检查 | borrow checker | ❌ 无 |
| 循环引用 | 手动 Weak | **任务结束统一释放或警告** |

### 为什么消除生命周期？

**核心洞察**：没有 `&T` 引用，就不需要生命周期 `'a`。

```yaoxiang
# Rust 的问题：返回引用需要生命周期
# fn returns_ref() -> &Point {  // 需要 'a
#     let p = Point::new(1.0, 2.0);
#     &p  // 悬空指针！
# }

# YaoXiang：返回所有权，不需要生命周期
returns_value: () -> Point = {
    p = Point(1.0, 2.0)
    p                             # Move，所有权转移，零拷贝
}
```

---

## 提案

### 1. Move（默认所有权转移）

```yaoxiang
# 规则：赋值 / 传参 / 返回 = Move，零拷贝

p: Point = Point(1.0, 2.0)
p2 = p                           # Move，p 不可再读

# 变量可以重新赋值（Python 风格，无遮蔽）
p = Point(3.0, 4.0)              # p 重新绑定，类型必须一致

# 函数参数：Move
process: (p: Point) -> Point = {
    p.transform()                # p 被移入
    p                            # Move 返回
}

# 函数返回：Move
create: () -> Point = {
    p = Point(1.0, 2.0)
    p                            # Move 返回，零拷贝
}
```

**特点**：
- 零拷贝（编译器移动指针）
- 移动后原绑定不可读（编译错误）
- RAII：作用域结束自动释放
- 函数签名 `(T) -> T` 本身就是文档——消费 T，返回 T

### 2. ref 关键字（编译器自动优化）

`ref` 是用户唯一需要知道的共享关键字。底层是 Rc 还是 Arc，用户不需要关心。

#### 2.1 基本使用

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # 共享，编译器自动选实现

# 多个 spawn 共享
@block 
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }           # 编译器：跨任务 → Arc
    spawn { use(data) }           # 编译器：跨任务 → Arc
}

# 单线程共享
@block
main: () -> Void = {
    data = ref heavy_data
    use(data)                     # 编译器：不跨任务 → Rc
}
```

**用户心智模型**：`ref` = 共享。够了。

#### 2.2 编译器逃逸分析：Rc vs Arc

```
ref 的数据流分析：

不逃逸到其他任务 → Rc（非原子引用计数，开销低）
逃逸到其他任务   → Arc（原子引用计数，线程安全）

用户不需要写 Rc 或 Arc，编译器自动选。
```

```yaoxiang
# 场景 A：不跨任务 → 编译器生成 Rc
process_local: (data: BigData) -> Void = {
    shared = ref data
    do_something(shared)
    do_something_else(shared)
    # 编译器：所有使用都在当前任务 → Rc
}

# 场景 B：跨任务 → 编译器生成 Arc
@block
process_concurrent: () -> Void = {
    data = BigData()
    shared = ref data
    spawn { worker_a(shared) }   # 跨任务！
    spawn { worker_b(shared) }   # 跨任务！
    # 编译器：shared 逃逸到其他任务 → Arc
}
```

#### 2.3 环检测策略

```
环检测分两级：

任务内环 → 不做警告。
  ├── 结构化并发保证任务结束时所有资源统一释放。
  ├── ref 永远保活，语义不掺水。
  └── 用户有权在任务内构建双向强引用（例如图计算中间态）。

跨任务环 → 警告。
  ├── 提醒用户重新思考：是否真的需要跨任务强引用？
  ├── 虽然父任务结束时子任务资源全释放，不会真泄漏，
  │   但跨任务强引用意味着所有权边界模糊。
  └── 需要打破环时，用标准库的 Weak。
```

```yaoxiang
# 任务内环：不做警告，允许双向强引用
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # 不做警告。任务结束时统一释放。
}

# 跨任务环：警告
@block
parent_task: () -> Void = {
    shared_a = ref a
    shared_b = ref b

    spawn {
        shared_a.child = ref shared_b   # ⚠️ 警告：跨任务循环引用
    }
}
```

**环检测规则**：

| 环类型 | 行为 | 原因 |
|--------|------|------|
| 任务内 ref 环 | 不做警告 | 用户的权限，任务结束统一释放 |
| 跨任务 ref 环 | 警告 | 提醒用户重新思考跨任务强引用的必要性 |

#### 2.4 ref 与并发注解的交互

`ref` 的行为与 `@block` / `@eager` 注解无关：

```yaoxiang
# 无论什么注解，ref 的语义不变
# 注解控制的是调度策略，不是内存语义

blocked: () -> Void @block = {
    shared = ref data              # 仍为 Rc（不跨任务）
    use(shared)
}

eager: () -> Void @eager = {
    shared = ref data              # 仍为 Rc（除非 spawn）
    use(shared)
}
```

### 3. clone() —— 显式复制

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # 深拷贝
# p 和 p2 独立，互不影响
```

**何时使用**：需要保留原值且不适合共享的场景。

### 4. unsafe + 裸指针（系统级编程）

```yaoxiang
p: Point = Point(1.0, 2.0)

unsafe {
    ptr: *Point = &p              # 裸指针
    (*ptr).x = 0.0                # 解引用（用户保证安全）
    ptr2 = ptr + 1                # 指针运算
}
```

**限制**：
- 只能在 `unsafe` 块中使用
- 用户保证不悬空、不释放后使用
- 可绕过 unsafe 块内的限制（用户负责）
- 用于 FFI、内存操作等系统级编程

---

## 综合示例

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,
}

Shape: Type = {
    points: List(Point),
    color: Color,
}

# 1. Move（默认）
shape1 = Shape([Point(0.0, 0.0)], Color.red())
shape2 = shape1                    # Move，shape1 不可再读
shape1 = Shape([Point(1.0, 1.0)], Color.blue())  # 重新赋值

# 2. ref（编译器自动选 Rc/Arc）
shared_shape = ref shape2         # 编译器分析是否需要 Arc
spawn { print(shared_shape.color) }

# 3. 链式调用（Move + 重新赋值）
shape1 = shape1.translate(10.0, 10.0)
shape1 = shape1.rotate(90)
shape1 = shape1.scale(2.0)

# 4. clone() 显式复制
backup = shape1.clone()

# 5. unsafe 系统级
unsafe {
    ptr: *Point = &shape1.points[0]
    (*ptr).x = 0.0
}

# 6. 任务内环：任务结束时统一释放
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                    # 环！任务结束时 a、b 一起释放
```

---

## 类型系统约束

### Send / Sync

| 类型 | Send | Sync | 说明 |
|------|------|------|------|
| 值类型 | ✅ | ✅ | Int, Float, Point... |
| `ref T` | ✅ | ✅ | 编译器自动选 Rc/Arc |
| `*T` | ❌ | ❌ | 裸指针，单线程 |

```yaoxiang
# 基本类型自动满足 Send + Sync
# ref(T) 自动满足 Send + Sync（编译器保证线程安全）

main: () -> Void @block = {
    p: Point = Point(1.0, 2.0)
    shared = ref p
    spawn { print(shared.x) }     # ✅ 编译器：跨任务 → Arc
}
```

---

## 性能分析

| 操作 | 成本 | 说明 |
|------|------|------|
| Move | 零 | 指针移动 |
| `ref`（不跨任务）| 低 | 编译为 Rc，非原子操作 |
| `ref`（跨任务）| 中 | 编译为 Arc，原子操作 |
| `clone()` | 视类型 | 小对象快，大对象慢 |
| `unsafe + *T` | 零 | 直接内存操作 |

### 对比

| 语言 | 共享机制 | 内存管理 | 循环处理 | 复杂度 |
|------|----------|----------|----------|--------|
| Rust | Arc / Mutex | 编译期检查 | 手动 Weak | 高 |
| Go | chan / pointer | GC | GC | 低 |
| C++ | shared_ptr | RAII | weak_ptr | 中 |
| **YaoXiang** | **ref（编译器自动）** | **RAII** | **任务边界释放** | **低** |

---

## 权衡

### 优点

1. **简单**：无生命周期，无借用检查器，用户只学 `ref` 一个关键字
2. **编译器智能**：自动选 Rc/Arc，用户不用管
3. **确定性**：ref 就是保活，不会悄悄变弱引用。任务内环靠任务结束统一释放
3. **高性能**：Move 零拷贝，ref 按需付费
4. **确定**：RAII 确定性释放，无 GC 暂停
5. **灵活**：`unsafe + *T` 支持系统级编程

### 缺点

1. **ref 运行时开销**：原子操作有成本（但这是共享的必然代价）
2. **unsafe 风险**：用户必须保证正确性
3. **跨任务环只是警告**：不像 Rust 那样编译报错，依赖用户重视警告

---

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| GC | 有运行时开销，无法预测暂停 |
| Rust 借用检查器 | 用户负担重，学习曲线陡 |
| 纯 Move | 无法处理并发共享 |
| 无裸指针 | 无法系统级编程 |
| 暴露 Rc/Arc 给用户 | 把实现细节甩给用户，增加认知负担 |

---

## 社区讨论

### 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| **默认值** | Move（零拷贝） | 高性能，零开销 | 2025-01-15 |
| **共享机制** | `ref` 关键字，编译器自动优化 | 用户简单，编译器负责 | 2025-01-15 |
| **复制** | `clone()` | 显式语义 | 2025-01-15 |
| **系统级** | `*T` + `unsafe` | 支持系统编程 | 2025-01-15 |
| **生命周期** | 不实现 | 无 `&T` 引用，不需要 | 2025-01-15 |
| **借用检查器** | 不实现 | Move + ref 替代 | 2025-01-15 |
| **Rc/Arc** | 编译器自动选择，用户不可见 | 降低认知负担 | 2025-01-15 |
| **循环引用** | 任务内不做警告（用户权限），跨任务警告 | 结构化并发天然保证 | 2025-01-16 |
| **消费分析** | 删除 | 迷你借用检查器，不需要 | 2026-05-11 |
| **所有权回流** | 删除 | 函数签名就是文档 | 2026-05-11 |
| **空状态重用** | 删除（作为特性） | Move 后重新赋值是自然行为 | 2026-05-11 |
| **逆函数生成** | 删除 | 未来特性，挪出核心 RFC | 2026-05-11 |
| **字段级三层可变性** | 删除 | 过度设计 | 2026-05-11 |

### 版本历史

| 版本 | 主要变更 | 日期 |
|------|----------|------|
| v1 | 初稿：基于 Rust 所有权模型 | 2025-01-08 |
| v2 | 引入 `ref` 关键字，删除借用检查器 | 2025-01-10 |
| v3 | 删除生命周期，简化设计 | 2025-01-13 |
| v4 | 默认 Move + 显式 ref = Arc | 2025-01-15 |
| v5 | 结构化并发 + 循环引用处理 | 2025-01-16 |
| v6 | 新增空状态重用、所有权回流 | 2025-02-04 |
| v7 | 新增消费分析、逆函数生成、字段级可变性 | 2025-02-05 |
| **v8** | **大幅修剪：删除过度设计，ref 编译器自动优化** | **2026-05-11** |

### 待决议题

| 议题 | 说明 | 状态 |
|------|------|------|
| Drop 语法 | 是否需要显式 `drop()` 函数 | 待讨论 |
| 逃逸分析算法 | ref 的跨任务检测实现 | 待讨论 |
| 环检测算法 | 跨任务环警告的实现方案 | 待讨论 |

---

## 参考文献

### YaoXiang 官方文档

- [语言规范](../language-spec.md)
- [设计宣言](../manifesto.md)
- [RFC-001 并作模型](./001-concurrent-model-error-handling.md)
- [YaoXiang 指南](../guides/YaoXiang-book.md)

### 外部参考

- [Rust 所有权模型](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)
- [Erlang 消息传递](https://www.erlang.org/doc/getting_concurrency/getting_concurrency.html)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 作者草稿，等待提交审核 |
| **审核中** | `docs/design/rfc/` | 开放社区讨论和反馈 |
| **已接受** | `docs/design/accepted/` | 成为正式设计文档 |
| **已拒绝** | `docs/design/rfc/` | 保留在 RFC 目录 |
