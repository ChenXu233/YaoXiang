---
title: 模式匹配
---

# 模式匹配

在 [match 基础](../control-flow/match.md) 中，你学会了 `match` 的基本用法——字面量、标识符、通配符。现在我们深入探索 YaoXiang 模式匹配的全部能力。

## 完整模式类型

根据语法规范，`Pattern` 的完整定义为：

```
Pattern     ::= Literal       # 字面量模式：42, "hello"
            | Identifier      # 标识符模式：捕获值
            | Wildcard        # 通配符：_
            | StructPattern   # 结构体模式：解构记录
            | TuplePattern    # 元组模式：解构元组
            | EnumPattern     # 枚举模式：解构变体
            | OrPattern       # 或模式：pattern1 | pattern2
```

你已经在前一章学习了前三种基础模式。本章聚焦于后四种进阶模式。

## 枚举模式

枚举模式是 `match` 最常用的高级特性。它能解构枚举变体并提取内部数据。

### 基本枚举匹配

```yaoxiang
// 定义 Result 类型
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// 函数使用 match 处理 Result
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "成功！得到的值是: {value}",
    err(msg) => "出错啦: {msg}",
}

a = ok(42)
b = err("连接超时")

print(handle(a))  // 成功！得到的值是: 42
print(handle(b))  // 出错啦: 连接超时
```

### Option 类型

```yaoxiang
// 使用 Option 避免 null
// 内置类型: Option: (T: Type) -> Type = some(T) | none

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "有值: {n}",
    none => "什么也没有",
}

print(describe(some(100)))  // 有值: 100
print(describe(none))       // 什么也没有
```

### 自定义枚举

```yaoxiang
// 定义颜色枚举
Color: Type = { red | green | blue | rgb(Int, Int, Int) }

to_hex: (c: Color) -> String = match c {
    red => "#FF0000",
    green => "#00FF00",
    blue => "#0000FF",
    rgb(r, g, b) => "#{r.to_hex()}{g.to_hex()}{b.to_hex()}",
}

print(to_hex(red))                // #FF0000
print(to_hex(rgb(128, 128, 128))) // #808080
```

`rgb(r, g, b)` 中的 `r`、`g`、`b` 是标识符模式——它们捕获了 `rgb` 变体内部的三个值。

## 结构体模式（记录解构）

结构体模式让你直接从结构体中提取感兴趣的字段：

```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 结构体模式解构
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(0.0, 0.0, 10.0, 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` 意味着"从记录中取出 `width` 字段绑定到变量 `w`，取出 `height` 字段绑定到变量 `h`"。`x: _` 和 `y: _` 表示"这些字段存在但不关心值"。

**简化写法**：当字段名和变量名相同时，可以缩写——编译器自动解构成同名变量：

```yaoxiang
describe_point: (p: Point) -> String = match p {
    { x: 0.0, y: 0.0 } => "原点",
    { x, y } => "坐标 ({x}, {y})",
}

print(describe_point(Point(0.0, 0.0)))  // 原点
print(describe_point(Point(3.0, 4.0)))  // 坐标 (3.0, 4.0)
```

## 元组模式

元组模式解构元组的各个元素：

```yaoxiang
Pair: Type = (Int, String)

first: (p: Pair) -> Int = match p {
    (n, _) => n,
}

second: (p: Pair) -> String = match p {
    (_, s) => s,
}

p = (42, "hello")
print(first(p))   // 42
print(second(p))  // "hello"
```

## 或模式

用 `|` 将多个模式组合在一起，匹配其中任意一个：

```yaoxiang
Token: Type = { number(Int) | plus | minus | times | divide | eof }

// 将多个变体组合为"运算符"类
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

print(is_operator(plus))      // true
print(is_operator(number(5))) // false
```

## 卫表达式（if 守卫）

在一个匹配臂后面加 `if 条件`，让匹配只在模式匹配**且**条件满足时才生效：

```yaoxiang
Age: Type = { adult(Int) | child(Int) }

// 卫表达式附加额外条件
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

print(can_drive(adult(20)))  // true
print(can_drive(adult(16)))  // false
```

卫表达式中的变量来自前面的模式——`adult(n) if n >= 18` 先用 `n` 捕获值，再用 `n >= 18` 检查。

## 穷尽性检查

YaoXiang 编译器确保 `match` 覆盖了所有可能的情况。如果遗漏分支，编译器会报错：

```yaoxiang
Direction: Type = { north | south | east | west }

// ✅ 正确：四个方向全部覆盖
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

// ❌ 编译错误：缺少 west
// broken: (d: Direction) -> Direction = match d {
//     north => east,
//     east => south,
//     south => west,
//     // west 未处理 → 编译错误
// }
```

这是 YaoXiang 防止运行时意外的重要机制——一旦新增变体，所有 `match` 处编译器都会提醒你更新。

## 嵌套模式

模式的真正威力来自**嵌套**——你可以在一个模式里嵌套另一个模式：

```yaoxiang
Expr: Type = { literal(Int) | add(Expr, Expr) | mul(Expr, Expr) }

// 嵌套模式：在 add 内部再匹配 literal
simplify: (e: Expr) -> Expr = match e {
    add(literal(0), right) => right,  // 0 + x = x
    add(left, literal(0)) => left,    // x + 0 = x
    mul(literal(1), right) => right,  // 1 * x = x
    mul(left, literal(1)) => left,    // x * 1 = x
    other => other,
}

e = add(literal(0), literal(5))
print(simplify(e))  // literal(5)
```

`add(literal(0), right)` 中，外层是 `add` 枚举模式，内层是 `literal(0)` 字面量模式——两层嵌套，一次匹配。

## 小结

| 模式类型 | 语法 | 用途 |
|----------|------|------|
| 字面量 | `42`, `"hi"` | 精确匹配值 |
| 标识符 | `x` | 捕获匹配的值 |
| 通配符 | `_` | 兜底匹配 |
| 枚举 | `ok(value)` | 解构枚举变体 |
| 结构体 | `{ x, y }` | 解构记录字段 |
| 元组 | `(a, b)` | 解构元组元素 |
| 或 | `a \| b \| c` | 多选一匹配 |
| 卫表达式 | `pattern if cond` | 附加条件判断 |

`match` + 模式匹配 = YaoXiang 中最强的控制流工具。掌握它，你将写出更安全、更清晰的代码。
