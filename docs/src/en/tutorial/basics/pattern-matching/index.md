---
title: 'Pattern Matching'
---

# Pattern Matching

In [match basics](../control-flow/match.md), you learned the basic usage of `match`—literals,
identifiers, and wildcards. Now we dive deeper into all the capabilities of YaoXiang pattern
matching.

## Complete Pattern Types

According to the language specification, the complete definition of `Pattern` is:

```
Pattern     ::= Literal       # Literal pattern: 42, "hello"
            | Identifier      # Identifier pattern: captures the value
            | Wildcard        # Wildcard: _
            | StructPattern   # Struct pattern: destructures records
            | TuplePattern    # Tuple pattern: destructures tuples
            | EnumPattern     # Enum pattern: destructures variants
            | OrPattern       # Or pattern: pattern1 | pattern2
```

You have already learned the first three basic patterns in the previous chapter. This chapter
focuses on the latter four advanced patterns.

## Enum Patterns

Enum patterns are the most commonly used advanced feature of `match`. They can destructure enum
variants and extract their internal data.

### Basic Enum Matching

```yaoxiang
use std.result

// Function uses match to handle Result
handle: (r: Result(Int, String)) -> String = match r {
    ok(value) => "成功！得到的值是: {value}",
    err(msg) => "出错啦: {msg}",
}

main: () -> Void = {
    // Variant constructors must be "type-qualified"—writing ok(42) bare reports E1001
    a = Result(Int, String).ok(42)
    b = Result(Int, String).err("连接超时")

    print(handle(a))  // Success! The value is: 42
    print(handle(b))  // Error: connection timed out
}
```

### Option Type

```yaoxiang
// Option comes from the standard library: you must use std.option to obtain the variant set
use std.option

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "有值: {n}",
    none() => "什么也没有",
}

main: () -> Void = {
    // Variant construction also requires type qualification; payload-less none must be written as none()
    print(describe(Option(Int).some(100)))  // Has value: 100
    print(describe(Option(Int).none()))     // Nothing here
}
```

### Custom Enums

```yaoxiang
// Define a color enum
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

to_hex: (c: Color) -> String = match c {
    red() => "#FF0000",
    green() => "#00FF00",
    blue() => "#0000FF",
}

main: () -> Void = {
    // Variant construction requires type qualification; for non-generic types, only an explicit `: Color` annotation on the construction result makes the match work
    c: Color = Color.red()
    print(to_hex(c))  // #FF0000
}
```

Variants with no data must be parenthesized in pattern position (`red()`); writing the bare `red`
would be treated as an identifier pattern, and the compiler would then report
`E1031 Unreachable pattern`.

## Struct Patterns (Record Destructuring)

⚠️ **Record destructuring patterns are not yet implemented in 0.8.2.** Although the language
specification includes a `StructPattern` entry, the current parser does not accept it:
`{ x: 0.0, y: 0.0 }` reports "Expected a type, found FloatLiteral" (`E0010`), and `{ x, y }` reports
"Unexpected token: Comma" (`E0011`). The following is the **target syntax**, provided for reading
only—pasting it into the editor will fail to compile:

<!-- docs-example: skip -->

```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Target syntax: struct pattern destructuring
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(x= 0.0, y= 0.0, width= 10.0, height= 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` means "from the record, take the `width` field and bind it to variable
`w`, and take the `height` field and bind it to variable `h`". `x: _` and `y: _` mean "these fields
exist, but we don't care about their values". A **shortened form** is `{ x, y }`—when the field name
and variable name are the same, you can abbreviate.

For now, to access fields, just use direct field access:

```yaoxiang
Point: Type = { x: Float, y: Float }

area_of: (p: Point) -> Float = p.x * p.y

main: () -> Void = {
    p = Point(x= 3.0, y= 4.0)
    print(area_of(p))  // 12.0
}
```

## Tuple Patterns

Tuple patterns destructure the elements of a tuple. Note that tuple types must be **inlined
directly**—a tuple alias declaration like `Pair: Type = (Int, String)` reports `E0012` from the
parser:

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

// Combine multiple variants into an "operator" category
// Note: variants must be parenthesized in pattern position; writing the bare `plus | minus`
// is treated as identifier patterns and reports E1033 "both sides of `|` must bind the same name set"
is_operator: (t: Token) -> Bool = match t {
    plus() | minus() | times() | divide() => true,
    _ => false,
}

main: () -> Void = {
    t: Token = Token.plus()
    print(is_operator(t))  // true
}
```

Two hard rules for `|`: ① both sides must bind the **same set of variable names**
(`circle(r) | square(r)` is valid, while `circle(r) | square(s)` reports `E1033`); ② or patterns are
only used for **multiple choices within a single match arm**—don't split them into multiple arms:
`ok(0) => ...` and `ok(n) => ...` trigger `E1031 Unreachable pattern`.

## Guard Expressions (if Guards)

Add an `if condition` after a match arm, so the match only takes effect when the pattern matches
**and** the condition is satisfied:

```yaoxiang
use std.result

// Guard expression adds an extra condition
// Note: arms with `if` are not counted in exhaustiveness checks; you must add `_` at the end (otherwise E1030)
can_drive: (a: Result(Int, String)) -> Bool = match a {
    ok(n) if n >= 18 => true,
    _ => false,
}

main: () -> Void = {
    print(can_drive(Result(Int, String).ok(20)))  // true
    print(can_drive(Result(Int, String).ok(16)))  // false
}
```

The variables in a guard expression come from the preceding pattern—`ok(n) if n >= 18` first uses
`n` to capture the value, then uses `n >= 18` to check it.

## Exhaustiveness Checking

The YaoXiang compiler ensures `match` covers all possible cases. If a branch is missing, the
compiler reports an error:

```yaoxiang
Direction: Type = { north: () -> Direction, south: () -> Direction, east: () -> Direction, west: () -> Direction }

// ✅ Correct: all four directions are covered
// Note: variant construction must be written as `Direction.east()`; the bare `east` is an undefined variable (E1001)
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
//     north() => Direction.east(),
//     east() => Direction.south(),
//     south() => Direction.west(),
//     // west not handled → compile error
// }
```

This is an important YaoXiang mechanism to prevent runtime surprises—once you add a new variant, the
compiler will remind you to update every `match` site.

## Nested Patterns

The true power of patterns comes from **nesting**—you can nest one pattern inside another:

```yaoxiang
Expr: Type = { literal: (Int) -> Expr, add: (Expr, Expr) -> Expr, mul: (Expr, Expr) -> Expr }

// Nested pattern: match literal inside add
// ⚠️ Each variant can only have one arm—writing add(literal(0), right) and
// add(left, literal(0)) as two arms reports E1031 Unreachable pattern
simplify: (e: Expr) -> Int = match e {
    add(literal(0), right) => 1,   // matches 0 + x
    mul(literal(1), right) => 2,   // matches 1 * x
    literal(n) => n,              // fallback: treat the literal itself as the result
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

In `add(literal(0), right)`, the outer is an `add` variant pattern, and the inner is a `literal(0)`
literal pattern—two levels of nesting, one match. **The trade-off is that each variant can only have
one arm**: to distinguish between the two shapes `0 + x` and `x + 0`, you have to move the check
into the arm body.

## Summary

| Pattern Type     | Syntax            | Purpose                                               |
| ---------------- | ----------------- | ----------------------------------------------------- |
| Literal          | `42`, `"hi"`      | Exact value match                                     |
| Identifier       | `x`               | Captures the matched value                            |
| Wildcard         | `_`               | Catch-all match                                       |
| Enum             | `ok(value)`       | Destructures enum variants                            |
| Struct           | `{ x, y }`        | Destructures record fields (not implemented in 0.8.2) |
| Tuple            | `(a, b)`          | Destructures tuple elements                           |
| Or               | `a \| b \| c`     | Match one of multiple                                 |
| Guard expression | `pattern if cond` | Adds an extra condition                               |

`match` + pattern matching = the most powerful control flow tool in YaoXiang. Master it, and you'll
write safer, clearer code.
