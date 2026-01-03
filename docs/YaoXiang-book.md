# YaoXiang（爻象）编程语言指南

> 版本：v1.0.0
> 状态：草稿
> 作者：晨煦
> 日期：2024-12-31

---

## 目录

1. [语言概述](#一语言概述)
2. [核心特性](#二核心特性)
3. [类型系统](#三类型系统)
4. [内存管理](#四内存管理)
5. [异步编程与并发](#五异步编程与并发)
6. [模块系统](#六模块系统)
7. [方法绑定与柯里化](#七方法绑定与柯里化)
8. [AI友好设计](#八ai友好设计)
9. [快速入门](#九快速入门)

---

## 一、语言概述

### 1.1 什么是 YaoXiang？

YaoXiang（爻象）是一门实验性的通用编程语言，其设计理念源于《易经》中「爻」与「象」的核心概念。「爻」是组成卦象的基本符号，象征着阴阳变化；「象」是事物本质的外在表现，代表万象万物。

YaoXiang 将这一哲学思想融入编程语言的类型系统之中，提出**「一切皆类型」**的核心理念。在 YaoXiang 的世界观中：

- **值**是类型的实例
- **类型**本身也是类型的实例（元类型）
- **函数**是输入类型到输出类型的映射
- **模块**是类型的命名空间组合

### 1.2 设计目标

YaoXiang 的设计目标可以概括为以下几个方面：

| 目标 | 说明 |
|------|------|
| **统一的类型抽象** | 类型是最高层的抽象单元，简化语言语义 |
| **自然的编程体验** | Python 风格语法，强调可读性 |
| **安全的内存管理** | Rust 风格所有权模型，无 GC |
| **无感的异步编程** | 自动管理异步，无需显式 await |
| **完整的类型反射** | 运行时类型信息完全可用 |
| **AI 友好的语法** | 严格结构化，易于 AI 处理 |

### 1.3 语言定位

| 维度 | 定位 |
|------|------|
| 范式 | 多范式（函数式 + 命令式 + 面向对象） |
| 类型系统 | 依赖类型 + 参数化多态 |
| 内存管理 | 所有权 + RAII（无 GC） |
| 编译模型 | AOT 编译（可选 JIT） |
| 目标场景 | 系统编程、应用开发、AI 辅助编程 |

### 1.4 代码示例

```yaoxiang
# 自动类型推断
x = 42                           # 推断为 Int
name = "YaoXiang"                # 推断为 String

# 默认不可变
x = 10
x = 20                           # 编译错误！

# 函数定义
add(Int, Int) -> Int = (a, b) => a + b

# 统一类型语法：构造器即类型
type Point = Point(x: Float, y: Float)
type Result[T, E] = ok(T) | err(E)

# 无感异步
fetch_data(String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main() -> Void = () => {
    # 值构造：与函数调用完全相同
    p = Point(3.0, 4.0)
    r = ok("success")

    data = fetch_data("https://api.example.com")
    # 自动等待，无需 await
    print(data.name)
}
```

---

## 二、核心特性

### 2.1 一切皆类型

YaoXiang 的核心设计哲学是**一切皆类型**。这意味着在 YaoXiang 中：

1. **值是类型的实例**：`42` 是 `Int` 类型的实例
2. **类型是类型的实例**：`Int` 是 `type` 元类型的实例
3. **函数是类型映射**：`fn add(Int, Int) -> Int` 是一个函数类型
4. **模块是类型组合**：模块是包含函数和类型的命名空间

```yaoxiang
# 值是类型的实例
x: Int = 42

# 类型是类型的实例
MyList: type = List(Int)

# 函数是类型之间的映射
add(Int, Int) -> Int = (a, b) => a + b

# 模块是类型的组合（使用文件作为模块）
# Math.yx
pi: Float = 3.14159
sqrt(Float) -> Float = (x) => { ... }
```

### 2.2 数学抽象

YaoXiang 的类型系统基于类型论和范畴论，提供了：

- **依赖类型**：类型可以依赖于值
- **泛型编程**：类型参数化
- **类型组合**：联合类型、交集类型

```yaoxiang
# 依赖类型：固定长度向量
type Vector[T, n: Nat] = vector(T, n)

# 类型联合
type Number = Int | Float

# 类型交集
type Printable = printable(fn() -> String)
type Serializable = serializable(fn() -> String)
type Versatile = Printable & Serializable
```

### 2.3 零成本抽象

YaoXiang 保证零成本抽象，即高层次的抽象不会带来运行时的性能开销：

- **单态化**：泛型函数在编译时展开为具体版本
- **内联优化**：简单函数自动内联
- **栈分配**：小对象默认栈分配

```yaoxiang
# 泛型展开（单态化）
identity<T>(T) -> T = (x) => x

# 使用
int_val = identity(42)      # 展开为 identity(Int) -> Int
str_val = identity("hello") # 展开为 identity(String) -> String

# 编译后无额外开销
```

### 2.4 自然语法

YaoXiang 采用 Python 风格的语法，追求可读性和自然语言感：

```yaoxiang
# 自动类型推断
x = 42
name = "YaoXiang"

# 简洁的函数定义
greet(String) -> String = (name) => "Hello, " + name

# 模式匹配
classify(Int) -> String = (n) => {
    match n {
        0 -> "zero"
        1 -> "one"
        _ if n < 0 -> "negative"
        _ -> "many"
    }
}
```

---

## 三、类型系统

### 3.1 类型层次

YaoXiang 的类型系统是层次化的：

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang 类型层次                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  type (元类型)                                               │
│    │                                                        │
│    ├── 原类型 (Primitive Types)                             │
│    │   ├── Void, Bool                                       │
│    │   ├── Int, Uint, Float                                 │
│    │   ├── Char, String, Bytes                              │
│    │                                                        │
│    ├── 构造器类型 (Constructor Types)                        │
│    │   ├── Name(args)              # 单构造器（结构体）      │
│    │   ├── A(T) | B(U)             # 多构造器（联合/枚举）   │
│    │   ├── A | B | C               # 零参构造器（枚举）      │
│    │   ├── tuple (T1, T2, ...)                            │
│    │   ├── list [T], dict [K->V]                           │
│    │                                                        │
│    ├── 函数类型 (Function Types)                            │
│    │   fn (T1, T2, ...) -> R                               │
│    │                                                        │
│    ├── 泛型类型 (Generic Types)                             │
│    │   List[T], Map[K, V], etc.                            │
│    │                                                        │
│    ├── 依赖类型 (Dependent Types)                           │
│    │   type [n: Nat] -> type                               │
│    │                                                        │
│    └── 模块类型 (Module Types)                              │
│        文件即模块                                            │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 类型定义

```yaoxiang
# 统一类型语法：只有构造器，没有 enum/struct/union 关键字
# 规则：用 | 分隔的都是构造器，构造器名(参数) 就是类型

# === 零参数构造器（枚举风格）===
type Color = red | green | blue              # 等价于 red() | green() | blue()

# === 多参数构造器（结构体风格）===
type Point = Point(x: Float, y: Float)       # 构造器就是类型

# === 泛型构造器 ===
type Result[T, E] = ok(T) | err(E)           # 泛型联合

# === 混合构造器 ===
type Shape = circle(Float) | rect(Float, Float)

# === 值构造（与函数调用完全相同）===
c: Color = green                              # 等价于 green()
p: Point = Point(1.0, 2.0)
r: Result[Int, String] = ok(42)
s: Shape = circle(5.0)
```

### 3.3 类型操作

```yaoxiang
# 类型作为值
MyInt = Int
MyList = List(Int)

# 类型反射（构造器模式匹配）
describe_type(type) -> String = (t) => {
    match t {
        Point(x, y) -> "Point with x=" + x + ", y=" + y
        red -> "Red color"
        ok(value) -> "Ok value"
        _ -> "Other type"
    }
}
```

### 3.4 类型推断

YaoXiang 具有强大的类型推断能力：

```yaoxiang
# 基本推断
x = 42                    # 推断为 Int
y = 3.14                  # 推断为 Float
z = "hello"               # 推断为 String

# 函数返回值推断
add(Int, Int) -> Int = (a, b) => a + b

# 泛型推断
first<T>(List[T]) -> Option[T] = (list) => {
    if list.length > 0 { some(list[0]) } else { none }
}
```

---

## 四、内存管理

### 4.1 所有权原则

YaoXiang 采用 Rust 风格的所有权模型：

```yaoxiang
# 默认不可变引用
process(ref Data) -> Void = (data) => {
    # data 是只读的
    # 不能修改 data 的字段
    # 不能转移 data 的所有权
}

# 可变引用
modify(mut Data) -> Void = (data) => {
    # 可以修改 data 的字段
    # 不能有其他活跃的引用
}

# 转移所有权
consume(Data) -> Void = (data) => {
    # data 的所有权转移进来
    # 函数结束后 data 被销毁
}

# 借用返回
borrow_field(ref Data) -> ref Field = (data) => ref data.field
```

### 4.2 生命周期

```yaoxiang
# 显式生命周期标注（复杂情况）
longest<'a>(&'a str, &'a str) -> &'a str = (s1, s2) => {
    if s1.length > s2.length { s1 } else { s2 }
}

# 自动生命周期推断
first<T>(ref List[T]) -> ref T = (list) => ref list[0]
```

### 4.3 智能指针

```yaoxiang
# Box - 堆分配
heap_data: Box[List[Int]] = Box.new([1, 2, 3])

# Rc - 引用计数
shared: Rc[Data] = Rc.new(data)

# Arc - 原子引用计数（线程安全）
thread_safe: Arc[Data] = Arc.new(data)
```

### 4.4 RAII

```yaoxiang
# RAII 自动释放
with_file(String) -> String = (path) => {
    file = File.open(path)  # 自动打开
    content = file.read_all()
    # 函数结束，file 自动关闭
    content
}
```

---

## 五、异步编程与并发

> 「万物并作，吾以观复。」——《易·复卦》
>
> YaoXiang 采用**并作模型**，一种基于**惰性求值**的无感异步并发范式。其核心设计理念是：**让开发者以同步、顺序的思维描述逻辑，而语言运行时令其中的计算单元如万物并作般自动、高效地并发执行，并在最终统一协同**。

> 详见 [《并作模型白皮书》](YaoXiang-async-whitepaper.md) 和 [异步实现方案](YaoXiang-async-implementation.md)。

### 5.1 并作模型核心概念

#### 5.1.1 并作图：万物并作的舞台

所有程序在编译时被转化为一个**有向无环计算图(DAG)**，称为**并作图**。节点代表表达式计算，边代表数据依赖。此图是惰性的，即节点仅在其输出被**真正需要**时才被求值。

```yaoxiang
# 编译器自动构建并作图
fetch_user() -> User spawn = (id) => { ... }
fetch_posts(User) -> Posts spawn = (user) => { ... }

main() -> Void = () => {
    user = fetch_user(1)     # 节点 A (Async[User])
    posts = fetch_posts(user) # 节点 B (Async[Posts])，依赖 A

    # 节点 C 需要 A 和 B 的结果
    print(posts.title)       # 自动等待：先确保 A 和 B 完成
}
```

#### 5.1.2 并作值：Async[T]

任何标记为 `spawn fn` 的函数调用会立即返回一个 `Async[T]` 类型的值，称为**并作值**。这是一个轻量级代理，它并非实际结果，而代表一个**正在并作中的未来值**。

**核心特性**：
- **类型透明**：`Async[T]` 在类型系统中是 `T` 的子类型，可在任何期望 `T` 的上下文中使用
- **自动等待**：当程序执行到必须使用 `T` 类型具体值的操作时，运行时自动挂起当前任务，等待计算完成
- **零传染**：异步代码与同步代码在语法和类型签名上无区别

```yaoxiang
# 并作值使用示例
fetch_data(String) -> JSON spawn = (url) => { ... }

main() -> Void = () => {
    data = fetch_data("url")  # Async[JSON]

    # Async[JSON] 可直接当作 JSON 使用
    # 自动等待在字段访问时发生
    print(data.name)          # 等价于 data.await().name
}
```

### 5.2 并作语法体系

`spawn` 关键字具有三重语义，是连接同步思维与异步实现的唯一桥梁：

| 官方术语 | 语法形式 | 语义 | 运行时行为 |
|----------|----------|------|------------|
| **并作函数** | `spawn fn` | 定义可参与并作执行的计算单元 | 其调用返回 `Async[T]` |
| **并作块** | `spawn { a(), b() }` | 显式声明的并发疆域 | 块内任务强制并行执行 |
| **并作循环** | `spawn for x in xs { ... }` | 数据并行范式 | 循环体在所有元素上并作执行 |

#### 5.2.1 并作函数

```yaoxiang
# 使用 spawn 标记并作函数
# 语法与普通函数完全一致，无额外负担

fetch_api(String) -> JSON spawn = (url) => {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

# 嵌套并作调用
process_user(Int) -> Report spawn = (user_id) => {
    user = fetch_user(user_id)     # Async[User]
    profile = fetch_profile(user)  # Async[Profile]，依赖 user
    generate_report(user, profile) # 依赖 profile
}
```

#### 5.2.2 并作块

```yaoxiang
# spawn { } - 显式并行构造
# 块内所有表达式作为独立任务并发执行

compute_all(Int, Int) -> (Int, Int, Int) spawn = (a, b) => {
    # 三个独立计算并行执行
    (x, y, z) = spawn {
        heavy_calc(a),        # 任务 1
        heavy_calc(b),        # 任务 2
        another_calc(a, b)    # 任务 3
    }
    (x, y, z)
}
```

#### 5.2.3 并作循环

```yaoxiang
# spawn for - 数据并行循环
# 每次迭代作为独立任务并行执行

parallel_sum(Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # 每次迭代并行
    }
    total
}
```

#### 5.2.4 数据并行循环

```yaoxiang
# spawn for - 数据并行循环
# 每次迭代作为独立任务并行执行

parallel_sum(Int) -> Int spawn = (n) => {
    total = spawn for i in 0..n {
        fibonacci(i)          # 每次迭代并行
    }
    total
}

# 矩阵乘法并行化
matmul[[A: Matrix], [B: Matrix]] -> Matrix spawn = (A, B) => {
    result = spawn for i in 0..A.rows {
        row = spawn for j in 0..B.cols {
            dot_product(A.row(i), B.col(j))
        }
        row
    }
    result
}
```

### 5.3 自动等待机制

```yaoxiang
# 无需显式 await，编译器自动插入等待点

main() -> Void = () => {
    # 自动并行：两个独立请求并行执行
    users = fetch_users()      # Async[List[User]]
    posts = fetch_posts()      # Async[List[Post]]

    # 等待点在"+"操作处自动插入
    count = users.length + posts.length

    # 字段访问触发等待
    first_user = users[0]      # 等待 users 就绪
    print(first_user.name)
}

# 条件分支中的等待
process_data() -> Void spawn = () => {
    data = fetch_data()        # Async[Data]

    if data.is_valid {         # 等待 data 就绪
        process(data)
    } else {
        log("Invalid data")
    }
}
```

### 5.4 并发控制工具

```yaoxiang
# 等待所有任务完成
await_all[List[T]](List[Async[T]]) -> List[T] = (tasks) => {
    # Barrier 等待
}

# 等待任意一个完成
await_any[List[T]](List[Async[T]]) -> T = (tasks) => {
    # 返回第一个完成的结果
}

# 超时控制
with_timeout[T](Async[T], Duration) -> Option[T] = (task, timeout) => {
    # 超时返回 None
}
```

### 5.5 线程安全：Send/Sync 约束

YaoXiang 采用类似 Rust 的 **Send/Sync 类型约束**来保证线程安全，在编译时消除数据竞争。

#### 5.5.1 Send 约束

**Send**：类型可以安全地跨线程**转移所有权**。

```yaoxiang
# 基本类型自动满足 Send
# Int, Float, Bool, String 都是 Send

# 结构体自动派生 Send
type Point = Point(x: Int, y: Float)
# Point 是 Send，因为 Int 和 Float 都是 Send

# 包含非 Send 字段的类型不是 Send
type NonSend = NonSend(data: Rc[Int])
# Rc 不是 Send（引用计数非原子），因此 NonSend 不是 Send
```

#### 5.5.2 Sync 约束

**Sync**：类型可以安全地跨线程**共享引用**。

```yaoxiang
# 基本类型都是 Sync
type Point = Point(x: Int, y: Float)
# &Point 是 Sync，因为 &Int 和 &Float 都是 Sync

# 包含内部可变性的类型
type Counter = Counter(value: Int, mutex: Mutex[Int])
# &Counter 是 Sync，因为 Mutex 提供内部可变性
```

#### 5.5.3 spawn 与线程安全

```yaoxiang
# spawn 要求参数和返回值满足 Send

# 有效：Data 是 Send
type Data = Data(value: Int)
task = spawn(|| => Data(42))

# 无效：Rc 不是 Send
type SharedData = SharedData(rc: Rc[Int])
# task = spawn(|| => SharedData(Rc.new(42))  # 编译错误！

# 解决方案：使用 Arc（原子引用计数）
type SafeData = SafeData(value: Arc[Int])
task = spawn(|| => SafeData(Arc.new(42)))  # Arc 是 Send + Sync
```

#### 5.5.4 线程安全类型派生规则

```yaoxiang
# 结构体类型
type Struct[T1, T2] = Struct(f1: T1, f2: T2)

# Send 派生
Struct[T1, T2]: Send ⇐ T1: Send 且 T2: Send

# Sync 派生
Struct[T1, T2]: Sync ⇐ T1: Sync 且 T2: Sync

# 联合类型
type Result[T, E] = ok(T) | err(E)

# Send 派生
Result[T, E]: Send ⇐ T: Send 且 E: Send
```

#### 5.5.5 标准库线程安全实现

| 类型 | Send | Sync | 说明 |
|------|:----:|:----:|------|
| `Int`, `Float`, `Bool` | ✅ | ✅ | 原类型 |
| `Arc[T]` | ✅ | ✅ | T: Send + Sync |
| `Mutex[T]` | ✅ | ✅ | T: Send |
| `RwLock[T]` | ✅ | ✅ | T: Send |
| `Channel[T]` | ✅ | ❌ | 只发送端 Send |
| `Rc[T]` | ❌ | ❌ | 非原子引用计数 |
| `RefCell[T]` | ❌ | ❌ | 运行时借用检查 |

```yaoxiang
# 线程安全计数器示例
type SafeCounter = SafeCounter(mutex: Mutex[Int])

main() -> Void = () => {
    counter: Arc[SafeCounter] = Arc.new(SafeCounter(Mutex.new(0)))

    # 并发更新
    spawn(|| => {
        guard = counter.mutex.lock()  # Mutex 提供线程安全
        guard.value = guard.value + 1
    })

    spawn(|| => {
        guard = counter.mutex.lock()
        guard.value = guard.value + 1
    })
}
```

### 5.6 阻塞操作

```yaoxiang
# 使用 @blocking 注解标记会阻塞 OS 线程的操作
# 运行时会将其分配到专用阻塞线程池

@blocking
read_large_file(String) -> String = (path) => {
    # 此调用不会阻塞核心调度器
    file = File.open(path)
    content = file.read_all()
    content
}
```

---

## 六、模块系统

### 6.1 模块定义

```yaoxiang
# 模块使用文件作为边界
# Math.yx 文件
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
```

### 6.2 模块导入

```yaoxiang
# 导入整个模块
use std.io

# 导入并重命名
use std.io as IO

# 导入具体函数
use std.io.{ read_file, write_file }
```

---

## 七、方法绑定与柯里化

YaoXiang 采用纯函数式设计，通过柯里化实现类似对象方法调用的语法糖，无需引入 `struct`、`class` 等关键字。

### 7.1 核心函数定义

类型的所有操作都通过普通函数实现，第一个参数约定为操作的主体：

```yaoxiang
# === Point.yx (模块) ===

# 统一语法：构造器就是类型
type Point = Point(x: Float, y: Float)

# 核心函数：欧几里得距离
# 第一个参数是操作的主体（a）
distance(Point, Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# 曼哈顿距离
manhattan(Point, Point) -> Float = (a, b) => {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
```

### 7.2 方法语法糖绑定

通过部分应用（柯里化）实现 `.method()` 语法糖：

```yaoxiang
# 方法语法糖绑定
# Point.distance(_) = distance(self, _)
# 表示：Point.distance 是将 self 作为第一个参数的柯里化函数
Point.distance(_) = distance(self, _)
Point.manhattan(_) = manhattan(self, _)
```

### 7.3 调用方式

```yaoxiang
# === main.yx ===

use Point

main() -> Void = () => {
    # 值构造：与函数调用完全相同
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)

    # 两种调用方式完全等价
    d1 = distance(p1, p2)           # 直接调用核心函数
    d2 = p1.distance(p2)            # 方法语法糖

    print(d1)  # ≈ 2.828
    print(d2)  # ≈ 2.828

    # 函数式用法：预先绑定第一个参数
    dist_from_origin = Point.distance(Point(0.0, 0.0))
    result = dist_from_origin(p1)   # 5.0

    # 柯里化用法：延迟求值
    get_dist_to_p2 = p2.distance(_)
    d3 = get_dist_to_p2(p1)         # 2.828
}
```

### 7.4 设计优势

| 特性 | 说明 |
|------|------|
| **签名一致** | 定义时 `distance(Point, Point)`，调用时传递 2 个参数，无隐藏的 `self` |
| **函数即值** | `Point.distance` 是可以直接赋值、传递的函数值 |
| **无额外关键字** | 不需要 `enum`、`struct`、`union`、`class`、`method` 等关键字 |
| **纯函数式** | 所有操作都是纯函数，易于测试和推理 |
| **灵活组合** | 柯里化使得函数组合自然流畅 |

### 7.5 模式匹配解构

类型同样支持构造器模式匹配：

```yaoxiang
# 解构
match point {
    Point(0.0, 0.0) -> "origin"
    Point(x, y) -> "point at (" + x + ", " + y + ")"
}
```

---

## 八、AI友好设计

YaoXiang 的语法设计特别考虑了 AI 代码生成和修改的需求：

### 8.1 设计原则

```yaoxiang
# AI友好设计目标：
# 1. 严格结构化，无歧义语法
# 2. AST清晰，定位容易
# 3. 语义明确，无隐藏行为
# 4. 代码块边界明确
# 5. 类型信息完整
```

### 8.2 严格缩进规则

```yaoxiang
# 必须使用 4 空格缩进
# 禁止使用 Tab

# 正确示例
example() -> Void = () => {
    if condition {
        do_something()
    } else {
        do_other()
    }
}

# 错误示例（禁止）
example() -> Void = () => {
if condition {    # 缩进不足
do_something()
  }                 # 缩进不一致
}
```

### 8.3 明确的代码块边界

```yaoxiang
# 函数定义 - 明确的开始和结束
function_name(Params) -> ReturnType = (params) => {
    # 函数体
}

# 条件语句 - 必须有花括号
if condition {
    # 条件体
}

# 类型定义 - 明确的字段列表
type MyType = {
    field1: Type1
    field2: Type2
}
```

### 8.4 无歧义语法

```yaoxiang
# 禁止省略括号
# 正确
foo(T) -> T = (x) => x
my_list = [1, 2, 3]

# 错误（禁止）
foo T { T }             # 函数参数必须有括号
my_list = [1 2 3]       # 列表元素必须有逗号
```

---

## 九、快速入门

### 9.1 Hello World

```yaoxiang
# hello.yx
use std.io

main() -> Void = () => {
    println("Hello, YaoXiang!")
}
```

运行方式：`yaoxiang hello.yx`

输出：
```
Hello, YaoXiang!
```

### 9.2 基本语法

```yaoxiang
# 变量与类型
x = 42                    # 自动推断为 Int
name = "YaoXiang"         # 自动推断为 String
pi = 3.14159              # 自动推断为 Float

# 函数
add(Int, Int) -> Int = (a, b) => a + b

# 条件
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

# 循环
for i in 0..10 {
    print(i)
}
```

### 9.3 下一步

- 阅读 [语言规范](./YaoXiang-language-specification.md) 了解完整语法
- 查看 [示例代码](./examples/) 学习常用模式
- 参考 [实现计划](./YaoXiang-implementation.md) 了解技术细节

---

## 附录

### A. 关键字与注解

| 关键字 | 作用 |
|--------|------|
| `type` | 类型定义 |
| `pub` | 公共导出 |
| `use` | 导入模块 |
| `spawn` | 异步标记（函数/块/循环） |
| `ref` | 不可变引用 |
| `mut` | 可变引用 |
| `if/elif/else` | 条件分支 |
| `match` | 模式匹配 |
| `while/for` | 循环 |
| `return/break/continue` | 控制流 |
| `as` | 类型转换 |
| `in` | 成员访问 |

| 注解 | 作用 |
|------|------|
| `@blocking` | 标记阻塞操作，分配到阻塞线程池 |
| `@eager` | 标记需急切求值的表达式 |
| `@Send` | 显式声明满足 Send 约束 |
| `@Sync` | 显式声明满足 Sync 约束 |

### B. 设计灵感

- **Rust**：所有权模型、零成本抽象
- **Python**：语法风格、可读性
- **Idris/Agda**：依赖类型、类型驱动开发
- **TypeScript**：类型注解、运行时类型

---

> 「道生一，一生二，二生三，三生万物。」
> —— 《道德经》
>
> 类型如道，万物皆由此生。
