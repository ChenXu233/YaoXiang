---
title: '標準庫概観'
description: 'YaoXiang 標準庫モジュールの概観と使用規約'
---

# 標準庫リファレンス

YaoXiang 標準庫（`std`）はモジュール単位で組織され、各モジュールは `use`
でインポート後、`モジュール名.関数名(...)`
で呼び出します。本ディレクトリはモジュールごとに分割された API リファレンスドキュメントです。

## モジュール索引

<!-- stdlib:index:modules start -->

| モジュール                       | エクスポート数 | 説明                                                          |
| -------------------------------- | -------------- | ------------------------------------------------------------- |
| [`std.convert`](./convert)       | 11             | 任意値から String への変換                                    |
| [`std.dict`](./dict)             | 11             | 辞書の読み書き、キー値ビュー、マージ                          |
| [`std.io`](./io)                 | 7              | 標準出力、標準入力、ファイル全体の読み書き                    |
| [`std.math`](./math)             | 18             | 整数、浮動小数点数、三角関数、PI/E/TAU 定数を含む             |
| [`std.string`](./string)         | 19             | 文字列の検索、分割、フォーマット、解析                        |
| [`std.time`](./time)             | 14             | タイムスタンプ、フォーマット、DateTime フィールドへのアクセス |
| [`std.result`](./result)         | 9              | Result と Error の構築と分解                                  |
| [`std.range`](./range)           | 10             | 区間反復、述語、遅延アダプタ                                  |
| [`std.assert`](./assert)         | 1              | アサーション                                                  |
| [`std.net`](./net)               | 4              | HTTP リクエストと URL パーセントエンコード/デコード           |
| [`std.concurrent`](./concurrent) | 3              | スリープ、スケジューリングの譲渡、スレッド識別子              |
| [`std.os`](./os)                 | 22             | ファイルハンドル、ディレクトリ、環境変数、作業ディレクトリ    |
| [`std.weak`](./weak)             | 2              | Arc / Weak 弱参照                                             |

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

モジュールから名前でインポートすることも可能で、定数も対象にできます：

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14)
}
```

## 引数借用規約

シグネチャ中の `&` は**読み取り専用自動借用**（RFC-009
§2.8）を示します：呼び出し元が渡した変数はムーブされず、呼び出し後も引き続き使用可能です。これは標準庫の多くの読み取り専用関数のデフォルト形式です。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 3か所の呼び出しはすべて nums を読み取り専用借用し、後でも使用可能
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

`&`
のない引数は**値渡し**を示します。多くの「変更」関数はそのため、**ソース値を消費して新しい値を返す**関数型形式であり、インプレース変更ではありません：

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // 新しいリストを返す。base はムーブ済み
    assert(list.len(extended) == 3)
}
```

各モジュールページの「意味分類」セクションには、そのモジュールのどの関数が借用し、消耗し、インプレース変更するかが記載されています。特に注意すべき点が一つあります：

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) のシグネチャは `&List(A)`
  と記載されていますが、実際にはリストを**インプレース変更**します

## エラーモデル

標準庫には2種類の失敗形態があり、各関数のエントリにはそれぞれ「エラー」と「戻り値」として記載されます：

| 形態                     | 動作                                  | 典型的なシナリオ                                                     |
| ------------------------ | ------------------------------------- | -------------------------------------------------------------------- |
| ランタイムエラーをスロー | `E6xxx` コードで現在の実行を終了      | 辞書のキー欠如 `E6008`、インデックス範囲外 `E6003`、アサーション失敗 |
| センチネル値を返す       | 中断せず、`Void` / `-1` / `""` を返す | リストの境界外読み取り、空リストの先頭要素取得、環境変数の欠如       |

一般的なランタイムエラーコード：

| エラーコード | 意味                               | トリガー例                                   |
| ------------ | ---------------------------------- | -------------------------------------------- |
| `E6003`      | インデックス範囲外                 | `list.set(l, 99, v)`                         |
| `E6005`      | アサーション失敗                   | `assert(false)`                              |
| `E6007`      | 汎用ランタイムエラー               | ファイルが存在しない、`result.unwrap` の失敗 |
| `E6008`      | キー欠如                           | `dict.get(d, "nope")`                        |
| `E6010`      | 整数解析失敗（Err 値として）       | `string.parse_int("abc")`                    |
| `E6011`      | 浮動小数点解析失敗（Err 値として） | `string.parse_float("abc")`                  |

エラーコードの全表は[エラーコードリファレンス](../error-code/)を参照してください。

`string.parse_int` / `string.parse_float` は第3の形態に属します：**エラーをスローせず**、失敗を
`Result` の `Err` 値としてラップして返し、[`std.result`](./result) で分解するか `?` で伝播できます。

```yaoxiang
use std.assert
use std.result
use std.string

main: () -> Void = {
    assert(result.is_ok(string.parse_int("42")))
    assert(result.is_err(string.parse_int("abc")))
}
```

## 反復プロトコル

`std.list` と `std.range` は同じイテレータプロトコルを提供します。イテレータ自体は `Tuple`
状態キャリアです。

> **ムーブセマンティクス**：`next` と `has_next`
> はどちらもイテレータを**ムーブ**します（シグネチャに `&`
> がありません）。したがって、毎回アクセスするたびに再作成するか、`for ... in`
> を直接使用してください。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // has_next が it をムーブしたため、要素取得の前に再作成
    it2 = list.iter([1, 2, 3])
    assert(list.next(it2) == 1)
}
```

日常的な走査は `for ... in` を直接使用します：

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
`reduce` / `for_each` / `for ... in` で消費された後に結果が生成されます：

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

以下はオペレーティングシステムの機能に依存するもので、`wasm32`
ターゲットでは**エクスポートされません**：

| 範囲                                                            | 必要                  |
| --------------------------------------------------------------- | --------------------- |
| `std.os` 全体、`std.net` 全体、`std.weak` 全体                  | ファイル/ネットワーク |
| `std.concurrent` 全体                                           | スレッド              |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | 標準 I/O              |
| `std.time.sleep`                                                | スレッドスリープ      |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert` および `std.io.print` / `println` / `format_fallback`
はすべてのターゲットで利用可能です。

## 実装済みのギャップ

以下の問題はドキュメント執筆時に実例を一つずつ実測して確認済みで、すべて issue で追跡中です。

**修正済み（2026-09-19）**：#337 / #338 / #339 /
#340 の4項目はすべて修正済みで、対応するページ本文は通常の使用方法の説明に同期して書き換え済みです：

| 位置                                                    | 元の問題                                                                        | 修正                                     |
| ------------------------------------------------------- | ------------------------------------------------------------------------------- | ---------------------------------------- |
| [`os.open`](./os#open)                                  | ハンドルが使い切りで、`open`→`write`→`close` がコンパイル不可                   | ハンドルは参照渡しに変更 ✅              |
| [`time.datetime_*`](./time#datetime-フィールドアクセス) | 8つのアクセサがソースコードから呼び出せない（エクスポート名に `::` が含まれる） | フラット名 `datetime_year` などに変更 ✅ |
| [`time.parse_time`](./time#parse_time)                  | `fmt` 引数が無視され、戻り値が使用できない                                      | fmt に基づいてステップ単位で解析 ✅      |
| [`math.clamp`](./math#clamp)                            | `min > max` でインタプリタが panic を起こしエラーを返さない                     | `E6007` を返す ✅                        |

**未解決**：

| 位置                                           | 問題                                                       | 追跡 |
| ---------------------------------------------- | ---------------------------------------------------------- | ---- |
| [`net.http_get`](./net#http_get) / `http_post` | プレースホルダ実装で、リクエストを送信せず説明文字列を返す | #56  |

## ドキュメント保守

本ディレクトリは**生成 + 手書き**の混合構造です：

- **生成領域**（`<!-- stdlib:KEY start/end -->` マーカー間）：関数一覧表とシグネチャブロックは
  `StdModule::exports()` から派生します。シグネチャはバイト単位で `NativeExport::signature`
  から取得されるため、実装とドリフトすることは不可能です。
- **手書き領域**（マーカー外）：モジュールの概要、借用/ムーブセマンティクス、エラーモデル、既知のギャップ、サンプル。

ゲート（CI 内で `cargo test --lib` と共に実行）：

| ゲート           | テスト                                        | 役割                                                                       |
| ---------------- | --------------------------------------------- | -------------------------------------------------------------------------- |
| ドリフト検出     | `test_stdlib_docs_match_generation`           | 生成領域は `exports()` と一致する必要がある                                |
| 孤立検出         | `test_stdlib_docs_has_no_orphan_module_pages` | モジュールページはジェネレータ出力を超えてはならない                       |
| カバレッジ       | `test_stdlib_docs_covers_interface_modules`   | ドキュメントモジュールセットはインターフェースビューをカバーする必要がある |
| サンプル実行可能 | `test_stdlib_docs_examples_run`               | 各 ```yaoxiang サンプルは実際に実行可能でなければならない                  |

`exports()` 変更後、修復ツールで生成領域を書き換えます：

```bash
cargo run --example gen-stdlib-docs
```

これは
`gen-std-interfaces`（RFC-037 インターフェースビュー）、`tools/code-tables --fix`（RFC-013 コードテーブル）と同型です。

## 関連ドキュメント

- [標準庫仕様](../language-spec/stdlib.md) —— 言語レベルの標準庫設計規約
- [FFI 仕様](../language-spec/ffi.md) —— ユーザー側 `native` 拡張と C ABI バインディング
- [エラーコードリファレンス](../error-code/) —— `E6xxx` ランタイムエラーコード全表
