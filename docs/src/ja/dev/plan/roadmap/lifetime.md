---
title: "借用检查器的状态"
---

# 借用検査器（Lifetime）

> **モジュール状態**：安定（4 項目が改善待ち）
> **位置**：`src/middle/passes/lifetime/`
> **最終更新**：2026-06-01

---

## モジュールの概要

借用検査器モジュールは、完全な**所有権分析とライフサイクル管理システム**であり、Move セマンティクス、借用の競合、可変性違反などの所有権関連問題を検査する責務を担っています。

**コード量**：約 300KB のソースコード（15 のサブファイル）

---

## 機能一覧

### コア検査器（OwnershipChecker 統一エントリポイントに統合済み）

| サブモジュール | ファイル | 機能 | 状態 |
|--------|------|------|------|
| **Move セマンティクス** | `move_semantics.rs` (575行) | UseAfterMove 検出、空状態(Empty)再代入をサポート | ✅ 完了 |
| **Drop セマンティクス** | `drop_semantics.rs` (143行) | UseAfterDrop、DropMovedValue、DoubleDrop 検出 | ✅ 完了 |
| **可変性検査** | `mut_check.rs` (395行) | 不変変数の代入、不変オブジェクトの变异メソッド、不変フィールドの代入 | ✅ 完了 |
| **Ref セマンティクス** | `ref_semantics.rs` (145行) | RefNonOwner 検出——ref は有効な所有者にのみ適用可能 | ✅ 完了 |
| **Clone セマンティクス** | `clone.rs` (173行) | CloneMovedValue、CloneDroppedValue 検出 | ✅ 完了 |
| **借用トークン** | `borrow_checker.rs` (503行) | 借用トークン競合検出：MutableBorrowConflict、BorrowAfterMove、UseWhileFrozen | ✅ 完了 |
| **spawn 間サイクル** | `cycle_check.rs` (616行) | タスク間循環参照検出、DFS 環検出 | ✅ 完了 |
| **タスク内サイクル** | `intra_task_cycle.rs` (406行) | タスク内 ref 循環追跡（警告モード） | ✅ 完了 |

### 補助アナライザー

| サブモジュール | ファイル | 機能 | 状態 |
|--------|------|------|------|
| **所有権フロー** | `ownership_flow.rs` (426行) | 関数パラメータが戻り値として返されるかを分析 | ✅ 完了 |
| **消費分析** | `consume_analysis.rs` (363行) | 関数間消費パターンクエリ、キャッシュをサポート | ✅ 完了 |
| **チェーン呼び出し** | `chain_calls.rs` (652行) | メソッドチェーンの所有権フロー分析 | ✅ 完了 |
| **ライフサイクル追跡** | `lifecycle.rs` (1037行) | 変数の完全なライフサイクル追跡 | ✅ 完了 |
| **空状態** | `empty_state.rs` (513行) | Move 後の変数空状態追跡 | ✅ 完了 |
| **制御フロー** | `control_flow.rs` (353行) | 分岐状態マージ分析 | ⚠️ 骨格は完了、コア分析ロジックは空実装 |
| **Unsafe 検査** | `unsafe_check.rs` (113行) | unsafe ブロック外での生ポインタ間接参照をエラー報告 | ✅ 完了 |
| **Send/Sync** | `send_sync.rs` (401行) | 型レベルの Send/Sync 制約検査と制約伝播 | ✅ 完了（独立使用） |

---

## テストカバレッジ

**83 のユニットテスト**、分布は以下の通りです：

| ファイル | テスト数 | カバレッジ状況 |
|------|--------|----------|
| `borrow_checker.rs` | 16 | 最も充実：ユニットテスト+エンドツーエンドテスト |
| `chain_calls.rs` | 13 | 充実：チェーン抽出、消費パターン推定、長チェーン、ミックス呼び出し |
| `consume_analysis.rs` | 11 | 充実：Returns/Consumes パターン、キャッシュ、複数パラメータ |
| `ownership_flow.rs` | 10 | 充実：直接返り値、間接返り値、複数パラメータの部分返り値 |
| `lifecycle.rs` | 10 | 充実：作成/消費/解放追跡、問題検出 |
| `cycle_check.rs` | 8 | 良好：無循環/一方向チェーン/深度制限/unsafe バイパス |
| `intra_task_cycle.rs` | 7 | 良好：無循環/単純循環/自己参照/複数循環 |
| `move_semantics.rs` | 6 | 基本：状態追跡、UseAfterMove |
| `control_flow.rs` | 1 | 不十分：状態マージ関数のみテスト |
| `empty_state.rs` | 1 | 不十分：状態マージのみテスト |
| その他 | 0 | テストなし：drop_semantics, clone, mut_check, ref_semantics, unsafe_check, send_sync |

---

## RFC 比較（RFC-009 所有権モデル）

| RFC 設計要点 | 実装状態 | 説明 |
|-------------|---------|------|
| Move セマンティクス（デフォルト） | ✅ 実装済み | MoveChecker が UseAfterMove を検出 |
| &T/&mut T 借用トークン | ✅ 実装済み | BorrowChecker がトークン競合検出を実装 |
| &T はコピー可能（Dup） | ✅ 実装済み | 複数の &T トークンを同時に保持可能 |
| &mut T は線形 | ✅ 実装済み | 同一ソースの &mut T は1つのみ有効 |
| トークン競合検出（フロースensitive 活性分析） | ✅ 実装済み | 関数本体でトークン状態を追跡 |
| ref キーワード（Rc/Arc 自動選択） | ⚠️ 部分実装 | ref セマンティクス検査器は存在 |
| clone() 明示的ディープコピー | ✅ 実装済み | CloneChecker が移動/解放された値の clone を検出 |
| unsafe + *T | ✅ 実装済み | UnsafeChecker が unsafe ブロック外の生ポインタ操作を検査 |
| タスク内サイクル：silent 許可 | ✅ 実装済み | IntraTaskCycleTracker が警告モードで追跡 |
| タスク間サイクル：lint | ✅ 実装済み | CycleChecker が spawn 間循環参照を検出 |
| ライフサイクル 'a なし | ✅ 設計符合 | ライフサイクルパラメータは実装なし |
| Send/Sync 制約 | ✅ 実装済み | SendSyncChecker は OwnershipChecker から独立 |

---

## コード品質評価

| 次元 | 評価 | 説明 |
|------|------|------|
| 未完了事項 | 3 | テスト補足、control_flow ロジック、ref エスケープ分析 |
| テストカバレッジ | 良好 | 83 のテスト、borrow_checker/chain_calls/consume_analysis のテストが充実 |
| ドキュメント品質 | 良好 | モジュール/構造体/メソッドレベルでドキュメントコメントあり |
| コードアーキテクチャ | 優秀 | OwnershipChecker が統一構成、責務分離が明確 |
| RFC 適合性 | 高度に符合 | RFC-009 v9 設計に高度に符合 |

---

## 改善待ち項目

1. **5 つのサブモジュールのユニットテスト補足**：drop_semantics, clone, mut_check, ref_semantics, unsafe_check
2. **control_flow 分析器のコアロジック実装**（現在は空の骨格）
3. **ref が Rc/Arc を自動選択するエスケープ分析の改善**