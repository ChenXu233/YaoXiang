---
title: 'Syntax Quick Reference'
---

# Syntax Quick Reference

Understand YaoXiang's core syntax in 5 minutes. For in-depth learning, visit
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
inc = x => x + 1            // parentheses can be omitted for a single parameter

// return is required inside a block
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

There is no `type`, `struct`, `trait`, or `impl` keyword. A single unified declaration handles
everything.

<!-- docs-example: skip -->

```yaoxiang
// Record type
Point: Type = { x: Float, y: Float }
p = Point(1.0, 2.0)            // positional arguments
p = Point(x=1.0, y=2.0)        // named arguments

// Fields with default values
Point: Type = { x: Float = 0, y: Float = 0 }
Point()                        // OK: x=0, y=0
Point(x=1.0)                   // OK: x=1.0, y=0

// Variant type (enum)
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// Note: std.option (Option + Try) is built into the standard library; defining your own is not recommended
// Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
// Note: bare constructors ok/err currently report E1001 (verified); see std.result in the standard library
// Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Interface (a record type whose fields are all function types)
Drawable: Type = { draw: (Surface) -> Void }

// Interface composition
DrawableSerializable: Type = Drawable & Serializable

// Implement interfaces inside a type declaration
Circle: Type = {
    radius: Float,
    Drawable,              // implements the Drawable interface
    Serializable,          // implements the Serializable interface
}

// Generic type
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
}

// Generic constraints
// Generic constraint signatures are not yet supported by the current parser (verified: E0011)
clone: (value: T) -> T = value
// Generic constraint signatures are not yet supported by the current parser (verified: E0011)
sort: (list: List(T)) -> List(T)
```

## Methods

<!-- docs-example: skip -->

```yaoxiang
// Namespace function (Type.method is only an attribution marker, not a binding)
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    // Note: parenthesized expressions cannot directly attach methods (verified E1053); bind to a variable first
    d2 = dx * dx + dy * dy
    return d2.sqrt()
}

// The . call syntax only works after explicit binding
Point.distance = distance[0]
// After this, p1.distance(p2) → distance(p1, p2)

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

// Dictionary
scores = {"Alice": 90, "Bob": 85}
a = scores["Alice"]       // 90

// List comprehension
evens = [x for x in nums if x % 2 == 0]
doubled = [x * 2 for x in nums]
```

## Pattern Matching

> **Note**: 0.8.2 **does not yet implement record/struct destructuring patterns** (verified:
> `match s { circle(r) => ... }` reports E0010). The syntax below is the target syntax and is not
> yet runnable.

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

// Guard expressions
match age {
    n if n >= 18 => true,
    _ => false,
}
```

## Modules and Imports

```yaoxiang
// Import the entire module and access it via the namespace
use std.io

// Import only specified entries for direct bare use
use std.math.{sqrt, sin, cos}
use std.{list, string}

// Inline alias for an entry
use std.io.{print as say}

io.println("hello")
result = sqrt(16.0)       // 4.0 (note: an Int argument returns 0.0; you must pass Float)
say("aliased")

// All top-level bindings of a module are importable by default; no pub required
add: (a: Int, b: Int) -> Int = a + b
Point: Type = { x: Float, y: Float }
```

For module paths, directory entry conventions, and the full form of `use`, see
[Module System](modules).

## Ownership

```yaoxiang
// Move: ownership is transferred by default
p1 = Point(1.0, 2.0)
p2 = p1                   // p1 is moved

// Borrow &: a token is created automatically (no manual & needed)
distance: (a: &Point, b: &Point) -> Float = ...
d = distance(p1, p2)      // the compiler creates borrow tokens automatically

// Mutable borrow &mut
update: (p: &mut Point, x: Float) -> Void = { p.x = x }

// ref: shared ownership (the compiler automatically picks Rc/Arc)
shared = ref data

// clone: explicit deep copy
backup = data.clone()
```

## Concurrency

spawn is the only parallel primitive. No async/await, no Send/Sync.

<!-- docs-example: skip -->

```yaoxiang
// spawn block: sub-expressions run in parallel automatically
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
print(f"Pi: {pi}")                    // verified: format specifiers are not yet implemented; output is 3.14159
```
