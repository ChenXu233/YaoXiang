---
title: 'std.time'
description: 'Метки времени, форматирование и доступ к полям DateTime'
---

# std.time

Модуль времени.

```yaoxiang
use std.time
```

> **В данном модуле имеется несколько пробелов в реализации (#338 / #340)**, что было подтверждено
> эмпирически при написании документации. Доступные и недоступные возможности отмечены далее по
> тексту раздельно, чтобы избежать написания некомпилируемых примеров «по сигнатуре».
>
> Можно использовать нормально: [`now`](#now) / [`timestamp`](#timestamp) /
> [`timestamp_ms`](#timestamp_ms) / [`sleep`](#sleep) / [`format_time`](#format_time) (принимает
> литерал метки времени `Int`).
>
> В настоящее время недоступны: возвращаемое значение [`parse_time`](#parse_time), а также все
> аксессоры [`DateTime::*`](#datetime-доступ-к-полям-недоступен).

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

Возвращает: значение `DateTime`, при печати отображается как `DateTime(1789471990)`. **Это не
`Int`**, поэтому оно не может напрямую участвовать в арифметике или сравнениях, а также не может
быть передано другим функциям в качестве параметра `Int` (см. [`format_time`](#format_time)).

```yaoxiang
use std.time

main: () -> Void = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> Если требуется участие в вычислениях, используйте [`timestamp`](#timestamp) или
> [`timestamp_ms`](#timestamp_ms) — они возвращают `Int` напрямую.

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

Возвращает текущую метку времени Unix (**в секундах**); может напрямую участвовать в арифметике и
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

Приостанавливает выполнение на указанное количество **секунд** (дробное допускается). Одноимённый
[`std.concurrent.sleep`](./concurrent#sleep) работает в **миллисекундах**, обратите внимание на
разницу.

- `seconds` — количество секунд задержки; принимает `Int` (интерпретируется как секунды) или `Float`

Ошибка: выбрасывает `E6007`, если аргумент не является ни `Int`, ни `Float`.

```yaoxiang
use std.time

main: () -> Void = {
    time.sleep(0.0)
}
```

> На целевой платформе `wasm32` не экспортируется.

## Форматирование и разбор

### format_time

<!-- stdlib:sig:time.format_time start -->

```yaoxiang
format_time: (dt: Int, fmt: String) -> String
```

<!-- stdlib:sig:time.format_time end -->

Форматирует метку времени в соответствии с шаблоном `fmt`; поддерживаются заполнители в стиле
`strftime`.

- `dt` — метка времени Unix (**в секундах**); должна иметь тип `Int`
- `fmt` — строка формата

> **Замечание о типах**: `dt` должен быть `Int`. Передача возвращаемого значения [`now`](#now) или
> [`parse_time`](#parse_time) приведёт к ошибке `E1002`
> (`expected type 'int64', found type 'DateTime'`), поскольку оба эти значения имеют тип `DateTime`.
> В настоящее время не существует способа преобразования `DateTime` → `Int`, поэтому **на практике
> можно передавать только литерал `Int` или результат [`timestamp`](#timestamp)**.

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

Разложение выполняется по **локальному времени**. Неизвестные заполнители сохраняются в выводе как
есть.

Ошибка: выбрасывает `E6007`, если `dt` не `Int`, `fmt` не `String`, либо аргументов недостаточно.

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

Разбирает строку времени в значение времени.

- `fmt` — **в настоящее время игнорируется** (см. ниже)
- `s` — разбираемая строка

Возвращает: значение `DateTime`.

> **Два ограничения реализации (#340)**:
>
> 1. Параметр `fmt` **не участвует в разборе**. Функция распознаёт только форму ISO 8601 —
>    `YYYY-MM-DDTHH:MM:SS` или `YYYY-MM-DD HH:MM:SS` (между датой и временем допускается `T` либо
>    пробел). Любая другая форма приводит к неудаче, вне зависимости от того, что записано в `fmt`.
> 2. Возвращённый `DateTime` **в настоящее время непригоден к дальнейшему использованию** — он не
>    является `Int` (его нельзя передать в `format_time` / `DateTime::*`), а вызываемых аксессоров у
>    него нет (см. следующий раздел).

Ошибка: выбрасывает `E6007` при несовпадении формата.

```yaoxiang
use std.time

main: () -> Void = {
    // 解析本身可以成功
    ts = time.parse_time("", "2024-01-15 10:30:00")
    println(ts)
}
```

## Доступ к полям DateTime (недоступен)

> Отслеживание: #338

`std.time` экспортирует 8 аксессоров с именами вида `DateTime::`:

| Имя экспорта          | Сигнатура             | Описание                      |
| --------------------- | --------------------- | ----------------------------- |
| `DateTime::year`      | `(dt: Int) -> Int`    | четырёхзначный год            |
| `DateTime::month`     | `(dt: Int) -> Int`    | месяц (1–12)                  |
| `DateTime::day`       | `(dt: Int) -> Int`    | день (1–31)                   |
| `DateTime::hour`      | `(dt: Int) -> Int`    | час (0–23)                    |
| `DateTime::minute`    | `(dt: Int) -> Int`    | минута (0–59)                 |
| `DateTime::second`    | `(dt: Int) -> Int`    | секунда (0–59)                |
| `DateTime::weekday`   | `(dt: Int) -> Int`    | день недели (0 = воскресенье) |
| `DateTime::to_string` | `(dt: Int) -> String` | строка в форме ISO 8601       |

**Однако в настоящее время эти имена не могут быть вызваны из исходного кода YaoXiang (#338).**
Имена экспортов содержат `::`, а `::` — это зарезервированная синтаксическая конструкция, которая не
может появляться в позиции доступа к полю. Эмпирически подтверждено, что все перечисленные ниже
варианты записи завершаются неудачей:

| Попытка записи             | Результат                                           |
| -------------------------- | --------------------------------------------------- |
| `time.DateTime::year(0)`   | `E0010 expected RParen, found ColonColon`           |
| `time.DateTime.year(0)`    | `E1042 Field 'DateTime' not found in struct 'time'` |
| `time.year(0)`             | `E1042 Field 'year' not found in struct 'time'`     |
| `time.DateTime_year(0)`    | `E1042 Field 'DateTime_year' not found`             |
| `time."DateTime::year"(0)` | `E0011 Unexpected token: StringLiteral`             |

**Альтернатива**: когда нужны компоненты даты, форматируйте метку времени через
[`format_time`](#format_time) — внутри уже выполнено разложение метки времени на год, месяц и день:

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // 用 format_time 取各分量
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")
    assert(time.format_time(0, "%d") == "01")
}
```

## Связанные разделы

- [`std.concurrent`](./concurrent) — задержка с точностью до миллисекунд
- [Справочник по кодам ошибок](../error-code/) — `E6007` общая ошибка времени выполнения
