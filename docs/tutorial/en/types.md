# Type System

> Version: v1.0.0
> Status: In Progress

---

## Type Classification

YaoXiang's types are divided into the following categories:

```
TypeExpr
├── Primitive
├── Struct
├── Enum
├── Tuple
├── Fn
├── Generic
└── Union/Intersection
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

### Integer with Bit Width

```yaoxiang
x: Int8 = 127
y: Int16 = 32000
z: Int32 = 100000
w: Int64 = 10000000000
u: Uint8 = 255
```

---

## Struct Types

Define structs using constructor syntax:

```yaoxiang
# Definition
type Point = { x: Float, y: Float }

# Construct values
p1 = Point(3.0, 4.0)
p2 = Point(x: 1.0, y: 2.0)

# Access members
x_coord = p1.x              # 3.0
y_coord = p1.y              # 4.0
```

### Nested Structs

```yaoxiang
type Rectangle = { width: Float, height: Float }
type Circle = { radius: Float }
type Shape = { Rectangle | Circle }

# Usage
rect = Rectangle(10.0, 20.0)
circle = Circle(5.0)
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
success = ok(42)
failure = err("not found")
value = some(100)
empty = none
```

### Pattern Matching

```yaoxiang
type Status = pending | processing | completed | failed(String)

process: Status -> String = (status) => {
    match status {
        pending => "Pending",
        processing => "Processing",
        completed => "Completed",
        failed(msg) => "Failed: " + msg,
    }
}
```

---

## Tuple Types

```yaoxiang
# Definition
Point2D = (Float, Float)
Triple = (Int, String, Bool)

# Usage
p = (3.0, 4.0)
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
add: (Int, Int) -> Int = (a, b) => a + b
```

---

## Generic Types

```yaoxiang
# Define generics
type List[T] = { elements: [T], length: Int }
type Map[K, V] = { keys: [K], values: [V] }
type Result[T, E] = ok(T) | err(E)

# Usage
numbers: List[Int] = [1, 2, 3]
names: List[String] = ["Alice", "Bob"]
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
    match n {
        i: Int => "Integer: " + i.to_string(),
        f: Float => "Float: " + f.to_string(),
    }
}
```

---

## Type Intersection

```yaoxiang
type Printable = { print: () -> Void }
type Serializable = { to_json: () -> String }

# Intersection type
type Document = Printable & Serializable

# Implementation
type MyDoc = MyDoc(content: String) & {
    print: () -> Void = () => print(self.content)
    to_json: () -> String = () => '{"content": "' + self.content + '"}'
}
```

---

## Type Conversion

```yaoxiang
# Using as
int_to_float = 42 as Float        # 42.0
float_to_int = 3.14 as Int        # 3
string_to_int = "123" as Int      # 123
```

---

## Next Steps

- [Functions and Closures](functions.md) - Learn function definitions and higher-order functions
- [Control Flow](control-flow.md) - Conditionals and loops
- [Error Handling](error-handling.md) - Result and Option
