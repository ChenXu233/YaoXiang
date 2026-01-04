# YaoXiang（爻象）编程语言指南

> 版本：v1.1.0
> 状态：草稿
> 作者：晨煦
> 日期：2024-12-31
> 更新：2025-01-04 - 修正泛型语法为 `[T]`，移除 `fn` 关键字

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
9. [类型集中约定](#九类型集中约定核心设计哲学)
10. [快速入门](#十快速入门)

---

**扩展文档**：
- [高级绑定特性与编译器实现](../works/plans/bind/YaoXiang-bind-advanced.md) - 深入的绑定机制、高级特性、编译器实现和边缘情况处理

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
x: Int = 42                           # 显式类型
y = 42                                # 推断为 Int
name = "YaoXiang"                     # 推断为 String

# 默认不可变
x: Int = 10
x = 20                                # ❌ 编译错误！不可变

# 统一声明语法：标识符: 类型 = 表达式
add: (Int, Int) -> Int = (a, b) => a + b  # 函数声明
inc: Int -> Int = x => x + 1               # 单参数函数

# 统一类型语法：构造器即类型
type Point = Point(x: Float, y: Float)
type Result[T, E] = ok(T) | err(E)

# 无感异步（并作函数）
fetch_data: (String) -> JSON spawn = (url) => {
    HTTP.get(url).json()
}

main: () -> Void = () => {
    # 值构造：与函数调用完全相同
    p = Point(3.0, 4.0)
    r = ok("success")

    data = fetch_data("https://api.example.com")
    # 自动等待，无需 await
    print(data.name)
}

# 泛型函数
identity: [T](T) -> T = x => x

# 高阶函数
apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)

# 柯里化
add_curried: Int -> Int -> Int = a => b => a + b
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
identity[T](T) -> T = (x) => x

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
greet: String -> String = (name) => "Hello, " + name

# 模式匹配
classify: Int -> String = (n) => {
    match n {
        0 -> "zero"
        1 -> "one"
        _ if n < 0 -> "negative"
        _ -> "many"
    }
}
```

### 2.5 完整语法规范

YaoXiang 采用统一的声明语法：**标识符: 类型 = 表达式**。同时提供向后兼容的旧语法。

#### 2.5.1 双语法策略与类型集中约定

为平衡创新与兼容，YaoXiang 支持两种语法形式，但采用统一的**类型集中标注约定**。

**语法形式对比：**

| 语法类型 | 格式 | 状态 | 说明 |
|---------|------|------|------|
| **新语法（标准）** | `name: Type = Lambda` | ✅ 推荐 | 官方标准，所有新代码应使用此形式 |
| **旧语法（兼容）** | `name(Types) -> Ret = Lambda` | ⚠️ 仅兼容 | 为历史代码保留，不推荐新项目使用 |

**核心约定：类型集中标注**

YaoXiang 采用**"声明优先，类型集中"**的设计约定：

```yaoxiang
# ✅ 正确：类型信息统一在声明行
add: (Int, Int) -> Int = (a, b) => a + b
#   └─────────────────┘   └─────────────┘
#       完整类型签名         实现逻辑

# ❌ 避免：类型信息分散在实现中
add = (a: Int, b: Int) => a + b
#     └───────────────┘
#     类型混在实现体中
```

**约定的好处：**

1. **语法一致性**：所有声明都遵循 `标识符: 类型 = 表达式`
2. **声明与实现分离**：类型信息一目了然，实现体专注逻辑
3. **AI友好性**：AI只需读声明行就能理解完整函数签名
4. **修改更安全**：修改类型只需改声明，不影响实现体
5. **柯里化友好**：支持清晰的柯里化类型签名

**选择建议**：
- **新项目**：必须使用新语法 + 类型集中约定
- **迁移项目**：逐步迁移到新语法和类型集中约定
- **维护旧代码**：可以继续使用旧语法，但建议采用类型集中约定

#### 2.5.2 基础声明语法

```yaoxiang
# === 新语法（推荐）===
# 所有声明都遵循：标识符: 类型 = 表达式

# 变量声明
x: Int = 42
name: String = "YaoXiang"
mut counter: Int = 0

# 函数声明
add: (Int, Int) -> Int = (a, b) => a + b
inc: Int -> Int = x => x + 1
getAnswer: () -> Int = () => 42
log: (String) -> Void = msg => print(msg)

# === 旧语法（兼容）===
# 仅用于函数，格式：name(Types) -> Ret = Lambda
add(Int, Int) -> Int = (a, b) => a + b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}
getRandom() -> Int = () => 42
```

#### 2.5.3 函数类型语法

```
函数类型 ::= '(' 参数类型列表 ')' '->' 返回类型
           | 参数类型 '->' 返回类型              # 单参数简写

参数类型列表 ::= [类型 (',' 类型)*]
返回类型 ::= 类型 | 函数类型 | 'Void'

# 函数类型是一等公民，可嵌套
# 高阶函数类型 ::= '(' 函数类型 ')' '->' 返回类型
```

| 示例 | 含义 |
|------|------|
| `Int -> Int` | 单参数函数类型 |
| `(Int, Int) -> Int` | 双参数函数类型 |
| `() -> Void` | 无参函数类型 |
| `(Int -> Int) -> Int` | 高阶函数：接收函数，返回 Int |
| `Int -> Int -> Int` | 柯里化函数（右结合） |

#### 2.5.4 泛型语法（仅用于类型参数）

```yaoxiang
# 泛型函数：<类型参数> 前缀
identity: [T](T) -> T = x => x
map: [A, B]((A) -> B, List[A]) -> List[B] = (f, xs) => case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# 泛型类型
List: Type = [T] List[T]
```

#### 2.5.5 Lambda 表达式语法

```
Lambda ::= '(' 参数列表 ')' '=>' 表达式
         | 参数 '=>' 表达式              # 单参数简写

参数列表 ::= [参数 (',' 参数)*]
参数 ::= 标识符 [':' 类型]               # 可选的类型注解
```

| 示例 | 含义 | 说明 |
|------|------|------|
| `(a, b) => a + b` | 多参数 Lambda | 配合声明使用：<br>`add: (Int, Int) = (a, b) => a + b` |
| `x => x + 1` | 单参数简写 | 配合声明使用：<br>`inc: Int = x => x + 1` |
| `(x: Int) => x + 1` | 带类型注解 | 仅用于Lambda内部临时需要 |
| `() => 42` | 无参 Lambda | 配合声明使用：<br>`get: () = () => 42` |

**注意**：Lambda 表达式中的类型注解 `(x: Int) => ...` 是**临时的、局部的**，主要用于：
- Lambda 内部需要类型信息时
- 配合声明语法使用时（类型在声明中已给出）
- 不应作为主要的类型声明方式

#### 2.5.6 完整示例

```yaoxiang
# === 基本函数声明 ===

# 基础函数（新语法）
add: (Int, Int) -> Int = (a, b) => a + b

# 单参数函数（两种形式）
inc: Int -> Int = x => x + 1
inc2: (Int) -> Int = (x) => x + 1

# 无参函数
getAnswer: () -> Int = () => 42

# 无返回值函数
log: (String) -> Void = msg => print(msg)

# === 递归函数 ===
# 递归在 lambda 中自然支持
fact: Int -> Int = (n) =>
  if n <= 1 then 1 else n * fact(n - 1)

# === 高阶函数与函数类型赋值 ===

# 函数类型作为一等公民
IntToInt: Type = Int -> Int
IntBinaryOp: Type = (Int, Int) -> Int

# 高阶函数声明
applyTwice: (IntToInt, Int) -> Int = (f, x) => f(f(x))

# 柯里化函数
addCurried: Int -> Int -> Int = a => b => a + b

# 函数组合
compose: (Int -> Int, Int -> Int) -> Int -> Int =
  (f, g) => x => f(g(x))

# 返回函数的函数
makeAdder: Int -> (Int -> Int) =
  x => y => x + y

# === 泛型函数 ===

# 泛型函数
identity: [T](T) -> T = x => x

# 泛型高阶函数
map: [A, B]((A) -> B, List[A]) -> List[B] =
  (f, xs) => case xs of
    [] => []
    (x :: rest) => f(x) :: map(f, rest)

# 泛型函数类型
Transformer: Type = [A, B](A) -> B

# 使用泛型类型
applyTransformer: [A, B](Transformer[A, B], A) -> B =
  (f, x) => f(x)

# === 复杂类型示例 ===

# 嵌套函数类型
higherOrder: ((Int) -> Int) -> (Int) -> Int =
  f => x => f(x) + 1

# 多参数高阶函数
zipWith: [A, B, C]((A, B) -> C, List[A], List[B]) -> List[C] =
  (f, xs, ys) => case (xs, ys) of
    ([], _) => []
    (_, []) => []
    (x::xs', y::ys') => f(x, y) :: zipWith(f, xs', ys')

# 函数类型别名
Predicate: Type = [T] (T) -> Bool
Mapper: Type = [A, B](A) -> B
Reducer: Type = [A, B](B, A) -> B

# === 旧语法示例（仅向后兼容） ===
# 不推荐在新代码中使用

mul(Int, Int) -> Int = (a, b) => a * b    # 多参数
square(Int) -> Int = (x) => x * x          # 单参数
empty() -> Void = () => {}                  # 无参
get_random() -> Int = () => 42              # 有返回值

# 等效的新语法（推荐）
mul: (Int, Int) -> Int = (a, b) => a * b
square: (Int) -> Int = (x) => x * x
empty: () -> Void = () => {}
get_random: () -> Int = () => 42
```

#### 2.5.7 语法解析规则

**类型解析优先级：**

| 优先级 | 类型 | 说明 |
|--------|------|------|
| 1 (最高) | 泛型应用 `List[T]` | 左结合 |
| 2 | 括号 `(T)` | 改变结合性 |
| 3 | 函数类型 `->` | 右结合 |
| 4 (最低) | 基础类型 `Int, String` | 原子类型 |

**类型解析示例：**

```yaoxiang
# (A -> B) -> C -> D
# 解析为: ((A -> B) -> (C -> D))

# A -> B -> C
# 解析为: (A -> (B -> C))  # 右结合

# (Int -> Int) -> Int
# 解析为: 接收函数，返回 Int -> Int

# List<Int -> Int>
# 解析为: List 的元素类型是 Int -> Int
```

**Lambda 解析示例：**

```yaoxiang
# a => b => a + b
# 解析为: a => (b => (a + b))  # 右结合，柯里化

# (a, b) => a + b
# 解析为: 接收两个参数，返回 a + b
```

#### 2.5.8 类型推断规则

YaoXiang 采用**双层处理**策略：解析层宽松放过，类型检查层严格推断。

**解析层规则：**
- 解析器只验证语法结构，不进行类型推断
- 缺少类型标注的声明，类型标注字段为 `None`
- 所有符合基础语法结构的声明都能通过解析
- **关键点**：`add = (a, b) => a + b` 在解析层是**合法**的

**类型检查层规则：**
- 验证语义正确性，包括类型完整性
- **参数必须有类型标注**：这是强制要求
- 返回类型可推断，但参数类型必须显式声明

**完整的类型推断规则：**

| 场景 | 参数推断 | 返回推断 | 解析结果 | 类型检查结果 | 推荐程度 |
|------|---------|---------|----------|-------------|---------|
| **标准函数** | 使用标注类型 | 使用标注类型 | ✅ | ✅ | ⭐⭐⭐⭐⭐ |
| `add: (Int, Int) -> Int = (a, b) => a + b` | | | | | |
| **部分推断** | 使用标注类型 | 从表达式推断 | ✅ | ✅ | ⭐⭐⭐⭐ |
| `add: (Int, Int) = (a, b) => a + b` | | | | | |
| `inc: Int -> Int = x => x + 1` | | | | | |
| `get: () = () => 42` | | | | | |
| **旧语法部分推断** | 使用标注类型 | 从表达式推断 | ✅ | ✅ | ⭐⭐⭐ (兼容) |
| `add(Int, Int) = (a, b) => a + b` | | | | | |
| `square(Int) = (x) => x * x` | | | | | |
| **参数无标注** | **无法推断** | - | ✅ | ❌ 错误 | ❌ 禁止 |
| `add = (a, b) => a + b` | | | | | |
| `identity = x => x` | | | | | |
| **无返回标注的块** | - | 从块内容推断 | ✅ | ✅ | ⭐⭐⭐⭐ |
| `main = () => {}` | | | | | |
| `get = () => { return 42; }` | | | | | |
| **无返回标注的块（无显式返回）** | - | 推断为 `Void` | ✅ | ✅ 不推荐 | ⚠️ 避免 |
| `bad = (x: Int) => { x }` | | | | | |

**详细推断规则：**

```
解析层：只看语法结构
├── 结构正确 → 通过
└── 结构错误 → 报错

类型检查层：验证语义
├── 参数类型推断
│   ├── 参数有类型标注 → 使用标注类型 ✅
│   ├── 参数无类型标注 → 拒绝 ❌
│   └── Lambda 参数必须标注 → 强制要求
│
├── 返回类型推断
│   ├── 有 return expr → 从 expr 推断 ✅
│   ├── 无 return，有表达式 → 从表达式推断 ✅
│   ├── 无 return，有块 `{ ... }`
│   │   ├── 块为空 `{}` → Void ✅
│   │   ├── 块有 return → 从 return 推断 ✅
│   │   └── 块无 return 且无显式返回 → 推断为 Void ✅（但不推荐）
│   └── 无法推断 → 拒绝 ❌
│
└── 完全无法推断 → 拒绝 ❌
```

**注意**：`bad = (x: Int) => { x }` 这种形式可以推断返回类型为 `Void`，但非常不推荐，因为：
- 代码意图不明确
- 容易造成理解错误
- 不符合函数式编程的表达式风格

**推断示例：**

```yaoxiang
# === 推断成功 ===

# 标准形式
main: () -> Void = () => {}                    # 完整标注
num: () -> Int = () => 42                      # 完整标注
inc: Int -> Int = x => x + 1                   # 单参数简写

# 部分推断（新语法）
add: (Int, Int) = (a, b) => a + b              # 参数有标注，返回推断
square: Int -> Int = x => x * x                # 参数有标注，返回推断
get_answer: () = () => 42                      # 参数有标注（空），返回推断

# 部分推断（旧语法，兼容）
add2(Int, Int) = (a, b) => a + b               # 参数有标注，返回推断
square2(Int) = (x) => x * x                    # 参数有标注，返回推断

# 从return推断
fact: Int -> Int = (n) => {
    if n <= 1 { return 1 }
    return n * fact(n - 1)
}

# === 推断失败 ===

# 参数无法推断（解析通过，类型检查失败）
add = (a, b) => a + b                          # ✗ 参数无类型
identity = x => x                              # ✗ 参数无类型

# 无显式返回的块
no_return = (x: Int) => { x }                  # ✗ 块无 return，无法推断隐式返回

# 全无法推断
bad_fn = x => x                                # ✗ 参数和返回都无法推断
```

#### 2.5.9 旧语法（向后兼容）

YaoXiang 提供旧语法支持以兼容历史代码，**不推荐在新代码中使用**。

```
旧语法 ::= 标识符 '(' [参数类型列表] ')' '->' 返回类型 '=' Lambda
```

| 特性 | 标准语法 | 旧语法 |
|------|---------|--------|
| 声明格式 | `name: Type = ...` | `name(Types) -> Type = ...` |
| 参数类型位置 | 在类型标注中 | 在函数名后的括号中 |
| 空参数 | 必须写 `()` | 可省略 `()` |
| **推荐程度** | ✅ **官方推荐** | ⚠️ **仅向后兼容** |
| **使用场景** | 所有新代码 | 历史代码维护 |

**不推荐原因：**
1. **学习成本**：与标准语法不一致，增加语言复杂度
2. **一致性**：参数类型位置不统一（一个在类型标注中，一个在函数名后）
3. **维护成本**：解析器需要额外处理两种形式
4. **AI友好性**：增加AI理解和生成代码的难度

**迁移建议：**
```yaoxiang
# 旧代码（不推荐）
mul(Int, Int) -> Int = (a, b) => a * b
square(Int) -> Int = (x) => x * x
empty() -> Void = () => {}

# 新代码（推荐）
mul: (Int, Int) -> Int = (a, b) => a * b
square: (Int) -> Int = (x) => x * x
empty: () -> Void = () => {}
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
add: (Int, Int) -> Int = (a, b) => a + b

# 泛型推断
first: [T](List[T]) -> Option[T] = (list) => {
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
first[T](ref List[T]) -> ref T = (list) => ref list[0]
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

YaoXiang 采用**纯函数式设计**，通过先进的绑定机制实现无缝的方法调用和柯里化，无需引入 `struct`、`class` 等关键字。

### 7.1 核心函数定义

所有操作都通过普通函数实现，第一个参数约定为操作的主体：

```yaoxiang
# === Point.yx (模块) ===

# 统一语法：构造器就是类型
type Point = Point(x: Float, y: Float)

# 核心函数：第一个参数是操作的主体
distance(Point, Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

add(Point, Point) -> Point = (a, b) => {
    Point(a.x + b.x, a.y + b.y)
}

scale(Point, Float) -> Point = (p, s) => {
    Point(p.x * s, p.y * s)
}

# 更复杂的函数
distance_with_scale(scale: Float, a: Point, b: Point) -> Float = (s, p1, p2) => {
    dx = (p1.x - p2.x) * s
    dy = (p1.y - p2.y) * s
    (dx * dx + dy * dy).sqrt()
}
```

### 7.2 基础方法绑定

#### 7.2.1 自动绑定（MoonBit风格）

YaoXiang 支持基于命名空间的自动绑定，**无需任何额外声明**：

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

# 核心函数
distance(Point, Point) -> Float = (a, b) => { ... }

# === main.yx ===

use Point

main() -> Void = () => {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)
    
    # ✅ 自动绑定：直接调用方法
    result = p1.distance(p2)  # 解析为 distance(p1, p2)
}
```

**自动绑定规则**：
- 在模块内定义的函数
- 如果第一个参数类型与模块名匹配
- 则自动支持方法调用语法

#### 7.2.2 无绑定选项（默认行为）

```yaoxiang
# === Vector.yx ===

type Vector = Vector(x: Float, y: Float, z: Float)

# 内部辅助函数，不希望自动绑定
dot_product_internal(v1: Vector, v2: Vector) -> Float = (a, b) => {
    a.x * b.x + a.y * b.y + a.z * b.z
}

# === main.yx ===

use Vector

main() -> Void = () => {
    v1 = Vector(1.0, 0.0, 0.0)
    v2 = Vector(0.0, 1.0, 0.0)
    
    # ❌ 无法绑定：非 pub 函数不会自动绑定
    # v1.dot_product_internal(v2)  # 编译错误！
    
    # ✅ 必须直接调用（在模块外部不可见）
}
```

### 7.3 基于位置的绑定语法

YaoXiang 提供**最优雅的绑定语法**，使用位置标记 `[n]` 来精确控制绑定位置：

#### 7.3.1 基本语法

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

# 核心函数
distance(Point, Point) -> Float = (a, b) => { ... }
add(Point, Point) -> Point = (a, b) => { ... }
scale(Point, Float) -> Point = (p, s) => { ... }

# 绑定语法：Type.method = func[position]
# 表示：调用方法时，将调用者绑定到 func 的 [position] 参数

Point.distance = distance[1]      # 绑定到第1个参数
Point.add = add[1]                 # 绑定到第1个参数
Point.scale = scale[1]             # 绑定到第1个参数
```

**语义解析**：
- `Point.distance = distance[1]`
  - `distance` 函数有两个参数：`distance(Point, Point)`
  - `[1]` 表示调用者绑定到第1个参数
  - 使用：`p1.distance(p2)` → `distance(p1, p2)`

#### 7.3.2 多位置联合绑定

```yaoxiang
# === Math.yx ===

# 函数：scale, point1, point2, extra1, extra2
calculate(scale: Float, a: Point, b: Point, x: Float, y: Float) -> Float = (s, p1, p2, x, y) => { ... }

# === Point.yx ===

type Point = Point(x: Float, y: Float)

# 绑定多个位置
Point.calc1 = calculate[1, 2]      # 绑定 scale 和 point1
Point.calc2 = calculate[1, 3]      # 绑定 scale 和 point2  
Point.calc3 = calculate[2, 3]      # 绑定 point1 和 point2

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. 绑定[1,2] - 剩余3,4,5
f1 = p1.calc1(2.0)  # 绑定 scale=2.0, point1=p1
# f1 现在需要 p2, x, y
result1 = f1(p2, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 2. 绑定[1,3] - 剩余2,4,5
f2 = p2.calc2(2.0)  # 绑定 scale=2.0, point2=p2
# f2 现在需要 point1, x, y
result2 = f2(p1, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)

# 3. 绑定[2,3] - 剩余1,4,5
f3 = p1.calc3(p2)  # 绑定 point1=p1, point2=p2
# f3 现在需要 scale, x, y
result3 = f3(2.0, 10.0, 20.0)  # calculate(2.0, p1, p2, 10.0, 20.0)
```

#### 7.3.3 剩余参数填入顺序

**核心规则**：绑定后，剩余参数按**原始函数的顺序**填入，跳过已绑定的位置。

```yaoxiang
# 假设函数：func(p1, p2, p3, p4, p5)

# 绑定第1和第3参数
Type.method = func[1, 3]

# 调用时：
method(p2_value, p4_value, p5_value)

# 映射为：
func(p1_bound, p2_value, p3_bound, p4_value, p5_value)
# 剩余参数：2,4,5 按原始顺序填入
```

#### 7.3.4 类型检查优势

```yaoxiang
# ✅ 合法绑定
Point.distance = distance[1]          # distance(Point, Point)
Point.calc = calculate[1, 2]          # calculate(scale, Point, Point, ...)

# ❌ 非法绑定（编译器报错）
Point.wrong = distance[5]             # 第5个参数不存在
Point.wrong = distance[0]             # 参数从1开始
Point.wrong = distance[1, 2, 3, 4]    # 超出函数参数个数
```

### 7.4 柯里化绑定的细粒度控制

```yaoxiang
# === Math.yx ===

distance_with_scale(scale: Float, a: Point, b: Point) -> Float = (s, p1, p2) => { ... }

# === Point.yx ===

type Point = Point(x: Float, y: Float)

# 绑定策略：灵活控制每个位置
Point.distance = distance[1]                    # 基础绑定
Point.distance_scaled = distance_with_scale[2]  # 绑定到第2参数

# === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 1. 基础自动绑定
d1 = p1.distance(p2)  # distance(p1, p2)

# 2. 绑定到不同位置
f = p1.distance_scaled(2.0)  # 绑定第2参数，剩余第1,3
result = f(p2)               # distance_with_scale(2.0, p1, p2)

# 3. 链式绑定
d2 = p1.distance(p2).distance_scaled(2.0)  # 链式调用
```

### 7.5 完整的绑定系统

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

# 核心函数
distance(Point, Point) -> Float = (a, b) => { ... }
add(Point, Point) -> Point = (a, b) => { ... }
scale(Point, Float) -> Point = (p, s) => { ... }

# 自动绑定（核心）
Point.distance = distance[1]
Point.add = add[1]
Point.scale = scale[1]

# === Math.yx ===

# 全局函数
multiply_by_scale(scale: Float, a: Point, b: Point) -> Float = (s, p1, p2) => { ... }

# === main.yx ===

use Point
use Math

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 使用
d = p1.distance(p2)          # distance(p1, p2)
r = p1.add(p2)               # add(p1, p2)
s = p1.scale(2.0)            # scale(p1, 2.0)

# 全局函数绑定
Point.multiply = multiply_by_scale[2]  # 绑定到第2参数
m = p1.multiply(2.0, p2)     # multiply_by_scale(2.0, p1, p2)
```

### 7.6 绑定的作用域与规则

#### 7.6.1 pub 的作用

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

# 非 pub 函数
internal_distance(a: Point, b: Point) -> Float = (a, b) => { ... }

# pub 函数
pub distance(a: Point, b: Point) -> Float = (a, b) => { ... }

# === main.yx ===

use Point

# 自动绑定只对 pub 函数有效
p1.distance(p2)      # ✅ distance 是 pub，可自动绑定
# p1.internal_distance(p2)  # ❌ 不是 pub，无法绑定
```

#### 7.6.2 模块内绑定

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

distance(Point, Point) -> Float = (a, b) => { ... }

# 在模块内部，所有函数都可见
# 但自动绑定只对 pub 导出的函数在外部有效

pub distance  # 导出，外部可用自动绑定
```

### 7.7 设计优势总结

| 特性 | 说明 |
|------|------|
| **零语法负担** | 自动绑定无需任何声明 |
| **位置精确控制** | `[n]` 精确指定绑定位置 |
| **多位置联合** | 支持 `[1, 2, 3]` 多参数绑定 |
| **类型安全** | 编译器验证绑定位置有效性 |
| **无关键字** | 无需 `bind` 或其他关键字 |
| **灵活柯里化** | 支持任意位置参数绑定 |
| **pub 控制** | 只有 pub 函数可外部绑定 |

### 7.8 与传统方法绑定的区别

| 传统语言 | YaoXiang |
|---------|----------|
| `obj.method(arg)` | `obj.method(arg)` |
| 需要类/方法定义 | 只需函数 + 绑定声明 |
| 语法 `class { method() {} }` | 语法 `Type.method = func[n]` |
| 继承、多态 | 纯函数式，无继承 |
| 方法表查找 | 编译时绑定，无运行时开销 |

**核心优势**：YaoXiang 的绑定是**编译时机制**，零运行时成本，同时保持了函数式编程的纯粹性和灵活性。

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

### 8.2 严格结构化语法

#### 8.2.1 声明语法的AI友好策略

```yaoxiang
# === AI代码生成最佳实践 ===

# ✅ 推荐：使用完整的新语法声明 + 类型集中约定
# AI可以准确理解意图，生成完整类型信息

add: (Int, Int) -> Int = (a, b) => a + b
inc: Int -> Int = x => x + 1
empty: () -> Void = () => {}

# ❌ 避免：省略类型标注或类型分散
# AI无法确定参数类型，可能生成错误代码
add = (a, b) => a + b          # 参数无类型
identity = x => x              # 参数无类型
add2 = (a: Int, b: Int) => a + b  # 类型分散在实现中

# ⚠️ 兼容：旧语法仅用于维护
# AI应优先生成新语法 + 类型集中约定
mul(Int, Int) -> Int = (a, b) => a * b  # 不推荐在新代码中使用
```

**类型集中约定的AI优势：**

1. **签名一目了然**：AI只需读声明行就能理解完整函数签名
2. **修改更安全**：修改类型只需改声明，不影响实现体
3. **生成更简单**：AI可以先生成声明，再填充实现
4. **柯里化友好**：清晰的柯里化类型签名便于AI处理

```yaoxiang
# AI处理示例
# 输入：实现体 (a, b) => a + b
# AI看到声明：add: (Int, Int) -> Int
# 结论：参数类型是 Int, Int，返回类型是 Int

# 对比：如果类型分散
# 输入：实现体 (a: Int, b: Int) => a + b
# AI需要：分析实现体提取类型信息
# 结果：更复杂的处理逻辑，容易出错
```

#### 8.2.2 双语法策略与AI

| 语法类型 | AI生成策略 | 使用场景 |
|---------|-----------|---------|
| **新语法** | ✅ 优先生成，完整类型信息 | 所有新代码生成 |
| **旧语法** | ⚠️ 仅在维护旧代码时使用 | 历史代码修改 |
| **无标注** | ❌ 避免生成 | 任何情况都不应生成 |

#### 8.2.3 语法边界明确

```yaoxiang
# AI友好的代码块边界

# ✅ 清晰的开始和结束标记
function_name: (Type1, Type2) -> ReturnType = (param1, param2) => {
    # 函数体
    if condition {
        do_something()
    } else {
        do_other()
    }
}

# ✅ 条件语句必须有花括号
if condition {
    # 条件体
}

# ✅ 类型定义明确
type MyType = Type1 | Type2

# ❌ 避免的模糊写法
if condition    # 缺少花括号
    do_something()
```

#### 8.2.4 无歧义语法约束

```yaoxiang
# AI生成时必须遵守的约束

# 1. 禁止省略括号
# ✅ 正确
foo: (T) -> T = (x) => x
my_list = [1, 2, 3]

# ❌ 错误（禁止）
foo T { T }             # 参数必须有括号
my_list = [1 2 3]       # 列表必须有逗号

# 2. 必须显式返回类型或可推断的形式
# ✅ 正确
get_num: () -> Int = () => 42
get_num2: () = () => 42          # 返回类型可推断

# ❌ 错误
get_bad = () => { 42 }           # 块中无return，无法推断

# 3. 参数必须有类型标注（新语法）
# ✅ 正确
add: (Int, Int) -> Int = (a, b) => a + b
inc: Int -> Int = x => x + 1

# ❌ 错误
add = (a, b) => a + b            # 参数无类型
identity = x => x                # 参数无类型
```

#### 8.2.5 AI生成推荐模式

```yaoxiang
# AI生成函数时的标准模板

# 模式1：完整类型标注
function_name: (ParamType1, ParamType2, ...) -> ReturnType = (param1, param2, ...) => {
    # 函数体
    return expression
}

# 模式2：返回类型推断
function_name: (ParamType1, ParamType2) = (param1, param2) => {
    # 函数体
    return expression
}

# 模式3：单参数简写
function_name: ParamType -> ReturnType = param => expression

# 模式4：无参函数
function_name: () -> ReturnType = () => expression

# 模式5：空函数
function_name: () -> Void = () => {}
```

### 8.3 错误消息的AI友好性

```yaoxiang
# 错误消息应该提供明确的修正建议

# 不友好的错误
# Syntax error at token 'a'

# AI友好的错误
# Missing type annotation for parameter 'a'
# Suggestion: add ': Int' or similar type to '(a, b) => a + b'
# Correct version: add: (Int, Int) -> Int = (a, b) => a + b
```

---

## 九、类型集中约定（核心设计哲学）

### 9.1 约定概述

YaoXiang 的核心设计约定是**"声明优先，类型集中"**。这个约定是语言AI友好性和开发效率的基石。

```yaoxiang
# ✅ 核心约定：类型信息统一在声明行
add: (Int, Int) -> Int = (a, b) => a + b

# ❌ 避免：类型信息分散在实现中
add = (a: Int, b: Int) => a + b
```

### 9.2 约定的五个核心优势

#### 1. 语法一致性
```yaoxiang
# 所有声明都遵循相同格式
x: Int = 42                           # 变量
name: String = "YaoXiang"             # 变量
add: (Int, Int) -> Int = (a, b) => a + b  # 函数
inc: Int -> Int = x => x + 1          # 函数
type Point = Point(x: Float, y: Float) # 类型
```

#### 2. 声明与实现分离
```yaoxiang
# 声明行提供完整类型信息
add: (Int, Int) -> Int = (a, b) => a + b
# └────────────────────┘
#   完整的函数签名

# 实现体专注业务逻辑
# (a, b) => a + b  不需要关心类型，只需要实现功能
```

#### 3. AI友好性
```yaoxiang
# AI处理流程：
# 1. 读声明行 → 完整理解函数签名
# 2. 生成实现 → 无需分析类型推断
# 3. 修改类型 → 只改声明行，不影响实现

# 对比：类型分散方式
add = (a: Int, b: Int) => a + b
# AI需要：分析实现体提取类型信息 → 更复杂，易出错
```

#### 4. 修改更安全
```yaoxiang
# 修改参数类型
# 原来: add: (Int, Int) -> Int = (a, b) => a + b
# 修改: add: (Float, Float) -> Float = (a, b) => a + b
# 实现体: (a, b) => a + b  无需修改！

# 如果类型分散：
# 原来: add = (a: Int, b: Int) => a + b
# 修改: add = (a: Float, b: Float) => a + b  # 需要改两处
```

#### 5. 柯里化友好
```yaoxiang
# 柯里化类型一目了然
add_curried: Int -> Int -> Int = a => b => a + b
#              └─────────────┘
#              柯里化签名

# 函数组合作为一等公民
compose: (Int -> Int, Int -> Int) -> Int -> Int = (f, g) => x => f(g(x))
```

### 9.3 约定的实施规则

#### 规则1：参数必须在声明中指定类型
```yaoxiang
# ✅ 正确
add: (Int, Int) -> Int = (a, b) => a + b

# ❌ 错误
add = (a, b) => a + b            # 参数类型缺失
identity = x => x                # 参数类型缺失
```

#### 规则2：返回类型可推断但推荐标注
```yaoxiang
# ✅ 推荐：完整标注
get_num: () -> Int = () => 42

# ✅ 可接受：返回类型推断
get_num: () = () => 42

# ✅ 空函数推断为 Void
empty: () = () => {}
```

#### 规则3：Lambda内部类型注解是临时的
```yaoxiang
# ✅ 正确：依赖声明中的类型
add: (Int, Int) -> Int = (a, b) => a + b

# ⚠️ 可以但不推荐：Lambda内重复标注
add: (Int, Int) -> Int = (a: Int, b: Int) => a + b

# ❌ 错误：缺少声明标注
add = (a: Int, b: Int) => a + b
```

#### 规则4：旧语法遵循相同理念
```yaoxiang
# 旧语法也应尽量在声明位置提供类型信息
# 虽然格式不同，但理念一致：
# - 声明行包含主要类型信息
# - 实现体相对简洁
add(Int, Int) -> Int = (a, b) => a + b
```

### 9.4 约定与类型推断的关系

```yaoxiang
# 约定不阻止类型推断，而是引导推断方向

# 1. 完整标注（不推断）
add: (Int, Int) -> Int = (a, b) => a + b

# 2. 部分推断（声明提供参数类型）
add: (Int, Int) = (a, b) => a + b  # 返回类型推断

# 3. 空函数推断
empty: () = () => {}  # 推断为 () -> Void
```

### 9.5 约定的AI实现优势

**AI代码生成流程：**

1. **读取需求** → 生成声明
   ```
   需求：加法函数
   生成：add: (Int, Int) -> Int = (a, b) => ???
   ```

2. **填充实现** → 无需类型分析
   ```
   实现：add: (Int, Int) -> Int = (a, b) => a + b
   ```

3. **类型修改** → 只改声明
   ```
   修改：add: (Float, Float) -> Float = (a, b) => a + b
   实现：(a, b) => a + b  保持不变
   ```

**对比无约定的AI处理：**
```
需求：加法函数
AI需要：
  1. 推断参数类型
  2. 推断返回类型
  3. 生成实现体
  4. 验证一致性
  5. 处理类型变化时的复杂更新

结果：更复杂，更容易出错
```

### 9.6 约定的哲学意义

这种约定体现了 YaoXiang 的核心理念：

- **声明即文档**：声明行就是完整的函数文档
- **类型即契约**：类型信息是调用者和实现者之间的契约
- **逻辑即实现**：实现体只关注"做什么"，不关注"什么类型"
- **工具即辅助**：类型系统、AI工具都可以基于清晰的声明工作

### 9.7 实际应用对比

#### 完整示例：计算器模块

```yaoxiang
# === 推荐做法：类型集中约定 ===

# 模块声明
pub add: (Int, Int) -> Int = (a, b) => a + b
pub multiply: (Int, Int) -> Int = (a, b) => a * b

# 高阶函数
pub apply_twice: (Int -> Int, Int) -> Int = (f, x) => f(f(x))

# 柯里化函数
pub make_adder: Int -> (Int -> Int) = x => y => x + y

# 泛型函数
pub map: [A, B]((A) -> B, List[A]) -> List[B] = (f, xs) => case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)

# 类型定义
type Point = Point(x: Float, y: Float)
pub distance: (Point, Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# === 不推荐做法：类型分散 ===

# 参数类型在Lambda中
add = (a: Int, b: Int) => a + b
multiply = (a: Int, b: Int) => a * b

# 高阶函数类型分散
apply_twice = (f: (Int) -> Int, x: Int) => f(f(x))

# 柯里化类型分散
make_adder = (x: Int) => (y: Int) => x + y

# 泛型类型分散
map = [A, B](f: (A) -> B, xs: List[A]) => List[B] => case xs of
  [] => []
  (x :: rest) => f(x) :: map(f, rest)
```

#### 代码维护对比

```yaoxiang
# 需求：将 add 从 Int 改为 Float

# === 推荐做法：只需改声明行 ===
# 原来
add: (Int, Int) -> Int = (a, b) => a + b

# 修改后
add: (Float, Float) -> Float = (a, b) => a + b
#              ↑↑↑↑↑↑↑↑↑          ↑↑↑↑↑↑↑
#              声明行修改          实现体保持不变

# === 不推荐做法：需要改多处 ===
# 原来
add = (a: Int, b: Int) => a + b

# 修改后
add = (a: Float, b: Float) => a + b
#     ↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑↑
#     所有参数类型都需要修改
```

#### AI辅助编程对比

```yaoxiang
# AI需求：实现一个函数，计算两点间的曼哈顿距离

# === AI看到推荐写法时 ===
type Point = Point(x: Float, y: Float)
pub manhattan: (Point, Point) -> Float = ???  # AI直接知道完整签名

# AI生成：
pub manhattan: (Point, Point) -> Float = (a, b) => {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}

# === AI看到不推荐写法时 ===
type Point = Point(x: Float, y: Float)
pub manhattan = ???  # AI需要推断：参数类型？返回类型？

# AI可能生成：
pub manhattan = (a: Point, b: Point) => Float => {
    (a.x - b.x).abs() + (a.y - b.y).abs()
}
# 或者可能出错，因为类型信息不完整
```

### 9.8 约定实施检查清单

在编写YaoXiang代码时，可以使用以下检查清单：

- [ ] 所有函数声明都有完整的类型标注在声明行
- [ ] 参数类型在声明中指定，不在Lambda中
- [ ] 返回类型尽可能在声明中标注
- [ ] 变量声明使用 `name: Type = value` 格式
- [ ] Lambda体保持简洁，不重复类型信息
- [ ] 使用新语法而非旧语法
- [ ] 复杂类型使用type定义，保持声明清晰

---

## 十、快速入门

### 10.1 Hello World

```yaoxiang
# hello.yx
use std.io

main: () -> Void = () => {
    println("Hello, YaoXiang!")
}
```

运行方式：`yaoxiang hello.yx`

输出：
```
Hello, YaoXiang!
```

### 10.2 基本语法

```yaoxiang
# 变量与类型
x = 42                    # 自动推断为 Int
name = "YaoXiang"         # 自动推断为 String
pi = 3.14159              # 自动推断为 Float

# 函数（使用新语法）
add: (Int, Int) -> Int = (a, b) => a + b

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

### 10.3 方法绑定示例

```yaoxiang
# === Point.yx ===

type Point = Point(x: Float, y: Float)

# 核心函数
distance(Point, Point) -> Float = (a, b) => {
    dx = a.x - b.x
    dy = a.y - b.y
    (dx * dx + dy * dy).sqrt()
}

# 自动绑定
Point.distance = distance[1]

# === main.yx ===

use Point

main() -> Void = () => {
    p1 = Point(3.0, 4.0)
    p2 = Point(1.0, 2.0)
    
    # 使用绑定
    d = p1.distance(p2)  # distance(p1, p2)
    print(d)
}
```

### 10.4 柯里化绑定示例

```yaoxiang
# === Math.yx ===

distance_with_scale(scale: Float, a: Point, b: Point) -> Float = (s, p1, p2) => {
    dx = (p1.x - p2.x) * s
    dy = (p1.y - p2.y) * s
    (dx * dx + dy * dy).sqrt()
}

# === Point.yx ===

type Point = Point(x: Float, y: Float)

Point.distance_scaled = distance_with_scale[2]  # 绑定到第2参数

# === main.yx ===

use Point

p1 = Point(3.0, 4.0)
p2 = Point(1.0, 2.0)

# 使用绑定
f = p1.distance_scaled(2.0)  # 绑定 scale 和 p1
result = f(p2)               # 最终调用

# 或直接使用
result2 = p1.distance_scaled(2.0, p2)
```

### 10.5 下一步

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

## 版本历史

| 版本 | 日期 | 作者 | 变更说明 |
|------|------|------|---------|
| v1.0.0 | 2024-12-31 | 晨煦 | 初始版本 |
| v1.1.0 | 2025-01-04 | 沫郁酱 | 修正泛型语法为 `[T]`（而非 `<T>`）；移除 `fn` 关键字；更新函数定义示例 |

---

> 「道生一，一生二，二生三，三生万物。」
> —— 《道德经》
>
> 类型如道，万物皆由此生。
