---
title: 'std.time'
description: 'タイムスタンプ、フォーマット、DateTimeフィールドアクセス'
---

# std.time

時間モジュール。

```yaoxiang
use std.time
```

> **本モジュールには実装上のギャップがいくつかあります（#338 /
> #340）**。本ドキュメント執筆時に実測で確認済みです。利用可能な面と利用不可能な面を以下でそれぞれ示し、署名通りに記述しただけではコンパイルできない例を書かないようにします。
>
> 通常使用可能：[`now`](#now) / [`timestamp`](#timestamp) / [`timestamp_ms`](#timestamp_ms) /
> [`sleep`](#sleep) / [`format_time`](#format_time)（`Int`タイムスタンプリテラルを受け付けます）。
>
> 現在利用不可：[`parse_time`](#parse_time) の戻り値、およびすべての
> [`DateTime::*`](#datetime-フィールドアクセス不可用) アクセサ。

## 関数一覧

<!-- stdlib:table:time start -->

| 関数                  | シグネチャ                             |
| --------------------- | -------------------------------------- |
| `now`                 | `() -> DateTime`                       |
| `timestamp`           | `() -> Int`                            |
| `timestamp_ms`        | `() -> Int`                            |
| `sleep`               | `(seconds: Float) -> Void`             |
| `format_time`         | `(dt: Int, fmt: String) -> String`     |
| `parse_time`          | `(fmt: String, s: String) -> DateTime` |
| `DateTime::year`      | `(dt: Int) -> Int`                     |
| `DateTime::month`     | `(dt: Int) -> Int`                     |
| `DateTime::day`       | `(dt: Int) -> Int`                     |
| `DateTime::hour`      | `(dt: Int) -> Int`                     |
| `DateTime::minute`    | `(dt: Int) -> Int`                     |
| `DateTime::second`    | `(dt: Int) -> Int`                     |
| `DateTime::weekday`   | `(dt: Int) -> Int`                     |
| `DateTime::to_string` | `(dt: Int) -> String`                  |

<!-- stdlib:table:time end -->

## 時間の取得

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> DateTime
```

<!-- stdlib:sig:time.now end -->

現在の時間を返します。

戻り値：`DateTime` 値で、出力形式は `DateTime(1789471990)` のようになります。**`Int`
ではない**ため、算術演算や比較に直接参加できず、`Int`
パラメータとして他の関数に渡すこともできません（[`format_time`](#format_time) を参照）。

```yaoxiang
use std.time

main: () -> Void = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> 計算に参加させる必要がある場合は [`timestamp`](#timestamp) または [`timestamp_ms`](#timestamp_ms)
> を使用してください。これらは直接 `Int` を返します。

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

現在の Unix タイムスタンプ（**秒**）を返します。算術演算や比較に直接参加できます。

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
は**ミリ秒**単位ですので、混同しないでください。

- `seconds` —— スリープ秒数；`Int`（秒として解釈）または `Float` を受け付けます

エラー：パラメータが `Int` でも `Float` でもない場合、`E6007` がスローされます。

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
スタイルのプレースホルダーをサポートします。

- `dt` —— Unix タイムスタンプ（**秒**）、`Int` である必要があります
- `fmt` —— フォーマット文字列

> **型注意**：`dt` は `Int` である必要があります。[`now`](#now) や [`parse_time`](#parse_time)
> の戻り値を渡すと、`E1002`（`expected type 'int64', found type 'DateTime'`）が報告されます。これらはすべて
> `DateTime` であるためです。現在、`DateTime` → `Int` の変換方法がないため、**実際には `Int`
> リテラルまたは [`timestamp`](#timestamp) の結果のみを渡すことができます**。

サポートされるプレースホルダー：

| プレースホルダー | 意味                  | 例           |
| ---------------- | --------------------- | ------------ |
| `%Y`             | 4桁の年               | `2024`       |
| `%m`             | 2桁の月               | `01`         |
| `%d`             | 2桁の日               | `15`         |
| `%H`             | 2桁の時間（24時間制） | `10`         |
| `%M`             | 2桁の分               | `30`         |
| `%S`             | 2桁の秒               | `00`         |
| `%w`             | 曜日（0 = 日曜日）    | `1`          |
| `%F`             | `%Y-%m-%d` と等価     | `2024-01-15` |
| `%T`             | `%H:%M:%S` と等価     | `10:30:00`   |

**ローカル時間**で分解します。認識できないプレースホルダーはそのまま保持されます。

エラー：`dt` が `Int` でない、`fmt` が `String` でない、またはパラメータが不足している場合、`E6007`
がスローされます。

```yaoxiang
use std.assert
use std.string
use std.time

main: () -> Void = {
    // 0 = Unixエポック
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")

    // 現在のタイムスタンプ（Int）も直接使用できます
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

時間文字列を解析して時間に変換します。

- `fmt` —— **現在無視されます**（下記参照）
- `s` —— 解析対象の文字列

戻り値：`DateTime` 値。

> **2つの実装上の制限（#340）**：
>
> 1. `fmt` パラメータは**解析に参加しません**。関数は ISO 8601 形式の `YYYY-MM-DDTHH:MM:SS` または
>    `YYYY-MM-DD HH:MM:SS`（日付と時刻の間は `T` またはスペース区切り）のみを認識し、`fmt`
>    の内容に関わらず他の形式を渡すとすべて失敗します。
> 2. 返される `DateTime` は**現在そのまま使用できません**——`Int` ではないため（`format_time` /
>    `DateTime::*` に渡せない）、呼び出し可能なアクセサもありません（次節参照）。

エラー：フォーマットが一致しない場合、`E6007` がスローされます。

```yaoxiang
use std.time

main: () -> Void = {
    // 解析自体は成功します
    ts = time.parse_time("", "2024-01-15 10:30:00")
    println(ts)
}
```

## DateTime フィールドアクセス（利用不可）

> 追跡：#338

`std.time` は `DateTime::` という名前の 8 つのアクセサをエクスポートしています：

| エクスポート名        | シグネチャ            | 説明                |
| --------------------- | --------------------- | ------------------- |
| `DateTime::year`      | `(dt: Int) -> Int`    | 4桁の年             |
| `DateTime::month`     | `(dt: Int) -> Int`    | 月（1〜12）         |
| `DateTime::day`       | `(dt: Int) -> Int`    | 日（1〜31）         |
| `DateTime::hour`      | `(dt: Int) -> Int`    | 時（0〜23）         |
| `DateTime::minute`    | `(dt: Int) -> Int`    | 分（0〜59）         |
| `DateTime::second`    | `(dt: Int) -> Int`    | 秒（0〜59）         |
| `DateTime::weekday`   | `(dt: Int) -> Int`    | 曜日（0 = 日曜日）  |
| `DateTime::to_string` | `(dt: Int) -> String` | ISO 8601 形式文字列 |

**ただし、これらの名前は現在 YaoXiang ソースコードから呼び出すことができません（#338）。**
エクスポート名に `::` が含まれていますが、`::`
は構文の予約記号であり、フィールドアクセス位置に現れません。以下の記述はすべて失敗することが実測で確認されています：

| 試行した記述               | 結果                                                |
| -------------------------- | --------------------------------------------------- |
| `time.DateTime::year(0)`   | `E0010 expected RParen, found ColonColon`           |
| `time.DateTime.year(0)`    | `E1042 Field 'DateTime' not found in struct 'time'` |
| `time.year(0)`             | `E1042 Field 'year' not found in struct 'time'`     |
| `time.DateTime_year(0)`    | `E1042 Field 'DateTime_year' not found`             |
| `time."DateTime::year"(0)` | `E0011 Unexpected token: StringLiteral`             |

**代替手段**：日付の各要素が必要な場合は、[`format_time`](#format_time)
を使用して必要に応じてフォーマットしてください。内部でタイムスタンプ → 年月日の分解がすでに完了しています：

```yaoxiang
use std.assert
use std.time

main: () -> Void = {
    // format_timeで各要素を取得
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")
    assert(time.format_time(0, "%d") == "01")
}
```

## 関連

- [`std.concurrent`](./concurrent) —— ミリ秒単位のスリープ
- [エラーコードリファレンス](../error-code/) —— `E6007` 一般的なランタイムエラー
