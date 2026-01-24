# Type System

> Version: v2.0.0
> Status: Updated (Based on RFC-010 Unified Type Syntax)

---

## Type Classification

YaoXiang's types are classified as follows:

```
TypeExpr
├── Primitive
├── Record
├── Enum
├── Interface
├── Tuple
├── Fn
├── Generic
└── Union/Intersection
```

---

## Unified Syntax: name: type = value

YaoXiang uses a **minimal unified type syntax**: `name: type = value`

```
├── Variable/Function: name: type = value
│   ├── x: Int = 42
│   └── add: (Int, Int) -> Int = (a, b) => { return a + b }
│
├── Type Definition: type Name = type_expression
│   ├── type Point = { x: Float, y: Float }
│   └── type Drawable = { draw: (Surface) -> Void }
│
└── Interface: Record type with all fields being function types
    └── type Serializable = { serialize: () -> String }
```

---

## Primitive Types

| Type | Description | Default Size | Example |
|------|-------------|--------------|---------|
| `Void` | Empty value | 0 bytes | `null` |
| `Bool` | Boolean | 1 byte | `true`, `false` |
| `Int` | Signed integer | 8 bytes | `42`, `-10` |
| `Uint` | Unsigned integer | 8 bytes | `100u` |
| `Float` | Floating point | 8 bytes | `3.14`, `-0.5` |
| `String` | UTF-8 string | Variable | `"hello"` |
| `Char` | Unicode character | 4 bytes | `'a'`, `'中'` |
| `Bytes` | Raw bytes | Variable | `b"\x00\x01"` |

### Width-Specified Integers

```yaoxiang
x: Int8 = 127
y: Int16 = 32000
z: Int32 = 100000
w: Int64 = 10000000000
u: Uint8 = 255
```

---

## Record Types

Define record types using curly braces `{}`:

```yaoxiang
# Definition
type Point = { x: Float, y: Float }

# Construct values
p1: Point = Point(3.0, 4.0)
p2: Point = Point(x: 1.0, y: 2.0)

# Access fields
x_coord: Float = p1.x              # 3.0
y_coord: Float = p1.y              # 4.0
```

### Nested Record Types

```yaoxiang
type Rectangle = { width: Float, height: Float }
type Circle = { radius: Float }

# Usage
rect: Rectangle = Rectangle(10.0, 20.0)
circle: Circle = Circle(5.0)
```

---

## Enum Types

Define enum variants using `|`:

```yaoxiang
# Simple enum
type Color = { red | green | blue }

# Variants with values
type Result[T, E] = { ok(T) | err(E) }
type Option[T] = { some(T) | none }

# Usage
success: Result[Int, String] = ok(42)
failure: Result[Int, String] = err("not found")
value: Option[Int] = some(100)
empty: Option[Int] = none
```

### Pattern Matching

```yaoxiang
type Status = { pending | processing | completed | failed(String) }

process: Status -> String = (status) => {
    match status {
        pending => "pending",
        processing => "processing",
        completed => "completed",
        failed(msg) => "failed: " + msg,
    }
}
```

---

## Interface Types

**Interface = Record type with all fields being function types**

```yaoxiang
# Interface definition
type Drawable = {
    draw: (Surface) -> Void,
    bounding_box: () -> Rect
}

type Serializable = {
    serialize: () -> String
}
```

### Types Implementing Interfaces

List interface names in the type definition:

```yaoxiang
type Point = {
    x: Float,
    y: Float,
    Drawable,      # Implement Drawable interface
    Serializable   # Implement Serializable interface
}

# Implement interface methods
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    return surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    return "Point(${self.x}, ${self.y})"
}
```

### Empty Interface

```yaoxiang
type EmptyInterface = {}
```

---

## Methods and Binding

### Type Method Definition

Use `Type.method: (Type, ...) -> ReturnType = ...` syntax:

```yaoxiang
# Type method: first parameter is self (caller)
Point.draw: (Point, Surface) -> Void = (self, surface) => {
    return surface.plot(self.x, self.y)
}

Point.serialize: (Point) -> String = (self) => {
    return "Point(${self.x}, ${self.y})"
}
```

### Position Binding [n]

Bind functions to specific parameter positions of a type:

```yaoxiang
# Define standalone function
distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# Bind to Point.distance (this bound to position 0)
Point.distance: (Point, Point) -> Float = distance[0]

# Call: functional style
d1: Float = distance(p1, p2)

# Call: OOP syntax sugar
d2: Float = p1.distance(p2)
```

### Multi-Position Binding

```yaoxiang
# Function receives multiple Point parameters
transform_points: (Point, Point, Float) -> Point = (p1, p2, factor) => {
    return Point(p1.x * factor, p1.y * factor)
}

# Bind multiple positions (automatic currying)
Point.transform: (Point, Point, Float) -> Point = transform_points[0, 1]

# Call
p1.transform(p2)(2.0)  # → transform_points(p1, p2, 2.0)
```

### Auto-Binding pub

Functions declared with `pub` are **automatically bound** to types defined in the same file:

```yaoxiang
type Point = { x: Float, y: Float }

# Using pub declaration, compiler auto-binds to Point
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    return (dx * dx + dy * dy).sqrt()
}

# Compiler infers: Point.distance = distance[0]

# Now you can call:
d1: Float = distance(p1, p2)      # Functional style
d2: Float = p1.distance(p2)       # OOP syntax sugar
```

---

## Tuple Types

```yaoxiang
# Definition
Point2D = (Float, Float)
Triple = (Int, String, Bool)

# Usage
p: (Float, Float) = (3.0, 4.0)
(x, y) = p
```

---

## Function Types

```yaoxiang
# Define function types
type Adder = (Int, Int) -> Int
type Callback[T] = (T) -> Void
type Predicate[T] = (T) -> Bool

# Usage
add: (Int, Int) -> Int = (a, b) => { return a + b }
```

---

## Generic Types

```yaoxiang
# Define generics
type List[T] = {
    data: Array[T],
    length: Int
}
type Map[K, V] = { keys: [K], values: [V] }
type Result[T, E] = { ok(T) | err(E) }

# Usage
numbers: List[Int] = List([1, 2, 3])
names: List[String] = List(["Alice", "Bob"])
```

### Generic Functions

```yaoxiang
# Generic function
identity: [T](T) -> T = (x) => { return x }

# Usage
n: Int = identity(42)
s: String = identity("hello")
```

---

## Type Union

```yaoxiang
# Using |
type Number = Int | Float
type Text = String | Char

# Usage
n1: Number = 42
n2: Number = 3.14

# Pattern matching
print_number: Number -> String = (n) => {
    return match n {
        i: Int => "Integer: " + i.to_string(),
        f: Float => "Float: " + f.to_string(),
    }
}
```

---

## Type Intersection

```yaoxiang
# Intersection type = type composition
type Printable = { print: () -> Void }
type Serializable = { to_json: () -> String }

# Intersection type
type Document = Printable & Serializable

# Implement intersection type
type MyDoc = { content: String } & {
    print: () -> Void = () => { return print(self.content) }
    to_json: () -> String = () => { return '{"content": "' + self.content + '"}' }
}
```

---

## Type Casting

```yaoxiang
# Using as
int_to_float: Float = 42 as Float
float_to_int: Int = 3.14 as Int
string_to_int: Int = "123" as Int
```

---

## Type Inference

YaoXiang supports type inference, allowing you to omit type annotations:

```yaoxiang
x = 42              # Inferred as Int
y = 3.14            # Inferred as Float
z = "hello"         # Inferred as String
p = Point(1.0, 2.0) # Inferred as Point
```

---

## Next Steps

- [Functions and Closures](functions.md) - Learn function definitions and higher-order functions
- [Control Flow](control-flow.md) - Conditionals and loops
- [Error Handling](error-handling.md) - Result and Option
