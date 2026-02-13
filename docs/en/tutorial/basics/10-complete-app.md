---
title: Chapter 10: Complete Application
---

# Chapter 10: Complete Application

> **Chapter Goal**: Comprehensively apply knowledge from Chapters 1-9 to complete a small project

## 10.1 Project Goal

We will create a **simple geometry calculator** that can:

- Define points (Point) and circles (Circle)
- Calculate distances
- Determine if points are inside/outside circles
- Batch process multiple shapes

## 10.2 Step 1: Define Basic Types

```yaoxiang
# ===== File: geometry.yx =====

# Point type (record type)
Point: Type = {
    x: Float,
    y: Float,

    # Method: move
    move: (self: Point, dx: Float, dy: Float) -> Point = {
        return Point(self.x + dx, self.y + dy)
    },

    # Method: distance
    distance: (self: Point, other: Point) -> Float = {
        dx = self.x - other.x
        dy = self.y - other.y
        return (dx * dx + dy * dy).sqrt()
    }
}
```

## 10.3 Step 2: Define Circle Type

```yaoxiang
# ===== File: geometry.yx (continued) =====

# Circle type
Circle: Type = {
    center: Point,
    radius: Float,

    # Method: area
    area: (self: Circle) -> Float = {
        return 3.14159 * self.radius * self.radius
    },

    # Method: check if point is inside circle
    contains: (self: Circle, p: Point) -> Bool = {
        distance = self.center.distance(p)
        return distance <= self.radius
    },

    # Method: resize
    scale: (self: Circle, factor: Float) -> Circle = {
        return Circle(self.center, self.radius * factor)
    }
}
```

## 10.4 Step 3: Create Shape Manager

```yaoxiang
# ===== File: geometry.yx (continued) =====

# Drawable interface
Drawable: Type = {
    draw: (self: Self) -> Void,
    bounding_box: (self: Self) -> Rect
}

# Rect type (implements Drawable)
Rect: Type = {
    x: Float,
    y: Float,
    width: Float,
    height: Float,
    Drawable
}

Rect.draw: (self: Rect) -> Void = {
    print("Draw rectangle: ${self.x}, ${self.y}, ${self.width}, ${self.height}")
}

Rect.bounding_box: (self: Rect) -> Rect = {
    return self
}
```

## 10.5 Step 4: Generic Shape Collection

```yaoxiang
# ===== File: geometry.yx (continued) =====

# Shape collection (using generics)
ShapeContainer: Type[T: Drawable] = {
    shapes: List[T],

    add: [T: Drawable](self: ShapeContainer[T], shape: T) -> Void = {
        self.shapes.push(shape)
    },

    draw_all: [T: Drawable](self: ShapeContainer[T]) -> Void = {
        for shape in self.shapes {
            shape.draw()
        }
    },

    count: [T: Drawable](self: ShapeContainer[T]) -> Int = {
        return self.shapes.length
    }
}
```

## 10.6 Step 5: Main Program

```yaoxiang
# ===== File: main.yx =====

use geometry.{Point, Circle, Rect, Drawable, ShapeContainer}

main: () -> Void = {
    # Create points
    origin: Point = Point(0.0, 0.0)
    p1: Point = Point(3.0, 4.0)

    # Calculate distance
    dist = origin.distance(p1)    # 5.0
    print("Distance: ${dist}")

    # Create circle
    circle: Circle = Circle(origin, 5.0)
    print("Circle area: ${circle.area()}")

    # Check if points are inside/outside circle
    inside = Point(1.0, 1.0)
    outside = Point(10.0, 10.0)

    print("Point inside circle: ${circle.contains(inside)}")    # true
    print("Point outside circle: ${circle.contains(outside)}")   # false

    # Ownership example
    mut c1: Circle = Circle(Point(0.0, 0.0), 10.0)

    # Chained calls (ownership backflow)
    c1 = c1.scale(2.0).move(5.0, 5.0)

    # Share example
    shared_circle = ref c1
    spawn(() => {
        print("Access circle in new task: ${shared_circle.area()}")
    })

    # Generic container example
    mut container: ShapeContainer[Circle] = ShapeContainer(List())
    container.add(circle)
    container.add(Circle(Point(1.0, 1.0), 3.0))

    print("Number of shapes in container: ${container.count()}")
    container.draw_all()

    print("Program completed!")
}
```

---

## 10.7 Complete Code Structure

```
Project Structure
┌─────────────────────────────────────────┐
│  geometry.yx                            │
│  ├── Point type definition               │
│  ├── Circle type definition              │
│  ├── Rect type definition                │
│  ├── Drawable interface                  │
│  └── ShapeContainer generic container    │
├─────────────────────────────────────────┤
│  main.yx                                │
│  ├── use geometry.{...}                  │
│  └── main function                      │
└─────────────────────────────────────────┘
```

## 10.8 Run the Program

```bash
# Save files
Save geometry.yx and main.yx

# Run
yaoxiang main.yx
```

**Expected Output**:

```
Distance: 5.0
Circle area: 78.53975
Point inside circle: true
Point outside circle: false
Access circle in new task: 314.159
Number of shapes in container: 2
Draw rectangle: 0.0, 0.0, 10.0, 10.0
Draw rectangle: 1.0, 1.0, 3.0, 3.0
Program completed!
```

## 10.9 Chapter Knowledge Review

| Chapter | Knowledge Point | Application |
|---------|-----------------|-------------|
| Chapter 1 | Program entry main | `main: () -> Void = { ... }` |
| Chapter 2 | Basic types | `Float`, `Int`, `Bool` |
| Chapter 3 | Variables and scope | `mut`, `let` |
| Chapter 4 | Type type | `Point: Type = { ... }` |
| Chapter 5 | Unified syntax | `name: type = value` |
| Chapter 6 | Custom types/methods | `Point.move: ...` |
| Chapter 7 | Generics | `ShapeContainer[T]` |
| Chapter 8 | Ownership | `mut c1 = ...` |
| Chapter 9 | Move/ref/clone | `shared = ref c1`, `c1.scale(...)` |

## 10.10 Congratulations!

Completing these 10 chapters, you have:

✅ Understood basic concepts of programs
✅ Mastered type systems
✅ Understood the secret of meta-types and Type
✅ Learned unified syntax `name: type = value`
✅ Can define custom types and interfaces
✅ Understood generic programming
✅ Mastered ownership model
✅ Can comprehensively apply this knowledge

**Next Steps**: Can continue learning advanced topics like concurrent model, error handling, etc.

## 10.11 I Ching Introduction

> "Great is the Qian element, all things depend on it for their beginning, thus it rules over heaven."
> — "Qian Hexagram", Book of Changes
>
> From the first line of Hello World to a complete geometry calculator,
> This is the process of "all things depend on it for their beginning".
>
> You have:
> - Learned "Tao" (programming thought)
> - Mastered "utensil" (language syntax)
> - Can "make utensils" (write programs)
>
> May you continue to explore and move forward on the path of programming.
>
> **Qian Hexagram says: Heaven moves vigorously, the gentleman strives for unremitting self-improvement.**
