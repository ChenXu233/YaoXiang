---
title: 'std.list'
description:
  'Списки: добавление и удаление элементов, срезы, функции высшего порядка и протокол итераторов'
---

# std.list

Модуль операций над списками. **Особое внимание к семантике перемещения**: существуют два вида
функций — те, которые только заимствуют исходный список для чтения, и те, которые потребляют
(перемещают) исходный список. Правила автоматического заимствования для параметров `&` см. в RFC-009
§2.8: если аргумент используется после вызова, компилятор автоматически создаёт токен только для
чтения.

```yaoxiang
use std.list
```

## Семантическая классификация

Параметры с `&` в сигнатуре являются заимствованиями только для чтения, после вызова исходное
значение остаётся доступным; параметры без `&` передаются по значению, после вызова исходное
значение **перемещено**, его повторное использование приведёт к ошибке `E2014`.

| Категория                       | Функции                                                                                                                         | Поведение                                      |
| ------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------- |
| **Потребляют исходный список**  | `push` `append` `prepend` `set` `pop` `remove_at`                                                                               | Исходный список перемещён, далее непригоден    |
| Заимствование только для чтения | `len` `is_empty` `get` `first` `last` `slice` `reverse` `concat` `contains` `find_index` `map` `filter` `reduce`                | Исходный список можно использовать многократно |
| Протокол итерации               | `iter` (потребляет исходный список, возвращает итератор) `has_next` `next` (заимствование / изменяемое заимствование итератора) | См. пояснения ниже                             |

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 只读借用：nums 可反复使用
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))

    // 消耗：base 在此之后不可再用
    base = [1, 2]
    extended = list.push(base, 3)
    assert(list.len(extended) == 3)
}
```

## Список функций

<!-- stdlib:table:list start -->

| Функция      | Сигнатура                                                                                  |
| ------------ | ------------------------------------------------------------------------------------------ |
| `push`       | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `pop`        | `(A: Type) -> (list: Vec(A)) -> Vec(A)`                                                    |
| `append`     | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `prepend`    | `(A: Type) -> (list: Vec(A), item: A) -> Vec(A)`                                           |
| `remove_at`  | `(A: Type) -> (list: Vec(A), index: Int) -> Vec(A)`                                        |
| `reverse`    | `(A: Type) -> (list: &Vec(A)) -> Vec(A)`                                                   |
| `concat`     | `(A: Type) -> (a: &Vec(A), b: &Vec(A)) -> Vec(A)`                                          |
| `map`        | `(T: Type, R: Type) -> (list: &Vec(T), f: (item: T) -> R) -> Vec(R)`                       |
| `filter`     | `(T: Type) -> (list: &Vec(T), keep: (item: T) -> Bool) -> Vec(T)`                          |
| `reduce`     | `(T: Type, Acc: Type) -> (list: &Vec(T), f: (acc: Acc, item: T) -> Acc, init: Acc) -> Acc` |
| `len`        | `(A: Type) -> (list: &Vec(A)) -> Int`                                                      |
| `is_empty`   | `(A: Type) -> (list: &Vec(A)) -> Bool`                                                     |
| `get`        | `(A: Type) -> (list: &Vec(A), index: Int) -> A`                                            |
| `set`        | `(A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)`                              |
| `first`      | `(A: Type) -> (list: &Vec(A)) -> A`                                                        |
| `last`       | `(A: Type) -> (list: &Vec(A)) -> A`                                                        |
| `slice`      | `(A: Type) -> (list: &Vec(A), start: Int, end: Int) -> Vec(A)`                             |
| `contains`   | `(A: Type) -> (list: &Vec(A), item: A) -> Bool`                                            |
| `find_index` | `(A: Type) -> (list: &Vec(A), item: A) -> Int`                                             |
| `iter`       | `(T: Type) -> (list: Vec(T)) -> Iter(T)`                                                   |
| `next`       | `(T: Type) -> (it: &mut Iter(T)) -> T`                                                     |
| `has_next`   | `(T: Type) -> (it: &Iter(T)) -> Bool`                                                      |
| `empty`      | `(T: Type) -> Vec(T)`                                                                      |
| `of`         | `(T: Type) -> (data: Vec(T)) -> Vec(T)`                                                    |

<!-- stdlib:table:list end -->

## Функции

### push

<!-- stdlib:sig:list.push start -->

```yaoxiang
push: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
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
append: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
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
prepend: (A: Type) -> (list: Vec(A), item: A) -> Vec(A)
```

<!-- stdlib:sig:list.prepend end -->

Возвращает новый список с `item`, вставленным в начало `list`. `list` передаётся по значению, после
вызова **перемещён**.

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
pop: (A: Type) -> (list: Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.pop end -->

Удаляет последний элемент и возвращает **укороченный список** (семантика значений). Исходный список
потребляется, и это уже не то исключение, что в нативной версии с «сигнатурой `&`, но значением,
изменённым на месте».

Возвращает: новый список без последнего элемента; если список пуст, возвращается как есть. Чтобы
прочитать удалённый элемент, перед вызовом извлеките значение через `last`.

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [1, 2, 3]
    rest = list.pop(l)               // l 被消费，rest 是缩短后的新列表
    assert(list.len(rest) == 2)
    assert(list.last(rest) == 2)     // 末元素 3 已被移除

    // 要读取被移除的元素，先用 last 取值再 pop
    l2 = [1, 2, 3]
    removed = list.last(l2)
    assert(removed == 3)

    empty = list.empty(Int)
    assert(list.is_empty(list.pop(empty)))
}
```

### remove_at

<!-- stdlib:sig:list.remove_at start -->

```yaoxiang
remove_at: (A: Type) -> (list: Vec(A), index: Int) -> Vec(A)
```

<!-- stdlib:sig:list.remove_at end -->

Удаляет элемент по индексу `index` и возвращает **новый укороченный список** (семантика значений).
Исходный список потребляется.

- `index` — индекс элемента

Возвращает: новый список без этого элемента. Ошибки: при отрицательном индексе или индексе ≥ длины
выбрасывается `E6003` (выход за границы).

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    l = [10, 20, 30]
    got = list.remove_at(l, 1)
    assert(list.len(got) == 2)
    assert(list.get(got, 0) == 10)
    assert(list.get(got, 1) == 30)
}
```

### set

<!-- stdlib:sig:list.set start -->

```yaoxiang
set: (A: Type) -> (list: Vec(A), index: Int, value: A) -> Vec(A)
```

<!-- stdlib:sig:list.set end -->

Возвращает новый список, в котором элемент по индексу `index` заменён на `value`. `list` передаётся
по значению, после вызова **перемещён**.

- `index` — индекс; по умолчанию `0`
- `value` — новое значение; по умолчанию `Void`

Ошибки: при отрицательном индексе или индексе ≥ длины выбрасывается `E6003` (запись за пределы более
не игнорируется молча).

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
get: (A: Type) -> (list: &Vec(A), index: Int) -> A
```

<!-- stdlib:sig:list.get end -->

Читает элемент по индексу `index` (только чтение, `list` можно использовать повторно).

- `index` — индекс; по умолчанию `0`

Возвращает: значение элемента; **при выходе за границы возвращается `Void`** (без ошибки). Ошибки:
при отрицательном индексе выбрасывается `E6007`.

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
first: (A: Type) -> (list: &Vec(A)) -> A
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
last: (A: Type) -> (list: &Vec(A)) -> A
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
slice: (A: Type) -> (list: &Vec(A), start: Int, end: Int) -> Vec(A)
```

<!-- stdlib:sig:list.slice end -->

Берёт подсписок в диапазоне `[start, end)`.

- `start` — начальный индекс; по умолчанию `0`
- `end` — конечный индекс (не включается); по умолчанию — до конца списка

Возвращает: новый список. Границы **зажимаются** в допустимый диапазон, ошибка не выбрасывается.
Ошибки: при отрицательном `start` или `end` выбрасывается `E6007`.

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
reverse: (A: Type) -> (list: &Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.reverse end -->

Возвращает новый список с обратным порядком элементов; исходный список не изменяется.

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
concat: (A: Type) -> (a: &Vec(A), b: &Vec(A)) -> Vec(A)
```

<!-- stdlib:sig:list.concat end -->

Соединяет два списка и возвращает новый. Оба исходных списка остаются без изменений.

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
len: (A: Type) -> (list: &Vec(A)) -> Int
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
    assert(list.len(nums) == 3)      // 可复用
}
```

### is_empty

<!-- stdlib:sig:list.is_empty start -->

```yaoxiang
is_empty: (A: Type) -> (list: &Vec(A)) -> Bool
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
contains: (A: Type) -> (list: &Vec(A), item: A) -> Bool
```

<!-- stdlib:sig:list.contains end -->

Содержится ли `item` в списке (сравнение по равенству значений).

Возвращает: `true`, если элемент присутствует; если аргумент не является списком, возвращает
`false`.

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
find_index: (A: Type) -> (list: &Vec(A), item: A) -> Int
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
map: (T: Type, R: Type) -> (list: &Vec(T), f: (item: T) -> R) -> Vec(R)
```

<!-- stdlib:sig:list.map end -->

Для каждого элемента вызывает `fn` и возвращает новый список из результатов. Передача значения
функции имеет **каррированную** форму: `list.map(nums, x => x * 2)`. Исходный список не изменяется.

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
filter: (T: Type) -> (list: &Vec(T), keep: (item: T) -> Bool) -> Vec(T)
```

<!-- stdlib:sig:list.filter end -->

Оставляет элементы, для которых `fn` возвращает истину. Исходный список не изменяется.

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
reduce: (T: Type, Acc: Type) -> (list: &Vec(T), f: (acc: Acc, item: T) -> Acc, init: Acc) -> Acc
```

<!-- stdlib:sig:list.reduce end -->

Свёртка слева направо: начиная с `init`, последовательно вызывается `fn(acc, item)`.

- `fn` — функция свёртки `(аккумулятор, элемент) -> новый аккумулятор`
- `init` — начальное значение аккумулятора

Возвращает: итоговое значение аккумулятора. Для пустого списка возвращает `init`.

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
iter: (T: Type) -> (list: Vec(T)) -> Iter(T)
```

<!-- stdlib:sig:list.iter end -->

Создаёт итератор. Итератор — это носитель состояния в виде кортежа `(список, индекс)`, после
создания он **последовательно потребляется** в `next`. Исходный список заимствуется только для
чтения и остаётся доступным во время итерации.

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
next: (T: Type) -> (it: &mut Iter(T)) -> T
```

<!-- stdlib:sig:list.next end -->

Извлекает текущий элемент и сдвигает внутренний индекс на единицу вперёд.

Возвращает: текущий элемент; по завершении итерации — `Void`.

> `next` и `has_next` **перемещают** итератор (в сигнатуре нет `&`), поэтому при каждом извлечении
> нужно заново создавать итератор или же напрямую использовать цикл `for ... in`. Это отличается от
> заимствующей формы [`std.range.next`](./range#next).

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
has_next: (T: Type) -> (it: &Iter(T)) -> Bool
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

### Обход через `for ... in`

Список можно обходить напрямую через `for ... in` без ручного вызова `next`:

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

## Связанные разделы

- [`std.range`](./range) — итерация по диапазонам и ленивые адаптеры
- [`std.assert`](./assert) — утилита утверждений в примерах
