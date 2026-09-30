---
title: 'std.fs'
description: 'Файлы, каталоги и операции с путями'
---

# std.fs

Файловые операции на уровне путей: чтение и запись целых файлов, управление каталогами и их обход,
метаданные файлов, временные файлы и операции над путями.

Для инкрементального чтения/записи по дескриптору (open/read/seek) используйте [`std.os`](./os);
консольный ввод-вывод — [`std.io`](./io).

```yaoxiang
use std.fs
```

## Доступность платформ

Данный модуль **не экспортируется** для целевой платформы `wasm32` (отсутствует семантика файловой
системы). `mkdtemp` / `tmpfile` / `temp_dir` зависят от безопасного создания временных файлов и
доступны только на нативных целевых платформах.

## Список функций

<!-- stdlib:table:fs start -->

| Функция          | Сигнатура                                   |
| ---------------- | ------------------------------------------- |
| `read_file`      | `(path: &String) -> String`                 |
| `write_file`     | `(path: &String, content: &String) -> Bool` |
| `append_file`    | `(path: &String, content: &String) -> Bool` |
| `exists`         | `(path: &String) -> Bool`                   |
| `is_file`        | `(path: &String) -> Bool`                   |
| `is_dir`         | `(path: &String) -> Bool`                   |
| `mkdir`          | `(path: &String) -> Bool`                   |
| `mkdir_all`      | `(path: &String) -> Bool`                   |
| `rmdir`          | `(path: &String) -> Bool`                   |
| `remove`         | `(path: &String) -> Bool`                   |
| `copy`           | `(src: &String, dst: &String) -> Bool`      |
| `rename`         | `(src: &String, dst: &String) -> Bool`      |
| `read_dir`       | `(path: &String) -> Vec(String)`            |
| `walk`           | `(path: &String) -> Vec(String)`            |
| `stat`           | `(path: &String) -> Dict(String, Any)`      |
| `temp_dir`       | `() -> String`                              |
| `mkdtemp`        | `(prefix: &String) -> String`               |
| `tmpfile`        | `(prefix: &String) -> String`               |
| `path_join`      | `(base: &String, rel: &String) -> String`   |
| `path_basename`  | `(path: &String) -> String`                 |
| `path_dirname`   | `(path: &String) -> String`                 |
| `path_extension` | `(path: &String) -> String`                 |

<!-- stdlib:table:fs end -->## Функции

### read_file

<!-- stdlib:sig:fs.read_file start -->

```yaoxiang
read_file: (path: &String) -> String
```

<!-- stdlib:sig:fs.read_file end -->

Читает всё содержимое файла в строку за один вызов.

- `path` — путь к файлу (неизменяемое заимствование)

Возвращает: полное содержимое файла. Ошибки: если файл не существует или нет прав доступа,
выбрасывается `E6007`. **Не возвращает пустую строку**.

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_read_file.txt"
    fs.write_file(p, "hello")

    content = fs.read_file(p)
    assert(content == "hello")

    fs.remove(p)
}
```

### write_file

<!-- stdlib:sig:fs.write_file start -->

```yaoxiang
write_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:fs.write_file end -->

Записывает `content` в `path`, **перезаписывая** прежнее содержимое; если файл не существует —
создаёт его.

- `path` — путь к файлу (неизменяемое заимствование)
- `content` — записываемое содержимое (неизменяемое заимствование)

Возвращает: `true` при успешной записи. Ошибки: если каталог не существует или нет прав доступа,
выбрасывается `E6007` (а не возвращается `false`).

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_write_file.txt"
    ok = fs.write_file(p, "hello")
    assert(ok)
    assert(fs.exists(p))
    fs.remove(p)
}
```

### append_file

<!-- stdlib:sig:fs.append_file start -->

```yaoxiang
append_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:fs.append_file end -->

**Дописывает** `content` в конец `path`; если файл не существует — создаёт его.

Возвращает: `true` при успешной записи. Ошибки: при отсутствии прав доступа выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_append_file.txt"
    fs.write_file(p, "hello")
    fs.append_file(p, " world")
    assert(fs.read_file(p) == "hello world")
    fs.remove(p)
}
```

### exists

<!-- stdlib:sig:fs.exists start -->

```yaoxiang
exists: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.exists end -->

Существует ли путь (как для файла, так и для каталога).

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_exists.txt"
    assert(!fs.exists(p), "absent before write")
    fs.write_file(p, "x")
    assert(fs.exists(p), "present after write")
    fs.remove(p)
}
```

### is_file

<!-- stdlib:sig:fs.is_file start -->

```yaoxiang
is_file: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.is_file end -->

Указывает ли путь на обычный файл. Если путь не существует, возвращает `false` (без ошибки).

### is_dir

<!-- stdlib:sig:fs.is_dir start -->

```yaoxiang
is_dir: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.is_dir end -->

Указывает ли путь на каталог. Если путь не существует, возвращает `false` (без ошибки).

### mkdir

<!-- stdlib:sig:fs.mkdir start -->

```yaoxiang
mkdir: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.mkdir end -->

Создаёт каталог **одного уровня**; родительский каталог должен уже существовать. Для рекурсивного
создания используйте [`mkdir_all`](#mkdir_all).

Возвращает: `true` при успехе. Ошибки: если родительский каталог отсутствует или каталог уже
существует, выбрасывается `E6007`.

### mkdir_all

<!-- stdlib:sig:fs.mkdir_all start -->

```yaoxiang
mkdir_all: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.mkdir_all end -->

Рекурсивно создаёт каталог, включая все недостающие родительские каталоги; если каталог уже
существует — считается успехом.

Возвращает: `true` при успехе. Ошибки: при отсутствии прав доступа выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    d = "__yx_doc_fs_mkdir_all/a/b"
    ok = fs.mkdir_all(d)
    assert(ok)
    assert(fs.is_dir(d))
    fs.rmdir("__yx_doc_fs_mkdir_all/a/b")
    fs.rmdir("__yx_doc_fs_mkdir_all/a")
    fs.rmdir("__yx_doc_fs_mkdir_all")
}
```

### rmdir

<!-- stdlib:sig:fs.rmdir start -->

```yaoxiang
rmdir: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.rmdir end -->

Удаляет **пустой** каталог. Удаление непустого каталога завершается неудачей с выбросом `E6007`.

### remove

<!-- stdlib:sig:fs.remove start -->

```yaoxiang
remove: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.remove end -->

Удаляет файл. Для удаления каталога используйте [`rmdir`](#rmdir) (только пустые каталоги).

Возвращает: `true` при успехе. Ошибки: если файл не существует, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_remove.txt"
    fs.write_file(p, "x")
    assert(fs.remove(p), "remove ok")
    assert(!fs.exists(p), "gone after remove")
}
```

### copy

<!-- stdlib:sig:fs.copy start -->

```yaoxiang
copy: (src: &String, dst: &String) -> Bool
```

<!-- stdlib:sig:fs.copy end -->

Копирует содержимое файла и биты прав доступа в целевой путь; если цель существует — перезаписывает
её (как `fs::copy` в Rust, временны́е метки метаданных не копируются).

Возвращает: `true` при успехе. Ошибки: если источник не существует, выбрасывается `E6007`.

### rename

<!-- stdlib:sig:fs.rename start -->

```yaoxiang
rename: (src: &String, dst: &String) -> Bool
```

<!-- stdlib:sig:fs.rename end -->

Переименовывает/перемещает файл или каталог. При перемещении между устройствами выбрасывается
`E6007` (в качестве обходного пути — copy, затем remove).

Возвращает: `true` при успехе.

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    a = "__yx_doc_fs_rename_a.txt"
    b = "__yx_doc_fs_rename_b.txt"
    fs.write_file(a, "data")
    assert(fs.rename(a, b), "rename ok")
    assert(fs.exists(b), "target exists")
    assert(!fs.exists(a), "source gone")
    fs.remove(b)
}
```

### read_dir

<!-- stdlib:sig:fs.read_dir start -->

```yaoxiang
read_dir: (path: &String) -> Vec(String)
```

<!-- stdlib:sig:fs.read_dir end -->

Перечисляет **имена** элементов каталога (без префикса пути), возвращая `List(String)`,
отсортированный по имени.

> В отличие от исходного `std.os.read_dir` (возвращавшего строку, соединённую через `"\n"`), данная
> функция возвращает корректно типизированный List — это семантическое улучшение при переносе
> файловых операций из `std.os` в `std.fs`.

```yaoxiang
use std.assert
use std.fs
use std.list

main: () -> Void = {
    d = "__yx_doc_fs_readdir"
    fs.mkdir_all(d)
    fs.write_file(d + "/b.txt", "b")
    fs.write_file(d + "/a.txt", "a")

    names = fs.read_dir(d)
    assert(list.len(names) == 2, "two entries")
    assert(list.get(names, 0) == "a.txt", "sorted first")
    assert(list.get(names, 1) == "b.txt", "sorted second")

    fs.remove(d + "/a.txt")
    fs.remove(d + "/b.txt")
    fs.rmdir(d)
}
```

### walk

<!-- stdlib:sig:fs.walk start -->

```yaoxiang
walk: (path: &String) -> Vec(String)
```

<!-- stdlib:sig:fs.walk end -->

Рекурсивно обходит дерево каталогов, возвращая `List(String)` **полных путей всех элементов**; на
каждом уровне сортирует по имени, причём каталог появляется раньше своего содержимого (обход в
глубину). Символические ссылки перечисляются как элементы, но не обходятся рекурсивно.

Ошибки: если `path` не является каталогом, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.fs
use std.list

main: () -> Void = {
    d = "__yx_doc_fs_walk"
    fs.mkdir_all(d + "/sub")
    fs.write_file(d + "/a.txt", "a")
    fs.write_file(d + "/sub/b.txt", "b")

    paths = fs.walk(d)
    // a.txt, каталог sub, sub/b.txt — три элемента
    assert(list.len(paths) == 3, "2 files + 1 dir")

    fs.remove(d + "/a.txt")
    fs.remove(d + "/sub/b.txt")
    fs.rmdir(d + "/sub")
    fs.rmdir(d)
}
```

### stat

<!-- stdlib:sig:fs.stat start -->

```yaoxiang
stat: (path: &String) -> Dict(String, Any)
```

<!-- stdlib:sig:fs.stat end -->

Читает метаданные файла/каталога, возвращая словарь:

| Ключ       | Тип    | Значение                       |
| ---------- | ------ | ------------------------------ |
| `size`     | `Int`  | Размер в байтах                |
| `is_dir`   | `Bool` | Является ли каталогом          |
| `is_file`  | `Bool` | Является ли обычным файлом     |
| `readonly` | `Bool` | Доступен ли только для чтения  |
| `mtime`    | `Int`  | Время изменения (секунды Unix) |

Ошибки: если путь не существует, выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = "__yx_doc_fs_stat.txt"
    fs.write_file(p, "12345")

    st = fs.stat(p)
    assert(st["size"] == 5, "byte size")
    assert(st["is_file"], "is a file")

    fs.remove(p)
}
```

### temp_dir

<!-- stdlib:sig:fs.temp_dir start -->

```yaoxiang
temp_dir: () -> String
```

<!-- stdlib:sig:fs.temp_dir end -->

Путь к системному каталогу временных файлов (на Windows — `%TEMP%`, на Unix — `$TMPDIR` или `/tmp`).

### mkdtemp

<!-- stdlib:sig:fs.mkdtemp start -->

```yaoxiang
mkdtemp: (prefix: &String) -> String
```

<!-- stdlib:sig:fs.mkdtemp end -->

Создаёт **уникальный** временный каталог (имя начинается с `prefix`) в системном каталоге временных
файлов и возвращает его полный путь.

> Каталог **не удаляется** автоматически: после использования скрипт должен сам вызвать
> [`rmdir`](#rmdir). Эта семантика осознанная — явный жизненный цикл предсказуемее неявных
> drop-хуков.

Возвращает: путь к новому каталогу. Ошибки: при сбое создания выбрасывается `E6007`.

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    d = fs.mkdtemp("yx_doc_")
    assert(fs.is_dir(d), "temp dir created")
    fs.rmdir(d)
}
```

### tmpfile

<!-- stdlib:sig:fs.tmpfile start -->

```yaoxiang
tmpfile: (prefix: &String) -> String
```

<!-- stdlib:sig:fs.tmpfile end -->

Создаёт **уникальный** пустой временный файл (имя начинается с `prefix`) в системном каталоге
временных файлов и возвращает его полный путь. Также **не удаляется** автоматически — после
использования вызовите [`remove`](#remove).

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    p = fs.tmpfile("yx_doc_")
    assert(fs.is_file(p), "temp file created")
    fs.write_file(p, "scratch")
    assert(fs.read_file(p) == "scratch")
    fs.remove(p)
}
```

### path_join

<!-- stdlib:sig:fs.path_join start -->

```yaoxiang
path_join: (base: &String, rel: &String) -> String
```

<!-- stdlib:sig:fs.path_join end -->

Соединяет компоненты пути. Если `rel` — абсолютный путь, он **полностью заменяет** `base` (как в
Rust/Python). Разделитель зависит от платформы (на Windows — `\`).

```yaoxiang
use std.assert
use std.fs
use std.string

main: () -> Void = {
    joined = fs.path_join("dir", "file.txt")
    assert(string.ends_with(joined, "file.txt"), "suffix kept")
    assert(string.starts_with(joined, "dir"), "base kept")
}
```

### path_basename

<!-- stdlib:sig:fs.path_basename start -->

```yaoxiang
path_basename: (path: &String) -> String
```

<!-- stdlib:sig:fs.path_basename end -->

Последний компонент пути; если последний компонент отсутствует (например, `/`, `..`), возвращается
пустая строка.

### path_dirname

<!-- stdlib:sig:fs.path_dirname start -->

```yaoxiang
path_dirname: (path: &String) -> String
```

<!-- stdlib:sig:fs.path_dirname end -->

Каталог — часть пути; если родительский каталог отсутствует (например, `a.txt`), возвращается пустая
строка.

```yaoxiang
use std.assert
use std.fs

main: () -> Void = {
    assert(fs.path_basename("dir/file.txt") == "file.txt")
    assert(fs.path_dirname("dir/file.txt") == "dir")
    assert(fs.path_extension("a.tar.gz") == "gz")
    assert(fs.path_extension("noext") == "")
}
```

### path_extension

<!-- stdlib:sig:fs.path_extension start -->

```yaoxiang
path_extension: (path: &String) -> String
```

<!-- stdlib:sig:fs.path_extension end -->

Расширение (без точки, берётся последнее); если расширения нет, возвращается пустая строка.

## Связанные разделы

- [`std.os`](./os) — инкрементальное чтение/запись по дескриптору файла и переменные окружения
- [`std.io`](./io) — консольный ввод/вывод
- [Справочник по кодам ошибок](../error-code/) — `E6007` общая ошибка времени выполнения
