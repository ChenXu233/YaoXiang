---
title: 'std.os'
description: 'Файловые дескрипторы, каталоги, переменные среды и рабочий каталог'
---

# std.os

Модуль интерфейса операционной системы: чтение и запись файловых дескрипторов, операции с
каталогами, переменные среды и рабочий каталог.

```yaoxiang
use std.os
```

> Все функции данного модуля зависят от возможностей операционной системы и **не экспортируются**
> для целевой платформы `wasm32`.

## Модель файловых дескрипторов

> **Важное ограничение (#337)**: дескриптор, возвращаемый `open`, является **одноразовым**. У него
> нет `&`, поэтому при первой передаче в `read` / `write` / `seek` / `tell` / `flush` / `close` он
> **перемещается**, после чего использовать его повторно нельзя. Следовательно, распространённая
> запись `open` → `write` → `close` в текущей реализации **не компилируется** (будет выдана ошибка
> `E2014`).
>
> Допустимы два способа записи:
>
> 1. **Встроить `open` в однократный вызов** — дескриптор сразу же потребляется после создания:
>
>    ```yaoxiang
>    n = os.write(os.open(p, "w"), "hello")
>    ```
>
> 2. **Использовать удобные функции, не открывающие дескриптор** —
>    [`std.io.read_file`](./io#read_file) / [`write_file`](./io#write_file) /
>    [`append_file`](./io#append_file), либо [`append_file`](#append_file) из данного модуля.
>
> Дескриптор существует в виде записи в таблице дескрипторов и освобождается при завершении
> процесса; поскольку после однократного использования на него нельзя ссылаться повторно, явный
> `close` в большинстве сценариев записать невозможно (но см. ниже форму однократного вызова).

`open` возвращает **файловый дескриптор типа `Int`** (внутри движка поддерживается таблица
дескрипторов), поэтому `File` в сигнатурах фактически является `Int`.

Содержимое записывается на диск сразу же, без необходимости явного `close`:

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

Поддерживаемые режимы `open`:

| Режим | Значение                                     |
| ----- | -------------------------------------------- |
| `r`   | Только чтение, файл должен существовать      |
| `w`   | Только запись, создание или очистка          |
| `a`   | Дописывание, создание или добавление в конец |
| `r+`  | Чтение и запись, файл должен существовать    |
| `w+`  | Чтение и запись, создание или очистка        |
| `a+`  | Чтение и запись, создание или добавление     |

## Сводная таблица функций

<!-- stdlib:table:os start -->

| Функция       | Сигнатура                                   |
| ------------- | ------------------------------------------- |
| `open`        | `(path: &String, mode: &String) -> File`    |
| `close`       | `(file: File) -> Void`                      |
| `read`        | `(file: File, n: Int) -> String`            |
| `write`       | `(file: File, content: String) -> Int`      |
| `seek`        | `(file: File, offset: Int) -> Bool`         |
| `tell`        | `(file: File) -> Int`                       |
| `flush`       | `(file: File) -> Void`                      |
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

Возвращает: дескриптор `Int`, выделенный во внутренней таблице дескрипторов. **Дескриптор может быть
использован только один раз** — любой нижестоящий вызов перемещает его (см.
[Модель файловых дескрипторов](#模型-файловых-дескрипторов)), поэтому обычно `open` встраивается в
однократный вызов.

Ошибки: при недопустимом режиме, отсутствии файла или отсутствии прав выбрасывается `E6007`.

> Дескриптор может быть использован только один раз (#337), поэтому возвращаемое значение обычно
> встраивается напрямую в нижестоящий вызов.

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
close: (file: File) -> Void
```

<!-- stdlib:sig:os.close end -->

Закрывает файловый дескриптор и освобождает запись в таблице.

Поскольку дескриптор может быть использован только один раз, `close` имеет смысл лишь в сценарии
«открыть и больше не использовать»; содержимое, записываемое через [`write`](#write), попадает на
диск к моменту возврата, поэтому явное закрытие обычно не требуется.

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
read: (file: File, n: Int) -> String
```

<!-- stdlib:sig:os.read end -->

Читает **не более** `n` байт с текущей позиции чтения/записи.

- `file` — файловый дескриптор
- `n` — желаемое число байт для чтения

Возвращает: фактически прочитанное содержимое (может быть короче `n`, при достижении конца файла —
пустая строка). Недопустимые байты UTF-8 возвращаются в виде символа замены, без ошибки. Ошибки: при
недопустимом дескрипторе или сбое чтения выбрасывается `E6007`.

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
write: (file: File, content: String) -> Int
```

<!-- stdlib:sig:os.write end -->

Записывает всё содержимое `content` в текущую позицию чтения/записи.

- `content` — передаётся по значению

Возвращает: число **записанных байт**. Ошибки: при недопустимом дескрипторе или сбое записи
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
seek: (file: File, offset: Int) -> Bool
```

<!-- stdlib:sig:os.seek end -->

Перемещает позицию чтения/записи на **абсолютное** смещение `offset` (относительно начала файла).

- `offset` — целевое смещение в байтах, должно быть неотрицательным

Возвращает: `true` в случае успеха. Ошибки: при недопустимом дескрипторе или недопустимом смещении
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
tell: (file: File) -> Int
```

<!-- stdlib:sig:os.tell end -->

Возвращает смещение в байтах текущей позиции чтения/записи.

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
flush: (file: File) -> Void
```

<!-- stdlib:sig:os.flush end -->

Сбрасывает буферизованное содержимое на диск.

Ошибки: при недопустимом дескрипторе или сбое сброса выбрасывается `E6007`.

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

Возвращает: `true` в случае успеха. Ошибки: при отсутствии родительского каталога или его
существовании выбрасывается `E6007`.

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

Возвращает: `true` в случае успеха. Ошибки: при отсутствии каталога или его непустоте выбрасывается
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

Возвращает: одна строка, в которой имена элементов соединены через **`\n`** (не `List`). Ошибки: при
отсутствии каталога или отсутствии прав выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    d = "__yx_doc_read_dir"
    os.mkdir(d)
    names = os.read_dir(d)
    // пустой каталог возвращает пустую строку
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

Удаляет файл, по семантике эквивалентно `remove_file` (**не может удалять каталоги**; для каталогов
используйте [`rmdir`](#rmdir)).

Возвращает: `true` в случае успеха. Ошибки: при отсутствии файла или если путь указывает на каталог
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

Существует ли путь (подходят и файл, и каталог). **Ошибки не возникают**; при отсутствии
возвращается `false`.

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

Является ли путь **обычным файлом**. Для каталога возвращается `false`, для отсутствующего пути —
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

Является ли путь **каталогом**. Для файла возвращается `false`, для отсутствующего пути — `false`.

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

Копирует файл. Если файл назначения уже существует, он **перезаписывается**.

Возвращает: `true` в случае успеха. Ошибки: при отсутствии исходного файла или отсутствии прав
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

Возвращает: `true` в случае успеха. Ошибки: при отсутствии исходного файла или существовании файла
назначения выбрасывается `E6007`.

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

Дописывает содержимое (удобная функция, не открывающая дескриптор). Если файл не существует —
создаёт его.

Возвращает: `true` в случае успеха. Ошибки: при отсутствии прав выбрасывается `E6007`.

> Это одноимённый аналог [`std.io.append_file`](./io#append_file); оба модуля предоставляют функцию
> с идентичным поведением.

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

Возвращает: значение переменной; **если переменная не существует — возвращается пустая строка** (без
ошибки). Поэтому невозможно отличить «не задана» от «задана пустой строкой».

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    // PATH гарантированно существует на всех основных платформах
    path = os.get_env("PATH")
    assert(string.len(path) > 0)

    // несуществующая переменная возвращает пустую строку
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

Возвращает: одна строка, в которой все argv соединены через **`\n`** (не `List`). Первый элемент —
путь к самой программе.

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

Возвращает: `true` в случае успеха. Ошибки: при отсутствии каталога выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    before = os.getcwd()
    assert(os.chdir(".."))
    assert(os.chdir(before))     // вернуться обратно
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

Ошибки: при невозможности получить значение выбрасывается `E6007`.

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
