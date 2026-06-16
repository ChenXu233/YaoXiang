---
title: "RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの排除"
status: "ドラフト"
author: "晨煦"
created: "2026-06-16"
updated: "2026-06-16"
---

# RFC-032: spawn 統一式修飾

> **核心となる洞察**：`spawn` は `{}` ブロックだけを修飾すべきではない。**任意**の式を修飾できる。`spawn for` は特殊な構文ではなく、`spawn` + `for` 式の自然な組み合わせである。

## 概要

`spawn` を `spawn { }`（ブロックのみ修飾）から `spawn <expr>`（任意式を修飾）へと拡張する。`Expr::SpawnFor` は AST から削除され、`Expr::Spawn { body: Expr::For { .. } }` で自然に置き換えられる。式構造の型（Block、For、While、If など）は新しい `MonoType` バリアントとして型システムに入り、`Spawn<T>` が並行実行される計算構造をラップし、コンパイル時にマークされ、型チェック後に消去される。

## 動機

### なぜこの変更が必要なのか？

現在の `spawn for x in items { body }` は独立したキーワードの組み合わせであり、AST には `Expr::SpawnFor` が専用の表現として存在する。これは言語の直交性を損なう：

1. **構文が不統一**：`spawn` は `{}` ブロックしか修飾できず、`spawn for` はハードコードされた例外
2. **直交性の欠如**：`spawn while`、`spawn if` などの組み合わせを自然に表現できない
3. **型システム不完全**：spawn が型システムで見えず、型リフレクションで並行構造を取得できない

### 現状の問題

```rust
// AST における 2 つの spawn バリアント
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }

// MonoType には値型しかなく、計算構造型がない
// spawn { a, b } の型 = Tuple(T_a, T_b)  ← 「これは spawn」という情報が失われる
// spawn for    の型 = List(T)             ← 「これはデータ並列」という情報が失われる
```

## 提案

### 中核設計

`spawn <expr>`：`spawn` は任意式を修飾する。式の形が DAG によるタスク分解方法を決定する。

**すべてが型である**：`MonoType` を「値型」から「値型 + 計算構造型」へと拡張する。重要な式構造はそれぞれ対応する `MonoType` バリアントを持つ。`Spawn<T>` は並行実行される計算構造をラップする。

### ユーザーのメンタルモデル

`spawn` = 「この式を取り出して並行に実行する」。式の形が分解方法を決める：

| 式の形 | 並行動作 | 型 |
|--------|---------|------|
| `spawn { a, b, c }` | `a`、`b`、`c` が独立に並列実行 | `Spawn(Block(Tuple(T_a, T_b, T_c)))` |
| `spawn for x in items { f(x) }` | N 個のイテレーションが独立に並列実行 | `Spawn(ForExpr { body_ty: List(T) })` |
| `spawn while cond { step() }` | 各イテレーションが独立タスク | `Spawn(WhileExpr { body_ty: List(T) })` |
| `spawn if c { a } else { b }` | 選択されたブランチ全体が spawn ドメイン | `Spawn(IfExpr { then_ty: T_a, else_ty: Some(T_b) })` |
| `spawn call(x)` | 呼び出し自体が 1 つのタスク | `Spawn(Call { fn_ty: Fn(A→R), result_ty: R })` |
| `spawn 42` | 単独のタスク | `Spawn(Int)` |

コンパイラが DAG 分析により依存関係を決定し、ランタイムが GMP モデルでスケジューリングする — 依存のないタスクは作業キューに投げ込まれ、worker が競って実行する。全体を同期的にブロックし、すべてのタスクの完了を待つ。

**Go との違い**：Go の `go` は「投げ出して放置」だが、YaoXiang の `spawn` は「分解して並列実行し、すべて完了するまで待つ」である。

### 制御フローの直交性

| 組み合わせ | セマンティクス | 差異 |
|------|------|------|
| `spawn for x in items { body }` | データ並列：各イテレーション = 独立タスク | DAG がイテレーション間の依存を分析 |
| `for x in items spawn { body }` | 各イテレーションが spawn ドメインを生成 | イテレーション間の分析はしない |
| `spawn while cond { body }` | 条件並列：各イテレーション = 独立タスク | イテレーション間の依存は条件で保証 |
| `while cond spawn { body }` | 各イテレーションが spawn ドメインを生成 | 上記とはセマンティクスが異なるが、特別な処理は不要 |
| `spawn if c { a } else { b }` | if-else 全体が 1 つの spawn ドメイン | 実行時に条件に応じてブランチ選択 |
| `if c spawn { a } else { b }` | 単一ブランチのみ spawn | if 式内部に spawn を包む |

### 排除される複雑さ

- ❌ `Expr::SpawnFor` を AST から削除
- ❌ `SpawnForAnalysis` を DAG 分析から削除
- ❌ `spawn for` を Parser で組み合わせキーワードとして特別扱いしない
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

**if の特殊ケース**：

| 書き方 | AST 構造 |
|------|---------|
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }` |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

両者はセマンティクスが異なるが、どちらも自然な組み合わせであり、特別なルールは不要。

### 2. Parser レイヤ

`spawn` の結合優先度は最低（`return` と同等）であり、後続の式全体を吸収する：

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser の変更：`pratt/nud.rs` において `spawn` は `{` を要求せず、汎用式解析を呼び出す：

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` は組み合わせキーワードとして扱わない — `for` は汎用式パーサが処理して `Expr::For` を生成し、`spawn` は単にそれをラップする。

### 3. 型システム

**新規 `MonoType` バリアント：**

```rust
// ========== 計算構造型 ==========

/// {} ブロック式
Block(Box<MonoType>),

/// for ループ式
ForExpr { body_ty: Box<MonoType> },

/// while ループ式
WhileExpr { body_ty: Box<MonoType> },

/// if-else ブランチ式
IfExpr {
    then_ty: Box<MonoType>,
    else_ty: Option<Box<MonoType>>,
},

/// 関数呼び出し式
Call {
    fn_ty: Box<MonoType>,
    result_ty: Box<MonoType>,
},

/// spawn 並行ラッパー：内部式が並行実行される
/// コンパイル時にマークされ、型チェック後に消去される
Spawn(Box<MonoType>),
```

**型推論ルール**：各式の型推論は「計算構造型」を返す。`Spawn` ラップなし = 逐次実行、`Spawn` ラップあり = 並行実行。型チェック完了後、`Spawn` は消去され、型は内部の値型に降格する。

**型チェックフロー**：
1. body 式の型 T（計算構造型）を推論
2. spawn ラップされていれば、`Spawn(T)` でラップ
3. 代入推論時に分解：`results: List(Data) = spawn for ... {}` — `Spawn(ForExpr { body_ty: List(Data) })` から `List(Data)` を抽出

`Spawn<T>` は型チェック完了後に消去され、ランタイムはデータが並行由来か逐次由来かを知る必要がない。ただし、コンパイル時リフレクション（`type_of(x)`）により、完全な並行トポロジ構造を取得できる。

### 4. DAG 分析レイヤ

現在の 2 つのエントリを 1 つに統合：

```rust
/// 統一エントリ：body 式の種類に応じてディスパッチ
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
    /// spawn { a, b, c } — コンパイル時既知の N 個の直接子式
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N 個のタスクはランタイムイテレーションで生成
    Iterate {
        kind: IterKind,
        iter_var: String,
        iterable: Option<Expr>,      // for のみあり、while はなし
        condition: Option<Expr>,     // while のみあり、for はなし
        body: Block,
        reads: HashSet<String>,
        writes: HashSet<String>,
        resource_vars: HashSet<String>,
    },
}

enum IterKind { For, While }
```

`SpawnForAnalysis` 構造体を削除。

| body の種類 | タスクへの分解方法 |
|-----------|--------------|
| `Expr::Block` | 直接子式 → タスクリスト |
| `Expr::For` | 各イテレーション → 1 タスク（データ並列） |
| `Expr::While` | 各イテレーション → 1 タスク |
| `Expr::If` | 選択されたブランチ全体 → 1 タスク |
| `Expr::Call` / その他 | 式自体 → 1 タスク |

DAG 分析完了後、ランタイムが GMP モデルでスケジューリングする — 依存のないタスクは作業キューに投げ込まれ、worker が競って実行する。

### 5. IR / Codegen レイヤ

`Ir::SpawnFor` を削除。`Ir::Spawn` に統一し、`TaskSource` 情報を保持する。

HIR → IR 翻訳は `SpawnAnalysis.source` に基づいてランタイム呼び出しを生成：
- `TaskSource::Explicit(tasks)` → コンパイル時既知のタスクリスト
- `TaskSource::Iterate { .. }` → ランタイム展開（コンパイラ駆動、par_iter ライクだがゼロコスト）

### 6. Placement レイヤ

現在の 2 つのブランチを 1 つに統合：

```rust
// 変更前
Expr::Spawn { body, .. } => self.check_block(body),
Expr::SpawnFor { body, iterable, .. } => {
    self.check_expr(iterable);
    self.check_block(body);
}

// 変更後
Expr::Spawn { body, .. } => self.check_expr(body),   // body は Expr なので再帰するだけ
```

### 7. 後方互換性

既存の `spawn for` コードのセマンティクスは変わらず、Parser が `spawn for x in items { body }` を自動的に `Expr::Spawn { body: Expr::For }` として解析する。内部表現は変化するが、ユーザーから見える振る舞いは変わらない。

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

## トレードオフ

### 利点

1. **構文の直交性**：`spawn` + 任意制御フロー = 自然な並行組み合わせ
2. **すべてが型である**：型システムが計算構造を完全に記録し、コンパイル時リフレクションで並行トポロジを取得
3. **特殊ケースの排除**：`Expr::SpawnFor` および関連特殊処理コードを削除
4. **拡張性**：将来の新規制御フロー構造が自動的に `spawn` と組み合わせ可能、spawn ロジックの変更不要

### 欠点

1. **型システムの膨張**：6 個の `MonoType` バリアントを新規追加し、型チェックの複雑度が増す
2. **破壊的変更**：内部 AST/IR 表現が変化し、`Expr::SpawnFor` を消費するすべてのコードの更新が必要
3. **式の型推論**：各式が計算構造型を返す必要があり、影響範囲が大きい

## 代替案

| 案 | 選択しない理由 |
|------|-------------|
| `spawn for` を独立構文として維持 | 直交性を損ない、言語で唯一のキーワード組み合わせ特例になる |
| `spawn` は `{}` のみ修飾し、データ並列は標準ライブラリ `par_iter` で実現 | 言語の原始能力がライブラリに降格し、コンパイラレベルの DAG 分析とリソース競合検出を失う |
| `SpawnFor` 削除のみで、型システムに計算構造型を導入しない | 型システムがリフレクション能力を失い、spawn が型レベルで見えなくなる |

## 実装戦略

### 段階区分

1. **AST + Parser**：`Spawn { body: Box<Expr> }`、`SpawnFor` を削除
2. **型システム**：6 個の `MonoType` バリアントを新規追加、すべての式の型推論が計算構造型を返すように
3. **DAG 分析の統一**：エントリを統合、`TaskSource` を Explicit + Iterate にマージ
4. **IR / Codegen 適応**：`Ir::SpawnFor` を削除、処理パスを統一
5. **Placement 簡素化**：`SpawnFor` ブランチを削除
6. **テスト検証**：既存の `spawn for` テストがすべてパス

### 影響範囲

| ファイル/ディレクトリ | 変更内容 |
|-----------|------|
| `frontend/core/parser/ast.rs` | `Spawn` の body を `Box<Expr>` に変更、`SpawnFor` を削除 |
| `frontend/core/parser/pratt/nud.rs` | `spawn` ハンドラを汎用式解析に簡素化 |
| `frontend/core/types/mono.rs` | `Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn` バリアントを新規追加 |
| `frontend/core/spawn/analysis.rs` | エントリを統一、`TaskSource` をマージ |
| `frontend/core/spawn/placement.rs` | `SpawnFor` ブランチを削除 |
| `frontend/core/typecheck/` | すべての式ノードを計算構造型推論に適応 |
| `middle/core/ir.rs` | `Ir::SpawnFor` を削除 |
| `middle/` (IR gen, codegen) | spawn パスを統一、`Spawn` 型を消去 |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | セマンティクス変化なし、パスを検証 |

### 依存関係

- RFC-024（spawn ブロック並行モデル）— 本 RFC はその直交性拡張
- RFC-010（統一型構文）— 型システム変更の基礎

## 設計決定の記録

| 決定 | 決定内容 | 理由 | 日付 |
|------|------|------|------|
| spawn の修飾範囲 | 任意式 | `spawn for` の特殊ケースを排除 | 2026-06-16 |
| `spawn while` サポート | サポート | 構文の直交性、実装コスト低 | 2026-06-16 |
| `spawn if` のセマンティクス | if-else 全体を修飾 | `if spawn { }` と区別 | 2026-06-16 |
| 型システム | 計算構造型を導入 | 「すべてが型である」、コンパイル時リフレクションをサポート | 2026-06-16 |
| spawn 型の消去 | 型チェック後に消去 | ランタイムは並行構造情報を必要としない | 2026-06-16 |
| spawn の結合優先度 | 最低（return と同等） | 後続の式全体を吸収 | 2026-06-16 |
| for 内部の DAG | for 内部子式を展開しない | 直接子式ルールは不変、for 全体が 1 つのタスクソース | 2026-06-16 |

---

## 参考文献

- [RFC-024: spawn ブロックベースの並行モデル](./024-concurrency-model.md)
- [並行モデル仕様](../../reference/language-spec/concurrency.md)
- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [spawn for 直交性の保留（討議稿）](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## ライフサイクルと帰結

| 状態 | 位置 | 説明 |
|------|------|------|
| **ドラフト** | `docs/design/rfc/draft/` | 著者の草稿 |