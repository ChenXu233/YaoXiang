# Functions and Closures

> Version: v1.0.0
> Status: In Progress

---

## Function Definition

### Form One: Type Centralization (Recommended)

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

### Form Two: Shorthand

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
n = identity(42)              # Int
s = identity("hello")         # String
b = identity(true)            # Bool

# Generic higher-order function
map: [T, U]((T) -> U, [T]) -> [U] = (f, list) => {
    result: [U] = []
    for item in list {
        result.append(f(item))
    }
    result
}

# Usage
doubled = map((x) => x * 2, [1, 2, 3])  # [2, 4, 6]
```

---

## Higher-Order Functions

### Accept Function as Parameter

```yaoxiang
# Higher-order function
apply: ((Int) -> Int, Int) -> Int = (f, x) => f(x)

# Usage
double: (Int) -> Int = x => x * 2
result = apply(double, 5)     # 10

# Shorthand
result2 = apply((x) => x + 1, 5)  # 6
```

### Return Function

```yaoxiang
# Return function
create_multiplier: (Int) -> (Int) -> Int = (factor) => (x) => x * factor

# Usage
double = create_multiplier(2)
triple = create_multiplier(3)
result1 = double(5)           # 10
result2 = triple(5)           # 15
```

---

## Closures

### Capture External Variables

```yaoxiang
# Create closure
create_counter() -> () -> Int = () => {
    mut count = 0
    () => {
        count = count + 1
        count
    }
}

# Usage
counter = create_counter()
c1 = counter()                # 1
c2 = counter()                # 2
c3 = counter()                # 3
```

### Capture Multiple Variables

```yaoxiang
create_adder(base: Int) -> (Int) -> Int = (base) => {
    add_to_base: (Int) -> Int = (x) => base + x
    add_to_base
}

add5 = create_adder(5)
result = add5(10)             # 15
```

---

## Currying

YaoXiang supports automatic currying:

```yaoxiang
# Multi-parameter function can be partially applied
add: (Int, Int) -> Int = (a, b) => a + b

# Full call
result1 = add(3, 5)           # 8

# Partial application
add5: (Int) -> Int = add(5)
result2 = add5(10)            # 15

# Chained partial application
curried_add: (Int) -> (Int) -> Int = add
add3 = curried_add(3)
add5_more = add3(5)           # 8
```

---

## Method Binding

### Position Binding

```yaoxiang
type MathOps = MathOps(add: (Int, Int) -> Int, mul: (Int, Int) -> Int)

ops = MathOps(
    add: (a, b) => a + b,
    mul: (a, b) => a * b
)

# Usage
sum = ops.add(3, 5)           # 8
product = ops.mul(3, 5)       # 15
```

---

## Built-in Functions

### String Functions

```yaoxiang
len = "hello".length          # 5
upper = "hello".to_upper()    # "HELLO"
lower = "HELLO".to_lower()    # "hello"
```

### List Functions

```yaoxiang
numbers = [1, 2, 3, 4, 5]

length = numbers.length       # 5
first = numbers[0]            # 1
last = numbers[-1]            # 5
reversed = numbers.reversed() # [5, 4, 3, 2, 1]
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
sum_list: ([Int]) -> Int = (list) => {
    if list.length == 0 { 0 } else { list[0] + sum_list(list[1..]) }
}
```

---

## Next Steps

- [Control Flow](control-flow.md) - Conditionals and loops
- [Error Handling](error-handling.md) - Result and Option
- [Generic Programming](generics.md) - More complex generic patterns
