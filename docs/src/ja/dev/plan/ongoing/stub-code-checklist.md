# 空実装コードチェックリスト

> 生成日：2026-06-13
> 検査範囲：プロジェクト全体（`src/`）
> 検査種別：`todo!()`、空の関数本体、ハードコードされた返り値、デッドコード

## 統計概要

| 種別 | 件数 | 優先度分布 |
|------|------|-----------|
| `todo!()` | 4 件 | P0: 4 |
| 空の関数本体 | 6 件 | P0: 2, P1: 2, P2: 2 |
| ハードコードされた返り値 | 14 件 | P0: 5, P1: 8, P2: 1 |
| デッドコード | 14 件 | P2: 14 |
| 重複実装 | 4 件 | P2: 4 |
| **合計** | **42 件** | |

---

## P0 - 高優先度（コア機能欠如）

### 1. デバッガのステップメソッド（4 件の `todo!()`）

**ファイル**：`src/backends/interpreter/executor/debug.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 32-34 | `fn step(&mut self) -> ExecutorResult<()>` | 1 命令だけステップ実行する | `todo!()` |
| 36-38 | `fn step_over(&mut self) -> ExecutorResult<()>` | ステップオーバー（関数内部には入らない） | `todo!()` |
| 40-42 | `fn step_out(&mut self) -> ExecutorResult<()>` | ステップアウト（現在の関数の実行を完了させる） | `todo!()` |
| 44-46 | `fn run(&mut self) -> ExecutorResult<()>` | 次のブレークポイントまで実行を続行する | `todo!()` |

**コンテキスト**：`DebuggableExecutor` trait の他のメソッド（`set_breakpoint`、`has_breakpoint`、`current_ip`、`current_function`、`breakpoints`）は実装済みで、ステップ制御のみが未実装。

**実装のヒント**：
- `step()`：現在の IP の命令を実行し、IP をインクリメント
- `step_over()`：現在の命令が関数呼び出しの場合、次の命令に一時ブレークポイントを設定してから run
- `step_out()`：現在の呼び出しスタックの深さを記録し、深さが減少するまで run
- `run()`：ブレークポイントまたはプログラム終了に達するまでループ実行

---

### 2. 制御フロー解析のコア（2 件の空関数本体）

**ファイル**：`src/middle/passes/lifetime/control_flow.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 145-154 | `fn analyze_instruction(&self, _instr: &Instruction, _state: &mut HashMap<Operand, ValueState>, _pos: (usize, usize))` | 単一命令のライフタイム状態変化を解析 | 空実装 |
| 155-163 | `fn merge_block_state(&mut self, _block_state: &HashMap<Operand, ValueState>, _block_idx: usize)` | 異なる基本ブロックからの ValueState をマージ | 空実装 |

**コンテキスト**：コメントには「現状は空実装で、必要に応じて今後拡張可能」「制御フロー解析は MoveChecker にすでに基本実装あり」とある。

**実装のヒント**：
- `analyze_instruction`：
  - 命令種別（Move、Copy、Call、Branch など）でパターンマッチ
  - `_state` 内の対応する Operand の ValueState（Moved/Empty/Partial）を更新
  - 関数呼び出し引数の Move を処理
- `merge_block_state`：
  - 複数の先行ブロックが合流する場合、ValueState の最小上界（LUB）を取る
  - いずれかの先行が Moved の場合、合流後は Moved
  - 先行状態が衝突する場合、エラーを報告するか MaybeMoved としてマーク

**影響**：現在の lifetime pass は no-op に退化しており、use-after-move エラーを検出できない。

---

### 3. Trait のオブジェクト安全性チェック（2 件のハードコードされた返り値）

**ファイル**：`src/frontend/core/typecheck/traits/object_safety.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 62-71 | `fn check_associated_types(&self, _trait_name: &str) -> Result<(), ObjectSafetyError>` | 関連型がオブジェクト安全かチェック | `Ok(())` をハードコード |
| 74-85 | `fn check_method_signatures(&self, _trait_name: &str) -> Result<(), ObjectSafetyError>` | メソッドシグネチャがオブジェクト安全かチェック | `Ok(())` をハードコード |

**コンテキスト**：コメントには「簡略実装：関連型を持たない、またはすべて安全と仮定」「基本特質のメソッドはすべてオブジェクト安全と仮定」とある。

**実装のヒント**：
- `check_associated_types`：
  - trait のすべての関連型を取得
  - 関連型に `Self` 制約があるかをチェック（安全でない）
  - 関連型がメソッドシグネチャ内で使用されているかをチェック（安全でない）
- `check_method_signatures`：
  - メソッドの返り型に `Self` が含まれているかをチェック（安全でない）
  - メソッドに generics パラメータがあるかをチェック（安全でない）
  - メソッドが `where Self: Sized` 制約を使用しているかをチェック（安全だが、特別な処理が必要）

---

### 4. Trait の一貫性チェック（3 件のハードコードされた返り値）

**ファイル**：`src/frontend/core/typecheck/traits/coherence.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 38-43 | `fn check_conflicting_implementations(&self) -> Result<()>` | 衝突する trait 実装があるかをチェック | `Ok(())` をハードコード |
| 46-50 | `fn check_orphan_rule(&self) -> Result<()>` | オーファンルールをチェック | `Ok(())` をハードコード |
| 80-86 | `fn find_orphan_implementations(&self) -> Result<()>` | すべての trait 実装を走査してチェック | `Ok(())` をハードコード |

**コンテキスト**：コメントには「簡略実装：重複する特質実装の有無をチェック」「特質実装がオーファンルールに準拠していることを確認」とある。

**実装のヒント**：
- `check_conflicting_implementations`：
  - すべての trait 実装を収集
  - 同一型に対する複数の実装について、重複があるかをチェック
  - 衝突する実装を報告
- `check_orphan_rule`：
  - 各 trait impl について、trait または型が現在の crate で定義されているかをチェック
  - どちらも定義されていない場合、オーファンルール違反を報告
- `find_orphan_implementations`：
  - すべてのモジュールの trait impl を走査
  - 各実装について `check_orphan_rule` を呼び出してチェック

---

### 5. Trait 実装のシグネチャチェック（1 件のハードコードされた返り値）

**ファイル**：`src/frontend/core/typecheck/traits/impl_check.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 95-103 | `fn check_signature(&self, _trait_def: &TraitDef, _params: &[Param]) -> Result<()>` | impl メソッドのシグネチャが trait 定義と一致するかをチェック | `Ok(())` をハードコード |

**コンテキスト**：現状はメソッド名の有無のみチェックしており、シグネチャチェックは空。

**実装のヒント**：
- 引数の型（generics パラメータを含む）を比較
- 返り型を比較
- `mut` 修飾子を比較
- ライフタイムパラメータを比較
- 一致しない具体的な位置を報告

---

## P1 - 中優先度（機能不完全）

### 6. LSP 進捗通知（1 件の空関数本体）

**ファイル**：`src/frontend/events/subscribe.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 357-364 | `fn on_event(&self, _event: &dyn Any, _metadata: &EventMetadata)` | イベントを LSP 通知に変換 | 空実装 |

**コンテキスト**：コメントには「ここで LSP 通知ロジックを追加可能（例：進捗イベントを `$/progress` 通知に変換）」とある。

**実装のヒント**：
- イベント種別（Progress、Diagnostic など）をチェック
- Progress イベントの場合、`window/workDoneProgress/create` および `$/progress` 通知を送信
- Diagnostic イベントの場合、`textDocument/publishDiagnostics` 通知を送信

---

### 7. 旧構文スキップ関数（1 件の空関数本体）

**ファイル**：`src/frontend/core/parser/statements/declarations.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 171-174 | `fn skip_old_function_syntax(_state: &mut ParserState<'_>)` | 旧関数構文の宣言全体をスキップ | 空実装 |

**コンテキスト**：コメントには「旧構文はすでに削除されており、本関数は不要」とある。

**推奨**：呼び出し元があるかをチェックし、なければ直接削除。

---

### 8. GAT チェック（2 件のハードコードされた返り値）

**ファイル**：`src/frontend/core/typecheck/traits/gat/checker.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 122-131 | `fn validate_generic_usage(&self, _ty: &MonoType) -> Result<()>` | generics パラメータの使用が正当かを検証 | `Ok(())` をハードコード |
| 174-184 | `pub fn check_associated_type_constraints(...)` | 関連型の制約をチェック | `Ok(())` をハードコード |

**実装のヒント**：
- `validate_generic_usage`：
  - generics パラメータが許可された位置で使用されているかをチェック
  - 未使用の generics パラメータがあるかをチェック
  - 制約に違反する使用があるかをチェック
- `check_associated_type_constraints`：
  - 関連型が制約を満たすかをチェック
  - 制約が充足可能かをチェック

---

### 9. 高階型のライフタイムチェック（1 件のハードコードされた返り値）

**ファイル**：`src/frontend/core/typecheck/traits/gat/higher_rank.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 100-109 | `fn check_lifetime_constraints(&self, _ty: &MonoType) -> Result<()>` | ライフタイムパラメータの使用をチェック | `Ok(())` をハードコード |

**実装のヒント**：
- ライフタイムパラメータが許可された位置で使用されているかをチェック
- ライフタイムパラメータが制約を満たすかをチェック
- 高階ライフタイムルールに違反する使用があるかをチェック

---

### 10. 制約伝播（1 件のハードコードされた返り値）

**ファイル**：`src/frontend/core/typecheck/traits/solver.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 311-318 | `pub fn propagate_constraints_to_type_args(&self, _ty: &MonoType, _trait_name: &str) -> Vec<TraitConstraint>` | 型引数からサブ制約を抽出して伝播 | `Vec::new()` をハードコード |

**実装のヒント**：
- 型の generics 引数を取得
- 各 generics 引数について、その制約をチェック
- 制約を具体的な型引数に伝播
- 伝播後の制約リストを返す

---

### 11. 境界チェック（2 件のハードコードされた返り値）

**ファイル**：`src/frontend/core/typecheck/inference/bounds.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 70-79 | `pub fn check_const_bounds(&self, _ty: &MonoType, _bounds: &[ConstBound]) -> Result<()>` | const 境界をチェック | `Ok(())` をハードコード |
| 81-90 | `pub fn check_lifetime_bounds(&self, _ty: &MonoType, _bounds: &[LifetimeBound]) -> Result<()>` | ライフタイム境界をチェック | `Ok(())` をハードコード |

**実装のヒント**：
- `check_const_bounds`：
  - const パラメータが境界制約を満たすかをチェック
  - const 式が評価可能かをチェック
- `check_lifetime_bounds`：
  - ライフタイムパラメータが境界制約を満たすかをチェック
  - ライフタイムが制約より長いかをチェック

---

### 12. 分解代入チェック（1 件のハードコードされた返り値）

**ファイル**：`src/frontend/core/typecheck/inference/assignment.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 137-146 | `pub fn check_destructuring(&self, _lhs_patterns: &[Pattern], _rhs: &MonoType, _span: Span) -> Result<()>` | 分解代入の形状が一致するかをチェック | `Ok(())` をハードコード |

**実装のヒント**：
- 左辺パターンの個数が右辺の型と一致するかをチェック
- 各パターンの型が右辺の対応する位置の型と一致するかをチェック
- 一致しない具体的な位置を報告

---

### 13. ジェネリック制約の解析（1 件のハードコードされた返り値）

**ファイル**：`src/frontend/core/typecheck/inference/generics.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 53-59 | `pub fn infer_generic_constraints(&mut self, _constraints: &[String]) -> Result<()>` | 制約文字列から内部表現へ解析 | `Ok(())` をハードコード |

**実装のヒント**：
- 制約文字列（例：`T: Clone + Debug`）を解析
- 型パラメータと制約を抽出
- 制約を内部表現（TraitConstraint）に変換
- 型環境へ追加

---

## P2 - 低優先度（安全に削除可能なデッドコード）

### 14-27. デッドコード一覧

| # | ファイル | 行番号 | 要素 | 種別 | 推奨 |
|---|------|------|------|------|------|
| 14 | `frontend/pipeline.rs` | 907-932 | `impl TypecheckResult` ブロック | デッドコード | 削除 |
| 15 | `frontend/pipeline.rs` | 960 | `failed_proofs` フィールド | デッドコード | 削除 |
| 16 | `parser/statements/declarations.rs` | 103-112 | `fn_returns_meta_type` | デッドコード | 削除 |
| 17 | `parser/statements/declarations.rs` | 114-132 | `generic_params_from_constructor_params` | デッドコード | 削除 |
| 18 | `parser/statements/types.rs` | 41-63 | `looks_like_parenthesized_lambda` | デッドコード | 削除 |
| 19 | `types/eval/evaluator.rs` | 1039-1093 | `substitute_type` | デッドコード | 削除 |
| 20 | `types/eval/evaluator.rs` | 1117-1125 | `integrate_evaluator` | デッドコード | 削除 |
| 21 | `types/eval/evaluator.rs` | 1131-1154 | `sync_caches` | デッドコード | 削除 |
| 22 | `module/cache.rs` | 35-36 | `cached_at` フィールド | デッドコード | 削除 |
| 23 | `util/diagnostic/session.rs` | 13-14 | `cache` フィールド | デッドコード | 削除 |
| 24 | `middle/passes/lifetime/cycle_check.rs` | 22-23 | `MAX_DETECTION_DEPTH` 定数 | デッドコード | 削除 |
| 25 | `middle/passes/lifetime/intra_task_cycle.rs` | 26-27 | `value_defs` フィールド | デッドコード | 削除 |
| 26 | `typecheck/proof/budget.rs` | 59-65 | `record_time_ms` + `time_ms_used` | デッドコード | 削除 |
| 27 | `typecheck/layers/termination.rs` | 854-938 | 3 つの関数 | デッドコード | 削除 |
| 28 | `typecheck/checker.rs` | 1661-1677 | `check_refined_binding` | デッドコード | 削除 |
| 29 | `typecheck/layers/ownership.rs` | 9-12 | ファイル全体 | デッドコード | 削除 |
| 30 | `util/diagnostic/emitter/text.rs` | 259-262 | `hint_prefix` | デッドコード | 削除 |

---

### 28-31. 重複する `substitute_type` 実装

| # | ファイル | 行番号 | シグネチャ | 差異 | 推奨 |
|---|------|------|------|------|------|
| 28 | `types/eval/evaluator.rs` | 1040 | `fn substitute_type(body, param_name, replacement)` | TypeRef のみ置換 | 削除（呼び出し元なし） |
| 29 | `types/solver.rs` | 685 | `fn substitute_type(&self, ty, substitution)` | 完全な子ノード置換 | 保持 |
| 30 | `types/traits/specialization/algorithm.rs` | 66 | `fn substitute_type(&self, ty)` | 完全な子ノード置換 | 保持 |
| 31 | `middle/passes/mono/cross_module.rs` | 609 | `fn substitute_type(generic_type, type_args, type_params)` | 引数リストによる置換 | 保持 |

---

## 妥当な空実装（保持）

| ファイル | 関数 | 理由 |
|------|------|------|
| `frontend/events/mod.rs:131` | `NullEmitter::emit/emit_with` | Null Object Pattern |
| `backends/runtime/facade.rs:306,331` | `EmbeddedRuntime::cancel/drive_until` | 組込みランタイムのセマンティクス |
| `backends/common/allocator.rs:195` | `BumpAllocator::dealloc` | Bump アロケータの特性 |
| `frontend/core/typecheck/passes/dead_code.rs:190` | `collect_definitions` | すでに deprecated、合理的なスタブ |

---

## 実装進捗の追跡

| 優先度 | 総数 | 完了 | 残り |
|--------|------|--------|------|
| P0 | 12 | 0 | 12 |
| P1 | 11 | 0 | 11 |
| P2 | 19 | 0 | 19 |
| **合計** | **42** | **0** | **42** |

---

## 備考

- P2 のデッドコードは機能に影響せず安全に削除可能
- P0/P1 は慎重な設計が必要。1 件ずつ実装し、テストを追加することを推奨
- 一部の関数には（trait オブジェクトやマクロを介した）隠れた呼び出し元が存在する可能性があるため、削除前に再度確認することを推奨
- 重複する `substitute_type` は単一実装への統合を推奨