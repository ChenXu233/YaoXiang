# RFC-009: 所有权模型设计

> **状态**: 审核中
> **作者**: 晨煦
> **创建日期**: 2025-01-08
> **最后更新**: 2025-01-08

## 参考文档

本文档的设计基于以下文档，并作为 language-spec 的详细设计来源：

| 文档 | 关系 | 说明 |
|------|------|------|
| [language-spec](../language-spec.md) | **规范目标** | 本 RFC 的设计将整合到语言规范中 |
| [manifesto](../manifesto.md) | **设计哲学** | 零成本抽象、默认不可变、无 GC |
| [RFC-001 并作模型](./001-concurrent-model-error-handling.md) | **并发安全** | Send/Sync 约束与所有权的交互 |
| [RFC-008 运行时并发模型](./accepted/008-runtime-concurrency-model.md) | **运行时集成** | 运行时内存管理与所有权的集成 |

## 摘要

本文档定义 YaoXiang 编程语言的**所有权模型（Ownership Model）**，包括所有权语义、移动语义、智能指针类型和 Send/Sync 约束。YaoXiang 采用简化的所有权模型，**不引入生命周期标注 `'a`**，通过编译器自动推断和移动语义实现内存安全，同时保持零成本抽象，无需 GC 即可实现高性能。

> **设计决策**：YaoXiang 选择**不实现生命周期标注**，通过以下策略解决借用问题：
> 1. 禁止返回借用引用（返回整个值）
> 2. 禁止结构体包含借用字段
> 3. 小对象直接复制（开销可忽略）
> 4. 大对象用移动语义（零拷贝）
> 5. 共享访问用 Arc（零拷贝）
> 6. 极端性能场景由标准库兜底

> **性能保证**：YaoXiang 的性能接近 Rust，远超 Go。小字段复制（< 1KB）开销 < 0.01%，大对象移动零拷贝。

## 动机

### 为什么需要所有权模型？

在传统编程语言中，内存管理一直是核心难题：

| 语言 | 内存管理方式 | 问题 |
|------|-------------|------|
| C/C++ | 手动管理 | 内存泄漏、野指针、双重释放 |
| Java/Python | GC | 延迟波动、内存开销、无法预测的暂停 |
| Rust | 所有权模型 | 编译期检查，无运行时开销 |

YaoXiang 采用所有权模型，旨在解决以下问题：

1. **内存安全**：自动释放，无内存泄漏
2. **数据竞争消除**：编译期检测并发访问问题
3. **零成本抽象**：无 GC 运行时开销
4. **可预测的性能**：无垃圾回收暂停

### 设计目标

```yaoxiang
# YaoXiang 所有权模型的核心目标

# 1. 自动内存管理（无 GC）
create_and_forget: () -> Data = () => {
    data = Data.new()  # 自动分配
    # data 离开作用域时自动释放
}

# 2. 移动语义（零拷贝）
transfer: (Data) -> Data = (data) => {
    Data(data.value * 2)  # 返回新值，所有权转移
}

# 3. 智能指针（共享所有权）
shared: (Data) -> Arc[Data] = (data) => Arc.new(data)
# Arc 零拷贝共享，多线程安全

# 4. 小对象复制（开销可忽略）
get_header: (BigData) -> Header = (data) => data.header
# 复制 64 字节开销 ~1ns，可忽略
```

### 关键设计决策：不实现生命周期标注

#### 为什么不做生命周期 `'a`？

| 问题 | 分析 |
|------|------|
| **`'a` 语法** | 丑、学习成本高、增加语言复杂度 |
| **编译器无法推断** | 返回哪个输入、返回新借用、结构体包含引用 |
| **用户负担** | 99% 的代码不需要生命周期，只有 1% 的复杂场景需要 |

#### 编译器无法推断的场景（被禁止）

```yaoxiang
# === 场景1：返回哪个输入引用？ ===
# pick: (ref Data, ref Data) -> ref Data = (a, b) => {
#     if a.value > b.value { a } else { b }
# }  # ❌ 编译器不知道返回哪个

# ✅ 替代方案：返回整个值
pick: (Data, Data) -> Data = (a, b) => {
    if a.value > b.value { a } else { b }
}

# === 场景2：返回"新借用" ===
# get_buffer: () -> ref Data = () => {
#     buffer = Data(42)
#     ref buffer  # ❌ 返回局部借用
# }

# ✅ 替代方案：返回整个值
get_buffer: () -> Data = () => Data(42)

# === 场景3：结构体包含借用 ===
# type Container = Container(data: ref Data)  # ❌

# ✅ 替代方案：直接包含值
type Container = Container(data: Data)
```

#### 损失 vs 收获

| | 损失 | 收获 |
|---|------|------|
| **表达能力** | 无法返回借用 | 简化 99% |
| **学习曲线** | - | 几乎为 0 |
| **代码安全** | - | 更高 |
| **常见代码影响** | < 1% | - |

> **结论**：99% 的情况，返回整个值完全可行。只有极端性能场景才需要避免复制，这些场景由标准库兜底。

## 提案

### 1. 所有权核心概念

#### 1.1 所有权规则

YaoXiang 的所有权系统基于以下核心规则：

```
┌─────────────────────────────────────────────────────────────────┐
│                    YaoXiang 所有权核心规则                       │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  规则 1：每个值有唯一的所有者                                     │
│  ├── 变量绑定是值的所有者                                        │
│  └── 所有者离开作用域时，值被自动销毁                            │
│                                                                 │
│  规则 2：所有权转移（Move）                                      │
│  ├── 赋值、函数传参、返回都可能转移所有权                        │
│  ├── 转移后原所有者失效                                         │
│  └── 转移成本为零（只是指针移动）                               │
│                                                                 │
│  规则 3：复制语义（Copy）                                        │
│  ├── 小对象（< 1KB）自动复制，开销可忽略                        │
│  ├── 复制后原所有者保持可用                                     │
│  └── 基础类型（Int、Float等）默认 Copy                          │
│                                                                 │
│  规则 4：引用传递（Borrow）                                      │
│  ├── ref Data - 不可变借用（只读）                              │
│  ├── mut Data - 可变借用（读写）                                │
│  └── 借用不能超过所有者作用域                                    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

#### 1.2 所有权语义示例

```yaoxiang
# === 规则 1：唯一所有者 ===

# 基础类型 - 复制语义
x: Int = 42
y = x  # 复制值，x 和 y 独立

# 复杂类型 - 移动语义
type Data = Data(value: Int)

create_data: () -> Data = () => {
    data = Data(42)  # data 是所有者
    data              # 返回时所有权转移给调用者
}

# ownership_move: () -> Void = () => {
#     data = Data(42)
#     data2 = data  # 所有权从 data 转移到 data2
#     # data 失效，无法再使用
#     print(data.value)  # ❌ 编译错误！
# }

# === 规则 2：引用传递 ===

# 不可变引用（可以有多个）
read_data: (ref Data) -> Int = (data) => data.value

# usage: () -> Void = () => {
#     data = Data(42)
#     r1 = read_data(data)   # 借用 1
#     r2 = read_data(data)   # 借用 2
#     # r1 和 r2 可以同时存在
# }

# 可变引用（只能有一个）
write_data: (mut Data) -> Void = (data) => {
    data.value = 100
}

# usage2: () -> Void = () => {
#     data = Data(42)
#     write_data(mut data)  # 可变借用
#     # data.value = 100     # ❌ 编译错误！借用期间不能直接修改
# }

# === 规则 3：所有权转移 ===

transfer: (Data) -> Data = (data) => {
    # data 的所有权转移进来
    # 函数结束时 data 被销毁
    Data(data.value * 2)  # 返回新值，所有权转移出去
}

# ownership_transfer: () -> Void = () => {
#     data = Data(42)
#     data2 = transfer(data)  # 所有权转移
#     # data 失效
#     print(data.value)  # ❌ 编译错误！
# }
```

### 2. 引用类型系统

#### 2.1 引用类型语法

```yaoxiang
# === 引用类型定义 ===

# ref T - 不可变引用
ref Int           # 指向 Int 的不可变引用
ref Data          # 指向 Data 的不可变引用

# mut T - 可变引用
mut Int           # 指向 Int 的可变引用
mut Data          # 指向 Data 的可变引用
```

#### 2.2 引用类型规则表

| 引用类型 | 可读 | 可写 | 可同时存在多个 | 可与可变引用共存 |
|----------|------|------|----------------|------------------|
| `ref T` | ✅ | ❌ | ✅ | ❌ |
| `mut T` | ✅ | ✅ | ❌ | ❌ |

```yaoxiang
# === 引用类型使用示例 ===

# 不可变引用 - 只读访问
readonly_access: (ref Data) -> Int = (data) => {
    data.value  # 只读，编译保证
}

# 可变引用 - 读写访问
readwrite_access: (mut Data) -> Void = (data) => {
    data.value = 100  # 可写
    x = data.value    # 可读
}
```

### 3. 性能分析

#### 3.1 小对象复制的实际开销

```yaoxiang
# === 小字段访问（99% 的情况） ===
type BigData = BigData(header: Header, payload: Bytes)
# 假设 Header = 64 字节

get_header: (BigData) -> Header = (data) => data.header
#                           ^^^^^^^^
#                           复制 64 字节

# 实际开销分析：
# - 复制 64 字节：~1 纳秒
# - 内存访问延迟：~100 纳秒
# - 函数调用开销：~10 纳秒
# - L1 缓存命中：~4 周期

# 结论：64 字节复制的开销可忽略不计
```

#### 3.2 大对象移动（零拷贝）

```yaoxiang
# === 大对象移动 ===
load_data: () -> BigData = () => BigData(...)

process: (BigData) -> Void = (data) => { ... }

main: () -> Void = () => {
    data = load_data()
    process(data)  # 移动，不是复制！
}
#                          ^^^^
#                          大对象直接移动，零拷贝
```

#### 3.3 性能对比

| 语言 | 小字段访问 | 大对象移动 | 共享访问 |
|------|-----------|-----------|---------|
| **Rust** | 零拷贝（借用） | 零拷贝（移动） | Arc 零拷贝 |
| **YaoXiang** | 复制 64B ⚠️ | 零拷贝（移动） | Arc 零拷贝 |
| **Go** | 指针（间接） | 拷贝 | 引用 |
| **Java** | 指针（间接） | 拷贝 | 引用 |

#### 3.4 真正影响性能的因素

```yaoxiang
# 比 64 字节复制更影响性能的是：

# 1. 内存分配
alloc: () -> Data = () => Data.new()  # 分配开销大

# 2. 缓存未命中
cache_miss: (BigStruct) -> Int = (s) => s.field[1000]  # 随机访问

# 3. 函数调用
call_overhead: () -> Void = () => tiny_func()  # 调用开销

# 4. 动态分派
dynamic: (Trait) -> Void = (t) => t.method()  # 虚表查找

# 64 字节复制？根本不在性能问题列表里！
```

#### 3.5 性能保证

```
┌────────────────────────────────────────────────────────────┐
│                                                            │
│  YaoXiang 性能保证：                                       │
│                                                            │
│  ✅ 小字段复制：开销 < 0.01% 运行时                         │
│  ✅ 大对象移动：零拷贝（所有权转移）                         │
│  ✅ 共享访问：Arc 原子计数，零拷贝                          │
│  ✅ 内存分配：优化器减少分配次数                             │
│                                                            │
│  与 Rust 对比：                                            │
│  - 借用优化：无（但影响可忽略）                             │
│  - 移动语义：✅ 相同                                        │
│  - Arc：✅ 相同                                            │
│  - 零成本抽象：✅ 相同                                      │
│                                                            │
│  结论：性能接近 Rust，远超 Go                               │
│                                                            │
└────────────────────────────────────────────────────────────┘
```

#### 3.6 极端性能场景（标准库兜底）

```yaoxiang
# 99.9% 的代码不需要任何特殊处理
get_header: (BigData) -> Header = (data) => data.header

# 极端性能场景：标准库提供视图（1% 情况）
use std.memory

get_header_view: (BigData) -> HeaderView = (data) => {
    std.memory.create_view(data, 0, 64)  # 零拷贝视图
}
```

### 4. 智能指针类型

#### 4.1 智能指针概览

```yaoxiang
# === 智能指针类型 ===

# Box[T] - 堆分配
# 用途：大小不确定的类型、递归类型、Trait 对象

# Rc[T] - 引用计数（非线程安全）
# 用途：单线程共享所有权

# Arc[T] - 原子引用计数（线程安全）
# 用途：多线程共享所有权

# RefCell[T] - 内部可变性（非线程安全）
# 用途：运行时借用检查

# Mutex[T] - 互斥锁包装（线程安全）
# 用途：线程安全的内部可变性

# RwLock[T] - 读写锁（线程安全）
# 用途：读多写少的场景
```

#### 4.2 Box - 堆分配

```yaoxiang
# === Box[T] - 堆分配 ===

# Box 用于将数据分配到堆上
heap_data: Box[Data] = Box.new(Data(42))

# 访问 Box 内容（自动解引用）
access_box: Box[Data] -> Int = (box) => {
    box.value           # 自动解引用
    (*box).value        # 显式解引用
    box.value           # 两种方式等价
}

# Box 主要用途 1：递归类型
type Tree = Tree(value: Int, children: List[Box[Tree]])
#                  ^^^^^^^^^^^^^^^^ 需要 Box 打破无限大小

# Box 主要用途 2：Trait 对象（未来特性）
# type Runnable = Runnable(run: () -> Void)
# process: (Box[Runnable]) -> Void = (r) => { r.run() }

# Box 主要用途 3：大小不确定的类型
# dynamic_size: (Box[DynamicData]) -> Void = (data) => { ... }
```

#### 4.3 Rc - 引用计数（非线程安全）

```yaoxiang
# === Rc[T] - 引用计数（单线程） ===

# 创建 Rc
shared_data: Rc[Data] = Rc.new(Data(42))

# 克隆 Rc（增加引用计数）
clone_rc: Rc[Data] -> Rc[Data] = (data) => {
    Rc.clone(data)  # 引用计数 +1
}

# 访问 Rc 内容（自动解引用）
access_rc: Rc[Data] -> Int = (rc) => {
    rc.value  # 自动解引用
}

# 引用计数变化
# rc_clone: () -> Void = () => {
#     data = Rc.new(Data(42))      # ref count = 1
#     data2 = Rc.clone(data)       # ref count = 2
#     data3 = data                 # ref count = 3（简写形式）
#     # data, data2, data3 都指向同一内存
#     # 最后销毁时 ref count = 0，内存释放
# }

# Rc 不是 Send（引用计数非原子）
# data: Rc[Data] = Rc.new(Data(42))
# spawn(() => process(data))  # ❌ 编译错误！Rc 不是线程安全

# Rc 不是 Sync（无法安全共享引用）
# ref_rc: ref Rc[Data] = ref data  # ❌ 编译错误！
```

#### 4.4 Arc - 原子引用计数（线程安全）

```yaoxiang
# === Arc[T] - 原子引用计数（多线程） ===

# 创建 Arc
thread_safe: Arc[Data] = Arc.new(Data(42))

# 克隆 Arc（原子操作，开销比 Rc 大）
clone_arc: Arc[Data] -> Arc[Data] = (data) => {
    Arc.clone(data)  # 原子增加引用计数
}

# 访问 Arc 内容（自动解引用）
access_arc: Arc[Data] -> Int = (arc) => {
    arc.value  # 自动解引用
}

# Arc 是 Send + Sync
# thread_safe: () -> Void = () => {
#     data = Arc.new(Data(42))
#
#     spawn(() => {
#         print(data.value)  # ✅ Arc 可以跨线程共享
#     })
#
#     spawn(() => {
#         print(data.value)  # ✅ 多个线程可同时读取
#     })
# }

# Arc 不是 Mutex，访问时需注意：
# Arc 提供共享所有权，但不提供同步访问
# 需要使用 Mutex/RwLock 实现内部可变性
```

#### 4.5 RefCell - 内部可变性（非线程安全）

```yaoxiang
# === RefCell[T] - 运行时借用检查（单线程） ===

# RefCell 允许在不可变上下文中修改内部数据
type Container = Container(value: RefCell[Int])

# 创建 RefCell
cell: RefCell[Int] = RefCell.new(42)

# 读取 - 通过 borrow()
read_cell: RefCell[Int] -> Int = (cell) => {
    # borrow() 返回不可变引用
    ref r = cell.borrow()
    r.value
}

# 写入 - 通过 borrow_mut()
write_cell: RefCell[Int] -> Void = (cell) => {
    # borrow_mut() 返回可变引用
    mut r = cell.borrow_mut()
    r.value = 100
}

# RefCell 借用规则在运行时检查
# runtime_borrow: () -> Void = () => {
#     cell = RefCell.new(42)
#
#     ref r1 = cell.borrow()      # ✅ 不可变借用成功
#     mut r2 = cell.borrow_mut()  # ❌ 运行时 panic！借用冲突
# }

# with 语法糖（自动释放借用）
with_cell: RefCell[Int] -> Int = (cell) => {
    cell.with(ref value) => value.value + 1
}
```

#### 4.6 Mutex - 互斥锁（线程安全）

```yaoxiang
# === Mutex[T] - 线程安全互斥锁 ===

# Mutex 提供线程安全的内部可变性
type SafeCounter = SafeCounter(mutex: Mutex[Int])

# 创建 Mutex
counter: Mutex[Int] = Mutex.new(0)

# 访问 - 通过 lock()（阻塞获取锁）
# lock() 返回 Guard，包含可变引用
access_mutex: Mutex[Int] -> Int = (mutex) => {
    guard = mutex.lock()  # 获取锁
    guard.value + 1       # 访问数据
}  # guard 离开作用域，自动释放锁

# with 语法糖（更简洁）
access_mutex2: Mutex[Int] -> Int = (mutex) => {
    mutex.with(mut value) => value + 1
}

# 并发访问示例
# concurrent_counter: () -> Void = () => {
#     counter = Mutex.new(0)
#
#     spawn(() => {
#         guard = counter.lock()
#         guard.value = guard.value + 1
#     })
#
#     spawn(() => {
#         guard = counter.lock()
#         guard.value = guard.value + 1
#     })
#     # 两个 spawn 会串行执行，因为 Mutex 保护数据
# }
```

#### 4.7 RwLock - 读写锁（线程安全）

```yaoxiang
# === RwLock[T] - 线程安全读写锁 ===

# RwLock 适合读多写少的场景
cache: RwLock[Map[String, Data]] = RwLock.new(Map.new())

# 读取 - read()（可多个并发读）
read_cache: RwLock[Map[String, Data]] -> Int = (cache) => {
    guard = cache.read()
    guard.size()
}

# 写入 - write()（排他性写）
write_cache: (RwLock[Map[String, Data]], String, Data) -> Void = (cache, key, data) => {
    guard = cache.write()
    guard.insert(key, data)
}

# with 语法糖
read_cache2: RwLock[Map[String, Data]] -> Int = (cache) => {
    cache.with(ref data) => data.size()
}

write_cache2: (RwLock[Map[String, Data]], String, Data) -> Void = (cache, key, data) => {
    cache.with(mut data) => data.insert(key, data)
}
```

### 5. Send/Sync 约束

#### 5.1 Send 约束

**Send**：类型可以安全地跨线程**转移所有权**。

```yaoxiang
# === Send 约束 ===

# 基本类型自动满足 Send
# Int, Float, Bool, String 都是 Send

# 结构体自动派生 Send
type Point = Point(x: Int, y: Float)
# Point 是 Send，因为 Int 和 Float 都是 Send

# 包含非 Send 字段的类型不是 Send
type NonSend = NonSend(data: Rc[Int])
# Rc 不是 Send（引用计数非原子），因此 NonSend 不是 Send

# spawn 要求参数满足 Send
# spawn_task: () -> Void = () => {
#     data = Rc.new(42)
#     spawn(() => {
#         print(data)  # ❌ 编译错误！Rc 不是 Send
#     })
# }

# 解决方案：使用 Arc（原子引用计数）
# safe_task: () -> Void = () => {
#     data = Arc.new(42)  # Arc 是 Send
#     spawn(() => {
#         print(data)  # ✅
#     })
# }
```

#### 5.2 Sync 约束

**Sync**：类型可以安全地跨线程**共享引用**。

```yaoxiang
# === Sync 约束 ===

# 基本类型都是 Sync
type Point = Point(x: Int, y: Float)
# &Point 是 Sync，因为 &Int 和 &Float 都是 Sync

# 包含内部可变性的类型
type Counter = Counter(value: Int, mutex: Mutex[Int])
# &Counter 是 Sync，因为 Mutex 提供内部可变性

# 非 Sync 类型
type NotSync = NotSync(data: Rc[Int])
# &NotSync 不是 Sync，因为 Rc 不提供线程安全的共享访问

# Sync 的语义
# shared_ref: () -> Void = () => {
#     data = Arc.new(42)  # Arc 是 Sync
#     ref r = ref data    # 可以安全共享引用
#
#     spawn(() => {
#         print(r.value)  # ✅ 多个线程可以同时读取
#     })
#
#     spawn(() => {
#         print(r.value)  # ✅
#     })
# }
```

#### 5.3 Send/Sync 派生规则

```yaoxiang
# === Send/Sync 派生规则 ===

# 结构体类型
type Struct[T1, T2] = Struct(f1: T1, f2: T2)

# Send 派生规则
# Struct[T1, T2]: Send ⇐ T1: Send 且 T2: Send

# Sync 派生规则
# Struct[T1, T2]: Sync ⇐ T1: Sync 且 T2: Sync

# 联合类型
type Result[T, E] = ok(T) | err(E)

# Send 派生规则
# Result[T, E]: Send ⇐ T: Send 且 E: Send

# 泛型容器
type Box[T] = Box(T)
type Option[T] = some(T) | none

# Send/Sync 派生
# Box[T]: Send ⇐ T: Send
# Box[T]: Sync ⇐ T: Sync
# Option[T]: Send ⇐ T: Send
# Option[T]: Sync ⇐ T: Sync
```

#### 5.4 标准库类型约束表

| 类型 | Send | Sync | 说明 |
|------|:----:|:----:|------|
| `Int`, `Float`, `Bool` | ✅ | ✅ | 原类型 |
| `String` | ✅ | ✅ | UTF-8 字符串 |
| `Box[T]` | ✅ | ✅ | T: Send + Sync |
| `Rc[T]` | ❌ | ❌ | 非原子引用计数 |
| `Arc[T]` | ✅ | ✅ | T: Send + Sync |
| `RefCell[T]` | ❌ | ❌ | 运行时借用检查 |
| `Mutex[T]` | ✅ | ✅ | T: Send |
| `RwLock[T]` | ✅ | ✅ | T: Send |
| `Channel[T]` | ✅ | ❌ | 只发送端 Send |
| `Vec[T]` | ✅ | ❌ | T: Send（但 &Vec 不是 Sync） |

### 6. RAII 与资源管理

#### 6.1 RAII 模式

```yaoxiang
# === RAII（资源获取即初始化）===

# RAII 保证资源在所有者销毁时自动释放
type File = File(handle: FileHandle)

# open_file: (String) -> File = (path) => {
#     handle = os.open(path)  # 获取资源
#     File(handle)             # 绑定到所有者
# }  # 函数结束时，如果成功则返回 File
#     如果失败则 handle 自动关闭

# use_file: (File) -> Void = (file) => {
#     content = file.read_all()
#     # 使用文件
# }  # file 离开作用域，自动关闭文件

# === 标准库 RAII 示例 ===

# with 语法糖（简化 RAII 使用）
with_file: (String) -> String = (path) => {
    with file = File.open(path) {
        file.read_all()
    }
}  # file 自动关闭

# === 自定义 RAII 类型 ===

type DatabaseConnection = DatabaseConnection(conn: DBConn)

new_connection: (String) -> DatabaseConnection = (url) => {
    conn = db.connect(url)
    DatabaseConnection(conn)
}

# 析构函数（未来特性）
# drop: (DatabaseConnection) -> Void = (conn) => {
#     db.disconnect(conn.conn)
# }
```

#### 6.2 Drop 语义

```yaoxiang
# === Drop 语义（资源释放）===

# 资源在所有者离开作用域时自动释放
scoped_resource: () -> Void = () => {
    resource = create_expensive_resource()
    # 使用 resource
}  # resource 被 drop，资源释放

# 显式释放
explicit_drop: () -> Void = () => {
    resource = create_expensive_resource()
    # 使用 resource
    resource.drop()  # 显式提前释放
}  # 此时 resource 已无效，不能再使用

# 移动后自动 drop
move_and_drop: () -> Void = () => {
    resource1 = create_resource()
    resource2 = create_resource()

    resource1 = resource2  # resource1 被 drop
                           # resource2 的所有权转移给 resource1
}  # resource2（在 resource1 中）被 drop
```

## 详细设计

### 语法定义

```bnf
# === 所有权语法 ===

# 引用类型
RefType      ::= 'ref' Type
              | 'mut' Type

# 泛型约束（未来特性）
WhereClause  ::= 'where' Type ':' Constraint (',' Constraint)*
Constraint   ::= 'Send'
              | 'Sync'

# 智能指针类型
SmartPointer ::= 'Box' '[' Type ']'
              | 'Rc' '[' Type ']'
              | 'Arc' '[' Type ']'
              | 'RefCell' '[' Type ']'
              | 'Mutex' '[' Type ']'
              | 'RwLock' '[' Type ']'

# 变量声明（带所有权）
LetStmt      ::= ('mut' | 'ref')? Identifier (':' Type)? '=' Expr
```

### 类型系统约束

```yaoxiang
# === 所有权类型系统规则 ===

# 规则 1：借用检查
# 不可变引用（ref）规则：
#   - 可以有任意多个 ref 借用
#   - 借用期间所有者不能修改
#   - 借用期间不能有 mut 借用

# 可变引用（mut）规则：
#   - 只能有一个 mut 借用
#   - 借用期间不能有其他 ref 或 mut 借用
#   - 所有者不能访问

# 规则 2：生命周期
# 引用的生命周期不能超过所有者
# lifetime_rule: [<'a, 'b>](&'a T) -> &'b T = (r) => {
#     r  # ❌ 编译错误！'b 可能比 'a 短
# }

# 规则 3：Send/Sync
# spawn 要求：
#   - 参数类型必须是 Send
#   - 返回类型必须是 Send
#   - 闭包捕获的所有变量必须是 Send

# 规则 4：智能指针
# Box[T] 需要 T: Sized（大小已知）
# Rc[T]/Arc[T] 需要 T: Clone（克隆语义）
# RefCell[T] 需要 T: Sized
# Mutex[T] 需要 T: Send
```

### 编译器实现

#### 借用检查器

```rust
// 借用检查器核心数据结构
struct BorrowChecker {
    // 所有权关系
    ownerships: Map<ValueId, Owner>,
    // 借用关系
    borrows: Vec<Borrow>,
    // 生命周期
    lifetimes: Map<LifetimeId, Lifetime>,
}

struct Borrow {
    borrow_id: BorrowId,
    value_id: ValueId,
    borrow_kind: BorrowKind,  // Ref 或 Mut
    lifetime: Lifetime,
    location: Span,
}
```

#### 生命周期分析

```rust
// 生命周期分析算法
fn analyze_lifetimes(fn: &Function) -> LifetimeMap {
    // 1. 收集所有借用
    let borrows = collect_borrows(fn);

    // 2. 构建生命周期约束
    let constraints = build_constraints(borrows);

    // 3. 求解生命周期
    let lifetimes = solve_constraints(constraints);

    // 4. 检查约束冲突
    check_conflicts(lifetimes);

    lifetimes
}
```

## 权衡

### 优点

1. **内存安全**：编译期消除内存泄漏和野指针
2. **数据竞争消除**：Send/Sync 约束保证并发安全
3. **零成本抽象**：无 GC 运行时开销
4. **可预测性能**：无垃圾回收暂停
5. **RAII 资源管理**：文件、网络等资源自动释放
6. **AI 友好**：明确的规则易于 AI 理解和生成
7. **学习曲线低**：无需理解生命周期标注
8. **简洁**：99% 的代码无需特殊处理

### 缺点

1. **小字段复制**：< 1KB 对象复制（开销可忽略）
2. **无法返回借用**：需要返回整个值
3. **结构体不能含借用**：需要包含值或用 Arc
4. **极端性能场景**：需要标准库视图（1% 情况）

## 替代方案

| 方案 | 为什么不选择 |
|------|-------------|
| GC（垃圾回收） | 有运行时开销，无法预测暂停 |
| 手动内存管理 | 内存泄漏、野指针风险 |
| ARC（自动引用计数） | 无法检测数据竞争 |
| 区域分析 | 复杂度高，实现困难 |

## 实现策略

### 阶段 1：基础所有权（v0.5）

- [ ] 所有权规则实现
- [ ] 移动语义
- [ ] 基本类型 Copy 语义
- [ ] 借用检查器

### 阶段 2：智能指针（v0.6）

- [ ] Box 实现
- [ ] Rc 实现
- [ ] Arc 实现
- [ ] RefCell/Mutex 实现

### 阶段 3：Send/Sync（v0.7）

- [ ] Send 约束检查
- [ ] Sync 约束检查
- [ ] spawn 集成
- [ ] 并发安全测试

### 阶段 4：标准库扩展（v0.8）

- [ ] 视图类型（std.memory）
- [ ] 句柄类型
- [ ] Copy-on-Write 优化

## 开放问题

| 议题 | 状态 | 说明 |
|------|------|------|
| Drop 语法 | 待定 | 是否需要显式 `drop()` 函数 |
| Copy 派生 | 已解决 | 基本类型自动实现 Copy |
| 视图类型 API | 待定 | 标准库视图的具体设计 |
| Pin/Unpin | 待定 | 是否需要 Future 安全特性 |

---

## 附录

### 附录A：与 Rust 的差异

| 特性 | Rust | YaoXiang |
|------|------|----------|
| 借用语法 | `&T`, `&mut T` | `ref T`, `mut T` |
| 生命周期 | `&'a T` | **无**（自动推断） |
| 返回借用 | ✅ 支持 | ❌ 禁止（返回整个值） |
| 结构体含借用 | ✅ 支持 | ❌ 禁止（包含值或 Arc） |
| Drop | `Drop` trait | 内置 drop 语义 |
| Copy | `Copy` trait | 基本类型自动 Copy |

### 附录B：设计决策记录

| 决策 | 决定 | 日期 | 记录人 |
|------|------|------|--------|
| 引用语法 | `ref T`, `mut T` | 2025-01-08 | 晨煦 |
| **生命周期** | **不实现** | 2025-01-08 | 晨煦 |
| **借用返回** | **禁止** | 2025-01-08 | 晨煦 |
| **结构体借用** | **禁止** | 2025-01-08 | 晨煦 |
| 小对象复制 | 自动（开销可忽略） | 2025-01-08 | 晨煦 |
| 大对象移动 | 零拷贝 | 2025-01-08 | 晨煦 |
| 智能指针 | Box/Rc/Arc/RefCell/Mutex | 2025-01-08 | 晨煦 |
| Send/Sync | 编译时约束 | 2025-01-08 | 晨煦 |
| RAII | 自动资源释放 | 2025-01-08 | 晨煦 |
| 极端性能场景 | 标准库兜底 | 2025-01-08 | 晨煦 |

### 附录C：术语表

| 术语 | 定义 |
|------|------|
| 所有者（Owner） | 负责释放值的变量或资源 |
| 借用（Borrow） | 引用值的临时访问 |
| 移动（Move） | 所有权的转移 |
| 复制（Copy） | 值的浅拷贝（< 1KB 自动复制） |
| Send | 可安全跨线程传输 |
| Sync | 可安全跨线程共享引用 |
| RAII | 资源获取即初始化 |
| 借用检查器 | 验证借用规则的编译器组件 |
| 视图类型 | 标准库提供的零拷贝访问方式 |

---

## 参考文献

### YaoXiang 官方文档

- [语言规范](../language-spec.md)
- [设计宣言](../manifesto.md)
- [RFC-001 并作模型](./001-concurrent-model-error-handling.md)
- [RFC-008 运行时并发模型](./accepted/008-runtime-concurrency-model.md)
- [YaoXiang 指南](../guides/YaoXiang-book.md)

### 外部参考

- [Rust 所有权模型](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [Rust 生命周期](https://doc.rust-lang.org/book/ch10-03-lifetime-syntax.html)
- [Rust Send/Sync](https://doc.rust-lang.org/book/ch16-04-extensible-concurrency-sync-and-send.html)
- [C++ RAII](https://en.wikipedia.org/wiki/Resource_acquisition_is_initialization)

---

## 生命周期与归宿

| 状态 | 位置 | 说明 |
|------|------|------|
| **草案** | `docs/design/rfc/` | 作者草稿，等待提交审核 |
| **审核中** | `docs/design/rfc/` | 开放社区讨论和反馈 |
| **已接受** | `docs/design/accepted/` | 成为正式设计文档 |
| **已拒绝** | `docs/design/rfc/` | 保留在 RFC 目录 |
