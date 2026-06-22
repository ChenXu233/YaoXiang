---
title: F-string
---

# F-string

f-string is the **template string** in YaoXiang—you can embed variables and expressions directly in a string, and the compiler automatically handles type conversion and concatenation.

## Basic Usage

Add the `f` prefix before a string and use `{expression}` to insert a value:

```yaoxiang
name = "Alice"
age = 25

greeting = f"Hello {name}, you are {age} years old"
println(greeting)  # Hello Alice, you are 25 years old
```

Compared with traditional concatenation, the advantages of f-string are immediately obvious:

```yaoxiang
# ❌ Traditional concatenation: verbose and error-prone
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())

# ✅ f-string: intuitive and concise
message = f"Hello {name}, age: {age}"
```

## Expression Interpolation

`{}` is not limited to variables—you can put any expression inside:

```yaoxiang
x = 10
y = 20

println(f"Sum: {x + y}")         # Sum: 30
println(f"Product: {x * y}")     # Product: 200
println(f"Is positive? {x > 0}") # Is positive? true
```

## Format Specifiers

Add `:` and a format specifier after an expression to control the output format:

```yaoxiang
pi = 3.14159265

println(f"Pi: {pi}")       # Pi: 3.14159265
println(f"Pi: {pi:.2f}")   # Pi: 3.14 (2 decimal places)
println(f"Pi: {pi:.4f}")   # Pi: 3.1416 (4 decimal places)
```

Common format specifiers:

| Specifier | Meaning | Example | Output |
|-----------|---------|---------|--------|
| `:.2f` | Float, 2 decimal places | `f"{3.14159:.2f}"` | `3.14` |
| `:d` | Decimal integer | `f"{42:d}"` | `42` |
| `:x` | Hexadecimal | `f"{255:x}"` | `ff` |
| `:e` | Scientific notation | `f"{1000:e}"` | `1.000000e+03` |
| `:s` | String | `f"{name:s}"` | `hello` |

## Calling Methods

You can call methods inside `{}`:

```yaoxiang
name = "alice"

println(f"Upper: {name.uppercase()}")   # Upper: ALICE
println(f"Length: {name.len()}")        # Length: 5
```

## Escaping Braces

If you need to output a literal `{` or `}`, simply **double it**:

```yaoxiang
println(f"{{literal braces}}")     # {literal braces}
println(f"Set: {{1, 2, 3}}")       # Set: {1, 2, 3}

# Mixed: doubled outputs a literal {, single is interpolation
name = "YaoXiang"
println(f"{{name}} is {name}")     # {name} is YaoXiang
```

## Multi-line f-string

f-strings can span multiple lines:

```yaoxiang
name = "Alice"
age = 25
city = "Beijing"

info = f"""
Name: {name}
Age: {age}
City: {city}
"""

println(info)
# Name: Alice
# Age: 25
# City: Beijing
```

## How f-string Works

When the compiler sees an f-string, it transforms it into efficient string concatenation:

```yaoxiang
# What you write
f"Hello {name}, age: {age}"

# Compiler's transformed result
"Hello ".concat(name.to_string()).concat(", age: ").concat(age.to_string())
```

This means f-string is not only more concise to write, but its runtime performance is comparable to hand-written concatenation—**zero overhead**.

## Summary

::: v-pre
| Key Point | Syntax |
|-----------|--------|
| Basic interpolation | `f"text {var}"` |
| Expression | `f"result: {x + y}"` |
| Formatting | `f"value: {pi:.2f}"` |
| Escaping braces | `f"{{not interpolation}}"` |
| Multi-line | `f"""..."""` |
:::
```