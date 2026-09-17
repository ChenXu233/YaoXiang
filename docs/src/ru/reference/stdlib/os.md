---
title: 'std.os'
description: 'Файловые дескрипторы, каталоги, переменные окружения и рабочий каталог'
---

# std.os

Модуль интерфейса операционной системы: чтение и запись файловых дескрипторов, операции с
каталогами, переменные окружения и рабочий каталог.

```yaoxiang
use std.os
```

> Все функции этого модуля зависят от возможностей операционной системы и **не экспортируются** для
> целевой платформы `wasm32`.

## Модель файлового дескриптора

> **Важное ограничение (#337)**: дескриптор, возвращаемый `open`, является **одноразовым**. У него
> нет `&`, поэтому при первом же вызове `read` / `write` / `seek` / `tell` / `flush` / `close` он
> **перемещается** и больше не может быть использован. Следовательно, распространённая запись `open`
> → `write` → `close` в текущей реализации **не скомпилируется** (будет выдана ошибка `E2014`).
>
> Возможны два способа записи:
>
> 1. **Встроить `open` непосредственно в одиночный вызов** — дескриптор создаётся и тут же
>    потребляется:
>
>    ```yaoxiang
>    n = os.write(os.open(p, "w"), "hello")
>    ```
>
> 2. **Использовать удобные функции без дескрипторов** — [`std.io.read_file`](./io#read_file) /
>    [`write_file`](./io#write_file) / [`append_file`](./io#append_file) или
>    [`append_file`](#append_file) данного модуля.
>
> Дескриптор существует в виде записи в таблице дескрипторов и освобождается при завершении
> процесса; поскольку после однократного использования ссылаться на него нельзя, явный вызов `close`
> в большинстве сценариев записать невозможно (но см. ниже форму одиночного вызова).

`open` возвращает **дескриптор файла типа `Int`** (внутри движок поддерживает таблицу дескрипторов),
поэтому `File` в сигнатурах фактически является `Int`.

Содержимое сбрасывается на диск сразу после записи, без необходимости явного `close`:

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    p = "__yx_doc_open.txt"
    n = os.write(os.open(p, "w"), "hello")
    assert(n == 5)
    assert(io.read_file(p) == "hello")
    os.remove(p)
}
```

Режимы, поддерживаемые `open`:

| Режим | Значение                                    |
| ----- | ------------------------------------------- |
| `r`   | Только чтение, файл должен существовать     |
| `w`   | Только запись, создание или очистка         |
| `a`   | Добавление, создание или добавление в конец |
| `r+`  | Чтение и запись, файл должен существовать   |
| `w+`  | Чтение и запись, создание или очистка       |
| `a+`  | Чтение и запись, создание или добавление    |

## Список функций

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

Возвращает: дескриптор типа `Int`, выделенный во внутренней таблице дескрипторов. **Этот дескриптор
может быть использован только один раз** — любой вызов нижестоящей функции переместит его (см.
[Модель файлового дескриптора](#модель-файлового-дескриптора)), поэтому обычно `open` встраивается
непосредственно в одиночный вызов.

Ошибки: при недопустимом режиме, отсутствии файла или отсутствии прав выбрасывается `E6007`.

> Дескриптор может быть использован только один раз (#337), поэтому возвращаемое значение обычно
> встраивается непосредственно в нижестоящий вызов.

```yaoxiang
use std.assert
use std.os

main = {
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

Поскольку дескриптор может быть использован только один раз, `close` имеет смысл только в сценариях
«открыл и больше ничего с ним не делаю»; содержимое записи уже сбрасывается на диск при возврате из
[`write`](#write), и явное закрытие обычно не требуется.

Ошибки: при недопустимом дескрипторе (не открыт или уже закрыт) выбрасывается `E6007`.

```yaoxiang
use std.os

main = {
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
- `n` — желаемое количество байт для чтения

Возвращает: фактически прочитанное содержимое (может быть короче `n`, при достижении конца файла —
пустая строка). Недопустимые байты UTF-8 возвращаются в виде символа замены, без ошибки. Ошибки: при
недопустимом дескрипторе или сбое чтения выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main = {
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

Возвращает: количество **записанных байт**. Ошибки: при недопустимом дескрипторе или сбое записи
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os

main = {
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

Возвращает: `true` при успехе. Ошибки: при недопустимом дескрипторе или недопустимом смещении
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main = {
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

Возвращает текущее смещение позиции чтения/записи в байтах.

Ошибки: при недопустимом дескрипторе выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os

main = {
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

main = {
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

Создаёт **одноуровневый** каталог (родительские каталоги не создаются рекурсивно).

Возвращает: `true` при успехе. Ошибки: при отсутствии родительского каталога или если каталог уже
существует выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os

main = {
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

Возвращает: `true` при успехе. Ошибки: при отсутствии каталога или если он не пуст, выбрасывается
`E6007`.

```yaoxiang
use std.assert
use std.os

main = {
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

Перечисляет имена элементов в каталоге.

Возвращает: одна строка, в которой имена элементов соединены символом **`\n`** (не `List`). Ошибки:
при отсутствии каталога или отсутствии прав выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os
use std.string

main = {
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

Удаляет файл, семантически эквивалентно `remove_file` (**не может удалять каталоги**; для удаления
каталогов используйте [`rmdir`](#rmdir)).

Возвращает: `true` при успехе. Ошибки: при отсутствии файла или если путь указывает на каталог,
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main = {
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

Проверяет существование пути (подходит и для файла, и для каталога). **Не выбрасывает ошибку**,
возвращает `false`, если путь не существует.

```yaoxiang
use std.assert
use std.os

main = {
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

main = {
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

main = {
    assert(os.is_dir("."))
}
```

### copy

<!-- stdlib:sig:os.copy start -->

```yaoxiang
copy: (src: &String, dst: &String) -> Bool
```

<!-- stdlib:sig:os.copy end -->

Копирует файл. **Перезаписывает** существующий целевой файл.

Возвращает: `true` при успехе. Ошибки: при отсутствии исходного файла или отсутствии прав
выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main = {
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

Возвращает: `true` при успехе. Ошибки: при отсутствии исходного файла или существовании целевого
файла выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.io
use std.os

main = {
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

Запись с добавлением (удобная функция без дескриптора). Создаёт файл, если он не существует.

Возвращает: `true` при успехе. Ошибки: при отсутствии прав выбрасывается `E6007`.

> Это одноимённый аналог [`std.io.append_file`](./io#append_file); оба модуля предоставляют её,
> поведение совпадает.

```yaoxiang
use std.assert
use std.io
use std.os

main = {
    p = "__yx_doc_os_append.txt"
    io.write_file(p, "a")
    os.append_file(p, "b")
    assert(io.read_file(p) == "ab")
    os.remove(p)
}
```

## Переменные окружения

### get_env

<!-- stdlib:sig:os.get_env start -->

```yaoxiang
get_env: (name: &String) -> String
```

<!-- stdlib:sig:os.get_env end -->

Читает переменную окружения.

Возвращает: значение переменной; **если переменная не задана, возвращается пустая строка** (без
ошибки). Поэтому невозможно отличить «не задана» от «задана как пустая строка».

```yaoxiang
use std.assert
use std.os
use std.string

main = {
    // PATH заведомо существует на всех основных платформах
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

Устанавливает переменную окружения (влияет на текущий процесс).

```yaoxiang
use std.assert
use std.os

main = {
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

Возвращает: одна строка, в которой все argv соединены символом **`\n`** (не `List`). Первым
элементом идёт путь к самой программе.

```yaoxiang
use std.assert
use std.os
use std.string

main = {
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

Возвращает: `true` при успехе. Ошибки: при отсутствии каталога выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os

main = {
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

Ошибки: при невозможности получить путь выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.os
use std.string

main = {
    cwd = os.getcwd()
    assert(string.len(cwd) > 0)
}
```

## Связанные разделы

- [`std.io`](./io) — удобные функции чтения/записи целого файла
- [Справочник по кодам ошибок](../error-code/) — `E6007` общая ошибка времени выполнения
