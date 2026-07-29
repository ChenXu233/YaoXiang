---
title: Pattern Matching
---

# Pattern Matching

In [match basics](../control-flow/match.md), you learned the fundamentals of `match` — literals,
identifiers, and wildcards. Now let's dive deep into the full power of YaoXiang's pattern matching.

## Complete Pattern Types

According to the grammar specification, the complete definition of `Pattern` is:

```
Pattern     ::= Literal       # Literal Pattern: 42, "hello"
            | Identifier      # Identifier Pattern: capture value
            | Wildcard        # Wildcard: _
            | StructPattern   # Struct Pattern: destructure records
            | TuplePattern    # Tuple Pattern: destructure tuples
            | EnumPattern     # Enum Pattern: destructure variants
            | OrPattern       # Or Pattern: pattern1 | pattern2
```

You already learned the first three basic patterns in the previous chapter. This chapter focuses on
the latter four advanced patterns.

## Enum Patterns

Enum patterns are the most commonly used advanced feature of `match`. They can destructure enum
variants and extract internal data.

### Basic Enum Matching

```yaoxiang
// Define Result type
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Function uses match to handle Result
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "Success! Got value: {value}",
    err(msg) => "Error: {msg}",
}

a = ok(42)
b = err("Connection timeout")

print(handle(a))  // Success! Got value: 42
print(handle(b))  // Error: Connection timeout
```

### Option Type

```yaoxiang
// Use Option to avoid null
// Built-in type: Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "Has value: {n}",
    none => "Nothing here",
}

print(describe(some(100)))  // Has value: 100
print(describe(none))       // Nothing here
```

### Custom Enums

```yaoxiang
// Define Color enum
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color, rgb: (Int, Int, Int) -> Color }

to_hex: (c: Color) -> String = match c {
    red => "#FF0000",
    green => "#00FF00",
    blue => "#0000FF",
    rgb(r, g, b) => "#{r.to_hex()}{g.to_hex()}{b.to_hex()}",
}

print(to_hex(red))                // #FF0000
print(to_hex(rgb(128, 128, 128))) // #808080
```

In `rgb(r, g, b)`, `r`, `g`, `b` are identifier patterns — they capture the three values inside the
`rgb` variant.

## Struct Patterns (Record Destructuring)

Struct patterns let you directly extract fields you're interested in from a struct:

```yaoxiang
Point: Type = { x: Float, y: Float }
Rect: Type = { x: Float, y: Float, width: Float, height: Float }

// Struct pattern destructuring
area: (shape: Rect) -> Float = match shape {
    { x: _, y: _, width: w, height: h } => w * h,
}

r = Rect(0.0, 0.0, 10.0, 20.0)
print(area(r))  // 200.0
```

`{ width: w, height: h }` means "extract the `width` field from the record and bind it to variable
`w`, extract the `height` field and bind it to variable `h`". `x: _` and `y: _` mean "these fields
exist but we don't care about their values".

**Shorthand syntax**: When field name and variable name are the same, you can abbreviate — the
compiler automatically destructures into a variable of the same name:

```yaoxiang
describe_point: (p: Point) -> String = match p {
    { x: 0.0, y: 0.0 } => "Origin",
    { x, y } => "Point ({x}, {y})",
}

print(describe_point(Point(0.0, 0.0)))  // Origin
print(describe_point(Point(3.0, 4.0)))  // Point (3.0, 4.0)
```

## Tuple Patterns

Tuple patterns destructure each element of a tuple:

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

## Or Patterns

Combine multiple patterns with `|` to match any one of them:

```yaoxiang
Token: Type = { number: (Int) -> Token, plus: () -> Token, minus: () -> Token, times: () -> Token, divide: () -> Token, eof: () -> Token }

// Combine multiple variants into an "operator" category
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

print(is_operator(plus))      // true
print(is_operator(number(5))) // false
```

## Guard Expressions (if guards)

Add `if condition` after a match arm so the match only succeeds when the pattern matches **and** the
condition is true:

```yaoxiang
Age: Type = { adult: (Int) -> Age, child: (Int) -> Age }

// Guard expressions add extra conditions
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

print(can_drive(adult(20)))  // true
print(can_drive(adult(16)))  // false
```

Variables in guard expressions come from the preceding pattern — `adult(n) if n >= 18` first uses
`n` to capture the value, then checks `n >= 18`.

## Exhaustiveness Checking

The YaoXiang compiler ensures `match` covers all possible cases. If you miss a branch, the compiler
will report an error:

```yaoxiang
Direction: Type = { north: () -> Direction, south: () -> Direction, east: () -> Direction, west: () -> Direction }

// ✅ Correct: all four directions covered
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

// ❌ Compile error: missing west
// broken: (d: Direction) -> Direction = match d {
//     north => east,
//     east => south,
//     south => west,
//     // west not handled → compile error
// }
```

This is an important mechanism in YaoXiang to prevent runtime surprises — once you add a new
variant, the compiler will remind you at every `match` site to update your code.

## Nested Patterns

The real power of patterns comes from **nesting** — you can nest one pattern inside another:

```yaoxiang
Expr: Type = { literal: (Int) -> Expr, add: (Expr, Expr) -> Expr, mul: (Expr, Expr) -> Expr }

// Nested pattern: match literal inside add
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

In `add(literal(0), right)`, the outer layer is an `add` enum pattern, and the inner layer is a
`literal(0)` literal pattern — two levels of nesting, matched at once.

## Summary

| Pattern Type | Syntax            | Purpose                       |
| ------------ | ----------------- | ----------------------------- |
| Literal      | `42`, `"hi"`      | Match values exactly          |
| Identifier   | `x`               | Capture matched values        |
| Wildcard     | `_`               | Fallback match                |
| Enum         | `ok(value)`       | Destructure enum variants     |
| Struct       | `{ x, y }`        | Destructure record fields     |
| Tuple        | `(a, b)`          | Destructure tuple elements    |
| Or           | `a \| b \| c`     | Match any one of multiple     |
| Guard        | `pattern if cond` | Additional condition checking |

`match` + pattern matching = the most powerful control flow tool in YaoXiang. Master it, and you'll
write safer, clearer code.
