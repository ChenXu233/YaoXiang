---
title: 'std.convert'
description: '任意値から String への変換'
---

# std.convert

型変換モジュール。現在、値から `String` への変換を提供します。

```yaoxiang
use std.convert
```

## 変換ルール

`to_string` は値のランタイム形式に従ってフォーマットします：

| 型       | 出力形式                   | 例                             |
| -------- | -------------------------- | ------------------------------ |
| `Void`   | `void`                     | `void`                         |
| `Bool`   | `true` / `false`           | `true`                         |
| `Int`    | 十進                       | `42`                           |
| `Float`  | 下記参照                   | `3.14` / `2.0`                 |
| `Char`   | 文字そのもの               | `a`                            |
| `String` | 元の内容（**引用符なし**） | `hello`                        |
| `List`   | `[要素, ...]`              | `[1, 2, 3]`                    |
| `Dict`   | `{k: v, ...}`              | `{a: 1}`                       |
| `Tuple`  | `(要素, ...)`              | `(1, hello)`                   |
| `Array`  | `[要素, ...]`              | `[1, 2]`                       |
| `Range`  | `開始..終了`               | `1..5`（step が 1 の場合省略） |
| `Bytes`  | `bytes[長さ]`              | `bytes[3]`                     |

`Float` の整数値には小数点が 1 桁付加されます（`2` ではなく `2.0`）。これは `Int` と `Float`
を区別するためです。

## 関数一覧

<!-- stdlib:table:convert start -->

| 関数               | シグネチャ          |
| ------------------ | ------------------- |
| `to_string`        | `(value) -> String` |
| `int.to_string`    | `(self) -> String`  |
| `float.to_string`  | `(self) -> String`  |
| `bool.to_string`   | `(self) -> String`  |
| `char.to_string`   | `(self) -> String`  |
| `string.to_string` | `(self) -> String`  |
| `list.to_string`   | `(self) -> String`  |
| `dict.to_string`   | `(self) -> String`  |
| `tuple.to_string`  | `(self) -> String`  |
| `set.to_string`    | `(self) -> String`  |
| `range.to_string`  | `(self) -> String`  |

<!-- stdlib:table:convert end -->

## 関数

### to_string

<!-- stdlib:sig:convert.to_string start -->

```yaoxiang
to_string: (value) -> String
```

<!-- stdlib:sig:convert.to_string end -->

任意の値をその文字列表現に変換します。

- `value` —— 任意の型の値

戻り値：フォーマット後の文字列。引数が欠落している場合、`"()"` を返します。

```yaoxiang
use std.assert
use std.convert

main: () -> Void = {
    assert(convert.to_string(42) == "42")
    assert(convert.to_string(true) == "true")
    assert(convert.to_string(false) == "false")
}
```

文字列自体には引用符が付きません：

```yaoxiang
use std.assert
use std.convert

main: () -> Void = {
    assert(convert.to_string("hi") == "hi")
}
```

複合型は再帰的に展開されます（フォーマットは脆弱性回避のため表明しません）：

```yaoxiang
use std.assert
use std.convert
use std.string

main: () -> Void = {
    s_list = convert.to_string([1, 2, 3])
    assert(string.len(s_list) > 0)

    s_dict = convert.to_string({})
    assert(string.len(s_dict) > 0)
}
```

### 型メソッド形式

汎用の `convert.to_string`
に加え、モジュールは型ごとに束縛された以下の同名関数をエクスポートしており、動作はそれと完全に一致します：

| エクスポート名     | シグネチャ         |
| ------------------ | ------------------ |
| `int.to_string`    | `(self) -> String` |
| `float.to_string`  | `(self) -> String` |
| `bool.to_string`   | `(self) -> String` |
| `char.to_string`   | `(self) -> String` |
| `string.to_string` | `(self) -> String` |
| `list.to_string`   | `(self) -> String` |
| `dict.to_string`   | `(self) -> String` |
| `tuple.to_string`  | `(self) -> String` |
| `set.to_string`    | `(self) -> String` |
| `range.to_string`  | `(self) -> String` |

これらの束縛はランタイムの `Stringable` ディスパッチで使用されます。通常のコードでは
[`convert.to_string`](#to_string) を直接使用すれば十分です。

> `Set` 型は言語層から既に削除されており、`set.to_string`
> は互換用プレースホルダとして残されています。

## 関連

- [`std.io`](./io) —— `print` / `println` の内部フォーマットは同じルールに従います
- [`std.string`](./string) —— 文字列操作
