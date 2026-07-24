---
title: "RFC-030: assert アサーション機構"
status: "承認済み"
author: "晨煦"
created: "2026-06-15"
updated: "2026-07-14"
decision: "assert と Assert は同一の二面であり、dispatch により自動分派される。6 Phase すべて実装済み（#157-#162 クローズ済み）。std.assert モジュールへの統一登録（#169 クローズ済み）、assert native 関数 + Assert/IsTrue 型族が同一経路。"
issue: "#97"
issues_impl:
  - "#155"
  - "#157"
  - "#158"
  - "#159"
  - "#160"
  - "#161"
  - "#162"
  - "#169"
---

# RFC-030: assert アサーション機構

## 概要

YaoXiang に `assert` アサーション機構を導入し、テスト、前置条件チェック、ランタイム panic に使用する。`assert` とコンパイル時の精化型 `Assert(C)`（RFC-011 §4.3 参照）は**同一の精化プリミティブの二面**であり、dispatch が「述語の自由変数がコンパイル時に到達可能か」に基づき、コンパイル時の証明またはランタイムチェックへ自動的に振り分ける。`assert(false, "msg")` は `raise` と同等であり、別途の `throw`/`raise` キーワードは不要である。

## 動機

### なぜこの機能が必要なのか？

現在の YaoXiang の E2E テストは `if` + `io.println` + `return` でアサーションを模倣することしかできない：

```yaoxiang
val = some_func()
if val != 42 {
    io.println("FAIL: expected 42")
    return
}
```

この書き方には 3 つの問題がある：

1. **ボイラープレートが多い**：各アサーションに 4 行必要で、テストファイルが膨張する
2. **エラーメッセージが弱い**：文字列を手動で連結し、ソースコードの位置情報がない
3. **合成不可**：アサーションを一括登録できず、テストフレームワークに引数として渡せない

### 現状の問題点

- 統一されたアサーション機構がない
- テストコードが `if` + 出力 + `return` のパターンに溢れる
- バイトコード層には既に `Throw` 命令があるが、言語層には公開されていない
- RFC-011 はコンパイル時の `Assert(C)` 条件型を定義しているが、ランタイムの `assert()` は未実装

### 設計原則

`assert` は YaoXiang で唯一のユーザーモード panic 機構である。`assert(false, "msg")` は `raise` と同等であり、別途の `throw`/`raise` キーワードは不要である。`assert` 関数自体が `if raise` の最良のカプセル化である。

**新しいキーワードを導入しない。新しい構文を導入しない。すべては関数呼び出しである。**

## 案 A：native 関数

`assert` を native 関数として実装し、新しいキーワードを導入しない。

```yaoxiang
use std.assert.assert

main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### オーバーロード署名

`assert` には 2 つのオーバーロードがある：

```
// 中核的署名：assert は Assert の値宇宙への導入子である
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))
//                                       ^^^^^^^^^^^^^^^^^^^^^^^^
//                                       戻り値は精化型、() ではない
//
// IsTrue: Bool -> Type は真理値から型への橋渡し：
//   IsTrue(true)  = Void   (⊤，プログラム続行)
//   IsTrue(false) = Never  (⊥，発散/コンパイルエラー)
```

`assert` の実際の振る舞いは dispatch によって決定される：
- すべての自由変数がコンパイル時に既知 → **コンパイル時**：コンパイラが cond を評価、true → Void として消去、false → コンパイルエラー（Never は居住不可）
- ランタイムの自由変数が存在する → **ランタイム**：check を挿入し、フロー敏感仮定集合 Γ に精化事実を注入

オプションのメッセージ `?msg` と Result オーバーロード（下記参照）はランタイムの raise ペイロードとして保持される。

#### オーバーロード 1：条件アサーション `(Bool, ?String | Error)`

`Bool` + オプションのメッセージ。メッセージは `String` または `Error` 値：

```yaoxiang
assert(1 + 1 == 2)                    // メッセージなし、デフォルトの panic メッセージ
assert(1 + 1 == 2, "math is broken")   // 文字列メッセージ
assert(x > 0, my_error)                // Error 値を直接送出
```

`assert(false, "msg")` は YaoXiang の `raise`/`throw` 等価体である——別途のキーワードは不要。

#### オーバーロード 2：Result アサーション `(Result)`

単一の `Result` 引数、`Err` かどうかを自動チェック：

### 利点

- **構文変更ゼロ**：純粋関数、新しいキーワード不要
- **新概念ゼロ**：既存の native 関数登録機構を再利用
- **高い拡張性**：関数オーバーロードで複数署名を自然にサポート
- **自己文書化**：`std.assert` 名前空間自体がドキュメント

### 欠点

- なし。assert の型署名が正しければ、コンパイラは関数の到達可能性解析でデッドコードを推論できる。追加の pass は不要。

### ランタイム動作

1. 第 1 引数 `condition: Bool` を評価
2. `true` なら、`Unit` を返す
3. `false` なら、ランタイム panic を発動する：
   - `message` の内容を出力（あれば）
   - コールスタックを出力（debug モード時）
   - 現在の実行を終了

#### 各オーバーロードの失敗時の動作

| 署名 | 失敗時の動作 |
|------|-----------|
| `assert(false)` | デフォルトの panic メッセージ |
| `assert(false, "msg")` | 文字列メッセージを出力後に panic |
| `assert(false, error_val)` | Error 値を送出 |
| `assert(Err(x))` | Err の内容を取り出して panic |

### コンパイル時 Assert との関係

`assert` と `Assert` は**同一の精化プリミティブの二面**であり、dispatch 分派パイプラインが「述語の自由変数がコンパイル時に到達可能か」に基づき自動的に選択する：

| 条件 | 分派 | 動作 |
|------|------|------|
| すべての自由変数がコンパイル時既知 | コンパイル時 → 証明パイプライン | Proved → 消去、Disproved → コンパイルエラー、Unknown → 証明を要求 |
| ランタイムの自由変数が存在 | ランタイム → check を挿入 | Bool チェック + フロー敏感仮定集合 Γ への精化事実の注入 |

```yaoxiang
use std.assert

# コンパイル時既知（ジェネリクス引数）—— コンパイル時経由、ランタイムオーバーヘッドゼロ
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N はジェネリクス引数、コンパイル時評価
}

# ランタイム値 —— ランタイム経由、Bool チェックを挿入
x = read_int()
assert.assert(x > 0, "expected positive")  # ランタイム check
```

> **2026-07-12 統一案**：従来の「完全独立」という結論は廃止された。`assert()` は `Assert` の値導入子であり、dispatch が自動的に振り分ける。

### コンパイラの変更点

**parser、AST、typecheck、IR gen の変更は不要。**

`src/std/` 下に native 関数の登録を追加するだけでよい：

1. `src/std/assert.rs` を新規追加
2. `std.assert.assert` と `std.assert.Assert`（後者はコンパイル時条件型、#155 参照）を登録
3. 内部で既存の `BytecodeInstr::Throw` 命令を呼び出す

### 利点

- **構文変更ゼロ**：純粋関数、新しいキーワード不要
- **新概念ゼロ**：既存の native 関数登録機構を再利用
- **高い拡張性**：関数署名は `assert_eq` 等のバリアントに拡張可能（将来）
- **自己文書化**：`std.assert` 名前空間自体がドキュメント

### 欠点

- ~~コンパイル時不可知：案 B（キーワード）と異なり、コンパイル時にデッドコード除去ができない~~ → **統一案下では成立しない**。コンパイル時モードの assert は証明パイプラインを経由し、コンパイル時に既知の cond は消去またはコンパイルエラーになる（`assert(false)` → Never → デッドコード）。
- debug モードでのみコールスタックを取得可能

## 案 B：組み込みキーワード（統一案により廃止済み）

> 廃止済み。案 A と B の対立は dispatch 分派パイプラインによって解消される——assert は Assert の値導入子であり、コンパイル時既知なら証明パイプラインを（ランタイムオーバーヘッドゼロ）、ランタイムなら check を経由する。「関数」と「キーワード」の二者択一は不要である。下記は歴史的記録。

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### 型署名

独立した型署名なし——キーワードは parser が処理する。

### ランタイム動作

案 A と同じ。

### コンパイラの変更点

parser、AST、typecheck、IR gen の変更が必要：
1. parser：`Expr::Assert` バリアントを新規追加
2. AST：`Expr::Assert` ノードを新規追加
3. typecheck：引数の型を検証
4. IR gen：`BytecodeInstr::Throw` を生成

### 利点

- コンパイル時にソースコードの位置が分かる（debug info に依存しない）
- コンパイル時に定数畳み込み可能：`assert(true)` → 空操作、`assert(false)` → コンパイルエラー

### 欠点

| 欠点 | 影響 |
|------|------|
| パーサーの変更が必要 | 新しい構文ノードが導入され、保守コストが増加 |
| キーワードは拡張不可 | `assert_eq` 等のバリアントも依然として関数が必要 |
| コンパイル時の利点は非実用的 | 下記分析参照 |

### 比較

| 次元 | 案 A（関数） | 案 B（キーワード） |
|------|---------------|-----------------|
| 実装コスト | 約 20 行 | parser + AST + typecheck + IR gen |
| 構文変更 | なし | 新キーワード |
| 拡張性 | 関数オーバーロード | 付随マクロが必要 |
| ソースコード位置 | debug info | コンパイル時に取得可能 |
| 定数畳み込み | pass のサポートが必要 | コンパイル時に取得可能 |
| ランタイムオーバーヘッド | 関数呼び出し | 極小 |

### コンパイル時分析の現実的制約

案 B の中核的利点——コンパイル時分析——は**定数畳み込み pass** が必要である。コンパイラがコンパイル時に `assert(false)` 中の `false` を評価し、これがデッドコードであると判断できなければならない。

YaoXiang には現在、定数畳み込み pass がない。仮に案 B を採用しても、`assert(x > 0)` のような一般的な書き方はコンパイル時には分析できない。`assert(true)` / `assert(false)` のようなリテラルだけが分析可能である。

したがって、案 B のコンパイル時の利点**は現状では理論上のものに過ぎず、実用的ではない**。

---

## オープン問題

- [x] ~~案 A と案 B のどちらを選ぶか？~~ → **統一案：assert は Assert の値導入子**。案 A と B の対立は dispatch 分派パイプラインによって解消される——コンパイル時既知なら証明パイプラインを、ランタイムなら check を経由する。「二者択一」は不要。
- [x] ~~`assert` は `message` を伴わない簡易形 `assert(cond)` をサポートする必要があるか？~~ → **サポートする。`assert(cond, ?msg)`、message はオプション。**
- [x] ~~`assert_eq`、`assert_ne` 等のバリアントは必要か？~~ → **不要。YAGNI。テストフレームワークが形になってから考える。**
- [x] ~~panic の出力にソースコードの位置は含まれるか？~~ → 案 A は debug info（コールスタック）に依存。
- [x] ~~assert / Assert の統一問題~~ → **確定済み**。統一案：`assert: (Bool) -> Assert(IsTrue(cond))`、同一の二面、dispatch による自動分派。詳細は [#156](https://github.com/ChenXu233/YaoXiang/issues/156)（クローズ済み）参照。`Never` 型（⊥）を `assert(false)` の戻り型として組み込む。

### 2026-07-05：案 A の選択（統一案により廃止済み）

案 A の 20 行実装が価値とコストの点で勝出した。2026-07-12 の統一案確定後、案 A/B の対立は dispatch 分派パイプラインにより解消——assert は Assert の値導入子であり、もはや「関数」と「キーワード」の二者択一ではない。

### 2026-07-12：統一案確定（2026-07-11 の「完全独立」結論を廃止）

**結論**：`assert` と `Assert` は 2 つの独立した機構ではない。`assert: (Bool) -> Assert(IsTrue(cond))` ——dispatch が自動的に振り分ける：

- コンパイル時既知 → 証明パイプラインへ（Proved 消去 / Disproved エラー / Unknown 証明要求）
- ランタイム入力 → check を挿入 + Γ 仮定を注入

**モジュール構造**：`std.assert` はランタイムアサーション（`assert`）とコンパイル時精化型（`Assert`、`IsTrue`）を統一的に担う。「別々に実装」するのではなく、同一のプリミティブの二面である。

### 2026-07-11：assert のオーバーロード設計

**問題**：なぜ `assert` は 2 つのオーバーロードを必要とし、統一的な `(Bool, ?String)` ではないのか？

**解答**：

ランタイムの `assert()` は YaoXiang で唯一のユーザーモード panic 機構である。`assert(false, "msg")` は他の言語の `raise`/`throw` と同等である。したがって、3 つのシナリオをカバーする必要がある：
1. 条件 + 簡易メッセージ：`assert(cond, "msg")`
2. 条件 + カスタム Error：`assert(cond, my_error)`
3. Result チェック：`assert(result)` — `if is_err { panic }` の最も簡潔な形

Result オーバーロードの合理性は、エラー伝播の最短経路にある——「Result は Ok であるべき、さもなくば死」。`.is_ok()` を先に呼び、エラーを別途処理する必要はない。

## 付録 B：設計決定の記録

| 決定 | 決定内容 | 日付 | 記録者 |
|------|------|------|--------|
| 案 A か案 B かの選択 | **統一案**：dispatch 分派パイプラインが A/B の対立を解消、assert は Assert の値導入子 | 2026-07-12 | 晨煦 |
| message はオプションか | **はい**：`assert(cond, ?msg)`、String または Error | 2026-07-11 | 晨煦 |
| assert_eq 等のバリアントは必要か | **不要**。YAGNI、テストフレームワークができてから判断 | 2026-07-11 | 晨煦 |
| 別途の raise/throw キーワードは必要か | **不要**。`assert(false, msg)` は raise と等価 | 2026-07-11 | 晨煦 |
| assert と Assert の関係 | **同一の二面**。`assert: (Bool) -> Assert(IsTrue(cond))`、dispatch による自動分派 | 2026-07-12 | 晨煦 |

## 参考文献

- [RFC-007: 関数定義構文統一案](007-function-syntax-unification.md) — `name: type = value` モデル
- [RFC-010: 統一型構文](010-unified-type-syntax.md) — 型システム基礎
- [RFC-011: ジェネリクスシステム設計 §4.3](../accepted/011-generic-type-system.md) — コンパイル時検証と `Assert(C)` 条件型
- [RFC-026: FFI コア機構](026-ffi-core-mechanism.md) — native 関数登録機構
- [RFC-027: コンパイル時述語と統一静的検証](../accepted/027-compile-time-evaluation-types.md) — コンパイル時評価システム