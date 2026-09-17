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

> **実装状況に関する警告（#56）**：本モジュールの4つの関数のうち、`url_encode` /
> `url_decode`のみが実際の実装です。`http_get` /
> `http_post`は**プレースホルダ実装であり、ネットワークリクエストは一切行わず**、単に引数を文字列として連結して返します。詳細は各関数の説明を参照してください。

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

> **プレースホルダ実装であり、HTTPクライアントには接続されていません（#56）。** 現在の動作は引数を
> `"GET: {url}"`という文字列に連結して返すだけであり、**ネットワークリクエストは一切行わず**、レスポンスボディも返しません。これに依存して実際のHTTP呼び出しを行うと静かに失敗します。取得できるのは説明的な文字列であり、レスポンスの内容ではありません。

- `url` —— リクエストアドレス（読み取り専用借用）

戻り値：`"GET: http://example.com"`のような形式の文字列。エラー：引数が欠落している場合、`E6007`をスローします。引数が`String`でない場合、型エラーをスローします。

```yaoxiang
use std.assert
use std.net

main = {
    // 現在の実装はレスポンスボディではなく説明文字列を返す
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

> **プレースホルダ実装であり、HTTPクライアントには接続されていません（#56）。**
> 現在の動作は`"POST {url}: {body}"`という文字列を連結して返すだけであり、**ネットワークリクエストは一切行いません**。

- `url` —— リクエストアドレス（読み取り専用借用）
- `body` —— リクエストボディ（読み取り専用借用）

戻り値：`"POST http://example.com: hello"`のような形式の文字列。エラー：引数が不足している場合、`E6007`をスローします。引数の型が一致しない場合、型エラーをスローします。

```yaoxiang
use std.assert
use std.net

main = {
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

パーセントエンコーディング。

- `s` —— エンコード対象の文字列（読み取り専用借用）

戻り値：エンコード後の文字列。スペースは`%20`としてエンコードされます（`+`ではない）。予約文字はRFC
3986に従ってエスケープされます。非予約文字はそのまま保持されます。

エラー：引数が欠落している場合、`E6007`をスローします。引数が`String`でない場合、型エラーをスローします。

```yaoxiang
use std.assert
use std.net

main = {
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

パーセントデコード。`url_encode`と逆関数の関係です。

- `s` —— エンコード済みの文字列（読み取り専用借用）

戻り値：デコード後の文字列。不正なエスケープシーケンスはそのまま保持されます。

エラー：引数が欠落している場合、`E6007`をスローします。引数が`String`でない場合、型エラーをスローします。

```yaoxiang
use std.assert
use std.net

main = {
    assert(net.url_decode("a%20b") == "a b")

    // ラウンドトリップ整合性
    orig = "hello world & friends"
    assert(net.url_decode(net.url_encode(orig)) == orig)
}
```

## 関連

- [エラーコードリファレンス](../error-code/) —— `E6007` 汎用ランタイムエラー
