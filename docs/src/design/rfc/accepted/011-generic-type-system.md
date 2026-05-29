---
title: RFC-011：泛型系统设计
---

# RFC-011: 泛型系统设计 - 零成本抽象与宏替代

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-25
> **最后更新**: 2026-04-22（更新为 Type 自描述机制，统一泛型调用语法）

## 摘要

本文档定义YaoXiang语言的**泛型系统设计**，通过强大的泛型能力实现零成本抽象，利用编译期优化减少对宏的依赖，并提供死代码消除机制。

**核心设计**：
- **统一签名语法**：`(T: Type, R: Type) -> ...` 泛型参数与普通参数统一
- **Type 自描述机制**：`Type` 是语言级特殊存在，签名中的 `Type` 位置可自动推断填充
- **类型约束**：`T: Clone + Add` 多重约束，函数类型约束
- **关联类型**：`Iterator: (Item: Type) -> Type = { next: () -> Option(Item), has_next: () -> Bool }`
- **编译期泛型**：`N: Int` 泛型值参数，编译期常量实例化
- **条件类型**：`If: (C: Bool, T: Type, E: Type) -> Type` 类型级计算，类型族

**价值**：
- 零成本抽象：编译期单态化，无运行时开销
- 死代码消除：实例化图分析 + LLVM优化
- 宏替代：泛型替代90%的宏使用场景
- 类型安全：编译期检查，IDE友好
- **显式优于隐式**：`Type` 自描述，编译器自动推断

## 参考文档

本文档的设计基于以下文档：

| 文档 | 关系 | 说明 |
|------|------|------|
| [RFC-010: 统一类型语法](./010-unified-type-syntax.md) | **语法基础** | 泛型语法与统一 `name: type = value` 模型集成 |
| [RFC-010: 统一类型语法](./010-unified-type-syntax.md) | **调用语法** | 第6节：泛型调用语法——统一 `()` 应用，`[]` 彻底移除 |
| [RFC-009: 所有权模型](./accepted/009-ownership-model.md) | **类型系统** | Move语义与泛型的自然结合 |
| [RFC-001: 并作模型](./accepted/001-concurrent-model-error-handling.md) | **执行模型** | DAG分析与泛型类型检查 |
| [RFC-008: 运行时模型](./accepted/008-runtime-concurrency-model.md) | **编译器架构** | 泛型单态化与编译期优化策略 |
| [类型宇宙思想](../reference/plan/ongoing/类型宇宙思想.md) | **理论核心** | 类型宇宙层级模型与值依赖类型设计 |
| [RFC-022: 霍尔逻辑静态验证](./draft/022-hol-logic-verification.md) | **终止检查** | decreases规约与编译期求值安全保障 |

## 类型宇宙思想与值依赖类型

YaoXiang 的泛型系统建立在**类型宇宙思想**之上，这一心智模型将语言中的所有概念统一为分层结构，核心创新在于将**值依赖类型**提升为 Type2 层的一等公民。

### 什么是值依赖类型？

**值依赖类型**是一种类型，它依赖于一个或多个**值**（而非仅依赖于其他类型）。这些值可以在编译期求值，从而在编译阶段就提供类型安全保证。

```yaoxiang
# 传统泛型：类型参数
List: (T: Type) -> Type

# 值依赖类型：值参数
Vec: (n: Int) -> Type  # 向量类型依赖于长度值 n
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type  # 矩阵类型依赖于行数和列数
```

### 值依赖类型的核心优势

相比传统泛型，YaoXiang 的值依赖类型具有以下核心优势：

| 特性 | 传统泛型 (C++/Rust) | YaoXiang 值依赖类型 |
|------|-------------------|---------------------|
| 类型依赖的值 | 仅依赖类型参数 | 可依赖任何值，包括函数调用结果 |
| 编译期求值 | C++模板手动特化，Rust无 | 自动编译期求值，保证终止 |
| 类型级计算 | 模板元编程（复杂/危险） | 统一的类型级计算引擎 |
| 类型安全 | C++无，Rust受限 | 完整类型安全，编译期检查 |
| 维度验证 | 运行时检查或手动特化 | 编译期维度验证，无运行时开销 |

### 类型宇宙层级与值依赖类型

类型宇宙思想将语言概念按语义角色划分为不同层级，值依赖类型位于 **Type2 层**：

| 层级 | 角色 | 示例 |
|------|------|------|
| Type-1 | 值 | `42`, `factorial(5)`, 函数本身 |
| Type0 | 元类型关键字 | `Type` |
| Type1 | 具体类型 | `Int`, `String`, `Vec(3)` |
| **Type2** | **函数/类型构造器/值依赖类型** | `add: (Int, Int) -> Int`, `Vec: (n: Int) -> Type`, `Matrix: (T: Type, Rows: Int, Cols: Int) -> Type` |

**关键设计**：Type2 层的函数、类型构造器和值依赖类型**统一语法**，都是 `(params) -> result` 的形式：
- 普通函数：`(Int, Int) -> Int` → 返回值是值
- 类型构造器：`(T: Type) -> Type` → 返回值是类型
- 值依赖类型：`(n: Int) -> Type` → 返回值是类型，但依赖于值参数

> **Curry-Howard 同构**：这种统一不是巧合。Curry-Howard 同构指出"类型即命题，程序即证明"——函数类型 `A → B` 对应逻辑蕴含"若 A 则 B"，泛型 `(T: Type) -> Type` 对应全称量化"对所有类型 T"，值依赖类型 `(n: Int) -> Type` 对应"对每个整数 n 存在一个类型"。YaoXiang 将函数、类型构造器和值依赖类型统一到 Type2 层，本质上是将"证明"和"计算"统一为同一概念——**构造性证明**。这正是 Curry-Howard 同构在语言设计中的直接体现：一种形式（`(params) -> result`）同时承载逻辑命题和计算过程。

### 编译期确定性保证

YaoXiang 的类型宇宙思想要求：**Type 层级的一切都是编译期确定的**。

```yaoxiang
# 编译期维度验证示例
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
    # 编译期检查：维度必须为正
    _assert: Assert[Rows > 0],
    _assert: Assert[Cols > 0],
}

# 创建 3x3 单位矩阵 - 编译期完成
identity: (T: Add + Zero + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    # ...
}

# 编译期计算：factorial(3) = 6，向量大小在编译期确定
vec: Vec(factorial(3)) = Vec(6)()
```

编译器会自动：
1. 检测类型位置上的函数调用
2. 验证函数是否标记了 `decreases` 规约（见下方终止检查机制）
3. 在编译期执行求值
4. 将结果嵌入生成的类型

### 值依赖类型的应用场景

#### 编译期维度验证
```yaoxiang
# 矩阵乘法：编译期验证维度匹配
multiply: (T: Add + Multiply + Zero,
           Rows: Int, Cols: Int, M: Int) -> ((
    a: Matrix(T, Rows, Cols),
    b: Matrix(T, Cols, M)
) -> Matrix(T, Rows, M)) = {
    # 编译期检查：a.Cols == b.Rows，否则编译错误
    result = Matrix(T, Rows, M)()
    # ...
}

# 错误在编译期捕获：
# multiply(matrix_2x3, matrix_4x2)  # 编译错误：2 != 4
```

#### 类型安全的数组大小
```yaoxiang
# 数组大小是编译期常量
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: N,
}

# N 是编译期常量，可以用于类型级计算
first_three: Array(Int, 3) = Array(Int, 3)(1, 2, 3)
# first_three.length == 3（编译期已知）
```

#### 条件类型
```yaoxiang
# 类型级If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# 类型族
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,
}
```

#### 泛型函数
```yaoxiang
# map: 泛型函数，类型参数 T, R 在编译期确定
map: (T: Type, R: Type) -> (
    (list: List(T), f: (x: T) -> R) -> List(R)
) = (list, f) => {
    result = List(R)()
    for x in list {
        result.push(f(x))
    }
    result
}

# 使用时完全透明，类型自动推导
numbers = List(1, 2, 3)
doubled = map(numbers, (x) => x * 2)  # 推导为 map[Int, Int]
```

### 与其他语言的对比

| 特性 | C++模板 | Rust泛型 | Haskell GADT | **YaoXiang** |
|------|---------|----------|--------------|--------------|
| 类型参数 | ✅ | ✅ | ✅ | ✅ |
| 值依赖类型 | ❌ | ❌ | ✅ | ✅ |
| 编译期求值 | 模板实例化 | ❌ | ✅ | ✅ |
| 终止保证 | ❌ | ❌ | ❌（危险） | ✅（decreases规约） |
| 类型安全 | ❌（宏展开） | ✅ | ✅ | ✅ |
| 统一语法 | ❌ | ❌ | ❌ | ✅ |
| 编译期维度验证 | 手动特化 | 运行时检查 | 类型族 | 编译期自动验证 |
| decreases规约 | ❌ | ❌ | ❌ | ✅ |

### 终止检查机制（与RFC-022集成）

值依赖类型的编译期求值必须**保证终止**，否则类型系统将陷入无限循环。YaoXiang 通过 **decreases 规约** 确保这一点，与 RFC-022 无缝集成。

#### 递归函数的终止规约
```yaoxiang
# 编译期阶乘：必须证明终止
factorial: (n: Int) -> Int = {
    //! requires: n >= 0
    //! ensures: result == n!
    //! decreases: n    # 每次递归 n 严格递减
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 使用：在类型位置调用
vec: Vec(factorial(5)) = Vec(120)()  # 编译期求值 factorial(5) = 120
```

#### 循环的终止规约
```yaoxiang
sum: (arr: Array(Int, n)) -> Int = {
    s = 0; i = 0
    while i < n {
        /*! invariant: s == sum(arr[0..i]) && 0 <= i <= n !*/
        /*! decreases: n - i !*/
        s += arr[i]; i += 1
    }
    return s
}
```

#### 终止检查的工作流程

```
┌─────────────────────────────────────────────────────────────┐
│  类型检查阶段                                                │
│  遇到类型位置上的函数调用（如 Vec(factorial(5))）            │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  1. 检查 decreases 规约                                      │
│     - 有 decreases: 验证递减条件在所有递归路径上成立           │
│     - 无 decreases 但明显可终止: 直接求值                      │
│     - 无 decreases 且可能不终止: 编译错误                     │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  2. 编译期求值（由内置解释器执行）                           │
│     - 纯函数：直接求值                                       │
│     - 副作用：编译错误（类型位置必须无副作用）                │
└─────────────────────────┬───────────────────────────────────┘
                          ▼
┌─────────────────────────────────────────────────────────────┐
│  3. 结果嵌入类型                                            │
│     - Vec(factorial(5)) → Vec(120)                          │
│     - Matrix(Float, 3, 3) → 具体类型                        │
└─────────────────────────────────────────────────────────────┘
```

#### 优势

- **安全性**：确保编译期求值必然终止，避免类型系统陷入无限循环
- **统一性**：终止检查与部分正确性验证共享同一套规约机制
- **渐进增强**：可从运行时检查逐步过渡到完全静态证明

## 动机

### 为什么需要强泛型系统？

当前主流语言的泛型存在局限：

| 语言 | 泛型能力 | 问题 |
|------|----------|------|
| Java | 边界类型 | 编译期单态化，无泛型特化 |
| C# | 泛型约束 | 运行时类型检查，有性能开销 |
| Rust | 泛型 + Trait | Trait系统复杂，学习曲线陡峭 |
| C++ | 模板 | 模板特化复杂，编译错误信息差 |
| **YaoXiang** | **值依赖类型** | **类型可依赖值，编译期维度验证，终止保证** |

### 核心矛盾

1. **性能 vs 灵活性**：运行时灵活性 vs 编译期优化
2. **复杂 vs 简洁**：强大的类型系统 vs 易用性
3. **宏 vs 泛型**：宏代码生成 vs 泛型类型安全
4. **值依赖 vs 类型安全**：传统泛型无法在编译期验证维度

### 值依赖类型的核心优势

YaoXiang 的**值依赖类型**是相对于传统泛型的核心优势：

| 优势 | 说明 |
|------|------|
| **类型依赖值** | `Vec: (n: Int) -> Type` 让类型依赖于具体的值 |
| **编译期求值** | 类型位置的函数调用在编译期求值，结果直接嵌入类型 |
| **维度验证** | `Matrix(Float, 3, 3)` 在编译期验证矩阵维度 |
| **类型级计算** | `If`, `Match` 等条件类型支持类型级计算 |
| **终止保证** | decreases 规约确保编译期求值必然终止 |

```yaoxiang
# C++/Rust 无法做到的编译期验证
matrix: Matrix(Float, factorial(3), factorial(2)) = ...
# 编译期计算：factorial(3) = 6, factorial(2) = 2
# 类型为 Matrix(Float, 6, 2)

# 维度不匹配在编译期捕获
identity: Matrix(Float, 3, 3) = ...
# multiply(matrix_2x3, identity_3x3)  # 编译错误：2 != 3
```

### 泛型系统的价值

```yaoxiang
# 示例：统一API设计
# 不同容器类型的map操作

# 传统方案：每个类型单独实现
map_int_array: (array: Array(Int), f: Fn(Int) -> Int) -> Array(Int) = ...
map_string_array: (array: Array(String), f: Fn(String) -> String) -> Array(String) = ...
map_int_list: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_string_list: (list: List(String), f: Fn(String) -> String) -> List(String) = ...

# 泛型方案：一个泛型函数覆盖所有类型
map: (T: Type, R: Type)(container: Container(T), f: Fn(T) -> R) -> Container(R) = {
    for item in container {
        result.push(f(item))
    }
    result
}
```

## 设计目标

### 核心目标

1. **零成本抽象** - 泛型调用等价于具体类型调用
2. **死代码消除** - 编译期分析，只实例化被使用的泛型
3. **宏替代** - 泛型替代90%的宏使用场景
4. **类型安全** - 编译期检查，无运行时类型开销
5. **IDE友好** - 智能提示，清晰错误信息
6. **值依赖类型** - 类型可依赖值，支持编译期维度验证
7. **编译期求值安全** - 通过 decreases 规约保证编译期求值终止

### 设计原则

- **编译期确定**：泛型参数在编译期确定
- **单态化优先**：生成具体代码，避免虚函数调用
- **约束驱动**：类型约束指导实例化
- **平台优化**：特化支持平台特定优化
- **类型宇宙统一**：函数/类型构造器/值依赖类型统一为 Type2 层
- **终止保证**：类型位置的函数调用必须证明终止

## 提案

### 1. 基础泛型

#### 1.1 泛型类型参数

> **关键规则**：泛型类型定义**必须显式标注 `: Type`**，否则会被 HM 推断为函数。
>
> | 写法 | 含义 |
> |------|------|
> | `List: (T: Type) -> Type = {...}` | ✅ 类型构造器 |
> | `List = {...}` | ❌ HM 推断为函数，不是类型 |

```yaoxiang
# 泛型类型定义（必须有 : Type）
Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self
}

Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    get: (self: List(T), index: Int) -> Option(T),
}

# 泛型函数（无 : Type，HM 推断为函数）
map: (T: Type, R: Type) -> ((opt: Option(T), f: Fn(T) -> R) -> Option(R)) = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# 泛型约束（直接表达式，单行可省略 return）
clone: (T: Clone)(value: T) -> T = value.clone()

# 多类型参数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

### 泛型函数调用语法

#### 1.1 统一签名语法

```yaoxiang
# 泛型函数使用统一的 (T: Type, R: Type) 签名语法
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = ...

# 多类型参数
combine: (T: Type, U: Type) -> ((a: T, b: U) -> (T, U)) = (a, b)
```

#### 1.2 Type 自描述机制

`Type` 是语言级特殊存在，编译器天然能识别签名中的 `Type` 位置，并自动从实际参数类型推断填充。

```yaoxiang
# 编译器自动推断泛型参数
numbers: List(Int) = List(Int)
#         ^^^^^^^^   ^^^^^^
#         类型声明   构造调用：Int 填充 T

# 函数调用推断
numbers: List(Int) = List(Int)
f: (x: Int) -> String = (x) => x.to_string()
strings: List(String) = map(numbers, f)
# 编译器推断：T=Int, R=String
```

#### 1.3 单态化

```yaoxiang
# 源代码
map: (T: Type, R: Type) -> ((list: List(T), f: (x: T) -> R) -> List(R)) = {
    result: List(R) = List(R)()
    for x in list {
        result.push(f(x))
    }
    return result
}

# 使用点
int_list: List(Int) = List(Int)
doubled: List(Int) = map(int_list, (x: Int) => x * 2)  # 实例化 map[Int, Int]

string_list: List(String) = List(String)
uppercased: List(String) = map(string_list, (s: String) => s.to_uppercase())  # 实例化 map[String, String]

# 编译后（等价代码）
map_Int_Int: (list: List(Int), f: (Int) -> Int) -> List(Int) = {
    result: List(Int) = List(Int)
    for x in list {
        result.push(f(x))
    }
    return result
}

map_String_String: (list: List(String), f: (String) -> String) -> List(String) = {
    result: List(String) = List(String)
    for s in list {
        result.push(f(s))
    }
    return result
}
```

#### 1.4 显式填充（当推断失败时）

```yaoxiang
# 可推断时省略 Type 参数
numbers: List(Int) = List(Int)
strings: List(String) = map(numbers, (x: Int) => x.to_string())

# 无法推断时必须显式填充
# map(numbers, (x) => x)  # ❌ Error: Cannot infer R

### 2. 类型约束系统

#### 2.1 单一约束

```yaoxiang
# 基本trait定义（接口类型）
Clone: Type = {
    clone: (Self) -> Self,
}

Display: Type = {
    fmt: (Self, Formatter) -> Result,
}

Debug: Type = {
    fmt: (Self, Formatter) -> Result,
}

# 使用约束：在签名中直接声明类型约束
clone: (T: Clone) -> (value: T) -> T = value.clone()

debug_print: (T: Debug)(value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
```

#### 2.2 多重约束

```yaoxiang
# 多重约束语法
combine: (T: Clone + Add)(a: T, b: T) -> T = {
    a.clone() + b
}

# 泛型容器的排序
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T) = {
    # 实现排序算法
    result: List(T) = list.clone()
    quicksort(&mut result)
    return result
}

# 函数类型约束
map: (T: Type, R: FnMut(T))(array: Array(T), f: R) -> Array(R) = {
    result: Array(R) = Array()
    for item in array {
        result.push(f(item))
    }
    return result
}

# 使用
doubled: Array(Int) = map(Array(1, 2, 3), (x: Int) => x * 2)  # 编译器推断
```

#### 2.3 函数类型约束

```yaoxiang
# 高阶函数约束
call_twice: (T: Type, F: Fn() -> T)(f: F) -> (T, T) = (f(), f())

call_with_arg: (T: Type, U: Type, F: Fn(T) -> U)(arg: T, f: F) -> U = f(arg)

compose: (A: Type, B: Type, C: Type, F: Fn(A) -> B, G: Fn(B) -> C)(a: A, f: F, g: G) -> C = g(f(a))

# 使用示例
result: Int = call_with_arg(42, (x: Int) => x * 2)  # result = 84
composed: String = compose(
    "hello",
    (s: String) => s.to_uppercase(),
    (s: String) => s + " WORLD"
)  # composed = "HELLO WORLD"
```

### 3. 关联类型

#### 3.1 关联类型定义

```yaoxiang
# Iterator trait（使用 (Item: Type) -> Type 语法）
Iterator: (Item: Type) -> Type = {
    next: (Self) -> Option(Item),
    has_next: (Self) -> Bool,
    collect: (T: Type)(Self) -> List(T),
}

# 使用
collect_all: (T: Type, I: Iterator(T))(iter: I) -> List(T) = {
    result: List(T) = List(T)
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}

# Array的Iterator实现
# 使用方法语法糖：Array.Item, Array.next, Array.has_next
Array.has_next: (T: Type)(self: Array(T)) -> Bool = {
    return self.index < self.length
}

Array.next: (T: Type)(self: Array(T)) -> Option(T) = {
    if has_next(self) {
        item = self.data[self.index]
        self.index = self.index + 1
        return Option.some(item)
    } else {
        return Option.none()
    }
}

Array.Item: (T: Type)(arr: Array(T)) -> T = {
    return arr.data[0]
}
```

#### 3.2 泛型关联类型（GAT）

```yaoxiang
# 更复杂的关联类型
Producer: (Item: Type) -> Type = {
    Item: T,
    produce: (Self) -> Option(Item),
}

# 关联类型可以是泛型的
Container: (Item: Type) -> Type = {
    Item: T,
    IteratorType: Iterator(Item),  # 关联类型也是泛型的
    iter: (Self) -> IteratorType,
}

# 使用
process_container: (T: Type, C: Container(T))(container: C) -> List(T) = {
    container.iter().collect()
}
```

### 4. 编译期泛型

#### 4.1 编译期常量参数

**核心设计**：泛型签名中的 `Type` 标记编译期类型参数，`Int` 等值参数在泛型上下文中默认编译期可确定。无需 `const` 关键字。

```yaoxiang
# ════════════════════════════════════════════════════════
# 编译期常量参数：泛型中的 Int 默认编译期确定
# ════════════════════════════════════════════════════════

# 编译期阶乘：N 必须是编译期已知的字面量
factorial: (N: Int) -> (n: N) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# 编译期加法
add: (a: Int, b: Int) -> (a: a, b: b) -> Int = a + b

# ════════════════════════════════════════════════════════
# 编译期常量数组
# ════════════════════════════════════════════════════════
StaticArray: (T: Type, N: Int) -> Type = {
    data: Array(T, N),  # 编译期已知大小的数组
    length: N,
}

# 使用方式
arr: StaticArray(Int, factorial(5))  # StaticArray(Int, 120)，编译器在编译期计算
```

#### 4.2 编译期计算

```yaoxiang
# ════════════════════════════════════════════════════════
# 编译期计算示例
# ════════════════════════════════════════════════════════

# 编译器在编译期计算字面量类型的函数调用
SIZE: Int = factorial(5)  # 编译期为 120

# 矩阵类型使用
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),
}

# 编译期维度验证
identity_matrix: (T: Add + Zero + One, N: Int)(size: N) -> Matrix(T, N, N) = {
    matrix: Matrix(T, N, N) = Matrix(T, N, N)()
    for i in 0..size {
        for j in 0..size {
            if i == j {
                matrix.data[i][j] = One::one()
            } else {
                matrix.data[i][j] = Zero::zero()
            }
        }
    }
    matrix
}

# 使用：编译期计算，生成 Matrix(Float, 3, 3)
identity_3x3: Matrix(Float, 3, 3) = identity_matrix(Float, 3)(3)
```

#### 4.3 编译期验证（标准库实现）

```yaoxiang
# ════════════════════════════════════════════════════════
# 标准库实现：利用条件类型
# ════════════════════════════════════════════════════════

# 标准库定义：Assert[C] 是一个类型
# - C 为 True 时，推导为 Void
# - C 为 False 时，推导为 compile_error("Assertion failed")
Assert: (C: Type) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed"),
}

# 使用方式1：在类型定义中作为约束
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    # 编译期检查：N 必须大于 0（Assert 在类型位置）
    length: Assert(N > 0),
}

# 使用方式2：在表达式中使用
IntArray: (N: Int) -> Type = StaticArray(Int, N)
# 验证：IntArray(10) 的大小等于 sizeof(Int) * 10
Assert(size_of(IntArray(10)) == sizeof(Int) * 10)
```

#### 4.4 编译期泛型特化

```yaoxiang
# 小数组优化：使用函数重载实现编译期泛型特化

# 通用实现
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    result = Zero::zero()
    for item in arr.data {
        result = result + item
    }
    return result
}

# N=1 特化
sum: (T: Type) -> ((arr: Array(T, 1)) -> T) = arr.data[0]

# N=2 特化
sum: (T: Type) -> ((arr: Array(T, 2)) -> T) = arr.data[0] + arr.data[1]

# 小数组循环展开（N <= 4）
sum: (T: Type, N: Int) -> ((arr: Array(T, N)) -> T) = {
    # 编译器优化：展开循环
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. 条件类型

> **Curry-Howard 同构**：条件类型从 Curry-Howard 视角看是逻辑中的 **case 分析**。`Bool` 类型对应一个有两个可能值的命题（True/False），`If` 根据该命题的真假选择不同的结果——这正是逻辑中的 case 析取。`match C { True => T, False => E }` 实际上在表达："已知命题 C 为 True 时结论是 T，C 为 False 时结论是 E"。

#### 5.1 If条件类型

```yaoxiang
# 类型级If
If: (C: Bool, T: Type, E: Type) -> Type = match C {
    True => T,
    False => E,
}

# 示例：编译期分支
NonEmpty: (T: Type) -> Type = If(T != Void, T, Never)

Optional: (T: Type) -> Type = If(T != Void, T, Void)

# 编译期验证
Assert: (C: Bool) -> Type = match C {
    True => Void,
    False => compile_error("Assertion failed"),
}

# 使用
# 类型计算：If(True, Int, String) => Int
# 类型计算：If(False, Int, String) => String
```

#### 5.2 类型族

> **Curry-Howard 同构**：类型族是"命题即类型"最直接的体现。`Add: (A: Type, B: Type) -> Type` 不是"在类型层面写了一个加法函数"，而是在 **构造一个关于自然数加法的命题**。`(Zero, B) => B` 是说"命题 Add(Zero, B) 等价于 B"，`(Succ(A'), B) => Succ(Add(A', B))` 是说"若 Add(A', B) 成立，则 Add(Succ(A'), B) 也成立"。这就是 Peano 公理中的加法定义本身。类型检查器验证这段 match 表达式通过，等价于验证了这个定义的逻辑一致性。

```yaoxiang
# 编译期类型转换
AsString: (T: Type) -> Type = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,  # 默认
}

# 类型级计算
Length: (T: Type) -> Type = match T.length {
    0 => Zero,
    1 => Succ(Zero),
    2 => Succ(Succ(Zero)),
    _ => TooLong,
}

# 类型级加法（Curry-Howard：这也是自然数加法的归纳定义）
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# 示例：编译期计算 2 + 3
Two: Type = Succ(Succ(Zero))
Three: Type = Succ(Succ(Succ(Zero)))
Five: Type = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. 函数重载特化

#### 6.1 基本特化

```yaoxiang
# 基本特化：使用函数重载（编译器自动选择）
sum: (arr: Array(Int)) -> Int = {
    # 编译为更高效的代码
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    # 使用SIMD指令
    return simd_sum_float(arr.data, arr.length)
}

# 通用实现
sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

#### 6.2 条件特化

```yaoxiang
# 完全符合RFC-010语法的特化方式：函数重载

# 具体类型特化
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# 泛型实现（编译器自动选择最优）
sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用时完全透明
int_arr = Array(Int)(1, 2, 3)
float_arr = Array(Float)(1.0, 2.0, 3.0)

# 编译器自动选择最优特化
sum(int_arr)     # 选择 sum: (Array(Int)) -> Int
sum(float_arr)    # 选择 sum: (Array(Float)) -> Float
```

#### 6.3 函数重载与内联的完美结合

**关键特性**：函数重载与内联优化天然结合，实现零成本抽象。

```yaoxiang
# ======== 源代码 ========
sum: (arr: Array(Int)) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array(Float)) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

sum: (T: Type) -> ((arr: Array(T)) -> T) = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用
int_arr = Array(Int)(1, 2, 3, 4, 5)
result = sum(int_arr)

# ======== 编译后（等价代码）=======
# 编译器自动选择最优特化，然后内联
result = native_sum_int(int_arr.data, int_arr.length)

# 完全等价于手写优化代码，无函数调用开销！
```

**核心优势**：

1. **编译器智能选择**
   ```yaoxiang
   sum(int_arr)      # 自动选择 sum: (Array(Int)) -> Int
   sum(float_arr)    # 自动选择 sum: (Array(Float)) -> Float
   sum(custom_arr)  # 自动选择 sum: (T: Type) -> ((arr: Array(T)) -> T)
   ```

2. **内联优化**
   - 小函数自动内联到调用点
   - 零函数调用开销
   - 完全等价于手写优化代码

3. **类型安全**
   - 编译期类型检查
   - 运行时零开销
   - 无需虚函数表

4. **完美契合RFC-010**
   ```yaoxiang
   # 完全使用统一语法
   name: type = value
   # 无需impl、where等新关键字
   ```

**实际应用示例**：

```yaoxiang
# 性能敏感的数值计算
fibonacci: (n: Int) -> Int = {
    if n <= 1 { return n }
    return fibonacci(n - 1) + fibonacci(n - 2)
}

fibonacci: (n: Float) -> Float = {
    # 使用Binet公式
    phi = (1.0 + 5.0.sqrt()) / 2.0
    return (phi.pow(n) - (-phi).pow(-n)) / 5.0.sqrt()
}

# 编译器自动选择并内联
fibonacci(10)      # 选择 Int 版本，完全内联
fibonacci(10.5)    # 选择 Float 版本，使用Binet公式
```

**这意味着什么？**

- ✅ **泛型特化** → 函数重载自然解决
- ✅ **性能优化** → 内联自动完成
- ✅ **代码复用** → 一个函数名，多种实现
- ✅ **零成本抽象** → 编译期多态，零运行时开销
- ✅ **无需新关键字** → 完美符合RFC-010统一语法
```

### 7. 死代码消除机制

#### 7.1 实例化图分析

```rust
// 编译器内部：构建泛型实例化依赖图
struct InstantiationGraph {
    // 节点：泛型实例化
    nodes: HashMap<InstanceKey, InstanceNode>,

    // 边：使用关系
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}

struct InstanceKey {
    generic: FunctionId,  // 泛型函数ID
    type_args: Vec<TypeId>,  // 类型参数
    const_args: Vec<ConstId>,  // Const参数
}

// 算法：可达性分析
fn eliminate_dead_instantiations(graph: &InstantiationGraph) {
    let mut reachable = HashSet::new();

    // 从入口点开始（main、导出函数等）
    let entry_points = find_entry_points();
    for entry in entry_points {
        dfs_visit(entry, &graph, &mut reachable);
    }

    // 未访问的实例化就是死代码
    for node in &graph.nodes {
        if !reachable.contains(node.key) {
            eliminate(node);
        }
    }
}
```

#### 7.2 使用点分析

```yaoxiang
# 源代码分析
map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 使用点1：实例化 map(Int, Int)
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # 需要 map[Int, Int]

# 使用点2：实例化 map(String, String)
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 需要 map[String, String]

# 未使用：map[Float, Float] 等
# 这些泛型实例不会被生成

# 编译后只包含被使用的实例
map_Int_Int: (list: List(Int), f: Fn(Int) -> Int) -> List(Int) = ...
map_String_String: (list: List(String), f: Fn(String) -> String) -> List(String) = ...
```

#### 7.3 编译期泛型DCE

```yaoxiang
# 编译期分析：编译期泛型使用情况
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
}

# 实际使用情况
arr_10_int = Array(Int, 10)(1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
arr_100_int = Array(Int, 100)(...)

# 编译后只生成被使用的Size
Array_Int_10: (Array(Int, 10)) = ...
Array_Int_100: (Array(Int, 100)) = ...

# 未使用的Size不会生成
# Array(Int, 50) 不会生成
```

#### 7.4 跨模块DCE

```yaoxiang
# 模块A
# A.yx
pub map: (T: Type, R: Type)(list: List(T), f: Fn(T) -> R) -> List(R) = ...

# 模块B
# B.yx
use A.{map}
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # 实例化 map(Int, Int)

# 模块C
# C.yx
use A.{map}
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 实例化 map(String, String)

# 编译分析：
# - 模块B使用 map[Int, Int]
# - 模块C使用 map[String, String]
# - 编译后二进制只包含这两个实例
```

#### 7.5 LLVM层面DCE

```rust
// 编译流水线
fn optimize_ir(ir: &mut IR) {
    // 1. 单态化（YaoXiang编译器）
    ir.monomorphize();

    // 2. 内联优化
    ir.inline_small_functions();

    // 3. 常量传播
    ir.constant_propagation();

    // 4. 生成LLVM IR
    let llvm_ir = ir.to_llvm();

    // 5. LLVM优化pass
    llvm_ir.add_pass(Passes::DEAD_CODE_ELIMINATION);
    llvm_ir.add_pass(Passes::INLINE_FUNCTION);
    llvm_ir.add_pass(Passes::GLOBAL_DCE);
    llvm_ir.add_pass(Passes::MERGE_FUNC);

    // 6. 运行优化
    llvm_ir.run_optimization_passes();
}
```

### 8. 宏替代策略

#### 8.1 代码生成替代

```yaoxiang
# ❌ 宏方案：代码生成
macro_rules! impl_debug {
    ($($t:ty),*) => {
        $(impl Debug for $t {
            fn fmt(&self, f: &mut Formatter) -> Result {
                write!(f, "{:?}", self)
            }
        })*
    };
}

# ✅ 泛型方案：自动派生
# 使用函数重载方式自动派生
debug_fmt: (T: fields...) -> ((self: Point(T)) -> String) = {
    return "Point { x: " + self.x.to_string() + ", y: " + self.y.to_string() + " }"
}

# 使用
p = Point { x: 1, y: 2 }
p.debug_fmt(&formatter)  # 自动生成调用
```

#### 8.2 DSL替代

```yaoxiang
# ❌ 宏方案：HTML DSL
html! {
    <div class="container">
        <h1> { title } </h1>
        <ul>
            { for item in items {
                <li> { item } </li>
            }}
        </ul>
    </div>
}

# ✅ 泛型方案：类型安全构建器
Element: Type = {
    tag: String,
    attrs: HashMap(String, String),
    children: List(Element),
    text: Option(String),
}

create_element: (tag: String) -> Element = {
    return Element(tag, HashMap::new(), List::new(), None)
}

with_class: [E: Element](elem: E, class: String) -> E = {
    elem.attrs.insert("class", class)
    return elem
}

with_text: [E: Element](elem: E, text: String) -> E = {
    return E { text: Some(text), ..elem }
}

# 构建DOM
container = create_element("div")
    |> with_class("container")
    |> with_children(List::new())

title_elem = create_element("h1") |> with_text(title)
items_li = items.map((item) =>
    create_element("li") |> with_text(item)
)
root = container |> with_children(List::new() + [title_elem, ul_elem])
```

#### 8.3 类型级编程替代

```yaoxiang
# ❌ 宏方案：类型级计算
macro_rules! add_types {
    ($a:ty, $b:ty) => {
        ($a, $b)
    };
}

# ✅ 泛型方案：条件类型
Add: (A: Type, B: Type) -> Type = match (A, B) {
    (Int, Int) => Int,
    (Float, Float) => Float,
    (Int, Float) => Float,
    (Float, Int) => Float,
    _ => TypeError,
}

# 编译期验证
AssertAddable: (A: Type, B: Type) -> Type = If(Add(A, B) != TypeError, (A, B), compile_error("Cannot add"))

# 使用
result_type = Add[Int, Float]  # 推导为 Float
```

### 9. 示例

#### 9.1 完整泛型容器示例

```yaoxiang
# ======== 1. 定义泛型容器 ========
# 使用 (T: Type) -> Type 语法
Result: (T: Type, E: Type) -> Type = {
    ok: (T) -> Self,
    err: (E) -> Self,
}

Option: (T: Type) -> Type = {
    some: (T) -> Self,
    none: () -> Self,
}

List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,

    # 泛型方法（T 由外层 List(T) 自动带入作用域）
    push: (self: List(T), item: T) -> Void,
    pop: (self: List(T)) -> Option(T),
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
    filter: (self: List(T), predicate: (T) -> Bool) -> List(T),
    fold: (U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U),
}

# ======== 2. 实现泛型方法 ========
# 使用 Type.method 语法糖：自动关联到 List 类型

List.push: (T: Type) -> ((self: List(T), item: T) -> Void) = {
    if self.length >= self.data.length {
        # 扩容
        new_data = Array(T)(self.data.length * 2)
        for i in 0..self.length {
            new_data[i] = self.data[i]
        }
        self.data = new_data
    }
    self.data[self.length] = item
    self.length = self.length + 1
}

List.pop: (T: Type) -> ((self: List(T)) -> Option(T)) = {
    if self.length > 0 {
        self.length = self.length - 1
        return Option.some(self.data[self.length])
    } else {
        return Option.none()
    }
}

List.map: (T: Type, R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)) = {
    result = List(R)()
    for i in 0..self.length {
        result.push(f(self.data[i]))
    }
    return result
}

List.filter: (T: Type) -> ((self: List(T), predicate: (T) -> Bool) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        if predicate(self.data[i]) {
            result.push(self.data[i])
        }
    }
    return result
}

List.fold: (T: Type, U: Type) -> ((self: List(T), initial: U, f: (U, T) -> U) -> U) = {
    result = initial
    for i in 0..self.length {
        result = f(result, self.data[i])
    }
    return result
}

# ======== 3. 类型约束使用 ========
# 实现 Clone for List
List.clone: (T: Clone) -> ((self: List(T)) -> List(T)) = {
    result = List(T)()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. 使用示例 ========
# 创建泛型List
numbers = List(Int)()
numbers.push(1)
numbers.push(2)
numbers.push(3)

# 使用泛型方法
doubled = numbers.map((x) => x * 2)
evens = numbers.filter((x) => x % 2 == 0)

# 使用fold计算
sum = numbers.fold(0, (acc, x) => acc + x)  # sum = 6

# 泛型组合
sum_of_evens = numbers
    .filter((x) => x % 2 == 0)
    .map((x) => x * 2)
    .fold(0, (acc, x) => acc + x)  # sum_of_evens = 8
```

#### 9.2 泛型算法示例

```yaoxiang
# ======== 1. 泛型排序算法 ========
Comparator: (T: Type) -> Type = {
    compare: (T, T) -> Int,  # -1 if a < b, 0 if a == b, 1 if a > b
}

# 泛型quicksort
quicksort: (T: Clone) -> ((array: Array(T), cmp: Comparator(T)) -> Array(T)) = {
    if array.length <= 1 {
        return array.clone()
    }

    pivot = array[array.length / 2]
    left = Array(T)()
    right = Array(T)()

    for i in 0..array.length {
        if i == array.length / 2 {
            continue
        }
        item = array[i]
        comparison = cmp.compare(item, pivot)
        if comparison < 0 {
            left.push(item)
        } else {
            right.push(item)
        }
    }

    sorted_left = quicksort(left, cmp)
    sorted_right = quicksort(right, cmp)

    result = sorted_left.clone()
    result.push(pivot)
    result.extend(sorted_right)
    return result
}

# ======== 2. IntComparator实现 ========
# 使用函数重载实现
compare: (a: Int, b: Int) -> Int = {
    if a < b {
        return -1
    } else if a > b {
        return 1
    } else {
        return 0
    }
}

# ======== 3. 使用示例 ========
# 排序Int数组
numbers = Array(Int)(3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator(Int)())

# 排序String数组（需要StringComparator）
strings = Array(String)("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator(String)())
```

#### 9.3 编译期泛型示例

```yaoxiang
# ======== 1. 编译期矩阵类型 ========
Matrix: (T: Type, Rows: Int, Cols: Int) -> Type = {
    data: Array(Array(T, Cols), Rows),

    # 编译期维度验证：利用 Assert 标准库类型
    _assert: Assert[Rows > 0],  # Rows > 0，否则编译错误
    _assert: Assert[Cols > 0],  # Cols > 0，否则编译错误

    # 矩阵运算
    multiply: (M: Int) -> ((self: Matrix(T, Rows, Cols), other: Matrix(T, Cols, M)) -> Matrix(T, Rows, M)) = {
        result = Matrix(T, Rows, M)()
        for i in 0..Rows {
            for j in 0..M {
                sum = Zero::zero()
                for k in 0..Cols {
                    sum = sum + self.data[i][k] * other.data[k][j]
                }
                result.data[i][j] = sum
            }
        }
        return result
    }
}

# ======== 2. 编译期矩阵创建 ========
identity: (T: Add + Multiply + One, N: Int) -> ((size: N) -> Matrix(T, N, N)) = {
    matrix = Matrix(T, N, N)()
    for i in 0..N {
        for j in 0..N {
            if i == j {
                matrix.data[i][j] = One::one()
            } else {
                matrix.data[i][j] = Zero::zero()
            }
        }
    }
    return matrix
}

# ======== 3. 使用示例 ========
# 创建编译期已知大小的矩阵
# 2x3 矩阵
matrix_2x3 = Matrix(Float, 2, 3)()
matrix_2x3.data[0][0] = 1.0
matrix_2x3.data[0][1] = 2.0
matrix_2x3.data[0][2] = 3.0
matrix_2x3.data[1][0] = 4.0
matrix_2x3.data[1][1] = 5.0
matrix_2x3.data[1][2] = 6.0

# 3x2 矩阵
matrix_3x2 = Matrix(Float, 3, 2)()
matrix_3x2.data[0][0] = 7.0
matrix_3x2.data[0][1] = 8.0
matrix_3x2.data[1][0] = 9.0
matrix_3x2.data[1][1] = 10.0
matrix_3x2.data[2][0] = 11.0
matrix_3x2.data[2][1] = 12.0

# 矩阵乘法：2x3 * 3x2 = 2x2
result = matrix_2x3.multiply(matrix_3x2)

# 编译期验证：result类型为 Matrix(Float, 2, 2)
# 2x2 单位矩阵
identity_3x3 = identity(Float, 3)()

# 维度不匹配：编译错误
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # 编译错误：3x3 != 2x3
```

## 权衡

### 优点

1. **零成本抽象**
   - 编译期单态化，无运行时开销
   - 无需虚函数，无RTTI

2. **死代码消除**
   - 编译期分析，只实例化被使用的泛型
   - 代码膨胀可控

3. **宏替代**
   - 类型安全的代码生成
   - IDE友好，错误信息清晰

4. **编译期计算**
   - 编译期泛型支持编译期计算
   - 维度验证等特性
   - 无需 `const` 关键字，纯类型约束

### 缺点

1. **编译时间**
   - 泛型实例化增加编译时间
   - 约束求解可能较慢

2. **内存占用**
   - 编译器内存占用增加
   - 缓存机制需要内存

3. **实现复杂度**
   - 约束求解器复杂
   - 类型级计算引擎复杂

4. **错误诊断**
   - 泛型错误可能复杂
   - 需要清晰的错误提示

### 缓解措施

1. **缓存策略**
   - 实例化结果缓存
   - LRU缓存限制内存

2. **增量编译**
   - 缓存编译结果
   - 增量实例化

3. **错误提示**
   - 清晰的错误信息
   - 泛型参数推导提示

4. **并行编译**
   - 并行实例化泛型
   - 多线程约束求解

## 替代方案

| 方案 | 为什么不选择 |
|------|--------------|
| 仅基础泛型 | 无法替代复杂宏 |
| 纯宏系统 | 无类型安全，错误信息差 |
| 仅依赖约束 | 灵活性不足 |
| 运行时泛型 | 有性能开销 |

### 风险

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 约束求解复杂度 | 编译时间过长 | 增量求解 + 缓存 |
| 代码膨胀 | 二进制文件过大 | DCE + 阈值控制 |
| 实现复杂度 | 开发周期延长 | 分阶段实现 |
| 错误诊断 | 用户体验差 | 详细错误信息 |

## 开放问题

### 待决议问题

| 议题 | 说明 | 状态 |
|------|------|------|
| 实例化策略 | Eager vs Lazy vs Threshold | 待讨论 |
| 缓存大小 | LRU缓存容量设置 | 待讨论 |
| 错误诊断 | 泛型错误信息详细程度 | 待讨论 |

### 后续优化

| 优化项 | 价值 | 实现难度 |
|--------|------|----------|
| 实例化图分析 | 高 | 中 |
| 类型级编程DSL | 中 | 高 |
| 泛型性能基准 | 中 | 低 |

## 附录

### 语法BNF

```bnf
# 泛型参数使用统一 () 语法，作为函数类型的一部分
# 如 map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R))

# 类型约束（在泛型参数中）
type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*

# 参数声明（类型 + 名字）
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# 函数声明：name: type = expression
# 泛型参数是函数类型中的第一个参数组：(T: Type) -> ((params) -> return)
function ::= identifier ':' type '=' (expression | block)

# 方法声明：Type.method: type = expression
method ::= identifier '.' identifier ':' type '=' (expression | block)

# 类型定义（统一 Binding 语法）
# 泛型类型如 List: (T: Type) -> Type = { ... }
generic_type ::= identifier ':' type '=' type_expression

# 泛型参数中的 Type 由编译器自动从实参类型填充
# 如 map(numbers, f)，T 从 numbers: List(Int) 提取，R 从 f: (Int) -> String 提取
```

## 生命周期与归宿

```
┌─────────────┐
│   草案      │  ← 当前状态
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  审核中     │  ← 开放社区讨论和反馈
└──────┬──────┘
       │
       ├──────────────────┐
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│  已接受     │    │  已拒绝     │
└──────┬──────┘    └──────┬──────┘
       │                  │
       ▼                  ▼
┌─────────────┐    ┌─────────────┐
│   accepted/ │    │    rfc/     │
│ (正式设计)  │    │ (保留原位)  │
└─────────────┘    └─────────────┘
```

---

## 参考文献

### YaoXiang官方文档

- [RFC-010: 统一类型语法](./010-unified-type-syntax.md)
- [RFC-009: 所有权模型](./accepted/009-ownership-model.md)
- [RFC-001: 并作模型](./accepted/001-concurrent-model-error-handling.md)
- [RFC-008: 运行时模型](./accepted/008-runtime-concurrency-model.md)
- [语言规范](../language-spec.md)
- [YaoXiang指南](../guides/YaoXiang-book.md)

### 外部参考

- [Rust泛型系统](https://doc.rust-lang.org/book/ch10-01-syntax.html)
- [C++模板特化](https://en.cppreference.com/w/cpp/language/template_specialization)
- [Haskell类型类](https://www.haskell.org/tutorial/classes.html)
- [Swift泛型](https://docs.swift.org/swift-book/LanguageGuide/Generics.html)
- [单态化优化](https://llvm.org/docs/Monomorphization.html)
- [死代码消除](https://en.wikipedia.org/wiki/Dead_code_elimination)
