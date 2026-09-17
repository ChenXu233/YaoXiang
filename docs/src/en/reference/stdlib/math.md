---
title: 'std.math'
description: 'Integer, floating-point and trigonometric functions, including PI/E/TAU constants'
---

# std.math

Math module. All are **pure value functions**: parameters are passed by value (Copy semantics), with
no borrowing, no moving, and no side effects.

```yaoxiang
use std.math
```

## Constants

Can be used by importing by name:

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

main = {
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

> The integer family takes `Int`, the floating-point family takes `Float`. Passing mismatched types
> will be treated as `0` (falls back to `0` when `to_int` / `to_float` conversion fails), and no
> error is reported — it is recommended to rely on the type checker to intercept at compile-time.

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

main = {
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

The larger of the two.

```yaoxiang
use std.assert
use std.math

main = {
    assert(math.max(3, 7) == 7)
}
```

### min

<!-- stdlib:sig:math.min start -->

```yaoxiang
min: (a: Int, b: Int) -> Int
```

<!-- stdlib:sig:math.min end -->

The smaller of the two.

```yaoxiang
use std.assert
use std.math

main = {
    assert(math.min(3, 7) == 3)
}
```

### clamp

<!-- stdlib:sig:math.clamp start -->

```yaoxiang
clamp: (value: Int, min: Int, max: Int) -> Int
```

<!-- stdlib:sig:math.clamp end -->

Clamp `value` to the `[min, max]` range.

- `value` — the value to be clamped
- `min` — the lower bound (inclusive)
- `max` — the upper bound (inclusive)

Returns: A value within the range. Returns `min` if below the lower bound, returns `max` if above
the upper bound.

> **`min > max` will cause the interpreter to panic (#339)** (the precondition of the underlying
> `i64::clamp`); it does not return an error value. Please ensure `min <= max`.

```yaoxiang
use std.assert
use std.math

main = {
    assert(math.clamp(15, 1, 10) == 10)
    assert(math.clamp(-5, 1, 10) == 1)
    assert(math.clamp(5, 1, 10) == 5)
}
```

## Floating-Point Functions

### fabs

<!-- stdlib:sig:math.fabs start -->

```yaoxiang
fabs: (n: Float) -> Float
```

<!-- stdlib:sig:math.fabs end -->

Floating-point absolute value.

```yaoxiang
use std.assert
use std.math

main = {
    assert(math.fabs(-2.5) == 2.5)
}
```

### fmax

<!-- stdlib:sig:math.fmax start -->

```yaoxiang
fmax: (a: Float, b: Float) -> Float
```

<!-- stdlib:sig:math.fmax end -->

Floating-point larger value.

```yaoxiang
use std.assert
use std.math

main = {
    assert(math.fmax(1.5, 2.5) == 2.5)
}
```

### fmin

<!-- stdlib:sig:math.fmin start -->

```yaoxiang
fmin: (a: Float, b: Float) -> Float
```

<!-- stdlib:sig:math.fmin end -->

Floating-point smaller value.

```yaoxiang
use std.assert
use std.math

main = {
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

main = {
    assert(math.pow(2.0, 10.0) == 1024.0)
}
```

### sqrt

<!-- stdlib:sig:math.sqrt start -->

```yaoxiang
sqrt: (n: Float) -> Float
```

<!-- stdlib:sig:math.sqrt end -->

Square root. Returns `NaN` for negative numbers (no error).

```yaoxiang
use std.assert
use std.math

main = {
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

main = {
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

main = {
    assert(math.ceil(3.2) == 4.0)
}
```

### round

<!-- stdlib:sig:math.round start -->

```yaoxiang
round: (n: Float) -> Float
```

<!-- stdlib:sig:math.round end -->

Round (round half away from zero); the return value is still `Float`.

```yaoxiang
use std.assert
use std.math

main = {
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

Sine; the parameter is in **radians**.

```yaoxiang
use std.assert
use std.math

main = {
    assert(math.sin(0.0) == 0.0)
}
```

### cos

<!-- stdlib:sig:math.cos start -->

```yaoxiang
cos: (n: Float) -> Float
```

<!-- stdlib:sig:math.cos end -->

Cosine; the parameter is in **radians**.

```yaoxiang
use std.assert
use std.math

main = {
    assert(math.cos(0.0) == 1.0)
}
```

### tan

<!-- stdlib:sig:math.tan start -->

```yaoxiang
tan: (n: Float) -> Float
```

<!-- stdlib:sig:math.tan end -->

Tangent; the parameter is in **radians**.

```yaoxiang
use std.assert
use std.math

main = {
    assert(math.tan(0.0) == 0.0)
}
```

## Related

- [`std.string.parse_float`](./string#parse_float) — Parse string as `Float`
