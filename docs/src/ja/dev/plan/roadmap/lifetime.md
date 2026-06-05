---
title: "借用チェッカーの状態"
---

# 借用チェッカー（Lifetime）

> **モジュール状態**：安定（4 項目の改善待ち）
> **位置**：`src/middle/passes/lifetime/`
> **最終更新**：2026-06-01

---

## モジュールの概要

借用チェッカーモジュールは完全な**所有権分析とライフタイム管理システム**であり、Move セマンティクス、借用競合、可変性違反などの所有権関連問題をチェックします。

**コード量**：約 300KB ソースコード（15 サブファイル）

---

## 機能一覧

### コアチェッカー（OwnershipChecker 統合エントリーポイントに統合済み）

| サブモジュール | ファイル | 機能 | 状態 |
|--------|------|------|------|
| **Move セマンティクス** | `move_semantics.rs` (575行) | UseAfterMove 検出、空状態(Empty)再代入をサポート | ✅ 完了 |
| **Drop セマンティクス** | `drop_semantics.rs` (143行) | UseAfterDrop、DropMovedValue、DoubleDrop 検出 | ✅ 完了 |
| **可変性チェック** | `mut_check.rs` (395行) | 不変変数代入、不変オブジェクト変更メソッド、不変フィールド代入 | ✅ 完了 |
| **Ref セマンティクス** | `ref_semantics.rs` (145行) | RefNonOwner 検出——ref は有効な所有者にのみ適用可能 | ✅ 完了 |
| **Clone セマンティクス** | `clone.rs` (173行) | CloneMovedValue、CloneDroppedValue 検出 | ✅ 完了 |
| **借用トークン** | `borrow_checker.rs` (503行) | 借用トークン競合検出：MutableBorrowConflict、BorrowAfterMove、UseWhileFrozen | ✅ 完了 |
| **spawn 間ループ** | `cycle_check.rs` (616行) | タスク間循環参照検出、DFS サイクル検出 | ✅ 完了 |
| **タスク内ループ** | `intra_task_cycle.rs` (406行) | タスク内 ref 循環追跡（警告モード） | ✅ 完了 |

### 補助アナライザー

| サブモジュール | ファイル | 機能 | 状態 |
|--------|------|------|------|
| **所有権フロー解析** | `ownership_flow.rs` (426行) | 関数引数が返り値に返されるかを分析 | ✅ 完了 |
| **消費分析** | `consume_analysis.rs` (363行) | 関数間消費パターンクエリをサポートしキャッシュ対応 | ✅ 完了 |
| **チェーン呼び出し** | `chain_calls.rs` (652行) | メソッドチェーン所有権フロー分析 | ✅ 完了 |
| **ライフタイム追跡** | `lifecycle.rs` (1037行) | 変数の完全なライフタイム追跡 | ✅ 完了 |
| **空状態** | `empty_state.rs` (513行) | Move 後の変数の空状態追跡 | ✅ 完了 |
| **制御フロー** | `control_flow.rs` (353行) | 分岐状態マージ分析 | ⚠️ 骨格完成、主要分析ロジックは空実装 |
| **Unsafe チェック** | `unsafe_check.rs` (113行) | unsafe ブロック外での生ポインタ逆参照でエラーを報告 | ✅ 完了 |
| **Send/Sync** | `send_sync.rs` (401行) | 型レベルの Send/Sync 制約チェックと制約伝播 | ✅ 完了（独立使用） |

---

## テストカバレッジ

**83 のユニットテスト**、分布は次のとおりです：

| ファイル | テスト数 | カバレッジ状況 |
|------|--------|----------|
| `borrow_checker.rs` | 16 | 最も充実：ユニットテスト＋エンドツーエンドテスト |
| `chain_calls.rs` | 13 | 充実：チェーン抽出、消費パターン推論、長チェーン、混合呼び出し |
| `consume_analysis.rs` | 11 | 充実：Returns/Consumes パターン、キャッシュ、複数パラメータ |
| `ownership_flow.rs` | 10 | 充実：直接戻り値、間接戻り値、複数パラメータ一部戻り |
| `lifecycle.rs` | 10 | 充実：作成/消費/解放追跡、問題検出 |
| `cycle_check.rs` | 8 | 良好：循環なし/単一チェーン/深度制限/unsafe バイパス |
| `intra_task_cycle.rs` | 7 | 良好：循環なし/単純循環/自己参照/複数循環 |
| `move_semantics.rs` | 6 | 基本：状態追跡、UseAfterMove |
| `control_flow.rs` | 1 | 不足：状態マージ関数のみテスト |
| `empty_state.rs` | 1 | 不足：状態マージのみテスト |
| その他 | 0 | テストなし：drop_semantics, clone, mut_check, ref_semantics, unsafe_check, send_sync |

---

## RFC 比較（RFC-009 所有権モデル）

| RFC 設計要点 | 実装状況 | 説明 |
|-------------|---------|------|
| Move セマンティクス（デフォルト） | ✅ 実装済み | MoveChecker が UseAfterMove を検出 |
| &T/&mut T 借用トークン | ✅ 実装済み | BorrowChecker がトークン競合検出を実装 |
| &T は複製可能（Dup） | ✅ 実装済み | 複数の &T トークンが同時に存在可能 |
| &mut T は線形 | ✅ 実装済み | 同一ソースからの &mut T は1つのみ有効 |
| フリーズメカニズム | ✅ 実装済み | freeze() は &mut T を &T にフリーズ |
| トークン競合検出（フロー感知活性分析） | ✅ 実装済み | 関数本体でトークン状態を追跡 |
| ref キーワード（Rc/Arc 自動選択） | ⚠️ 部分実装 | ref セマンティクeschecker が存在 |
| clone() は明示的なディープコピー | ✅ 実装済み | CloneChecker が clone した移動/解放された値を検出 |
| unsafe + *T | ✅ 実装済み | UnsafeChecker が unsafe ブロック外の生ポインタ操作をチェック |
| タスク内ループ：黙認 | ✅ 実装済み | IntraTaskCycleTracker が警告モードで追跡 |
| タスク間ループ：lint | ✅ 実装済み | CycleChecker が spawn 間循環参照を検出 |
| ライフタイム 'a なし | ✅ 設計適合 | ライフタイムパラメータは実装されていない |
| Send/Sync 制約 | ✅ 実装済み | SendSyncChecker は OwnershipChecker から独立 |

---

## コード品質評価

| 次元 | 評価 | 説明 |
|------|------|------|
| 未完了事項 | 4 | テスト補完、control_flow ロジック、freeze メカニズム、ref エスケープ解析 |
| テストカバレッジ | 良好 | 83 テスト、borrow_checker/chain_calls/consume_analysis のテストは充実 |
| ドキュメント品質 | 良好 | モジュール/構造体/メソッドレベル 모두 문서 주석 있음 |
| コードアーキテクチャ | 優秀 | OwnershipChecker が統一的に編成、責務分離が清晰 |
| RFC コンプライアンス | 高い適合 | RFC-009 v9 設計に高い適合度 |

---

## 改善待ち項目

1. **5 つのサブモジュールのユニットテストを補完する**：drop_semantics, clone, mut_check, ref_semantics, unsafe_check
2. **control_flow アナライザーの主要ロジックを実装する**（現在、空の骨格のみ）
3. **freeze メカニズムを改善する**（&mut T の一時的な &T へのフリーズ）
4. **ref の Rc/Arc 自動選択のエスケープ解析を改善する**