---
title: 'std.os'
description: 'Файловые дескрипторы, переменные среды и рабочий каталог'
---

# std.os

Модуль интерфейса операционной системы: чтение и запись через файловые дескрипторы, переменные среды
и рабочий каталог. Операции над путями (чтение/запись целых файлов, каталоги, метаданные,
манипуляции с путями) см. в [`std.fs`](./fs).

```yaoxiang
use std.os
```

> Все функции этого модуля зависят от возможностей ОС и **не экспортируются** для целевой платформы
> `wasm32`.

## Модель файлового дескриптора

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
> Также доступно чтение/запись с позиционированием (для чего существует `seek`):
>
> ```yaoxiang
> r = os.open(p, "r")
> os.seek(r, 6)
> tail = os.read(r, 5)     // "world"
> os.close(r)
> ```
>
> До исправления сигнатуры были без `&`, дескриптор передавался по значению → линейное владение →
> использовался лишь однажды, и `open → write → close` приводил к ошибке `E2014`.
>
> Если вы хотите избежать ручного управления дескрипторами, по-прежнему можно использовать удобные
> функции без дескрипторов: [`std.io.read_file`](./io#read_file) / [`write_file`](./io#write_file) /
> [`append_file`](./io#append_file), либо [`append_file`](#append_file) из этого модуля.

`open` возвращает **файловый дескриптор типа `Int`** (внутри движок поддерживает таблицу
дескрипторов), поэтому `File` в сигнатурах — это, по сути, `Int`.

Содержимое попадает на диск сразу после записи, без явного `close`:

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_open.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    assert(fs.read_file(p) == "hello")
    fs.remove(p)
}
```

Режимы, поддерживаемые `open`:

| Режим | Значение                                   |
| ----- | ------------------------------------------ |
| `r`   | Только чтение, файл должен существовать    |
| `w`   | Только запись, создаёт или очищает         |
| `a`   | Дописывание, создаёт или добавляет в конец |
| `r+`  | Чтение и запись, файл должен существовать  |
| `w+`  | Чтение и запись, создаёт или очищает       |
| `a+`  | Чтение и запись, создаёт или дописывает    |

## Список функций

<!-- stdlib:table:os start -->

| Функция   | Сигнатура                                 |
| --------- | ----------------------------------------- |
| `open`    | `(path: &String, mode: &String) -> File`  |
| `close`   | `(file: &File) -> Void`                   |
| `read`    | `(file: &File, n: Int) -> String`         |
| `write`   | `(file: &File, content: String) -> Int`   |
| `seek`    | `(file: &File, offset: Int) -> Bool`      |
| `tell`    | `(file: &File) -> Int`                    |
| `flush`   | `(file: &File) -> Void`                   |
| `get_env` | `(name: &String) -> String`               |
| `set_env` | `(name: &String, value: &String) -> Void` |
| `args`    | `() -> String`                            |
| `chdir`   | `(path: &String) -> Bool`                 |
| `getcwd`  | `() -> String`                            |

<!-- stdlib:table:os end -->## Операции с файлами

### open

<!-- stdlib:sig:os.open start -->

```yaoxiang
open: (path: &String, mode: &String) -> File
```

<!-- stdlib:sig:os.open end -->

Открывает файл и возвращает файловый дескриптор.

- `path` — путь к файлу (неизменяемое заимствование)
- `mode` — режим открытия, см. таблицу выше

Возвращает: дескриптор `Int`, выделенный во внутренней таблице дескрипторов. **Этот дескриптор можно
использовать только один раз** — любой нижестоящий вызов перемещает его (см.
[Модель файлового дескриптора](#модель-файлового-дескриптора)), поэтому обычно `open` встраивают в
единственный вызов.

Ошибки: при недопустимом режиме, отсутствии файла или отсутствии прав выбрасывается `E6007`.

> Дескриптор можно использовать только один раз (#337), поэтому возвращаемое значение обычно сразу
> встраивается в нижестоящий вызов.

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_open_only.txt"
    f = os.open(p, "w")
    assert(fs.exists(p))
    fs.remove(p)
}
```

### close

<!-- stdlib:sig:os.close start -->

```yaoxiang
close: (file: &File) -> Void
```

<!-- stdlib:sig:os.close end -->

Закрывает файловый дескриптор и освобождает запись в таблице.

Поскольку дескриптор можно использовать только один раз, `close` имеет смысл лишь в сценариях
«открыл и больше не использую»; содержимое попадает на диск уже при возврате из [`write`](#write),
поэтому явное закрытие обычно не требуется.

Ошибки: при недопустимом дескрипторе (не открыт или уже закрыт) выбрасывается `E6007`.

```yaoxiang
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_close.txt"
    f = os.open(p, "w")
    os.close(f)
    fs.remove(p)
}
```

### read

<!-- stdlib:sig:os.read start -->

```yaoxiang
read: (file: &File, n: Int) -> String
```

<!-- stdlib:sig:os.read end -->

Считывает **не более** `n` байт с текущей позиции чтения/записи.

- `file` — файловый дескриптор
- `n` — желаемое количество байт для чтения

Возвращает: фактически считанное содержимое (может быть короче `n`, при достижении конца файла —
пустая строка). Недопустимые байты UTF-8 возвращаются в виде символа замены, без ошибки. Ошибки: при
недопустимом дескрипторе или сбое чтения выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_read.txt"
    fs.write_file(p, "abcdef")

    part = os.read(os.open(p, "r"), 3)
    assert(part == "abc")
    fs.remove(p)
}
```

### write

<!-- stdlib:sig:os.write start -->

```yaoxiang
write: (file: &File, content: String) -> Int
```

<!-- stdlib:sig:os.write end -->

Записывает всё содержимое `content` с текущей позиции чтения/записи.

- `content` — передаётся по значению

Возвращает: **количество записанных байт**. Ошибки: при недопустимом дескрипторе или сбое записи
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_write.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    fs.remove(p)
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

Возвращает: `true` в случае успеха. Ошибки: при недопустимом дескрипторе или недопустимом смещении
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_seek.txt"
    fs.write_file(p, "abcdef")

    ok = os.seek(os.open(p, "r"), 2)
    assert(ok)
    fs.remove(p)
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
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_tell.txt"
    pos = os.tell(os.open(p, "w"))
    assert(pos == 0)
    fs.remove(p)
}
```

### flush

<!-- stdlib:sig:os.flush start -->

```yaoxiang
flush: (file: &File) -> Void
```

<!-- stdlib:sig:os.flush end -->

Сбрасывает буферизованное содержимое на диск.

Ошибки: при недопустимом дескрипторе или сбое сброса выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.fs
use std.os

main: () -> Void = {
    p = "__yx_doc_flush.txt"
    os.flush(os.open(p, "w"))
    assert(fs.exists(p))
    fs.remove(p)
}
```

## Переменные среды

### get_env

<!-- stdlib:sig:os.get_env start -->

```yaoxiang
get_env: (name: &String) -> String
```

<!-- stdlib:sig:os.get_env end -->

Считывает значение переменной среды.

Возвращает: значение переменной; **если переменная не существует — возвращается пустая строка** (без
ошибки). Поэтому отличить «не установлена» от «установлена в пустую строку» невозможно.

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

Возвращает: все argv, объединённые в одну строку через **`\n`** (а не `List`). Первый элемент — путь
к самой программе.

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

Возвращает: `true` в случае успеха. Ошибки: если каталог не существует, выбрасывается `E6007`.

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

Ошибки: если получить значение не удалось, выбрасывается `E6007`.

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

- [`std.fs`](./fs) — операции над путями, файлами и каталогами
- [Справочник по кодам ошибок](../error-code/) — общая ошибка времени выполнения `E6007`
