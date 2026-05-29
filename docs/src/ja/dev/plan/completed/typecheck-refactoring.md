# 型検査フローの完全リファクタリング計画

> **状態**：✅ 完了  
> **完了日**：2025-07  
> **テスト結果**：1469個のテストがすべて通過（1434 + 30 + 5）、0失敗

## コア目標

技術的負債を一掃し、型検査フローを明確化・簡素化・拡張可能にしつつ、既存の特性モジュールとの良好な協調を維持する。

---

## 既存モジュール構造の分析（リファクタリング前）

> 以下はリファクタリング前の構造であり、参考のみとする。

```
src/frontend/typecheck/
├── mod.rs                      # 統一エントリポイント
├── checking/                   # ❌ 問題：inference と責務が重叠
│   ├── mod.rs                 # BodyChecker, AssignmentChecker, SubtypeChecker...
│   ├── assignment.rs
│   ├── bounds.rs
│   ├── compatibility.rs
│   └── subtyping.rs
├── inference/                  # ❌ 問題：checking と責務が重叠
│   ├── mod.rs
│   ├── expressions.rs          # ExprInferrer
│   ├── generics.rs
│   ├── patterns.rs
│   └── statements.rs
├── specialization/             # ✅ 維持（独立した特性）
├── traits/                     # ✅ 維持（独立した特性）
├── gat/                        # ✅ 維持（独立した特性）
├── tests/                      # ✅ 維持
├── overload.rs                 # ✅ 維持（独立した特性）
├── type_eval.rs                # ✅ 維持（独立した特性）
└── specialize.rs              # ✅ 維持（互換性）
```

**問題**：`checking/` と `inference/` は実際は同じことをしているが、二つのディレクトリに分割されている！

---

## リファクタリング方案：checking/ を inference/ に統合

### ディレクトリ構造（リファクタリング後）

```
src/frontend/typecheck/
├── mod.rs                  # 統一エントリポイント、全モジュールをエクスポート
│
# ✅ 統合後のコアモジュール inference/
├── inference/
│   ├── mod.rs             # エクスポート + TypeChecker のメインエントリ
│   ├── scope.rs           # 🆕 統一スコープ管理
│   ├── types.rs           # 🆕 型システムツール
│   ├── statements.rs      # 🆕 文の検査（checking + inference の文部分を統合）
│   ├── expressions.rs     # 🆕 式の推論（既存の expressions.rs を統合）
│   #
│   # ✅ checking/ から移動
│   ├── assignment.rs      # 代入検査
│   ├── subtyping.rs       # サブタイピング検査
│   ├── compatibility.rs   # 互換性検査
│   ├── bounds.rs          # 境界検査
│   #
│   # ✅ 維持（強化）
│   ├── generics.rs        # 泛型推論
│   └── patterns.rs        # パターン推論
│
# ✅ 維持：独立特性モジュール（変更なし、インターフェースを通じて呼び出し）
├── specialization/         # 特化ロジック
├── traits/                # 特質ロジック
├── gat/                   # GAT ロジック
├── overload.rs            # オーバーロード解決
├── type_eval.rs           # 型評価
├── specialize.rs          # 互換性
│
# ❌ 删除 checking/ ディレクトリ
└── tests/                  # テスト
```

### モジュール責務の分割

| モジュール | 責務 | 説明 |
|------|------|------|
| `inference/scope.rs` | 変数スコープの統一管理 | 全変数の追加・削除・変更・参照 |
| `inference/types.rs` | 型ツール | unify、infer_element_type など |
| `inference/statements.rs` | 文の検査 | Var、Fn、For、If、Expr などの文 |
| `inference/expressions.rs` | 式の推論 | Lit、Var、BinOp、Call、For などの式 |
| `inference/assignment.rs` | 代入検査 | checking/ から移動 |
| `inference/subtyping.rs` | サブタイピング検査 | checking/ から移動 |
| `inference/compatibility.rs` | 互換性検査 | checking/ から移動 |
| `inference/bounds.rs` | 境界検査 | checking/ から移動 |
| `specialization/*` | 特化 | 独立プラグイン |
| `traits/*` | 特質 | 独立プラグイン |
| `gat/*` | GAT | 独立プラグイン |
| `overload.rs` | オーバーロード解決 | 独立プラグイン |

### 重要な設計原則

1. **单一入口**：`inference/` は唯一の型推論エントリポイント
2. **ScopeManager 单一インスタンス**：整个検査フローで同一个 ScopeManager を共有
3. **特性モジュールの独立性**：specialization/traits/gat/overload はプラグインとして呼び出される
4. **重複コードなし**：BodyChecker と ExprInferrer の重複 scopes を削除

---

## 詳細設計

### inference/scope.rs - 統一スコープ管理

```rust
/// スコープマネージャー
/// 单一責務：変数スコープスタックを管理
pub struct ScopeManager {
    scopes: Vec<HashMap<String, PolyType>>,
}

impl ScopeManager {
    pub fn new() -> Self
    pub fn enter_scope(&mut self)
    pub fn exit_scope(&mut self)
    pub fn add_var(&mut self, name: String, poly: PolyType)
    pub fn get_var(&self, name: &str) -> Option<&PolyType>
    pub fn update_var(&mut self, name: &str, poly: PolyType)
    pub fn var_in_current_scope(&self, name: &str) -> bool
    pub fn var_in_any_scope(&self, name: &str) -> bool
}
```

### inference/types.rs - 型システムツール

```rust
/// 型システムツール
pub struct TypeSystem;

impl TypeSystem {
    /// 2つの型を統合する
    pub fn unify(ty1: &MonoType, ty2: &MonoType, solver: &mut TypeConstraintSolver) -> Result<(), Box<Diagnostic>>

    /// umerable 型から要素型を推論する
    pub fn infer_element_type(iter_ty: &MonoType) -> MonoType

    /// リスト型を構成する
    pub fn make_list_type(elem_ty: MonoType) -> MonoType

    /// 型がumerable かどうかチェック
    pub fn is_iterable(ty: &MonoType) -> bool

    /// 特質モジュールを呼び出して特質制約をチェック
    pub fn check_trait_bounds(ty: &MonoType, bounds: &[TraitBound], trait_table: &TraitTable) -> Result<(), Box<Diagnostic>>

    /// 特化モジュールを呼び出してインスタンス化
    pub fn instantiate(ty: &MonoType, args: &[MonoType]) -> Result<MonoType, Box<Diagnostic>>
}
```

### inference/statements.rs - 文の検査

```rust
use crate::inference::scope::ScopeManager;
use crate::inference::types::TypeSystem;
use crate::inference::assignment::AssignmentChecker;
use crate::inference::subtyping::SubtypeChecker;

/// 文チェッカー
pub struct StatementChecker<'a> {
    scope: &'a mut ScopeManager,
    solver: &'a mut TypeConstraintSolver,
    type_system: &'a TypeSystem,
}

impl<'a> StatementChecker<'a> {
    pub fn new(scope: &'a mut ScopeManager, solver: &'a mut TypeConstraintSolver) -> Self

    pub fn check(&mut self, stmt: &Stmt) -> Result<(), Box<Diagnostic>> {
        match &stmt.kind {
            StmtKind::Var { .. } => self.check_var(),
            StmtKind::Fn { .. } => self.check_fn(),
            StmtKind::For { .. } => self.check_for(),
            StmtKind::If { .. } => self.check_if(),
            StmtKind::Expr { .. } => self.check_expr_stmt(),
            // ...
        }
    }

    fn check_var(&mut self, name: &str, init: Option<&Expr>, annot: Option<&Type>) -> Result<(), Box<Diagnostic>>
    fn check_fn(&mut self, ...) -> Result<(), Box<Diagnostic>>
    fn check_for(&mut self, ...) -> Result<(), Box<Diagnostic>>
}
```

### inference/expressions.rs - 式の推論

```rust
use crate::inference::scope::ScopeManager;
use crate::inference::types::TypeSystem;

/// 式推論器（統一 ScopeManager を使用）
pub struct ExpressionInferrer<'a> {
    scope: &'a ScopeManager,  // 読み取り専用参照
    solver: &'a mut TypeConstraintSolver,
    type_system: &'a TypeSystem,
}

impl<'a> ExpressionInferrer<'a> {
    pub fn infer(&mut self, expr: &Expr) -> Result<MonoType, Box<Diagnostic>> {
        match expr {
            Expr::Lit(..) => self.infer_literal(),
            Expr::Var(..) => self.infer_var(),
            Expr::BinOp(..) => self.infer_binop(),
            Expr::Call(..) => self.infer_call(),
            Expr::For(..) => self.infer_for(),
            Expr::Lambda(..) => self.infer_lambda(),
            // ...
        }
    }

    fn infer_literal(&mut self, lit: &Literal) -> Result<MonoType, Box<Diagnostic>>
    fn infer_var(&mut self, name: &str, span: Span) -> Result<MonoType, Box<Diagnostic>>
    fn infer_binop(&mut self, op: &BinOp, left: &Expr, right: &Expr) -> Result<MonoType, Box<Diagnostic>>
}
```

### inference/mod.rs - 統一エントリポイント

```rust
// 全モジュールをエクスポート
pub mod scope;
pub mod types;
pub mod statements;
pub mod expressions;
pub mod assignment;
pub mod subtyping;
pub mod compatibility;
pub mod bounds;
pub mod generics;
pub mod patterns;

pub use scope::ScopeManager;
pub use types::TypeSystem;
pub use statements::StatementChecker;
pub use expressions::ExpressionInferrer;
pub use assignment::AssignmentChecker;
pub use subtyping::SubtypeChecker;
pub use compatibility::CompatibilityChecker;
pub use bounds::BoundsChecker;

// 統一型チェッカーのエントリポイント
pub struct TypeChecker {
    scope: ScopeManager,
    solver: TypeConstraintSolver,
    type_system: TypeSystem,
    // 特性モジュール参照
    trait_table: TraitTable,
    specialization_context: SpecializationContext,
}

impl TypeChecker {
    pub fn new() -> Self

    pub fn check_module(&mut self, module: &Module) -> Result<TypeCheckResult, Vec<Diagnostic>> {
        // 1. 型定義を収集
        // 2. 関数シグネチャを収集
        // 3. 全文をチェック
        // 4. 制約を解く
    }
}
```

---

## リファクタリング手順

### フェーズ 1：scope.rs と types.rs の作成 ✅

**目標**：基盤モジュールの作成

**成果物**：
- ✅ `inference/scope.rs` - ScopeManager（enter_scope/exit_scope/add_var/get_var/update_var/var_in_current_scope/var_in_any_scope/vars/scope_level を含む）
- ✅ `inference/types.rs` - TypeSystem（unify/infer_element_type/make_list_type/is_iterable を含む）

### フェーズ 2：statements.rs の作成 ✅

**目標**：BodyChecker + StmtInferrer の文検査ロジックの統合

**成果物**：
- ✅ `inference/statements.rs` - StatementChecker（861行、完全な文検査ロジックを含む）

**実装詳細**：
- StatementChecker は `scope: ScopeManager` と `solver: TypeConstraintSolver` を所有
- `check_expr()` は Rust の部分的借用を通じて `&mut self.scope` と `&mut self.solver` を ExpressionInferrer に渡し、変数コピーを解消
- 後方互換性エイリアスを維持：`pub type BodyChecker = StatementChecker;`

### フェーズ 3：expressions.rs の作成 ✅

**目標**：ExprInferrer の式推論ロジックの統合

**成果物**：
- ✅ `inference/expressions.rs` - ExpressionInferrer（897行、共有 ScopeManager を使用）

**実装詳細**：
- ExpressionInferrer は `scope: &'a mut ScopeManager` と `solver: &'a mut TypeConstraintSolver` を借用
- コンストラクタ署名：`new(scope, solver, overload_candidates)` / `with_native_signatures(scope, solver, overloads, natives)`
- 後方互換性エイリアスを維持：`pub type ExprInferrer<'a> = ExpressionInferrer<'a>;`

### フェーズ 4：checking/ のファイルを inference/ に移動 ✅

**目標**：checking/ を inference/ に統合

**移動**：
- ✅ `checking/assignment.rs` → `inference/assignment.rs`
- ✅ `checking/subtyping.rs` → `inference/subtyping.rs`
- ✅ `checking/compatibility.rs` → `inference/compatibility.rs`
- ✅ `checking/bounds.rs` → `inference/bounds.rs`

### フェーズ 5：mod.rs エントリポイントの修正 ✅

**ファイル**：`src/frontend/typecheck/mod.rs`

**修正**：
- ✅ `pub mod checking;` を削除
- ✅ `pub use inference::*;` エクスポートを更新
- ✅ `infer_expression()` を更新して ScopeManager + ExpressionInferrer を使用
- ✅ `TypeChecker` 参照を `inference::StatementChecker` に更新

### フェーズ 6：旧コードとディレクトリの削除 ✅

**削除**：
- ✅ `checking/` ディレクトリを完全に削除
- ✅ 旧 BodyChecker コードを StatementChecker に置換
- ✅ ExprInferrer.scopes を共有 ScopeManager に置換

### フェーズ 7：回帰テスト ✅

```bash
cargo test
# test result: ok. 1434 passed; 0 failed; 4 ignored
# test result: ok. 30 passed; 0 failed
# test result: ok. 5 passed; 0 failed; 11 ignored
```

**テストファイル更新**：
- ✅ `tests/shadowing.rs` - BodyChecker インポートパスを更新、ExprInferrer に ScopeManager パラメータを追加
- ✅ `tests/scope.rs` - ExprInferrer に ScopeManager パラメータを追加
- ✅ `tests/infer.rs` - 39箇所の ExprInferrer 署名更新、StmtInferrer テストを StatementChecker に書き直し
- ✅ `tests/constraint.rs` - 6箇所の checking:: → inference:: インポートパス更新
- ✅ `tests/basic.rs` - 18箇所の ExprInferrer 署名更新

---

## クリーンアップが必要なコード

### 1. BodyChecker → statements.rs

| 元の位置 | 目標位置 |
|--------|---------|
| `checking/mod.rs` - `BodyChecker` | `inference/statements.rs` - `StatementChecker` |
| `check_stmt`, `check_var_stmt` など | `StatementChecker::check_*` |

### 2. ExprInferrer → expressions.rs

| 元の位置 | 目標位置 |
|--------|---------|
| `inference/expressions.rs` - `ExprInferrer` | `inference/expressions.rs` - `ExpressionInferrer` |
| `scopes` フィールド | `ScopeManager` を使用 |

### 3. checking/ → inference/

| 元の位置 | 目標位置 |
|--------|---------|
| `checking/assignment.rs` | `inference/assignment.rs` |
| `checking/subtyping.rs` | `inference/subtyping.rs` |
| `checking/compatibility.rs` | `inference/compatibility.rs` |
| `checking/bounds.rs` | `inference/bounds.rs` |

### 4. 削除

| 削除項目 | 説明 |
|--------|------|
| `checking/` ディレクトリ | 完全に削除 |
| `BodyChecker` 構造体 | StatementChecker に移行済み |
| `ExprInferrer.scopes` | ScopeManager に変更 |

---

## 拡張性設計

### 新しい文型の追加

```rust
// inference/statements.rs
impl StatementChecker {
    pub fn check(&mut self, stmt: &Stmt) -> Result<(), Box<Diagnostic>> {
        match &stmt.kind {
            // ... 既存の文
            StmtKind::Match { .. } => self.check_match(),  // 🆕
            StmtKind::While { .. } => self.check_while(),  // 🆕
        }
    }
}
```

### 新しい式型の追加

```rust
// inference/expressions.rs
impl ExpressionInferrer {
    pub fn infer(&mut self, expr: &Expr) -> Result<MonoType, Box<Diagnostic>> {
        match expr {
            // ... 既存の式
            Expr::Macro { .. } => self.infer_macro(),  // 🆕
            Expr::Await { .. } => self.infer_await(),  // 🆕
        }
    }
}
```

---

## 検収基準

### アーキテクチャ検収

- [x] `inference/scope.rs` がスコープ管理を独立して担当
- [x] `inference/statements.rs` が文の検査を独立して担当
- [x] `inference/expressions.rs` が式の推論を独立して担当
- [x] `inference/types.rs` が型システムツールを提供
- [x] `inference/assignment.rs`、`subtyping.rs`、`compatibility.rs`、`bounds.rs` が正常に動作
- [x] 特性モジュール（specialization/traits/gat/overload）が独立を維持
- [x] `checking/` ディレクトリを削除
- [x] 変数の手動同期ロジックがない（共有 ScopeManager の Rust 部分的借用パターンを使用）

### 機能検収

| テストケース | 予期結果 |
|---------|---------|
| `nums = [1,2,3]; for n in nums { print(n) }` | コンパイル成功 |
| `x = 10; for i in 1..3 { x = i }` | コンパイル成功 |
| `entry: FileEntry = item` | 型注釈が正常に動作 |

### 回帰テスト

```bash
cargo test
```

予期：全テストが通過

---

## テスト計画

### フェーズ 1：ユニットテスト

| テスト名 | モジュールド |
|---------|------|
| test_enter_scope | scope.rs |
| test_exit_scope | scope.rs |
| test_add_var | scope.rs |
| test_get_var_outer | scope.rs |
| test_unify_int | types.rs |
| test_infer_element_type | types.rs |

### フェーズ 2：統合テスト

| テスト名 | テストコード | 予期結果 |
|---------|---------|---------|
| test_for_list | `for n in [1,2,3] { print(n) }` | コンパイル成功 |
| test_var_scope | 変数スコープが正しい | 通過 |
| test_type_annotation | `x: Int = 1` | コンパイル成功 |
| test_generic_fn | 泛型関数 | 正常に動作 |
| test_trait_bound | 特質制約 | 正常に動作 |

### フェーズ 3：回帰テスト

```bash
cargo test
```

予期：全テストが通過