---
title: 'std.result'
description: 'Конструирование и распаковка Result и Error'
---

# std.result

Распаковка `Result(T, E)` и доступ к полям носителя `Error`. Сам `Result` представляет собой запись
и перечисление, экспортируемые из `std.result` (RFC-010): конструирование выполняется с помощью
**синтаксиса конструирования вариантов**, деконструкция — с помощью `match` по образцам вариантов,
распространение через `?` управляется интерфейсом `Try` (см. ниже).

```yaoxiang
use std.result

r = Result(Int, String).ok(5)
e = Result(Int, String).err("boom")
```

## Представление во время выполнения

| Значение                  | Представление                         |
| ------------------------- | ------------------------------------- |
| `Result(T, E).ok(value)`  | вариант перечисления, несущий `value` |
| `Result(T, E).err(error)` | вариант перечисления, несущий `error` |
| `Error`                   | структура с полями `(code, message)`  |

`Error.code` — это зарегистрированный код из сегментов `E6xxx` / `E7xxx` согласно RFC-013
(стабильный контракт между версиями), а `Error.message` — человекочитаемое описание.

## Интерфейс Try (распространение через `?`)

`Result` реализует в теле типа интерфейс `Try(Result(T, E), T, E)` с четырьмя методами. Оператор `?`
руководствуется ими: `is_failure` определяет неуспех, `success` извлекает полезную нагрузку успеха,
`residual` — нагрузку неудачи, `from_error` воссоздаёт `Result` из значения ошибки. Эти методы также
можно вызывать явно.

## Список функций

<!-- stdlib:table:result start -->

| Функция      | Сигнатура                                                  |
| ------------ | ---------------------------------------------------------- |
| `is_ok`      | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `is_err`     | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `unwrap`     | `(T: Type, E: Type)(self: &Result(T, E)) -> T`             |
| `unwrap_or`  | `(T: Type, E: Type)(self: &Result(T, E), default: T) -> T` |
| `unwrap_err` | `(T: Type, E: Type)(self: &Result(T, E)) -> E`             |
| `code`       | `(self: &Error) -> String`                                 |
| `message`    | `(self: &Error) -> String`                                 |
| `error`      | `(code: &String, message: &String) -> Error`               |

<!-- stdlib:table:result end -->## Проверка

### is_ok

<!-- stdlib:sig:result.is_ok start -->

```yaoxiang
is_ok: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_ok end -->

Является ли значение вариантом успеха. Только чтение по ссылке, `self` можно использовать
многократно.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = Result(Int, String).ok(1)
    assert(result.is_ok(r))
    assert(result.is_ok(r))      // допускает повторное использование
}
```

### is_err

<!-- stdlib:sig:result.is_err start -->

```yaoxiang
is_err: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_err end -->

Является ли значение вариантом ошибки. Только чтение по ссылке.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = Result(Int, String).err("e")
    assert(result.is_err(r))
}
```

## Извлечение значений

### unwrap

<!-- stdlib:sig:result.unwrap start -->

```yaoxiang
unwrap: (T: Type, E: Type)(self: &Result(T, E)) -> T
```

<!-- stdlib:sig:result.unwrap end -->

Извлекает значение успеха.

Возвращает: значение, которое несёт вариант `Ok`. Ошибка: при вызове на значении `Err` выбрасывает
`E6007`, при этом в сообщении **содержится исходный код ошибки и её описание**, например
`unwrap called on Err value (E6010: parse_int: ...)`, поэтому можно увидеть причину неудачи без
предварительного вызова `unwrap_err`.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("42")
    assert(result.unwrap(r) == 42)
}
```

### unwrap_or

<!-- stdlib:sig:result.unwrap_or start -->

```yaoxiang
unwrap_or: (T: Type, E: Type)(self: &Result(T, E), default: T) -> T
```

<!-- stdlib:sig:result.unwrap_or end -->

Извлекает значение успеха или возвращает `default`, если получено `Err`.

- `default` — резервное значение в случае `Err`

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    good = string.parse_int("42")
    assert(result.unwrap_or(good, 0) == 42)

    bad = string.parse_int("abc")
    assert(result.unwrap_or(bad, 0) == 0)
}
```

### unwrap_err

<!-- stdlib:sig:result.unwrap_err start -->

```yaoxiang
unwrap_err: (T: Type, E: Type)(self: &Result(T, E)) -> E
```

<!-- stdlib:sig:result.unwrap_err end -->

Извлекает значение ошибки.

Возвращает: значение, которое несёт вариант `Err`. Ошибка: при вызове на значении `Ok` выбрасывает
`E6007`.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(result.is_err(r))
}
```

## Поля Error

### code

<!-- stdlib:sig:result.code start -->

```yaoxiang
code: (self: &Error) -> String
```

<!-- stdlib:sig:result.code end -->

Считывает строку кода ошибки, например `"E6010"`.

> Сигнатурный тип — `Error`, но носитель ошибки во время выполнения представляет собой структуру с
> полями `(code, message)`. Вызывается непосредственно на значении `Error`.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(result.code(e) == "E6010")
}
```

### message

<!-- stdlib:sig:result.message start -->

```yaoxiang
message: (self: &Error) -> String
```

<!-- stdlib:sig:result.message end -->

Считывает текст описания ошибки.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(string.len(result.message(e)) > 0)
}
```

## Связанные материалы

- [`std.string`](./string#parse_int) — функция разбора, возвращающая `Result`
- [Справочник по кодам ошибок](../error-code/) — коды ошибок времени выполнения, такие как `E6010` /
  `E6011`
