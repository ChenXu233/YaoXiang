---
title: 'Chapter 5: The Art of Unity'
---

# Chapter 5: The Art of Unity

> **Chapter Goal**: Understand YaoXiang's core philosophy — the `name: type = value` unified syntax model

## 5.1 Different "Things"

In other programming languages, there are many different "things":

| Concept in language | Example |
|---------------------|---------|
| Variable | `let x = 42` |
| Function | `fn add(a, b) { return a + b }` |
| Constant | `const PI = 3.14` |
| Type definition | `struct Point { x: f64, y: f64 }` |
| Interface | `trait Drawable { fn draw(&self); }` |
| Method | `impl Point { fn draw(&self) { ... } }` |

**Problem**: Each "thing" has different syntax, needs learning many rules.

## 5.2 YaoXiang's Answer: Unity

YaoXiang says: **All things can use the same formula!**

```
name: type = value
```

| Concept | YaoXiang写法 | Formula correspondence |
|---------|----------------|------------------------|
| Variable | `x: Int = 42` | `name: type = value` |
| Function | `add: (a: Int, b: Int) -> Int = a + b` | `name: type = value` |
| Constant | `PI: Float = 3.14` | `name: type = value` |
| Type | `Point: Type = { x: Float, y: Float }` | `name: type = value` |
| Interface | `Drawable: Type = { draw: (Surface) -> Void }` | `name: type = value` |

**This is YaoXiang's magic!**

## 5.3 Variable: First Example

```yaoxiang
# Variable: name + type = value
age: Int = 25
name: String = "小明"
is_student: Bool = true
```

## 5.4 Function: Second Example

```yaoxiang
# Function: name + (parameter types) -> return type = implementation
add: (a: Int, b: Int) -> Int = a + b

greet: (name: String) -> String = "你好, ${name}!"

# Multi-line function
max: (a: Int, b: Int) -> Int = {
    if a > b {
        return a
    } else {
        return b
    }
}
```

**Note**:
- `(a: Int, b: Int)` is the **parameter list**
- `-> Int` is the **return type**
- `= a + b` is the **function body** (return value)

## 5.5 Type Definition: Third Example

```yaoxiang
# Type: name + Type = structure
Point: Type = {
    x: Float,
    y: Float
}

# Using the type
p: Point = Point(1.0, 2.0)
```

## 5.6 Interface: Fourth Example

**Interface** is "a type with only methods" — it defines **what can be done**, not **how to do it**:

```yaoxiang
# Interface: name + Type = method collection
Drawable: Type = {
    draw: (surface: Surface) -> Void,
    bounding_box: () -> Rect
}

# Using the interface
circle: Circle = Circle(0.0, 0.0, 5.0)
drawable: Drawable = circle  # ✅ Circle implements Drawable
```

## 5.7 Method: Fifth Example

**Method** is "a function belonging to a certain type":

```yaoxiang
# Regular function
distance: (p1: Point, p2: Point) -> Float = {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# Method写法 (belongs to Point)
Point.distance: (self: Point, other: Point) -> Float = {
    dx = self.x - other.x
    dy = self.y - other.y
    return (dx * dx + dy * dy).sqrt()
}
```

**Calling methods**:

```yaoxiang
p1: Point = Point(0.0, 0.0)
p2: Point = Point(3.0, 4.0)

# Both calling methods work
d1 = distance(p1, p2)        # Functional calling
d2 = p1.distance(p2)          # Method-style calling (syntactic sugar)
```

## 5.8 The Power of Unified Syntax

| Feature | Benefit |
|---------|---------|
| **Learn only one rule** | Don't need to remember multiple syntaxes |
| **Unified code** | All code looks consistent in style |
| **Easy to extend** | New features naturally integrate into existing model |
| **Theoretically elegant** | Symmetrically beautiful in mathematics |

## 5.9 Lambda (Anonymous Function)

In YaoXiang, **named functions are essentially Lambdas**:

```yaoxiang
# Named function (recommended)
add: (a: Int, b: Int) -> Int = a + b

# Lambda form (completely equivalent)
add: (a: Int, b: Int) -> Int = (a, b) => a + b

# Shorthand form
double: (x: Int) -> Int = x * 2
double = (x: Int) => x * 2
```

**Understanding**: Named functions are "Lambdas with names".

## 5.10 Chapter Summary

| Concept | Understanding |
|---------|---------------|
| Unified syntax | `name: type = value` covers all situations |
| Variable | `x: Int = 42` |
| Function | `add: (a: Int, b: Int) -> Int = a + b` |
| Type | `Point: Type = { x: Float, y: Float }` |
| Interface | `Drawable: Type = { draw: (Surface) -> Void }` |
| Method | `Point.draw: (self: Point) -> Void = ...` |

**Core philosophy**: Remember one formula, write all kinds of code!

## 5.11 I Ching Introduction

> "Heaven and earth are not benevolent, they treat all things as straw dogs."
> — Tao Te Ching
>
> In the world of programming languages, rules don't differ based on object.
> Whether it's variables, functions, or types,
> All follow the same law: `name: type = value`.
>
> This is the programming interpretation of "Tao produces one, one produces two, two produces three, three produces all things" —
> One formula, myriad possibilities.
