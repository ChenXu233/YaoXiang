```markdown
---
title: "ランタイム状態"
---

# ランタイム（Runtime）

> **モジュール状態**：已完成（段階 A）、段階 B 部分は未開始
> **位置**：`src/backends/runtime/`
> **最終更新**：2026-06-01

---

## モジュール概要

ランタイムモジュールはタスクスケジューリングと並列実行を担当します。RFC-008 で定義された三層ランタイムアーキテクチャを実装しています：Embedded / Standard / Full。

**コード量**：約 95KB（4 つのソースファイル）

---

## 機能一覧

### engine.rs — DAG スケジューリングコア（実装済み）

- ✅ **タスクライフサイクル管理**：spawn / mark_running / complete / cancel / yield_now
- ✅ **DAG 依存グラフ**：ハード依存（hard_deps、障害/キャンセル伝播）+ 制御依存（control_deps、順序のみ、障害伝播なし）
- ✅ **リソース直列化**：ResourceKey メカニズム、同じキーのタスクは厳密に直列化、キャンセル時は制御依存を待ってリソース順序を維持
- ✅ **循環依存検出**：add_dependency 時に到達可能性検査で循環を検出、CycleDetected エラーを返す
- ✅ **障害/キャンセル伝播**：障害はハード依存辺で BFS 伝播、複数の依存が同時に失敗した場合はキャンセル理由をマージ（primary + others）
- ✅ **協調式タイムスライシング**：drive_until_polled が TaskPoll::Pending をサポートし、タスクが公平にローテーション
- ✅ **ターゲット優先スケジューリング**：next_ready_for(target) がターゲット依存チェーンを優先的に進行、孤立タスクはターゲット完了をブロックしない
- ✅ **統計データ**：RuntimeStats（pending/running/completed/failed/cancelled/total_spawned/avg_execution_time）

### facade.rs — 三層ランタイム\Facade（実装済み）

- ✅ **Embedded Runtime**：即時実行、spawn 時に即座にクロージャを実行、DAG なし、deps/resources 非対応
- ✅ **Standard Runtime**：シングルスレッド DAG スケジューリング、通常の TaskFn と協調式 CoopTaskFn をサポート
- ✅ **Full Runtime**：マルチスレッド実行、crossbeam チャネル通信、worker スレッドプール
- ✅ **統一Facade Runtime**：RuntimeConfig(mode, workers, work_stealing) で設定可能

### task.rs — タスク抽象化（実装済み）

- ✅ **TaskId**：一意の識別子、Display をサポート
- ✅ **TaskPriority**：Low/Normal/High/Critical の四段階
- ✅ **TaskState**：Pending/Running/Completed/Failed/Cancelled
- ✅ **TaskConfig**：ビルダーパターンで設定（priority/name/stack_size/parent_id）
- ✅ **Task**：タスクエンティティ、id/config/state/result を保持
- ✅ **TaskContext**：タスク実行コンテキスト（registers/stack/locals/entry_ip）
- ✅ **Scheduler trait**：抽象スケジューラーインターフェース
- ✅ **TaskSpawner**：ジェネリクスタスクスケジューラー封装

---

## テストカバレッジ

**約 22 件のユニットテスト**、コアシナリオをカバー：

| テストファイル | テスト数 | カバーシナリオ |
|----------|--------|----------|
| `engine.rs` | 14 | 線形依存、菱形依存、孤立タスク、ターゲットスケジューリング、障害伝播、キャンセル、リソース直列化、循環検出、協調スライシング |
| `facade.rs` | 5 | Standard/Full 整合性、並列実行、リソース直列化、work-stealing スイッチ、協調スライシング |
| `task.rs` | 3 | TaskId、TaskConfig、TaskContext |

---

## RFC 比較（RFC-008）

| RFC-008 要求事項 | 実装状態 | 説明 |
|-------------|---------|------|
| 三層アーキテクチャ Embedded/Standard/Full | ✅ 実装済み | facade.rs に三種類の RuntimeInner |
| スケジューラー分離（ジェネリクス + 注入） | ⚠️ 部分実装 | task.rs に Scheduler trait があるが、facade.rs は直接 enum を使用 |
| 同期 = スケジューリングの特例（num_workers=1） | ✅ 実装済み | Full workers=1 テストで Standard との整合性を検証 |
| DAG 遅延評価 | ✅ 実装済み | engine.rs の LocalRuntime |
| ボトムアップ実行モデル | ✅ 実装済み | drive_until / next_ready_for がターゲット依存チェーンを優先 |
| 孤立 DAG の独立並列実行がブロックしない | ✅ 実装済み | 専用テストあり |
| WorkStealer | ⚠️ 宣言 поддержка だが実際には独立実装なし | FullRuntime は crossbeam チャネルを使用、本当の work-stealing キューなし |
| コンパイル時 DAG 分析 | ❌ 未実装（段階 B） | 現在の DAG は実行時に構築 |
| スケジューラー静的ライブラリ（200-500KB） | ❌ 未実装（段階 B） | LLVM AOT コンパイラーの範疇 |
| リフレクションメタデータのオンデマンドロード | ❌ 未実装（段階 B） | 今後の計画 |

---

## 主要な発見

1. **task.rs の Scheduler trait と facade.rs の実際のスケジューリングは分離している**：task.rs で Scheduler trait を定義しているが、facade.rs ではこの trait を使用せず、直接 enum でディスパッチしている
2. **task.rs に重複した型定義がある**：SyncValue、TaskResult、RuntimeError、SchedulerStats が engine.rs と task.rs に重複して定義されている
3. **WorkStealing が実際に実装されていない**：RuntimeConfig に work_stealing フィールドがあるが、FullRuntime は実際にはシンプルなスレッドプール + チャネルモデル

---

## コード品質評価

| 評価項目 | スコア | 説明 |
|------|------|------|
| 機能完成度 | 80% | 段階 A は全面的に完了、段階 B は未開始 |
| テストカバレッジ | 良好 | 22 件のテストがコアシナリオをカバー |
| ドキュメント品質 | 良好 | モジュールレベルとメソッドレベルのドキュメントが完整 |
| コードアーキテクチャ | 優秀 | 三層アーキテクチャが清晰、責務分離が良好 |
| RFC 準拠 | 高度に準拠 | 段階 A のすべての受け入れ基準がチェック済み |

---

## 改善待ち項目

1. **本当の WorkStealing キューの実装**
2. **task.rs 内の重複した型定義の消除**
3. **Scheduler trait と facade.rs のスケジューリング実装の統一**
4. **段階 B の開始：コンパイラー接入**
```