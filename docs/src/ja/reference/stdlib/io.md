---
title: 'std.io'
description: '標準出力、標準入力とフォーマット'
---

# std.io

入出力モジュール。標準出力と標準入力の読み取りを提供します。ファイル全体の読み書き、ディレクトリとパスの操作は
[`std.fs`](./fs) を参照してください。ハンドルレベルの増分読み書きは [`std.os`](./os)
を参照してください。

```yaoxiang
use std.io
```

## プラットフォーム可用性

`read_line` はオペレーティングシステムの I/O に依存しており、`wasm32`
ターゲットでは**エクスポートされません**。`print` / `println` / `format_fallback`
はすべてのターゲットで利用可能です。

## 関数一覧

<!-- stdlib:table:io start -->

| 関数              | シグネチャ                              |
| ----------------- | --------------------------------------- |
| `print`           | `(...args) -> Void`                     |
| `println`         | `(...args) -> ()`                       |
| `read_line`       | `() -> String`                          |
| `format_fallback` | `(value, type_name: &String) -> String` |

<!-- stdlib:table:io end -->## 関数

### print

<!-- stdlib:sig:io.print start -->

```yaoxiang
print: (...args) -> Void
```

<!-- stdlib:sig:io.print end -->

すべての引数を順番に出力し、**改行は追加しません**。複数の引数の間は単一のスペースで区切られます。

引数はフォーマットされます：`String` は内容を直接出力します；`List` / `Dict` / `Tuple`
は再帰的に展開されます；その他の値はリテラルとして出力されます。

```yaoxiang

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

[`print`](#print) と同じですが、出力の末尾に改行を追加します。

`println()` は引数なしで呼び出すと空行を出力します：

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

標準入力から 1 行読み取ります。

戻り値：読み取られた行全体、**末尾の改行文字（`\n` または
`\r\n`）は削除されます**。エラー：読み取りが失敗した場合、`E6007` がスローされます。

> インタラクティブな例はドキュメント内で自動実行できないため、以下のコードは参考用です。

```yaoxiang
use std.io

main: () -> Void = {
    println("请输入你的名字：")
    name = io.read_line()
    println("你好，" + name)
}
```

### format_fallback

<!-- stdlib:sig:io.format_fallback start -->

```yaoxiang
format_fallback: (value, type_name: &String) -> String
```

<!-- stdlib:sig:io.format_fallback end -->

型名に従って値をフォーマットし、`int(42)` / `list@3` のような型接頭辞付きの表現を出力します。

これは内部ヘルパー関数であり、ランタイムの汎用フォーマットパスからのコールバック用です。日常的なコードでは
[`std.convert.to_string`](./convert#to_string) を直接使用してください。

- `value` —— 任意の値
- `type_name` —— 型名の文字列

戻り値：型接頭辞付きの文字列表現。

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

- [`std.fs`](./fs) —— ファイル、ディレクトリ、パス操作
- [`std.os`](./os) —— ファイルハンドルと環境変数
- [`std.convert`](./convert) —— 値を文字列に変換
