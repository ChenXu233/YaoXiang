---
title: "インタープリタの状態"
---

# インタープリタ（Interpreter）

> **モジュール状態**：安定（5 項目の改善待ち）
> **位置**：`src/backends/interpreter/`
> **最終更新**：2026-06-01

---

## モジュール概要

インタープリタはバイトコードの実行を担当する。レジスタ式仮想マシンアーキテクチャを採用しており、39 種類のバイトコード命令を完全サポートしている。RFC-008（並行モデル）および RFC-009（所有権モデル）と完全に整合している。

**コード量**：3,768 行（9 つのソースファイル）

---

## 機能一覧

### コア実行エンジン（execute.rs - 1,308 行）

**制御フロー（10 種類）**：
- ✅ Nop, Return, ReturnValue, Yield (何もしない), EvalPush, EvalPop, Spawn, Jmp, JmpIf, JmpIfNot, Switch (簡略化)

**レジスタ操作（5 種類）**：
- ✅ Mov, LoadConst, LoadLocal, StoreLocal, LoadArg

**算術/論理演算（11 種類の BinaryOp + 1 種類の UnaryOp）**：
- ✅ Add/Sub/Mul/Div/Rem/And/Or/Xor/Shl/Sar/Shr、Int と Float の両対応
- ⚠️ UnaryOp は Int の反転のみ実装

**比較（6 種類の CompareOp）**：
- ✅ Eq/Ne/Lt/Le/Gt/Ge、Int と String の両対応

**メモリ操作（9 種類）**：
- ✅ StackAlloc (何もしない), HeapAlloc, Drop (何もしない), GetField, SetField, LoadElement, StoreElement, NewListWithCap, CreateStruct

**Arc/Weak 操作（5 種類）**：
- ✅ ArcNew, ArcClone, ArcDrop (何もしない), WeakNew, WeakUpgrade

**借用トークン（2 種類）**：
- ✅ Borrow (ZST、実行時は Mov と同等), Release (ZST、実行時は Nop と同等)

**関数呼び出し（7 種類）**：
- ✅ CallStatic, CallNative, CallVirt, CallDyn, MakeClosure, LoadUpvalue, StoreUpvalue, CloseUpvalue

**文字列操作（6 種類）**：
- ✅ StringLength, StringConcat, StringEqual, StringGetChar, StringFromInt, StringFromFloat

**例外処理（3 種類）**：
- ✅ TryBegin (何もしない), TryEnd (何もしない), Throw

**デバッグ/型（4 種類）**：
- ⚠️ BoundsCheck (何もしない), TypeCheck (何もしない), Cast (パススルー), TypeOf (プレースホルダ)

### コアアーキテクチャ機能

- ✅ **ヒープ（Heap）**：List/Tuple/Array/Dict/Struct の動的確保
- ✅ **呼び出しスタック（Frame）**：レジスタファイル + 局所変数 + upvalue + eval スタック + spawn group
- ✅ **定数プール**：モジュール間共有
- ✅ **関数テーブル**：名前順 (HashMap) とインデックス順 (Vec) の二重テーブル、クロージャを ID で呼び出し可能
- ✅ **FFI レジストリ**：`std.io.*` シリーズの関数を事前ロード、カスタムネイティブ関数の拡張可能
- ✅ **DAG タスクスケジューリング（LocalRuntime）**：RFC-008 ベースの遅延/並行評価
- ✅ **3 種類の評価戦略**：Block (同期)、Auto (遅延/並行)、Eager (先行)
- ✅ **構造化並行性**：spawn group 追跡、スコープ退出時の全タスク待機、依存失敗時のカスケードキャンセル

---

## テストカバレッジ

**約 60 個のテスト**：

| テスト種類 | 数量 | カバー範囲 |
|------------|------|------------|
| ユニットテスト（モジュール内） | ~35 | registers, ffi, frames, tests, debug, execute |
| 統合テスト | 25 | 完全コンパイルパイプライン：hello world、変数宣言、算術、比較、lambda、関数定義、if/elif/else、while、for、match、List/Tuple/Dict、リスト内包表記、クロージャ高階関数、モジュールインポート、f-string |

---

## RFC 比較

### RFC-008（Runtime 並行モデル）

| 設計要件 | 実装状態 | 説明 |
|----------|----------|------|
| 3 層ランタイム：Embedded / Standard / Full | ✅ 実装済み | `RuntimeMode` で設定可能 |
| 3 種類の評価戦略：Block / Auto / Eager | ✅ 実装済み | |
| DAG タスクスケジューリング（`LocalRuntime`） | ✅ 実装済み | |
| タスク依存追跡、キャンセル伝播、構造化並行性 | ✅ 実装済み | |
| 同期 = スケジュールの特例（Embedded モード） | ✅ 実装済み | |

### RFC-009（所有権モデル）

| 設計要件 | 実装状態 | 説明 |
|----------|----------|------|
| Borrow/Release をゼロサイズトークン（ZST）として実装 | ✅ 実装済み | 実行時は Mov/Nop と同等 |
| ArcNew/ArcClone/ArcDrop で `ref` キーワードのセマンティクスを実装 | ✅ 実装済み | |
| WeakNew/WeakUpgrade で弱参照を実装 | ✅ 実装済み | |
| Move セマンティクス（デフォルト動作） | ✅ 実装済み | |
| `clone()` はコンパイル層で処理 | ✅ 実装済み | 実行時は特殊命令不要 |

---

## 簡略化/プレースホルダ実装

| 命令 | 現在の動作 | 設計意図 |
|------|----------|----------|
| Switch | IP を直接 advance | 値でディスパッチしてジャンプすべき |
| TypeOf | type_table の長さを返すプレースホルダ | 実行時の型情報を返すべき |
| Cast | 値を通す（実際の変換なし） | 対象型に変換すべき |
| BoundsCheck / TypeCheck | 何もしない | debug モードで実行時チェックすべき |
| StringGetChar | インデックス引数を無視して先頭文字のみ取得 | インデックスで文字を取得すべき |
| UnaryOp | op 型を無視して Int の反転のみ | 他の単項演算もサポートすべき |
| step/step_over/step_out/run | `todo!()` | デバッガのステップ実行機能未実装 |

---

## コード品質評価

| 次元 | 評価 | 説明 |
|------|------|------|
| 未完了事項 | 5 | Switch 命令、デバッガのステップ実行、命令の完善、debug チェック、テスト補完 |
| テストカバレッジ | 良好 | 約 60 個のテスト、主要な機能パスをカバー |
| ドキュメント品質 | 良好 | 各ソースファイルにモジュールレベルの doc comment、RFC 番号の参照あり |
| コードアーキテクチャ | 優秀 | レイヤリングが清晰：executor/frames/registers/ffi/runtime |
| RFC 準拠 | 完全整合 | RFC-008 および RFC-009 の設計と完全整合 |

---

## 改善待ち項目

1. **Switch 命令の本物のディスパッチ実装**
2. **デバッガのステップ実行機能実装**（step/step_over/step_out/run）
3. **StringGetChar/UnaryOp などの命令完善**
4. **BoundsCheck/TypeCheck の debug モードチェック実装**
5. **境界条件とエラーパステストの補完**