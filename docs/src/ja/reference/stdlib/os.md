---
title: 'std.os'
description: 'ファイルハンドル、環境変数、作業ディレクトリ'
---

# std.os

オペレーティングシステムインターフェースモジュール：ファイルハンドルの読み書き、環境変数、作業ディレクトリ。パスレベルのファイル操作（ファイル全体の読み書き、ディレクトリ、メタデータ、パス演算）については
[`std.fs`](./fs) を参照してください。

```yaoxiang
use std.os
```

> 本モジュールのすべての関数はオペレーティングシステムの機能に依存しているため、 `wasm32`
> ターゲットでは**エクスポートされません**。

## ファイルハンドルモデル

> **ハンドルは参照渡し（#337 修正済み）**：`read` / `write` / `seek` / `tell` / `flush` / `close`
> のシグネチャはすべて `(file: &File, ...)` となっているため、ハンドルは繰り返し使用できます：
>
> ```yaoxiang
> f = os.open(p, "w")
> os.write(f, "hello world")
> os.close(f)
> ```
>
> 位置決め後の読み書き（`seek` の存在理由）も利用可能です：
>
> ```yaoxiang
> r = os.open(p, "r")
> os.seek(r, 6)
> tail = os.read(r, 5)     // "world"
> os.close(r)
> ```
>
> 修正前のシグネチャには `&` がなく、ハンドルは値渡し → 線形所有 → 一度使うと無効化され、
> `open → write → close` は `E2014` エラーになります。
>
> 手動でのハンドル管理を避けたい場合は、ハンドルを開かない便利な関数
> [`std.io.read_file`](./io#read_file) / [`write_file`](./io#write_file) /
> [`append_file`](./io#append_file)、または本モジュールの [`append_file`](#append_file)
> を使用できます。

`open` が返すのは **`Int`
型のファイルディスクリプタ**（エンジンが内部でハンドルテーブルを管理）です。そのためシグネチャ中の
`File` は実質的に `Int` です。

書き込み後は内容が直ちにディスクに反映されるため、明示的な `close` は不要です：

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

`open` がサポートするモード：

| モード | 意味                                       |
| ------ | ------------------------------------------ |
| `r`    | 読み取り専用、ファイルが存在する必要がある |
| `w`    | 書き込み専用、作成または空にする           |
| `a`    | 追記、作成または末尾に追記                 |
| `r+`   | 読み書き、ファイルが存在する必要がある     |
| `w+`   | 読み書き、作成または空にする               |
| `a+`   | 読み書き、作成または追記                   |

## 関数一覧

<!-- stdlib:table:os start -->

| 関数      | シグネチャ                                |
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

<!-- stdlib:table:os end -->## ファイル操作

### open

<!-- stdlib:sig:os.open start -->

```yaoxiang
open: (path: &String, mode: &String) -> File
```

<!-- stdlib:sig:os.open end -->

ファイルを開き、ファイルディスクリプタを返します。

- `path` —— ファイルパス（読み取り専用借用）
- `mode` —— オープンモード、上記の表を参照

戻り値：内部ハンドルテーブルによって割り当てられた `Int`
ディスクリプタ。**このハンドルは一度しか使用できません**
— 後続の呼び出しはいずれもそれを移動します（[ファイルハンドルモデル](#ファイルハンドルモデル)
を参照）。したがって通常、`open` を単一の呼び出しにインライン化します。

エラー：モードが無効、ファイルが存在しない、アクセス権がない場合は `E6007` をスローします。

> ハンドルは一度しか使用できない（#337）ため、戻り値は通常、後続の呼び出しに直接インライン化されます。

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

ファイルハンドルを閉じ、テーブルエントリを解放します。

ハンドルは一度しか使用できないため、`close`
は「開いた後、他の用途には使用しない」シナリオでのみ意味を持ちます。書き込まれた内容は
[`write`](#write) が返った時点で既にディスクに反映されているため、通常は明示的なクローズは不要です。

エラー：ディスクリプタが無効（開かれていないか既に閉じられている）の場合、 `E6007` をスローします。

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

現在の読み書き位置から最大 `n` バイトを読み取ります。

- `file` —— ファイルディスクリプタ
- `n` —— 読み取りたいバイト数

戻り値：実際に読み取られた内容（`n`
より短い場合があり、ファイル末尾に到達した場合は空文字列）。不正な UTF-8 バイトは置換文字として返され、エラーにはなりません。エラー：ディスクリプタが無効または読み取りに失敗した場合は
`E6007` をスローします。

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

現在の読み書き位置に `content` 全体を書き込みます。

- `content` —— 値渡し

戻り値：書き込まれた**バイト数**。エラー：ディスクリプタが無効または書き込みに失敗した場合は `E6007`
をスローします。

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

読み書き位置を**絶対**オフセット `offset`（ファイル先頭からの相対位置）に移動します。

- `offset` —— 目標バイトオフセット、非負である必要がある

戻り値：成功した場合は `true`。エラー：ディスクリプタが無効またはオフセットが無効な場合は `E6007`
をスローします。

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

現在の読み書き位置のバイトオフセットを返します。

エラー：ディスクリプタが無効な場合は `E6007` をスローします。

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

バッファされた内容をディスクにフラッシュします。

エラー：ディスクリプタが無効またはフラッシュに失敗した場合は `E6007` をスローします。

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

## 環境変数

### get_env

<!-- stdlib:sig:os.get_env start -->

```yaoxiang
get_env: (name: &String) -> String
```

<!-- stdlib:sig:os.get_env end -->

環境変数を読み取ります。

戻り値：変数の値。**変数が存在しない場合は空文字列を返します**（エラーにはなりません）。したがって「未設定」と「空文字列に設定」の区別はできません。

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    // PATH は主要プラットフォームに必ず存在する
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

main: () -> Void = {
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

戻り値：すべての argv を **`\n` で連結した**単一の文字列（`List`
ではありません）。最初の要素はプログラム自身のパスです。

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

現在の作業ディレクトリを変更します。

戻り値：成功した場合は `true`。エラー：ディレクトリが存在しない場合は `E6007` をスローします。

```yaoxiang
use std.assert
use std.os

main: () -> Void = {
    before = os.getcwd()
    assert(os.chdir(".."))
    assert(os.chdir(before))     // 元に戻す
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

エラー：取得できない場合は `E6007` をスローします。

```yaoxiang
use std.assert
use std.os
use std.string

main: () -> Void = {
    cwd = os.getcwd()
    assert(string.len(cwd) > 0)
}
```

## 関連

- [`std.fs`](./fs) —— パスレベルファイルとディレクトリ操作
- [エラーコードリファレンス](../error-code/) —— `E6007` 汎用ランタイムエラー
