---
title: 'F-string'
---

# F-string

f-string is YaoXiang's **template string**—you can embed variables and expressions directly in the
string, and the compiler automatically handles type conversion and concatenation.

## Basic usage

Prefix the string with `f`, and use `{expression}` to insert values:

```yaoxiang
name = "Alice"
age = 25

greeting = f"Hello {name}, you are {age} years old"
print(greeting)  // Hello Alice, you are 25 years old
```

Compared to traditional concatenation, the difference with f-strings is clear at a glance:

```yaoxiang
use std.string

name = "Alice"
age = 25

// ❌ Traditional concatenation: verbose and error-prone
// Note: String has no .concat() instance method, nor Int.to_string()—both report E1053
message = "Hello, age: " + string.format("{0}", age)

// ✅ f-string: intuitive, concise
message2 = f"Hello {name}, age: {age}"
```

## Expression interpolation

Inside `{}` you can put more than variables—any expression works:

```yaoxiang
x = 10
y = 20

print(f"Sum: {x + y}")         // Sum: 30
print(f"Product: {x * y}")     // Product: 200
print(f"Is positive? {x > 0}") // Is positive? true
```

## Format specifiers

Add `:` and a format specifier after the expression to control the output format:

```yaoxiang
pi = 3.14159265
name = "hello"

print(f"Pi: {pi}")       // Pi: 3.14159265
print(f"Pi: {pi:.2f}")   // Pi: 3.14 (keep 2 decimal places)
print(f"{255:x}")        // ff (hexadecimal)
print(f"{1000:e}")       // 1.000000e+03 (scientific notation)
print(f"{name:s}")       // hello (string display)
```

Common format specifiers:

| Specifier | Meaning                 | Example            | Output         |
| --------- | ----------------------- | ------------------ | -------------- |
| `:.2f`    | Float, 2 decimal places | `f"{3.14159:.2f}"` | `3.14`         |
| `:d`      | Decimal integer         | `f"{42:d}"`        | `42`           |
| `:x`      | Hexadecimal             | `f"{255:x}"`       | `ff`           |
| `:e`      | Scientific notation     | `f"{1000:e}"`      | `1.000000e+03` |
| `:s`      | String                  | `f"{name:s}"`      | `hello`        |

## Calling methods

You can call methods inside `{}`:

```yaoxiang
use std.string

// Note: conversion goes through std.string's function form—`name.uppercase()` / `name.len()` reports E1053
name = "alice"

print(f"Upper: {string.upper(name)}")  // Upper: ALICE
print(f"Length: {string.len(name)}")   // Length: 5
```

## Escaping braces

To output a literal `{` or `}`, **double it** (consistent with Python):

```yaoxiang
print(f"{{literal braces}}")     // {literal braces}
print(f"Set: {{1, 2, 3}}")       // Set: {1, 2, 3}
```

Mix and match: double it to output a literal `{`, single it for interpolation:

```yaoxiang
name = "YaoXiang"

print(f"{{name}} is {name}")     // {name} is YaoXiang
```

## Multi-line f-string

Triple quotes `f"""..."""` support multi-line templates, where newlines are part of the content
(consistent with ordinary `"""` multi-line strings):

```yaoxiang
name = "Alice"
age = 25
city = "Beijing"

info = f"""Name: {name}
Age: {age}
City: {city}"""

print(info)
// Name: Alice
// Age: 25
// City: Beijing
```

Interpolation and escaped braces work as usual in multi-line templates, as do string escapes (`\n`
etc.).

## How f-strings work

When the compiler sees an f-string, it converts it to string concatenation. Semantically equivalent
to:

```yaoxiang
use std.string

// What you write
f"Hello {name}, age: {age}"

// Equivalent hand-written form (note: no .concat() / .to_string() instance method,
// nor implicit Int→String conversion)
"Hello " + string.format("{0}", name) + ", age: " + string.format("{0}", age)
```

This means f-strings are not only more concise to write, but their runtime performance is on par
with hand-written concatenation—**zero extra overhead**.

## Summary

:::: v-pre

| Key point           | Syntax                     |
| ------------------- | -------------------------- |
| Basic interpolation | `f"text {var}"`            |
| Expression          | `f"result: {x + y}"`       |
| Formatting          | `f"value: {pi:.2f}"`       |
| Escaped braces      | `f"{{not interpolation}}"` |
| Multi-line          | `f"""..."""`               |
