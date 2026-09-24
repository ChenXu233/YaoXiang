---
title: 'RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの除去'
status: '審査中'
author: '晨煦'
created: '2026-06-16'
updated: '2026-08-19'
issue: '#98'
---

# RFC-032: spawn 統一式修飾

> **本文書は `spawn`
> の構文、AST/IR 再構築を定義する**。実行時の挙動セマンティクス（タスク分解粒度、所有権、スコープ、エラー伝播、リソース型、ネスト）については
> [RFC-024: spawn ベースの並行ランタイムセマンティクス](../accepted/024-concurrency-model.md)
> を参照。
>
> 二つの RFC が協調して `spawn`
> を定義する——024 は「何をするか」に答え、032 は「どう表現するか」に答える。

> **核心的な洞察**：`spawn` は `{}`
> ブロックのみを修飾すべきではない。**任意の式**を修飾できる。`spawn for`
> は特殊な構文ではなく、`spawn` + `for` 式の自然な組み合わせに過ぎない。

## 要約

`spawn` を `spawn { }`（ブロックのみ修飾）から `spawn <expr>`（任意の式を修飾）に拡張する。
`Expr::SpawnFor` を AST から削除し、`Expr::Spawn { body: Expr::For { .. } }`
で自然に置き換える。本 RFC は AST/IR/Parser の整理のみを行い、型システムの変更は含まない。

> **計算構造型（`MonoType` 拡張）は別個の RFC に延期する。** 本 RFC が `SpawnFor`
> の特殊ケースを削除した後、 `spawn`
> の証明パイプライン統合には計算構造を認識する型システムが必要となる——これは汎用的な仕組みであり、spawn に限定されるものではなく、独立した設計に値する。

## 動機

### なぜこの変更が必要か？

現在の `spawn for x in items { body }` は独立したキーワード組み合わせであり、AST には専用の
`Expr::SpawnFor` がある。これは言語の直交性を損なう：

1. **構文の不統一**：`spawn` は `{}` ブロックのみ修飾でき、`spawn for` はハードコードされた例外
2. **直交性の欠如**：`spawn while`、`spawn if` などの組み合わせを自然に表現できない

### 現在の問題

```rust
// AST 中の二つの spawn バリアント
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }
```

## 提案

### 中核設計

`spawn <expr>`：`spawn` は任意の式を修飾する。式の形が DAG のタスク分解方法を決定する。

### ユーザーのメンタルモデル

`spawn` = 「この式を並行処理に持って行く」。式の形が分解方法を決定する：

| 式の形                          | 並行挙動                            |
| ------------------------------- | ----------------------------------- |
| `spawn { a, b, c }`             | `a`、`b`、`c` が独立して並行実行    |
| `spawn for x in items { f(x) }` | N 個のイテレーションが独立並行      |
| `spawn while cond { step() }`   | 各イテレーションが独立タスク        |
| `spawn if c { a } else { b }`   | 選択された分岐全体が spawn ドメイン |
| `spawn call(x)`                 | 呼び出し自体が単一タスク            |
| `spawn 42`                      | 単独の単一タスク                    |

コンパイラが DAG 分析で依存関係を決定し、ランタイムが GMP モデルでスケジュールする——依存のないタスクは作業キューに投げ込まれ、worker が奪い合う。全体は同期的にブロックし、全タスクの完了を待つ。

**Go との違い**：Go の `go` は「投げ出したら知らん」だが、YaoXiang の `spawn`
は「分解して並行実行し、全部終わってから次へ進む」。

### 制御フローの直交性

| 組み合わせ                      | セマンティクス                           | 差異                                 |
| ------------------------------- | ---------------------------------------- | ------------------------------------ |
| `spawn for x in items { body }` | データ並列：各イテレーション＝独立タスク | DAG がイテレーション跨りの依存を分析 |
| `for x in items spawn { body }` | 各イテレーションが spawn ドメインを生成  | イテレーション跨り分析なし           |
| `spawn while cond { body }`     | 条件並列：各イテレーション＝独立タスク   | イテレーション間の依存は条件で保証   |
| `while cond spawn { body }`     | 各イテレーションが spawn ドメインを生成  | 上記と意味は異なるが特殊処理不要     |
| `spawn if c { a } else { b }`   | if-else 全体が単一 spawn ドメイン        | 実行時に条件で分岐選択               |
| `if c spawn { a } else { b }`   | 単一分岐のみ spawn                       | if 式内部に spawn を包む             |

### 除去される複雑性

- ❌ `Expr::SpawnFor` を AST から削除
- ❌ `SpawnForAnalysis` を DAG 分析から削除
- ❌ `spawn for` を Parser で組み合わせキーワードとして特殊処理しない
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

### 2. Parser 層

`spawn` の結合優先度は最低（`return` と同等）であり、後続の式全体を吸収する：

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser の変更：`pratt/nud.rs` において `spawn` は `{` を要求せず、汎用式解析を呼び出す：

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` は組み合わせキーワードとして処理されない——`for` は汎用式パーサが処理して `Expr::For`
を生成し、`spawn` は単にそれを包む。

### 3. DAG 分析層

現在の二つの入口を一つに統合する：

```rust
/// 統一入口：body 式の種類に応じてディスパッチ
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
    /// spawn { a, b, c } — コンパイル時既知の N 個の直接子式
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N 個のタスクはランタイムイテレーションで生成
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for あり、while なし
        condition: Option<Expr>,     // while あり、for なし
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

`SpawnForAnalysis` 構造体を削除。

| body の種類           | タスクへの分解方法                          |
| --------------------- | ------------------------------------------- |
| `Expr::Block`         | 直接子式 → タスクリスト                     |
| `Expr::For`           | 各イテレーション → 単一タスク（データ並列） |
| `Expr::While`         | 各イテレーション → 単一タスク               |
| `Expr::If`            | 選択された分岐全体 → 単一タスク             |
| `Expr::Call` / その他 | 式自体 → 単一タスク                         |

DAG 分析完了後、ランタイムが GMP モデルでスケジュールする——依存のないタスクは作業キューに投げ込まれ、worker が奪い合う。

### 4. IR / Codegen 層

`Ir::SpawnFor` を削除。`Ir::Spawn` に統一し、`TaskSource` 情報を保持する。

HIR → IR 変換は `SpawnAnalysis.source` に基づいてランタイム呼び出しを生成：

- `TaskSource::Explicit(tasks)` → コンパイル時既知のタスクリスト
- `TaskSource::Iterate { .. }` → ランタイム展開（コンパイラ駆動、par_iter ライクだがゼロコスト）

### 5. Placement 層

現在の二つの分岐を一つに統合する：

```rust
// 変更前
Expr::Spawn { body, .. } => self.check_block(body),
Expr::SpawnFor { body, iterable, .. } => {
    self.check_expr(iterable);
    self.check_block(body);
}

// 変更後
Expr::Spawn { body, .. } => self.check_expr(body),   // body は Expr、再帰するだけ
```

### 6. 後方互換性

既存の `spawn for` コードのセマンティクスは変わらず、Parser が `spawn for x in items { body }`
を自動的に `Expr::Spawn { body: Expr::For }`
として解析する。内部表現は変わるが、ユーザーから見える振る舞いは変わらない。

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

**単一タスク spawn の警告**：`spawn call(x)` や `spawn 42`
などの単一式を修飾する場合、DAG 分析がコンパイル警告を生成する：「単一式を修飾する spawn には並行効果がない」。構文は合法だが、ユーザーの意図確認を促す。

## トレードオフ

### 利点

1. **構文の直交性**：`spawn` + 任意の制御フロー = 自然な並行組み合わせ
2. **特殊ケースの除去**：`Expr::SpawnFor` および関連する特殊処理コードを削除
3. **拡張性**：将来新しい制御フロー構造が追加されても、spawn ロジックを変更せず自動的に `spawn`
   と組み合わさる

### 欠点

1. **破壊的変更**：内部 AST/IR 表現が変化し、`Expr::SpawnFor` を消費する全コードの更新が必要
2. **証明パイプラインの適応が必要**：`SpawnFor`
   削除後、証明パイプラインは AST ディスパッチ（`match body { Expr::For => ..., Expr::While => ... }`）で動作する——この適応は本 RFC の範囲内で DAG 統一入口によって達成される

## 代替案

| 案                                                                 | 選択しない理由                                                                        |
| ------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| `spawn for` を独立構文として保持                                   | 直交性を損ない、言語で唯一のキーワード組み合わせ特例になる                            |
| `spawn` は `{}` のみ修飾、データ並列は標準ライブラリ `par_iter` で | 言語の原始能力がライブラリに降り、 compiler レベルの DAG 分析とリソース競合検出を失う |

## 計算構造型（独立 RFC に延期）

本 RFC が `SpawnFor` を削除した後、`spawn`
の証明パイプライン統合はアーキテクチャ上の問題に直面する：証明パイプラインは型層で動作し、正しい証明戦略を選択するには spawn 内部の計算構造（For/While/Block/If/Call）を知る必要がある。現在の証明パイプラインは AST ディスパッチで動作するが、長期的方向は計算構造を
`MonoType`
バリアント（`Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn`）としてエンコードし、パイプラインが完全に型層で動作するようにすることである。

これは [RFC-019: 型レベル同像性](../draft/019-typed-homoiconicity.md)
の弱化実用版である——コンパイラ内蔵の計算構造が型システムに入るが、ユーザーカスタム構文は開放しない。理論的基礎は ECMTT（Contextual
Modal Types for Algebraic Effects and Handlers, ICFP 2021）：`Spawn<T>` はモーダル演算子 `□`
に対応し、証明パイプラインは handler に対応する。

この仕組みは spawn に限定されない——将来あらゆる effect（純粋計算、IO、fallible）も同じパターンで型システムに入り得る。spawn は最初の消費者であり、唯一の消費者ではない。

> **独立 RFC が定義する内容**：6 つの MonoType バリアントの完全なセマンティクス、型検査器の適応戦略、証明パイプラインの型ディスパッチ統一インタフェース、RFC-027 との統合方案。

## 実装戦略

### 段階分け

1. **AST + Parser**：`Spawn { body: Box<Expr> }`、`SpawnFor` を削除
2. **DAG 分析統一**：入口を統合、`TaskSource` を Explicit +
   Iterate にマージ。単一タスク spawn（`spawn call(x)`、`spawn 42`）はコンパイル警告を生成
3. **IR / Codegen 適応**：`Ir::SpawnFor` を削除、処理パスを統一
4. **Placement 簡略化**：`SpawnFor` 分岐を削除
5. **テスト検証**：既存の `spawn for` テストが全て通過

### 影響範囲

| ファイル/ディレクトリ                        | 変更内容                                                |
| -------------------------------------------- | ------------------------------------------------------- |
| `frontend/core/parser/ast.rs`                | `Spawn` body を `Box<Expr>` に変更、`SpawnFor` を削除   |
| `frontend/core/parser/pratt/nud.rs`          | `spawn` 処理器を汎用式解析に簡略化                      |
| `frontend/core/spawn/analysis.rs`            | 入口を統一、`TaskSource` を Explicit + Iterate にマージ |
| `frontend/core/spawn/placement.rs`           | `SpawnFor` 分岐を削除                                   |
| `middle/core/ir.rs`                          | `Ir::SpawnFor` を削除                                   |
| `middle/` (IR gen, codegen)                  | spawn パスを統一                                        |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | セマンティクス不变、通過を検証                          |

### 依存関係

- RFC-024（spawn ブロック並行モデル）— 本 RFC はその直交性拡張
- RFC-010（統一型構文）— 構文統一の基礎

## 設計決定記録

| 決定                      | 決定内容                                            | 理由                                                                                         | 日付       |
| ------------------------- | --------------------------------------------------- | -------------------------------------------------------------------------------------------- | ---------- |
| spawn の修飾範囲          | 任意の式                                            | `spawn for` 特殊ケースを除去                                                                 | 2026-06-16 |
| `spawn while` サポート    | サポート                                            | 構文直交性、実装コスト低。証明パイプラインはイテレーション跨り依存ケースを拒否する可能性あり | 2026-06-16 |
| `spawn if` セマンティクス | if-else 全体を修飾                                  | `if spawn { }` と区別                                                                        | 2026-06-16 |
| spawn 結合優先度          | 最低（return と同等）                               | 後続の式全体を吸収                                                                           | 2026-06-16 |
| DAG の for 内部処理       | for 内部子式を展開しない                            | 直接子式ルール不変、for 全体は単一タスクソース                                               | 2026-06-16 |
| 単一タスク spawn 警告     | `spawn call(x)` / `spawn 42` はコンパイル警告を生成 | 並行効果なし、ユーザーの意図確認を促す                                                       | 2026-08-19 |
| 計算構造型                | 独立 RFC に延期                                     | 汎用仕組み、spawn に限定されず。ECMTT 理論基礎                                               | 2026-08-19 |

---

## 参考文献

- [RFC-024: spawn ブロックベースの並行モデル](../accepted/024-concurrency-model.md)
- [RFC-010: 統一型構文](../accepted/010-unified-type-syntax.md)
- [ECMTT: Contextual Modal Types for Algebraic Effects and Handlers (ICFP 2021)](https://arxiv.org/abs/2103.02976)
  — 計算構造型の理論的基礎
- [並行モデル仕様](../../../reference/language-spec/concurrency.md)
- [spawn for 直交性保留（討論稿）](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## ライフサイクルと帰趣

| 状態       | 位置                      | 説明                     |
| ---------- | ------------------------- | ------------------------ |
| **審査中** | `docs/design/rfc/review/` | オープンコミュニティ討論 |
