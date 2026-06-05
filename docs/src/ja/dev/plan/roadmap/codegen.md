---
title: "コード生成ステータス"
---

# コード生成（Codegen）

> **モジュールステータス**：ギャップあり（5 項目改善待ち）
> **位置**：`src/middle/passes/codegen/`
> **最終更新**：2026-06-01

---

## モジュールの概要

コード生成モジュールは、IR（中間表現）をバイトコードに翻訳する役割を担います。デバッグ情報セグメントを含む、完全な `.yx` バイトコードファイル形式（.42 形式）をサポートしています。

**コード量**：約 2400 行（7 つのソースファイル）

---

## 機能一覧

### 翻訳者（translator.rs - 1,073 行）

**算術/ビット演算（全て完了）**：
- ✅ Add, Sub, Mul, Div, Mod, And, Or, Xor, Shl, Shr, Sar, Neg

**比較演算（全て完了）**：
- ✅ Eq, Ne, Lt, Le, Gt, Ge

**制御フロー（全て完了）**：
- ✅ Jmp, JmpIf, JmpIfNot, Ret、ジャンプオフセットバックフィリング機構を含む

**関数呼び出し（全て完了）**：
- ✅ CallStatic, CallNative, CallVirt, CallDyn, TailCall

**変数操作（全て完了）**：
- ✅ Move, Load, Store

**メモリ/オブジェクト操作（全て完了）**：
- ✅ Alloc, AllocArray, HeapAlloc
- ✅ LoadField, StoreField, LoadIndex, StoreIndex
- ✅ CreateStruct

**所有権システム（全て完了）**：
- ✅ Drop, ArcNew, ArcClone, ArcDrop
- ✅ Borrow, Release（借用トークン）
- ✅ ShareRef（一時的に Nop として実装）

**クロージャ/Upvalue（全て完了）**：
- ✅ MakeClosure, LoadUpvalue, StoreUpvalue, CloseUpvalue

**文字列操作（全て完了）**：
- ✅ StringLength, StringConcat, StringGetChar, StringFromInt, StringFromFloat

**並行処理（部分的に完了）**：
- ✅ Spawn
- ✅ EvalPush, EvalPop, Yield

### バイトコードファイル形式（bytecode.rs）

- ✅ 完全な `BytecodeFile` 構造体：ファイルヘッダー + 型テーブル + 定数プール + コードセグメント + オプションのデバッグ情報セグメント
- ✅ ファイルヘッダー：マジックナンバー `YXBC` (0x59584243)、バージョン番号 2
- ✅ 混合エンディアンサポート：マジックナンバーはビッグエンディアン、データはリトルエンディアン
- ✅ 定数タイプサポート：Void, Bool, Int, Float, Char, String, Bytes
- ✅ **デバッグ情報セグメント**（DebugSection）：ソース位置マッピングをサポート（IP -> Span）

### オペコードシステム（opcode.rs、shared）

- ✅ 全部で **80+ 個の Opcode** を定義、12 のカテゴリに分類
- ✅ 完全な `TryFrom<u8>` 実装
- ✅ 各種ヘルパーメソッド：`is_numeric_op`, `is_call_op`, `is_jump_op` など

---

## 未実装/プレースホルダーの命令

| IR 命令 | 現在の実装 | 説明 |
|---------|----------|------|
| ShareRef | Nop | コード内に TODO コメントあり |
| Free | Nop | 処理なし |
| Dup, Swap | Nop | スタック操作は未実装 |
| UnsafeBlockStart/End | Nop | unsafe ブロックマーカー |
| PtrFromRef/PtrDeref/PtrStore/PtrLoad | 全て Nop | 生のポインタ操作は未サポート |
| TypeTest | プレースホルダー TypeCheck | オペランドがハードコード [0, 0, 0] |
| Cast | オペランドがハードコード | ターゲット型がエンコードされていない |

---

## テストカバレッジ

**13 のユニットテスト**、全て合格：

| テストファイル | テスト数 | カバー範囲 |
|----------|----------|----------|
| `mod.rs` | 1 | 基本的なコンテキスト作成 |
| `buffer.rs` | 2 | 定数プール追加/取得、バイトコードバッファエミッション |
| `emitter.rs` | 2 | 命令エミッションとマッピング、ジャンプバックフィリング |
| `operand.rs` | 2 | レジスタ変換、オーバーフロー検出 |
| `flow.rs` | 5 | ラベル生成器、レジスタアロケータ、フローマネージャー、記号表の基礎、スコープネスト |
| `bytecode.rs` | 1 | DebugSection のエンコード/デコード往返テスト |

**テスト不足の部分**：
- ❌ translator.rs に独立したテストがない
- ❌ 完全な `generate()` プロセスをカバーする統合テストがない
- ❌ エンドツーエンドテストがない（ソースコード -> バイトコード -> 逆シリアル化検証）

---

## コード品質評価

| Dimensi | 評価 | 説明 |
|------|------|------|
| 未完了事項 | 5 | テスト補足、プレースホルダー命令、unsafe 命令、翻訳者の分割、フォーマット文書化 |
| テストカバレッジ | 不十分 | 約 30%、コアの translator がテスト欠如 |
| 文書化 | 中程度 | モジュールと型のドキュメントあり、フォーマット仕様とユーザードキュメントが不足 |
| コード品質 | 良好 | アーキテクチャ清晰、責務分離良好だが、translator.rs が过大 |
| RFC 一貫性 | 基本的な一貫性 | VM バックエンドの要件を満たしている、LLVM バックエンドは未実装 |

---

## 改善待ちな項目

1. **translator.rs のユニットテストを補足する**
2. **ShareRef/Dup/Swap などのプレースホルダー命令を実装する**
3. **unsafe/ポインタ命令を実装する**
4. **translator.rs を分割する**（1,073 行は过大）
5. **バイトコードフォーマット仕様ドキュメントを追加する**