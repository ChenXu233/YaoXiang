---
title: 'Syntax Cheatsheet'
---

# Syntax Cheatsheet

Grasp the core syntax of YaoXiang in 5 minutes. For deeper learning, see the
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

// expression form (returns the value directly)
add: (a: Int, b: Int) -> Int = a + b

// block form (explicit return)
factorial: (n: Int) -> Int = {
    if n <= 1 { return 1 }
    return n * factorial(n - 1)
}

// Lambda (parameter names can be omitted when the signature is complete)
double = (x) => x * 2
add = (a, b) => a + b
inc = x => x + 1            // parentheses can be omitted for a single parameter

// a block body requires return
process: (x: Int) -> Int = {
    a = x * 2
    b = a + 1
    return b
}

// Void function does not require return
greet: (name: String) -> Void = {
    io.println("Hello, " + name)
}
```

## Types

There are no `type`, `struct`, `trait`, or `impl` keywords. A single unified declaration handles
everything.

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

// Note: std.option (Option + Try) is already built into the standard library; defining your own is not recommended
// Option: (T: Type) -> Type = { some: (T) -> Option(T), none: () -> Option(T) }
// Note: bare constructors ok/err currently report E1001 (verified); see std.result in the standard library
// Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// interface (a record type whose fields are all function types)
Drawable: Type = { draw: (Surface) -> Void }

// interface composition
DrawableSerializable: Type = Drawable & Serializable

// declaring interface implementations within a type
Circle: Type = {
    radius: Float,
    Drawable,              // implements the Drawable interface
    Serializable,          // implements the Serializable interface
}

// generic type
List: (T: Type) -> Type = {
    data: Array(T),
    length: Int,
    push: (self: List(T), item: T) -> Void,
    map: (R: Type) -> ((self: List(T), f: (T) -> R) -> List(R)),
}

// generic constraint
// Generic constraint signatures are not yet supported by the current parser; verified E0011
clone: (value: T) -> T = value
// Generic constraint signatures are not yet supported by the current parser; verified E0011
sort: (list: List(T)) -> List(T)
```

## Methods

<!-- docs-example: skip -->

```yaoxiang
// namespace function (Type.method is just an attribution marker, not a binding)
Point.distance: (a: &Point, b: &Point) -> Float = {
    dx = a.x - b.x
    dy = a.y - b.y
    // Note: parenthesized expressions cannot attach methods directly (verified E1053); bind to a variable first
    d2 = dx * dx + dy * dy
    return d2.sqrt()
}

// the . call syntax is only available after explicit binding
Point.distance = distance[0]
// afterwards p1.distance(p2) → distance(p1, p2)

// quick definition + binding
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

// loop
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

> **Note**: 0.8.2 **has not yet implemented record/struct destructuring patterns** (verified
> `match s { circle(r) => ... }` reports E0010). The following is target syntax and is not yet
> runnable.

<!-- docs-example: skip -->

```yaoxiang
match shape {
    circle(r) => pi * r * r,
    rect(w, h) => w * h,
    point => 0,
}

// struct/tuple pattern
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
// import the whole module, access via namespace
use std.io

// import only specific entries, used as bare names
use std.math.{sqrt, sin, cos}
use std.{list, string}

// inline alias for an entry
use std.io.{print as say}

io.println("hello")
result = sqrt(16.0)       // 4.0 (note: an Int argument returns 0.0; you must pass a Float)
say("aliased")

// All top-level bindings of a module are importable from outside by default; no pub is needed
add: (a: Int, b: Int) -> Int = a + b
Point: Type = { x: Float, y: Float }
```

For module paths, directory entry conventions, and the complete form of `use`, see
[Module System](./modules).

## Ownership

```yaoxiang
// Move: default ownership transfer
p1 = Point(1.0, 2.0)
p2 = p1                   // p1 is moved

// Borrow &: automatically creates a token (no manual & needed)
distance: (a: &Point, b: &Point) -> Float = ...
d = distance(p1, p2)      // the compiler automatically creates a borrow token

// mutable borrow &mut
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

// spawn + ref: share across tasks
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
print(f"Pi: {pi}")                    // Verified: format specifiers are not yet implemented; outputs 3.14159
```
