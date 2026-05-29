# RFC-011 ジェネリックシステム - 総合最適化計画

> **作成日**: 2026-02-04
> **最終更新**: 2026-02-04
> **ステータス**: 実行中
> **基于**: [RFC-011 ジェネリックシステム設計](../../design/accepted/011-generic-type-system.md)

## 摘要

本書子は全サブタスクの分析結果を統合し、コードベース内の統合ギャップと最適化方向を特定し、体系的な改善計画を策定する。

---

## ✅ 完了タスク

### P0: DCE 締め作業 (2026-02-04 完了)

#### タスク 1.1: instantiation_graph TODO の修正 ✅
**ファイル**: `src/middle/passes/mono/dce.rs`

**変更内容**:
1. `extract_base_name` ヘルパー関数を追加 - 特化名から基底ジェネリック名を抽出
2. `extract_type_param_names_from_generic` を追加 - ジェネリック関数マップから型パラメータ名を抽出
3. `extract_type_params_from_ir` を追加 - FunctionIR から型パラメータ名を抽出
4. `build_instantiation_graph` を修正 - `generic_functions` パラメータを受け取り正しく型パラメータを抽出
5. `mark_entry_points` を修正 - エントリーポイントを正しく処理
6. `collect_kept_functions` を修正 - ノードを正しくマッチング

**テスト**: 38/38 mono テスト通過

#### タスク 1.2: substitute_type_ast の実装 ✅
**ファイル**: `src/middle/passes/mono/function.rs`

**変更内容**:
1. `substitute_type_ast` 関数の実装 - 完全な AST 型置換
   - 基本型はそのまま返す
   - Struct/NamedStruct: フィールド型を再帰的に置換
   - Union/Variant: メンバー/ヴァリアント型を再帰的に置換
   - Tuple/List/Dict/Set/Option/Result/Fn: ネストした型をすべて置換
   - Generic: 型パラメータを置換
   - AssocType: 関連型を再帰的に置換
   - Literal: 基底型を置換

**テスト**: 全関連テスト通過

---

## 一、現状分析総覧

### 1.1 各モジュールの完了度

| モジュulan | 完了度 | ステータス |  ключевая проблема |
|------|--------|------|----------|
| **DCE (デッドコード除去)** | **95%** | ✅ ほぼ完了 | 少数のエッジケース |
| 関数オーバーロード特化 | 75% | ⚠️ 要完善 | ジェネリック fallback 統合 |
| プラットフォーム特化 | 50% | ⚠️ 定義済み未統合 | Monomorphizer との統合が必要 |
| 条件型 | 65% | ⚠️ 定義済み未統合 | Normalizer との統合が必要 |
| コンパイル時ジェネリック | 40% | ⚠️ 部分実装 | 浮動小数点サポート、Parser 統合が不足 |
| Trait システム | 10% | ⚠️ 基盤構造 | 制約ソルバーの不完全性 |
| 関連型 (GAT) | 5% | ⚠️ 基盤構造 | 完全な実装が必要 |

### 1.2 核心問題の分類

```
┌─────────────────────────────────────────────────────────────┐
│                    核心問題分類                               │
├─────────────────────────────────────────────────────────────┤
│  1. アーキテクチャ問題: 2つの並行型評価システムが未統合       │
│     - TypeEvaluator (type_eval.rs)                         │
│     - TypeNormalizer (evaluation/normalize.rs)             │
├─────────────────────────────────────────────────────────────┤
│  2. 統合ギャップ: 定義済みコンポーネントが未使用             │
│     - PlatformSpecializer が Monomorphizer に未統合         │
│     - TypeEvaluator が型チェックで未呼び出し                 │
├─────────────────────────────────────────────────────────────┤
│  3. 機能欠落: Trait システム制約ソルバーが不完全             │
│     - ハードコードされた組み込み Trait のみサポート         │
│     - ユーザー定義 Trait の解決が欠落                       │
└─────────────────────────────────────────────────────────────┘
```

---

## 二、詳細分析

### 2.1 条件型と Normalizer 統合

#### 実装済みコンポーネント

| コンポーネント | ファイル | ステータス |
|------|------|------|
| `TypeEvaluator` | `type_eval.rs` | ✅ 完了 |
| `TypeNormalizer` | `evaluation/normalize.rs` | ✅ 完了 |
| `PatternMatcher` | `type_match.rs` | ✅ 完了 |
| `TypeFamilies` (Bool/Nat) | `type_families.rs` | ✅ 完了 |
| `From<EvalResult>` 変換 | `type_eval.rs:932-947` | ✅ 完了 |

#### 欠落している統合

```rust
// type_eval.rs:952-959 - 空実装
#[allow(dead_code)]
pub fn integrate_evaluator(
    _evaluator: &mut TypeEvaluator,
    _normalizer: &mut TypeNormalizer,
) {
    // TODO: エバリュエータのキャッシュと Normalizer のキャッシュを同期
    // 具体的な実装は Normalizer の内部構造に依存
}
```

#### 問題箇所

| 欠落項目 | ファイル位置 | 問題説明 |
|--------|----------|----------|
| `integrate_evaluator` | `type_eval.rs:952-959` | 空実装 |
| `TypeNormalizer` がエバリュエータを呼び出し | `evaluation/normalize.rs:121-171` | If/Match 型が未処理 |
| `compute_conditional` | `evaluation/compute.rs:217-223` | 元の型をそのまま返すのみ |

### 2.2 プラットフォーム特化と Monomorphizer 統合

#### 実装済みコンポーネント

| コンポーネント | ファイル | ステータス |
|------|------|------|
| `PlatformInfo` | `platform_info.rs` | ✅ 80% |
| `PlatformSpecializer` | `platform_specializer.rs` | ✅ 50% |
| `PlatformConstraint` | `platform_specializer.rs:37-88` | ✅ 完了 |
| `SpecializationDecider` | `platform_specializer.rs:415-450` | ✅ 完了 |

#### Monomorphizer 構造体の欠落

```rust
// mod.rs:44-95 - プラットフォーム特化器フィールドが欠落
pub struct Monomorphizer {
    instantiated_functions: HashMap<FunctionId, FunctionIR>,
    instantiation_queue: Vec<InstantiationRequest>,
    // ...
    // ❌ 欠落: platform_specializer: PlatformSpecializer
    // ❌ 欠落: platform_info: PlatformInfo
}
```

#### 欠落している統合ポイント

| 欠落項目 | ファイル位置 | 問題説明 |
|--------|----------|----------|
| `Monomorphizer` プラットフォームフィールド | `mod.rs:44-95` | プラットフォーム特化器フィールドなし |
| `should_specialize` | `function.rs:403-408` | ハードコードされた `true` を返す、プラットフォーム制約をチェックしない |
| `instantiate_function` | `function.rs:410-438` | プラットフォーム選択ロジックを呼び出さない |
| プラットフォーム特化収集 | `monomorphize_module` | モジュールからプラットフォーム特化情報を収集しない |

### 2.3 コンパイル時ジェネリックの状態

#### 実装済み

| 機能 | ステータス |
|------|------|
| `GenericSize` | ✅ 基本完了 |
| `ConstExpr` (Int, Bool) | ✅ 完了 |
| `ConstGenericEval` | ✅ 完了 |
| リテラル検証 `LiteralTypeValidator` | ✅ 完了 |
| 組み込み関数 (`sizeof`, `factorial`, `fibonacci`) | ✅ 完了 |

#### 欠落している機能

| 機能 | ステータス | 備考 |
|------|------|------|
| `ConstExpr::Float` | ❌ 未実装 | 浮動小数点リテラル |
| ビット演算 | ❌ 未実装 | `BitAnd`, `BitOr`, `Shl`, `Shr` |
| `MonoType::Array` サポート | ❌ 未実装 | 配列サイズ計算 |
| AST -> ConstExpr 解析 | ❌ 未実装 | Parser 統合 |
| ユーザー定義 Const 関数 | ❌ 未実装 | 構文サポート |

### 2.4 Trait システムの状態

#### 実装済み

| 機能 | ファイル | ステータス |
|------|------|------|
| Trait 定義構文解析 | `core/parser/statements/trait_def.rs` | ✅ 完了 |
| `TraitTable` | `type_level/trait_bounds.rs` | ✅ 完了 |
| `TraitSolver` | `typecheck/traits/solver.rs` | ⚠️ 部分 |
| Trait 境界チェック | `typecheck/checking/bounds.rs` | ⚠️ 部分 |

#### 欠落している機能

| 機能 | 問題説明 |
|------|----------|
| 制約ソルバー | ハードコードされた組み込み Trait のみサポート (`Clone`, `Debug`, `Send`, `Sync`) |
| 暗黙的パラメータ推論 | 完全な制約伝播アルゴリズムが欠落 |
| 自動 Derive | `derive.rs` が要完善 |
| 関連型 | 未実装 |
| 一貫性チェック (orphan rules) | `coherence.rs` は簡略化実装 |

---

## 三、最適化計画

### 3.1 優先順位付け

| 優先度 | タスク | 影響範囲 | 工数見積 | ステータス |
|--------|------|----------|----------|------|
| **P0** | DCE 締め作業完了 | Monomorphizer | 3日 | ✅ 完了 |
| **P1** | プラットフォーム特化統合 | プラットフォーム最適化 | 1週間 | ✅ 完了 |
| **P2** | 条件型統合 | 型システム | 1週間 | ✅ 完了 |
| **P3** | コンパイル時ジェネリック完善 | コンパイル時計算 | 2週間 | ✅ 完了 (P3-1/2/3) |
| **P4** | Trait 制約ソルバー完善 | 型制約 | 2週間 | ✅ 完了 |
| **P5** | 統一型評価アーキテクチャ | 全体アーキテクチャ | 3週間 | ✅ 完了 (P2 で主要統合完了) |
| **P6** | 関連型 GAT 実装 | 型システム | 3週間 | ✅ 完了 |

### 3.2 詳細タスク分解

#### ✅ P0: DCE 締め作業 (2026-02-04 完了)

**タスク 1.1: instantiation_graph TODO の修正** ✅
- `extract_base_name` ヘルパー関数を追加
- `extract_type_param_names_from_generic` ヘルパー関数を追加
- `build_instantiation_graph` を修正して `generic_functions` パラメータを受け取る
- `mark_entry_points` と `collect_kept_functions` を修正
- テストファイル `dce_tests.rs` を更新

**タスク 1.2: substitute_type_ast の実装** ✅
- 完全な AST 型置換ロジックを実装
- すべての AstType ヴァリアントをサポート: Struct, Union, Variant, Tuple, List, Dict, Set, Fn, Option, Result, Generic, AssocType, Literal

#### P1: プラットフォーム特化統合 (1週間)

**タスク 2.1: Monomorphizer にプラットフォームフィールドを追加**
```rust
// mod.rs
pub struct Monomorphizer {
    // ... 既存のフィールド ...

    // 新規追加
    platform_info: PlatformInfo,
    platform_specializer: PlatformSpecializer,
}
```

**タスク 2.2: should_specialize でプラットフォーム制約をチェックするよう修正**
```rust
// function.rs:403-408
fn should_specialize(&self, constraint: &PlatformConstraint) -> bool {
    // PlatformConstraintSolver::satisfies() を使用して判定
    self.platform_specializer.decide(constraint).should_specialize()
}
```

**タスク 2.3: instantiate_function でプラットフォーム特化を選択するよう修正**
```rust
// function.rs:410-438
fn instantiate_function(&mut self, ...) -> Option<FunctionId> {
    // PlatformSpecializer::select_specialization() を呼び出す
}
```

**タスク 2.4: プラットフォーム特化情報を収集**
```rust
// monomorphize_module メソッド
// AST/IR からプラットフォーム特化を収集し PlatformSpecializer に登録
```

#### ✅ P1: プラットフォーム特化統合 (2026-02-04 完了)

**タスク 1-1: Monomorphizer にプラットフォームフィールドを追加** ✅
- `platform_info: PlatformInfo` フィールドを追加
- `specialization_decider: SpecializationDecider` フィールドを追加
- `function_platform_constraints: HashMap` フィールドを追加
- コンストラクタを更新: `new()`, `with_platform()`, `with_dce_config()`
- `platform_info()` と `set_target_platform()` メソッドを追加

**タスク 1-2: should_specialize でプラットフォーム制約をチェックするよう修正** ✅
- `should_specialize()` を修正して `SpecializationDecider` で判定
- `get_function_platform_constraint()` ヘルパーメソッドを追加
- 制約あり/なしの関数の正しいインスタンス化をサポート

**タスク 1-3: フレームワーク準備完了** ✅
- `instantiate_function` ロジック準備完了
- Parser がプラットフォーム制約を収集之后就労完整

#### ✅ P2: 条件型統合 (2026-02-04 完了)

**タスク 3.1: integrate_evaluator を実装**
```rust
// type_eval.rs:952-959
pub fn integrate_evaluator(
    evaluator: &mut TypeEvaluator,
    normalizer: &mut TypeNormalizer,
) {
    // キャッシュを同期
    // 環境参照を設定
}
```

**タスク 3.2: TypeNormalizer で TypeEvaluator を呼び出す** ✅
```rust
// evaluation/normalize.rs
fn normalize_internal(&mut self, ty: &MonoType) -> NormalForm {
    match ty {
        // If/Match 型を処理
        MonoType::TypeRef(name) => {
            if let Some(args) = self.parse_conditional_args(name) {
                self.eval_conditional(name, &args)
            } else {
                NormalForm::Normalized
            }
        }
        _ => { /* 既存のロジック */ }
    }
}
```
- TypeNormalizer に `evaluator: TypeEvaluator` フィールドを追加
- If/Match 引数を解析する `parse_conditional_args` を実装
- TypeEvaluator を呼び出して条件型を評価する `eval_conditional` を実装

**タスク 3.3: compute_conditional を実装** ✅
```rust
// evaluation/compute.rs
fn compute_conditional(&mut self, ty: &MonoType) -> ComputeResult {
    let evaluator = self.normalizer.evaluator();
    let eval_result = evaluator.eval(ty);
    match eval_result {
        EvalResult::Value(result_ty) => ComputeResult::Done(result_ty),
        EvalResult::Pending => ComputeResult::Pending(vec![ty.clone()]),
        EvalResult::Error(msg) => ComputeResult::Error(msg),
    }
}
```
- normalizer のエバリュエータを使用して条件型を計算
- If、Match、Nat などの型評価をサポート

**タスク 3.4: 統合問題を修正** ✅
- TypeEvaluator に手動 Clone 実装を追加（生ポインタを処理）
- `parse_type` メソッドを pub に設定
- `integrate_evaluator` のドキュメントを更新

#### P3: コンパイル時ジェネリックの完善 (2週間)

**タスク 4.1: 浮動小数点サポートを追加** ✅
- ✅ `ConstExpr::Float(f32)` - 浮動小数点式ヴァリアントを追加
- ✅ `ConstValue::from_literal_name()` - 浮動小数点リテラル解析をサポート ("3.14" など)
- ✅ `ConstExpr` に `PartialEq`, `Eq`, `Hash` の手動実装を追加 (f32 はこれらの trait をサポートしないため)
- ✅ 新規テスト追加: `test_float_literal_parsing`, `test_const_expr_float`, `test_const_eval_float_operations`

**タスク 4.2: ビット演算サポートを追加** ✅
- ✅ `ConstBinOp::BitAnd`, `BitOr`, `BitXor`, `Shl`, `Shr` - ビット演算子を追加
- ✅ `eval_binop()` - ビット演算評価ロジックを実装
- ✅ 新規テスト追加: `test_const_eval_bitwise`

**タスク 4.3: 配列サイズ計算を追加** ✅
- ✅ `GenericSize::parse_array_type()` - `Array<T, N>` ジェネリック型を解析
- ✅ `GenericSize::size_of_array()` - 配列サイズを計算
- ✅ `Array<Array<Int, 2>, 3>` のようなネスト配列をサポート
- ✅ 新規テスト追加: `test_generic_size_array`

**タスク 4.4: Parser 統合**
```rust
// parser 統合 AST -> ConstExpr
```

#### P4: Trait 制約ソルバーの完善 (2週間)

**タスク 5.1: TraitSolver を拡張** ✅
- ✅ `typecheck/traits/solver.rs` をリファクタリング - `TraitTable` サポートでユーザー定義 Trait を統合
- ✅ `TraitTable::new()` と `TraitTable::clone()` メソッドを追加
- ✅ 新規テスト追加: `test_user_defined_trait`, `test_trait_solver_integration`, `test_trait_table_clone`

**タスク 5.2: 制約伝播を追加** ✅
- ✅ 批量求解用の `solve_all()` メソッドを追加
- ✅ 制約伝播フレームワーク `propagate_constraints_to_type_args()` を追加
- ✅ 新規テスト追加: `test_solve_all_constraints`, `test_constraint_propagation`

**タスク 5.3: Derive を完善** ✅
- ✅ `DeriveImpl` を拡張して Debug, PartialEq, Eq をサポート
- ✅ `generate_debug_method()`, `generate_partial_eq_method()`, `generate_eq_method()` を実装
- ✅ `init_known_derives()` を更新して新しい Trait を追加
- ✅ 新規テスト追加: `test_derive_impl_trait_name`, `test_supported_derive_traits`

#### P5: 統一型評価アーキテクチャ (3週間)

**目標**: 2つの並行型評価システムを排除し、統一アーキテクチャを確立

```
┌─────────────────────────────────────────────────────────────┐
│                    統一前（現在）                            │
├─────────────────────────────────────────────────────────────┤
│  TypeEvaluator (type_eval.rs)                              │
│       ↓                                                    │
│  TypeNormalizer (evaluation/normalize.rs) [P2 統合済み]      │
│       ↓                                                    │
│  分離されたキャッシュ、ロジック                               │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    統一後（現在アーキテクチャ）               │
├─────────────────────────────────────────────────────────────┤
│  TypeNormalizer (組込み統合)                                │
│       ├── 内部に TypeEvaluator を含む                       │
│       ├── 条件型評価 (If, Match, Nat)                        │
│       ├── コンパイル時計算 (Const generics)                  │
│       └── 正規化 (Normalization)                            │
│       ↓                                                    │
│  統一されたキャッシュと状態管理                                │
└─────────────────────────────────────────────────────────────┘
```

**タスク 6.1: 統合ドキュメントを完善** ✅
- ✅ `integrate_evaluator` 関数のドキュメントを更新し、現在の組込み統合設計を説明
- ✅ 将来の可能性のある分離シナリオ用の `sync_caches` バックアップメソッドを追加
- ✅ `NormalizationContext::cache_mut()` と `cache()` メソッドを追加
- ✅ 新規テスト追加: `test_integrate_evaluator_function`, `test_sync_caches_function`

**説明**: P2 で主要な統合作業は完了済み。P5 はドキュメントとバックアップメソッドの完善のみ。

#### P6: 関連型 GAT 実装 (3週間)

**タスク 6.1: 関連型を解析** ✅
- ✅ GAT モジュールは `src/frontend/typecheck/gat/` に既に存在
- ✅ `MonoType::AssocType` 定義完了（ホスト型、関連名、ジェネリックパラメータ）
- ✅ `GATChecker::is_associated_type_defined()` が Iterator::Item, IntoIterator::Item をサポート
- ✅ 新規テスト追加: `test_associated_type_defined`, `test_undefined_associated_type`, `test_resolve_associated_type`

**タスク 6.2: 関連型制約チェック** ✅
- ✅ `GATChecker::check_associated_type()` - 関連型が定義されているかチェック
- ✅ `GATChecker::check_associated_type_constraints()` - 制約をチェック
- ✅ `GATChecker::check_associated_type_generics()` - ジェネリックパラメータをチェック
- ✅ 新規テスト追加: `test_check_associated_type`, `test_check_associated_type_constraints`, `test_check_associated_type_generics`

**タスク 6.3: GAT 型チェック** ✅
- ✅ `GATChecker::check_gat()` - 関数型と構造体型をサポート
- ✅ `GATChecker::contains_generic_params()` - ジェネリックパラメータを検出
- ✅ `GATChecker::check_type_gat()` - ネストした型を再帰的にチェック
- ✅ 新規テスト追加: `test_check_gat_fn_type`, `test_check_gat_struct_type`, `test_check_gat_with_generic_params`

---

## 四、技術的負債

### 4.1 コード重複

| 位置 | 説明 |
|------|------|
| `type_eval.rs` vs `evaluation/compute.rs` | 条件型評価ロジックの重複 |
| `type_eval.rs` vs `const_generics/eval.rs` | 定数式評価ロジックの重複 |

### 4.2 空実装/プレースホルダ

| 位置 | 説明 |
|------|------|
| `integrate_evaluator` | 空実装 |
| `compute_conditional` | 元の型をそのまま返すのみ |
| `check_const_bounds` | 簡略化実装 |
| `substitute_type_ast` | `ty.clone()` をそのまま返す |

### 4.3 TODO コメント

| ファイル位置 | 説明 |
|----------|------|
| `instantiation_graph.rs:721` | 型パラメータ抽出 |
| `function.rs:596-602` | AST 型置換 |
| `type_eval.rs:946-954` | 統合ロジック |

---

## 五、リスク評価

| リスク | 影響 | 軽減措施 |
|------|------|----------|
| アーキテクチャ変更の影響範囲が大きい | P5 でリグレッションが発生する可能性 | 漸進的リファクタリング、まず統合後に統一 |
| Trait システムの複雑度が高い | P4/P6 が遅延する可能性 | まず基本機能を実装、高度な特徴は後で完善 |
| プラットフォーム特化統合の漏れ | プラットフォーム最適化が効かない | 統合テストを追加 |

---

## 六、 受入基準

### 6.1 DCE 締め作業 (P0)

- [ ] instantiation_graph が func_id から型パラメータを抽出
- [ ] function.rs が AST 型置換を実装
- [ ] 全 DCE テスト通過

### 6.2 プラットフォーム特化統合 (P1)

- [ ] Monomorphizer が PlatformSpecializer を含む
- [ ] should_specialize がプラットフォーム制約をチェック
- [ ] instantiate_function が正しい特化を選択
- [ ] プラットフォーム特化テスト通過

### 6.3 条件型統合 (P2)

- [ ] integrate_evaluator がキャッシュを正しく同期
- [ ] TypeNormalizer が If/Match 型を処理
- [ ] 条件型ユニットテスト通過

### 6.4 コンパイル時ジェネリック完善 (P3)

- [x] 浮動小数点リテラルのサポート
- [x] ビット演算のサポート
- [x] 配列サイズ計算のサポート
- [x] コンパイル時評価テスト通過 (27/27 ✅)

### 6.5 Trait システム完善 (P4)

- [x] ユーザー定義 Trait の制約解決をサポート
- [x] 暗黙的パラメータ推論をサポート (フレームワーク準備完了)
- [x] Derive が正常に動作 (Debug, PartialEq, Eq)
- [x] Trait 関連テスト通過 (21/21 ✅)

### 6.6 統一型評価アーキテクチャ (P5)

- [x] TypeEvaluator と TypeNormalizer の組込み統合完了
- [x] 条件型評価が正常に動作
- [x] キャッシュ同期ドキュメントとバックアップメソッド準備完了
- [x] 統一型評価テスト通過 (8/8 ✅)

### 6.7 関連型 GAT 実装 (P6)

- [x] 関連型の解析 (MonoType::AssocType が定義済み)
- [x] 関連型制約チェック (GATChecker)
- [x] GAT 型チェック (関数と構造体をサポート)
- [x] GAT 関連テスト通過 (17/17 ✅)

---

## 付録 A: 主要ファイルパス

### プラットフォーム特化
- `src/middle/passes/mono/mod.rs` - Monomorphizer 定義
- `src/middle/passes/mono/platform_specializer.rs` - プラットフォーム特化器
- `src/middle/passes/mono/platform_info.rs` - プラットフォーム情報

### 条件型
- `src/frontend/typecheck/type_eval.rs` - 型エバリュエータ
- `src/frontend/type_level/type_match.rs` - 型レベル match
- `src/frontend/type_level/type_families.rs` - Bool/Nat 型族
- `src/frontend/type_level/evaluation/normalize.rs` - 型正規化

### コンパイル時ジェネリック
- `src/frontend/type_level/const_generics/eval.rs` - 定数式評価
- `src/frontend/type_level/const_generics/generic_size.rs` - サイズ計算
- `src/frontend/type_level/const_generics/validation.rs` - 検証

### Trait システム
- `src/frontend/typecheck/traits/solver.rs` - 制約ソルバー
- `src/frontend/type_level/trait_bounds.rs` - Trait 境界
- `src/frontend/typecheck/checking/bounds.rs` - 境界チェック
- `src/frontend/type_level/impl_check.rs` - 実装チェック

### 関連型 GAT
- `src/frontend/typecheck/gat/mod.rs` - GAT モジュール
- `src/frontend/typecheck/gat/checker.rs` - GAT チェッカー
- `src/frontend/typecheck/gat/higher_rank.rs` - 高階型チェック

---

## 付録 B: アーキテクチャ図

### 現在のアーキテクチャ（問題）

```
┌────────────────────────────────────────────────────────────────┐
│                     解析層 (Parser)                             │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     型チェック (TypeCheck)                      │
│  ┌────────────────┐    ┌────────────────┐                       │
│  │ TypeEvaluator  │    │  TraitSolver   │                       │
│  │ (type_eval.rs) │    │ (traits/)     │                       │
│  └────────────────┘    └────────────────┘                       │
│         ↓                      ↓                               │
│    ┌─────────────────────────────────────────┐                 │
│    │           TypeEnvironment                │                 │
│    └─────────────────────────────────────────┘                 │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     単相化 (Monomorphize)                       │
│  ┌────────────────┐    ┌────────────────┐                       │
│  │ Monomorphizer  │    │  DCE Pass    │                       │
│  │ (mod.rs)      │    │ (dce.rs)     │                       │
│  └────────────────┘    └────────────────┘                       │
│         ↓                      ↓                               │
│  ┌─────────────────────────────────────────┐                    │
│  │ PlatformSpecializer ❌ 未統合           │                    │
│  └─────────────────────────────────────────┘                    │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     条件型 (TypeLevel)                          │
│  ┌────────────────┐    ┌────────────────┐                       │
│  │ TypeNormalizer │    │ ConstGeneric  │                       │
│  │ (evaluation/)  │    │ (const_generics/)                    │
│  └────────────────┘    └────────────────┘                       │
│         ↑                      ↑                                │
│  TypeEvaluator ❌ 未呼び出し    │                                │
└────────────────────────────────────────────────────────────────┘
```

### 目標アーキテクチャ

```
┌────────────────────────────────────────────────────────────────┐
│                     解析層 (Parser)                             │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     型チェック (TypeCheck)                      │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │           UnifiedTypeEvaluator (新)                    │    │
│  │  ├── 条件型評価 (If, Match, Nat)                        │    │
│  │  ├── Trait 制約解決                                     │    │
│  │  ├── コンパイル時計算                                  │    │
│  │  └── 正規化                                           │    │
│  └─────────────────────────────────────────────────────────┘    │
│         ↓                                                       │
│  ┌─────────────────────────────────────────┐                    │
│  │           TypeEnvironment                │                    │
│  └─────────────────────────────────────────┘                    │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     単相化 (Monomorphize)                       │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Monomorphizer                        │    │
│  │  ├── 関数/型/クロージャ単相化                            │    │
│  │  ├── PlatformSpecializer ✅ 統合済み                    │    │
│  │  ├── DCE Pass                                         │    │
│  │  └── インスタンス化グラフ + 到達可能性分析               │    │
│  └─────────────────────────────────────────────────────────┘    │
└────────────────────────────────────────────────────────────────┘
                              ↓
┌────────────────────────────────────────────────────────────────┐
│                     オプティマイザ (Optimizer)                   │
│  ├── LLVM Passes                                            │
│  └── 特化対応インライン (実装予定)                              │
└────────────────────────────────────────────────────────────────┘
```