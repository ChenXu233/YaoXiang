---
title: 'std.time'
description: 'タイムスタンプ、フォーマット、DateTime フィールドアクセス'
---

# std.time

時間モジュール。

```yaoxiang
use std.time
```

> **実装上のギャップは修正されました（#338 / #340、2026-09-19）**。
>
> `DateTime` は現在**タイムスタンプの別名**（つまり `Int`）です。実行時には本来 `RuntimeValue::Int`
> であるにもかかわらず、以前は実体のない名前だったため、`now()` の戻り値を `format_time`
> やアクセサに渡すことができませんでした。アクセサのエクスポート名も `DateTime::year` から
> **`datetime_year`** に変更されました（`::`
> は字句の予約記号であり、フィールドアクセスの位置には使用できません）。
>
> これで `now()` / `parse_time()` の戻り値を、算術演算・フォーマット・全アクセサに直接使用できます。

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

<!-- stdlib:table:time end -->

## 時間の取得

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> DateTime
```

<!-- stdlib:sig:time.now end -->

現在時刻を返します。

戻り値：`DateTime` 値。出力形式は `DateTime(1789471990)` のようになります。**これは `Int`
ではない**ため、直接算術演算や比較には参加できず、`Int`
型を引数に取る他の関数にも渡せません（[`format_time`](#format_time) を参照）。

```yaoxiang
use std.time

main: () -> Void = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> 計算に使用する場合は [`timestamp`](#timestamp) または [`timestamp_ms`](#timestamp_ms)
> を使用してください。これらは直接 `Int` を返します。

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

現在の Unix タイムスタンプ（**秒**）を返します。算術演算と比較に直接使用できます。

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
は**ミリ秒**単位である点に注意が必要です。

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

タイムスタンプを `fmt` に従ってフォーマットします。`strftime`
形式のプレースホルダーをサポートします。

- `dt` —— Unix タイムスタンプ（**秒**）。`Int` である必要があります
- `fmt` —— フォーマット文字列

> **型に関する注意**：`dt` は `Int` である必要があります。[`now`](#now) や
> [`parse_time`](#parse_time) の戻り値を渡すと
> `E1002`（`expected type 'int64', found type 'DateTime'`）が報告されます。これらはどちらも
> `DateTime` 型であるためです。現時点では `DateTime` → `Int` の変換手段がないため、**実際には `Int`
> リテラルまたは [`timestamp`](#timestamp) の結果しか渡せません**。

サポートされるプレースホルダー：

| プレースホルダー | 意味                    | 例           |
| ---------------- | ----------------------- | ------------ |
| `%Y`             | 4 桁の年                | `2024`       |
| `%m`             | 2 桁の月                | `01`         |
| `%d`             | 2 桁の日                | `15`         |
| `%H`             | 2 桁の時間（24 時間制） | `10`         |
| `%M`             | 2 桁の分                | `30`         |
| `%S`             | 2 桁の秒                | `00`         |
| `%w`             | 曜日（0 = 日曜日）      | `1`          |
| `%F`             | `%Y-%m-%d` と等価       | `2024-01-15` |
| `%T`             | `%H:%M:%S` と等価       | `10:30:00`   |

**ローカルタイム**で分解されます。認識できないプレースホルダーはそのまま保持されます。

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

時間文字列を解析して時刻に変換します。

- `fmt` —— **現在無視されます**（下記参照）
- `s` —— 解析対象の文字列

戻り値：`DateTime` 値。

> **2 つの実装上の制限（#340）**：
>
> 1. `fmt` 引数は**解析に参加しません**。この関数は ISO 8601 形式の `YYYY-MM-DDTHH:MM:SS` または
>    `YYYY-MM-DD HH:MM:SS`（日付と時刻の間に `T` または空白）だけを認識し、`fmt`
>    の内容に関わらず他の形式は失敗します。
> 2. 返される `DateTime` は**現在そのまま使用できません**。`Int` ではないため（`format_time` /
>    `DateTime::*` に渡せず）、呼び出し可能なアクセサもありません（次節参照）。

エラー：形式が一致しない場合、`E6007` が発生します。

```yaoxiang
use std.time

main: () -> Void = {
    // 解析自体は成功する
    ts = time.parse_time("", "2024-01-15 10:30:00")
    println(ts)
}
```

## DateTime フィールドアクセス（使用不可）

> 追跡：#338

`std.time` は `DateTime::` で始まる 8 個のアクセサをエクスポートしています：

| エクスポート名        | シグネチャ            | 説明                  |
| --------------------- | --------------------- | --------------------- |
| `DateTime::year`      | `(dt: Int) -> Int`    | 4 桁の年              |
| `DateTime::month`     | `(dt: Int) -> Int`    | 月（1–12）            |
| `DateTime::day`       | `(dt: Int) -> Int`    | 日（1–31）            |
| `DateTime::hour`      | `(dt: Int) -> Int`    | 時（0–23）            |
| `DateTime::minute`    | `(dt: Int) -> Int`    | 分（0–59）            |
| `DateTime::second`    | `(dt: Int) -> Int`    | 秒（0–59）            |
| `DateTime::weekday`   | `(dt: Int) -> Int`    | 曜日（0 = 日曜日）    |
| `DateTime::to_string` | `(dt: Int) -> String` | ISO 8601 形式の文字列 |

**しかし、これらの名前は現時点では YaoXiang のソースコードから呼び出すことができません（#338）。**
エクスポート名に `::` が含まれていますが、`::`
は構文上の予約記号であり、フィールドアクセスの位置には使用できません。実測により、以下のすべての書き方が失敗することが確認されています：

| 試行した書き方             | 結果                                                |
| -------------------------- | --------------------------------------------------- |
| `time.DateTime::year(0)`   | `E0010 expected RParen, found ColonColon`           |
| `time.DateTime.year(0)`    | `E1042 Field 'DateTime' not found in struct 'time'` |
| `time.year(0)`             | `E1042 Field 'year' not found in struct 'time'`     |
| `time.DateTime_year(0)`    | `E1042 Field 'DateTime_year' not found`             |
| `time."DateTime::year"(0)` | `E0011 Unexpected token: StringLiteral`             |

**代替手段**：日付の各要素が必要な場合は、[`format_time`](#format_time)
で必要に応じてフォーマットしてください。内部でタイムスタンプから年月日への分解が行われています：

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // format_time で各要素を取得
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")
    assert(time.format_time(0, "%d") == "01")
}
```

## 関連

- [`std.concurrent`](./concurrent) —— ミリ秒単位のスリープ
- [エラーコードリファレンス](../error-code/) —— `E6007` 一般的なランタイムエラー
