---
title: 'std.result'
description: 'Конструирование и распаковка Result и Error'
---

# std.result

Распаковка `Result(T, E)` и доступ к полям носителя `Error`. Сам `Result` — это запись-подобный
суммарный тип (RFC-010), экспортируемый из `std.result`: конструирование выполняется **синтаксисом
конструирования вариантов**, деструктуризация — с помощью `match` и шаблонов вариантов, а
распространение `?` управляется интерфейсом `Try` (см. ниже).

```yaoxiang
use std.result

r = Result(Int, String).ok(5)
e = Result(Int, String).err("boom")
```

## Представление во время выполнения

| Значение                  | Представление                        |
| ------------------------- | ------------------------------------ |
| `Result(T, E).ok(value)`  | вариант перечисления, несёт `value`  |
| `Result(T, E).err(error)` | вариант перечисления, несёт `error`  |
| `Error`                   | структура с полями `(code, message)` |

`Error.code` — это стабильный межверсионный код из сегментов `E6xxx` / `E7xxx` согласно RFC-013,
`Error.message` — человекочитаемое описание.

## Интерфейс Try (распространение `?`)

`Result` инстанцирует в теле типа интерфейс `Try(Result(T, E), T, E)` с четырьмя методами, которыми
управляет оператор `?`: `is_failure` определяет неудачу, `success` извлекает полезную нагрузку
успеха, `residual` — полезную нагрузку неудачи, `from_error` восстанавливает `Result` из значения
ошибки. Эти методы также могут вызываться явно.

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

<!-- stdlib:table:result end -->

## Проверка

### is_ok

<!-- stdlib:sig:result.is_ok start -->

```yaoxiang
is_ok: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_ok end -->

Является ли значение успешным вариантом. Только-для-чтения заимствование, `self` может
использоваться повторно.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = Result(Int, String).ok(1)
    assert(result.is_ok(r))
    assert(result.is_ok(r))      // можно повторно использовать
}
```

### is_err

<!-- stdlib:sig:result.is_err start -->

```yaoxiang
is_err: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_err end -->

Является ли значение вариантом ошибки. Только-для-чтения заимствование.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = Result(Int, String).err("e")
    assert(result.is_err(r))
}
```

## Извлечение

### unwrap

<!-- stdlib:sig:result.unwrap start -->

```yaoxiang
unwrap: (T: Type, E: Type)(self: &Result(T, E)) -> T
```

<!-- stdlib:sig:result.unwrap end -->

Извлекает значение успеха.

Возврат: значение, которое несёт вариант `Ok`. Ошибка: при вызове на значении `Err` выбрасывает
`E6007`, при этом **содержит исходный код ошибки и описание**, вида
`unwrap called on Err value (E6010: parse_int: ...)`, поэтому можно увидеть причину неудачи без
необходимости сначала вызывать `unwrap_err`.

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

Извлекает значение успеха или возвращает `default` в случае `Err`.

- `default` — резервное значение на случай `Err`

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

Возврат: значение, которое несёт вариант `Err`. Ошибка: при вызове на значении `Ok` выбрасывает
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

Читает строку кода ошибки, например `"E6010"`.

> Тип в сигнатуре — `Error`, но носитель ошибки во время выполнения — структура с полями
> `(code, message)`. Вызывается непосредственно на значении `Error`.

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

Читает текст описания ошибки.

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

## Связанные

- [`std.string`](./string#parse_int) — функция разбора, возвращающая `Result`
- [Справочник по кодам ошибок](../error-code/) — коды ошибок времени выполнения, такие как `E6010` /
  `E6011`
