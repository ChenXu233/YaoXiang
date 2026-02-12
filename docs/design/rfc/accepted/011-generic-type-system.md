---
title: RFC-011：泛型系统设计
---

# RFC-011: 泛型系统设计 - 零成本抽象与宏替代

> **状态**: 已接受
> **作者**: 晨煦
> **创建日期**: 2025-01-25
> **最后更新**: 2025-02-04（移除 const 关键字，改用字面量类型约束）

## 摘要

本文档定义YaoXiang语言的**泛型系统设计**，通过强大的泛型能力实现零成本抽象，利用编译期优化减少对宏的依赖，并提供死代码消除机制。

**核心设计**：
- **基础泛型**：`[T]` 类型参数，支持泛型函数和泛型类型
- **类型约束**：`[T: Clone]` 多重约束，函数类型约束
- **关联类型**：`type Iterator[T] = { Item: T, next: () -> Option[T] }`
- **编译期泛型**：`[T, N: Int]` 编译期常量参数，字面量类型约束区分编译期与运行时
- **条件类型**：`type If[C: Bool, T, E]` 类型级计算，类型族
- **平台特化**：`[P: X86_64]` 预定义泛型参数 P，平台即类型

**价值**：
- 零成本抽象：编译期单态化，无运行时开销
- 死代码消除：实例化图分析 + LLVM优化
- 宏替代：泛型替代90%的宏使用场景
- 类型安全：编译期检查，IDE友好

## 参考文档

本文档的设计基于以下文档：

| 文档 | 关系 | 说明 |
|------|------|------|
| [RFC-010: 统一类型语法](./010-unified-type-syntax.md) | **语法基础** | 泛型语法与统一 `name: type = value` 模型集成 |
| [RFC-009: 所有权模型](./accepted/009-ownership-model.md) | **类型系统** | Move语义与泛型的自然结合 |
| [RFC-001: 并作模型](./accepted/001-concurrent-model-error-handling.md) | **执行模型** | DAG分析与泛型类型检查 |
| [RFC-008: 运行时模型](./accepted/008-runtime-concurrency-model.md) | **编译器架构** | 泛型单态化与编译期优化策略 |

## 动机

### 为什么需要强泛型系统？

当前主流语言的泛型存在局限：

| 语言 | 泛型能力 | 问题 |
|------|----------|------|
| Java | 边界类型 | 编译期单态化，无泛型特化 |
| C# | 泛型约束 | 运行时类型检查，有性能开销 |
| Rust | 泛型 + Trait | Trait系统复杂，学习曲线陡峭 |
| C++ | 模板 | 模板特化复杂，编译错误信息差 |

### 核心矛盾

1. **性能 vs 灵活性**：运行时灵活性 vs 编译期优化
2. **复杂 vs 简洁**：强大的类型系统 vs 易用性
3. **宏 vs 泛型**：宏代码生成 vs 泛型类型安全

### 泛型系统的价值

```yaoxiang
# 示例：统一API设计
# 不同容器类型的map操作

# 传统方案：每个类型单独实现
map_int_array: (array: Array[Int], f: Fn(Int) -> Int) -> Array[Int] = ...
map_string_array: (array: Array[String], f: Fn(String) -> String) -> Array[String] = ...
map_int_list: (list: List[Int], f: Fn(Int) -> Int) -> List[Int] = ...
map_string_list: (list: List[String], f: Fn(String) -> String) -> List[String] = ...

# 泛型方案：一个泛型函数覆盖所有类型
map: [T, R](container: Container[T], f: Fn(T) -> R) -> Container[R] = {
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

### 设计原则

- **编译期确定**：泛型参数在编译期确定
- **单态化优先**：生成具体代码，避免虚函数调用
- **约束驱动**：类型约束指导实例化
- **平台优化**：特化支持平台特定优化

## 提案

### 1. 基础泛型

#### 1.1 泛型类型参数

```yaoxiang
# 泛型类型定义（统一使用记录类型语法）
type Option[T] = {
    some: (T) -> Self,
    none: () -> Self
}

type Result[T, E] = {
    ok: (T) -> Self,
    err: (E) -> Self
}

type List[T] = {
    data: Array[T],
    length: Int,
    push: [T](self: List[T], item: T) -> Void,
    get: [T](self: List[T], index: Int) -> Option[T],
}

# 泛型函数（参数名在签名中声明）
map: [T, R](opt: Option[T], f: Fn(T) -> R) -> Option[R] = {
    return match opt {
        some => Option.some(f(some)),
        none => Option.none(),
    }
}

# 泛型约束（直接表达式，单行可省略 return）
clone: [T: Clone](value: T) -> T = value.clone()

# 多类型参数
combine: [T, U](a: T, b: U) -> (T, U) = (a, b)
```

#### 1.2 类型推导

```yaoxiang
# 编译器自动推导泛型参数
numbers: List[Int] = List[Int](1, 2, 3)
# 可推导为：
numbers = List(1, 2, 3)  # 编译器推导 List[Int]

# 函数调用推导
result = map(Some(42), (x) => x + 1)
# 推导为：
result = map[Int, Int](Some(42), (x) => x + 1)
```

#### 1.3 单态化

```yaoxiang
# 源代码
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = {
    result = List[R]()
    for x in list {
        result.push(f(x))
    }
    return result
}

# 使用点
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # 实例化 map[Int, Int]

string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 实例化 map[String, String]

# 编译后（等价代码）
map_Int_Int: (list: List[Int], f: Fn(Int) -> Int) -> List[Int] = {
    result = List[Int]()
    for x in list {
        result.push(f(x))
    }
    return result
}

map_String_String: (list: List[String], f: Fn(String) -> String) -> List[String] = {
    result = List[String]()
    for s in list {
        result.push(f(s))
    }
    return result
}
```

### 2. 类型约束系统

#### 2.1 单一约束

```yaoxiang
# 基本trait定义（接口类型）
type Clone = {
    clone: (Self) -> Self,
}

type Display = {
    fmt: (Self, Formatter) -> Result,
}

type Debug = {
    fmt: (Self, Formatter) -> Result,
}

# 使用约束
clone: [T: Clone](value: T) -> T = value.clone()

debug_print: [T: Debug](value: T) -> Void = {
    formatter = Formatter.new()
    value.fmt(formatter)
    print(formatter.to_string())
}
```

#### 2.2 多重约束

```yaoxiang
# 多重约束语法
combine: [T: Clone + Add](a: T, b: T) -> T = {
    a.clone() + b
}

# 泛型容器的排序
sort: [T: Clone + PartialOrd](list: List[T]) -> List[T] = {
    # 实现排序算法
    result = list.clone()
    quicksort(&mut result)
    return result
}

# 函数类型约束
map: [T, R: FnMut(T)](array: Array[T], f: R) -> Array[R] = {
    result = Array[R]()
    for item in array {
        result.push(f(item))
    }
    return result
}

# 使用
doubled = map(Array(1, 2, 3), (x) => x * 2)  # R 推导为 FnMut(Int) -> Int
```

#### 2.3 函数类型约束

```yaoxiang
# 高阶函数约束
call_twice: [T, F: Fn() -> T](f: F) -> (T, T) = (f(), f())

call_with_arg: [T, U, F: Fn(T) -> U](arg: T, f: F) -> U = f(arg)

compose: [A, B, C, F: Fn(A) -> B, G: Fn(B) -> C](a: A, f: F, g: G) -> C = g(f(a))

# 使用示例
result = call_with_arg(42, (x) => x * 2)  # result = 84

composed = compose(
    "hello",
    (s) => s.to_uppercase(),
    (s) => s + " WORLD"
)  # composed = "HELLO WORLD"
```

### 3. 关联类型

#### 3.1 关联类型定义

```yaoxiang
# Iterator trait（使用记录类型语法）
type Iterator[T] = {
    Item: T,  # 关联类型
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool,
    collect: [T](Self) -> List[T],
}

# 使用 [T, I: Iterator[T]](iter: I) -> List[T] = {
    result = List[T]()
    while iter.has_next() {
        if let Some(item) = iter.next() {
            result.push(item)
        }
    }
    return result
}

# Array的Iterator实现
# 使用方法语法糖：Array.Item, Array.next, Array.has_next
Array.has_next: [T](self: Array[T]) -> Bool = {
    return self.index < self.length
}

Array.next: [T](self: Array[T]) -> Option[T] = {
    if has_next(self) {
        item = self.data[self.index]
        self.index = self.index + 1
        return Option.some(item)
    } else {
        return Option.none()
    }
}

Array.Item: [T](arr: Array[T]) -> T = {
    return arr.data[0]
}
```

#### 3.2 泛型关联类型（GAT）

```yaoxiang
# 更复杂的关联类型
type Producer[T] = {
    Item: T,
    produce: (Self) -> Option[T],
}

# 关联类型可以是泛型的
type Container[T] = {
    Item: T,
    IteratorType: Iterator[T],  # 关联类型也是泛型的
    iter: (Self) -> IteratorType,
}

# 使用
process_container: [T, C: Container[T]](container: C) -> List[T] = {
    container.iter().collect()
}
```

### 4. 编译期泛型

#### 4.1 编译期常量参数

**核心设计**：用 `[n: Int]` 泛型参数 + `(n: n)` 值参数，区分编译期常量与运行时值。无需 `const` 关键字。

```yaoxiang
# ════════════════════════════════════════════════════════
# 字面量类型约束：编译期 vs 运行时
# ════════════════════════════════════════════════════════

# [n: Int] 声明泛型参数，类型为 Int
# (n: n)   声明值参数，类型为字面量类型 "n"
#           n 既是类型也是值，编译器在调用时必须已知 n 的值

# 编译期阶乘：参数必须是编译期已知的字面量
factorial: [n: Int](n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# 编译期加法
add: [a: Int, b: Int](a: a, b: b) -> Int = a + b

# ════════════════════════════════════════════════════════
# 编译期常量数组
# ════════════════════════════════════════════════════════
type StaticArray[T, N: Int] = {
    data: T[N],  # 编译期已知大小的数组
    length: N,
}

# 使用方式
arr: StaticArray[Int, factorial(5)]  # StaticArray[Int, 120]，编译器在编译期计算
```

#### 4.2 编译期计算

```yaoxiang
# ════════════════════════════════════════════════════════
# 编译期计算示例
# ════════════════════════════════════════════════════════

# 编译器在编译期计算字面量类型的函数调用
SIZE: Int = factorial(5)  # 编译期为 120

# 矩阵类型使用
type Matrix[T, Rows: Int, Cols: Int] = {
    data: Array[Array[T, Cols], Rows],
}

# 编译期维度验证
identity_matrix: [T: Add + Zero + One, N: Int](size: N) -> Matrix[T, N, N] = {
    matrix = Matrix[T, N, N]()
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

# 使用：编译期计算，生成 Matrix[Float, 3, 3]
identity_3x3 = identity_matrix[Float, 3](3)
```

#### 4.3 编译期验证（标准库实现）

```yaoxiang
# ════════════════════════════════════════════════════════
# 标准库实现：利用条件类型
# ════════════════════════════════════════════════════════

# 标准库定义：Assert[C] 是一个类型
# - C 为 True 时，推导为 Void
# - C 为 False 时，推导为 compile_error("Assertion failed")
type Assert[C: Bool] = match C {
    True => Void,
    False => compile_error("Assertion failed"),
}

# 使用方式1：在类型定义中作为约束
type Array[T, N: Int] = {
    data: T[N],
    # 编译期检查：N 必须大于 0（Assert 在类型位置）
    length: Assert[N > 0],
}

# 使用方式2：在表达式中使用
type IntArray[N: Int] = StaticArray[Int, N]
# 验证：IntArray[10] 的大小等于 sizeof(Int) * 10
Assert[size_of(IntArray[10]) == sizeof(Int) * 10]
```

#### 4.4 编译期泛型特化

```yaoxiang
# 小数组优化：使用函数重载实现编译期泛型特化

# 通用实现
sum: [T, N: Int](arr: Array[T, N]) -> T = {
    result = Zero::zero()
    for item in arr.data {
        result = result + item
    }
    return result
}

# N=1 特化
sum: [T](arr: Array[T, 1]) -> T = arr.data[0]

# N=2 特化
sum: [T](arr: Array[T, 2]) -> T = arr.data[0] + arr.data[1]

# 小数组循环展开（N <= 4）
sum: [T, N: Int](arr: Array[T, N]) -> T = {
    # 编译器优化：展开循环
    return arr.data[0] + arr.data[1] + arr.data[2] + arr.data[3]
}
```

### 5. 条件类型

#### 5.1 If条件类型

```yaoxiang
# 类型级If
type If[C: Bool, T, E] = match C {
    True => T,
    False => E,
}

# 示例：编译期分支
type NonEmpty[T] = If[T != Void, T, Never]

type Optional[T] = If[T != Void, T, Void]

# 编译期验证
type Assert[C: Bool] = match C {
    True => Void,
    False => compile_error("Assertion failed"),
}

# 使用
# 类型计算：If[True, Int, String] => Int
# 类型计算：If[False, Int, String] => String
```

#### 5.2 类型族

```yaoxiang
# 编译期类型转换
type AsString[T] = match T {
    Int => String,
    Float => String,
    Bool => String,
    _ => String,  # 默认
}

# 类型级计算
type Length[T: TupleType] = match T.length {
    0 => Zero,
    1 => Succ(Zero),
    2 => Succ(Succ(Zero)),
    _ => TooLong,
}

# 类型级加法
type Add[A: Nat, B: Nat] = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}

# 示例：编译期计算 2 + 3
type Two = Succ(Succ(Zero))
type Three = Succ(Succ(Succ(Zero)))
type Five = Add[Two, Three]  # Succ(Succ(Succ(Succ(Succ(Zero)))))
```

### 6. 函数重载特化

#### 6.1 基本特化

```yaoxiang
# 基本特化：使用函数重载（编译器自动选择）
sum: (arr: Array[Int]) -> Int = {
    # 编译为更高效的代码
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array[Float]) -> Float = {
    # 使用SIMD指令
    return simd_sum_float(arr.data, arr.length)
}

# 通用实现
sum: [T](arr: Array[T]) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}
```

#### 6.2 平台特化

**核心设计**：
- 平台类型由标准库定义
- `P` 是预定义的泛型参数名，被解析器占用，代表当前平台
- 使用纯类型约束语法，无 `#[cfg]` 等宏

```yaoxiang
# ======== 标准库定义（std） ========
# 平台类型枚举
type Platform = X86_64 | AArch64 | RISC_V | ARM | X86 | ...

# 预定义泛型参数 P：解析器自动识别，代表当前编译平台

# ======== 用户代码 ========
# 通用实现（所有平台可用）
sum: [T: Add](arr: Array[T]) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 平台特化：P 是预定义泛型参数，代表当前平台
# 编译器根据当前平台自动选择匹配的特化
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}

sum: [P: RISC_V](arr: Array[Float]) -> Float = {
    return riscv_vec_sum(arr.data, arr.length)
}

# 匹配语法（更灵活的方式）
sum: [P](arr: Array[Float]) -> Float = match P {
    X86_64 => avx2_sum(arr.data, arr.length),
    AArch64 => neon_sum(arr.data, arr.length),
    RISC_V => riscv_vec_sum(arr.data, arr.length),
    _ => basic_sum_iter(arr),
}
```

**设计原则**：
- **平台即类型**：一切皆是类型，平台不应该是预编译条件
- **标准库定义**：平台类型可扩展，用户可定义自己的平台类型
- **解析器占用 P**：语法简洁，无需导入，`P` 自动绑定到当前平台
- **100% 类型安全**：所有平台代码都参与类型检查，拼写错误在任何平台都能发现

**平台类型能力**：

```yaoxiang
# 平台类型可用于类型级计算
type PlatformSupportsSIMD[P] = match P {
    X86_64 => True,
    AArch64 => True,
    RISC_V => True,
    _ => False,
}

# 验证当前平台是否支持 SIMD（使用 Assert 标准库类型）
Assert[PlatformSupportsSIMD[P]]
```

#### 6.3 条件特化

```yaoxiang
# 完全符合RFC-010语法的特化方式：函数重载

# 具体类型特化
sum: (arr: Array[Int]) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array[Float]) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

# 泛型实现（编译器自动选择最优）
sum: [T](arr: Array[T]) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用时完全透明
int_arr = Array[Int](1, 2, 3)
float_arr = Array[Float](1.0, 2.0, 3.0)

# 编译器自动选择最优特化
sum(int_arr)     # 选择 sum: (Array[Int]) -> Int
sum(float_arr)    # 选择 sum: (Array[Float]) -> Float
```

#### 6.4 函数重载与内联的完美结合

**关键特性**：函数重载与内联优化天然结合，实现零成本抽象。

```yaoxiang
# ======== 源代码 ========
sum: (arr: Array[Int]) -> Int = {
    return native_sum_int(arr.data, arr.length)
}

sum: (arr: Array[Float]) -> Float = {
    return simd_sum_float(arr.data, arr.length)
}

sum: [T](arr: Array[T]) -> T = {
    result = Zero::zero()
    for item in arr {
        result = result + item
    }
    return result
}

# 使用
int_arr = Array[Int](1, 2, 3, 4, 5)
result = sum(int_arr)

# ======== 编译后（等价代码）=======
# 编译器自动选择最优特化，然后内联
result = native_sum_int(int_arr.data, int_arr.length)

# 完全等价于手写优化代码，无函数调用开销！
```

**核心优势**：

1. **编译器智能选择**
   ```yaoxiang
   sum(int_arr)      # 自动选择 sum: (Array[Int]) -> Int
   sum(float_arr)    # 自动选择 sum: (Array[Float]) -> Float
   sum(custom_arr)  # 自动选择 sum: [T](Array[T]) -> T
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
map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = ...

# 使用点1：实例化 map[Int, Int]
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # 需要 map[Int, Int]

# 使用点2：实例化 map[String, String]
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 需要 map[String, String]

# 未使用：map[Float, Float] 等
# 这些泛型实例不会被生成

# 编译后只包含被使用的实例
map_Int_Int: (list: List[Int], f: Fn(Int) -> Int) -> List[Int] = ...
map_String_String: (list: List[String], f: Fn(String) -> String) -> List[String] = ...
```

#### 7.3 编译期泛型DCE

```yaoxiang
# 编译期分析：编译期泛型使用情况
type Array[T, N: Int] = {
    data: T[N],
}

# 实际使用情况
arr_10_int = Array[Int, 10](1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
arr_100_int = Array[Int, 100](...)

# 编译后只生成被使用的Size
Array_Int_10: (Array[Int, 10]) = ...
Array_Int_100: (Array[Int, 100]) = ...

# 未使用的Size不会生成
# Array[Int, 50] 不会生成
```

#### 7.4 跨模块DCE

```yaoxiang
# 模块A
# A.yx
pub map: [T, R](list: List[T], f: Fn(T) -> R) -> List[R] = ...

# 模块B
# B.yx
use A.{map}
int_list = List(1, 2, 3)
doubled = map(int_list, (x) => x * 2)  # 实例化 map[Int, Int]

# 模块C
# C.yx
use A.{map}
string_list = List("a", "b", "c")
uppercased = map(string_list, (s) => s.to_uppercase())  # 实例化 map[String, String]

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
debug_fmt: [T: fields...](self: Point[T]) -> String = {
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
type Element = {
    tag: String,
    attrs: HashMap[String, String],
    children: List[Element],
    text: Option[String],
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
type Add[A, B] = match (A, B) {
    (Int, Int) => Int,
    (Float, Float) => Float,
    (Int, Float) => Float,
    (Float, Int) => Float,
    _ => TypeError,
}

# 编译期验证
type AssertAddable[A, B] = If[Add[A, B] != TypeError, (A, B), compile_error("Cannot add")]

# 使用
result_type = Add[Int, Float]  # 推导为 Float
```

### 9. 示例

#### 9.1 完整泛型容器示例

```yaoxiang
# ======== 1. 定义泛型容器 ========
# 使用记录类型语法
type Result[T, E] = {
    ok: (T) -> Self,
    err: (E) -> Self,
}

type Option[T] = {
    some: (T) -> Self,
    none: () -> Self,
}

type List[T] = {
    data: Array[T],
    length: Int,

    # 泛型方法（使用 Type.method 语法）
    push: [T](self: List[T], item: T) -> Void,
    pop: [T](self: List[T]) -> Option[T],
    map: [T, R](self: List[T], f: Fn(T) -> R) -> List[R],
    filter: [T](self: List[T], predicate: Fn(T) -> Bool) -> List[T],
    fold: [T, U](self: List[T], initial: U, f: Fn(U, T) -> U) -> U,
}

# ======== 2. 实现泛型方法 ========
# 使用 Type.method 语法糖：自动关联到 List 类型

List.push: [T](self: List[T], item: T) -> Void = {
    if self.length >= self.data.length {
        # 扩容
        new_data = Array[T](self.data.length * 2)
        for i in 0..self.length {
            new_data[i] = self.data[i]
        }
        self.data = new_data
    }
    self.data[self.length] = item
    self.length = self.length + 1
}

List.pop: [T](self: List[T]) -> Option[T] = {
    if self.length > 0 {
        self.length = self.length - 1
        return Option.some(self.data[self.length])
    } else {
        return Option.none()
    }
}

List.map: [T, R](self: List[T], f: Fn(T) -> R) -> List[R] = {
    result = List[R]()
    for i in 0..self.length {
        result.push(f(self.data[i]))
    }
    return result
}

List.filter: [T](self: List[T], predicate: Fn(T) -> Bool) -> List[T] = {
    result = List[T]()
    for i in 0..self.length {
        if predicate(self.data[i]) {
            result.push(self.data[i])
        }
    }
    return result
}

List.fold: [T, U](self: List[T], initial: U, f: Fn(U, T) -> U) -> U = {
    result = initial
    for i in 0..self.length {
        result = f(result, self.data[i])
    }
    return result
}

# ======== 3. 类型约束使用 ========
# 实现 Clone for List
List.clone: [T: Clone](self: List[T]) -> List[T] = {
    result = List[T]()
    for i in 0..self.length {
        result.push(self.data[i].clone())
    }
    return result
}

# ======== 4. 使用示例 ========
# 创建泛型List
numbers = List[Int]()
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
type Comparator[T] = {
    compare: (T, T) -> Int,  # -1 if a < b, 0 if a == b, 1 if a > b
}

# 泛型quicksort
quicksort: [T: Clone](array: Array[T], cmp: Comparator[T]) -> Array[T] = {
    if array.length <= 1 {
        return array.clone()
    }

    pivot = array[array.length / 2]
    left = Array[T]()
    right = Array[T]()

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
numbers = Array[Int](3, 1, 4, 1, 5, 9, 2, 6)
sorted = quicksort(numbers, Comparator[Int]())

# 排序String数组（需要StringComparator）
strings = Array[String]("hello", "world", "foo", "bar")
sorted_strings = quicksort(strings, Comparator[String]())
```

#### 9.3 编译期泛型示例

```yaoxiang
# ======== 1. 编译期矩阵类型 ========
type Matrix[T, Rows: Int, Cols: Int] = {
    data: Array[Array[T, Cols], Rows],

    # 编译期维度验证：利用 Assert 标准库类型
    _assert: Assert[Rows > 0],  # Rows > 0，否则编译错误
    _assert: Assert[Cols > 0],  # Cols > 0，否则编译错误

    # 矩阵运算
    multiply: [T: Add + Multiply + Zero, M: Int](self: Matrix[T, Rows, Cols], other: Matrix[T, Cols, M]) -> Matrix[T, Rows, M] = {
        result = Matrix[T, Rows, M]()
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
identity: [T: Add + Multiply + One, N: Int](size: N) -> Matrix[T, N, N] = {
    matrix = Matrix[T, N, N]()
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
matrix_2x3 = Matrix[Float, 2, 3]()
matrix_2x3.data[0][0] = 1.0
matrix_2x3.data[0][1] = 2.0
matrix_2x3.data[0][2] = 3.0
matrix_2x3.data[1][0] = 4.0
matrix_2x3.data[1][1] = 5.0
matrix_2x3.data[1][2] = 6.0

# 3x2 矩阵
matrix_3x2 = Matrix[Float, 3, 2]()
matrix_3x2.data[0][0] = 7.0
matrix_3x2.data[0][1] = 8.0
matrix_3x2.data[1][0] = 9.0
matrix_3x2.data[1][1] = 10.0
matrix_3x2.data[2][0] = 11.0
matrix_3x2.data[2][1] = 12.0

# 矩阵乘法：2x3 * 3x2 = 2x2
result = matrix_2x3.multiply(matrix_3x2)

# 编译期验证：result类型为 Matrix[Float, 2, 2]
# 2x2 单位矩阵
identity_3x3 = identity[Float, 3]()

# 维度不匹配：编译错误
# bad_multiply = matrix_2x3.multiply(identity_3x3)  # 编译错误：3x3 != 2x3
```

## 详细设计

### 编译器架构

#### 1. 类型系统核心

```rust
struct TypeSystem {
    // 类型环境
    types: HashMap<String, Type>,

    // 约束求解器
    solver: ConstraintSolver,

    // 归一化器
    normalizer: TypeNormalizer,
}

struct ConstraintSolver {
    // 约束集合
    constraints: Vec<Constraint>,

    // 求解方法
    fn solve(&mut self) -> Result<(), TypeError> {
        // 实现约束求解算法
    }
}

struct TypeNormalizer {
    // 类型归一化
    fn normalize(&self, ty: &Type) -> Type {
        // 实现类型归一化
    }
}
```

#### 2. 泛型实例化器

```rust
struct Instantiator {
    // 实例化缓存
    cache: HashMap<InstanceKey, InstanceId>,

    // 单态化器
    monomorphizer: Monomorphizer,

    // 特化器
    specializer: Specializer,
}

struct InstanceKey {
    generic: FunctionId,
    type_args: Vec<TypeId>,
    const_args: Vec<ConstId>,
}

impl Instantiator {
    fn instantiate(
        &mut self,
        generic: &Function,
        type_args: &[Type],
        const_args: &[Const],
    ) -> Result<FunctionId, InstantiationError> {
        let key = InstanceKey::new(generic.id, type_args, const_args);

        if let Some(&cached_id) = self.cache.get(&key) {
            return Ok(cached_id);
        }

        let instantiated = self.monomorphize(generic, type_args, const_args)?;
        let instance_id = self.register_instance(instantiated);
        self.cache.insert(key, instance_id);

        Ok(instance_id)
    }
}
```

#### 3. 编译期求值器

```rust
struct CompileTimeEvaluator {
    // 常量折叠
    folder: ConstantFolder,

    // 类型级计算器
    type_computer: TypeLevelComputer,

    // 死代码消除器
    dce: DeadCodeEliminator,
}

struct ConstantFolder {
    // 常量表达式折叠
    fn fold(&self, expr: &Expr) -> Result<Const, ConstEvalError> {
        // 实现常量折叠算法
    }
}

struct TypeLevelComputer {
    // 类型级计算
    fn compute(&self, ty: &Type) -> Result<Type, TypeComputeError> {
        // 实现条件类型求值
    }
}

struct DeadCodeEliminator {
    // 死代码消除
    fn eliminate(&mut self, ir: &mut IR) {
        // 实现DCE算法
    }
}
```

#### 4. 代码生成器

```rust
struct Codegen {
    // LLVM IR生成器
    llvm: LLVMCodeGenerator,

    // 优化器
    optimizer: Optimizer,
}

impl Codegen {
    fn generate(&self, ir: &IR) -> LLVMModule {
        // 1. 生成LLVM IR
        let llvm_ir = self.llvm.translate(ir);

        // 2. 运行优化pass
        let optimized_ir = self.optimizer.optimize(llvm_ir);

        // 3. 生成目标代码
        optimized_ir.to_object_code()
    }
}
```

### 性能优化

#### 1. 实例化策略

```rust
enum InstantiationStrategy {
    // 急切实例化：编译期立即实例化所有可能的使用
    Eager,

    // 延迟实例化：仅实例化实际被调用的泛型
    Lazy,

    // 阈值实例化：小型泛型急切，大型泛型延迟
    Threshold(usize),  // 超过阈值则延迟
}

impl Instantiator {
    fn should_instantiate(&self, generic: &Function, usage_count: usize) -> bool {
        match self.strategy {
            InstantiationStrategy::Eager => true,
            InstantiationStrategy::Lazy => usage_count > 0,
            InstantiationStrategy::Threshold(threshold) => {
                generic.size() < threshold || usage_count > 0
            }
        }
    }
}
```

#### 2. 缓存策略

```rust
struct InstanceCache {
    // LRU缓存
    lru: LruCache<InstanceKey, InstanceId, 1000>,

    // 统计信息
    stats: CacheStats,
}

struct CacheStats {
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl InstanceCache {
    fn get(&mut self, key: &InstanceKey) -> Option<InstanceId> {
        if let Some(&id) = self.lru.get(key) {
            self.stats.hits += 1;
            Some(id)
        } else {
            self.stats.misses += 1;
            None
        }
    }
}
```

#### 3. 代码膨胀控制

```rust
struct CodeSizeManager {
    // 最大实例化数量
    max_instances: usize,

    // 当前实例化数量
    current_instances: usize,

    // 膨胀阈值
    bloat_threshold: f64,
}

impl CodeSizeManager {
    fn should_instantiate(&self, generic: &Function) -> bool {
        if self.current_instances >= self.max_instances {
            return false;
        }

        // 计算预估膨胀
        let膨胀 = self.estimate_bloat(generic);
        if膨胀 > self.bloat_threshold {
            return false;
        }

        true
    }

    fn estimate_bloat(&self, generic: &Function) -> f64 {
        // 基于函数大小和调用次数估算膨胀
        generic.size() as f64 / self.current_instances as f64
    }
}
```

## 实现策略

### 分阶段实现

#### Phase 1: 基础泛型（独立实现）

**目标**：支持基础泛型语法和单态化

**实现**：
- 解析 `[T]` 泛型语法
- 类型变量替换
- 基础实例化
- 简单DCE

**示例**：
```yaoxiang
type Option[T] = Some(T) | None
map: [T, R](opt: Option[T], f: Fn(T) -> R) -> Option[R] = ...
```

**预计工期**：1个月

#### Phase 2: 类型约束（依赖类型系统）

**目标**：支持类型约束和trait系统

**实现**：
- 约束检查
- trait实现验证
- 多重约束
- 函数类型约束

**示例**：
```yaoxiang
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = a.clone() + b
```

**预计工期**：2个月

#### Phase 3: 关联类型（复杂类型系统）

**目标**：支持关联类型和GAT

**实现**：
- 关联类型解析
- 类型族检查
- 依赖类型分析

**示例**：
```yaoxiang
type Iterator[T] = {
    Item: T,
    next: () -> Option[T],
}
```

**预计工期**：3个月

#### Phase 4: 编译期泛型（编译期计算）

**目标**：支持编译期泛型和字面量类型约束

**实现**：
- 字面量类型作为参数类型
- 编译期表达式求值
- 编译期泛型实例化
- 维度验证

**示例**：
```yaoxiang
type Array[T, N: Int] = { data: T[N] }
factorial: [n: Int](n: n) -> Int = (n) => ...
```

**预计工期**：2个月

#### Phase 5: 条件类型（类型级编程）

**目标**：支持条件类型和类型族

**实现**：
- 类型级计算引擎
- 类型归一化
- 约束求解器

**示例**：
```yaoxiang
type If[C: Bool, T, E] = match C {
    True => T,
    False => E,
}
```

**预计工期**：4个月

### 编译器流水线

```
源代码
  ↓
解析器（Parse） - 泛型语法解析
  ↓
类型检查（TypeCheck） - 泛型约束检查
  ↓
实例化（Instantiate） - 泛型实例化
  ↓
单态化（Monomorphize） - 生成具体代码
  ↓
死代码消除（DCE） - 消除未使用实例
  ↓
优化（Optimize） - LLVM优化pass
  ↓
代码生成（Codegen） - 生成目标代码
```

### 关键算法

#### 1. 约束求解算法

```rust
struct ConstraintGraph {
    // 约束节点
    nodes: HashMap<TypeVar, ConstraintNode>,

    // 约束边
    edges: Vec<ConstraintEdge>,
}

impl ConstraintSolver {
    fn solve(&mut self) -> Result<Substitution, TypeError> {
        // 1. 构建约束图
        let graph = self.build_constraint_graph();

        // 2. 拓扑排序
        let order = graph.topological_sort()?;

        // 3. 按顺序求解
        let mut substitution = Substitution::new();
        for constraint in order {
            let binding = self.solve_constraint(constraint)?;
            substitution.extend(binding);
        }

        Ok(substitution)
    }
}
```

#### 2. 单态化算法

```rust
struct Monomorphizer {
    // 实例化映射
    instances: HashMap<InstanceKey, FunctionId>,

    // 待实例化队列
    queue: VecDeque<InstanceKey>,
}

impl Monomorphizer {
    fn monomorphize(&mut self, generic: &Function, args: &[Type]) -> Result<Function, Error> {
        let key = InstanceKey::new(generic.id, args, &[]);

        if let Some(&instance_id) = self.instances.get(&key) {
            return Ok(self.get_instance(instance_id));
        }

        // 1. 替换类型变量
        let substituted = self.substitute_types(generic, args)?;

        // 2. 替换泛型参数
        let instantiated = self.substitute_generics(&substituted, args)?;

        // 3. 注册实例
        let instance_id = self.register_instance(instantiated.clone());
        self.instances.insert(key, instance_id);

        Ok(instantiated)
    }
}
```

#### 3. 类型级计算算法

```rust
struct TypeLevelComputer {
    // 计算缓存
    cache: HashMap<Type, Type>,
}

impl TypeLevelComputer {
    fn compute(&mut self, ty: &Type) -> Result<Type, TypeComputeError> {
        if let Some(&cached) = self.cache.get(ty) {
            return Ok(cached);
        }

        let result = match ty {
            Type::If(condition, true_branch, false_branch) => {
                let computed_cond = self.evaluate_bool(condition)?;
                if computed_cond {
                    self.compute(true_branch)?
                } else {
                    self.compute(false_branch)?
                }
            }
            Type::Match(ty, arms) => {
                let computed_ty = self.compute(ty)?;
                self.match_type(&computed_ty, arms)?
            }
            _ => ty.clone(),
        };

        self.cache.insert(ty.clone(), result.clone());
        Ok(result)
    }
}
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

5. **平台特化**
   - 平台即类型：平台是标准库定义的枚举类型
   - 预定义泛型参数 `P`：解析器自动识别，代表当前编译平台
   - 100% 类型安全：所有平台代码都参与类型检查
   - SIMD指令自动选择
   - 无 `#[cfg]` 等宏，纯类型约束

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

## 实现计划

### 里程碑

| 阶段 | 目标 | 时间 |
|------|------|------|
| **Phase 1** | 基础泛型 + 单态化 | 1个月 |
| **Phase 2** | 类型约束 + trait系统 | 2个月 |
| **Phase 3** | 关联类型 + GAT | 3个月 |
| **Phase 4** | 编译期泛型 + 字面量类型 | 2个月 |
| **Phase 5** | 条件类型 + 类型级编程 | 4个月 |

### 依赖关系

- **Phase 1**: 无依赖，可独立实现
- **Phase 2**: 依赖Phase 1
- **Phase 3**: 依赖Phase 2
- **Phase 4**: 依赖Phase 2
- **Phase 5**: 依赖Phase 3

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
| 特化优先级 | 平台特化 vs 类型特化 | 待讨论 |
| 错误诊断 | 泛型错误信息详细程度 | 待讨论 |

### 后续优化

| 优化项 | 价值 | 实现难度 |
|--------|------|----------|
| 实例化图分析 | 高 | 中 |
| 平台自动特化 | 高 | 高 |
| 类型级编程DSL | 中 | 高 |
| 泛型性能基准 | 中 | 低 |

## 附录

### 语法BNF

```bnf
# 泛型参数（支持约束）
generic_params ::= '[' identifier (',' identifier)* ']'
                 | '[' identifier ':' type_bound (',' identifier ':' type_bound)* ']'
                 | '[' 'P' (':' platform_type)? ']'  # 平台特化：P 是预定义泛型参数

platform_type ::= identifier  # Platform 的变体类型，如 X86_64, AArch64 等

type_bound ::= identifier
             | identifier '+' identifier ('+' identifier)*
             | platform_type

# 参数声明（类型 + 可选名字）
parameter ::= identifier ':' type

parameters ::= parameter (',' parameter)*

# 函数声明：name [泛型] (参数列表) -> 返回类型 = 函数体
function ::= identifier generic_params? '(' parameters? ')' '->' type '=' (expression | block)

# 平台特化函数（使用预定义泛型参数 P）
# P 自动绑定到当前编译平台类型
sum: [P: X86_64](arr: Array[Float]) -> Float = { ... }
sum: [P: AArch64](arr: Array[Float]) -> Float = { ... }

# 方法声明：Type.method [泛型] (参数列表) -> 返回类型 = 函数体
method ::= identifier '.' identifier generic_params? '(' parameters? ')' '->' type '=' (expression | block)

# 类型定义
generic_type ::= 'type' identifier generic_params? '=' type_expression

# 字面量类型参数（编译期函数）
# [n: Int](n: n) 表示 n 是编译期常量，类型为字面量 "n"
literal_param ::= identifier ':' identifier  # 声明泛型参数
               | identifier ':' identifier   # 值参数，类型为泛型参数名（字面量类型）
```

### 平台类型系统

```yaoxiang
# ======== 标准库定义 ========
# 平台类型枚举（可扩展）
type Platform = X86_64 | AArch64 | RISC_V | ARM | X86 | ...

# 平台感知泛型函数
sum: [P: Platform](arr: Array[Float]) -> Float = match P {
    X86_64 => avx2_sum(arr.data, arr.length),
    AArch64 => neon_sum(arr.data, arr.length),
    _ => basic_sum_iter(arr),
}

# 平台类型级计算
type PlatformSupportsSIMD[P] = match P {
    X86_64 => True,
    AArch64 => True,
    RISC_V => True,
    _ => False,
}
```

### 示例代码库

```yaoxiang
# 标准库泛型实现
# 标准库应该提供完整的泛型容器和算法

type Vec[T] = List[T]  # 别名

type HashMap[K, V] = {
    buckets: Array[List[(K, V)]],
    size: Int,

    get: [K: Eq + Hash](self: HashMap[K, V], key: K) -> Option[V],
    insert: [K: Eq + Hash](self: HashMap[K, V], key: K, value: V) -> Void,
    remove: [K: Eq + Hash](self: HashMap[K, V], key: K) -> Option[V],
}

type HashSet[T] = {
    map: HashMap[T, Void],

    insert: [T: Eq + Hash](self: HashSet[T], value: T) -> Void,
    contains: [T: Eq + Hash](self: HashSet[T], value: T) -> Bool,
    remove: [T: Eq + Hash](self: HashSet[T], value: T) -> Bool,
}

# 泛型算法
sort: [T: Clone + PartialOrd](list: List[T]) -> List[T] = {
    # 实现排序算法
}

binary_search: [T: PartialOrd](array: Array[T], target: T) -> Option[Int] = {
    # 实现二分查找
}

# 函数式操作
filter_map: [T, R](list: List[T], f: Fn(T) -> Option[R]) -> List[R] = {
    result = List[R]()
    for item in list {
        if let Some(mapped) = f(item) {
            result.push(mapped)
        }
    }
    return result
}

# 异步泛型
async_map: [T, R](list: List[T], f: Fn(T) -> Promise[R]) -> Promise[List[R]] = {
    # 异步map操作
}
```

### 性能基准

```yaoxiang
# 泛型性能测试
benchmark_map: () -> Void = {
    # 测试泛型map性能
    array = Array[Int](1, 2, 3, 4, 5, 6, 7, 8, 9, 10)

    start = current_time()
    result = array.map((x) => x * 2)
    end = current_time()

    println("Map time: " + (end - start).to_string())

    # 验证结果
    assert(result.length == 10)
    assert(result[0] == 2)
}

# 编译期计算性能测试
benchmark_const: () -> Void = {
    # 测试编译期泛型性能
    start = compile_time()

    # 大量编译期泛型实例
    arrays = List[Array[Int, 100]]()
    for i in 0..1000 {
        arrays.push(Array[Int, 100]())
    }

    end = compile_time()
    println("Compile time: " + (end - start).to_string())
}
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
