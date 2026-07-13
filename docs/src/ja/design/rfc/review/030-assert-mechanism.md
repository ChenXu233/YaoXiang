---
title: "RFC-030: assert アサーション機構"
status: "審査中"
author: "晨煦"
created: "2026-06-15"
updated: "2026-07-14"
decision: "assert と Assert は表裏一体であり、dispatch により自動振り分け。6 Phase すべて実装完了（#157-#162 クローズ済み）。"
issue: "#97"
issues_impl:
  - "#155"
  - "#157"
  - "#158"
  - "#159"
  - "#160"
  - "#161"
  - "#162"
---

# RFC-030: assert アサーション機構

## 要約

YaoXiang に `assert` アサーション機構を導入し、テスト・事前条件チェック・ランタイム panic に用いる。`assert` とコンパイル時の精化型 `Assert(C)`（RFC-011 §4.3 参照）は**同じ精化プリミティブの二面**であり、dispatch が「述語の自由変数がコンパイル時に到達可能か」に応じてコンパイル時証明またはランタイム検査へ自動振り分けする。`assert(false, "msg")` は `raise` と等価であり、専用の `throw`/`raise` キーワードは要らない。

## 動機

### なぜこの機能が必要か？

現在の YaoXiang の E2E テストは `if` + `io.println` + `return` でアサーションを模倣するしかない：

```yaoxiang
val = some_func()
if val != 42 {
    io.println("FAIL: expected 42")
    return
}
```

この書き方には 3 つの問題がある：

1. **ボイラープレートが多い**：アサーション 1 件につき 4 行必要になり、テストファイルが肥大化する
2. **エラーメッセージが貧弱**：文字列を手動で連結するため、ソースコードの位置情報がない
3. **合成できない**：アサーションを一括登録したり、テストフレームワークに引数として渡したりできない

### 現状の問題

- 統一されたアサーション機構がない
- テストコードに `if` + 出力 + `return` のパターンが氾濫している
- バイトコード層には既に `Throw` 命令があるが、言語層では公開されていない
- RFC-011 でコンパイル時 `Assert(C)` 条件型は定義されているが、ランタイム `assert()` は未実装

### 設計原則

`assert` は YaoXiang における唯一のユーザーランド panic 機構である。`assert(false, "msg")` は `raise` と等価であり、専用の `throw`/`raise` キーワードは要らない。`assert` 関数自体が `if raise` の最良のラッパーである。

**新しいキーワードは導入しない。新しい構文も導入しない。すべて関数呼び出しで実現する。**

## 案 A：native 関数

`assert` を native 関数として実装し、新しいキーワードを導入しない。

```yaoxiang
use std.assert.assert

main = {
    assert(1 + 1 == 2, "math is broken")
    assert(get_name() == "YaoXiang", "name mismatch")
}
```

### オーバーロードシグネチャ

`assert` には 2 つのオーバーロードがある：

```
// 中核シグネチャ：assert は Assert の値宇宙への導入子
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))
//                                       ^^^^^^^^^^^^^^^^^^^^^^^^
//                                       返り値は精化型であり、() ではない
//
// IsTrue: Bool -> Type は真理値から型への橋渡し：
//   IsTrue(true)  = Void   (⊤、プログラムは継続)
//   IsTrue(false) = Never  (⊥、発散／コンパイルエラー)
```

`assert` の実際の挙動は dispatch 振り分けにより決定される：
- すべての自由変数がコンパイル時既知 → **CompileTime**：コンパイラが cond を評価し、true → Void へ消去、false → コンパイルエラー（Never は inhabited 不可）
- ランタイム自由変数が存在 → **Runtime**：check を挿入し、フロー敏感仮定集合 Γ へ精化事実を注入

オプションのメッセージ `?msg` および Result オーバーロード（後述）は、ランタイム raise のペイロードとして保持する。

#### オーバーロード 1：条件アサーション `(Bool, ?String | Error)`

`Bool` + 任意のメッセージ。メッセージは `String` または `Error` 値をとり得る：

```yaoxiang
assert(1 + 1 == 2)                    // メッセージなし、デフォルトの panic メッセージ
assert(1 + 1 == 2, "math is broken")   // 文字列メッセージ
assert(x > 0, my_error)                // Error 値を直接送出
```

`assert(false, "msg")` は YaoXiang における `raise`/`throw` の等価体であり、専用キーワードは要らない。

#### オーバーロード 2：Result アサーション `(Result)`

`Result` を 1 引数として取り、`Err` かどうかを自動判定する：

### 利点

- **構文変更ゼロ**：純粋な関数であり、新しいキーワードは不要
- **新概念ゼロ**：既存の native 関数登録機構を再利用
- **高い拡張性**：関数のオーバーロードにより複数シグネチャを自然にサポート
- **自己文書化**：`std.assert` 名前空間自体がドキュメントとなる

### 欠点

- なし。assert の型シグネチャが正しければ、コンパイラは関数の到達性解析によりデッドコードを推論できる。追加 pass は不要。

### ランタイム挙動

1. 第 1 引数 `condition: Bool` を評価
2. `true` なら `Unit` を返す
3. `false` ならランタイム panic を発火：
   - `message` の内容を出力（あれば）
   - コールスタックを出力（debug モード時）
   - 現在の実行を終了

#### 各オーバーロードの失敗時挙動

| シグネチャ | 失敗時の挙動 |
|------|-----------|
| `assert(false)` | デフォルトの panic メッセージ |
| `assert(false, "msg")` | 文字列メッセージを出力して panic |
| `assert(false, error_val)` | Error 値を送出 |
| `assert(Err(x))` | Err の内容を抽出して panic |

### コンパイル時 Assert との関係

`assert` と `Assert` は**同じ精化プリミティブの二面**であり、dispatch 振り分けパイプラインが「述語の自由変数がコンパイル時に到達可能か」に応じて自動的に選択する：

| 条件 | 振り分け | 挙動 |
|------|------|------|
| すべての自由変数がコンパイル時既知 | CompileTime → 証明パイプライン | Proved → 消去、Disproved → コンパイルエラー、Unknown → 証明要求 |
| ランタイム自由変数が存在 | Runtime → check 挿入 | Bool 検査 + フロー敏感仮定集合 Γ へ精化事実を注入 |

```yaoxiang
use std.assert

# コンパイル時既知（ジェネリックパラメータ）— CompileTime 経路、ランタイムオーバーヘッドゼロ
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N はジェネリックパラメータなのでコンパイル時評価
}

# ランタイム値 — Runtime 経路、Bool check を挿入
x = read_int()
assert.assert(x > 0, "expected positive")  # ランタイム check
```

> **2026-07-12 統一方案**：従来の「完全独立」結論は廃止された。`assert()` は `Assert` の値導入子であり、dispatch が自動振り分けする。

### コンパイラ変更点

**parser、AST、typecheck、IR gen の変更は不要。**

`src/std/` 配下への native 関数登録追加のみ：

1. `src/std/assert.rs` を新設
2. `std.assert.assert` および `std.assert.Assert`（後者はコンパイル時条件型、#155 参照）を登録
3. 内部で既存の `BytecodeInstr::Throw` 命令を呼び出す

### 利点

- **構文変更ゼロ**：純粋な関数であり、新しいキーワードは不要
- **新概念ゼロ**：既存の native 関数登録機構を再利用
- **高い拡張性**：関数シグネチャは `assert_eq` などの変種（将来）へ拡張可能
- **自己文書化**：`std.assert` 名前空間自体がドキュメントとなる

### 欠点

- ~~コンパイル時に知り得ない：案 B（キーワード）と異なり、コンパイル時のデッドコード除去ができない~~ → **統一方案の下では成立しない**。CompileTime モードの assert は証明パイプラインを辿り、コンパイル時既知の cond は消去されるかコンパイルエラーとなる（`assert(false)` → Never → デッドコード）。
- debug モードでないとコールスタックを取得できない

## 案 B：組み込みキーワード（統一方案により廃止）

> 廃止済み。案 A と B の対立は dispatch 振り分けパイプラインにより解消された — assert は Assert の値導入子であり、コンパイル時既知なら証明パイプライン（ランタイムオーバーヘッドゼロ）、ランタイムなら check。「関数」と「キーワード」の二者択一は不要。以下は歴史的記録である。

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### 型シグネチャ

独立した型シグネチャはなし — キーワードは parser が処理する。

### ランタイム挙動

案 A と同じ。

### コンパイラ変更点

parser、AST、typecheck、IR gen の変更が必要：
1. parser：`Expr::Assert` 変体を新設
2. AST：`Expr::Assert` ノードを新設
3. typecheck：引数の型を検証
4. IR gen：`BytecodeInstr::Throw` を生成

### 利点

- コンパイル時にソース位置を認識（debug 情報に依存しない）
- コンパイル時の定数畳み込みが可能：`assert(true)` → 空操作、`assert(false)` → コンパイルエラー

### 欠点

| 欠点 | 影響 |
|------|------|
| パーサー変更が必要 | 新規構文ノードの導入で保守コスト増 |
| キーワードは拡張不可 | `assert_eq` などの変種も結局関数が必要 |
| コンパイル時の利点は実際的でない | 後述の分析を参照 |

### 比較

| 観点 | 案 A（関数） | 案 B（キーワード） |
|------|---------------|-----------------|
| 実装コスト | 約 20 行 | parser + AST + typecheck + IR gen |
| 構文変更 | なし | 新規キーワード |
| 拡張性 | 関数のオーバーロード | 補助マクロが必要 |
| ソース位置 | debug 情報 | コンパイル時取得可 |
| 定数畳み込み | pass のサポートが必要 | コンパイル時取得可 |
| ランタイムオーバーヘッド | 関数呼び出し | 極小 |

### コンパイル時分析の現実的制約

案 B の核心的利点 — コンパイル時分析 — は **定数畳み込み pass** がなければ機能しない。すなわち、コンパイラがコンパイル時に `assert(false)` 中の `false` を評価できて初めて、デッドコードと認識できる。

YaoXiang には現状、定数畳み込み pass がない。仮に案 B を採用しても、`assert(x > 0)` のような一般的な書き方ではコンパイル時分析できない。`assert(true)` / `assert(false)` のようなリテラルのみが分析可能である。

したがって案 B のコンパイル時の利点は**現段階では理論的であり、実際的でない**。

---

## 未解決問題

- [x] ~~案 A と案 B のどちらを選ぶか？~~ → **統一方案：assert は Assert の値導入子**。案 A と B の対立は dispatch 振り分けパイプラインにより解消 — コンパイル時既知なら証明パイプライン、ランタイムなら check。「二者択一」は不要。
- [x] ~~`assert` は `message` を伴わない簡略形 `assert(cond)` をサポートすべきか？~~ → **サポートする。`assert(cond, ?msg)` で message は任意。**
- [x] ~~`assert_eq`、`assert_ne` などの変種は要るか？~~ → **不要。YAGNI。テストフレームワークの成熟を待って判断。**
- [x] ~~panic 出力にソース位置情報を含めるか？~~ → 案 A では debug 情報（コールスタック）に依存。
- [x] ~~assert / Assert 統一問題~~ → **確定済み**。統一方案：`assert: (Bool) -> Assert(IsTrue(cond))`、表裏一体、dispatch 自動振り分け。詳細は [#156](https://github.com/ChenXu233/YaoXiang/issues/156)（クローズ済み）参照。`Never` 型（⊥）を `assert(false)` の返り値型として組み込む。

### 2026-07-05：案 A を選択（統一方案により廃止）

案 A の 20 行実装が価値とコストの観点で勝った。2026-07-12 に統一方案が確定した後、案 A/B の対立は dispatch 振り分けパイプラインにより解消され — assert は Assert の値導入子であり、もはや「関数」と「キーワード」の二者択一ではない。

### 2026-07-12：統一方案確定（2026-07-11 の「完全独立」結論を廃止）

**結論**：`assert` と `Assert` は 2 つの独立した機構ではない。`assert: (Bool) -> Assert(IsTrue(cond))` — dispatch により自動振り分け：

- コンパイル時既知 → 証明パイプラインへ（Proved は消去 / Disproved はエラー / Unknown は証明要求）
- ランタイム入力 → check 挿入 + Γ 仮定の注入

**モジュール構成**：`std.assert` がランタイムアサーション（`assert`）とコンパイル時精化型（`Assert`、`IsTrue`）を統一して担う。「別々に実装」するのではなく、同じプリミティブの二面として扱う。

### 2026-07-11：assert オーバーロード設計

**問題**：なぜ `assert` は統一された `(Bool, ?String)` ではなく、2 つのオーバーロードを要するのか？

**解答**：

ランタイム `assert()` は YaoXiang における唯一のユーザーランド panic 機構である。`assert(false, "msg")` は他言語の `raise`/`throw` と等価である。したがって以下 3 つのシナリオをカバーする必要がある：
1. 条件 + 簡易メッセージ：`assert(cond, "msg")`
2. 条件 + カスタム Error：`assert(cond, my_error)`
3. Result 検査：`assert(result)` — `if is_err { panic }` の最簡形式

Result オーバーロードの妥当性：これはエラー伝播の最短経路である — 「Result は Ok であるべき、さもなくば死ぬ」。先に `.is_ok()` して別途エラーを処理する必要はない。

## 付録 B：設計決定ログ

| 決定事項 | 決定内容 | 日付 | 記録者 |
|------|------|------|--------|
| 案 A と案 B の選択 | **統一方案**：dispatch 振り分けパイプラインが A/B の対立を解消、assert は Assert の値導入子 | 2026-07-12 | 晨煦 |
| message を任意にするか | **はい**：`assert(cond, ?msg)`、String または Error | 2026-07-11 | 晨煦 |
| assert_eq などの変種を要するか | **不要**。YAGNI、テストフレームワークの成熟を待つ | 2026-07-11 | 晨煦 |
| 専用の raise/throw キーワードを要するか | **不要**。`assert(false, msg)` が raise と等価 | 2026-07-11 | 晨煦 |
| assert と Assert の関係 | **表裏一体**。`assert: (Bool) -> Assert(IsTrue(cond))`、dispatch 自動振り分け | 2026-07-12 | 晨煦 |

## 参考文献

- [RFC-007: 関数定義構文統一方案](007-function-syntax-unification.md) — `name: type = value` モデル
- [RFC-010: 統一型構文](010-unified-type-syntax.md) — 型システム基礎
- [RFC-011: ジェネリック型システム設計 §4.3](../accepted/011-generic-type-system.md) — コンパイル時検証と `Assert(C)` 条件型
- [RFC-026: FFI 中核機構](../review/026-ffi-core-mechanism.md) — native 関数登録機構
- [RFC-027: コンパイル時述語と統一静的検証](../accepted/027-compile-time-evaluation-types.md) — コンパイル時評価システム