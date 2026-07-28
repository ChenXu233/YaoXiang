---
title: 'RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの解消'
status: '审核中'
author: '晨煦'
created: '2026-06-16'
updated: '2026-07-03'
issue: '#98'
---

# RFC-032: spawn 統一式修飾

> **本文書は `spawn`
> の構文、AST/IR 再構成、型システム拡張を定義する**。実行時動作のセマンティクス（タスク分割粒度、所有権、スコープ、エラー伝播、リソース型、ネスト）については
> [RFC-024: spawn ベースの並行ランタイムセマンティクス](./024-concurrency-model.md) を参照。
>
> 2つの RFC が協調して `spawn` を定義する —
> 024 が「何をすべきか」に答え、032 が「どう表現するか」に答える。

> **核心的な洞察**：`spawn` は `{}`
> ブロックのみを修飾すべきではない。**任意の式**を修飾できる。`spawn for`
> は特殊構文ではなく、`spawn` + `for` 式の自然な組み合わせに過ぎない。

## 摘要

`spawn` を `spawn { }`（ブロックのみ修飾）から
`spawn <expr>`（任意の式を修飾）へ拡張する。`Expr::SpawnFor`
を AST から削除し、`Expr::Spawn { body: Expr::For { .. } }`
で自然に代替する。式の構造の型（Block、For、While、If など）が新しい `MonoType`
バリアントとして型システムに入り、`Spawn<T>`
が並行実行される計算構造をラップする。コンパイル時にマークされ、チェック後に消去される。

## 動機

### なぜこの変更が必要か？

現在の `spawn for x in items { body }`
は独立したキーワードの組み合わせであり、AST にはそれを表す专门的 `Expr::SpawnFor`
がある。これは言語の直交性を壊している：

1. **構文の不統一**：`spawn` は `{}` ブロックのみを修飾でき、`spawn for` はハードコードされた例外
2. **直交性の欠如**：`spawn while`、`spawn if` などの組み合わせを自然に表現できない
3. **型システムが不完全**：spawn は型システムで見えず、型リフレクションで並行構造を取得できない

### 現在の問題

```rust
// AST に2つの spawn バリアントがある
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }

// MonoType には値型のみがあり、計算構造型がない
// spawn { a, b } の型 = Tuple(T_a, T_b)  ← 「これが spawn である」という情報が欠落
// spawn for    の型 = List(T)             ← 「これがデータ並列である」という情報が欠落
```

## 提案

### 核心設計

`spawn <expr>`：`spawn` が任意の式を修飾する。式の形状が DAG タスク分割方法を決定する。

**すべてを型にする**：`MonoType`
を「値型」から「値型 + 計算構造型」に拡張する。主要な式構造ごとに型システムに対応する型バリアントがある。`Spawn<T>`
が並行実行される計算構造をラップする。

### ユーザーメンタルモデル

`spawn` = 「この式を並行処理に回す」。式の形状が分割方法を決定する：

| 式の形状                        | 並行動作                            | 型                                                   |
| ------------------------------- | ----------------------------------- | ---------------------------------------------------- |
| `spawn { a, b, c }`             | `a`、`b`、`c` が独立並行            | `Spawn(Block(Tuple(T_a, T_b, T_c)))`                 |
| `spawn for x in items { f(x) }` | N 回の反復が独立並行                | `Spawn(ForExpr { body_ty: List(T) })`                |
| `spawn while cond { step() }`   | 各回の反復が独立タスク              | `Spawn(WhileExpr { body_ty: List(T) })`              |
| `spawn if c { a } else { b }`   | 選択された分岐全体が spawn ドメイン | `Spawn(IfExpr { then_ty: T_a, else_ty: Some(T_b) })` |
| `spawn call(x)`                 | 呼び出し自体が1つのタスク           | `Spawn(Call { fn_ty: Fn(A→R), result_ty: R })`       |
| `spawn 42`                      | 単一のタスク                        | `Spawn(Int)`                                         |

コンパイラが DAG 分析で依存関係を決定し、ランタイムが GMP モデルに従いスケジュールする — 依存関係のないタスクはワークキューに投入され、worker が競合しながら実行する。全体は同期ブロックし、すべてのタスク完了を待つ。

**Go との違い**：Go の `go` は「放っておいて待つ」で、YaoXiang の `spawn`
は「分割して並行実行し、すべて完了してから次に進む」。

### 制御流れの直交性

| 組み合わせ                      | セマンティクス                     | 差異                                       |
| ------------------------------- | ---------------------------------- | ------------------------------------------ |
| `spawn for x in items { body }` | データ並列：各反復 = 独立タスク    | DAG が反復をまたいだ依存分析を行う         |
| `for x in items spawn { body }` | 各反復で spawn ドメインを生成      | 反復をまたいだ分析なし                     |
| `spawn while cond { body }`     | 条件並列：各反復 = 独立タスク      | 反復間依存は条件が保証                     |
| `while cond spawn { body }`     | 各反復で spawn ドメインを生成      | 上記と異なるセマンティクスだが特殊処理不要 |
| `spawn if c { a } else { b }`   | if-else 全体が1つの spawn ドメイン | 実行時に条件に応じて分岐を選択             |
| `if c spawn { a } else { b }`   | 単一分岐のみ spawn                 | if 式内部に spawn を含む                   |

### 解消される複雑さ

- ❌ `Expr::SpawnFor` が AST から削除
- ❌ `SpawnForAnalysis` が DAG 分析から削除
- ❌ `spawn for` がパーサーで組み合わせキーワードとして特殊処理されなくなる
- ❌ `Ir::SpawnFor` が IR から削除

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

`Expr::SpawnFor` を削除。`spawn for x in items { body }` の AST 表現：

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

**if の特殊ケース：**

| 書き方                           | AST 構造                                            |
| -------------------------------- | --------------------------------------------------- |
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }`                  |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

両者のセマンティクスは異なるが、いずれも自然な組み合わせで特殊ルールは不要。

### 2. Parser 層

`spawn` のバインド優先度は最低（`return` と同等）で、後続の式全体を消費する：

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

パーサー変更：`pratt/nud.rs` で `spawn` は `{` を要求せず、汎用式解析を呼び出す：

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` は組み合わせキーワードとして特殊処理されない — `for` は汎用式パーサーで処理されて
`Expr::For` を生成し、`spawn` は単にラップを担当する。

### 3. 型システム

**新規 `MonoType` バリアント：**

```rust
// ========== 計算構造型 ==========

/// {} ブロック式
Block(Box<MonoType>),

/// for 反復式
ForExpr { body_ty: Box<MonoType> },

/// while 反復式
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

/// spawn 並行ラッパー：内部の式が並行実行される
/// コンパイル時マーカー、型チェック後に消去
Spawn(Box<MonoType>),
```

**型導出ルール**：各式の型導出は「計算構造型」を返す。`Spawn` ラッパーなし = 逐次実行、`Spawn`
ラッパーあり = 並行実行。型チェック完了後 `Spawn` は消去され、型は内部の値型に格下げられる。

**型チェックの流れ：**

1. body 式の型 T を導出（計算構造型）
2. spawn でラップされている場合、`Spawn(T)` でラップ
3. 代入導出時に分解：`results: List(Data) = spawn for ... {}` —
   `Spawn(ForExpr { body_ty: List(Data) })` から `List(Data)` を抽出

`Spawn<T>`
は型チェック完了後に消去され、実行時はデータが並行か逐次かを知る必要はない。ただし、コンパイル時リフレクション（`type_of(x)`）は完全な並行トポロジ構造を取得できる。

### 4. DAG 分析層

現在の2つの入口を1つに統合：

```rust
/// 統合入口：body 式の種別に従って分岐
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

**統合結果構造：**

```rust
struct SpawnAnalysis {
    source: TaskSource,
    plan: ExecutionPlan,
}

enum TaskSource {
    /// spawn { a, b, c } — コンパイル時に既知の N 個の直接部分式
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N 個のタスクは実行時に反復によって生成
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for にはあり、while にはなし
        condition: Option<Expr>,     // while にはあり、for にはなし
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

`SpawnForAnalysis` 構造体を削除。

| body 種別             | タスクへの分解方法                 |
| --------------------- | ---------------------------------- |
| `Expr::Block`         | 直接部分式 → タスクリスト          |
| `Expr::For`           | 各反復 → 1つのタスク（データ並列） |
| `Expr::While`         | 各反復 → 1つのタスク               |
| `Expr::If`            | 選択された分岐全体 → 1つのタスク   |
| `Expr::Call` / その他 | 式自体 → 1つのタスク               |

DAG 分析完了後、ランタイムは GMP モデルに従いスケジュールする — 依存関係のないタスクはワークキューに投入され、worker が競合しながら実行する。

### 5. IR / Codegen 層

`Ir::SpawnFor` を削除。統一して `Ir::Spawn` とし、`TaskSource` 情報を携带する。

HIR → IR 翻訳は `SpawnAnalysis.source` に応じて実行時呼び出しを生成：

- `TaskSource::Explicit(tasks)` → コンパイル時に既知のタスクリスト
- `TaskSource::Iterate { .. }` → 実行時に展開（コンパイラ駆動、par_iter 類似だがゼロコスト）

### 6. Placement 層

現在の2つの分岐を1つに統合：

```rust
// 変更前
Expr::Spawn { body, .. } => self.check_block(body),
Expr::SpawnFor { body, iterable, .. } => {
    self.check_expr(iterable);
    self.check_block(body);
}

// 変更後
Expr::Spawn { body, .. } => self.check_expr(body),   // body は Expr、再帰で処理
```

### 7. 後方互換性

既存の `spawn for` コードのセマンティクスは変更なし。パーソーは `spawn for x in items { body }`
を自動的に `Expr::Spawn { body: Expr::For }`
として解析する。内部表現は変化するが、ユーザーから見える動作は変わらない。

新しい構文も自然に使える：

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

## 权衡

### メリット

1. **構文の直交性**：`spawn` + 任意の制御流れ = 自然な並行組み合わせ
2. **すべてを型に**：型システムが計算構造を完全に記録し、コンパイル時リフレクションで並行トポロジを取得
3. **特殊ケースの解消**：`Expr::SpawnFor` および関連する特殊処理コードを削除
4. **拡張性**：将来の制御流れ構造の追加は自動的に `spawn` と組み合わせられ、spawn ロジックの変更不要

### デメリット

1. **型システムの膨張**：6個の新規 `MonoType` バリアントで型チェック複雑度が増加
2. **破壊的変更**：内部 AST/IR 表現が変化し、`Expr::SpawnFor` を消費する全コードを更新する必要がある
3. **式型導出**：各式が計算構造型を返す必要があり、影響範囲が大きい

## 代替案

| 案                                                                   | 選択しない理由                                                                                    |
| -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `spawn for` を独立構文として維持                                     | 直交性を壊し、言語内で唯一のキーワード組み合わせ特例になる                                        |
| `spawn` は `{}` のみを修飾し、データ並列は標準庫の `par_iter` を使用 | 言語の原始的能力がライブラリに降りてしまい、コンパイラレベルでの DAG 分析とリソース競合検出を失う |
| `SpawnFor` のみ削除し、型システムに計算構造型を導入しない            | 型システムがリフレクション能力を失い、spawn が型レベルで見えなくなる                              |

## RFC-019 との関係

本 RFC で導入される6個の `MonoType` バリアント（Block/ForExpr/WhileExpr/IfExpr/Call/Spawn）は
[RFC-019: 型レベル同像性](./019-typed-homoiconicity.md)
の**コンパイラ組み込みサブセット**である。RFC-019 の核心的理念「構文構造が型システムに入る」は、ここでは6種類のコンパイラが原生的に理解する計算構造に対応する型表現として実現される。ユーザーが
`SyntaxRule`
で新しい計算構造型を自作することはできないが、コンパイラ組み込みの6種類ですべての主要制御流れをカバーしている。

## 証明パイプライン統合

6個の `MonoType`
バリアントが存在する理由：[RFC-027 コンパイル時証明パイプライン](../accepted/027-compile-time-evaluation-types.md)
に対して**検証すべき命題の形状**を伝えるためである。パイプライン自体が実際の証明作業（自由変数分析、エフェクト分類、エイリアス分析、競合検出）を担当し、MonoType は1つのことだけを行う — 構造化された入力インターフェースを提供する。

### バリアント → 命題マッピング

| 型                                   | 命題形状                                                | 証明戦略                                                                                    |
| ------------------------------------ | ------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| `Spawn(ForExpr { body_ty })`         | データ並列：N 個の反復タスクが反復をまたいだ競合なし    | body の自由変数を抽出 → エフェクト分類 → Write(Shared) / `&mut`(Shared) なしを確認          |
| `Spawn(WhileExpr { body_ty })`       | 条件並列：各回の反復が独立 + 反復をまたいだ因果依存なし | 上記 + 反復条件が反復をまたいだ副作用を持つかどうかを確認                                   |
| `Spawn(Block(T))`                    | 明示的タスク群：タスク間依存関係は DAG で与えられる     | DAG 分析の依存グラフを検証 — 各タスクが必要とする入力がその開始時に利用可能であることを確認 |
| `Spawn(IfExpr { then_ty, else_ty })` | 分岐 spawn：選択された分岐全体が1つの spawn ドメイン    | 分岐選択に競合なし、body 内では再帰的にチェック                                             |
| `Spawn(Call { fn_ty, result_ty })`   | 呼び出し spawn：呼び出された関数が独立タスクとして実行  | 関数の純粋性または隔離性を検証                                                              |
| `Spawn(T)`（値、`spawn 42` など）    | 単値 spawn：並列性なし                                  | 自明にパス                                                                                  |

### 証明シナリオ

**シナリオ 1 — 純粋なデータ並列（通過）：**

```yaoxiang
items = [1, 2, 3, 4, 5]
results = spawn for item in items { item * 2 }
// 型：Spawn(ForExpr { body_ty: List(Int) })
```

1. 自由変数：`item`（ループローカル、各反復で独立コピー）、`items`（外部、body 内で読み取りのみ）
2. エフェクト分類：すべて Read(Local) または Read(Shared)、書き込みなし
3. Proved ✓

**シナリオ 2 — 読み取り専用共有（通過）：**

```yaoxiang
config = load_config()
results = spawn for item in items { process(item, config) }
// 型：Spawn(ForExpr { body_ty: List(Result) })
```

1. 自由変数：`item`（Read(Local)）、`config`（外部、body 内に書き込みパスなし → Read(Shared)）
2. エフェクト分類：すべて読み取り専用
3. Proved ✓

**シナリオ 3 — 書き込み競合（拒否）：**

```yaoxiang
mut counter = 0
spawn for item in items { counter += 1 }
```

1. 自由変数：`item`（Read(Local)）、`counter`（外部、`+=` が脱糖されて書き込み）
2. エフェクト分類：`counter` は Write(Shared)、反復をまたいで同一メモリに書き込み
3. インスタンス化競合：`Write(task_0, counter) ∧ Write(task_1, counter) = True`
4. Disproved ✗
   → コンパイルエラー：`エラー：spawn for body に反復をまたいだ書き込み競合が存在します。変数 counter が複数の並行タスクから書き込まれています。`

**シナリオ 4 — while + 状態ありイテレータ（警告/拒否）：**

```yaoxiang
spawn while iter.has_next() {
    item = iter.next()
    process(item)
}
// 型：Spawn(WhileExpr { body_ty: List(Processed) })
```

1. 自由変数：`iter`（外部、`next()` → `&mut self` → `&mut`(Shared)）
2. `next()` がイテレータの状態を変更し、反復 N+1 が反復 N の副作用に依存
3. これは独立タスクではない → `Spawn(WhileExpr)` の独立性制約に違反
4. コンパイラが反復をまたいだ因果依存を報告し、`spawn for` の使用を推奨

**シナリオ 5 — spawn if（通過）：**

```yaoxiang
result = spawn if use_cache { load(key) } else { fetch(key) }
// 型：Spawn(IfExpr { then_ty: T, else_ty: Option(T) })
```

1. 1つの分岐のみ実行され、タスク間の競合が存在しない
2. body 内にサブ spawn がある場合は再帰的にチェック
3. Proved ✓

**シナリオ 6 — spawn ブロックタスク間依存（DAG + パイプライン検証）：**

```yaoxiang
spawn {
    a = fetch_user(id)
    b = fetch_orders(a.user_id)  // a に依存
    c = compute_stats()           // 独立
}
// 型：Spawn(Block(Tuple(User, Orders, Stats)))
```

1. DAG 分析：`a` と `c` は独立（並列可能）、`b` は `a` に依存（a の後にスケジュール）
2. パイプライン検証：`b` の入力（`a.user_id`）が b 起動時に計算完了済み
3. Proved ✓

### MonoType のやらないこと

| やること                           | やらないこと                                       |
| ---------------------------------- | -------------------------------------------------- |
| 命題の形状を識別                   | 証明を実行しない                                   |
| 型レベルで計算構造を記録           | DAG 分析を代替しない                               |
| RFC-027 パイプラインに型入力を提供 | 自由変数分析、エイリアス分析、競合検出を代替しない |

実際の証明作業はコンパイラの標準分析パスが完了する。MonoType の価値は、これらのパスを統一された型フレームワークの下でスケジュールできることにある — 証明パイプラインは各 AST ノード向けに特殊的分岐を書く必要がない。

## 実装戦略

### 段階的划分

1. **AST + Parser**：`Spawn { body: Box<Expr> }`、`SpawnFor` を削除
2. **型システム**：6個の新規 `MonoType` バリアント、すべての式型導出が計算構造型を返す
3. **DAG 分析統合**：入口を統合、`TaskSource` 列挙型で Explicit + Iterate を統合
4. **IR / Codegen 適応**：`Ir::SpawnFor` を削除、パスを統一
5. **Placement 簡略化**：`SpawnFor` 分岐を削除
6. **テスト検証**：既存の `spawn for` テストがすべて通過

### 影響範囲

| ファイル/ディレクトリ                        | 変更                                                                  |
| -------------------------------------------- | --------------------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                | `Spawn` body を `Box<Expr>` に変更、`SpawnFor` を削除                 |
| `frontend/core/parser/pratt/nud.rs`          | `spawn` ハンドラを汎用式解析に簡略化                                  |
| `frontend/core/types/mono.rs`                | 新規 `Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn` バリアント |
| `frontend/core/spawn/analysis.rs`            | 入口を統合、`TaskSource` 列挙型で Explicit + Iterate を統合           |
| `frontend/core/spawn/placement.rs`           | `SpawnFor` 分岐を削除                                                 |
| `frontend/core/typecheck/`                   | すべての式ノードを計算構造型導出に適応                                |
| `middle/core/ir.rs`                          | `Ir::SpawnFor` を削除                                                 |
| `middle/` (IR gen, codegen)                  | spawn パスを統一、Spawn 型を消去                                      |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | セマンティクスは変更なし、検証通過                                    |

### 依存関係

- RFC-024（spawn ブロック並行モデル）— 本 RFC はその直交性拡張
- RFC-010（統一型構文）— 型システム変更の基礎
- RFC-027（コンパイル時証明パイプライン）— MonoType バリアントがパイプラインに命題形状入力を提供
- RFC-019（型レベル同像性）— MonoType バリアントはそのコンパイラ組み込みサブセット

## 設計决策記録

| 決定                      | 決定                                                                                      | 理由                                                                            | 日付       |
| ------------------------- | ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------- | ---------- |
| spawn 修飾範囲            | 任意の式                                                                                  | `spawn for` 特殊ケースの解消                                                    | 2026-06-16 |
| `spawn while` 対応        | 対応                                                                                      | 構文の直交性、実装コストが低い                                                  | 2026-06-16 |
| `spawn if` セマンティクス | 全体 if-else を修飾                                                                       | `if spawn { }` との区別                                                         | 2026-06-16 |
| 型システム                | 計算構造型を導入                                                                          | 「すべてを型に」、コンパイル時リフレクション対応                                | 2026-06-16 |
| spawn 型消去              | 型チェック後に消去                                                                        | 実行時は並行構造情報不要                                                        | 2026-06-16 |
| spawn バインド優先度      | 最低（return と同等）                                                                     | 後続の式全体を消費                                                              | 2026-06-16 |
| DAG の for 内部           | for 内部部分式を展開しない                                                                | 直接部分式のルールそのまま、for 全体が1つのタスク源                             | 2026-06-16 |
| 証明パイプライン統合      | MonoType バリアントを RFC-027 証明命題にマッピング                                        | パイプラインは検証命題の形状を知る必要があり、MonoType が構造化された入力を提供 | 2026-07-03 |
| RFC-019 関係              | コンパイラ組み込みサブセット                                                              | ユーザーは自作できないが、「構文即型」という理念を共有                          | 2026-07-03 |
| 証明境界                  | 6シナリオをカバー：純粋並列/読み取り専用共有/書き込み競合/while 依存/spawn if/spawn block | 各 MonoType バリアントの証明義務と失敗条件を明確化                              | 2026-07-03 |

---

## 参考文献

- [RFC-024: spawn ブロックベースの並行モデル](./024-concurrency-model.md)
- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-027: コンパイル時述語と統一静的検証](../accepted/027-compile-time-evaluation-types.md)
- [RFC-019: 型レベル同像性](./019-typed-homoiconicity.md)
- [並行モデル仕様](../../reference/language-spec/concurrency.md)
- [spawn for 直交性保留（議論稿）](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## ライフサイクルと归宿

| 状態       | 位置                      | 説明                 |
| ---------- | ------------------------- | -------------------- |
| **審査中** | `docs/design/rfc/review/` | コミュニティ議論公開 |
