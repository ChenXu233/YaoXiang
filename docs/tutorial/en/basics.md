# Quick Start

> Version: v1.0.0
> Status: In Progress

---

## Install YaoXiang

```bash
cargo install yaoxiang
```

## First Program

Create file `hello.yx`:

```yaoxiang
main: () -> Void = () => {
    print("Hello, YaoXiang!")
}
```

Run:

```bash
yaoxiang hello.yx
```

---

## Variables and Types

### Basic Types

```yaoxiang
# Integer
age: Int = 25

# Floating point
price: Float = 19.99

# String
name: String = "YaoXiang"

# Boolean
is_active: Bool = true

# Character
grade: Char = 'A'
```

### Type Inference

YaoXiang supports type inference, allowing you to omit type annotations:

```yaoxiang
x = 42              # Inferred as Int
y = 3.14            # Inferred as Float
z = "hello"         # Inferred as String
```

### Mutable Variables

Variables are immutable by default, use `mut` to declare mutable variables:

```yaoxiang
count: Int = 0
count = 1            # ❌ Error: Immutable

mut counter = 0
counter = counter + 1  # ✅ Correct: Mutable
```

---

## Operators

### Arithmetic Operators

```yaoxiang
a = 10 + 5          # Addition: 15
b = 10 - 3          # Subtraction: 7
c = 4 * 6           # Multiplication: 24
d = 15 / 2          # Division: 7.5
e = 15 // 2         # Integer division: 7
f = 15 % 4          # Modulo: 3
```

### Comparison Operators

```yaoxiang
equal = a == b      # Equal
not_equal = a != b  # Not equal
less = a < b        # Less than
greater = a > b     # Greater than
less_equal = a <= b # Less than or equal
greater_equal = a >= b  # Greater than or equal
```

### Logical Operators

```yaoxiang
and_result = true and false   # false
or_result = true or false     # true
not_result = not true         # false
```

### Bit Operators

```yaoxiang
and_result = 5 & 3            # 1 (0101 & 0011)
or_result = 5 | 3             # 7 (0101 | 0011)
xor_result = 5 ^ 3            # 6 (0101 ^ 0011)
not_result = not 5            # -6
left_shift = 5 << 1           # 10
right_shift = 5 >> 1          # 2
```

---

## Collection Types

### Lists

```yaoxiang
numbers: List[Int] = [1, 2, 3, 4, 5]
empty_list: List[String] = []

# Access elements
first = numbers[0]             # 1
last = numbers[-1]             # 5
```

```yaoxiang
# Modify list
mut nums = [1, 2, 3]
nums.append(4)                 # [1, 2, 3, 4]
nums.remove(0)                 # [2, 3, 4]
```

### Dictionaries

```yaoxiang
scores = {"Alice": 95, "Bob": 87}

# Access
alice_score = scores["Alice"]  # 95

# Modify
scores["Charlie"] = 92
scores["Bob"] = 90
```

### Tuples

```yaoxiang
point = (3.0, 4.0)
coordinate = (x: Int, y: Int, z: Int) = (1, 2, 3)

# Destructure
(x, y) = point
```

---

## Comments

```yaoxiang
# Single-line comment

#! Multi-line comment
   Can span multiple lines
   Second line !#
```

---

## Next Steps

- [Type System](types.md) - Deep dive into the type system
- [Functions and Closures](functions.md) - Learn function definitions
- [Control Flow](control-flow.md) - Conditionals and loops
