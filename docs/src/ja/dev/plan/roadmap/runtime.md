---
title: "ランタイム状態"
---

# ランタイム（Runtime）

> **モジュール状態**：ギャップあり（4項の改善待ち、ステージB未開始）
> **位置**：`src/backends/runtime/`
> **最終更新**：2026-06-01

---

## モジュール概要

ランタイムモジュールはタスクスケジューリングと並行実行を担当します。RFC-008で定義された3層ランタイムアーキテクチャ（Embedded / Standard / Full）を実装しています。

**コード量**：約95KB（4つのソースファイル）

---

## 機能一覧

### engine.rs — DAGスケジューリングコア（実装済み）

- ✅ **タスクライフサイクル管理**：spawn / mark_running / complete / cancel / yield_now
- ✅ **DAG依存グラフ**：ハード依存（hard_deps、失敗/キャンセル伝播）+ コントロール依存（control_deps、順序のみ）
- ✅ **リソース直列化**：ResourceKey機構、同キーを持つタスクは厳密に直列化、キャンセル時はリソース順序を維持するためにコントロール依存を待つ
- ✅ **循環依存検出**：add_dependency時に到達可能性検出で循環を検出し、CycleDetectedエラーを返す
- ✅ **失敗/キャンセル伝播**：失敗はハード依存辺でBFS伝播、複数の依存が同時に失敗した場合はキャンセル理由をマージ（primary + others）
- ✅ **協調式タイムスライシング**：drive_until_polledはTaskPoll::Pendingをサポートして譲渡し、タスク間の公平なラウンドロビンを行う
- ✅ **ターゲット優先スケジューリング**：next_ready_for(target)はターゲットの依存チェーンを先に進行させ、孤立タスクがターゲット完了をブロックしない
- ✅ **統計データ**：RuntimeStats（pending/running/completed/failed/cancelled/total_spawned/avg_execution_time）

### facade.rs — 3層ランタイムファサード（実装済み）

- ✅ **Embedded Runtime**：即時実行、spawn時にクロージャを直ちに実行、DAGなし、deps/resources非対応
- ✅ **Standard Runtime**：単一スレッドDAGスケジューリング、通常のTaskFnと協調式CoopTaskFnをサポート
- ✅ **Full Runtime**：マルチスレッド実行、crossbeamチャネル通信、workerスレッドプール
- ✅ **統一ファサードRuntime**：RuntimeConfig(mode, workers, work_stealing)で設定可能

### task.rs — タスク抽象化（実装済み）

- ✅ **TaskId**：一意の識別子、Displayをサポート
- ✅ **TaskPriority**：Low/Normal/High/Criticalの4段階
- ✅ **TaskState**：Pending/Running/Completed/Failed/Cancelled
- ✅ **TaskConfig**：builderパターンで設定（priority/name/stack_size/parent_id）
- ✅ **Task**：タスクエンティティ、id/config/state/resultを保持
- ✅ **TaskContext**：タスク実行コンテキスト（registers/stack/locals/entry_ip）
- ✅ **Scheduler trait**：抽象スケジューラインターフェース
- ✅ **TaskSpawner**：ジェネリクスによるタスクスケジューララッパー

---

## テストカバレッジ

**約22件のユニットテスト**、コアシナリオをカバー：

| テストファイル | テスト数 | カバーシナリオ |
|----------|--------|----------|
| `engine.rs` | 14 | 線形依存、菱形依存、孤立タスク、ターゲットスケジューリング、失敗伝播、キャンセルリソース直列化、循環検出協調スライシング |
| `facade.rs` | 5 | Standard/Full整合性、並行実行リソース直列化、work-stealingスイッチ協調スライシング |
| `task.rs` | 3 | TaskId、TaskConfig、TaskContext |

---

## RFC比較（RFC-008）

| RFC-008要件 | 実装状態 | 説明 |
|-------------|---------|------|
| 3層アーキテクチャ Embedded/Standard/Full | ✅ 実装済み | facade.rsの3種類のRuntimeInner |
| スケジューラ切り離し（ジェネリクス + 注入） | ⚠️ 部分実装 | task.rsにScheduler traitはあるが、facade.rsは直接enumを使用 |
| 同期 = スケジューリングの特例（num_workers=1） | ✅ 実装済み | Full workers=1テストでStandardとの整合性を検証 |
| DAG遅延評価 | ✅ 実装済み | engine.rsのLocalRuntime |
| ボトムアップ実行モデル | ✅ 実装済み | drive_until / next_ready_forはターゲットの依存チェーンを先に進行 |
| 孤立DAGは独立して並行実行しブロックしない | ✅ 実装済み | 専用テストあり |
| WorkStealer | ⚠️ 宣言はあるが独立実装なし | FullRuntimeはcrossbeamチャネルを使用、真のwork-stealingキューなし |
| コンパイル時DAG解析 | ❌ 未実装（ステージB） | 現在のDAGはランタイムで構築 |
| スケジューラ静的ライブラリ（200-500KB） | ❌ 未実装（ステージB） | LLVM AOTコンパイラの範疇 |
| リフレクションメタデータのオンデマンドロード | ❌ 未実装（ステージB） | 今後の計画 |

---

## 主な発見

1. **task.rsのScheduler traitとfacade.rsの実態スケジューリングは分離している**：task.rsにはScheduler traitが定義されているが、facade.rsはこのtraitを使用せず、直接enumでディスパッチしている
2. **task.rsに重複した型定義がある**：SyncValue、TaskResult、RuntimeError、SchedulerStatsがengine.rsとtask.rsの両方に定義されている
3. **WorkStealingが本当の意味で実装されていない**：RuntimeConfigにはwork_stealingフィールドがあるが、FullRuntimeは実際にはシンプルなスレッドプール + チャネルモデル

---

## コード品質評価

| 次元 | スコア | 説明 |
|------|------|------|
| 未完了事項 | 4 | WorkStealing、重複型、統一スケジューラ、ステージB |
| テストカバレッジ | 良好 | 22件のテストがコアシナリオをカバー |
| ドキュメント品質 | 良好 | モジュールレベルとメソッドレベルのドキュメントが完整 |
| コードアーキテクチャ | 優秀 | 3層アーキテクチャが明確、責務分離が良好 |
| RFC整合性 | 高度に準拠 | ステージAのすべての受入基準にチェック済み |

---

## 改善待ち項目

1. **真のWorkStealingキューを実装する**
2. **task.rs内の重複型定義を解消する**
3. **Scheduler traitとfacade.rsのスケジューリング実装を統一する**
4. **ステージBを開始する：コンパイラ連携**