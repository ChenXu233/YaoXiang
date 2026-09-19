---
title: 'std.net'
description: 'HTTPリクエストとURLパーセントエンコーディング'
---

# std.net

ネットワークモジュール。

```yaoxiang
use std.net
```

> 本モジュールはオペレーティングシステムのネットワーク機能に依存しており、`wasm32`ターゲットでは**エクスポートされません**。

> **実装状態の警告（#56）**：本モジュールの4つの関数のうち、`url_encode` / `url_decode`
> のみが実際の実装です。`http_get` / `http_post`
> は**プレースホルダ実装であり、ネットワークリクエストを一切発行しません**。引数を文字列に連結して返すだけです。詳細は各項目を参照してください。

## 関数一覧

<!-- stdlib:table:net start -->

| 関数         | シグネチャ                                |
| ------------ | ----------------------------------------- |
| `http_get`   | `(url: &String) -> String`                |
| `http_post`  | `(url: &String, body: &String) -> String` |
| `url_encode` | `(s: &String) -> String`                  |
| `url_decode` | `(s: &String) -> String`                  |

<!-- stdlib:table:net end -->

## 関数

### http_get

<!-- stdlib:sig:net.http_get start -->

```yaoxiang
http_get: (url: &String) -> String
```

<!-- stdlib:sig:net.http_get end -->

> **プレースホルダ実装。HTTPクライアントには接続されていません（#56）。** 現在の動作は、引数を
> `"GET: {url}"`
> という文字列に連結して返すだけであり、**いかなるネットワークリクエストも発行しません**。レスポンスボディも返しません。これに依存して実際のHTTP呼び出しを行うと静かに失敗します。取得できるのはレスポンスの内容ではなく、説明用の文字列です。

- `url` —— リクエストURL（読み取り専用借用）

戻り値：`"GET: http://example.com"` 形式の文字列。エラー：引数が不足している場合 `E6007`
をスローします。引数が `String` 以外の場合は型エラーをスローします。

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    // 現在の実装はレスポンスボディではなく説明用文字列を返す
    r = net.http_get("http://example.com")
    assert(r == "GET: http://example.com")
}
```

### http_post

<!-- stdlib:sig:net.http_post start -->

```yaoxiang
http_post: (url: &String, body: &String) -> String
```

<!-- stdlib:sig:net.http_post end -->

> **プレースホルダ実装。HTTPクライアントには接続されていません（#56）。**
> 現在の動作は、`"POST {url}: {body}"`
> を連結した文字列を返すだけであり、**いかなるネットワークリクエストも発行しません**。

- `url` —— リクエストURL（読み取り専用借用）
- `body` —— リクエストボディ（読み取り専用借用）

戻り値：`"POST http://example.com: hello"` 形式の文字列。エラー：引数が不足している場合 `E6007`
をスローします。引数の型が一致しない場合は型エラーをスローします。

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    r = net.http_post("http://example.com", "hello")
    assert(r == "POST http://example.com: hello")
}
```

### url_encode

<!-- stdlib:sig:net.url_encode start -->

```yaoxiang
url_encode: (s: &String) -> String
```

<!-- stdlib:sig:net.url_encode end -->

パーセントエンコーディング（percent-encoding）。

- `s` —— エンコード対象の文字列（読み取り専用借用）

戻り値：エンコード後の文字列。空白は `%20` にエンコードされます（`+` ではありません）。予約文字はRFC
3986に従ってエスケープされます。非予約文字はそのまま保持されます。

エラー：引数が不足している場合 `E6007` をスローします。引数が `String`
以外の場合は型エラーをスローします。

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    assert(net.url_encode("a b") == "a%20b")
    assert(net.url_encode("a&b=c?d") == "a%26b%3Dc%3Fd")
}
```

### url_decode

<!-- stdlib:sig:net.url_decode start -->

```yaoxiang
url_decode: (s: &String) -> String
```

<!-- stdlib:sig:net.url_decode end -->

パーセントデコード。`url_encode` と逆関数の関係です。

- `s` —— エンコード済みの文字列（読み取り専用借用）

戻り値：デコード後の文字列。不正なエスケープシーケンスはそのまま保持されます。

エラー：引数が不足している場合 `E6007` をスローします。引数が `String`
以外の場合は型エラーをスローします。

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    assert(net.url_decode("a%20b") == "a b")

    // ラウンドトリップで一致
    orig = "hello world & friends"
    assert(net.url_decode(net.url_encode(orig)) == orig)
}
```

## 関連項目

- [エラーコードリファレンス](../error-code/) —— `E6007` 汎用ランタイムエラー
