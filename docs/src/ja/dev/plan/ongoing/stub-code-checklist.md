# 空壳コードチェックリスト

> 生成日付：2026-06-13
> 検査範囲：プロジェクト全体 (`src/`)
> 検査タイプ：`todo!()`、空関数本体、ハードコードされた戻り値、デッドコード

## 統計概要

| タイプ | 数 | 優先度分布 |
|------|------|-----------|
| `todo!()` | 4 箇所 | P0: 4 |
| 空関数本体 | 6 箇所 | P0: 2, P1: 2, P2: 2 |
| ハードコードされた戻り値 | 14 箇所 | P0: 5, P1: 8, P2: 1 |
| デッドコード | 14 箇所 | P2: 14 |
| 重複実装 | 4 箇所 | P2: 4 |
| **合計** | **42 箇所** | |

---

## P0 - 高優先度（コア機能欠如）

### 1. デバッガステップメソッド（4 箇所の `todo!()`）

**ファイル**：`src/backends/interpreter/executor/debug.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 32-34 | `fn step(&mut self) -> ExecutorResult<()>` | 単一命令をステップ実行 | `todo!()` |
| 36-38 | `fn step_over(&mut self) -> ExecutorResult<()>` | ステップオーバー（関数内に入らない） | `todo!()` |
| 40-42 | `fn step_out(&mut self) -> ExecutorResult<()>` | ステップアウト（現在の関数を実行完了） | `todo!()` |
| 44-46 | `fn run(&mut self) -> ExecutorResult<()>` | 次のブレークポイントまで実行継続 | `todo!()` |

**コンテキスト**：`DebuggableExecutor` trait の他のメソッド（`set_breakpoint`、`has_breakpoint`、`current_ip`、`current_function`、`breakpoints`）は実装済みで、ステップ制御のみ未実装。

**実装提案**：
- `step()`：現在の IP 命令を実行、IP++
- `step_over()`：現在の命令が関数呼び出しの場合、次の命令に一時ブレークポイントを設定してから run
- `step_out()`：現在の呼び出しスタックの深さを記録、スタック深度が減少するまで run
- `run()`：ブレークポイントまたはプログラム終了に遭遇するまでループ実行

---

## P1 - 中優先度（機能不完全）

### 6. LSP 進捗通知（1 箇所の空関数本体）

**ファイル**：`src/frontend/events/subscribe.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 357-364 | `fn on_event(&self, _event: &dyn Any, _metadata: &EventMetadata)` | イベントを LSP 通知に変換 | 空実装 |

**コンテキスト**：コメント「ここで LSP 通知ロジックを追加できる。例えば進捗イベントを `$/progress` 通知に変換する」。

**実装提案**：
- イベントタイプを確認（Progress、Diagnostic など）
- Progress イベントの場合、`window/workDoneProgress/create` と `$/progress` 通知を送信
- Diagnostic イベントの場合、`textDocument/publishDiagnostics` 通知を送信

---

### 7. 旧構文スキップ関数（1 箇所の空関数本体）

**ファイル**：`src/frontend/core/parser/statements/declarations.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 171-174 | `fn skip_old_function_syntax(_state: &mut ParserState<'_>)` | 旧関数構文の宣言全体をスキップ | 空実装 |

**コンテキスト**：コメント「旧構文は削除済み、この関数は不要」。

**提案**：呼び出し元があるか確認し、なければ直接削除。

### 11. 境界チェック（2 箇所のハードコードされた戻り値）

**ファイル**：`src/frontend/core/typecheck/inference/bounds.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 70-79 | `pub fn check_const_bounds(&self, _ty: &MonoType, _bounds: &[ConstBound]) -> Result<()>` | const 境界をチェック | `Ok(())` をハードコード |
| 81-90 | `pub fn check_lifetime_bounds(&self, _ty: &MonoType, _bounds: &[LifetimeBound]) -> Result<()>` | ライフタイム境界をチェック | `Ok(())` をハードコード |

**実装提案**：
- `check_const_bounds`：
  - const パラメータが境界制約を満たしているかチェック
  - const 式が評価可能かチェック
- `check_lifetime_bounds`：
  - ライフタイムパラメータが境界制約を満たしているかチェック
  - ライフタイムが制約より長いかチェック

---

### 12. 分割代入チェック（1 箇所のハードコードされた戻り値）

**ファイル**：`src/frontend/core/typecheck/inference/assignment.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 137-146 | `pub fn check_destructuring(&self, _lhs_patterns: &[Pattern], _rhs: &MonoType, _span: Span) -> Result<()>` | 分割代入の形状が一致するかチェック | `Ok(())` をハードコード |

**実装提案**：
- 左側パターンの数が右側の型と一致するかチェック
- 各パターンの型が右側の対応する位置の型と一致するかチェック
- 不一致の具体的な位置を報告

---

### 13. ジェネリック制約の解析（1 箇所のハードコードされた戻り値）

**ファイル**：`src/frontend/core/typecheck/inference/generics.rs`

| 行番号 | 関数シグネチャ | 期待される機能 | 状態 |
|------|----------|----------|------|
| 53-59 | `pub fn infer_generic_constraints(&mut self, _constraints: &[String]) -> Result<()>` | 制約文字列を内部表現に解析 | `Ok(())` をハードコード |

**実装提案**：
- 制約文字列を解析（例：`T: Clone + Debug`）
- 型パラメータと制約を抽出
- 制約を内部表現（TraitConstraint）に変換
- 型環境に追加

---

## P2 - 低優先度（安全に削除可能なデッドコード）

### 28-31. 重複する `substitute_type` 実装

| # | ファイル | 行番号 | シグネチャ | 差異 | 提案 |
|---|------|------|------|------|------|
| 28 | `types/eval/evaluator.rs` | 1040 | `fn substitute_type(body, param_name, replacement)` | TypeRef のみ置換 | 削除（呼び出し元なし） |
| 29 | `types/solver.rs` | 685 | `fn substitute_type(&self, ty, substitution)` | 完全な子ノード置換 | 保持 |
| 31 | `middle/passes/mono/cross_module.rs` | 609 | `fn substitute_type(generic_type, type_args, type_params)` | パラメータリストによる置換 | 保持 |

---

## 合理的な空実装（保持）

| ファイル | 関数 | 理由 |
|------|------|------|
| `frontend/events/mod.rs:131` | `NullEmitter::emit/emit_with` | Null Object Pattern |
| `backends/runtime/facade.rs:306,331` | `EmbeddedRuntime::cancel/drive_until` | 埋め込みランタイムセマンティクス |
| `backends/common/allocator.rs:195` | `BumpAllocator::dealloc` | Bump アロケータ特性 |
| `frontend/core/typecheck/passes/dead_code.rs:190` | `collect_definitions` | 既に deprecated、合理的なスタブ |

---

## 実装進捗追跡

---

## 備考
- P0/P1 は慎重な設計が必要、一つずつ実装してテストを追加することを推奨
- 一部の関数には隠れた呼び出し元がある可能性（trait object やマクロ経由）、削除前に再確認を推奨
- 重複する `substitute_type` は単一実装への統一を推奨