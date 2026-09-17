---
title: 'std.os'
description: 'ファイルハンドル、ディレクトリ、環境変数、作業ディレクトリ'
---

# std.os

オペレーティングシステムインターフェースモジュール：ファイルハンドルの読み書き、ディレクトリ操作、環境変数、作業ディレクトリ。

```yaoxiang
use std.os
```

> 本モジュールのすべての関数はオペレーティングシステムの機能に依存しており、`wasm32`
> ターゲットでは**エクスポートされません**。

## ファイルハンドルモデル

> **重要な制限（#337）**：`open` が返すハンドルは**一回限り**のものです。`&` を持たないため、最初に
> `read` / `write` / `seek` / `tell` / `flush` / `close`
> に渡した時点で**ムーブ**され、再利用できません。そのため `open` → `write` → `close`
> のような一般的な書き方は現在の実装では**コンパイルできません**（`E2014` が報告されます）。
>
> 実行可能な2つの書き方：
>
> 1. **`open` を単一の呼び出しにインライン化する**——ハンドルは生成後すぐに消費されます：
>
>    ```yaoxiang
>    n = os.write(os.open(p, "w"), "hello")
>    ```
>
> 2. **ハンドルを開かない便利関数を使う**——[`std.io.read_file`](./io#read_file) /
>    [`write_file`](./io#write_file) / [`append_file`](./io#append_file)、または本モジュールの
>    [`append_file`](#append_file)。
>
> ハンドルはハンドルテーブルエントリとして存在し、プロセス終了時にプロセスと共に回収されます。一度使用すると再参照できないため、多くのシナリオでは明示的な
> `close` を記述できません（ただし、下記の単一呼び出し形式を参照してください）。

`open` が返すのは **`Int`
型のファイルディスクリプタ**です（エンジンが内部でハンドルテーブルを保持します）。したがってシグネチャ中の
`File` は実際には `Int` です。

書き込み後は内容が直ちにディスクに反映され、明示的な `close` は不要です：

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

`open` がサポートするモード：

| モード | 意味                           |
| ------ | ------------------------------ |
| `r`    | 読み取り専用、ファイル必須     |
| `w`    | 書き込み専用、作成または初期化 |
| `a`    | 追記、作成または末尾に追記     |
| `r+`   | 読み書き、ファイル必須         |
| `w+`   | 読み書き、作成または初期化     |
| `a+`   | 読み書き、作成または追記       |

## 関数一覧

<!-- stdlib:table:os start -->

| 関数          | シグネチャ                                  |
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

<!-- stdlib:table:os end -->## ファイル操作

### open

<!-- stdlib:sig:os.open start -->

```yaoxiang
open: (path: &String, mode: &String) -> File
```

<!-- stdlib:sig:os.open end -->

ファイルを開き、ファイルディスクリプタを返します。

- `path` —— ファイルパス（読み取り専用借用）
- `mode` —— オープンモード、上の表を参照

戻り値：内部ハンドルテーブルが割り当てる `Int`
ディスクリプタ。**このハンドルは一度しか使用できません**——任意の下流呼び出しがこれをムーブします（[ファイルハンドルモデル](#ファイルハンドルモデル)
を参照）。そのため通常は `open` を単一の呼び出しにインライン化します。

エラー：モードが無効、ファイルが存在しない、または権限がない場合、`E6007` をスローします。

> ハンドルは一度しか使用できないため（#337）、この戻り値は通常、直接下流の呼び出しにインライン化されます。

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

ファイルハンドルを閉じ、テーブルエントリを解放します。

ハンドルは一度しか使用できないため、`close`
は「開いた後、他に使用しない」シナリオでのみ意味があります。書き込まれた内容は [`write`](#write)
が返る時点で既にディスクに反映されているため、通常は明示的なクローズは不要です。

エラー：ディスクリプタが無効な場合（未オープンまたは既にクローズ済み）、`E6007` をスローします。

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

現在の読み書き位置から**最大** `n` バイトを読み取ります。

- `file` —— ファイルディスクリプタ
- `n` —— 読み取り希望のバイト数

戻り値：実際に読み取られた内容（`n`
より短い可能性があり、ファイル末尾に到達した場合は空文字列）。不正な UTF-8 バイトは置換文字として返され、エラーにはなりません。エラー：ディスクリプタが無効または読み取りに失敗した場合、`E6007`
をスローします。

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

現在の読み書き位置に `content` のすべてを書き込みます。

- `content` —— 値で渡される

戻り値：書き込まれた**バイト数**。エラー：ディスクリプタが無効または書き込みに失敗した場合、`E6007`
をスローします。

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

読み書き位置を**絶対**オフセット `offset`（ファイル先頭からの相対位置）に移動します。

- `offset` —— 目標バイトオフセット、非負であること

戻り値：成功時は `true`
を返します。エラー：ディスクリプタが無効またはオフセットが不正な場合、`E6007` をスローします。

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

現在の読み書き位置のバイトオフセットを返します。

エラー：ディスクリプタが無効な場合、`E6007` をスローします。

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

バッファ内容をディスクにフラッシュします。

エラー：ディスクリプタが無効またはフラッシュに失敗した場合、`E6007` をスローします。

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

## ディレクトリ操作

### mkdir

<!-- stdlib:sig:os.mkdir start -->

```yaoxiang
mkdir: (path: &String) -> Bool
```

<!-- stdlib:sig:os.mkdir end -->

**単一階層**のディレクトリを作成します（親ディレクトリは再帰的に作成しません）。

戻り値：成功時は `true`
を返します。エラー：親ディレクトリが存在しない、またはディレクトリが既に存在する場合、`E6007`
をスローします。

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

**空の**ディレクトリを削除します。

戻り値：成功時は `true` を返します。エラー：ディレクトリが存在しない、または空でない場合、`E6007`
をスローします。

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

ディレクトリ内のエントリ名を列挙します。

戻り値：エントリ名が **`\n` で連結された**単一の文字列（`List`
ではありません）。エラー：ディレクトリが存在しない、または権限がない場合、`E6007` をスローします。

```yaoxiang
use std.assert
use std.os
use std.string

main = {
    d = "__yx_doc_read_dir"
    os.mkdir(d)
    names = os.read_dir(d)
    // 空ディレクトリは空文字列を返す
    assert(string.is_empty(names))
    os.rmdir(d)
}
```

## パスとファイルユーティリティ

### remove

<!-- stdlib:sig:os.remove start -->

```yaoxiang
remove: (path: &String) -> Bool
```

<!-- stdlib:sig:os.remove end -->

ファイルを削除します。セマンティクスは `remove_file`
と同等です（**ディレクトリは削除できません**。ディレクトリの削除は [`rmdir`](#rmdir)
を使用してください）。

戻り値：成功時は `true`
を返します。エラー：ファイルが存在しない、またはパスがディレクトリの場合、`E6007` をスローします。

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

パスが存在するかどうか（ファイルまたはディレクトリ）。**エラーは発生せず**、存在しない場合は `false`
を返します。

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

パスが**通常のファイル**であるかどうか。ディレクトリは `false` を返し、存在しない場合も `false`
を返します。

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

パスが**ディレクトリ**であるかどうか。ファイルは `false` を返し、存在しない場合も `false`
を返します。

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

ファイルをコピーします。ターゲットが既に存在する場合は**上書き**します。

戻り値：成功時は `true`
を返します。エラー：ソースファイルが存在しない、または権限がない場合、`E6007` をスローします。

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

ファイル名を変更またはファイルを移動します。

戻り値：成功時は `true`
を返します。エラー：ソースファイルが存在しない、またはターゲットが既に存在する場合、`E6007`
をスローします。

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

追記書き込み（ハンドルを開かない便利関数）。ファイルが存在しない場合は作成します。

戻り値：成功時は `true` を返します。エラー：権限がない場合、`E6007` をスローします。

> これは [`std.io.append_file`](./io#append_file)
> の同名同種インターフェースで、両方のモジュールで提供されており、動作は同一です。

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

## 環境変数

### get_env

<!-- stdlib:sig:os.get_env start -->

```yaoxiang
get_env: (name: &String) -> String
```

<!-- stdlib:sig:os.get_env end -->

環境変数を読み取ります。

戻り値：変数の値。**変数が存在しない場合は空文字列を返します**（エラーは発生しません）。したがって「未設定」と「空文字列に設定」の区別はできません。

```yaoxiang
use std.assert
use std.os
use std.string

main = {
    // PATH は主要プラットフォームで必ず存在する
    path = os.get_env("PATH")
    assert(string.len(path) > 0)

    // 存在しない変数は空文字列を返す
    assert(string.is_empty(os.get_env("__YX_DEFINITELY_MISSING__")))
}
```

### set_env

<!-- stdlib:sig:os.set_env start -->

```yaoxiang
set_env: (name: &String, value: &String) -> Void
```

<!-- stdlib:sig:os.set_env end -->

環境変数を設定します（現在のプロセスに影響します）。

```yaoxiang
use std.assert
use std.os

main = {
    os.set_env("__YX_DOC_ENV", "hello")
    assert(os.get_env("__YX_DOC_ENV") == "hello")
}
```

## プロセスと作業ディレクトリ

### args

<!-- stdlib:sig:os.args start -->

```yaoxiang
args: () -> String
```

<!-- stdlib:sig:os.args end -->

コマンドライン引数を返します。

戻り値：すべての argv が **`\n` で連結された**単一の文字列（`List`
ではありません）。最初の項目はプログラム自身のパスです。

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

現在の作業ディレクトリを切り替えます。

戻り値：成功時は `true` を返します。エラー：ディレクトリが存在しない場合、`E6007` をスローします。

```yaoxiang
use std.assert
use std.os

main = {
    before = os.getcwd()
    assert(os.chdir(".."))
    assert(os.chdir(before))     // 戻る
    assert(os.getcwd() == before)
}
```

### getcwd

<!-- stdlib:sig:os.getcwd start -->

```yaoxiang
getcwd: () -> String
```

<!-- stdlib:sig:os.getcwd end -->

現在の作業ディレクトリの絶対パスを返します。

エラー：取得できない場合、`E6007` をスローします。

```yaoxiang
use std.assert
use std.os
use std.string

main = {
    cwd = os.getcwd()
    assert(string.len(cwd) > 0)
}
```

## 関連項目

- [`std.io`](./io) —— ファイル全体の読み書き便利関数
- [エラーコードリファレンス](../error-code/) —— `E6007` 汎用ランタイムエラー
