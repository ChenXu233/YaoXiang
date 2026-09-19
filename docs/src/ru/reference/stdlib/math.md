---
title: 'std.math'
description: 'Целочисленные, вещественные и тригонометрические функции, включая константы PI/E/TAU'
---

# std.math

Математический модуль. Все функции **чистые по значению**: аргументы передаются по значению
(Copy-семантика), без заимствования, без перемещения, без побочных эффектов.

```yaoxiang
use std.math
```

## Константы

Импортируются по имени:

```yaoxiang
use std.math.{PI, E, TAU}
```

| Константа | Тип     | Значение               |
| --------- | ------- | ---------------------- |
| `PI`      | `Float` | π ≈ 3.141592653589793  |
| `E`       | `Float` | e ≈ 2.718281828459045  |
| `TAU`     | `Float` | 2π ≈ 6.283185307179586 |

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14 and PI < 3.15)
    assert(E > 2.71 and E < 2.72)
    assert(TAU > 6.28 and TAU < 6.29)
}
```

## Список функций

<!-- stdlib:table:math start -->

| Функция | Сигнатура                                 |
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

<!-- stdlib:table:math end -->Семейство целочисленных функций принимает `Int`, семейство вещественных — `Float`. При передаче значения несоответствующего типа оно приводится к `0` (если преобразование `to_int` / `to_float` завершается неудачей — выполняется откат к `0`); ошибка при этом не возникает — рекомендуется полагаться на проверку типов во время компиляции.

## Целочисленные функции

### abs

<!-- stdlib:sig:math.abs start -->

```yaoxiang
abs: (n: Int) -> Int
```

<!-- stdlib:sig:math.abs end -->

Абсолютное значение.

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

Большее из двух значений.

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

Меньшее из двух значений.

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

Ограничивает `value` диапазоном `[min, max]`.

- `value` — ограничиваемое значение
- `min` — нижняя граница (включительно)
- `max` — верхняя граница (включительно)

Возвращает: значение, попадающее в диапазон. Если значение меньше нижней границы, возвращается
`min`; если больше верхней — возвращается `max`.

> **`min > max` приводит к панике интерпретатора (#339)** (предусловие базового `i64::clamp`), а не
> к возврату ошибочного значения. Пожалуйста, гарантируйте, что `min <= max`.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.clamp(15, 1, 10) == 10)
    assert(math.clamp(-5, 1, 10) == 1)
    assert(math.clamp(5, 1, 10) == 5)
}
```

## Вещественные функции

### fabs

<!-- stdlib:sig:math.fabs start -->

```yaoxiang
fabs: (n: Float) -> Float
```

<!-- stdlib:sig:math.fabs end -->

Абсолютное значение вещественного числа.

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

Большее из двух вещественных значений.

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

Меньшее из двух вещественных значений.

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

Возведение `base` в степень `exp`.

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

Квадратный корень. Для отрицательных чисел возвращается `NaN` (без ошибки).

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

Округление вниз, результат остаётся `Float`.

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

Округление вверх, результат остаётся `Float`.

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

Округление до ближайшего (половинки округляются в направлении от нуля), результат остаётся `Float`.

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

Синус, аргумент задаётся в **радианах**.

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

Косинус, аргумент задаётся в **радианах**.

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

Тангенс, аргумент задаётся в **радианах**.

```yaoxiang
use std.assert
use std.math

main: () -> Void = {
    assert(math.tan(0.0) == 0.0)
}
```

## Связанные материалы

- [`std.string.parse_float`](./string#parse_float) — разбор строки в `Float`
