---
title: '標準ライブラリ概要'
description: 'YaoXiang 標準ライブラリモジュールの概要と使用規約'
---

# 標準ライブラリリファレンス

YaoXiang 標準ライブラリ（`std`）はモジュールを組織単位とし、各モジュールは `use` でインポート後、
`モジュール名.関数名(...)`
の形で呼び出します。本ディレクトリはモジュールごとに分割された API リファレンスです。

## モジュール索引

<!-- stdlib:index:modules start -->

| モジュール                       | 公開数 | 説明                                                       |
| -------------------------------- | ------ | ---------------------------------------------------------- |
| [`std.convert`](./convert)       | 11     | 任意値から String への変換                                 |
| [`std.dict`](./dict)             | 11     | 辞書の読み書き、キー値ビューとマージ                       |
| [`std.io`](./io)                 | 7      | 標準出力、標準入力とファイル全体の読み書き                 |
| [`std.list`](./list)             | 22     | リストの増減、スライス、高階関数とイテレータプロトコル     |
| [`std.math`](./math)             | 18     | 整数、浮動小数点と三角関数、PI/E/TAU 定数を含む            |
| [`std.string`](./string)         | 19     | 文字列検索、分割、フォーマットと解析                       |
| [`std.time`](./time)             | 14     | タイムスタンプ、フォーマットと DateTime フィールドアクセス |
| [`std.result`](./result)         | 9      | Result と Error の構築と分解                               |
| [`std.range`](./range)           | 10     | 区間反復、述語と遅延アダプタ                               |
| [`std.assert`](./assert)         | 1      | アサーション                                               |
| [`std.net`](./net)               | 4      | HTTP リクエストと URL パーセントエンコード/デコード        |
| [`std.concurrent`](./concurrent) | 3      | スリープ、スケジューリング譲歩とスレッド識別子             |
| [`std.os`](./os)                 | 22     | ファイルハンドル、ディレクトリ、環境変数と作業ディレクトリ |
| [`std.weak`](./weak)             | 2      | Arc / Weak 弱参照                                          |

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

モジュールから名前を指定してインポートすることもできます（定数を含む）：

```yaoxiang
use std.assert
use std.math.{E, PI, TAU}

main: () -> Void = {
    assert(PI > 3.14)
}
```

## 引数の借用規約

シグネチャ中の `&` は**読み取り専用の自動借用**を表します（RFC-009
§2.8）：呼び出し側が渡した変数はムーブされず、呼び出し後も引き続き使用可能です。これは標準ライブラリで多くの読み取り専用関数が採る既定の形式です。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    nums = [1, 2, 3]

    // 3 か所すべて nums を読み取り借用しており、呼び出し後も利用可能
    assert(list.len(nums) == 3)
    assert(list.len(nums) == 3)
    assert(list.contains(nums, 2))
}
```

`&` を伴わない引数は**値渡し**を意味します。そのため多くの「変更」関数は、
**ソース値を消費して新しい値を返す**関数型の形式となり、インプレースでの書き換えではありません。

```yaoxiang
use std.assert
use std.list

main: () -> Void = {
    base = [1, 2]
    extended = list.push(base, 3)   // 新しいリストを返す。base はムーブ済み
    assert(list.len(extended) == 3)
}
```

各モジュールページの「意味分類」セクションには、そのモジュールでどの関数が借用し、どれが消費し、どれがインプレース変更を行うかが列挙されています。次の 2 点には特に注意が必要です：

- [`list.pop`](./list#pop) / [`list.remove_at`](./list#remove_at) のシグネチャは `&List(A)`
  ですが、リストを**インプレースに変更**します
- [`os.open`](./os#open) が返すハンドルは**ワンショット**であり、`open` → `write` → `close`
  はコンパイルできません

## エラーモデル

標準ライブラリには 2 種類の失敗形態があり、各関数のエントリでは「エラー」と「戻り値 ...」として区別して記載されています：

| 形態                   | 挙動                                  | 典型的なシナリオ                                                   |
| ---------------------- | ------------------------------------- | ------------------------------------------------------------------ |
| ランタイムエラーを送出 | `E6xxx` コードで現在の実行を終了      | 辞書キー欠如 `E6008`、インデックス範囲外 `E6003`、アサーション失敗 |
| センチネル値を返却     | 中断せず、`Void` / `-1` / `""` を返す | リスト範囲外読み取り、空リストの先頭要素取得、環境変数欠如         |

よく使われるランタイムエラーコード：

| エラーコード | 意味                               | 発生例                                     |
| ------------ | ---------------------------------- | ------------------------------------------ |
| `E6003`      | インデックス範囲外                 | `list.set(l, 99, v)`                       |
| `E6005`      | アサーション失敗                   | `assert(false)`                            |
| `E6007`      | 汎用ランタイムエラー               | ファイルが存在しない、`result.unwrap` 失敗 |
| `E6008`      | キー欠如                           | `dict.get(d, "nope")`                      |
| `E6010`      | 整数解析失敗（Err 値として）       | `string.parse_int("abc")`                  |
| `E6011`      | 浮動小数点解析失敗（Err 値として） | `string.parse_float("abc")`                |

エラーコードの全表は[エラーコードリファレンス](../error-code/)を参照してください。

`string.parse_int` / `string.parse_float` は第 3 の形態に属します：**エラーを送出せず**、失敗を
`Result` の `Err` 値としてラップして返します。`Result` は [`std.result`](./result) で分解するか `?`
で伝播できます。

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

`std.list` と `std.range`
は同一のイテレータプロトコルを提供します。イテレータ自体は状態キャリアとして `Tuple`
で表現されます。

> **ムーブセマンティクス**：`next` と `has_next`
> はいずれもイテレータを**ムーブ**します（シグネチャに `&`
> が付きません）。そのため、毎回取得するたびにイテレータを再作成するか、`for ... in`
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

日常的な走査には `for ... in` を直接使います：

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

[`range.map`](./range#map) / [`range.filter`](./range#filter)
は**遅延**アダプタを返し、結果を生成するには `collect` / `reduce` / `for_each` / `for ... in`
で消費する必要があります：

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

以下は OS の機能に依存しており、`wasm32` ターゲットでは**エクスポートされません**：

| 範囲                                                            | 必要機能              |
| --------------------------------------------------------------- | --------------------- |
| `std.os` 全部、`std.net` 全部、`std.weak` 全部                  | ファイル/ネットワーク |
| `std.concurrent` 全部                                           | スレッド              |
| `std.io.read_line` / `read_file` / `write_file` / `append_file` | 標準 I/O              |
| `std.time.sleep`                                                | スレッドスリープ      |

`std.string` / `std.list` / `std.dict` / `std.math` / `std.convert` / `std.result` / `std.range` /
`std.assert` および `std.io.print` / `println` / `format_fallback`
はすべてのターゲットで利用可能です。

## 既知のギャップ

以下は本ドキュメント執筆時にサンプルを実際に実行して一つずつ確認した問題で、すべて issue で追跡中です。修正後は対応するページの本文も同期して書き換える必要があります（生成領域のゲートはシグネチャが変化した際に自動で通知します）：

| 位置                                                 | 問題                                                                         | 追跡 |
| ---------------------------------------------------- | ---------------------------------------------------------------------------- | ---- |
| [`net.http_get`](./net#http_get) / `http_post`       | プレースホルダ実装で、リクエストを送らず説明文字列を返す                     | #56  |
| [`os.open`](./os#open)                               | ハンドルがワンショットで、`open`→`write`→`close` がコンパイル不可            | #337 |
| [`time.DateTime::*`](./time#datetime-字段访问不可用) | 8 つのアクセサがソースコードから呼び出せない（エクスポート名に `::` を含む） | #338 |
| [`time.parse_time`](./time#parse_time)               | `fmt` 引数が無視される；戻り値がそれ以降使用できない                         | #340 |
| [`math.clamp`](./math#clamp)                         | `min > max` のときインタプリタが panic し、エラーを返さない                  | #339 |

## ドキュメント保守

本ディレクトリは**生成 + 手書き**のハイブリッド構造です：

- **生成領域**（`<!-- stdlib:KEY start/end -->`
  マークの間）：関数一覧表とシグネチャブロック。`StdModule::exports()` から派生します。シグネチャは
  `NativeExport::signature` からバイト単位で取得されるため、実装と乖離することは原理的にありません。
- **手書き領域**（マークの外）：モジュールの概要、借用/ムーブセマンティクス、エラーモデル、既知のギャップ、サンプル。

ゲート（`cargo test --lib` とともに CI で実行）：

| ゲート               | テスト                                        | 役割                                                                         |
| -------------------- | --------------------------------------------- | ---------------------------------------------------------------------------- |
| 乖離検出             | `test_stdlib_docs_match_generation`           | 生成領域は `exports()` と一致しなければならない                              |
| 孤児検出             | `test_stdlib_docs_has_no_orphan_module_pages` | モジュールのページはジェネレータの成果物より多くあってはならない             |
| カバレッジ           | `test_stdlib_docs_covers_interface_modules`   | ドキュメントのモジュール集合はインターフェースビューを網羅しなければならない |
| サンプルの実行可能性 | `test_stdlib_docs_examples_run`               | 各 ```yaoxiang サンプルは実際に実行できなければならない                      |

`exports()` 変更後は、ヒールツールで生成領域を書き換えてください：

```bash
cargo run --example gen-stdlib-docs
```

`gen-std-interfaces`（RFC-037 インターフェースビュー）、`tools/code-tables --fix`（RFC-013 コード表）と同型です。

## 関連ドキュメント

- [標準ライブラリ仕様](../language-spec/stdlib.md) —— 言語レベルでの標準ライブラリ設計規約
- [FFI 仕様](../language-spec/ffi.md) —— ユーザ側の `native` 拡張と C ABI バインディング
- [エラーコードリファレンス](../error-code/) —— `E6xxx` ランタイムエラーコードの全表
