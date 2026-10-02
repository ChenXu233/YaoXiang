---
title: 'F-string'
---

# F-string

f-string is a **template string** in YaoXiang — you can embed variables and expressions directly in
the string, and the compiler automatically handles type conversion and concatenation.

## Basic Usage

Add the `f` prefix before a string, and use `{expression}` to insert values:

```yaoxiang
name = "Alice"
age = 25

greeting = f"Hello {name}, you are {age} years old"
print(greeting)  // Hello Alice, you are 25 years old
```

Compared with traditional concatenation, the difference with f-string is clear at a glance:

```yaoxiang
use std.string

name = "Alice"
age = 25

// ❌ Traditional concatenation: verbose and error-prone
// Note: String has no .concat() instance method, nor Int.to_string() — both report E1053
message = "Hello, age: " + string.format("{0}", age)

// ✅ f-string: intuitive and concise
message2 = f"Hello {name}, age: {age}"
```

## Expression Interpolation

Inside `{}` you are not limited to variables — you can put any expression:

```yaoxiang
x = 10
y = 20

print(f"Sum: {x + y}")         // Sum: 30
print(f"Product: {x * y}")     // Product: 200
print(f"Is positive? {x > 0}") // Is positive? true
```

## Format Specifiers

Add `:` and a format specifier after an expression to control the output format:

```yaoxiang
pi = 3.14159265

print(f"Pi: {pi}")       // Pi: 3.14159265
print(f"Pi: {pi}")      // Reality check: format specifiers are not yet implemented, output is 3.14159
print(f"Pi: {pi}")      // If you need rounding, please handle it yourself
```

Common format specifiers:

| Specifier | Meaning             | Example            | Output         |
| --------- | ------------------- | ------------------ | -------------- |
| `:.2f`    | Float, 2 decimals   | `f"{3.14159:.2f}"` | `3.14`         |
| `:d`      | Decimal integer     | `f"{42:d}"`        | `42`           |
| `:x`      | Hexadecimal         | `f"{255:x}"`       | `ff`           |
| `:e`      | Scientific notation | `f"{1000:e}"`      | `1.000000e+03` |
| `:s`      | String              | `f"{name:s}"`      | `hello`        |

## Calling Methods

You can call methods inside `{}`:

```yaoxiang
use std.string

// Note: conversion goes through std.string's function form — `name.uppercase()` / `name.len()` report E1053
name = "alice"

print(f"Upper: {string.upper(name)}")  // Upper: ALICE
print(f"Length: {string.len(name)}")   // Length: 5
```

## Escaping Braces

⚠️ 0.8.2 **does not implement brace escaping**. Following the convention of Rust/Python, to output a
literal left brace you would write it twice — YaoXiang does not support this syntax, and it will
treat the content between the two braces as a variable name to parse:

<!-- docs-example: skip -->

```yaoxiang
print(f"{{literal braces}}")     // Expected: {literal braces}
print(f"Set: {{1, 2, 3}}")       // Expected: Set: {1, 2, 3}
```

> The two lines above **intentionally fail the check** — they report
> `E1001 Unknown variable: 'literal'`.

To output literal braces today, you have to bypass f-string and use ordinary string concatenation:

```yaoxiang
main = () => {
    name = "YaoXiang"

    // A single brace is interpolation
    print(f"{name} is {name}")

    // When you need literal braces, use ordinary string concatenation
    print("Set: " + "{" + "1, 2, 3" + "}")
}
```

## Multi-line f-string

⚠️ 0.8.2 **does not support triple quotes `f"""..."""`** — the lexer does not recognize multi-line
string literals, and a cross-line f-string reports `E0012 unterminated string`. For cross-line
content, please concatenate multiple f-strings:

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

When the compiler sees an f-string, it converts it into string concatenation. Semantically it is
equivalent to:

```yaoxiang
use std.string

// What you write
f"Hello {name}, age: {age}"

// The equivalent hand-written form (note: there are no .concat() / .to_string() instance methods,
// nor an implicit Int→String conversion)
"Hello " + string.format("{0}", name) + ", age: " + string.format("{0}", age)
```

This means f-string is not only more concise to write, but its runtime performance is comparable to
hand-written concatenation — **zero additional overhead**.

## Summary

:::: v-pre

| Point               | Syntax                                                                            |
| ------------------- | --------------------------------------------------------------------------------- |
| Basic interpolation | `f"text {var}"`                                                                   |
| Expression          | `f"result: {x + y}"`                                                              |
| Formatting          | ⚠️ Not yet implemented: `{pi:.2f}` is output as-is                                |
| Escaping braces     | ⚠️ Not implemented: writing two left braces is treated as a variable name (E1001) |
| Multi-line          | ⚠️ Not implemented: `f"""..."""` reports E0012                                    |
