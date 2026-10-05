---
title: 'Syntax Cheat Sheet'
---

# Syntax Cheat Sheet

Understand the core syntax of YaoXiang in 5 minutes. For in-depth learning, visit the
[Tutorial](../tutorial/index.md).

## Variables

```yaoxiang
x = 42                    // Immutable (default)
mut y = 0                 // Mutable

name: String = "hello"    // Explicit type
count: Int = 100          // Type annotation

pub version = "1.0"       // Public export
```

## Functions

Everything is `name: type = value`. Functions are values too.

```yaoxiang
use std.io

// Expression form (returns the value directly)
add: (a: Int, b: Int) -> Int = a + b

// Block form (explicit return)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Lambda (parameter names can be omitted when the signature is complete)
double = (x) => x * 2
add = (a, b) => a + b
inc = x => x + 1            // Single parameter can omit parentheses

// A code block requires return
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// Void functions do not need return
greet: (name: String) -> Void = {
    io.println("Hello, " + name)
}
```

## Types

There are no `type`, `struct`, `trait`, or `impl` keywords. A single unified declaration handles
everything.

<!-- docs-example: skip -->

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

// Note: the standard library already provides std.option (Option + Try); defining your own is not recommended
// Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
// Note: bare constructors ok/err currently report E1001 (verified); see std.result in the standard library
// Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Interface (a record type whose fields are all function types)
Drawable: Type = { draw: (Surface) -> Void }

// Interface composition
DrawableSerializable: Type = Drawable & Serializable

// Implementing an interface inside a type declaration
Circle: Type = {
    radius: Float,
    Drawable,              // Implements Drawable interface
    Serializable,          // Implements Serializable interface
}

// Generic type
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
}

// Generic constraint
// Generic constraint signatures are not yet supported by the current parser, verified to report E0011
clone: (value: T) -> T = value
// Generic constraint signatures are not yet supported by the current parser, verified to report E0011
sort: (list: List(T)) -> List(T)
```

## Methods

<!-- docs-example: skip -->

```yaoxiang
// Namespace function (Type.method is just an ownership tag, not a binding)
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    // Note: parenthesized expressions cannot directly carry a method (verified E1053); bind to a variable first
    d2 = dx * dx + dy * dy
    return d2.sqrt()
}

// Only after explicit binding does the . call syntax become available
Point.distance = distance[0]
// After this, p1.distance(p2) → distance(p1, p2)

// Quick definition + binding
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

// Loops
for i in 0..5 { io.println(i) }
for item in items { io.println(item) }

mut n = 0
while n < 5 { io.println(n); n = n + 1 }
```

## Data Structures

```yaoxiang
use std.dict
use std.list

// List
nums = [1, 2, 3, 4, 5]
first = nums[0]           // 1

// Dict
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       // 90

// List comprehension
evens = [x for x in nums if x % 2 == 0]
doubled = [x * 2 for x in nums]
```

## Pattern Matching

> **Note**: 0.8.2 **does not yet implement record/struct destructuring patterns** (verified that
> `match s { circle(r) => ... }` reports E0010). The following is the target syntax, currently not
> runnable.

<!-- docs-example: skip -->

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

// Struct/tuple patterns
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
// Import an entire module, access via namespace
use std.io

// Import only specified entries, use them directly
use std.math.{sqrt, sin, cos}
use std.{list, string}

// Inline alias for an entry
use std.io.{print as say}

io.println("hello")
result = sqrt(16.0)       // 4.0 (note: passing an Int returns 0.0; you must pass Float)
say("aliased")

// All top-level bindings in a module can be imported externally by default; no pub needed
add: (a: Int, b: Int) -> Int = a + b
Point: Type = { x: Float, y: Float }
```

See [Module System](modules) for module paths, directory entry conventions, and the full form of
`use`.

## Ownership

```yaoxiang
// Move: ownership transfer by default
p1 = Point(1.0, 2.0)
p2 = p1                   // p1 is moved

// Borrow &: automatically create a token (no manual & needed)
distance: (a: &Point, b: &Point) -> Float = ...
d = distance(p1, p2)      // The compiler automatically creates borrow tokens

// Mutable borrow &mut
update: (p: &mut Point, x: Float) -> Void = { p.x = x }

// ref: shared ownership (compiler automatically picks Rc/Arc)
shared = ref data

// clone: explicit deep copy
backup = data.clone()
```

## Concurrency

spawn is the only parallel primitive. No async/await, no Send/Sync.

<!-- docs-example: skip -->

```yaoxiang
// spawn block: sub-expressions automatically run in parallel
result = spawn {
    user = fetch_user(1)
    posts = fetch_posts()
    return (user, posts)
}

// spawn for: data parallelism
results = spawn for item in items {
    return process(item)
}

// spawn + ref: sharing across tasks
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
print(f"Pi: {pi}")                    // Verified: format specifiers are not yet implemented, outputs 3.14159
```
