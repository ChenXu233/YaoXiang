```markdown
---
title: "RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの除去"
status: "レビュー中"
author: "晨煦"
created: "2026-06-16"
updated: "2026-07-03"
---

# RFC-032: spawn 統一式修飾

> **本文書は `spawn` の文法、AST/IR 再構築、型システムの拡張を定義する**。
> 実行時の動作意味論（タスク分解の粒度、所有権、スコープ、エラー伝播、リソース型、ネスト）については [RFC-024: spawn ベースの並行実行意味論](./024-concurrency-model.md) を参照。
>
> 二つの RFC は協調して `spawn` を定義する。024 は「何をするか」、032 は「どう表現するか」を答える。

> **核心となる洞察**：`spawn` は `{}` ブロックのみを修飾すべきではない。**任意の式**を修飾できる。`spawn for` は特殊な構文ではなく、`spawn` + `for` 式の自然な組み合わせである。

## 概要

`spawn` を `spawn { }`（ブロックのみ修飾）から `spawn <expr>`（任意の式を修飾）に拡張する。`Expr::SpawnFor` は AST から削除され、`Expr::Spawn { body: Expr::For { .. } }` によって自然に置き換えられる。式の構造に相当する型（Block、For、While、If など）が新しい `MonoType` バリアントとして型システムに入り、`Spawn<T>` は並行実行される計算構造をラップする。コンパイル時にマークされ、型チェック後に消去される。

## 動機

### なぜこの変更が必要なのか

現在の `spawn for x in items { body }` は独立したキーワードの組み合わせであり、AST には `Expr::SpawnFor` という専用の表現がある。これは言語の直交性を破壊している。

1. **文法の不統一**：`spawn` は `{}` ブロックのみを修飾でき、`spawn for` はハードコードされた例外である
2. **直交性の欠如**：`spawn while`、`spawn if` などの組み合わせを自然に表現できない
3. **型システムの不完全さ**：spawn が型システムに見えず、並行構造を型リフレクションで取得できない

### 現在の問題

```rust
// AST における二つの spawn バリアント
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }

// MonoType には値型しかなく、計算構造型がない
// spawn { a, b } の型 = Tuple(T_a, T_b)  ← 「これは spawn である」という情報が失われる
// spawn for    の型 = List(T)             ← 「これはデータ並列である」という情報が失われる
```

## 提案

### 核心となる設計

`spawn <expr>`：`spawn` は任意の式を修飾する。式の形状が DAG のタスク分解方法を決定する。

**すべてが型である**：`MonoType` を「値型」から「値型 + 計算構造型」に拡張する。重要な式構造はそれぞれ型システムに対応するバリアントを持つ。`Spawn<T>` は並行実行される計算構造をラップする。

### ユーザーのメンタルモデル

`spawn` = 「この式を並行実行に持ち込む」。式の形状が分解方法を決定する。

| 式の形状 | 並行動作 | 型 |
|---------|---------|------|
| `spawn { a, b, c }` | `a`、`b`、`c` が独立して並列実行 | `Spawn(Block(Tuple(T_a, T_b, T_c)))` |
| `spawn for x in items { f(x) }` | N 個のイテレーションが独立して並列実行 | `Spawn(ForExpr { body_ty: List(T) })` |
| `spawn while cond { step() }` | 各イテレーションが独立タスク | `Spawn(WhileExpr { body_ty: List(T) })` |
| `spawn if c { a } else { b }` | 選択された分岐全体が spawn ドメイン | `Spawn(IfExpr { then_ty: T_a, else_ty: Some(T_b) })` |
| `spawn call(x)` | 呼び出し自体がタスクになる | `Spawn(Call { fn_ty: Fn(A→R), result_ty: R })` |
| `spawn 42` | 単一のタスク | `Spawn(Int)` |

コンパイラは DAG 分析で依存関係を決定し、ランタイムは GMP モデルでスケジューリングする。依存関係のないタスクはワークキューに投入され、worker が奪い合う。全体は同期的にブロックし、全タスクの完了を待つ。

**Go との違い**：Go の `go` は「投げ出して知らん顔」だが、YaoXiang の `spawn` は「分解して並列実行し、全部終わってから先に進む」。

### 制御フローの直交性

| 組み合わせ | 意味 | 差異 |
|------|------|------|
| `spawn for x in items { body }` | データ並列：各イテレーション = 独立タスク | DAG はイテレーションを跨いで依存関係を分析 |
| `for x in items spawn { body }` | 各イテレーションが一つの spawn ドメインを生成 | イテレーションを跨ぐ分析は行わない |
| `spawn while cond { body }` | 条件並列：各イテレーション = 独立タスク | イテレーション間の依存は条件で保証される |
| `while cond spawn { body }` | 各イテレーションが一つの spawn ドメインを生成 | 上記とは意味が異なるが、特別な処理は不要 |
| `spawn if c { a } else { b }` | if-else 全体が一つの spawn ドメイン | 実行時は条件で分岐を選択 |
| `if c spawn { a } else { b }` | 単一分岐のみ spawn | if 式内部に spawn を包む |

### 除去される複雑さ

- ❌ `Expr::SpawnFor` を AST から削除
- ❌ `SpawnForAnalysis` を DAG 分析から削除
- ❌ `spawn for` を Parser で特殊な組み合わせキーワードとして扱わない
- ❌ `Ir::SpawnFor` を IR から削除

## 詳細設計

### 1. AST 層

**変更前：**

```rust
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

**変更後：**

```rust
Spawn { body: Box<Expr>, span: Span },           // spawn <任意の式>
```

`Expr::SpawnFor` を削除する。`spawn for x in items { body }` の AST 表現：

```rust
Expr::Spawn {
    body: Box::new(Expr::For {
        var: "x",
        iterable: items,
        body: body_block,
        ..
    })
}
```

**IF の特殊ケース**：

| 書き方 | AST 構造 |
|------|---------|
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }` |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

両者は意味が異なるが、いずれも自然な組み合わせであり、特別なルールは不要。

### 2. Parser 層

`spawn` の結合優先度は最も低く（`return` と同等）、後続の式全体を取り込む：

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser の変更：`pratt/nud.rs` において `spawn` は `{` を要求せず、汎用式パーサーを呼び出す：

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` は組み合わせキーワードとして扱わない。`for` は汎用式パーサーで処理されて `Expr::For` を生成し、`spawn` は単なるラップを担当する。

### 3. 型システム

**新しい `MonoType` バリアント：**

```rust
// ========== 計算構造型 ==========

/// {} ブロック式
Block(Box<MonoType>),

/// for ループ式
ForExpr { body_ty: Box<MonoType> },

/// while ループ式
WhileExpr { body_ty: Box<MonoType> },

/// if-else 分岐式
IfExpr {
    then_ty: Box<MonoType>,
    else_ty: Option<Box<MonoType>>,
},

/// 関数呼び出し式
Call {
    fn_ty: Box<MonoType>,
    result_ty: Box<MonoType>,
},

/// spawn 並行ラップ：内部の式が並行実行される
/// コンパイル時にマークされ、型チェック後に消去される
Spawn(Box<MonoType>),
```

**型推論ルール**：各式の型推論は「計算構造型」を返す。`Spawn` ラッピングがない = 順次実行、`Spawn` ラッピングがある = 並行実行。型チェック完了後 `Spawn` は消去され、型は内部の値型に降格する。

**型チェックの流れ**：
1. body 式の型 T（計算構造型）を推論する
2. spawn でラップされている場合、`Spawn(T)` として包む
3. 代入推論時に分解する：`results: List(Data) = spawn for ... {}` — `Spawn(ForExpr { body_ty: List(Data) })` から `List(Data)` を抽出する

`Spawn<T>` は型チェック完了後に消去され、ランタイムはデータが来たか並列かを知る必要がない。ただし、コンパイル時のリフレクション（`type_of(x)`）で完全な並行トポロジを取得できる。

### 4. DAG 分析層

現在の二つの入り口を一つに統合する：

```rust
/// 統一入り口：body 式の種類に応じてディスパッチ
fn analyze_spawn_expr(body: &Expr, ...) -> SpawnAnalysis {
    match body {
        Expr::Block(block)       => analyze_block_tasks(block, ...),
        Expr::For { .. }         => analyze_iter_tasks(IterKind::For, body, ...),
        Expr::While { .. }       => analyze_iter_tasks(IterKind::While, body, ...),
        Expr::If { .. }          => analyze_if_task(body, ...),
        _                        => single_task(body, ...),
    }
}
```

**統一結果構造：**

```rust
struct SpawnAnalysis {
    source: TaskSource,
    plan: ExecutionPlan,
}

enum TaskSource {
    /// spawn { a, b, c } — コンパイル時に既知の N 個の直接子式
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N 個のタスクがランタイムイテレーションで生成される
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for はあり、while はなし
        condition: Option<Expr>,     // while はあり、for はなし
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

`SpawnForAnalysis` 構造体を削除する。

| body の種類 | タスクへの分解方法 |
|-----------|--------------|
| `Expr::Block` | 直接の子式 → タスクリスト |
| `Expr::For` | 各イテレーション → 一つのタスク（データ並列） |
| `Expr::While` | 各イテレーション → 一つのタスク |
| `Expr::If` | 選択された分岐全体 → 一つのタスク |
| `Expr::Call` / その他 | 式自体 → 一つのタスク |

DAG 分析完了後、ランタイムは GMP モデルでスケジューリングする。依存関係のないタスクはワークキューに投入され、worker が奪い合う。

### 5. IR / Codegen 層

`Ir::SpawnFor` を削除する。`Ir::Spawn` に統一し、`TaskSource` 情報を保持する。

HIR → IR 変換は `SpawnAnalysis.source` に基づいてランタイム呼び出しを生成する：
- `TaskSource::Explicit(tasks)` → コンパイル時に既知のタスクリスト
- `TaskSource::Iterate { .. }` → ランタイム展開（コンパイラ駆動、par_iter ライクだがゼロコスト）

### 6. Placement 層

現在の二つの分岐を一つに統合する：

```rust
// 変更前
Expr::Spawn { body, .. } => self.check_block(body),
Expr::SpawnFor { body, iterable, .. } => {
    self.check_expr(iterable);
    self.check_block(body);
}

// 変更後
Expr::Spawn { body, .. } => self.check_expr(body),   // body は Expr、再帰するだけで十分
```

### 7. 後方互換性

既存の `spawn for` コードの意味は変わらない。Parser は `spawn for x in items { body }` を自動的に `Expr::Spawn { body: Expr::For }` として解析する。内部表現は変わるが、ユーザーから見える振る舞いは変わらない。

新しい文法は自然に獲得される：
```yx
spawn while has_next() {
    item = next()
    process(item)
}

spawn if use_cache {
    load_from_cache(key)
} else {
    fetch(key)
}
```

## トレードオフ

### 利点

1. **文法の直交性**：`spawn` + 任意の制御フロー = 自然な並行組み合わせ
2. **すべてが型**：型システムが計算構造を完全に記録し、コンパイル時リフレクションで並行トポロジを取得できる
3. **特殊ケースの除去**：`Expr::SpawnFor` および関連する特殊な処理コードを削除
4. **拡張性**：将来追加される新しい制御フロー構造は `spawn` と自動的に組み合わせ可能、spawn ロジックの修正不要

### 欠点

1. **型システムの肥大化**：6 つの新しい `MonoType` バリアントが追加され、型チェックの複雑さが増す
2. **破壊的変更**：内部の AST/IR 表現が変化し、`Expr::SpawnFor` を消費するすべてのコードの更新が必要
3. **式の型推論**：各式が計算構造型を返す必要があり、影響範囲が大きい

## 代替案

| 代替案 | 選択しない理由 |
|------|-------------|
| `spawn for` を独立した構文として保持 | 直交性を破壊し、言語で唯一のキーワード組み合わせ特例になる |
| `spawn` は `{}` のみを修飾し、データ並列は標準ライブラリ `par_iter` で提供 | 言語の原始能力がライブラリに降りてしまい、コンパイラ層の DAG 分析とリソース競合検出を失う |
| `SpawnFor` を削除するだけで型システムに計算構造型を導入しない | 型システムがリフレクション能力を失い、spawn が型のレベルで見えなくなる |

## RFC-019 との関係

本 RFC で導入される 6 つの `MonoType` バリアント（Block/ForExpr/WhileExpr/IfExpr/Call/Spawn）は [RFC-019: 型レベル同像性](./019-typed-homoiconicity.md) の**コンパイラ組み込みサブセット**である。RFC-019 の核となる理念「構文構造が型システムに入る」はここで次のように実現される：コンパイラがネイティブに理解する 6 種類の計算構造が対応する型表現を持つ。ユーザーは `SyntaxRule` で新しい計算構造型をカスタマイズできないが、コンパイラに組み込まれたこの 6 種類で全ての主要な制御フローをカバーする。

## 証明パイプライン統合

6 つの `MonoType` バリアントが存在する理由：それらは [RFC-027 コンパイル時証明パイプライン](../accepted/027-compile-time-evaluation-types.md) に**検証対象の命題の形**を伝える。パイプライン自体が実際の証明作業（自由変数分析、効果分類、別名分析、競合検出）を担当し、MonoType は一つのことだけを行う——構造化された入力インターフェースを提供する。

### バリアント → 命題マッピング

| 型 | 命題の形 | 証明戦略 |
|------|---------|---------|
| `Spawn(ForExpr { body_ty })` | データ並列：N 個のイテレーションタスクにイテレーション間競合なし | body の自由変数を抽出 → 効果分類 → Write(Shared) / `&mut`(Shared) がないことをチェック |
| `Spawn(WhileExpr { body_ty })` | 条件並列：各イテレーションが独立 + イテレーション間の因果依存なし | 上記に加えてイテレーション条件にイテレーション間の副作用がないことをチェック |
| `Spawn(Block(T))` | 明示的タスクグループ：タスク間の依存関係は DAG で与えられる | DAG 分析の依存グラフを検証——各タスクが必要とする入力はその開始時点で準備済み |
| `Spawn(IfExpr { then_ty, else_ty })` | 分岐 spawn：選択された分岐全体が一つの spawn ドメイン | 分岐選択に競合がなく、body 内で再帰的にチェック |
| `Spawn(Call { fn_ty, result_ty })` | 呼び出し spawn：呼び出される関数が一つの独立タスク | 関数の純粋性または分離性を検証 |
| `Spawn(T)`（値、`spawn 42` 等） | 単一値 spawn：並行性なし |  trivially pass |

### 証明シナリオ

**シナリオ 1 — 純粋なデータ並列（パス）：**

```yaoxiang
items = [1, 2, 3, 4, 5]
results = spawn for item in items { item * 2 }
// 型：Spawn(ForExpr { body_ty: List(Int) })
```

1. 自由変数：`item`（ループローカル、各イテレーション独立のコピー）、`items`（外部、body 内で読み取り専用）
2. 効果分類：全て Read(Local) または Read(Shared)、書き込みなし
3. 証明成功 ✓

**シナリオ 2 — 読み取り専用共有（パス）：**

```yaoxiang
config = load_config()
results = spawn for item in items { process(item, config) }
// 型：Spawn(ForExpr { body_ty: List(Result) })
```

1. 自由変数：`item`（Read(Local)）、`config`（外部、body 内に書き込みパスなし → Read(Shared)）
2. 効果分類：全て読み取り専用
3. 証明成功 ✓

**シナリオ 3 — 書き込み競合（拒否）：**

```yaoxiang
mut counter = 0
spawn for item in items { counter += 1 }
```

1. 自由変数：`item`（Read(Local)）、`counter`（外部、`+=` は書き込みに脱糖される）
2. 効果分類：`counter` は Write(Shared)、イテレーション間で同じメモリに書き込み
3. 競合のインスタンス化：`Write(task_0, counter) ∧ Write(task_1, counter) = True`
4. 証明失敗 ✗ → コンパイルエラー：`エラー：spawn for body にイテレーション間の書き込み競合が存在します。変数 counter が複数の並行タスクから書き込まれています。`

**シナリオ 4 — while + 状態を持つイテレータ（警告/拒否）：**

```yaoxiang
spawn while iter.has_next() {
    item = iter.next()
    process(item)
}
// 型：Spawn(WhileExpr { body_ty: List(Processed) })
```

1. 自由変数：`iter`（外部、`next()` → `&mut self` → `&mut`(Shared)）
2. `next()` はイテレータの状態を変更し、イテレーション N+1 はイテレーション N の副作用に依存する
3. これは独立タスクではない → `Spawn(WhileExpr)` の独立性制約に違反
4. コンパイラはイテレーション間の因果依存を報告し、`spawn for` への変更を提案

**シナリオ 5 — spawn if（パス）：**

```yaoxiang
result = spawn if use_cache { load(key) } else { fetch(key) }
// 型：Spawn(IfExpr { then_ty: T, else_ty: Option(T) })
```

1. 実行される分岐は一つだけで、タスク間の競合は存在しない
2. body 内にサブ spawn がある場合、再帰的にチェック
3. 証明成功 ✓

**シナリオ 6 — spawn ブロックのタスク間依存（DAG + パイプライン検証）：**

```yaoxiang
spawn {
    a = fetch_user(id)
    b = fetch_orders(a.user_id)  // a に依存
    c = compute_stats()           // 独立
}
// 型：Spawn(Block(Tuple(User, Orders, Stats)))
```

1. DAG 分析：`a` と `c` は独立（並列可能）、`b` は `a` に依存（a の後でスケジュール）
2. パイプライン検証：`b` の入力（`a.user_id`）は b 開始時に計算済み
3. 証明成功 ✓

### MonoType が行わないこと

| 行うこと | 行わないこと |
|--------|---------|
| 命題の形を識別する | 証明を実行しない |
| 計算構造を型レベルで記録する | DAG 分析を代替しない |
| RFC-027 パイプラインに型の入力を提供する | 自由変数分析、別名分析、競合検出を代替しない |

実際の証明作業はコンパイラの標準的な分析パスが担当する。MonoType の価値は、これらのパスを統一された型フレームワークの下でスケジューリングできることにある——証明パイプラインは AST ノードごとに特別な分岐を書く必要がない。

## 実装戦略

### 段階分け

1. **AST + Parser**：`Spawn { body: Box<Expr> }`、`SpawnFor` を削除
2. **型システム**：6 つの `MonoType` バリアントを追加、すべての式の型推論が計算構造型を返す
3. **DAG 分析の統合**：入り口を統合、`TaskSource` を Explicit + Iterate にマージ
4. **IR / Codegen 適応**：`Ir::SpawnFor` を削除、処理パスを統一
5. **Placement 簡略化**：`SpawnFor` 分岐を削除
6. **テスト検証**：既存の `spawn for` テストが全てパス

### 影響範囲

| ファイル/ディレクトリ | 変更 |
|-----------|------|
| `frontend/core/parser/ast.rs` | `Spawn` の body を `Box<Expr>` に変更、`SpawnFor` を削除 |
| `frontend/core/parser/pratt/nud.rs` | `spawn` ハンドラを汎用式パースに簡略化 |
| `frontend/core/types/mono.rs` | `Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn` バリアントを追加 |
| `frontend/core/spawn/analysis.rs` | 入り口を統一、`TaskSource` を統合 |
| `frontend/core/spawn/placement.rs` | `SpawnFor` 分岐を削除 |
| `frontend/core/typecheck/` | すべての式ノードを計算構造型の推論に適応 |
| `middle/core/ir.rs` | `Ir::SpawnFor` を削除 |
| `middle/` (IR gen, codegen) | spawn パスを統一、Spawn 型の消去 |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | 意味は不変、検証パス |

### 依存関係

- RFC-024（spawn ブロック並行モデル）— 本 RFC はその直交性拡張
- RFC-010（統一型構文）— 型システム変更の基礎
- RFC-027（コンパイル時証明パイプライン）— MonoType バリアントがパイプラインに命題形の入力を提供
- RFC-019（型レベル同像性）— MonoType バリアントはそのコンパイラ組み込みサブセット

## 設計決定記録

| 決定 | 決定内容 | 理由 | 日付 |
|------|------|------|------|
| spawn の修飾範囲 | 任意の式 | `spawn for` の特殊ケースを除去 | 2026-06-16 |
| `spawn while` のサポート | サポートする | 文法の直交性、実装コストが低い | 2026-06-16 |
| `spawn if` の意味論 | if-else 全体を修飾 | `if spawn { }` との区別 | 2026-06-16 |
| 型システム | 計算構造型を導入 | 「すべてが型」、コンパイル時リフレクションのサポート | 2026-06-16 |
| spawn 型の消去 | 型チェック後に消去 | ランタイムは並行構造情報を必要としない | 2026-06-16 |
| spawn の結合優先度 | 最も低い（return と同等） | 後続の式全体を取り込む | 2026-06-16 |
| for 内部の DAG | for 内部の子式を展開しない | 直接の子式ルールは不変、for 全体が一つのタスクソース | 2026-06-16 |
| 証明パイプライン統合 | MonoType バリアントを RFC-027 証明命題にマッピング | パイプラインは検証対象の命題の形を知る必要があり、MonoType は構造化された入力を提供 | 2026-07-03 |
| RFC-019 との関係 | コンパイラ組み込みサブセット | ユーザーはカスタマイズできないが、「構文は型なり」の理念を共有 | 2026-07-03 |
| 証明境界 | 6 つのシナリオでカバー：純粋並列/読み取り専用共有/書き込み競合/while 依存/spawn if/spawn block | 各 MonoType バリアントの証明義務と失敗条件を明確化 | 2026-07-03 |

---

## 参考文献

- [RFC-024: spawn ブロックベースの並行モデル](./024-concurrency-model.md)
- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-027: コンパイル時述語と統一静的検証](../accepted/027-compile-time-evaluation-types.md)
- [RFC-019: 型レベル同像性](./019-typed-homoiconicity.md)
- [並行モデル仕様](../../reference/language-spec/concurrency.md)
- [spawn for 直交性の保留（議論稿）](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## ライフサイクルと帰趣

| 状態 | 場所 | 説明 |
|------|------|------|
| **レビュー中** | `docs/design/rfc/review/` | コミュニティでの議論を公開中 |
```