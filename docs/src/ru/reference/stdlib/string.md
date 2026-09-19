---
title: 'std.string'
description: 'поиск, разбиение, форматирование и разбор строк'
---

# std.string

Модуль строковых операций. За исключением `format`, все функции принимают аргументы по заимствованию
только для чтения (`&String`); исходная строка остаётся доступной для дальнейшего использования
после вызова.

Если типы аргументов не совпадают, все функции вырождаются в **семантику пустой строки** (а не
выдают ошибку): `split`/`trim`/`upper` и т.д. обрабатывают аргументы не типа `String` как `""`. Это
означает, что вызов с ошибочным типом аргумента не прерывается, но и не даёт ожидаемого результата —
рекомендуется полагаться на проверку типов во время компиляции.

```yaoxiang
use std.string
```

## Обзор функций

<!-- stdlib:table:string start -->

| Функция       | Сигнатура                                            |
| ------------- | ---------------------------------------------------- |
| `split`       | `(s: &String, sep: &String) -> List(String)`         |
| `trim`        | `(s: &String) -> String`                             |
| `upper`       | `(s: &String) -> String`                             |
| `lower`       | `(s: &String) -> String`                             |
| `replace`     | `(s: &String, old: &String, new: &String) -> String` |
| `contains`    | `(s: &String, sub: &String) -> Bool`                 |
| `starts_with` | `(s: &String, prefix: &String) -> Bool`              |
| `ends_with`   | `(s: &String, suffix: &String) -> Bool`              |
| `index_of`    | `(s: &String, sub: &String) -> Int`                  |
| `substring`   | `(s: &String, start: Int, end: Int) -> String`       |
| `is_empty`    | `(s: &String) -> Bool`                               |
| `len`         | `(s: &String) -> Int`                                |
| `chars`       | `(s: &String) -> List(String)`                       |
| `concat`      | `(s1: &String, s2: &String) -> String`               |
| `repeat`      | `(s: &String, n: Int) -> String`                     |
| `reverse`     | `(s: &String) -> String`                             |
| `format`      | `(format: &String, ...args) -> String`               |
| `parse_int`   | `(s: &String) -> Result(Int, Error)`                 |
| `parse_float` | `(s: &String) -> Result(Float, Error)`               |

<!-- stdlib:table:string end -->

## Функции

### split

<!-- stdlib:sig:string.split start -->

```yaoxiang
split: (s: &String, sep: &String) -> List(String)
```

<!-- stdlib:sig:string.split end -->

Разбивает `s` по `sep`, возвращает список подстрок.

- `s` — разбиваемая строка
- `sep` — разделитель; **если пустая строка — разбивает посимвольно**

Возвращает: `List(String)`. Если разделитель не найден, возвращается одноэлементный список.

```yaoxiang
use std.assert
use std.list
use std.string

main: () -> Void = {
    assert(list.len(string.split("a,b,c", ",")) == 3)
    assert(list.get(string.split("a,b,c", ","), 0) == "a")

    // 空分隔符 → 逐字符
    cs = string.split("abc", "")
    assert(list.len(cs) == 3)
}
```

### trim

<!-- stdlib:sig:string.trim start -->

```yaoxiang
trim: (s: &String) -> String
```

<!-- stdlib:sig:string.trim end -->

Удаляет начальные и конечные пробельные символы Unicode.

Возвращает: новую строку с удалёнными начальными и конечными пробелами (не изменяет `s`).

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.trim("  hi  ") == "hi")
}
```

### upper

<!-- stdlib:sig:string.upper start -->

```yaoxiang
upper: (s: &String) -> String
```

<!-- stdlib:sig:string.upper end -->

Преобразует в верхний регистр (с учётом Unicode).

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.upper("abc") == "ABC")
}
```

### lower

<!-- stdlib:sig:string.lower start -->

```yaoxiang
lower: (s: &String) -> String
```

<!-- stdlib:sig:string.lower end -->

Преобразует в нижний регистр (с учётом Unicode).

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.lower("ABC") == "abc")
}
```

### replace

<!-- stdlib:sig:string.replace start -->

```yaoxiang
replace: (s: &String, old: &String, new: &String) -> String
```

<!-- stdlib:sig:string.replace end -->

Заменяет **все** вхождения `old` в `s` на `new`.

- `old` — если пустая строка, **возвращает `s` без изменений** (без вставки)

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.replace("a-b-c", "-", "+") == "a+b+c")
    assert(string.replace("abc", "", "x") == "abc")
}
```

### contains

<!-- stdlib:sig:string.contains start -->

```yaoxiang
contains: (s: &String, sub: &String) -> Bool
```

<!-- stdlib:sig:string.contains end -->

Проверяет, содержится ли `sub` в `s`. Для пустой подстроки всегда возвращает `true`.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.contains("hello", "ell"))
    assert(!string.contains("hello", "xyz"))
}
```

### starts_with

<!-- stdlib:sig:string.starts_with start -->

```yaoxiang
starts_with: (s: &String, prefix: &String) -> Bool
```

<!-- stdlib:sig:string.starts_with end -->

Проверяет, начинается ли `s` с `prefix`. Для пустого `prefix` всегда возвращает `true`.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.starts_with("hello", "he"))
}
```

### ends_with

<!-- stdlib:sig:string.ends_with start -->

```yaoxiang
ends_with: (s: &String, suffix: &String) -> Bool
```

<!-- stdlib:sig:string.ends_with end -->

Проверяет, заканчивается ли `s` на `suffix`. Для пустого `suffix` всегда возвращает `true`.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.ends_with("hello", "lo"))
}
```

### index_of

<!-- stdlib:sig:string.index_of start -->

```yaoxiang
index_of: (s: &String, sub: &String) -> Int
```

<!-- stdlib:sig:string.index_of end -->

Индекс первого вхождения `sub` в **байтах**.

Возвращает: индекс при обнаружении; `-1`, если не найдено.

> Возвращается смещение в байтах. При наличии многобайтовых символов можно использовать `chars` для
> преобразования и последующего определения индекса символа.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.index_of("hello", "ll") == 2)
    assert(string.index_of("hello", "xyz") == -1)
}
```

### substring

<!-- stdlib:sig:string.substring start -->

```yaoxiang
substring: (s: &String, start: Int, end: Int) -> String
```

<!-- stdlib:sig:string.substring end -->

Извлекает диапазон `[start, end)` по **символьным** индексам.

- `start` — начальный индекс символа, по умолчанию `0`
- `end` — конечный индекс символа (не включается), по умолчанию — конец строки

Возвращает: результат извлечения. Выходящие за границы значения **зажимаются** в допустимый
диапазон, ошибка не выдаётся; при `start > end` результат зажимается до пустой строки.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.substring("hello", 1, 4) == "ell")
    assert(string.substring("hello", 1, 99) == "ello")   // 上界钳制
}
```

### is_empty

<!-- stdlib:sig:string.is_empty start -->

```yaoxiang
is_empty: (s: &String) -> Bool
```

<!-- stdlib:sig:string.is_empty end -->

Проверяет, является ли `s` пустой строкой.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.is_empty(""))
    assert(!string.is_empty("x"))
}
```

### len

<!-- stdlib:sig:string.len start -->

```yaoxiang
len: (s: &String) -> Int
```

<!-- stdlib:sig:string.len end -->

Возвращает **длину в байтах UTF-8**, а не количество символов.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.len("hello") == 5)
    assert(string.len("中") == 3)   // 字节长度
}
```

### chars

<!-- stdlib:sig:string.chars start -->

```yaoxiang
chars: (s: &String) -> List(String)
```

<!-- stdlib:sig:string.chars end -->

Разбивает на список односимвольных строк (по скалярным значениям Unicode).

```yaoxiang
use std.assert
use std.list
use std.string

main: () -> Void = {
    cs = string.chars("ab")
    assert(list.len(cs) == 2)
    assert(list.get(cs, 0) == "a")
}
```

### concat

<!-- stdlib:sig:string.concat start -->

```yaoxiang
concat: (s1: &String, s2: &String) -> String
```

<!-- stdlib:sig:string.concat end -->

Сцепляет две строки. Также можно использовать оператор `+`.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.concat("a", "b") == "ab")
}
```

### repeat

<!-- stdlib:sig:string.repeat start -->

```yaoxiang
repeat: (s: &String, n: Int) -> String
```

<!-- stdlib:sig:string.repeat end -->

Повторяет `s` `n` раз.

- `n` — количество повторений; при `n <= 0` возвращает пустую строку

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.repeat("ab", 3) == "ababab")
    assert(string.repeat("ab", 0) == "")
}
```

### reverse

<!-- stdlib:sig:string.reverse start -->

```yaoxiang
reverse: (s: &String) -> String
```

<!-- stdlib:sig:string.reverse end -->

Переворачивает по символам.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.reverse("abc") == "cba")
}
```

### format

<!-- stdlib:sig:string.format start -->

```yaoxiang
format: (format: &String, ...args) -> String
```

<!-- stdlib:sig:string.format end -->

Форматирует с использованием плейсхолдеров `{index}`, с необязательными спецификаторами
ширины/выравнивания.

Синтаксис плейсхолдера:

| Форма    | Значение                                               |
| -------- | ------------------------------------------------------ |
| `{0}`    | 0-й аргумент (аргументы после `format` нумеруются с 0) |
| `{0:03}` | ширина 3                                               |
| `{0:>3}` | ширина 3, выравнивание по правому краю (по умолчанию)  |
| `{0:<3}` | ширина 3, выравнивание по левому краю                  |
| `{0:^3}` | ширина 3, выравнивание по центру                       |

Литеральные фигурные скобки обозначаются удвоением: две открывающие фигурные скобки дают одну
литеральную открывающую скобку, две закрывающие — аналогично.

Возвращаемое значение: отформатированная строка. Аргументы сначала преобразуются в строки
(аналогично `convert.to_string`); при выходе индекса за границы берётся пустая строка, недопустимая
ширина обрабатывается как `0`.

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.format("{0}-{1}", "a", "b") == "a-b")
    assert(string.format("[{0:>5}]", "ab") == "[   ab]")
    assert(string.format("[{0:<5}]", "ab") == "[ab   ]")
}
```

### parse_int

<!-- stdlib:sig:string.parse_int start -->

```yaoxiang
parse_int: (s: &String) -> Result(Int, Error)
```

<!-- stdlib:sig:string.parse_int end -->

Разбирает десятичное целое число (автоматически удаляет начальные и конечные пробелы).

Возвращает: при успехе `Result.ok(Int)`; при неудаче `Result.err(Error)` с `code` `E6010`.
**Исключение не выбрасывается**.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_int("42")))
    assert(result.is_err(string.parse_int("abc")))
}
```

### parse_float

<!-- stdlib:sig:string.parse_float start -->

```yaoxiang
parse_float: (s: &String) -> Result(Float, Error)
```

<!-- stdlib:sig:string.parse_float end -->

Разбирает число с плавающей точкой (автоматически удаляет начальные и конечные пробелы).

Возвращает: при успехе `Result.ok(Float)`; при неудаче `Result.err(Error)` с `code` `E6011`.

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_float("3.14")))
    assert(result.is_err(string.parse_float("xxx")))
}
```

## Связанные разделы

- [`std.convert`](./convert) — преобразование чисел в строки
- [`std.result`](./result) — распаковка результатов `parse_*`
