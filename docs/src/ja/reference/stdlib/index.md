---
title: '標準ライブラリ総覧'
description: 'YaoXiang 標準ライブラリモジュールの総覧と使用規約'
---

# 標準ライブラリリファレンス

YaoXiang 標準ライブラリ（`std`）はモジュール単位で組織され、各モジュールは `use` でインポート後、
`モジュール名.関数名(...)`
で呼び出されます。本ディレクトリはモジュールごとに分割された API リファレンスです。

## モジュール索引

<!-- stdlib:index:modules start -->

| モジュール                       | エクスポート数 | 説明                                                             |
| -------------------------------- | -------------- | ---------------------------------------------------------------- |
| [`std.convert`](./convert)       | 11             | 任意値から String への変換                                       |
| [`std.dict`](./dict)             | 11             | 辞書の読み書き、キー値ビューとマージ                             |
| [`std.io`](./io)                 | 7              | 標準出力、標準入力、ファイル全体の読み書き                       |
| [`std.math`](./math)             | 18             | 整数・浮動小数点・三角関数。PI/E/TAU 定数を含む                  |
| [`std.string`](./string)         | 19             | 文字列の検索・分割・フォーマット・解析                           |
| [`std.time`](./time)             | 14             | タイムスタンプ、フォーマット、DateTime フィールドアクセス        |
| [`std.result`](./result)         | 7              | Result と Error の構築と分解                                     |
| [`std.range`](./range)           | 10             | 区間イテレーション、述語、遅延アダプタ                           |
| [`std.assert`](./assert)         | 1              | 断言                                                             |
| [`std.net`](./net)               | 4              | HTTP リクエストと URL パーセントエンコーディング／デコーディング |
| [`std.concurrent`](./concurrent) | 3              | スリープ、スケジューリング譲歩、スレッド識別子                   |
| [`std.os`](./os)                 | 22             | ファイルハンドル、ディレクトリ、環境変数、作業ディレクトリ       |
| [`std.weak`](./weak)             | 2              | Arc / Weak 弱参照                                                |

<!-- stdlib:index:modules end -->

## インポート規約

モジュールの全体インポート：

```yaoxiang
use std.list
use std.string

main: () -> Void = {
    parts = string.split("a,b,c", ",")
    println(list.len(parts))
}
```

モジュールから名前を指定してインポートすることもできます（定数も可）：

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14)
}
```

## 引数借用規約

シグネチャ中の `&` は**読み取り専用の自動借用**を表します（RFC-009
§2.8）：呼び出し側から渡された変数はムーブされず、呼び出し後も引き続き使用できます。これは標準ライブラリにおける多くの読み取り専用関数のデフォルトの形式です。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 三か所すべて nums を読み取り専用借用しており、以降も利用可能
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

`&`
を伴わない引数は**値渡し**を意味します。多くの「変更」関数はこのため、**ソース値を消費して新しい値を返す**関数型の形式となり、インプレースでの書き換えではありません。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // 新しいリストを返す。base はムーブ済み
    assert(list.len(extended) == 3)
}
```

各モジュールページの「意味分類」セクションには、そのモジュール内の関数のうちどれが借用で、どれが消費で、どれがインプレース変更であるかが列挙されています。一つ特に注意すべき点があります。

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) はシグネチャ上は `&List(A)`
  ですが、リストを**インプレースで変更**します。

## エラーモデル

標準ライブラリには二種類の失敗形態があり、各関数エントリではそれぞれ「エラー」と「戻り値 …」として記載されます。

| 形態                 | 表現                                  | 典型的なシナリオ                                             |
| -------------------- | ------------------------------------- | ------------------------------------------------------------ |
| ランタイムエラー送出 | `E6xxx` コードで現在の実行を終了      | 辞書のキー欠如 `E6008`、インデックス範囲外 `E6003`、断言失敗 |
| センチネル値返却     | 中断せず、`Void` / `-1` / `""` を返す | リストの範囲外読み取り、空リストの先頭要素取得、環境変数欠如 |

主なランタイムエラーコード：

| エラーコード | 意味                               | トリガ例                             |
| ------------ | ---------------------------------- | ------------------------------------ |
| `E6003`      | インデックス範囲外                 | `list.set(l, 99, v)`                 |
| `E6005`      | 断言失敗                           | `assert(false)`                      |
| `E6007`      | 汎用ランタイムエラー               | ファイル不存在、`result.unwrap` 失敗 |
| `E6008`      | キー欠落                           | `dict.get(d, "nope")`                |
| `E6010`      | 整数解析失敗（Err 値として）       | `string.parse_int("abc")`            |
| `E6011`      | 浮動小数点解析失敗（Err 値として） | `string.parse_float("abc")`          |

エラーコードの全表は [エラーコードリファレンス](../error-code/) を参照してください。

`string.parse_int` / `string.parse_float` は第三の形態に属します。**エラーを送出せず**、失敗を
`Result` の `Err` 値としてラップして返すため、[`std.result`](./result) で分解するか `?`
で伝播させることができます。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_int("42")))
    assert(result.is_err(string.parse_int("abc")))
}
```

## イテレーションプロトコル

`std.list` と `std.range` は同じイテレータプロトコルを提供します。イテレータ自体は `Tuple`
による状態キャリアです。

> **ムーブセマンティクス**：`next` と `has_next`
> はどちらもイテレータを**ムーブ**します（シグネチャに `&`
> がありません）。したがって、使用するたびに再作成するか、 `for ... in` を直接利用してください。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // has_next で it がムーブされるため、再作成してから要素を取り出す
    it2 = list.iter([1, 2, 3])
    assert(list.next(it2) == 1)
}
```

日常的な走査には `for ... in` を直接使います。

```yaoxiang
use std.assert

main: () -> Void = {
    mut sum = 0
    for x in [1, 2, 3] {
        sum = sum + x
    }
    assert(sum == 6)
}
```

[`range.map`](./range#map) / [`range.filter`](./range#filter) は**遅延**アダプタを返すため、
`collect` / `reduce` / `for_each` / `for ... in` で消費して初めて結果が得られます。

```yaoxiang
use std.assert
use std.list
use std.range
use std.result

main: () -> Void = {
    doubled = range.collect(range.map(result.unwrap(range.iter(1..4)), x => x * 2))
    assert(list.get(doubled, 0) == 2)
    assert(list.len(doubled) == 3)
}
```

## プラットフォーム可用性

以下は OS の機能に依存しており、`wasm32` ターゲットでは**エクスポートされません**。

| 範囲                                                            | 必要機能               |
| --------------------------------------------------------------- | ---------------------- |
| `std.os` 全体、`std.net` 全体、`std.weak` 全体                  | ファイル／ネットワーク |
| `std.concurrent` 全体                                           | スレッド               |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | 標準 I/O               |
| `std.time.sleep`                                                | スレッドスリープ       |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert`、および `std.io.print` / `println` / `format_fallback`
はすべてのターゲットで利用可能です。

## 実装済みのギャップ

以下の問題はドキュメント執筆時に実機でサンプルを走らせて一つずつ確認しており、すべて issue で追跡中です。

**修正済み（2026-09-19）**：#337 / #338 / #339 /
#340 の 4 件はすべて修正済みで、対応するページ本文も通常の使い方の説明に同期して書き直されています。

| 位置                                          | 元の問題                                                               | 修正                                       |
| --------------------------------------------- | ---------------------------------------------------------------------- | ------------------------------------------ |
| [`os.open`](./os#open)                        | ハンドルが使い切りで `open`→`write`→`close` がコンパイル不可           | ハンドルを参照渡しに変更 ✅                |
| [`time.datetime_*`](./time#datetime-字段访问) | 8 つのアクセサがソースから呼び出せない（エクスポート名に `::` を含む） | フラットな名前 `datetime_year` 等に変更 ✅ |
| [`time.parse_time`](./time#parse_time)        | `fmt` 引数が無視され、戻り値が使い続けられない                         | fmt に従って段階的に解析 ✅                |
| [`math.clamp`](./math#clamp)                  | `min > max` でインタプリタがパニックしエラーを返さない                 | `E6007` を返すように修正 ✅                |

**未解決**：

| 位置                                           | 問題                                                   | 追跡 |
| ---------------------------------------------- | ------------------------------------------------------ | ---- |
| [`net.http_get`](./net#http_get) / `http_post` | プレースホルダ実装でリクエストを送らず説明文字列を返す | #56  |

## ドキュメント保守

本ディレクトリは**生成＋手書き**の混在構造です。

- **生成領域**（`<!-- stdlib:KEY start/end -->` マーカー間）：関数一覧表とシグネチャブロックは
  `StdModule::exports()` から派生します。シグネチャは `NativeExport::signature`
  からバイト単位で得ているため、実装と乖離することはできません。
- **手書き領域**（マーカー外）：モジュールの概要、借用／ムーブセマンティクス、エラーモデル、既知のギャップ、サンプル。

ゲート（CI で `cargo test --lib` と一緒に実行されます）：

| ゲート           | テスト                                        | 役割                                                                 |
| ---------------- | --------------------------------------------- | -------------------------------------------------------------------- |
| ドリフト検出     | `test_stdlib_docs_match_generation`           | 生成領域が `exports()` と一致すること                                |
| 孤立検出         | `test_stdlib_docs_has_no_orphan_module_pages` | モジュールページがジェネレータの出力より多くないこと                 |
| カバレッジ       | `test_stdlib_docs_covers_interface_modules`   | ドキュメントのモジュール集合がインターフェースビューをカバーすること |
| サンプル実行可否 | `test_stdlib_docs_examples_run`               | 各 ```yaoxiang サンプルが実際に動作すること                          |

`exports()` を変更した後は、治療ツールで生成領域を書き直してください。

```bash
cargo run --example gen-stdlib-docs
```

`gen-std-interfaces`（RFC-037 インターフェースビュー）や
`tools/code-tables --fix`（RFC-013 コードテーブル）と同形です。

## 関連ドキュメント

- [標準ライブラリ仕様](../language-spec/stdlib.md) —— 言語レベルでの標準ライブラリの設計規約
- [FFI 仕様](../language-spec/ffi.md) —— ユーザーサイドの `native` 拡張と C ABI バインディング
- [エラーコードリファレンス](../error-code/) —— `E6xxx` ランタイムエラーコードの全表
