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
| `Result.ok(value)`  | バリアント、`value` を保持             |
| `Result.err(error)` | バリアント、`error` を保持             |
| `Error`             | 構造体、フィールドは `(code, message)` |

`Error.code` は RFC-013 の `E6xxx` / `E7xxx`
セグメント登録コード（バージョン横断の安定契約）、`Error.message` は人間が読める説明。

## 関数一覧

<!-- stdlib:table:result start -->

| 関数         | 署名                                                       |
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

成功値をラップする。

`?` で分解された `Ok` 値を `Result` 戻り型に沿って伝播させるには再ラップが必要であり、`ok`
がそのラッパーとなる。

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

エラー値をラップする。

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

成功バリアントかどうか。読み取り専用の借用で、`self` は反復して使用可能。

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

エラーバリアントかどうか。読み取り専用の借用。

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

成功値を取り出す。

戻り値：`Ok` バリアントが保持する値。エラー：`Err` 値に対して呼び出すと `E6007`
が送出され、メッセージには**元エラーコードと説明が付記**され、`unwrap called on Err value (E6010: parse_int: ...)`
のような形式となるため、事前に `unwrap_err` を呼ぶことなく失敗原因を確認できる。

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

成功値を取り出すか、`Err` の場合は `default` を返す。

- `default` —— `Err` 時のフォールバック値

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

エラー値を取り出す。

戻り値：`Err` バリアントが保持する値。エラー：`Ok` 値に対して呼び出すと `E6007` が送出される。

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

エラーコード文字列（例：`"E6010"`）を読み取る。

> 署名の型は `Error` だが、ランタイムのエラーキャリアは `(code, message)`
> をフィールドとする構造体である。`Error` 値に対して直接呼び出す。

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

エラー説明テキストを読み取る。

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
