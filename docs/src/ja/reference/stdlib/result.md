---
title: 'std.result'
description: 'Result と Error の構築と分解'
---

# std.result

`Result(T, E)` の構築と分解、および `Error` キャリアのフィールドアクセス。

```yaoxiang
use std.result
```

## ランタイム表現

| 値                  | 表現                                   |
| ------------------- | -------------------------------------- |
| `Result.ok(value)`  | 列挙型バリアント、`value` を保持       |
| `Result.err(error)` | 列挙型バリアント、`error` を保持       |
| `Error`             | 構造体、フィールドは `(code, message)` |

`Error.code` は RFC-013 の `E6xxx` / `E7xxx`
セグメント登録コード（バージョン間安定契約）、`Error.message` は人間が読める説明です。

## 関数一覧

<!-- stdlib:table:result start -->

| 関数         | シグネチャ                                                 |
| ------------ | ---------------------------------------------------------- |
| `is_ok`      | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `is_err`     | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `unwrap`     | `(T: Type, E: Type)(self: &Result(T, E)) -> T`             |
| `unwrap_or`  | `(T: Type, E: Type)(self: &Result(T, E), default: T) -> T` |
| `ok`         | `(T: Type, E: Type)(value: T) -> Result(T, E)`             |
| `err`        | `(T: Type, E: Type)(error: E) -> Result(T, E)`             |
| `unwrap_err` | `(T: Type, E: Type)(self: &Result(T, E)) -> E`             |
| `code`       | `(self: &Error) -> String`                                 |
| `message`    | `(self: &Error) -> String`                                 |

<!-- stdlib:table:result end -->

## 構築

### ok

<!-- stdlib:sig:result.ok start -->

```yaoxiang
ok: (T: Type, E: Type)(value: T) -> Result(T, E)
```

<!-- stdlib:sig:result.ok end -->

成功値を包みます。

`?` で分解された `Ok` 値は `Result` の戻り型に沿って伝播を続けるために再包装が必要であり、`ok`
がそのラッパーです。

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.ok(42)
    assert(result.is_ok(r))
}
```

### err

<!-- stdlib:sig:result.err start -->

```yaoxiang
err: (T: Type, E: Type)(error: E) -> Result(T, E)
```

<!-- stdlib:sig:result.err end -->

エラー値を包みます。

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.err("boom")
    assert(result.is_err(r))
}
```

## 判定

### is_ok

<!-- stdlib:sig:result.is_ok start -->

```yaoxiang
is_ok: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_ok end -->

成功バリアントかどうかを判定します。読み取り専用借用で、`self` は繰り返し使用可能です。

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.ok(1)
    assert(result.is_ok(r))
    assert(result.is_ok(r))      // 再利用可能
}
```

### is_err

<!-- stdlib:sig:result.is_err start -->

```yaoxiang
is_err: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_err end -->

エラーバリアントかどうかを判定します。読み取り専用借用。

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = result.err("e")
    assert(result.is_err(r))
}
```

## 値の取り出し

### unwrap

<!-- stdlib:sig:result.unwrap start -->

```yaoxiang
unwrap: (T: Type, E: Type)(self: &Result(T, E)) -> T
```

<!-- stdlib:sig:result.unwrap end -->

成功値を取り出します。

戻り値：`Ok` バリアントが保持する値。エラー：`Err` 値に対して呼び出した場合 `E6007`
を送出し、メッセージには**元々のエラーコードと説明が付属**し、例えば
`unwrap called on Err value (E6010: parse_int: ...)` のようになります。そのため `unwrap_err`
を先に呼ばなくても失敗原因を確認できます。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("42")
    assert(result.unwrap(r) == 42)
}
```

### unwrap_or

<!-- stdlib:sig:result.unwrap_or start -->

```yaoxiang
unwrap_or: (T: Type, E: Type)(self: &Result(T, E), default: T) -> T
```

<!-- stdlib:sig:result.unwrap_or end -->

成功値を取り出すか、`Err` の場合は `default` を返します。

- `default` —— `Err` 時の既定値

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    good = string.parse_int("42")
    assert(result.unwrap_or(good, 0) == 42)

    bad = string.parse_int("abc")
    assert(result.unwrap_or(bad, 0) == 0)
}
```

### unwrap_err

<!-- stdlib:sig:result.unwrap_err start -->

```yaoxiang
unwrap_err: (T: Type, E: Type)(self: &Result(T, E)) -> E
```

<!-- stdlib:sig:result.unwrap_err end -->

エラー値を取り出します。

戻り値：`Err` バリアントが保持する値。エラー：`Ok` 値に対して呼び出した場合 `E6007` を送出します。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(result.is_err(r))
}
```

## Error フィールド

### code

<!-- stdlib:sig:result.code start -->

```yaoxiang
code: (self: &Error) -> String
```

<!-- stdlib:sig:result.code end -->

エラーコード文字列を `"E6010"` のような形式で読み取ります。

> シグネチャの型は `Error` ですが、ランタイムエラーキャリアは `(code, message)`
> をフィールドとする構造体です。`Error` 値に対して直接呼び出します。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(result.code(e) == "E6010")
}
```

### message

<!-- stdlib:sig:result.message start -->

```yaoxiang
message: (self: &Error) -> String
```

<!-- stdlib:sig:result.message end -->

エラー説明テキストを読み取ります。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    r = string.parse_int("abc")
    e = result.unwrap_err(r)
    assert(string.len(result.message(e)) > 0)
}
```

## 関連

- [`std.string`](./string#parse_int) —— `Result` を生成する解析関数
- [エラーコードリファレンス](../error-code/) —— `E6010` / `E6011` などのランタイムエラー値コード
