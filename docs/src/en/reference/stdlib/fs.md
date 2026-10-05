---
title: 'std.fs'
description: 'File, directory, and path operations'
---

# std.fs

Path-level filesystem operations: whole-file read/write, directory management and traversal, file
metadata, temporary files, and path arithmetic.

For incremental read/write by handle (open/read/seek), use [`std.os`](os); for console input/output,
use [`std.io`](io).

```yaoxiang
use std.fs
```

## Platform availability

This module is **not exported** on the `wasm32` target (no filesystem semantics). `mkdtemp` /
`tmpfile` / `temp_dir` depend on safe temporary file creation and are only available on native
targets.

## Function overview

<!-- stdlib:table:fs start -->

| Function         | Signature                                   |
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

<!-- stdlib:table:fs end -->## Functions

### read_file

<!-- stdlib:sig:fs.read_file start -->

```yaoxiang
read_file: (path: &String) -> String
```

<!-- stdlib:sig:fs.read_file end -->

Reads the entire file content as a string in one shot.

- `path` —— file path (read-only borrow)

Returns: the entire file content. Errors: throws `E6007` when the file does not exist or permission
is denied. **Does not return an empty string**.

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

Writes `content` to `path`, **overwriting** the existing content; creates the file if it does not
exist.

- `path` —— file path (read-only borrow)
- `content` —— content to write (read-only borrow)

Returns: `true` on successful write. Errors: throws `E6007` when the directory does not exist or
permission is denied (does not return `false`).

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

**Appends** `content` to the end of `path`; creates the file if it does not exist.

Returns: `true` on successful write. Errors: throws `E6007` when permission is denied.

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

Whether the path exists (either file or directory).

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

Whether the path points to a regular file. Returns `false` (no error) when the path does not exist.

### is_dir

<!-- stdlib:sig:fs.is_dir start -->

```yaoxiang
is_dir: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.is_dir end -->

Whether the path points to a directory. Returns `false` (no error) when the path does not exist.

### mkdir

<!-- stdlib:sig:fs.mkdir start -->

```yaoxiang
mkdir: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.mkdir end -->

Creates a **single-level** directory; the parent directory must already exist. For recursive
creation, use [`mkdir_all`](#mkdir_all).

Returns: `true` on success. Errors: throws `E6007` when the parent directory is missing or the
directory already exists.

### mkdir_all

<!-- stdlib:sig:fs.mkdir_all start -->

```yaoxiang
mkdir_all: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.mkdir_all end -->

Recursively creates directories; missing parent directories are created as well; an already-existing
directory is treated as success.

Returns: `true` on success. Errors: throws `E6007` when permission is denied.

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

Removes an **empty** directory. Deleting a non-empty directory fails and throws `E6007`.

### remove

<!-- stdlib:sig:fs.remove start -->

```yaoxiang
remove: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.remove end -->

Removes a file. To remove a directory, use [`rmdir`](#rmdir) (empty directories only).

Returns: `true` on success. Errors: throws `E6007` when the file does not exist.

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

Copies file contents and permission bits to the destination path; overwrites the destination if it
already exists (consistent with Rust's `fs::copy`; does not copy metadata timestamps).

Returns: `true` on success. Errors: throws `E6007` when the source does not exist.

### rename

<!-- stdlib:sig:fs.rename start -->

```yaoxiang
rename: (src: &String, dst: &String) -> Bool
```

<!-- stdlib:sig:fs.rename end -->

Renames/moves a file or directory. Throws `E6007` when moving across devices (use copy followed by
remove instead).

Returns: `true` on success.

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

Lists the **names** of entries in a directory (without path prefix), returning `List(String)`,
sorted by name.

> Unlike the original `std.os.read_dir` (which returned a string joined by `"\n"`), this function
> returns a properly typed List—this is a semantic upgrade made when the `std.os` file operations
> were migrated into `std.fs`.

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

Recursively traverses a directory tree, returning the **full paths of all entries** as
`List(String)`; each level is sorted by name, with directories appearing before their contents
(depth-first). Symbolic links themselves are listed as entries but are not descended into.

Errors: throws `E6007` when `path` is not a directory.

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
    // three entries: a.txt, sub directory, sub/b.txt
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

Reads file/directory metadata, returning a dictionary:

| Key        | Type   | Meaning                             |
| ---------- | ------ | ----------------------------------- |
| `size`     | `Int`  | Number of bytes                     |
| `is_dir`   | `Bool` | Whether it is a directory           |
| `is_file`  | `Bool` | Whether it is a regular file        |
| `readonly` | `Bool` | Whether it has read-only permission |
| `mtime`    | `Int`  | Modification time (Unix seconds)    |

Errors: throws `E6007` when the path does not exist.

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

The system temporary directory path (`%TEMP%` on Windows, `$TMPDIR` or `/tmp` on Unix).

### mkdtemp

<!-- stdlib:sig:fs.mkdtemp start -->

```yaoxiang
mkdtemp: (prefix: &String) -> String
```

<!-- stdlib:sig:fs.mkdtemp end -->

Creates a **unique** temporary directory (with a name starting with `prefix`) inside the system
temporary directory, returning its full path.

> The directory is **not** automatically cleaned up: the script should call [`rmdir`](#rmdir) when
> done. This semantic is intentional—an explicit lifecycle is more predictable than an implicit drop
> hook.

Returns: the new directory path. Errors: throws `E6007` on creation failure.

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

Creates a **unique** empty temporary file (with a name starting with `prefix`) inside the system
temporary directory, returning its full path. Also **not** automatically cleaned up; call
[`remove`](#remove) when done.

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

Joins path components. When `rel` is an absolute path, it **directly replaces** `base` (consistent
with Rust/Python). The separator follows the platform (`\` on Windows).

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

The final component of the path; returns an empty string when there is no final component (e.g. `/`,
`..`).

### path_dirname

<!-- stdlib:sig:fs.path_dirname start -->

```yaoxiang
path_dirname: (path: &String) -> String
```

<!-- stdlib:sig:fs.path_dirname end -->

The directory portion of the path; returns an empty string when there is no parent (e.g. `a.txt`).

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

The extension (without the dot, taking the last one); returns an empty string when there is no
extension.

## Related

- [`std.os`](os) —— file-handle-level incremental read/write and environment variables
- [`std.io`](io) —— console input/output
- [Error code reference](../error-code/) —— `E6007` general runtime error
