---
title: F-string
---

# F-string

F-strings are YaoXiang's **template strings** — you can embed variables and expressions directly inside strings, with the compiler handling type conversion and concatenation automatically.

## Basic Usage

Prefix a string with `f` and use `{expression}` to insert values:

```yaoxiang
name = "Alice"
age = 25

greeting = f"Hello {name}, you are {age} years old"
println(greeting)  # Hello Alice, you are 25 years old
```

Compare with traditional concatenation — the difference is clear:

```yaoxiang
# ❌ Traditional: verbose and error-prone
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())

# ✅ F-string: clean, concise
message = f"Hello {name}, age: {age}"
```

## Expression Interpolation

`{}` accepts more than variables — any expression works:

```yaoxiang
x = 10
y = 20

println(f"Sum: {x + y}")         # Sum: 30
println(f"Product: {x * y}")     # Product: 200
println(f"Is positive? {x > 0}") # Is positive? true
```

## Format Specifiers

Add `:` followed by a format specifier after the expression to control output formatting:

```yaoxiang
pi = 3.14159265

println(f"Pi: {pi}")       # Pi: 3.14159265
println(f"Pi: {pi:.2f}")   # Pi: 3.14 (2 decimal places)
println(f"Pi: {pi:.4f}")   # Pi: 3.1416 (4 decimal places)
```

Common format specifiers:

| Specifier | Meaning | Example | Output |
|-----------|---------|---------|--------|
| `:.2f` | Float, 2 decimals | `f"{3.14159:.2f}"` | `3.14` |
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

To output literal `{` or `}`, **double** them:

```yaoxiang
println(f"{{literal braces}}")     # {literal braces}
println(f"Set: {{1, 2, 3}}")       # Set: {1, 2, 3}

# Mixed: doubled = literal, single = interpolation
name = "YaoXiang"
println(f"{{name}} is {name}")     # {name} is YaoXiang
```

## Multi-line F-strings

F-strings can span multiple lines:

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

## How F-strings Work

The compiler transforms f-strings into efficient string concatenation at compile time:

```yaoxiang
# What you write
f"Hello {name}, age: {age}"

# What the compiler generates
"Hello ".concat(name.to_string()).concat(", age: ").concat(age.to_string())
```

This means f-strings are not only more concise to write — they perform equally well at runtime. **Zero overhead.**

## Summary

::: v-pre
| Feature | Syntax |
|---------|--------|
| Basic interpolation | `f"text {var}"` |
| Expressions | `f"result: {x + y}"` |
| Formatting | `f"value: {pi:.2f}"` |
| Escaping braces | `f"{{not interpolation}}"` |
| Multi-line | `f"""..."""` |
:::
