---
title: 'Chapter 6: Custom Types'
---

# Chapter 6: Custom Types

> **Chapter Goal**: Learn to create your own types, understand the usage of record types, interfaces, and methods

## 6.1 Why Custom Types?

Basic types (Int, Float, String) are like basic LEGO blocks. But we need more complex "shapes" to describe the real world:

| Real-world thing | Corresponding custom type |
|------------------|-------------------------|
| A point on screen | `Point` |
| A book | `Book` |
| A shape | `Shape` |
| A user | `User` |

## 6.2 Record Type

**Record type** is bundling multiple data together:

```yaoxiang
# Define a Point type (record type)
Point: Type = {
    x: Float,    # x coordinate
    y: Float     # y coordinate
}

# Create a Point value
p: Point = Point(1.0, 2.0)

# Access fields
print(p.x)      # 1.0
print(p.y)      # 2.0
```

**Structure diagram**:

```
Point type
┌─────────────────────┐
│        Point         │
├─────────────────────┤
│  ┌─────┬─────┐     │
│  │  x  │  y  │     │
│  │Float│Float│     │
│  └─────┴─────┘     │
└─────────────────────┘
```

## 6.3 More Complex Record Types

```yaoxiang
# A person
Person: Type = {
    name: String,      # Name
    age: Int,          # Age
    is_student: Bool   # Whether student
}

# Create and use
person: Person = Person("小明", 18, true)
print(person.name)        # 小明
print(person.age)         # 18

# Modify mutable fields
mut p: Person = Person("小红", 20, false)
p.age = 21               # ✅ Can modify
```

## 6.4 Interface

**Interface** is "a type with only methods" — it defines **what can be done**, but not **how to do it**.

### 6.4.1 Interface Definition

```yaoxiang
# Define a drawable interface
Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}
```

### 6.4.2 Built-in Interface Implementation

Interfaces can **have built-in implementation**, directly write methods inside:

```yaoxiang
# Interface with built-in implementation (complete definition)
Drawable: Type = {
    # Define method + implementation
    draw: (self: Self, surface: Surface) -> Void = {
        print("Draw shape")
    },
    bounding_box: (self: Self) -> Rect = {
        return Rect(0, 0, 100, 100)
    }
}

# Use
surface: Surface = Surface()
d: Drawable = Drawable()
d.draw(surface)
```

### 6.4.3 Implementing Interfaces

```yaoxiang
# Circle implements Drawable interface
Circle: Type = {
    x: Float,
    y: Float,
    radius: Float,
    Drawable    # Implement Drawable interface
}

# Rectangle also implements Drawable interface
Rectangle: Type = {
    x: Float,
    y: Float,
    width: Float,
    height: Float,
    Drawable    # Implement Drawable interface
}
```

**Uses of interfaces**:

```yaoxiang
# No matter what shape, as long as it implements Drawable, can be handled uniformly
draw_all: (drawables: List[Drawable]) -> Void = {
    for d in drawables {
        d.draw(screen)
    }
}

# List[Drawable] can contain Circle, Rectangle, any implementation
```

## 6.5 Methods

### 6.5.1 Define Methods

```yaoxiang
# Add methods to Point type
Point.move: (self: Point, dx: Float, dy: Float) -> Point = {
    return Point(self.x + dx, self.y + dy)
}

Point.distance: (self: Point, other: Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    return (dx * dx + dy * dy).sqrt()
}
```

### 6.5.2 Call Methods

```yaoxiang
p1: Point = Point(0.0, 0.0)
p2: Point = Point(3.0, 4.0)

# Method call syntax (syntactic sugar)
moved = p1.move(1.0, 2.0)        # Point(1.0, 2.0)
dist = p1.distance(p2)            # 5.0

# Real function call (the truth)
dist = Point.distance(p1, p2)    # Equivalent!
```

### 6.5.3 self Parameter

In methods, `self` is just **the name of the first parameter**, doesn't have special meaning:

```yaoxiang
# Usage of self (self is just a parameter name)
Point.translate: (self: Point, dx: Float, dy: Float) -> Point = {
    # self is the value of the first parameter
    return Point(self.x + dx, self.y + dy)
}

# Use
p: Point = Point(10.0, 20.0)
p2 = p.translate(5.0, 5.0)   # self = p
# p2 = Point(15.0, 25.0)

# self can be replaced with any name
Point.translate: (this: Point, dx: Float, dy: Float) -> Point = {
    return Point(this.x + dx, this.y + dy)
}
```

## 6.6 Essence of Methods: Functional vs OOP

**Important**: YaoXiang is a **functional language**, not an OOP language!

### 6.6.1 We Are Not OOP

| Feature | OOP languages (like Java, C++) | YaoXiang |
|---------|--------------------------------|----------|
| Mindset | Objects are core, methods belong to objects | Functions are core, methods are syntactic sugar |
| Inheritance | Has inheritance relationships | No inheritance, only interface implementation |
| self | self is implicit, must have | self is not required, just a parameter name |
| Method call | Different syntax | Still function call essentially |

**OOP's methods**:
```java
// Java: methods belong to objects
point.move(1.0, 2.0)
```

**YaoXiang's "methods" (essentially functions)**:
```yaoxiang
# Still function call, just looks like methods
point.move(1.0, 2.0)
# Equivalent to
Point.move(point, 1.0, 2.0)
```

### 6.6.2 Why Does It Look Like OOP?

YaoXiang achieves OOP-like syntax sugar through **unified syntax** + **currying binding**:

```yaoxiang
# 1. Unified syntax: everything is name: type = value
# 2. Currying: functions can "bind" the first parameter

# Original function (the real thing)
calculate_distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# Bind: bind first parameter to Point type
Point.distance = calculate_distance[0]

# Call: looks like OOP
p1: Point = Point(0.0, 0.0)
p2: Point = Point(3.0, 4.0)
d = p1.distance(p2)        # Syntactic sugar
# Equivalent to
d = calculate_distance(p1, p2)  # Truth!
```

### 6.6.3 self is Not Required

In YaoXiang, `self` is just a **regular parameter name**, not a keyword!

```yaoxiang
# self can be replaced with any name
Point.move: (self: Point, dx: Float, dy: Float) -> Point = {
    return Point(self.x + dx, self.y + dy)
}

# Completely equivalent
Point.move: (p: Point, dx: Float, dy: Float) -> Point = {
    return Point(p.x + dx, p.y + dy)
}

# Use
p: Point = Point(1.0, 2.0)
p2 = p.move(3.0, 4.0)   # self = p
```

### 6.6.4 Auto-binding Rules

Compiler's auto-binding rules:

```
Binding point = First parameter's type = Type name

For example:
- Point.move: (p: Point, ...) → p is the first parameter, type is Point
- So p is bound to Point
- Calling p.move(other) is equivalent to Point.move(p, other)
```

```yaoxiang
# Understanding auto-binding
# 1. Define function, first parameter is Point
add_points: (p1: Point, p2: Point) -> Point = {
    return Point(p1.x + p2.x, p1.y + p2.y)
}

# 2. Compiler automatically binds to Point.add
Point.add = add_points[0]

# 3. Use
p1: Point = Point(1.0, 2.0)
p2: Point = Point(3.0, 4.0)
p3 = p1.add(p2)          # ✅ Syntactic sugar
# Equivalent to
p3 = add_points(p1, p2)  # ✅ Truth
```

## 6.7 Manual Binding vs Auto-binding

### 6.7.1 Manual Binding

```yaoxiang
# Original function
calculate_distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# Manual binding: Point.distance = calculate_distance[0]
# [0] means bind the first parameter
Point.distance = calculate_distance[0]

# Use
p1: Point = Point(0.0, 0.0)
p2: Point = Point(3.0, 4.0)
d = p1.distance(p2)        # ✅ Syntactic sugar
```

### 6.7.2 Auto-binding

When functions are defined in **module files**, and the first parameter is the module type, the compiler **automatically binds**:

```yaoxiang
# ===== File: Point.yx =====
# This file defines Point type
type Point = { x: Float, y: Float }

# Define function, first parameter is Point
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# ===== main.yx =====
use Point

p1: Point = Point(0.0, 0.0)
p2: Point = Point(3.0, 4.0)

# Compiler automatically binds to Point.distance
d = p1.distance(p2)        # ✅ Auto-binding!
```

## 6.8 Comprehensive Example

```yaoxiang
# Define Shape interface
Shape: Type = {
    area: () -> Float,
    perimeter: () -> Float
}

# Circle implements Shape
Circle: Type = {
    radius: Float,
    Shape    # Implement Shape interface
}

Circle.area: (self: Circle) -> Float = {
    return 3.14159 * self.radius * self.radius
}

Circle.perimeter: (self: Circle) -> Float = {
    return 2 * 3.14159 * self.radius
}

# Rectangle implements Shape
Rectangle: Type = {
    width: Float,
    height: Float,
    Shape    # Implement Shape interface
}

Rectangle.area: (self: Rectangle) -> Float = {
    return self.width * self.height
}

Rectangle.perimeter: (self: Rectangle) -> Float = {
    return 2 * (self.width + self.height)
}

# Use
circle: Circle = Circle(5.0)
rect: Rectangle = Rectangle(4.0, 6.0)

print(circle.area())       # 78.53975
print(rect.area())         # 24.0

# Calculate total
total: Float = circle.area() + rect.area()  # 102.53975
```

## 6.9 Chapter Summary

| Concept | Description | Example |
|---------|-------------|---------|
| Record type | Bundle multiple data | `Point: Type = { x: Float, y: Float }` |
| Interface | Define method collection | `Drawable: Type = { draw: (Surface) -> Void }` |
| Built-in interface | Define methods directly in interface | `Drawable: Type = { draw: (Self) -> Void = {...} }` |
| Method binding | Bind function to type | `Point.move = func[0]` |
| Syntactic sugar | `p.move()` equals `Point.move(p)` | `p.distance(other)` |
| Functional essence | Methods are syntactic sugar, essentially functions | `Point.distance(p1, p2)` |
| self | Just parameter name, not keyword | Can be replaced with any name |

**Key understanding**:
- ✅ YaoXiang is a functional language
- ✅ Methods are syntactic sugar, essentially functions
- ✅ self is not required, just naming convention
- ✅ Unified syntax makes everything expressible

## 6.10 I Ching Introduction

> "What is above form is called Tao; what is below form is called utensil."
> — "Xici Zhuan", Book of Changes
>
> Utensil is concrete form — record types are like containers, holding data.
> Tao is abstract rule — interfaces are like contracts, defining behavior.
>
> Custom types are the programmer's process of "making utensils":
> - Record types are "utensils", holding data
> - Interfaces are "Tao", defining specifications
> - Methods are "use", making types useful
>
> **Guide utensils with Tao, then accomplish things.**
