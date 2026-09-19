---
title: 'std.range'
description: 'Итерация диапазонов, предикаты и ленивые адаптеры'
---

# std.range

Итерация по диапазонам (`Range`) и адаптеры.

```yaoxiang
use std.range
```

## Литералы диапазонов

| Запись    | Значение                  |
| --------- | ------------------------- |
| `a..b`    | от `a` до `b` с шагом `1` |
| `a..b..s` | от `a` до `b` с шагом `s` |

Диапазон **не включает конечное значение** (полуоткрытый — левая граница включается, правая — нет).
Шаг может быть отрицательным, что означает убывание.

## Протокол итератора

[`iter`](#iter) возвращает `Result` — при шаге `0` это ошибочный путь (`E6009`), поэтому сначала
нужно вызвать `unwrap` или явно обработать результат:

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

> **Семантика перемещения**: сигнатуры `has_next` и `next` не принимают `&`, поэтому они
> **перемещают** итератор. Следовательно, при каждом использовании нужно создавать итератор заново,
> либо сразу обходить его через `for ... in`. Это соответствует поведению итератора в
> [`std.list`](./list).

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    // Создаём итератор заново для каждого использования
    a = result.unwrap(range.iter(1..3))
    assert(range.has_next(a))

    b = result.unwrap(range.iter(1..3))
    assert(range.next(b) == 1)
}
```

Для повседневного обхода используйте `for ... in`:

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
| `collect`            | `(it: Iterator(Any)) -> Vec(Any)`                             |
| `reduce`             | `(it: Iterator(Any), init: Any, f: (Any, Any) -> Any) -> Any` |
| `for_each`           | `(it: Iterator(Any), f: (Any) -> Void) -> Void`               |

<!-- stdlib:table:range end -->## Протокол итератора

### iter

<!-- stdlib:sig:range.iter start -->

```yaoxiang
iter: (r: Range(Int)) -> Result(Iterator(Any), Error)
```

<!-- stdlib:sig:range.iter end -->

Создаёт итератор из диапазона.

- `r` — диапазон, например `1..6` или `3..0..-1`

Возвращает: в случае успеха — `Result.ok(итератор)`; при шаге `0` — `Result.err` с `code` равным
`E6009`.

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

Есть ли ещё необработанные элементы.

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

Возвращает: текущий элемент; по завершении итерации — `Void`.

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

Убывающие диапазоны также поддерживаются:

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

Возвращает: `Result.ok(Bool)`. Конечное значение не включается (открытый интервал); при заданном
шаге сопоставляются только элементы, выровненные по шагу.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    assert(result.unwrap(range.contains(1..10, 5)))
    assert(!result.unwrap(range.contains(1..10, 10)))     // Конечное значение не включается

    assert(result.unwrap(range.contains(0..10..2, 4)))    // Выровнено по шагу
    assert(!result.unwrap(range.contains(0..10..2, 3)))   // Не выровнено
}
```

### abort_invalid_step

<!-- stdlib:sig:range.abort_invalid_step start -->

```yaoxiang
abort_invalid_step: (r: Range(Int)) -> Any
```

<!-- stdlib:sig:range.abort_invalid_step end -->

Обработчик прерывания при недопустимом шаге, вызываемый при потреблении `for ... in` диапазона с
шагом `0`.

**Всегда** выбрасывает `E6007` с сообщением `Range step must be non-zero (for/in consumption)`. В
обычном коде прямой вызов не требуется.

```yaoxiang
use std.range

main: () -> Void = {
    // Прямое использование iter вернёт Err, обращаться к этому хуку не нужно
    r = range.iter(1..3)
}
```

## Адаптеры

`map` и `filter` возвращают **ленивые** адаптеры — они не вычисляются немедленно, а формируют
результат только после потребления через [`collect`](#collect) / [`reduce`](#reduce) /
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
    assert(doubled.length == 3)
    assert(doubled[0] == 2)
}
```

### filter

<!-- stdlib:sig:range.filter start -->

```yaoxiang
filter: (it: Iterator(Any), p: (Any) -> Bool) -> Iterator(Any)
```

<!-- stdlib:sig:range.filter end -->

Оставляет элементы, для которых `p` истинно, и возвращает новый ленивый итератор.

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    big = range.collect(range.filter(result.unwrap(range.iter(1..6)), x => x > 3))
    assert(big.length == 2)
    assert(big[0] == 4)
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
    // Семантика значений: `chained` потребляется при чтении по индексу, элементы последовательно связываются с локальными переменными
    first = chained[0]
    second = chained[1]
    assert(first == 20)
    assert(second == 40)
}
```

### collect

<!-- stdlib:sig:range.collect start -->

```yaoxiang
collect: (it: Iterator(Any)) -> Vec(Any)
```

<!-- stdlib:sig:range.collect end -->

Потребляет итератор, собирая все элементы в `List`.

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

> Обратите внимание, что порядок аргументов отличается от [`std.list.reduce`](./list#reduce): в
> данном модуле это `(итератор, начальное_значение, функция)`, тогда как в `std.list` —
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

Выполняет `f` для каждого элемента. Используется для побочных эффектов.

```yaoxiang
use std.assert
use std.range
use std.result

main: () -> Void = {
    // Выведет 1, 2, 3
    range.for_each(result.unwrap(range.iter(1..4)), x => println(x))
    assert(true)
}
```

> В настоящее время замыкания **не могут захватывать и изменять** внешние переменные `mut`, поэтому
> использовать `for_each` для накопления не получится (будет выдана ошибка `E1001`) — для накопления
> используйте [`reduce`](#reduce).

## Связанные модули

- [`std.list`](./list) — списки и их итераторы
- [`std.result`](./result) — распаковка возвращаемых значений `iter` / `contains`
- [Справочник по кодам ошибок](../error-code/) — `E6009` недопустимый шаг
