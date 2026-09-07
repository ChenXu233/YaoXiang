---
title: 'RFC-030: assert アサート機構'
status: 'Accepted'
author: '晨煦'
created: '2026-06-15'
updated: '2026-07-14'
decision:
  'assert と Assert は表裏一体であり、dispatch が自動分派する。6 Phase すべて実装完了（#157-#162
  クローズ済み）。std.assert モジュールに統一登録（#169 クローズ済み）、 assert native 関数 +
  Assert/IsTrue 型族は同一経路。'
issue: '#97'
issues_impl:
  - '#155'
  - '#157'
  - '#158'
  - '#159'
  - '#160'
  - '#161'
  - '#162'
  - '#169'
---

# RFC-030: assert アサート機構

## 要約

YaoXiang に `assert` アサート機構を導入し、テスト・事前条件チェック・runtime
panicに使用する。`assert` と compile-time 精緻化型 `Assert(C)`（RFC-011 §4.3 参照）は
**同じ精緻化プリミティブの二面であり**——「述語の自由変数が compile-time に到達可能か」に基づき dispatch が自動的に compile-time 証明と runtime チェックへ分派する。
`assert(false, "msg")` は `raise` と等価であり、個別の `throw`/`raise` キーワードは不要。

## 動機

### なぜこの機能が必要か

現在の YaoXiang の E2E テストは `if` + `io.println` + `return` でアサートを模倣するしかない：

```yaoxiang
val = some_func()
if val != 42 {
    io.println("FAIL: expected 42")
    return
}
```

この記述には 3 つの問題がある：

1. **ボイラープレートが多い**：アサートごとに 4 行必要で、テストファイルが肥大化する
2. **エラーメッセージが弱い**：文字列を手動で連結し、ソースコードの位置情報がない
3. **合成できない**：アサートを一括登録できず、テストフレームワークへ引数として渡せない

### 現状の問題

- 統一されたアサート機構がない
- テストコードに `if` + 出力 + `return` のパターンが氾濫する
- バイトコード層には既に `Throw` 命令があるが、言語層では公開されていない
- RFC-011 で compile-time `Assert(C)` conditional type は定義されているが、runtime `assert()`
  は未実装

### 設計原則

`assert` は YaoXiang で唯一のユーザーランド panic 機構である。`assert(false, "msg")` は `raise`
と等価であり、個別の `throw`/`raise` キーワードは不要。`assert` 関数自体が `if raise`
の最適なカプセル化である。

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

### オーバーロードシグネチャ

`assert` には 2 つのオーバーロードがある：

```
// 中核シグネチャ：assert は Assert の値宇宙導入子
assert: (cond: Bool, ?msg: String | Error) -> Assert(IsTrue(cond))
//                                       ^^^^^^^^^^^^^^^^^^^^^^^^
//                                       戻り値は精緻化型、() ではない
//
// IsTrue: Bool -> Type は真理値から型への橋渡し：
//   IsTrue(true)  = Void   (⊤、プログラム続行)
//   IsTrue(false) = Never  (⊥、発散／コンパイルエラー)
```

`assert` の実際の挙動は dispatch 分派によって決定される：

- すべての自由変数が compile-time 既知 → **CompileTime**：コンパイラが cond を評価、true →
  Void として消去、false → コンパイルエラー（Never は居住不能）
- runtime 自由変数が存在 → **Runtime**：check を挿入し、フロー敏感仮定集合 Γ に精緻化事実を注入

オプションメッセージ `?msg` および Result オーバーロード（後述）は runtime
raise ペイロードとして保持する。

#### オーバーロード 1：条件アサート `(Bool, ?String | Error)`

`Bool` + オプションメッセージ。メッセージは `String` または `Error` 値：

```yaoxiang
assert(1 + 1 == 2)                    // メッセージなし、デフォルト panic メッセージ
assert(1 + 1 == 2, "math is broken")  // 文字列メッセージ
assert(x > 0, my_error)               // Error 値を直接スロー
```

`assert(false, "msg")` は YaoXiang の `raise`/`throw` 等価体である——個別キーワードは不要。

#### オーバーロード 2：Result アサート `(Result)`

単一の `Result` 引数を取り、`Err` かどうかを自動チェックする：

### 利点

- **構文変更ゼロ**：純関数であり、新しいキーワード不要
- **新概念ゼロ**：既存の native 関数登録機構を再利用
- **高い拡張性**：関数オーバーロードにより複数シグネチャに自然対応
- **自己文書化**：`std.assert` 名前空間自体がドキュメント

### 欠点

- なし。`assert`
  の型シグネチャが正しければ、コンパイラは関数の到達可能性解析によりデッドコードを推論可能。追加 pass は不要。

### ランタイム挙動

1. 第 1 引数 `condition: Bool` を評価
2. `true` なら `Unit` を返す
3. `false` なら runtime panic を発動：
   - `message` の内容を出力（あれば）
   - コールスタックを出力（デバッグモード時）
   - 現在の実行を終了

#### 各オーバーロードの失敗時挙動

| シグネチャ                 | 失敗時挙動                     |
| -------------------------- | ------------------------------ |
| `assert(false)`            | デフォルト panic メッセージ    |
| `assert(false, "msg")`     | 文字列メッセージ出力後に panic |
| `assert(false, error_val)` | Error 値をスロー               |
| `assert(Err(x))`           | Err 内容を抽出して panic       |

### compile-time Assert との関係

`assert` と `Assert` は
**同じ精緻化プリミティブの二面であり**——「述語の自由変数が compile-time に到達可能か」に基づき dispatch 分派パイプラインが自動的に選択する：

| 条件                                 | 分派                           | 挙動                                                            |
| ------------------------------------ | ------------------------------ | --------------------------------------------------------------- |
| すべての自由変数が compile-time 既知 | CompileTime → 証明パイプライン | Proved → 消去、Disproved → コンパイルエラー、Unknown → 証明要求 |
| runtime 自由変数が存在               | Runtime → check 挿入           | Bool チェック + フロー敏感仮定集合 Γ への精緻化事実注入         |

```yaoxiang
use std.assert

# compile-time 既知（generics 引数）—— CompileTime 経路、runtime オーバーヘッドゼロ
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N は generics 引数、compile-time 評価
}

# runtime 値 —— Runtime 経路、Bool チェックを挿入
x = read_int()
assert.assert(x > 0, "expected positive")  # runtime check
```

> **2026-07-12 統一案**：従来の「完全独立」という結論は置き換えられた。`assert()` は `Assert`
> の値導入子であり、dispatch が自動分派する。

### コンパイラ変更

**parser、AST、typecheck、IR gen の変更は不要。**

`src/std/` 配下に native 関数を登録するだけでよい：

1. `src/std/assert.rs` を新規追加
2. `std.assert.assert` と `std.assert.Assert`（後者は compile-time conditional type）を登録
3. 内部で既存の `BytecodeInstr::Throw` 命令を呼び出す

### 利点

- **構文変更ゼロ**：純関数であり、新しいキーワード不要
- **新概念ゼロ**：既存の native 関数登録機構を再利用
- **高い拡張性**：`assert_eq` など将来的な変種へ関数シグネチャを拡張可能
- **自己文書化**：`std.assert` 名前空間自体がドキュメント

### 欠点

- ~~compile-time 非到達：案 B（キーワード）と異なり、compile-time でのデッドコード除去不可~~ →
  **統一案により既に成立しない**。CompileTime モードの assert は証明パイプラインを通り、compile-time 既知の cond
  → 消去またはコンパイルエラー（`assert(false)` → Never →デッドコード）。
- デバッグモードでのみコールスタックを取得可能

## 案 B：組み込みキーワード（統一案により置き換え済み）

> 棄却済み。案 A と B の対立は dispatch 分派パイプラインにより解消された——assert は Assert の値導入子であり、compile-time 既知なら証明パイプライン（runtime オーバーヘッドゼロ）、runtime なら check。「関数」と「キーワード」の二者択一は不要。以下は歴史的記録。

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### 型シグネチャ

独立した型シグネチャなし——キーワードは parser が処理する。

### ランタイム挙動

案 A と同じ。

### コンパイラ変更

parser、AST、typecheck、IR gen の変更が必要：

1. parser：`Expr::Assert` 変種を新規追加
2. AST：`Expr::Assert` ノードを新規追加
3. typecheck：引数の型を検証
4. IR gen：`BytecodeInstr::Throw` を生成

### 利点

- compile-time にソースコード位置を取得可能（デバッグ情報に依存しない）
- compile-time に定数畳み込み可能：`assert(true)` → 空操作、`assert(false)` → コンパイルエラー

### 欠点

| 欠点                            | 影響                                       |
| ------------------------------- | ------------------------------------------ |
| パーサー変更が必要              | 新規構文ノードを導入し、保守コストが増大   |
| キーワードは拡張不可            | `assert_eq` 等の変種も依然として関数が必要 |
| compile-time 上の利点は非実用的 | 後述の解析を参照                           |

### 比較

| 観点                   | 案 A（関数）       | 案 B（キーワード）                |
| ---------------------- | ------------------ | --------------------------------- |
| 実装コスト             | 約 20 行           | parser + AST + typecheck + IR gen |
| 構文変更               | なし               | 新規キーワード                    |
| 拡張性                 | 関数オーバーロード | 補助マクロが必要                  |
| ソースコード位置       | デバッグ情報       | compile-time に取得可能           |
| 定数畳み込み           | pass 支援が必要    | compile-time に取得可能           |
| runtime オーバーヘッド | 関数呼び出し       | 極小                              |

### compile-time 解析の現実的制約

案 B の中心的利点——compile-time 解析——は **定数畳み込み pass**
がなければ機能しない。すなわち、コンパイラは `assert(false)` 中の `false`
を compile-time に評価して、これがデッドコードであることを認識する必要がある。

YaoXiang には現在、定数畳み込み pass がない。仮に案 B を採用しても、`assert(x > 0)`
のような一般的な記述は compile-time に解析できない。解析できるのは `assert(true)` / `assert(false)`
のような literal のみである。

したがって案 B の compile-time 上の利点は **現時点では理論上のものであり、実用的ではない**。

---

## 未解決問題

- [x] ~~案 A か案 B か？~~ →
      **統一案：assert は Assert の値導入子**。案 A/B の対立は dispatch 分派パイプラインにより解消——compile-time 既知なら証明パイプライン、runtime なら check。「二者択一」は不要。
- [x] ~~`assert` は `message` を伴わない簡略形 `assert(cond)` をサポートすべきか？~~ →
      **サポートする。`assert(cond, ?msg)`、message は任意。**
- [x] ~~`assert_eq`、`assert_ne` 等の変種は必要か？~~ →
      **不要。YAGNI。テストフレームワーク確立後に再検討。**
- [x] ~~panic 出力にソースコード位置を含めるか？~~ → 案 A はデバッグ情報（コールスタック）に依存。
- [x] ~~assert / Assert 統一問題~~ →
      **確定済み**。統一案：`assert: (Bool) -> Assert(IsTrue(cond))`、表裏一体、dispatch 自動分派。`Never`
      型（⊥）を `assert(false)` の戻り型として組み込む。

### 2026-07-05：案 A の選択（統一案により置き換え済み）

案 A の 20 行実装が価値とコストの観点で優位。2026-07-12 に統一案が確定した後、案 A/B の対立は dispatch 分派パイプラインにより解消された——assert は Assert の値導入子であり、もはや「関数」と「キーワード」の二者択一は不要。

### 2026-07-12：統一案確定（2026-07-11 の「完全独立」結論を置き換え）

**結論**：`assert` と `Assert`
は 2 つの独立した機構ではない。`assert: (Bool) -> Assert(IsTrue(cond))`—— dispatch が自動分派する：

- compile-time 既知 → 証明パイプラインへ（Proved 消去 / Disproved エラー / Unknown 証明要求）
- runtime 入力 → check 挿入 + Γ 仮定注入

**モジュール構造**：`std.assert`
が runtime アサート（`assert`）と compile-time 精緻化型（`Assert`、`IsTrue`）を統一的に担う。「別実装」ではなく、同一プリミティブの二面である。

### 2026-07-11：assert オーバーロード設計

**問題**：なぜ `assert` は 2 つのオーバーロードを要し、統一 `(Bool, ?String)` ではダメなのか？

**解答**：

runtime `assert()` は YaoXiang で唯一のユーザーランド panic 機構である。`assert(false, "msg")`
は他言語の `raise`/`throw` と等価。したがって以下 3 つのシナリオをカバーする必要がある：

1. 条件 + 簡易メッセージ：`assert(cond, "msg")`
2. 条件 + カスタム Error：`assert(cond, my_error)`
3. Result チェック：`assert(result)` — `if is_err { panic }` の最も簡潔な形

Result オーバーロードの妥当性：これはエラー伝播の最短経路——「Result は Ok であるべき、さもなくば死」。
`.is_ok()` を呼んでから別途エラーを処理する必要がない。

## 付録 B：設計意思決定記録

| 意思決定                              | 決定                                                                                | 日付       | 記録者 |
| ------------------------------------- | ----------------------------------------------------------------------------------- | ---------- | ------ |
| 案 A か案 B か                        | **統一案**：dispatch 分派パイプラインが A/B 対立を解消、assert は Assert の値導入子 | 2026-07-12 | 晨煦   |
| message の任意性                      | **可**：`assert(cond, ?msg)`、String または Error                                   | 2026-07-11 | 晨煦   |
| assert_eq 等の変種の必要性            | **不要**。YAGNI、テストフレームワーク確立後に再検討                                 | 2026-07-11 | 晨煦   |
| 個別の raise/throw キーワードの必要性 | **不要**。`assert(false, msg)` が raise と等価                                      | 2026-07-11 | 晨煦   |
| assert と Assert の関係               | **表裏一体**。`assert: (Bool) -> Assert(IsTrue(cond))`、dispatch 自動分派           | 2026-07-12 | 晨煦   |

## 参考文献

- [RFC-007: 関数定義構文統一案](007-function-syntax-unification.md) — `name: type = value` モデル
- [RFC-010: 統一型構文](010-unified-type-syntax.md) — 型システム基礎
- [RFC-011: generics システム設計 §4.3](../accepted/011-generic-type-system.md) —
  compile-time 検証と `Assert(C)` conditional type
- [RFC-026: FFI 中核機構](026-ffi-core-mechanism.md) — native 関数登録機構
- [RFC-027: compile-time 述語と統一静的検証](../accepted/027-compile-time-evaluation-types.md) —
  compile-time 評価システム
