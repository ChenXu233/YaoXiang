---
title: 'std.result'
description: 'Result と Error の構築とアンラップ'
---

# std.result

`Result(T, E)` のアンラップと `Error` キャリアのフィールドアクセス。`Result` 自体は `std.result`
がエクスポートする和型（RFC-010）です。構築は**バリアント構築構文**、分解は `match`
バリアントパターン、 `?` 伝播は `Try` インターフェースが駆動します（以下参照）。

```yaoxiang
use std.result

r = Result(Int, String).ok(5)
e = Result(Int, String).err("boom")
```

## ランタイム表現

| 値                        | 表現                                   |
| ------------------------- | -------------------------------------- |
| `Result(T, E).ok(value)`  | 列挙バリアント、`value` を保持         |
| `Result(T, E).err(error)` | 列挙バリアント、`error` を保持         |
| `Error`                   | 構造体、フィールドは `(code, message)` |

`Error.code` は RFC-013 の `E6xxx` / `E7xxx`
セグメント登録コード（バージョン間で安定的な契約）、`Error.message` は人間が読める説明です。

## Try インターフェース（`?` 伝播）

`Result` は型本体で `Try(Result(T, E), T, E)` の 4 メソッドインターフェースをインスタンス化し、`?`
演算子はこれに基づいて駆動します。`is_failure` は失敗を判定、`success`
は成功ペイロードを取得、`residual` は失敗ペイロードを取得、 `from_error` はエラー値から `Result`
を再構築します。これらのメソッドは明示的にも呼び出せます。

## 関数一覧

<!-- stdlib:table:result start -->

| 関数         | シグネチャ                                                 |
| ------------ | ---------------------------------------------------------- |
| `is_ok`      | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `is_err`     | `(T: Type, E: Type)(self: &Result(T, E)) -> Bool`          |
| `unwrap`     | `(T: Type, E: Type)(self: &Result(T, E)) -> T`             |
| `unwrap_or`  | `(T: Type, E: Type)(self: &Result(T, E), default: T) -> T` |
| `unwrap_err` | `(T: Type, E: Type)(self: &Result(T, E)) -> E`             |
| `code`       | `(self: &Error) -> String`                                 |
| `message`    | `(self: &Error) -> String`                                 |
| `error`      | `(code: &String, message: &String) -> Error`               |

<!-- stdlib:table:result end -->## 判定

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
    r = Result(Int, String).ok(1)
    assert(result.is_ok(r))
    assert(result.is_ok(r))      // 再利用可
}
```

### is_err

<!-- stdlib:sig:result.is_err start -->

```yaoxiang
is_err: (T: Type, E: Type)(self: &Result(T, E)) -> Bool
```

<!-- stdlib:sig:result.is_err end -->

エラーバリアントかどうかを判定します。読み取り専用借用です。

```yaoxiang
use std.assert
use std.result

main: () -> Void = {
    r = Result(Int, String).err("e")
    assert(result.is_err(r))
}
```

## 値の取得

### unwrap

<!-- stdlib:sig:result.unwrap start -->

```yaoxiang
unwrap: (T: Type, E: Type)(self: &Result(T, E)) -> T
```

<!-- stdlib:sig:result.unwrap end -->

成功値を取り出します。

戻り値：`Ok` バリアントが保持する値。エラー：`Err` 値に対して呼び出すと `E6007`
をスローし、メッセージには**元のエラーコードと説明が含まれます**。形式は
`unwrap called on Err value (E6010: parse_int: ...)` のようになり、`unwrap_err`
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

- `default` — `Err` 時のフォールバック値

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

戻り値：`Err` バリアントが保持する値。エラー：`Ok` 値に対して呼び出すと `E6007` をスローします。

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

エラーコード文字列を読み取ります（例：`"E6010"`）。

> シグネチャの型は `Error` ですが、ランタイムのエラーキャリアは `(code, message)`
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

- [`std.string`](./string#parse_int) — `Result` を生成する解析関数
- [エラーコードリファレンス](../error-code/) — `E6010` / `E6011` などのランタイムエラー値コード
