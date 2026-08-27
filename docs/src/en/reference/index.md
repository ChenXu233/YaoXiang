# YaoXiang Reference Documentation

> This document is under construction...

YaoXiang is currently in the **experimental verification phase**, with the standard library and API
being gradually improved.

## Language Specification

- [Language Specification Overview](./language-spec/index.md)
- [Syntax Specification](./language-spec/syntax.md) - Lexical structure, grammar rules, operator
  precedence
- [Type System](./language-spec/type-system.md) - Basic types, composite types, generics, trait
- [Module System](./language-spec/modules.md) - Module definition, import/export, scope
- [Concurrency Model](./language-spec/concurrency.md) - Asynchronous programming, concurrency
  primitives, memory model
- [Standard Library](./language-spec/stdlib.md) - Core library, IO library, math library

## Current Status

| Module           | Status                | Description            |
| ---------------- | --------------------- | ---------------------- |
| `std.io`         | 🔨 Under Construction | Input/Output           |
| `std.string`     | 🔨 Under Construction | String operations      |
| `std.list`       | 🔨 Under Construction | List operations        |
| `std.dict`       | ✅ Implemented        | Dictionary operations  |
| `std.math`       | 🔨 Under Construction | Mathematical functions |
| `std.net`        | 📋 Planned            | Network operations     |
| `std.concurrent` | 📋 Planned            | Concurrency primitives |

## Built-in Types

### Primitive Types

| Type     | Description            | Example         |
| -------- | ---------------------- | --------------- |
| `Void`   | Void / no return value | `()`            |
| `Bool`   | Boolean value          | `true`, `false` |
| `Int`    | Integer                | `42`, `-10`     |
| `Float`  | Float                  | `3.14`, `-0.5`  |
| `Char`   | Character              | `'a'`, `'中'`   |
| `String` | String                 | `"hello"`       |

### Composite Types

| Type                 | Description                 | Example        |
| -------------------- | --------------------------- | -------------- |
| `Tuple(T1, T2, ...)` | Heterogeneous element tuple | `(1, "hello")` |
| `(Args) -> Ret`      | Function type               | `(Int) -> Int` |

> #299: Container types (`List(T)` / `Array(T, N)` / `Dict(K, V)`) are not built-in primitives —
> they are generic type constructors, treated the same as user-defined generics, and processed
> through a unified generic instantiation path. Literal syntax (`[...]` / `{...}`) is reserved in
> the core, with the landing point determined by context annotations. Set has been removed (#300),
> see [Language Specification](language-spec/syntax.md).

### User-defined Types

```yaoxiang
// 记录类型（结构体）
Point: Type = { x: Float, y: Float }

// 枚举类型
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// 接口类型（所有字段为函数）
Callable: Type = { call: (String) -> Void }
```

## Built-in Functions

### Output

```yaoxiang
print(value)           // 打印，无换行
println(value)         // 打印，有换行
```

### Conversion

```yaoxiang
to_string(value)       // 转换为字符串
to_int(value)          // 转换为整数
to_float(value)        // 转换为浮点数
```

### Type Checking

```yaoxiang
typeof(value)         // 返回类型名称
is_type(value, type)  // 检查类型
```

## Keywords

| Keyword                   | Description         |
| ------------------------- | ------------------- |
| `Type`                    | Meta type           |
| `spawn`                   | Mark spawn function |
| `spawn for`               | Parallel loop       |
| `spawn {}`                | Spawn block         |
| `if` / `else if` / `else` | Conditional branch  |
| `match`                   | Pattern matching    |
| `while` / `for`           | Loop                |
| `return`                  | Return value        |
| `ref`                     | Create reference    |
| `mut`                     | Mutable marker      |

## Syntax Cheatsheet

### Variable Declaration

```yaoxiang
// 不可变变量（默认）
x: Int = 42
y = 42                 // 类型推断

// 可变变量
mut count: Int = 0
count = count + 1
```

### Function Definition

```yaoxiang
// 普通函数
add: (a: Int, b: Int) -> Int = a + b

// 并作函数（自动并发）
fetch: (url: String) -> JSON spawn = HTTP.get(url).json()

// 泛型函数
identity: [T](x: T) -> T = x
```

### Control Flow

```yaoxiang
// 条件
if x > 0 {
    print("positive")
} else if x < 0 {
    print("negative")
} else {
    print("zero")
}

// 模式匹配
match result {
    ok(value) => print("success: " + value),
    err(error) => print("error: " + error),
}

// 循环
for i in 0..10 {
    print(i)
}
```

### Error Handling

```yaoxiang
// ? 运算符传播错误
data = fetch_file(path)?
```

## Operator Precedence

| Precedence | Operator                     |
| ---------- | ---------------------------- |
| Highest    | `( )` Function call          |
|            | `.` Field access             |
|            | `[ ]` Index                  |
|            | `unary -` Unary minus        |
|            | `* / %` Mul, Div, Mod        |
|            | `+ -` Add, Subtract          |
|            | `== != < > <= >=` Comparison |
|            | `and or` Logical operations  |
| Lowest     | `=` Assignment               |

## Standard Library Usage Examples

```yaoxiang
// 导入标准库
use std.io.{print, println}

// 列表操作
use std.list.{list_push, list_pop, list_len}

// 数学函数
use std.math.{sqrt, sin, cos, PI}

// 使用
println("Hello, YaoXiang!")
result = sqrt(16.0)  // 4.0
```

## Command Line Tool

```bash
# 运行脚本
yaoxiang run hello.yx

# 构建字节码
yaoxiang build hello.yx -o hello.42

# 解释执行
yaoxiang eval 'println("Hello")'

# 查看帮助
yaoxiang --help
```

## Complete Example

```yaoxiang
// 计算斐波那契数列
fib: (n: Int) -> Int = if n <= 1 {
    n
} else {
    fib(n - 1) + fib(n - 2)
}

// 主函数
main: () -> Void = {
    print("Fibonacci(10) = " + fib(10).to_string())
}
```

## Related Resources

- [Tutorials](../tutorial/) - Learn YaoXiang
- [Design Documents](../design/) - Language design decisions
- [GitHub](https://github.com/ChenXu233/YaoXiang)

## Contribution Guide

The standard library is under construction, contributions are welcome!

1. Choose a module (e.g. `std.io`, `std.net`)
2. Implement functions in `src/std/`
3. Add documentation comments
4. Submit PR
