---
title: 'std.math'
description: '整数、浮点与三角函数，含 PI/E/TAU 常量'
---

# std.math

数学模块。全部为**纯值函数**：参数按值（Copy 语义）传入，无借用、无移动、无副作用。

```yaoxiang
use std.math
```

## 常量

按名导入即可使用：

```yaoxiang
use std.math.{PI, E, TAU}
```

| 常量  | 类型    | 值                     |
| ----- | ------- | ---------------------- |
| `PI`  | `Float` | π ≈ 3.141592653589793  |
| `E`   | `Float` | e ≈ 2.718281828459045  |
| `TAU` | `Float` | 2π ≈ 6.283185307179586 |

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14 and PI < 3.15)
    assert(E > 2.71 and E < 2.72)
    assert(TAU > 6.28 and TAU < 6.29)
}
```

## 函数一览

<!-- stdlib:table:math start -->

| 函数    | 签名                                      |
| ------- | ----------------------------------------- |
| `abs`   | `(n: Int) -> Int`                         |
| `max`   | `(a: Int, b: Int) -> Int`                 |
| `min`   | `(a: Int, b: Int) -> Int`                 |
| `clamp` | `(value: Int, min: Int, max: Int) -> Int` |
| `fabs`  | `(n: Float) -> Float`                     |
| `fmax`  | `(a: Float, b: Float) -> Float`           |
| `fmin`  | `(a: Float, b: Float) -> Float`           |
| `pow`   | `(base: Float, exp: Float) -> Float`      |
| `sqrt`  | `(n: Float) -> Float`                     |
| `floor` | `(n: Float) -> Float`                     |
| `ceil`  | `(n: Float) -> Float`                     |
| `round` | `(n: Float) -> Float`                     |
| `sin`   | `(n: Float) -> Float`                     |
| `cos`   | `(n: Float) -> Float`                     |
| `tan`   | `(n: Float) -> Float`                     |
| `PI`    | `Float`                                   |
| `E`     | `Float`                                   |
| `TAU`   | `Float`                                   |

<!-- stdlib:table:math end -->> 整数族取 `Int`，浮点族取 `Float`。传入不匹配的类型会被按 `0` 处理

> （`to_int` / `to_float` 转换失败时回退为 `0`），不会报错——建议依赖类型检查器在编译期拦截。

## 整数函数

### abs

<!-- stdlib:sig:math.abs start -->

```yaoxiang
abs: (n: Int) -> Int
```

<!-- stdlib:sig:math.abs end -->

绝对值。

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

两者中的较大值。

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

两者中的较小值。

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

把 `value` 钳制到 `[min, max]` 区间。

- `value` —— 待钳制的值
- `min` —— 下界（含）
- `max` —— 上界（含）

返回：区间内的值。低于下界返回 `min`，高于上界返回 `max`。

> **`min > max` 会使解释器 panic（#339）**（底层 `i64::clamp` 的前置条件），不会返回错误值。请保证
> `min <= max`。

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.clamp(15, 1, 10) == 10)
    assert(math.clamp(-5, 1, 10) == 1)
    assert(math.clamp(5, 1, 10) == 5)
}
```

## 浮点函数

### fabs

<!-- stdlib:sig:math.fabs start -->

```yaoxiang
fabs: (n: Float) -> Float
```

<!-- stdlib:sig:math.fabs end -->

浮点绝对值。

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

浮点较大值。

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

浮点较小值。

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

`base` 的 `exp` 次幂。

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

平方根。负数返回 `NaN`（不报错）。

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

向下取整，返回仍为 `Float`。

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

向上取整，返回仍为 `Float`。

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

四舍五入（远离零方向取半），返回仍为 `Float`。

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

正弦，参数为**弧度**。

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

余弦，参数为**弧度**。

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

正切，参数为**弧度**。

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.tan(0.0) == 0.0)
}
```

## 相关

- [`std.string.parse_float`](./string#parse_float) —— 字符串解析为 `Float`
