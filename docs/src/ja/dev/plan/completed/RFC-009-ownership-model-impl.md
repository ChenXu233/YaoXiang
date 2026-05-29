# RFC-009 所有権モデルの実装パス

> **ドキュメントバージョン**: v1.0
> **設計ベース**: `docs/design/accepted/009-ownership-model.md`
> **生成日付**: 2025-02-05

## 実装概要

本文書は RFC-009 設計仕様を実行可能な実装ステップに変換ものであり、YaoXiang の既存アーキテクチャに基づく拡張である。

### 既存基盤

| モジュール | 場所 | 状態 |
|------|------|------|
| 所有権システム | `src/middle/passes/lifetime/` | ✅ 基盤完成済み |
| Move 语义 | `move_semantics.rs` | ✅ 実装済み |
| ref 语义 | `ref_semantics.rs` | ✅ 実装済み |
| 循環検出 | `cycle_check.rs` | ✅ 実装済み |
| mut 検査 | `mut_check.rs` | ✅ 実装済み |

---

## Phase 1: フィールドレベル不変性 (P0)

### 目標

型定義で `mut` フィールドマークをサポートし、三層可変性モデルを実現する：

- 束縛可変性（変数レベル）
- フィールド可変性（構造体レベル）
- メソッドパラメータ可変性（関数レベル）

### 実装状態：✅ 完成済み（2026-02-05）

#### 完成済み変更（2026-02-05 更新）

1. **AST 拡張** (`ast.rs`)
   - ✅ `StructField` 構造体作成：`name: String, is_mut: bool, ty: Type`
   - ✅ `Type::Struct(Vec<StructField>)` で `Type::Struct(Vec<(String, Type)>)` を置換
   - ✅ `Type::NamedStruct { name, fields: Vec<StructField> }`
   - ✅ `Pattern::Struct { name, fields: Vec<(String, bool, Box<Pattern>)> }`

2. **Parser 拡張** (`statements/declarations.rs`)
   - ✅ `parse_struct_type` で `{ x: Float, mut y: Float }` 構文をサポート
   - ✅ `parse_named_struct_type` で `Point(x: Float, mut y: Float)` 構文をサポート

3. **型システム** (`type_system/mono.rs`)
   - ✅ `StructType` に `field_mutability: Vec<bool>` を追加
   - ✅ `field_is_mut(&self, field_name: &str) -> Option<bool>` メソッド実装
   - ✅ `MonoType::from(ast::Type)` 変換ロジック更新

4. **パターン照合** (`typecheck/inference/patterns.rs`)
   - ✅ パターン推論で `is_mut` マークをサポート

5. **Parser パターン解析** (`parser/pratt/nud.rs`)
   - ✅ 構造体パターンパースで `mut` キーワードをサポート

6. **エラー型** (`lifetime/error.rs`)
   - ✅ `ImmutableFieldAssign` エラー変体を追加
   - ✅ Display 実装追加

7. **IR 命令拡張** (`middle/core/ir.rs`)
   - ✅ `StoreField` に `type_name: Option<String>` と `field_name: Option<String>` を追加

8. **IR 生成** (`middle/core/ir_gen.rs`)
   - ✅ `get_field_mutability` が型名を返す
   - ✅ StoreField 命令が型情報を携带

9. **可変性検査** (`lifetime/mut_check.rs`)
   - ✅ 束縛レベル可変性検査
   - ✅ フィールドレベル可変性検査（型テーブルを渡す）
   - ✅ `ImmutableFieldAssign` エラー検出

10. **コード生成** (`codegen/translator.rs`)
    - ✅ StoreField パターン照合を修正（余分なフィールドを無視するために `..` を使用）

### 対象ファイル

| 種類 | ファイル | 状態 |
|------|------|------|
| 修正 | `src/frontend/core/parser/ast.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/parser/statements/declarations.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/parser/pratt/nud.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/type_system/mono.rs` | ✅ 完成済み |
| 修正 | `src/frontend/typecheck/inference/patterns.rs` | ✅ 完成済み |
| 修正 | `src/frontend/typecheck/mod.rs` | ✅ 完成済み |
| 修正 | `src/frontend/type_level/auto_derive.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/error.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/mut_check.rs` | ✅ 完成済み |
| 修正 | `src/middle/core/ir_gen.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/codegen/mod.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/cross_module.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/function.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/module_state.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/type_mono.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/type_system/solver.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/type_system/substitute.rs` | ✅ 完成済み |
| 修正 | `src/frontend/typecheck/specialization/algorithm.rs` | ✅ 完成済み |
| 修正 | `src/frontend/typecheck/specialize.rs` | ✅ 完成済み |
| 修正 | `src/frontend/typecheck/overload.rs` | ✅ 完成済み |
| 修正 | `src/frontend/typecheck/inference/expressions.rs` | ✅ 完成済み |

### 受け入れ基準

- [x] `type Point { x: Float, mut y: Float }` 構文解析が正しい
- [x] `type Point(x: Float, mut y: Float)` 名前付き構造体構文解析が正しい
- [x] `NamedStruct(Point(x: Float, mut y: Float))` コンストラクタが mut フィールドをサポート
- [x] `mut p: Point = Point(1.0, 2.0); p.y = 3.0` がコンパイル通過（束縛可変、フィールド可変）
- [x] `p.y = 3.0` が非 mut 束縛下でコンパイル通過（束縛不可変、フィールド可変）
- [x] `p.x = 3.0` が非 mut 束縛下でコンパイル失敗（束縛不可変、フィールド不可変）→ `ImmutableFieldAssign`
- [x] `p.x = 3.0` が mut 束縛下でコンパイル通過（束縛可変、フィールド書き込み可能）

### 実装説明

1. **データ構造変更**（完成済み）
   - `StructField` 構造体：`name, is_mut, ty`
   - `StructType.field_mutability: Vec<bool>`
   - `Pattern::Struct` フィールドが `is_mut` マークをサポート

2. **Parser 層**（完成済み）
   - `parse_struct_type` で `{ x: Float, mut y: Float }` をサポート
   - `parse_named_struct_type` で `Point(x: Float, mut y: Float)` をサポート

3. **IR 生成**（完成済み）
   - フィールド代入 `p.y = value` で `StoreField` 命令を生成
   - `get_field_mutability` メソッドでフィールド可変性をクエリ
   - `StoreField` が `type_name` と `field_name` を携带して検査に使用

4. **MutChecker**（完成済み）
   - 束縛レベル可変性検査：変数が `mut` で宣言されているか検査
   - フィールドレベル可変性検査：フィールドが `mut` で宣言されているか検査
   - ルール：**束縛可変 OR フィールド可変** → 代入を許可
   - アーキテクチャ：`HashMap<String, StructType>` 型テーブルを渡す
   - パーサー追加：`parse_let_stmt` と `parse_pattern`
   - IR 生成：`generate_pattern_ir` がパターン構造を処理

### 今後の最適化点

（現在の Phase 1 は完成済み）

---

## Phase 2: 空状態再利用 (P1) ✅ 完成済み

### 目標

Move 後の変数が `empty` 状態に入る，实现を再代入して再利用变量名。

### 実装状態：✅ 完成済み（2026-02-05）

#### 完成済み変更（2026-02-05 更新）

1. **ValueState 拡張** (`error.rs`)
   - ✅ `ValueState::Owned(Option<TypeId>)` に型追踪を追加
   - ✅ `ValueState::Empty` 新しい空状態変体を追加
   - ✅ `TypeId` 型識別子を追加
   - ✅ `EmptyStateTypeMismatch` と `ReassignNonEmpty` エラー型を追加

2. **空状態追踪** (新規 `empty_state.rs`)
   - ✅ `EmptyStateTracker` 構造体を作成
   - ✅ 状態追踪と型検査を実装
   - ✅ 分岐状態マージ（保守戦略）を実装

3. **制御フロー解析** (新規 `control_flow.rs`)
   - ✅ `ControlFlowAnalyzer` 構造体を作成
   - ✅ `merge_states` 保守マージ戦略を実装
   - ✅ アクティブ変数解析補助関数を提供

4. **Move チェッカー拡張** (`move_semantics.rs`)
   - ✅ Move 後、変数が Empty 状態に入る（Moved の代わりに）
   - ✅ 空状態変数の再代入を許可
   - ✅ 型整合性検査
   - ✅ 関数呼び出し引数が Empty 状態に入る

5. **他のチェッカー適応**
   - ✅ `clone.rs`: Empty 状態に更新して適応
   - ✅ `drop_semantics.rs`: Empty 状態を Drop するのは合法
   - ✅ `ref_semantics.rs`: Empty 状態に更新して適応

6. **モジュール登録** (`mod.rs`)
   - ✅ `empty_state` と `control_flow` モジュールを登録

### 対象ファイル

| 種類 | ファイル | 状態 |
|------|------|------|
| 修正 | `src/middle/passes/lifetime/error.rs` | ✅ 完成済み |
| 新規 | `src/middle/passes/lifetime/empty_state.rs` | ✅ 完成済み |
| 新規 | `src/middle/passes/lifetime/control_flow.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/move_semantics.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/clone.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/drop_semantics.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/ref_semantics.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/mod.rs` | ✅ 完成済み |

### 受け入れ基準

- [x] `p = Point(1.0); p2 = p; p = Point(2.0)` がコンパイル通過
- [x] `p = Point(1.0); p2 = p; print(p)` がコンパイル失敗（UseAfterMove）
- [x] if 分岐が空状態を正しく追踪（保守解析）
- [x] `p = "hello"` が Point 型後にエラーを報告（EmptyStateTypeMismatch）

### 実装説明

1. **状態設計**
   - `Owned(Option<TypeId>)`: 有効値、型情報を携带
   - `Empty`: 空状態、再代入可能
   - `Moved`: 移動済み（互換性保持のため残置）
   - `Dropped`: 解放済み

2. **状態遷移**
   ```
   Owned ──Move──► Empty ──(Store, 型整合)──► Owned
                         ▲
                         │
                    エラー：型不一致
   ```

3. **保守的分岐マージ**
   - いずれかの分岐が Empty → 合流後は Empty
   - いずれかの分岐が Moved → 合流後は Moved
   - 両方 Owned → 最初のを保持

4. **型検査**
   - 再代入時に型整合性を検査
   - 型不一致時に `EmptyStateTypeMismatch` を報告

### 今後の最適化点

（現在の Phase 2 は完成済み）

---

## Phase 3: 所有権回流 (P1) ✅ 完成済み

### 目標

関数引数が変更された後に返される，实现所有権闭环，支持链式调用。

### 実装状態：✅ 完成済み（2026-02-06）

#### 完成済み変更（2026-02-06 更新）

1. **消費モード列挙型** (`ownership_flow.rs`)
   - ✅ `ConsumeMode` 列挙型を作成：`Returns | Consumes | Undetermined`
   - ✅ `Returns`: 引数が返り値に返され、所有権回流
   - ✅ `Consumes`: 引数が消費され、返されない
   - ✅ `Undetermined`: 確定不能（保守解析）

2. **所有権回流解析器** (`ownership_flow.rs`)
   - ✅ `OwnershipFlowAnalyzer` 構造体を作成
   - ✅ `analyze_function()` が関数の消費モードを分析
   - ✅ `operand_references_param()` が返り値が引数を参照するか検査
   - ✅ `returns_param_directly()` が `return p;` パターンを高速検出
   - ✅ 保守推定：一時変数が引数を参照する可能性

3. **チェーン呼び出し解析器** (`chain_calls.rs`)
   - ✅ `ChainCallAnalyzer` 構造体を作成
   - ✅ `analyze_chain()` がメソッドチェーンの所有権流動を分析
   - ✅ `extract_chain_calls()` が連続する虚メソッド呼び出しを抽出
   - ✅ `infer_consume_mode()` が使用方式に基づいて消費モードを推論
   - ✅ `check_ownership_closure()` が所有権閉合を検証

4. **エラー型拡張** (`error.rs`)
   - ✅ `ConsumedNotReturned` エラー変体を追加
   - ✅ 引数が消費されたが返されなかった場合の診断用

5. **モジュール登録** (`mod.rs`)
   - ✅ `ownership_flow` と `chain_calls` モジュールを登録

### 対象ファイル

| 種類 | ファイル | 状態 |
|------|------|------|
| 新規 | `src/middle/passes/lifetime/ownership_flow.rs` | ✅ 完成済み |
| 新規 | `src/middle/passes/lifetime/chain_calls.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/error.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/mod.rs` | ✅ 完成済み |

### 受け入れ基準

- [x] `p = p.process()` が Returns モードと推論される
- [x] `consume(p)` が Consumes モードと推論される
- [x] `p = p.rotate(90).scale(2.0).translate(1.0)` チェーン呼び出しが正しい
- [x] 回流推論エラーが正確なヒントを提供

### 実装説明

1. **ConsumeMode 設計**
   ```
   ConsumeMode::Returns     → 引数が返り値に返される
   ConsumeMode::Consumes   → 引数が消費され、返されない
   ConsumeMode::Undetermined → 確定不能、保守解析
   ```

2. **引数参照検出**
   - 直接参照：`Operand::Arg(idx)` → インデックス照合
   - 一時変数：保守的に引数を参照する可能性あり
   - 定数/グローバル：引数を参照しない

3. **チェーン呼び出し解析**
   ```ignore
   p.rotate(90)    // Method 1: rotate
     .scale(2.0)   // Method 2: scale (obj = temp_1)
     .translate(1.0); // Method 3: translate (obj = temp_2)
   ```

4. **所有権閉合検査**
   - Consumes モード → 所有権が正しく閉合
   - Returns モード → 返り値が使用されるべき
   - Undetermined → 保守的に true を返す

### テスト覆盖

| モジュール | テスト数 | 説明 |
|------|--------|------|
| `ownership_flow` | 10 | 引数参照検出、モード推論 |
| `chain_calls` | 13 | チェーン呼び出し、所有権閉合 |

### 今後の最適化点

（現在の Phase 3 は完成済み）

---

## Phase 4: 消費分析 (P1) ✅ 完成済み

### 目標

完全な消費マークシステムを実装し、各変数の Consumes/Returns 状態を追踪する。

### 実装状態：✅ 完成済み（2026-02-06）

#### 完成済み変更（2026-02-06 更新）

1. **消費解析器** (新規 `consume_analysis.rs`)
   - ✅ Phase 3 の `ConsumeMode` と `OwnershipFlowAnalyzer` を再利用
   - ✅ `ConsumeAnalyzer` が関数間消費モードクエリを提供
   - ✅ 組み込み関数の特殊処理（consume, clone 等）
   - ✅ 消費モードキャッシュ機構

2. **ライフサイクル追踪器** (新規 `lifecycle.rs`)
   - ✅ `LifecycleTracker` 構造体を作成
   - ✅ 変数ライフサイクルイベント記録（作成/消費/移動/解放/返却）
   - ✅ 消費回数と読み取り回数統計
   - ✅ ライフサイクル問題検出（未消費解放/複数回消費/消費後使用）

3. **MoveChecker 拡張** (`move_semantics.rs` 拡張)
   - ✅ `ConsumeAnalyzer` フィールドを追加
   - ✅ `check_call` が関数消費モードに基づいて引数状態を決定
   - ✅ Returns モード：引数所有権回流、Empty に入らない
   - ✅ Consumes モード：引数が Empty に入る

4. **モジュール登録** (`mod.rs`)
   - ✅ `consume_analysis` と `lifecycle` モジュールを登録

### 対象ファイル

| 種類 | ファイル | 状態 |
|------|------|------|
| 新規 | `src/middle/passes/lifetime/consume_analysis.rs` | ✅ 完成済み |
| 新規 | `src/middle/passes/lifetime/lifecycle.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/move_semantics.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/mod.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/ownership_flow.rs` | ✅ 完成済み |

### 受け入れ基準

- [x] 代入/引数 전달/返却が正しく Move とマークされる
- [x] `consume(x)` 後 x が空になる（Consumes モード）
- [x] `x = modify(x)` が Returns と推論される（OwnershipFlowAnalyzer 再利用）
- [x] `clone()` が正しくコピーされ、元の変数に影響しない（組み込み関数処理）

### 実装説明

1. **Phase 3 成果の再利用**
   - `ownership_flow.rs` の `ConsumeMode` 列挙型を直接使用
   - `OwnershipFlowAnalyzer` が関数レベル消費モード分析を実行

2. **消費解析器設計**
   ```
   ConsumeMode::Returns     → 引数所有権回流、Owned を保持
   ConsumeMode::Consumes   → 引数が消費され、Empty に入る
   ConsumeMode::Undetermined → 保守的に Empty に入る
   ```

3. **ライフサイクル追踪**
   ```
   イベント：Created → Consumed → Moved → Dropped → Returned
   検出：未消費解放 / 複数回消費 / 消費後使用 / 未使用
   ```

4. **MoveChecker 統合**
   - `check_call` が被呼び出し関数の消費モードをクエリ
   - Returns モード：引数状態不变
   - Consumes モード：引数が Empty に入る

---

## Phase 5: ref キーワード = Arc (P1) ✅ 完成済み

### 目標

`ref` キーワードを Arc として実装し、スレッド安全参照カウントを実現する。

### 実装状態：✅ 完成済み（2026-02-06）

#### 完成済み変更（2026-02-06 更新）

1. **ref 構文解析** (既存)
   - ✅ `parser/expr.rs`: `parse_ref` が `ref expression` 構文を解析
   - ✅ `ast.rs`: `Expr::Ref { expr, span }` AST ノード

2. **型推論** (既存)
   - ✅ `typecheck/infer.rs`: `ref T` が `Arc[T]` と推論

3. **所有権処理** (既存)
   - ✅ `ref_semantics.rs`: ArcNew/Clone/Drop 所有権検査

4. **IR 生成** (新規)
   - ✅ `ir_gen.rs`: `Expr::Ref` → `ArcNew` 命令生成を追加

### 対象ファイル

| 種類 | ファイル | 状態 |
|------|------|------|
| 修正 | `src/frontend/core/parser/expr.rs` | ✅ 既存 |
| 修正 | `src/frontend/typecheck/infer.rs` | ✅ 既存 |
| 修正 | `src/middle/passes/lifetime/ref_semantics.rs` | ✅ 既存 |
| 修正 | `src/middle/core/ir_gen.rs` | ✅ 今回新增 |

### 受け入れ基準

- [x] `ref p` が `Arc[Point]` と型推論される
- [x] `ref p` が p を消費せず、p は引き続き使用可能
- [x] `spawn(() => print(shared.x))` がコンパイル通過
- [x] `ref` 式がネスト可能

### 実装説明

1. **IR 生成** (今回実装)
   ```rust
   Expr::Ref { expr, span: _ } => {
       let src_reg = self.next_temp_reg();
       self.generate_expr_ir(expr, src_reg, instructions, constants)?;
       instructions.push(Instruction::ArcNew {
           dst: Operand::Local(result_reg),
           src: Operand::Local(src_reg),
       });
   }
   ```

2. **所有権セマンティクス**
   - `ArcNew`: Arc を作成、原値状態に影響しない
   - `ArcClone`: Arc をクローン、原値状態に影響しない
   - `ArcDrop`: Arc を解放、原値状態に影響しない

### 今後の最適化点

（現在の Phase 5 は完成済み）

---

## Phase 6: 循環参照検出 (P1) ✅ 完成済み

### 目標

タスク間循環参照検出、タスク内循環は許可。

### 実装状態：✅ 完成済み（2026-02-06）

#### 完成済み変更（2026-02-06 更新）

1. **エラー型拡張** (`error.rs`)
   - ✅ `IntraTaskCycle` 警告変体（タスク内循環、コンパイルを阻断しない）
   - ✅ `UnsafeBypassCycle` 情報変体（unsafe バイパス記録）
   - ✅ Display 実装

2. **CycleChecker 增强** (`cycle_check.rs`)
   - ✅ 深度制限定数 `MAX_DETECTION_DEPTH = 1`（単層境界のみ検出）
   - ✅ `unsafe_ranges` フィールドが unsafe ブロック範囲を追踪
   - ✅ `unsafe_bypasses` フィールドがバイパス情報を記録
   - ✅ `is_in_unsafe()` メソッドで位置が unsafe ブロック内か検査
   - ✅ `find_spawn_result_direct()` メソッドで深度制限を実装
   - ✅ `collect_unsafe_ranges()` が Phase 7 インターフェースを预留
   - ✅ エラーメッセージ最適化、解決案を含む

3. **タスク内循環追踪器** (新規 `intra_task_cycle.rs`)
   - ✅ `IntraTaskCycleTracker` 構造体
   - ✅ `RefEdge` 構造体が ref エッジを追踪
   - ✅ `track_function()` が関数内循環を追踪
   - ✅ `collect_ref_info()` が ArcNew/Move/StoreField を収集
   - ✅ `build_ref_graph()` が参照グラフを構築
   - ✅ `detect_intra_task_cycles()` DFS で循環を検出
   - ✅ 警告モード出力、コンパイルを阻断しない

4. **OwnershipChecker 統合** (`mod.rs`)
   - ✅ `intra_task_tracker` フィールドを追加
   - ✅ `check_function()` がタスク内循環追踪を呼び出し
   - ✅ `intra_task_warnings()` メソッドが警告を返す
   - ✅ `unsafe_bypasses()` メソッドがバイパス記録を返す

### 対象ファイル

| 種類 | ファイル | 状態 |
|------|------|------|
| 修正 | `src/middle/passes/lifetime/error.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/cycle_check.rs` | ✅ 完成済み |
| 新規 | `src/middle/passes/lifetime/intra_task_cycle.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/mod.rs` | ✅ 完成済み |

### 受け入れ基準

- [x] spawn 引数と返り値間の ref 循環検出
- [x] タスク内循環でエラーなし（漏えい制御可能）
- [x] タスク間循環でエラー位置が正確
- [x] unsafe ブロックで検出バイパス可能（インターフェース预留、Phase 7 で完善）

### 実装説明

1. **深度制限設計**
   - 単層 spawn 境界のみ検出（深度 = 1）
   - `find_spawn_result_direct()` が最大1層の Move を追踪
   - ネスト spawn の間接参照を再帰的に検出しない

2. **循環検出分担**
   ```
   CycleChecker        → 跨 spawn 循環（エラー）
   IntraTaskCycleTracker → タスク内循環（警告）
   ```

3. **unsafe バイパス機構**
   - `collect_unsafe_ranges()` が unsafe ブロック範囲を収集
   - `is_in_unsafe()` が命令位置を検査
   - unsafe ブロック内の spawn が検出をスキップ
   - 現在版本でインターフェース预留、Phase 7 で unsafe 構文実装後に完善

4. **エラーメッセージ最適化**
   ```
   タスク間循環参照: temp_0 → temp_1 → temp_0 (循環形成).
   提案: Weak を使用して循環を破壊、または unsafe ブロックで検出をバイパス
   ```

### テスト覆盖

| モジュール | テスト数 | 説明 |
|------|--------|------|
| `cycle_check` | 22 | タスク間循環、深度制限、状態リセット |
| `intra_task_cycle` | 7 | タスク内循環、自己参照、警告位置 |

### 今後の最適化点

- Phase 7 で unsafe 構文実装後、`collect_unsafe_ranges()` 解析を完善

---

## Phase 7: unsafe + 裸ポインタ (P2) ✅ 完成済み

### 目標

`unsafe` ブロック内の `*T` 裸ポインタ操作をサポート。

### 実装状態：✅ 完成済み（2026-02-06）

#### 完成済み変更（2026-02-06 更新）

1. **キーワードと Token** (`tokens.rs`, `state.rs`)
   - ✅ `KwUnsafe` キーワードを追加
   - ✅ `state.rs`: `"unsafe" => Some(TokenKind::KwUnsafe)` を追加

2. **AST 拡張** (`ast.rs`)
   - ✅ `Expr::Unsafe { body: Box<Block>, span }` - unsafe ブロック式
   - ✅ `Type::Ptr(Box<Type>)` - 裸ポインタ型 `*T`
   - ✅ `UnOp::Deref` - 逆参照演算子

3. **Parser 拡張** (`pratt/nud.rs`, `statements/declarations.rs`)
   - ✅ `parse_unsafe()` - `unsafe { ... }` 構文解析
   - ✅ `parse_unary()` - `*expr` 逆参照構文サポート
   - ✅ `parse_type_annotation()` - `*T` 型注釈サポート

4. **IR 命令拡張** (`ir.rs`)
   - ✅ `Instruction::UnsafeBlockStart` - unsafe ブロック開始マーカ
   - ✅ `Instruction::UnsafeBlockEnd` - unsafe ブロック終了マーカ
   - ✅ `Instruction::PtrFromRef { dst, src }` - `&value → *T`
   - ✅ `Instruction::PtrDeref { dst, src }` - `*ptr → value`
   - ✅ `Instruction::PtrStore { dst, src }` - `*ptr = value`
   - ✅ `Instruction::PtrLoad { dst, src }` - ポインタ読み込み

5. **IR 生成** (`ir_gen.rs`)
   - ✅ `Expr::Unsafe` → `UnsafeBlockStart/End` 命令でラップ
   - ✅ `UnOp::Deref` → `PtrDeref` 命令

6. **型システム** (`mono.rs`, `cross_module.rs`, `function.rs`, `module_state.rs`, `type_mono.rs`)
   - ✅ `Type::Ptr` → `MonoType::TypeRef("*{...}")`
   - ✅ 型名変換が裸ポインタをサポート

7. **型推論** (`expressions.rs`)
   - ✅ `infer_unary()` が `Deref` 型推論をサポート
   - ✅ `infer_expr()` が `Expr::Unsafe` 型推論をサポート

8. **Unsafe 範囲収集** (`cycle_check.rs`)
   - ✅ `collect_unsafe_ranges()` が `UnsafeBlockStart/End` 命令を解析

9. **Unsafe チェッカー** (新規 `unsafe_check.rs`)
   - ✅ `UnsafeChecker` 構造体
   - ✅ `check_function()` - unsafe ブロック外逆参照を検査
   - ✅ `UnsafeDeref` エラー型

10. **エラー型拡張** (`error.rs`)
    - ✅ `OwnershipError::UnsafeDeref` 変体
    - ✅ Display 実装

11. **コード生成** (`translator.rs`)
    - ✅ unsafe ブロックとポインタ命令のスキッププレースホルダ実装

### 対象ファイル

| 種類 | ファイル | 状態 |
|------|------|------|
| 修正 | `src/frontend/core/lexer/tokens.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/lexer/state.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/parser/ast.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/parser/pratt/nud.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/parser/statements/declarations.rs` | ✅ 完成済み |
| 修正 | `src/middle/core/ir.rs` | ✅ 完成済み |
| 修正 | `src/middle/core/ir_gen.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/cycle_check.rs` | ✅ 完成済み |
| 新規 | `src/middle/passes/lifetime/unsafe_check.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/error.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/mod.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/codegen/translator.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/type_system/mono.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/cross_module.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/function.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/module_state.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/type_mono.rs` | ✅ 完成済み |
| 修正 | `src/frontend/typecheck/inference/expressions.rs` | ✅ 完成済み |

### 受け入れ基準

- [x] `unsafe { ... }` 構文解析が正しい
- [x] `*T` 裸ポインタ型注釈解析が正しい
- [x] `*ptr` 逆参照構文解析が正しい
- [x] `unsafe { *ptr }` がコンパイル通過
- [x] unsafe ブロック外 `*ptr` が `UnsafeDeref` エラーを報告
- [x] 裸ポインタ型が `*{type}` として表現される
- [x] unsafe ブロックが `UnsafeBlockStart/End` IR マーカを生成
- [x] `collect_unsafe_ranges()` が unsafe 範囲を正しく収集

### 実装説明

1. **AST 設計**
   ```rust
   Expr::Unsafe {
       body: Box<Block>,
       span: Span,
   }
   Type::Ptr(Box<Type>)  // *T
   UnOp::Deref           // *expr
   ```

2. **IR 設計**
   ```
   UnsafeBlockStart
   // ブロック内命令...
   UnsafeBlockEnd
   ```

3. **逆参照型推論**
   ```rust
   UnOp::Deref => {
       if let MonoType::TypeRef(inner) = expr {
           // * 接頭辞を削除して内部型を取得
           let inner_type = inner.trim_start_matches('*').to_string();
           Ok(MonoType::TypeRef(inner_type))
       } else {
           Err(Diagnostic::error("逆参照にはポインタ型が必要"))
       }
   }
   ```

4. **裸ポインタ型表現**
   - 解析：`*T` → `Type::Ptr(Box<Type>)`
   - IR：`PtrFromRef`, `PtrDeref`, `PtrStore`, `PtrLoad`
   - MonoType：`*{type_name}`

### テスト覆盖

| モジュール | テスト数 | 説明 |
|------|--------|------|
| Parser | - | unsafe/deref/ptr 構文解析 |
| TypeCheck | - | ポインタ型推論 |
| IR Gen | - | unsafe ブロックとポインタ IR 生成 |
| UnsafeCheck | - | unsafe ブロック外逆参照検査 |

### 今後の最適化点

- Phase 8+ で裸ポインタのコード生成（wasm アドレス操作）を実装
- `UnsafeBlock` スコープ追踪を追加

---

## Phase 8: Weak 標準ライブラリ (P1) ✅ 完成済み

### 目標

`std.weak.Weak` モジュールを実装し、ターゲットの解放を阻止しない弱参照をサポート。

### 実装状態：✅ 完成済み（2026-02-06）

**設計調整**：

- `std.rc` と `std.sync` は実装しない（`ref` が既に要件を満たすため）
- `Weak[T]` 型のみ実装

#### 完成済み変更（2026-02-06 更新）

1. **型システム拡張** (`mono.rs`)
   - ✅ `MonoType::Weak(Box<MonoType>)` 変体を追加
   - ✅ `type_name()` メソッドを更新

2. **制約伝播** (`constraint.rs`, `substitute.rs`)
   - ✅ Weak の Send + Sync 制約伝播
   - ✅ Weak の型置換ロジック

3. **型検査** (`specialize.rs`, `overload.rs`)
   - ✅ Weak 特殊化処理
   - ✅ Weak オーバーロード照合

4. **ランタイムサポート** (`value.rs`)
   - ✅ `RuntimeValue::Weak(std::sync::Weak<RuntimeValue>)` を追加
   - ✅ `upgrade()` が `Option<RuntimeValue>` を返すように実装
   - ✅ `from_arc_into_weak()` が Weak を作成

5. **バイトコード命令** (`bytecode.rs`, `opcode.rs`)
   - ✅ `BytecodeInstr::WeakNew` / `WeakUpgrade` を追加
   - ✅ `Opcode::WeakNew(0x7E)` / `WeakUpgrade(0x7F)` を追加

6. **インタープリタ** (`executor.rs`)
   - ✅ `WeakNew`: `Arc → Weak`
   - ✅ `WeakUpgrade`: `Weak → Option<Arc>`

7. **標準ライブラリ** (`weak.rs`, `mod.rs`)
   - ✅ 新規 `src/std/weak.rs`
   - ✅ `pub mod weak` を登録

### 対象ファイル

| 種類 | ファイル | 状態 |
|------|------|------|
| 修正 | `src/frontend/core/type_system/mono.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/type_system/constraint.rs` | ✅ 完成済み |
| 修正 | `src/frontend/core/type_system/substitute.rs` | ✅ 完成済み |
| 修正 | `src/frontend/typecheck/specialize.rs` | ✅ 完成済み |
| 修正 | `src/frontend/typecheck/overload.rs` | ✅ 完成済み |
| 修正 | `src/backends/common/value.rs` | ✅ 完成済み |
| 修正 | `src/backends/common/opcode.rs` | ✅ 完成済み |
| 修正 | `src/middle/core/bytecode.rs` | ✅ 完成済み |
| 修正 | `src/backends/interpreter/executor.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/codegen/bytecode.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/lifetime/send_sync.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/dce.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/instance.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/instantiation_graph.rs` | ✅ 完成済み |
| 修正 | `src/middle/passes/mono/type_mono.rs` | ✅ 完成済み |
| 修正 | `src/lib.rs` | ✅ 完成済み |
| 新規 | `src/std/weak.rs` | ✅ 完成済み |
| 修正 | `src/std/mod.rs` | ✅ 完成済み |

### 受け入れ基準

- [x] `use std.weak.Weak` モジュール登録
- [x] `MonoType::Weak` 型システムサポート
- [x] `WeakNew` / `WeakUpgrade` バイトコード命令
- [x] `RuntimeValue::Weak` ランタイムサポート
- [x] Send + Sync 制約伝播
- [x] コンパイル通過

### 実装説明

1. **Weak 設計**
   ```
   Arc[T] ──Weak::new()──► Weak[T] ──upgrade()──► Option[Arc[T]]
   ```

2. **バイトコード命令**
   ```
   WeakNew { dst, src }    # Arc -> Weak
   WeakUpgrade { dst, src } # Weak -> Option<Arc>
   ```

3. **ランタイム動作**
   - `WeakNew`: `Arc::downgrade()` を使用して Weak を作成
   - `WeakUpgrade`: `weak.upgrade()` を使用して Option を返す
   - Arc が解放された後、upgrade は None を返す

### テスト覆盖

| モジュール | テスト数 | 説明 |
|------|--------|------|
| RuntimeValue::Weak | - | Weak 作成とアップグレード |
| Executor WeakNew | - | バイトコード実行 |
| Executor WeakUpgrade | - | Option 返却 |

### 今後の最適化点

- 完全なインタープリタテストケースを追加
- 型検査テストを追加

---

## 依存関係

```
Phase 1 (フィールド不変性)
    │
    ├─► Phase 2 (空状態再利用)
    │       │
    │       └─► Phase 3 (所有権回流)
    │
    ├─► Phase 4 (消費分析)
    │       │
    │       └─► Phase 5 (ref = Arc)
    │               │
    │               └─► Phase 6 (循環検出)
    │
    ├─► Phase 7 (unsafe + 裸ポインタ)
    │
    └─► Phase 8 (Rc/Arc/Weak)
```

---

## ファイルリスト

### 新規ファイル

| ファイル | Phase | 説明 |
|------|-------|------|
| `src/middle/passes/lifetime/empty_state.rs` | P2 | 空状態追踪 |
| `src/middle/passes/lifetime/control_flow.rs` | P2 | 制御フロー解析 |
| `src/middle/passes/lifetime/ownership_flow.rs` | P3 ✅ | 所有権回流推論 |
| `src/middle/passes/lifetime/chain_calls.rs` | P3 ✅ | チェーン呼び出し解析 |
| `src/middle/passes/lifetime/consume_analysis.rs` | P4 ✅ | 消費マークシステム |
| `src/middle/passes/lifetime/lifecycle.rs` | P4 ✅ | 変数ライフサイクル追踪 |
| `src/middle/passes/lifetime/unsafe_check.rs` | P7 | unsafe 検査 |
| `src/middle/passes/lifetime/intra_task_cycle.rs` | P6 ✅ | タスク内循環処理 |
| `src/std/rc.rs` | P8 | Rc/Weak 実装 |
| `src/std/sync.rs` | P8 | Arc 実装 |

### 修正ファイル

| ファイル | Phase | 修正内容 |
|------|-------|----------|
| `src/frontend/core/parser/ast.rs` | P1 | StructField 作成、Type/Pattern 修正 |
| `src/frontend/core/parser/statements/declarations.rs` | P1 | Parser が mut フィールドをサポート |
| `src/frontend/core/parser/pratt/nud.rs` | P1 | 構造体パターン解析が mut をサポート |
| `src/frontend/core/type_system/mono.rs` | P1 | StructType に field_mutability を追加 |
| `src/frontend/typecheck/inference/patterns.rs` | P1 | パターン推論が is_mut をサポート |
| `src/frontend/typecheck/mod.rs` | P1 | StructField に適応 |
| `src/frontend/type_level/auto_derive.rs` | P1 | StructField に適応 |
| `src/frontend/core/type_system/solver.rs` | P1 | field_mutability に適応 |
| `src/frontend/core/type_system/substitute.rs` | P1 | field_mutability に適応 |
| `src/frontend/typecheck/specialization/algorithm.rs` | P1 | field_mutability に適応 |
| `src/frontend/typecheck/specialize.rs` | P1 | field_mutability に適応 |
| `src/frontend/typecheck/overload.rs` | P1 | field_mutability に適応 |
| `src/middle/passes/lifetime/error.rs` | P1 | ImmutableFieldAssign を追加 |
| `src/middle/passes/lifetime/mut_check.rs` | P1 | StoreField 検査拡張 |
| `src/middle/core/ir_gen.rs` | P1 | StructField に適応 |
| `src/middle/passes/codegen/mod.rs` | P1 | StructField に適応 |
| `src/middle/passes/mono/cross_module.rs` | P1 | field_mutability に適応 |
| `src/middle/passes/mono/function.rs` | P1 | StructField に適応 |
| `src/middle/passes/mono/module_state.rs` | P1 | StructField に適応 |
| `src/middle/passes/mono/type_mono.rs` | P1 | field_mutability に適応 |
| `src/middle/passes/lifetime/move_semantics.rs` | P2, P4 ✅ | 空状態検査、消費分析 |
| `src/middle/passes/lifetime/error.rs` | P3 | 回流エラー診断 |
| `src/middle/passes/lifetime/ownership_flow.rs` | P4 | ConsumeMode に Copy を追加 |
| `src/frontend/core/parser/expr.rs` | P5 | ref 式解析 |
| `src/frontend/typecheck/infer.rs` | P5 | ref 型推論 |
| `src/middle/passes/lifetime/ref_semantics.rs` | P5 | ref 所有権処理 |
| `src/middle/passes/lifetime/cycle_check.rs` | P6 ✅ | タスク間循環検出、深度制限、unsafe バイパス |
| `src/middle/passes/lifetime/error.rs` | P6 ✅ | IntraTaskCycle, UnsafeBypassCycle |
| `src/middle/passes/lifetime/mod.rs` | P6 ✅ | IntraTaskCycleTracker を統合 |
| `src/frontend/core/parser/block.rs` | P7 | unsafe 構文解析 |

---

## 時間見積もり

| Phase | 複雑度 | 見積もり工期 |
|-------|--------|----------|
| P1: フィールド不変性 | 中 | 3-4 日 |
| P2: 空状態再利用 | 中 | 2-3 日 |
| P3: 所有権回流 | 低 | 1-2 日 |
| P4: 消費分析 | 中 | 2-3 日 |
| P5: ref = Arc | 低 | 1 日（既存基盤） |
| P6: 循環検出 | 中 | 2 日（既存基盤） |
| P7: unsafe + 裸ポインタ | 中 | 2-3 日 |
| P8: Rc/Arc/Weak | 中 | 3-4 日 |

**合計**: 約 16-22 営業日