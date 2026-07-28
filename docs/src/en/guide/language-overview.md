---
title: Syntax Quick Reference
---

# Syntax Quick Reference

Learn YaoXiang core syntax in 5 minutes. For in-depth learning, visit the [Tutorial](/tutorial/).

## Variables

```yaoxiang
x = 42                    // Immutable (default)
mut y = 0                 // Mutable

name: String = "hello"    // Explicit type
count: Int = 100          // Type annotation

pub version = "1.0"       // Public export
```

## Functions

Everything is `name: type = value`. Functions are also values.

```yaoxiang
// Expression form (returns value directly)
add: (a: Int, b: Int) -> Int = a + b

// Code block form (explicit return)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Lambda (parameter name can be omitted when signature is complete)
double = (x) => x * 2
add = (a, b) => a + b
inc = x => x + 1            // Single parameter can omit parentheses

// Code blocks need return
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// Void functions don't need return
greet: (name: String) -> Void = {
    io.println("Hello, " + name)
}
```

## Types

No `type`, `struct`, `trait`, `impl` keywords. One unified declaration handles everything.

```yaoxiang
// Record type
Point: Type = { x: Float, y: Float }
p = Point(1.0, 2.0)            // Positional arguments
p = Point(x=1.0, y=2.0)        // Named arguments

// Fields with default values
Point: Type = { x: Float = 0, y: Float = 0 }
Point()                        // OK: x=0, y=0
Point(x=1.0)                   // OK: x=1.0, y=0

// Variant type (enum)
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Interface (record type where all fields are function types)
Drawable: Type = { draw: (Surface) -> Void }

// Interface composition
DrawableSerializable: Type = Drawable & Serializable

// Interface implementation declared within type
Circle: Type = {
    radius: Float,
    Drawable,              // Implement Drawable trait
    Serializable,          // Implement Serializable trait
}

// Generic type
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
}

// Generic constraint
clone: (T: Clone)(value: T) -> T = value.clone()
sort: (T: Clone + PartialOrd)(list: List(T)) -> List(T)
```

## Methods

```yaoxiang
// Namespace function (Type.method is just an ownership marker, not a binding)
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    return (dx * dx + dy * dy).sqrt()
}

// Only after explicit binding can you use . call syntax
Point.distance = distance[0]
// After this, p1.distance(p2) → distance(p1, p2)

// Quick define + bind
Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

## Control Flow

```yaoxiang
// if is an expression
grade = if score >= 90 { "A" } else if score >= 60 { "B" } else { "C" }

// match
result = match value {
    ok(v) => "success: {v}",
    err(e) => "error: {e}",
    _ => "unknown",
}

// Loop
for i in 0..5 { io.println(i) }
for item in items { io.println(item) }

mut n = 0
while n < 5 { io.println(n); n = n + 1 }
```

## Data Structures

```yaoxiang
// List
nums = [1, 2, 3, 4, 5]
first = nums[0]           // 1

// Dictionary
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       // 90

// List comprehension
evens = [x for x in nums if x % 2 == 0]
doubled = [x * 2 for x in nums]
```

## Pattern Matching

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

// Struct/Tuple pattern
match p {
    { x: 0, y: 0 } => "origin",
    { x, y } => "({x}, {y})",
}
match t {
    (0, 0) => "origin",
    (x, y) => "({x}, {y})",
}

// Destructuring assignment
a, b = (1, 2)              // a=1, b=2

// Guard expression
match age {
    n if n >= 18 => true,
    _ => false,
}
```

## Modules and Imports

```yaoxiang
use std.io
use std.math.{sqrt, sin, cos}
use std.{io, list}

io.println("hello")
result = sqrt(16)         // 4.0

// Alias
use std.math as math
use std.{io as print}

// Public export
pub add: (a: Int, b: Int) -> Int = a + b
pub Point: Type = { x: Float, y: Float }
```

## Ownership

```yaoxiang
// Move: default ownership transfer
p1 = Point(1.0, 2.0)
p2 = p1                   // p1 is moved away

// Borrow &: automatically creates token (no manual & needed)
distance: (a: &Point, b: &Point) -> Float = ...
d = distance(p1, p2)      // Compiler automatically creates borrow token

// Mutable borrow &mut
update: (p: &mut Point, x: Float) -> Void = { p.x = x }

// ref: shared ownership (compiler automatically chooses Rc/Arc)
shared = ref data

// clone: explicit deep copy
backup = data.clone()
```

## Concurrency

spawn is the only parallelism primitive. No async/await, no Send/Sync.

```yaoxiang
// spawn block: sub-expressions automatically parallelize
result = spawn {
    user = fetch_user(1)
    posts = fetch_posts()
    return (user, posts)
}

// spawn for: data parallelism
results = spawn for item in items {
    return process(item)
}

// spawn + ref: shared across tasks
main = {
    shared = ref data
    result = spawn {
        a = shared
        return a
    }
}
```

## F-string

```yaoxiang
name = "YaoXiang"
io.println(f"Hello {name}")          // Hello YaoXiang
io.println(f"Sum: {10 + 20}")        // Sum: 30
io.println(f"Pi: {pi:.2f}")          // Pi: 3.14
```
