---
title: Type System
---

# Type System

In the basics tutorial, you learned to use built-in types like `Int`, `String`, and `Bool`. This
chapter dives into YaoXiang's type system, where you'll learn to **define your own types**.

## Unified Syntax Model

YaoXiang's type system is built on the unified syntax defined in RFC-010: **everything is
`name: type = value`**.

| Concept      | Syntax                                         |
| ------------ | ---------------------------------------------- |
| Variable     | `x: Int = 42`                                  |
| Function     | `add: (a: Int, b: Int) -> Int = a + b`         |
| Record type  | `Point: Type = { x: Float, y: Float }`         |
| Interface    | `Drawable: Type = { draw: (Surface) -> Void }` |
| Generic type | `List: (T: Type) -> Type = { ... }`            |

Note: **Type definitions themselves are also `name: Type = value`**.

## Record Types

Record types (called "structs" in other languages) are the most fundamental way to organize data in
YaoXiang:

```yaoxiang
// Define a record type
Point: Type = { x: Float, y: Float }

// Create instances
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

// Access fields
print(p.x)  // 3.0
print(p.y)  // 4.0
```

### Field Default Values

Fields can have default values, which can be optionally provided during construction:

```yaoxiang
User: Type = {
    name: String,
    age: Int = 0,
    active: Bool = true,
}

alice = User(name: "Alice", age: 25)        // active takes default value true
bob = User(name: "Bob")                      // age=0, active=true
anonymous = User(name: "guest", active: false)  // age=0
```

### Method Definitions

Use the `Type.method` syntax to define methods for a type:

```yaoxiang
Point: Type = { x: Float, y: Float }

// Define method: Point.method syntax
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

// Both calling styles are equivalent
print(Point.length(p))  // 5.0 — functional call
print(p.length())       // 5.0 — . call syntax
```

### pub Auto-Binding

Within the same file, functions declared with `pub` are automatically bound to types defined in the
same file:

```yaoxiang
Point: Type = { x: Float, y: Float }

// pub functions auto-bind to Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

p1 = Point(x: 0.0, y: 0.0)
p2 = Point(x: 3.0, y: 4.0)

// Auto-bound methods are called with .
print(p1.distance(p2))  // 5.0
```

## Enum Types

Enums define a set of mutually exclusive variants. Variants without data use lowercase, while
variants with data use functional syntax:

```yaoxiang
// Simple enum
Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }

// Enum with data
Result: (T: Type, E: Type) -> Type = { ok: (T) -> Result(T, E), err: (E) -> Result(T, E) }

// Nested enum
Shape: Type = { circle: (Float) -> Shape, rect: (Float, Float) -> Shape, point: () -> Shape }
```

The core idea of enums: **each variant is itself a type**.

```yaoxiang
area: (s: Shape) -> Float = match s {
    circle(r) => 3.14159 * r * r,
    rect(w, h) => w * h,
    point => 0,
}

print(area(circle(5.0)))    // 78.53975
print(area(rect(3.0, 4.0))) // 12.0
```

## Interfaces

Interfaces are **record types where all fields are function types**. To implement an interface, a
record includes the interface name:

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
    Drawable,       // Implements the Drawable interface
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

Interfaces enable polymorphism — any type that implements `Drawable` can be passed to functions that
accept `Drawable`.

## Generic Types

Generics allow you to write type definitions **without committing to specific types**:

```yaoxiang
// Generic Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

// Usage
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

Generic functions:

```yaoxiang
// Generic map: applies a function to each element of a list
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = {
    mut result: List(R) = []
    for item in list {
        result.append(f(item))
    }
    return result
}

numbers = [1, 2, 3, 4]
doubled = map(Int, Int)(numbers, (x) => x * 2)
print(doubled)  // [2, 4, 6, 8]
```

## Summary

| Concept     | Syntax                                                                      | Purpose                              |
| ----------- | --------------------------------------------------------------------------- | ------------------------------------ |
| Record type | `Point: Type = { x: Float, y: Float }`                                      | Organize related data                |
| Enum        | `Color: Type = { red: () -> Color, green: () -> Color, blue: () -> Color }` | One of many choices                  |
| Interface   | `Drawable: Type = { draw: ... }`                                            | Polymorphic abstraction              |
| Generic     | `List: (T: Type) -> Type = { ... }`                                         | Type parameterization                |
| Never       | `Never` is a system-built-in bottom type                                    | Diverging/never-returning code paths |
| Method      | `Type.method: (self: Type, ...) -> ...`                                     | Attach behavior                      |
