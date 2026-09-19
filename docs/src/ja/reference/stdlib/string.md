---
title: 'std.string'
description: '文字列の検索、分割、フォーマット、解析'
---

# std.string

文字列操作モジュール。`format`
を除き、すべての関数は引数を読み取り専用借用（`&String`）し、呼び出し後も元の文字列は引き続き使用可能。

すべての関数は引数の型が一致しない場合、**空文字列の意味論**
に退化する（エラーではない）：`split`/`trim`/`upper` などは `String` 以外の入力を `""`
として扱う。これは誤った引数の型でも中断されないことを意味するが、期待される結果も得られない——コンパイラによる型チェックでコンパイル時に捕捉されることを推奨する。

```yaoxiang
use std.string
```

## 関数一覧

<!-- stdlib:table:string start -->

| 関数          | シグネチャ                                           |
| ------------- | ---------------------------------------------------- |
| `split`       | `(s: &String, sep: &String) -> Vec(String)`          |
| `trim`        | `(s: &String) -> String`                             |
| `upper`       | `(s: &String) -> String`                             |
| `lower`       | `(s: &String) -> String`                             |
| `replace`     | `(s: &String, old: &String, new: &String) -> String` |
| `contains`    | `(s: &String, sub: &String) -> Bool`                 |
| `starts_with` | `(s: &String, prefix: &String) -> Bool`              |
| `ends_with`   | `(s: &String, suffix: &String) -> Bool`              |
| `index_of`    | `(s: &String, sub: &String) -> Int`                  |
| `substring`   | `(s: &String, start: Int, end: Int) -> String`       |
| `is_empty`    | `(s: &String) -> Bool`                               |
| `len`         | `(s: &String) -> Int`                                |
| `chars`       | `(s: &String) -> Vec(String)`                        |
| `concat`      | `(s1: &String, s2: &String) -> String`               |
| `repeat`      | `(s: &String, n: Int) -> String`                     |
| `reverse`     | `(s: &String) -> String`                             |
| `format`      | `(format: &String, ...args) -> String`               |
| `parse_int`   | `(s: &String) -> Result(Int, Error)`                 |
| `parse_float` | `(s: &String) -> Result(Float, Error)`               |

<!-- stdlib:table:string end -->## 関数

### split

<!-- stdlib:sig:string.split start -->

```yaoxiang
split: (s: &String, sep: &String) -> Vec(String)
```

<!-- stdlib:sig:string.split end -->

`sep` で `s` を分割し、部分文字列のリストを返す。

- `s` —— 分割対象の文字列
- `sep` —— 区切り文字。**空文字列の場合は文字単位で分割**

戻り値：`List(String)`。区切り文字が見つからない場合は単一要素のリストを返す。

```yaoxiang
use std.assert
use std.list
use std.string

main: () -> Void = {
    assert(list.len(string.split("a,b,c", ",")) == 3)
    assert(list.get(string.split("a,b,c", ","), 0) == "a")

    // 空区切り → 文字単位
    cs = string.split("abc", "")
    assert(list.len(cs) == 3)
}
```

### trim

<!-- stdlib:sig:string.trim start -->

```yaoxiang
trim: (s: &String) -> String
```

<!-- stdlib:sig:string.trim end -->

先頭と末尾の Unicode 空白文字を除去する。

戻り値：先頭と末尾の空白を除去した新しい文字列（`s` は変更しない）。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.trim("  hi  ") == "hi")
}
```

### upper

<!-- stdlib:sig:string.upper start -->

```yaoxiang
upper: (s: &String) -> String
```

<!-- stdlib:sig:string.upper end -->

大文字に変換する（Unicode 対応）。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.upper("abc") == "ABC")
}
```

### lower

<!-- stdlib:sig:string.lower start -->

```yaoxiang
lower: (s: &String) -> String
```

<!-- stdlib:sig:string.lower end -->

小文字に変換する（Unicode 対応）。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.lower("ABC") == "abc")
}
```

### replace

<!-- stdlib:sig:string.replace start -->

```yaoxiang
replace: (s: &String, old: &String, new: &String) -> String
```

<!-- stdlib:sig:string.replace end -->

`s` 中の**すべて**の `old` を `new` に置換する。

- `old` —— 空文字列の場合は `s` を**そのまま返す**（挿入しない）

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.replace("a-b-c", "-", "+") == "a+b+c")
    assert(string.replace("abc", "", "x") == "abc")
}
```

### contains

<!-- stdlib:sig:string.contains start -->

```yaoxiang
contains: (s: &String, sub: &String) -> Bool
```

<!-- stdlib:sig:string.contains end -->

`sub` が `s` 中に現れるか否か。空文字列は常に `true`。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.contains("hello", "ell"))
    assert(!string.contains("hello", "xyz"))
}
```

### starts_with

<!-- stdlib:sig:string.starts_with start -->

```yaoxiang
starts_with: (s: &String, prefix: &String) -> Bool
```

<!-- stdlib:sig:string.starts_with end -->

`s` が `prefix` で始まるか否か。空の `prefix` は常に `true`。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.starts_with("hello", "he"))
}
```

### ends_with

<!-- stdlib:sig:string.ends_with start -->

```yaoxiang
ends_with: (s: &String, suffix: &String) -> Bool
```

<!-- stdlib:sig:string.ends_with end -->

`s` が `suffix` で終わるか否か。空の `suffix` は常に `true`。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.ends_with("hello", "lo"))
}
```

### index_of

<!-- stdlib:sig:string.index_of start -->

```yaoxiang
index_of: (s: &String, sub: &String) -> Int
```

<!-- stdlib:sig:string.index_of end -->

`sub` が最初に現れる位置の**バイト**インデックス。

戻り値：見つかった場合はインデックスを返し、見つからなかった場合は `-1`。

> 返されるのはバイトオフセットである。マルチバイト文字を含む場合、`chars`
> で変換してから文字位置を調べること。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.index_of("hello", "ll") == 2)
    assert(string.index_of("hello", "xyz") == -1)
}
```

### substring

<!-- stdlib:sig:string.substring start -->

```yaoxiang
substring: (s: &String, start: Int, end: Int) -> String
```

<!-- stdlib:sig:string.substring end -->

**文字**インデックスで `[start, end)` の範囲を取得する。

- `start` —— 開始文字インデックス。デフォルト `0`
- `end` —— 終了文字インデックス（含まない）。デフォルトは文字列の末尾

戻り値：切り出した結果。範囲外の境界は有効な範囲に**クランプ**され、エラーにはならない。`start > end`
の場合は空文字列にクランプされる。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.substring("hello", 1, 4) == "ell")
    assert(string.substring("hello", 1, 99) == "ello")   // 上限のクランプ
}
```

### is_empty

<!-- stdlib:sig:string.is_empty start -->

```yaoxiang
is_empty: (s: &String) -> Bool
```

<!-- stdlib:sig:string.is_empty end -->

`s` が空文字列か否か。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.is_empty(""))
    assert(!string.is_empty("x"))
}
```

### len

<!-- stdlib:sig:string.len start -->

```yaoxiang
len: (s: &String) -> Int
```

<!-- stdlib:sig:string.len end -->

**UTF-8 バイト長** を返す。文字数ではない。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.len("hello") == 5)
    assert(string.len("中") == 3)   // バイト長
}
```

### chars

<!-- stdlib:sig:string.chars start -->

```yaoxiang
chars: (s: &String) -> Vec(String)
```

<!-- stdlib:sig:string.chars end -->

単一文字の文字列リストに分解する（Unicode スカラー値ごと）。

```yaoxiang
use std.assert
use std.list
use std.string

main: () -> Void = {
    cs = string.chars("ab")
    assert(cs.length == 2)
    assert(cs[0] == "a")
}
```

### concat

<!-- stdlib:sig:string.concat start -->

```yaoxiang
concat: (s1: &String, s2: &String) -> String
```

<!-- stdlib:sig:string.concat end -->

2 つの文字列を連結する。`+` 演算子を直接使用してもよい。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.concat("a", "b") == "ab")
}
```

### repeat

<!-- stdlib:sig:string.repeat start -->

```yaoxiang
repeat: (s: &String, n: Int) -> String
```

<!-- stdlib:sig:string.repeat end -->

`s` を `n` 回繰り返す。

- `n` —— 繰り返し回数。`n <= 0` の場合は空文字列を返す

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.repeat("ab", 3) == "ababab")
    assert(string.repeat("ab", 0) == "")
}
```

### reverse

<!-- stdlib:sig:string.reverse start -->

```yaoxiang
reverse: (s: &String) -> String
```

<!-- stdlib:sig:string.reverse end -->

文字単位で反転する。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.reverse("abc") == "cba")
}
```

### format

<!-- stdlib:sig:string.format start -->

```yaoxiang
format: (format: &String, ...args) -> String
```

<!-- stdlib:sig:string.format end -->

`{index}` プレースホルダに従ってフォーマットする。オプションで幅／配置指定子も使用可能。

プレースホルダの構文：

| 形式     | 意味                                             |
| -------- | ------------------------------------------------ |
| `{0}`    | 0 番目の引数（`format` の後の引数は 0 から採番） |
| `{0:03}` | 幅 3                                             |
| `{0:>3}` | 幅 3、右寄せ（デフォルト）                       |
| `{0:<3}` | 幅 3、左寄せ                                     |
| `{0:^3}` | 幅 3、中央寄せ                                   |

リテラルの波括弧は二重に記述して表現する：左波括弧 2 つでリテラル左波括弧 1 つ、右波括弧 2 つでリテラル右波括弧 1 つ。

戻り値：フォーマット後の文字列。引数はまず文字列に変換される（`convert.to_string`
と同様）。インデックスが範囲外の場合は空文字列、不正な幅は `0` として扱われる。

```yaoxiang
use std.assert
use std.string

main: () -> Void = {
    assert(string.format("{0}-{1}", "a", "b") == "a-b")
    assert(string.format("[{0:>5}]", "ab") == "[   ab]")
    assert(string.format("[{0:<5}]", "ab") == "[ab   ]")
}
```

### parse_int

<!-- stdlib:sig:string.parse_int start -->

```yaoxiang
parse_int: (s: &String) -> Result(Int, Error)
```

<!-- stdlib:sig:string.parse_int end -->

十進整数を解析する（先頭と末尾の空白は自動的に除去）。

戻り値：成功時は `Result.ok(Int)`、失敗時は `Result.err(Error)` で、その `code` は
`E6010`。**例外をスローしない**。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_int("42")))
    assert(result.is_err(string.parse_int("abc")))
}
```

### parse_float

<!-- stdlib:sig:string.parse_float start -->

```yaoxiang
parse_float: (s: &String) -> Result(Float, Error)
```

<!-- stdlib:sig:string.parse_float end -->

浮動小数点数を解析する（先頭と末尾の空白は自動的に除去）。

戻り値：成功時は `Result.ok(Float)`、失敗時は `Result.err(Error)` で、その `code` は `E6011`。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_float("3.14")))
    assert(result.is_err(string.parse_float("xxx")))
}
```

## 関連

- [`std.convert`](./convert) —— 数値から文字列への変換
- [`std.result`](./result) —— `parse_*` の結果のアンラップ
