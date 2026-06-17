---
title: Pattern Matching
---

# Pattern Matching

In [match basics](../control-flow/match.md), you learned the basic usage of `match` — literals, identifiers, wildcards. Now we'll dive deep into the full power of YaoXiang pattern matching.

## Complete Pattern Types

According to the syntax specification, the complete definition of `Pattern` is:

```
Pattern     ::= Literal       # Literal pattern: 42, "hello"
            | Identifier      # Identifier pattern: capture value
            | Wildcard        # Wildcard: _
            | StructPattern   # Struct pattern: destructure record
            | TuplePattern    # Tuple pattern: destructure tuple
            | EnumPattern     # Enum pattern: destructure variant
            | OrPattern       # Or pattern: pattern1 | pattern2
```

You've already learned the first three basic patterns in the previous chapter. This chapter focuses on the last four advanced patterns.

## Enum Pattern

The enum pattern is the most commonly used advanced feature of `match`. It can destructure enum variants and extract their inner data.

### Basic Enum Matching

```yaoxiang
# Define Result type
Result: (T: Type, E: Type) -> Type = ok(T) | err(E)

# Function uses match to handle Result
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "Success! The value is: ${value}",
    err(msg) => "Error: ${msg}",
}

a = ok(42)
b = err("connection timeout")

println(handle(a))  # Success! The value is: 42
println(handle(b))  # Error: connection timeout
```

### Option Type

```yaoxiang
# Use Option to avoid null
# Built-in type: Option: (T: Type) -> Type = some(T) | none

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "Has value: ${n}",
    none => "Nothing",
}

println(describe(some(100)))  # Has value: 100
println(describe(none))       # Nothing
```

### Custom Enum

```yaoxiang
# Define color enum
type Color = red | green | blue | rgb(Int, Int, Int)

to_hex: (c: Color) -> String = match c {
    red => "#FF0000",
    green => "#00FF00",
    blue => "#0000FF",
    rgb(r, g, b) => "#${r.to_hex()}${g.to_hex()}${b.to_hex()}",
}

println(to_hex(red))                # #FF0000
println(to_hex(rgb(128, 128, 128))) # #808080
```

The `r`, `g`, `b` in `rgb(r, g, b)` are identifier patterns — they capture the three values inside the `rgb` variant.

## Struct Pattern (Record Destructuring)

Struct patterns let you extract fields of interest directly from a struct:

```yaoxiang
type Point = { x: Float, y: Float }
type Rect = { x: Float, y: Float, width: Float, height: Float }

# Struct pattern destructuring
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(x: 0.0, y: 0.0, width: 10.0, height: 20.0)
println(area(r))  # 200.0
```

`{ width: w, height: h }` means "take the `width` field from the record and bind it to the variable `w`, take the `height` field and bind it to the variable `h`". `x: _` and `y: _` mean "these fields exist but we don't care about their values".

**Simplified syntax**: When a field name and a variable name are the same, you can use shorthand — the compiler automatically destructures into a same-named variable:

```yaoxiang
describe_point: (p: Point) -> String = match p {
    { x: 0.0, y: 0.0 } => "Origin",
    { x, y } => "Coordinate (${x}, ${y})",
}

println(describe_point(Point(x: 0.0, y: 0.0)))  # Origin
println(describe_point(Point(x: 3.0, y: 4.0)))  # Coordinate (3.0, 4.0)
```

## Tuple Pattern

Tuple patterns destructure individual elements of a tuple:

```yaoxiang
type Pair = (Int, String)

first: (p: Pair) -> Int = match p {
    (n, _) => n,
}

second: (p: Pair) -> String = match p {
    (_, s) => s,
}

p = (42, "hello")
println(first(p))   # 42
println(second(p))  # "hello"
```

## Or Pattern

Use `|` to combine multiple patterns to match any one of them:

```yaoxiang
type Token = number(Int) | plus | minus | times | divide | eof

# Group multiple variants as "operator" category
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

println(is_operator(plus))      # true
println(is_operator(number(5))) # false
```

## Guard Expressions (if guards)

Add `if condition` after a match arm so the match only takes effect when the pattern matches **and** the condition holds:

```yaoxiang
type Age = adult(Int) | child(Int)

# Guard expression adds an extra condition
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

println(can_drive(adult(20)))  # true
println(can_drive(adult(16)))  # false
```

Variables in a guard expression come from the preceding pattern — `adult(n) if n >= 18` first captures the value with `n`, then checks `n >= 18`.

## Exhaustiveness Checking

The YaoXiang compiler ensures that `match` covers all possible cases. If a branch is missing, the compiler reports an error:

```yaoxiang
type Direction = north | south | east | west

# ✅ Correct: all four directions are covered
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

# ❌ Compile error: missing west
# broken: (d: Direction) -> Direction = match d {
#     north => east,
#     east => south,
#     south => west,
#     # west not handled → compile error
# }
```

This is an important mechanism in YaoXiang for preventing runtime surprises — once a new variant is added, the compiler will remind you to update every `match` location.

## Nested Patterns

The real power of patterns comes from **nesting** — you can nest one pattern inside another:

```yaoxiang
type Expr = literal(Int) | add(Expr, Expr) | mul(Expr, Expr)

# Nested pattern: match literal inside add
simplify: (e: Expr) -> Expr = match e {
    add(literal(0), right) => right,  # 0 + x = x
    add(left, literal(0)) => left,    # x + 0 = x
    mul(literal(1), right) => right,  # 1 * x = x
    mul(left, literal(1)) => left,    # x * 1 = x
    other => other,
}

e = add(literal(0), literal(5))
println(simplify(e))  # literal(5)
```

In `add(literal(0), right)`, the outer layer is the `add` enum pattern, and the inner layer is the `literal(0)` literal pattern — two levels of nesting, matched in one go.

## Summary

| Pattern Type | Syntax | Purpose |
|--------------|--------|---------|
| Literal | `42`, `"hi"` | Match a value exactly |
| Identifier | `x` | Capture the matched value |
| Wildcard | `_` | Catch-all match |
| Enum | `ok(value)` | Destructure enum variants |
| Struct | `{ x, y }` | Destructure record fields |
| Tuple | `(a, b)` | Destructure tuple elements |
| Or | `a \| b \| c` | Match one of multiple |
| Guard | `pattern if cond` | Add extra condition |

`match` + pattern matching = the most powerful control-flow tool in YaoXiang. Master it, and you'll write safer, clearer code.