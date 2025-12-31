# YaoXiang（爻象）编程语言指南

> 版本：v1.0.0
> 状态：草稿
> 作者：沫郁酱
> 日期：2024-12-31

---

## 目录

1. [语言概述](#一语言概述)
2. [核心特性](#二核心特性)
3. [类型系统](#三类型系统)
4. [内存管理](#四内存管理)
5. [异步编程](#五异步编程)
6. [模块系统](#六模块系统)
7. [AI友好设计](#七ai友好设计)
8. [快速入门](#八快速入门)

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
fn add(a: Int, b: Int) -> Int {
    a + b
}

# 类型定义
type Point = struct {
    x: Float
    y: Float
}

# 无感异步
fn fetch_data(url: String) -> JSON spawn {
    HTTP.get(url).json()
}

fn main() {
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
fn add(a: Int, b: Int) -> Int {
    a + b
}

# 模块是类型的组合
mod Math {
    pi: Float = 3.14159
    fn sin(x: Float) -> Float { ... }
}
```

### 2.2 数学抽象

YaoXiang 的类型系统基于类型论和范畴论，提供了：

- **依赖类型**：类型可以依赖于值
- **泛型编程**：类型参数化
- **类型组合**：联合类型、交集类型

```yaoxiang
# 依赖类型：固定长度向量
type Vector[T, n: Nat] = struct {
    data: [T; n]  # 固定长度数组
}

# 类型联合
type Number = Int | Float

# 类型交集
type Printable = struct { to_string: fn() -> String }
type Serializable = struct { to_json: fn() -> String }
type Versatile = Printable & Serializable
```

### 2.3 零成本抽象

YaoXiang 保证零成本抽象，即高层次的抽象不会带来运行时的性能开销：

- **单态化**：泛型函数在编译时展开为具体版本
- **内联优化**：简单函数自动内联
- **栈分配**：小对象默认栈分配

```yaoxiang
# 泛型展开（单态化）
fn identity[T](x: T) -> T {
    x
}

# 使用
int_val = identity(42)      # 展开为 fn identity_int(Int) -> Int
str_val = identity("hello") # 展开为 fn identity_str(String) -> String

# 编译后无额外开销
```

### 2.4 自然语法

YaoXiang 采用 Python 风格的语法，追求可读性和自然语言感：

```yaoxiang
# 自动类型推断
x = 42
name = "YaoXiang"

# 简洁的函数定义
fn greet(name: String) -> String {
    "Hello, " + name
}

# 模式匹配
fn classify(n: Int) -> String {
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
│    ├── 复合类型 (Composite Types)                           │
│    │   ├── struct { fields }                               │
│    │   ├── union { variants }                              │
│    │   ├── enum { variants }                               │
│    │   ├── tuple (T1, T2, ...)                             │
│    │   ├── list [T], dict [K->V]                           │
│    │                                                        │
│    ├── 函数类型 (Function Types)                            │
│    │   fn (T1, T2, ...) -> R                               │
│    │                                                        │
│    ├── 泛型类型 (Generic Types)                             │
│    │   List[T], Map[K, V], etc.                            │
│    │                                                        │
│    ├── 依赖类型 (Dependent Types)                           │
│    │   type { n: Nat } -> type                             │
│    │                                                        │
│    └── 模块类型 (Module Types)                              │
│        mod { exports }                                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 类型定义

```yaoxiang
# 结构体类型
type Point = struct {
    x: Float
    y: Float
}

# 联合类型
type Result[T, E] = union {
    ok: T
    err: E
}

# 枚举类型
type Color = enum {
    red
    green
    blue
}

# 泛型类型
type List[T] = struct {
    elements: [T]
    length: Int
}

# 依赖类型
type Vector[T, n: Nat] = struct {
    data: [T; n]  # 固定长度数组
}
```

### 3.3 类型操作

```yaoxiang
# 类型作为值
MyInt = Int
MyList = List(Int)

# 类型组合
type Pair[T, U] = struct {
    first: T
    second: U
}

# 类型联合
type Number = Int | Float

# 类型反射
fn describe_type(t: type) -> String {
    match t {
        struct { fields } -> "Struct with " + fields.length + " fields"
        union { variants } -> "Union with " + variants.length + " variants"
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
fn add(a: Int, b: Int) {
    a + b                 # 推断返回类型为 Int
}

# 泛型推断
fn first[T](list: List[T]) -> option[T] {
    if list.length > 0 { some(list[0]) } else { none }
}
```

---

## 四、内存管理

### 4.1 所有权原则

YaoXiang 采用 Rust 风格的所有权模型：

```yaoxiang
# 默认不可变引用
fn process(data: ref Data) {
    # data 是只读的
    # 不能修改 data 的字段
    # 不能转移 data 的所有权
}

# 可变引用
fn modify(data: mut Data) {
    # 可以修改 data 的字段
    # 不能有其他活跃的引用
}

# 转移所有权
fn consume(data: Data) {
    # data 的所有权转移进来
    # 函数结束后 data 被销毁
}

# 借用返回
fn borrow_field(data: ref Data) -> ref Field {
    ref data.field
}
```

### 4.2 生命周期

```yaoxiang
# 显式生命周期标注（复杂情况）
fn longest<'a>(s1: &'a str, s2: &'a str) -> &'a str {
    if s1.length > s2.length { s1 } else { s2 }
}

# 自动生命周期推断
fn first[T](list: ref List[T]) -> ref T {
    ref list[0]
}
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
fn with_file(path: String) -> String {
    file = File.open(path)  # 自动打开
    content = file.read_all()
    # 函数结束，file 自动关闭
    content
}
```

---

## 五、异步编程

### 5.1 spawn 标记函数

YaoXiang 采用了创新的**无感异步**机制：

```yaoxiang
# 使用 spawn 标记异步函数
fn fetch_api(url: String) -> JSON spawn {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

fn calculate-heavy(n: Int) -> Int spawn {
    result = 0
    for i in 0..n {
        result += i
    }
    result
}
```

### 5.2 自动等待

```yaoxiang
# 调用 spawn 函数的代码自动等待
fn main() {
    # fetch_api 是异步的，但调用时自动等待
    data = fetch_api("https://api.example.com/data")
    # data 在这里已经就绪
    print(data.value)

    # 多个异步调用可以并行
    users = fetch_api("https://api.example.com/users")
    posts = fetch_api("https://api.example.com/posts")

    # users 和 posts 可能并行执行
    print(users.length + posts.length)
}
```

### 5.3 并发控制

```yaoxiang
# 并行执行多个异步任务
fn parallel_example() {
    tasks = [
        fetch_api("https://api1.com"),
        fetch_api("https://api2.com"),
        fetch_api("https://api3.com")
    ]

    # 显式并行
    results = parallel(tasks)

    # 或者等待全部完成
    all_results = await_all(tasks)

    # 或者任意一个完成即可
    first_result = await_any(tasks)
}
```

---

## 六、模块系统

### 6.1 模块定义

```yaoxiang
mod Math {
    pub fn sqrt(x: Float) -> Float { ... }
    pub pi = 3.14159

    fn internal_helper() { ... }  # 私有
}
```

### 6.2 模块导入

```yaoxiang
# 导入整个模块
use std.io

# 导入并重命名
use std.io as IO

# 导入具体函数
use std.io.{ read_file, write_file }

# 重新导出
mod MyMath {
    use std.math
    pub use std.math.{ sin, cos }
}
```

---

## 七、AI友好设计

YaoXiang 的语法设计特别考虑了 AI 代码生成和修改的需求：

### 7.1 设计原则

```yaoxiang
# AI友好设计目标：
# 1. 严格结构化，无歧义语法
# 2. AST清晰，定位容易
# 3. 语义明确，无隐藏行为
# 4. 代码块边界明确
# 5. 类型信息完整
```

### 7.2 严格缩进规则

```yaoxiang
# 必须使用 4 空格缩进
# 禁止使用 Tab

# 正确示例
fn example() {
    if condition {
        do_something()
    } else {
        do_other()
    }
}

# 错误示例（禁止）
fn example() {
if condition {    # 缩进不足
do_something()
  }                 # 缩进不一致
}
```

### 7.3 明确的代码块边界

```yaoxiang
# 函数定义 - 明确的开始和结束
fn function_name(params) -> ReturnType {
    # 函数体
}

# 条件语句 - 必须有花括号
if condition {
    # 条件体
}

# 类型定义 - 明确的字段列表
type MyType = struct {
    field1: Type1
    field2: Type2
}
```

### 7.4 无歧义语法

```yaoxiang
# 禁止省略括号
# 正确
fn foo(x) { x }
my_list = [1, 2, 3]

# 错误（禁止）
fn foo x { x }           # 函数参数必须有括号
my_list = [1 2 3]        # 列表元素必须有逗号
```

---

## 八、快速入门

### 8.1 Hello World

```yaoxiang
# hello.yx
use std.io

fn main() {
    println("Hello, YaoXiang!")
}
```

运行方式：`yaoxiang hello.yx`

输出：
```
Hello, YaoXiang!
```

### 8.2 基本语法

```yaoxiang
# 变量与类型
x = 42                    # 自动推断为 Int
name = "YaoXiang"         # 自动推断为 String
pi = 3.14159              # 自动推断为 Float

# 函数
fn add(a: Int, b: Int) -> Int {
    a + b
}

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

### 8.3 下一步

- 阅读 [语言规范](./YaoXiang-language-specification.md) 了解完整语法
- 查看 [示例代码](./examples/) 学习常用模式
- 参考 [实现计划](./YaoXiang-implementation.md) 了解技术细节

---

## 附录

### A. 关键字

| 关键字 | 作用 |
|--------|------|
| `type` | 类型定义 |
| `fn` | 函数定义 |
| `pub` | 公共导出 |
| `mod` | 模块定义 |
| `use` | 导入模块 |
| `spawn` | 异步标记 |
| `ref` | 不可变引用 |
| `mut` | 可变引用 |
| `if/elif/else` | 条件分支 |
| `match` | 模式匹配 |
| `while/for` | 循环 |
| `return/break/continue` | 控制流 |
| `as` | 类型转换 |

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
