---
title: 'RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの削除'
status: '審査中'
author: '晨煦'
created: '2026-06-16'
updated: '2026-08-19'
issue: '#98'
---

# RFC-032: spawn 統一式修飾

> **本文書は `spawn` の文法、AST/IR 再構成を定義します。** ランタイム動作意味論（タスク分割粒度、所有権、スコープ、エラー伝播、リソース型、ネスト）については [RFC-024: spawn ベースの並行処理ランタイム意味論](../accepted/024-concurrency-model.md) を参照してください。
>
> 2つの RFC が協調して `spawn` を定義します — 024 が「何をすべきか」を答え、032 が「どう表現するか」を答えます。

> **核心的な洞察**：`spawn` は `{}` ブロックのみを修飾すべきではありません。**任意の式**を修飾できます。`spawn for` は特殊構文ではなく、`spawn` + `for` 式の自然な組み合わせです。

## 摘要

`spawn` を `spawn { }`（ブロックのみ修飾）から `spawn <expr>`（任意の式を修飾）に拡張します。`Expr::SpawnFor` を AST から削除し、`Expr::Spawn { body: Expr::For { .. } }` で自然に代替します。本 RFC は AST/IR/Parser の整理のみを行い、型システム変更には触れません。

> **計算構造型（`MonoType` 拡張）は独立 RFC に延期します。** 本 RFC で `SpawnFor` 特殊ケースを削除した後、`spawn` の証明パイプライン統合には型システムによる計算構造の理解が必要です — これは spawn だけに限定されない汎用機構であり、独立した設計に値します。

## 動機

### なぜこの変更が必要か？

現在の `spawn for x in items { body }` は独立したキーワードの組み合わせであり、AST にはこれを表す `Expr::SpawnFor` が別途存在します。これは言語の直交性を損なっています：

1. **文法が統一されていない**：`spawn` は `{}` ブロックのみを修飾でき、`spawn for` はハードコードされた例外
2. **直交性の欠如**：`spawn while`、`spawn if` などの組み合わせを自然に表現できない

### 現在の問題

```rust
// AST 内の2つの spawn バリアント
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

## 提案

### コアデザイン

`spawn <expr>`：`spawn` が任意の式を修飾します。式の形状が DAG のタスク分解方法を決定します。

### ユーザーメンタルモデル

`spawn` = 「この式を並行処理に引き渡す」。式の形状が分割方法を決定：

| 式形状                         | 並行処理動作                         |
| ------------------------------ | ------------------------------------ |
| `spawn { a, b, c }`            | `a`、`b`、`c` が独立に並列実行       |
| `spawn for x in items { f(x) }`| N 回の反復が独立に並列実行           |
| `spawn while cond { step() }` | 各反復が独立タスク                   |
| `spawn if c { a } else { b }` | 選択された分岐整体が spawn ドメイン  |
| `spawn call(x)`                | 呼び出し自体が1つのタスク            |
| `spawn 42`                     | 単一のタスク                         |

コンパイラが DAG 分析で依存関係を決定し、ランタイムは GMP モデルでスケジューリング — 依存のないタスクはワークキューに投入され、worker が奪い合って実行。全体は同期ブロックし、全タスク完了を待ちます。

**Go との違い**：Go の `go` は「投げて放置」、YaoXiang の `spawn` は「分解して並列実行、全員完了を待ってから次へ」。

### 制御フローの直交性

| 組み合わせ                            | 意味                                    | 差異                                |
| ------------------------------------- | --------------------------------------- | ----------------------------------- |
| `spawn for x in items { body }`       | データ並列：各反復 = 独立タスク         | DAG が反復をまたいで依存を分析      |
| `for x in items spawn { body }`       | 各反復で spawn ドメインを生成           | 反復間の分析なし                    |
| `spawn while cond { body }`           | 条件並列：各反復 = 独立タスク           | 反復間依存は条件で保証              |
| `while cond spawn { body }`           | 各反復で spawn ドメインを生成           | 上とは意味が異なるが特殊処理不要    |
| `spawn if c { a } else { b }`         | if-else 全体を1つの spawn ドメインに    | 実行時に条件で分岐を選択            |
| `if c spawn { a } else { b }`         | 単一分岐のみ spawn                      | if 式内部に spawn が含まれる        |

### 削除される複雑さ

- ❌ `Expr::SpawnFor` が AST から削除
- ❌ `SpawnForAnalysis` が DAG 分析から削除
- ❌ `spawn for` が Parser で組み合わせキーワードとして特殊処理不再
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

**if 特殊ケース：**

| 書き方                              | AST 構造                                         |
| ----------------------------------- | ------------------------------------------------ |
| `spawn if cond { a } else { b }`   | `Spawn { body: Expr::If { ... } }`               |
| `if cond spawn { a } else { b }`   | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

両者の意味は異なるが、いずれも自然な組み合わせで、特殊ルールは不要。

### 2. Parser 層

`spawn` のバインディング優先度は最低（`return` と同等）で、後続の式全体を食べます：

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser 変更：`pratt/nud.rs` の `spawn` は `{` を要求不再、而是呼び出し通用式解析：

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` は組み合わせキーワードとして特殊処理不再 — `for` は通用式解析器で処理され `Expr::For` を生成し、`spawn` は包装のみを担当。

### 3. DAG 分析層

現在の2つのエントリポイントを1つに統合：

```rust
/// 統合エントリポイント：body 式の種別に従ってディスパッチ
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
    /// spawn for/while — N 個のタスクはランタイム反復で生成
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for にはあり、while にはない
        condition: Option<Expr>,     // while にはあり、for にはない
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

`SpawnForAnalysis` 構造体を削除。

| body 種別          | タスクへの分解方法                        |
| ------------------ | ----------------------------------------- |
| `Expr::Block`      | 直接部分式 → タスクリスト                 |
| `Expr::For`        | 各反復 → 1つのタスク（データ並列）         |
| `Expr::While`      | 各反復 → 1つのタスク                      |
| `Expr::If`         | 選択された分岐全体 → 1つのタスク           |
| `Expr::Call` / その他 | 式自体 → 1つのタスク                   |

DAG 分析完了後、ランタイムは GMP モデルでスケジューリング — 依存のないタスクはワークキューに投入され、worker が奪い合って実行。

### 4. IR / Codegen 層

`Ir::SpawnFor` を削除。`TaskSource` 情報を携带する `Ir::Spawn` に統合。

HIR → IR 翻訳は `SpawnAnalysis.source` に基づいてランタイム呼び出しを生成：

- `TaskSource::Explicit(tasks)` → コンパイル時に既知のタスクリスト
- `TaskSource::Iterate { .. }` → ランタイム展開（コンパイラ駆動、par_iter 類似だがゼロコスト）

### 5. Placement 層

現在の2つの分支を1つに統合：

```rust
// 変更前
Expr::Spawn { body, .. } => self.check_block(body),
Expr::SpawnFor { body, iterable, .. } => {
    self.check_expr(iterable);
    self.check_block(body);
}

// 変更後
Expr::Spawn { body, .. } => self.check_expr(body),   // body は Expr で再帰すればよい
```

### 6. 後方互換性

既存の `spawn for` コードの意味は不变、Parser が `spawn for x in items { body }` を自動的に `Expr::Spawn { body: Expr::For }` として解析。内部表現は変化するが、ユーザーから見える動作は不变。

新しい文法が自然に利用可能に：

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

**単一タスク spawn 警告**：`spawn call(x)` や `spawn 42` など単一式を修飾する場合、DAG 分析がコンパイル警告を生成：「spawn が単一式を修飾しても並行処理効果はなし」。文法は合法だが、ユーザーの意図を確認するよう促します。

## 权衡

### メリット

1. **文法定交**：`spawn` + 任意の制御フロー = 自然な並行処理の組み合わせ
2. **特殊ケースの削除**：`Expr::SpawnFor` 及相关特殊処理コードの削除
3. **拡張性**：将来の制御フロー構造追加は自動的に `spawn` と組み合わせ可能、spawn ロジック修正不要

### デメリット

1. **破壊的変更**：内部 AST/IR 表現が変化、`Expr::SpawnFor` を使用する全コードの更新が必要
2. **証明パイプライン適応が必要**：`SpawnFor` 削除後、証明パイプラインは AST によるディスパッチ（`match body { Expr::For => ..., Expr::While => ... }`）を通じて動作 — この適応は本 RFC 範囲で DAG 統合エントリポイントを通じて完了

## 代替案

| 案                                             | なぜ選択しないか                                            |
| ---------------------------------------------- | ----------------------------------------------------------- |
| `spawn for` を独立文法として維持              | 直交性を破壊し、言語で唯一のキーワード組み合わせ特例に      |
| `spawn` は `{}` のみを修飾、data 並列は標準ライブラリの `par_iter` を使用 | 言語の元来能力がライブラリに下落、コンパイラレベルの DAG 分析とリソース競合検出を失う |

## 計算構造型（独立 RFC に延期）

本 RFC で `SpawnFor` を削除した後、`spawn` の証明パイプライン統合はアーキテクチャ上の課題に直面します：証明パイプラインは型レベルで動作し、spawn 内部の計算構造（For/While/Block/If/Call）を知って初めて正しい証明策略を選択できます。現在の証明パイプラインは AST によるディスパッチですが、長期方向は計算構造を `MonoType` バリアント（`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`）としてエンコードし、パイプラインを完全に型レベルで動作させることです。

これは [RFC-019: 型レベル同像性](../draft/019-typed-homoiconicity.md) の弱化された実用的バージョンです — コンパイラ組み込みの計算構造が型システムに入り、用户自定义構文は開放しません。理論的基盤は ECMTT（Contextual Modal Types for Algebraic Effects and Handlers, ICFP 2021）：`Spawn<T>` は様相演算子 `□` に対応し、証明パイプラインは handler に対応します。

この機構は spawn だけに限られません — 将来の任意の effect（純粋計算、IO、failible）は同じパターンで型システムに入れます。spawn は最初のコンシューマーであって、唯一のコンシューマーではありません。

> **独立 RFC が定義する内容**：6つの MonoType バリアントの完全意味論、型チェッカー適応戦略、型によるディスパッチを行う証明パイプラインの統合インターフェース、RFC-027 との統合方案。

## 実装策略

### 段階的划分

1. **AST + Parser**：`Spawn { body: Box<Expr> }`、削除 `SpawnFor`
2. **DAG 分析統合**：エントリポイントを統合し、`TaskSource` 列挙型を統合。単一タスク spawn（`spawn call(x)`、`spawn 42`）がコンパイル警告を生成
3. **IR / Codegen 適応**：削除 `Ir::SpawnFor`、統合処理パスを実現
4. **Placement 簡略化**：削除 `SpawnFor` 分支
5. **テスト検証**：既存の `spawn for` テストがすべて通過

### 影響範囲

| ファイル/ディレクトリ                        | 変更内容                                                      |
| -------------------------------------------- | ------------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                | `Spawn` body を `Box<Expr>` に変更、`SpawnFor` を削除         |
| `frontend/core/parser/pratt/nud.rs`           | `spawn` ハンドラを通用式解析に簡略化                           |
| `frontend/core/spawn/analysis.rs`            | 統合エントリポイント、`TaskSource` で Explicit + Iterate を統合|
| `frontend/core/spawn/placement.rs`          | `SpawnFor` 分支を削除                                         |
| `middle/core/ir.rs`                          | `Ir::SpawnFor` を削除                                         |
| `middle/` (IR gen, codegen)                  | spawn パスを統合                                              |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | 意味は不变、検証通過                                          |

### 依存関係

- RFC-024（spawn ブロック並行処理モデル）— 本 RFC はその直交性拡張
- RFC-010（統合型文法）— 文法統合の基礎

## 設計決定記録

| 決定                 | 決定内容                                                                | 理由                                               | 日付        |
| -------------------- | ---------------------------------------------------------------------- | -------------------------------------------------- | ----------- |
| spawn 修飾範囲       | 任意の式                                                                | `spawn for` 特殊ケースを削除                       | 2026-06-16 |
| `spawn while` サポート | サポート                                                              | 文法定交、実装コスト低。証明パイプラインは反復をまたぐ依存ケースを拒否する可能性あり | 2026-06-16 |
| `spawn if` 意味       | if-else 全体を修飾                                                      | `if spawn { }` と区別                              | 2026-06-16 |
| spawn バインディング優先度 | 最低（`return` 同等）                                             | 後続の式全体を食べ                                 | 2026-06-16 |
| DAG の for 内部       | for 内部部分式を展開しない                                              | 直接部分式の規則は不变、for 整体が1つのタスクソース | 2026-06-16 |
| 単一タスク spawn 警告  | `spawn call(x)` / `spawn 42` がコンパイル警告を生成                    | 並行処理効果なし、ユーザーの意図を確認するよう促す | 2026-08-19 |
| 計算構造型           | 独立 RFC に延期                                                         | 汎用機構、spawn だけに限らない。ECMTT 理論基盤     | 2026-08-19 |

---

## 参考文献

- [RFC-024: spawn ブロックベースの並行処理モデル](../accepted/024-concurrency-model.md)
- [RFC-010: 統合型文法](../accepted/010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976) — 計算構造型の理論的基盤
- [並行処理モデル仕様](../../../reference/language-spec/concurrency.md)

---

## ライフサイクルと归宿

| 状態         | 位置                        | 説明            |
| ------------ | --------------------------- | --------------- |
| **審査中**   | `docs/design/rfc/review/`   | コミュニティ議論募集中 |