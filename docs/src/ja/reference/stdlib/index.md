---
title: '標準庫概要'
description: 'YaoXiang 標準庫モジュールの概要と利用規約'
---

# 標準庫リファレンス

YaoXiang 標準庫（`std`）はモジュール単位で編成されており、各モジュールは `use` でインポートした後
`モジュール名.関数名(...)`
で呼び出します。このディレクトリはモジュール別に分割された API リファレンスです。

## モジュール索引

<!-- stdlib:index:modules start -->

| モジュール                       | エクスポート数 | 説明                                                       |
| -------------------------------- | -------------- | ---------------------------------------------------------- |
| [`std.convert`](./convert)       | 11             | 任意の値から String への変換                               |
| [`std.dict`](./dict)             | 11             | 辞書の読み書き、キー・値ビューとマージ                     |
| [`std.fs`](./fs)                 | 22             | ファイル、ディレクトリ、パス操作                           |
| [`std.io`](./io)                 | 4              | 標準出力、標準入力とフォーマット                           |
| [`std.math`](./math)             | 18             | 整数、浮動小数点と三角関数。PI/E/TAU 定数を含む            |
| [`std.string`](./string)         | 21             | 文字列検索、分割、フォーマットと解析                       |
| [`std.time`](./time)             | 14             | タイムスタンプ、フォーマットと DateTime フィールドアクセス |
| [`std.result`](./result)         | 8              | Result と Error の構築とアンラップ                         |
| [`std.range`](./range)           | 10             | 区間反復、述語と遅延アダプタ                               |
| [`std.assert`](./assert)         | 1              | アサーション                                               |
| [`std.net`](./net)               | 4              | HTTP リクエストと URL パーセントエンコーディング           |
| [`std.concurrent`](./concurrent) | 3              | スリープ、スケジューリングの明け渡しとスレッド識別         |
| [`std.os`](./os)                 | 12             | ファイルハンドル、環境変数と作業ディレクトリ               |
| [`std.weak`](./weak)             | 2              | Arc / Weak 弱参照                                          |

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

モジュールから名前で個別にインポートすることもできます（定数を含む）：

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14)
}
```

## 引数の借用規約

シグネチャ中の `&` は**読み取り専用の自動借用**（RFC-009
§2.8）を示します。呼び出し元が渡した変数はムーブされず、呼び出し後も引き続き使用可能です。これは標準庫の多数の読み取り専用関数におけるデフォルトの形式です。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 3 か所とも nums を読み取り専用で借用しているので、呼び出し後も引き続き使用可能
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

`&`
の付かない引数は**値渡し**を意味します。多くの「変更」関数はそのため**ソース値を消費して新しい値を返す**関数型の形式であり、インプレースで書き換えるものではありません：

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // 新しいリストを返す。base はすでにムーブ済み
    assert(list.len(extended) == 3)
}
```

各モジュールページの「意味的分類」セクションには、そのモジュールでどの関数が借用し、どの関数が消費し、どの関数がインプレース変更を行うかが記載されています。特に注意すべき点が一つあります：

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) はシグネチャ上は `&List(A)`
  となっていますが、リストを**インプレース変更**します

## エラーモデル

標準庫には 2 種類の失敗形態があり、各関数のエントリではそれぞれ「エラー」と「戻り値 ...」で示されます。

| 形態                     | 動作                                  | 典型的なシナリオ                                                     |
| ------------------------ | ------------------------------------- | -------------------------------------------------------------------- |
| ランタイムエラーをスロー | `E6xxx` コードで現在の実行を終了      | 辞書のキー欠如 `E6008`、インデックス範囲外 `E6003`、アサーション失敗 |
| センチネル値を返す       | 中断せず、`Void` / `-1` / `""` を返す | リストの境界外読み取り、空リストの先頭要素取得、環境変数の欠如       |

一般的なランタイムエラーコード：

| エラーコード | 意味                               | トリガー例                                 |
| ------------ | ---------------------------------- | ------------------------------------------ |
| `E6003`      | インデックス範囲外                 | `list.set(l, 99, v)`                       |
| `E6005`      | アサーション失敗                   | `assert(false)`                            |
| `E6007`      | 汎用ランタイムエラー               | ファイルが存在しない、`result.unwrap` 失敗 |
| `E6008`      | キー欠如                           | `dict.get(d, "nope")`                      |
| `E6010`      | 整数解析失敗（Err 値として）       | `string.parse_int("abc")`                  |
| `E6011`      | 浮動小数点解析失敗（Err 値として） | `string.parse_float("abc")`                |

エラーコードの全表は[エラーコードリファレンス](../error-code/)を参照してください。

`string.parse_int` / `string.parse_float` は第 3 の形態に属します：**エラーをスローせず**、失敗を
`Result` の `Err` 値にラップして返します。[`std.result`](./result) でアンラップするか、`?`
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

## 反復プロトコル

`std.list` と `std.range` は同じ反復子プロトコルを提供します。反復子自体は `Tuple`
状態キャリアです。

> **ムーブセマンティクス**：`next` と `has_next` はどちらも反復子を**ムーブ**します（シグネチャに
> `&` がありません）。したがって、毎回取得するたびに再作成するか、`for ... in`
> を直接使用する必要があります。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    it = list.iter([1, 2, 3])
    assert(list.has_next(it))

    // has_next が it をムーブしてしまったので、再作成してから要素を取り出す
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

[`range.map`](./range#map) / [`range.filter`](./range#filter) は**遅延**アダプタを返し、`collect` /
`reduce` / `for_each` / `for ... in` で消費された後にのみ結果を生成します：

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

以下はオペレーティングシステムの機能に依存しており、`wasm32`
ターゲットでは**エクスポートされません**：

| 範囲                                                            | 必要条件              |
| --------------------------------------------------------------- | --------------------- |
| `std.os` 全部、`std.net` 全部、`std.weak` 全部                  | ファイル/ネットワーク |
| `std.concurrent` 全部                                           | スレッド              |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | 標準 I/O              |
| `std.time.sleep`                                                | スレッドスリープ      |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert`、および `std.io.print` / `println` / `format_fallback`
はすべてのターゲットで利用可能です。

## 実装済みのギャップ

以下の問題はドキュメント作成時にサンプルを実際に実行して一つずつ確認しており、すべて issue で追跡中です。

**修正済み（2026-09-19）**：#337 / #338 / #339 /
#340 の 4 件はすべて修正済みで、対応するページの本文も通常の使い方の説明に同期して書き換えられました：

| 位置                                          | 元の問題                                                                             | 修正                                         |
| --------------------------------------------- | ------------------------------------------------------------------------------------ | -------------------------------------------- |
| [`os.open`](./os#open)                        | ハンドルが使い捨てで、`open`→`write`→`close` がコンパイルできない                    | ハンドルは参照渡しに変更 ✅                  |
| [`time.datetime_*`](./time#datetime-字段访问) | 8 つのアクセサがソースコードから呼び出せない（エクスポート名に `::` が含まれている） | フラットな名前 `datetime_year` などに変更 ✅ |
| [`time.parse_time`](./time#parse_time)        | `fmt` 引数が無視される；戻り値が使い続けられない                                     | fmt に従って段階的に解析 ✅                  |
| [`math.clamp`](./math#clamp)                  | `min > max` でインタプリタがパニックし、エラーを返さない                             | `E6007` を返す ✅                            |

**未解決**：

| 位置                                           | 問題                                                         | 追跡 |
| ---------------------------------------------- | ------------------------------------------------------------ | ---- |
| [`net.http_get`](./net#http_get) / `http_post` | プレースホルダ実装で、リクエストを送信せず、説明文字列を返す | #56  |

## ドキュメント保守

このディレクトリは**生成 + 手書き**の混合構造です：

- **生成エリア**（`<!-- stdlib:KEY start/end -->` マーカー間）：関数一覧表とシグネチャブロックは
  `StdModule::exports()` から派生します。シグネチャはバイト単位で `NativeExport::signature`
  から取得されるため、実装とずれることはありません。
- **手書きエリア**（マーカー外）：モジュールの概要、借用/ムーブセマンティクス、エラーモデル、既知のギャップとサンプル。

ゲート（CI 内で `cargo test --lib` と一緒に実行）：

| ゲート           | テスト                                        | 役割                                                                           |
| ---------------- | --------------------------------------------- | ------------------------------------------------------------------------------ |
| ドリフト検出     | `test_stdlib_docs_match_generation`           | 生成エリアが `exports()` と一致している必要がある                              |
| 孤児検出         | `test_stdlib_docs_has_no_orphan_module_pages` | モジューページがジェネレータの出力より多くてはならない                         |
| カバレッジ       | `test_stdlib_docs_covers_interface_modules`   | ドキュメントのモジュール集合がインターフェースビューをカバーしている必要がある |
| サンプル実行可能 | `test_stdlib_docs_examples_run`               | 各 ```yaoxiang サンプルが実際に実行可能である必要がある                        |

`exports()` 変更後は、ヒールツールを使って生成エリアを書き換えます：

```bash
cargo run --example gen-stdlib-docs
```

これは
`gen-std-interfaces`（RFC-037 インターフェースビュー）、`tools/code-tables --fix`（RFC-013 コード表）と同形です。

## 関連ドキュメント

- [標準庫仕様](../language-spec/stdlib.md) —— 言語レベルの標準庫設計規約
- [FFI 仕様](../language-spec/ffi.md) —— ユーザーサイドの `native` 拡張と C ABI バインディング
- [エラーコードリファレンス](../error-code/) —— `E6xxx` ランタイムエラーコード全表
