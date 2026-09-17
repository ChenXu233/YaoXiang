---
title: 'std.dict'
description: 'Чтение и запись словаря, представления ключей/значений и слияние'
---

# std.dict

Модуль операций над словарями (`Dict(K, V)`).

```yaoxiang
use std.dict
```

## Семантические категории

| Категория                      | Функции                                                        | Поведение                                                          |
| ------------------------------ | -------------------------------------------------------------- | ------------------------------------------------------------------ |
| Только чтение (заимствование)  | `get` `has` `values` `keys` `entries` `len` `is_empty` `merge` | Исходный словарь можно использовать повторно                       |
| **Поглощает исходный словарь** | `set` `delete`                                                 | Исходный словарь перемещается, после этого использовать его нельзя |

```yaoxiang
use std.assert
use std.dict

main = {
    d = dict.set({}, "a", 1)

    // Только чтение (заимствование): d можно использовать повторно
    assert(dict.get(d, "a") == 1)
    assert(dict.len(d) == 1)
    assert(dict.has(d, "a"))
}
```

## Список функций

<!-- stdlib:table:dict start -->

| Функция    | Сигнатура                                                                  |
| ---------- | -------------------------------------------------------------------------- |
| `get`      | `(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any`                   |
| `set`      | `(K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)` |
| `has`      | `(K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool`                  |
| `values`   | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)`                |
| `keys`     | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)`                |
| `entries`  | `(A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)`                |
| `delete`   | `(K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)`             |
| `len`      | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Int`                             |
| `is_empty` | `(K: Type, V: Type)(dict: &Dict(K, V)) -> Bool`                            |
| `merge`    | `(A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)`         |

<!-- stdlib:table:dict end -->

## Функции

### set

<!-- stdlib:sig:dict.set start -->

```yaoxiang
set: (K: Type, V: Type)(dict: Dict(K, V), key: Any, value: Any) -> Dict(K, V)
```

<!-- stdlib:sig:dict.set end -->

Возвращает **новый словарь** с записью `key` → `value`. `dict` передаётся по значению, после вызова
он **перемещается**.

```yaoxiang
use std.assert
use std.dict

main = {
    d1 = dict.set({}, "a", 1)
    d2 = dict.set(d1, "b", 2)
    assert(dict.len(d2) == 2)
}
```

### get

<!-- stdlib:sig:dict.get start -->

```yaoxiang
get: (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Any
```

<!-- stdlib:sig:dict.get end -->

Получение значения по ключу (только чтение, `dict` можно использовать повторно).

Возвращает: значение, соответствующее ключу. Ошибка: **если ключ отсутствует, выбрасывается
`E6008`** (ключ отсутствует). Перед получением значения можно проверить его наличие с помощью
[`has`](#has).

```yaoxiang
use std.assert
use std.dict

main = {
    d = dict.set({}, "a", 1)
    assert(dict.get(d, "a") == 1)
}
```

Проверка существования перед получением:

```yaoxiang
use std.assert
use std.dict

main = {
    d = dict.set({}, "a", 1)
    assert(!dict.has(d, "nope"))
    if dict.has(d, "a") {
        assert(dict.get(d, "a") == 1)
    }
}
```

### has

<!-- stdlib:sig:dict.has start -->

```yaoxiang
has: (K: Type, V: Type)(dict: &Dict(K, V), key: Any) -> Bool
```

<!-- stdlib:sig:dict.has end -->

Содержится ли `key` в словаре.

Ошибка: если первый аргумент не является словарём, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.dict

main = {
    d = dict.set({}, "a", 1)
    assert(dict.has(d, "a"))
    assert(!dict.has(d, "zzz"))
}
```

### delete

<!-- stdlib:sig:dict.delete start -->

```yaoxiang
delete: (K: Type, V: Type)(dict: Dict(K, V), key: Any) -> Dict(K, V)
```

<!-- stdlib:sig:dict.delete end -->

Возвращает **новый словарь** с удалённым `key`. `dict` передаётся по значению, после вызова он
**перемещается**.

Возвращает: новый словарь. Удаление несуществующего ключа не вызывает ошибки, словарь остаётся без
изменений.

```yaoxiang
use std.assert
use std.dict

main = {
    d = dict.set({}, "a", 1)
    deleted = dict.delete(d, "a")
    assert(!dict.has(deleted, "a"))
}
```

### keys

<!-- stdlib:sig:dict.keys start -->

```yaoxiang
keys: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.keys end -->

Возвращает список всех ключей (только чтение, заимствование).

> Порядок возврата зависит от реализации хеширования и **не гарантируется стабильным**. Если
> требуется упорядоченный вывод, выполните сортировку самостоятельно.

```yaoxiang
use std.assert
use std.dict
use std.list

main = {
    d = dict.set({}, "a", 1)
    ks = dict.keys(d)
    assert(list.len(ks) == 1)
}
```

### values

<!-- stdlib:sig:dict.values start -->

```yaoxiang
values: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.values end -->

Возвращает список всех значений (только чтение, заимствование). Порядок не гарантируется стабильным.

```yaoxiang
use std.assert
use std.dict
use std.list

main = {
    d = dict.set({}, "a", 1)
    vs = dict.values(d)
    assert(list.len(vs) == 1)
}
```

### entries

<!-- stdlib:sig:dict.entries start -->

```yaoxiang
entries: (A: Type, B: Type, C: Type)(dict: &Dict(A, B)) -> List(C)
```

<!-- stdlib:sig:dict.entries end -->

Возвращает список пар ключ-значение, где каждый элемент является кортежем `(key, value)`. Порядок не
гарантируется стабильным.

```yaoxiang
use std.assert
use std.dict
use std.list

main = {
    d = dict.set({}, "a", 1)
    es = dict.entries(d)
    assert(list.len(es) == 1)
}
```

### len

<!-- stdlib:sig:dict.len start -->

```yaoxiang
len: (K: Type, V: Type)(dict: &Dict(K, V)) -> Int
```

<!-- stdlib:sig:dict.len end -->

Количество элементов. Только чтение, `dict` можно использовать повторно.

Ошибка: если аргумент не является словарём, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.dict

main = {
    d = dict.set({}, "a", 1)
    assert(dict.len(d) == 1)
    assert(dict.len(d) == 1)      // можно использовать повторно
}
```

### is_empty

<!-- stdlib:sig:dict.is_empty start -->

```yaoxiang
is_empty: (K: Type, V: Type)(dict: &Dict(K, V)) -> Bool
```

<!-- stdlib:sig:dict.is_empty end -->

Является ли словарь пустым.

Ошибка: если аргумент не является словарём, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.dict

main = {
    assert(dict.is_empty({}))
    d = dict.set({}, "a", 1)
    assert(!dict.is_empty(d))
}
```

### merge

<!-- stdlib:sig:dict.merge start -->

```yaoxiang
merge: (A: Type, B: Type)(a: &Dict(A, B), b: &Dict(A, B)) -> Dict(A, B)
```

<!-- stdlib:sig:dict.merge end -->

Слияние двух словарей, возвращает новый словарь. Оба исходных словаря передаются только для чтения и
не изменяются.

При конфликте ключей **значение из `b` перезаписывает значение из `a`**.

Ошибка: если любой из аргументов не является словарём, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.dict

main = {
    m = dict.merge(dict.set({}, "x", 10), dict.set({}, "y", 20))
    assert(dict.get(m, "x") == 10)
    assert(dict.get(m, "y") == 20)
}
```

## См. также

- [`std.list`](./list) — обработка возвращаемых значений `keys` / `values` / `entries`
- [Справочник по кодам ошибок](../error-code/) — `E6008` ключ отсутствует
