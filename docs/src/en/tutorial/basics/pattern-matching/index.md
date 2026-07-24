---
title: Pattern Matching
---

# Pattern Matching

In [match basics](../control-flow/match.md), you learned the basic usage of `match` — literals, identifiers, and wildcards. Now we dive deep into the full capabilities of YaoXiang pattern matching.

## Complete Pattern Types

According to the grammar specification, the full definition of `Pattern` is:

```
Pattern     ::= Literal       # Literal pattern: 42, "hello"
            | Identifier      # Identifier pattern: capture value
            | Wildcard        # Wildcard: _
            | StructPattern   # Struct pattern: destructure record
            | TuplePattern    # Tuple pattern: destructure tuple
            | EnumPattern     # Enum pattern: destructure variant
            | OrPattern       # Or pattern: pattern1 | pattern2
```

You have already learned the first three basic patterns in the previous chapter. This chapter focuses on the four advanced patterns.

## Enum Pattern

Enum pattern is the most commonly used advanced feature of `match`. It can destructure enum variants and extract internal data.

### Basic Enum Matching

```yaoxiang
// Define Result type
Result: (T: Type, E: Type) -> Type = { ok(T) | err(E) }

// Function uses match to handle Result
handle: (result: Result(Int, String)) -> String = match result {
    ok(value) => "Success! The value is: {value}",
    err(msg) => "Error: {msg}",
}

a = ok(42)
b = err("Connection timeout")

print(handle(a))  // Success! The value is: 42
print(handle(b))  // Error: Connection timeout
```

### Option Type

```yaoxiang
// Use Option to avoid null
// Builtin type: Option: (T: Type) -> Type = some(T) | none

describe: (opt: Option(Int)) -> String = match opt {
    some(n) => "Has value: {n}",
    none => "Nothing",
}

print(describe(some(100)))  // Has value: 100
print(describe(none))       // Nothing
```

### Custom Enums

```yaoxiang
// Define Color enum
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

The `r`, `g`, `b` in `rgb(r, g, b)` are identifier patterns — they capture the three values inside the `rgb` variant.

## Struct Pattern (Record Destructuring)

Struct pattern lets you extract fields of interest directly from a struct:

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

`{ width: w, height: h }` means "take the `width` field from the record and bind it to variable `w`, take the `height` field and bind it to variable `h`". `x: _` and `y: _` mean "these fields exist but we don't care about their values".

**Simplified syntax**: When the field name and variable name are the same, you can abbreviate — the compiler automatically destructures into a variable of the same name:

```yaoxiang
describe_point: (p: Point) -> String = match p {
    { x: 0.0, y: 0.0 } => "Origin",
    { x, y } => "Coordinates ({x}, {y})",
}

print(describe_point(Point(0.0, 0.0)))  // Origin
print(describe_point(Point(3.0, 4.0)))  // Coordinates (3.0, 4.0)
```

## Tuple Pattern

Tuple pattern destructures individual elements of a tuple:

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

## Or Pattern

Use `|` to combine multiple patterns, matching any one of them:

```yaoxiang
Token: Type = { number(Int) | plus | minus | times | divide | eof }

// Combine multiple variants into the "operator" class
is_operator: (t: Token) -> Bool = match t {
    plus | minus | times | divide => true,
    _ => false,
}

print(is_operator(plus))      // true
print(is_operator(number(5))) // false
```

## Guard Expressions (if guard)

Add `if condition` after a match arm so the match only takes effect when the pattern matches **and** the condition is satisfied:

```yaoxiang
Age: Type = { adult(Int) | child(Int) }

// Guard expressions add extra conditions
can_drive: (a: Age) -> Bool = match a {
    adult(n) if n >= 18 => true,
    adult(n) if n < 18 => false,
    child(_) => false,
}

print(can_drive(adult(20)))  // true
print(can_drive(adult(16)))  // false
```

Variables in the guard expression come from the preceding pattern — `adult(n) if n >= 18` first uses `n` to capture the value, then checks `n >= 18`.

## Exhaustiveness Check

The YaoXiang compiler ensures that `match` covers all possible cases. If a branch is missing, the compiler will report an error:

```yaoxiang
Direction: Type = { north | south | east | west }

// ✅ Correct: all four directions are covered
turn: (d: Direction) -> Direction = match d {
    north => east,
    east => south,
    south => west,
    west => north,
}

// ❌ Compilation error: west is missing
// broken: (d: Direction) -> Direction = match d {
//     north => east,
//     east => south,
//     south => west,
//     // west not handled → compilation error
// }
```

This is an important mechanism in YaoXiang for preventing runtime surprises — once a new variant is added, the compiler will remind you to update all `match` sites.

## Nested Patterns

The real power of patterns comes from **nesting** — you can nest one pattern inside another:

```yaoxiang
Expr: Type = { literal(Int) | add(Expr, Expr) | mul(Expr, Expr) }

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

In `add(literal(0), right)`, the outer layer is an `add` enum pattern, and the inner layer is a `literal(0)` literal pattern — two layers of nesting, matched in one go.

## Summary

| Pattern Type | Syntax | Use |
|----------|------|------|
| Literal | `42`, `"hi"` | Match exact values |
| Identifier | `x` | Capture the matched value |
| Wildcard | `_` | Fallback match |
| Enum | `ok(value)` | Destructure enum variants |
| Struct | `{ x, y }` | Destructure record fields |
| Tuple | `(a, b)` | Destructure tuple elements |
| Or | `a \| b \| c` | Match one of multiple |
| Guard | `pattern if cond` | Attach extra condition |

`match` + pattern matching = the most powerful control flow tool in YaoXiang. Master it, and you'll write safer, clearer code.