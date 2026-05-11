---
title: RFC-009：所有权模型设计
---

# RFC-009: 所有权模型设计

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-08
> **最后更新**: 2026-05-11（新增丐版借用 &T/&mut T，完善所有权梯度）

## 摘要

本文档定义 YaoXiang 编程语言的**所有权模型（Ownership Model）**。

**核心设计——五个概念，一个梯度**：

```
看一眼/原地改     拿走           共享持有         复制一份        系统级
    │              │              │              │              │
   &T            Move           ref          clone()        unsafe
  &mut T         零拷贝        编译器自动      显式深拷贝      *T
  函数参数         默认          选Rc/Arc                   用户负责
  禁止逃逸                     任务内环静默
                               跨任务环 lint
                               标准库Weak
```

- **Move（默认）**：赋值/传参/返回 = 所有权转移，零拷贝，RAII 自动释放
- **`&T` / `&mut T`（丐版借用）**：只做函数参数，禁止逃逸。零标注，零生命周期。编译器在调用侧自动借用
- **`ref` 关键字**：跨作用域共享。编译器自动选 Rc（不跨任务）还是 Arc（跨任务）
- **`clone()`**：显式深拷贝
- **`unsafe` + `*T`**：裸指针，系统级逃生舱

**消除的复杂性**：
- ❌ 无生命周期 `'a`
- ❌ 无借用检查器（禁止而非标注，不需要 Rust 式的全局生命周期推导）
- ❌ 无 GC
- ❌ 无消费分析/所有权回流等"迷你借用检查器"
- ❌ 用户不需要知道 Rc/Arc 的区别（编译器自动选）

> **编程负担**：`&T`/`&mut T` 三条规则，`ref` 一个关键字，编译器全自动。
> **性能保证**：Move 零开销，borrow 零开销，ref 按需付费，无 GC 暂停。

## 动机

### 为什么需要所有权模型？

| 语言 | 内存管理 | 问题 |
|------|----------|------|
| C/C++ | 手动管理 | 内存泄漏、野指针、双重释放 |
| Java/Python | GC | 延迟波动、内存开销、无法预测的暂停 |
| Rust | 所有权 + 借用检查 | 生命周期 `'a` 学习曲线陡峭 |
| **YaoXiang** | **Move + Borrow + ref** | **简单、确定、无 GC** |

### 设计目标

```yaoxiang
# 1. 默认 Move（零拷贝）
p = Point(1.0, 2.0)
p2 = p                         # Move，p 不可再读

# 2. &T / &mut T 借用（零开销，只读/原地修改，禁止逃逸）
print_info(p2)                 # 编译器自动借用 &p2，用完即还
shift(p2, 1.0, 1.0)           # 编译器自动借用 &mut p2

# 3. ref = 共享（编译器自动选 Rc/Arc）
shared = ref p2                # 跨作用域持有
spawn { use(shared) }          # 编译器：跨任务 → Arc

# 4. clone() = 显式复制
backup = p2.clone()            # 深拷贝，独有

# 5. unsafe + *T = 系统级
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

### 与 Rust 的核心区别

| 特性 | Rust | YaoXiang |
|------|------|----------|
| 默认语义 | 借用 `&T`（需显式 `.clone()`） | **Move（值传递，零拷贝）** |
| 借用 | `&T`/`&mut T`，可返回，需生命周期 | **`&T`/`&mut T` 只做参数，禁止逃逸** |
| 共享机制 | `Arc::new()` + 手动 Weak | **`ref` 关键字（编译器自动选 Rc/Arc）** |
| 复制 | `clone()` | `clone()` |
| 裸指针 | `*T` | `*T` |
| 生命周期 | `'a` | ❌ 无 |
| 借用检查 | 全局推导 | **仅函数体范围内检查** |
| 循环引用 | 手动 Weak | **任务结束统一释放 / 跨任务 lint / 标准库 Weak** |

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
    p.transform()
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

---

### 2. &T / &mut T（丐版借用）

**核心原则：借用只是"看一眼"或"原地改一下"，不允许占有。**

#### 2.1 三条规则

```
1. &T / &mut T 只能作为函数参数出现
2. 不能返回、不能存结构体、不能赋给局部变量、不能被闭包捕获后逃逸
3. 调用侧无需标注 &，编译器根据方法签名自动选择借用或 Move
```

**零标注。零生命周期。** 编译器只做一件事：保证借用不离开当前函数。这不需要跨函数分析——因为跨函数被"禁止"堵死了，不需要推导。

#### 2.2 基本使用

```yaoxiang
# 方法端：声明 self 类型，决定借用方式
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # 读字段
    print(self.y)
    # 函数结束，借用结束
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # 原地修改
    self.y = self.y + dy
}

# 调用端：编译器自动选择借用或 Move
p = Point(1.0, 2.0)
p.print()                          # 编译器：&p，print 结束后借用自动释放
p.shift(1.0, 1.0)                  # 编译器：&mut p，shift 结束后借用自动释放
p.print()                          # OK，p 仍然有效

# 自由函数同理
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # 读两个参数
}
d = distance(p, p2)                     # 编译器：&p, &p2
```

#### 2.3 禁止的行为

```yaoxiang
# ❌ 禁止：借用逃出函数
get_x_ref: (p: &Point) -> &Float = { p.x }      # 返回借用 → 编译错误
store_ref: (p: &Point) -> Wrapper = {             # 存进结构体 → 编译错误
    Wrapper { ref: p }
}

# ❌ 禁止：借用赋给局部变量
bad_bind: (p: &Point) -> Void = {
    q = p                         # &Point 赋给局部变量 → 编译错误
}

# ❌ 禁止：穿过任务边界
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # 借用逃逸到任务 → 编译错误
}

# ❌ 禁止：借来的不能 Move
bad_move: (p: &Point) -> Void = {
    p2 = p                        # 不是你的，没资格转移 → 编译错误
}

# ❌ 禁止：借来的不能持久持有
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # 借用不是所有权，不能 ref → 编译错误
}
```

#### 2.4 &mut 的别名保护

```yaoxiang
# ✅ 允许：多个 &T 同时活跃
read_both: (a: &Point, b: &Point) -> Float = { a.x + b.y }

# ✅ 允许：&mut 结束后可以再次借用
shift_and_read: (p: &mut Point) -> Void = {
    shift(p, 1.0, 1.0)           # &mut p 的借用在此调用中
    print_info(p)                 # 上一个借用已结束，可以再借 &
}

# ❌ 禁止：&mut 和 &T 同时活跃
# 编译器在函数体范围内做流敏感分析，保证同一时刻只有一个 &mut
```

#### 2.5 自动借用选择规则

调用端编译器按以下优先级自动选择：

```
1. 如果实参后续还有使用 → 优先借用（&T 或 &mut T，根据方法签名）
2. 如果实参后续不再使用 → Move
3. 优先匹配顺序：&T < &mut T < Move
```

```yaoxiang
# 示例：自动选择
p = Point(1.0, 2.0)
p.print()        # print 声明 &self → &p，借用结束后 p 还能用
p.shift(1.0, 1.0) # shift 声明 &mut self → &mut p
p2 = p           # Move，p 不再使用
```

---

### 3. ref 关键字（编译器自动优化）

`ref` 是跨作用域共享的唯一方式。底层是 Rc 还是 Arc，用户不需要关心。

#### 3.1 基本使用

```yaoxiang
p: Point = Point(1.0, 2.0)
shared = ref p                   # 共享，编译器自动选实现

# 跨任务共享
@block
main: () -> Void = {
    data = ref heavy_data
    spawn { use(data) }           # 编译器：跨任务 → Arc
    spawn { use(data) }           # 编译器：跨任务 → Arc
}

# 单任务共享
@block
main: () -> Void = {
    data = ref heavy_data
    use(data)                     # 编译器：不跨任务 → Rc
}
```

**用户心智模型**：`ref` = 共享持有。够了。

#### 3.2 编译器逃逸分析：Rc vs Arc

```
ref 的数据流分析：

不逃逸到其他任务 → Rc（非原子引用计数，开销低）
逃逸到其他任务   → Arc（原子引用计数，线程安全）
```

#### 3.3 环检测策略

```
任务内环 → 静默允许。
  ├── 结构化并发保证任务结束时所有资源统一释放。
  ├── ref 永远保活，语义不掺水。
  └── 用户有权在任务内构建双向强引用（例如图计算中间态）。

跨任务环 → lint（默认 warn，可配置）。
  ├── 程序行为正确，不会真泄漏（父任务结束时子任务资源全释放）。
  ├── 但跨任务强引用意味着所有权边界模糊，值得停下来重新思考。
  ├── 默认 warn 级别，编译通过但有提示。
  └── 团队可在项目配置中设为 deny，纳入 CI 质量门。
```

**Lint 级别**（类似 Rust clippy）：

| 级别 | 行为 | 场景 |
|------|------|------|
| `allow` | 不检查 | 个人项目 |
| `warn`（默认） | 编译通过，有提示 | 开发阶段 |
| `deny` | 编译失败 | 团队 CI 质量门 |
| `forbid` | 编译失败，不可覆盖 | 组织级强制规则 |

```yaoxiang
# 任务内环：静默允许，双向强引用
build_graph: () -> Void = {
    a = Node("a")
    b = Node("b")
    a.next = ref b
    b.prev = ref a                # 环。任务结束时统一释放。
}

# 跨任务环：lint（默认 warn）
@block
parent_task: () -> Void = {
    shared_a = ref a
    shared_b = ref b
    spawn {
        shared_a.child = ref shared_b   # ⚠️ warn: 跨任务循环引用
    }
}
```

**项目配置示例**：

```toml
# yaoxiang.toml
[lints]
cross-task-cycle = "deny"    # 跨任务环在 CI 上直接拒绝
```

| 环类型 | 行为 | 原因 |
|--------|------|------|
| 任务内 ref 环 | 不做检查 | 用户的权限，任务结束统一释放 |
| 跨任务 ref 环 | lint（默认 warn） | 提醒重新思考，可配置 deny |

#### 3.4 Weak：标准库提供

```yaoxiang
use std.rc.Weak

# 高级用户显式选择
a.next = ref b
b.prev = Weak.new(a.next)        # 用户显式控制哪个方向是弱的
```

**`Weak` 不是语言内置，是标准库类型。** 日常用 `ref` 就够了。需要精细控制内存的高级用户手动引入 `Weak`。

#### 3.5 借用 vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 做什么 | 看一眼/原地改 | 共享持有 |
| 范围 | 函数参数，用完即还 | 跨作用域 |
| 成本 | 零开销 | Rc 或 Arc（编译器选） |
| 逃逸 | 禁止 | 本来就是用来逃逸的 |
| 成环 | 不涉及 | 静默允许，跨任务 lint |

---

### 4. clone() —— 显式复制

```yaoxiang
p: Point = Point(1.0, 2.0)
p2 = p.clone()                   # 深拷贝
# p 和 p2 独立，互不影响
```

**何时使用**：需要保留原值且不适合 Move、不适合共享的场景。

### 5. unsafe + 裸指针（系统级编程）

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
- 用于 FFI、内存操作等系统级编程

---

### 6. 所有权梯度总览

```
  借用（零开销）         Move（零开销）      共享（按需付费）    复制
   │                      │                  │                │
  &T 看一眼            默认所有权转移     ref Rc/Arc       clone()
  &mut T 原地改        链式消费回流       编译器自动选      显式深拷贝
   │                      │                  │                │
  函数参数               作用域内           跨作用域         任何时候
  禁止逃逸               T -> T 回流        ref 跨任务 → Arc  独立副本
  自动选择               T -> Void 消费     ref 不跨任务 → Rc
                                            任务内环静默
                                            跨任务环 lint
                                            标准库 Weak 逃生
```

---

## 综合示例

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,

    # &T：只读
    print: (self: &Point) -> Void = {
        print(self.x)
        print(self.y)
    }

    # &mut T：原地修改
    shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
        self.x = self.x + dx
        self.y = self.y + dy
    }

    # Move → Move：消费回流
    scale: (self: Point, f: Float) -> Point = {
        self.x = self.x * f
        self.y = self.y * f
        self                            # 拿走，改，还给你
    }
}

# 综合使用
p = Point(1.0, 2.0)
p.print()                           # &p，借用
p.shift(1.0, 1.0)                   # &mut p，借用
p = p.scale(2.0)                    # Move → 回流
shared = ref p                      # ref 共享
spawn { use(shared) }

# clone 独立副本
backup = p.clone()

# 任务内环：静默允许
a = Node("a")
b = Node("b")
a.next = ref b
b.prev = ref a                      # 环，任务结束时统一释放

# unsafe 系统级
unsafe {
    ptr: *Point = &p
    (*ptr).x = 0.0
}
```

---

## 类型系统约束

### Send / Sync

| 类型 | Send | Sync | 说明 |
|------|------|------|------|
| 值类型 | ✅ | ✅ | Int, Float, Point... |
| `ref T` | ✅ | ✅ | 编译器自动选 Rc/Arc |
| `&T` / `&mut T` | ❌ | ❌ | 借用不能跨线程（不允许逃逸） |
| `*T` | ❌ | ❌ | 裸指针，单线程 |

---

## 性能分析

| 操作 | 成本 | 说明 |
|------|------|------|
| Move | 零 | 指针移动 |
| `&T` / `&mut T` | 零 | 编译期检查，零运行时开销 |
| `ref`（不跨任务）| 低 | 编译为 Rc，非原子操作 |
| `ref`（跨任务）| 中 | 编译为 Arc，原子操作 |
| `clone()` | 视类型 | 小对象快，大对象慢 |
| `unsafe + *T` | 零 | 直接内存操作 |

### 对比

| 语言 | 共享机制 | 内存管理 | 循环处理 | 复杂度 |
|------|----------|----------|----------|--------|
| Rust | Arc / Mutex + 借用检查 | 编译期检查 | 手动 Weak | 高 |
| Go | chan / pointer | GC | GC | 低 |
| C++ | shared_ptr | RAII | weak_ptr | 中 |
| **YaoXiang** | **ref + 丐版借用** | **RAII** | **任务边界释放 / 跨任务 lint / 标准库 Weak** | **低** |

---

## 权衡

### 优点

1. **简单**：无生命周期，无全局借用检查器。`&T`/`&mut T` 三条规则
2. **编译器智能**：ref 自动选 Rc/Arc，调用侧自动选择借用
3. **确定性**：ref 就是保活，不会悄悄变弱引用
4. **高性能**：Move 零拷贝，borrow 零开销
5. **灵活**：`unsafe + *T` 支持系统级编程

### 缺点

1. **ref 运行时开销**：原子操作有成本（但这是共享的必然代价）
2. **unsafe 风险**：用户必须保证正确性
3. **跨任务环是 lint 不是编译错误**：不像 Rust 那样编译报错，默认 warn，需要团队配置 deny 才能作为质量门

---

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| GC | 有运行时开销，无法预测暂停 |
| Rust 借用检查器 | 需生命周期 `'a`，学习曲线陡 |
| 纯 Move | 无法处理并发共享 |
| 无裸指针 | 无法系统级编程 |
| 暴露 Rc/Arc 给用户 | 把实现细节甩给用户，增加认知负担 |

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| **默认值** | Move（零拷贝） | 高性能，零开销 | 2025-01-15 |
| **共享机制** | `ref` 关键字，编译器自动优化 | 用户简单，编译器负责 | 2025-01-15 |
| **借用** | `&T`/`&mut T`，只做参数，禁止逃逸 | 不需要生命周期，简单安全 | 2025-01-15 |
| **复制** | `clone()` | 显式语义 | 2025-01-15 |
| **系统级** | `*T` + `unsafe` | 支持系统编程 | 2025-01-15 |
| **生命周期** | 不实现 | 借用的简单规则不需要生命周期 | 2025-01-15 |
| **Rc/Arc** | 编译器自动选择，用户不可见 | 降低认知负担 | 2025-01-15 |
| **循环引用** | 任务内不做检查，跨任务 lint（默认 warn） | 结构化并发天然保证，lint 可配 deny | 2025-01-16 |
| **Weak** | 标准库提供 | 高级用户显式选择 | 2025-01-16 |
| **消费分析** | 删除 | 迷你借用检查器，不需要 | 2026-05-11 |
| **所有权回流** | 删除 | `(T) -> T` 签名就是文档 | 2026-05-11 |
| **空状态重用** | 删除（作为特性） | Move 后重新赋值是自然行为 | 2026-05-11 |
| **逆函数/部分消费/字段三层可变性** | 删除 | 过度设计 | 2026-05-11 |

### 版本历史

| 版本 | 主要变更 | 日期 |
|------|----------|------|
| v1 | 初稿：基于 Rust 所有权模型 | 2025-01-08 |
| v4 | 默认 Move + 显式 ref | 2025-01-15 |
| v5 | 结构化并发 + 循环引用处理 | 2025-01-16 |
| v6 | 新增空状态重用、所有权回流 | 2025-02-04 |
| v7 | 新增消费分析、逆函数、字段级可变性 | 2025-02-05 |
| **v8** | **删除过度设计，新增丐版借用 &T/&mut T** | **2026-05-11** |

### 待决议题

| 议题 | 说明 | 状态 |
|------|------|------|
| Drop 语法 | 是否需要显式 `drop()` 函数 | 待讨论 |
| 逃逸分析算法 | ref 的跨任务检测实现 | 待讨论 |
| 交叉借用检查 | 调用点就地检查 + 函数体流敏感，见下文 | ✅ 已解决 |

### 交叉借用检查：调用点就地 + 流敏感

**分析范围**：仅函数体内。因为 `&T`/`&mut T` 只能做参数，每条调用链在进入下一层函数时，上一层函数的借用随调用结束自动释放。不需要跨函数分析。

**层 1：调用点检查**——每个实参不能同时出现在 `&mut` 位置和其他借用位置：

```yaoxiang
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)    # ❌ p 同时作为 &mut 和 &，编译器拒绝
```

**层 2：函数体流敏感**——`&mut` 传给调用后，调用返回即释放，后续可再借：

```yaoxiang
process_twice: (p: &mut Point) -> Void = {
    shift(p, 1.0, 1.0)    # &mut 传给 shift，shift 返回后借用结束
    print_info(p)          # 重新借 &p，不冲突
}
```

**不需要的东西**：跨函数生命周期追踪、全局别名分析、借用图约束求解、NLL。因为借用不逃逸、不存变量、不返回——离开调用点即失效。

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
