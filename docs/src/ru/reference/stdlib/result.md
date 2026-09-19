---
title: 'std.result'
description: 'Конструирование и распаковка Result и Error'
---

# std.result

Конструирование и распаковка `Result(T, E)`, а также доступ к полям носителя `Error`.

```yaoxiang
use std.result
```

## Представление во время выполнения

| Значение            | Представление                         |
| ------------------- | ------------------------------------- |
| `Result.ok(value)`  | вариант перечисления, несущий `value` |
| `Result.err(error)` | вариант перечисления, несущий `error` |
| `Error`             | структура с полями `(code, message)`  |

`Error.code` — это регистрационный код из сегментов `E6xxx` / `E7xxx` согласно RFC-013 (стабильный
контракт между версиями), а `Error.message` — человекочитаемое описание.

## Обзор функций

<!-- stdlib:table:result start -->

| Функция      | Сигнатура                                                  |
| ------------ | ---------------------------------------------------------- |
| `is_ok`      | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `is_err`     | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `unwrap`     | `(T: Type, E: Type)(self: &Result(T, E)) -> T`             |
| `unwrap_or`  | `(T: Type, E: Type)(self: &Result(T, E), default: T) -> T` |
| `ok`         | `(T: Type, E: Type)(value: T) -> Result(T, E)`             |
| `err`        | `(T: Type, E: Type)(error: E) -> Result(T, E)`             |
| `unwrap_err` | `(T: Type, E: Type)(self: &Result(T, E)) -> E`             |
| `code`       | `(self: &Error) -> String`                                 |
| `message`    | `(self: &Error) -> String`                                 |

<!-- stdlib:table:result end -->

## Конструирование

### ok

<!-- stdlib:sig:result.ok start -->

```yaoxiang
ok: (T: Type, E: Type)(value: T) -> Result(T, E)
```

<!-- stdlib:sig:result.ok end -->

Оборачивает успешное значение.

Значение `Ok`, извлечённое через `?`, необходимо повторно обернуть, чтобы продолжить распространение
по типу возврата `Result`; `ok` как раз и является этим обёртывателем.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.ok(42)
    assert(result.is_ok(r))
}
```

### err

<!-- stdlib:sig:result.err start -->

```yaoxiang
err: (T: Type, E: Type)(error: E) -> Result(T, E)
```

<!-- stdlib:sig:result.err end -->

Оборачивает значение ошибки.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.err("boom")
    assert(result.is_err(r))
}
```

## Проверка

### is_ok

<!-- stdlib:sig:result.is_ok start -->

```yaoxiang
is_ok: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_ok end -->

Является ли вариантом успеха. Неизменяемое заимствование, `self` можно использовать повторно.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.ok(1)
    assert(result.is_ok(r))
    assert(result.is_ok(r))      // можно использовать повторно
}
```

### is_err

<!-- stdlib:sig:result.is_err start -->

```yaoxiang
is_err: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_err end -->

Является ли вариантом ошибки. Неизменяемое заимствование.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.err("e")
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

Извлекает успешное значение.

Возврат: значение, которое несёт вариант `Ok`. Ошибка: при вызове на значении `Err` выбрасывает
`E6007`, при этом сообщение **содержит исходный код ошибки и описание**, в форме наподобие
`unwrap called on Err value (E6010: parse_int: ...)`, поэтому нет необходимости сначала вызывать
`unwrap_err`, чтобы увидеть причину сбоя.

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

Извлекает успешное значение или возвращает `default` в случае `Err`.

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

> Тип в сигнатуре — `Error`, но носитель ошибки во время выполнения — это структура с полями
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

## См. также

- [`std.string`](./string#parse_int) — функции разбора, возвращающие `Result`
- [Справочник по кодам ошибок](../error-code/) — коды ошибок времени выполнения, такие как `E6010` /
  `E6011`
