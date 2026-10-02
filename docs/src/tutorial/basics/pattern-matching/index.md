---
title: '模式匹配'
---

# 模式匹配

在 [match 基础](../control-flow/match.md) 中，你学会了 `match`
的基本用法——字面量、标识符、通配符。现在我们深入探索 YaoXiang 模式匹配的全部能力。

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
use std.result

// 函数使用 match 处理 Result
handle: (r: Result(Int, String)) -> String = match r {
    ok(value) => "成功！得到的值是: {value}",
    err(msg) => "出错啦: {msg}",
}

main: () -> Void = {
    // 变体构造器必须「类型限定」——裸写 ok(42) 报 E1001
    a = Result(Int, String).ok(42)
    b = Result(Int, String).err("连接超时")

    print(handle(a))  // 成功！得到的值是: 42
    print(handle(b))  // 出错啦: 连接超时
}
```

### Option 类型

```yaoxiang
// Option 来自标准库：先 use std.option 才能拿到变体集
use std.option

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "有值: {n}",
    none() => "什么也没有",
}

main: () -> Void = {
    // 变体构造同样要类型限定；无载荷的 none 要写成 none()
    print(describe(Option(Int).some(100)))  // 有值: 100
    print(describe(Option(Int).none()))     // 什么也没有
}
```

### 自定义枚举

```yaoxiang
// 定义颜色枚举
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

to_hex: (c: Color) -> String = match c {
    red() => "#FF0000",
    green() => "#00FF00",
    blue() => "#0000FF",
}

main: () -> Void = {
    // 变体构造要类型限定；非泛型和类型的构造结果带 `: Color` 标注才匹配得上
    c: Color = Color.red()
    print(to_hex(c))  // #FF0000
}
```

无数据的变体在模式位置要带括号（`red()`）；写成裸 `red` 会被当成标识符模式，编译器随即
报 `E1031 Unreachable pattern`。

## 结构体模式（记录解构）

⚠️ **0.8.2 尚未实现记录解构模式。** 语法规范里有 `StructPattern` 这一项，但当前解析器
不接受它：`{ x: 0.0, y: 0.0 }` 报「Expected a type, found FloatLiteral」(`E0010`)，
`{ x, y }` 报「Unexpected token: Comma」(`E0011`)。下面是**目标语法**，只作阅读参考，
粘进编辑器编译不过：

<!-- docs-example: skip -->
```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// 目标语法：结构体模式解构
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(x= 0.0, y= 0.0, width= 10.0, height= 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` 意味着"从记录中取出 `width` 字段绑定到变量 `w`，取出 `height`
字段绑定到变量 `h`"。`x: _` 和 `y: _` 表示"这些字段存在但不关心值"。**简化写法**是
`{ x, y }`——字段名与变量名相同时可缩写。

今天想取字段，直接访问即可：

```yaoxiang
Point: Type = { x: Float, y: Float }

area_of: (p: Point) -> Float = p.x * p.y

main: () -> Void = {
    p = Point(x= 3.0, y= 4.0)
    print(area_of(p))  // 12.0
}
```

## 元组模式

元组模式解构元组的各个元素。注意元组类型要**直接内联**写——`Pair: Type = (Int, String)`
这种元组别名声明解析报 `E0012`：

```yaoxiang
first: (p: (Int, String)) -> Int = match p {
    (n, _) => n,
}

second: (p: (Int, String)) -> String = match p {
    (_, s) => s,
}

main: () -> Void = {
    p = (42, "hello")
    print(first(p))   // 42
    print(second(p))  // hello
}
```

## 或模式

用 `|` 将多个模式组合在一起，匹配其中任意一个：

```yaoxiang
Token: Type = { number: (Int) -> Token, plus: () -> Token, minus: () -> Token, times: () -> Token, divide: () -> Token, eof: () -> Token }

// 将多个变体组合为"运算符"类
// 注意：变体在模式位置要带括号，写成裸 `plus | minus` 会被当成标识符模式，
// 报 E1033「both sides of `|` must bind the same name set」
is_operator: (t: Token) -> Bool = match t {
    plus() | minus() | times() | divide() => true,
    _ => false,
}

main: () -> Void = {
    t: Token = Token.plus()
    print(is_operator(t))  // true
}
```

`|` 的两条硬规则：① 两侧必须绑定**同名变量集**（`circle(r) | square(r)` 合法，
`circle(r) | square(s)` 报 `E1033`）；② 或模式只用于**单个匹配臂内**的多选一，
不要拆成多条臂——`ok(0) => ...` 和 `ok(n) => ...` 会触发 `E1031 Unreachable pattern`。

## 卫表达式（if 守卫）

在一个匹配臂后面加 `if 条件`，让匹配只在模式匹配**且**条件满足时才生效：

```yaoxiang
use std.result

// 卫表达式附加额外条件
// 注意：带 `if` 的臂不计入穷尽性检查，末尾必须补 `_`（否则 E1030）
can_drive: (a: Result(Int, String)) -> Bool = match a {
    ok(n) if n >= 18 => true,
    _ => false,
}

main: () -> Void = {
    print(can_drive(Result(Int, String).ok(20)))  // true
    print(can_drive(Result(Int, String).ok(16)))  // false
}
```

卫表达式中的变量来自前面的模式——`ok(n) if n >= 18` 先用 `n` 捕获值，再用 `n >= 18` 检查。

## 穷尽性检查

YaoXiang 编译器确保 `match` 覆盖了所有可能的情况。如果遗漏分支，编译器会报错：

```yaoxiang
Direction: Type = { north: () -> Direction, south: () -> Direction, east: () -> Direction, west: () -> Direction }

// ✅ 正确：四个方向全部覆盖
// 注意：变体构造要写成 `Direction.east()`，裸 `east` 是未定义变量（E1001）
turn: (d: Direction) -> Direction = match d {
    north() => Direction.east(),
    east() => Direction.south(),
    south() => Direction.west(),
    west() => Direction.north(),
}

main: () -> Void = {
    d: Direction = Direction.north()
    r = turn(d)
    print(r)
}

// ❌ 编译错误：缺少 west
// broken: (d: Direction) -> Direction = match d {
//     north() => Direction.east(),
//     east() => Direction.south(),
//     south() => Direction.west(),
//     // west 未处理 → 编译错误
// }
```

这是 YaoXiang 防止运行时意外的重要机制——一旦新增变体，所有 `match` 处编译器都会提醒你更新。

## 嵌套模式

模式的真正威力来自**嵌套**——你可以在一个模式里嵌套另一个模式：

```yaoxiang
Expr: Type = { literal: (Int) -> Expr, add: (Expr, Expr) -> Expr, mul: (Expr, Expr) -> Expr }

// 嵌套模式：在 add 内部再匹配 literal
// ⚠️ 每个变体只能有一条臂——写成 add(literal(0), right) 与
// add(left, literal(0)) 两条会报 E1031 Unreachable pattern
simplify: (e: Expr) -> Int = match e {
    add(literal(0), right) => 1,   // 命中 0 + x
    mul(literal(1), right) => 2,   // 命中 1 * x
    literal(n) => n,              // 兜底：把字面量本身当结果
}

main: () -> Void = {
    five: Expr = Expr.literal(5)
    zero: Expr = Expr.literal(0)
    one: Expr = Expr.literal(1)
    a: Expr = Expr.add(zero, five)
    b: Expr = Expr.mul(one, five)
    print(simplify(a))     // 1
    print(simplify(b))     // 2
    print(simplify(five))  // 5
}
```

`add(literal(0), right)` 中，外层是 `add` 变体模式，内层是 `literal(0)`
字面量模式——两层嵌套，一次匹配。**代价是每个变体只能有一条臂**：想区分
`0 + x` 和 `x + 0` 两种形状，得把判断挪进臂体里做。

## 小结

| 模式类型 | 语法              | 用途         |
| -------- | ----------------- | ------------ |
| 字面量   | `42`, `"hi"`      | 精确匹配值   |
| 标识符   | `x`               | 捕获匹配的值 |
| 通配符   | `_`               | 兜底匹配     |
| 枚举     | `ok(value)`       | 解构枚举变体 |
| 结构体   | `{ x, y }`        | 解构记录字段（0.8.2 未实现） |
| 元组     | `(a, b)`          | 解构元组元素 |
| 或       | `a \| b \| c`     | 多选一匹配   |
| 卫表达式 | `pattern if cond` | 附加条件判断 |

`match` + 模式匹配 = YaoXiang 中最强的控制流工具。掌握它，你将写出更安全、更清晰的代码。
