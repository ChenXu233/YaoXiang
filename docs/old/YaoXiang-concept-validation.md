# YaoXiang（爻象）编程语言 - 概念验证文档

> 版本：v0.1.0-draft
> 作者：沫郁酱
> 日期：2024-12-31

---

## 目录

1. [语言概述](#1-语言概述)
2. [核心概念验证](#2-核心概念验证)
3. [类型系统设计](#3-类型系统设计)
4. [所有权与内存模型](#4-所有权与内存模型)
5. [无感异步机制](#5-无感异步机制)
6. [语法设计](#6-语法设计)
7. [AI友好性设计](#7-ai友好性设计)
8. [性能与实现考量](#8-性能与实现考量)
9. [与现有语言的对比](#9-与现有语言的对比)
10. [风险与挑战](#10-风险与挑战)
11. [下一步计划](#11-下一步计划)

---

## 1. 语言概述

### 1.1 设计目标

YaoXiang（爻象）是一门实验性的通用编程语言，旨在融合以下特性：

- **类型即一切**：值、函数、模块、泛型都是类型，类型是一等公民
- **数学抽象**：基于类型论的统一抽象框架
- **零成本抽象**：高性能，无GC，所有权模型保证内存安全
- **自然语法**：Python般的可读性，接近自然语言
- **无感异步**：无需显式await，编译器自动处理
- **AI友好**：严格结构化，AST清晰，易于解析和修改

### 1.2 核心设计哲学

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang 设计哲学                        │
├─────────────────────────────────────────────────────────────┤
│  一切皆类型 → 统一抽象 → 类型即数据 → 运行时可用            │
│                                                              │
│  所有权模型 → 零成本抽象 → 无GC → 高性能                    │
│                                                              │
│  Python语法 → 自然语言感 → 可读性 → 新手友好                │
│                                                              │
│  自动推断 → 极简关键字 → 简洁表达 → AI友好                  │
└─────────────────────────────────────────────────────────────┘
```

### 1.3 语言定位

| 维度 | 定位 |
|------|------|
| 范式 | 多范式（函数式 + 命令式 + 面向对象） |
| 类型系统 | 依赖类型 + 参数化多态 |
| 内存管理 | 所有权 + RAII（无GC） |
| 编译模型 | AOT编译（可选JIT） |
| 目标场景 | 系统编程、应用开发、AI辅助编程 |

---

## 2. 核心概念验证

### 2.1 "一切皆类型"的可行性

#### 理论依据

在类型论中，类型可以看作是命题，值可以看作是证明。这个 Curry-Howard 同构揭示了类型与值之间的深层联系。YaoXiang 将这个思想推广到极致：

```
值是类型的实例
类型是类型的实例（元类型）
函数是输入类型到输出类型的映射
模块是类型的组合
泛型是类型的工厂
```

#### 验证示例

```yaoxiang
# 值是类型的实例
x: Int = 42
# x 是 Int 类型的实例

# 类型是类型的实例
MyList: type = List(Int)
# MyList 是 type（元类型）的实例

# 函数是类型之间的映射
add(Int, Int) -> Int = (a, b) => a + b
# add 是 (Int, Int) -> Int 类型的实例

# 模块是类型的组合（使用文件作为模块）
# Math.yx
pi: Float = 3.14159
sqrt(Float) -> Float = (x) => { ... }
# Math 模块是一种命名空间类型
```

#### 验证结论

✅ **可行** - 一切皆类型在数学上有坚实的理论基础（类型论、范畴论），在实践中可以通过统一的类型表示来实现。

### 2.2 依赖类型的高性能保证

#### 挑战

依赖类型语言（如Agda、Idris）通常性能较低，因为：

1. 复杂的类型检查
2. 运行时类型表示
3. 模式匹配的完全性检查

#### YaoXiang 的解决方案

```yaoxiang
# 编译时类型擦除（可选）
# 运行时类型信息按需加载

# 零成本抽象保证
identity<T>(T) -> T = (x) => x
# 编译为直接返回，无额外开销

# 类型层面的优化
type Nat = struct { n: Int }
# 编译为普通整型，无额外包装
```

#### 性能保证机制

| 机制 | 说明 |
|------|------|
| 单态化 | 泛型函数在编译时展开为具体版本 |
| 内联优化 | 简单函数自动内联 |
| 栈分配 | 小对象默认栈分配 |
| 逃逸分析 | 大对象才堆分配 |
| 条件类型擦除 | 可选运行时类型信息 |

#### 验证结论

✅ **可行** - 通过精心设计的编译策略，可以在保持依赖类型能力的同时实现高性能。

### 2.3 无感异步的可行性

#### 核心思想

```yaoxiang
# 自动await模型
# 函数调用时，编译器自动检测异步依赖
# 并插入适当的同步屏障

fetch_user(Int) -> User spawn = (id) => {
    database.query("SELECT * FROM users WHERE id = ?", id)
}

display_user(Int) -> String = (id) => {
    user = fetch_user(id)  # 自动等待结果
    "User: " + user.name   # 确保user已就绪
}
```

#### 编译器自动处理流程

```
源代码
   ↓
类型检查 + 异步依赖分析
   ↓
识别spawn调用
   ↓
生成状态机
   ↓
自动插入await点
   ↓
优化同步屏障
   ↓
目标代码
```

#### 验证结论

✅ **可行** - 类似于Kotlin的协程和Rust的async/await，但通过编译期分析自动管理，减少程序员负担。

---

## 3. 类型系统设计

### 3.1 类型层次

```
┌─────────────────────────────────────────────────────────────┐
│                    YaoXiang 类型层次                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  type (元类型)                                               │
│    │                                                        │
│    ├── 原类型 (Primitive Types)                             │
│    │   ├── Void                                             │
│    │   ├── Bool                                             │
│    │   ├── Int (8/16/32/64/128)                            │
│    │   ├── Uint (8/16/32/64/128)                           │
│    │   ├── Float (32/64)                                   │
│    │   ├── Char, String                                    │
│    │   └── Bytes                                           │
│    │                                                        │
│    ├── 复合类型 (Composite Types)                           │
│    │   ├── struct { fields }                               │
│    │   ├── union { variants }                              │
│    │   ├── enum { variants }                               │
│    │   ├── tuple (T1, T2, ...)                             │
│    │   ├── list [T], dict [K->V]                           │
│    │   └── option [T]                                      │
│    │                                                        │
│    ├── 函数类型 (Function Types)                            │
│    │   fn (T1, T2, ...) -> R                               │
│    │                                                        │
│    ├── 泛型类型 (Generic Types)                             │
│    │   List[T], Map[K, V], etc.                            │
│    │                                                        │
│    ├── 依赖类型 (Dependent Types)                           │
│    │   type { n: Nat } -> type                             │
│    │   Vec[n: Nat, T]                                      │
│    │                                                        │
│    └── 模块类型 (Module Types)                              │
│        mod { exports }                                      │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 类型定义语法

```yaoxiang
# 原类型（内置）
# 无需定义，直接使用

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

# 函数类型
type Adder = fn(Int, Int) -> Int
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

# 类型交集
type Printable = struct { to_string: fn() -> String }
type Serializable = struct { to_json: fn() -> String }
type Versatile = Printable & Serializable

# 类型条件
type Conditional[T] = if T == Int {
    Int64
} else {
    T
}
```

### 3.4 运行时类型信息

```yaoxiang
# 类型反射
fn describe(t: type) -> String {
    match t {
        struct { fields } -> "Struct with " + fields.length + " fields"
        union { variants } -> "Union with " + variants.length + " variants"
        enum { variants } -> "Enum with " + variants.length + " cases"
        list { element } -> "List of " + element.name
        fn { params, ret } -> "Function: (" + params + ") -> " + ret
        primitive { name } -> "Primitive: " + name
    }
}

# 类型检查
fn is_number(t: type) -> Bool {
    t == Int or t == Float or t == Number
}

# 类型实例检查
value: type = ...
if value has_type Int {
    print("It's an integer")
}

# 类型转换
fn safe_cast[T, U](value: T, target: type) -> option[U] {
    if value has_type target {
        some(value as U)
    } else {
        none
    }
}
```

---

## 4. 所有权与内存模型

### 4.1 所有权原则

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

# RefCell - 内部可变性
internal_mut: RefCell[Data] = RefCell.new(data)
```

### 4.4 内存安全保证

```yaoxiang
# 编译期检查
unsafe_example() -> Void = () => {
    data: Data = ...
    ref1 = ref data
    ref2 = ref data  # 编译错误！多个活跃引用

    mut_data = mut data
    ref_mut = ref mut_data
    mut_data2 = mut mut_data  # 编译错误！可变与不可变引用同时存在
}
```

---

## 5. 无感异步机制

### 5.1 spawn 标记函数

```yaoxiang
# 使用 spawn 标记异步函数
fetch_api(String) -> JSON spawn = (url) => {
    response = HTTP.get(url)
    JSON.parse(response.body)
}

calculate_heavy(Int) -> Int spawn = (n) => {
    # 耗时计算
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
main() -> Void = () => {
    # fetch_api 是异步的，但调用时自动等待
    data = fetch_api("https://api.example.com/data")
    # data 在这里已经就绪

    # 可以继续使用 data
    print(data.value)

    # 多个异步调用可以并行
    users = fetch_api("https://api.example.com/users")
    posts = fetch_api("https://api.example.com/posts")

    # 赋值时自动等待
    # users 和 posts 可能并行执行
    print(users.length + posts.length)
}
```

### 5.3 底层实现机制

```yaoxiang
# 编译器内部转换
# 源代码：
#   result = async_func()

# 编译后（伪代码）：
#   if result.is_pending() {
#       yield_to_scheduler()
#   }
#   return result.value()
```

### 5.4 显式并发控制

```yaoxiang
# 并行执行多个异步任务
parallel_example() -> Void = () => {
    tasks = [
        fetch_api("https://api1.com"),
        fetch_api("https://api2.com"),
        fetch_api("https://api3.com")
    ]

    # 显式并行（使用所有CPU核心）
    results = parallel(tasks)

    # 或者等待全部完成
    all_results = await_all(tasks)

    # 或者任意一个完成即可
    first_result = await_any(tasks)
}
```

---

## 6. 语法设计

### 6.1 关键字（17个）

YaoXiang 共定义 17 个关键字，这些关键字是保留的，不能用作标识符。

| # | 关键字 | 作用 | 示例 |
|---|--------|------|------|
| 1 | `type` | 类型定义 | `type Point = struct { x: Int, y: Int }` |
| 2 | `pub` | 公共导出 | `pub add(Int, Int) -> Int = ...` |
| 3 | `use` | 导入模块 | `use std.io` |
| 4 | `spawn` | 异步标记 | `fetch(String) -> T spawn = ...` |
| 5 | `ref` | 不可变引用 | `process(ref Data) -> Void = ...` |
| 6 | `mut` | 可变引用 | `modify(mut Data) -> Void = ...` |
| 7 | `if` | 条件分支 | `if x > 0 { ... }` |
| 8 | `elif` | 多重条件 | `elif x == 0 { ... }` |
| 9 | `else` | 默认分支 | `else { ... }` |
| 10 | `match` | 模式匹配 | `match x { 0 -> "zero" }` |
| 11 | `while` | 条件循环 | `while i < 10 { ... }` |
| 12 | `for` | 迭代循环 | `for item in items { ... }` |
| 13 | `return` | 返回值 | `return result` |
| 14 | `break` | 跳出循环 | `break` |
| 15 | `continue` | 继续循环 | `continue` |
| 16 | `as` | 类型转换 | `x as Float` |
| 17 | `in` | 成员检测/列表推导式 | `x in [1, 2, 3]`, `[x * 2 for x in list]` |

**无限循环替代方案：**

```yaoxiang
# 使用 while True 替代 loop 关键字
while True {
    input = read_line()
    if input == "quit" {
        break
    }
    process(input)
}
```

### 6.2 保留字

保留字是语言预定义的特殊值，不能用作标识符，但它们不是关键字（不能用于语法结构）。

| 保留字 | 类型 | 说明 |
|--------|------|------|
| `true` | Bool | 布尔值真 |
| `false` | Bool | 布尔值假 |
| `null` | Void | 空值 |
| `none` | Option | Option 类型的无值变体 |
| `some(T)` | Option | Option 类型的值变体（函数） |
| `ok(T)` | Result | Result 类型的成功变体（函数） |
| `err(E)` | Result | Result 类型的错误变体（函数） |

```yaoxiang
# 布尔值
flag = true
flag = false

# Option 类型使用
maybe_value: option[String] = none
maybe_value = some("hello")

# Result 类型使用
result: result[Int, String] = ok(42)
result = err("error message")
```

### 6.3 变量与赋值

```yaoxiang
# 自动类型推断
x = 42                    # Int
name = "YaoXiang"         # String
pi = 3.14159              # Float
is_valid = true           # Bool

# 显式类型注解（可选）
count: Int = 100
price: Float = 19.99

# 不可变（默认）
x = 10
x = 20  # 编译错误！

# 可变变量
mut count = 0
count = count + 1  # OK

# 引用
original = 42
alias = ref original  # 只读引用
mutable = mut 42
modifier = mut mutable  # 可变引用
```

### 6.3 函数定义

```yaoxiang
# 基本函数
greet(String) -> String = (name) => "Hello, " + name

# 返回类型推断
add(Int, Int) -> Int = (a, b) => a + 1  # 最后表达式作为返回值

# 多返回值
divmod(Int, Int) -> (Int, Int) = (a, b) => (a / b, a % b)

# 泛型函数
identity<T>(T) -> T = (x) => x

# 高阶函数
apply<T, U>((T) -> U, T) -> U = (f, value) => f(value)

# 闭包
create_counter() -> () -> Int = () => {
    mut count = 0
    () => {
        count += 1
        count
    }
}
```

### 6.4 控制流

```yaoxiang
# 条件
if x > 0 {
    "positive"
} elif x == 0 {
    "zero"
} else {
    "negative"
}

# 模式匹配
classify(Int) -> String = (n) => {
    match n {
        0 -> "zero"
        1 -> "one"
        2 -> "two"
        _ if n < 0 -> "negative"
        _ -> "many"
    }
}

# 循环
mut i = 0
while i < 10 {
    print(i)
    i += 1
}

# 迭代
for item in [1, 2, 3] {
    print(item)
}

# 无限循环（配合 break）
loop {
    input = read_line()
    if input == "quit" {
        break
    }
    process(input)
}
```

### 6.5 模块系统

```yaoxiang
# 模块定义（使用文件作为模块）
# math.yx
pub pi: Float = 3.14159
pub sqrt(Float) -> Float = (x) => { ... }
internal_helper() -> Void = () => { ... }  # 私有

# 导入模块
use std.io
use std.list as ListLib

# 导入具体函数
use std.io.{ read_file, write_file }

# 模块别名
use math as M
result = M.sqrt(4.0)
```

---

## 7. AI友好性设计

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
example() -> Void = () => {
    if condition {
        do_something()
    } else {
        do_other()
    }
}

# 错误示例（禁止）
example() -> Void = () => {
if condition {
do_something()  # 缩进不足
  }               # 缩进不一致
}
```

### 7.3 明确的代码块边界

```yaoxiang
# 函数定义 - 明确的开始和结束
function_name(Params) -> ReturnType = (params) => {
    # 函数体
}

# 条件语句 - 必须有花括号
if condition {
    # 条件体
}

# 循环语句 - 必须有花括号
for item in items {
    # 循环体
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
foo(T) -> T = (x) => x
my_list = [1, 2, 3]

# 错误（禁止）
foo T { x }             # 函数参数必须有括号
my_list = [1 2 3]       # 列表元素必须有逗号

# 禁止行尾冒号的特殊含义
# 冒号仅用于类型注解和字典
my_dict = { "key": "value" }
foo() -> Int = () => 42
```

### 7.5 类型信息完整

```yaoxiang
# AI 可以轻松获取：
# 1. 变量的推断类型
# 2. 函数的参数和返回类型
# 3. 类型的完整结构
# 4. 模块的导出接口

# 类型注解提供完整信息
complex_function(ref List[Int], mut Config, (Result) -> Void) -> Result[Data] = (
    data,
    config,
    callback
) => {
    # 函数签名完整，AI 可以准确理解
}

# 类型定义完整
type APIResponse = struct {
    status: Int
    message: String
    data: option[List[DataItem]]
    timestamp: Int64
}
```

### 7.6 易于定位的关键位置

```yaoxiang
# 1. 类型定义位置明确
# type 关键字开头

type User = struct {
    id: Int
    name: String
}
# ↑ 类型定义从这里开始

# 2. 函数定义位置明确
# 函数名开头

pub process_user(ref User) -> Result = (user) => {
    # ↑ 函数从这里开始
}

# 3. 模块边界明确
# 文件即模块，文件名即为模块名

# Database.yx
# ↑ 模块从这里开始

# 4. 导入语句位置明确
# use 关键字开头

use std.io
use std.database
# ↑ 导入语句集中在此
```

---

## 8. 性能与实现考量

### 8.1 零成本抽象

```yaoxiang
# 泛型展开（单态化）
identity<T>(T) -> T = (x) => x

# 使用
int_val = identity(42)      # 展开为 identity(Int) -> Int
str_val = identity("hello") # 展开为 identity(String) -> String

# 编译后无额外开销
```

### 8.2 无GC内存管理

```yaoxiang
# RAII 自动释放
with_file(String) -> String = (path) => {
    file = File.open(path)  # 自动打开
    # 使用 file
    content = file.read_all()
    # 函数结束，file 自动关闭
    content
}

# 所有权转移释放
create_resource() -> Resource = () => {
    Resource.new()  # 创建
}  # 返回时转移所有权

use_resource(Resource) -> Void = (res) => {
    # 使用 res
}  # res 在此销毁
```

### 8.3 编译优化

```yaoxiang
# 内联优化
inline add(Int, Int) -> Int = (a, b) => a + b

# 循环展开
# 编译器自动优化简单循环

# 逃逸分析
create_large_object() -> List[Int] = () => {
    large_data = [0; 1000000]  # 大数组
    if need_return(large_data) {
        return large_data  # 堆分配
    }
    # 否则优化为栈分配或直接消除
}
```

### 8.4 并发性能

```yaoxiang
# 绿色线程模型
# 轻量级线程，高并发

main() -> Void = () => {
    # 启动 10,000 个并发任务
    for i in 0..10000 {
        spawn process_item(i)
    }
}
```

---

## 9. 与现有语言的对比

### 9.1 对比矩阵

| 特性 | YaoXiang | Rust | Python | TypeScript | Idris |
|------|----------|------|--------|------------|-------|
| 一切皆类型 | ✅ | ❌ | ❌ | ❌ | ✅ |
| 自动类型推断 | ✅ | ✅ | ✅ | ✅ | ✅ |
| 默认不可变 | ✅ | ✅ | ❌ | ❌ | ✅ |
| 所有权模型 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 无感异步 | ✅ | ❌ | ❌ | ❌ | ❌ |
| 依赖类型 | ✅ | ❌ | ❌ | ❌ | ✅ |
| 运行时类型 | ✅ | ❌ | ✅ | ✅ | ❌ |
| 零成本抽象 | ✅ | ✅ | ❌ | ❌ | ❌ |
| 无GC | ✅ | ✅ | ❌ | ❌ | ✅ |
| AI友好语法 | ✅ | ❌ | ✅ | ❌ | ❌ |
| 关键字数量 | 16 | 51+ | 35 | 64+ | 30+ |

### 9.2 详细对比

#### vs Rust

| 维度 | YaoXiang | Rust |
|------|----------|------|
| 语法复杂度 | 简单（Python风格） | 复杂（学习曲线陡峭） |
| async/await | 自动，无需标记 | 需显式标记 |
| 错误处理 | ? 运算符或 Result | Result / Option |
| 生命周期 | 可选标注 | 必须标注 |

#### vs Python

| 维度 | YaoXiang | Python |
|------|----------|--------|
| 类型安全 | 编译期检查 | 动态类型 |
| 性能 | 高（编译型） | 低（解释型） |
| 内存管理 | 所有权，无GC | GC |
| 并发 | 高性能绿色线程 | GIL限制 |

#### vs TypeScript

| 维度 | YaoXiang | TypeScript |
|------|----------|------------|
| 类型系统 | 依赖类型 | 仅泛型 |
| 运行时类型 | 完整内省 | 有限 |
| 编译目标 | 原生机器码 | JavaScript |
| 性能 | 高 | 中 |

---

## 10. 风险与挑战

### 10.1 技术风险

| 风险 | 可能性 | 影响 | 缓解措施 |
|------|--------|------|----------|
| 依赖类型编译时间过长 | 中 | 高 | 增量编译，缓存 |
| 自动await 语义复杂 | 中 | 中 | 渐进式实现 |
| 所有权模型学习曲线 | 低 | 中 | 编译器友好提示 |
| 类型系统过于复杂 | 中 | 高 | 简化子集优先 |

### 10.2 实现挑战

```yaoxiang
# 挑战1：类型推断的完整性
# 需要实现 Hindley-Milner 类型系统的扩展

# 挑战2：依赖类型检查
# 需要实现类型论中的判决定算法

# 挑战3：自动await 的正确性
# 需要确保所有依赖正确识别

# 挑战4：所有权检查
# 需要实现类似 Rust 的借用检查器
```

### 10.3 语言设计风险

- **风险**：类型系统过于强大可能导致编译时间过长
- **缓解**：提供类型检查模式选择
- **风险**：语法限制可能影响灵活性
- **缓解**：保持核心简洁，可选扩展

---

## 11. 下一步计划

### 11.1 短期计划（1-2个月）

- [ ] 完成语言规范文档
- [ ] 设计核心数据类型
- [ ] 实现简单的类型检查器
- [ ] 验证自动await 机制

### 11.2 中期计划（3-6个月）

- [ ] 实现完整的类型系统
- [ ] 实现所有权检查
- [ ] 构建基础标准库
- [ ] 编写用户教程

### 11.3 长期计划（6-12个月）

- [ ] 完整的编译器实现
- [ ] 依赖类型支持
- [ ] 工具链完善（IDE、调试器）
- [ ] 性能优化

---

## 附录

### A. 设计灵感来源

- **Rust**：所有权模型、零成本抽象
- **Python**：语法风格、可读性
- **Idris/Agda**：依赖类型、类型驱动开发
- **TypeScript**：类型注解、运行时类型
- **MoonBit**：AI友好设计

### B. 参考资料

- [Type Theory - Wikipedia](https://en.wikipedia.org/wiki/Type_theory)
- [Rust Ownership - The Rust Programming Language](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [Idris - A Language for Type-Driven Development](https://www.idris-lang.org/)
- [Zero-Cost Abstractions in Rust](https://blog.stackademic.com/zero-cost-abstractions-in-rust-high-level-code-with-low-level-performance-18810eddfbed)

---

> "道生一，一生二，二生三，三生万物。"
> —— 《道德经》
>
> 类型如道，万物皆由此生。
