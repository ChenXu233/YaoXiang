---
title: 'Pattern Matching'
---

# Pattern Matching

In [match basics](../control-flow/match.md), you learned the basic usage of `match`—literals,
identifiers, wildcards. Now we dive deeper into the full power of YaoXiang pattern matching.

## Complete Pattern Types

According to the syntax specification, the full definition of `Pattern` is:

```
Pattern     ::= Literal       # Literal pattern: 42, "hello"
            | Identifier      # Identifier pattern: capture value
            | Wildcard        # Wildcard: _
            | StructPattern   # Struct pattern: destructure record
            | TuplePattern    # Tuple pattern: destructure tuple
            | EnumPattern     # Enum pattern: destructure variant
            | OrPattern       # Or pattern: pattern1 | pattern2
```

You have already learned the first three basic patterns in the previous chapter. This chapter
focuses on the last four advanced patterns.

## Enum Patterns

Enum patterns are the most commonly used advanced feature of `match`. They can destructure enum
variants and extract internal data.

### Basic Enum Matching

```yaoxiang
use std.result

// Function uses match to handle Result
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

### Option Type

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

### Custom Enum

```yaoxiang
// Define color enum
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

`r`, `g`, `b` in `rgb(r, g, b)` are identifier patterns—they capture the three values inside the
`rgb` variant.

## Struct Patterns (Record Destructuring)

Struct patterns let you directly extract fields of interest from a struct:

```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Struct pattern destructuring
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(x= 0.0, y= 0.0, width= 10.0, height= 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` means "take the `width` field from the record and bind it to variable `w`,
take the `height` field and bind it to variable `h`". `x: _` and `y: _` mean "these fields exist but
we don't care about the values".

**Shorthand syntax**: When the field name and variable name are the same, you can abbreviate—the
compiler automatically destructures into a variable with the same name:

```yaoxiang
Point: Type = { x: Float, y: Float }

area_of: (p: Point) -> Float = p.x * p.y

main: () -> Void = {
    p = Point(x= 3.0, y= 4.0)
    print(area_of(p))  // 12.0
}
```

## Tuple Patterns

Tuple patterns destructure the individual elements of a tuple:

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

## Or Patterns

Use `|` to combine multiple patterns and match any one of them:

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

## Guard Expressions (if guards)

Add `if condition` after a match arm, so the match only takes effect when the pattern matches
**and** the condition is satisfied:

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

The variables in a guard expression come from the preceding pattern—`adult(n) if n >= 18` first
captures the value with `n`, then checks `n >= 18`.

## Exhaustiveness Check

The YaoXiang compiler ensures that `match` covers all possible cases. If a branch is missing, the
compiler will report an error:

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

// ❌ Compile error: missing west
// broken: (d: Direction) -> Direction = match d {
//     north => east,
//     east => south,
//     south => west,
//     // west not handled → compile error
// }
```

This is an important mechanism in YaoXiang for preventing runtime surprises—as soon as a new variant
is added, the compiler will remind you to update all `match` locations.

## Nested Patterns

The real power of patterns comes from **nesting**—you can nest one pattern inside another:

```yaoxiang
Expr: Type = { literal: (Int) -> Expr, add: (Expr, Expr) -> Expr, mul: (Expr, Expr) -> Expr }

// 嵌套模式：在 add 内部再匹配 literal
// ⚠️ 每个变体只能有一条臂——写成 add(literal(0), right) 与
// add(left, literal(0)) 两条会报 E1031 Unreachable pattern
simplify: (e: Expr) -> Int = match e {
    add(literal(0), right) => 1,  // 0 + x = x
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

In `add(literal(0), right)`, the outer layer is an `add` enum pattern, and the inner layer is a
`literal(0)` literal pattern—two levels of nesting, matched in one go.

## Summary

| Pattern Type | Syntax            | Use Case              |
| ------------ | ----------------- | --------------------- |
| Literal      | `42`, `"hi"`      | Match exact value     |
| Identifier   | `x`               | Capture matched value |
| Wildcard     | `_`               | Catch-all match       |
| Enum         | `ok(value)`       | Destructure variant   |
| Struct       | `{ x, y }`        | Destructure fields    |
| Tuple        | `(a, b)`          | Destructure elements  |
| Or           | `a \| b \| c`     | Match any of several  |
| Guard        | `pattern if cond` | Extra condition check |

`match` + pattern matching = the most powerful control flow tool in YaoXiang. Master it, and you
will write safer, clearer code.
