---
title: 'std.time'
description: 'タイムスタンプ、フォーマット、DateTime フィールドアクセス'
---

# std.time

時間モジュール。

```yaoxiang
use std.time
```

> **実装ギャップは修正済み（#338 / #340、2026-09-19）**。
>
> `DateTime` は現在**タイムスタンプのエイリアス**（すなわち `Int`）です——ランタイム本来就是
> `RuntimeValue::Int` で、以前は実体のない名前だったため `now()` の戻り値を `format_time`
> やアクセサに渡せませんでした。アクセサのエクスポート名も `DateTime::year` から **`datetime_year`**
> に変更されました（`::` は字句予約トークンであり、フィールドアクセス位置には現れません）。
>
> 現在では `now()` / `parse_time()` の戻り値を直接算術・フォーマット・全アクセサに使用できます。

## 関数一覧

<!-- stdlib:table:time start -->

| 関数                 | シグネチャ                             |
| -------------------- | -------------------------------------- |
| `now`                | `() -> DateTime`                       |
| `timestamp`          | `() -> Int`                            |
| `timestamp_ms`       | `() -> Int`                            |
| `sleep`              | `(seconds: Float) -> Void`             |
| `format_time`        | `(dt: Int, fmt: String) -> String`     |
| `parse_time`         | `(fmt: String, s: String) -> DateTime` |
| `datetime_year`      | `(dt: Int) -> Int`                     |
| `datetime_month`     | `(dt: Int) -> Int`                     |
| `datetime_day`       | `(dt: Int) -> Int`                     |
| `datetime_hour`      | `(dt: Int) -> Int`                     |
| `datetime_minute`    | `(dt: Int) -> Int`                     |
| `datetime_second`    | `(dt: Int) -> Int`                     |
| `datetime_weekday`   | `(dt: Int) -> Int`                     |
| `datetime_to_string` | `(dt: Int) -> String`                  |

<!-- stdlib:table:time end -->## 時刻取得

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> DateTime
```

<!-- stdlib:sig:time.now end -->

現在時刻を返します。

戻り値：`DateTime` 値。表示は `DateTime(1789471990)` の形式です。**これは `Int`
ではない**ため、直接算術や比較に参加できず、また `Int`
仮引数を受け取る他の関数にも渡せません（[`format_time`](#format_time) 参照）。

```yaoxiang
use std.time

main: () -> Void = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> 計算に参加させる場合は [`timestamp`](#timestamp) または [`timestamp_ms`](#timestamp_ms)
> を使用してください。これらは直接 `Int` を返します。

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

現在の Unix タイムスタンプ（**秒**）を返します。直接算術や比較に参加できます。

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    assert(time.timestamp() > 0)
}
```

### timestamp_ms

<!-- stdlib:sig:time.timestamp_ms start -->

```yaoxiang
timestamp_ms: () -> Int
```

<!-- stdlib:sig:time.timestamp_ms end -->

現在の Unix タイムスタンプ（**ミリ秒**）を返します。

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // ミリ秒精度は秒精度を下回らない
    assert(time.timestamp_ms() >= time.timestamp())
}
```

### sleep

<!-- stdlib:sig:time.sleep start -->

```yaoxiang
sleep: (seconds: Float) -> Void
```

<!-- stdlib:sig:time.sleep end -->

指定された**秒数**（小数可）スリープします。同名の [`std.concurrent.sleep`](./concurrent#sleep)
は**ミリ秒**単位なので注意してください。

- `seconds` —— スリープ秒数。`Int`（秒として解釈）または `Float` を受け付けます

エラー：引数が `Int` でも `Float` でもない場合、`E6007` が発生します。

```yaoxiang
use std.time

main: () -> Void = {
    time.sleep(0.0)
}
```

> `wasm32` ターゲットではエクスポートされません。

## フォーマットと解析

### format_time

<!-- stdlib:sig:time.format_time start -->

```yaoxiang
format_time: (dt: Int, fmt: String) -> String
```

<!-- stdlib:sig:time.format_time end -->

タイムスタンプを `fmt` にしたがってフォーマットします。`strftime`
形式のプレースホルダをサポートします。

- `dt` —— Unix タイムスタンプ（**秒**）。`Int` 必須
- `fmt` —— フォーマット文字列

> **型に関する注意**：`dt` は `Int` 必須です。[`now`](#now) や [`parse_time`](#parse_time)
> の戻り値を渡すと `E1002`（`expected type 'int64', found type 'DateTime'`）が発生します。これらは
> `DateTime` であるためです。現時点では `DateTime` → `Int` の変換手段がないため、**実際には `Int`
> リテラルまたは [`timestamp`](#timestamp) の結果のみ渡せます**。

サポートされるプレースホルダ：

| プレースホルダ | 意味                    | 例           |
| -------------- | ----------------------- | ------------ |
| `%Y`           | 4 桁の年                | `2024`       |
| `%m`           | 2 桁の月                | `01`         |
| `%d`           | 2 桁の日                | `15`         |
| `%H`           | 2 桁の時間（24 時間制） | `10`         |
| `%M`           | 2 桁の分                | `30`         |
| `%S`           | 2 桁の秒                | `00`         |
| `%w`           | 曜日（0 = 日曜）        | `1`          |
| `%F`           | `%Y-%m-%d` と等価       | `2024-01-15` |
| `%T`           | `%H:%M:%S` と等価       | `10:30:00`   |

**ローカル時間**で分解します。認識できないプレースホルダはそのまま保持されます。

エラー：`dt` が `Int` でない、`fmt` が `String` でない、または引数が不足している場合、`E6007`
が発生します。

```yaoxiang
use std.assert
use std.string
use std.time

main: () -> Void = {
    // 0 = Unix epoch
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")

    // 現在のタイムスタンプ（Int）も直接使用可能
    s = time.format_time(time.timestamp(), "%Y")
    assert(string.len(s) == 4)
}
```

### parse_time

<!-- stdlib:sig:time.parse_time start -->

```yaoxiang
parse_time: (fmt: String, s: String) -> DateTime
```

<!-- stdlib:sig:time.parse_time end -->

時間文字列を解析して時刻を返します。

- `fmt` —— **現在無視されます**（後述）
- `s` —— 解析対象の文字列

戻り値：`DateTime` 値。

> **2 つの実装上の制限（#340）**：
>
> 1. `fmt` 引数は**解析に参加しません**。この関数は ISO 8601 形式の `YYYY-MM-DDTHH:MM:SS` または
>    `YYYY-MM-DD HH:MM:SS`（日付と時刻の間は `T` または空白区切り）のみを認識し、それ以外の形式は
>    `fmt` に関わらずすべて失敗します。
> 2. 返される `DateTime` は**現時点では使用できません**——`Int` ではないため（`format_time` /
>    `DateTime::*` に渡せない）、呼び出し可能なアクセサもありません（次節参照）。

エラー：フォーマットが一致しない場合、`E6007` が発生します。

```yaoxiang
use std.time

main: () -> Void = {
    // 解析自体は成功する
    ts = time.parse_time("", "2024-01-15 10:30:00")
    println(ts)
}
```

## DateTime フィールドアクセス

`std.time` は 8 つの日付要素アクセサをエクスポートします（`#338`
で修正済み、エクスポート名はフラット形式）：

| エクスポート名       | シグネチャ            | 説明                  |
| -------------------- | --------------------- | --------------------- |
| `datetime_year`      | `(dt: Int) -> Int`    | 4 桁の年              |
| `datetime_month`     | `(dt: Int) -> Int`    | 月（1–12）            |
| `datetime_day`       | `(dt: Int) -> Int`    | 日（1–31）            |
| `datetime_hour`      | `(dt: Int) -> Int`    | 時（0–23）            |
| `datetime_minute`    | `(dt: Int) -> Int`    | 分（0–59）            |
| `datetime_second`    | `(dt: Int) -> Int`    | 秒（0–59）            |
| `datetime_weekday`   | `(dt: Int) -> Int`    | 曜日（0 = 日曜）      |
| `datetime_to_string` | `(dt: Int) -> String` | ISO 8601 形式の文字列 |

`DateTime` は**タイムスタンプのエイリアス**（すなわち `Int`）です——`now()` / `parse_time()`
の戻り値は直接これらのアクセサに渡すことができ、また [`format_time`](#format_time)
にも直接使用できます：

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    ts = time.parse_time("%Y-%m-%d", "2024-01-15")
    assert(time.datetime_year(ts) == 2024)
    assert(time.datetime_month(ts) == 1)
    assert(time.datetime_day(ts) == 15)

    // format_time と同等
    assert(time.format_time(ts, "%Y-%m-%d") == "2024-01-15")
}
```

## 関連

- [`std.concurrent`](./concurrent) —— ミリ秒単位のスリープ
- [エラーコードリファレンス](../error-code/) —— `E6007` 汎用ランタイムエラー
