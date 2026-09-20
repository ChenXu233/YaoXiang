---
title: 'std.convert'
description: 'Преобразование произвольных значений в String'
---

# std.convert

Модуль преобразования типов, в настоящее время предоставляет преобразование значений в `String`.

```yaoxiang
use std.convert
```

## Правила преобразования

`to_string` форматирует в соответствии с формой значения во время выполнения:

| Тип      | Формат вывода                         | Пример                                    |
| -------- | ------------------------------------- | ----------------------------------------- |
| `Void`   | `void`                                | `void`                                    |
| `Bool`   | `true` / `false`                      | `true`                                    |
| `Int`    | десятичная                            | `42`                                      |
| `Float`  | см. ниже                              | `3.14` / `2.0`                            |
| `Char`   | сам символ                            | `a`                                       |
| `String` | исходное содержимое (**без кавычек**) | `hello`                                   |
| `List`   | `[элемент, ...]`                      | `[1, 2, 3]`                               |
| `Dict`   | `{k: v, ...}`                         | `{a: 1}`                                  |
| `Tuple`  | `(элемент, ...)`                      | `(1, hello)`                              |
| `Array`  | `[элемент, ...]`                      | `[1, 2]`                                  |
| `Range`  | `начало..конец`                       | `1..5` (когда `step` равен 1, опускается) |
| `Bytes`  | `bytes[длина]`                        | `bytes[3]`                                |

Для целочисленных значений `Float` добавляется один десятичный знак (`2.0` вместо `2`), чтобы
отличать `Int` от `Float`.

## Обзор функций

<!-- stdlib:table:convert start -->

| Функция            | Сигнатура           |
| ------------------ | ------------------- |
| `to_string`        | `(value) -> String` |
| `int.to_string`    | `(self) -> String`  |
| `float.to_string`  | `(self) -> String`  |
| `bool.to_string`   | `(self) -> String`  |
| `char.to_string`   | `(self) -> String`  |
| `string.to_string` | `(self) -> String`  |
| `list.to_string`   | `(self) -> String`  |
| `dict.to_string`   | `(self) -> String`  |
| `tuple.to_string`  | `(self) -> String`  |
| `set.to_string`    | `(self) -> String`  |
| `range.to_string`  | `(self) -> String`  |

<!-- stdlib:table:convert end -->

## Функции

### to_string

<!-- stdlib:sig:convert.to_string start -->

```yaoxiang
to_string: (value) -> String
```

<!-- stdlib:sig:convert.to_string end -->

Преобразует произвольное значение в его строковое представление.

- `value` — значение произвольного типа

Возвращает: отформатированную строку. При отсутствии аргумента возвращает `"()"`.

```yaoxiang
use std.assert
use std.convert

main: () -> Void = {
    assert(convert.to_string(42) == "42")
    assert(convert.to_string(true) == "true")
    assert(convert.to_string(false) == "false")
}
```

Сама строка не заключается в кавычки:

```yaoxiang
use std.assert
use std.convert

main: () -> Void = {
    assert(convert.to_string("hi") == "hi")
}
```

Составные типы раскрываются рекурсивно (формат не фиксируется, во избежание хрупкости):

```yaoxiang
use std.assert
use std.convert
use std.string

main: () -> Void = {
    s_list = convert.to_string([1, 2, 3])
    assert(string.len(s_list) > 0)

    s_dict = convert.to_string({})
    assert(string.len(s_dict) > 0)
}
```

### Форма методов типа

Помимо общего `convert.to_string`, модуль также экспортирует следующие одноимённые функции,
привязанные к типу, поведение которых полностью совпадает:

| Имя экспорта       | Сигнатура          |
| ------------------ | ------------------ |
| `int.to_string`    | `(self) -> String` |
| `float.to_string`  | `(self) -> String` |
| `bool.to_string`   | `(self) -> String` |
| `char.to_string`   | `(self) -> String` |
| `string.to_string` | `(self) -> String` |
| `list.to_string`   | `(self) -> String` |
| `dict.to_string`   | `(self) -> String` |
| `tuple.to_string`  | `(self) -> String` |
| `set.to_string`    | `(self) -> String` |
| `range.to_string`  | `(self) -> String` |

Эти привязки используются для диспетчеризации `Stringable` во время выполнения; в повседневном коде
достаточно использовать [`convert.to_string`](#to_string).

> Тип `Set` уже удалён на уровне языка, `set.to_string` сохранён как заполнитель для совместимости.

## Связанное

- [`std.io`](./io) — внутреннее форматирование `print` / `println` использует тот же набор правил
- [`std.string`](./string) — операции со строками
