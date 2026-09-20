---
title: 'std.result'
description: 'Создание и распаковка Result и Error'
---

# std.result

Создание и распаковка `Result(T, E)`, а также доступ к полям носителя `Error`.

```yaoxiang
use std.result
```

## Представление во время выполнения

| Значение            | Представление                                  |
| ------------------- | ---------------------------------------------- |
| `Result.ok(value)`  | значение-вариант перечисления, несущий `value` |
| `Result.err(error)` | значение-вариант перечисления, несущий `error` |
| `Error`             | структура с полями `(code, message)`           |

`Error.code` — это регистрационный код сегмента `E6xxx` / `E7xxx` из RFC-013 (стабильный контракт
между версиями), `Error.message` — удобочитаемое описание для человека.

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

## Создание

### ok

<!-- stdlib:sig:result.ok start -->

```yaoxiang
ok: (T: Type, E: Type)(value: T) -> Result(T, E)
```

<!-- stdlib:sig:result.ok end -->

Оборачивает успешное значение.

Значение `Ok`, распакованное через `?`, необходимо обернуть заново, чтобы продолжить распространение
по типу возврата `Result`; `ok` — именно этот обёртчик.

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

Является ли вариантом успеха. Только чтение по ссылке, `self` можно использовать многократно.

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

Является ли вариантом ошибки. Только чтение по ссылке.

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.err("e")
    assert(result.is_err(r))
}
```

## Извлечение значения

### unwrap

<!-- stdlib:sig:result.unwrap start -->

```yaoxiang
unwrap: (T: Type, E: Type)(self: &Result(T, E)) -> T
```

<!-- stdlib:sig:result.unwrap end -->

Извлекает успешное значение.

Возврат: значение, которое несёт вариант `Ok`. Ошибка: при вызове на значении `Err` выбрасывается
`E6007`, а сообщение **содержит исходный код ошибки и описание**, например
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

Возврат: значение, которое несёт вариант `Err`. Ошибка: при вызове на значении `Ok` выбрасывается
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

> Сигнатура типизирована как `Error`, но носитель ошибки во время выполнения — структура с полями
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

## Связанные ссылки

- [`std.string`](./string#parse_int) — функция разбора, возвращающая `Result`
- [Справочник по кодам ошибок](../error-code/) — коды значений ошибок времени выполнения, такие как
  `E6010` / `E6011`
