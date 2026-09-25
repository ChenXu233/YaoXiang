---
title: 'RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの解消'
status: 'レビュー中'
author: '晨煦'
created: '2026-06-16'
updated: '2026-08-19'
issue: '#98'
---

# RFC-032: spawn 統一式修飾

> **本文書では `spawn`
> の構文、AST/IR 再構築を定義する**。実行時動作のセマンティクス（タスク分解の粒度、所有権、スコープ、エラー伝播、リソース型、ネスト）については
> [RFC-024: spawn ブロックに基づく並行実行時セマンティクス](../accepted/024-concurrency-model.md)
> を参照。
>
> 二つの RFC が協調して `spawn`
> を定義する——024 は「何をするか」に答え、032 は「どう表現するか」に答える。

> **核心的な洞察**：`spawn` は `{}`
> ブロックのみを修飾すべきではない。**任意の式**を修飾できる。`spawn for`
> は特殊な構文ではなく、`spawn` + `for` 式の自然な組み合わせである。

## 概要

`spawn` を `spawn { }`（ブロックのみを修飾）から
`spawn <expr>`（任意の式を修飾）へと拡張する。`Expr::SpawnFor`
を AST から削除し、`Expr::Spawn { body: Expr::For { .. } }`
によって自然に置き換える。本 RFC は AST/IR/Parser の整理のみを行い、型システムの変更には及ばない。

> **計算構造型（`MonoType` 拡張）は別個の RFC に延期する。** 本 RFC で `SpawnFor`
> 特殊ケースを削除した後、`spawn`
> の証明パイプライン統合には型システムが計算構造を認識する必要がある——これは汎用機構であり spawn に限らないため、独立した設計に値する。

## 動機

### なぜこの変更が必要か？

現在の `spawn for x in items { body }` は独立したキーワード組み合わせであり、AST には専用の
`Expr::SpawnFor` が存在する。これは言語の直交性を破壊している：

1. **構文の不統一**：`spawn` は `{}` ブロックのみを修飾でき、`spawn for`
   はハードコードされた例外である
2. **直交性の欠如**：`spawn while`、`spawn if` などの組み合わせを自然に表現できない

### 現状の問題

```rust
// AST 中两个 spawn 变体
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

## 提案

### 核となる設計

`spawn <expr>`：`spawn` は任意の式を修飾する。式の形状が DAG によるタスク分解方法を決定する。

### ユーザーのメンタルモデル

`spawn` = 「この式を並行実行に持っていく」。式の形状が分解方法を決定する：

| 式の形状                        | 並行動作                             |
| ------------------------------- | ------------------------------------ |
| `spawn { a, b, c }`             | `a`、`b`、`c` が独立に並列実行       |
| `spawn for x in items { f(x) }` | N 個のイテレーションが独立に並列実行 |
| `spawn while cond { step() }`   | 各イテレーションが独立タスク         |
| `spawn if c { a } else { b }`   | 選択された分岐全体が spawn ドメイン  |
| `spawn call(x)`                 | 呼び出し自体が 1 つのタスク          |
| `spawn 42`                      | 単独の 1 タスク                      |

コンパイラが DAG 分析により依存関係を決定し、実行時は GMP モデルでスケジューリングする——依存関係のないタスクは作業キューに投げ込まれ、worker が奪い合うように実行する。全体は同期的にブロックし、すべてのタスクの完了を待つ。

**Go との違い**：Go の `go` は「投げ出したら知らん」だが、YaoXiang の `spawn`
は「分解して並列実行、すべて完了するまで先に進まない」である。

### 制御フローの直交性

| 組み合わせ                      | セマンティクス                            | 差異                                         |
| ------------------------------- | ----------------------------------------- | -------------------------------------------- |
| `spawn for x in items { body }` | データ並列：各イテレーション = 独立タスク | DAG がイテレーション横断で依存を分析         |
| `for x in items spawn { body }` | 各イテレーションが spawn ドメインを生成   | イテレーション横断分析なし                   |
| `spawn while cond { body }`     | 条件並列：各イテレーション = 独立タスク   | イテレーション間の依存は条件で保証           |
| `while cond spawn { body }`     | 各イテレーションが spawn ドメインを生成   | 上記とはセマンティクスが異なるが特殊処理不要 |
| `spawn if c { a } else { b }`   | if-else 全体が一つの spawn ドメイン       | 実行時に条件で分岐を選択                     |
| `if c spawn { a } else { b }`   | 単一分岐のみ spawn                        | if 式内部に spawn を含む                     |

### 削除される複雑さ

- ❌ `Expr::SpawnFor` を AST から削除
- ❌ `SpawnForAnalysis` を DAG 分析から削除
- ❌ `spawn for` を Parser で組み合わせキーワードとして特殊扱いをしない
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
Spawn { body: Box<Expr>, span: Span },           // spawn <任意表达式>
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

**IF 特殊ケース**：

| 書き方                           | AST 構造                                            |
| -------------------------------- | --------------------------------------------------- |
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }`                  |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

両者はセマンティクスが異なるが、どちらも自然な組み合わせであり、特別なルールは不要である。

### 2. Parser 層

`spawn` の結合優先度は最も低い（`return` と同等）で、後続の式全体を吸収する：

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser の変更：`pratt/nud.rs` において `spawn` は `{` を要求せず、汎用式解析を呼び出す：

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` は組み合わせキーワードとして扱わない——`for` は汎用式パーサーにより処理されて `Expr::For`
を生成し、`spawn` はラップのみを担当する。

### 3. DAG 分析層

現在の 2 つの入口を 1 つに統合する：

```rust
/// 统一入口：根据 body 表达式种类分发
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

**統一結果構造**：

```rust
struct SpawnAnalysis {
    source: TaskSource,
    plan: ExecutionPlan,
}

enum TaskSource {
    /// spawn { a, b, c } — 编译期已知的 N 个直接子表达式
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N 个任务由运行时迭代产生
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for 有，while 无
        condition: Option<Expr>,     // while 有，for 无
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

`SpawnForAnalysis` 構造体を削除する。

| body の種類           | タスクへの分解方法                        |
| --------------------- | ----------------------------------------- |
| `Expr::Block`         | 直接の子式 → タスク一覧                   |
| `Expr::For`           | 各イテレーション → 1 タスク（データ並列） |
| `Expr::While`         | 各イテレーション → 1 タスク               |
| `Expr::If`            | 選択された分岐全体 → 1 タスク             |
| `Expr::Call` / その他 | 式そのもの → 1 タスク                     |

DAG 分析完了後、実行時は GMP モデルでスケジューリングする——依存関係のないタスクは作業キューに投げ込まれ、worker が奪い合うように実行する。

### 4. IR / Codegen 層

`Ir::SpawnFor` を削除。`Ir::Spawn` に統一し、`TaskSource` 情報を保持する。

HIR → IR 変換は `SpawnAnalysis.source` に基づいて実行時呼び出しを生成する：

- `TaskSource::Explicit(tasks)` → コンパイル時に既知のタスク一覧
- `TaskSource::Iterate { .. }` → 実行時に展開（コンパイラ駆動、`par_iter` と同様だがゼロコスト）

### 5. Placement 層

現在の 2 つの分岐を 1 つに統合する：

```rust
// 之前
Expr::Spawn { body, .. } => self.check_block(body),
Expr::SpawnFor { body, iterable, .. } => {
    self.check_expr(iterable);
    self.check_block(body);
}

// 之后
Expr::Spawn { body, .. } => self.check_expr(body),   // body 是 Expr，递归即可
```

### 6. 後方互換性

既存の `spawn for` コードのセマンティクスは変わらず、Parser は `spawn for x in items { body }`
を自動的に `Expr::Spawn { body: Expr::For }`
として解析する。内部表現は変化するが、ユーザーに見える振る舞いは変わらない。

新構文が自然に得られる：

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

**単一タスク spawn 警告**：`spawn call(x)` や `spawn 42`
など単一式を修飾する場合、DAG 分析はコンパイル警告を生成する：「spawn による単一式の修飾は並列効果がない」。構文としては有効だが、ユーザーの意図確認を促す。

## トレードオフ

### 利点

1. **構文の直交性**：`spawn` + 任意の制御フロー = 自然な並行組み合わせ
2. **特殊ケースの解消**：`Expr::SpawnFor` および関連特殊処理コードを削除
3. **拡張性**：将来追加される制御フロー構造は `spawn` と自動的に組み合わせ可能、`spawn`
   ロジックの変更不要

### 欠点

1. **破壊的変更**：内部 AST/IR 表現が変化し、`Expr::SpawnFor` を消費するすべてのコードの更新が必要
2. **証明パイプラインの適応が必要**：`SpawnFor`
   削除後、証明パイプラインは AST ディスパッチ（`match body { Expr::For => ..., Expr::While => ... }`）を介する——この適応は本 RFC の範囲内で DAG 統一入口によって実現する

## 代替案

| 案                                                                         | 採用しない理由                                                                    |
| -------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `spawn for` を独立構文として保持                                           | 直交性を破壊し、言語内で唯一のキーワード組み合わせ特例となる                      |
| `spawn` は `{}` のみを修飾し、データ並列は標準ライブラリ `par_iter` で対応 | 言語の原始能力がライブラリに降り、コンパイラ層の DAG 分析とリソース競合検出を失う |

## 計算構造型（独立した RFC に延期）

本 RFC で `SpawnFor` を削除した後、`spawn`
の証明パイプライン統合はアーキテクチャ上の問題に直面する：証明パイプラインは型層で動作し、正しい証明戦略を選択するために spawn 内部の計算構造（For/While/Block/If/Call）を知る必要がある。現在、証明パイプラインは AST ディスパッチを介するが、長期的方向は計算構造を
`MonoType`
バリアント（`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`）としてエンコードし、パイプラインを完全に型層で動作させることである。

これは [RFC-019: 型レベル同像性](../draft/019-typed-homoiconicity.md)
の弱化実用版である——コンパイラ内蔵の計算構造が型システムに入るが、ユーザーカスタム構文は開放しない。理論的基礎は ECMTT（Contextual
Modal Types for Algebraic Effects and Handlers, ICFP 2021）である：`Spawn<T>` はモーダル演算子 `□`
に対応し、証明パイプラインは handler に対応する。

この機構は spawn に限定されない——将来あらゆる effect（純粋計算、IO、fallible）は同じパターンにより型システムに入り得る。spawn は最初の消費者であり、唯一の消費者ではない。

> **独立した RFC で定義される**：6 つの MonoType バリアントの完全なセマンティクス、型チェッカー適応戦略、型による証明パイプラインの統一ディスパッチインターフェース、RFC-027 との統合方案。

## 実装戦略

### フェーズ分け

1. **AST + Parser**：`Spawn { body: Box<Expr> }`、`SpawnFor` を削除
2. **DAG 分析統一**：入口を統合、`TaskSource`
   列挙を統一。単一タスク spawn（`spawn call(x)`、`spawn 42`）はコンパイル警告を生成
3. **IR / Codegen 適応**：`Ir::SpawnFor` を削除、処理パスを統一
4. **Placement 簡素化**：`SpawnFor` 分岐を削除
5. **テスト検証**：既存の `spawn for` テストはすべて合格

### 影響範囲

| ファイル/ディレクトリ                        | 変更内容                                                 |
| -------------------------------------------- | -------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                | `Spawn` の body を `Box<Expr>` に変更、`SpawnFor` を削除 |
| `frontend/core/parser/pratt/nud.rs`          | `spawn` ハンドラを汎用式解析に簡素化                     |
| `frontend/core/spawn/analysis.rs`            | 入口を統一、`TaskSource` を Explicit + Iterate に統合    |
| `frontend/core/spawn/placement.rs`           | `SpawnFor` 分岐を削除                                    |
| `middle/core/ir.rs`                          | `Ir::SpawnFor` を削除                                    |
| `middle/` (IR gen, codegen)                  | spawn パスを統一                                         |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | セマンティクス不変、検証合格                             |

### 依存関係

- RFC-024（spawn ブロック並行モデル）— 本 RFC はその直交性拡張
- RFC-010（統一型構文）— 構文統一の基礎

## 設計決定の記録

| 決定                        | 決定内容                                            | 理由                                                                                             | 日付       |
| --------------------------- | --------------------------------------------------- | ------------------------------------------------------------------------------------------------ | ---------- |
| spawn 修飾範囲              | 任意の式                                            | `spawn for` 特殊ケースの解消                                                                     | 2026-06-16 |
| `spawn while` サポート      | サポート                                            | 構文の直交性、実装コスト低。証明パイプラインはイテレーション横断依存のケースを拒否する可能性あり | 2026-06-16 |
| `spawn if` セマンティクス   | if-else 全体を修飾                                  | `if spawn { }` との区別                                                                          | 2026-06-16 |
| spawn 結合優先度            | 最低（return と同等）                               | 後続の式全体を吸収                                                                               | 2026-06-16 |
| DAG の for 内部に対する扱い | for 内部の子式を展開しない                          | 直接の子式ルールは不変、for 全体が 1 タスクソース                                                | 2026-06-16 |
| 単一タスク spawn 警告       | `spawn call(x)` / `spawn 42` はコンパイル警告を生成 | 並列効果なし、ユーザーの意図確認を促す                                                           | 2026-08-19 |
| 計算構造型                  | 独立 RFC に延期                                     | 汎用機構、spawn に限らない。ECMTT 理論的基礎                                                     | 2026-08-19 |

---

## 参考文献

- [RFC-024: spawn ブロックに基づく並行モデル](../accepted/024-concurrency-model.md)
- [RFC-010: 統一型構文](../accepted/010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976)
  — 計算構造型の理論的基礎
- [並行モデル仕様](../../../reference/language-spec/concurrency.md)

---

## ライフサイクルと帰属

| ステータス     | 位置                      | 説明                                 |
| -------------- | ------------------------- | ------------------------------------ |
| **レビュー中** | `docs/design/rfc/review/` | オープンコミュニティディスカッション |
