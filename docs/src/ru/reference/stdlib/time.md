---
title: 'std.time'
description: 'Временные метки, форматирование и доступ к полям DateTime'
---

# std.time

Модуль времени.

```yaoxiang
use std.time
```

> **В этом модуле имеется ряд пробелов в реализации (#338 / #340)**, что подтверждено при написании
> документации. Доступные и недоступные части отмечены ниже по отдельности, чтобы не приводить
> примеры, которые не компилируются, опираясь только на сигнатуры.
>
> Можно использовать: [`now`](#now) / [`timestamp`](#timestamp) / [`timestamp_ms`](#timestamp_ms) /
> [`sleep`](#sleep) / [`format_time`](#format_time) (принимает `Int`-литерал временной метки).
>
> В настоящее время недоступно: возвращаемое значение [`parse_time`](#parse_time), а также все
> аксессоры [`DateTime::*`](#доступ-к-полям-datetime-недоступно).

## Обзор функций

<!-- stdlib:table:time start -->

| Функция               | Сигнатура                              |
| --------------------- | -------------------------------------- |
| `now`                 | `() -> DateTime`                       |
| `timestamp`           | `() -> Int`                            |
| `timestamp_ms`        | `() -> Int`                            |
| `sleep`               | `(seconds: Float) -> Void`             |
| `format_time`         | `(dt: Int, fmt: String) -> String`     |
| `parse_time`          | `(fmt: String, s: String) -> DateTime` |
| `DateTime::year`      | `(dt: Int) -> Int`                     |
| `DateTime::month`     | `(dt: Int) -> Int`                     |
| `DateTime::day`       | `(dt: Int) -> Int`                     |
| `DateTime::hour`      | `(dt: Int) -> Int`                     |
| `DateTime::minute`    | `(dt: Int) -> Int`                     |
| `DateTime::second`    | `(dt: Int) -> Int`                     |
| `DateTime::weekday`   | `(dt: Int) -> Int`                     |
| `DateTime::to_string` | `(dt: Int) -> String`                  |

<!-- stdlib:table:time end -->

## Получение времени

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> DateTime
```

<!-- stdlib:sig:time.now end -->

Возвращает текущее время.

Возврат: значение `DateTime`, при выводе выглядит как `DateTime(1789471990)`. **Оно не является
`Int`**, поэтому не может напрямую участвовать в арифметических операциях или сравнениях, а также не
может быть передано другим функциям в качестве параметра типа `Int` (см.
[`format_time`](#format_time)).

```yaoxiang
use std.time

main: () -> Void = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> Когда требуется участие в вычислениях, используйте [`timestamp`](#timestamp) или
> [`timestamp_ms`](#timestamp_ms) — они сразу возвращают `Int`.

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

Возвращает текущую Unix-временную метку (**в секундах**); может напрямую участвовать в
арифметических операциях и сравнениях.

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    assert(time.timestamp() > 0)
}
```

### timestamp_ms

<!-- stdlib:sig:time.timestamp_ms start -->

```yaoxiang
timestamp_ms: () -> Int
```

<!-- stdlib:sig:time.timestamp_ms end -->

Возвращает текущую Unix-временную метку (**в миллисекундах**).

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // точность в миллисекундах не ниже точности в секундах
    assert(time.timestamp_ms() >= time.timestamp())
}
```

### sleep

<!-- stdlib:sig:time.sleep start -->

```yaoxiang
sleep: (seconds: Float) -> Void
```

<!-- stdlib:sig:time.sleep end -->

Приостанавливает выполнение на указанное число **секунд** (допускается дробное значение).
Одноимённая функция [`std.concurrent.sleep`](./concurrent#sleep) использует **миллисекунды** —
обратите на это внимание.

- `seconds` — число секунд; принимает `Int` (интерпретируется как секунды) или `Float`.

Ошибка: если аргумент не является ни `Int`, ни `Float`, выбрасывается `E6007`.

```yaoxiang
use std.time

main: () -> Void = {
    time.sleep(0.0)
}
```

> Не экспортируется для целевой платформы `wasm32`.

## Форматирование и разбор

### format_time

<!-- stdlib:sig:time.format_time start -->

```yaoxiang
format_time: (dt: Int, fmt: String) -> String
```

<!-- stdlib:sig:time.format_time end -->

Форматирует временную метку согласно `fmt`; поддерживаются подстановочные знаки в стиле `strftime`.

- `dt` — Unix-временная метка (**в секундах**), обязательно `Int`.
- `fmt` — строка формата.

> **Замечание по типам**: `dt` должен быть `Int`. Передача возвращаемого значения [`now`](#now) или
> [`parse_time`](#parse_time) приведёт к ошибке `E1002`
> (`expected type 'int64', found type 'DateTime'`), так как оба они имеют тип `DateTime`. В
> настоящее время нет способа преобразовать `DateTime` в `Int`, поэтому **на практике можно
> передавать только `Int`-литералы или результат [`timestamp`](#timestamp)**.

Поддерживаемые подстановочные знаки:

| Знак | Значение                           | Пример       |
| ---- | ---------------------------------- | ------------ |
| `%Y` | четырёхзначный год                 | `2024`       |
| `%m` | двузначный номер месяца            | `01`         |
| `%d` | двузначный номер дня               | `15`         |
| `%H` | двузначный час (24-часовой формат) | `10`         |
| `%M` | двузначные минуты                  | `30`         |
| `%S` | двузначные секунды                 | `00`         |
| `%w` | день недели (0 = воскресенье)      | `1`          |
| `%F` | эквивалент `%Y-%m-%d`              | `2024-01-15` |
| `%T` | эквивалент `%H:%M:%S`              | `10:30:00`   |

Разбор выполняется по **локальному времени**. Неизвестные подстановочные знаки остаются в строке как
есть.

Ошибка: если `dt` не `Int`, `fmt` не `String` или аргументов недостаточно, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.string
use std.time

main: () -> Void = {
    // 0 = Unix epoch
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")

    // текущая временная метка (Int) тоже может быть использована напрямую
    s = time.format_time(time.timestamp(), "%Y")
    assert(string.len(s) == 4)
}
```

### parse_time

<!-- stdlib:sig:time.parse_time start -->

```yaoxiang
parse_time: (fmt: String, s: String) -> DateTime
```

<!-- stdlib:sig:time.parse_time end -->

Разбирает строку времени в значение времени.

- `fmt` — **в настоящее время игнорируется** (см. ниже).
- `s` — разбираемая строка.

Возврат: значение `DateTime`.

> **Два ограничения реализации (#340)**:
>
> 1. Параметр `fmt` **не участвует в разборе**. Функция распознаёт только форму ISO 8601
>    `YYYY-MM-DDTHH:MM:SS` или `YYYY-MM-DD HH:MM:SS` (дата и время разделяются символом `T` или
>    пробелом); любые другие формы завершаются неудачей вне зависимости от того, что указано в
>    `fmt`.
> 2. Возвращённый `DateTime` **в настоящее время нельзя использовать далее** — он не является `Int`
>    (его нельзя передать в `format_time` / `DateTime::*`), и для него нет вызываемых аксессоров
>    (см. следующий раздел).

Ошибка: при несовпадении формата выбрасывается `E6007`.

```yaoxiang
use std.time

main: () -> Void = {
    // сам разбор может завершиться успешно
    ts = time.parse_time("", "2024-01-15 10:30:00")
    println(ts)
}
```

## Доступ к полям DateTime (недоступно)

> Отслеживание: #338

`std.time` экспортирует 8 аксессоров с префиксом `DateTime::`:

| Имя экспорта          | Сигнатура             | Описание                      |
| --------------------- | --------------------- | ----------------------------- |
| `DateTime::year`      | `(dt: Int) -> Int`    | четырёхзначный год            |
| `DateTime::month`     | `(dt: Int) -> Int`    | месяц (1–12)                  |
| `DateTime::day`       | `(dt: Int) -> Int`    | день (1–31)                   |
| `DateTime::hour`      | `(dt: Int) -> Int`    | час (0–23)                    |
| `DateTime::minute`    | `(dt: Int) -> Int`    | минуты (0–59)                 |
| `DateTime::second`    | `(dt: Int) -> Int`    | секунды (0–59)                |
| `DateTime::weekday`   | `(dt: Int) -> Int`    | день недели (0 = воскресенье) |
| `DateTime::to_string` | `(dt: Int) -> String` | строка в форме ISO 8601       |

**Однако в настоящее время эти имена нельзя вызвать из исходного кода YaoXiang (#338).** Имена
экспорта содержат `::`, а `::` является зарезервированным символом в синтаксисе и не может
появляться в позиции доступа к полю. Следующие варианты записи были проверены на практике и все
завершаются неудачей:

| Попытка записи             | Результат                                           |
| -------------------------- | --------------------------------------------------- |
| `time.DateTime::year(0)`   | `E0010 expected RParen, found ColonColon`           |
| `time.DateTime.year(0)`    | `E1042 Field 'DateTime' not found in struct 'time'` |
| `time.year(0)`             | `E1042 Field 'year' not found in struct 'time'`     |
| `time.DateTime_year(0)`    | `E1042 Field 'DateTime_year' not found`             |
| `time."DateTime::year"(0)` | `E0011 Unexpected token: StringLiteral`             |

**Альтернативное решение**: когда нужны компоненты даты, используйте [`format_time`](#format_time)
для форматирования по мере необходимости — внутри него уже выполнено разбиение временной метки на
год, месяц и день:

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // получаем компоненты с помощью format_time
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")
    assert(time.format_time(0, "%d") == "01")
}
```

## См. также

- [`std.concurrent`](./concurrent) — спящий режим с миллисекундной точностью.
- [Справочник по кодам ошибок](../error-code/) — `E6007` общая ошибка runtime.
