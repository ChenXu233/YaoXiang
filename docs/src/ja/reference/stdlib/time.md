---
title: 'std.time'
description: 'タイムスタンプ、フォーマット、DateTimeフィールドアクセス'
---

# std.time

時間モジュール。

```yaoxiang
use std.time
```

> **本モジュールには実装ギャップがいくつかあります（#338 /
> #340）**。ドキュメント執筆時に実機で確認済みです。利用可能な部分と利用できない部分を以下で個別に示し、署名通りにコンパイルできない例を書かないようにしてください。
>
> 正常に利用可能：[`now`](#now) / [`timestamp`](#timestamp) / [`timestamp_ms`](#timestamp_ms) /
> [`sleep`](#sleep) / [`format_time`](#format_time)（`Int`タイムスタンプリテラルを受け取る）。
>
> 現在は利用不可：[`parse_time`](#parse_time) の戻り値、およびすべての
> [`DateTime::*`](#datetimeフィールドアクセス利用不可) アクセサ。

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

## 時刻取得

### now

<!-- stdlib:sig:time.now start -->

```yaoxiang
now: () -> DateTime
```

<!-- stdlib:sig:time.now end -->

現在時刻を返します。

戻り値：`DateTime` 値で、表示は `DateTime(1789471990)` の形式です。**これは `Int`
ではない**ため、算術演算や比較に直接使用することはできず、`Int`
仮引数として他の関数に渡すこともできません（[`format_time`](#format_time) を参照）。

```yaoxiang
use std.time

main = {
    t = time.now()
    println(t)          // DateTime(1789471990)
}
```

> 計算に使用する場合は [`timestamp`](#timestamp) または [`timestamp_ms`](#timestamp_ms)
> を使用してください。これらは `Int` を直接返します。

### timestamp

<!-- stdlib:sig:time.timestamp start -->

```yaoxiang
timestamp: () -> Int
```

<!-- stdlib:sig:time.timestamp end -->

現在のUnixタイムスタンプ（**秒**）を返します。算術演算や比較に直接使用できます。

```yaoxiang
use std.assert
use std.time

main = {
    assert(time.timestamp() > 0)
}
```

### timestamp_ms

<!-- stdlib:sig:time.timestamp_ms start -->

```yaoxiang
timestamp_ms: () -> Int
```

<!-- stdlib:sig:time.timestamp_ms end -->

現在のUnixタイムスタンプ（**ミリ秒**）を返します。

```yaoxiang
use std.assert
use std.time

main = {
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
は**ミリ秒**単位であることに注意してください。

- `seconds` —— スリープ秒数。`Int`（秒として解釈）または `Float` を受け付けます

エラー：引数が `Int` でも `Float` でもない場合、`E6007` をスローします。

```yaoxiang
use std.time

main = {
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
スタイルのプレースホルダをサポートします。

- `dt` —— Unixタイムスタンプ（**秒**）、`Int` 必須
- `fmt` —— フォーマット文字列

> **型に関する注意**：`dt` は `Int` 必須です。[`now`](#now) や [`parse_time`](#parse_time)
> の戻り値を渡すと
> `E1002`（`expected type 'int64', found type 'DateTime'`）が発生します。これらはどちらも `DateTime`
> であるためです。現在 `DateTime` → `Int` の変換手段がないため、**実際には `Int` リテラルまたは
> [`timestamp`](#timestamp) の結果のみを渡すことができます**。

サポートされるプレースホルダ：

| プレースホルダ | 意味                | 例           |
| -------------- | ------------------- | ------------ |
| `%Y`           | 4桁の年             | `2024`       |
| `%m`           | 2桁の月             | `01`         |
| `%d`           | 2桁の日             | `15`         |
| `%H`           | 2桁の時（24時間制） | `10`         |
| `%M`           | 2桁の分             | `30`         |
| `%S`           | 2桁の秒             | `00`         |
| `%w`           | 曜日（0 = 日曜日）  | `1`          |
| `%F`           | `%Y-%m-%d` と等価   | `2024-01-15` |
| `%T`           | `%H:%M:%S` と等価   | `10:30:00`   |

**ローカル時間**で分解されます。認識できないプレースホルダはそのまま保持されます。

エラー：`dt` が `Int` でない、`fmt` が `String` でない、または引数が不足している場合、`E6007`
をスローします。

```yaoxiang
use std.assert
use std.string
use std.time

main = {
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

時刻文字列を解析して時刻を取得します。

- `fmt` —— **現在無視されます**（下記参照）
- `s` —— 解析対象の文字列

戻り値：`DateTime` 値。

> **2つの実装上の制限（#340）**：
>
> 1. `fmt` 引数は**解析に参加しません**。関数は ISO 8601 形式の `YYYY-MM-DDTHH:MM:SS` または
>    `YYYY-MM-DD HH:MM:SS`（日付と時刻の間は `T` またはスペース区切り）のみを認識し、他の形式は
>    `fmt` の内容に関わらず失敗します。
> 2. 返される `DateTime` は**現在使用することができません**。`Int` ではないため（`format_time` /
>    `DateTime::*` に渡せない）、呼び出し可能なアクセサもありません（次節参照）。

エラー：形式が一致しない場合、`E6007` をスローします。

```yaoxiang
use std.time

main = {
    // 解析自体は成功する
    ts = time.parse_time("", "2024-01-15 10:30:00")
    println(ts)
}
```

## DateTimeフィールドアクセス（利用不可）

> 追跡：#338

`std.time` は `DateTime::` で始まる 8 つのアクセサをエクスポートしています：

| エクスポート名        | シグネチャ            | 説明                  |
| --------------------- | --------------------- | --------------------- |
| `DateTime::year`      | `(dt: Int) -> Int`    | 4桁の年               |
| `DateTime::month`     | `(dt: Int) -> Int`    | 月（1–12）            |
| `DateTime::day`       | `(dt: Int) -> Int`    | 日（1–31）            |
| `DateTime::hour`      | `(dt: Int) -> Int`    | 時（0–23）            |
| `DateTime::minute`    | `(dt: Int) -> Int`    | 分（0–59）            |
| `DateTime::second`    | `(dt: Int) -> Int`    | 秒（0–59）            |
| `DateTime::weekday`   | `(dt: Int) -> Int`    | 曜日（0 = 日曜日）    |
| `DateTime::to_string` | `(dt: Int) -> String` | ISO 8601 形式の文字列 |

**しかし、これらの名前は現時点ではYaoXiangソースコードから呼び出すことができません（#338）。**
エクスポート名に `::` が含まれていますが、`::`
は構文における予約トークンであり、フィールドアクセス位置に現れません。実機での確認により、以下のすべての試行が失敗することを確認済みです：

| 試行した書き方             | 結果                                                |
| -------------------------- | --------------------------------------------------- |
| `time.DateTime::year(0)`   | `E0010 expected RParen, found ColonColon`           |
| `time.DateTime.year(0)`    | `E1042 Field 'DateTime' not found in struct 'time'` |
| `time.year(0)`             | `E1042 Field 'year' not found in struct 'time'`     |
| `time.DateTime_year(0)`    | `E1042 Field 'DateTime_year' not found`             |
| `time."DateTime::year"(0)` | `E0011 Unexpected token: StringLiteral`             |

**代替手段**：日付の構成要素が必要な場合は、[`format_time`](#format_time)
を使用して必要に応じてフォーマットしてください。タイムスタンプ → 年月日への分解は内部で完了しています：

```yaoxiang
use std.assert
use std.time

main = {
    // format_time で各構成要素を取得
    assert(time.format_time(0, "%Y") == "1970")
    assert(time.format_time(0, "%m") == "01")
    assert(time.format_time(0, "%d") == "01")
}
```

## 関連

- [`std.concurrent`](./concurrent) —— ミリ秒単位のスリープ
- [エラーコードリファレンス](../error-code/) —— `E6007` 汎用ランタイムエラー
