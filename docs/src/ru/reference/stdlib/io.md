---
title: 'std.io'
description: 'Стандартный вывод, стандартный ввод и форматирование'
---

# std.io

Модуль ввода-вывода. Предоставляет стандартный вывод и чтение стандартного ввода. Чтение и запись
целых файлов, операции с каталогами и путями см. в [`std.fs`](./fs); инкрементальное чтение-запись
на уровне дескрипторов см. в [`std.os`](./os).

```yaoxiang
use std.io
```

## Доступность по платформам

`read_line` зависит от системного ввода-вывода и **не экспортируется** для целевой платформы
`wasm32`. `print` / `println` / `format_fallback` доступны на всех платформах.

## Обзор функций

<!-- stdlib:table:io start -->

| Функция           | Сигнатура                               |
| ----------------- | --------------------------------------- |
| `print`           | `(...args) -> Void`                     |
| `println`         | `(...args) -> ()`                       |
| `read_line`       | `() -> String`                          |
| `format_fallback` | `(value, type_name: &String) -> String` |

<!-- stdlib:table:io end -->## Функции

### print

<!-- stdlib:sig:io.print start -->

```yaoxiang
print: (...args) -> Void
```

<!-- stdlib:sig:io.print end -->

Выводит все аргументы по порядку, **без перевода строки**. Несколько аргументов разделяются одним
пробелом.

Аргументы форматируются: `String` выводит своё содержимое напрямую; `List` / `Dict` / `Tuple`
разворачиваются рекурсивно; остальные значения выводятся как литералы.

```yaoxiang

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

То же, что [`print`](#print), но в конце вывода добавляет перевод строки.

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

Возвращает: всё содержимое прочитанной строки, **с удалённым завершающим символом перевода строки**
(`\n` или `\r\n`). Ошибка: при неудачном чтении выбрасывает `E6007`.

> Интерактивные примеры не могут быть запущены автоматически в документации; приведённый ниже код
> приведён только для справки.

```yaoxiang
use std.io

main: () -> Void = {
    println("请输入你的名字：")
    name = io.read_line()
    println("你好，" + name)
}
```

### format_fallback

<!-- stdlib:sig:io.format_fallback start -->

```yaoxiang
format_fallback: (value, type_name: &String) -> String
```

<!-- stdlib:sig:io.format_fallback end -->

Форматирует значение по имени типа, выводя представление с префиксом вида `int(42)` / `list@3`.

Это внутренняя вспомогательная функция, вызываемая общим путём форматирования среды выполнения; в
повседневном коде следует напрямую использовать [`std.convert.to_string`](./convert#to_string).

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

## Связанные модули

- [`std.fs`](./fs) — операции с файлами, каталогами и путями
- [`std.os`](./os) — дескрипторы файлов и переменные среды
- [`std.convert`](./convert) — преобразование значений в строку
