---
title: '標準庫概観'
description: 'YaoXiang 標準庫モジュールの概観と使用規約'
---

# 標準庫リファレンス

YaoXiang 標準庫（`std`）はモジュール単位で組織され、各モジュールは `use` でインポートした後
`モジュール名.関数名(...)`
の形式で呼び出します。本ディレクトリはモジュール別に分割された API リファレンスです。

## モジュール索引

<!-- stdlib:index:modules start -->

| モジュール                       | エクスポート数 | 説明                                                         |
| -------------------------------- | -------------- | ------------------------------------------------------------ |
| [`std.convert`](./convert)       | 11             | 任意値から String への変換                                   |
| [`std.dict`](./dict)             | 10             | 辞書の読み書き、キー・値のビュー、マージ                     |
| [`std.io`](./io)                 | 7              | 標準出力、標準入力、ファイル全体の読み書き                   |
| [`std.list`](./list)             | 22             | リストの追加・削除、スライス、高階関数、イテレータプロトコル |
| [`std.math`](./math)             | 18             | 整数・浮動小数点・三角関数、PI/E/TAU 定数を含む              |
| [`std.string`](./string)         | 19             | 文字列の検索・分割・フォーマット・解析                       |
| [`std.time`](./time)             | 14             | タイムスタンプ、フォーマット、DateTime フィールドアクセス    |
| [`std.result`](./result)         | 9              | Result と Error の構築と分解                                 |
| [`std.range`](./range)           | 10             | 区間反復、述語、遅延アダプタ                                 |
| [`std.assert`](./assert)         | 1              | アサーション                                                 |
| [`std.net`](./net)               | 4              | HTTP リクエストと URL パーセントエンコード・デコード         |
| [`std.concurrent`](./concurrent) | 3              | スリープ、スケジューラ譲り、スレッド識別                     |
| [`std.os`](./os)                 | 22             | ファイルハンドル、ディレクトリ、環境変数、作業ディレクトリ   |
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

モジュールから名前を指定してインポートすることもできます（定数を含む）：

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14)
}
```

## 引数借用規約

シグネチャ中の `&` は**読み取り専用の自動借用**（RFC-009
§2.8）を意味します：呼び出し元が渡した変数はムーブされず、呼び出し後も引き続き使用可能です。これは標準庫の多くの読み取り専用関数におけるデフォルトの形式です。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 3 か所すべて nums を読み取り専用借用し、呼び出し後も引き続き使用可能
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

`&` の付かない引数は**値渡し**を意味します。そのため多くの「変更」関数は
**ソース値を消費して新しい値を返す**関数型の形式であり、インプレースで書き換えるものではありません：

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // 新しいリストを返す；base はムーブ済み
    assert(list.len(extended) == 3)
}
```

各モジュールページの「意味論による分類」セクションでは、そのモジュール内の関数のうちどれが借用で、どれが消費で、どれがインプレース変更かを一覧しています。特に注意すべき点が 2 つあります：

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) のシグネチャは `&List(A)`
  と表記されていますが、実際にはリストを**インプレース変更**します
- [`os.open`](./os#open) が返すハンドルは**ワンショット**であり、 `open` → `write` → `close`
  の流れはコンパイルできません

## エラーモデル

標準庫には失敗の形態が 2 種類あり、各関数のエントリではそれぞれ「エラー」と「戻り値…」として注記されています：

| 形態                   | 振る舞い                              | 典型的なシナリオ                                                   |
| ---------------------- | ------------------------------------- | ------------------------------------------------------------------ |
| ランタイムエラーを送出 | `E6xxx` コードで現在の実行を終了      | 辞書キー欠落 `E6008`、インデックス範囲外 `E6003`、アサーション失敗 |
| センチネル値を返す     | 中断せず、`Void` / `-1` / `""` を返す | リストの境界外読み取り、空リストの先頭要素取得、環境変数の欠落     |

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

`string.parse_int` / `string.parse_float` は第 3 の形態に分類されます：**エラーを送出せず**、失敗を
`Result` の `Err` 値にラップして返します。分解には [`std.result`](./result) を使用するか、`?`
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

## イテレータプロトコル

`std.list` と `std.range` は同一のイテレータプロトコルを提供します。イテレータ自体は `Tuple`
による状態キャリアです。

> **ムーブセマンティクス**：`next` と `has_next`
> はいずれもイテレータを**ムーブ**します（シグネチャに `&`
> がありません）。したがって、取得するたびに再作成するか、あるいは `for ... in`
> を直接使用してください。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // has_next が it をムーブしたため、再作成してから要素を取得
    it2 = list.iter([1, 2, 3])
    assert(list.next(it2) == 1)
}
```

日常的な走査には `for ... in` を直接使用します：

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

[`range.map`](./range#map) / [`range.filter`](./range#filter) は**遅延**アダプタを返し、 `collect` /
`reduce` / `for_each` / `for ... in` で消費された時点で結果が生成されます：

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

## プラットフォーム対応

以下はオペレーティングシステムの機能に依存するため、`wasm32`
ターゲットでは**エクスポートされません**：

| 範囲                                                            | 必要条件              |
| --------------------------------------------------------------- | --------------------- |
| `std.os` 全体、`std.net` 全体、`std.weak` 全体                  | ファイル/ネットワーク |
| `std.concurrent` 全体                                           | スレッド              |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | 標準 I/O              |
| `std.time.sleep`                                                | スレッドスリープ      |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert` および `std.io.print` / `println` / `format_fallback`
はすべてのターゲットで利用可能です。

## 実装済みの欠陥

以下の問題はドキュメント執筆時に一つずつ実例を実行して確認したもので、すべて issue で追跡中です。修正後は対応するページ本文も同期して書き換える必要があります（生成領域のゲートがシグネチャ変更時に自動的に通知します）：

| 位置                                                         | 問題                                                                         | 追跡 |
| ------------------------------------------------------------ | ---------------------------------------------------------------------------- | ---- |
| [`net.http_get`](./net#http_get) / `http_post`               | プレースホルダ実装であり、リクエストを送信せず説明文字列を返す               | #56  |
| [`os.open`](./os#open)                                       | ハンドルがワンショットであり、`open`→`write`→`close` がコンパイル不可        | #337 |
| [`time.DateTime::*`](./time#datetime-フィールドアクセス不可) | 8 つのアクセサがソースコードから呼び出せない（エクスポート名に `::` を含む） | #338 |
| [`time.parse_time`](./time#parse_time)                       | `fmt` 引数が無視される；戻り値が継続使用不可                                 | #340 |
| [`math.clamp`](./math#clamp)                                 | `min > max` の場合、エラーを返さずインタプリタがパニックする                 | #339 |

## ドキュメント保守

本ディレクトリは**生成 + 手書き**の混在構造です：

- **生成領域**（`<!-- stdlib:KEY start/end -->` マークの間）：関数一覧表とシグネチャブロックは
  `StdModule::exports()` から派生します。シグネチャは `NativeExport::signature`
  からバイト単位で取得されるため、実装と乖離することは不可能です。
- **手書き領域**（マークの外側）：モジュールの概要、借用/ムーブセマンティクス、エラーモデル、既知の欠陥、サンプル。

ゲート（CI 上で `cargo test --lib` と共に実行）：

| ゲート         | テスト                                        | 役割                                                                       |
| -------------- | --------------------------------------------- | -------------------------------------------------------------------------- |
| ドリフト検出   | `test_stdlib_docs_match_generation`           | 生成領域が `exports()` と一致する必要がある                                |
| 孤児検出       | `test_stdlib_docs_has_no_orphan_module_pages` | モジューページがジェネレータの成果物より多くてはならない                   |
| カバレッジ     | `test_stdlib_docs_covers_interface_modules`   | ドキュメントのモジュール集合がインターフェースビューをカバーする必要がある |
| サンプル実行可 | `test_stdlib_docs_examples_run`               | 各 ```yaoxiang サンプルが実際に実行可能でなければならない                  |

`exports()` を変更した後、治癒ツールで生成領域を書き換えます：

```bash
cargo run --example gen-stdlib-docs
```

`gen-std-interfaces`（RFC-037 インターフェースビュー）、`tools/code-tables --fix`（RFC-013 コード表）と同型です。

## 関連ドキュメント

- [標準庫仕様](../language-spec/stdlib.md) —— 言語レベルの標準庫設計規約
- [FFI 仕様](../language-spec/ffi.md) —— ユーザー側 `native` 拡張と C ABI バインディング
- [エラーコードリファレンス](../error-code/) —— `E6xxx` ランタイムエラーコードの全表
