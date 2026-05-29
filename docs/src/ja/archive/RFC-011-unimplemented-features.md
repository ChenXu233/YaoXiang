# RFC-011 ジェネリクスシステム - 未実装機能一覧

> **作成日**: 2026-02-03
> **最終更新**: 2026-02-04
> **ステータス**: 进行中
> **RFC 基于**: [RFC-011 ジェネリクスシステム設計](../accepted/011-generic-type-system.md)

## 摘要

本文档记录 RFC-011 ジェネリクスシステム設計中已完成和未实现的功能模块。基于对编译器实现的分析，识别出当前系统的能力边界和待完善部分。

---

## 实现状态总览

| Phase | 功能模块 | 状态 | 完成度 | 关键文件 |
|-------|---------|------|--------|----------|
| Phase 1 | 基础ジェネリクス | ✅ 部分实现 | 70% | `src/middle/passes/mono/mod.rs` |
| Phase 2 | 型制約システム | ⚠️ 基础结构 | 30% | `src/frontend/type_level/` |
| Phase 3 | 関連型 | ⚠️ 基础结构 | 5% | `src/frontend/typecheck/gat/` |
| Phase 4 | コンパイル時ジェネリクス | ⚠️ 基础结构 | 40% | `src/frontend/type_level/const_generics/` |
| Phase 5 | 条件型 | ✅ 基础实现 | 65% | `src/frontend/type_level/type_match.rs` |
| - | 関数オーバーロード特殊化 | ✅ 已实现 | 75% | `src/frontend/typecheck/overload.rs` |
| - | プラットフォーム固有最適化 | ⚠️ 基础实现 | 50% | `src/middle/passes/mono/platform_specializer.rs` |
| - | 完全DCE | ✅ 部分实现 | 90% | `src/middle/passes/mono/` |

---

## 未实现功能详述

### 1. 関数オーバーロード特殊化メカニズム

#### 1.1 功能描述

RFC-011 設計使用**関数オーバーロード**實現特殊化：
```yaoxiang
# 具象型特殊化
sum: (arr: Array[Int]) -> Int = (arr) => {
    native_sum_int(arr.data, arr.length)
}

sum: (arr: Array[Float]) -> Float = (arr) => {
    simd_sum_float(arr.data, arr.length)
}

# ジェネリクス実装（自動選択）
sum: [T](arr: Array[T]) -> T = (arr) => { ... }
```

#### 1.2 当前状态

- ✅ データ構造はオーバーロードをサポート (`instance.rs`)
- ✅ オーバーロード解決モジュールが存在 (`overload.rs`)
- ✅ 型環境はオーバーロード候補の保存をサポート
- ✅ 関数呼び出しオーバーロード解決を統合 (`expressions.rs`)
- ⚠️ ジェネリクスfallback統合（改善待ち）

#### 1.3 需要的实现

```
src/frontend/typecheck/overload.rs              # ✅ オーバーロード解決（完了）
src/frontend/typecheck/mod.rs                  # ✅ 型環境拡張（完了）
src/middle/passes/mono/instance.rs             # ✅ インスタンス化ID拡張（完了）
src/frontend/typecheck/inference/expressions.rs  # ✅ オーバーロード解決統合（完了）
src/frontend/typecheck/checking/mod.rs          # ✅ BodyChecker拡張（完了）
```

#### 1.4 验收标准

- [x] 同名関数の異なる型シグネチャを解析可能（データ構造サポート済み）
- [x] 呼び出し時に実引数の型に基づいて最適なマッチを自動選択（統合済み）
- [x] コンパイルエラー：曖昧な呼び出しまたはマッチする定義がない（実装済み）
- [x] ジェネリクスシステムとの統合：ジェネリクスをfallbackとして使用（完了）

---

### 2. プラットフォーム固有最適化

#### 2.1 功能描述

RFC-011 設計支持通过預定義ジェネリクス参数 `P` 實現平台特殊化（不使用 `#[cfg]`）：
```yaoxiang
# 汎用実装（すべてのプラットフォームで使用可能）
sum: [T: Add](arr: Array[T]) -> T = { ... }

# プラットフォーム特殊化：Pは事前定義されたジェネリクスパラメータで、現在プラットフォームを表す
sum: [P: X86_64](arr: Array[Float]) -> Float = {
    return avx2_sum(arr.data, arr.length)
}

sum: [P: AArch64](arr: Array[Float]) -> Float = {
    return neon_sum(arr.data, arr.length)
}
```

#### 2.2 当前状态

- ✅ `platform_info.rs` 已实现（80%）
  - TargetPlatform: X86_64, AArch64, RiscV64, Arm, X86, Wasm32
  - PlatformDetector: ターゲットトリプル/環境変数から検出
  - 事前定義ジェネリクスパラメータ `P` サポート

- ✅ `platform_specializer.rs` 已实现（50%）
  - PlatformConstraint: `[P: X86_64]` 制約
  - PlatformSpecializer: プラットフォーム特殊化選択
  - 複数プラットフォーム特殊化バージョンの登録と選択をサポート

- ❌ `#[cfg]` 属性解析なし（RFC設計ではこの方式不使用）
- ❌ プラットフォーム特殊化とモノーファイヤの統合（実装待ち）
- ❌ プラットフォーム対応コード生成（実装待ち）

#### 2.3 需要的实现

```
src/frontend/core/parser/attr.rs                    # 任意：属性解析（RFC設計では #[cfg] 不使用）
src/middle/passes/mono/platform_info.rs             # ✅ 実装済み
src/middle/passes/mono/platform_specializer.rs      # ✅ 実装済み
src/middle/passes/mono/mod.rs                       # 変更：プラットフォーム特殊化を統合
```

#### 2.4 验收标准

- [x] ターゲットプラットフォームを検出可能（X86_64, AArch64, など）
- [x] 事前定義ジェネリクスパラメータ `P` を識別
- [ ] プラットフォーム特殊化とモノーファイヤが正しく統合
- [ ] 現在のプラットフォームに一致する特殊化コードのみを生成
- [ ] コンパイル時にターゲットプラットフォームに基づいて特殊化バージョンを自動選択

---

### 3. 死コード消除(DCE)完全实现

#### 3.1 功能描述

RFC-011 設計了多层次的 DCE：

1. **インスタンス化グラフ分析**：ジェネリクスインスタンス化依存グラフを構築し、エントリーポイントから到達可能性分析を実行
2. **使用点分析**：実際に呼び出されたジェネリスのみをインスタンス化
3. **クロスモジュールDCE**：モジュール間依存関係を分析し、未使用のエクスポートを削除
4. **LLVMレベルDCE**：LLVMの最適化パスを利用

```rust
// コンパイラ内部：ジェネリクスインスタンス化依存グラフを構築
struct InstantiationGraph {
    nodes: HashMap<InstanceKey, InstanceNode>,
    edges: HashMap<InstanceKey, Vec<InstanceKey>>,
}
```

#### 3.2 当前状态

- ✅ 基本的モノーファイヤが存在 (`mono/mod.rs`)
- ✅ 需求的特殊化基本実装
- ✅ インスタンス化グラフ構築完了 (`instantiation_graph.rs`)
- ✅ 完全到達可能性分析完了 (`reachability.rs`)
- ⚠️ クロスモジュールDCE基本実装（本番環境検証が必要）
- ✅ コード膨張制御完了 (`dce.rs`)

#### 3.3 需要的实现

```rust
// 新規モジュール
src/middle/passes/mono/instantiation_graph.rs      # インスタンス化グラフ構築
src/middle/passes/mono/reachability.rs              # 到達可能性分析
src/middle/passes/mono/cross_module_dce.rs          # クロスモジュールDCE
src/middle/passes/mono/code_bloat_control.rs        # コード膨張制御
```

#### 3.4 验收标准

- [x] 完全なインスタンス化依存グラフを構築
- [x] mainエントリーポイントから到達可能性分析を実行
- [x] 未使用のジェネリクスインスタンスを削除
- [x] クロスモジュール依存関係分析（本番環境検証）
- [x] コード膨張しきい値制御
- [x] 統計情報出力（詳細版+JSON形式）

---

### 4. 完全 Trait システム

#### 4.1 功能描述

RFC-011 設計了型制約システム（Rust Trait类似）：

```yaoxiang
# Trait 定義
type Clone = { clone: (Self) -> Self }
type Add = { add: (Self, Self) -> Self }

# 制約を使用
clone: [T: Clone](value: T) -> T = value.clone()
combine: [T: Clone + Add](a: T, b: T) -> T = a.clone() + b
```

#### 4.2 当前状态

- ⚠️ 型レベル計算モジュールが存在 (`type_level/`)
- ⚠️ 基本的 `Some`/`None` ラッパー実装
- ❌ Trait 定義構文解析なし
- ❌ Trait 実装検証なし
- ❌ Trait 継承/派生なし
- ❌ 制約ソルバー不完全

#### 4.3 需要的实现

```
src/frontend/core/parser/trait_def.rs               # 新規：Trait 定義解析
src/frontend/typecheck/trait_resolution.rs          # 新規：Trait 制約解決
src/frontend/typecheck/trait_impl.rs                # 新規：Trait 実装チェック
src/frontend/type_level/trait_bounds.rs             # 新規：Trait境界表現
```

#### 4.4 验收标准

- [ ] `type TraitName = { ... }` 構文を解析可能
- [ ] `[T: Trait]` 制約構文を解析可能
- [ ] 型が Trait 制約を満たすことを検証
- [ ] 複数制約 `[T: A + B]` をサポート
- [ ] エラーメッセージで不足している Trait 実装を指摘

---

### 5. 関連型 (GAT)

#### 5.1 功能描述

```yaoxiang
# 関連型定義
type Iterator[T] = {
    Item: T,                           # 関連型
    next: (Self) -> Option[T],
    has_next: (Self) -> Bool,
}

# 関連型を使用
collect: [T, I: Iterator[T]](iter: I) -> List[T] = { ... }
```

#### 5.2 当前状态

- ❌ 関連型解析なし
- ❌ 関連型制約チェックなし
- ❌ GAT サポートなし

#### 5.3 需要的实现

```
src/frontend/type_level/associated_types.rs         # 新規：関連型表現
src/frontend/typecheck/gat_check.rs                 # 新規：GAT 型チェック
```

#### 5.4 验收标准

- [ ] 関連型を含む Trait 定義を解析可能
- [ ] 制約として関連型を使用可能
- [ ] 型チェックで関連型を正しく解析
- [ ] ジェネリクス関連型をサポート

---

### 6. コンパイル時ジェネリクス完全实现

#### 6.1 功能描述

```yaoxiang
# コンパイル時定数パラメータ
type Array[T, N: Int] = { data: T[N] }

# コンパイル時関数：リテラル型制約を使用
factorial: [n: Int](n: n) -> Int = {
    match n {
        0 => 1,
        _ => n * factorial(n - 1)
    }
}

# コンパイル時計算（コンパイラがコンパイル時に計算）
SIZE: Int = factorial(5)  # 120
```

#### 6.2 当前状态

- ⚠️ `const_generics/` モジュールが存在
- ⚠️ 基本的 `GenericSize` 表現
- ⚠️ 基本的定数式評価
- ❌ リテラル型パラメータ解析なし `[n: Int](n: n)`
- ❌ コンパイル時関数インスタンス化なし
- ❌ コンパイル時次元検証なし
- ⚠️ `static_assert` は条件型標準ライブラリで実装（7. 条件型参照）

#### 6.3 需要的实现

```
src/frontend/core/parser/literal_param.rs           # 新規：リテラル型パラメータ解析
src/frontend/typecheck/const_eval.rs                 # 新規：コンパイル時式評価
src/middle/passes/mono/compile_time_monomorphization.rs  # 新規：コンパイル時ジェネリクス特殊化
```

#### 6.4 验收标准

- [ ] `[n: Int](n: n)` リテラル型パラメータ構文を解析可能
- [ ] `[N: Int]` コンパイル時ジェネリクスパラメータを解析可能
- [ ] リテラル型パラメータの関数呼び出しをコンパイル時に評価
- [ ] コンパイル時ジェネリクスインスタンス化をサポート
- [ ] 注：`Assert` は条件型を使用して標準ライブラリで実装（7.4 验收标准参照）

---

### 7. 条件型完全实现

#### 7.1 功能描述

```yaoxiang
# 型レベルIf
type If[C: Bool, T, E] = match C {
    True => T,
    False => E,
}

# 型族
type Add[A: Nat, B: Nat] = match (A, B) {
    (Zero, B) => B,
    (Succ(A'), B) => Succ(Add(A', B)),
}
```

#### 7.2 当前状态

- ✅ `type_families.rs` 已实现（60%）
  - Bool 型族: `True`, `False`
  - Nat 型族: `Zero`, `Succ[N]`
  - 条件型: `IsTrue`, `IsFalse`, `IsZero`, `IsSucc`
  - TypeFamily trait が統一処理を実装

- ✅ `type_match.rs` 已实现（70%）
  - MatchPattern: リテラル/コンストラクタ/タプル/ワイルドカードパターン
  - PatternMatcher: パターンマッチングエンジン
  - MatchType: 完全型マッチ
  - PatternBuilder: 流れるようなAPIでパターンを構築

- ✅ `type_eval.rs` 已实现（65%）
  - If 条件評価: `If<True, Int, String> => Int`
  - Nat 演算: `Add`, `Sub`, `Mul`, `Div`, `Mod`, `Eq`, `Lt`
  - キャッシュ、循環検出、依存関係追跡
  - 条件組み合わせ: `And`, `Or`, `Not`

- ⚠️ `conditional_types.rs` が存在（基本フレームワーク）

- ❌ 型正規化器との完全統合（実装待ち）
- ❌ 標準ライブラリ `Assert` 実装（実装待ち）

#### 7.3 需要的实现

```
src/frontend/type_level/type_match.rs               # ✅ 実装済み
src/frontend/type_level/type_families.rs            # ✅ 実装済み
src/frontend/typecheck/type_eval.rs                 # ✅ 実装済み
src/frontend/type_level/evaluation/mod.rs          # 変更：評価器を統合
```

#### 7.4 验收标准

- [x] `If[C, T, E]` 条件型をサポート
- [x] Bool 型族をサポート (True, False)
- [x] Nat 型族をサポート (Zero, Succ)
- [x] 型レベル match 式をサポート
- [x] コンパイル時型計算（If, Nat 演算）
- [ ] 型正規化器との完全統合
- [ ] 標準ライブラリ `Assert` 実装（コンパイル時アサート）
  ```yaoxiang
  type Assert[C: Bool] = match C {
      True => Void,
      False => compile_error("Assertion failed"),
  }
  ```

---

### 8. インライン最適化と特殊化の組み合わせ

#### 8.1 功能描述

RFC-011 設計関数オーバーロードとインライン最適化が自然に組み合わせ可能：

```yaoxiang
sum: (arr: Array[Int]) -> Int = (arr) => {
    native_sum_int(arr.data, arr.length)
}

# 使用時にコンパイラが自動選択してインライン化
result = sum(int_arr)  # => native_sum_int(int_arr.data, int_arr.length)
```

#### 8.2 当前状态

- ❌ 特殊化+インライン連携なし
- ❌ オプティマイザに特殊化認識なし
- ❌ インライン決定に特殊化考慮なし

#### 8.3 需要的实现

```
src/middle/optimizer/specialization_aware_inlining.rs  # 新規：特殊化認識インライン化
src/middle/passes/opt/size_analysis.rs                 # 新規：関数サイズ分析
```

#### 8.4 验收标准

- [ ] 特殊化後のコードがさらにインライン化可能
- [ ] 小さい特殊化体が自動的に呼び出し点にインライン化
- [ ] 生成コードが手書き最適化と同等

---

## 优先级排序（2026-02-04 更新）

| 优先级 | 功能 | 预估工期 | 依赖 | 状态 |
|--------|------|----------|------|------|
| **P0** | 完全DCE | 1週間 | 基本的モノーファイヤ | 90% - 締めくくり |
| **P1** | 関数オーバーロード特殊化統合 | 2週間 | オーバーロード解決 | 75% - 改善中 |
| **P2** | 条件型統合 | 2週間 | 型正規化器 | 65% - 中期 |
| **P3** | プラットフォーム特殊化統合 | 2週間 | モノーファイヤ | 50% - 中期 |
| **P4** | コンパイル時ジェネリクス完全 | 3週間 | Phase 4 | 40% - 中期 |
| **P5** | 完全Traitシステム | 4週間 | Phase 2 | 10% - 長期 |
| **P6** | 関連型 | 4週間 | Traitシステム | 5% - 長期 |
| **P7** | 特殊化認識インライン化 | 2週間 | P1 + オプティマイザ | 0% - 長期 |
| **P8** | マクロ置換 | 3週間 | ジェネリクス+Trait | 0% - 長期 |

### 下一步建议

**短期 (1-2週間)**：
1. DCE の締めくくり作業を完了
2. 条件型を型正規化器に統合
3. プラットフォーム特殊化をモノーファイヤに統合

**中期 (1ヶ月)**：
1. 関数オーバーロードとジェネリクスの統合を改善
2. コンパイル時ジェネリクスを改善（リテラルパラメータ）
3. Trait システム基礎実装を開始

**長期 (2-3ヶ月)**：
1. 関連型 (GAT)
2. 特殊化認識インライン化
3. マクロ置換機能

---

## 实现建议

### 短期目标 (1-2ヶ月)

1. **基本的DCEを完了**
   - インスタンス化グラフを構築
   - 到達可能性分析を実装
   - これにより大部分の不要なコード膨張が解消

2. **関数オーバーロード特殊化を実装**
   - これは RFC-011 のコア機能
   - 特殊な最適化を後押し

### 中期目标 (3-4ヶ月)

1. **完全Traitシステム**
   - ジェネリクス制約を後押し
   - 標準ライブラリに基盤を提供

2. **コンパイル時ジェネリクス**
   - コンパイル時計算を後押し
   - 静的配列最適化をサポート
   - `const` キーワードが不要

### 长期目标 (5-6ヶ月)

1. **条件型**
   - 型レベルプログラミング
   - より強力なジェネリクス能力

2. **プラットフォーム特殊化**
   - SIMD最適化
   - アーキテクチャ固有コード

---

## 相关文件清单

### 已实现模块（部分/基础）

| 文件路径 | 状态 | 说明 |
|----------|------|------|
| `src/middle/passes/mono/mod.rs` | ⚠️ 70% | モノーファイヤ本体 |
| `src/middle/passes/mono/function.rs` | ⚠️ 70% | 関数モノーフィケーション |
| `src/middle/passes/mono/type_mono.rs` | ⚠️ 50% | 型モノーフィケーション |
| `src/middle/passes/mono/closure.rs` | ⚠️ 50% | クロージャモノーフィケーション |
| `src/middle/passes/mono/platform_info.rs` | ✅ 80% | プラットフォーム情報検出 |
| `src/middle/passes/mono/platform_specializer.rs` | ✅ 50% | プラットフォーム特殊化 |
| `src/frontend/type_level/mod.rs` | ⚠️ 40% | 型レベル計算エントリーポイント |
| `src/frontend/type_level/conditional_types.rs` | ⚠️ 35% | 条件型フレームワーク |
| `src/frontend/type_level/const_generics/mod.rs` | ⚠️ 40% | コンパイル時ジェネリクスフレームワーク |
| `src/frontend/type_level/evaluation/compute.rs` | ⚠️ 30% | 型レベル計算 |
| `src/frontend/type_level/type_match.rs` | ✅ 70% | 型レベル match |
| `src/frontend/type_level/type_families.rs` | ✅ 60% | 型族 (Bool/Nat) |
| `src/frontend/typecheck/type_eval.rs` | ✅ 65% | コンパイル時型評価器 |
| `src/frontend/typecheck/gat/mod.rs` | ⚠️ 5% | GAT 基礎構造 |
| `src/frontend/typecheck/traits/mod.rs` | ⚠️ 10% | Trait 基礎構造 |

### 需要新增/完善的模块

| 文件路径 | 功能 | 状态 |
|----------|------|------|
| `src/frontend/core/parser/overload.rs` | 関数オーバーロード解析 | 既に存在 |
| `src/frontend/core/parser/trait_def.rs` | Trait定義解析 | ❌ 未実装 |
| `src/frontend/core/parser/literal_param.rs` | リテラル型パラメータ解析 | ❌ 未実装 |
| `src/frontend/core/parser/attr.rs` | 属性解析（任意） | ❌ 未実装 |
| `src/frontend/typecheck/trait_resolution.rs` | Trait制約解決 | ⚠️ 一部 |
| `src/frontend/typecheck/trait_impl.rs` | Trait実装チェック | ⚠️ 一部 |
| `src/frontend/typecheck/const_eval.rs` | コンパイル時式評価 | ⚠️ 一部 |
| `src/frontend/type_level/associated_types.rs` | 関連型 | ❌ 未実装 |
| `src/middle/passes/mono/instantiation_graph.rs` | インスタンス化グラフ | 既に存在 |
| `src/middle/passes/mono/reachability.rs` | 到達可能性分析 | 既に存在 |
| `src/middle/passes/mono/cross_module_dce.rs` | クロスモジュールDCE | 既に存在 |
| `src/middle/optimizer/specialization_aware_inlining.rs` | 特殊化認識インライン化 | ❌ 未実装 |

---

## 付録：RFC-011 設計回顾

### 核心特性清单（2026-02-04 更新）

| 特性 | RFC設計 | 当前实现 | 差距 | 优先级 |
|------|---------|----------|------|--------|
| 基础ジェネリクス `[T]` | ✅ | ✅ 70% | 改善が必要 | P1 |
| 型推論 | ✅ | ⚠️ 基礎 | 拡張が必要 | P1 |
| 型制約 (Trait) | ✅ | ⚠️ 10% | 実装が必要 | P2 |
| 関連型 (GAT) | ✅ | ⚠️ 5% | 実装が必要 | P4 |
| コンパイル時ジェネリクス | ✅ | ⚠️ 40% | 改善が必要 | P3 |
| 条件型 | ✅ | ✅ 65% | 条件型フレームワーク完了 | P2 |
| 関数特殊化 | ✅ | ✅ 75% | オーバーロードメカニズム完了 | P1 |
| プラットフォーム特殊化 | ✅ | ⚠️ 50% | 基礎構造完了 | P2 |
| 完全DCE | ✅ | ⚠️ 90% | ほぼ完了 | P0 |
| マクロ置換 | ✅ | ❌ 0% | 実装が必要 | P5 |
| 特殊化認識インライン化 | ✅ | ❌ 0% | 実装が必要 | P5 |

> **更新说明 (2026-02-04)**：
> - 条件型 35% → **65%**：`type_match.rs`, `type_families.rs`, `type_eval.rs` 実装済み
> - プラットフォーム特殊化 0% → **50%**：`platform_info.rs`, `platform_specializer.rs` 実装済み
> - 関連型 0% → **5%**：`src/frontend/typecheck/gat/` 基礎構造が作成済み

### 依赖关系图

```
基础ジェネリクス ([T])
    │
    ├──> 型制約システム (Trait)
    │        │
    │        ├──> 関連型 (GAT)
    │        │
    │        └──> Trait継承
    │
    ├──> コンパイル時ジェネリクス
    │        │
    │        ├──> リテラル型パラメータ
    │        │
    │        └──> コンパイル時計算
    │
    └──> 条件型
             │
             └──> 型レベルプログラミング

関数オーバーロード特殊化 ─────────────> プラットフォーム固有最適化
                             │
                             └──> 特殊化認識インライン化

完全DCE ──────────────────> クロスモジュールDCE
                             │
                             └──> コード膨張制御
```