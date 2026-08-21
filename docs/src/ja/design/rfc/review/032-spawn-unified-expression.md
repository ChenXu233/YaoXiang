---
title: 'RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの排除'
status: 'レビュー中'
author: '晨煦'
created: '2026-06-16'
updated: '2026-08-19'
issue: '#98'
---

# RFC-032: spawn 統一式修飾

> **本文書は `spawn`
> の構文、AST/IR 再構築を定義する**。ランタイム挙動セマンティクス（タスク分解粒度、所有権、スコープ、エラー伝播、リソースタイプ、ネスト）については
> [RFC-024: spawn ベースの並行ランタイムセマンティクス](./024-concurrency-model.md) を参照。

> **二つの RFC は協調して `spawn` を定義する —
> 024 は「何をするか」に答え、032 は「どう表現するか」に答える。**

> **核心的洞察**：`spawn` は `{}`
> ブロックのみを修飾すべきではない。**任意の式**を修飾できる。`spawn for`
> は特殊な構文ではなく、`spawn` + `for` 式の自然な組み合わせに過ぎない。

## 概要

`spawn` を `spawn { }`（ブロックのみ修飾）から
`spawn <expr>`（任意の式を修飾）に拡張する。`Expr::SpawnFor`
は AST から削除され、`Expr::Spawn { body: Expr::For { .. } }`
で自然に置換される。本 RFC は AST/IR/Parser のクリーンアップのみを行い、型システムの変更には触れない。

> **計算構造型（`MonoType` 拡張）は独立した RFC に延期する。** 本 RFC で `SpawnFor`
> 特殊ケースを削除した後、`spawn`
> の証明パイプライン統合には型システムが計算構造を認識する必要がある — これは汎用機構であり spawn に限らず、独立した設計に値する。

## 動機

### なぜこの変更が必要か？

現在の `spawn for x in items { body }`
は独立したキーワード組み合わせであり、AST にはそれを表す専用の `Expr::SpawnFor`
がある。これは言語の直交性を損なう：

1. **構文の不統一**：`spawn` は `{}` ブロックのみを修飾でき、`spawn for` はハードコードされた例外
2. **直交性の欠如**：`spawn while`、`spawn if` などの組み合わせを自然に表現できない

### 現在の問題

```rust
// AST 中两个 spawn 变体
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

## 提案

### 中核設計

`spawn <expr>`：`spawn` は任意の式を修飾する。式の形状が DAG のタスク分解方法を決定する。

### ユーザーのメンタルモデル

`spawn` = 「この式を取り出して並行実行する」。式の形状が分解方法を決定する：

| 式の形状                        | 並行動作                             |
| ------------------------------- | ------------------------------------ |
| `spawn { a, b, c }`             | `a`、`b`、`c` が独立に並行実行       |
| `spawn for x in items { f(x) }` | N 回のイテレーションが独立に並行実行 |
| `spawn while cond { step() }`   | 各イテレーションが独立タスク         |
| `spawn if c { a } else { b }`   | 選択された分岐全体が spawn ドメイン  |
| `spawn call(x)`                 | 呼び出し自体が 1 つのタスク          |
| `spawn 42`                      | 単独のタスク                         |

コンパイラは DAG 分析により依存関係を判定し、ランタイムは GMP モデルに従ってスケジュールする — 依存関係のないタスクは作業キューに投げ込まれ、worker が奪い合う。全体は同期的にブロックし、すべてのタスクの完了を待つ。

**Go との違い**：Go の `go` は「投げ出して放っておく」だが、YaoXiang の `spawn`
は「分解して並行実行し、すべて完了するまで待機する」。

### 制御フローの直交性

| 組み合わせ                      | セマンティクス                            | 差異                                         |
| ------------------------------- | ----------------------------------------- | -------------------------------------------- |
| `spawn for x in items { body }` | データ並列：各イテレーション = 独立タスク | DAG がイテレーション間の依存を分析           |
| `for x in items spawn { body }` | 各イテレーションが spawn ドメインを作成   | イテレーション間の分析を行わない             |
| `spawn while cond { body }`     | 条件並列：各イテレーション = 独立タスク   | イテレーション間の依存は条件で保証           |
| `while cond spawn { body }`     | 各イテレーションが spawn ドメインを作成   | 上のセマンティクスとは異なるが特殊処理は不要 |
| `spawn if c { a } else { b }`   | if-else 全体が 1 つの spawn ドメイン      | 実行時に条件に応じて分岐を選択               |
| `if c spawn { a } else { b }`   | 単一分岐のみ spawn                        | if 式内部に spawn を含む                     |

### 排除された複雑さ

- ❌ `Expr::SpawnFor` を AST から削除
- ❌ `SpawnForAnalysis` を DAG 分析から削除
- ❌ `spawn for` を Parser で組み合わせキーワードとして特殊処理しない
- ❌ `Ir::SpawnFor` を IR から削除

## 詳細設計

### 1. AST レイヤ

**変更前：**

```rust
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

**変更後：**

```rust
Spawn { body: Box<Expr>, span: Span },           // spawn <任意式>
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

両者のセマンティクスは異なるが、いずれも自然な組み合わせであり、特殊ルールは不要。

### 2. Parser レイヤ

`spawn` の結合優先度は最低（`return` と同等）で、後続の式全体を取り込む：

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser の変更：`pratt/nud.rs` 内の `spawn` は `{` を要求しなくなり、汎用式パーサーを呼び出す：

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` は組み合わせキーワードとして処理されなくなる — `for` は汎用式パーサーで処理され
`Expr::For` を生成し、`spawn` は単にそれをラップする。

### 3. DAG 分析レイヤ

現在の 2 つのエントリを 1 つに統合：

```rust
/// 統一入口：根据 body 表达式种类分发
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

`SpawnForAnalysis` 構造体を削除。

| body の種類           | タスクへの分解方法                        |
| --------------------- | ----------------------------------------- |
| `Expr::Block`         | 直接の子式 → タスク一覧                   |
| `Expr::For`           | 各イテレーション → 1 タスク（データ並列） |
| `Expr::While`         | 各イテレーション → 1 タスク               |
| `Expr::If`            | 選択された分岐全体 → 1 タスク             |
| `Expr::Call` / その他 | 式自体 → 1 タスク                         |

DAG 分析完了後、ランタイムは GMP モデルに従ってスケジュールする — 依存関係のないタスクは作業キューに投げ込まれ、worker が奪い合う。

### 4. IR / Codegen レイヤ

`Ir::SpawnFor` を削除。`Ir::Spawn` に統一し、`TaskSource` 情報を保持する。

HIR → IR 変換は `SpawnAnalysis.source` に基づいてランタイム呼び出しを生成する：

- `TaskSource::Explicit(tasks)` → コンパイル時に既知のタスク一覧
- `TaskSource::Iterate { .. }` → ランタイム展開（コンパイラ駆動、`par_iter` ライクだがゼロコスト）

### 5. Placement レイヤ

現在の 2 つの分岐を 1 つに統合：

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
として解析する。内部表現は変わるが、ユーザーから見える振る舞いは変わらない。

新構文が自然に利用可能：

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
など単一の式を修飾する場合、DAG 分析がコンパイル警告を生成する：「spawn が単一式を修飾しても並列効果がない」。構文上は合法だが、ユーザーの意図を確認するためのリマインダー。

## トレードオフ

### 利点

1. **構文の直交性**：`spawn` + 任意の制御フロー = 自然な並行組み合わせ
2. **特殊ケースの排除**：`Expr::SpawnFor` および関連特殊処理コードを削除
3. **拡張性**：将来追加される制御フロー構造が自動的に `spawn`
   と組み合わせられ、spawn ロジックの修正不要

### 欠点

1. **破壊的変更**：内部 AST/IR 表現が変化し、`Expr::SpawnFor` を消費するすべてのコードの更新が必要
2. **証明パイプラインの適応が必要**：`SpawnFor`
   削除後、証明パイプラインは AST ディスパッチ（`match body { Expr::For => ..., Expr::While => ... }`）を通じて動作する — この適応は本 RFC の範囲内で DAG 統一エントリによって完了する

## 代替案

| 案                                                                         | 採用しない理由                                                                    |
| -------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| `spawn for` を独立構文として保持                                           | 直交性を損ない、言語で唯一のキーワード組み合わせ特例になる                        |
| `spawn` は `{}` のみ修飾、データ並列は標準ライブラリの `par_iter` に任せる | 言語の原始能力がライブラリに降り、コンパイラ層の DAG 分析とリソース競合検出を失う |

## 計算構造型（独立した RFC に延期）

本 RFC で `SpawnFor` を削除した後、`spawn`
の証明パイプライン統合はアーキテクチャ上の問題に直面する：証明パイプラインは型層で動作し、spawn 内部の計算構造（For/While/Block/If/Call）を知って初めて正しい証明戦略を選択できる。現在、証明パイプラインは AST ディスパッチを通じて動作するが、長期的方向は計算構造を
`MonoType`
バリアント（`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`）としてエンコードし、パイプラインを完全に型層で動作させることである。

これは [RFC-019: 型レベル同像性](./019-typed-homoiconicity.md)
の弱化実用的バージョンである — コンパイラ内蔵の計算構造が型システムに入るが、ユーザー定義構文は開放しない。理論的基盤は ECMTT（Contextual
Modal Types for Algebraic Effects and Handlers, ICFP 2021）であり、`Spawn<T>` はモーダル演算子 `□`
に対応し、証明パイプラインは handler に対応する。

この機構は spawn に限らず、将来あらゆる effect（純粋計算、IO、fallible）が同じパターンで型システムに入ることができる。spawn は最初の消費者であり、唯一の消費者ではない。

> **独立 RFC は以下を定義する**：6 つの MonoType バリアントの完全なセマンティクス、型検査器の適応戦略、証明パイプラインの型ディスパッチ統一インターフェース、RFC-027 との統合方案。

## 実装戦略

### フェーズ分け

1. **AST + Parser**：`Spawn { body: Box<Expr> }`、`SpawnFor` 削除
2. **DAG 分析統一**：エントリ統合、`TaskSource`
   列挙を統一。単一タスク spawn（`spawn call(x)`、`spawn 42`）はコンパイル警告を生成
3. **IR / Codegen 適応**：`Ir::SpawnFor` 削除、処理パスを統一
4. **Placement 簡素化**：`SpawnFor` 分岐を削除
5. **テスト検証**：既存の `spawn for` テストがすべてパス

### 影響範囲

| ファイル/ディレクトリ                        | 変更                                                    |
| -------------------------------------------- | ------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                | `Spawn` の body を `Box<Expr>` に変更、`SpawnFor` 削除  |
| `frontend/core/parser/pratt/nud.rs`          | `spawn` 処理器を汎用式パーサーに簡素化                  |
| `frontend/core/spawn/analysis.rs`            | エントリ統一、`TaskSource` で Explicit + Iterate を統合 |
| `frontend/core/spawn/placement.rs`           | `SpawnFor` 分岐を削除                                   |
| `middle/core/ir.rs`                          | `Ir::SpawnFor` を削除                                   |
| `middle/` (IR gen, codegen)                  | spawn パスを統一                                        |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | セマンティクス不変、検証パス                            |

### 依存関係

- RFC-024（spawn ブロック並行モデル）— 本 RFC はその直交性拡張
- RFC-010（統一型構文）— 構文統一の基礎

## 設計決定ログ

| 決定                      | 結論                                                | 理由                                                                                           | 日付       |
| ------------------------- | --------------------------------------------------- | ---------------------------------------------------------------------------------------------- | ---------- |
| spawn 修飾範囲            | 任意の式                                            | `spawn for` 特殊ケースの排除                                                                   | 2026-06-16 |
| `spawn while` サポート    | サポート                                            | 構文直交性、実装コスト低。証明パイプラインはイテレーション間依存のケースを拒否する可能性がある | 2026-06-16 |
| `spawn if` セマンティクス | if-else 全体を修飾                                  | `if spawn { }` との区別                                                                        | 2026-06-16 |
| spawn 結合優先度          | 最低（return と同等）                               | 後続の式全体を取り込む                                                                         | 2026-06-16 |
| DAG の for 内部           | for 内部の子式を展開しない                          | 直接子式ルール不変、for 全体を 1 つのタスクソースとする                                        | 2026-06-16 |
| 単一タスク spawn 警告     | `spawn call(x)` / `spawn 42` がコンパイル警告を生成 | 並列効果なし、ユーザーの意図確認のリマインダー                                                 | 2026-08-19 |
| 計算構造型                | 独立した RFC に延期                                 | 汎用機構、spawn に限らず。ECMTT 理論基盤                                                       | 2026-08-19 |

---

## 参考文献

- [RFC-024: spawn ブロックベースの並行モデル](./024-concurrency-model.md)
- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976)
  — 計算構造型の理論的基盤
- [並行モデル仕様](../../reference/language-spec/concurrency.md)
- [spawn for 直交性保留（討議稿）](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## ライフサイクルと帰属

| 状態           | 位置                      | 説明                       |
| -------------- | ------------------------- | -------------------------- |
| **レビュー中** | `docs/design/rfc/review/` | オープンなコミュニティ討議 |
