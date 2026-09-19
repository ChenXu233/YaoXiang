---
title: 'std.math'
description: 'Integer, floating-point and trigonometric functions, including PI/E/TAU constants'
---

# std.math

Math module. All are **pure value functions**: arguments are passed by value (Copy semantics), with
no borrowing, no moving, and no side effects.

```yaoxiang
use std.math
```

## Constants

Import by name to use:

```yaoxiang
use std.math.{PI, E, TAU}
```

| Constant | Type    | Value                  |
| -------- | ------- | ---------------------- |
| `PI`     | `Float` | π ≈ 3.141592653589793  |
| `E`      | `Float` | e ≈ 2.718281828459045  |
| `TAU`    | `Float` | 2π ≈ 6.283185307179586 |

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14 and PI < 3.15)
    assert(E > 2.71 and E < 2.72)
    assert(TAU > 6.28 and TAU < 6.29)
}
```

## Function Overview

<!-- stdlib:table:math start -->

| Function | Signature                                 |
| -------- | ----------------------------------------- |
| `abs`    | `(n: Int) -> Int`                         |
| `max`    | `(a: Int, b: Int) -> Int`                 |
| `min`    | `(a: Int, b: Int) -> Int`                 |
| `clamp`  | `(value: Int, min: Int, max: Int) -> Int` |
| `fabs`   | `(n: Float) -> Float`                     |
| `fmax`   | `(a: Float, b: Float) -> Float`           |
| `fmin`   | `(a: Float, b: Float) -> Float`           |
| `pow`    | `(base: Float, exp: Float) -> Float`      |
| `sqrt`   | `(n: Float) -> Float`                     |
| `floor`  | `(n: Float) -> Float`                     |
| `ceil`   | `(n: Float) -> Float`                     |
| `round`  | `(n: Float) -> Float`                     |
| `sin`    | `(n: Float) -> Float`                     |
| `cos`    | `(n: Float) -> Float`                     |
| `tan`    | `(n: Float) -> Float`                     |
| `PI`     | `Float`                                   |
| `E`      | `Float`                                   |
| `TAU`    | `Float`                                   |

<!-- stdlib:table:math end -->

> The integer family takes `Int`, the float family takes `Float`. Arguments of mismatched types are
> treated as `0` (falling back to `0` when `to_int` / `to_float` conversion fails), and no error is
> raised — it is recommended to rely on the type checker to catch this at compile-time.

## Integer Functions

### abs

<!-- stdlib:sig:math.abs start -->

```yaoxiang
abs: (n: Int) -> Int
```

<!-- stdlib:sig:math.abs end -->

Absolute value.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.abs(-5) == 5)
    assert(math.abs(5) == 5)
}
```

### max

<!-- stdlib:sig:math.max start -->

```yaoxiang
max: (a: Int, b: Int) -> Int
```

<!-- stdlib:sig:math.max end -->

The larger of the two values.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.max(3, 7) == 7)
}
```

### min

<!-- stdlib:sig:math.min start -->

```yaoxiang
min: (a: Int, b: Int) -> Int
```

<!-- stdlib:sig:math.min end -->

The smaller of the two values.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.min(3, 7) == 3)
}
```

### clamp

<!-- stdlib:sig:math.clamp start -->

```yaoxiang
clamp: (value: Int, min: Int, max: Int) -> Int
```

<!-- stdlib:sig:math.clamp end -->

Clamps `value` to the `[min, max]` range.

- `value` — the value to be clamped
- `min` — the lower bound (inclusive)
- `max` — the upper bound (inclusive)

Returns: the value within the range. Values below the lower bound return `min`; values above the
upper bound return `max`.

> **`min > max` will cause the interpreter to panic (#339)** (a precondition of the underlying
> `i64::clamp`); it will not return an error value. Please ensure that `min <= max`.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.clamp(15, 1, 10) == 10)
    assert(math.clamp(-5, 1, 10) == 1)
    assert(math.clamp(5, 1, 10) == 5)
}
```

## Float Functions

### fabs

<!-- stdlib:sig:math.fabs start -->

```yaoxiang
fabs: (n: Float) -> Float
```

<!-- stdlib:sig:math.fabs end -->

Float absolute value.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.fabs(-2.5) == 2.5)
}
```

### fmax

<!-- stdlib:sig:math.fmax start -->

```yaoxiang
fmax: (a: Float, b: Float) -> Float
```

<!-- stdlib:sig:math.fmax end -->

Float maximum.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.fmax(1.5, 2.5) == 2.5)
}
```

### fmin

<!-- stdlib:sig:math.fmin start -->

```yaoxiang
fmin: (a: Float, b: Float) -> Float
```

<!-- stdlib:sig:math.fmin end -->

Float minimum.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.fmin(1.5, 2.5) == 1.5)
}
```

### pow

<!-- stdlib:sig:math.pow start -->

```yaoxiang
pow: (base: Float, exp: Float) -> Float
```

<!-- stdlib:sig:math.pow end -->

`base` raised to the `exp` power.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.pow(2.0, 10.0) == 1024.0)
}
```

### sqrt

<!-- stdlib:sig:math.sqrt start -->

```yaoxiang
sqrt: (n: Float) -> Float
```

<!-- stdlib:sig:math.sqrt end -->

Square root. Negative numbers return `NaN` (no error is raised).

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.sqrt(4.0) == 2.0)
    assert(math.sqrt(2.0) > 1.41 and math.sqrt(2.0) < 1.42)
}
```

### floor

<!-- stdlib:sig:math.floor start -->

```yaoxiang
floor: (n: Float) -> Float
```

<!-- stdlib:sig:math.floor end -->

Round down; the return value is still `Float`.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.floor(3.7) == 3.0)
}
```

### ceil

<!-- stdlib:sig:math.ceil start -->

```yaoxiang
ceil: (n: Float) -> Float
```

<!-- stdlib:sig:math.ceil end -->

Round up; the return value is still `Float`.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.ceil(3.2) == 4.0)
}
```

### round

<!-- stdlib:sig:math.round start -->

```yaoxiang
round: (n: Float) -> Float
```

<!-- stdlib:sig:math.round end -->

Round half away from zero; the return value is still `Float`.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.round(3.5) == 4.0)
    assert(math.round(3.4) == 3.0)
}
```

### sin

<!-- stdlib:sig:math.sin start -->

```yaoxiang
sin: (n: Float) -> Float
```

<!-- stdlib:sig:math.sin end -->

Sine; the argument is in **radians**.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.sin(0.0) == 0.0)
}
```

### cos

<!-- stdlib:sig:math.cos start -->

```yaoxiang
cos: (n: Float) -> Float
```

<!-- stdlib:sig:math.cos end -->

Cosine; the argument is in **radians**.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.cos(0.0) == 1.0)
}
```

### tan

<!-- stdlib:sig:math.tan start -->

```yaoxiang
tan: (n: Float) -> Float
```

<!-- stdlib:sig:math.tan end -->

Tangent; the argument is in **radians**.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.tan(0.0) == 0.0)
}
```

## Related

- [`std.string.parse_float`](./string#parse_float) — parse a string into `Float`
