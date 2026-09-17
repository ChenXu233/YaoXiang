---
title: 'std.convert'
description: 'Преобразование любого значения в String'
---

# std.convert

Модуль преобразования типов, в настоящее время предоставляет преобразование значений в `String`.

```yaoxiang
use std.convert
```

## Правила преобразования

`to_string` форматирует значение в соответствии с его формой во время выполнения:

| Тип      | Формат вывода                         | Пример                                |
| -------- | ------------------------------------- | ------------------------------------- |
| `Void`   | `void`                                | `void`                                |
| `Bool`   | `true` / `false`                      | `true`                                |
| `Int`    | десятичный                            | `42`                                  |
| `Float`  | см. ниже                              | `3.14` / `2.0`                        |
| `Char`   | сам символ                            | `a`                                   |
| `String` | исходное содержимое (**без кавычек**) | `hello`                               |
| `List`   | `[элемент, ...]`                      | `[1, 2, 3]`                           |
| `Dict`   | `{k: v, ...}`                         | `{a: 1}`                              |
| `Tuple`  | `(элемент, ...)`                      | `(1, hello)`                          |
| `Array`  | `[элемент, ...]`                      | `[1, 2]`                              |
| `Range`  | `начало..конец`                       | `1..5` (опускается при step равном 1) |
| `Bytes`  | `bytes[длина]`                        | `bytes[3]`                            |

Целочисленные значения `Float` дополняются десятичной частью (`2.0`, а не `2`) для удобства
различения `Int` и `Float`.

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

Преобразует любое значение в его строковое представление.

- `value` — значение любого типа

Возвращает: отформатированную строку. При отсутствии аргумента возвращает `"()"`.

```yaoxiang
use std.assert
use std.convert

main = {
    assert(convert.to_string(42) == "42")
    assert(convert.to_string(true) == "true")
    assert(convert.to_string(false) == "false")
}
```

Сама строка не заключается в кавычки:

```yaoxiang
use std.assert
use std.convert

main = {
    assert(convert.to_string("hi") == "hi")
}
```

Составные типы рекурсивно разворачиваются (формат не утверждается, чтобы не быть хрупким):

```yaoxiang
use std.assert
use std.convert
use std.string

main = {
    s_list = convert.to_string([1, 2, 3])
    assert(string.len(s_list) > 0)

    s_dict = convert.to_string({})
    assert(string.len(s_dict) > 0)
}
```

### Метод-форма по типу

Помимо универсального `convert.to_string`, модуль также экспортирует следующие одноимённые функции,
привязанные к типу, поведение которых полностью идентично:

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

> Тип `Set` удалён на уровне языка, `set.to_string` сохранён как заполнитель для совместимости.

## Связанные

- [`std.io`](./io) — внутреннее форматирование `print` / `println` использует тот же набор правил
- [`std.string`](./string) — операции со строками
