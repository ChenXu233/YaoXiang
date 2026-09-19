---
title: '標準ライブラリ概要'
description: 'YaoXiang 標準ライブラリモジュール概要と利用規約'
---

# 標準ライブラリリファレンス

YaoXiang 標準ライブラリ（`std`）はモジュール単位で組織され、各モジュールは `use` でインポートした後
`モジュール名.関数名(...)`
で呼び出します。本ディレクトリはモジュール別に分割された API リファレンスドキュメントです。

## モジュール索引

<!-- stdlib:index:modules start -->

| モジュール                       | エクスポート数 | 説明                                                         |
| -------------------------------- | -------------- | ------------------------------------------------------------ |
| [`std.convert`](./convert)       | 11             | 任意の値から String への変換                                 |
| [`std.dict`](./dict)             | 11             | 辞書の読み書き、キー・値のビューとマージ                     |
| [`std.io`](./io)                 | 7              | 標準出力、標準入力とファイル全体の読み書き                   |
| [`std.list`](./list)             | 22             | リストの追加・削除、スライス、高階関数とイテレータプロトコル |
| [`std.math`](./math)             | 18             | 整数、浮動小数点と三角関数、PI/E/TAU 定数を含む              |
| [`std.string`](./string)         | 19             | 文字列検索、分割、フォーマットと解析                         |
| [`std.time`](./time)             | 14             | タイムスタンプ、フォーマットと DateTime フィールドアクセス   |
| [`std.result`](./result)         | 9              | Result と Error の構築と分解                                 |
| [`std.range`](./range)           | 10             | 区間イテレーション、述語と遅延アダプタ                       |
| [`std.assert`](./assert)         | 1              | アサーション                                                 |
| [`std.net`](./net)               | 4              | HTTP リクエストと URL パーセントエンコード・デコード         |
| [`std.concurrent`](./concurrent) | 3              | スリープ、譲歩スケジューリングとスレッド識別子               |
| [`std.os`](./os)                 | 22             | ファイルハンドル、ディレクトリ、環境変数と作業ディレクトリ   |
| [`std.weak`](./weak)             | 2              | Arc / Weak 弱参照                                            |

<!-- stdlib:index:modules end -->

## インポート規約

モジュール全体のインポート：

```yaoxiang
use std.list
use std.string

main: () -> Void = {
    parts = string.split("a,b,c", ",")
    println(list.len(parts))
}
```

モジュールから名前指定でインポートすることも可能です。定数も含みます：

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14)
}
```

## パラメータ借用規約

シグネチャ中の `&` は**読み取り専用自動借用**（RFC-009
§2.8）を表します：呼び出し側が渡した変数はムーブされず、呼び出し後も引き続き使用可能です。これは標準ライブラリの多くの読み取り専用関数のデフォルト形式です。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 3 つの呼び出しはいずれも nums を読み取り専用借用し、その後も使用可能
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

`&`
のないパラメータは**値渡し**を意味します。多くの「変更」関数はそのため、**ソース値を消費して新しい値を返す**関数型形式であり、インプレース変更ではありません：

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // 新しいリストを返す；base はムーブ済み
    assert(list.len(extended) == 3)
}
```

各モジュールページの「セマンティクス分類」セクションには、そのモジュール内の関数のうちどれが借用し、どれが消費し、どれがインプレース変更を行うかが列挙されています。一つ特に注意すべき点があります：

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) のシグネチャは `&List(A)`
  と表記されていますが、リストを**インプレース変更**します

## エラーモデル

標準ライブラリには 2 種類の失敗形態があり、各関数項目ではそれぞれ「エラー」と「戻り値 ...」として注記されます：

| 形態                   | 挙動                                  | 典型的なシナリオ                                                     |
| ---------------------- | ------------------------------------- | -------------------------------------------------------------------- |
| ランタイムエラーの送出 | `E6xxx` コードで現在の実行を終了      | 辞書のキー欠如 `E6008`、インデックス範囲外 `E6003`、アサーション失敗 |
| センチネル値の返却     | 中断せず、`Void` / `-1` / `""` を返す | リストの境界外読み取り、空リストの先頭要素取得、環境変数の欠落       |

一般的なランタイムエラーコード：

| エラーコード | 意味                               | 発生例                                     |
| ------------ | ---------------------------------- | ------------------------------------------ |
| `E6003`      | インデックス範囲外                 | `list.set(l, 99, v)`                       |
| `E6005`      | アサーション失敗                   | `assert(false)`                            |
| `E6007`      | 汎用ランタイムエラー               | ファイルが存在しない、`result.unwrap` 失敗 |
| `E6008`      | キー欠落                           | `dict.get(d, "nope")`                      |
| `E6010`      | 整数解析失敗（Err 値として）       | `string.parse_int("abc")`                  |
| `E6011`      | 浮動小数点解析失敗（Err 値として） | `string.parse_float("abc")`                |

エラーコードの全表は[エラーコードリファレンス](../error-code/)を参照してください。

`string.parse_int` / `string.parse_float` は第 3 の形態に属します：**エラーを送出せず**、失敗を
`Result` の `Err` 値にラップして返します。[`std.result`](./result) で分解するか `?`
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

`std.list` と `std.range` は同一のイテレータプロトコルを提供します。イテレータ自体は `Tuple`
状態キャリアです。

> **ムーブセマンティクス**：`next` と `has_next`
> はいずれもイテレータを**ムーブ**します（シグネチャに `&`
> がない）。そのため、毎回取得時に再作成するか、直接 `for ... in` を使用してください。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // has_next が it をムーブしたため、要素取得前に再作成
    it2 = list.iter([1, 2, 3])
    assert(list.next(it2) == 1)
}
```

日常的な走査は直接 `for ... in` を使用します：

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

[`range.map`](./range#map) / [`range.filter`](./range#filter) は**遅延**アダプタを返し、`collect` /
`reduce` / `for_each` / `for ... in` で消費された後に結果を生成します：

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

以下はオペレーティングシステムの機能に依存し、`wasm32` ターゲットでは**エクスポートされません**：

| 範囲                                                            | 必要事項              |
| --------------------------------------------------------------- | --------------------- |
| `std.os` 全体、`std.net` 全体、`std.weak` 全体                  | ファイル/ネットワーク |
| `std.concurrent` 全体                                           | スレッド              |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | 標準 I/O              |
| `std.time.sleep`                                                | スレッドスリープ      |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert` および `std.io.print` / `println` / `format_fallback`
はすべてのターゲットで利用可能です。

## 実装済みのギャップ

以下の問題はドキュメント執筆時に個別に実例を実行して確認済みで、すべて issue で追跡されています。

**修正済み（2026-09-19）**：#337 / #338 / #339 /
#340 の 4 件はすべて修正され、対応するページ本文は正常な使用方法の説明に同期更新されました：

| 位置                                                    | 元の問題                                                                     | 修正                                     |
| ------------------------------------------------------- | ---------------------------------------------------------------------------- | ---------------------------------------- |
| [`os.open`](./os#open)                                  | ハンドルが使い捨てで、`open`→`write`→`close` がコンパイル不可                | ハンドルを参照渡しに変更 ✅              |
| [`time.datetime_*`](./time#datetime-フィールドアクセス) | 8 つのアクセサがソースコードから呼び出せない（エクスポート名に `::` を含む） | フラット名 `datetime_year` などに変更 ✅ |
| [`time.parse_time`](./time#parse_time)                  | `fmt` パラメータが無視される；戻り値が使用不能                               | fmt に従ってステップ解析 ✅              |
| [`math.clamp`](./math#clamp)                            | `min > max` でインタプリタがパニックし、エラーを返さない                     | `E6007` を返すように修正 ✅              |

**未解決**：

| 位置                                           | 問題                                                       | 追跡 |
| ---------------------------------------------- | ---------------------------------------------------------- | ---- |
| [`net.http_get`](./net#http_get) / `http_post` | プレースホルダ実装で、リクエストを送信せず説明文字列を返す | #56  |

## ドキュメント保守

本ディレクトリは**生成 + 手書き**のハイブリッド構造です：

- **生成領域**（`<!-- stdlib:KEY start/end -->` マーカーの間）：関数一覧表とシグネチャブロックは
  `StdModule::exports()` から派生します。シグネチャはバイト単位で `NativeExport::signature`
  から取得されるため、実装とドリフトすることは不可能です。
- **手書き領域**（マーカー外）：モジュール概要、借用/ムーブセマンティクス、エラーモデル、既知のギャップとサンプル。

ゲート（CI で `cargo test --lib` とともに実行）：

| ゲート             | テスト                                        | 役割                                                               |
| ------------------ | --------------------------------------------- | ------------------------------------------------------------------ |
| ドリフト検出       | `test_stdlib_docs_match_generation`           | 生成領域が `exports()` と一致していること                          |
| 孤児検出           | `test_stdlib_docs_has_no_orphan_module_pages` | モジュールページがジェネレータ出力を超えないこと                   |
| カバレッジ         | `test_stdlib_docs_covers_interface_modules`   | ドキュメントモジュール集合がインターフェースビューをカバーすること |
| サンプル実行可能性 | `test_stdlib_docs_examples_run`               | 各 ```yaoxiang サンプルが実際に実行可能であること                  |

`exports()` 変更後は、修復ツールで生成領域を再生成します：

```bash
cargo run --example gen-stdlib-docs
```

これは
`gen-std-interfaces`（RFC-037 インターフェースビュー）、`tools/code-tables --fix`（RFC-013 コード表）と同型です。

## 関連ドキュメント

- [標準ライブラリ仕様](../language-spec/stdlib.md) —— 言語レベルの標準ライブラリ設計規約
- [FFI 仕様](../language-spec/ffi.md) —— ユーザー側 `native` 拡張と C ABI バインディング
- [エラーコードリファレンス](../error-code/) —— `E6xxx` ランタイムエラーコード全表
