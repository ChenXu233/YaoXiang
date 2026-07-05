---
title: "RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの排除"
status: "レビュー中"
author: "晨煦"
created: "2026-06-16"
updated: "2026-07-03"
issue: "#98"
---

# RFC-032: spawn 統一式修飾

> **本文書は `spawn` の構文、AST/IR 再構築、型システムの拡張を定義する**。
> 実行時動作セマンティクス（タスク分解の粒度、所有権、スコープ、エラー伝播、リソース型、ネスト）については [RFC-024: spawn に基づく並行実行時セマンティクス](./024-concurrency-model.md) を参照。
>
> 二つの RFC が協調して `spawn` を定義する — 024 は「何をするか」、032 は「どう表現するか」に答える。

> **核心的な洞察**：`spawn` は `{}` ブロックのみを修飾するべきではない。**任意の式**を修飾できる。`spawn for` は特殊な構文ではなく、`spawn` + `for` 式の自然な組み合わせに過ぎない。

## 概要

`spawn` を `spawn { }`（ブロックのみを修飾）から `spawn <expr>`（任意の式を修飾）へ拡張する。`Expr::SpawnFor` を AST から削除し、`Expr::Spawn { body: Expr::For { .. } }` で自然に置き換える。式構造の型（Block、For、While、If など）を新たな `MonoType` バリアントとして型システムに導入し、`Spawn<T>` が並行実行される計算構造をラップし、コンパイル時にマークされ、型チェック後に消去される。


## 動機

### なぜこの変更が必要か？

現在の `spawn for x in items { body }` は独立したキーワード組み合わせであり、AST には `Expr::SpawnFor` がこれを専用に表現している。これは言語の直交性を破壊する：

1. **構文の不統一**：`spawn` は `{}` ブロックのみを修飾でき、`spawn for` はハードコードされた例外
2. **直交性の欠如**：`spawn while`、`spawn if` などの組み合わせを自然に表現できない
3. **型システムの不完全さ**：spawn が型システム上に見えず、並行構造を型リフレクションで取得できない

### 現在の問題

```rust
// AST 中に二つの spawn バリアント
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }

// MonoType は値型のみを持ち、計算構造型を持たない
// spawn { a, b } の型 = Tuple(T_a, T_b)  ← 「これは spawn」という情報が失われる
// spawn for    の型 = List(T)             ← 「これはデータ並列」という情報が失われる
```

## 提案

### コア設計

`spawn <expr>`：`spawn` は任意の式を修飾する。式の形状が DAG のタスク分解方法を決定する。

**すべてが型である**：`MonoType` を「値型」から「値型 + 計算構造型」へ拡張する。各キーとなる式構造が型システムに対応する型バリアントを持つ。`Spawn<T>` が並行実行される計算構造をラップする。

### ユーザのメンタルモデル

`spawn` = 「この式を並行実行せよ」。式の形状が分解方法を決定する：

| 式の形状 | 並行動作 | 型 |
|---------|---------|------|
| `spawn { a, b, c }` | `a`、`b`、`c` が独立に並列実行 | `Spawn(Block(Tuple(T_a, T_b, T_c)))` |
| `spawn for x in items { f(x) }` | N 回の反復が独立に並列実行 | `Spawn(ForExpr { body_ty: List(T) })` |
| `spawn while cond { step() }` | 各反復が独立タスク | `Spawn(WhileExpr { body_ty: List(T) })` |
| `spawn if c { a } else { b }` | 選択された分岐全体が spawn ドメイン | `Spawn(IfExpr { then_ty: T_a, else_ty: Some(T_b) })` |
| `spawn call(x)` | 呼び出し自体が単一タスク | `Spawn(Call { fn_ty: Fn(A→R), result_ty: R })` |
| `spawn 42` | 単独タスク | `Spawn(Int)` |

コンパイラが DAG 分析で依存関係を決定し、実行時は GMP モデルでスケジューリングする — 依存関係のないタスクはワークキューに投入され、worker が奪い合う。全体は同期的にブロックし、全タスクの完了を待つ。

**Go との違い**：Go の `go` は「投げて知らんぷり」、YaoXiang の `spawn` は「分解して並列実行し、すべて完了するまで先に進まない」。

### 制御フローの直交性

| 組み合わせ | セマンティクス | 差異 |
|----------|-------------|------|
| `spawn for x in items { body }` | データ並列：各反復 = 独立タスク | DAG が反復横断で依存関係を分析 |
| `for x in items spawn { body }` | 各反復が spawn ドメインを生成 | 反復横断で分析しない |
| `spawn while cond { body }` | 条件並列：各反復 = 独立タスク | 反復間の依存は条件が保証 |
| `while cond spawn { body }` | 各反復が spawn ドメインを生成 | 上記と意味は異なるが特別な処理は不要 |
| `spawn if c { a } else { b }` | if-else 全体が一つの spawn ドメイン | 実行時は条件に応じて分岐を選択 |
| `if c spawn { a } else { b }` | 単一分岐のみ spawn | if 式内部に spawn を包む |

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

**IF の特殊ケース**：

| 書き方 | AST 構造 |
|------|---------|
| `spawn if cond { a } else { b }` | `Spawn { body: Expr::If { ... } }` |
| `if cond spawn { a } else { b }` | `Expr::If { then: Spawn { body: {a} }, else: {b} }` |

両者のセマンティクスは異なるが、いずれも自然な組み合わせであり、特別なルールは不要。

### 2. Parser レイヤ

`spawn` の結合優先度は最も低く（`return` と同等）、後続の式全体を取り込む：

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser の変更：`pratt/nud.rs` において `spawn` は `{` を要求せず、汎用式解析を呼び出す：

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` は組み合わせキーワードとして扱わない — `for` は汎用式パーサが処理して `Expr::For` を生成し、`spawn` は単にラップする。

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

/// spawn 並行ラッパー：内部式が並行実行される
/// コンパイル時にマークされ、型チェック後に消去される
Spawn(Box<MonoType>),
```

**型推論ルール**：各式の型推論は「計算構造型」を返す。`Spawn` ラッパーなし = 順次実行、`Spawn` ラッパーあり = 並行実行。型チェック完了後 `Spawn` は消去され、型は内部の値型に降格する。

**型チェックフロー**：
1. body 式の型 T（計算構造型）を推論
2. spawn ラッパー付きの場合、`Spawn(T)` でラップ
3. 代入推論時に分解：`results: List(Data) = spawn for ... {}` — `Spawn(ForExpr { body_ty: List(Data) })` から `List(Data)` を抽出

`Spawn<T>` は型チェック完了後に消去され、実行時はデータが並行由来か順次由来かを知る必要がない。しかしコンパイル時リフレクション（`type_of(x)`）は完全な並行トポロジ構造を取得できる。

### 4. DAG 分析レイヤ

現在の二つのエントリを一つに統合：

```rust
/// 統一エントリ：body 式の種別に応じてディスパッチ
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
    /// spawn { a, b, c } — コンパイル時に既知の N 個の直接子式
    Explicit(Vec<TaskInfo>),
    /// spawn for/while — N 個のタスクが実行時の反復で生成される
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

`SpawnForAnalysis` 構造体を削除。

| body の種別 | タスク分解方法 |
|----------|-------------|
| `Expr::Block` | 直接の子式 → タスクリスト |
| `Expr::For` | 各反復 → 一つのタスク（データ並列） |
| `Expr::While` | 各反復 → 一つのタスク |
| `Expr::If` | 選択された分岐全体 → 一つのタスク |
| `Expr::Call` / その他 | 式自体 → 一つのタスク |

DAG 分析完了後、実行時は GMP モデルでスケジューリングする — 依存関係のないタスクはワークキューに投入され、worker が奪い合う。

### 5. IR / Codegen レイヤ

`Ir::SpawnFor` を削除。`Ir::Spawn` に統一し、`TaskSource` 情報を保持する。

HIR → IR 翻訳は `SpawnAnalysis.source` に基づき実行時呼び出しを生成：
- `TaskSource::Explicit(tasks)` → コンパイル時既知のタスクリスト
- `TaskSource::Iterate { .. }` → 実行時に展開（コンパイラ駆動、par_iter に類似するがゼロコスト）

### 6. Placement レイヤ

現在の二つの分岐を一つに統合：

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

### 7. 後方互換性

既存の `spawn for` コードのセマンティクスは変わらず、Parser は `spawn for x in items { body }` を自動的に `Expr::Spawn { body: Expr::For }` として解析する。内部表現は変わるが、ユーザに見えてる動作は変わらない。

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

1. **構文の直交性**：`spawn` + 任意の制御フロー = 自然な並行組み合わせ
2. **すべてが型**：型システムが計算構造を完全に記録し、コンパイル時リフレクションで並行トポロジを取得可能
3. **特殊ケースの排除**：`Expr::SpawnFor` および関連特殊処理コードを削除
4. **拡張性**：将来的に新しい制御フロー構造が追加されても `spawn` ロジックの変更なしで自動組み合わせ

### 欠点

1. **型システムの肥大化**：6 つの新規 `MonoType` バリアントにより型チェックの複雑さが増す
2. **破壊的変更**：内部 AST/IR 表現の変更により、`Expr::SpawnFor` を消費するすべてのコードの更新が必要
3. **式の型推論**：各式が計算構造型を返す必要があり、影響範囲が広い

## 代替案

| 案 | 採用しない理由 |
|------|-------------|
| `spawn for` を独立構文として維持 | 直交性を破壊し、言語で唯一のキーワード組み合わせ特例になる |
| `spawn` は `{}` のみを修飾し、データ並列は標準ライブラリ `par_iter` で提供 | 言語の原始能力がライブラリに降格し、コンパイラレベルの DAG 分析とリソース競合検出を失う |
| `SpawnFor` のみ削除し、型システムに計算構造型を導入しない | 型システムがリフレクション能力を失い、spawn が型レベルで見えなくなる |


## RFC-019 との関係

本 RFC で導入される 6 つの `MonoType` バリアント（Block/ForExpr/WhileExpr/IfExpr/Call/Spawn）は [RFC-019: 型レベル同像性](./019-typed-homoiconicity.md) の**コンパイラ組み込みサブセット**である。RFC-019 の核となる理念「構文構造が型システムに入る」はここで次のように実現される：コンパイラがネイティブに理解する 6 種類の計算構造が対応する型表現を持つ。ユーザは `SyntaxRule` で新しい計算構造型を定義できないが、コンパイラ組み込みのこれら 6 種類がすべての主要制御フローをカバーする。

## 証明パイプライン統合

6 つの `MonoType` バリアントが存在する理由：それらは [RFC-027 コンパイル時証明パイプライン](../accepted/027-compile-time-evaluation-types.md) に対し**検証対象の命題がどんな形状か**を伝える。パイプライン自体が実際の証明作業（自由変数分析、効果分類、別名分析、競合検出）を担い、MonoType は一つのことだけを行う — 構造化された入力インタフェースを提供する。

### バリアント → 命題マッピング

| 型 | 命題の形状 | 証明戦略 |
|------|---------|---------|
| `Spawn(ForExpr { body_ty })` | データ並列：N 個の反復タスクに反復横断の競合なし | body の自由変数を抽出 → 効果分類 → Write(Shared) / `&mut`(Shared) なしをチェック |
| `Spawn(WhileExpr { body_ty })` | 条件並列：各反復が独立 + 反復横断の因果依存なし | 同上 + 反復条件に反復横断の副作用がないかチェック |
| `Spawn(Block(T))` | 明示的タスクグループ：タスク間の依存関係は DAG で与えられる | DAG 分析の依存グラフを検証 — 各タスクが必要とする入力はその開始時点で準備済み |
| `Spawn(IfExpr { then_ty, else_ty })` | 分岐 spawn：選択された分岐全体が一つの spawn ドメイン | 分岐選択に競合なし、body 内で再帰チェック |
| `Spawn(Call { fn_ty, result_ty })` | 呼び出し spawn：呼び出される関数が一つの独立タスク | 関数の純粋性または隔離性を検証 |
| `Spawn(T)`（値、`spawn 42` など） | 単一値 spawn：並行性なし | トリビアルに通過 |

### 証明シナリオ

**シナリオ 1 — 純粋データ並列（通過）：**

```yaoxiang
items = [1, 2, 3, 4, 5]
results = spawn for item in items { item * 2 }
// 型：Spawn(ForExpr { body_ty: List(Int) })
```

1. 自由変数：`item`（ループローカル、各反復で独立コピー）、`items`（外部、body 内では読み取り専用）
2. 効果分類：すべて Read(Local) または Read(Shared)、書き込みなし
3. Proved ✓

**シナリオ 2 — 読み取り専用共有（通過）：**

```yaoxiang
config = load_config()
results = spawn for item in items { process(item, config) }
// 型：Spawn(ForExpr { body_ty: List(Result) })
```

1. 自由変数：`item`（Read(Local)）、`config`（外部、body 内に書き込みパスなし → Read(Shared)）
2. 効果分類：すべて読み取り専用
3. Proved ✓

**シナリオ 3 — 書き込み競合（拒否）：**

```yaoxiang
mut counter = 0
spawn for item in items { counter += 1 }
```

1. 自由変数：`item`（Read(Local)）、`counter`（外部、`+=` は書き込みに脱糖）
2. 効果分類：`counter` は Write(Shared)、反復横断で同じメモリに書き込み
3. インスタンス化による競合：`Write(task_0, counter) ∧ Write(task_1, counter) = True`
4. Disproved ✗ → コンパイルエラー：`エラー：spawn for body 内に反復横断の書き込み競合があります。変数 counter が複数の並行タスクから書き込まれています。`

**シナリオ 4 — while + 状態を持つイテレータ（警告/拒否）：**

```yaoxiang
spawn while iter.has_next() {
    item = iter.next()
    process(item)
}
// 型：Spawn(WhileExpr { body_ty: List(Processed) })
```

1. 自由変数：`iter`（外部、`next()` → `&mut self` → `&mut`(Shared)）
2. `next()` がイテレータ状態を変更し、N+1 回目の反復が N 回目の副作用に依存
3. これは独立タスクではない → `Spawn(WhileExpr)` の独立性制約に違反
4. コンパイラが反復横断の因果依存を報告し、`spawn for` への変更を提案

**シナリオ 5 — spawn if（通過）：**

```yaoxiang
result = spawn if use_cache { load(key) } else { fetch(key) }
// 型：Spawn(IfExpr { then_ty: T, else_ty: Option(T) })
```

1. 一つの分岐のみ実行されるため、タスク横断の競合は存在しない
2. body 内に子 spawn がある場合は再帰チェック
3. Proved ✓

**シナリオ 6 — spawn ブロックのタスク間依存（DAG + パイプライン検証）：**

```yaoxiang
spawn {
    a = fetch_user(id)
    b = fetch_orders(a.user_id)  // a に依存
    c = compute_stats()           // 独立
}
// 型：Spawn(Block(Tuple(User, Orders, Stats)))
```

1. DAG 分析：`a` と `c` は独立（並列可能）、`b` は `a` に依存（`a` の後にスケジューリング）
2. パイプライン検証：`b` の入力（`a.user_id`）は `b` 起動時点で計算完了
3. Proved ✓

### MonoType が行わないこと

| 行うこと | 行わないこと |
|--------|----------|
| 命題の形状を識別 | 証明を実行 |
| 型レベルで計算構造を記録 | DAG 分析を代替 |
| RFC-027 パイプラインに型入力を提供 | 自由変数分析、別名分析、競合検出を代替 |

実際の証明作業はコンパイラの標準分析 pass が担当する。MonoType の価値はこれらの pass を統一された型フレームワーク下でスケジューリングできる点にある — 証明パイプラインが各 AST ノードに対して特別な分岐を書く必要がない。
## 実装戦略

### 段階分け

1. **AST + Parser**：`Spawn { body: Box<Expr> }`、`SpawnFor` を削除
2. **型システム**：6 つの新規 `MonoType` バリアント、全式の型推論が計算構造型を返す
3. **DAG 分析の統一**：エントリを統合、`TaskSource` で Explicit + Iterate をマージ
4. **IR / Codegen 適応**：`Ir::SpawnFor` を削除、処理パスを統一
5. **Placement 簡略化**：`SpawnFor` 分岐を削除
6. **テスト検証**：既存の `spawn for` テストがすべて通過

### 影響範囲

| ファイル/ディレクトリ | 変更内容 |
|-----------|------|
| `frontend/core/parser/ast.rs` | `Spawn` の body を `Box<Expr>` に変更、`SpawnFor` を削除 |
| `frontend/core/parser/pratt/nud.rs` | `spawn` ハンドラを汎用式解析に簡略化 |
| `frontend/core/types/mono.rs` | `Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn` バリアントを追加 |
| `frontend/core/spawn/analysis.rs` | 統一エントリ、`TaskSource` で Explicit + Iterate をマージ |
| `frontend/core/spawn/placement.rs` | `SpawnFor` 分岐を削除 |
| `frontend/core/typecheck/` | 全式ノードを計算構造型推論に適応 |
| `middle/core/ir.rs` | `Ir::SpawnFor` を削除 |
| `middle/` (IR gen, codegen) | spawn パスを統一、Spawn 型を消去 |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | セマンティクス不変、検証通過 |

### 依存関係

- RFC-024（spawn ブロック並行モデル）— 本 RFC はその直交性拡張
- RFC-010（統一型構文）— 型システム変更の基盤
- RFC-027（コンパイル時証明パイプライン）— MonoType バリアントがパイプラインに命題形状入力を提供
- RFC-019（型レベル同像性）— MonoType バリアントはそのコンパイラ組み込みサブセット

## 設計決定記録

| 決定 | 決定内容 | 理由 | 日付 |
|------|------|------|------|
| spawn の修飾範囲 | 任意の式 | `spawn for` の特殊ケースを排除 | 2026-06-16 |
| `spawn while` サポート | サポート | 構文の直交性、実装コスト低 | 2026-06-16 |
| `spawn if` セマンティクス | if-else 全体を修飾 | `if spawn { }` と区別 | 2026-06-16 |
| 型システム | 計算構造型を導入 | 「すべてが型」、コンパイル時リフレクションをサポート | 2026-06-16 |
| spawn 型の消去 | 型チェック後に消去 | 実行時は並行構造情報を必要としない | 2026-06-16 |
| spawn の結合優先度 | 最低（`return` と同等） | 後続の式全体を取り込む | 2026-06-16 |
| for 内部の DAG | for 内部の子式を展開しない | 直接子式のルール不変、for 全体が一つのタスクソース | 2026-06-16 |
| 証明パイプライン統合 | MonoType バリアントを RFC-027 証明命題にマッピング | パイプラインは検証対象の命題形状を知る必要があり、MonoType が構造化入力を提供 | 2026-07-03 |
| RFC-019 との関係 | コンパイラ組み込みサブセット | ユーザはカスタム定義不可だが「構文＝型」理念を共有 | 2026-07-03 |
| 証明境界 | 6 シナリオでカバー：純粋並列/読み取り専用共有/書き込み競合/while 依存/spawn if/spawn block | 各 MonoType バリアントの証明義務と失敗条件を明確化 | 2026-07-03 |

---

## 参考文献

- [RFC-024: spawn ブロックベースの並行モデル](./024-concurrency-model.md)
- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-027: コンパイル時述語と統一静的検証](../accepted/027-compile-time-evaluation-types.md)
- [RFC-019: 型レベル同像性](./019-typed-homoiconicity.md)
- [並行モデル仕様](../../reference/language-spec/concurrency.md)
- [spawn for 直交性保留（討論稿）](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## ライフサイクルと帰属

| 状態 | 場所 | 説明 |
|------|------|------|
| **レビュー中** | `docs/design/rfc/review/` | オープンコミュニティ討論 |