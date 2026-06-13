---
title: "借用チェッカー状態"
---

# 借用チェッカー（ライフタイム）

> **モジュール状態**：安定（4 項目要改善）
> **位置**：`src/middle/passes/lifetime/`
> **最終更新**：2026-06-01

---

## モジュール概要

借用チェッカーモジュールは、完全な**所有権解析とライフタイム管理システム**であり、Move セマンティクス、借用衝突、可変性違反など、所有権に関する問題を検査する。

**コード量**：約 300KB ソースコード（15 サブファイル）

---

## 機能一覧

### 中核チェッカー（OwnershipChecker 統合エントリに組み込み済み）

| サブモジュール | ファイル | 機能 | 状態 |
|--------|------|------|------|
| **Move セマンティクス** | `move_semantics.rs` (575行) | UseAfterMove 検出、空状態(Empty)への再代入をサポート | ✅ 完了 |
| **Drop セマンティクス** | `drop_semantics.rs` (143行) | UseAfterDrop、DropMovedValue、DoubleDrop 検出 | ✅ 完了 |
| **可変性検査** | `mut_check.rs` (395行) | 不変変数への代入、不変オブジェクトの可変メソッド呼び出し、不変フィールドへの代入 | ✅ 完了 |
| **Ref セマンティクス** | `ref_semantics.rs` (145行) | RefNonOwner 検出——ref は有効な所有者にのみ適用可能 | ✅ 完了 |
| **Clone セマンティクス** | `clone.rs` (173行) | CloneMovedValue、CloneDroppedValue 検出 | ✅ 完了 |
| **借用トークン** | `borrow_checker.rs` (503行) | 借用トークン衝突検出：MutableBorrowConflict、BorrowAfterMove、UseWhileFrozen | ✅ 完了 |
| **spawn 跨ぎループ** | `cycle_check.rs` (616行) | タスク跨ぎの参照ループ検出、DFS 環検出 | ✅ 完了 |
| **タスク内ループ** | `intra_task_cycle.rs` (406行) | タスク内 ref ループ追跡（警告モード） | ✅ 完了 |

### 補助解析器

| サブモジュール | ファイル | 機能 | 状態 |
|--------|------|------|------|
| **所有権の逆流** | `ownership_flow.rs` (426行) | 関数の引数が戻り値で返されるかを解析 | ✅ 完了 |
| **消費解析** | `consume_analysis.rs` (363行) | 関数跨ぎ消費パターンクエリ、キャッシュ対応 | ✅ 完了 |
| **チェーン呼び出し** | `chain_calls.rs` (652行) | メソッドチェーンの所有権フロー解析 | ✅ 完了 |
| **ライフタイム追跡** | `lifecycle.rs` (1037行) | 変数の完全なライフタイム追跡 | ✅ 完了 |
| **空状態** | `empty_state.rs` (513行) | Move 後の変数の空状態追跡 | ✅ 完了 |
| **制御フロー** | `control_flow.rs` (353行) | 分岐状態の合流解析 | ⚠️ 骨組みのみ完了、中核解析ロジックは空実装 |
| **Unsafe 検査** | `unsafe_check.rs` (113行) | unsafe ブロック外での生ポインタ逆参照をエラー報告 | ✅ 完了 |
| **Send/Sync** | `send_sync.rs` (401行) | 型レベルの Send/Sync 型制約検査および制約伝播 | ✅ 完了（独立して使用） |

---

## テストカバレッジ

**83 個の単体テスト**、内訳は以下の通り：

| ファイル | テスト数 | カバレッジ |
|------|--------|----------|
| `borrow_checker.rs` | 16 | 最も充実：単体テスト＋エンドツーエンドテスト |
| `chain_calls.rs` | 13 | 充実：チェーン抽出、消費パターン推論、長鎖、混合呼び出し |
| `consume_analysis.rs` | 11 | 充実：Returns/Consumes パターン、キャッシュ、複数引数 |
| `ownership_flow.rs` | 10 | 充実：直接返却、間接返却、複数引数の部分的返却 |
| `lifecycle.rs` | 10 | 充実：作成／消費／解放の追跡、問題検出 |
| `cycle_check.rs` | 8 | 良好：ループなし／単方向鎖／深さ制限／unsafe バイパス |
| `intra_task_cycle.rs` | 7 | 良好：ループなし／単純ループ／自己参照／複数ループ |
| `move_semantics.rs` | 6 | 基本：状態追跡、UseAfterMove |
| `control_flow.rs` | 1 | 不十分：状態合流関数のテストのみ |
| `empty_state.rs` | 1 | 不十分：状態合流のテストのみ |
| その他 | 0 | テストなし：drop_semantics, clone, mut_check, ref_semantics, unsafe_check, send_sync |

---

## RFC との対比（RFC-009 所有権モデル）

| RFC 設計要点 | 実装状態 | 説明 |
|-------------|---------|------|
| Move セマンティクス（デフォルト） | ✅ 実装済み | MoveChecker が UseAfterMove を検出 |
| &T/&mut T 借用トークン | ✅ 実装済み | BorrowChecker がトークン衝突検出を実装 |
| &T によるソースデータの凍結（ReadToken） | ✅ 実装済み | ReadToken の生存期間中 WriteToken を禁止、凍結保証下で安全に Dup 可能 |
| &mut T の線形性 | ✅ 実装済み | 同一由来の &mut T は同時に 1 つしかアクティブにできない |
| トークン衝突検出（フロー依存生存解析） | ✅ 実装済み | 関数本体内でトークン状態を追跡 |
| ref キーワード（Rc/Arc 自動選択） | ⚠️ 部分実装 | ref セマンティクスチェッカーが存在 |
| clone() による明示的なディープコピー | ✅ 実装済み | CloneChecker が clone された移動／解放値の検出 |
| unsafe + *T | ✅ 実装済み | UnsafeChecker が unsafe ブロック外の生ポインタ操作を検査 |
| タスク内ループ：黙って許可 | ✅ 実装済み | IntraTaskCycleTracker が警告モードで追跡 |
| タスク跨ぎループ：lint | ✅ 実装済み | CycleChecker がタスク跨ぎの spawn 参照ループを検出 |
| ライフタイムパラメータ 'a なし | ✅ 設計準拠 | ライフタイム引数は実装されていない |
| Send/Sync 型制約 | ✅ 実装済み | SendSyncChecker は OwnershipChecker から独立 |

---

## コード品質評価

| 評価軸 | スコア | 説明 |
|------|------|------|
| 未完了事項 | 3 | テスト追加、control_flow ロジック、ref エスケープ解析 |
| テストカバレッジ | 良好 | 83 個のテスト、borrow_checker/chain_calls/consume_analysis はテスト充実 |
| ドキュメント品質 | 良好 | モジュール／構造体／メソッドレベルすべてにドキュメントコメントあり |
| コードアーキテクチャ | 優秀 | OwnershipChecker による統一オーケストレーション、責務分離が明確 |
| RFC 準拠 | 高度に準拠 | RFC-009 v9 設計に高度に準拠 |

---

## 改善項目

1. **5 サブモジュールの単体テストを追加**：drop_semantics, clone, mut_check, ref_semantics, unsafe_check
2. **control_flow 解析器の中核ロジックを実装**（現状は空の骨組み）
3. **ref による Rc/Arc の自動選択エスケープ解析を完成させる**