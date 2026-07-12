---
title: "RFC-030: assert アサーション機構"
status: "審査中"
author: "晨煦"
created: "2026-06-15"
updated: "2026-07-11"
decision: "Assert と assert コンパイル時アサーションとランタイムアサーション。"
issue: "#97"
issues_impl:
  - "#155"
---

# RFC-030: assert アサーション機構

## 概要

YaoXiang に `assert` アサーション機構を導入し、テスト、前置条件チェック、ランタイム panic に使用する。`assert` およびコンパイル時精錬型 `Assert(C)`（RFC-011 §4.3 参照）は、**同じ精錬プリミティブの二面であり**——「述語の自由変数がコンパイル時に到達可能か否か」に基づき dispatch によって自動的にコンパイル時証明またはランタイムチェックへ振り分けられる。`assert(false, "msg")` は `raise` と等価であり、個別の `throw`/`raise` キーワードを必要としない。

## 動機

### なぜこの機能が必要なのか？

現在の YaoXiang の E2E テストは `if` + `io.println` + `return` でアサーションを模倣するしかない：

```yaoxiang
val = some_func()
if val != 42 {
    io.println("FAIL: expected 42")
    return
}
```

この記述方法には 3 つの問題がある：

1. **ボイラープレートが多い**：各アサーションに 4 行必要で、テストファイルが肥大化する
2. **エラーメッセージが弱い**：文字列を手動で連結し、ソースコードの位置情報がない
3. **合成不可能**：アサーションを一括登録できず、テストフレームワークに引数として渡せない

### 現状の問題

- 統一されたアサーション機構がない
- テストコードに `if` + プリント + `return` のパターンが氾濫している
- バイトコード層には既に `Throw` 命令が存在するが、言語層では公開されていない
- RFC-011 でコンパイル時 `Assert(C)` 条件型が定義されているが、ランタイム `assert()` は未実装である

### 設計原則

`assert` は YaoXiang における唯一のユーザー空間 panic 機構である。`assert(false, "msg")` は `raise` と等価であり、個別の `throw`/`raise` キーワードを必要としない。`assert` 関数自体が `if raise` の最良のラッパーである。

**新しいキーワードを導入せず、新しい構文を導入しない。すべては関数呼び出しである。**

## プラン A：native 関数

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
//                                       戻り値は精錬型であり、() ではない
//
// IsTrue: Bool -> Type は真値から型への橋渡し：
//   IsTrue(true)  = Void   (⊤，プログラム続行)
//   IsTrue(false) = Never  (⊥，発散/コンパイルエラー)
```

`assert` の実際の振る舞いは dispatch の振り分けによって決定される：
- すべての自由変数がコンパイル時に既知 → **CompileTime**：コンパイラが cond を評価し、true → Void に消去、false → コンパイルエラー（Never は居住不可能）
- ランタイムの自由変数が存在 → **Runtime**：check を挿入し、フロー依存の仮定集合 Γ に精錬事実を注入する

オプションのメッセージ `?msg` および Result オーバーロード（後述）はランタイムの raise ペイロードとして保持される。

#### オーバーロード 1：条件アサーション `(Bool, ?String | Error)`

`Bool` + オプションのメッセージ。メッセージは `String` または `Error` 値：

```yaoxiang
assert(1 + 1 == 2)                    // メッセージなし、デフォルトの panic 情報
assert(1 + 1 == 2, "math is broken")   // 文字列メッセージ
assert(x > 0, my_error)                // Error 値を直接スロー
```

`assert(false, "msg")` は YaoXiang の `raise`/`throw` 等価体であり——個別のキーワードを必要としない。

#### オーバーロード 2：Result アサーション `(Result)`

単一の `Result` 引数で、自動的に `Err` か否かをチェックする：

### 利点

- **構文変更ゼロ**：純粋な関数であり、新しいキーワードを必要としない
- **新概念ゼロ**：既存の native 関数登録機構を再利用
- **高い拡張性**：関数のオーバーロードが自然に複数シグネチャをサポート
- **自己文書化**：`std.assert` 名前空間自体がドキュメント

### 欠点

- なし。`assert` の型シグネチャが正しければ、コンパイラは関数の到達可能性解析によりデッドコードを推論できる。追加の pass は不要。

### ランタイムの振る舞い

1. 第 1 引数 `condition: Bool` を評価
2. `true` の場合、`Unit` を返す
3. `false` の場合、ランタイム panic を発火：
   - `message` の内容を出力（あれば）
   - コールスタックを出力（debug モード時）
   - 現在の実行を終了

#### 各オーバーロードの失敗時の振る舞い

| シグネチャ | 失敗時の振る舞い |
|------|-----------|
| `assert(false)` | デフォルトの panic 情報 |
| `assert(false, "msg")` | 文字列メッセージを出力後 panic |
| `assert(false, error_val)` | Error 値をスロー |
| `assert(Err(x))` | Err 内容を抽出して panic |

### コンパイル時 Assert との関係

`assert` と `Assert` は**同じ精錬プリミティブの二面であり**——「述語の自由変数がコンパイル時に到達可能か否か」に基づき dispatch 振り分けパイプラインによって自動的に選択される：

| 条件 | 振り分け | 振る舞い |
|------|------|------|
| すべての自由変数がコンパイル時に既知 | CompileTime → 証明パイプライン | Proved → 消去、Disproved → コンパイルエラー、Unknown → 証明を要求 |
| ランタイムの自由変数が存在 | Runtime → check を挿入 | Bool チェック + フロー依存仮定集合 Γ への精錬事実の注入 |

```yaoxiang
use std.assert

# コンパイル時に既知（generics パラメータ）—— CompileTime へ、ランタイムオーバーヘッドゼロ
Array: (T: Type, N: Int) -> Type = {
    data: Array(T, N),
    length: assert.Assert(N > 0),   # N は generics パラメータ、コンパイル時に評価
}

# ランタイム値 —— Runtime へ、Bool チェックを挿入
x = read_int()
assert.assert(x > 0, "expected positive")  # ランタイム check
```

> **2026-07-12 統一プラン**：「完全に独立」という以前の結論は置き換えられた。`assert()` は `Assert` の値導入子であり、dispatch が自動的に振り分ける。

### コンパイラの改変

**parser、AST、typecheck、IR gen の改変は不要。**

`src/std/` 下に native 関数の登録を追加するだけ：

1. `src/std/assert.rs` を新規追加
2. `std.assert.assert` と `std.assert.Assert`（後者はコンパイル時条件型、#155 参照）を登録
3. 内部的には既存の `BytecodeInstr::Throw` 命令を呼び出す

### 利点

- **構文変更ゼロ**：純粋な関数であり、新しいキーワードを必要としない
- **新概念ゼロ**：既存の native 関数登録機構を再利用
- **高い拡張性**：関数シグネチャは `assert_eq` などの変種に拡張可能（将来）
- **自己文書化**：`std.assert` 名前空間自体がドキュメント

### 欠点

- ~~コンパイル時不可知：プラン B（キーワード）と異なり、コンパイル時にデッドコード除去を行えない~~ → **統一プラン下では成立しない**。CompileTime モードの assert は証明パイプラインを通り、コンパイル時に既知の cond → 消去またはコンパイルエラー（`assert(false)` → Never → デッドコード）。
- debug モードでのみコールスタックを取得可能

## プラン B：組み込みキーワード（統一プランにより置き換え済み）

> 棄却済み。プラン A と B の対立は dispatch 振り分けパイプラインにより解消された——assert は Assert の値導入子であり、コンパイル時に既知の場合は証明パイプライン（ランタイムオーバーヘッドゼロ）を、ランタイムの場合は check を通る。「関数」と「キーワード」の二者択一は不要。以下は歴史的記録である。

```yaoxiang
assert(1 + 1 == 2, "math is broken")
```

### 型シグネチャ

独立した型シグネチャなし——キーワードは parser が処理する。

### ランタイムの振る舞い

プラン A と同じ。

### コンパイラの改変

parser、AST、typecheck、IR gen の改変が必要：
1. parser：`Expr::Assert` バリアントを新規追加
2. AST：`Expr::Assert` ノードを新規追加
3. typecheck：引数の型を検証
4. IR gen：`BytecodeInstr::Throw` を生成

### 利点

- コンパイル時にソースコード位置を取得可能（debug info に依存しない）
- コンパイル時に定数畳み込み可能：`assert(true)` → 空操作、`assert(false)` → コンパイルエラー

### 欠点

| 欠点 | 影響 |
|------|------|
| パーサの改変が必要 | 新しい構文ノードを導入し、保守コストが増大 |
| キーワードは拡張不可 | `assert_eq` などの変種は依然として関数を必要とする |
| コンパイル時の利点は非実用的 | 後述の解析を参照 |

### 比較

| 次元 | プラン A（関数） | プラン B（キーワード） |
|------|---------------|-----------------|
| 実装コスト | 約 20 行 | parser + AST + typecheck + IR gen |
| 構文変更 | なし | 新キーワード |
| 拡張性 | 関数のオーバーロード | 付随するマクロが必要 |
| ソースコード位置 | debug info | コンパイル時に取得可能 |
| 定数畳み込み | pass のサポートが必要 | コンパイル時に取得可能 |
| ランタイムオーバーヘッド | 関数呼び出し | 極小 |

### コンパイル時解析の現実的制約

プラン B の核心的利点——コンパイル時解析——は **定数畳み込み pass** によって初めて有効になる。すなわち、コンパイラがコンパイル時に `assert(false)` の `false` を評価して初めて、これがデッドコードであることがわかる。

YaoXiang には現在、定数畳み込み pass が存在しない。仮にプラン B を採用したとしても、`assert(x > 0)` のような一般的な記述はコンパイル時に解析不可能である。`assert(true)` / `assert(false)` のようなリテラルのみが解析可能である。

したがってプラン B のコンパイル時の利点は**現段階では理論的であり、実用的ではない**。

---

## オープンな問題

- [x] ~~プラン A とプラン B のどちらを選択するか？~~ → **統一プラン：assert は Assert の値導入子**。プラン A と B の対立は dispatch 振り分けパイプラインにより解消される——コンパイル時に既知の場合は証明パイプラインを、ランタイムの場合は check を通る。「二者択一」は不要。
- [x] ~~`assert` は `message` を伴わない簡略形 `assert(cond)` をサポートする必要があるか？~~ → **サポートする。`assert(cond, ?msg)`、message はオプション。**
- [x] ~~`assert_eq`、`assert_ne` などの変種は必要か？~~ → **不要。YAGNI（You Aren't Gonna Need It）。テストフレームワークが形になってから議論する。**
- [x] ~~panic 出力にソースコード位置を含めるか？~~ → プラン A は debug info（コールスタック）に依存する。
- [x] ~~assert / Assert の統一問題~~ → **確定済み**。統一プラン：`assert: (Bool) -> Assert(IsTrue(cond))`、一体二面、dispatch 自動振り分け。詳細は [#156](https://github.com/ChenXu233/YaoXiang/issues/156)（クローズ済み）を参照。`Never` 型（⊥）を `assert(false)` の戻り型として組み込む。

### 2026-07-05：プラン A を選択（統一プランにより置き換え済み）

プラン A の 20 行実装が価値とコストの双方で勝る。2026-07-12 に統一プランが確定した後、プラン A/B の対立は dispatch 振り分けパイプラインにより解消された——assert は Assert の値導入子であり、もはや「関数」と「キーワード」の二者択一ではない。

### 2026-07-12：統一プラン確定（2026-07-11 の「完全に独立」という結論を置き換え）

**結論**：`assert` と `Assert` は 2 つの独立した機構ではない。`assert: (Bool) -> Assert(IsTrue(cond))` ——dispatch が自動的に振り分ける：

- コンパイル時に既知 → 証明パイプラインへ（Proved 消去 / Disproved エラー / Unknown は証明を要求）
- ランタイム入力 → check を挿入 + Γ 仮定を注入

**モジュール構造**：`std.assert` がランタイムアサーション（`assert`）とコンパイル時精錬型（`Assert`、`IsTrue`）を統一的に担う。「別々に実装」するのではなく、同じプリミティブの二面である。

### 2026-07-11：assert オーバーロード設計

**問題**：なぜ `assert` は統一された `(Bool, ?String)` ではなく、2 つのオーバーロードを必要とするのか？

**解答**：

ランタイム `assert()` は YaoXiang 唯一のユーザー空間 panic 機構である。`assert(false, "msg")` は他言語の `raise`/`throw` と等価である。したがって 3 つのシナリオをカバーする必要がある：
1. 条件 + 簡単なメッセージ：`assert(cond, "msg")`
2. 条件 + カスタム Error：`assert(cond, my_error)`
3. Result チェック：`assert(result)` — `if is_err { panic }` の最も簡潔な形

Result オーバーロードの妥当性：これはエラー伝播の最短経路だからである——「Result は Ok であるべき、さもなくば死」。先に `.is_ok()` を呼び、エラーを別途処理する必要はない。

## 付録 B：設計決定の記録

| 決定 | 結論 | 日付 | 記録者 |
|------|------|------|--------|
| プラン A とプラン B の選択 | **統一プラン**：dispatch 振り分けパイプラインが A/B の対立を解消、assert は Assert の値導入子 | 2026-07-12 | 晨煦 |
| message がオプションか | **はい**：`assert(cond, ?msg)`、String または Error | 2026-07-11 | 晨煦 |
| assert_eq などの変種の必要性 | **不要**。YAGNI、テストフレームワークが形になってから議論 | 2026-07-11 | 晨煦 |
| 個別の raise/throw キーワードの必要性 | **不要**。`assert(false, msg)` は raise と等価 | 2026-07-11 | 晨煦 |
| assert と Assert の関係 | **一体二面**。`assert: (Bool) -> Assert(IsTrue(cond))`、dispatch 自動振り分け | 2026-07-12 | 晨煦 |

## 参考文献

- [RFC-007: 関数定義構文の統一プラン](007-function-syntax-unification.md) — `name: type = value` モデル
- [RFC-010: 統一型構文](010-unified-type-syntax.md) — 型システム基礎
- [RFC-011: generics システム設計 §4.3](../accepted/011-generic-type-system.md) — コンパイル時検証と `Assert(C)` 条件型
- [RFC-026: FFI 中核機構](../review/026-ffi-core-mechanism.md) — native 関数登録機構
- [RFC-027: コンパイル時述語と統一静的検証](../accepted/027-compile-time-evaluation-types.md) — コンパイル時評価システム