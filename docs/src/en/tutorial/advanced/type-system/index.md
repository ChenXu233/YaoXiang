---
title: 'Type System'
---

# Type System

In the basic tutorial you learned to use builtin types like `Int`, `String`, and `Bool`. This
chapter dives deep into YaoXiang's type system, teaching you to **define your own types**.

## Unified Syntax Model

YaoXiang's type system is built on the unified syntax defined by RFC-010: **everything is
`name: type = value`**.

| Concept      | Syntax                                         |
| ------------ | ---------------------------------------------- |
| Variable     | `x: Int = 42`                                  |
| Function     | `add: (a: Int, b: Int) -> Int = a + b`         |
| Record type  | `Point: Type = { x: Float, y: Float }`         |
| Interface    | `Drawable: Type = { draw: (Surface) -> Void }` |
| Generic type | `List: (T: Type) -> Type = { ... }`            |

Note: **the type definition itself is also `name: Type = value`**.

## Record Type

Record types (called "structs" in other languages) are the most basic way to organize data in
YaoXiang:

```yaoxiang
// Define a record type
Point: Type = { x: Float, y: Float }

// Create an instance
origin = Point(x= 0.0, y= 0.0)
p = Point(x= 3.0, y= 4.0)

// Access fields
print(p.x)  // 3.0
print(p.y)  // 4.0
```

### Field Default Values

Fields can be given default values and are optional at construction:

```yaoxiang
User: Type = {
    name: String,
    age: Int = 0,
    active: Bool = true,
}

alice = User(name= "Alice", age= 25)        // active takes default value true
bob = User(name= "Bob")                      // age=0, active=true
anonymous = User(name= "guest", active= false)  // age=0
```

### Method Definition

Use the `Type.method` syntax to define methods on a type:

```yaoxiang
use std.math

Point: Type = { x: Float, y: Float }

// Define a method: the Point.method syntax
// Note: self must be written explicitly in the signature; when written as `() -> Float`, self in the body reports E1001
Point.length: (self: Point) -> Float = {
    d = self.x * self.x + self.y * self.y
    return math.sqrt(d)
}

main: () -> Void = {
    p = Point(x= 3.0, y= 4.0)

    // Only the . call syntax can be used—writing Point.length(p) reports E1010
    print(p.length())  // 5.0
}
```

### pub Auto-Binding

Within the same file, functions declared `pub` are automatically bound to the types defined in the
same file:

<!-- docs-example: skip -->

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub functions are automatically bound to Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    d = dx * dx + dy * dy
    return math.sqrt(d)
}

p1 = Point(x= 0.0, y= 0.0)
p2 = Point(x= 3.0, y= 4.0)

// The auto-bound method is called with .
print(p1.distance(p2))  // 5.0
```

## Enum Type

An enum defines a set of mutually exclusive variants. Variants without data use lowercase, variants
with data use function-like syntax:

<!-- docs-example: skip -->

```yaoxiang
// Simple enum
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// Enum with data
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Nested enum
Shape: Type = { circle: (Float) -> Shape, rect: (Float, Float) -> Shape, point: () -> Shape }
```

The core idea of enums: **each variant is itself a type**.

<!-- docs-example: skip -->

```yaoxiang
area: (s: Shape) -> Float = match s {
    circle(r) => 3.14159 * r * r,
    rect(w, h) => w * h,
    point => 0,
}

print(area(circle(5.0)))    // 78.53975
print(area(rect(3.0, 4.0))) // 12.0
```

## Interface

An interface is **a record type whose fields are all function types**. To implement an interface,
include the interface name in the record:

<!-- docs-example: skip -->

```yaoxiang
// Define an interface
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

// Implement the interface: include the interface name in the record type
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable,       // Implement the Drawable interface
}

// Provide the methods required by the interface
Circle.draw: (self: Circle, surface: Surface) -> Void = {
    surface.draw_circle(self.x, self.y, self.radius)
}

Circle.bounding_box: (self: Circle) -> Rect = {
    return Rect(
        x: self.x - self.radius,
        y: self.y - self.radius,
        width: self.radius * 2.0,
        height: self.radius * 2.0,
    )
}
```

Interfaces enable polymorphism—any type that implements `Drawable` can be passed to a function that
accepts `Drawable`.

## Generic Type

Generics let you write **type definitions that are not tied to specific types**:

```yaoxiang
// Generic Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// Usage
string_pair = Pair(Int, String)(first= 1, second= "hello")
float_pair = Pair(Float, Float)(first= 3.14, second= 2.71)
```

Generic function:

```yaoxiang
use std.list

// Apply a function to every element of a list
// Note: parameterized functions over type parameters in 0.8.2 cannot yet be instantiated—writing
// `(T: Type, R: Type) -> (function type)` and then `map2(Int, Int)(numbers, f)` reports E1002,
// so an equivalent monomorphic signature is used here to demonstrate the same thing.
map: (numbers: Vec(Int), f: (Int) -> Int) -> Vec(Int) = {
    mut result = []
    for item in numbers {
        result = list.push(result, f(item))
    }
    return result
}

main: () -> Void = {
    numbers = [1, 2, 3, 4]
    doubled = map(numbers, (x) => x * 2)
    print(doubled)  // [2, 4, 6, 8]
}
```

## Summary

| Concept     | Syntax                                                                      | Purpose                           |
| ----------- | --------------------------------------------------------------------------- | --------------------------------- |
| Record type | `Point: Type = { x: Float, y: Float }`                                      | Organize related data             |
| Enum        | `Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }` | Choose one of several             |
| Interface   | `Drawable: Type = { draw: ... }`                                            | Polymorphic abstraction           |
| Generic     | `List: (T: Type) -> Type = { ... }`                                         | Type parameterization             |
| Never       | `Never` is a builtin bottom type                                            | Diverging / never-returning paths |
| Method      | `Type.method: (self: Type, ...) -> ...`                                     | Attaching behavior                |
