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
> `DateTime` теперь является **псевдонимом метки времени** (т.е. `Int`) — в runtime это уже
> `RuntimeValue::Int`, ранее это было лишь именем без сущности, из-за чего возвращаемое значение
> `now()` не могло быть передано в `format_time` и в методы доступа. Имена методов доступа также
> были изменены с `DateTime::year` на **`datetime_year`** (`::` — лексически зарезервированный
> символ, не может появляться в позиции доступа к полю).
>
> Теперь возвращаемые значения `now()` / `parse_time()` можно напрямую использовать в арифметике,
> форматировании и во всех методах доступа.

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

Возвращает: значение `DateTime`, при печати отображается как `DateTime(1789471990)`. **Это не
`Int`**, поэтому не может напрямую участвовать в арифметических операциях или сравнениях, а также не
может быть передано в качестве параметра `Int` другим функциям (см. [`format_time`](#format_time)).

```yaoxiang
use std.time

main: () -> Void = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> Когда требуется участие в вычислениях, используйте [`timestamp`](#timestamp) или
> [`timestamp_ms`](#timestamp_ms), они возвращают `Int` напрямую.

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

Возвращает текущую Unix-метку времени (**секунды**), может напрямую участвовать в арифметических
операциях и сравнениях.

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

Возвращает текущую Unix-метку времени (**миллисекунды**).

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // 毫秒精度不低于秒精度
    assert(time.timestamp_ms() >= time.timestamp())
}
```

### sleep

<!-- stdlib:sig:time.sleep start -->

```yaoxiang
sleep: (seconds: Float) -> Void
```

<!-- stdlib:sig:time.sleep end -->

Засыпает на указанное **число секунд** (допускается дробное). Одноимённая
[`std.concurrent.sleep`](./concurrent#sleep) работает в **миллисекундах**, обратите внимание на
различие.

- `seconds` — количество секунд сна; принимает `Int` (интерпретируется как секунды) или `Float`

Ошибка: выбрасывает `E6007`, если аргумент не является ни `Int`, ни `Float`.

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

Форматирует метку времени по `fmt`, поддерживает заполнители в стиле `strftime`.

- `dt` — Unix-метка времени (**секунды**), должна быть `Int`
- `fmt` — строка формата

> **Замечание о типах**: `dt` должен быть `Int`. При передаче возвращаемого значения [`now`](#now)
> или [`parse_time`](#parse_time) будет выдана ошибка `E1002`
> (`expected type 'int64', found type 'DateTime'`), поскольку они оба являются `DateTime`. В
> настоящее время нет средства преобразования `DateTime` → `Int`, поэтому **фактически можно
> передавать только литералы `Int` или результат [`timestamp`](#timestamp)**.

Поддерживаемые заполнители:

| Заполнитель | Значение                             | Пример       |
| ----------- | ------------------------------------ | ------------ |
| `%Y`        | Год из четырёх цифр                  | `2024`       |
| `%m`        | Месяц из двух цифр                   | `01`         |
| `%d`        | День из двух цифр                    | `15`         |
| `%H`        | Час из двух цифр (24-часовой формат) | `10`         |
| `%M`        | Минута из двух цифр                  | `30`         |
| `%S`        | Секунда из двух цифр                 | `00`         |
| `%w`        | День недели (0 = воскресенье)        | `1`          |
| `%F`        | Эквивалентно `%Y-%m-%d`              | `2024-01-15` |
| `%T`        | Эквивалентно `%H:%M:%S`              | `10:30:00`   |

Разбирается по **местному времени**. Неизвестные заполнители сохраняются как есть.

Ошибка: выбрасывает `E6007`, если `dt` не `Int`, `fmt` не `String`, или аргументов недостаточно.

```yaoxiang
use std.assert
use std.string
use std.time

main: () -> Void = {
    // 0 = Unix epoch
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")

    // 当前时间戳（Int）也可直接使用
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
> 1. Параметр `fmt` **не участвует в разборе**. Функция распознаёт только формы ISO 8601
>    `YYYY-MM-DDTHH:MM:SS` или `YYYY-MM-DD HH:MM:SS` (между датой и временем используется `T` или
>    пробел), любая другая форма приведёт к ошибке, независимо от того, что написано в `fmt`.
> 2. Возвращаемый `DateTime` **в настоящее время не может быть использован далее** — он не является
>    `Int` (не может быть передан в `format_time` / `DateTime::*`), и у него нет вызываемых методов
>    доступа (см. следующий раздел).

Ошибка: выбрасывает `E6007` при несоответствии формата.

```yaoxiang
use std.time

main: () -> Void = {
    // 解析本身可以成功
    ts = time.parse_time("", "2024-01-15 10:30:00")
    println(ts)
}
```

## Доступ к полям DateTime

`std.time` экспортирует 8 методов доступа к компонентам даты (`#338` устранён, имена экспорта имеют
плоскую форму):

| Имя экспорта         | Сигнатура             | Описание                      |
| -------------------- | --------------------- | ----------------------------- |
| `datetime_year`      | `(dt: Int) -> Int`    | Год из четырёх цифр           |
| `datetime_month`     | `(dt: Int) -> Int`    | Месяц (1–12)                  |
| `datetime_day`       | `(dt: Int) -> Int`    | День (1–31)                   |
| `datetime_hour`      | `(dt: Int) -> Int`    | Час (0–23)                    |
| `datetime_minute`    | `(dt: Int) -> Int`    | Минута (0–59)                 |
| `datetime_second`    | `(dt: Int) -> Int`    | Секунда (0–59)                |
| `datetime_weekday`   | `(dt: Int) -> Int`    | День недели (0 = воскресенье) |
| `datetime_to_string` | `(dt: Int) -> String` | Строка в формате ISO 8601     |

`DateTime` — это **псевдоним метки времени** (т.е. `Int`). Возвращаемые значения `now()` /
`parse_time()` можно напрямую передавать в эти методы доступа, а также напрямую использовать с
[`format_time`](#format_time):

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    ts = time.parse_time("%Y-%m-%d", "2024-01-15")
    assert(time.datetime_year(ts) == 2024)
    assert(time.datetime_month(ts) == 1)
    assert(time.datetime_day(ts) == 15)

    // 与 format_time 等价
    assert(time.format_time(ts, "%Y-%m-%d") == "2024-01-15")
}
```

## Связанные разделы

- [`std.concurrent`](./concurrent) — задержка с точностью до миллисекунд
- [Справочник по кодам ошибок](../error-code/) — `E6007` общая ошибка runtime
