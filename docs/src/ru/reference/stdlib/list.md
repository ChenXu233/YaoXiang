---
title: 'std.list'
description:
  'Добавление и удаление элементов списка, срезы, функции высшего порядка и протокол итераторов'
---

# std.list

Модуль операций со списками. **Особое внимание к семантике перемещения**: существует два вида
функций — те, которые только читают (заимствуют) исходный список, и те, которые потребляют
(перемещают) его. Правила автоматического заимствования для параметров с `&` см. в RFC-009 §2.8:
если аргумент используется после вызова, компилятор автоматически создаёт токен только для чтения.

```yaoxiang
use std.list
```

## Семантическая классификация

Параметры с `&` в сигнатуре заимствуются только для чтения, исходное значение остаётся доступным
после вызова; параметры без `&` передаются по значению, после вызова исходное значение
**перемещено**, повторное использование приведёт к ошибке `E2014`.

| Категория                      | Функции                                                                                                                         | Поведение                                                      |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------- |
| **Потребляет исходный список** | `push` `append` `prepend` `set` `pop` `remove_at`                                                                               | Исходный список перемещён, после этого его нельзя использовать |
| Только чтение (заимствование)  | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce`                | Исходный список можно использовать многократно                 |
| Протокол итерации              | `iter` (потребляет исходный список, возвращает итератор) `has_next` `next` (заимствование / изменяемое заимствование итератора) | См. пояснение ниже                                             |

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // Только чтение (заимствование): nums можно использовать многократно
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))

    // Потребление: base после этого использовать нельзя
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

## Сводная таблица функций

<!-- stdlib:table:list start -->

| Функция      | Сигнатура                                                                     |
| ------------ | ----------------------------------------------------------------------------- |
| `push`       | `(A: Type)(list: List(A), item: A) -> List(A)`                                |
| `pop`        | `(A: Type)(list: &List(A)) -> Any`                                            |
| `append`     | `(A: Type)(list: List(A), item: A) -> List(A)`                                |
| `prepend`    | `(A: Type)(list: List(A), item: A) -> List(A)`                                |
| `remove_at`  | `(A: Type)(list: &List(A), index: Int) -> Any`                                |
| `reverse`    | `(A: Type)(list: &List(A)) -> List(A)`                                        |
| `concat`     | `(A: Type)(a: &List(A), b: &List(A)) -> List(A)`                              |
| `map`        | `(T: Type)(list: &List(T), fn: (item: T) -> T) -> List(T)`                    |
| `filter`     | `(T: Type)(list: &List(T), fn: (item: T) -> Bool) -> List(T)`                 |
| `reduce`     | `(T: Type)(list: &List(T), fn: (acc: Any, item: T) -> Any, init: Any) -> Any` |
| `len`        | `(A: Type)(list: &List(A)) -> Int`                                            |
| `is_empty`   | `(A: Type)(list: &List(A)) -> Bool`                                           |
| `get`        | `(A: Type)(list: &List(A), index: Int) -> Any`                                |
| `set`        | `(A: Type)(list: List(A), index: Int, value: A) -> List(A)`                   |
| `first`      | `(A: Type)(list: &List(A)) -> Any`                                            |
| `last`       | `(A: Type)(list: &List(A)) -> Any`                                            |
| `slice`      | `(A: Type)(list: &List(A), start: Int, end: Int) -> List(A)`                  |
| `contains`   | `(A: Type)(list: &List(A), item: Any) -> Bool`                                |
| `find_index` | `(A: Type)(list: &List(A), item: Any) -> Int`                                 |
| `iter`       | `(A: Type)(list: &List(A)) -> Tuple`                                          |
| `next`       | `(iterator: Tuple) -> Any`                                                    |
| `has_next`   | `(iterator: Tuple) -> Bool`                                                   |

<!-- stdlib:table:list end -->## Функции

### push

<!-- stdlib:sig:list.push start -->

```yaoxiang
push: (A: Type)(list: List(A), item: A) -> List(A)
```

<!-- stdlib:sig:list.push end -->

Возвращает **новый список** с `item`, добавленным в конец `list`. `list` передаётся по значению,
после вызова он **перемещён** и не может быть использован.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

### append

<!-- stdlib:sig:list.append start -->

```yaoxiang
append: (A: Type)(list: List(A), item: A) -> List(A)
```

<!-- stdlib:sig:list.append end -->

Псевдоним `push`, поведение полностью идентично.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    extended = list.append([1, 2], 3)
    assert(list.len(extended) == 3)
}
```

### prepend

<!-- stdlib:sig:list.prepend start -->

```yaoxiang
prepend: (A: Type)(list: List(A), item: A) -> List(A)
```

<!-- stdlib:sig:list.prepend end -->

Возвращает новый список с `item`, вставленным в начало `list`. `list` передаётся по значению, после
вызова он **перемещён**.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = list.prepend([2, 3], 1)
    assert(list.first(l) == 1)
}
```

### pop

<!-- stdlib:sig:list.pop start -->

```yaoxiang
pop: (A: Type)(list: &List(A)) -> Any
```

<!-- stdlib:sig:list.pop end -->

Удаляет и возвращает последний элемент. **Изменяет `list` на месте** — это исключение, когда в
сигнатуре стоит `&`, но значение источника всё же модифицируется.

Возвращает: удалённый элемент; если список пуст — возвращает `Void`, список остаётся пустым.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    mut l = [1, 2, 3]
    gone = list.pop(l)
    assert(list.len(l) == 2)         // укорочен на месте
    assert(gone == 3)

    mut empty = []
    v = list.pop(empty)
    assert(list.is_empty(empty))
}
```

### remove_at

<!-- stdlib:sig:list.remove_at start -->

```yaoxiang
remove_at: (A: Type)(list: &List(A), index: Int) -> Any
```

<!-- stdlib:sig:list.remove_at end -->

Удаляет и возвращает элемент по индексу `index`. **Изменяет `list` на месте**.

- `index` — индекс символа/элемента; по умолчанию `0`

Возвращает: удалённый элемент. Ошибки: при отрицательном индексе или индексе ≥ длины выбрасывается
`E6003` (выход индекса за границы), список не изменяется.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    mut l = [10, 20, 30]
    x = list.remove_at(l, 1)
    assert(x == 20)
    assert(list.len(l) == 2)
}
```

### set

<!-- stdlib:sig:list.set start -->

```yaoxiang
set: (A: Type)(list: List(A), index: Int, value: A) -> List(A)
```

<!-- stdlib:sig:list.set end -->

Возвращает новый список, в котором элемент по индексу `index` заменён на `value`. `list` передаётся
по значению, после вызова он **перемещён**.

- `index` — индекс; по умолчанию `0`
- `value` — новое значение; по умолчанию `Void`

Ошибки: при отрицательном индексе или индексе ≥ длины выбрасывается `E6003` (запись за границы
больше не игнорируется молча).

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = list.set([1, 2, 3], 1, 99)
    assert(list.get(l, 1) == 99)
}
```

### get

<!-- stdlib:sig:list.get start -->

```yaoxiang
get: (A: Type)(list: &List(A), index: Int) -> Any
```

<!-- stdlib:sig:list.get end -->

Читает элемент по индексу `index` (только чтение, `list` можно использовать повторно).

- `index` — индекс; по умолчанию `0`

Возвращает: значение элемента; **при выходе за границы возвращает `Void`** (без ошибки). Ошибки: при
отрицательном индексе выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3, 4]
    assert(list.get(nums, 1) == 2)
}
```

### first

<!-- stdlib:sig:list.first start -->

```yaoxiang
first: (A: Type)(list: &List(A)) -> Any
```

<!-- stdlib:sig:list.first end -->

Возвращает первый элемент; для пустого списка возвращает `Void`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.first([1, 2, 3]) == 1)
}
```

### last

<!-- stdlib:sig:list.last start -->

```yaoxiang
last: (A: Type)(list: &List(A)) -> Any
```

<!-- stdlib:sig:list.last end -->

Возвращает последний элемент; для пустого списка возвращает `Void`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.last([1, 2, 3]) == 3)
}
```

### slice

<!-- stdlib:sig:list.slice start -->

```yaoxiang
slice: (A: Type)(list: &List(A), start: Int, end: Int) -> List(A)
```

<!-- stdlib:sig:list.slice end -->

Берёт подсписок в диапазоне `[start, end)`.

- `start` — начальный индекс; по умолчанию `0`
- `end` — конечный индекс (не включается); по умолчанию — конец списка

Возвращает: новый список. Границы **зажимаются** в допустимый диапазон, ошибка не выбрасывается.
Ошибки: при отрицательных `start` или `end` выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    sub = list.slice([1, 2, 3, 4], 1, 3)
    assert(list.len(sub) == 2)
    assert(list.first(sub) == 2)
}
```

### reverse

<!-- stdlib:sig:list.reverse start -->

```yaoxiang
reverse: (A: Type)(list: &List(A)) -> List(A)
```

<!-- stdlib:sig:list.reverse end -->

Возвращает новый список с элементами в обратном порядке; исходный список не изменяется.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    rev = list.reverse([1, 2, 3])
    assert(list.first(rev) == 3)
}
```

### concat

<!-- stdlib:sig:list.concat start -->

```yaoxiang
concat: (A: Type)(a: &List(A), b: &List(A)) -> List(A)
```

<!-- stdlib:sig:list.concat end -->

Соединяет два списка, возвращает новый список. Оба исходных списка остаются без изменений.

Ошибки: если второй аргумент не является списком, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    joined = list.concat([1, 2], [3, 4])
    assert(list.len(joined) == 4)
}
```

### len

<!-- stdlib:sig:list.len start -->

```yaoxiang
len: (A: Type)(list: &List(A)) -> Int
```

<!-- stdlib:sig:list.len end -->

Количество элементов. Только чтение, `list` можно использовать многократно.

Ошибки: если аргумент не является списком, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)      // можно использовать повторно
}
```

### is_empty

<!-- stdlib:sig:list.is_empty start -->

```yaoxiang
is_empty: (A: Type)(list: &List(A)) -> Bool
```

<!-- stdlib:sig:list.is_empty end -->

Является ли список пустым.

Ошибки: если аргумент не является списком, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.is_empty([]))
    assert(!list.is_empty([1]))
}
```

### contains

<!-- stdlib:sig:list.contains start -->

```yaoxiang
contains: (A: Type)(list: &List(A), item: Any) -> Bool
```

<!-- stdlib:sig:list.contains end -->

Содержится ли `item` в списке (сравнение по равенству значений).

Возвращает: `true`, если элемент найден; `false`, если аргумент не является списком.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3, 4]
    assert(list.contains(nums, 3))
    assert(!list.contains(nums, 99))
}
```

### find_index

<!-- stdlib:sig:list.find_index start -->

```yaoxiang
find_index: (A: Type)(list: &List(A), item: Any) -> Int
```

<!-- stdlib:sig:list.find_index end -->

Индекс первого вхождения `item`.

Возвращает: индекс, если элемент найден; `-1`, если не найден.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    assert(list.find_index([1, 2, 3, 4], 3) == 2)
    assert(list.find_index([1, 2], 99) == -1)
}
```

### map

<!-- stdlib:sig:list.map start -->

```yaoxiang
map: (T: Type)(list: &List(T), fn: (item: T) -> T) -> List(T)
```

<!-- stdlib:sig:list.map end -->

Вызывает `fn` для каждого элемента, возвращает новый список из результатов. Передача
значения-функции имеет **каррированную** форму: `list.map(nums, x => x * 2)`. Исходный список не
изменяется.

Ошибки: если второй аргумент не является функцией, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    doubled = list.map([1, 2, 3], x => x * 2)
    assert(list.get(doubled, 0) == 2)
}
```

### filter

<!-- stdlib:sig:list.filter start -->

```yaoxiang
filter: (T: Type)(list: &List(T), fn: (item: T) -> Bool) -> List(T)
```

<!-- stdlib:sig:list.filter end -->

Оставляет те элементы, для которых `fn` возвращает истину. Исходный список не изменяется.

Ошибки: если второй аргумент не является функцией, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    evens = list.filter([1, 2, 3, 4], x => x % 2 == 0)
    assert(list.len(evens) == 2)
}
```

### reduce

<!-- stdlib:sig:list.reduce start -->

```yaoxiang
reduce: (T: Type)(list: &List(T), fn: (acc: Any, item: T) -> Any, init: Any) -> Any
```

<!-- stdlib:sig:list.reduce end -->

Свёртка слева направо: начиная с `init` в качестве начального значения, последовательно вызывается
`fn(acc, item)`.

- `fn` — функция свёртки `(накопитель, элемент) -> новый накопитель`
- `init` — начальное значение накопителя

Возвращает: конечное значение накопителя. Для пустого списка возвращает `init`.

Ошибки: если второй аргумент не является функцией, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    total = list.reduce([1, 2, 3, 4], (acc, x) => acc + x, 0)
    assert(total == 10)
}
```

### iter

<!-- stdlib:sig:list.iter start -->

```yaoxiang
iter: (A: Type)(list: &List(A)) -> Tuple
```

<!-- stdlib:sig:list.iter end -->

Создаёт итератор. Итератор представляет собой кортеж состояния `(список, индекс)`, после создания он
**последовательно потребляется** в `next`. Исходный список заимствуется только для чтения, во время
итерации его по-прежнему можно использовать.

Возвращает: кортеж-итератор, который передаётся в `next` / `has_next`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))
}
```

### next

<!-- stdlib:sig:list.next start -->

```yaoxiang
next: (iterator: Tuple) -> Any
```

<!-- stdlib:sig:list.next end -->

Извлекает текущий элемент и сдвигает внутренний индекс на одну позицию вперёд.

Возвращает: текущий элемент; `Void` по завершении итерации.

> И `next`, и `has_next` **перемещают** итератор (в сигнатуре нет `&`), поэтому при каждом
> извлечении необходимо заново создавать итератор, либо использовать обход `for ... in`. Это
> отличается от формы заимствования в [`std.range.next`](./range#next).

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([7, 8])
    assert(list.next(it) == 7)
}
```

### has_next

<!-- stdlib:sig:list.has_next start -->

```yaoxiang
has_next: (iterator: Tuple) -> Bool
```

<!-- stdlib:sig:list.has_next end -->

Есть ли ещё не потреблённые элементы.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1])
    assert(list.has_next(it))
}
```

### Обход `for ... in`

Список можно обходить напрямую с помощью `for ... in`, без ручного вызова `next`:

```yaoxiang
use std.assert

main: () -> Void = {
    mut sum = 0
    for x in [1, 2, 3] {
        sum = sum + x
    }
    assert(sum == 6)
}
```

## См. также

- [`std.range`](./range) — итерация по диапазону и ленивые адаптеры
- [`std.assert`](./assert) — утилита утверждений из примеров
