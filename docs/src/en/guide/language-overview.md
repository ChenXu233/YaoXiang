---
title: 'Syntax Cheat Sheet'
---

# Syntax Cheat Sheet

Understand YaoXiang's core syntax in 5 minutes. For in-depth learning, visit the
[Tutorial](../tutorial/index.md).

## Variables

```yaoxiang
x = 42                    // immutable (default)
mut y = 0                 // mutable

name: String = "hello"    // explicit type
count: Int = 100          // type annotation

pub version = "1.0"       // public export
```

## Functions

Everything is `name: type = value`. Functions are values too.

```yaoxiang
use std.io

// Expression form (returns value directly)
add: (a: Int, b: Int) -> Int = a + b

// Block form (explicit return)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Lambda (parameter names can be omitted when signature is complete)
double = (x) => x * 2
add = (a, b) => a + b
inc = x => x + 1            // parentheses can be omitted for a single parameter

// Block form requires return
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// Void functions don't require return
greet: (name: String) -> Void = {
    io.println("Hello, " + name)
}
```

## Types

No `type`, `struct`, `trait`, `impl` keywords. One unified declaration handles everything.

<!-- docs-example: skip -->

```yaoxiang
// record type
Point: Type = { x: Float, y: Float }
p = Point(1.0, 2.0)            // positional argument
p = Point(x=1.0, y=2.0)        // named argument

// fields with default values
Point: Type = { x: Float = 0, y: Float = 0 }
Point()                        // OK: x=0, y=0
Point(x=1.0)                   // OK: x=1.0, y=0

// variant type (enum)
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// Note: std.option (Option + Try) is already built into the standard library, defining your own is not recommended
// Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
// Note: bare constructors ok/err currently report E1001 (verified), see std.result in the standard library
// Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Interface (record type where all fields are function types)
Drawable: Type = { draw: (Surface) -> Void }

// Interface composition
DrawableSerializable: Type = Drawable & Serializable

// Declare interface implementation within a type
Circle: Type = {
    radius: Float,
    Drawable,              // implement Drawable interface
    Serializable,          // implement Serializable interface
}

// Generic type
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
}

// Type constraint
// Generic constraint signatures are not yet supported by the current parser, reports E0011 (verified)
clone: (value: T) -> T = value
// Generic constraint signatures are not yet supported by the current parser, reports E0011 (verified)
sort: (list: List(T)) -> List(T)
```

## Methods

<!-- docs-example: skip -->

```yaoxiang
// Namespace function (Type.method is only an ownership marker, not a binding)
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    // Note: parenthesized expressions cannot attach methods directly (verified E1053), must bind to a variable first
    d2 = dx * dx + dy * dy
    return d2.sqrt()
}

// Dot-call syntax only available after explicit binding
Point.distance = distance[0]
// afterwards p1.distance(p2) → distance(p1, p2)

// Quick define + bind
Point.draw: (self: &Point, surface: Surface) -> Void = {
    surface.plot(self.x, self.y)
}
```

## Control Flow

```yaoxiang
use std.result

// if is an expression
score = 72
value: Result(Int, String) = Result(Int, String).ok(1)
grade = if score >= 90 { "A" } else if score >= 60 { "B" } else { "C" }   // "B"

// match
result = match value {
    ok(v) => "success: {v}",
    err(e) => "error: {e}",
    _ => "unknown",
}

// loops
for i in 0..5 { io.println(i) }
for item in items { io.println(item) }

mut n = 0
while n < 5 { io.println(n); n = n + 1 }
```

## Data Structures

```yaoxiang
use std.dict
use std.list

// list
nums = [1, 2, 3, 4, 5]
first = nums[0]           // 1

// dictionary
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       // 90

// list comprehension
evens = [x for x in nums if x % 2 == 0]
doubled = [x * 2 for x in nums]
```

## Pattern Matching

> **Note**: record/struct destructuring patterns are **not yet implemented** in 0.8.2 (verified
> `match s { circle(r) => ... }` reports E0010); the following is the target syntax and is not yet
> runnable.

<!-- docs-example: skip -->

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

// struct/tuple patterns
match p {
    { x: 0, y: 0 } => "origin",
    { x, y } => "({x}, {y})",
}
match t {
    (0, 0) => "origin",
    (x, y) => "({x}, {y})",
}

// destructuring assignment
a, b = (1, 2)              // a=1, b=2

// guard expression
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
result = sqrt(16.0)       // 4.0 (Note: Int arguments return 0.0, must pass Float)

// alias
use std.math as math
use std.{io as print}

// public export
pub add: (a: Int, b: Int) -> Int = a + b
pub Point: Type = { x: Float, y: Float }
```

## Ownership

```yaoxiang
// Move: default ownership transfer
p1 = Point(1.0, 2.0)
p2 = p1                   // p1 is moved away

// Borrow &: auto-create token (no manual & needed)
distance: (a: &Point, b: &Point) -> Float = ...
d = distance(p1, p2)      // compiler auto-creates borrow tokens

// Mutable borrow &mut
update: (p: &mut Point, x: Float) -> Void = { p.x = x }

// ref: shared ownership (compiler auto-selects Rc/Arc)
shared = ref data

// clone: explicit deep copy
backup = data.clone()
```

## Concurrency

spawn is the only parallel primitive. No async/await, no Send/Sync.

<!-- docs-example: skip -->

```yaoxiang
// spawn block: sub-expressions auto-parallel
result = spawn {
    user = fetch_user(1)
    posts = fetch_posts()
    return (user, posts)
}

// spawn for: data parallel
results = spawn for item in items {
    return process(item)
}

// spawn + ref: cross-task sharing
main: () -> Void = {
    shared = ref data
    result = spawn {
        a = shared
        return a
    }
}
```

## F-string

```yaoxiang
pi = 3.14159
name = "YaoXiang"
print(f"Hello {name}")               // Hello YaoXiang
print(f"Sum: {10 + 20}")             // Sum: 30
print(f"Pi: {pi}")                    // Verified: format specifiers not yet implemented, outputs 3.14159
```
