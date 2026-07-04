---
title: "RFC-032: spawn 統一式修飾 — spawn for 特殊ケースの除去"
status: "レビュー中"
author: "晨煦"
created: "2026-06-16"
updated: "2026-07-03"
---

# RFC-032: spawn 統一式修飾

> **核心洞察**：`spawn` は `{}` ブロックのみを修飾するべきではない。**任意式**を修飾できる。`spawn for` は特殊構文ではなく、`spawn` + `for` 式の自然な組み合わせである。

## 概要

`spawn` を `spawn { }`(ブロックのみ修飾)から `spawn <expr>`(任意式修飾)に拡張する。`Expr::SpawnFor` を AST から削除し、`Expr::Spawn { body: Expr::For { .. } }` で自然に置き換える。式構造の型(Block、For、While、If など)が新しい `MonoType` バリアントとして型システムに入り、`Spawn<T>` は並列実行される計算構造をラップし、コンパイル時にマークされ、型チェック後に消去される。

## 動機

### なぜこの変更が必要か？

現在の `spawn for x in items { body }` は独立したキーワード組み合わせであり、AST には `Expr::SpawnFor` という専用表現が存在する。これは言語の直交性を破壊している：

1. **構文の不統一**：`spawn` は `{}` ブロックのみを修飾でき、`spawn for` はハードコードされた例外である
2. **直交性の欠如**：`spawn while`、`spawn if` などの組み合わせを自然に表現できない
3. **型システムの不完全性**：spawn が型システムに見えず、型リフレクションで並列構造を取得できない

### 現在の問題

```rust
// AST 中的两个 spawn 变体
Spawn { body: Box<Block>, span: Span },         // spawn { ... }
SpawnFor { var, var_mut, iterable, body, span },  // spawn for x in items { ... }

// MonoType 只有值类型，没有计算结构类型
// spawn { a, b } 类型 = Tuple(T_a, T_b)  ← 丢失了"这是 spawn"的信息
// spawn for    类型 = List(T)             ← 丢失了"这是数据并行"的信息
```

## 提案

### 核心設計

`spawn <expr>`：`spawn` は任意式を修飾する。式の形が DAG のタスク分解を決定する。

**すべてが型である**：`MonoType` を「値型」から「値型 + 計算構造型」に拡張する。各重要な式構造が型システムに対応する型バリアントを持つ。`Spawn<T>` は並列実行される計算構造をラップする。

### ユーザーのメンタルモデル

`spawn` = 「この式を並列実行に持ち込む」。式の形が分解方法を決定する：

| 式の形 | 並列動作 | 型 |
|---------|---------|------|
| `spawn { a, b, c }` | `a`、`b`、`c` が独立並列 | `Spawn(Block(Tuple(T_a, T_b, T_c)))` |
| `spawn for x in items { f(x) }` | N 個のイテレーションが独立並列 | `Spawn(ForExpr { body_ty: List(T) })` |
| `spawn while cond { step() }` | 各ラウンドのイテレーションが独立タスク | `Spawn(WhileExpr { body_ty: List(T) })` |
| `spawn if c { a } else { b }` | 選択された分岐全体が spawn ドメイン | `Spawn(IfExpr { then_ty: T_a, else_ty: Some(T_b) })` |
| `spawn call(x)` | 呼び出し自体が1つのタスク | `Spawn(Call { fn_ty: Fn(A→R), result_ty: R })` |
| `spawn 42` | 単独タスク | `Spawn(Int)` |

コンパイラは DAG 分析で依存関係を決定し、ランタイムは GMP モデルでスケジューリングする——依存関係のないタスクは作業キューに投げ込まれ、worker が奪い合う。全体が同期ブロックし、すべてのタスクの完了を待つ。

**Go との違い**：Go の `go` は「投げ出して忘れる」だが、YaoXiang の `spawn` は「分解して並列実行、すべて完了してから次へ進む」である。

### 制御フローの直交性

| 組み合わせ | セマンティクス | 差異 |
|------|------|------|
| `spawn for x in items { body }` | データ並列：各イテレーション = 独立タスク | DAG がイテレーション間の依存を分析 |
| `for x in items spawn { body }` | 各イテレーションが spawn ドメインを作成 | イテレーション間の分析を行わない |
| `spawn while cond { body }` | 条件並列：各イテレーション = 独立タスク | イテレーション間の依存は条件で保証 |
| `while cond spawn { body }` | 各イテレーションが spawn ドメインを作成 | 上記とはセマンティクスが異なるが特殊処理は不要 |
| `spawn if c { a } else { b }` | if-else 全体が1つの spawn ドメイン | 実行時に条件で分岐を選択 |
| `if c spawn { a } else { b }` | 単一分岐のみ spawn | if 式内部に spawn を包む |

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
Spawn { body: Box<Expr>, span: Span },           // spawn <任意式>
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

両者はセマンティクスが異なるがどちらも自然な組み合わせであり、特殊ルールは不要。

### 2. Parser 層

`spawn` の結合優先度は最低(`return` と同じ)で、後続の式全体を吸収する：

```
spawn a + b        →  spawn (a + b)         ≠  (spawn a) + b
spawn f(x).y       →  spawn (f(x).y)
```

Parser の変更：`pratt/nud.rs` において `spawn` は `{` を要求せず、汎用式解析を呼び出す：

```
token spawn → parse_expr(min_precedence) → Expr::Spawn { body: expr }
```

`spawn for` は組み合わせキーワードとして処理しない——`for` は汎用式パーサーで処理されて `Expr::For` を生成し、`spawn` はラップのみを担当する。

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

/// spawn 並列ラッパー：内部式が並列実行される
/// コンパイル時にマークされ、型チェック後に消去
Spawn(Box<MonoType>),
```

**型推論ルール**：各式の型推論は「計算構造型」を返す。`Spawn` ラッパーなし = 順次実行、`Spawn` ラッパーあり = 並列実行。型チェック完了後、`Spawn` は消去され、型は内部の値型に降格する。

**型チェックフロー**：
1. body 式の型 T(計算構造型)を推論する
2. spawn で包裹されている場合、`Spawn(T)` でラップする
3. 代入推論時に分解する：`results: List(Data) = spawn for ... {}` — `Spawn(ForExpr { body_ty: List(Data) })` から `List(Data)` を抽出

`Spawn<T>` は型チェック完了後に消去され、ランタイムはデータが並列か順次かを認識する必要がない。しかし、コンパイル時リフレクション(`type_of(x)`)で完全な並列トポロジー構造を取得できる。

### 4. DAG 分析層

現在の2つの入口を1つにマージする：

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

| body 種類 | タスクへの分解方法 |
|-----------|--------------|
| `Expr::Block` | 直接子式 → タスクリスト |
| `Expr::For` | 各イテレーション → 1タスク(データ並列) |
| `Expr::While` | 各イテレーション → 1タスク |
| `Expr::If` | 選択された分岐全体 → 1タスク |
| `Expr::Call` / その他 | 式自体 → 1タスク |

DAG 分析完了後、ランタイムは GMP モデルでスケジューリングする——依存関係のないタスクは作業キューに投げ込まれ、worker が奪い合う。

### 5. IR / Codegen 層

`Ir::SpawnFor` を削除する。`Ir::Spawn` に統一し、`TaskSource` 情報を携帯する。

HIR → IR 翻訳は `SpawnAnalysis.source` に基づいてランタイム呼び出しを生成する：
- `TaskSource::Explicit(tasks)` → コンパイル時既知のタスクリスト
- `TaskSource::Iterate { .. }` → ランタイム展開(コンパイラ駆動、`par_iter` に類似だがゼロコスト)

### 6. Placement 層

現在の2つの分岐を1つにマージする：

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

既存の `spawn for` コードのセマンティクスは変わらず、Parser は自動的に `spawn for x in items { body }` を `Expr::Spawn { body: Expr::For }` に解析する。内部表現は変化するが、ユーザーから見える動作は変わらない。

新しい構文が自然に得られる：
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

1. **構文の直交性**：`spawn` + 任意制御フロー = 自然な並列組み合わせ
2. **すべてが型である**：型システムが計算構造を完全に記録し、コンパイル時リフレクションで並列トポロジーを取得可能
3. **特殊ケースの除去**：`Expr::SpawnFor` および関連特殊処理コードを削除
4. **拡張性**：将来新しい制御フロー構造が追加されても `spawn` と自動組み合わせ可能、`spawn` ロジックの変更不要

### 欠点

1. **型システムの膨張**：6つの `MonoType` バリアントが追加され、型チェックの複雑度が増す
2. **破壊的変更**：内部 AST/IR 表現が変化し、`Expr::SpawnFor` を消費するすべてのコードの更新が必要
3. **式型推論**：各式が計算構造型を返す必要があり、影響範囲が広い

## 代替案

| 代替案 | 選択しない理由 |
|------|-------------|
| `spawn for` を独立構文として維持 | 直交性を破壊し、言語で唯一のキーワード組み合わせ特例になる |
| `spawn` は `{}` のみ修飾、データ並列は標準ライブラリ `par_iter` に任せる | 言語の原始能力がライブラリに降格し、コンパイラレベルの DAG 分析とリソース競合検出を失う |
| `SpawnFor` の削除だけで、型システムに計算構造型を導入しない | 型システムがリフレクション能力を失い、spawn が型レベルで見えなくなる |


## RFC-019 との関係

本 RFC で導入される 6 つの `MonoType` バリアント(Block/ForExpr/WhileExpr/IfExpr/Call/Spawn)は、[RFC-019: 型レベル同像性](./019-typed-homoiconicity.md) の**コンパイラ組み込みサブセット**である。RFC-019 の核心理念「構文構造が型システムに入る」はここで次のように実現される：コンパイラがネイティブに理解する 6 種類の計算構造が対応する型表現を持つ。ユーザーは `SyntaxRule` を通じて新しい計算構造型をカスタマイズできないが、コンパイラ組み込みのこれら 6 種類で主要な制御フローをすべてカバーする。

## 証明パイプライン統合

6 つの `MonoType` バリアントが存在する理由：それらは [RFC-027 コンパイル時証明パイプライン](../accepted/027-compile-time-evaluation-types.md) に**検証対象の命題がどのような形かを伝える**。パイプライン自体が実際の証明作業(自由変数分析、効果分類、別名分析、競合検出)を処理し、MonoType は一つのこと——構造化された入力インターフェースを提供する——だけを行う。

### バリアント → 命題マッピング

| 型 | 命題の形 | 証明戦略 |
|------|---------|---------|
| `Spawn(ForExpr { body_ty })` | データ並列：N 個のイテレーションタスクにイテレーション間競合なし | body の自由変数を抽出 → 効果分類 → Write(Shared) / `&mut`(Shared) がないことをチェック |
| `Spawn(WhileExpr { body_ty })` | 条件並列：各ラウンドのイテレーションが独立 + イテレーション間因果依存なし | 上記 + イテレーション条件にイテレーション間副作用がないかチェック |
| `Spawn(Block(T))` | 明示的タスクグループ：タスク間依存関係は DAG が提供する | DAG 分析の依存グラフを検証——各タスクに必要な入力はその開始時に準備完了 |
| `Spawn(IfExpr { then_ty, else_ty })` | 分岐 spawn：選択された分岐全体が1つの spawn ドメイン | 分岐選択に競合なし、body 内で再帰チェック |
| `Spawn(Call { fn_ty, result_ty })` | 呼び出し spawn：呼び出された関数が独立タスク | 関数の純粋性または分離性を検証 |
| `Spawn(T)`(値、`spawn 42` など) | 単一値 spawn：並列なし | 平凡にパス |

### 証明シナリオ

**シナリオ 1 — 純粋データ並列(パス)：**

```yaoxiang
items = [1, 2, 3, 4, 5]
results = spawn for item in items { item * 2 }
// 型：Spawn(ForExpr { body_ty: List(Int) })
```

1. 自由変数：`item`(ループローカル、各イテレーション独立コピー)、`items`(外部、body 内では読み取りのみ)
2. 効果分類：すべて Read(Local) または Read(Shared)、書き込みなし
3. 証明済み ✓

**シナリオ 2 — 読み取り専用共有(パス)：**

```yaoxiang
config = load_config()
results = spawn for item in items { process(item, config) }
// 型：Spawn(ForExpr { body_ty: List(Result) })
```

1. 自由変数：`item`(Read(Local))、`config`(外部、body 内に書き込みパスなし → Read(Shared))
2. 効果分類：すべて読み取り専用
3. 証明済み ✓

**シナリオ 3 — 書き込み競合(拒否)：**

```yaoxiang
mut counter = 0
spawn for item in items { counter += 1 }
```

1. 自由変数：`item`(Read(Local))、`counter`(外部、`+=` は書き込みに脱糖される)
2. 効果分類：`counter` は Write(Shared)、イテレーション間で同じメモリに書き込み
3. 競合の実例化：`Write(task_0, counter) ∧ Write(task_1, counter) = True`
4. 証明失敗 ✗ → コンパイルエラー：`エラー：spawn for body にイテレーション間書き込み競合が存在。変数 counter が複数の並列タスクで書き込まれている。`

**シナリオ 4 — while + 状態を持つイテレータ(警告/拒否)：**

```yaoxiang
spawn while iter.has_next() {
    item = iter.next()
    process(item)
}
// 型：Spawn(WhileExpr { body_ty: List(Processed) })
```

1. 自由変数：`iter`(外部、`next()` → `&mut self` → `&mut`(Shared))
2. `next()` はイテレータ状態を変更し、イテレーション N+1 はイテレーション N の副作用に依存
3. これは独立タスクではない → `Spawn(WhileExpr)` の独立性制約に違反
4. コンパイラがイテレーション間因果依存を報告、`spawn for` への変更を提案

**シナリオ 5 — spawn if(パス)：**

```yaoxiang
result = spawn if use_cache { load(key) } else { fetch(key) }
// 型：Spawn(IfExpr { then_ty: T, else_ty: Option(T) })
```

1. 1つの分岐のみ実行され、タスク間競合は存在しない
2. body 内にサブ spawn がある場合は再帰チェック
3. 証明済み ✓

**シナリオ 6 — spawn ブロックタスク間依存(DAG + パイプライン検証)：**

```yaoxiang
spawn {
    a = fetch_user(id)
    b = fetch_orders(a.user_id)  // 依赖 a
    c = compute_stats()           // 独立
}
// 型：Spawn(Block(Tuple(User, Orders, Stats)))
```

1. DAG 分析：`a` と `c` は独立(並列可能)、`b` は `a` に依存(`a` 後にスケジューリング)
2. パイプライン検証：`b` の入力(`a.user_id`)は `b` 起動時に計算完了済み
3. 証明済み ✓

### MonoType が行わないこと

| 行うこと | 行わないこと |
|---------|---------|
| 命題の形を識別する | 証明を実行しない |
| 型レベルで計算構造を記録する | DAG 分析を代替しない |
| RFC-027 パイプラインに型入力を提供する | 自由変数分析、別名分析、競合検出を代替しない |

実際の証明作業はコンパイラ標準分析パスで処理される。MonoType の価値はこれらのパスを統一された型フレームワーク下でスケジューリングできることである——証明パイプラインは各 AST ノードに対して特殊分岐を書く必要がない。
## 実装戦略

### 段階区分

1. **AST + Parser**：`Spawn { body: Box<Expr> }`、`SpawnFor` の削除
2. **型システム**：6 つの `MonoType` バリアントを追加、すべての式型推論が計算構造型を返す
3. **DAG 分析統一**：入口のマージ、`TaskSource` の Explicit + Iterate マージ
4. **IR / Codegen 適応**：`Ir::SpawnFor` の削除、統一処理パス
5. **Placement 簡素化**：`SpawnFor` 分岐の削除
6. **テスト検証**：既存の `spawn for` テストがすべてパス

### 影響範囲

| ファイル/ディレクトリ | 変更内容 |
|-----------|------|
| `frontend/core/parser/ast.rs` | `Spawn` の body を `Box<Expr>` に変更、`SpawnFor` を削除 |
| `frontend/core/parser/pratt/nud.rs` | `spawn` ハンドラーを汎用式解析に簡素化 |
| `frontend/core/types/mono.rs` | `Block`/`ForExpr`/`WhileExpr`/`IfExpr`/`Call`/`Spawn` バリアントを追加 |
| `frontend/core/spawn/analysis.rs` | 統一入口、`TaskSource` のマージ |
| `frontend/core/spawn/placement.rs` | `SpawnFor` 分岐の削除 |
| `frontend/core/typecheck/` | すべての式ノードが計算構造型推論に適応 |
| `middle/core/ir.rs` | `Ir::SpawnFor` を削除 |
| `middle/` (IR gen, codegen) | spawn パスの統一、Spawn 型消去 |
| `tests/yaoxiang/04-concurrency/spawn_for.yx` | セマンティクス不変、検証パス |

### 依存関係

- RFC-024(spawn ブロック並列モデル)— 本 RFC はその直交性拡張
- RFC-010(統一型構文)— 型システム変更の基礎
- RFC-027(コンパイル時証明パイプライン)— MonoType バリアントがパイプラインに命題形状入力を提供
- RFC-019(型レベル同像性)— MonoType バリアントがそのコンパイラ組み込みサブセット

## 設計決定記録

| 決定 | 決定内容 | 理由 | 日付 |
|------|------|------|------|
| spawn 修飾範囲 | 任意式 | `spawn for` 特殊ケースの除去 | 2026-06-16 |
| `spawn while` サポート | サポート | 構文の直交性、実装コスト低 | 2026-06-16 |
| `spawn if` セマンティクス | if-else 全体を修飾 | `if spawn { }` と区別 | 2026-06-16 |
| 型システム | 計算構造型の導入 | 「すべてが型」、コンパイル時リフレクションをサポート | 2026-06-16 |
| spawn 型消去 | 型チェック後に消去 | ランタイムは並列構造情報を必要としない | 2026-06-16 |
| spawn 結合優先度 | 最低(return と同じ) | 後続の式全体を吸収 | 2026-06-16 |
| for 内部の DAG | for 内部の子式を展開しない | 直接子式ルールは不変、for 全体が1つのタスクソース | 2026-06-16 |
| 証明パイプライン統合 | MonoType バリアントを RFC-027 証明命題にマッピング | パイプラインは検証対象の命題形状を知る必要があり、MonoType が構造化入力を提供 | 2026-07-03 |
| RFC-019 との関係 | コンパイラ組み込みサブセット | ユーザーはカスタマイズできないが、「構文即型」理念を共有 | 2026-07-03 |
| 証明境界 | 6 つのシナリオでカバー：純粋並列/読み取り専用共有/書き込み競合/while 依存/spawn if/spawn block | 各 MonoType バリアントの証明義務と失敗条件を明確化 | 2026-07-03 |

---

## 参考文献

- [RFC-024: spawn ブロックベースの並列モデル](./024-concurrency-model.md)
- [RFC-010: 統一型構文](./010-unified-type-syntax.md)
- [RFC-027: コンパイル時述語と統一静的検証](../accepted/027-compile-time-evaluation-types.md)
- [RFC-019: 型レベル同像性](./019-typed-homoiconicity.md)
- [並列モデル仕様](../../reference/language-spec/concurrency.md)
- [spawn for 直交性懸案(討論稿)](../../dev/plan/ongoing/spawn-for-orthogonality.md)

---

## ライフサイクルと帰属

| ステータス | 位置 | 説明 |
|------|------|------|
| **レビュー中** | `docs/design/rfc/review/` | オープンコミュニティ討論 |