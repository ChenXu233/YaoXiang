---
title: "所有権チェッカー状態"
---

# 所有権チェッカー（Ownership）

> **モジュール状態**：移行完了——フロントエンドホーア命题（命题）パイプラインが引き継ぎ済み
> **新アーキテクチャ位置**：`src/frontend/core/typecheck/layers/ownership.rs`（約 1600 行）
> **レガシー位置**：`src/middle/passes/lifetime/`（保持、徐々にクリーンアップ）
> **最終更新**：2026-06-15
>
> **関連 RFC**：
> - [RFC-009: 所有権モデル設計](../design/rfc/accepted/009-ownership-model.md) — 承認済み
> - [RFC-009a: トークンライフタイム分析——ホーア証明パイプライン基盤](../design/rfc/accepted/009a-borrow-proof-pipeline.md) — 承認済み
>
> **既知の問題**：[ongoing/ownership-known-issues.md](../ongoing/ownership-known-issues.md) — 6 件のバグと精度のトレードオフ

---

## モジュール概要

所有権チェッカーは YaoXiang の所有権解析を担当する——Move セマンティクス、借用トークンの競合、Drop 正確性、可変性違反、NLL 精密解放、クロージャキャプチャ、関数シグネチャクエリ、ref エスケープ解析。

**現在のアーキテクチャ**（v9 ホーア命题（命题）パイプライン）：
- ブランドツリー（BrandTree）がトークン派生関係と競合判定を追跡
- コンシューマ分析が NLL 解放（ReleasePlan）を駆動
- 逆方向 BFS 活性解析（ファストパス、95% 以上のシナリオをカバー）
- SMT 論理切断フォールバック（極めて稀な while + パス条件シナリオ）
- スコープ駆動 Drop（スコープ退出時に自動的に `VarState::Dropped` をマーク）
- クロージャキャプチャ分析（save/restore/diff → CapturesStore）
- 関数シグネチャクエリ（TypeEnvironment → T/&T/&mut T → Move/ReadBorrow/WriteBorrow）
- ref エスケープ解析（spawn 内で使用 → Arc、それ以外 → Rc）
- ir_gen が ReleasePlan を読み取り Drop 命令を挿入 + `escaped_refs` に従って RcNew/ArcNew を選択

---

## RFC 整合状態

### RFC-009 五つのコアコンセプト

| コンセプト | ユーザ可視動作 | 内部実装 |
|------|-------------|---------|
| **Move** | ✅ 完了 | OwnershipChecker、UseAfterMove 検出 |
| **&T / &mut T** | ✅ 完了 | ブランドツリートークン競合検出（ファストパス + SMT フォールバック） |
| **ref** | ✅ 完了 | エスケープ解析による Rc/Arc 自動選択 |
| **clone()** | ✅ 完了 | CloneChecker、0 tests |
| **unsafe + *T** | ✅ 完了 | UnsafeChecker |

### RFC-009a 六段階（新バージョン——フロントエンド実装）

| 段階 | 内容 | 状態 | 説明 |
|------|------|------|------|
| 1 | ブランドツリーデータ構造 | ✅ 完了 | `BrandTree` + `BrandNode` + `BrandId` |
| 2 | コンシューマ分析 | ✅ 完了 | `BrandNode.consumers`、AST 走査による自動収集 |
| 3 | 逆方向 BFS 活性解析 | ✅ 完了 | `fast_path_check()`、break によるバックエッジ切断 |
| 4 | スコープ駆動 Release | ✅ 完了 | `ReleasePlan` + `scope_vars` スタック、LIFO Drop |
| 5 | SMT 論理切断 | ✅ 完了 | `smt_cut(path_cond, loop_cond)` via Z3 |
| 6 | クリーンアップ | ✅ 完了 | レガシーファイル削除済み、エラーコード形式未統一（P2） |

### 補足段階

| 段階 | 内容 | 状態 | 説明 |
|------|------|------|------|
| D.1 | ref エスケープ解析（Rc vs Arc） | ✅ 完了 | `ref_vars` + `escaped_refs` + `inside_spawn`、ref 属性伝播 |
| D.2 | テストカバレッジ拡張 | ✅ 完了 | 61 tests（元 31 → 目標 50+） |
| D.3 | Drop セマンティクストリガーポイント | ✅ 完了 | `VarState::Dropped` 有効化、スコープ退出で自動マーク |
| D.4 | 可変性チェック | ✅ 完了 | `&mut` と代入が `var_mutability` をチェック、`mut_violation` を emit |
| D.5 | ロードマップ同期 | ✅ 完了 | 本ドキュメント |
| — | クロージャキャプチャ分析 | ✅ 完了 | save→walk→diff→restore→CapturesStore |
| — | 関数シグネチャクエリ | ✅ 完了 | TypeEnvironment.get_var → T/&T/&mut T |
| — | Spawn walk | ✅ 完了 | save/restore で外層汚染防止、ref エスケープ検出 |

---

## 新アーキテクチャのコアコンポーネント

### `src/frontend/core/typecheck/layers/ownership.rs`（約 1600 行）

| コンポーネント | 機能 |
|------|------|
| `BrandId` / `BrandTree` | トークン識別 + 派生ツリー + 競合判定 + コンシューマ追跡 |
| `ControlFlowGraph` | CFG ノード/エッジ/パス条件、Break/BackEdge |
| `fast_path_check()` | 逆方向 BFS 活性解析（ファストパス） |
| `smt_cut()` | SMT 論理切断（スローパス、Z3） |
| 5 種類のシステム述語 | borrow_conflict / use_after_move / use_after_drop / double_drop / mut_violation |
| `OwnershipChecker` | AST 走査 + ブランドツリー + CFG + 述語検証 |
| `ReleasePlan` | NLL 精密解放プラン（コンシューマ + スコープ Drop の二源マージ） |
| `VarState` | Alive / Moved / Dropped の三状態 |
| `Captures` / `CapturesStore` | クロージャキャプチャ変数集合 + ストレージ |
| `StateSnapshot` | save_state / restore_state / diff_captures |
| `ParamOwnership` | Move / ReadBorrow / WriteBorrow |
| `ref_vars` / `escaped_refs` / `inside_spawn` | ref エスケープ解析（属性伝播を含む） |

### `src/middle/core/ir_gen.rs`

- `TypeCheckResult.release_plan` を読み取り → `Drop` 命令を挿入（NLL 精密解放ポイント）
- `TypeCheckResult.escaped_refs` を読み取り → `Expr::Ref` は `RcNew` または `ArcNew` を選択

### `src/middle/core/ir.rs` / `bytecode.rs` / `opcode.rs`

- 新規 `RcNew` 命令 + `Opcode::RcNew(0x89)`

---

## 現在の middle 層モジュール一覧

> 注：`borrow_checker.rs`、`control_flow.rs`、`consume_analysis.rs`、`move_semantics.rs`、
> `drop_semantics.rs`、`mut_check.rs`、`ref_semantics.rs`、`clone.rs`、`empty_state.rs`、
> `send_sync.rs` は削除済み。以下は保持されているアクティブモジュール。

| サブモジュール | ファイル | 機能 |
|--------|------|------|
| **チェーン呼び出し** | `chain_calls.rs` | チェーンメソッド呼び出し分析 |
| **クロスタスク環** | `cycle_check.rs` | クロス spawn 循環参照 DFS |
| **タスク内環** | `intra_task_cycle.rs` | タスク内 ref 循環追跡 |
| **ライフサイクル** | `lifecycle.rs` | IR レベル Drop 位置追跡 |
| **所有権フロー** | `ownership_flow.rs` | 関数所有権フロー分析 |
| **Unsafe** | `unsafe_check.rs` | unsafe ブロックバイパスチェック |
| **エラー型** | `error.rs` | ValueState + Checker trait |

---

## テストカバレッジ

**フロントエンド所有権チェッカー：61 個のユニットテスト**

| テストカテゴリ | テスト数 | カバレッジ内容 |
|----------|--------|----------|
| 基本（BrandId/競合/カスケード/コンシューマ/ファストパス） | 17 | トークンプレフィックス、競合判定、カスケード削除、コンシューマ追跡、BFS 活性 |
| システム述語 | 6 | borrow_conflict / use_after_move / use_after_drop / double_drop / mut_violation |
| E2E 統合（基本） | 7 | use after move、valid move、argument move、借用競合、書き込み書き込み競合、読み取り読み取り安全 |
| E2E 可変性 | 5 | &mut 非 mut、&mut mut、代入非 mut、代入 mut、パラメータ非 mut |
| E2E Drop | 2 | スコープ Drop（ReleasePlan）、ネストブロック Drop |
| E2E Move+Borrow | 1 | move 後の borrow 検出 |
| E2E 制御フロー | 2 | if/else 双分岐競合、while ループ内借用 |
| E2E Drop 順序 | 1 | 複数変数同 span 解放 |
| E2E 戻り値 | 2 | return Move、return 後 use |
| E2E 多重借用 | 2 | 三つの ReadToken、Read+Write 競合 |
| E2E ブロック式 | 1 | 内側ブロック変数スコープ |
| E2E 連続 Move | 2 | 連続 Move、double move 検出 |
| E2E パラメータ | 2 | パラメータ move 後 use、パラメータが ReleasePlan に無い |
| E2E クロージャキャプチャ | 5 | Move キャプチャ、Read キャプチャ、無キャプチャ、定義後呼び出し前、二次呼び出し |
| E2E 関数シグネチャ | 2 | 未知関数フォールバック Move、未登録関数フォールバック |
| E2E ref エスケープ | 4 | spawn なしエスケープなし、spawn 内エスケープ、非 ref エスケープなし、ネスト spawn |

**middle 層テスト：53 個のユニットテスト**

---

## コード品質評価

| ディメンション | 評価 | 説明 |
|------|------|------|
| 未完了事項 | 4 | エラーメッセージ形式統一 (P2) + 段階 0 テスト補完 (5) + 既知の問題 6 件 |
| テストカバレッジ | 良好 | フロントエンド 61 tests + middle 53 tests = 114 tests |
| ドキュメント品質 | 良好 | モジュール/構造体/メソッドレベルのドキュメントコメントあり |
| コードアーキテクチャ | 移行完了 | フロントエンドホーア命题（命题）パイプラインがコアルジックを引き継ぎ済み；middle 層のレガシーファイルは削除済み |