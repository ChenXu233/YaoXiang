---
title: 'std.io'
description: '標準出力、標準入力とファイル全体の読み書き'
---

# std.io

入出力モジュール。標準出力、標準入力の読み取り、および「ファイル全体を一度に読み書きする」便利な関数を提供する。ハンドルに基づいてファイルを段階的に読み書きする場合は
[`std.os`](./os)を使う。

```yaoxiang
use std.io
```

## プラットフォームの可用性

`read_line` 以降の関数は OS の I/O に依存しており、`wasm32` ターゲットでは**エクスポートされない**：
`read_line`、`read_file`、`write_file`、`append_file`。 `print` / `println` / `format_fallback`
はすべてのターゲットで利用できる。

## 関数一覧

<!-- stdlib:table:io start -->

| 関数              | シグネチャ                                  |
| ----------------- | ------------------------------------------- |
| `print`           | `(...args) -> Void`                         |
| `println`         | `(...args) -> ()`                           |
| `read_line`       | `() -> String`                              |
| `read_file`       | `(path: &String) -> String`                 |
| `write_file`      | `(path: &String, content: &String) -> Bool` |
| `append_file`     | `(path: &String, content: &String) -> Bool` |
| `format_fallback` | `(value, type_name: &String) -> String`     |

<!-- stdlib:table:io end -->## 関数

### print

<!-- stdlib:sig:io.print start -->

```yaoxiang
print: (...args) -> Void
```

<!-- stdlib:sig:io.print end -->

すべての引数を順番に出力し、**改行は追加しない**。複数の引数の間は単一のスペースで区切られる。

引数はフォーマットされる：`String` は内容を直接出力；`List` / `Dict` / `Tuple`
は再帰的に展開；その他の値はリテラルとして出力。

```yaoxiang
use std.io

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

[`print`](#print) と同じだが、出力の末尾に改行を追加する。

`println()` を引数なしで呼び出すと空行を出力する：

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

標準入力から 1 行を読み取る。

戻り値：読み取った行全体の内容。**末尾の改行文字（`\n` または
`\r\n`）は除去済み**。エラー：読み取り失敗時は `E6007` をスローする。

> インタラクティブな例はドキュメント内では自動実行できないため、以下の書き方は参考にとどめてほしい。

```yaoxiang
use std.io

main: () -> Void = {
    println("请输入你的名字：")
    name = io.read_line()
    println("你好，" + name)
}
```

### read_file

<!-- stdlib:sig:io.read_file start -->

```yaoxiang
read_file: (path: &String) -> String
```

<!-- stdlib:sig:io.read_file end -->

ファイル内容全体を一度に文字列として読み取る。

- `path` —— ファイルパス（読み取り専用借用）

戻り値：ファイルの内容全体。エラー：ファイルが存在しないか権限がない場合は `E6007`
をスローする。**空文字列は返さない**。

```yaoxiang
use std.assert
use std.io
use std.os
use std.string

main: () -> Void = {
    p = "__yx_doc_read_file.txt"
    io.write_file(p, "hello")

    content = io.read_file(p)
    assert(content == "hello")

    os.remove(p)
}
```

### write_file

<!-- stdlib:sig:io.write_file start -->

```yaoxiang
write_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:io.write_file end -->

`content` を `path` に書き込み、既存の内容を**上書き**する；ファイルが存在しない場合は作成する。

- `path` —— ファイルパス（読み取り専用借用）
- `content` —— 書き込む内容（読み取り専用借用）

戻り値：書き込み成功時は `true` を返す。エラー：ディレクトリが存在しないか権限がない場合は `E6007`
をスローする（`false` は返さない）。

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_write_file.txt"
    ok = io.write_file(p, "hello")
    assert(ok)
    assert(os.exists(p))
    os.remove(p)
}
```

### append_file

<!-- stdlib:sig:io.append_file start -->

```yaoxiang
append_file: (path: &String, content: &String) -> Bool
```

<!-- stdlib:sig:io.append_file end -->

`content` を `path` の末尾に**追加**する；ファイルが存在しない場合は作成する。

戻り値：書き込み成功時は `true` を返す。エラー：権限がない場合は `E6007` をスローする。

```yaoxiang
use std.assert
use std.io
use std.os

main: () -> Void = {
    p = "__yx_doc_append_file.txt"
    io.write_file(p, "hello")
    io.append_file(p, " world")
    assert(io.read_file(p) == "hello world")
    os.remove(p)
}
```

### format_fallback

<!-- stdlib:sig:io.format_fallback start -->

```yaoxiang
format_fallback: (value, type_name: &String) -> String
```

<!-- stdlib:sig:io.format_fallback end -->

型名に従って値をフォーマットし、`int(42)` / `list@3` のような型プレフィックス付きの表現を出力する。

これは内部ヘルパー関数であり、ランタイムの汎用フォーマット経路からのコールバック用である。通常のコードでは
[`std.convert.to_string`](./convert#to_string) を直接使うこと。

- `value` —— 任意の値
- `type_name` —— 型名の文字列

戻り値：型プレフィックス付きの文字列表現。

```yaoxiang
use std.assert
use std.io
use std.string

main: () -> Void = {
    s = io.format_fallback(42, "int")
    assert(string.contains(s, "42"))
}
```

## 関連

- [`std.os`](./os) —— ファイルハンドル、ディレクトリ、環境変数
- [`std.convert`](./convert) —— 値の文字列変換
