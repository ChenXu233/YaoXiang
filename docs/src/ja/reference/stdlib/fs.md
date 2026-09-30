---
title: 'std.fs'
description: 'ファイル、ディレクトリ、パス操作'
---

# std.fs

パスレベルのファイルシステム操作：ファイル全体の一括読み書き、ディレクトリの管理と走査、ファイルメタデータ、一時ファイル、パス操作。

ファイルハンドル単位で逐次読み書き（open/read/seek）を行う場合は [`std.os`](./os)
を使用してください。コンソール入出力には [`std.io`](./io) を使用します。

```yaoxiang
use std.fs
```

## プラットフォームの可用性

本モジュールは `wasm32`
ターゲットでは**エクスポートされません**（ファイルシステムのセマンティクスが存在しないため）。`mkdtemp`
/ `tmpfile` / `temp_dir`
は安全な一時ファイル作成に依存しているため、ネイティブターゲットでのみ利用可能です。

## 関数一覧

<!-- stdlib:table:fs start -->

| 関数             | シグネチャ                                  |
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

<!-- stdlib:table:fs end -->## 関数

### read_file

<!-- stdlib:sig:fs.read_file start -->

```yaoxiang
read_file: (path: &String) -> String
```

<!-- stdlib:sig:fs.read_file end -->

ファイル内容を一括で読み込み、文字列として返します。

- `path` —— ファイルパス（読み取り専用の借用）

戻り値：ファイル内容全体。エラー：ファイルが存在しないか権限がない場合は `E6007`
を送出します。**空文字列は返しません**。

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

`content` を `path`
に書き込み、既存の内容を**上書き**します。ファイルが存在しない場合は作成されます。

- `path` —— ファイルパス（読み取り専用の借用）
- `content` —— 書き込む内容（読み取り専用の借用）

戻り値：書き込みが成功した場合は `true`。エラー：ディレクトリが存在しないか権限がない場合は `E6007`
を送出します（`false` は返しません）。

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

`content` を `path` の末尾に**追加**します。ファイルが存在しない場合は作成されます。

戻り値：書き込みが成功した場合は `true`。エラー：権限がない場合は `E6007` を送出します。

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

パスが存在するかどうか（ファイルでもディレクトリでも可）。

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

パスが通常ファイルを指しているかどうか。パスが存在しない場合は `false`
を返します（エラーにはなりません）。

### is_dir

<!-- stdlib:sig:fs.is_dir start -->

```yaoxiang
is_dir: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.is_dir end -->

パスがディレクトリを指しているかどうか。パスが存在しない場合は `false`
を返します（エラーにはなりません）。

### mkdir

<!-- stdlib:sig:fs.mkdir start -->

```yaoxiang
mkdir: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.mkdir end -->

**単一階層**のディレクトリを作成します。親ディレクトリはすでに存在している必要があります。再帰的に作成する場合は
[`mkdir_all`](#mkdir_all) を使用してください。

戻り値：成功した場合は
`true`。エラー：親ディレクトリが存在しないか、ディレクトリがすでに存在する場合は `E6007`
を送出します。

### mkdir_all

<!-- stdlib:sig:fs.mkdir_all start -->

```yaoxiang
mkdir_all: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.mkdir_all end -->

ディレクトリを再帰的に作成し、不足している親ディレクトリもまとめて作成します。ディレクトリがすでに存在する場合は成功とみなされます。

戻り値：成功した場合は `true`。エラー：権限がない場合は `E6007` を送出します。

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

**空の**ディレクトリを削除します。空でないディレクトリの削除は失敗し、`E6007` を送出します。

### remove

<!-- stdlib:sig:fs.remove start -->

```yaoxiang
remove: (path: &String) -> Bool
```

<!-- stdlib:sig:fs.remove end -->

ファイルを削除します。ディレクトリの削除には [`rmdir`](#rmdir)
を使用してください（空のディレクトリのみ）。

戻り値：成功した場合は `true`。エラー：ファイルが存在しない場合は `E6007` を送出します。

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

ファイル内容と権限ビットをコピー先のパスへコピーします。コピー先がすでに存在する場合は上書きされます（Rust の
`fs::copy` と同じで、メタデータのタイムスタンプはコピーされません）。

戻り値：成功した場合は `true`。エラー：コピー元が存在しない場合は `E6007` を送出します。

### rename

<!-- stdlib:sig:fs.rename start -->

```yaoxiang
rename: (src: &String, dst: &String) -> Bool
```

<!-- stdlib:sig:fs.rename end -->

ファイルまたはディレクトリの名前変更/移動を行います。デバイスをまたぐ移動の場合は `E6007`
を送出します（事前にコピーしてから削除する方法で代用してください）。

戻り値：成功した場合は `true`。

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

ディレクトリ内のエントリの**名前**を列挙します（パスのプレフィックスは含まない）。`List(String)`
を返し、名前順にソートされます。

> 元の `std.os.read_dir`（`"\n"`
> で連結された文字列を返す）とは異なり、本関数は正しい型の List を返します。これは `std.os`
> のファイル操作が `std.fs` へ移行した際のセマンティクスのアップグレードです。

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

ディレクトリツリーを再帰的に走査し、**すべてのエントリの完全パス**を `List(String)`
として返します。各階層は名前順にソートされ、ディレクトリはその内容より先に出現します（深さ優先）。シンボリックリンク自体はエントリとして列挙されますが、それ以上は降りません。

エラー：`path` がディレクトリでない場合は `E6007` を送出します。

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
    // a.txt、sub 目录、sub/b.txt 三个条目
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

ファイル/ディレクトリのメタデータを読み取り、辞書として返します：

| キー       | 型     | 意味                     |
| ---------- | ------ | ------------------------ |
| `size`     | `Int`  | バイト数                 |
| `is_dir`   | `Bool` | ディレクトリかどうか     |
| `is_file`  | `Bool` | 通常ファイルかどうか     |
| `readonly` | `Bool` | 読み取り専用権限かどうか |
| `mtime`    | `Int`  | 更新時刻（Unix 秒）      |

エラー：パスが存在しない場合は `E6007` を送出します。

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

システムの一時ディレクトリパス（Windows では `%TEMP%`、Unix では `$TMPDIR` または `/tmp`）。

### mkdtemp

<!-- stdlib:sig:fs.mkdtemp start -->

```yaoxiang
mkdtemp: (prefix: &String) -> String
```

<!-- stdlib:sig:fs.mkdtemp end -->

システム一時ディレクトリ内に**一意の**一時ディレクトリを作成します（名前は `prefix`
で始まります）。その完全パスを返します。

> ディレクトリは**自動的には**クリーンアップされません。スクリプト側で使い終わったら手動で
> [`rmdir`](#rmdir)
> を呼び出してください。このセマンティクスは意図的なものです。暗黙の drop フックより明示的なライフサイクルのほうが予測しやすいためです。

戻り値：新しいディレクトリのパス。エラー：作成に失敗した場合は `E6007` を送出します。

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

システム一時ディレクトリ内に**一意の**空の一時ファイルを作成します（名前は `prefix`
で始まります）。その完全パスを返します。同様に**自動的には**クリーンアップされないので、使い終わったら手動で
[`remove`](#remove) を呼び出してください。

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

パスの構成要素を連結します。`rel` が絶対パスの場合は `base`
を**そのまま置き換え**ます（Rust/Python と同じ挙動）。区切り文字はプラットフォームに依存します（Windows は
`\`）。

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

パスの最終構成要素。最終構成要素が存在しない場合（例：`/`、`..`）は空文字列を返します。

### path_dirname

<!-- stdlib:sig:fs.path_dirname start -->

```yaoxiang
path_dirname: (path: &String) -> String
```

<!-- stdlib:sig:fs.path_dirname end -->

パスのディレクトリ部分。親ディレクトリが存在しない場合（例：`a.txt`）は空文字列を返します。

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

拡張子（ドットを含まず、最後のもの）。拡張子がない場合は空文字列を返します。

## 関連

- [`std.os`](./os) —— ファイルハンドルレベルの逐次読み書きと環境変数
- [`std.io`](./io) —— コンソール入出力
- [エラーコードリファレンス](../error-code/) —— `E6007` 汎用ランタイムエラー
