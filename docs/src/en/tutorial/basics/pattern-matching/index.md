---
title: Pattern Matching
---

# Pattern Matching

In [match basics](../control-flow/match.md), you learned the fundamentals — literal patterns, identifier patterns, and wildcards. Now let's explore the full power of YaoXiang's pattern matching.

## All Pattern Types

According to the language specification, `Pattern` is fully defined as:

```
Pattern     ::= Literal       # Literal patterns: 42, "hello"
            | Identifier      # Identifier patterns: capture value
            | Wildcard        # Wildcard: _
            | StructPattern   # Struct patterns: destructure records
            | TuplePattern    # Tuple patterns: destructure tuples
            | EnumPattern     # Enum patterns: destructure variants
            | OrPattern       # Or patterns: pattern1 | pattern2
```

You already know the first three. This chapter covers the remaining four.

## Enum Patterns

Enum patterns destructure enum variants and extract their inner data — the most commonly used advanced feature of `match`.

### Basic Enum Matching

```yaoxiang
# Define Result type
Result: (T: Type, E: Type) -> Type = ok(T) | err(E)

# Function using match to handle Result
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "Success! Got value: ${value}",
    err(msg) => "Error: ${msg}",
}

a = ok(42)
b = err("connection timeout")

println(handle(a))  # Success! Got value: 42
println(handle(b))  # Error: connection timeout
```

### Option Type

```yaoxiang
# Use Option to avoid null
# Built-in: Option: (T: Type) -> Type = some(T) | none

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "Got value: ${n}",
    none => "Nothing at all",
}

println(describe(some(100)))  # Got value: 100
println(describe(none))       # Nothing at all
```

### Custom Enums

```yaoxiang
# Define a color enum
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

## Struct Patterns (Record Destructuring)

Struct patterns extract specific fields from a record:

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

`{ width: w, height: h }` means "extract the `width` field and bind it to `w`, extract `height` and bind it to `h`". `x: _` and `y: _` mean "these fields exist but I don't care about their values."

**Shorthand**: when the field name and variable name are the same, you can abbreviate — the compiler auto-destructures to variables with matching names:

```yaoxiang
describe_point: (p: Point) -> String = match p {
    { x: 0.0, y: 0.0 } => "origin",
    { x, y } => "point (${x}, ${y})",
}

println(describe_point(Point(x: 0.0, y: 0.0)))  # origin
println(describe_point(Point(x: 3.0, y: 4.0)))  # point (3.0, 4.0)
```

## Tuple Patterns

Tuple patterns destructure the elements of a tuple:

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

## Or Patterns

Use `|` to combine multiple patterns — the arm matches if any one of them matches:

```yaoxiang
type Token = number(Int) | plus | minus | times | divide | eof

# Group multiple variants as "operators"
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

println(is_operator(plus))      # true
println(is_operator(number(5))) # false
```

## Guard Expressions (if guards)

Add `if condition` after a pattern — the arm matches only when the pattern matches **and** the condition holds:

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

The variables in the guard come from the pattern — `adult(n) if n >= 18` first captures the value as `n`, then checks `n >= 18`.

## Exhaustiveness Checking

The YaoXiang compiler ensures `match` covers all possible cases. If you miss a branch, the compiler reports an error:

```yaoxiang
type Direction = north | south | east | west

# ✅ Correct: all four directions covered
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

# ❌ Compilation error: missing west
# broken: (d: Direction) -> Direction = match d {
#     north => east,
#     east => south,
#     south => west,
#     # west not handled → compiler error
# }
```

This is a key safety mechanism — whenever you add a new variant, the compiler reminds you to update every `match`.

## Nested Patterns

The real power of patterns comes from **nesting** — you can nest one pattern inside another:

```yaoxiang
type Expr = literal(Int) | add(Expr, Expr) | mul(Expr, Expr)

# Nested patterns: match literal inside add
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

In `add(literal(0), right)`, the outer layer is an `add` enum pattern, and the inner layer is a `literal(0)` literal pattern — two levels of nesting, one match.

## Summary

| Pattern Type | Syntax | Use Case |
|--------------|--------|----------|
| Literal | `42`, `"hi"` | Match exact values |
| Identifier | `x` | Capture matched value |
| Wildcard | `_` | Catch-all fallback |
| Enum | `ok(value)` | Destructure enum variants |
| Struct | `{ x, y }` | Destructure record fields |
| Tuple | `(a, b)` | Destructure tuple elements |
| Or | `a \| b \| c` | Match one of many |
| Guard | `pattern if cond` | Add extra conditions |

`match` + pattern matching = the most powerful control flow tool in YaoXiang. Master it, and you'll write safer, clearer code.
