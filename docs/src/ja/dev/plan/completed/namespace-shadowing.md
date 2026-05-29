# 変数名前空間とシャドーイング機構コードレビュー報告

## レビュー日時
2026-02-18

## 修正日時
2026-02-18

## レビュー範囲
- 変数名前空間管理（スコープ）
- 変数シャドーイング検出
- For ループ変数のセマンティクス
- `mut` 宣言処理

---

## 要件振り返り

設計に基づく言語の変数シャドーイングルールは以下の通りです：

1. **シャドーイング禁止** - 外側のスコープに既に存在する変数名を、内側で再宣言しようとするとエラーになる
2. **let キーワードなし** - `mut` または暗黙的な宣言で変数を作成
3. **シャドーイングはエラー** - 通常宣言でも `mut` 宣言でも、シャドーイングはエラー
4. **ローカル変数はスコープ終了時に破棄** - ローカルスコープ结束时変数被销毁
5. **For ループのセマンティクス** - `for i in iter` の `i` は再バインディングであり、毎迭代新しいローカルバインディングを作成

---

## 評価：🟢 修正済み

## スコープ実装状況の概要

### 型推論段階（ExprInferrer）

| スコープタイプ | 実装状況 | コード位置 |
|-----------|---------|-----------|
| For ループ | ✅ 実装済み | expressions.rs（enter_scope/exit_scope + try_add_var） |
| 関数定義 | ✅ 実装済み | expressions.rs（enter_scope/exit_scope） |
| Lambda 式 | ✅ 実装済み | expressions.rs（enter_scope/exit_scope） |
| リスト内包表記 | ✅ 実装済み | expressions.rs（enter_scope/exit_scope） |
| If 文の分岐 | ✅ **修正済み** | expressions.rs（各分支 enter_scope/exit_scope） |
| While ループ本体 | ✅ **修正済み** | expressions.rs（enter_scope/exit_scope） |

### 型チェック段階（BodyChecker）

| スコープタイプ | 実装状況 | 説明 |
|-----------|---------|------|
| スコープスタック | ✅ **修正済み** | `scopes: Vec<HashMap<String, PolyType>>` |
| For ループスコープ | ✅ **修正済み** | `check_for_stmt` が enter/exit_scope を使用 |
| 関数パラメータースコープ | ✅ **修正済み** | `check_fn_def` が enter/exit_scope を使用 |
| If 文のスコープ | ✅ **修正済み** | `check_block` が自動的にスコープを作成 |
| 通常コードブロックスコープ | ✅ **修正済み** | `check_block` が自動的にスコープを作成 |
| mut シャドーイングチェック | ✅ **修正済み** | `check_var_stmt` が var_exists_in_any_scope をチェック |
| For ループシャドーイングチェック | ✅ 実装済み | `check_for_stmt` が var_exists_in_any_scope をチェック |
| 代入式のシャドーイングチェック | ✅ **修正済み** | `check_expr_stmt` が現在/外側スコープを区別 |

---

## 修正済み問題

### 修正 1：BodyChecker スコープスタック（原問題 2）

**修改ファイル**: `src/frontend/typecheck/checking/mod.rs`

**修改内容**：`vars: HashMap<String, PolyType>` を `scopes: Vec<HashMap<String, PolyType>>` に置換

新增方法：
- `enter_scope()` - 新規スコープに入る
- `exit_scope()` - 現在スコープを退出
- `var_exists_in_any_scope(name)` - 変数が任意のスコープに存在するかをチェック
- `var_exists_in_current_scope(name)` - 変数が現在スコープに存在するかをチェック
- `update_var(name, poly)` - 既存スコープで変数を更新

`add_var` は现在作用域（`scopes.last_mut()`）に追加され、`get_var` は内側から外側に向かって検索する。

---

### 修正 2：mut 宣言シャドーイングチェック（原問題 1）

**修改ファイル**: `src/frontend/typecheck/checking/mod.rs` - `check_var_stmt`

**修改前**：
```rust
// 既に変数が存在する場合、型を統合  ← 错误！シャドーイングを許可していた
if let Some(existing_poly) = self.vars.get(name) {
    let _ = self.solver.unify(&existing_poly.body, &ty);
}
self.vars.insert(name.to_string(), PolyType::mono(ty));
```

**修改後**：
```rust
// シャドーイングチェック：変数が任意のスコープに既に存在する場合、エラー
if self.var_exists_in_any_scope(name) {
    return Err(Box::new(
        ErrorCodeDefinition::variable_shadowing(name).build(),
    ));
}
self.add_var(name.to_string(), PolyType::mono(ty));
```

---

### 修正 3：For ループスコープ（原問題 3）

**修改ファイル**: `src/frontend/typecheck/checking/mod.rs` - `check_for_stmt`

**修改内容**：ループ変数チェックとループ本体チェックの前後に `enter_scope()` / `exit_scope()` を追加し、ループ変数がループ終了後に破棄されることを保証。

---

### 修正 4：代入式シャドーイングチェック（新規）

**修改ファイル**: `src/frontend/typecheck/checking/mod.rs` - `check_expr_stmt`

**修改内容**：`BinOp::Assign` の処理に3つのケースの区別を追加：
1. **现在スコープに存在** → 代入操作（型を統合）
2. **外側のスコープのみに存在** → シャドーイングエラー
3. **存在しない** → 新規変数を作成

---

### 修正 5：関数定義スコープ（新規）

**修改ファイル**: `src/frontend/typecheck/checking/mod.rs` - `check_fn_def`

**修改内容**：関数定義チェックは獨立した関数スコープを作成し、関数パラメータとローカル変数は関数終了後に破棄される。

同時に `check_fn_stmt` から重複したパラメータ追加ロジックを移除し、パラメータは `check_fn_def` 内で関数スコープ統一管理。

---

### 修正 6：If/コードブロックスコープ（新規）

**修改ファイル**: `src/frontend/typecheck/checking/mod.rs` - `check_block`

**修改内容**：`check_block` は If 文の then/elif/else 分岐のために、自動的にスコープを作成・退出。

---

### 修正 7：ExprInferrer If/While スコープ（新規）

**修改ファイル**: `src/frontend/typecheck/inference/expressions.rs`

**修改内容**：
- If 式：各分岐（then/elif/else）が独立的スコープを使用
- While 式：ループ本体が独立的スコープを使用
- For ループ：`try_add_var` 失敗時にスコープを退出しないバグを修正

---

## テストカバレッジ

新規テストファイル：`src/frontend/typecheck/tests/shadowing.rs`（14個のテスト）

| テスト名称 | テスト内容 |
|---------|---------|---------|
| `test_body_checker_scope_basic` | 基础スコープ进入/退出 |
| `test_body_checker_nested_scopes` | ネストされたスコープの変数可視性と破棄 |
| `test_body_checker_get_var_finds_innermost` | get_var が最内層変数を優先的に返す |
| `test_body_checker_vars_returns_all` | vars() が全スコープ変数を返す |
| `test_mut_shadowing_error` | mut 重複宣言でシャドーイングエラーを報告 |
| `test_for_loop_shadowing_error` | for ループで既存変数名を使用するとエラー |
| `test_for_loop_variable_scoped` | for ループ変数がループ終了後に破棄される |
| `test_for_loop_no_conflict_with_unique_var` | 競合なしの for ループが正常に動作 |
| `test_if_block_creates_scope` | if ブロック内変数が外側に漏れ出さない |
| `test_assignment_shadowing_in_block` | if ブロック内で外側変数に代入するとシャドーイングエラー |
| `test_assignment_in_same_scope_ok` | 同一スコープ内の重複代入が正常 |
| `test_inferrer_try_add_var_shadowing` | ExprInferrer のシャドーイング検出 |
| `test_inferrer_scope_destroyed_on_exit` | ExprInferrer がスコープ退出後に変数を破棄 |
| `test_fn_def_creates_scope` | 関数パラメータが関数終了後に破棄される |

---

## まとめ

### 推論段階（ExprInferrer）

| 機能 | 状態 |
|------|------|
| For ループスコープ | ✅ 実装済み |
| 関数定義スコープ | ✅ 実装済み |
| Lambda スコープ | ✅ 実装済み |
| リスト内包表記スコープ | ✅ 実装済み |
| If 文スコープ | ✅ **修正済み** |
| While ループ本体スコープ | ✅ **修正済み** |

### チェック段階（BodyChecker）

| 機能 | 状態 |
|------|------|
| スコープスタック | ✅ **修正済み** |
| For ループスコープ | ✅ **修正済み** |
| 関数パラメータースコープ | ✅ **修正済み** |
| If 文スコープ | ✅ **修正済み** |
| 通常コードブロックスコープ | ✅ **修正済み** |
| mut シャドーイングチェック | ✅ **修正済み** |
| For ループシャドーイングチェック | ✅ 実装済み |
| 代入式シャドーイングチェック | ✅ **修正済み** |

---

## 遗留事項

### For ループの「再バインディング」セマンティクス

For ループの `i` セマンティクス（毎迭代新しいバインディングを作成）は、IR 生成とインタープリター层面で處理し、型チェック段階では変数がスコープ内に存在することを確認すればよい。现在的実装で問題なし。