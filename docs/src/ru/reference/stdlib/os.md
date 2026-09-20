---
title: 'std.os'
description: 'Файловые дескрипторы, каталоги, переменные среды и рабочий каталог'
---

# std.os

Модуль интерфейса операционной системы: чтение/запись файловых дескрипторов, операции с каталогами,
переменные среды и рабочий каталог.

```yaoxiang
use std.os
```

> Все функции данного модуля зависят от возможностей операционной системы и **не экспортируются**
> для целевой платформы `wasm32`.

## Модель файловых дескрипторов

> **Дескриптор передаётся по ссылке (исправлено в #337)**: сигнатуры `read` / `write` / `seek` /
> `tell` / `flush` / `close` имеют вид `(file: &File, ...)`, поэтому дескриптор можно использовать
> многократно:
>
> ```yaoxiang
> f = os.open(p, "w")
> os.write(f, "hello world")
> os.close(f)
> ```
>
> Чтение/запись после позиционирования (для чего и существует `seek`) также доступно:
>
> ```yaoxiang
> r = os.open(p, "r")
> os.seek(r, 6)
> tail = os.read(r, 5)     // "world"
> os.close(r)
> ```
>
> До исправления сигнатуры не имели `&`, дескриптор передавался по значению → линейное владение →
> использовался лишь однократно, `open → write → close` приводил к ошибке `E2014`.
>
> Если хотите избежать ручного управления дескриптором, можно использовать удобные функции без
> дескриптора: [`std.io.read_file`](./io#read_file) / [`write_file`](./io#write_file) /
> [`append_file`](./io#append_file), либо [`append_file`](#append_file) из данного модуля.

`open` возвращает **дескриптор файла типа `Int`** (движок внутри поддерживает таблицу дескрипторов),
поэтому `File` в сигнатурах фактически является `Int`.

Содержимое записывается на диск сразу после `write`, явный вызов `close` не требуется:

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_open.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    assert(io.read_file(p) == "hello")
    os.remove(p)
}
```

`open` поддерживает следующие режимы:

| Режим | Значение                                  |
| ----- | ----------------------------------------- |
| `r`   | Только чтение, файл должен существовать   |
| `w`   | Только запись, создать или очистить       |
| `a`   | Дописывание, создать или дописать в конец |
| `r+`  | Чтение и запись, файл должен существовать |
| `w+`  | Чтение и запись, создать или очистить     |
| `a+`  | Чтение и запись, создать или дописать     |

## Список функций

<!-- stdlib:table:os start -->

| Функция       | Сигнатура                                   |
| ------------- | ------------------------------------------- |
| `open`        | `(path: &String, mode: &String) -> File`    |
| `close`       | `(file: &File) -> Void`                     |
| `read`        | `(file: &File, n: Int) -> String`           |
| `write`       | `(file: &File, content: String) -> Int`     |
| `seek`        | `(file: &File, offset: Int) -> Bool`        |
| `tell`        | `(file: &File) -> Int`                      |
| `flush`       | `(file: &File) -> Void`                     |
| `mkdir`       | `(path: &String) -> Bool`                   |
| `rmdir`       | `(path: &String) -> Bool`                   |
| `read_dir`    | `(path: &String) -> String`                 |
| `remove`      | `(path: &String) -> Bool`                   |
| `exists`      | `(path: &String) -> Bool`                   |
| `is_file`     | `(path: &String) -> Bool`                   |
| `is_dir`      | `(path: &String) -> Bool`                   |
| `copy`        | `(src: &String, dst: &String) -> Bool`      |
| `rename`      | `(old: &String, new: &String) -> Bool`      |
| `get_env`     | `(name: &String) -> String`                 |
| `set_env`     | `(name: &String, value: &String) -> Void`   |
| `args`        | `() -> String`                              |
| `chdir`       | `(path: &String) -> Bool`                   |
| `getcwd`      | `() -> String`                              |
| `append_file` | `(path: &String, content: &String) -> Bool` |

<!-- stdlib:table:os end -->## Файловые операции

### open

<!-- stdlib:sig:os.open start -->

```yaoxiang
open: (path: &String, mode: &String) -> File
```

<!-- stdlib:sig:os.open end -->

Открывает файл и возвращает файловый дескриптор.

- `path` — путь к файлу (неизменяемое заимствование)
- `mode` — режим открытия, см. таблицу выше

Возвращает: дескриптор типа `Int`, выделенный во внутренней таблице дескрипторов. **Данный
дескриптор может быть использован только один раз** — любой нижестоящий вызов перемещает его (см.
[Модель файловых дескрипторов](#модель-файловых-дескрипторов)), поэтому обычно `open` встраивают
прямо в одиночный вызов.

Ошибки: при недопустимом режиме, отсутствии файла или отсутствии прав выбрасывается `E6007`.

> Дескриптор может быть использован только один раз (#337), поэтому возвращаемое значение обычно
> встраивается прямо в нижестоящий вызов.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    p = "__yx_doc_open_only.txt"
    f = os.open(p, "w")
    assert(os.exists(p))
    os.remove(p)
}
```

### close

<!-- stdlib:sig:os.close start -->

```yaoxiang
close: (file: &File) -> Void
```

<!-- stdlib:sig:os.close end -->

Закрывает файловый дескриптор и освобождает запись в таблице.

Поскольку дескриптор может быть использован только один раз, `close` имеет смысл лишь в сценарии
«открыл и больше нигде не использую»; содержимое записи уже сохраняется на диск к моменту возврата
из [`write`](#write), обычно явное закрытие не требуется.

Ошибки: при недопустимом дескрипторе (не открыт или уже закрыт) выбрасывается `E6007`.

```yaoxiang
use std.os

main: () -> Void = {
    p = "__yx_doc_close.txt"
    f = os.open(p, "w")
    os.close(f)
    os.remove(p)
}
```

### read

<!-- stdlib:sig:os.read start -->

```yaoxiang
read: (file: &File, n: Int) -> String
```

<!-- stdlib:sig:os.read end -->

Считывает с текущей позиции чтения/записи **не более** `n` байт.

- `file` — файловый дескриптор
- `n` — желаемое количество байт для чтения

Возвращает: фактически прочитанное содержимое (может быть короче `n`, при достижении конца файла —
пустая строка). Недопустимые байты UTF-8 возвращаются в виде символа замены, ошибка не
выбрасывается. Ошибки: при недопустимом дескрипторе или ошибке чтения выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_read.txt"
    io.write_file(p, "abcdef")

    part = os.read(os.open(p, "r"), 3)
    assert(part == "abc")
    os.remove(p)
}
```

### write

<!-- stdlib:sig:os.write start -->

```yaoxiang
write: (file: &File, content: String) -> Int
```

<!-- stdlib:sig:os.write end -->

Записывает всё содержимое `content` в текущую позицию чтения/записи.

- `content` — передаётся по значению

Возвращает: количество записанных **байт**. Ошибки: при недопустимом дескрипторе или ошибке записи
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    p = "__yx_doc_write.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    os.remove(p)
}
```

### seek

<!-- stdlib:sig:os.seek start -->

```yaoxiang
seek: (file: &File, offset: Int) -> Bool
```

<!-- stdlib:sig:os.seek end -->

Перемещает позицию чтения/записи на **абсолютное** смещение `offset` (относительно начала файла).

- `offset` — целевое смещение в байтах, должно быть неотрицательным

Возвращает: `true` при успехе. Ошибки: при недопустимом дескрипторе или недопустимом смещении
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_seek.txt"
    io.write_file(p, "abcdef")

    ok = os.seek(os.open(p, "r"), 2)
    assert(ok)
    os.remove(p)
}
```

### tell

<!-- stdlib:sig:os.tell start -->

```yaoxiang
tell: (file: &File) -> Int
```

<!-- stdlib:sig:os.tell end -->

Возвращает текущее смещение позиции чтения/записи в байтах.

Ошибки: при недопустимом дескрипторе выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    p = "__yx_doc_tell.txt"
    pos = os.tell(os.open(p, "w"))
    assert(pos == 0)
    os.remove(p)
}
```

### flush

<!-- stdlib:sig:os.flush start -->

```yaoxiang
flush: (file: &File) -> Void
```

<!-- stdlib:sig:os.flush end -->

Сбрасывает буферизованное содержимое на диск.

Ошибки: при недопустимом дескрипторе или ошибке сброса выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    p = "__yx_doc_flush.txt"
    os.flush(os.open(p, "w"))
    assert(os.exists(p))
    os.remove(p)
}
```

## Операции с каталогами

### mkdir

<!-- stdlib:sig:os.mkdir start -->

```yaoxiang
mkdir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.mkdir end -->

Создаёт **одноуровневый** каталог (без рекурсивного создания родительских каталогов).

Возвращает: `true` при успехе. Ошибки: если родительский каталог не существует или каталог уже
существует, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    d = "__yx_doc_mkdir"
    assert(os.mkdir(d))
    assert(os.is_dir(d))
    os.rmdir(d)
}
```

### rmdir

<!-- stdlib:sig:os.rmdir start -->

```yaoxiang
rmdir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.rmdir end -->

Удаляет **пустой** каталог.

Возвращает: `true` при успехе. Ошибки: если каталог не существует или не пуст, выбрасывается
`E6007`.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    d = "__yx_doc_rmdir"
    os.mkdir(d)
    assert(os.rmdir(d))
    assert(!os.exists(d))
}
```

### read_dir

<!-- stdlib:sig:os.read_dir start -->

```yaoxiang
read_dir: (path: &String) -> String
```

<!-- stdlib:sig:os.read_dir end -->

Перечисляет имена элементов каталога.

Возвращает: единственную строку с именами записей, соединёнными через **`\n`** (не `List`). Ошибки:
если каталог не существует или нет прав доступа, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    d = "__yx_doc_read_dir"
    os.mkdir(d)
    names = os.read_dir(d)
    // Пустой каталог возвращает пустую строку
    assert(string.is_empty(names))
    os.rmdir(d)
}
```

## Утилиты для путей и файлов

### remove

<!-- stdlib:sig:os.remove start -->

```yaoxiang
remove: (path: &String) -> Bool
```

<!-- stdlib:sig:os.remove end -->

Удаляет файл, семантически эквивалентно `remove_file` (**не может удалить каталог**, для удаления
каталогов используйте [`rmdir`](#rmdir)).

Возвращает: `true` при успехе. Ошибки: если файл не существует или путь является каталогом,
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_remove.txt"
    io.write_file(p, "x")
    assert(os.remove(p))
    assert(!os.exists(p))
}
```

### exists

<!-- stdlib:sig:os.exists start -->

```yaoxiang
exists: (path: &String) -> Bool
```

<!-- stdlib:sig:os.exists end -->

Существует ли путь (файл или каталог). **Не выбрасывает ошибку**, при отсутствии возвращает `false`.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    assert(os.exists("."))
    assert(!os.exists("__yx_definitely_missing_path__"))
}
```

### is_file

<!-- stdlib:sig:os.is_file start -->

```yaoxiang
is_file: (path: &String) -> Bool
```

<!-- stdlib:sig:os.is_file end -->

Является ли путь **обычным файлом**. Для каталога возвращает `false`, для несуществующего пути —
`false`.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    assert(!os.is_file("."))
}
```

### is_dir

<!-- stdlib:sig:os.is_dir start -->

```yaoxiang
is_dir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.is_dir end -->

Является ли путь **каталогом**. Для файла возвращает `false`, для несуществующего пути — `false`.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    assert(os.is_dir("."))
}
```

### copy

<!-- stdlib:sig:os.copy start -->

```yaoxiang
copy: (src: &String, dst: &String) -> Bool
```

<!-- stdlib:sig:os.copy end -->

Копирует файл. Если целевой файл существует, он **перезаписывается**.

Возвращает: `true` при успехе. Ошибки: если исходный файл не существует или нет прав доступа,
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    a = "__yx_doc_copy_a.txt"
    b = "__yx_doc_copy_b.txt"
    io.write_file(a, "data")
    assert(os.copy(a, b))
    assert(io.read_file(b) == "data")
    os.remove(a)
    os.remove(b)
}
```

### rename

<!-- stdlib:sig:os.rename start -->

```yaoxiang
rename: (old: &String, new: &String) -> Bool
```

<!-- stdlib:sig:os.rename end -->

Переименовывает или перемещает файл.

Возвращает: `true` при успехе. Ошибки: если исходный файл не существует или целевой уже существует,
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    a = "__yx_doc_rename_a.txt"
    b = "__yx_doc_rename_b.txt"
    io.write_file(a, "data")
    assert(os.rename(a, b))
    assert(os.exists(b))
    os.remove(b)
}
```

### append_file

<!-- stdlib:sig:os.append_file start -->

```yaoxiang
append_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:os.append_file end -->

Дописывание содержимого (удобная функция без дескриптора). Если файл не существует, он создаётся.

Возвращает: `true` при успехе. Ошибки: при отсутствии прав доступа выбрасывается `E6007`.

> Это одноимённый аналог [`std.io.append_file`](./io#append_file), оба модуля предоставляют его,
> поведение одинаково.

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_os_append.txt"
    io.write_file(p, "a")
    os.append_file(p, "b")
    assert(io.read_file(p) == "ab")
    os.remove(p)
}
```

## Переменные среды

### get_env

<!-- stdlib:sig:os.get_env start -->

```yaoxiang
get_env: (name: &String) -> String
```

<!-- stdlib:sig:os.get_env end -->

Читает переменную среды.

Возвращает: значение переменной; **если переменная не установлена, возвращается пустая строка** (без
ошибки). Поэтому невозможно отличить «не установлено» от «установлено в пустую строку».

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    // PATH гарантированно существует на основных платформах
    path = os.get_env("PATH")
    assert(string.len(path) > 0)

    // Несуществующая переменная возвращает пустую строку
    assert(string.is_empty(os.get_env("__YX_DEFINITELY_MISSING__")))
}
```

### set_env

<!-- stdlib:sig:os.set_env start -->

```yaoxiang
set_env: (name: &String, value: &String) -> Void
```

<!-- stdlib:sig:os.set_env end -->

Устанавливает переменную среды (влияет на текущий процесс).

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    os.set_env("__YX_DOC_ENV", "hello")
    assert(os.get_env("__YX_DOC_ENV") == "hello")
}
```

## Процесс и рабочий каталог

### args

<!-- stdlib:sig:os.args start -->

```yaoxiang
args: () -> String
```

<!-- stdlib:sig:os.args end -->

Возвращает аргументы командной строки.

Возвращает: единственную строку со всеми argv, соединёнными через **`\n`** (не `List`). Первый
элемент — путь к самой программе.

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    argv = os.args()
    assert(string.len(argv) > 0)
}
```

### chdir

<!-- stdlib:sig:os.chdir start -->

```yaoxiang
chdir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.chdir end -->

Переключает текущий рабочий каталог.

Возвращает: `true` при успехе. Ошибки: если каталог не существует, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    before = os.getcwd()
    assert(os.chdir(".."))
    assert(os.chdir(before))     // Вернуться обратно
    assert(os.getcwd() == before)
}
```

### getcwd

<!-- stdlib:sig:os.getcwd start -->

```yaoxiang
getcwd: () -> String
```

<!-- stdlib:sig:os.getcwd end -->

Возвращает абсолютный путь текущего рабочего каталога.

Ошибки: если не удаётся получить путь, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    cwd = os.getcwd()
    assert(string.len(cwd) > 0)
}
```

## Связанные материалы

- [`std.io`](./io) — удобные функции чтения/записи целых файлов
- [Справочник по кодам ошибок](../error-code/) — общая ошибка времени выполнения `E6007`
