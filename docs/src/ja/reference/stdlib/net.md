---
title: 'std.net'
description: 'HTTP リクエストと URL パーセントエンコーディング'
---

# std.net

ネットワークモジュール。

```yaoxiang
use std.net
```

> 本モジュールはオペレーティングシステムのネットワーク機能に依存しており、`wasm32`
> ターゲットでは**エクスポートされません**。

HTTP 関数は同期ブロッキングクライアント実装（rustls
TLS）に基づき、リダイレクトはデフォルトで追跡します（最大 5 回）。ブロッキングは呼び出しが置かれている実行スレッドにのみ作用し、[`spawn`](../../language-spec/concurrency)
の明示的並列モデルと整合します。

## 関数一覧

<!-- stdlib:table:net start -->

| 関数         | シグネチャ                                                                                                |
| ------------ | --------------------------------------------------------------------------------------------------------- |
| `http_get`   | `(url: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)`                |
| `http_post`  | `(url: &String, body: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)` |
| `url_encode` | `(s: &String) -> String`                                                                                  |
| `url_decode` | `(s: &String) -> String`                                                                                  |

<!-- stdlib:table:net end -->

## 関数

### http_get

<!-- stdlib:sig:net.http_get start -->

```yaoxiang
http_get: (url: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)
```

<!-- stdlib:sig:net.http_get end -->

HTTP GET リクエストを発行し、構造化されたレスポンス辞書を返します。

| キー      | 型                     | 意味                                                                   |
| --------- | ---------------------- | ---------------------------------------------------------------------- |
| `status`  | `Int`                  | HTTP ステータスコード（例：`200`、`404`）                              |
| `headers` | `Dict(String, String)` | レスポンスヘッダー、キーは統一して小文字、同名の複数値は `", "` で結合 |
| `body`    | `String`               | レスポンスボディ（UTF-8 でデコード）                                   |

- `url` —— リクエストアドレス（読み取り専用借用）
- `headers` —— オプションのリクエストヘッダー辞書、省略時は `{}` を送信
- `timeout_secs` —— オプションの全体タイムアウト秒数、省略時は `30`

**4xx/5xx は正常なレスポンス**（`status`
フィールドにステータスコードを含む）であり、エラーではありません。トランスポート層の障害（DNS 解決失敗、接続拒否、タイムアウトなど）のみが
`E6007` をスローします。

```yaoxiang
use std.net

// レスポンス形態の例（ネットワークが必要、実行可能なサンプルではありません）：
// resp = net.http_get("https://httpbin.org/get")
// status = resp["status"]        // 200
// body   = resp["body"]          // レスポンスボディテキスト
// ctype  = resp["headers"]["content-type"]
```

リクエストヘッダーとカスタムタイムアウトを携带：

```yaoxiang
use std.net

// net.http_get(
//     "https://api.example.com/v1/data",
//     { "Authorization": "Bearer token123" },
//     10,
// )
```

### http_post

<!-- stdlib:sig:net.http_post start -->

```yaoxiang
http_post: (url: &String, body: &String, ?headers: &Dict(String, String), ?timeout_secs: Int) -> Dict(String, Any)
```

<!-- stdlib:sig:net.http_post end -->

HTTP
POST リクエストを発行します。リクエストボディは UTF-8 でエンコードされて送信され、`Content-Length`
は自動的に設定されます。レスポンス辞書の構造は [`http_get`](#http_get) と同じです。

- `url` —— リクエストアドレス（読み取り専用借用）
- `body` —— リクエストボディ（読み取り専用借用）
- `headers` —— オプションのリクエストヘッダー辞書、省略時は `{}` を送信
- `timeout_secs` —— オプションの全体タイムアウト秒数、省略時は `30`

```yaoxiang
use std.net

// net.http_post(
//     "https://httpbin.org/post",
//     "payload=1&name=yx",
//     { "Content-Type": "application/x-www-form-urlencoded" },
// )
```

### url_encode

<!-- stdlib:sig:net.url_encode start -->

```yaoxiang
url_encode: (s: &String) -> String
```

<!-- stdlib:sig:net.url_encode end -->

パーセントエンコーディング（percent-encoding）。

- `s` —— エンコード対象の文字列（読み取り専用借用）

戻り値：エンコード後の文字列。スペースは `%20` にエンコード（`+` ではない）、予約文字は RFC
3986 に従ってエスケープ、非予約文字はそのまま保持されます。

エラー：引数が欠落している場合 `E6007` をスロー；引数が `String` でない場合は型エラーをスロー。

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

パーセントデコード。`url_encode` と可逆。

- `s` —— エンコード済み文字列（読み取り専用借用）

戻り値：デコード後の文字列。不正なエスケープシーケンスはそのまま保持されます。

エラー：引数が欠落している場合 `E6007` をスロー；引数が `String` でない場合は型エラーをスロー。

```yaoxiang
use std.assert
use std.net

main: () -> Void = {
    assert(net.url_decode("a%20b") == "a b")

    // ラウンドトリップの一貫性
    orig = "hello world & friends"
    assert(net.url_decode(net.url_encode(orig)) == orig)
}
```

## 関連

- [エラーコードリファレンス](../error-code/) —— `E6007` 汎用ランタイムエラー
