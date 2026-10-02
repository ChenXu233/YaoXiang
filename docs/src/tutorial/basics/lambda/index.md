---
title: 'Lambda 表达式'
---

# Lambda 表达式

Lambda 是**匿名的、可以随手定义的函数**。在 YaoXiang 中，普通函数本质上就是具名的 Lambda。

## 语法

根据语法规范：

```
Lambda      ::= '(' ParamList? ')' '=>' Expr
            |  '(' ParamList? ')' '=>' Block
```

最简单的 Lambda：

```yaoxiang
// 表达式形式的 Lambda
double = (x) => x * 2

print(double(5))   // 10
print(double(10))  // 20
```

## Lambda 与函数的统一

YaoXiang 的核心设计哲学是统一语法。**函数就是绑定到名字的 Lambda**：

```yaoxiang
// 这两者完全等价：

// Lambda 形式
add = (a, b) => a + b

// 函数形式（语法糖）
add: (a: Int, b: Int) -> Int = a + b
```

第一行是"把一个 Lambda 赋值给变量 `add`"，第二行是"定义一个名为 `add`
的函数"。编译器处理它们的方式几乎一样。

## 什么时候用 Lambda

Lambda 最适合两个场景：

### 1. 高阶函数——把函数作为参数传递

```yaoxiang
use std.list

// 对列表的每个元素应用一个操作
// 注意：形参不要命名为 list（会遮蔽 std.list 模块），
// 类型写 Vec(Int)——`List(T)` 标注的值当前不被 for 接受（E1002）
apply_to_all: (xs: Vec(Int), op: (Int) -> Int) -> Vec(Int) = {
    mut result = []
    for item in xs {
        result = list.push(result, op(item))
    }
    return result
}

main = () => {
    // 传入 Lambda
    doubled = apply_to_all([1, 2, 3, 4, 5], (x) => x * 2)
    squared = apply_to_all([1, 2, 3, 4, 5], (x) => x * x)

    print(doubled)  // [2, 4, 6, 8, 10]
    print(squared)  // [4, 16, 36, 64, 100]
}
```

### 2. 临时的一次性操作

不需要为只用一次的逻辑专门定义函数：

```yaoxiang
use std.list

// 注意：0.8.2 的 std.list 没有 sort / sort_by（实测 E1042），
// 排序得自己写；这里演示同样「临时定义规则」的 map / filter
main = () => {
    scores = [90, 85, 92, 78]

    passed = list.filter(scores, (s) => s >= 85)   // [90, 85, 92]
    boosted = list.map(scores, (s) => s + 5)       // [95, 90, 97, 83]

    print(passed)
    print(boosted)
}
```

## 代码块形式的 Lambda

当 Lambda 需要多行逻辑时，用代码块形式：

```yaoxiang
use std.string

// 代码块 Lambda：可以包含多条语句
// 注意：String 没有实例方法——`data.trim()` 报 E1053，用 std.string 的函数形式
process = (data: String) => {
    cleaned = string.trim(data)
    lower = string.lower(cleaned)
    return lower
}

main = () => {
    result = process("  Hello World  ")
    print(result)  // "hello world"
}
```

注意代码块形式需要用 `return` 来返回值，这一点和函数完全一致。

## 多参数 Lambda

```yaoxiang
// 三个参数
add_three = (x, y, z) => x + y + z
print(add_three(1, 2, 3))  // 6

// 无参 Lambda
greet = () => "Hello, YaoXiang!"
print(greet())  // "Hello, YaoXiang!"
```

## 类型推断

Lambda 的参数类型可以从上下文推断：

```yaoxiang
// 类型从使用处推断——不需要写 (x: Int) => x * 2
apply: (op: (Int) -> Int, value: Int) -> Int = op(value)

result = apply((x) => x + 10, 5)
print(result)  // 15
```

编译器知道 `op` 的类型是 `(Int) -> Int`，所以 Lambda `(x) => x + 10` 中的 `x` 自动推断为 `Int`。

> **注意**：根据函数定义的规则，参数类型必须在签名或 Lambda 头至少一处标注。当 Lambda 作为参数传递时，类型通常由接收方的签名提供。

## 小结

| 要点       | 说明                                               |
| ---------- | -------------------------------------------------- |
| 语法       | `(params) => expr` 或 `(params) => { return ... }` |
| 本质       | 函数 = 具名的 Lambda                               |
| 高阶函数   | Lambda 可以当作参数传递                            |
| 代码块形式 | 多行逻辑用 `{}` + `return`                         |
| 类型推断   | 参数类型从上下文自动推断                           |

Lambda 是 YaoXiang 中表达"临时逻辑"最简洁的方式。掌握它，你的代码会更灵活、更紧凑。
