---
title: 'std.range'
description: 'Итерация по диапазонам, предикаты и ленивые адаптеры'
---

# std.range

Итерация по диапазонам (`Range`) и адаптеры.

```yaoxiang
use std.range
```

## Литералы диапазона

| 写法      | 含义                   |
| --------- | ---------------------- |
| `a..b`    | От `a` до `b`, шаг `1` |
| `a..b..s` | От `a` до `b`, шаг `s` |

Диапазон **не включает** конечное значение (полуоткрытый интервал). Шаг может быть отрицательным,
что означает убывание.

## Протокол итератора

[`iter`](#iter) возвращает `Result` — при шаге `0` это путь ошибки (`E6009`), поэтому нужно сначала
вызвать `unwrap` или явно обработать:

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

> **Семантика перемещения**: сигнатуры `has_next` и `next` не содержат `&`, поэтому итератор будет
> **перемещён**. Следовательно, при каждом использовании нужно либо создавать итератор заново, либо
> применять цикл `for ... in`. Это согласуется с итератором [`std.list`](./list).

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    // Создаём итератор для каждого вызова
    a = result.unwrap(range.iter(1..3))
    assert(range.has_next(a))

    b = result.unwrap(range.iter(1..3))
    assert(range.next(b) == 1)
}
```

Для повседневного обхода просто используйте `for ... in`:

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]
    mut sum = 0
    for x in nums {
        sum = sum + x
    }
    assert(sum == 6)
}
```

## Список функций

<!-- stdlib:table:range start -->

| Функция              | Сигнатура                                                     |
| -------------------- | ------------------------------------------------------------- |
| `iter`               | `(r: Range(Int)) -> Result(Iterator(Any), Error)`             |
| `has_next`           | `(it: Iterator(Any)) -> Bool`                                 |
| `next`               | `(it: &Iterator(Any)) -> Any`                                 |
| `contains`           | `(r: Range(Int), x: Int) -> Result(Bool, Error)`              |
| `abort_invalid_step` | `(r: Range(Int)) -> Any`                                      |
| `map`                | `(it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)`       |
| `filter`             | `(it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)`      |
| `collect`            | `(it: Iterator(Any)) -> List(Any)`                            |
| `reduce`             | `(it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any` |
| `for_each`           | `(it: Iterator(Any), f: (Any) -> Void) -> Void`               |

<!-- stdlib:table:range end -->

## Протокол итератора

### iter

<!-- stdlib:sig:range.iter start -->

```yaoxiang
iter: (r: Range(Int)) -> Result(Iterator(Any), Error)
```

<!-- stdlib:sig:range.iter end -->

Создаёт итератор из диапазона.

- `r` — диапазон, например `1..6` или `3..0..-1`

Возвращает: при успехе — `Result.ok(итератор)`; при шаге `0` — `Result.err` с `code` равным `E6009`.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

### has_next

<!-- stdlib:sig:range.has_next start -->

```yaoxiang
has_next: (it: Iterator(Any)) -> Bool
```

<!-- stdlib:sig:range.has_next end -->

Есть ли ещё не использованные элементы.

> **Перемещает** итератор.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.has_next(it))
}
```

### next

<!-- stdlib:sig:range.next start -->

```yaoxiang
next: (it: &Iterator(Any)) -> Any
```

<!-- stdlib:sig:range.next end -->

Извлекает текущий элемент и сдвигает внутренний курсор на одну позицию вперёд.

Возвращает: текущий элемент; по завершении итерации возвращает `Void`.

> **Перемещает** итератор.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    it = result.unwrap(range.iter(1..4))
    assert(range.next(it) == 1)
}
```

Убывающие диапазоны тоже поддерживаются:

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    desc = result.unwrap(range.iter(3..0..-1))
    assert(range.next(desc) == 3)
}
```

### contains

<!-- stdlib:sig:range.contains start -->

```yaoxiang
contains: (r: Range(Int), x: Int) -> Result(Bool, Error)
```

<!-- stdlib:sig:range.contains end -->

Проверяет, попадает ли `x` в диапазон.

- `r` — диапазон
- `x` — проверяемое значение

Возвращает: `Result.ok(Bool)`. Конечное значение образует **открытый** интервал (не включается); при
заданном шаге совпадают только элементы, выровненные по шагу.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    assert(result.unwrap(range.contains(1..10, 5)))
    assert(!result.unwrap(range.contains(1..10, 10)))     // конечное значение не включается

    assert(result.unwrap(range.contains(0..10..2, 4)))    // выровнено по шагу
    assert(!result.unwrap(range.contains(0..10..2, 3)))   // не выровнено
}
```

### abort_invalid_step

<!-- stdlib:sig:range.abort_invalid_step start -->

```yaoxiang
abort_invalid_step: (r: Range(Int)) -> Any
```

<!-- stdlib:sig:range.abort_invalid_step end -->

Хук прерывания при недопустимом шаге. Вызывается, когда `for ... in` встречает диапазон с шагом `0`.

**Всегда** выбрасывает `E6007` с сообщением `Range step must be non-zero (for/in consumption)`.
Обычному коду вызывать его напрямую не нужно.

```yaoxiang
use std.range

main: () -> Void = {
    // При прямом использовании iter вернётся Err, идти через этот хук не нужно
    r = range.iter(1..3)
}
```

## Адаптеры

`map` и `filter` возвращают **ленивые** адаптеры — они не вычисляются немедленно. Результат
получается только после потребления через [`collect`](#collect) / [`reduce`](#reduce) /
[`for_each`](#for_each) / `for ... in`.

### map

<!-- stdlib:sig:range.map start -->

```yaoxiang
map: (it: Iterator(Any), f: (Any) -> Any) -> Iterator(Any)
```

<!-- stdlib:sig:range.map end -->

Применяет `f` к каждому элементу и возвращает новый ленивый итератор.

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    doubled = range.collect(range.map(result.unwrap(range.iter(1..4)), x => x * 2))
    assert(list.len(doubled) == 3)
    assert(list.get(doubled, 0) == 2)
}
```

### filter

<!-- stdlib:sig:range.filter start -->

```yaoxiang
filter: (it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)
```

<!-- stdlib:sig:range.filter end -->

Сохраняет элементы, для которых `p` возвращает истину, и возвращает новый ленивый итератор.

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    big = range.collect(range.filter(result.unwrap(range.iter(1..6)), x => x > 3))
    assert(list.len(big) == 2)
    assert(list.get(big, 0) == 4)
}
```

Адаптеры можно компоновать в цепочку:

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    r = 1..6
    chained = range.collect(range.map(range.filter(result.unwrap(range.iter(r)), x => x % 2 == 0), x => x * 10))
    assert(list.get(chained, 0) == 20)
    assert(list.get(chained, 1) == 40)
}
```

### collect

<!-- stdlib:sig:range.collect start -->

```yaoxiang
collect: (it: Iterator(Any)) -> List(Any)
```

<!-- stdlib:sig:range.collect end -->

Потребляет итератор и собирает все элементы в `List`.

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    xs = range.collect(result.unwrap(range.iter(1..4)))
    assert(list.len(xs) == 3)
}
```

### reduce

<!-- stdlib:sig:range.reduce start -->

```yaoxiang
reduce: (it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any
```

<!-- stdlib:sig:range.reduce end -->

Потребляет итератор и выполняет свёртку.

- `it` — итератор
- `init` — начальное значение аккумулятора
- `f` — функция свёртки `(аккумулятор, элемент) -> новый аккумулятор`

> Обратите внимание, что порядок аргументов отличается от [`std.list.reduce`](./list#reduce): в этом
> модуле — `(итератор, начальное_значение, функция)`, а в `std.list` —
> `(список, функция, начальное_значение)`.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    total = range.reduce(result.unwrap(range.iter(1..6)), 0, (acc, x) => acc + x)
    assert(total == 15)
}
```

### for_each

<!-- stdlib:sig:range.for_each start -->

```yaoxiang
for_each: (it: Iterator(Any), f: (Any) -> Void) -> Void
```

<!-- stdlib:sig:range.for_each end -->

Выполняет `f` для каждого элемента; используется для побочных эффектов.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    // Выводит 1, 2, 3
    range.for_each(result.unwrap(range.iter(1..4)), x => println(x))
    assert(true)
}
```

> В текущей реализации замыкания **не могут** захватывать и изменять внешние `mut`-переменные,
> поэтому использовать `for_each` для накопления не получится (будет выдана ошибка `E1001`) — для
> накопления применяйте [`reduce`](#reduce).

## См. также

- [`std.list`](./list) — список и его итератор
- [`std.result`](./result) — распаковка возвращаемых значений `iter` / `contains`
- [Справочник по кодам ошибок](../error-code/) — `E6009`: недопустимый шаг
