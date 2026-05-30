---
title: RFC-009 v9 実装完全性監査レポート
status: ongoing
created: 2026-05-29
---

# RFC-009 v9 実装完全性監査レポート

## 監査範囲

RFC-009 v9 借用トークンシステム設計文書と照合し、コンパイラ実装の完全性を項目ごとにチェックする。型システム、パーサ、借用チェッカー、IR生成、Dupシステム、クロージャキャプチャの6つの次元をカバーする。

---

## 1. フロントエンド（型システム + パーサ）

| # | チェック項目 | 状態 | ファイル | 説明 |
|---|--------|------|------|------|
| 1.1 | _lexical_ `&`/`&mut` | ✅ | `tokenizer.rs` L249-268, `tokens.rs` L75-76 | `Ampersand` と `MutRef` の2つの TokenKind が存在、`&&`(And) は影響なし |
| 1.2 | AST `Type::Ref` | ✅ | `ast.rs` L474-480 | `Ref { mutable: bool, inner: Box<Type>, span: Span }` フィールドが完備 |
| 1.3 | AST `Expr::Borrow` | ✅ | `ast.rs` L131-137 | `Borrow { mutable: bool, expr: Box<Expr>, span: Span }` フィールドが完備 |
| 1.4 | パーサ `&T`/`&mut T` 型 | ✅ | `types.rs` L73-93 | `parse_type_annotation` が正しく `Ampersand` と `MutRef` をマッチ |
| 1.5 | パーサ `&expr`/`&mut expr` | ✅ | `nud.rs` L38-39, L196-213 | `parse_borrow` メソッドがmutable を正しく区別 |
| 1.6 | MonoType `Ref { mutable, inner }` | ✅ | `mono.rs` L189-196 | コメント「コンパイル時ゼロサイズ型、ランタイム表現なし」 |
| 1.7 | MonoType Display | ✅ | `mono.rs` L342-348 | `&T` または `&mut T` を出力 |
| 1.8 | From<ast::Type> 変換 | ✅ | `mono.rs` L556-559 | `Type::Ref` が正しく `MonoType::Ref` に変換される |
| 1.9 | 型チェッカー Borrow 推論 | ✅ | `expressions.rs` L1096-1106 | 内部型を推論して `MonoType::Ref` でラップ |
| 1.10 | Formatter | ✅ | `types.rs` L107-113, `expr.rs` L168-178 | `Type::Ref` と `Expr::Borrow` のフォーマットが正しい |
| **1.11** | **Dup trait: `&T` Dup, `&mut T` 不Dup** | **⚠️** | **`solver.rs` L201-233** | **`check_dup_trait` に `MonoType::Ref` の明示的マッチがなく、両方とも `_ => false` に落ちる** |

---

## 2. ミドルエンド — 借用チェッカー

| # | チェック項目 | 状態 | ファイル | 説明 |
|---|--------|------|------|------|
| 2.1 | BorrowChecker の存在 | ✅ | `borrow_checker.rs` | 496行、メソッド完備 |
| 2.2 | トークンステートマシン | ✅ | `borrow_checker.rs` | `Active`/`Frozen`/`Moved` の3状態 |
| 2.3 | 複数の `&T` 同源を許可（Dup） | ✅ | `borrow_checker.rs` L174 | 不変+不変の組み合わせでエラーなし |
| 2.4 | `&mut T` アクティブ時に `&T` 生成でエラー | ✅ | `borrow_checker.rs` L165-173 | `MutableBorrowConflict` を生成 |
| 2.5 | `&mut T` アクティブ時に `&mut T` 生成でエラー | ✅ | `borrow_checker.rs` L147-155 | `MutableBorrowConflict` を生成 |
| 2.6 | 冻结後使用でエラー | ✅ | `borrow_checker.rs` L203-207 | `UseWhileFrozen` を生成 |
| 2.7 | 移動後使用でエラー | ✅ | `borrow_checker.rs` L209-214 | `BorrowAfterMove` を生成 |
| 2.8 | 冻结機構 | ✅ | `borrow_checker.rs` | `&mut T` は `&T` に冻结可能、元の `&mut` は自動解凍 |
| 2.9 | OwnershipChecker 統合 | ✅ | `mod.rs` L122, L153-154 | `borrow_checker` フィールドが存在、`check_function` で呼び出し |
| 2.10 | エラータイプ | ✅ | `error.rs` | `MutableBorrowConflict`/`BorrowAfterMove`/`UseWhileFrozen` の3バリアント |
| **2.11** | **ブランド機構（Brand）** | **❌** | **`borrow_checker.rs`** | **変数名を文字列としてのみ使用、コンパイル時一意IDなし、派生ブランドチェーンなし** |
| **2.12** | **`&mut T` IR 命令** | **❌** | **`ir.rs`** | **IR に `&mut T` を作成 命令がなく、`create_borrow(mutable=true)` はIR解析で到達不能** |

---

## 3. ミドルエンド — IR 生成

| # | チェック項目 | 状態 | ファイル | 説明 |
|---|--------|------|------|------|
| **3.1** | **IR Borrow/Release 命令** | **❌** | **`ir.rs`** | **`Instruction` 列挙型に `Borrow` も `Release` もなく、`ArcNew`/`ArcClone`/`ArcDrop` のみ** |
| **3.2** | **ir_gen Expr::Borrow 処理** | **❌** | **`ir_gen.rs`** | **`generate_expr_ir` に `Expr::Borrow` 分岐が完全になく、`&expr` はIR段階で黙って無視** |
| 3.3 | MakeClosure env 填充 | ⚠️ | `ir_gen.rs` L3186-3201 | キャプチャ分析は動作するが、ZST最適化が欠落（TODOコメント） |
| 3.4 | Bytecode type_id | ⚠️ | `bytecode.rs` L413 | type_id 49が割り当てられているが対応命令なし |
| 3.5 | Bytecode From<MonoType> | ⚠️ | `bytecode.rs` L1418-1424 | スタブ、すべての方言が `IrType::Void` にマッピング |
| **3.6** | **解釈器 Borrow 処理** | **❌** | **`execute.rs`** | **borrow関連処理なし、`RuntimeValue` にborrowバリアントなし** |
| **3.7** | **ZST 最適化** | **❌** | **`ir_gen.rs`** | **`MonoType::Ref` はZSTとしてコメントされているが、IR生成に最適化ロジックなし** |

---

## 4. Dupシステム（RFC-011 Section 2.4）

| # | チェック項目 | 状態 | ファイル | 説明 |
|---|--------|------|------|------|
| 4.1 | Dup trait 登録 | ✅ | `std_traits.rs` L31 | `"Dup"` が `STD_TRAITS` に存在 |
| 4.2 | is_marker フィールド | ✅ | `trait_data.rs` L31 | `pub is_marker: bool` が存在 |
| 4.3 | Dup implies Clone | ✅ | `std_traits.rs` L115 | `parent_traits: vec!["Clone"]` |
| 4.4 | プリミティブ Dup impl | ✅ | `std_traits.rs` L153-186 | Int/Float/Bool/Char/String/Bytes |
| 4.5 | Solver 再帰チェック | ✅ | `solver.rs` L201-233 | 再帰的 Struct/Enum/Tuple フィールド |
| 4.6 | Auto-derive ジェネリクス対応 | ✅ | `auto_derive.rs` | `Type::Generic`/`Type::Tuple` の再帰処理 |
| 4.7 | Bounds 統合 | ✅ | `bounds.rs` L51-58 | 失敗時にauto-deriveを試行 |
| **4.8** | **Send/Sync 残滓** | **⚠️** | **`solver.rs`, `auto_derive.rs`, `send_sync.rs`** | **`check_send_trait`/`check_sync_trait` メソッドの残滓；`BUILTIN_DERIVES` に Send/Sync を含む；`send_sync.rs` がまだ `pub mod` でエクスポート** |

---

## 5. クロージャキャプチャ（RFC-023）

| # | チェック項目 | 状態 | ファイル | 説明 |
|---|--------|------|------|------|
| 5.1 | capture.rs モジュール | ✅ | `capture.rs` | 約1065行、テスト含む |
| 5.2 | キャプチャ分析 | ✅ | `capture.rs` L133-170 | Lambda本体を走査、Read/Write を区別 |
| 5.3 | エスケープ分析 | ✅ | `capture.rs` L83-123 | Spawn/Return/代入 → Escaping |
| 5.4 | パターン選択 | ✅ | `capture.rs` L186-206 | Dup→Copy, Escaping→Move, Inline→Borrow/BorrowMut |
| 5.5 | 型チェックへの統合 | ✅ | `expressions.rs` L934-968 | Lambda推論時にキャプチャ分析を呼び出し |
| 5.6 | MakeClosure env 填充 | ✅ | `ir_gen.rs` L3177-3243 | `env: Vec::new()` が実際のキャプチャ変数に置き換え済み |

---

## 問題リスト（優先度順）

### P0 — ブロッキング項目（借用トークンがランタイムで動作しない）

| # | 問題 | 場所 | 影響 | 修復方向 |
|---|------|------|------|----------|
| P0-1 | IR に Borrow/Release 命令がない | `ir.rs` | 借用トークンをIRで表現できない | 新規 `Borrow { dst, src, mutable }` と `Release { src }` 命令を追加 |
| P0-2 | `Expr::Borrow` IR 生成缺失 | `ir_gen.rs` | `&expr` がIR段階で黙って無視される | `generate_expr_ir` に Borrow 分岐を追加、Borrow 命令を生成 |
| P0-3 | 解釈器に Borrow 処理がない | `execute.rs` | ランタイムで借用操作を実行できない | `BytecodeInstr::Borrow`/`Release` 処理を追加 |

### P1 — 重要項目（意味的正確性）

| # | 問題 | 場所 | 影響 | 修復方向 |
|---|------|------|------|----------|
| P1-1 | `&T` が Dup を満たさない | `solver.rs` L201-233 | RFCの中核意味「&T は自由に複製可能」に反する | `MonoType::Ref { mutable: false, .. } => true` 分岐を追加 |
| P1-2 | ブランド機構缺失 | `borrow_checker.rs` | トークン派生関係を追跡できない | コンパイル時一意IDと派生ブランドチェーンを追加 |
| P1-3 | `&mut T` IR 命令が到達不能 | `borrow_checker.rs` | 可変借用衝突検出が発火しない | IR層に MutRef 命令を追加すれば自動解決 |

### P2 — 改善項目（コード品質）

| # | 問題 | 場所 | 影響 | 修復方向 |
|---|------|------|------|----------|
| P2-1 | MakeClosure ZST 最適化 | `ir_gen.rs` L3196-3198 | トークンキャプチャ時に無意味なオーバーヘッドを生成 | ZST 型はenvをスキップ |
| P2-2 | Send/Sync 残滓 | `solver.rs`, `auto_derive.rs`, `send_sync.rs` | コード残滓 | `check_send_trait`/`check_sync_trait` を削除、`BUILTIN_DERIVES` をクリーンアップ |
| P2-3 | Bytecode From<MonoType> スタブ | `bytecode.rs` L1418-1424 | すべての方言が Void にマッピング | 本当の方言変換を実装 |

---

## 実装状況概要

```
フロントエンド（型システム + パーサ）  ████████████████████░  91% (10/11)
ミドルエンド（借用チェッカー）         ██████████████░░░░░░░  75% (9/12)
ミドルエンド（IR 生成）                ░░░░░░░░░░░░░░░░░░░░   0% (3コア中0/3)
Dup システム                           ██████████████████░░  88% (7/8)
クロージャキャプチャ                   ████████████████████ 100% (6/6)
```

**全体完成度：約65%**。フロントエンドは完成、ミドルエンドの借用チェッカーロジックは完成だがIR層が途切れている。

---

## 推奨実装順序

1. **P1-1**: `solver.rs` に `MonoType::Ref` Dup マッチを追加（1行変更）
2. **P2-2**: Send/Sync 残滓コードをクリーンアップ
3. **P0-1**: `ir.rs` に Borrow/Release IR 命令を追加
4. **P0-2**: `ir_gen.rs` に `Expr::Borrow` IR 生成を追加
5. **P0-3**: `execute.rs` に解釈器 Borrow 処理を追加
6. **P2-1**: MakeClosure ZST 最適化
7. **P1-2**: ブランド機構（延期可、現在の変数名追跡で基本機能は十分）