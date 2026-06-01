---
title: "コード生成ステータス"
---

# コード生成（Codegen）

> **モジュールステータス**：完成（基本機能）
> **位置**：`src/middle/passes/codegen/`
> **最終更新**：2026-06-01

---

## モジュール概要

コード生成モジュールは、IR（中間表現）をバイトコードに変換する役割を担います。デバッグ情報セグメントを含む完全な `.yx` バイト码ファイル形式（.42形式）をサポートします。

**コード量**：約 2400 行（7 つのソースファイル）

---

## 機能一覧

### トランスレーター（translator.rs - 1,073 行）

**算術/ビット演算（すべて完成）**：
- ✅ Add, Sub, Mul, Div, Mod, And, Or, Xor, Shl, Shr, Sar, Neg

**比較演算（すべて完成）**：
- ✅ Eq, Ne, Lt, Le, Gt, Ge

**制御フロー（すべて完成）**：
- ✅ Jmp, JmpIf, JmpIfNot, Ret、ジャンプオフセットバックフィル機構を含む

**関数呼び出し（すべて完成）**：
- ✅ CallStatic, CallNative, CallVirt, CallDyn, TailCall

**変数操作（すべて完成）**：
- ✅ Move, Load, Store

**メモリ/オブジェクト操作（すべて完成）**：
- ✅ Alloc, AllocArray, HeapAlloc
- ✅ LoadField, StoreField, LoadIndex, StoreIndex
- ✅ CreateStruct

**所有権システム（すべて完成）**：
- ✅ Drop, ArcNew, ArcClone, ArcDrop
- ✅ Borrow, Release（借用トークン）
- ✅ ShareRef（一時的に Nop で実装）

**クロージャ/Upvalue（すべて完成）**：
- ✅ MakeClosure, LoadUpvalue, StoreUpvalue, CloseUpvalue

**文字列操作（すべて完成）**：
- ✅ StringLength, StringConcat, StringGetChar, StringFromInt, StringFromFloat

**並行処理（部分的に完成）**：
- ✅ Spawn
- ✅ EvalPush, EvalPop, Yield

### バイトコードファイル形式（bytecode.rs）

- ✅ 完全な `BytecodeFile` 構造：ファイルヘッダー + 型テーブル + 定数プール + コードセグメント + オプションのデバッグ情報セグメント
- ✅ ファイルヘッダー：マジックナンバー `YXBC` (0x59584243)、バージョン番号 2
- ✅ 混合エンディアンサポート：マジックナンバーはビッグエンディアン、データはリトルエンディアン
- ✅ 定数型サポート：Void, Bool, Int, Float, Char, String, Bytes
- ✅ **デバッグ情報セグメント**（DebugSection）：ソース位置マッピングをサポート（IP -> Span）

### オペコードシステム（opcode.rs、shared）

- ✅ 合計 **80+ 個の Opcode** を定義、12 のカテゴリに分類
- ✅ 完全な `TryFrom<u8>` 実装
- ✅ 各種ヘルパーメソッド：`is_numeric_op`, `is_call_op`, `is_jump_op` など

---

## 未実装/プレースホルダーの命令

| IR 命令 | 現在の実装 | 説明 |
|---------|----------|------|
| ShareRef | Nop | コードに TODO コメントあり |
| Free | Nop | 操作なし |
| Dup, Swap | Nop | スタック操作は未実装 |
| UnsafeBlockStart/End | Nop | unsafe ブロックマーキング |
| PtrFromRef/PtrDeref/PtrStore/PtrLoad | すべて Nop | 生のポインタ操作は未サポート |
| TypeTest | プレースホルダー TypeCheck | オペランドが [0, 0, 0] でハードコード |
| Cast | オペランドがハードコード | ターゲット型がエンコードされていない |

---

## テストカバレッジ

**13 件のユニットテスト**、すべてパス：

| テストファイル | テスト数 | カバー範囲 |
|----------|----------|----------|
| `mod.rs` | 1 | 基本コンテキスト作成 |
| `buffer.rs` | 2 | 定数プール追加/取得、バイトコードバッファエミッション |
| `emitter.rs` | 2 | 命令エミッションとマッピング、ジャンプバックフィル待機 |
| `operand.rs` | 2 | レジスタ変換、オーバーフロー検出 |
| `flow.rs` | 5 | ラベルジェネレータ、レジスタアロケータ、フローマネージャー、記号表基礎、スコープネスト |
| `bytecode.rs` | 1 | DebugSection のエンコード/デコード往返テスト |

**テスト不足点**：
- ❌ translator.rs に独立テストがない
- ❌ 完全な `generate()` プロセスの統合テストがない
- ❌ エンドツーエンドテストがない（ソースコード -> バイトコード -> 逆シリアル化検証）

---

## コード品質評価

| 次元 | スコア | 説明 |
|------|------|------|
| 機能完成度 | 85% | コア翻訳プロセスは完整、少数命令はプレースホルダー |
| テストカバレッジ | 不十分 | 約 30%、コア translator がテスト欠落 |
| ドキュメンテーション | 中程度 | モジュールと型のドキュメントあり、形式仕様とユーザードキュメント欠落 |
| コード品質 | 良好 | アーキテクチャ清晰、責務分明だが、translator.rs が大きすぎる |
| RFC 一貫性 | 基本的一致 | VM バックエンドの要件を満たしている、LLVM バックエンドは未実装 |

---

## 改善項目

1. **translator.rs ユニットテストの補完**
2. **ShareRef/Dup/Swap などのプレースホルダー命令の実装**
3. **unsafe/ポインタ命令の実装**
4. **translator.rs の分割**（1,073 行过大）
5. **バイトコード形式仕様ドキュメントの追加**