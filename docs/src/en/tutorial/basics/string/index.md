---
title: F-string
---

# F-string

f-string is YaoXiang's **template string** — you can embed variables and expressions directly within
the string, and the compiler automatically handles type conversion and concatenation.

## Basic Usage

Add an `f` prefix before the string and use `{expression}` to insert values:

```yaoxiang
name = "Alice"
age = 25

greeting = f"Hello {name}, you are {age} years old"
print(greeting)  // Hello Alice, you are 25 years old
```

The difference with traditional concatenation is clear at a glance:

```yaoxiang
// ❌ Traditional concatenation: verbose and error-prone
message = "Hello ".concat(name).concat(", age: ").concat(age.to_string())

// ✅ f-string: intuitive, concise
message = f"Hello {name}, age: {age}"
```

## Expression Interpolation

`{}` is not limited to variables — you can place any expression inside:

```yaoxiang
x = 10
y = 20

print(f"Sum: {x + y}")         // Sum: 30
print(f"Product: {x * y}")     // Product: 200
print(f"Is positive? {x > 0}") // Is positive? true
```

## Format Specifiers

Add `:` followed by a format specifier after an expression to control output formatting:

```yaoxiang
pi = 3.14159265

print(f"Pi: {pi}")       // Pi: 3.14159265
print(f"Pi: {pi:.2f}")   // Pi: 3.14 (2 decimal places)
print(f"Pi: {pi:.4f}")   // Pi: 3.1416 (4 decimal places)
```

Common format specifiers:

| Specifier | Meaning           | Example            | Output         |
| --------- | ----------------- | ------------------ | -------------- |
| `:.2f`    | Float, 2 decimals | `f"{3.14159:.2f}"` | `3.14`         |
| `:d`      | Decimal integer   | `f"{42:d}"`        | `42`           |
| `:x`      | Hexadecimal       | `f"{255:x}"`       | `ff`           |
| `:e`      | Scientific        | `f"{1000:e}"`      | `1.000000e+03` |
| `:s`      | String            | `f"{name:s}"`      | `hello`        |

## Calling Methods

You can call methods inside `{}`:

```yaoxiang
name = "alice"

print(f"Upper: {name.uppercase()}")   // Upper: ALICE
print(f"Length: {name.len()}")        // Length: 5
```

## Escaping Braces

To output literal `{` or `}`, **double them**:

```yaoxiang
print(f"{{literal braces}}")     // {literal braces}
print(f"Set: {{1, 2, 3}}")       // Set: {1, 2, 3}

// Mixed: double for literal {, single for interpolation
name = "YaoXiang"
print(f"{{name}} is {name}")     // {name} is YaoXiang
```

## Multi-line f-string

f-string can span multiple lines:

```yaoxiang
name = "Alice"
age = 25
city = "Beijing"

info = f"""
Name: {name}
Age: {age}
City: {city}
"""

print(info)
// Name: Alice
// Age: 25
// City: Beijing
```

## How f-string Works

When the compiler encounters an f-string, it converts it to efficient string concatenation:

```yaoxiang
// What you write
f"Hello {name}, age: {age}"

// Compiler transformation
"Hello ".concat(name.to_string()).concat(", age: ").concat(age.to_string())
```

This means f-string not only writes more concisely but also has comparable runtime performance to
manual concatenation — **zero additional overhead**.

## Summary

:::: v-pre

| Point         | Syntax                     |
| ------------- | -------------------------- |
| Basic insert  | `f"text {var}"`            |
| Expression    | `f"result: {x + y}"`       |
| Formatting    | `f"value: {pi:.2f}"`       |
| Escape braces | `f"{{not interpolation}}"` |
| Multi-line    | `f"""..."""`               |
