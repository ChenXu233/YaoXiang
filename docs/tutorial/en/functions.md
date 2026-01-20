# Functions and Closures

> Version: v2.0.0
> Status: Updated (Based on RFC-010 Unified Type Syntax + RFC-004 Position Binding)

---

## Unified Syntax: name: type = value

YaoXiang uses a unified syntax for all declarations:

```yaoxiang
# Variable
x: Int = 42

# Function
add: (Int, Int) -> Int = (a, b) => a + b

# Type method
Point.distance: (Point, Point) -> Float = (p1, p2) => ...
```

---

## Function Definition

### Full Form (Recommended)

```yaoxiang
# Basic function
greet: (String) -> String = (name) => "Hello, " + name

# Multi-parameter function
add: (Int, Int) -> Int = (a, b) => a + b

# Single parameter shorthand
inc: Int -> Int = x => x + 1

# Multi-line function
fact: (Int) -> Int = (n) => {
    if n == 0 { 1 } else { n * fact(n - 1) }
}
```

### Shorthand Form

```yaoxiang
# Shorthand form
add(Int, Int) -> Int = (a, b) => a + b

greet(String) -> String = (name) => "Hello, " + name
```

---

## Generic Functions

```yaoxiang
# Generic function
identity: [T](T) -> T = (x) => x

# Usage
n: Int = identity(42)              # Int
s: String = identity("hello")       # String
b: Bool = identity(true)            # Bool

# Generic higher-order function
map: [T, U]((T) -> U, List[T]) -> List[U] = (f, list) => {
    result: List[U] = List()
    for item in list {
        result.append(f(item))
    }
    result
}

# Usage
doubled: List[Int] = map((x) => x * 2, List([1, 2, 3]))  # [2, 4, 6]
```

---

## Higher-Order Functions

### Accept Function as Parameter

```yaoxiang
# Higher-order function
apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)

# Usage
double: (Int) -> Int = x => x * 2
result: Int = apply(double, 5)     # 10

# Shorthand
result2: Int = apply((x) => x + 1, 5)  # 6
```

### Return Function

```yaoxiang
# Return function
create_multiplier: (Int) -> (Int) -> Int = (factor) => (x) => x * factor

# Usage
double: (Int) -> Int = create_multiplier(2)
triple: (Int) -> Int = create_multiplier(3)
result1: Int = double(5)           # 10
result2: Int = triple(5)           # 15
```

---

## Closures

### Capture External Variables

```yaoxiang
# Create closure
create_counter: () -> () -> Int = () => {
    mut count: Int = 0
    () => {
        count = count + 1
        count
    }
}

# Usage
counter: () -> Int = create_counter()
c1: Int = counter()                # 1
c2: Int = counter()                # 2
c3: Int = counter()                # 3
```

### Capture Multiple Variables

```yaoxiang
create_adder: (Int) -> (Int) -> Int = (base) => {
    add_to_base: (Int) -> Int = (x) => base + x
    add_to_base
}

add5: (Int) -> Int = create_adder(5)
result: Int = add5(10)             # 15
```

---

## Currying

YaoXiang supports automatic currying:

```yaoxiang
# Multi-parameter function can be partially applied
add: (Int, Int) -> Int = (a, b) => a + b

# Full call
result1: Int = add(3, 5)           # 8

# Partial application
add5: (Int) -> Int = add(5)
result2: Int = add5(10)            # 15

# Chained partial application
curried_add: (Int) -> (Int) -> Int = add
add3: (Int) -> Int = curried_add(3)
add5_more: Int = add3(5)           # 8
```

---

## Methods and Binding

### Type Method Definition

Use `Type.method: (Type, ...) -> ReturnType = ...` syntax:

```yaoxiang
type Point = { x: Float, y: Float }

# Type method: first parameter is self (caller)
Point.distance: (Point, Point) -> Float = (self, other) => {
    dx = self.x - other.x
    dy = self.y - other.y
    (dx * dx + dy * dy).sqrt()
}

# Usage
p1: Point = Point(3.0, 4.0)
p2: Point = Point(1.0, 1.0)
d: Float = p1.distance(p2)
```

### Position Binding [n]

Bind standalone functions to specific parameter positions of a type:

```yaoxiang
type Point = { x: Float, y: Float }

# Define standalone function
distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# Bind to Point.distance (this bound to position 0)
Point.distance: (Point, Point) -> Float = distance[0]

# Call: functional style
d1: Float = distance(p1, p2)

# Call: OOP syntax sugar
d2: Float = p1.distance(p2)
```

### Specify Binding Position

```yaoxiang
# Function signature is transform(Vector, Point)
transform: (Vector, Point) -> Point = (v, p) => {
    Point(p.x + v.x, p.y + v.y)
}

# Bind Point.transform, bind this to position 1
Point.transform: (Point, Vector) -> Point = transform[1]

# Call: p.transform(v) → transform(v, p)
result: Point = p1.transform(v1)
```

### Multi-Position Binding

```yaoxiang
type Point = { x: Float, y: Float }

# Function receives multiple Point parameters
scale_points: (Point, Point, Float) -> Point = (p1, p2, factor) => {
    Point(p1.x * factor, p1.y * factor)
}

# Bind multiple positions (automatic currying)
Point.scale: (Point, Point, Float) -> Point = scale_points[0, 1]

# Call
p1.scale(p2)(2.0)  # → scale_points(p1, p2, 2.0)
```

### Placeholder _

Skip certain positions:

```yaoxiang
# Only bind position 1, keep positions 0 and 2
Point.custom_op: (Point, Point, Float) -> Float = func[1, _]

# Call: p1.custom_op(p2, 0.5) → func(p1, p2, 0.5)
```

### Auto-Binding pub

Functions declared with `pub` are **automatically bound** to types defined in the same file:

```yaoxiang
type Point = { x: Float, y: Float }

# Using pub declaration, compiler auto-binds to Point
pub distance: (Point, Point) -> Float = (p1, p2) => {
    dx = p1.x - p2.x
    dy = p1.y - p2.y
    (dx * dx + dy * dy).sqrt()
}

# Compiler infers: Point.distance = distance[0]

# Now you can call:
d1: Float = distance(p1, p2)      # Functional style
d2: Float = p1.distance(p2)       # OOP syntax sugar
```

### Auto-Binding Rules

| Function Declaration | Auto-Binding Result |
|---------------------|---------------------|
| `pub distance: (Point, Point) -> Float = ...` | `Point.distance = distance[0]` |
| `pub draw: (Point, Surface) -> Void = ...` | `Point.draw = draw[0]` |
| `pub transform: (Vector, Point) -> Point = ...` | Manual position required |

---

## Built-in Methods

### String Functions

```yaoxiang
len: Int = "hello".length          # 5
upper: String = "hello".to_upper()    # "HELLO"
lower: String = "HELLO".to_lower()    # "hello"
```

### List Functions

```yaoxiang
numbers: List[Int] = List([1, 2, 3, 4, 5])

length: Int = numbers.length       # 5
first: Int = numbers[0]            # 1
last: Int = numbers[-1]            # 5
reversed: List[Int] = numbers.reversed() # [5, 4, 3, 2, 1]
```

---

## Recursive Functions

```yaoxiang
# Factorial
fact: (Int) -> Int = (n) => {
    if n <= 1 { 1 } else { n * fact(n - 1) }
}

# Fibonacci
fib: (Int) -> Int = (n) => {
    if n <= 1 { n } else { fib(n - 1) + fib(n - 2) }
}

# List sum
sum_list: (List[Int]) -> Int = (list) => {
    if list.length == 0 { 0 } else { list[0] + sum_list(list.tail()) }
}
```

---

## Next Steps

- [Control Flow](control-flow.md) - Conditionals and loops
- [Error Handling](error-handling.md) - Result and Option
- [Generic Programming](generics.md) - More complex generic patterns
