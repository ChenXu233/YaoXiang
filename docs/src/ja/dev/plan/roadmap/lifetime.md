---
title: "借用チェッカー状態"
---

# 借用チェッカー（Lifetime）

> **モジュール状態**：過渡期——v8 リニアスキャンアーキテクチャ → v9 ホーア命题パイプ
> **位置**：`src/middle/passes/lifetime/`
> **最終更新**：2026-06-13
>
> **関連 RFC**：
> - [RFC-009: 所有権モデル設計](../design/rfc/accepted/009-ownership-model.md) — 受理済み
> - [RFC-009a: トークンライフタイム解析——ホーア証明パイプベース](../design/rfc/accepted/009a-borrow-proof-pipeline.md) — 受理済み

---

## モジュール概要

借用チェッカーモジュールは YaoXiang の所有権解析——Move セマンティクス、借用トークン衝突、Drop/Clone 正確性、ref 環検出、可変性違反——を担当する。

**現在のアーキテクチャ**（過渡期）：
- ir_gen がハードコードで Borrow/Release 命令を挿入（字句スコープ）
- BorrowChecker は IR をリニアスキャンし、トークン衝突を受動的に検証
- ControlFlowAnalyzer は存在するが、コアのロジックは空
- ユーザーから見える動作は概ね正しいが、低層は RFC-009a のホーア命题パイプではない

**目標アーキテクチャ**（RFC-009 + RFC-009a）：
- ブランド木によるトークン派生関係の追跡
- コンシューマー解析が NLL 解放を駆動
- 逆方向 BFS 活性解析（高速パスで 95%+ のシナリオをカバー）
- SMT 論理的切断によるフォールバック（極めて稀な while + パス条件のシナリオ）
- Release はスコープ解析が駆動し、ir_gen でハードコードしない

**コード量**：約 300KB ソースコード（15 サブファイル）

---

## RFC 整合状態

### RFC-009 五つの核心概念

| 概念 | ユーザー可視動作 | 低層実装 |
|------|-------------|---------|
| **Move** | ✅ 完成 | MoveChecker、UseAfterMove 検出 |
| **&T / &mut T** | ✅ 完成 | BorrowChecker リニアスキャン、Borrow/Release 命令に受動応答 |
| **ref** | ⚠️ 環検出完成、エスケープ解析欠如 | ref_semantics + cycle_check + intra_task_cycle |
| **clone()** | ✅ 完成 | CloneChecker、0 tests |
| **unsafe + *T** | ✅ 完成 | UnsafeChecker |

### RFC-009a 六段階

| 段階 | 内容 | 状態 | 説明 |
|------|------|------|------|
| 1 | ブランド木データ構造 | ❌ 未着手 | HashMap<String, BorrowToken> を置換、ブランド ID + 親ノード + コンシューマーリスト |
| 2 | コンシューマー解析 | ❌ 未着手 | DAG 構築時にトークンコンシューマーを自動収集、NLL の基礎 |
| 3 | 逆方向 BFS 活性解析 | ❌ 未着手 | ブランド木 + コンシューマー + break 切断 → 95%+ のシナリオをカバー |
| 4 | スコープ駆動 Release | ❌ 未着手 | ir_gen ハードコード削除、スコープ出口点で LIFO 挿入、? 自動処理 |
| 5 | SMT 論理的切断 | ❌ 未着手 | RFC-027 Phase 2 にブロック、while + パス条件時のみトリガー |
| 6 | クリーンアップ | ❌ 未着手 | BorrowChecker → BorrowPredicateEmitter、ControlFlowAnalyzer 削除 |

---

## 現在のモジュール一覧

### コアチェッカー

| サブモジュール | ファイル | 機能 | テスト |
|--------|------|------|------|
| **Move セマンティクス** | `move_semantics.rs` | UseAfterMove 検出、空状態再代入 | 6 |
| **Drop セマンティクス** | `drop_semantics.rs` | UseAfterDrop、DropMovedValue、DoubleDrop | 0 |
| **可変性検査** | `mut_check.rs` | 不変変数への代入 / 変異メソッド / フィールド代入 | 0 |
| **Ref セマンティクス** | `ref_semantics.rs` | RefNonOwner 検出 | 0 |
| **Clone セマンティクス** | `clone.rs` | CloneMovedValue、CloneDroppedValue | 0 |
| **借用トークン** | `borrow_checker.rs` | トークン衝突検出（リニアスキャンアーキテクチャ） | 16 |
| **タスク横断環** | `cycle_check.rs` | spawn を跨ぐ循環参照の DFS 検出 | 8 |
| **タスク内環** | `intra_task_cycle.rs` | タスク内 ref 循環追跡（警告モード） | 7 |

### 補助モジュール

| サブモジュール | ファイル | 帰属先 |
|--------|------|------|
| **所有権フロー** | `ownership_flow.rs` | 保持 |
| **コンシューム解析** | `consume_analysis.rs` | → Phase 2 でブランド木に統合 |
| **チェーン呼び出し** | `chain_calls.rs` | 保持 |
| **ライフサイクル追跡** | `lifecycle.rs` | 保持——Drop 挿入に必要 |
| **空状態** | `empty_state.rs` | 保持 |
| **制御フロー** | `control_flow.rs` | → Phase 6 で削除 |
| **Unsafe 検査** | `unsafe_check.rs` | 保持 |
| **Send/Sync** | `send_sync.rs` | 保持（独立使用） |

---

## 実装ロードマップ

### 段階 0：テスト補完（即時着手可能、リファクタリングをブロック）

> アーキテクチャに手を入れる前に、まず既存動作のテスト網を敷く。

| # | タスク | ファイル |
|---|------|------|
| 0.1 | Drop セマンティクステスト補完 | `tests/drop_semantics.rs` |
| 0.2 | Clone セマンティクステスト補完 | `tests/clone.rs` |
| 0.3 | 可変性検査テスト補完 | `tests/mut_check.rs` |
| 0.4 | Ref セマンティクステスト補完 | `tests/ref_semantics.rs` |
| 0.5 | Unsafe 検査テスト補完 | `tests/unsafe_check.rs` |

### 段階 1：ブランド木データ構造（RFC-009a Phase 1）

| # | タスク | 成果物 |
|---|------|------|
| 1.1 | `BrandTree`、`BrandNode` 構造体の定義 | `brand_tree.rs` |
| 1.2 | プレフィックス一致による衝突判定実装 | `conflicts()` |
| 1.3 | DAG 構築時のブランドノード登録実装 | ir_gen に統合 |
| 1.4 | 単体テスト | `tests/brand_tree.rs` |

### 段階 2：コンシューマー解析（RFC-009a Phase 2）

| # | タスク | 成果物 |
|---|------|------|
| 2.1 | DAG 構築時に各トークンのコンシューマーリストを自動収集 | `BrandNode.consumers` |
| 2.2 | システム述語生成器の定義（Borrow/Move/Drop/Mut → `{P} op {Q}`） | インターフェース定義 |

### 段階 3：逆方向 BFS 活性解析（RFC-009a Phase 3）

| # | タスク | 成果物 |
|---|------|------|
| 3.1 | 逆方向 BFS アルゴリズム実装（break で逆辺を切断） | 高速パス |
| 3.2 | RFC-027 証明パイプインターフェース接続（Proved/Disproved） | パイプ接続 |
| 3.3 | NLL 反復境界ルール実装 | ループ内トークンの反復横断セマンティクス |
| 3.4 | BorrowChecker リニアスキャンの置換 | `check_instruction` 内の Borrow/Release match 削除 |

### 段階 4：スコープ駆動 Release（RFC-009a Phase 4-5）

| # | タスク | 成果物 |
|---|------|------|
| 4.1 | スコープ出口点の収集（`}`、`?`、明示的 return） | ir_gen |
| 4.2 | LIFO Release 挿入（ブランド木の親子関係で自動カスケード） | ir_gen |
| 4.3 | `ir_gen.rs` の Call 後ハードコード Release 削除 | コードクリーンアップ |

### 段階 5：SMT 論理的切断（RFC-009a Phase 5、RFC-027 Phase 2 に依存）

| # | タスク | 成果物 |
|---|------|------|
| 5.1 | パス条件収集の統合 | RFC-027 パイプから取得 |
| 5.2 | SMT フォールバック：`path_cond ⇒ !loop_cond` | 低速パス |
| 5.3 | while ループ体内借用検査の活性化 | 現在保守的に拒否しているシナリオ |

### 段階 6：クリーンアップ（RFC-009a Phase 6）

| # | タスク | 成果物 |
|---|------|------|
| 6.1 | `BorrowChecker` → `BorrowPredicateEmitter` | リネーム、責務明確化 |
| 6.2 | `ControlFlowAnalyzer` 削除 | パイプで統一処理 |
| 6.3 | `consume_analysis.rs` のコンシューマー情報をブランド木へ移行 | 重複排除 |
| 6.4 | エラーメッセージフォーマットの更新 | RFC-009a §エラーメッセージ設計と整合 |

---

## 独立タスク（メインライン非ブロッキング）

| # | タスク | 説明 |
|---|------|------|
| I.1 | ref エスケープ解析（Rc vs Arc 自動選択） | 現在コンパイラはタスク横断か否かを区別せず、Arc を統一使用 |
| I.2 | `control_flow.rs` の `ControlFlowAnalyzer` 削除までは新コードを追加しないこと | — |

---

## テストカバレッジ

**現在：83 単体テスト**

| ファイル | テスト数 | カバレッジ状況 |
|------|--------|----------|
| `borrow_checker.rs` | 16 | 充分 |
| `chain_calls.rs` | 13 | 充分 |
| `consume_analysis.rs` | 11 | 充分 |
| `ownership_flow.rs` | 10 | 充分 |
| `lifecycle.rs` | 10 | 充分 |
| `cycle_check.rs` | 8 | 良好 |
| `intra_task_cycle.rs` | 7 | 良好 |
| `move_semantics.rs` | 6 | 基本 |
| `control_flow.rs` | 1 | 不足 |
| `empty_state.rs` | 1 | 不足 |
| その他 | 0 | **欠落**：drop_semantics, clone, mut_check, ref_semantics, unsafe_check |

---

## コード品質評価

| 次元 | スコア | 説明 |
|------|------|------|
| 未完了事項 | 10 | 段階 0 テスト (5) + 段階 1-6 アーキテクチャ (6) + ref エスケープ解析 (1) |
| テストカバレッジ | 要強化 | 5 サブモジュール 0 tests、リファクタリング前に必ず補完 |
| ドキュメント品質 | 良好 | モジュール / 構造体 / メソッドレベルすべてにドキュメントコメントあり |
| コードアーキテクチャ | 過渡期 | 現在のリニアスキャンアーキテクチャは動作するが、RFC-009a と整合しない |