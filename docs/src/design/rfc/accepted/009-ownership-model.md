---
title: "RFC-009：所有权模型设计"
---

# RFC-009: 所有权模型设计

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-08
> **最后更新**: 2026-05-29（借用令牌系统替代丐版借用，统一类型系统）

## 摘要

本文档定义 YaoXiang 编程语言的**所有权模型（Ownership Model）**。

**核心设计——五个概念，一个梯度**：

```
看一眼/原地改     拿走           共享持有         复制一份        系统级
    │              │              │              │              │
   &T            Move           ref          clone()        unsafe
  &mut T         零拷贝        编译器自动      显式深拷贝      *T
  零大小令牌       默认          选Rc/Arc                   用户负责
  类型属性自然                              
  推导权限                              
```

- **Move（默认）**：赋值/传参/返回 = 所有权转移，零拷贝，RAII 自动释放
- **`&T` / `&mut T`（借用令牌）**：零大小的编译期令牌类型。`&T` 可复制（共享只读），`&mut T` 线性（独占可变）。权限由类型属性自然推导，无需特殊规则。可返回、可存结构体、可被闭包捕获。
- **`ref` 关键字**：跨作用域共享。编译器自动选 Rc（不跨任务）还是 Arc（跨任务）
- **`clone()`**：显式深拷贝
- **`unsafe` + `*T`**：裸指针，系统级逃生舱

**消除的复杂性**：
- ❌ 无生命周期 `'a`
- ❌ 无借用检查器（令牌的类型属性——Copy/Linear——替代了专用 borrow checker）
- ❌ 无 GC
- ❌ 无"禁止逃逸"等特殊规则（令牌是普通类型，作用域由类型系统统一处理）
- ❌ 用户不需要知道 Rc/Arc 的区别（编译器自动选）

> **编程负担**：`&T` 可复制，`&mut T` 不可复制——两条类型属性，零条特殊规则，编译器全自动。
> **性能保证**：Move 零开销，令牌零开销（零大小类型，编译后消失），ref 按需付费，无 GC 暂停。

## 动机

### 为什么需要所有权模型？

| 语言 | 内存管理 | 问题 |
|------|----------|------|
| C/C++ | 手动管理 | 内存泄漏、野指针、双重释放 |
| Java/Python | GC | 延迟波动、内存开销、无法预测的暂停 |
| Rust | 所有权 + 借用检查 | 生命周期 `'a` 学习曲线陡峭 |
| **YaoXiang** | **Move + Token + ref** | **简单、确定、无 GC** |

### 设计目标

```yaoxiang
# 1. 默认 Move（零拷贝）
p = Point(1.0, 2.0)
p2 = p                         # Move，p 不可再读

# 2. &T / &mut T 借用令牌（零开销，类型属性自然推导权限）
print_info(p2)                 # 编译器自动创建 &Point 令牌，用完即释放
shift(p2, 1.0, 1.0)           # 编译器自动创建 &mut Point 令牌

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
| 借用 | `&T`/`&mut T`，可返回，需生命周期 | **`&T`/`&mut T` 零大小令牌，Copy/Linear 类型属性自然推导** |
| 共享机制 | `Arc::new()` + 手动 Weak | **`ref` 关键字（编译器自动选 Rc/Arc）** |
| 复制 | `clone()` | `clone()` |
| 裸指针 | `*T` | `*T` |
| 生命周期 | `'a` | ❌ 无 |
| 借用检查 | 全局推导 | **类型检查器流敏感活性分析（令牌状态追踪）** |
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

### 2. &T / &mut T（借用令牌）

**核心原则：`&T` 和 `&mut T` 是零大小的编译期令牌类型。它们不是"引用"，而是"访问权限的类型级证明"。**

#### 2.1 两条类型属性

```
&T      →  零大小，可复制（Copy），授予只读权限
&mut T  →  零大小，线性（非Copy），授予独占读写权限
```

**这不是需要记忆的"规则"——这是类型系统的基本属性。** `Copy` 类型可以自由复制（多个 `&T` 共存），线性类型不能复制（`&mut T` 天然唯一）。没有"借用检查器"——只有类型检查器在做它一直在做的事。

#### 2.2 基本使用

```yaoxiang
# 方法端：声明参数类型，决定需要的权限
Point.print: (self: &Point) -> Void = {
    print(self.x)                  # &Point 令牌授予读权限
    print(self.y)
}

Point.shift: (self: &mut Point, dx: Float, dy: Float) -> Void = {
    self.x = self.x + dx           # &mut Point 令牌授予写权限
    self.y = self.y + dy
}

# 调用端：编译器自动选择借用或 Move
p = Point(1.0, 2.0)
p.print()                          # 编译器自动创建 &Point 令牌
p.shift(1.0, 1.0)                  # 编译器自动创建 &mut Point 令牌
p.print()                          # OK，上一个令牌已随 shift 调用结束而释放

# 自由函数同理
distance: (a: &Point, b: &Point) -> Float = {
    sqrt((a.x - b.x)**2 + (a.y - b.y)**2)  # 两个 &Point 令牌共存——Copy 类型
}
d = distance(p, p2)
```

#### 2.3 为什么不需要"禁止逃逸"

RFC-009 v8 对 `&T`/`&mut T` 施加了三条特殊规则——只能做参数、不能返回、不能存结构体。这是在给"借用"概念打补丁。

令牌系统不需要这些规则。令牌是**普通类型**，遵循和所有其他类型一样的作用域规则。

**返回引用——自然支持**：

```yaoxiang
# ✅ 令牌随返回值一起传播
Point.get_x: (self: &Point) -> (&Float, &Point) = {
    return (&self.x, self)  # 子令牌和父令牌一起返回
}

# 使用
p = Point(1.0, 2.0)
(px_ref, p) = p.get_x()    # 令牌返回给调用者
print(px_ref)               # OK，令牌仍在作用域
```

**存结构体——自然支持**：

```yaoxiang
# ✅ 结构体携带令牌作为字段
Window: Type = {
    target: Point,
    view: &Point,      # 令牌字段——持有对 target 的只读视图
}

# view 的令牌从 target 派生，Window 持有两者的所有权
# 只要 Window 存在，view 令牌就有效
```

**闭包捕获——自然支持**：

```yaoxiang
# ✅ 闭包捕获令牌，就像捕获任何值
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    # 闭包捕获 threshold 的 &Float 令牌（Copy 类型，自由复制到闭包中）
    items.filter(|p| p.x > threshold)
}

# 这是 RFC-009 v8 做不到的——v8 禁止闭包捕获借用
```

**跨任务——令牌不能穿线**：

```yaoxiang
# ❌ 令牌不能跨任务边界
bad_task: (p: &Point) -> Void = {
    spawn { print(p.x) }          # ❌ 编译错误：令牌类型未实现 Send
}

# 这不是特殊规则——&T 令牌类型未实现 Send trait
# 如果需要跨任务共享，请使用 ref
```

**令牌不能 ref**：

```yaoxiang
# ❌ 令牌是权限证明，不是所有权
bad_ref: (p: &Point) -> Void = {
    shared = ref p                # ❌ 编译错误：&T 不是可拥有类型
}
```

#### 2.4 令牌的生命周期

令牌的生命周期由**普通的作用域规则**决定，不需要生命周期参数：

- 函数参数中的令牌：在调用期间存活，调用结束后释放
- 返回的令牌：所有权转移给调用者
- 存储在结构体中的令牌：随结构体一起存活
- 闭包捕获的令牌：随闭包一起存活

编译器不需要 `'a` 标注，因为令牌是**值**，值的生命周期由所有权系统（Move/RAII）统一管理。**将借用问题降维为所有权问题。**

#### 2.5 冻结机制

`&mut T` 令牌可以临时"冻结"以产生 `&T` 令牌：

```yaoxiang
modify_and_read: (p: &mut Point) -> Void = {
    p.x = 10.0                      # 使用 &mut Point 修改
    
    # 冻结 &mut，获取只读视图
    view: &Point = freeze(p)         # p 在此处被冻结
    print(view.x)                   # 通过 &Point 读取
    print(view.y)
    # view 离开作用域，冻结解除
    
    p.y = 20.0                      # &mut Point 恢复可用
}
```

`freeze` 的语义：
- 接受 `&mut T`，返回 `&T`
- 在 `&T` 存活期间，原 `&mut T` 不可用
- `&T` 离开作用域后，`&mut T` 自动恢复
- 这是**流敏感分析**——编译器在函数体内追踪令牌状态

#### 2.6 令牌冲突检测

替代 RFC-009 v8 的"交叉借用检查"。原理更简单——对令牌值的**流敏感活性分析**：

```yaoxiang
# ❌ &mut 和派生的 &T 不能同时活跃
bad_alias: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)        # p 被冻结
    p.x = 10.0                      # ❌ 编译错误：WriteToken 在冻结状态
    print(view.x)                   
}

# ✅ 冻结解除后可继续使用 &mut
good_seq: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)        # p 被冻结
    print(view.x)                   # 使用 &T
    # view 离开作用域
    p.x = 10.0                      # ✅ WriteToken 已恢复
}
```

**检测方式**：这不是专门的"借用检查器"——这是对令牌值的**流敏感活性分析**。编译器在函数体内追踪每个令牌的状态（活跃/冻结/已移动），与追踪任何线性类型值的方式完全相同。

#### 2.7 编译器内部：品牌机制

用户从不接触品牌。编译器在内部为每个令牌分配编译期唯一标识：

```
用户看到的         编译器内部表示
────────────────────────────────────────
&Point         →  ReadToken(Point, #N)    // #N 是编译期唯一整数
&mut Point     →  WriteToken(Point, #M)   // #M 是编译期唯一整数
```

品牌的用途：
- **防伪造**：令牌只能从所有者胶囊或 freeze 操作获得，不能凭空构造
- **关联追踪**：从 `&Point` 派生 `&Float`（字段访问）时，`&Float` 携带派生品牌（`#N.field_x`），编译器可追踪到父令牌
- **冲突检测**：同源 `WriteToken` 和派生 `ReadToken` 不能同时活跃

品牌在单态化和内联后完全消失，生成的机器码中不存在。**零运行时开销。**

#### 2.8 自动借用选择规则

调用端编译器按以下优先级自动选择：

```
1. 如果实参后续还有使用 → 优先创建令牌（&T 或 &mut T，根据方法签名）
2. 如果实参后续不再使用 → Move
3. 优先匹配顺序：&T < &mut T < Move
```

```yaoxiang
# 示例：自动选择
p = Point(1.0, 2.0)
p.print()        # print 声明 &self → 编译器创建 &Point 令牌
p.shift(1.0, 1.0) # shift 声明 &mut self → 编译器创建 &mut Point 令牌
p2 = p           # Move，p 不再使用
```

#### 2.9 与 RFC-009 v8 丐版借用的对比

| 特性 | 丐版借用 (v8) | 借用令牌 (v9) |
|------|--------------|--------------|
| 返回引用 | ❌ 硬编码禁止 | ✅ 令牌随返回值传播 |
| 存结构体 | ❌ 硬编码禁止 | ✅ 令牌作为结构体字段 |
| 闭包捕获 | ❌ 硬编码禁止 | ✅ 闭包捕获令牌值 |
| 特殊规则 | 3条（只能做参数/不能返回/不能存） | 0条——类型属性自然推导 |
| 借用检查 | 专用交叉借用检查 | 类型检查器流敏感活性分析 |
| 生命周期标注 | 不需要 | 不需要 |
| 运行时开销 | 零 | 零（零大小类型，编译后消失） |
| 错误信息 | "借用不能逃逸" | "WriteToken(#3)已被冻结"（常规类型错误） |
| 用户心智模型 | 理解"借用"的特殊地位 | `&T` 可复制，`&mut T` 不可复制 |

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

#### 3.5 借用令牌 vs ref

| | `&T` / `&mut T` | `ref` |
|------|------|------|
| 做什么 | 看一眼/原地改 | 共享持有 |
| 范围 | 随令牌值的作用域 | 跨作用域 |
| 成本 | 零开销（零大小类型） | Rc 或 Arc（编译器选） |
| 逃逸 | 可（令牌随返回值/结构体/闭包传播） | 本来就是用来逃逸的 |
| 跨任务 | 不可（令牌未实现 Send） | 可（编译器自动选 Arc） |
| 成环 | 不涉及 | 任务内静默允许，跨任务 lint |

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
  借用令牌（零开销）     Move（零开销）      共享（按需付费）    复制
   │                      │                  │                │
  &T 可复制令牌        默认所有权转移     ref Rc/Arc       clone()
  &mut T 线性令牌      链式消费回流       编译器自动选      显式深拷贝
   │                      │                  │                │
  令牌值作用域           作用域内           跨作用域         任何时候
  可返回/存结构体        T -> T 回流        ref 跨任务 → Arc  独立副本
  可被闭包捕获           T -> Void 消费     ref 不跨任务 → Rc
  零大小编译后消失                          任务内环静默
                                            跨任务环 lint
                                            标准库 Weak 逃生
```

---

## 综合示例

```yaoxiang
Point: Type = {
    x: Float,
    y: Float,

    # &T：只读令牌
    print: (self: &Point) -> Void = {
        print(self.x)
        print(self.y)
    }

    # &mut T：可变令牌
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

    # 返回引用：令牌随返回值传播
    get_x: (self: &Point) -> (&Float, &Point) = {
        return (&self.x, self)
    }
}

# 闭包捕获令牌（v8 做不到的能力）
filter_by_threshold: (items: List(Point), threshold: &Float) -> List(Point) = {
    items.filter(|p| p.x > threshold)
}

# 综合使用
p = Point(1.0, 2.0)
p.print()                           # &Point 令牌
p.shift(1.0, 1.0)                   # &mut Point 令牌
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
| `&T` / `&mut T` | ❌ | ❌ | 令牌未实现 Send/Sync，不能跨任务 |
| `*T` | ❌ | ❌ | 裸指针，单线程 |

---

## 性能分析

| 操作 | 成本 | 说明 |
|------|------|------|
| Move | 零 | 指针移动 |
| `&T` / `&mut T` | 零 | 零大小类型，编译后消失，零运行时开销 |
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
| **YaoXiang** | **ref + 借用令牌** | **RAII** | **任务边界释放 / 跨任务 lint / 标准库 Weak** | **低** |

---

## 权衡

### 优点

1. **统一**：`&T`/`&mut T` 是普通类型，不是特殊语言特性。与 RFC-010 的 `name: type = value` 完全一致
2. **简单**：无生命周期，无全局借用检查器。`&T` 可复制，`&mut T` 不可复制——两条类型属性
3. **强大**：可返回引用、存结构体、闭包捕获——表达能力与 Rust 同级
4. **编译器智能**：ref 自动选 Rc/Arc，调用侧自动选择借用
5. **确定性**：ref 就是保活，不会悄悄变弱引用
6. **高性能**：Move 零拷贝，令牌零开销（零大小类型，编译后消失）
7. **灵活**：`unsafe + *T` 支持系统级编程

### 缺点

1. **泛型品牌参数传染**：令牌携带品牌标识，返回引用的函数签名中会体现额外的泛型参数
2. **ref 运行时开销**：原子操作有成本（但这是共享的必然代价）
3. **unsafe 风险**：用户必须保证正确性
4. **跨任务环是 lint 不是编译错误**：不像 Rust 那样编译报错，默认 warn，需要团队配置 deny 才能作为质量门

---

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| GC | 有运行时开销，无法预测暂停 |
| Rust 借用检查器 | 需生命周期 `'a`，学习曲线陡 |
| 纯 Move | 无法处理并发共享 |
| 无裸指针 | 无法系统级编程 |
| 暴露 Rc/Arc 给用户 | 把实现细节甩给用户，增加认知负担 |
| 丐版借用（v8） | 禁止逃逸的策略牺牲了闭包捕获、返回引用等关键表达能力 |

---

## 设计决策记录

| 决策 | 决定 | 原因 | 日期 |
|------|------|------|------|
| **默认值** | Move（零拷贝） | 高性能，零开销 | 2025-01-15 |
| **共享机制** | `ref` 关键字，编译器自动优化 | 用户简单，编译器负责 | 2025-01-15 |
| **借用** | `&T`/`&mut T` 作为零大小令牌类型 | 类型属性（Copy/Linear）自然推导权限，统一类型系统 | 2025-01-15 |
| **借用令牌** | 替代丐版借用，`&T` Copy，`&mut T` Linear | 消除"禁止逃逸"等特殊规则，支持闭包捕获/返回引用/存结构体 | 2026-05-29 |
| **复制** | `clone()` | 显式语义 | 2025-01-15 |
| **系统级** | `*T` + `unsafe` | 支持系统编程 | 2025-01-15 |
| **生命周期** | 不实现 | 令牌是值，生命周期由 Move/RAII 统一管理，将借用降维为所有权问题 | 2025-01-15 |
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
| **v9** | **借用令牌系统替代丐版借用，统一类型系统** | **2026-05-29** |

### 待决议题

| 议题 | 说明 | 状态 |
|------|------|------|
| Drop 语法 | 是否需要显式 `drop()` 函数 | 待讨论 |
| 逃逸分析算法 | ref 的跨任务检测实现 | 待讨论 |
| 令牌冲突检测 | 流敏感活性分析，见下文 | ✅ 已解决 |

### 令牌冲突检测：流敏感活性分析

**分析范围**：仅函数体内。对令牌值做流敏感活性分析，追踪每个令牌的状态（活跃/冻结/已移动）。

**层 1：调用点检查**——同一实参不能同时创建 `&mut` 令牌和其他令牌：

```yaoxiang
alias_bad: (a: &mut Point, b: &Point) -> Void = { ... }
p = Point(1.0, 2.0)
alias_bad(p, p)    # ❌ p 同时派生 &mut 和 & 令牌，编译器拒绝
```

**层 2：函数体流敏感**——`&mut` 令牌传给调用后，调用返回即释放，后续可再创建令牌：

```yaoxiang
process_twice: (p: &mut Point) -> Void = {
    shift(p, 1.0, 1.0)    # &mut 令牌传给 shift，shift 返回后令牌释放
    print_info(p)          # 重新创建 &Point 令牌，不冲突
}
```

**层 3：冻结状态追踪**——`freeze` 产生的 `&T` 令牌活跃期间，原 `&mut` 令牌不可用：

```yaoxiang
frozen: (p: &mut Point) -> Void = {
    view: &Point = freeze(p)    # p 进入冻结状态
    print(view.x)
    p.x = 10.0                  # ❌ 编译错误：WriteToken 处于冻结状态
}
```

**不需要的东西**：跨函数生命周期追踪、全局别名分析、借用图约束求解、NLL、`'a` 标注。因为令牌是值，值的活性分析由类型检查器统一处理——与追踪任何线性类型值完全相同。

---

## 参考文献

### YaoXiang 官方文档

- [语言规范](../language-spec.md)
- [设计宣言](../manifesto.md)
- [RFC-001 并作模型](./001-concurrent-model-error-handling.md)
- [RFC-010 统一类型语法](./010-unified-type-syntax.md)
- [RFC-011 泛型系统设计](./011-generic-type-system.md)
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
