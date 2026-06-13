---
title: "ランタイム状態"
---

# ランタイム（Runtime）

> **モジュール状態**：ギャップあり（未改善項目 4 件、フェーズ B 未着手）
> **場所**：`src/backends/runtime/`
> **最終更新日**：2026-06-05

---

## モジュール概要

ランタイムモジュールはタスクスケジューリングと並列実行を担当する。RFC-008 で定義された 3 層ランタイムアーキテクチャ（Embedded / Standard / Full）を実装している。

**コード量**：約 95KB（ソースファイル 4 個）

---

## 機能一覧

### engine.rs — DAG スケジューリングコア（実装済み）

- ✅ **タスクライフサイクル管理**：spawn / mark_running / complete / cancel / yield_now
- ✅ **DAG 依存グラフ**：ハード依存（hard_deps、失敗/キャンセルを伝播）+ 制御依存（control_deps、順序付けのみで失敗は伝播しない）
- ✅ **リソース直列化**：ResourceKey 機構により、同ーキーのタスクは厳密に直列化され、キャンセル時は制御依存を待機してリソース順序を維持
- ✅ **循環依存検出**：add_dependency 時に到達可能性で循環を検出し、CycleDetected エラーを返す
- ✅ **失敗/キャンセル伝播**：失敗はハード依存エッジに沿って BFS で伝播、複数依存が同時失敗した場合はキャンセル理由をマージ（primary + others）
- ✅ **協調的時分割**：drive_until_polled が TaskPoll::Pending による譲渡をサポートし、複数タスク間で公平にラウンドロビン
- ✅ **目標優先スケジューリング**：next_ready_for(target) が目標依存チェーンを優先的に推進、孤立タスクは目標完了をブロックしない
- ✅ **統計データ**：RuntimeStats（pending/running/completed/failed/cancelled/total_spawned/avg_execution_time）

### facade.rs — 3 層ランタイムファサード（実装済み）

- ✅ **Embedded Runtime**：即時実行、spawn 時にクロージャを即座に実行、DAG なし、deps/resources 非対応
- ✅ **Standard Runtime**：シングルスレッド DAG スケジューリング、通常の TaskFn と協調的 CoopTaskFn に対応
- ✅ **Full Runtime**：マルチスレッド実行、crossbeam channel による通信、ワーカースレッドプール
- ✅ **統一ファサード Runtime**：RuntimeConfig(mode, workers, work_stealing) で設定

### task.rs — タスク抽象（実装済み）

- ✅ **TaskId**：一意識別子、Display 対応
- ✅ **TaskPriority**：Low/Normal/High/Critical の 4 レベル
- ✅ **TaskState**：Pending/Running/Completed/Failed/Cancelled
- ✅ **TaskConfig**：ビルダーパターンによる設定（priority/name/stack_size/parent_id）
- ✅ **Task**：タスクエンティティ、id/config/state/result を保持
- ✅ **TaskContext**：タスク実行コンテキスト（registers/stack/locals/entry_ip）
- ✅ **Scheduler trait**：抽象スケジューラインターフェース
- ✅ **TaskSpawner**：ジェネリクスタスクスケジューラのカプセル化

---

## テストカバレッジ

**単体テスト約 22 個**がコアシナリオをカバー：

| テストファイル | テスト数 | カバレッジシナリオ |
|----------|--------|----------|
| `engine.rs` | 14 | 線形依存、ダイヤモンド依存、孤立タスク、目標スケジューリング、失敗伝播、キャンセル、リソース直列化、循環検出、協調的スライス |
| `facade.rs` | 5 | Standard/Full 一貫性、並列実行、リソース直列化、work stealing オン/オフ、協調的スライス |
| `task.rs` | 3 | TaskId、TaskConfig、TaskContext |

---

## RFC 比較（RFC-008 / RFC-024）

| RFC 要件 | 実装状態 | 説明 |
|-------------|---------|------|
| 3 層アーキテクチャ Embedded/Standard/Full | ✅ 実装済み | facade.rs の 3 種類の RuntimeInner |
| スケジューラの疎結合（ジェネリクス + 注入） | ⚠️ 部分実装 | task.rs に Scheduler trait あり、ただし facade.rs は enum を直接使用 |
| 同期 = スケジューリングの特殊ケース（num_workers=1） | ✅ 実装済み | Full workers=1 のテストで Standard との一貫性を検証 |
| DAG 遅延評価 | ✅ 実装済み | engine.rs の LocalRuntime |
| ボトムアップ実行モデル | ✅ 実装済み | drive_until / next_ready_for が目標依存チェーンを優先 |
| 孤立 DAG の独立並列・非ブロック | ✅ 実装済み | 専用テストあり |
| WorkStealer | ⚠️ サポートを宣言しているが独立実装は未着手 | FullRuntime は crossbeam channel 使用、真の work stealing キューはなし |
| コンパイル時 DAG 分析 | ❌ 未実装（RFC-024 フェーズ B） | 現在の DAG はランタイム構築、RFC-024 ではコンパイル時移行を計画 |
| spawn ブロックの直接サブ式の並列実行 | ❌ 未実装（RFC-024） | 現状の spawn は全体を単一クロージャとしてパッケージ |
| スケジューラ静的ライブラリ（200-500KB） | ❌ 未実装（フェーズ B） | LLVM AOT コンパイラの范畴 |
| リフレクションメタデータのオンデマンド読み込み | ❌ 未実装（フェーズ B） | 今後の計画范畴 |

---

## 主な発見事項

1. **task.rs の Scheduler trait と facade.rs の実際のスケジューリングが分離している**：task.rs は Scheduler trait を定義しているが、facade.rs はこの trait を使用せず、enum で直接ディスパッチしている
2. **task.rs に重複する型定義がある**：SyncValue、TaskResult、RuntimeError、SchedulerStats が engine.rs と task.rs にそれぞれ定義されている
3. **WorkStealing は実際には実装されていない**：RuntimeConfig に work_stealing フィールドはあるが、FullRuntime は実際にはシンプルなスレッドプール + channel モデル
4. **RFC-024 は spawn の実行モデルを変更する**：現状の spawn は全体を単一クロージャにパッケージしランタイム DAG でスケジューリングしているが、RFC-024 ではコンパイル時に spawn ブロック内の直接サブ式の依存関係を分析して実行計画を生成し、ランタイムで計画に従ってグループ化して並列実行する予定

---

## コード品質評価

| 観点 | 評価 | 説明 |
|------|------|------|
| 未完了項目 | 4 | WorkStealing、重複型、スケジューラ統一、フェーズ B |
| テストカバレッジ | 良好 | 22 個のテストがコアシナリオをカバー |
| ドキュメント品質 | 良好 | モジュールレベルとメソッドレベルのドキュメントが完備 |
| コードアーキテクチャ | 優秀 | 3 層アーキテクチャが明確、責務分離が良好 |
| RFC コンプライアンス | 高い適合度 | フェーズ A のすべての受け入れ基準がチェック済み |

---

## 改善項目

1. **真の Work Stealing キューの実装**
2. **task.rs の重複型定義の除去**
3. **Scheduler trait と facade.rs のスケジューリング実装の統一**
4. **RFC-024 フェーズ B：コンパイラ統合**
   - ~~旧モデルのクリーンアップ~~ ✅ 完了（`@block`/`@eager`/`@auto`、`Send`/`Sync` は削除済み）
   - コンパイル時 DAG 分析パスの新規追加
   - `Instruction::Spawn` を修正し、複数クロージャ + 実行計画をサポート
   - ランタイムはコンパイル時の実行計画に従いグループ化して並列実行