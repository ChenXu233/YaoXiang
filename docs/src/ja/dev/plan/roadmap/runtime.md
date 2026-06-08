---
title: "ランタイム状態"
---

# ランタイム（Runtime）

> **モジュール状態**：ギャップあり（4項目の改善待ち、段階B未開始）
> **位置**：`src/backends/runtime/`
> **最終更新**：2026-06-05

---

## モジュール概要

ランタイムモジュールはタスクスケジューリングと並行実行を担当する。RFC-008で定義された三層ランタイムアーキテクチャ（Embedded / Standard / Full）を実装している。

**コード量**：約95KB（4つのソースファイル）

---

## 機能一覧

### engine.rs — DAGスケジューリングコア（実装済み）

- ✅ **タスクライフサイクル管理**：spawn / mark_running / complete / cancel / yield_now
- ✅ **DAG依存グラフ**：ハード依存（hard_deps、障害/キャンセル伝播）+ 制御依存（control_deps、順序のみ、障害伝播なし）
- ✅ **リソース直列化**：ResourceKey機構、同じキーのタスクは厳密に直列化、キャンセル時は制御依存を待ってリソース順序を維持
- ✅ **循環依存検出**：add_dependency時に到達可能性検査で循環を検出、CycleDetectedエラーを返す
- ✅ **障害/キャンセル伝播**：障害はハード依存のエッジに沿ってBFSで伝播、複数の依存先が同時に失敗した場合はキャンセル理由をマージ（primary + others）
- ✅ **協調的タイムスライシング**：drive_until_polledはTaskPoll::Pendingを返し、複数のタスク間で公平にローテーション
- ✅ **ターゲット優先スケジューリング**：next_ready_for(target)はターゲットの依存チェーンを先に進行、孤立タスクはターゲットの完了をブロックしない
- ✅ **統計データ**：RuntimeStats（pending/running/completed/failed/cancelled/total_spawned/avg_execution_time）

### facade.rs — 三層ランタイムファサード（実装済み）

- ✅ **Embedded Runtime**：即時実行、spawn時に即座にクロージャを実行、DAGなし、deps/resources非対応
- ✅ **Standard Runtime**：シングルスレッドDAGスケジューリング、通常のTaskFnと協調的CoopTaskFnをサポート
- ✅ **Full Runtime**：マルチスレッド実行、crossbeamチャネルによる通信、workerスレッドプール
- ✅ **統合ファサードRuntime**：RuntimeConfig(mode, workers, work_stealing)で設定

### task.rs — タスク抽象化（実装済み）

- ✅ **TaskId**：一意の識別子、Displayをサポート
- ✅ **TaskPriority**：Low/Normal/High/Criticalの4段階
- ✅ **TaskState**：Pending/Running/Completed/Failed/Cancelled
- ✅ **TaskConfig**：builderパターンで設定（priority/name/stack_size/parent_id）
- ✅ **Task**：タスクエンティティ、id/config/state/resultを保持
- ✅ **TaskContext**：タスク実行コンテキスト（registers/stack/locals/entry_ip）
- ✅ **Scheduler trait**：抽象スケジューラインターフェース
- ✅ **TaskSpawner**：ジェネリクスタスクスケジューララッパー

---

## テストカバレッジ

**約22個のユニットテスト**、コアシナリオをカバー：

| テストファイル | テスト数 | カバーシナリオ |
|----------|--------|----------|
| `engine.rs` | 14 | 線形依存、菱形依存、孤立タスク、ターゲットスケジューリング、障害伝播、キャンセルリソース直列化、循環検出、協調スライシング |
| `facade.rs` | 5 | Standard/Full整合性、並行実行リソース直列化、work-stealingスイッチ、協調スライシング |
| `task.rs` | 3 | TaskId、TaskConfig、TaskContext |

---

## RFC比較（RFC-008 / RFC-024）

| RFC要件 | 実装状態 | 説明 |
|-------------|---------|------|
| 三層アーキテクチャ Embedded/Standard/Full | ✅ 実装済み | facade.rsの3種類のRuntimeInner |
| スケジューラデカップリング（ジェネリクス + 注入） | ⚠️ 部分実装 | task.rsにScheduler traitはあるが、facade.rsはenumを直接使用 |
| 同期 = スケジューリングの特例（num_workers=1） | ✅ 実装済み | Full workers=1テストでStandardとの整合性を検証 |
| DAG遅延評価 | ✅ 実装済み | engine.rsのLocalRuntime |
| ボトムアップ実行モデル | ✅ 実装済み | drive_until / next_ready_forはターゲットの依存チェーンを優先 |
| 孤立DAGは独立して並行実行しブロックしない | ✅ 実装済み | 専用テストあり |
| WorkStealer | ⚠️ 宣言はされているが実際には独立実装なし | FullRuntimeはcrossbeamチャネルを使用、真のwork-stealingキューなし |
| コンパイル時DAG解析 | ❌ 未実装（RFC-024段階B） | 現在のDAGはランタイム時に構築、RFC-024でコンパイル時への移動を計画 |
| spawnブロックの直接部分式の並行実行 | ❌ 未実装（RFC-024） | 現在spawn整体は単一クロージャとして包装 |
| スケジューラ静的ライブラリ（200-500KB） | ❌ 未実装（段階B） | LLVM AOTコンパイラの範疇 |
| リフレクションメタデータのオンデマンドロード | ❌ 未実装（段階B） | 今後の計画 |

---

## 主な発見

1. **task.rsのScheduler traitとfacade.rsの実質的なスケジューリングは分離している**：task.rsにはScheduler traitが定義されているが、facade.rsはこのtraitを使用せず、enumでディスパッチしている
2. **task.rsに重複した型定義がある**：SyncValue、TaskResult、RuntimeError、SchedulerStatsがengine.rsとtask.rsの両方に定義されている
3. **WorkStealingが実際に実装されていない**：RuntimeConfigにはwork_stealingフィールドがあるが、FullRuntimeは実際のところシンプルなスレッドプール + チャネルモデル
4. **RFC-024はspawnの実行モデルを変える**：現在spawn整体は単一クロージャとして包装されランタイムDAGでスケジューリングされる；RFC-024ではコンパイル時にspawnブロック内の直接部分式の依存関係を解析し、実行計画を生成、ランタイムはその計画に従ってグループ化して並行実行する

---

## コード品質評価

| 次元 | スコア | 説明 |
|------|------|------|
| 未完了項目 | 4 | WorkStealing、重複型、統合スケジューラ、段階B |
| テストカバレッジ | 良好 | 22個のテストがコアシナリオをカバー |
| ドキュメント品質 | 良好 | モジュールレベルとメソッドレベルのドキュメントが完整 |
| コードアーキテクチャ | 優秀 | 三層アーキテクチャが明確、責務分離が良好 |
| RFC準拠 | 高度に準拠 | 段階Aのすべての受け入れ基準がチェック済み |

---

## 改善待项目

1. **真のWorkStealingキューの実装**
2. **task.rs内の重複型定義の消除**
3. **Scheduler traitとfacade.rsのスケジューリング実装の統合**
4. **RFC-024段階B：コンパイラとの連携**
   - 古いモデルのクリーンアップ（`@block`/`@eager`/`@auto`、`EvalMode`、`EvalStrategy`の削除）
   - 新たなコンパイル時DAG解析パスの追加
   - 複数クロージャ + 実行計画をサポートする`Instruction::Spawn`の修正
   - ランタイムでのコンパイル時実行計画に基づくグループ化並行実行