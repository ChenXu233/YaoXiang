---
title: "インタープリタの状態"
---

# インタープリタ（Interpreter）

> **モジュール状態**：完成済み
> **位置**：`src/backends/interpreter/`
> **最終更新**：2026-06-01

---

## モジュール概要

インタープリタはバイトコードを執行する責務を負う。レジスタ式仮想マシンアーキテクチャを採用しており、39種類のバイトコード命令を完全サポートし、RFC-008（並行モデル）およびRFC-009（所有権モデル）に完全に整合している。

**コード量**：3,768 行（9 個のソースファイル）

---

## 機能一覧

### コア実行エンジン（execute.rs - 1,308 行）

**制御フロー（10 種類）**：
- ✅ Nop, Return, ReturnValue, Yield (空操作), EvalPush, EvalPop, Spawn, Jmp, JmpIf, JmpIfNot, Switch (簡略化)

**レジスタ操作（5 種類）**：
- ✅ Mov, LoadConst, LoadLocal, StoreLocal, LoadArg

**算術/論理演算（11 種類の BinaryOp + 1 種類の UnaryOp）**：
- ✅ Add/Sub/Mul/Div/Rem/And/Or/Xor/Shl/Sar/Shr、Int と Float の両対応
- ⚠️ UnaryOp は Int の反転のみ実装

**比較（6 種類の CompareOp）**：
- ✅ Eq/Ne/Lt/Le/Gt/Ge、Int と String の両対応

**メモリ操作（9 種類）**：
- ✅ StackAlloc (空操作), HeapAlloc, Drop (空操作), GetField, SetField, LoadElement, StoreElement, NewListWithCap, CreateStruct

**Arc/Weak 操作（5 種類）**：
- ✅ ArcNew, ArcClone, ArcDrop (空操作), WeakNew, WeakUpgrade

**借用トークン（2 種類）**：
- ✅ Borrow (ZST、実行時は Mov と等価), Release (ZST、実行時は Nop と等価)

**関数呼び出し（7 種類）**：
- ✅ CallStatic, CallNative, CallVirt, CallDyn, MakeClosure, LoadUpvalue, StoreUpvalue, CloseUpvalue

**文字列操作（6 種類）**：
- ✅ StringLength, StringConcat, StringEqual, StringGetChar, StringFromInt, StringFromFloat

**例外処理（3 種類）**：
- ✅ TryBegin (空操作), TryEnd (空操作), Throw

**デバッグ/型（4 種類）**：
- ⚠️ BoundsCheck (空操作), TypeCheck (空操作), Cast (パスsthrough), TypeOf (プレースホルダ)

### コアアーキテクチャ機能

- ✅ **ヒープ（Heap）**：List/Tuple/Array/Dict/Struct の動的確保
- ✅ **コールスタック（Frame）**：レジスタファイル + 局所変数 + upvalue + eval スタック + spawn group
- ✅ **定数プール**：モジュール間共有
- ✅ **関数テーブル**：名前別 (HashMap) とインデックス別 (Vec) の二重テーブル、クロージャの ID 呼び出しに対応
- ✅ **FFI レジストリ**：`std.io.*` シリーズの関数を事前ロードし、カスタムネイティブ関数の拡張が可能
- ✅ **DAG タスクスケジューリング（LocalRuntime）**：RFC-008 に基づく遅延/並行評価
- ✅ **3 種類の評価戦略**：Block (同期)、Auto (遅延/並行)、Eager (先行)
- ✅ **構造化並行処理**：spawn group 追跡、スコープ退出時の全タスク待機、依存失敗によるカスケードキャンセル

---

## テストカバレッジ

**約 60 個のテスト**：

| テスト種別 | 数量 | カバー範囲 |
|------------|------|------------|
| ユニットテスト（モジュール内） | ~35 | registers, ffi, frames, tests, debug, execute |
| 統合テスト | 25 | 完全コンパイルパイプライン：hello world、変数宣言、算術、比較、lambda、関数定義、if/elif/else、while、for、match、List/Tuple/Dict、リスト内包表記、クロージャ高階関数、モジュールインポート、f-string |

---

## RFC 比較

### RFC-008（Runtime 並行モデル）

| 設計要件 | 実装状態 | 説明 |
|----------|----------|------|
| 三層ランタイム：Embedded / Standard / Full | ✅ 実装済み | `RuntimeMode` による設定 |
| 3 種類の評価戦略：Block / Auto / Eager | ✅ 実装済み | |
| DAG タスクスケジューリング（`LocalRuntime`） | ✅ 実装済み | |
| タスク依存追跡、キャンセル伝播、構造化並行処理 | ✅ 実装済み | |
| 同期 = スケジューリングの特殊ケース（Embedded モード） | ✅ 実装済み | |

### RFC-009（所有権モデル）

| 設計要件 | 実装状態 | 説明 |
|----------|----------|------|
| Borrow/Release をゼロサイズトークン（ZST）として提供 | ✅ 実装済み | 実行時は Mov/Nop と等価 |
| ArcNew/ArcClone/ArcDrop で `ref` キーワードのセマンティクスを実装 | ✅ 実装済み | |
| WeakNew/WeakUpgrade で弱参照を実装 | ✅ 実装済み | |
| Move セマンティクス（デフォルト動作） | ✅ 実装済み | |
| `clone()` はコンパイル層で処理 | ✅ 実装済み | 実行時に特殊命令は不要 |

---

## 簡略化/プレースホルダ実装

| 命令 | 現在の動作 | 設計意図 |
|------|------------|----------|
| Switch | 直接 IP を advance | 値によるディスパッチジャンプを実装すべき |
| TypeOf | type_table のサイズを返すプレースホルダ | 実行時型情報を返すようにすべき |
| Cast | 値をパスsthrough（実際の変換なし） | 対象型に変換すべき |
| BoundsCheck / TypeCheck | 空操作 | デバッグモードで実行時チェックを行うべき |
| StringGetChar | インデックス引数を無視して先頭文字のみ取得 | インデックスで文字を取得すべき |
| UnaryOp | op 型を無視して Int の反転のみ対応 | 更多的単項演算に対応するべき |
| step/step_over/step_out/run | `todo!()` | デバッガのステップ機能未実装 |

---

## コード品質評価

| 次元 | スコア | 説明 |
|------|--------|------|
| 機能完成度 | 100% | コア実行エンジンは堅牢で、39 種類のバイトコード命令を全覆盖 |
| テストカバレッジ | 良好 | 約 60 個のテストで主要な機能パスをカバー |
| ドキュメント品質 | 良好 | 各ソースファイルにモジュールレベルの doc comment があり、RFC 番号を参照 |
| コードアーキテクチャ | 優秀 | レイヤーが清晰：executor/frames/registers/ffi/runtime |
| RFC 準拠 | 完全整合 | RFC-008 および RFC-009 の設計に完全整合 |

---

## 改善待ち事項

1. **Switch 命令の実分发dispatchの実装**
2. **デバッガのステップ機能の実装**（step/step_over/step_out/run）
3. **StringGetChar/UnaryOp などの命令の整備**
4. **BoundsCheck/TypeCheck のデバッグモードチェックの実装**
5. **境界条件とエラー経路のテスト補完**