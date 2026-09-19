---
title: 'std.math'
description: '整数、浮動小数点、三角関数。PI/E/TAU 定数を含む'
---

# std.math

数学モジュール。すべて**純粋な値関数**：引数は値渡し（Copy セマンティクス）で渡され、借用・ムーブ・副作用はありません。

```yaoxiang
use std.math
```

## 定数

名前でインポートすれば使用できます。

```yaoxiang
use std.math.{PI, E, TAU}
```

| 定数  | 型      | 値                     |
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

## 関数一覧

<!-- stdlib:table:math start -->

| 関数    | シグネチャ                                |
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

<!-- stdlib:table:math end -->> 整数族は `Int`、浮動小数点族は `Float` を取ります。一致しない型を渡すと `0` として扱われます

> （`to_int` / `to_float` 変換に失敗した場合は `0`
> にフォールバック）。エラーは発生しません。型チェッカーを使用してコンパイル時に検出することをお勧めします。

## 整数関数

### abs

<!-- stdlib:sig:math.abs start -->

```yaoxiang
abs: (n: Int) -> Int
```

<!-- stdlib:sig:math.abs end -->

絶対値。

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

2 つの値のうち大きい方。

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

2 つの値のうち小さい方。

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

`value` を `[min, max]` 区間にクランプします。

- `value` —— クランプ対象の値
- `min` —— 下限（含）
- `max` —— 上限（含）

戻り値：区間内の値。下限未満は `min`、上限超過は `max` を返します。

> **`min > max` の場合、インタプリタが panic します（#339）**（下層の `i64::clamp`
> の前提条件）。エラー値は返されません。`min <= max` となるようにしてください。

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.clamp(15, 1, 10) == 10)
    assert(math.clamp(-5, 1, 10) == 1)
    assert(math.clamp(5, 1, 10) == 5)
}
```

## 浮動小数点関数

### fabs

<!-- stdlib:sig:math.fabs start -->

```yaoxiang
fabs: (n: Float) -> Float
```

<!-- stdlib:sig:math.fabs end -->

浮動小数点の絶対値。

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

浮動小数点のうち大きい方。

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

浮動小数点のうち小さい方。

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

`base` の `exp` 乗。

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

平方根。負数を渡すと `NaN` を返します（エラーは発生しません）。

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

切り捨て。戻り値は `Float` のままです。

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

切り上げ。戻り値は `Float` のままです。

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

四捨五入（ゼロから遠い方向で丸める）。戻り値は `Float` のままです。

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

正弦。引数は**ラジアン**です。

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

余弦。引数は**ラジアン**です。

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

正接。引数は**ラジアン**です。

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.tan(0.0) == 0.0)
}
```

## 関連

- [`std.string.parse_float`](./string#parse_float) —— 文字列を `Float` にパースする
