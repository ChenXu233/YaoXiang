---
title: 'std.time'
description: 'Метки времени, форматирование и доступ к полям DateTime'
---

# std.time

Модуль времени.

```yaoxiang
use std.time
```

> **Пробел в реализации устранён (#338 / #340, 2026-09-19)**.
>
> `DateTime` теперь является **псевдонимом метки времени** (т.е. `Int`) — во время выполнения это и
> так `RuntimeValue::Int`, а ранее это было лишь именем без сущности, из-за чего возвращаемое
> значение `now()` не могло быть передано в `format_time` и аксессоры. Имя экспортируемого аксессора
> также изменено с `DateTime::year` на **`datetime_year`** (`::` — лексически зарезервированный
> символ, который не может встречаться в позиции доступа к полю).
>
> Теперь возвращаемые значения `now()` / `parse_time()` могут напрямую использоваться в арифметике,
> форматировании и со всеми аксессорами.

## Обзор функций

<!-- stdlib:table:time start -->

| Функция              | Сигнатура                              |
| -------------------- | -------------------------------------- |
| `now`                | `() -> DateTime`                       |
| `timestamp`          | `() -> Int`                            |
| `timestamp_ms`       | `() -> Int`                            |
| `sleep`              | `(seconds: Float) -> Void`             |
| `format_time`        | `(dt: Int, fmt: String) -> String`     |
| `parse_time`         | `(fmt: String, s: String) -> DateTime` |
| `datetime_year`      | `(dt: Int) -> Int`                     |
| `datetime_month`     | `(dt: Int) -> Int`                     |
| `datetime_day`       | `(dt: Int) -> Int`                     |
| `datetime_hour`      | `(dt: Int) -> Int`                     |
| `datetime_minute`    | `(dt: Int) -> Int`                     |
| `datetime_second`    | `(dt: Int) -> Int`                     |
| `datetime_weekday`   | `(dt: Int) -> Int`                     |
| `datetime_to_string` | `(dt: Int) -> String`                  |

<!-- stdlib:table:time end -->## Получение времени

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> DateTime
```

<!-- stdlib:sig:time.now end -->

Возвращает текущее время.

Возвращает: значение `DateTime`, при печати выглядит как `DateTime(1789471990)`. **Оно не является
`Int`**, поэтому не может напрямую участвовать в арифметике или сравнениях, а также не может быть
передано как параметр типа `Int` в другие функции (см. [`format_time`](#format_time)).

```yaoxiang
use std.time

main: () -> Void = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> Если нужно участвовать в вычислениях, используйте [`timestamp`](#timestamp) или
> [`timestamp_ms`](#timestamp_ms) — они напрямую возвращают `Int`.

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

Возвращает текущую метку времени Unix (**в секундах**), может напрямую участвовать в арифметике и
сравнениях.

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

Возвращает текущую метку времени Unix (**в миллисекундах**).

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

Засыпает на указанное **количество секунд** (допускается дробное значение). Одноимённый
[`std.concurrent.sleep`](./concurrent#sleep) использует **миллисекунды**, обратите внимание на
различие.

- `seconds` — количество секунд сна; принимает `Int` (интерпретируется как секунды) или `Float`

Ошибки: выбрасывает `E6007`, если аргумент не является ни `Int`, ни `Float`.

```yaoxiang
use std.time

main: () -> Void = {
    time.sleep(0.0)
}
```

> Не экспортируется на целевой платформе `wasm32`.

## Форматирование и разбор

### format_time

<!-- stdlib:sig:time.format_time start -->

```yaoxiang
format_time: (dt: Int, fmt: String) -> String
```

<!-- stdlib:sig:time.format_time end -->

Форматирует метку времени по шаблону `fmt`, поддерживает заполнители в стиле `strftime`.

- `dt` — метка времени Unix (**в секундах**), должна быть `Int`
- `fmt` — строка формата

> **Замечание о типах**: `dt` должен быть `Int`. Передача возвращаемого значения [`now`](#now) или
> [`parse_time`](#parse_time) вызовет ошибку `E1002`
> (`expected type 'int64', found type 'DateTime'`), поскольку оба они имеют тип `DateTime`. В
> настоящее время нет средства преобразования `DateTime` → `Int`, поэтому **фактически можно
> передать только литерал `Int` или результат [`timestamp`](#timestamp)**.

Поддерживаемые заполнители:

| Заполнитель | Значение                           | Пример       |
| ----------- | ---------------------------------- | ------------ |
| `%Y`        | четырёхзначный год                 | `2024`       |
| `%m`        | двузначный месяц                   | `01`         |
| `%d`        | двузначный день                    | `15`         |
| `%H`        | двузначный час (24-часовой формат) | `10`         |
| `%M`        | двузначные минуты                  | `30`         |
| `%S`        | двузначные секунды                 | `00`         |
| `%w`        | день недели (0 = воскресенье)      | `1`          |
| `%F`        | эквивалент `%Y-%m-%d`              | `2024-01-15` |
| `%T`        | эквивалент `%H:%M:%S`              | `10:30:00`   |

Разбирается по **локальному времени**. Неизвестные заполнители сохраняются как есть.

Ошибки: выбрасывает `E6007`, если `dt` не `Int`, `fmt` не `String`, или аргументов недостаточно.

```yaoxiang
use std.assert
use std.string
use std.time

main: () -> Void = {
    // 0 = Unix epoch
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")

    // текущую метку времени (Int) также можно использовать напрямую
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

Разбирает строку времени.

- `fmt` — **в настоящее время игнорируется** (см. ниже)
- `s` — разбираемая строка

Возвращает: значение `DateTime`.

> **Два ограничения реализации (#340)**:
>
> 1. Параметр `fmt` **не участвует в разборе**. Функция распознаёт только форму ISO 8601:
>    `YYYY-MM-DDTHH:MM:SS` или `YYYY-MM-DD HH:MM:SS` (между датой и временем используется `T` или
>    пробел). Любые другие формы приведут к ошибке, независимо от того, что написано в `fmt`.
> 2. Возвращённый `DateTime` **в настоящее время не может быть использован дальше** — он не является
>    `Int` (не может быть передан в `format_time` / `DateTime::*`), и у него нет вызываемых
>    аксессоров (см. следующий раздел).

Ошибки: выбрасывает `E6007`, если формат не совпадает.

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

`std.time` экспортирует 8 аксессоров с именами вида `DateTime::`:

| Экспортируемое имя    | Сигнатура             | Описание                      |
| --------------------- | --------------------- | ----------------------------- |
| `DateTime::year`      | `(dt: Int) -> Int`    | четырёхзначный год            |
| `DateTime::month`     | `(dt: Int) -> Int`    | месяц (1–12)                  |
| `DateTime::day`       | `(dt: Int) -> Int`    | день (1–31)                   |
| `DateTime::hour`      | `(dt: Int) -> Int`    | час (0–23)                    |
| `DateTime::minute`    | `(dt: Int) -> Int`    | минуты (0–59)                 |
| `DateTime::second`    | `(dt: Int) -> Int`    | секунды (0–59)                |
| `DateTime::weekday`   | `(dt: Int) -> Int`    | день недели (0 = воскресенье) |
| `DateTime::to_string` | `(dt: Int) -> String` | строка в форме ISO 8601       |

**Но эти имена в настоящее время не могут быть вызваны из исходного кода YaoXiang (#338).** Имена
экспорта содержат `::`, а `::` — зарезервированный символ в синтаксисе, который не может встречаться
в позиции доступа к полю. Проверено на практике, что все следующие варианты записи завершаются
неудачей:

| Попытка записи             | Результат                                           |
| -------------------------- | --------------------------------------------------- |
| `time.DateTime::year(0)`   | `E0010 expected RParen, found ColonColon`           |
| `time.DateTime.year(0)`    | `E1042 Field 'DateTime' not found in struct 'time'` |
| `time.year(0)`             | `E1042 Field 'year' not found in struct 'time'`     |
| `time.DateTime_year(0)`    | `E1042 Field 'DateTime_year' not found`             |
| `time."DateTime::year"(0)` | `E0011 Unexpected token: StringLiteral`             |

**Альтернатива**: когда нужны компоненты даты, используйте [`format_time`](#format_time) для
форматирования по необходимости — внутри уже выполнено разложение метки времени на год, месяц и
день:

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // используем format_time для получения каждого компонента
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")
    assert(time.format_time(0, "%d") == "01")
}
```

## Связанные

- [`std.concurrent`](./concurrent) — засыпание с миллисекундной точностью
- [Справочник по кодам ошибок](../error-code/) — общая ошибка времени выполнения `E6007`
