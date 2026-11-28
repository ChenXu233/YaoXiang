---
title: 'Pattern Matching'
---

# Pattern Matching

In [match basics](../control-flow/match.md), you learned the basic uses of `match`—literals,
identifiers, and wildcards. Now we will explore the full capabilities of YaoXiang pattern matching.

## Complete Pattern Types

According to the syntax specification, the complete definition of `Pattern` is:

```
Pattern     ::= Literal       # Literal pattern: 42, "hello"
            | Identifier      # Identifier pattern: captures the value
            | Wildcard        # Wildcard: _
            | StructPattern   # Struct pattern: destructures a record
            | TuplePattern    # Tuple pattern: destructures a tuple
            | EnumPattern     # Enum pattern: destructures a variant
            | OrPattern       # Or pattern: pattern1 | pattern2
```

You have already learned the first three basic patterns in the previous chapter. This chapter
focuses on the latter four advanced patterns.

## Enum Patterns

Enum patterns are the most commonly used advanced feature of `match`. They can destructure enum
variants and extract the data inside them.

### Basic Enum Matching

```yaoxiang
use std.result

// The function uses match to handle Result
handle: (r: Result(Int, String)) -> String = match r {
    ok(value) -> "成功！得到的值是: {value}",
    err(msg) -> "出错啦: {msg}",
}

main: () -> Void = {
    // Variant constructors must be type-qualified—writing bare ok(42) reports E1001
    a = Result(Int, String).ok(42)
    b = Result(Int, String).err("连接超时")

    print(handle(a))  // Output: 成功！得到的值是: 42
    print(handle(b))  // Output: 出错啦: 连接超时
}
```

### The Option Type

```yaoxiang
// Option comes from the standard library: use std.option first to obtain the variant set
use std.option

describe: (opt: Option(Int)) -> String = match opt {
    some(n) -> "有值: {n}",
    none() -> "什么也没有",
}

main: () -> Void = {
    // Variant construction must likewise be type-qualified; none without a payload must be written as none()
    print(describe(Option(Int).some(100)))  // Output: 有值: 100
    print(describe(Option(Int).none()))     // Output: 什么也没有
}
```

### Custom Enums

```yaoxiang
// Define the color enum
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

to_hex: (c: Color) -> String = match c {
    red() -> "#FF0000",
    green() -> "#00FF00",
    blue() -> "#0000FF",
}

main: () -> Void = {
    // Variant construction must be type-qualified; for a non-generic typed enum, the construction result must carry a `: Color` annotation to match
    c: Color = Color.red()
    print(to_hex(c))  // #FF0000
}
```

Variants without data require parentheses in pattern position (`red()`); writing bare `red` is
treated as an identifier pattern, and the compiler immediately reports `E1031 Unreachable pattern`.

## Struct Patterns (Record Destructuring)

⚠️ **Record destructuring patterns are not yet implemented in 0.8.2.** The syntax specification
includes a `StructPattern` entry, but the current parser does not accept it: `{ x: 0.0, y: 0.0 }`
reports `Expected a type, found FloatLiteral` (`E0010`), and `{ x, y }` reports
`Unexpected token: Comma` (`E0011`). The following is the **target syntax**, provided for reading
reference only; it will not compile if pasted into the editor:

<!-- docs-example: skip -->

```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Target syntax: struct pattern destructuring
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } -> w * h,
}

r = Rect(x= 0.0, y= 0.0, width= 10.0, height= 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` means “extract the `width` field from the record and bind it to the
variable `w`, and extract the `height` field and bind it to the variable `h`.” `x: _` and `y: _`
mean “these fields exist, but their values do not matter.” The **shorthand form** is `{ x, y }`—it
can be abbreviated when the field and variable names are the same.

To access a field today, simply access it directly:

```yaoxiang
Point: Type = { x: Float, y: Float }

area_of: (p: Point) -> Float = p.x * p.y

main: () -> Void = {
    p = Point(x= 3.0, y= 4.0)
    print(area_of(p))  // 12.0
}
```

## Tuple Patterns

Tuple patterns destructure the individual elements of a tuple. Note that tuple types must be written
**inline directly**—a tuple alias declaration such as `Pair: Type = (Int, String)` fails to parse
with `E0012`:

```yaoxiang
first: (p: (Int, String)) -> Int = match p {
    (n, _) -> n,
}

second: (p: (Int, String)) -> String = match p {
    (_, s) -> s,
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

// Combine multiple variants into an "operator" category
// Note: variants in pattern position must include parentheses; writing bare `plus | minus` is treated as an identifier pattern,
// and reports E1033 "both sides of `|` must bind the same name set"
is_operator: (t: Token) -> Bool = match t {
    plus() | minus() | times() | divide() -> true,
    _ -> false,
}

main: () -> Void = {
    t: Token = Token.plus()
    print(is_operator(t))  // true
}
```

There are two hard rules for `|`: ① both sides must bind the **same set of variable names**
(`circle(r) | square(r)` is valid, whereas `circle(r) | square(s)` reports `E1033`); ② an or pattern
is used only to select one alternative **within a single match arm**—do not split it into multiple
arms, because `ok(0) -> ...` and `ok(n) -> ...` trigger `E1031 Unreachable pattern`.

## Guard Expressions (if Guards)

Add `if condition` after a match arm so that the match succeeds only when the pattern matches
**and** the condition is satisfied:

```yaoxiang
use std.result

// The guard expression adds an extra condition
// Note: an arm with `if` is not included in exhaustiveness checking; `_` must be added at the end (otherwise E1030)
can_drive: (a: Result(Int, String)) -> Bool = match a {
    ok(n) if n >= 18 -> true,
    _ -> false,
}

main: () -> Void = {
    print(can_drive(Result(Int, String).ok(20)))  // true
    print(can_drive(Result(Int, String).ok(16)))  // false
}
```

The variables in a guard expression come from the preceding pattern—`ok(n) if n >= 18` first
captures the value with `n`, then checks it with `n >= 18`.

## Exhaustiveness Checking

The YaoXiang compiler ensures that `match` covers every possible case. If a branch is missing, the
compiler reports an error:

```yaoxiang
Direction: Type = { north: () -> Direction, south: () -> Direction, east: () -> Direction, west: () -> Direction }

// ✅ Correct: all four directions are covered
// Note: variant construction must be written as `Direction.east()`; bare `east` is an undefined variable (E1001)
turn: (d: Direction) -> Direction = match d {
    north() -> Direction.east(),
    east() -> Direction.south(),
    south() -> Direction.west(),
    west() -> Direction.north(),
}

main: () -> Void = {
    d: Direction = Direction.north()
    r = turn(d)
    print(r)
}

// ❌ Compile error: missing west
// broken: (d: Direction) -> Direction = match d {
//     north() -> Direction.east(),
//     east() -> Direction.south(),
//     south() -> Direction.west(),
//     // west is not handled → compile error
// }
```

This is an important mechanism for preventing unexpected behavior at runtime—once you add a new
variant, the compiler will remind you to update every `match` where it is used.

## Nested Patterns

The true power of patterns comes from **nesting**—you can nest one pattern inside another:

```yaoxiang
Expr: Type = { literal: (Int) -> Expr, add: (Expr, Expr) -> Expr, mul: (Expr, Expr) -> Expr }

// Nested pattern: match literal again inside add
// ⚠️ Each variant may have only one arm—writing add(literal(0), right) and
// add(left, literal(0)) as two arms reports E1031 Unreachable pattern
simplify: (e: Expr) -> Int = match e {
    add(literal(0), right) -> 1,   // Matches 0 + x
    mul(literal(1), right) -> 2,   // Matches 1 * x
    literal(n) -> n,              // Fallback: use the literal itself as the result
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

In `add(literal(0), right)`, the outer layer is the `add` variant pattern, while the inner layer is
the `literal(0)` literal pattern—two levels of nesting in a single match. The **cost is that each
variant may have only one arm**: to distinguish the two shapes `0 + x` and `x + 0`, move the check
into the arm body.

## Summary

| Pattern Type     | Syntax            | Purpose                                              |
| ---------------- | ----------------- | ---------------------------------------------------- |
| Literal          | `42`, `"hi"`      | Match an exact value                                 |
| Identifier       | `x`               | Capture the matched value                            |
| Wildcard         | `_`               | Catch-all match                                      |
| Enum             | `ok(value)`       | Destructure enum variants                            |
| Struct           | `{ x, y }`        | Destructure record fields (not implemented in 0.8.2) |
| Tuple            | `(a, b)`          | Destructure tuple elements                           |
| Or               | `a \| b \| c`     | Match one of several alternatives                    |
| Guard expression | `pattern if cond` | Add a conditional check                              |

`match` + pattern matching = the most powerful control-flow tool in YaoXiang. Master it, and you
will write safer, clearer code.
