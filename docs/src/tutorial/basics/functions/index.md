---
title: 函数定义与调用
---

# 函数定义与调用

在上一章中，你学会了如何声明变量。本章将带你掌握 YaoXiang 的核心——函数。YaoXiang 的函数语法与变量声明共享同一个 `name: type = value` 模型，所以你应该会觉得似曾相识。

## 函数就是 Lambda

先说一个最重要的概念：**在 YaoXiang 中，函数本质上就是一个 lambda 表达式**。没有特殊的 `fn` 关键字，没有复杂的仪式。定义一个函数，就是给一个 lambda 赋个名字。

```
# 任何函数本质上都是这四样东西的组合：
name: (params) -> Return = body
 ^       ^        ^        ^
 |       |        |        +-- 函数体（lambda 表达式或代码块）
 |       |        +-- 返回值类型
 |       +-- 参数列表（签名）
 +-- 函数名
```

这和你在上一章学到的 `name: type = value` 完全一致——只是这里的"类型"恰好是一个函数类型。

---

## 表达式形式：直接返回值

最简单的函数不需要 `return` 关键字。当函数体是单个表达式时，它直接作为返回值：

```yaoxiang
# 表达式形式——直接返回值，无需 return
add: (a: Int, b: Int) -> Int = a + b
square: (x: Int) -> Int = x * x
greet: (name: String) -> String = "你好, " + name
```

调用它们：

```yaoxiang
sum = add(3, 5)          # sum = 8
sq = square(4)           # sq = 16
msg = greet("世界")       # msg = "你好, 世界"
```

这叫做**表达式形式**。当函数体是一个表达式（不是 `{ }` 代码块），它的值直接作为函数的返回值。不需要写 `return`，写了反而是错的。

```yaoxiang
# 正确：表达式直接作为返回值
double: (x: Int) -> Int = x * 2

# 错误：表达式形式中写 return 是语法错误
# double: (x: Int) -> Int = return x * 2   // ❌
```

---

## 代码块形式：显式 return

当函数包含多步计算时，用 `{ }` 代码块包裹函数体。**在代码块中，必须用 `return` 语句来返回值**：

```yaoxiang
# 代码块形式——必须用 return 返回值
factorial: (n: Int) -> Int = {
    if n <= 1 {
        return 1
    }
    return n * factorial(n - 1)
}

# 计算结果
f5 = factorial(5)        # f5 = 120
```

规则很简单：**表达式形式直接返回值；代码块形式必须显式 `return`**。如果在代码块中忘了写 `return`，函数默认返回 `Void`。

```yaoxiang
# 注意：这个函数有 bug
# bad_add: (a: Int, b: Int) -> Int = {
#     a + b   // 没有 return！块默认返回 Void，但签名要求 Int → 类型错误
# }

# 正确写法
good_add: (a: Int, b: Int) -> Int = {
    return a + b
}
```

总结：

| 形式 | 语法 | 返回值方式 |
|------|------|------------|
| 表达式形式 | `name: ... = expr` | 表达式值直接作为返回值 |
| 代码块形式 | `name: ... = { ... }` | 必须用 `return` 显式返回 |

---

## 参数定义

### 基本参数

参数写在函数签名中，每个参数可以标注类型：

```yaoxiang
# 两个参数，都标注了类型
multiply: (a: Int, b: Int) -> Int = a * b
```

### 参数类型必须在签名或 Lambda 头之一标注

YaoXiang 的规则是：**有输入参数时，参数类型必须在签名或 Lambda 头至少一处显式出现**。两边都省略会被编译器拒绝。

```yaoxiang
# 方式一：参数类型写在签名中（省略 Lambda 头）
add: (a: Int, b: Int) -> Int = a + b

# 方式二：参数类型写在 Lambda 头中（省略签名）
add = (a: Int, b: Int) => a + b

# 方式三：完整形式（签名 + Lambda 头都有）
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# 错误：两边都不写类型
# add = (a, b) => a + b   // ❌ 编译器无法推断参数类型
```

**推荐使用方式一**——参数类型写在签名中，省略 Lambda 头。这是最简洁也最清晰的写法。

---

## 返回值

函数的返回值类型写在 `->` 后面。`->` 是函数类型的标志，不能省略（省略后会被解析为其他类型）。

```yaoxiang
# 返回 Int
add_one: (x: Int) -> Int = x + 1

# 返回 String
to_string: (n: Int) -> String = n.to_string()

# 返回 Void（无返回值）
log: (msg: String) -> Void = {
    println(msg)    # 无 return，默认返回 Void
}
```

返回值类型也可以省略，让 HM 类型推断帮你处理：

```yaoxiang
# 编译器推断返回类型为 Int
add = (a: Int, b: Int) => a + b

# 编译器推断返回类型为 String
greet = (name: String) => "你好, " + name
```

---

## 函数调用

### 位置参数

最基本的调用方式——按顺序传参：

```yaoxiang
add: (a: Int, b: Int) -> Int = a + b

result = add(1, 2)        # result = 3
```

语法规范中函数调用的形式定义是：

```
Expr '(' ArgList? ')'
```

翻译成日常语言就是：表达式后面跟一对括号，括号里可以放参数列表。

### 命名参数

除了按位置传参，YaoXiang 还支持**命名参数**——用参数名指定值，顺序不限：

```yaoxiang
# 命名参数——参数名后面跟冒号，然后是值
result = add(a: 3, b: 5)     # result = 8
result = add(b: 5, a: 3)     # 顺序任意，结果相同

# 可以和位置参数混用，但位置参数必须在前面
result = add(3, b: 5)        # OK
```

命名参数让调用更可读，在参数较多时特别有用：

```yaoxiang
# 函数签名
send: (to: String, title: String, body: String) -> Void = {
    println("发送给: " + to)
    println("标题: " + title)
    println("正文: " + body)
}

# 命名参数让调用意图一目了然
send(
    to: "alice@example.com",
    title: "会议通知",
    body: "明天下午 3 点开会"
)
```

---

## 无参函数

不需要参数的函数可以省略参数列表：

```yaoxiang
# 完整形式：显式声明空参数
hello: () -> Void = {
    println("Hello!")
}

# 最简形式：省略签名，编译器自动推断为 () -> Void
hello = {
    println("Hello!")
}

# 调用无参函数
hello()
```

`main` 函数就是最常见的无参函数：

```yaoxiang
# main 函数的几种写法

# 完整形式
main: () -> Void = {
    println("Hello, YaoXiang!")
}

# 最简形式（推荐）
main = {
    println("Hello, YaoXiang!")
}
```

---

## 多行函数

当函数逻辑比较复杂时，用代码块形式组织代码。YaoXiang 强制使用 4 个空格缩进：

```yaoxiang
# 多步计算
calculate_stats: (numbers: List(Int)) -> Float = {
    # 声明局部变量
    mut total = 0
    mut count = 0

    # 循环累加
    for n in numbers {
        total = total + n
        count = count + 1
    }

    # 避免除零
    if count == 0 {
        return 0.0
    }

    # 返回平均值
    return total:as(Float) / count:as(Float)
}
```

多行函数中可以用 `#` 写注释，可以声明 `mut` 局部变量，可以用 `for` 和 `if` 构建逻辑。

---

## pub 与自动绑定

在模块中，用 `pub` 关键字声明的函数可以被其他模块导入使用。更有趣的是，**`pub` 函数会自动绑定到同文件中定义的类型上**，让你可以用 OOP 风格调用。

```yaoxiang
# point.yx

# 定义类型
type Point = { x: Float, y: Float }

# pub 函数：编译器自动将其绑定为 Point.distance
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# 两种调用方式都可以
p1 = Point(x: 3.0, y: 4.0)
p2 = Point(x: 1.0, y: 2.0)

d1 = distance(p1, p2)       # 函数式调用
d2 = p1.distance(p2)        # OOP 风格调用（语法糖）
```

编译器看到 `pub distance(p1: Point, p2: Point)`，发现 `Point` 在同一个文件中定义，就自动创建了 `Point.distance` 的绑定。你不需要写任何额外的 `impl` 代码。

---

## 快速参考

```yaoxiang
# ── 函数定义语法一览 ──

# 表达式形式（最常用）
add: (a: Int, b: Int) -> Int = a + b

# 代码块形式（多步逻辑）
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

# 无参函数（最简）
main = { println("Hello!") }

# 有参—省略签名
double = (x: Int) => x * 2

# 有参—省略 Lambda 头（推荐）
triple: (x: Int) -> Int = x * 3

# pub 导出 + 自动绑定
pub add: (a: Int, b: Int) -> Int = a + b

# ── 调用语法 ──

result = add(1, 2)          # 位置参数
result = add(a: 1, b: 2)    # 命名参数
result = add(1, b: 2)       # 混用（位置在前）
```

---

## 小结

你已经掌握了 YaoXiang 函数的核心知识：

- **统一语法**：`name: (params) -> Return = body`，和变量声明的 `name: type = value` 同源
- **表达式形式**：`= expr`，表达式值直接作为返回值，不需要 `return`
- **代码块形式**：`= { ...; return expr }`，块内必须用 `return` 显式返回
- **参数类型标注**：签名或 Lambda 头至少一处写类型，推荐写在签名中
- **调用**：位置参数或命名参数，命名参数顺序任意
- **pub 自动绑定**：`pub` 函数自动绑定到同文件的类型上，支持 `obj.method()` 调用
- **无参最简**：`name = { ... }`，编译器自动推断为 `() -> Void`

下一步，你可以继续学习[控制流](./control-flow.md)章节，了解如何在函数中使用 `if`、`for` 和 `while`。
