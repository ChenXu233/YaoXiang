---
title: Type System
---

# Type System

In the basic tutorial you learned to use built-in types like `Int`, `String`, `Bool`. This chapter goes deep into YaoXiang's type system, learning to **define your own types**.

## Unified Syntax Model

YaoXiang's type system is built on the unified syntax defined by RFC-010: **everything is `name: type = value`**.

| Concept | Syntax |
|------|------|
| Variable | `x: Int = 42` |
| Function | `add: (a: Int, b: Int) -> Int = a + b` |
| Record type | `Point: Type = { x: Float, y: Float }` |
| Interface | `Drawable: Type = { draw: (Surface) -> Void }` |
| Generic type | `List: (T: Type) -> Type = { ... }` |

Note: **type definitions themselves are also `name: Type = value`**.

## Record Types

Record types (called "structs" in other languages) are the most basic way of organizing data in YaoXiang:

```yaoxiang
# Define a record type
Point: Type = { x: Float, y: Float }

# Create an instance
origin = Point(x: 0.0, y: 0.0)
p = Point(x: 3.0, y: 4.0)

# Access a field
println(p.x)  # 3.0
println(p.y)  # 4.0
```

### Field Default Values

Fields can have default values; they are optional when constructing:

```yaoxiang
User: Type = {
    name: String,
    age: Int = 0,
    active: Bool = true,
}

alice = User(name: "Alice", age: 25)        # active takes the default value true
bob = User(name: "Bob")                      # age=0, active=true
anonymous = User(name: "guest", active: false)  # age=0
```

### Method Definition

Use the `Type.method` syntax to define methods for a type:

```yaoxiang
Point: Type = { x: Float, y: Float }

# Define a method: the Point.method syntax
Point.length: (self: Point) -> Float = {
    return (self.x * self.x + self.y * self.y).sqrt()
}

p = Point(x: 3.0, y: 4.0)

# The two calling styles are equivalent
println(Point.length(p))  # 5.0 — functional call
println(p.length())       # 5.0 — . call syntax
```

### `pub` Auto-binding

Within the same file, functions declared with `pub` are automatically bound to types defined in the same file:

```yaoxiang
Point: Type = { x: Float, y: Float }

# The pub function is auto-bound to Point
pub distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

p1 = Point(x: 0.0, y: 0.0)
p2 = Point(x: 3.0, y: 4.0)

# Auto-bound methods use the . call
println(p1.distance(p2))  # 5.0
```

## Enum Types

Enums define a set of mutually exclusive variants. Variants without data use lowercase, while variants with data use function-call syntax:

```yaoxiang
# Simple enum
type Color = red | green | blue

# Enum with data
type Result(T, E) = ok(T) | err(E)

# Nested enum
type Shape = circle(Float) | rect(Float, Float) | point
```

The core idea of enums: **each variant is itself a type**.

```yaoxiang
area: (s: Shape) -> Float = match s {
    circle(r) => 3.14159 * r * r,
    rect(w, h) => w * h,
    point => 0,
}

println(area(circle(5.0)))    # 78.53975
println(area(rect(3.0, 4.0))) # 12.0
```

## Interfaces

An interface is **a record type whose fields are all function types**. Implementing an interface means including the interface name in a record:

```yaoxiang
# Define an interface
Drawable: Type = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect,
}

# Implement the interface: include the interface name in the record type
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable,       # Implement the Drawable interface
}

# Provide the methods required by the interface
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

Interfaces achieve polymorphism — any type that implements `Drawable` can be passed to a function that accepts `Drawable`.

## Generic Types

Generics let you write type definitions that are **not tied to specific types**:

```yaoxiang
# Generic Pair
Pair: (T: Type, U: Type) -> Type = { first: T, second: U }

# Usage
string_pair = Pair(Int, String)(first: 1, second: "hello")
float_pair = Pair(Float, Float)(first: 3.14, second: 2.71)
```

Generic functions:

```yaoxiang
# Generic map: apply a function to every element of a list
map: (T: Type, R: Type) -> ((list: List(T), f: (T) -> R) -> List(R)) = {
    mut result: List(R) = []
    for item in list {
        result.append(f(item))
    }
    return result
}

numbers = [1, 2, 3, 4]
doubled = map(Int, Int)(numbers, (x) => x * 2)
println(doubled)  # [2, 4, 6, 8]
```

## Summary

| Concept | Syntax | Purpose |
|------|------|------|
| Record type | `Point: Type = { x: Float, y: Float }` | Organize related data |
| Enum | `type Color = red \| green \| blue` | Choose one of many |
| Interface | `Drawable: Type = { draw: ... }` | Polymorphic abstraction |
| Generic | `List: (T: Type) -> Type = { ... }` | Type parameterization |
| Method | `Type.method: (self: Type, ...) -> ...` | Attach behavior |