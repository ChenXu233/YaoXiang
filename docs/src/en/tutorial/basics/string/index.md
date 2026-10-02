---
title: 'F-string'
---

# F-string

f-string is the **template string** in YaoXiang — you can directly embed variables and expressions
in a string, and the compiler automatically performs type conversion and concatenation.

## Basic Usage

Add the `f` prefix before a string and use `{expression}` to insert values:

```yaoxiang
name = "Alice"
age = 25

greeting = f"Hello {name}, you are {age} years old"
print(greeting)  // Hello Alice, you are 25 years old
```

Compared with traditional concatenation, the differences with f-string are immediately clear:

```yaoxiang
use std.string

name = "Alice"
age = 25

// ❌ Traditional concatenation: verbose and error-prone
// Note: String has no .concat() instance method, and Int has no to_string()
//—both raise E1053
message = "Hello, age: " + string.format("{0}", age)

// ✅ f-string: intuitive and concise
message2 = f"Hello {name}, age: {age}"
```

## Expression Interpolation

`{}` is not limited to variables — any expression can be placed inside:

```yaoxiang
x = 10
y = 20

print(f"Sum: {x + y}")         // Sum: 30
print(f"Product: {x * y}")     // Product: 200
print(f"Is positive? {x > 0}") // Is positive? true
```

## Format Specifiers

Add `:` and a format specifier after the expression to control the output format:

```yaoxiang
pi = 3.14159265

print(f"Pi: {pi}")       // Pi: 3.14159265
print(f"Pi: {pi:.2f}")   // Pi: 3.14 (2 decimal places)
print(f"Pi: {pi:.4f}")   // Pi: 3.1416 (4 decimal places)
```

Common format specifiers:

| Specifier | Meaning             | Example            | Output         |
| --------- | ------------------- | ------------------ | -------------- |
| `:.2f`    | Float, 2 decimals   | `f"{3.14159:.2f}"` | `3.14`         |
| `:d`      | Decimal integer     | `f"{42:d}"`        | `42`           |
| `:x`      | Hexadecimal         | `f"{255:x}"`       | `ff`           |
| `:e`      | Scientific notation | `f"{1000:e}"`      | `1.000000e+03` |
| `:s`      | String              | `f"{name:s}"`      | `hello`        |

## Method Calls

You can call methods inside `{}`:

```yaoxiang
use std.string

name = "alice"

print(f"Upper: {string.upper(name)}")   // Upper: ALICE
print(f"Length: {string.len(name)}")    // Length: 5
```

## Escaping Braces

If you need to output a literal `{` or `}`, simply **double them**:

<!-- docs-example: skip -->
```yaoxiang
print(f"{{literal braces}}")     // {literal braces}
print(f"Set: {{1, 2, 3}}")       // Set: {1, 2, 3}

// Mixed: doubled outputs a literal {, single denotes interpolation
name = "YaoXiang"
print(f"{{name}} is {name}")     // {name} is YaoXiang
```

## Multi-line f-string

f-string can span multiple lines:

```yaoxiang
name = "Alice"
age = 25
city = "Beijing"

info = f"Name: {name}" + "\n" + f"Age: {age}" + "\n" + f"City: {city}"

print(info)
// Name: Alice
// Age: 25
// City: Beijing
```

## How f-string Works

When the compiler sees an f-string, it converts it into efficient string concatenation:

```yaoxiang
use std.string

// What you write
f"Hello {name}, age: {age}"

// The equivalent hand-written form (note: there is no .concat() /
// to_string() instance method, and no implicit Int→String conversion)
"Hello " + string.format("{0}", name) + ", age: " + string.format("{0}", age)
```

This means f-string is not only more concise to write, but its runtime performance is comparable to
hand-written concatenation — **zero overhead**.

## Summary

:::: v-pre

| Key Point           | Syntax                     |
| ------------------- | -------------------------- |
| Basic interpolation | `f"text {var}"`            |
| Expression          | `f"result: {x + y}"`       |
| Formatting          | `f"value: {pi:.2f}"`       |
| Escaping braces     | `f"{{not interpolation}}"` |
| Multi-line          | `f"""..."""`               |
