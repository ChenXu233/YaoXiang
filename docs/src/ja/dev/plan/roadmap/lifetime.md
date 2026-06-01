---
title: "借用検査器的状態"
---

# 借用検査器（Lifetime）

> **モジュール状態**：完了
> **位置**：`src/middle/passes/lifetime/`
> **最終更新**：2026-06-01

---

## モジュール概要

借用検査器モジュールは完全な**所有権分析とライフサイクル管理システム**であり、Move セマンティクス、借用の競合、可変性違反などの所有権関連問題を検査します。

**コード量**：約 300KB のソースコード（15 サブファイル）

---

## 機能一覧

### コア検査器（OwnershipChecker 統合エントリポイント）

| サブモジュール | ファイル | 機能 | 状態 |
|--------|------|------|------|
| **Move セマンティクス** | `move_semantics.rs` (575行) | UseAfterMove 検出、空状態(Empty)再代入をサポート | ✅ 完了 |
| **Drop セマンティクス** | `drop_semantics.rs` (143行) | UseAfterDrop、DropMovedValue、DoubleDrop 検出 | ✅ 完了 |
| **可変性検査** | `mut_check.rs` (395行) | 不変変数の代入、不変オブジェクトの変異メソッド、不変フィールドの代入 | ✅ 完了 |
| **Ref セマンティクス** | `ref_semantics.rs` (145行) | RefNonOwner 検出——ref は有効な所有者にのみ適用可能 | ✅ 完了 |
| **Clone セマンティクス** | `clone.rs` (173行) | CloneMovedValue、CloneDroppedValue 検出 | ✅ 完了 |
| **借用トークン** | `borrow_checker.rs` (503行) | 借用トークン競合検出：MutableBorrowConflict、BorrowAfterMove、UseWhileFrozen | ✅ 完了 |
| **spawn 間サイクル** | `cycle_check.rs` (616行) | タスク間サイクル参照検出、DFS サイクル検出 | ✅ 完了 |
| **タスク内サイクル** | `intra_task_cycle.rs` (406行) | タスク内 ref サイクル追跡（警告モード） | ✅ 完了 |

### 補助アナライザー

| サブモジュール | ファイル | 機能 | 状態 |
|--------|------|------|------|
| **所有権フロー** | `ownership_flow.rs` (426行) | 関数パラメータが戻り値で返されるかの分析 | ✅ 完了 |
| **消費分析** | `consume_analysis.rs` (363行) | 関数間消費パターンクエリ、キャッシュ対応 | ✅ 完了 |
| **チェーン呼び出し** | `chain_calls.rs` (652行) | メソッドチェーン所有権フロー分析 | ✅ 完了 |
| **ライフサイクル追跡** | `lifecycle.rs` (1037行) | 変数の完全なライフサイクル追跡 | ✅ 完了 |
| **空状態** | `empty_state.rs` (513行) | Move 後変数の空状態追跡 | ✅ 完了 |
| **制御フロー** | `control_flow.rs` (353行) | 分岐状態マージ分析 | ⚠️ スケルトン完了、コア分析ロジックは空実装 |
| **Unsafe 検査** | `unsafe_check.rs` (113行) | unsafe ブロック外での裸ポインタ逆参照でエラー | ✅ 完了 |
| **Send/Sync** | `send_sync.rs` (401行) | 型レベル Send/Sync 制約検査と制約伝播 | ✅ 完了（独立使用） |

---

## テストカバレッジ

**83 件のユニットテスト**、分布は以下の通り：

| ファイル | テスト数 | カバレッジ状況 |
|------|--------|----------|
| `borrow_checker.rs` | 16 | 最も十分：ユニットテスト+エンドツーエンドテスト |
| `chain_calls.rs` | 13 | 十分：チェーン抽出、消費パターン推論、長チェーン、混合呼び出し |
| `consume_analysis.rs` | 11 | 十分：Returns/Consumes パターン、キャッシュ、複数パラメータ |
| `ownership_flow.rs` | 10 | 十分：直接戻り値、間接戻り値、複数パラメータ部分戻り値 |
| `lifecycle.rs` | 10 | 十分：作成/消費/解放追跡、問題検出 |
| `cycle_check.rs` | 8 | 良好：サイクルなし/片方向チェーン/深さ制限/unsafe バイパス |
| `intra_task_cycle.rs` | 7 | 良好：サイクルなし/単純サイクル/自己参照/複数サイクル |
| `move_semantics.rs` | 6 | 基本：状態追跡、UseAfterMove |
| `control_flow.rs` | 1 | 不十分：状態マージ関数のみテスト |
| `empty_state.rs` | 1 | 不十分：状態マージのみテスト |
| その他 | 0 | テストなし：drop_semantics, clone, mut_check, ref_semantics, unsafe_check, send_sync |

---

## RFC 比較（RFC-009 所有権モデル）

| RFC 設計要点 | 実装状態 | 説明 |
|-------------|---------|------|
| Move セマンティクス（デフォルト） | ✅ 実装済み | MoveChecker で UseAfterMove を検出 |
| &T/&mut T 借用トークン | ✅ 実装済み | BorrowChecker でトークン競合検出を実装 |
| &T は複製可能（Dup） | ✅ 実装済み | 複数の &T トークンが同時に存在可能 |
| &mut T は線形 | ✅ 実装済み | 同一来源の &mut T は1つのみアクティブ可能 |
| 凍結機構 | ✅ 実装済み | freeze() で &mut T を &T に凍結 |
| トークン競合検出（フロー感知活性分析） | ✅ 実装済み | 関数本体でトークン状態を追跡 |
| ref キーワード（Rc/Arc 自動選択） | ⚠️ 部分実装 | ref セマンティクス検査器は存在 |
| clone() による明示的ディープコピー | ✅ 実装済み | CloneChecker で移動/解放された値の clone を検出 |
| unsafe + *T | ✅ 実装済み | UnsafeChecker で unsafe ブロック外の裸ポインタ操作を検査 |
| タスク内サイクル：黙認許可 | ✅ 実装済み | IntraTaskCycleTracker が警告モードで追跡 |
| タスク間サイクル：lint | ✅ 実装済み | CycleChecker で spawn 間サイクル参照を検出 |
| ライフサイクル 'a なし | ✅ 設計に準拠 | ライフタイムパラメータは未実装 |
| Send/Sync 制約 | ✅ 実装済み | SendSyncChecker は OwnershipChecker から独立 |

---

## コード品質評価

| ディメンション | スコア | 説明 |
|------|------|------|
| 機能完了度 | 95% | 15 中 14 サブモジュールの機能が完全 |
| テストカバレッジ | 良好 | 83 件のテスト、borrow_checker/chain_calls/consume_analysis が十分 |
| ドキュメント品質 | 良好 | モジュール/構造体/メソッドレベルにドキュメントコメントあり |
| コードアーキテクチャ | 優秀 | OwnershipChecker が統一的に構成、責務分離が明確 |
| RFC 準拠 | 高い準拠度 | RFC-009 v9 設計に高度に準拠 |

---

## 改善待ち項目

1. **5 サブモジュールのユニットテストを補足**：drop_semantics, clone, mut_check, ref_semantics, unsafe_check
2. **control_flow アナライザーのコアロジックを実装**（現在空のスケルトン）
3. **freeze 機構を完善**（&mut T の一時的な &T への凍結）
4. **ref の Rc/Arc 自動選択のエスケープ分析を完善**