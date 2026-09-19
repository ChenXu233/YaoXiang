---
title: 'std.io'
description: 'Стандартный вывод, стандартный ввод и чтение/запись файлов целиком'
---

# std.io

Модуль ввода-вывода. Предоставляет стандартный вывод, чтение стандартного ввода, а также удобные
функции для «одноразового чтения/записи всего файла». Для инкрементального чтения/записи файлов по
дескриптору используйте [`std.os`](./os).

```yaoxiang
use std.io
```

## Доступность по платформам

Функции начиная с `read_line` зависят от системного ввода-вывода и **не экспортируются** для целевой
платформы `wasm32`: `read_line`, `read_file`, `write_file`, `append_file`. Функции `print` /
`println` / `format_fallback` доступны на всех платформах.

## Обзор функций

<!-- stdlib:table:io start -->

| Функция           | Сигнатура                                   |
| ----------------- | ------------------------------------------- |
| `print`           | `(...args) -> Void`                         |
| `println`         | `(...args) -> ()`                           |
| `read_line`       | `() -> String`                              |
| `read_file`       | `(path: &String) -> String`                 |
| `write_file`      | `(path: &String, content: &String) -> Bool` |
| `append_file`     | `(path: &String, content: &String) -> Bool` |
| `format_fallback` | `(value, type_name: &String) -> String`     |

<!-- stdlib:table:io end -->## Функции

### print

<!-- stdlib:sig:io.print start -->

```yaoxiang
print: (...args) -> Void
```

<!-- stdlib:sig:io.print end -->

Выводит все аргументы по порядку, **без добавления перевода строки**. Несколько аргументов
разделяются одним пробелом.

Аргументы форматируются: `String` выводит содержимое напрямую; `List` / `Dict` / `Tuple`
раскрываются рекурсивно; остальные значения выводятся как литералы.

```yaoxiang
use std.io

main: () -> Void = {
    print("hello")
    print(" ")
    print("world")
    println("")
}
```

### println

<!-- stdlib:sig:io.println start -->

```yaoxiang
println: (...args) -> ()
```

<!-- stdlib:sig:io.println end -->

Аналогично [`print`](#print), но добавляет перевод строки в конце вывода.

`println()` без аргументов выводит пустую строку:

```yaoxiang
main: () -> Void = {
    println("Hello, YaoXiang!")
}
```

### read_line

<!-- stdlib:sig:io.read_line start -->

```yaoxiang
read_line: () -> String
```

<!-- stdlib:sig:io.read_line end -->

Читает одну строку из стандартного ввода.

Возвращает: всё содержимое прочитанной строки, **с удалённым конечным символом перевода строки**
(`\n` или `\r\n`). Ошибка: при неудачном чтении выбрасывает `E6007`.

> Интерактивные примеры не могут быть автоматически запущены в документации, приведённый ниже код
> приведён только для справки.

```yaoxiang
use std.io

main: () -> Void = {
    println("Введите ваше имя:")
    name = io.read_line()
    println("Привет, " + name)
}
```

### read_file

<!-- stdlib:sig:io.read_file start -->

```yaoxiang
read_file: (path: &String) -> String
```

<!-- stdlib:sig:io.read_file end -->

Считывает содержимое всего файла в строку за один раз.

- `path` — путь к файлу (только для чтения, заимствование)

Возвращает: всё содержимое файла. Ошибка: при отсутствии файла или отсутствии прав доступа
выбрасывает `E6007`. **Не возвращает пустую строку**.

```yaoxiang
use std.assert
use std.io
use std.os
use std.string

main: () -> Void = {
    p = "__yx_doc_read_file.txt"
    io.write_file(p, "hello")

    content = io.read_file(p)
    assert(content == "hello")

    os.remove(p)
}
```

### write_file

<!-- stdlib:sig:io.write_file start -->

```yaoxiang
write_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:io.write_file end -->

Записывает `content` в `path`, **перезаписывая** существующее содержимое; если файл не существует,
он создаётся.

- `path` — путь к файлу (только для чтения, заимствование)
- `content` — содержимое для записи (только для чтения, заимствование)

Возвращает: `true` при успешной записи. Ошибка: если каталог не существует или нет прав доступа,
выбрасывает `E6007` (не возвращает `false`).

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_write_file.txt"
    ok = io.write_file(p, "hello")
    assert(ok)
    assert(os.exists(p))
    os.remove(p)
}
```

### append_file

<!-- stdlib:sig:io.append_file start -->

```yaoxiang
append_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:io.append_file end -->

**Дописывает** `content` в конец `path`; если файл не существует, он создаётся.

Возвращает: `true` при успешной записи. Ошибка: при отсутствии прав доступа выбрасывает `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_append_file.txt"
    io.write_file(p, "hello")
    io.append_file(p, " world")
    assert(io.read_file(p) == "hello world")
    os.remove(p)
}
```

### format_fallback

<!-- stdlib:sig:io.format_fallback start -->

```yaoxiang
format_fallback: (value, type_name: &String) -> String
```

<!-- stdlib:sig:io.format_fallback end -->

Форматирует значение по имени типа, выводя представление с префиксом вида `int(42)` / `list@3`.

Это внутренняя вспомогательная функция, используемая для обратного вызова в общем пути
форматирования во время выполнения; в повседневном коде следует напрямую использовать
[`std.convert.to_string`](./convert#to_string).

- `value` — произвольное значение
- `type_name` — строка с именем типа

Возвращает: строковое представление с префиксом типа.

```yaoxiang
use std.assert
use std.io
use std.string

main: () -> Void = {
    s = io.format_fallback(42, "int")
    assert(string.contains(s, "42"))
}
```

## Связанные разделы

- [`std.os`](./os) — дескрипторы файлов, каталоги и переменные среды
- [`std.convert`](./convert) — преобразование значения в строку
